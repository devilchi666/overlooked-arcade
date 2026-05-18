//! Phase 4 slice D — frame-by-frame video capture.
//!
//! Spawns a worker thread that pulls RGBA framebuffers off a bounded
//! channel and writes them to disk as PNG files (one per frame). A
//! sibling `manifest.json` records system / fps / dimensions so a later
//! pass can mux into WebM via ffmpeg (slice D-2) or any other tool the
//! operator prefers.
//!
//! Design choices:
//!
//! - **Bounded channel + try_send.** When the encoder can't keep up
//!   (PNG compression takes a few ms per frame; high-res systems will
//!   eventually saturate a single thread), we drop the new frame
//!   rather than block the emu thread. Dropped count surfaces in the
//!   manifest so the operator knows the recording has gaps.
//!
//! - **Per-recording thread.** Cheap to spawn; doesn't deal with the
//!   "long-lived worker handling start/stop messages" state machine.
//!   When the operator stops the capture we close the sender; the
//!   worker drains remaining frames, writes the manifest, and exits.
//!
//! - **PNG over raw RGBA.** PNG is well-supported, browsable in any
//!   image viewer, and compresses ~3-5× compared to raw. Worse than
//!   real video compression but the user can convert offline.
//!
//! - **One directory per recording.** Avoids name collisions and lets
//!   the operator delete entire clips with one rmdir.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

/// One captured frame on its way to the encoder thread.
pub struct VideoFrame {
    pub frame_idx: u64,
    pub width: u32,
    pub height: u32,
    /// Tightly-packed RGBA8 (4 bytes per pixel, no row padding).
    /// `width × height × 4` bytes long.
    pub rgba: Vec<u8>,
}

/// Manifest written alongside the PNG sequence. Captures everything a
/// post-processor needs to assemble the frames into a real video file.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoManifest {
    pub system_id: String,
    pub rom_stem: String,
    pub display_name: String,
    pub fps: f64,
    pub frame_count: u64,
    pub dropped_frame_count: u64,
    /// Native framebuffer size at capture start. Cores can change
    /// resolution mid-game (PCE 256/352/512-wide modes), but we
    /// stamp the first-frame dimensions here and let each PNG carry
    /// its own real size for resolution-change handling.
    pub width: u32,
    pub height: u32,
    pub started_at_unix_ms: i64,
    pub stopped_at_unix_ms: i64,
    /// printf-style pattern for the PNG filenames in this directory.
    /// Always `frame_%06d.png` in v1; carried explicitly so a future
    /// version can shift to a different pattern without breaking old
    /// recordings.
    pub frame_pattern: String,
}

/// Channel capacity = ~0.5 s of buffering at 60 fps. Beyond that the
/// encoder is too far behind to ever catch up, and the kindest thing
/// to do is drop incoming frames so the emu loop doesn't stall on
/// channel send.
const CHANNEL_CAPACITY: usize = 30;

/// Worker handle returned by [`start`]. Holds the channel sender +
/// the join handle; drop the worker to stop the recording cleanly.
pub struct VideoCaptureWorker {
    sender: Option<mpsc::SyncSender<VideoFrame>>,
    join: Option<JoinHandle<WorkerStats>>,
    pub clip_dir: PathBuf,
    pub started_at_unix_ms: i64,
    /// Frames the emu thread tried to push but had to drop because the
    /// channel was full. Capped at u64; counted by the emu thread.
    pub dropped_frame_count: u64,
}

/// Returned by the worker thread on join. The emu thread folds these
/// counts into the manifest at finalize time.
#[derive(Clone, Copy, Debug, Default)]
pub struct WorkerStats {
    pub frames_written: u64,
    pub write_errors: u64,
}

/// Start a new capture. Spawns the encoder thread and returns a handle
/// the caller pushes frames into. The clip directory is created if it
/// doesn't exist; failure to create it is an error returned to the
/// caller (no thread is spawned in that case).
pub fn start(
    clip_dir: PathBuf,
) -> std::io::Result<VideoCaptureWorker> {
    std::fs::create_dir_all(&clip_dir)?;
    let (tx, rx) = mpsc::sync_channel::<VideoFrame>(CHANNEL_CAPACITY);
    let dir_for_thread = clip_dir.clone();
    let join = std::thread::Builder::new()
        .name("oa-video-encoder".into())
        .spawn(move || encoder_loop(rx, dir_for_thread))?;
    let started_at_unix_ms = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    Ok(VideoCaptureWorker {
        sender: Some(tx),
        join: Some(join),
        clip_dir,
        started_at_unix_ms,
        dropped_frame_count: 0,
    })
}

impl VideoCaptureWorker {
    /// Non-blocking frame submission. Returns true if the frame was
    /// accepted; false (and bumps `dropped_frame_count`) if the channel
    /// was full. The caller should NOT retry; just keep the emu loop
    /// moving and accept the gap.
    pub fn try_submit(&mut self, frame: VideoFrame) -> bool {
        let Some(tx) = self.sender.as_ref() else {
            return false;
        };
        match tx.try_send(frame) {
            Ok(()) => true,
            Err(mpsc::TrySendError::Full(_)) => {
                self.dropped_frame_count = self.dropped_frame_count.saturating_add(1);
                false
            }
            Err(mpsc::TrySendError::Disconnected(_)) => {
                // Worker died (write errors, panic, …). Forget the sender so
                // future submits short-circuit immediately.
                self.sender = None;
                false
            }
        }
    }

    /// Stop the recording. Closes the channel (so the worker drains any
    /// remaining frames and exits), joins the worker thread, writes the
    /// manifest if `discard = false`, and returns the manifest path on
    /// success. Discarding skips the manifest write AND deletes the
    /// whole clip directory.
    pub fn stop_and_finalize(
        mut self,
        system_id: &str,
        rom_stem: &str,
        display_name: &str,
        fps: f64,
        first_width: u32,
        first_height: u32,
        discard: bool,
    ) -> std::io::Result<FinalizeResult> {
        // Drop the sender so the worker's recv() returns Err and the
        // loop exits cleanly.
        self.sender = None;
        let stats = match self.join.take() {
            Some(h) => h.join().unwrap_or_else(|_| {
                log::warn!("oa-video: encoder thread panicked");
                WorkerStats::default()
            }),
            None => WorkerStats::default(),
        };

        if discard {
            // Best-effort dir delete. Logging on error rather than
            // bubbling up — discard is a hint, not a guarantee.
            if let Err(e) = std::fs::remove_dir_all(&self.clip_dir) {
                log::warn!("oa-video: discard remove_dir_all({}) failed: {e:?}", self.clip_dir.display());
            }
            return Ok(FinalizeResult {
                manifest_path: PathBuf::new(),
                clip_dir: self.clip_dir,
                stats,
                dropped: self.dropped_frame_count,
                discarded: true,
            });
        }

        let stopped_at_unix_ms = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let manifest = VideoManifest {
            system_id: system_id.to_string(),
            rom_stem: rom_stem.to_string(),
            display_name: display_name.to_string(),
            fps,
            frame_count: stats.frames_written,
            dropped_frame_count: self.dropped_frame_count,
            width: first_width,
            height: first_height,
            started_at_unix_ms: self.started_at_unix_ms,
            stopped_at_unix_ms,
            frame_pattern: "frame_%06d.png".to_string(),
        };
        let manifest_path = self.clip_dir.join("manifest.json");
        let body = serde_json::to_string_pretty(&manifest).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        })?;
        std::fs::write(&manifest_path, body)?;
        Ok(FinalizeResult {
            manifest_path,
            clip_dir: self.clip_dir,
            stats,
            dropped: self.dropped_frame_count,
            discarded: false,
        })
    }
}

/// Outcome reported back to the emu thread at finalize. Lets the UI
/// surface "captured N frames, dropped M" and the manifest location.
#[derive(Debug)]
pub struct FinalizeResult {
    pub manifest_path: PathBuf,
    pub clip_dir: PathBuf,
    pub stats: WorkerStats,
    pub dropped: u64,
    pub discarded: bool,
}

fn encoder_loop(rx: mpsc::Receiver<VideoFrame>, dir: PathBuf) -> WorkerStats {
    let mut stats = WorkerStats::default();
    while let Ok(frame) = rx.recv() {
        let filename = format!("frame_{:06}.png", frame.frame_idx);
        let path = dir.join(&filename);
        match write_png_rgba8(&path, frame.width, frame.height, &frame.rgba) {
            Ok(()) => stats.frames_written = stats.frames_written.saturating_add(1),
            Err(e) => {
                stats.write_errors = stats.write_errors.saturating_add(1);
                log::warn!("oa-video: write {} failed: {e:?}", path.display());
            }
        }
    }
    log::info!(
        "oa-video: encoder thread exiting ({} frames written, {} write errors)",
        stats.frames_written, stats.write_errors
    );
    stats
}

fn write_png_rgba8(path: &Path, width: u32, height: u32, rgba: &[u8]) -> std::io::Result<()> {
    // png crate is already a workspace dep (used by save_state thumbnails).
    // We use it directly here instead of the heavier `image` crate so we
    // skip the format-detection overhead per frame.
    let file = std::fs::File::create(path)?;
    let bw = std::io::BufWriter::new(file);
    let mut enc = png::Encoder::new(bw, width, height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    // Default compression — png crate's Fast is ~30% faster but ~20%
    // larger. Default is a good middle ground for 60-fps capture.
    let mut writer = enc
        .write_header()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    writer
        .write_image_data(rgba)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "oa-video-{}-{}-{}",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ))
    }

    fn dummy_frame(idx: u64, w: u32, h: u32) -> VideoFrame {
        VideoFrame {
            frame_idx: idx,
            width: w,
            height: h,
            rgba: vec![0u8; (w * h * 4) as usize],
        }
    }

    #[test]
    fn round_trip_writes_png_files_and_manifest() {
        let dir = tmp_dir("roundtrip");
        let mut w = start(dir.clone()).expect("start");
        for i in 0..3 {
            assert!(w.try_submit(dummy_frame(i, 4, 4)));
        }
        let r = w
            .stop_and_finalize("tg16", "Bonk", "test", 60.0, 4, 4, false)
            .expect("finalize");
        assert!(!r.discarded);
        assert_eq!(r.stats.frames_written, 3);
        assert!(r.manifest_path.is_file());
        for i in 0..3 {
            assert!(dir.join(format!("frame_{:06}.png", i)).is_file());
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn discard_removes_directory() {
        let dir = tmp_dir("discard");
        let mut w = start(dir.clone()).expect("start");
        assert!(w.try_submit(dummy_frame(0, 4, 4)));
        let r = w
            .stop_and_finalize("tg16", "Bonk", "trash", 60.0, 4, 4, true)
            .expect("finalize");
        assert!(r.discarded);
        assert!(!dir.exists(), "discard should remove the clip dir");
    }

    #[test]
    fn channel_overflow_drops_frames() {
        // Saturate the channel before the encoder thread can drain
        // by sending more frames than CHANNEL_CAPACITY in one tight loop.
        // Some submits succeed (drained between sends), some fail.
        let dir = tmp_dir("overflow");
        let mut w = start(dir.clone()).expect("start");
        let big_frame = VideoFrame {
            frame_idx: 0,
            width: 512,
            height: 512,
            rgba: vec![0u8; 512 * 512 * 4],
        };
        // Burst CHANNEL_CAPACITY + 50 frames; at least some should fail
        // because the encoder can't possibly drain that fast even for
        // tiny 512x512 frames.
        let mut accepted = 0u64;
        for i in 0..(CHANNEL_CAPACITY as u64 + 50) {
            let mut f = big_frame.clone();
            f.frame_idx = i;
            if w.try_submit(f) {
                accepted += 1;
            }
        }
        // At least some accepted (the channel buffer + whatever the
        // encoder drained).
        assert!(accepted >= CHANNEL_CAPACITY as u64);
        // dropped_frame_count counts the rejections.
        assert!(w.dropped_frame_count > 0, "expected some drops; got 0 (channel too eager? bump load)");
        let _ = w
            .stop_and_finalize("tg16", "Bonk", "overflow", 60.0, 512, 512, true)
            .expect("finalize");
    }
}

// Cargo-test workaround: VideoFrame needs Clone for the overflow test
// above. Manual impl rather than derive so the production code stays
// move-only (we never clone in the hot path).
#[cfg(test)]
impl Clone for VideoFrame {
    fn clone(&self) -> Self {
        Self {
            frame_idx: self.frame_idx,
            width: self.width,
            height: self.height,
            rgba: self.rgba.clone(),
        }
    }
}
