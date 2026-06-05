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
    /// Sega Mega Drive / Genesis (cart only; Sega CD + 32X land as
    /// separate SystemIds when those slugs onboard).
    Genesis,
    /// Sega Mega-CD / Sega CD. CD-shape addon for the Mega Drive —
    /// shares the 6-button MD controller but loads game data from CD
    /// images (`.cue` / `.chd` / `.iso` / `.m3u`). Requires a regional
    /// BIOS (`bios_CD_E.bin` / `bios_CD_U.bin` / `bios_CD_J.bin`) in
    /// `<exe_dir>/system/`; the shell pre-checks SHA-1 against canonical
    /// Genesis Plus GX-blessed dumps before retro_load_game.
    SegaCd,
    /// Sega 32X. Cart-based expansion adding two SH-2 CPUs to the Mega
    /// Drive (1994 launch); only PicoDrive's libretro side handles it
    /// cleanly. Cart-only path here; 32X-CD goes through SegaCd with a
    /// stacked override.
    Sega32X,
    /// Sega Saturn (1994 JP / 1995 US-EU). Heavyweight CD-shape system
    /// — dual SH-2 CPUs + Saturn VDP1/VDP2 + 68k sound CPU. Beetle
    /// Saturn is the canonical libretro core; alternates Kronos +
    /// YabaSanshiro available for lighter hosts. Requires a regional
    /// BIOS (sega_100.bin / sega_101.bin / mpr-17933.bin / mpr-19367b.bin)
    /// in `<exe_dir>/system/`; pre-checked by `check_saturn_bios`.
    Saturn,
    /// Sony PlayStation (PS1). CD-shape with broad library (3000+
    /// retail titles). Beetle PSX HW is the default (hardware-accelerated
    /// Vulkan/OpenGL upscaling); Beetle PSX SW is the portable
    /// software-renderer fallback. Requires a regional BIOS
    /// (scph5500/5501/5502 v3.0, scph7001/7501 v4.x, etc.) in
    /// `<exe_dir>/system/`; pre-checked by `check_psx_bios`. DualShock
    /// analog sticks deferred to Phase 2 alongside shared analog-input
    /// infra.
    Playstation,
    /// SNK Neo Geo (AES home + MVS arcade). Cart-shape (ROM-set in
    /// .zip / .neo) — FBNeo is the default libretro core. AES home and
    /// MVS arcade share the same hardware and 4-button controller
    /// (A/B/C/D); the difference is operator-mode-specific (coin slot
    /// vs home pad) which FBNeo handles via core option. Requires
    /// `neogeo.zip` in `<exe_dir>/system/`; pre-checked by check_neogeo_bios.
    NeoGeo,
    /// SNK Neo Geo CD. CD-shape variant of the AES home console — same
    /// 4-button controller, CD media instead of carts. `neocd_libretro.dll`
    /// is the default. Requires `neocd_z.rom` (top-loader) or
    /// `neocd_t.rom` (front-loader) in `<exe_dir>/system/`; pre-checked
    /// by check_neocd_bios; slots into the CD-launch BIOS dispatch arm.
    NeoGeoCd,
    /// SNK Neo Geo Pocket / Color (NGP + NGPC). Single SystemId covers
    /// both hardware variants — Beetle NeoPop auto-detects from the
    /// ROM header (same pattern as `Gb` covering DMG + CGB and
    /// `WonderSwan` covering mono + color). No BIOS required. Simple
    /// d-pad + A + B + OPTION controller.
    NeoGeoPocket,
    /// Atari Jaguar (1993). 64-bit cart console with a notoriously
    /// complex Pro Controller — d-pad + 3 face buttons (A/B/C) +
    /// OPTION/PAUSE + 12-key keypad (0-9 + * + #). Cart-shape, BIOS
    /// optional. Virtual Jaguar is the default libretro core.
    Jaguar,
    /// Atari Jaguar CD (1995). CD-ROM expansion atop the Jaguar
    /// cartridge slot — a separate SystemId from `Jaguar` because the
    /// load path differs (CD images vs cart files) and the BIOS
    /// requirement is mandatory rather than optional. Same Virtual
    /// Jaguar core (`virtualjaguar_libretro.dll`); needs both
    /// `jagboot.rom` (cart-side boot, also required for cart play)
    /// and `jagcd.rom` (CD-side boot) in `<exe_dir>/system/`. Library
    /// is small (~13 retail releases — Battlemorph, Highlander I,
    /// Hover Strike: Unconquered Lands, etc.) plus a growing homebrew
    /// scene. Shares the Jaguar controller layout 1:1 — same bindings
    /// module covers both.
    JaguarCd,
    /// 3DO Interactive Multiplayer (1993). CD-shape; the launch console
    /// was Panasonic's FZ-1, later joined by FZ-10 / GoldStar GDO-101M
    /// / Sanyo Try. Opera (formerly 4DO) is the default libretro core.
    /// Regional BIOS required (panafz1.bin / panafz10.bin / etc.).
    ThreeDo,
    /// NEC PC-FX (1994-1998). The PC Engine's CD-only 32-bit
    /// successor — Japan-only platform with a library overwhelmingly
    /// dominated by anime-style visual novels + dating sims. Beetle
    /// PC-FX (Mednafen lineage shared with pce-cd / saturn / psx /
    /// vb / ws / lynx / ngp) is the default core. BIOS required
    /// (pcfx.rom).
    PcFx,
    /// Nintendo 64 (1996). 64-bit cart console; Mupen64Plus-Next is
    /// the default libretro core (with GLideN64 video plugin). Heavy
    /// CPU + GPU. BIOS-free (CIC boot ROM emulated internally). The
    /// N64 controller's analog stick is the primary movement input
    /// for nearly every title — Phase 0 ships digital fallbacks with
    /// the d-pad-to-analog core option, full analog axes plumbed via
    /// the new `set_analog_state` trait method.
    N64,
    /// Nintendo GameCube + Wii (2001 / 2006). Optical-disc-shape; one
    /// Dolphin .dll covers both via runtime auto-detect from disc
    /// container. Heavy CPU + GPU + 64-bit host required. BIOS-free.
    /// GC pad analog + C-stick + analog triggers + Wii Remote /
    /// Nunchuk all need analog dispatch (Phase 0 ships base analog
    /// stick infra; full per-axis Bindings UI is Phase 2.5).
    GameCube,
    /// Sega Dreamcast (1998-2001). CD-shape (GD-ROM media; `.cdi` /
    /// `.gdi` are Dreamcast-unique container formats, `.chd` shared
    /// with other CD systems). Flycast is the default libretro core.
    /// Regional BIOS required (`dc_boot.bin` + `dc_flash.bin` per
    /// region). The DC controller has a single analog stick (no
    /// C-stick); flows through the cross-cutting analog input infra.
    /// VMU peripheral + light gun support deferred to Phase 2.5.
    Dreamcast,
    /// Sony PlayStation Portable (2004-2014). UMD-shape (Universal
    /// Media Disc) handheld. PPSSPP is the default libretro core.
    /// BIOS-free (PPSSPP synthesizes firmware behavior). Single
    /// analog stick (no right stick on PSP-1000/2000/3000; PSP Go
    /// added second stick but is rare). Analog stick flows through
    /// the shared analog infra.
    Psp,
    /// Sony PlayStation 2 (2000-2013). DVD-shape with some CD-format
    /// titles. LRPS2 (PCSX2) is the default libretro core. Regional
    /// BIOS required (scph10000 JP launch / scph39001 US fat /
    /// scph70000 US-EU slim / etc.); slots into the CD-launch BIOS
    /// dispatch arm as the 9th CD-shape system. DualShock 2 controller
    /// — PSX-shape + dual analog sticks (analog infra) + pressure-
    /// sensitive face buttons + analog L2/R2 (Phase 2.5).
    Ps2,
    /// Nintendo DS (2004-2014). Cart-shape (DS game card). melonDS is
    /// the default libretro core. 3-file BIOS required (bios7.bin +
    /// bios9.bin + firmware.bin). Touch screen is the platform's
    /// defining input — flows through the new RETRO_DEVICE_POINTER
    /// dispatch (mouse-as-touch at Phase 0; window-relative pixel-
    /// perfect mapping is Phase 2.5).
    Nds,
    /// Nintendo Game Boy + Game Boy Color (single SystemId — Gambatte
    /// auto-detects DMG vs CGB from the loaded ROM header).
    Gb,
    /// Nintendo Game Boy Advance (separate SystemId from `Gb` — different
    /// CPU architecture, different libretro cores).
    Gba,
    /// Atari 2600 / VCS — the granddaddy. Variant named `Atari2600`
    /// (not just `2600`) because Rust identifiers can't start with a
    /// digit; the slug stays `"2600"` in parse_system_id + the
    /// frontend SystemId string union.
    Atari2600,
    /// Mattel Intellivision (1979). 16-direction disc controller (mapped
    /// to libretro D-pad as 8-way in Phase 0; full disc is a Phase 2
    /// analog-input dependency).
    Intellivision,
    /// Magnavox Odyssey² / Videopac G7000 (1978). Joystick + single
    /// fire button + 47-key alphanumeric keyboard. Slug stays `"o2"`.
    Odyssey2,
    /// Fairchild Channel F (1976). The FIRST cartridge-based home
    /// console; predates the 2600 by a year. Plunger controller with
    /// 4-axis analog + push-in fire (simplified to 6 RetroPad buttons
    /// in Phase 0).
    ChannelF,
    /// Atari 5200 SuperSystem (1982). Atari's console counterpart to the
    /// 400/800 home computers — same 6502 + ANTIC/CTIA hardware, paired
    /// with the iconic (notoriously fragile) analog-joystick + 12-key
    /// keypad controller. Slug stays `"5200"`; variant named
    /// `Atari5200` to dodge Rust's "no leading digit" identifier rule.
    /// Default core `atari800_libretro.dll`. Requires `5200.rom` BIOS
    /// in `<exe_dir>/system/`.
    Atari5200,
    /// Nintendo Pokémon Mini (2001-2002). The smallest first-party
    /// Nintendo hardware ever shipped — single-button keypad (A / B /
    /// C) + d-pad + shake sensor + IR. PokeMini libretro core handles
    /// playback. Requires `bios.min` (4 KB).
    PokeMini,
    /// ScummVM — adventure-game engine launcher (not a hardware
    /// platform). A "game" is a directory of data files (`MONKEY.000`,
    /// `MONKEY.001`, …) plus a tiny `.scummvm` descriptor file that
    /// names the game ID + engine. The libretro core auto-detects the
    /// engine and opens game data next to the descriptor file. Mouse-
    /// primary + keyboard passthrough for text input (sword-fighting
    /// insults, password prompts). No BIOS.
    ScummVm,
    /// DOSBox — MS-DOS game runner (not a hardware platform). A "game"
    /// is a directory containing the game's executable + data files;
    /// the libretro `dosbox_pure_libretro.dll` core auto-detects the
    /// entry point at launch. Operators can override the entry point
    /// via `GameOverrides.dosbox_entry_point` for the ~10% of titles
    /// where auto-detect picks the wrong .exe (typically install
    /// utilities sitting next to the real game binary). Keyboard +
    /// gamepad-mapped action layout for arcade-style DOS games (Doom,
    /// Wolfenstein 3D, Commander Keen, Jazz Jackrabbit); pointer for
    /// mouse-driven games (X-COM, SimCity, Civilization). No BIOS.
    DosBox,
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
/// for each core defines its layout). `axes` carries analog stick inputs (libretro
/// RETRO_DEVICE_ANALOG range, -32768..32767; layout: `[Left X, Left Y, Right X,
/// Right Y]`). `pointer` carries touch-screen / mouse pointer state for systems
/// that use libretro RETRO_DEVICE_POINTER (Nintendo DS stylus, light-gun games,
/// etc.). All three are zero-filled for purely-digital cores that don't poll them.
#[derive(Debug, Clone, Copy)]
pub struct InputState {
    /// System-specific button bitfield.
    pub buttons: u32,
    /// Analog axes in signed 16-bit range, 0 for unused.
    pub axes: [i16; 4],
    /// Pointer device state. `(x, y, pressed, in_viewport)` where:
    /// - `x` / `y` are signed 16-bit normalized coordinates in libretro's
    ///   POINTER range (-32768 = top-left edge, 32767 = bottom-right edge).
    /// - `pressed` is whether the pointer is currently "touching" the screen
    ///   (mouse button held for a mouse-as-touch input, finger down for a
    ///   real touch screen).
    /// - `in_viewport` is whether the pointer is currently inside the
    ///   game-output rectangle. When the operator aims the cursor off-
    ///   screen this flag flips to false; light-gun cores read it via
    ///   `RETRO_DEVICE_ID_LIGHTGUN_IS_OFFSCREEN` to trigger the reload
    ///   gesture (House of the Dead 2, Time Crisis series, etc.).
    /// Set to `(0, 0, false, false)` for systems that don't use the
    /// pointer device.
    pub pointer: (i16, i16, bool, bool),
    /// Secondary pointer for multi-touch dispatch. Same shape as
    /// `pointer`; cores polling `RETRO_DEVICE_POINTER` with `index = 1`
    /// receive this tuple's fields. NDS dual-stylus titles (Hotel Dusk
    /// 3D mode, Glory of Heracles two-finger gestures, the handful of
    /// homebrew that exercise multi-touch) are the v1 motivating
    /// consumers. Cores polling with `index = 0` see `pointer`;
    /// `index ≥ 2` reads zero. The `RETRO_DEVICE_ID_POINTER_COUNT`
    /// poll reports how many pointers are currently `pressed`
    /// (0 / 1 / 2).
    ///
    /// V1 plumbing only: `InputPoller` leaves this at
    /// `(0, 0, false, false)` until a real second-finger input source
    /// is wired (operator-driven; second-mouse / actual touchscreen /
    /// future Surface-pen path). The shape exists today so cores can
    /// poll it without crashing and so a second-source PR is purely
    /// additive at the poll site.
    pub pointer_secondary: (i16, i16, bool, bool),
    /// Per-button analog pressure for cores that poll
    /// `RETRO_DEVICE_INDEX_ANALOG_BUTTON`. Slot `i` corresponds to
    /// `RETRO_DEVICE_ID_JOYPAD_<bit i>`. Range 0..32767 (positive only —
    /// analog buttons don't go below "fully released"). For most buttons
    /// the value is binary (0 when released, 32767 when pressed); slots
    /// 12 (L2) and 13 (R2) get continuous values from gilrs's actual
    /// trigger axes when a gamepad is connected, so games that read
    /// trigger pressure (Gran Turismo brake / Metal Gear Solid prone)
    /// receive real analog data instead of all-or-nothing presses.
    ///
    /// Zero-filled for cores that don't poll the analog-button index;
    /// the digital `buttons` bitmask remains the source of truth for
    /// `RETRO_DEVICE_JOYPAD` polls regardless of this field's contents.
    pub analog_buttons: [i16; 16],
    /// Gun-side button bitmask for `RETRO_DEVICE_LIGHTGUN`. Bit
    /// position == `RETRO_DEVICE_ID_LIGHTGUN_*`:
    /// - bit 3  → AUX_A     (Time Crisis alt-fire, Wild Gunman dodge)
    /// - bit 4  → AUX_B     (Hogan's Alley reload-by-button alt)
    /// - bit 6  → START     (pause / continue prompts)
    /// - bit 7  → SELECT    (menu screens on console-ported guns)
    /// - bit 8  → AUX_C     (third gun-side button on Justifier)
    /// - bit 9  → DPAD_UP   (menu nav on console-ported guns)
    /// - bit 10 → DPAD_DOWN
    /// - bit 11 → DPAD_LEFT
    /// - bit 12 → DPAD_RIGHT
    /// - bit 16 → RELOAD    (Time Crisis pedal alternative; the canonical
    ///                       off-screen-aim reload still flows through
    ///                       `IS_OFFSCREEN` from `pointer.in_viewport`)
    ///
    /// `u32` deliberately rather than `u16` — RELOAD is libretro id 16,
    /// which doesn't fit in `u16`. Bits not listed above are reserved
    /// (libretro hasn't allocated 5 or 17+ to LIGHTGUN ids).
    /// `TRIGGER` (id 2) stays driven by `pointer.pressed`, not this
    /// bitmask, so left-click continues to fire even when the operator
    /// has no per-system gun-side bindings configured.
    ///
    /// Zero-filled for cores not polling LIGHTGUN.
    pub lightgun_buttons: u32,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            buttons: 0,
            axes: [0; 4],
            pointer: (0, 0, false, false),
            pointer_secondary: (0, 0, false, false),
            analog_buttons: [0; 16],
            lightgun_buttons: 0,
        }
    }
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

/// One supported controller / peripheral device that a core advertises
/// for a given port via libretro's
/// `RETRO_ENVIRONMENT_SET_CONTROLLER_INFO` env call.
///
/// Cores extend libretro's base device space (`RETRO_DEVICE_JOYPAD = 1`,
/// `RETRO_DEVICE_MOUSE = 2`, `RETRO_DEVICE_LIGHTGUN = 4`, …) via
/// `RETRO_DEVICE_SUBCLASS(base, id) = ((id + 1) << 8) | base` to declare
/// system-specific peripherals (FCEUmm's Zapper = 258, snes9x's Super
/// Scope = 260, Genesis Plus GX's Light Phaser = 260, Beetle PSX's
/// GunCon = 260, etc.). Each core publishes its own `(label, id)` list
/// per port — the frontend reads this directly instead of maintaining
/// hardcoded per-system option tables that go stale on every core
/// update.
///
/// Labels are authored by core maintainers in the core's canonical
/// language ("Zapper" vs "Light Phaser" vs "GunCon") and should be
/// displayed verbatim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ControllerDeviceDescriptor {
    /// Human-readable label sourced from the core's
    /// `retro_controller_description::desc`. Empty when the core passed
    /// a NULL `desc` pointer (spec-permitted but rare).
    pub label: String,
    /// Wire id passed to `retro_set_controller_port_device`. May be a
    /// base id (1 / 2 / 4 / …) or a subclass-encoded id (258 / 260 /
    /// 257 / …). The frontend dispatches this value verbatim.
    pub id: u32,
}

/// One per-game button (or stick / mouse / pointer / etc.) description
/// that a core advertises via libretro's
/// `RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS` env call.
///
/// Where [`ControllerDeviceDescriptor`] declares "this port supports
/// THESE device types," `InputDescriptor` declares "for this specific
/// game, the button at `(port, device, index, id)` does THIS thing
/// semantically." FCEUmm publishes Super Mario Bros' B as "Run" and
/// Castlevania's B as "Whip" — same RetroPad bit, different in-game
/// meaning, both authored upstream by the core maintainer.
///
/// Per-game key: cores can re-publish across a game's lifetime when
/// internal modes change (fighting-game character-select vs in-match),
/// so the latest publish wins. Cache layer keys on
/// `(core_filename, game_sha1)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputDescriptor {
    /// Controller port (0-indexed). Standard RetroPad layout uses
    /// port 0 for the primary controller; light-gun games typically
    /// use port 1 for the gun.
    pub port: u32,
    /// libretro device type the descriptor applies to. Matches the
    /// device id passed to `retro_set_controller_port_device`.
    /// Common values: `RETRO_DEVICE_JOYPAD` (1), `RETRO_DEVICE_ANALOG`
    /// (5), `RETRO_DEVICE_MOUSE` (2), `RETRO_DEVICE_LIGHTGUN` (4).
    pub device: u32,
    /// Sub-index within the device. For JOYPAD this is 0; for ANALOG
    /// it distinguishes the left stick (0) from the right stick (1).
    pub index: u32,
    /// Per-device button / axis id (e.g. `RETRO_DEVICE_ID_JOYPAD_A`
    /// for JOYPAD; `RETRO_DEVICE_ID_ANALOG_X` for ANALOG).
    pub id: u32,
    /// Human-readable description the core authored (e.g. "Run",
    /// "Whip", "Jump", "Light Punch"). Empty string when the core
    /// passed a NULL `description` pointer (spec-permitted but rare).
    pub description: String,
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

/// Severity hint for a [`CoreMessage`]. Maps onto the toast-stack
/// levels in the shell — Info / Warn / Error pick the matching tint.
/// libretro's Debug-level messages fold into Info.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CoreMessageLevel {
    /// Routine OSD ("Save state slot 1 saved", "Disc 2 inserted").
    Info,
    /// Soft warning ("BIOS missing, falling back to HLE").
    Warn,
    /// Hard error ("ROM rejected: bad checksum").
    Error,
}

/// One OSD-style status string surfaced by the core.
///
/// Cores call libretro's `SET_MESSAGE` (legacy, frames-based duration)
/// or `SET_MESSAGE_EXT` (modern, ms-based + priority + target) to push
/// short user-visible status updates. The wrapper layer normalizes the
/// two formats into this shape and queues them; the shell drains the
/// queue per frame and renders via the toast stack.
///
/// `log_only` is set by cores that explicitly target the log channel
/// instead of the OSD (SET_MESSAGE_EXT target = LOG). The shell skips
/// toasting those — the underlying `log::info!` line is already
/// emitted at parse time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreMessage {
    /// UTF-8 message body.
    pub text: String,
    /// Severity hint for the toast tint.
    pub level: CoreMessageLevel,
    /// True when the core specifically targeted the LOG channel (not
    /// the OSD). The shell logs the text but skips toasting.
    pub log_only: bool,
}

/// A single descriptor from the core's published memory map.
///
/// Cores call libretro's `SET_MEMORY_MAPS` once during init (after
/// `retro_load_game`) to describe how their guest address space maps
/// onto host buffers. Each descriptor names a region: a `start` guest
/// address, a `len` byte count, plus `select` / `disconnect` masks that
/// describe address-line mirroring (NES PRG mirroring, SNES bank
/// folding, etc.). The host-side base pointer is intentionally NOT
/// surfaced here — [`Core::memory_region`] is the safe accessor for
/// raw bytes. Memory descriptors are metadata, useful for:
///
/// - **RetroAchievements** rcheevos integration (translates achievement
///   conditions written against guest addresses → host bytes).
/// - **Cheat search UIs** (operator picks an address range; the
///   descriptor's `len` + `addrspace` tag makes the choices readable).
/// - **AI / scripting layers** that want to read game state by symbolic
///   guest address rather than raw region offsets.
///
/// Empty Vec for cores that don't publish a memory map (most pre-NES
/// hardware, lightweight cores, anything where `SET_MEMORY_MAPS` was
/// never called).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryDescriptor {
    /// Bitmask of `RETRO_MEMDESC_*` access / type flags from libretro.
    /// We don't decode the bits — consumers that care (rcheevos) read
    /// them directly. Stored opaquely so future libretro spec additions
    /// pass through without a wrapper-layer update.
    pub flags: u64,
    /// Offset into the host buffer (the libretro spec's "where in the
    /// underlying ptr does this region start"). Combined with
    /// [`Core::memory_region`] data when the host needs to read bytes.
    pub offset: u64,
    /// Guest address where this region begins.
    pub start: u64,
    /// Address-bit mask that selects this descriptor when matching a
    /// guest address. See libretro spec for the (somewhat involved)
    /// resolution rules — most descriptors set `select = 0` to mean
    /// "single contiguous region."
    pub select: u64,
    /// Address bits the chip ignores (e.g. NES PRG mirroring at
    /// `$C000-$FFFF` when only `$8000-$BFFF` is wired). Zero for
    /// non-mirrored regions.
    pub disconnect: u64,
    /// Region length in bytes.
    pub len: u64,
    /// Optional address-space tag — single-letter conventions like
    /// `"S"` for system bus, `"V"` for VRAM. None when the core didn't
    /// declare one.
    pub addrspace: Option<String>,
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
    /// Filesystem paths for each disc image (v2 cores only). Empty
    /// Vec for v1 cores or when the core declined to fill the path
    /// buffer. Length equals `num_discs` when present. Useful for
    /// "show me where this disc lives" tooltips + as a label fallback
    /// when the core didn't populate `labels`.
    pub paths: Vec<String>,
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

    /// Option keys the core currently wants hidden — RetroArch parity
    /// for libretro's `SET_CORE_OPTIONS_DISPLAY` env. Cores push this
    /// set to hide options that don't apply given the current values
    /// of other options (e.g. "Lightgun crosshair color" disappears
    /// when "Lightgun" is off). The settings panel filters these out
    /// at render time. Default empty for cores without a visibility
    /// concept; they show every option.
    fn hidden_option_keys(&self) -> Vec<String> {
        Vec::new()
    }

    /// Ask the core to re-evaluate which options should be hidden
    /// given the current set of variable values — RetroArch parity
    /// for libretro's `SET_CORE_OPTIONS_UPDATE_DISPLAY_CALLBACK` env.
    /// Called by the shell after `set_option` mutates a value so any
    /// downstream options the core wants to hide / reveal get updated
    /// before the panel re-fetches. Default no-op for cores that
    /// don't expose a visibility callback; their initial hidden set
    /// remains in effect.
    fn refresh_option_visibility(&mut self) {}

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

    /// Drain OSD-style status messages the core has queued via
    /// libretro's `SET_MESSAGE` / `SET_MESSAGE_EXT`. The shell calls
    /// this each frame after `run_frame` and routes the entries to
    /// the toast stack. Returns an empty Vec for cores that don't push
    /// messages — most pre-NES systems never do.
    fn drain_messages(&mut self) -> Vec<CoreMessage> {
        Vec::new()
    }

    /// Memory map descriptors as registered by the core via
    /// libretro's `SET_MEMORY_MAPS`. Empty Vec for cores that don't
    /// publish a memory map (most pre-NES hardware, lightweight cores,
    /// anything where the env was never called).
    ///
    /// Stored as metadata only — host-side raw pointers stay inside
    /// the Core implementation. To read bytes from a region named here,
    /// pair the descriptor with [`Core::memory_region`] (when the
    /// region maps onto a libretro standard region) or a future
    /// rcheevos-style helper.
    fn memory_map(&self) -> Vec<MemoryDescriptor> {
        Vec::new()
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
