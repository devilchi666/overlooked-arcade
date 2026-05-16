//! oa-core — system-agnostic interface every emulator core implements.
//!
//! The Tauri shell, renderer, audio sink, input layer, and save-state machinery all
//! work against this trait. Adding a new system means: vendor an upstream core into
//! a new `oa-<sys>-sys` crate, write a thin `oa-<sys>` wrapper that implements
//! [`Core`], and register it in the shell. The shell never imports a concrete core.
//!
//! This module deliberately stays small: types here propagate to every crate in the
//! workspace, so churn is expensive. New variants and methods are additive only.

#![deny(rust_2018_idioms)]
#![warn(missing_docs)]

use std::io::{Read, Write};

/// Which physical system a [`Core`] implementation emulates.
///
/// Variants are added as systems are brought online — Phase 1 is PCE-only, the rest
/// are placeholders for the planned lineup. Marked `#[non_exhaustive]` so adding a
/// new system isn't a breaking change to downstream match arms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SystemId {
    /// TurboGrafx-16 / PC Engine HuCard.
    PcEngine,
    /// PC Engine CD-ROM² / TurboGrafx-CD.
    PceCdRom2,
    /// Atari Lynx.
    Lynx,
    /// Atari 7800.
    Atari7800,
    /// Sega Master System.
    Sms,
    /// Sega Game Gear.
    GameGear,
    /// MSX (and MSX2).
    Msx,
    /// ColecoVision.
    Colecovision,
    /// GCE Vectrex.
    Vectrex,
    /// Nintendo Virtual Boy.
    VirtualBoy,
    /// Bandai WonderSwan (and WonderSwan Color).
    WonderSwan,
}

/// Native output dimensions and timing.
///
/// `width × height` is the system's native framebuffer resolution; the renderer
/// scales (and optionally post-processes via shaders). `fps` and `sample_rate` are
/// the cadence the shell pumps the core and the audio sink at, respectively.
#[derive(Debug, Clone, Copy)]
pub struct Timing {
    /// Framebuffer width in pixels.
    pub width: u32,
    /// Framebuffer height in pixels.
    pub height: u32,
    /// Frames per second.
    pub fps: f64,
    /// Audio sample rate in Hz (stereo).
    pub sample_rate: u32,
}

/// A borrow of the core's current framebuffer.
///
/// Pixels are RGBA8 (4 bytes per pixel), `width × height × 4` long. Lifetime is
/// tied to the core — calling [`Core::run_frame`] invalidates the borrow. The
/// renderer is expected to upload the bytes into a wgpu texture immediately.
#[derive(Debug)]
pub struct Framebuffer<'a> {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Pixel bytes (RGBA8 packed).
    pub pixels: &'a [u8],
    /// Display aspect ratio (final image W:H — *not* pixel W:H). 0.0 tells the
    /// renderer to fall back to `width as f32 / height as f32`. Cores with
    /// non-square pixels (PCE: 256/352/512-wide modes, Lynx, etc.) report this
    /// per-frame so the shell can letterbox/pillarbox correctly.
    pub display_aspect: f32,
}

/// Which controller port input applies to.
///
/// PCE supports up to 5 controllers via a multitap. Most cores ignore ports past 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortIndex {
    /// Player 1.
    Port0,
    /// Player 2.
    Port1,
    /// Player 3 (multitap).
    Port2,
    /// Player 4 (multitap).
    Port3,
    /// Player 5 (multitap).
    Port4,
}

/// Per-frame controller state for a single port.
///
/// `buttons` is a bitfield whose semantics are system-specific (the wrapper crate
/// for each core defines its layout). `axes` is reserved for analog inputs
/// (Vectrex stick, future systems) — set to zero for purely-digital cores.
#[derive(Debug, Clone, Copy, Default)]
pub struct InputState {
    /// System-specific button bitfield.
    pub buttons: u32,
    /// Analog axes in signed 16-bit range, 0 for unused.
    pub axes: [i16; 4],
}

/// Errors a core may surface across the FFI boundary or during state I/O.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    /// The provided ROM payload could not be parsed or is unsupported.
    #[error("invalid ROM: {0}")]
    InvalidRom(String),

    /// A save-state blob is malformed or truncated.
    #[error("malformed save state")]
    SaveStateMalformed,

    /// A save-state blob was produced by a different core/version.
    #[error("save-state version mismatch (got {got}, expected {expected})")]
    SaveStateVersionMismatch {
        /// Version embedded in the blob.
        got: u32,
        /// Version this core understands.
        expected: u32,
    },

    /// Underlying I/O error during state serialization.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// The core's underlying C library returned an error (FFI layer translates it).
    #[error("core internal error: {0}")]
    Internal(String),
}

/// The interface every emulator core implements.
///
/// Cores are owned by the shell on a dedicated emulation thread. The shell calls
/// [`Core::run_frame`] at the system's native rate, then reads framebuffer + audio
/// + (eventually) state. Inputs are pushed in before each frame.
pub trait Core: Send {
    /// Which system this core emulates. Stable for the lifetime of the instance.
    fn system(&self) -> SystemId;

    /// Native timing — resolution, framerate, audio sample rate.
    fn timing(&self) -> Timing;

    /// Hard reset (power cycle). Preserves loaded ROM.
    fn reset(&mut self);

    /// Advance emulation by exactly one frame.
    ///
    /// On return, [`Core::framebuffer`] and [`Core::drain_audio`] reflect the new
    /// frame. Cores should not block on I/O here — this runs at ~16 ms cadence.
    fn run_frame(&mut self);

    /// Borrow the current framebuffer. Valid until the next [`Core::run_frame`].
    fn framebuffer(&self) -> Framebuffer<'_>;

    /// Borrow accumulated stereo audio samples since the last call.
    ///
    /// Samples are interleaved L/R `i16`. The slice is valid until the next call
    /// to either [`Core::run_frame`] or [`Core::drain_audio`].
    fn drain_audio(&mut self) -> &[i16];

    /// Push controller state for a port. Effective on the next [`Core::run_frame`].
    ///
    /// Cores ignore ports they don't wire up; semantics of `InputState::buttons`
    /// are defined per-system by the wrapper crate.
    fn set_input(&mut self, port: PortIndex, input: InputState);

    /// Serialize emulator state into `writer`.
    ///
    /// Format is opaque and per-core. Compression and framing are handled by the
    /// `oa-savestate` crate, not here.
    fn save_state(&self, writer: &mut dyn Write) -> Result<(), CoreError>;

    /// Restore emulator state from `reader`.
    fn load_state(&mut self, reader: &mut dyn Read) -> Result<(), CoreError>;
}
