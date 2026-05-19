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

use serde::{Deserialize, Serialize};

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
    /// Nintendo Entertainment System / Famicom.
    Nes,
    /// Super Nintendo Entertainment System / Super Famicom.
    Snes,
    /// MAME — arcade. Drives many different arcade boards via one core;
    /// the shell treats it as a single SystemId, with the ROM-set naming
    /// inside the .zip distinguishing individual games.
    Mame,
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

/// One of the four memory regions libretro cores expose via
/// `retro_get_memory_data` / `retro_get_memory_size`. Non-libretro
/// cores can map these to their nearest equivalents.
///
/// Region IDs match the libretro constants so the libretro impl is a
/// straight passthrough.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MemoryRegionId {
    /// Battery-backed save RAM (cart SRAM, SNES SRAM, etc.). Persists
    /// across power cycles.
    SaveRam = 0,
    /// Real-time clock (Pokémon Crystal etc.).
    Rtc = 1,
    /// Main system RAM. The interesting region for memory inspectors +
    /// achievement-style milestone tracking — variables and game state
    /// almost always live here.
    SystemRam = 2,
    /// Video RAM. Less commonly inspected; sometimes useful for
    /// "is the player in a specific scene" heuristics.
    VideoRam = 3,
}

impl MemoryRegionId {
    /// String tag used for serde + Tauri command payloads. Stable
    /// across versions so saved milestone configs keep parsing.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SaveRam => "save_ram",
            Self::Rtc => "rtc",
            Self::SystemRam => "system_ram",
            Self::VideoRam => "video_ram",
        }
    }

    /// Inverse of [`as_str`]. Returns `None` for unrecognized tags so
    /// the caller can decide whether to default or reject.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "save_ram" => Some(Self::SaveRam),
            "rtc" => Some(Self::Rtc),
            "system_ram" => Some(Self::SystemRam),
            "video_ram" => Some(Self::VideoRam),
            _ => None,
        }
    }
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

/// A single value choice within a [`CoreOption`].
///
/// `value` is the canonical string the core sees when it queries the
/// option (via the libretro `GET_VARIABLE` callback for that core).
/// `label` is an optional human-readable display string — when None,
/// the UI shows `value` directly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreOptionValue {
    /// The wire value passed to the core.
    pub value: String,
    /// Optional display label. None means "show `value`".
    pub label: Option<String>,
}

/// One configurable option exposed by a core.
///
/// Mirrors libretro's `retro_core_option_v2_definition` shape, but
/// system-agnostic so non-libretro cores can implement this too if
/// they choose. Cores expose their full option set via [`Core::options`]
/// after `retro_set_environment` / `retro_load_game` runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreOption {
    /// Stable identifier the core uses to query its value (e.g.
    /// `"pce_fast_cdimagecache"`). Persistence keys this.
    pub key: String,
    /// Short human description shown as the row label in the UI.
    pub desc: String,
    /// Longer help text shown as a tooltip / info hover. None when the
    /// core didn't supply one.
    pub info: Option<String>,
    /// libretro V2 category grouping. When Some, the UI may group
    /// options sharing the same category under a collapsible section
    /// labeled by the matching [`CoreOptionCategory`]. None means
    /// "uncategorized" (shown in the default top-level group).
    pub category_key: Option<String>,
    /// The core's recommended default value. Must appear in [`values`].
    pub default_value: String,
    /// Allowed value set. Order is preserved for dropdown rendering.
    pub values: Vec<CoreOptionValue>,
}

/// A category grouping for [`CoreOption`]s — libretro V2 only.
///
/// When the core registers categories via `SET_CORE_OPTIONS_V2`, options
/// with a matching [`CoreOption::category_key`] are grouped under this
/// header in the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreOptionCategory {
    /// Identifier referenced by [`CoreOption::category_key`].
    pub key: String,
    /// Short human description shown as the group header.
    pub desc: String,
    /// Optional longer help text.
    pub info: Option<String>,
}

/// Disc control snapshot for multi-disc games.
///
/// Cores supporting multi-disc images (PCE-CD with `.m3u`, PSX, Saturn,
/// etc.) register a disc-control callback during `retro_load_game` that
/// lets the frontend ask "how many discs?" + swap between them. The
/// shell wraps that interface in this struct so the UI doesn't have to
/// know about libretro specifics.
///
/// Single-disc CD games return `num_discs == 1` and `ejected == false`;
/// HuCard / cart games return None from `Core::disc_state` entirely.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscInfo {
    /// Total number of disc images registered by the core (typically
    /// 1 for single-disc games, 2+ for `.m3u` playlists).
    pub num_discs: u32,
    /// Which disc is currently loaded. 0-based.
    pub current_index: u32,
    /// True when the virtual tray is open. The disc-swap protocol is
    /// eject → set_image_index → close, so this is briefly true during
    /// a user-initiated swap.
    pub ejected: bool,
    /// Optional human-readable labels for each disc (v2 cores only).
    /// Empty Vec for v1 cores; the UI falls back to "Disc 1", "Disc 2", etc.
    /// Length equals `num_discs` when present.
    pub labels: Vec<String>,
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

    /// The set of configurable options this core exposes.
    ///
    /// libretro cores register option definitions via the V2 / V1 / legacy
    /// `SET_VARIABLES` environment callbacks during `retro_set_environment`
    /// or `retro_load_game`. The shell collects them here so the per-system
    /// + per-game settings pages can render dropdowns. Returns an empty Vec
    /// for cores that don't declare options (or before declaration runs).
    fn options(&self) -> Vec<CoreOption> {
        Vec::new()
    }

    /// Category groupings for [`Core::options`] — libretro V2 only.
    /// Returns an empty Vec for cores using V1 or the legacy variable
    /// callback; the UI shows uncategorized options in a single list.
    fn option_categories(&self) -> Vec<CoreOptionCategory> {
        Vec::new()
    }

    /// Override the value of a single option. Takes effect on the
    /// core's next `GET_VARIABLE` poll (most cores re-read at the top
    /// of each frame; some only at load_game time). The shell pushes
    /// each user-changed value here individually + sets a "variables
    /// updated" flag so the core knows to refresh.
    ///
    /// `value` must be one of the values from the option's
    /// [`CoreOption::values`] list. The core itself enforces this if
    /// it cares; the shell does not validate.
    fn set_option(&mut self, _key: &str, _value: &str) {}

    /// Disc control snapshot for multi-disc games. Returns None for
    /// cores without disc support (HuCard, cartridge systems) AND for
    /// CD-capable cores when no disc-control callback has been
    /// registered (e.g. a single-image launch that didn't trigger
    /// the multi-disc machinery).
    fn disc_state(&self) -> Option<DiscInfo> {
        None
    }

    /// Open or close the virtual disc tray. The disc-swap protocol is:
    /// `set_disc_eject(true)` → `set_disc_image(N)` → `set_disc_eject(false)`.
    /// Cores resume reading from the new disc on the next frame.
    fn set_disc_eject(&mut self, _ejected: bool) {}

    /// Load disc `index` (0-based). Only valid while the tray is
    /// ejected; the core typically refuses + logs otherwise.
    fn set_disc_image(&mut self, _index: u32) {}

    /// Borrow a memory region exposed by the core. Returns None if the
    /// region is empty (size = 0) or unsupported by this core.
    ///
    /// Like [`Core::framebuffer`], the slice aliases through the core's
    /// internal state — calling [`Core::run_frame`] (or any other
    /// `&mut self` method) invalidates the borrow. The shell is
    /// expected to copy out the bytes it needs and drop the borrow
    /// before resuming emulation.
    ///
    /// Used by Phase 4 slice E (memory inspector) + slice F (per-game
    /// milestones) to read game state for inspection and predicate
    /// evaluation. No-op default impl returns None so cores without a
    /// memory surface (test stubs etc.) don't have to implement it.
    fn memory_region(&self, _id: MemoryRegionId) -> Option<&[u8]> {
        None
    }

    /// Mutable borrow of a memory region — same lifetime tie as
    /// [`Core::memory_region`] but `&mut self`-bound so callers can
    /// write into the region's bytes.
    ///
    /// Used by the cheat system (RetroArch parity) — every frame, the
    /// emu thread enumerates enabled cheats and writes their `value`
    /// to the configured `(region, offset, width)` triple. Cores that
    /// don't expose mutable memory return None and cheats no-op for
    /// that system.
    fn memory_region_mut(&mut self, _id: MemoryRegionId) -> Option<&mut [u8]> {
        None
    }

    /// Clear every libretro-format cheat the core knows about. Called by
    /// the shell on every LoadRom and before reseating the active set so
    /// disabled / deleted cheats stop firing. Pairs with [`Core::cheat_set`].
    /// No-op default for cores without a cheat machinery.
    fn cheat_reset(&mut self) {}

    /// Register a libretro-format cheat code with the core (Game Genie /
    /// GameShark / Action Replay / Pro Action Replay / raw — the core
    /// decodes per its own conventions). `index` is the slot identifier
    /// (cores typically apply by index); `enabled` toggles without
    /// removing; `code` is the raw user-entered string. Default no-op so
    /// cores without a cheat machinery silently ignore.
    fn cheat_set(&mut self, _index: u32, _enabled: bool, _code: &str) {}
}
