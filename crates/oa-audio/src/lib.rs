//! oa-audio — cpal output sink with an SPSC ring buffer between the emulation
//! thread and the audio callback.
//!
//! Phase 1 scope: take stereo `i16` samples at the core's native rate (44.1 kHz
//! for PCE) and push them to the default output device. Three things happen on
//! the producer side per call:
//!   1. **Resample** from `source_rate` to the device's `output_rate` using
//!      linear interpolation with state carried across calls (click-free at
//!      batch boundaries). Linear is cheap and audibly fine for chip music /
//!      sampled SFX; sinc/rubato will arrive when we care.
//!   2. Push the resampled stereo i16 stream into the SPSC ring buffer.
//!   3. cpal's audio callback (on a different thread) pulls from the ring and
//!      converts the i16 stream to whatever format the device wants (f32, i16,
//!      or u16).
//!
//! Concurrency: AudioSink is owned by the emulation thread and `push()` is the
//! only producer-side entry point. The cpal Stream lives inside AudioSink and
//! its callback runs on cpal's own thread; the ring buffer bridges the two.

#![deny(rust_2018_idioms)]

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, SampleRate, StreamConfig, SupportedStreamConfigRange};
use ringbuf::{
    traits::{Consumer, Producer, Split},
    HeapRb,
};

/// Audio errors surfaced during AudioSink construction.
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    /// No default output device is available.
    #[error("no default audio output device")]
    NoDevice,
    /// A named device was requested but no output device with that name exists.
    #[error("audio device not found: {0}")]
    DeviceNotFound(String),
    /// Couldn't enumerate available output devices.
    #[error("output_devices failed: {0}")]
    EnumerateDevices(#[from] cpal::DevicesError),
    /// Couldn't enumerate the device's supported configs.
    #[error("supported_output_configs failed: {0}")]
    EnumerateConfigs(#[from] cpal::SupportedStreamConfigsError),
    /// Couldn't build the output stream with any acceptable config.
    #[error("build_output_stream failed: {0}")]
    BuildStream(#[from] cpal::BuildStreamError),
    /// Couldn't start playback.
    #[error("stream.play failed: {0}")]
    Play(#[from] cpal::PlayStreamError),
    /// No supported config has stereo + a sample format we know.
    #[error("no compatible audio config (need stereo, i16/f32/u16)")]
    NoCompatibleConfig,
}

/// Identity record for an enumerated output device. `name` is the persistent
/// identifier — stable enough on Windows + macOS for use as a settings key.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub is_default: bool,
}

/// Enumerate currently-attached output devices. Returns an empty list on
/// enumeration failure (logged at warn level). The default device, if any, is
/// flagged via `is_default`.
pub fn list_devices() -> Vec<DeviceInfo> {
    let host = cpal::default_host();
    let default_name = host
        .default_output_device()
        .and_then(|d| d.name().ok());
    let iter = match host.output_devices() {
        Ok(it) => it,
        Err(e) => {
            log::warn!("oa-audio: output_devices() failed: {e:?}");
            return Vec::new();
        }
    };
    let mut out = Vec::new();
    for d in iter {
        let Ok(name) = d.name() else { continue };
        let is_default = default_name.as_deref() == Some(name.as_str());
        out.push(DeviceInfo { name, is_default });
    }
    out
}

type RbProd = ringbuf::CachingProd<std::sync::Arc<HeapRb<i16>>>;

/// Default ring-buffer capacity in `i16` samples. ~170 ms at 48 kHz
/// stereo (16384 / 96 samples/ms); ~85 ms at 96 kHz. Most cores'
/// `RETRO_ENVIRONMENT_SET_MINIMUM_AUDIO_LATENCY` requests (Genesis
/// Plus GX / Beetle PSX / Flycast cluster around 64-100 ms) fit
/// comfortably under this; `AudioSink::ensure_min_latency_ms`
/// handles the outliers by rebuilding with a larger ring.
const DEFAULT_RING_CAPACITY: usize = 16384;

/// Compute the ring-buffer capacity in `i16` samples needed to hold
/// `ms` milliseconds of stereo audio at `output_rate`. Each ring slot
/// is one i16 (one channel) so the formula multiplies by the channel
/// count and rounds up:
///
///   capacity = ⌈ms × output_rate × CHANNELS / 1000⌉
///
/// e.g. 64 ms @ 48 kHz stereo = 6144 samples; 512 ms @ 48 kHz stereo
/// = 49152 samples (the spec ceiling, well under any reasonable cap).
fn capacity_for_ms(ms: u32, output_rate: u32) -> usize {
    const CHANNELS: u32 = 2;
    let needed = (u64::from(ms) * u64::from(output_rate) * u64::from(CHANNELS) + 999) / 1000;
    needed as usize
}

/// Stereo linear resampler with carry-over state for click-free batches.
struct Resampler {
    source_rate: u32,
    target_rate: u32,
    /// Fractional source-frame position for the *next* output frame. After
    /// each batch we subtract the batch length so the next call's first input
    /// frame is at index 0; a negative pos means "interpolate between the
    /// previous batch's last frame (`last`) and this batch's first frame".
    pos: f64,
    last: (i16, i16),
}

impl Resampler {
    fn new(source_rate: u32, target_rate: u32) -> Self {
        Self { source_rate, target_rate, pos: 0.0, last: (0, 0) }
    }

    fn process_into(&mut self, input: &[i16], out: &mut Vec<i16>) {
        if input.is_empty() || self.source_rate == 0 || self.target_rate == 0 {
            return;
        }
        let in_frames = (input.len() / 2) as i64;
        let step = self.source_rate as f64 / self.target_rate as f64;

        loop {
            let i = self.pos.floor() as i64;
            if i >= in_frames {
                break; // entirely past end
            }
            let frac = self.pos - (i as f64);
            if i == in_frames - 1 && frac > 0.0 {
                // Would need next batch's first frame to interpolate — defer.
                break;
            }

            let (l1, r1) = if i < 0 {
                self.last
            } else {
                let ii = i as usize;
                (input[ii * 2], input[ii * 2 + 1])
            };
            let (l2, r2) = if i + 1 < 0 {
                self.last
            } else {
                let ii = (i + 1) as usize;
                (input[ii * 2], input[ii * 2 + 1])
            };

            let outl = (l1 as f64 * (1.0 - frac) + l2 as f64 * frac).round() as i16;
            let outr = (r1 as f64 * (1.0 - frac) + r2 as f64 * frac).round() as i16;
            out.push(outl);
            out.push(outr);

            self.pos += step;
        }

        // Cache last frame of this batch and rewind pos to be relative to the next.
        self.last = (input[input.len() - 2], input[input.len() - 1]);
        self.pos -= in_frames as f64;
    }
}

/// cpal-backed audio output. Owns the stream + ring buffer producer.
pub struct AudioSink {
    producer: RbProd,
    source_rate: u32,
    output_rate: u32,
    resampler: Resampler,
    resample_buf: Vec<i16>,
    pushed_total: u64,
    dropped_total: u64,
    /// Display name of the active device. `None` indicates the system default
    /// was selected at the time the stream was opened.
    current_device: Option<String>,
    /// Ring buffer capacity in `i16` samples. Used by
    /// `ensure_min_latency_ms` to decide whether a core's latency
    /// request demands a bigger ring than the current allocation.
    capacity: usize,
    // Keep the stream alive — its callback owns the consumer half.
    _stream: cpal::Stream,
}

struct OpenedStream {
    producer: RbProd,
    output_rate: u32,
    stream: cpal::Stream,
    device_label: String,
    /// Actual capacity allocated for the ring buffer. May be larger
    /// than the requested `min_capacity_samples` since `open_stream`
    /// uses `max(DEFAULT_RING_CAPACITY, min_capacity_samples)`.
    capacity: usize,
}

/// Resolve + open a stream against either the default device (None) or a
/// specifically-named one. Centralizes the device-pick / supported-config / build
/// path so `new()` / `set_device()` / `ensure_min_latency_ms` share it.
///
/// `min_capacity_samples` is the minimum ring-buffer size required by
/// the caller — pass 0 to take the default. `open_stream` picks
/// `max(DEFAULT_RING_CAPACITY, min_capacity_samples)` so callers never
/// silently SHRINK the ring across a re-open.
fn open_stream(
    source_rate: u32,
    device_name: Option<&str>,
    min_capacity_samples: usize,
) -> Result<OpenedStream, AudioError> {
    let host = cpal::default_host();
    let device = match device_name {
        Some(name) => host
            .output_devices()?
            .find(|d| d.name().ok().as_deref() == Some(name))
            .ok_or_else(|| AudioError::DeviceNotFound(name.to_string()))?,
        None => host.default_output_device().ok_or(AudioError::NoDevice)?,
    };
    let device_label = device.name().unwrap_or_else(|_| "<?>".into());
    log::info!("oa-audio: opening output = {}", device_label);

    let supported: Vec<SupportedStreamConfigRange> =
        device.supported_output_configs()?.collect();
    let chosen =
        pick_stereo_config(&supported, source_rate).ok_or(AudioError::NoCompatibleConfig)?;
    let output_rate = chosen.sample_rate().0;
    let format = chosen.sample_format();
    let stream_config: StreamConfig = chosen.config();

    if output_rate == source_rate {
        log::info!(
            "oa-audio: stream {} Hz, {:?}, {} channels — native rate, no resampling",
            output_rate, format, stream_config.channels
        );
    } else {
        log::info!(
            "oa-audio: stream {} Hz, {:?}, {} channels — resampling from {} Hz (linear interp)",
            output_rate, format, stream_config.channels, source_rate
        );
    }

    // Ring buffer sized to at least `min_capacity_samples` (caller
    // demand) and never below `DEFAULT_RING_CAPACITY` (the original
    // 16384-sample baseline that covers ~170 ms of 48 kHz stereo).
    // Callers that don't have a specific minimum pass 0.
    let capacity = min_capacity_samples.max(DEFAULT_RING_CAPACITY);
    let rb: HeapRb<i16> = HeapRb::new(capacity);
    let (producer, consumer) = rb.split();

    let stream = build_stream(&device, &stream_config, format, consumer)?;
    stream.play()?;

    Ok(OpenedStream { producer, output_rate, stream, device_label, capacity })
}

impl AudioSink {
    /// Build a sink consuming `source_rate` samples; resamples to whatever
    /// the device opens at (typically 48 kHz on Windows).
    pub fn new(source_rate: u32) -> Result<Self, AudioError> {
        Self::with_device(source_rate, None)
    }

    /// Build a sink against a specifically-named output device, or the system
    /// default if `device_name` is `None`. The shell uses this on startup to
    /// honor the persisted device preference.
    pub fn with_device(source_rate: u32, device_name: Option<&str>) -> Result<Self, AudioError> {
        let opened = open_stream(source_rate, device_name, 0)?;
        Ok(Self {
            producer: opened.producer,
            source_rate,
            output_rate: opened.output_rate,
            resampler: Resampler::new(source_rate, opened.output_rate),
            resample_buf: Vec::with_capacity(4096),
            pushed_total: 0,
            dropped_total: 0,
            current_device: device_name.map(|s| s.to_string()).or(Some(opened.device_label)),
            capacity: opened.capacity,
            _stream: opened.stream,
        })
    }

    /// Swap the active output device at runtime. Builds the new stream first;
    /// only on success does the old stream get dropped (so a failed swap leaves
    /// audio playing on the previous device). `None` selects the system default.
    /// Preserves any latency-driven capacity expansion that
    /// `ensure_min_latency_ms` previously applied — the new device opens with
    /// at least the current capacity.
    pub fn set_device(&mut self, device_name: Option<&str>) -> Result<(), AudioError> {
        let opened = open_stream(self.source_rate, device_name, self.capacity)?;
        self.producer = opened.producer;
        self.output_rate = opened.output_rate;
        self.resampler = Resampler::new(self.source_rate, self.output_rate);
        self.resample_buf.clear();
        self.current_device = device_name.map(|s| s.to_string()).or(Some(opened.device_label));
        self.capacity = opened.capacity;
        // Drop the previous stream LAST so the callback can't fire on a
        // half-replaced ring during the swap.
        self._stream = opened.stream;
        Ok(())
    }

    /// Ensure the ring buffer holds at least `ms` milliseconds of stereo
    /// audio at the current `output_rate`. No-op when the current
    /// capacity already covers the request OR when `ms == 0` (the spec's
    /// "use frontend default" sentinel).
    ///
    /// When growth IS needed, rebuilds the stream + ring via the same
    /// path `set_device` uses; the previous stream is dropped LAST so
    /// the cpal callback can't fire on a half-replaced ring during the
    /// swap. A brief audio glitch is possible during the rebuild — the
    /// alternative is crackling on every CPU-heavy frame, which is
    /// what cores set this env to prevent.
    ///
    /// Called by the oa-shell LoadRom handler after every successful
    /// core load with the value from
    /// `oa_libretro::loaded_core_min_audio_latency_ms()` so cores like
    /// Genesis Plus GX / Beetle PSX / Flycast that declare 64-100 ms
    /// requests get the headroom they asked for.
    pub fn ensure_min_latency_ms(&mut self, ms: u32) -> Result<(), AudioError> {
        if ms == 0 {
            return Ok(());
        }
        let needed = capacity_for_ms(ms, self.output_rate);
        if needed <= self.capacity {
            return Ok(());
        }
        log::info!(
            "oa-audio: growing ring buffer for {ms} ms @ {} Hz — {} → {} samples",
            self.output_rate, self.capacity, needed,
        );
        let opened = open_stream(self.source_rate, self.current_device.as_deref(), needed)?;
        self.producer = opened.producer;
        self.output_rate = opened.output_rate;
        self.resampler = Resampler::new(self.source_rate, self.output_rate);
        self.resample_buf.clear();
        self.capacity = opened.capacity;
        self._stream = opened.stream;
        Ok(())
    }

    /// Current ring-buffer capacity in `i16` samples. Diagnostic /
    /// test hook — production code doesn't need this since growth is
    /// declarative via `ensure_min_latency_ms`.
    pub fn capacity_samples(&self) -> usize {
        self.capacity
    }

    /// Push interleaved stereo `i16` samples produced at `source_rate`.
    ///
    /// Internally resamples to `output_rate` and writes into the ring buffer.
    /// Returns the number of resampled samples accepted by the ring; overflow
    /// is dropped (we'd rather glitch audio than block the emu thread).
    pub fn push(&mut self, samples: &[i16]) -> usize {
        self.resample_buf.clear();
        if self.source_rate == self.output_rate {
            self.resample_buf.extend_from_slice(samples);
        } else {
            self.resampler.process_into(samples, &mut self.resample_buf);
        }
        let n = self.producer.push_slice(&self.resample_buf);
        self.pushed_total += n as u64;
        if n < self.resample_buf.len() {
            self.dropped_total += (self.resample_buf.len() - n) as u64;
        }
        n
    }

    /// Effective output sample rate the stream is running at.
    pub fn sample_rate(&self) -> u32 {
        self.output_rate
    }

    /// Source sample rate the sink expects on `push()`.
    pub fn source_rate(&self) -> u32 {
        self.source_rate
    }

    /// Diagnostic counters: (output samples accepted, output samples dropped).
    pub fn stats(&self) -> (u64, u64) {
        (self.pushed_total, self.dropped_total)
    }

    /// Name of the currently-active output device. `None` if the sink was
    /// constructed against the system default and we couldn't resolve a name.
    pub fn current_device(&self) -> Option<&str> {
        self.current_device.as_deref()
    }
}

/// Pick the best 2-channel config available, preferring exact `desired_rate` match.
fn pick_stereo_config(
    supported: &[SupportedStreamConfigRange],
    desired_rate: u32,
) -> Option<cpal::SupportedStreamConfig> {
    let acceptable_format = |f: SampleFormat| matches!(f, SampleFormat::I16 | SampleFormat::F32 | SampleFormat::U16);

    // First pass: 2-channel ranges with an acceptable format that COVERS desired_rate.
    let stereo: Vec<_> = supported
        .iter()
        .filter(|r| r.channels() == 2 && acceptable_format(r.sample_format()))
        .collect();

    if let Some(range) = stereo
        .iter()
        .find(|r| r.min_sample_rate().0 <= desired_rate && desired_rate <= r.max_sample_rate().0)
    {
        return Some(range.with_sample_rate(SampleRate(desired_rate)));
    }

    // Fall back to whatever 2-channel range exists, at its max rate (usually device-preferred).
    if let Some(range) = stereo.first() {
        return Some(range.with_max_sample_rate());
    }

    None
}

fn build_stream(
    device: &cpal::Device,
    config: &StreamConfig,
    format: SampleFormat,
    consumer: ringbuf::CachingCons<std::sync::Arc<HeapRb<i16>>>,
) -> Result<cpal::Stream, cpal::BuildStreamError> {
    let err_fn = |e| log::error!("oa-audio: stream error: {e:?}");

    match format {
        SampleFormat::I16 => {
            let mut consumer = consumer;
            device.build_output_stream(
                config,
                move |out: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    let n = consumer.pop_slice(out);
                    for x in &mut out[n..] {
                        *x = 0;
                    }
                },
                err_fn,
                None,
            )
        }
        SampleFormat::F32 => {
            let mut consumer = consumer;
            // Reusable scratch buffer so the callback doesn't allocate.
            let mut scratch = vec![0i16; 4096];
            device.build_output_stream(
                config,
                move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    if scratch.len() < out.len() {
                        scratch.resize(out.len(), 0);
                    }
                    let n = consumer.pop_slice(&mut scratch[..out.len()]);
                    for (dst, src) in out.iter_mut().zip(scratch.iter().take(n)) {
                        *dst = (*src as f32) / 32768.0;
                    }
                    for x in &mut out[n..] {
                        *x = 0.0;
                    }
                },
                err_fn,
                None,
            )
        }
        SampleFormat::U16 => {
            let mut consumer = consumer;
            device.build_output_stream(
                config,
                move |out: &mut [u16], _: &cpal::OutputCallbackInfo| {
                    let mut scratch = [0i16; 1024];
                    let mut filled = 0usize;
                    while filled < out.len() {
                        let take = (out.len() - filled).min(scratch.len());
                        let n = consumer.pop_slice(&mut scratch[..take]);
                        for (dst, src) in out[filled..filled + n].iter_mut().zip(scratch.iter().take(n)) {
                            // i16 [-32768..32767] -> u16 [0..65535] (offset by 32768)
                            *dst = ((*src as i32) + 32768) as u16;
                        }
                        if n == 0 {
                            for x in &mut out[filled + n..] {
                                *x = 32768; // silence in unsigned PCM
                            }
                            return;
                        }
                        filled += n;
                    }
                },
                err_fn,
                None,
            )
        }
        other => {
            log::error!("oa-audio: unsupported sample format {other:?}");
            Err(cpal::BuildStreamError::StreamConfigNotSupported)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // `capacity_for_ms` is pure math — testable without cpal. The
    // `ensure_min_latency_ms` rebuild path requires a real audio
    // device (cpal stream construction), so it's exercised by the
    // production load path during operator validation (Slice C) and
    // not in unit tests.

    #[test]
    fn capacity_for_ms_at_48khz_stereo() {
        // 48 kHz × 2 channels × ms / 1000 (round up).
        // Concrete: 1 ms = 96 samples; 64 ms = 6144; 100 ms = 9600;
        // 170 ms ≈ 16320; 512 ms = 49152.
        assert_eq!(capacity_for_ms(1, 48000), 96);
        assert_eq!(capacity_for_ms(64, 48000), 6144);
        assert_eq!(capacity_for_ms(100, 48000), 9600);
        assert_eq!(capacity_for_ms(170, 48000), 16320);
        assert_eq!(capacity_for_ms(512, 48000), 49152);
    }

    #[test]
    fn capacity_for_ms_at_96khz_stereo() {
        // 96 kHz doubles the sample rate → doubles the per-ms cost.
        // 64 ms = 12288 samples (vs 6144 at 48 kHz).
        assert_eq!(capacity_for_ms(64, 96000), 12288);
        assert_eq!(capacity_for_ms(100, 96000), 19200);
    }

    #[test]
    fn capacity_for_ms_at_44_1khz_stereo() {
        // PCE / NES native rate. 1 ms = 88.2 samples — round-up
        // arithmetic produces 89, then ms-scaled. 64 ms = 5645.
        assert_eq!(capacity_for_ms(1, 44100), 89);
        assert_eq!(capacity_for_ms(64, 44100), 5645);
    }

    #[test]
    fn capacity_for_ms_zero_is_zero() {
        // Caller (`ensure_min_latency_ms`) shortcircuits ms == 0 before
        // calling the helper, but the helper itself is well-defined.
        assert_eq!(capacity_for_ms(0, 48000), 0);
    }

    #[test]
    fn default_ring_capacity_covers_typical_core_requests_at_48khz() {
        // Sanity check: every core we've audited (Genesis Plus GX,
        // Beetle PSX, Flycast) requests <= ~100 ms. At 48 kHz, that's
        // 9600 samples — well under DEFAULT_RING_CAPACITY (16384).
        // Means ensure_min_latency_ms is a no-op for those cores at
        // 48 kHz, which is correct + cheap.
        assert!(capacity_for_ms(100, 48000) <= DEFAULT_RING_CAPACITY);
        assert!(capacity_for_ms(170, 48000) <= DEFAULT_RING_CAPACITY);
        // At 200 ms the default is no longer enough — would trigger
        // a rebuild. Spec ceiling 512 ms certainly does.
        assert!(capacity_for_ms(200, 48000) > DEFAULT_RING_CAPACITY);
        assert!(capacity_for_ms(512, 48000) > DEFAULT_RING_CAPACITY);
    }
}
