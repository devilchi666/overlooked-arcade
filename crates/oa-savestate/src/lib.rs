//! oa-savestate — save-state machinery shared by the shell.
//!
//! Today this is mostly the rewind ring: a memory-bounded buffer of recent
//! core save-state snapshots, owned by the emu thread, used to walk
//! emulation backwards a frame at a time when the user holds the rewind key.
//!
//! On-disk save states (F5/F8 slots) still serialise straight through
//! `oa_core::Core::{save_state, load_state}` — they don't pass through this
//! crate yet. A future revision may fold versioned-blob + zstd compression
//! work in here so both surfaces share one container format; for now the ring
//! is intentionally minimal — push raw bytes, pop raw bytes, hard cap on size.

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
pub const DEFAULT_MAX_BYTES: usize = 64 * 1024 * 1024;

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

    /// Append a fresh snapshot. Evicts the oldest entries as needed.
    pub fn push(&mut self, snapshot: Vec<u8>) {
        self.current_bytes = self.current_bytes.saturating_add(snapshot.len());
        self.snapshots.push_back(snapshot);
        self.evict_until_under_cap();
    }

    /// Pop the newest snapshot — the one closest to "now". Returns None if
    /// the ring is empty (or has been fully rewound).
    pub fn pop_back(&mut self) -> Option<Vec<u8>> {
        let snap = self.snapshots.pop_back()?;
        self.current_bytes = self.current_bytes.saturating_sub(snap.len());
        Some(snap)
    }

    /// Peek at the newest snapshot without consuming it. Used by scrubbing
    /// UI when it wants to preview the current rewind head.
    pub fn peek_back(&self) -> Option<&[u8]> {
        self.snapshots.back().map(|v| v.as_slice())
    }

    /// Peek at a snapshot by distance from the newest. `steps_back = 0` is
    /// the newest (same as [`peek_back`]); `steps_back = len - 1` is the
    /// oldest still held. Returns None when the index is out of bounds.
    /// Used by the scrubbing UI for preview without mutating the ring.
    pub fn peek_at(&self, steps_back: usize) -> Option<&[u8]> {
        let len = self.snapshots.len();
        if steps_back >= len {
            return None;
        }
        // VecDeque is newest-at-back: index `len - 1 - steps_back` from
        // the front matches `steps_back` from the back.
        self.snapshots.get(len - 1 - steps_back).map(|v| v.as_slice())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_pop_back_lifo_order() {
        let mut ring = RewindRing::new(1024);
        ring.push(vec![1, 2, 3]);
        ring.push(vec![4, 5, 6]);
        ring.push(vec![7, 8, 9]);
        assert_eq!(ring.len(), 3);
        assert_eq!(ring.byte_size(), 9);
        assert_eq!(ring.pop_back().unwrap(), vec![7, 8, 9]);
        assert_eq!(ring.pop_back().unwrap(), vec![4, 5, 6]);
        assert_eq!(ring.pop_back().unwrap(), vec![1, 2, 3]);
        assert!(ring.pop_back().is_none());
        assert_eq!(ring.byte_size(), 0);
    }

    #[test]
    fn evicts_oldest_when_over_byte_cap() {
        let mut ring = RewindRing::new(20);
        ring.push(vec![0u8; 10]);
        ring.push(vec![0u8; 10]);
        ring.push(vec![0u8; 10]); // forces eviction of the first
        assert_eq!(ring.len(), 2);
        assert_eq!(ring.byte_size(), 20);
        // Pushing one more should evict another — len stays at 2.
        ring.push(vec![0u8; 10]);
        assert_eq!(ring.len(), 2);
        assert_eq!(ring.byte_size(), 20);
    }

    #[test]
    fn always_retains_at_least_one_snapshot() {
        // Even if a single snapshot busts the cap, we keep it rather than
        // emptying the ring. Better to briefly exceed than to lose all
        // history.
        let mut ring = RewindRing::new(5);
        ring.push(vec![0u8; 100]);
        assert_eq!(ring.len(), 1);
        assert_eq!(ring.byte_size(), 100);
    }

    #[test]
    fn set_max_bytes_evicts_immediately() {
        let mut ring = RewindRing::new(1024);
        for _ in 0..10 {
            ring.push(vec![0u8; 50]);
        }
        assert_eq!(ring.len(), 10);
        ring.set_max_bytes(200);
        // 200 / 50 = 4 entries; eviction is greedy down to 4.
        assert!(ring.byte_size() <= 200);
        assert!(ring.len() <= 4);
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
        assert_eq!(ring.peek_at(0), Some(&[3u8, 3, 3][..]));
        assert_eq!(ring.peek_at(1), Some(&[2u8, 2, 2][..]));
        assert_eq!(ring.peek_at(2), Some(&[1u8, 1, 1][..]));
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
        assert_eq!(ring.byte_size(), 30);
        // The remaining newest is `2`.
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
