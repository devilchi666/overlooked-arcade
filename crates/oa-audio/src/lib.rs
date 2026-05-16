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

type RbProd = ringbuf::CachingProd<std::sync::Arc<HeapRb<i16>>>;

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
    // Keep the stream alive — its callback owns the consumer half.
    _stream: cpal::Stream,
}

impl AudioSink {
    /// Build a sink consuming `source_rate` samples; resamples to whatever
    /// the device opens at (typically 48 kHz on Windows).
    pub fn new(source_rate: u32) -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or(AudioError::NoDevice)?;
        log::info!(
            "oa-audio: default output = {}",
            device.name().unwrap_or_else(|_| "<?>".into())
        );

        let supported: Vec<SupportedStreamConfigRange> =
            device.supported_output_configs()?.collect();
        let chosen = pick_stereo_config(&supported, source_rate).ok_or(AudioError::NoCompatibleConfig)?;
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

        // Ring buffer sized for ~100 ms of stereo at 48 kHz = 9600 samples. We pick
        // power-of-2 16384 for ringbuf's preferred capacity.
        let rb: HeapRb<i16> = HeapRb::new(16384);
        let (producer, consumer) = rb.split();

        let stream = build_stream(&device, &stream_config, format, consumer)?;
        stream.play()?;

        Ok(Self {
            producer,
            source_rate,
            output_rate,
            resampler: Resampler::new(source_rate, output_rate),
            resample_buf: Vec::with_capacity(4096),
            pushed_total: 0,
            dropped_total: 0,
            _stream: stream,
        })
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
