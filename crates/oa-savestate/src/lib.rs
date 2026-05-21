//! oa-savestate — save-state machinery shared by the shell.
//!
//! Today this is mostly the rewind ring: a memory-bounded buffer of recent
//! core save-state snapshots, owned by the emu thread, used to walk
//! emulation backwards a frame at a time when the user holds the rewind key.
//!
//! Snapshots are stored zstd-compressed inside the ring (since the Phase D
//! multi-core CPU awareness work — 2026-05-21). Compression typically
//! buys 5–10× memory reduction on raw core save-states, letting the ring
//! hold proportionally more rewind history for the same byte cap. Push
//! pays a few-ms compression cost (zstd level 1, the fast preset); pop
//! pays a sub-ms decompression cost. zstd's magic number drives the
//! "is this compressed?" probe so a failed compression on push (rare —
//! OOM only) silently degrades to storing raw bytes; pop still works.
//!
//! On-disk save states (F5/F8 slots) still serialise straight through
//! `oa_core::Core::{save_state, load_state}` — they don't pass through this
//! crate yet. A future revision may fold versioned-blob + zstd compression
//! work in here so both surfaces share one container format.

#![deny(rust_2018_idioms)]
#![warn(missing_docs)]

pub mod tas;

use std::collections::VecDeque;

/// Default capture interval: 6 frames between snapshots. At 60 fps this is
/// ~100 ms granularity — RetroArch's default. Lower = smoother rewind +
/// finer scrubbing, but more RAM + more CPU per frame. Higher = coarser
/// rewind but cheaper.
pub const DEFAULT_CAPTURE_INTERVAL_FRAMES: u32 = 6;

/// Default rewind buffer cap: 10 seconds at 60 fps with the default capture
/// interval = 100 snapshots. Per-snapshot size varies wildly by system
/// (PCE ~50 KB, SNES ~300 KB), so the actual cap is bytes-based; this is
/// just the seconds the UI defaults to and the byte cap is derived from
/// `seconds × fps / interval × expected_snapshot_bytes`. The byte cap is
/// the hard limit at runtime.
pub const DEFAULT_BUFFER_SECONDS: u32 = 10;

/// Default byte cap if the caller can't supply one: 64 MiB. Big enough to
/// hold 10 s of any current core at 6-frame interval with headroom. Cores
/// with truly enormous states (Mednafen Saturn etc.) will fit fewer
/// seconds; that's surfaced through the live `seconds_held()` accessor.
///
/// Note: bytes are counted *post-compression*, so this cap accommodates
/// significantly more emulation history than the same number suggests for
/// raw states. PS2 / N64 / GameCube save-states typically compress 5–10×.
pub const DEFAULT_MAX_BYTES: usize = 64 * 1024 * 1024;

/// zstd compression level used by the rewind ring. Level 1 is the fast
/// preset — typical throughput 300–600 MB/s on modern desktop CPUs. The
/// push-time cost on a 16 MB PS2 state is ~30–50 ms; on a 50 KB PCE state
/// it's well under a millisecond. The cost is paid once every
/// `capture_interval_frames` (default 6 = every 100 ms at 60 fps), so the
/// amortised CPU footprint is modest. Higher levels would compress better
/// but at exponentially worse push cost — wrong tradeoff for a per-frame
/// hot path.
pub const REWIND_COMPRESSION_LEVEL: i32 = 1;

/// zstd magic-number first 4 bytes (little-endian: `28 B5 2F FD`). Used to
/// probe whether a stored snapshot is compressed — defensive against
/// failed compressions and against any future migration that stores
/// uncompressed bytes alongside compressed ones.
const ZSTD_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];

/// User-facing rewind configuration. Lives in OA-wide / per-system / per-game
/// settings with the three-tier inheritance chain — resolved at launch time
/// into one `RewindConfig` that the emu thread consumes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RewindConfig {
    /// Whether the ring captures snapshots at all. When false, the emu
    /// thread skips both capture and rewind-key handling.
    pub enabled: bool,
    /// How many frames between snapshots. 1 = every frame (smoothest, most
    /// expensive); 6 = ~100 ms at 60 fps (default).
    pub capture_interval_frames: u32,
    /// Hard cap on the ring's memory footprint in bytes. The ring evicts the
    /// oldest snapshot whenever pushing would exceed this cap.
    pub max_bytes: usize,
}

impl Default for RewindConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            capture_interval_frames: DEFAULT_CAPTURE_INTERVAL_FRAMES,
            max_bytes: DEFAULT_MAX_BYTES,
        }
    }
}

/// A bounded ring buffer of core save-state snapshots.
///
/// Push adds to the back; pop_back returns the newest snapshot (one rewind
/// step). The ring evicts from the front whenever the byte total exceeds
/// `max_bytes`, so the oldest snapshots are dropped first — recent history
/// wins over deep history when budget is tight.
#[derive(Debug)]
pub struct RewindRing {
    snapshots: VecDeque<Vec<u8>>,
    current_bytes: usize,
    max_bytes: usize,
}

impl RewindRing {
    /// Build an empty ring with the given byte cap. `max_bytes` is the
    /// memory ceiling; the ring won't hold more bytes than this even if
    /// the snapshot count looks small.
    pub fn new(max_bytes: usize) -> Self {
        Self {
            snapshots: VecDeque::new(),
            current_bytes: 0,
            max_bytes,
        }
    }

    /// Resize the byte cap. Drops oldest snapshots until under the new cap.
    /// Used when the user changes the buffer size in Settings without
    /// having to flush the whole ring.
    pub fn set_max_bytes(&mut self, max_bytes: usize) {
        self.max_bytes = max_bytes;
        self.evict_until_under_cap();
    }

    /// Append a fresh snapshot. The bytes are zstd-compressed before
    /// being stored — the byte-cap accounting (and the value returned
    /// from `byte_size()`) reflects the post-compression footprint, so
    /// a 64 MiB cap typically holds *substantially* more than 64 MiB of
    /// raw history. Evicts the oldest entries as needed.
    pub fn push(&mut self, snapshot: Vec<u8>) {
        let stored = match zstd::encode_all(&snapshot[..], REWIND_COMPRESSION_LEVEL) {
            Ok(c) => c,
            Err(e) => {
                // Compression should essentially never fail at level 1
                // (zstd itself doesn't fail for valid input + valid
                // level); if it does, fall back to raw bytes so we
                // don't lose a frame of history. The pop path uses the
                // zstd magic-number probe so mixed compressed/raw
                // entries in the ring are fine.
                log::warn!(
                    "oa-savestate: zstd compress failed ({} B): {e}; storing raw",
                    snapshot.len()
                );
                snapshot
            }
        };
        self.current_bytes = self.current_bytes.saturating_add(stored.len());
        self.snapshots.push_back(stored);
        self.evict_until_under_cap();
    }

    /// Pop the newest snapshot — the one closest to "now". Returns the
    /// *decompressed* bytes ready for `Core::load_state`. Returns None
    /// if the ring is empty (or has been fully rewound).
    pub fn pop_back(&mut self) -> Option<Vec<u8>> {
        let stored = self.snapshots.pop_back()?;
        self.current_bytes = self.current_bytes.saturating_sub(stored.len());
        Some(decompress_if_needed(stored))
    }

    /// Peek at the newest snapshot without consuming it. Used by scrubbing
    /// UI when it wants to preview the current rewind head. Returns owned
    /// decompressed bytes — since 2026-05-21 the ring stores zstd-compressed
    /// snapshots, there's no useful `&[u8]` view to hand back.
    pub fn peek_back(&self) -> Option<Vec<u8>> {
        let stored = self.snapshots.back()?;
        Some(decompress_if_needed(stored.clone()))
    }

    /// Peek at a snapshot by distance from the newest. `steps_back = 0` is
    /// the newest (same as [`peek_back`]); `steps_back = len - 1` is the
    /// oldest still held. Returns None when the index is out of bounds.
    /// Used by the scrubbing UI for preview without mutating the ring.
    /// Returns owned decompressed bytes — see [`peek_back`] for context.
    pub fn peek_at(&self, steps_back: usize) -> Option<Vec<u8>> {
        let len = self.snapshots.len();
        if steps_back >= len {
            return None;
        }
        // VecDeque is newest-at-back: index `len - 1 - steps_back` from
        // the front matches `steps_back` from the back.
        let stored = self.snapshots.get(len - 1 - steps_back)?;
        Some(decompress_if_needed(stored.clone()))
    }

    /// Drop every snapshot newer than the one at `steps_back`. After this
    /// call the snapshot that was at `steps_back` is the new newest.
    /// `steps_back = 0` is a no-op. Returns the number of snapshots dropped.
    /// Used by the scrubbing UI on commit — the user has chosen a point in
    /// the past, and the "future" history above it gets rewritten.
    pub fn truncate_above(&mut self, steps_back: usize) -> usize {
        let len = self.snapshots.len();
        if steps_back >= len {
            // Out of bounds — nothing to do. (Caller should clamp first.)
            return 0;
        }
        let mut dropped = 0;
        for _ in 0..steps_back {
            if let Some(snap) = self.snapshots.pop_back() {
                self.current_bytes = self.current_bytes.saturating_sub(snap.len());
                dropped += 1;
            }
        }
        dropped
    }

    /// Drop every snapshot. Called on ROM swap / unload — old snapshots
    /// from a previous game are nonsense to feed back into a new game.
    pub fn clear(&mut self) {
        self.snapshots.clear();
        self.current_bytes = 0;
    }

    /// Number of snapshots currently held.
    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    /// True iff no snapshots are held.
    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }

    /// Total bytes the ring is currently using.
    pub fn byte_size(&self) -> usize {
        self.current_bytes
    }

    /// The configured byte cap.
    pub fn max_bytes(&self) -> usize {
        self.max_bytes
    }

    /// Estimate how many seconds of emulation the ring currently represents.
    /// Caller supplies the system's `fps` + capture interval — the ring
    /// itself is unitless. Useful for the rewind status row in the UI.
    pub fn seconds_held(&self, fps: f64, capture_interval_frames: u32) -> f64 {
        if fps <= 0.0 || capture_interval_frames == 0 {
            return 0.0;
        }
        let frames_per_snapshot = capture_interval_frames as f64;
        let seconds_per_snapshot = frames_per_snapshot / fps;
        self.snapshots.len() as f64 * seconds_per_snapshot
    }

    fn evict_until_under_cap(&mut self) {
        // Always retain at least one snapshot — losing the only history we
        // have to a momentary cap squeeze is worse than briefly exceeding
        // the cap. New pushes will reset the equilibrium on the next frame.
        while self.current_bytes > self.max_bytes && self.snapshots.len() > 1 {
            if let Some(dropped) = self.snapshots.pop_front() {
                self.current_bytes = self.current_bytes.saturating_sub(dropped.len());
            }
        }
    }
}

/// Decompress a stored snapshot if its first 4 bytes match the zstd
/// magic number; otherwise return the bytes as-is. Lets push degrade to
/// "store raw on compression failure" without breaking pop.
fn decompress_if_needed(stored: Vec<u8>) -> Vec<u8> {
    if stored.len() < 4 || stored[..4] != ZSTD_MAGIC {
        return stored;
    }
    match zstd::decode_all(&stored[..]) {
        Ok(v) => v,
        Err(e) => {
            // Decode of zstd-magic'd bytes failing is bad — corruption or
            // a malformed compression on push. Best we can do is return
            // the raw bytes; load_state will most likely fail downstream
            // and the user will see the symptom (rewind step did nothing)
            // rather than crash.
            log::warn!(
                "oa-savestate: zstd decode failed ({} B compressed): {e}; returning raw bytes",
                stored.len()
            );
            stored
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_pop_back_lifo_order() {
        // Byte-size assertions can't survive compression — they're
        // covered by the dedicated compression test below. This test
        // now only checks order semantics: pop_back returns most-recent
        // first, the ring empties to zero bytes when fully drained, and
        // decompression on pop recovers the original payload.
        let mut ring = RewindRing::new(1024);
        ring.push(vec![1, 2, 3]);
        ring.push(vec![4, 5, 6]);
        ring.push(vec![7, 8, 9]);
        assert_eq!(ring.len(), 3);
        assert_eq!(ring.pop_back().unwrap(), vec![7, 8, 9]);
        assert_eq!(ring.pop_back().unwrap(), vec![4, 5, 6]);
        assert_eq!(ring.pop_back().unwrap(), vec![1, 2, 3]);
        assert!(ring.pop_back().is_none());
        assert_eq!(ring.byte_size(), 0);
    }

    #[test]
    fn evicts_oldest_when_over_byte_cap() {
        // With compression, stored sizes are smaller than input sizes —
        // but the cap is still enforced on the stored size. Use highly
        // compressible payloads + a tight cap so the assertion stays
        // meaningful even after zstd squashes them.
        let mut ring = RewindRing::new(80);
        // 1024 bytes of all-zeros compresses to ~20-30 bytes; pushing
        // four of them eventually crosses the cap.
        for _ in 0..6 {
            ring.push(vec![0u8; 1024]);
        }
        // Hard to predict the exact len under compression, but the
        // byte cap must be honoured and the ring must retain ≥1.
        assert!(ring.byte_size() <= 80, "byte_size {} > cap 80", ring.byte_size());
        assert!(ring.len() >= 1);
        // Pop ordering still works after eviction.
        let recovered = ring.pop_back().expect("at least one held");
        assert_eq!(recovered.len(), 1024);
        assert!(recovered.iter().all(|b| *b == 0));
    }

    #[test]
    fn always_retains_at_least_one_snapshot() {
        // Even if a single snapshot busts the cap, we keep it rather than
        // emptying the ring. Better to briefly exceed than to lose all
        // history. (1024 bytes of incompressible random data is the test
        // payload — zstd can't shrink it materially, so the single push
        // exceeds the cap on its own.)
        let mut ring = RewindRing::new(5);
        // Use non-zero non-repeating bytes so compression doesn't
        // accidentally fit inside the cap.
        let snapshot: Vec<u8> = (0..100u16).map(|x| (x.wrapping_mul(7919)) as u8).collect();
        ring.push(snapshot.clone());
        assert_eq!(ring.len(), 1);
        // pop returns the original bytes after decompression
        assert_eq!(ring.pop_back().unwrap(), snapshot);
    }

    #[test]
    fn set_max_bytes_evicts_immediately() {
        // Same compression-aware assertion: post-eviction byte_size must
        // be ≤ new cap, exact counts depend on zstd's compression ratio.
        let mut ring = RewindRing::new(1024 * 1024);
        for _ in 0..10 {
            ring.push(vec![0u8; 50]);
        }
        assert_eq!(ring.len(), 10);
        ring.set_max_bytes(64);
        assert!(ring.byte_size() <= 64, "byte_size {} > cap 64", ring.byte_size());
    }

    #[test]
    fn clear_drops_everything() {
        let mut ring = RewindRing::new(1024);
        ring.push(vec![1, 2, 3]);
        ring.push(vec![4, 5, 6]);
        ring.clear();
        assert_eq!(ring.len(), 0);
        assert_eq!(ring.byte_size(), 0);
        assert!(ring.pop_back().is_none());
    }

    #[test]
    fn compression_actually_shrinks_compressible_payloads() {
        // A 100 KB run of zeros should compress dramatically — zstd
        // headers + RLE bring it well under 1 KB. This is the headline
        // win that justified the Phase D work: rewind history depth
        // scales 5-10x for free on real save-state data (which is
        // mostly RAM, often very repetitive).
        let mut ring = RewindRing::new(usize::MAX);
        let raw_size = 100 * 1024;
        ring.push(vec![0u8; raw_size]);
        let stored = ring.byte_size();
        assert!(
            stored < raw_size / 50,
            "expected ≥50x compression on zero-fill, got {raw_size} → {stored}"
        );
        // Round-trip still recovers original bytes.
        let recovered = ring.pop_back().unwrap();
        assert_eq!(recovered.len(), raw_size);
        assert!(recovered.iter().all(|b| *b == 0));
    }

    #[test]
    fn decompress_passes_through_uncompressed_bytes() {
        // Verifies the magic-number probe so a future failed-compress-
        // and-store-raw scenario still pops correctly. The fake stored
        // payload doesn't start with the zstd magic, so decompress_if_needed
        // returns it untouched.
        let raw = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x42, 0x42, 0x42];
        let out = decompress_if_needed(raw.clone());
        assert_eq!(out, raw);
    }

    #[test]
    fn seconds_held_matches_capture_cadence() {
        // 100 snapshots at 6 frames/snapshot, 60 fps = 100 * 6 / 60 = 10 s.
        let mut ring = RewindRing::new(usize::MAX);
        for _ in 0..100 {
            ring.push(vec![0u8; 1]);
        }
        let s = ring.seconds_held(60.0, 6);
        assert!((s - 10.0).abs() < 1e-9, "expected 10.0 seconds, got {s}");
        // 0 fps / 0 interval guard.
        assert_eq!(ring.seconds_held(0.0, 6), 0.0);
        assert_eq!(ring.seconds_held(60.0, 0), 0.0);
    }

    #[test]
    fn peek_at_indexes_from_newest() {
        let mut ring = RewindRing::new(1024);
        ring.push(vec![1, 1, 1]);
        ring.push(vec![2, 2, 2]);
        ring.push(vec![3, 3, 3]);
        // steps_back = 0 -> newest (3,3,3); steps_back = 2 -> oldest (1,1,1).
        // peek_at returns owned decompressed bytes (Vec<u8>).
        assert_eq!(ring.peek_at(0), Some(vec![3u8, 3, 3]));
        assert_eq!(ring.peek_at(1), Some(vec![2u8, 2, 2]));
        assert_eq!(ring.peek_at(2), Some(vec![1u8, 1, 1]));
        // Out of bounds returns None instead of panicking.
        assert_eq!(ring.peek_at(3), None);
        assert_eq!(ring.peek_at(99), None);
        // peek_at doesn't mutate.
        assert_eq!(ring.len(), 3);
    }

    #[test]
    fn truncate_above_drops_newer_snapshots() {
        let mut ring = RewindRing::new(1024);
        for n in 0..5 {
            ring.push(vec![n; 10]);
        }
        // Ring is [0,1,2,3,4] front-to-back; newest at back.
        // truncate_above(2) drops the 2 newest, leaving [0,1,2].
        let dropped = ring.truncate_above(2);
        assert_eq!(dropped, 2);
        assert_eq!(ring.len(), 3);
        // The remaining newest is `2`. peek_back returns owned
        // decompressed bytes since the compression refactor.
        assert_eq!(ring.peek_back().unwrap()[0], 2);
        // steps_back = 0 is a no-op.
        assert_eq!(ring.truncate_above(0), 0);
        assert_eq!(ring.len(), 3);
        // Out-of-bounds steps_back is a no-op (caller should clamp).
        assert_eq!(ring.truncate_above(99), 0);
        assert_eq!(ring.len(), 3);
    }

    #[test]
    fn default_config_disabled_by_default() {
        // Off-by-default — the user has to opt in. (Rewind has a non-zero
        // RAM + CPU cost; not every session wants it.)
        let cfg = RewindConfig::default();
        assert!(!cfg.enabled);
        assert_eq!(cfg.capture_interval_frames, DEFAULT_CAPTURE_INTERVAL_FRAMES);
        assert_eq!(cfg.max_bytes, DEFAULT_MAX_BYTES);
    }
}
