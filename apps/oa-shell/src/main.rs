// oa-shell — the Tauri binary.
//
// Two shell modes, picked at startup via the `OA_SHELL_MODE` env var:
//   - "two-window" (default): a Tauri WebviewWindow holds the library UI,
//     and a separate native Window hosts the wgpu game surface.
//   - "single-window": one transparent WebviewWindow whose underlying HWND
//     hosts the wgpu game surface beneath the library UI. Requires
//     `html, body { background: transparent }` in the loaded document.
//     Validated by spike 04 on Windows + DWM (see docs/DECISIONS.md).
//
// ROMs are launched from the library WebView via the `launch_rom` Tauri
// command. The legacy `OA_ROM` env-var still works as a fallback that loads
// a ROM at startup before the library can issue any commands.

#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod archive;
mod bindings;
mod cd_id;
mod cheat_search;
mod cli;
mod core_installer;
mod core_options;
mod layout;
mod library_db;
mod library_groups;
mod library_prefs;
mod logger;
mod media;
mod metadata;
mod normalize;
mod patch;
mod rom_hashes;
mod rom_header;
mod scan_service;
mod shader_presets;
mod shader_presets_watcher;
mod system_settings;
mod title_parse;
mod video_capture;
mod watcher;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use oa_core::PortIndex;
use oa_input::Keycode;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use tauri::{Emitter, Manager};

use bindings::Bindings;

/// Toast event payload sent to the frontend over the `oa://toast` channel.
/// `level` drives the leading glyph + per-level accent (info/success neutral,
/// warn amber, error red); `system` lets the toast pick up that system's CSS
/// cascade colors via `[data-system="<id>"]` (defaults to the document root's
/// active system). Auto-dismissed after 2.5 s (errors after 4 s) by the
/// `ToastStack` component on the frontend.
#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ToastPayload {
    level: ToastLevel,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
}

#[derive(serde::Serialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum ToastLevel {
    Info,
    Success,
    Warn,
    Error,
}

/// Best-effort toast emit — failures are logged and swallowed because toasts
/// are transient feedback, never load-bearing.
fn emit_toast(
    handle: &tauri::AppHandle,
    level: ToastLevel,
    system: Option<&str>,
    message: impl Into<String>,
) {
    let payload = ToastPayload {
        level,
        message: message.into(),
        system: system.map(|s| s.to_string()),
    };
    if let Err(e) = handle.emit("oa://toast", payload) {
        log::warn!("oa-shell: toast emit failed: {e:?}");
    }
}

/// Convenience — every toast we emit today is from the tg16 core. Once a
/// second system comes online this will take a system_id parameter; for now
/// the system tag is hard-coded so the cascade picks up the TG-16 accent.
fn toast(handle: &tauri::AppHandle, level: ToastLevel, message: impl Into<String>) {
    emit_toast(handle, level, Some("tg16"), message);
}

enum EmuCommand {
    LoadRom {
        path: String,
        bytes: Vec<u8>,
        restore_slot: Option<u32>,
        /// Direct-launch `--state-file PATH`: after the core accepts the ROM,
        /// read this state file (full absolute path, bypasses the per-game
        /// slot directory convention) and load it. Mutually exclusive with
        /// `restore_slot` — clap rejects both at parse time.
        restore_state_path: Option<PathBuf>,
        /// Per-game core override (filename of a .dll/.so/.dylib in `<exe_dir>/cores/`).
        /// `None` = fall back to the per-system pref → hardcoded default. If the
        /// resolved core differs from what's currently loaded, the emu thread drops
        /// the running core and loads the new one before invoking load_rom.
        core_override: Option<String>,
        // The frontend's `SystemId` for this ROM (e.g. "tg16", "lynx"). Used
        // by the emu thread to look up the right per-system core pref + the
        // right input bit remap, and tagged into the `LibretroCore` for
        // metadata (`core.system()`). Defaults to "tg16" if absent for
        // backward compatibility with any path that hasn't been updated yet.
        system_id: String,
    },
    /// Release the currently-loaded ROM but keep the core initialised so a
    /// subsequent LoadRom can re-use it. The renderer keeps presenting the
    /// last framebuffer (libretro `run_frame` is a no-op when no ROM is
    /// loaded). `title` is the operator-visible ROM title for the success
    /// toast; `None` => generic "Unloaded".
    UnloadRom { title: Option<String> },
    SetScalingMode(oa_render::ScalingMode),
    /// Phase 3 slice C — apply a TOML-resolved shader preset to the
    /// renderer. The preset name resolves on the Tauri-command side
    /// (file I/O + PNG decode happen there); this command carries the
    /// already-decoded values to the emu thread:
    ///   - `base` selects the renderer's pipeline branch.
    ///   - `bloom_amount = Some(x)` overrides the Phosphor composite
    ///     weight; `None` leaves the current value alone.
    ///   - `bezel = Some(rgba,w,h)` installs a bezel overlay; `None`
    ///     clears any active bezel (so selecting `plain` after a
    ///     bezel'd preset removes the bezel).
    /// Replaces the slice-A `SetShaderPreset(enum)` variant — the
    /// preset name is the source of truth now, not the enum value.
    ApplyShaderPreset(shader_presets::ResolvedPreset),
    /// Phase 3 slice C polish — per-system / per-game override for the
    /// Phosphor composite weight. Applied AFTER `ApplyShaderPreset` so it
    /// supersedes the TOML preset's default. Clamped 0..1 on the renderer
    /// side. No-op for non-Phosphor presets (the renderer accepts it
    /// regardless; the shader branch only reads it when preset_id == 3).
    SetBloomAmount(f32),
    /// Override the framebuffer's reported display_aspect at the
    /// renderer. `None` = restore "trust the core" (the default).
    /// `Some(x)` substitutes `x` until cleared. Resolves per-game →
    /// per-system → core-reported at the shell layer; this is the
    /// resolved value the emu thread applies.
    SetDisplayAspectOverride(Option<f32>),
    /// Apply a per-edge overscan crop at the renderer. `OverscanCrop::NONE`
    /// (the default) leaves the framebuffer un-cropped. Resolves
    /// per-game → per-system → none at the shell layer; this is the
    /// resolved value the emu thread applies. Crops larger than half
    /// the framebuffer clamp to 1 visible pixel per axis renderer-side.
    SetOverscanCrop(oa_render::OverscanCrop),
    /// Apply a bezel image override on top of whatever the active
    /// shader preset's TOML `[bezel]` block produced. `Some(rgba)`
    /// uploads + shows; `None` reverts to the preset's default (which
    /// may itself be no bezel). Resolves per-game → per-system → null
    /// at the shell layer.
    SetBezelOverride(Option<shader_presets::ResolvedBezel>),
    /// RetroArch-parity slice — push a single libretro core-option value
    /// into the running core. Calls `Core::set_option(key, value)` which
    /// writes into the libretro state map + raises the
    /// `GET_VARIABLE_UPDATE` flag so the core re-reads on its next poll.
    /// Drops silently if no core is loaded.
    SetCoreOption { key: String, value: String },
    /// RetroArch-parity slice — bulk-apply every effective core-option
    /// value for the active ROM (per-game overlay on per-system overlay
    /// on schema default). Sent once on every LoadRom right after the
    /// core finishes load_game.
    ApplyCoreOptions(std::collections::HashMap<String, String>),
    /// RetroArch-parity slice 7 — set the run-ahead frame count.
    /// 0 = disabled (default); 1-5 = run that many frames forward each
    /// real frame, present the future framebuffer, then rollback via
    /// save/load_state. Reduces perceived input latency by N frames.
    /// Suppressed during scrub / TAS / pause / FF / SM.
    SetRunAhead(u32),
    /// RetroArch-parity slice — open or close the disc tray (multi-disc
    /// CD games). Drops silently if the current core hasn't registered
    /// a disc-control interface. Emu thread refreshes `disc_state` cache
    /// after the call so the frontend's next `get_disc_state` reflects
    /// the new tray state.
    SetDiscEject(bool),
    /// RetroArch-parity slice — swap to disc `index` (0-based). Only
    /// effective while the tray is ejected; cores typically refuse +
    /// log otherwise. Emu thread refreshes `disc_state` cache.
    SetDiscImage(u32),
    ApplyBindings(Bindings),
    /// Phase 2.5 — push analog routing (deadzone, sensitivity, invert,
    /// keyboard-axis fallback, stick-swap) to one port's slot. Resolved
    /// per-game → per-system → identity at the shell layer; this is the
    /// resolved value the emu thread applies. `port` is 0..=4.
    SetAnalogRouting { port: u32, routing: oa_input::AnalogRouting },
    /// Wire a controller port to a specific libretro device type. Used
    /// to switch a game from the default RetroPad to Mouse (Mario Paint),
    /// Light Gun (Zapper / Time Crisis), Paddle (Breakout / Arkanoid),
    /// or back to None / Disconnected. `device` is one of
    /// `oa_libretro::ffi::RETRO_DEVICE_*` (NONE / JOYPAD / MOUSE /
    /// KEYBOARD / LIGHTGUN / ANALOG / POINTER). Must run AFTER
    /// `retro_load_game` per the
    /// `reference_libretro_controller_after_load_game` note — emu thread
    /// already orders LoadRom → ApplyCoreOptions → … so this is
    /// dispatched the same way (frontend's `arm_libretro_device` fires
    /// it after launch_rom completes).
    SetPortDevice { port: u32, device: u32 },
    /// `None` = system default device. The emu thread rebuilds the cpal stream
    /// against the new selection; failure is logged but doesn't kill the thread.
    SetAudioDevice(Option<String>),
    /// Phase 4 slice A — reconfigure the rewind ring. Frontend resolves
    /// `enabled` / `capture_interval_frames` / `max_bytes` from the
    /// per-game → per-system → OA-wide inheritance chain at launch time
    /// and pushes the result here. Toggling `enabled` off flushes the ring.
    SetRewindConfig(oa_savestate::RewindConfig),
    /// Phase 4 slice B — enter scrub mode. Forward play + capture freeze
    /// until [`EmuCommand::EndRewindScrub`] arrives. Idempotent: a second
    /// Start while already scrubbing is a no-op.
    StartRewindScrub,
    /// Phase 4 slice B — preview the snapshot at `steps_back` from the
    /// newest. Clamped to ring length on the emu side; if the ring is
    /// empty the command is ignored. Cheap to spam at drag rate.
    SetRewindScrubPosition { steps_back: u32 },
    /// Phase 4 slice B — exit scrub mode. `commit = true` truncates
    /// snapshots newer than the current scrub position (the user has
    /// chosen a point in the past and the "future" is rewritten);
    /// `commit = false` restores the most-recent snapshot so no history
    /// is lost. Either way, forward play resumes from the next frame.
    EndRewindScrub { commit: bool },
    /// Phase 4 slice C — start TAS recording. Captures the current
    /// `Core::save_state` as the initial state, then logs every input
    /// frame the emu thread dispatches until [`EmuCommand::StopTasRecording`]
    /// arrives. Hold-Backspace rewind is disabled for the duration of
    /// the recording (v1 simplification; v2 will truncate the input log
    /// on rewind). Ignored when no ROM is loaded or when already
    /// recording / replaying.
    StartTasRecording { display_name: String },
    /// Phase 4 slice C — finalize the in-progress recording. Writes a
    /// `.tas` file under `appDataDir/tas/<rom-stem>/<timestamp>.tas`
    /// and emits an event with the resulting path. `discard = true`
    /// skips the write entirely.
    StopTasRecording { discard: bool },
    /// Phase 4 slice C — start replay of an already-decoded recording.
    /// The frontend reads the file via `start_tas_replay` Tauri command
    /// which decodes + passes the parsed struct here. Emu thread loads
    /// the initial state, then dispatches recorded input each frame.
    /// Replay stops automatically at end of input_frames.
    StartTasReplay(Box<oa_savestate::tas::TasRecording>),
    /// Phase 4 slice C — abort replay early. Forward play resumes from
    /// the current frame (whatever state the replay had reached). No-op
    /// if no replay is in progress.
    StopTasReplay,
    /// Phase 4 slice D — start frame-by-frame video capture. Spawns a
    /// PNG-encoder worker thread; emu thread pushes RGBA framebuffers
    /// into a bounded channel after each `run_frame`. Channel overflow
    /// drops frames (counted in the manifest). Ignored if already
    /// capturing or no ROM is loaded.
    StartVideoCapture { display_name: String },
    /// Phase 4 slice D — stop video capture. `discard = true` deletes
    /// the clip directory; otherwise the worker drains remaining
    /// frames and writes `manifest.json`.
    StopVideoCapture { discard: bool },
    /// Phase 4 slice F — load this game's milestones into the active
    /// runtime evaluator. Sent on every LoadRom by the shell-side
    /// launch flow (which reads from SQLite). The emu thread evaluates
    /// each predicate on every frame; on rising-edge it stamps the
    /// trigger time via `mark_milestone_triggered` on the same
    /// shared LibraryDb handle.
    /// LoadMilestones doubles as the clear-runtime command — send an
    /// empty Vec to drop every armed milestone. (Used to have a separate
    /// `ClearMilestones` variant; removed 2026-05-18 after the empty-Vec
    /// path superseded it.)
    LoadMilestones(Vec<library_db::Milestone>),
    /// RetroArch parity slice 5 — load the per-game cheat set into the
    /// runtime evaluator. Replaces whatever was previously armed. Sent
    /// from `handleLaunch` after `arm_milestones` so every frame's
    /// post-`run_frame` write loop sees the fresh set. Enabled cheats
    /// write `value` (`width` bytes, little-endian) into memory at
    /// `(region, offset)`.
    LoadCheats(Vec<library_db::Cheat>),
}

/// Phase 4 slice E — snapshot of the four libretro memory regions,
/// refreshed each emu frame so the Tauri-side memory inspector can
/// read bytes without round-tripping through the emu loop.
///
/// Each region is `None` when the loaded core doesn't expose it; the
/// Vec inside is a copy of the live bytes (libretro guarantees the
/// pointer + size are stable between load_game/unload_game calls, but
/// the bytes themselves mutate every frame).
#[derive(Clone, Debug, Default)]
struct MemorySnapshot {
    save_ram: Option<Vec<u8>>,
    rtc: Option<Vec<u8>>,
    system_ram: Option<Vec<u8>>,
    video_ram: Option<Vec<u8>>,
}

impl MemorySnapshot {
    fn region(&self, id: oa_core::MemoryRegionId) -> Option<&[u8]> {
        use oa_core::MemoryRegionId::*;
        match id {
            SaveRam => self.save_ram.as_deref(),
            Rtc => self.rtc.as_deref(),
            SystemRam => self.system_ram.as_deref(),
            VideoRam => self.video_ram.as_deref(),
            // `MemoryRegionId` is `#[non_exhaustive]` in oa-core so a
            // future region (e.g. expansion RAM) lands as a clean
            // "unavailable" until the snapshot grows a matching field.
            _ => None,
        }
    }
}

/// Phase 4 slice D — frame-by-frame video capture status surfaced to
/// the frontend via `get_video_state`. Updated on transitions + every
/// 30 frames during capture so the UI's "frames captured" status row
/// stays current without 60 Hz mutex thrash.
#[derive(Clone, Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SharedVideoState {
    capturing: bool,
    /// Frames pushed to the encoder so far. `dropped_frame_count`
    /// counts those that didn't fit in the channel buffer.
    frame_count: u64,
    dropped_frame_count: u64,
    display_name: String,
    /// Path to the in-progress clip directory; empty when idle.
    clip_dir: String,
}

/// Phase 4 slice C — TAS recording / replay state machine. Mutually
/// exclusive: at most one mode is active at a time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum TasMode {
    /// No recording, no replay — normal gameplay.
    #[default]
    Idle,
    /// Capturing inputs; finalize via StopTasRecording.
    Recording,
    /// Dispatching inputs from a loaded recording; auto-stops at EOF.
    Replaying,
}

/// Phase 4 slice C — TAS recording/replay status surfaced to the
/// frontend via `get_tas_state`. Updated by the emu thread on every
/// state transition + every Nth frame during recording/replay so the
/// UI's "frame X / Y" status row stays current.
#[derive(Clone, Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SharedTasState {
    mode: TasMode,
    /// Frames captured (recording) or dispatched so far (replaying).
    frame: u64,
    /// Total frames in the recording being replayed. 0 when not
    /// replaying.
    total_frames: u64,
    /// Display name set when recording started. Empty otherwise.
    display_name: String,
}

/// Live emu-thread perf stats. Surfaces FPS / frame count / audio
/// counters to the frontend so the Tools → Performance HUD can display
/// real telemetry instead of just UI render-loop FPS. Updated by the emu
/// thread every ~30 frames (cheap; doesn't churn the Mutex on every
/// frame). Reset to defaults when the core unloads so a fresh launch
/// starts with a zeroed counter.
#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SharedPerfStats {
    /// True while a core is loaded + running. When false, the other
    /// fields are stale (preserved from the last session for one
    /// frontend poll, then cleared by the unload handler).
    core_loaded: bool,
    /// Rolling-average actual frame rate the emu thread is hitting.
    /// Calculated from `frame_count / elapsed_since_core_load`. Will
    /// match `core_fps_nominal` when the host can keep up.
    fps: f64,
    /// Total emu frames since the current core was loaded.
    frame_count: u64,
    /// Audio samples pushed to the host's audio sink since core load.
    audio_pushed: u64,
    /// Audio samples dropped (ring-buffer full) since core load. Should
    /// stay near 0 in healthy operation; a non-zero counter means the
    /// emu thread is producing samples faster than the host can consume
    /// them (host fell behind, or buffer cap too small).
    audio_dropped: u64,
    /// Core's nominal frame rate (`retro_system_av_info.timing.fps`).
    /// PCE 59.83, SNES 60.10, Lynx 75, etc. 0.0 when no core is loaded.
    core_fps_nominal: f64,
}

/// Live rewind-ring stats published by the emu thread for Tauri commands
/// to read. Cheap to copy + small enough that locking the Mutex is faster
/// than any atomic-per-field scheme. The emu thread writes after every
/// capture / pop / scrub operation; the UI polls when a refresh is wanted
/// (typically on Quick Settings open + per scrub interaction).
#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SharedRewindState {
    /// Whether the ring is capturing at all (mirrors `RewindConfig.enabled`).
    enabled: bool,
    /// Current snapshot count.
    snapshot_count: u32,
    /// Total bytes the ring holds.
    byte_size: u64,
    /// Active capture cadence (frames between snapshots).
    capture_interval_frames: u32,
    /// Active core's frame rate. 0.0 when no core is loaded.
    fps: f64,
    /// True while a scrub interaction is active (set by StartRewindScrub,
    /// cleared by EndRewindScrub). When true, normal forward play +
    /// capture are paused; the displayed frame reflects `scrub_position`.
    scrubbing: bool,
    /// 0 = newest (live edge); count - 1 = oldest still held. Meaningful
    /// only while `scrubbing` is true; reset to 0 on scrub end.
    scrub_position: u32,
}

/// File extensions that must be loaded path-based (not bytes-based) — these
/// are CD container formats where the libretro core reads multiple files
/// relative to the given path (`.cue` references `.bin` tracks, etc.). Loading
/// these via in-memory bytes would either fail (no relative-path context) or
/// waste hundreds of MB of RAM for CHD images.
fn is_cd_extension(ext: &str) -> bool {
    // .pbp is the PSP-format PS1 EBOOT container — Beetle PSX HW reads
    // it directly and the BIOS pre-check still needs to fire (PSX BIOS
    // required regardless of container). Single-file format so the
    // Path vs Bytes routing also works either way; classifying as CD
    // routes .pbp through the same launch path the .cue/.chd PSX games
    // use, including the regional BIOS pre-check.
    matches!(ext, "cue" | "chd" | "ccd" | "toc" | "m3u" | "iso" | "pbp")
}

/// Map the frontend's `SystemId` string to `oa_core::SystemId`. The tag is
/// used by `LibretroCore` for metadata (`core.system()`) but doesn't change
/// any runtime behavior — wrong-tag is recoverable, not fatal. Unknown ids
/// fall back to `PcEngine` rather than panic since this can race a frontend
/// update that introduces a new system before the Rust side is rebuilt.
fn parse_system_id(s: &str) -> oa_core::SystemId {
    match s {
        "lynx" => oa_core::SystemId::Lynx,
        "atari7800" => oa_core::SystemId::Atari7800,
        "sms" => oa_core::SystemId::Sms,
        "gamegear" | "game-gear" => oa_core::SystemId::GameGear,
        "msx" | "msx2" => oa_core::SystemId::Msx,
        "coleco" | "colecovision" => oa_core::SystemId::Colecovision,
        // Mattel Intellivision — the canonical slug is "intv". Accept
        // "intellivision" as a longer-form alias.
        "intv" | "intellivision" => oa_core::SystemId::Intellivision,
        // Magnavox Odyssey² (US) / Videopac G7000 (EU). Slug stays "o2"
        // since "odyssey2" / "videopac" are both regional variants of
        // the same hardware.
        "o2" | "odyssey2" | "odyssey-2" | "videopac" => oa_core::SystemId::Odyssey2,
        // Fairchild Channel F — the granddaddy granddaddy (predates
        // 2600). Slug "channelf"; accept "channel-f" alias.
        "channelf" | "channel-f" | "fairchild" => oa_core::SystemId::ChannelF,
        // Atari 5200 SuperSystem — Atari's 1982 console. Slug stays "5200"
        // to dodge collision with Atari 7800 / 2600 slugs.
        "5200" | "atari5200" | "atari-5200" => oa_core::SystemId::Atari5200,
        // Nintendo Pokémon Mini — tiny 2001 handheld. Slug stays "pokemini".
        "pokemini" | "pokémon-mini" | "pokemon-mini" => oa_core::SystemId::PokeMini,
        "vectrex" => oa_core::SystemId::Vectrex,
        "virtualboy" | "virtual-boy" => oa_core::SystemId::VirtualBoy,
        "wonderswan" => oa_core::SystemId::WonderSwan,
        "pce-cd" => oa_core::SystemId::PceCdRom2,
        "nes" | "famicom" => oa_core::SystemId::Nes,
        "snes" | "super-famicom" => oa_core::SystemId::Snes,
        "mame" | "arcade" => oa_core::SystemId::Mame,
        "genesis" | "megadrive" | "mega-drive" => oa_core::SystemId::Genesis,
        // Sega CD / Mega-CD. CD-shape Mega Drive addon. Accept the JP
        // "Mega-CD" branding + the "mega-cd" / "megacd" aliases that
        // show up in saved configs.
        "segacd" | "sega-cd" | "mega-cd" | "megacd" | "mcd" => oa_core::SystemId::SegaCd,
        // Sega 32X. Cart-shape Mega Drive cart-slot addon. The JP name
        // ("Super 32X" / "Mega 32X") is rarely typed; accept the "32x"
        // shorthand for completeness.
        "sega32x" | "32x" | "sega-32x" => oa_core::SystemId::Sega32X,
        // Sega Saturn. Accept the "sat" / "ss" shorthand operators
        // sometimes use in saved configs, plus the JP "satturn" alias.
        "saturn" | "sat" | "ss" | "sega-saturn" => oa_core::SystemId::Saturn,
        // Sony PlayStation (PS1). Accept "ps1" / "ps" / "playstation"
        // — Sony's official naming was "PlayStation" (single word).
        "psx" | "ps1" | "ps" | "playstation" => oa_core::SystemId::Playstation,
        // SNK Neo Geo (AES home + MVS arcade — same SystemId; FBNeo
        // handles both via core option).
        "neogeo" | "neo-geo" | "aes" | "mvs" => oa_core::SystemId::NeoGeo,
        // SNK Neo Geo CD — separate slug from neogeo because the load
        // path differs (CD images need BIOS pre-check; carts don't).
        "neocd" | "neo-geo-cd" | "neogeocd" => oa_core::SystemId::NeoGeoCd,
        // SNK Neo Geo Pocket / Color — single slug per the gb pattern.
        // Beetle NeoPop auto-detects mono vs color from ROM header.
        "ngp" | "ngpc" | "neopocket" | "neo-geo-pocket" => oa_core::SystemId::NeoGeoPocket,
        "jaguar" | "jag" | "atari-jaguar" => oa_core::SystemId::Jaguar,
        // 3DO Interactive Multiplayer. Accept "3do" (canonical slug) and
        // a few common typed-in aliases — Rust identifier can't start
        // with a digit so the enum variant is `ThreeDo`.
        "3do" | "threedo" | "panasonic-3do" => oa_core::SystemId::ThreeDo,
        // NEC PC-FX — the PC Engine's CD-only 32-bit successor.
        "pcfx" | "pc-fx" | "pcefx" => oa_core::SystemId::PcFx,
        "n64" | "nintendo-64" | "nintendo64" => oa_core::SystemId::N64,
        // Single slug covers both GameCube + Wii via Dolphin's runtime
        // auto-detect from disc container.
        "gamecube" | "gc" | "wii" | "nintendo-gamecube" => oa_core::SystemId::GameCube,
        "dreamcast" | "dc" | "sega-dreamcast" => oa_core::SystemId::Dreamcast,
        "psp" | "playstation-portable" => oa_core::SystemId::Psp,
        "ps2" | "playstation-2" | "playstation2" => oa_core::SystemId::Ps2,
        "nds" | "ds" | "nintendo-ds" => oa_core::SystemId::Nds,
        // Single slug covers DMG + CGB — Gambatte auto-detects from the
        // ROM header. Accept the standard slug + common aliases users
        // might pass in from URLs or saved configs.
        "gb" | "gbc" | "gameboy" | "game-boy" | "game-boy-color" => oa_core::SystemId::Gb,
        // Game Boy Advance — separate SystemId from `Gb` since the
        // hardware is a different generation (32-bit ARM7TDMI vs Sharp
        // LR35902) and the libretro cores don't overlap.
        "gba" | "game-boy-advance" | "gameboyadvance" => oa_core::SystemId::Gba,
        // Atari 2600 / VCS — the granddaddy. Slug stays "2600" per the
        // plan; Rust variant is `Atari2600` since identifiers can't
        // start with a digit. Accept "vcs" + "atari2600" as common
        // aliases that show up in saved configs and external URLs.
        "2600" | "atari2600" | "vcs" => oa_core::SystemId::Atari2600,
        _ => oa_core::SystemId::PcEngine,
    }
}

/// Default libretro core .dll filename for a system. Used when the user has
/// no per-system pref set and no per-game override. The frontend's
/// `resolveScannableExtensions` already unions in `list_cores` valid_exts
/// so a dropped-in core gets picked up; this is just the fallback identity.
fn default_core_dll_for_system(system_id: &str) -> &'static str {
    match system_id {
        "lynx" => "mednafen_lynx_libretro.dll",
        // FCEUmm is the long-standing libretro NES default — broad
        // compatibility, light CPU. Operators wanting cycle-accurate
        // behavior swap to `mesen_libretro.dll` via PerSystemSettingsPage
        // → Cores.
        "nes" => "fceumm_libretro.dll",
        // Snes9x — the standard libretro SNES core. bsnes for accuracy.
        "snes" => "snes9x_libretro.dll",
        // MAME (latest) — the standard libretro MAME build. Operators
        // who want lighter perf-vs-compat tradeoffs (mame2003_plus_libretro,
        // mame2010_libretro, etc.) swap via the per-system Cores dialog.
        "mame" => "mame_libretro.dll",
        // ProSystem — the long-standing libretro Atari 7800 core. BIOS
        // (`7800 BIOS (U).rom`) optional but recommended; without it
        // games skip the boot logo but otherwise run. Operators who
        // want the alternate `a7800_libretro` build swap via the
        // per-system Cores dialog.
        "atari7800" => "prosystem_libretro.dll",
        // ClownMDEmu — modern, active-development Mega Drive core
        // (operator pick 2026-05-19 over Genesis Plus GX). Cart only;
        // Sega CD / 32X are future segacd / sega32x slugs. BIOS-free
        // for stock MD playback. Alternates via per-system Cores:
        // genesis_plus_gx_libretro (multi-Sega), picodrive_libretro,
        // blastem_libretro (higher accuracy).
        "genesis" => "clownmdemu_libretro.dll",
        // vecx — the libretro Vectrex default. Mature, light. The
        // Vectrex was a vector-display console (1982-1984); vecx renders
        // the vector beam paths to a raster framebuffer. No widely-shipped
        // alternate. Optional BIOS (vectrex.bin) for the era-correct boot
        // screen + Mine Storm pack-in game.
        "vectrex" => "vecx_libretro.dll",
        // Beetle VB — the libretro Virtual Boy default. Mednafen-derived,
        // mature. No BIOS required (VB never shipped with one).
        // Stereoscopic 3D output is handled core-side via configurable
        // anaglyph / side-by-side / 2D-flat modes.
        "virtualboy" => "mednafen_vb_libretro.dll",
        // Beetle WonderSwan — the libretro Wonderswan + WS Color default.
        // Mednafen-derived. Optional BIOS. Auto-detects mono vs color from
        // ROM header; also handles game-rotation flag (vertical games
        // automatically swap the active D-pad from X to Y physical pads).
        "wonderswan" => "mednafen_wswan_libretro.dll",
        // blueMSX — the long-standing libretro ColecoVision default.
        // Wide MSX-family compatibility (covers MSX1/2 + ColecoVision +
        // SVI-3x8 + several other Z80-era systems). Alternates: `gearcoleco`
        // (Coleco-only, lighter footprint). BIOS REQUIRED: `coleco.rom`
        // (~8 KB) in `<exe_dir>/system/` — the Coleco won't boot without
        // its system BIOS (the ROM is the entire firmware including the
        // boot screen).
        "coleco" => "bluemsx_libretro.dll",
        // FreeIntv — the libretro Intellivision default. Modern,
        // actively maintained. BIOS REQUIRED: `exec.bin` (4 KB) +
        // `grom.bin` (2 KB) in `<exe_dir>/system/`. The disc controller
        // gets mapped to libretro D-pad as 8-way in default
        // configuration; FreeIntv has core options to enable 16-direction
        // analog-stick mapping (Phase 2 polish).
        "intv" => "freeintv_libretro.dll",
        // O2EM — the libretro Magnavox Odyssey² / Videopac default.
        // Mature, light. BIOS REQUIRED: `o2rom.bin` for US Odyssey²
        // and/or `c52.bin` for EU Videopac G7000 + Videopac+ G7400.
        // The 47-key keyboard maps via libretro RETRO_DEVICE_KEYBOARD
        // (OA's keyboard passthrough mechanism handles it).
        "o2" => "o2em_libretro.dll",
        // FreeChaF — the libretro Fairchild Channel F default. Tiny
        // library, simple core. BIOS OPTIONAL: `sl31253.bin` /
        // `sl31254.bin` / `sl90025.bin` in `<exe_dir>/system/` (the
        // Channel F's BIOS handles the title menu); games run without
        // it via FreeChaF's internal BIOS replacement.
        "channelf" => "freechaf_libretro.dll",
        // Stella — the long-standing libretro Atari 2600 default.
        // Mature, comprehensive game compat (handles 50+ obscure
        // bankswitching schemes), light CPU. No widely-shipped
        // alternate in the libretro buildbot. BIOS-less — the 2600
        // had no BIOS at all (the cart ROM is the entire system
        // firmware), so there's nothing for the operator to install.
        "2600" => "stella_libretro.dll",
        // Atari800 — libretro core for the Atari 8-bit family (400/800/XL/
        // XE home computers + the 5200 console). Requires `5200.rom` BIOS
        // in `<exe_dir>/system/`. Pre-checked by check_atari5200_bios.
        "5200" => "atari800_libretro.dll",
        // PokeMini — the libretro Pokémon Mini default (in fact the only
        // option). Requires `bios.min` (4 KB) in `<exe_dir>/system/`.
        // Pre-checked by check_pokemini_bios.
        "pokemini" => "pokemini_libretro.dll",
        // mGBA — the libretro GBA gold standard. Mature, broad compat,
        // light CPU. Alternates via per-system Cores:
        // `vba_next_libretro.dll` (VBA-Next, lighter / less accurate),
        // `vbam_libretro.dll` (VBA-M). BIOS optional (`gba_bios.bin` in
        // `<exe_dir>/system/`) — a small number of games refuse to boot
        // without it but most run fine.
        "gba" => "mgba_libretro.dll",
        // Gambatte — the long-standing libretro Game Boy default,
        // covering both DMG (Game Boy) and CGB (Game Boy Color) via the
        // same .dll. ROM-header auto-detect picks the right hardware
        // mode. Mature, light CPU, broad compat. Alternates via
        // per-system Cores: `sameboy_libretro.dll` (more accurate,
        // slightly heavier), `tgbdual_libretro` (link-cable focus).
        // BIOS optional (`dmg_boot.bin` / `cgb_boot.bin` in
        // `<exe_dir>/system/`) — without it Gambatte just skips the
        // boot logo.
        "gb" => "gambatte_libretro.dll",
        // Genesis Plus GX — long-standing libretro multi-Sega core that
        // covers SMS + Game Gear + Mega Drive + Sega CD behind one .dll.
        // Picked over PicoDrive for SMS/GG because GPGX is the de-facto
        // libretro Sega 8-bit default and ships a single-install path for
        // operators who want every Sega cart-shape system at once. BIOS
        // (`bios.sms` / `bios.gg`) optional — games run fine without it,
        // just skip the era-correct boot logo. Per-system Cores override
        // can swap to picodrive_libretro for a lighter footprint.
        "sms" | "gamegear" => "genesis_plus_gx_libretro.dll",
        // Genesis Plus GX also drives Sega CD — same .dll already
        // shipping for SMS / Game Gear. Drop one .dll, light up four
        // Sega systems (SMS, GG, segacd, and genesis-via-override).
        // BIOS REQUIRED: `bios_CD_E.bin` / `bios_CD_U.bin` / `bios_CD_J.bin`
        // in `<exe_dir>/system/` — the regional BIOS for the launching
        // game. The shell pre-checks SHA-1 against canonical Genesis
        // Plus GX-blessed dumps (mirrors PCE-CD's check_pce_cd_bios)
        // and refuses missing/wrong content before retro_load_game so
        // the user gets a clean error toast instead of an access
        // violation deep in CD init. Alternates via per-system Cores:
        // picodrive_libretro (lighter, covers Sega CD too).
        "segacd" => "genesis_plus_gx_libretro.dll",
        // PicoDrive — the only mainstream libretro core with Sega 32X
        // support. Wraps PicoDrive's MD core + dedicated SH-2 emulation
        // for the 32X's twin RISC CPUs. No BIOS required for cart-only
        // 32X games (32X-CD games would also need the Sega CD BIOS,
        // but the cart path doesn't hit it). Extensions: .32x. No
        // widely-shipped alternate — Genesis Plus GX doesn't do 32X,
        // ClownMDEmu is MD-only.
        "sega32x" => "picodrive_libretro.dll",
        // Beetle Saturn — the Mednafen-derived libretro Saturn default.
        // Heavyweight: dual SH-2 + VDP1/VDP2 + 68k sound CPU emulation
        // is genuinely CPU-intensive (needs a decent modern host).
        // BIOS REQUIRED: a regional Saturn BIOS in `<exe_dir>/system/`
        // — `sega_101.bin` (JP v1.01) / `mpr-17933.bin` (US v1.00) /
        // `mpr-19367b.bin` (EU PAL) etc. The shell pre-checks SHA-1
        // against canonical Mednafen-blessed dumps (see
        // check_saturn_bios) and refuses missing/wrong content before
        // retro_load_game. Alternates via per-system Cores:
        // `kronos_libretro.dll` (lighter, less accurate),
        // `yabasanshiro_libretro.dll`.
        "saturn" => "mednafen_saturn_libretro.dll",
        // Beetle PSX HW — the hardware-accelerated Vulkan/OpenGL
        // Mednafen-derived libretro PSX default. Provides upscaling +
        // texture filtering + PGXP geometry correction; visually
        // premium choice for PSX. If the libretro core fails to obtain
        // a GL/Vulkan surface from our wgpu DX12 host on a particular
        // operator's machine, the per-system Cores dialog surfaces
        // `mednafen_psx_libretro.dll` (SW renderer, software-only —
        // pre-registered as a recommended catalog peer) as the
        // bulletproof fallback. Other alternates: `swanstation_libretro.dll`.
        // BIOS REQUIRED: regional PSX BIOSes in `<exe_dir>/system/`
        // (`scph5500.bin` JP / `scph5501.bin` US / `scph5502.bin` EU
        // v3.0; `scph7001.bin` / `scph7501.bin` US v4.x revisions etc.).
        // Pre-checked by `check_psx_bios`.
        "psx" => "mednafen_psx_hw_libretro.dll",
        // FBNeo (Final Burn Neo) — the canonical libretro Neo Geo core.
        // Handles both AES home + MVS arcade cart-shape via the same
        // .dll. ROM-sets land as .zip files in MAME-compatible format,
        // or .neo single-file dumps (No-Intro standard). BIOS REQUIRED:
        // `neogeo.zip` in `<exe_dir>/system/` — FBNeo reads the BIOS
        // ROMs out of the zip rather than expecting a single .bin file.
        // Pre-checked by check_neogeo_bios. No widely-shipped alternate
        // (MAME proper can also drive Neo Geo but at much higher CPU
        // cost; FBNeo is the libretro-buildbot default for the platform).
        "neogeo" => "fbneo_libretro.dll",
        // NeoCD — the dedicated libretro Neo Geo CD core. CD-shape
        // (multi-track CD images via .cue/.chd/.iso/.m3u/.ccd/.toc).
        // BIOS REQUIRED: `neocd_z.rom` (top-loader v1) or `neocd_t.rom`
        // (front-loader v2) in `<exe_dir>/system/`. The two BIOS
        // variants are functionally interchangeable for game launch;
        // the top-loader is more commonly tested. Pre-checked by
        // check_neocd_bios.
        "neocd" => "neocd_libretro.dll",
        // Beetle NeoPop — the canonical libretro Neo Geo Pocket / Color
        // core. Mednafen-derived; same upstream lineage as Beetle PCE
        // Fast / Beetle Saturn / Beetle PSX / Beetle VB / Beetle
        // WonderSwan / Beetle Lynx. Single .dll covers both NGP (mono)
        // and NGPC (color) — Beetle NeoPop auto-detects from ROM
        // header (same pattern as Gambatte covering DMG + CGB and
        // Beetle WonderSwan covering WS + WSC). No BIOS required.
        "ngp" => "mednafen_ngp_libretro.dll",
        // Virtual Jaguar — the canonical libretro Atari Jaguar core.
        // Cart-shape (`.j64` / `.jag`). BIOS optional — `jagboot.rom`
        // in `<exe_dir>/system/` enables the boot logo + a handful of
        // games that touch the BIOS, but most of the library boots
        // without it. No widely-shipped alternate libretro core.
        "jaguar" => "virtualjaguar_libretro.dll",
        // Opera (formerly 4DO) — the canonical libretro 3DO core.
        // CD-shape. BIOS REQUIRED: a regional/manufacturer 3DO BIOS in
        // `<exe_dir>/system/` (Panasonic FZ-1 `panafz1.bin` / FZ-10
        // `panafz10.bin` / GoldStar GDO-101M / Sanyo Try IMP-21J).
        // Pre-checked by check_3do_bios.
        "3do" => "opera_libretro.dll",
        // Beetle PC-FX — the Mednafen-derived libretro PC-FX core.
        // Shares the Mednafen-family lineage with Beetle PCE Fast
        // (pce-cd), Beetle Saturn, Beetle PSX, Beetle VB, Beetle
        // WonderSwan, Beetle Lynx, Beetle NeoPop. CD-shape. BIOS
        // REQUIRED: `pcfx.rom` in `<exe_dir>/system/` (single canonical
        // BIOS — PC-FX shipped Japan-only with no regional variants).
        // Pre-checked by check_pcfx_bios.
        "pcfx" => "mednafen_pcfx_libretro.dll",
        // Mupen64Plus-Next — the canonical libretro N64 default with
        // the GLideN64 video plugin. BIOS-free (CIC boot ROM emulated
        // internally). Heavy CPU + GPU. The N64 controller's analog
        // stick is the primary movement input for nearly every game;
        // Phase 0 plumbs analog axes via the new RETRO_DEVICE_ANALOG
        // dispatch in oa-libretro, so users with a gamepad's analog
        // stick get full movement. Keyboard-only users enable the core
        // option "Map d-pad to analog stick" to get digital arrow keys
        // → full-tilt analog input. Alternates via per-system Cores:
        // `parallel_n64_libretro.dll` (more accurate, heavier).
        "n64" => "mupen64plus_next_libretro.dll",
        // Dolphin — the canonical libretro GameCube + Wii core. One
        // .dll covers both via runtime auto-detect from disc container
        // shape (.iso/.gcm/.gcz/.rvz = GameCube; .wbfs = Wii; .iso
        // overlaps and is disambiguated by Dolphin's header check).
        // BIOS-free (Dolphin synthesizes firmware behavior). Heavy
        // CPU + GPU + 64-bit host required. Wii Remote / Nunchuk /
        // Classic Controller motion-controls deferred to Phase 2.5
        // alongside full per-system analog Bindings UI.
        "gamecube" => "dolphin_libretro.dll",
        // Flycast — the canonical libretro Dreamcast core. GD-ROM
        // media (.cdi / .gdi / .chd). BIOS REQUIRED: `dc_boot.bin`
        // (universal boot ROM, ~2 MB) + `dc_flash.bin` (regional
        // flash RAM, 256 KB, varies per region) in `<exe_dir>/system/`.
        // Pre-checked by `check_dreamcast_bios`; slots into the
        // CD-launch dispatch arm as the 8th CD-shape system.
        // Alternates via per-system Cores: `redream_libretro.dll`
        // (lighter, less accurate; not always packaged for libretro
        // buildbot).
        "dreamcast" => "flycast_libretro.dll",
        // PPSSPP — the canonical libretro PSP core. UMD-shape
        // (.iso/.cso/.pbp). BIOS-free (PPSSPP synthesizes firmware
        // behavior). Heavy CPU + GPU. PSP-1000/2000/3000 had a single
        // analog stick; PSP Go added a second but is rare. Analog
        // stick flows through the shared analog infra (gamepad
        // LeftStick → axes[0..2]).
        "psp" => "ppsspp_libretro.dll",
        // LRPS2 (PCSX2) — the canonical libretro PS2 core. DVD-shape
        // (most PS2 games shipped on DVD; some used CD). BIOS REQUIRED:
        // regional scph10000 (JP) / scph39001 (US fat) / scph70000
        // (US/EU slim) / scph90001 etc. in `<exe_dir>/system/`.
        // Pre-checked by `check_ps2_bios`; slots into the CD-launch
        // dispatch arm as the 9th CD-shape system. DualShock 2
        // controller — PSX-shape + dual analog sticks (analog infra).
        // Very heavy — needs a strong 64-bit host.
        "ps2" => "pcsx2_libretro.dll",
        // melonDS — the canonical libretro DS core. Cart-shape (.nds).
        // BIOS REQUIRED: 3 files — `bios7.bin` (ARM7 BIOS, 16 KB) +
        // `bios9.bin` (ARM9 BIOS, 4 KB) + `firmware.bin` (DS firmware
        // / settings, 256 KB). Pre-checked by `check_nds_bios` (multi-
        // file shape). Touch screen flows through the new POINTER
        // input infra (mouse-as-touch). Alternates via per-system
        // Cores: `desmume_libretro.dll`.
        "nds" => "melonds_libretro.dll",
        // Default — covers tg16, pce-cd, and any unknown system that ships
        // before the table is updated. The PCE Fast Mednafen build handles
        // both HuCard + CD if its BIOS is present in `<exe_dir>/system/`.
        _ => "mednafen_pce_fast_libretro.dll",
    }
}

/// Known-good SHA-1 hashes Mednafen tests PCE-CD BIOSes against. Wrong-content
/// BIOSes with the right filename typically cause the core to crash deep in
/// CD init with an access violation, so we pre-check + refuse rather than let
/// the user hit the unrelated-looking crash.
const PCE_BIOS_KNOWN_HASHES: &[(&str, &str, &str)] = &[
    // (filename,           SHA-1 uppercase,                                   description)
    // Pre-existing Mednafen-canonical hashes (retained — operators with
    // these BIOSes from earlier Mednafen-distributed sets continue to
    // validate as OkCanonical).
    ("syscard3.pce",  "1F8B161A2DB40DBA2079A87C10C0A3340B56ED3B", "US TurboGrafx-CD System Card v3.00 (Mednafen-canonical)"),
    ("syscard2.pce",  "056E3A8A7F3B7BE60EE6DEAEB0BAA67E1BA62B18", "US System Card v2.00 (Mednafen-canonical)"),
    ("syscard1.pce",  "6DCA8A0AFD0CB1C14CFFC1CFFEA34915CD496E44", "US System Card v1.00 (Mednafen-canonical)"),
    ("syscard3j.pce", "A01CE5F5A90F9F3A2E76EC3D34D8B03B9BD9E62A", "JP Super CD-ROM² System Card v3.00 (Mednafen-canonical)"),
    ("gexpress.pce",  "F8A06F08F8E7BF4D7117F1B22DA5074E0F49C2BC", "Games Express CD Card (Mednafen-canonical)"),
    // libretro-database/dat/System.dat canonical (no-intro-derived).
    // Same filenames, different content lineage — both validate.
    ("syscard1.pce",  "A39A66DA7DE6BA94AB84D04EEF7AFEEC7D4EE66A", "JP CD-ROM² System Card v1.00 (libretro-database)"),
    ("syscard2.pce",  "88DA02E2503F7C32810F5D93A34849D470742B6D", "JP CD-ROM² System Card v2.00 (libretro-database)"),
    ("syscard2u.pce", "2BEA3DAC98F84B2F2F469FA77EA720B8770D598D", "US TurboGrafx-CD System Card v2.00 (libretro-database)"),
    ("syscard3.pce",  "79F5FF55DD10187C7FD7B8DAAB0B3FFBD1F56A2C", "JP Super CD-ROM² System Card v3.00 (libretro-database)"),
    ("syscard3u.pce", "D02611D99921986147C753DF14C7349B31D71950", "US TurboGrafx-CD System Card v3.00 (libretro-database)"),
    ("gecard.pce",    "014881A959E045E00F4DB8F52955200865D40280", "Games Express CD Card (libretro-database)"),
    ("gexpress.pce",  "014881A959E045E00F4DB8F52955200865D40280", "Games Express CD Card (libretro-database alternate name)"),
];

enum BiosCheck {
    /// File's filename + content both match a known canonical entry.
    OkCanonical { name: String, sha1: String },
    /// File present but hash doesn't match anything known. Includes the
    /// file's actual SHA so the user can compare against expected.
    OkUnknownHash { name: String, sha1: String },
}

enum BiosError {
    Missing,
    Io(std::io::Error),
}

/// Scan `<system_dir>` for any PCE-CD BIOS file matching a known filename,
/// compute its SHA-1, classify against the known-good table. If the user has
/// content from a different known BIOS under the wrong filename, the warning
/// message names what they actually have so they can rename or replace.
fn check_pce_cd_bios(system_dir: &Path) -> Result<BiosCheck, BiosError> {
    use sha1::{Digest, Sha1};

    for (name, _, _) in PCE_BIOS_KNOWN_HASHES {
        let p = system_dir.join(name);
        if !p.is_file() {
            continue;
        }
        let bytes = std::fs::read(&p).map_err(BiosError::Io)?;
        let hash = Sha1::digest(&bytes);
        let sha_str = hash.iter().map(|b| format!("{:02X}", b)).collect::<String>();

        // Did this exact filename + hash combination match a canonical entry?
        let exact_match = PCE_BIOS_KNOWN_HASHES
            .iter()
            .any(|(n, h, _)| *n == *name && *h == sha_str);
        if exact_match {
            return Ok(BiosCheck::OkCanonical { name: name.to_string(), sha1: sha_str });
        }

        // Does the hash match some OTHER known BIOS (i.e. user has the wrong
        // file renamed)? If so the warning calls it out specifically.
        let renamed_as = PCE_BIOS_KNOWN_HASHES
            .iter()
            .find(|(_, h, _)| *h == sha_str)
            .map(|(actual_name, _, desc)| format!("{actual_name} — {desc}"));
        if let Some(actual) = renamed_as {
            log::warn!(
                "oa-shell: CD load — {} content matches a DIFFERENT BIOS: {}. Rename the file or fetch the correct {}.",
                name, actual, name
            );
        }

        return Ok(BiosCheck::OkUnknownHash { name: name.to_string(), sha1: sha_str });
    }
    Err(BiosError::Missing)
}

/// Known-good SHA-1 hashes for Sega CD / Mega-CD BIOSes. The Genesis Plus
/// GX libretro core (the default `segacd` core per `default_core_dll_for_system`)
/// requires one of three regional BIOS files in `<exe_dir>/system/` —
/// `bios_CD_E.bin` (EU) / `bios_CD_U.bin` (US) / `bios_CD_J.bin` (JP) —
/// matching the region of the disc the user is launching. Wrong content
/// at the right filename typically causes the core to fail CD-init with
/// an unrelated-looking access violation, so we pre-check + refuse rather
/// than let the user hit it.
///
/// The set covers the most-common Genesis Plus GX-tested dumps. Operators
/// with a BIOS dump whose SHA-1 doesn't match still get an OkUnknownHash
/// warn-level toast — the launch proceeds and the operator can validate
/// the BIOS against their dump's documented hash (cross-reference Redump
/// or the libretro wiki). Same pattern as `check_pce_cd_bios`.
const SEGA_CD_BIOS_KNOWN_HASHES: &[(&str, &str, &str)] = &[
    // (filename,         SHA-1 uppercase,                            description)
    // Hashes from libretro-database/dat/System.dat. Each of the three
    // regional Mega-CD / Sega CD BIOSes has a single canonical dump in
    // the no-intro set; Genesis Plus GX accepts these by filename and
    // region-checks the disc against them at launch.
    ("bios_CD_E.bin", "F891E0EA651E2232AF0C5C4CB46A0CAE2EE8F356", "EU Mega-CD v1.00 (PAL, canonical)"),
    ("bios_CD_J.bin", "4846F448160059A7DA0215A5DF12CA160F26DD69", "JP Mega-CD v1.00 (canonical)"),
    ("bios_CD_U.bin", "F4F315ADCEF9B8FEB0364C21AB7F0EAF5457F3ED", "US Sega CD v1.10 (canonical)"),
];

/// Scan `<system_dir>` for any Sega CD BIOS matching a known regional
/// filename, compute its SHA-1, classify against the known-good table.
/// Mirrors `check_pce_cd_bios` — same shape, same BiosCheck/BiosError
/// vocabulary so the launch path can branch by system without duplicating
/// the outer match shape.
fn check_sega_cd_bios(system_dir: &Path) -> Result<BiosCheck, BiosError> {
    use sha1::{Digest, Sha1};

    for (name, _, _) in SEGA_CD_BIOS_KNOWN_HASHES {
        let p = system_dir.join(name);
        if !p.is_file() {
            continue;
        }
        let bytes = std::fs::read(&p).map_err(BiosError::Io)?;
        let hash = Sha1::digest(&bytes);
        let sha_str = hash.iter().map(|b| format!("{:02X}", b)).collect::<String>();

        let exact_match = SEGA_CD_BIOS_KNOWN_HASHES
            .iter()
            .any(|(n, h, _)| *n == *name && *h == sha_str);
        if exact_match {
            return Ok(BiosCheck::OkCanonical { name: name.to_string(), sha1: sha_str });
        }

        // Hash matches a DIFFERENT canonical region (user renamed the
        // file). The launch can still proceed via OkUnknownHash, but the
        // log line names what the user actually has so they can rename.
        let renamed_as = SEGA_CD_BIOS_KNOWN_HASHES
            .iter()
            .find(|(_, h, _)| *h == sha_str)
            .map(|(actual_name, _, desc)| format!("{actual_name} — {desc}"));
        if let Some(actual) = renamed_as {
            log::warn!(
                "oa-shell: Sega CD load — {} content matches a DIFFERENT BIOS: {}. Rename the file or fetch the correct {}.",
                name, actual, name
            );
        }

        return Ok(BiosCheck::OkUnknownHash { name: name.to_string(), sha1: sha_str });
    }
    Err(BiosError::Missing)
}

/// Known-good SHA-1 hashes for Sega Saturn BIOSes. Beetle Saturn (the
/// default `saturn` core per `default_core_dll_for_system`) requires
/// one of the regional BIOS files in `<exe_dir>/system/`. Saturn region-
/// locks strictly — JP discs need a JP BIOS, US/EU discs need a US/EU
/// BIOS. The core fails CD init with an access violation if the BIOS
/// is missing or has the wrong content; we pre-check + refuse cleanly.
///
/// Set covers the most-common Mednafen-tested Saturn dumps across the
/// three regional + revision variants. Operators with less-common
/// Saturn BIOS dumps (e.g. ST-V arcade variant) get OkUnknownHash —
/// the launch proceeds with a warn-level toast.
///
/// All hashes sourced from libretro-database/dat/System.dat.
const SATURN_BIOS_KNOWN_HASHES: &[(&str, &str, &str)] = &[
    // (filename,           SHA-1 uppercase,                            description)
    // JP Saturn BIOS lineage. v1.00 shipped with the 1994 JP launch
    // hardware; v1.01 was the 1995 revision shipped with later units.
    ("sega_100.bin",    "2B8CB4F87580683EB4D760E4ED210813D667F0A2", "JP Saturn BIOS v1.00 (1994 launch)"),
    ("sega_100a.bin",   "3BB41FEB82838AB9A35601AC666DE5AACFD17A58", "JP Saturn BIOS v1.00a (revised 1994)"),
    ("sega_101.bin",    "DF94C5B4D47EB3CC404D88B33A8FDA237EAF4720", "JP Saturn BIOS v1.01 (1995 revision)"),
    ("sega1003.bin",    "7B23B53D62DE0F29A23E423D0FE751DFB469C2FA", "JP Saturn ST-V Compatible BIOS v1.003"),
    // US/EU Saturn BIOS — Sega used a single set of mpr-* mask ROM
    // dumps across the international Models with region byte
    // distinguishing US vs EU at the disc level.
    ("mpr-17933.bin",   "FAA8EA183A6D7BBE5D4E03BB1332519800D3FBC3", "US/EU Saturn BIOS v1.00a (most common)"),
    ("mpr-18100.bin",   "8A22710E09CE75F39625894366CAFE503ED1942D", "JP Saturn BIOS Special v1.01"),
    ("mpr-18811-mx.ic1","A67CD4F550751F8B91DE2B8B74528AB4E0C11C77", "JP Saturn Movie Card v1.10"),
    ("mpr-19367-mx.ic1","56C1B93DA6B660BF393FBF48CA47569000EF4047", "EU Saturn Movie Card v1.20"),
    // Generic "saturn_bios.bin" alias used by some retroarch / launchbox
    // distributions — same content as sega_100.bin.
    ("saturn_bios.bin", "2B8CB4F87580683EB4D760E4ED210813D667F0A2", "Generic Saturn BIOS (alias for sega_100.bin / JP v1.00)"),
    // Hitachi-OEM HiSaturn + JVC VSaturn BIOSes — uncommon but appear in
    // some collector dumps.
    ("hisaturn.bin",    "49D8493008FA715CA0C94D99817A5439D6F2C796", "Hitachi HiSaturn BIOS (1995 OEM clone)"),
    ("vsaturn.bin",     "4154E11959F3D5639B11D7902B3A393A99FB5776", "JVC V-Saturn BIOS (1995 OEM clone)"),
];

/// Scan `<system_dir>` for any Saturn BIOS matching a known regional
/// filename, compute its SHA-1, classify against the known-good table.
/// Same shape as `check_pce_cd_bios` and `check_sega_cd_bios`.
fn check_saturn_bios(system_dir: &Path) -> Result<BiosCheck, BiosError> {
    use sha1::{Digest, Sha1};

    for (name, _, _) in SATURN_BIOS_KNOWN_HASHES {
        let p = system_dir.join(name);
        if !p.is_file() {
            continue;
        }
        let bytes = std::fs::read(&p).map_err(BiosError::Io)?;
        let hash = Sha1::digest(&bytes);
        let sha_str = hash.iter().map(|b| format!("{:02X}", b)).collect::<String>();

        let exact_match = SATURN_BIOS_KNOWN_HASHES
            .iter()
            .any(|(n, h, _)| *n == *name && *h == sha_str);
        if exact_match {
            return Ok(BiosCheck::OkCanonical { name: name.to_string(), sha1: sha_str });
        }

        let renamed_as = SATURN_BIOS_KNOWN_HASHES
            .iter()
            .find(|(_, h, _)| *h == sha_str)
            .map(|(actual_name, _, desc)| format!("{actual_name} — {desc}"));
        if let Some(actual) = renamed_as {
            log::warn!(
                "oa-shell: Saturn load — {} content matches a DIFFERENT BIOS: {}. Rename the file or fetch the correct {}.",
                name, actual, name
            );
        }

        return Ok(BiosCheck::OkUnknownHash { name: name.to_string(), sha1: sha_str });
    }
    Err(BiosError::Missing)
}

/// Known-good SHA-1 hashes for Sony PlayStation BIOSes. Beetle PSX HW
/// (the default `psx` core per `default_core_dll_for_system`) requires
/// a regional BIOS in `<exe_dir>/system/` matching the disc's region.
/// PSX region-locking is enforced at the BIOS level — JP discs need
/// scph5500, US discs need scph5501, EU discs need scph5502.
///
/// Beetle PSX SW (the catalog peer alternate) uses the same BIOS file
/// set — both Mednafen-derived PSX cores share the same canonical
/// hash list.
const PSX_BIOS_KNOWN_HASHES: &[(&str, &str, &str)] = &[
    // (filename,         SHA-1 uppercase,                            description)
    // All hashes sourced from libretro-database/dat/System.dat.
    //
    // v1.x family — 1994-1995 SCPH-1000 / SCPH-3000 / SCPH-3500 launch units.
    ("scph1000.bin", "343883A7B555646DA8CEE54AADD2795B6E7DD070", "JP PSX BIOS v1.0 (SCPH-1000, 1994 launch)"),
    ("scph1001.bin", "10155D8D6E6E832D6EA66DB9BC098321FB5E8EBF", "US PSX BIOS v2.2 (SCPH-1001, common North America)"),
    ("scph1002.bin", "20B98F3D80F11CBF5A7BFD0779B0E63760ECC62C", "EU PSX BIOS v2.0 (SCPH-1002, PAL)"),
    ("scph3000.bin", "B06F4A861F74270BE819AA2A07DB8D0563A7CC4E", "JP PSX BIOS v2.1 (SCPH-3000)"),
    ("scph3500.bin", "E38466A4BA8005FBA7E9E3C7B9EFEBA7205BEE3F", "JP PSX BIOS v2.2 (SCPH-3500)"),
    ("scph5000.bin", "E340DB2696274DDA5FDC25E434A914DB71E8B02B", "JP PSX BIOS v3.0 (SCPH-5000)"),
    // v3.0 family — the most-commonly-installed regional set. Sony
    // shipped these in the 1995 launch / first-revision hardware.
    ("scph5500.bin", "B05DEF971D8EC59F346F2D9AC21FB742E3EB6917", "JP PSX BIOS v3.0 (SCPH-5500, 1995)"),
    ("scph5501.bin", "0555C6FAE8906F3F09BAF5988F00E55F88E9F30B", "US PSX BIOS v3.0 (SCPH-5501, 1995, most common NA)"),
    ("scph5502.bin", "F6BC2D1F5EB6593DE7D089C425AC681D6FFFD3F0", "EU PSX BIOS v3.0 (SCPH-5502, PAL)"),
    // v4.x family — 1997-1998 revisions.
    ("scph7001.bin", "14DF4F6C1E367CE097C11DEAE21566B4FE5647A9", "US PSX BIOS v4.1 (SCPH-7001, 1997)"),
    ("scph7002.bin", "8D5DE56A79954F29E9006929BA3FED9B6A418C1D", "EU PSX BIOS v4.1 (SCPH-7002, 1997)"),
    ("scph7003.bin", "0555C6FAE8906F3F09BAF5988F00E55F88E9F30B", "US PSX BIOS v3.0 (SCPH-7003, alias for SCPH-5501 content)"),
    ("scph7502.bin", "8D5DE56A79954F29E9006929BA3FED9B6A418C1D", "EU PSX BIOS v4.1 (SCPH-7502, alias for SCPH-7002 content)"),
    // PSone (slim) BIOS — 2000-era. SCPH-100x / SCPH-101 are the PSone model line.
    ("scph100.bin",  "339A48F4FCF63E10B5B867B8C93CFD40945FAF6C", "JP PSone BIOS v4.3 (SCPH-100)"),
    ("scph101.bin",  "DCFFE16BD90A723499AD46C641424981338D8378", "US PSone BIOS v4.5 (SCPH-101)"),
    ("scph102.bin",  "BEB0AC693C0DC26DAF5665B3314DB81480FA5C7C", "EU PSone BIOS v4.4 (SCPH-102, variant A)"),
    // PSP PS1 emulator BIOSes — present in PSone-on-PSP dumps.
    ("psxonpsp660.bin", "96880D1CA92A016FF054BE5159BB06FE03CB4E14", "Sony PSP PS1 emulator BIOS v6.60"),
    ("ps1_rom.bin",     "C40146361EB8CF670B19FDC9759190257803CAB7", "Sony PSP-style PS1 ROM (alternate dump)"),
];

/// Scan `<system_dir>` for any PSX BIOS matching a known regional
/// filename, compute its SHA-1, classify against the known-good table.
/// Same shape as `check_pce_cd_bios` / `check_sega_cd_bios` / `check_saturn_bios`.
fn check_psx_bios(system_dir: &Path) -> Result<BiosCheck, BiosError> {
    use sha1::{Digest, Sha1};

    for (name, _, _) in PSX_BIOS_KNOWN_HASHES {
        let p = system_dir.join(name);
        if !p.is_file() {
            continue;
        }
        let bytes = std::fs::read(&p).map_err(BiosError::Io)?;
        let hash = Sha1::digest(&bytes);
        let sha_str = hash.iter().map(|b| format!("{:02X}", b)).collect::<String>();

        let exact_match = PSX_BIOS_KNOWN_HASHES
            .iter()
            .any(|(n, h, _)| *n == *name && *h == sha_str);
        if exact_match {
            return Ok(BiosCheck::OkCanonical { name: name.to_string(), sha1: sha_str });
        }

        let renamed_as = PSX_BIOS_KNOWN_HASHES
            .iter()
            .find(|(_, h, _)| *h == sha_str)
            .map(|(actual_name, _, desc)| format!("{actual_name} — {desc}"));
        if let Some(actual) = renamed_as {
            log::warn!(
                "oa-shell: PSX load — {} content matches a DIFFERENT BIOS: {}. Rename the file or fetch the correct {}.",
                name, actual, name
            );
        }

        return Ok(BiosCheck::OkUnknownHash { name: name.to_string(), sha1: sha_str });
    }
    Err(BiosError::Missing)
}

/// Known-good SHA-1 hashes for SNK Neo Geo CD BIOSes. NeoCD (the
/// default `neocd` core) requires one of the regional/model BIOS files
/// in `<exe_dir>/system/`. SNK shipped three CD hardware models —
/// front-loader CD, top-loader CDZ, and CDT — each with its own BIOS.
/// All canonical hashes sourced from libretro-database/dat/System.dat.
///
/// Universe BIOS CD (community-modified BIOS adding region toggle +
/// cheat menu) is also recognized so operators using it don't trip the
/// OkUnknownHash path.
const NEOCD_BIOS_KNOWN_HASHES: &[(&str, &str, &str)] = &[
    // (filename,          SHA-1 uppercase,                            description)
    ("neocd.bin",      "7BB26D1E5D1E930515219CB18BCDE5B7B23E2EDA", "Neo Geo CD boot ROM (default libretro name)"),
    ("neocd_f.rom",    "A5F4A7A627B3083C979F6EBE1FABC5D2DF6D083B", "Neo Geo CD Front-loader BIOS (CD-F)"),
    ("neocd_sf.rom",   "4A94719EE5D0E3F2B981498F70EFC1B8F1CEF325", "Neo Geo CD Front-loader BIOS (CD-SF revision)"),
    ("neocd_t.rom",    "CC92B54A18A8BFF6E595AABE8E5C360BA9E62EB5", "Neo Geo CDT BIOS (front-loader CDT model)"),
    ("neocd_st.rom",   "19729B51BDAB60C42AAFEF6E20EA9234C7EB8410", "Neo Geo CDT BIOS (CD-ST revision)"),
    ("neocd_z.rom",    "B0F1C4FA8D4492A04431805F6537138B842B549F", "Neo Geo CDZ BIOS (top-loader)"),
    ("neocd_sz.rom",   "6A947457031DD3A702A296862446D7485AA89DBB", "Neo Geo CDZ BIOS (CD-SZ revision)"),
    ("front-sp1.bin",  "53BC1F283CDF00FA2EFBB79F2E36D4C8038D743A", "Neo Geo CD Front-loader system program"),
    ("top-sp1.bin",    "235F4D1D74364415910F73C10AE5482D90B4274F", "Neo Geo CD Top-loader system program"),
    ("000-lo.lo",      "5992277DEBADEB64D1C1C64B0A92D9293EAF7E4A", "Neo Geo CD LO-ROM (shared with cart AES/MVS)"),
    ("uni-bioscd.rom", "5142F205912869B673A71480C5828B1EAED782A8", "Universe BIOS CD (community-modified, region toggle + cheats)"),
];

/// Scan `<system_dir>` for any Neo Geo CD BIOS matching a known
/// regional/model filename, compute its SHA-1, classify against the
/// known-good table. Same shape as the other check_*_bios functions —
/// slots into the CD-launch BIOS dispatch arm next to pce-cd / segacd
/// / saturn / psx.
fn check_neocd_bios(system_dir: &Path) -> Result<BiosCheck, BiosError> {
    use sha1::{Digest, Sha1};

    for (name, _, _) in NEOCD_BIOS_KNOWN_HASHES {
        let p = system_dir.join(name);
        if !p.is_file() {
            continue;
        }
        let bytes = std::fs::read(&p).map_err(BiosError::Io)?;
        let hash = Sha1::digest(&bytes);
        let sha_str = hash.iter().map(|b| format!("{:02X}", b)).collect::<String>();

        let exact_match = NEOCD_BIOS_KNOWN_HASHES
            .iter()
            .any(|(n, h, _)| *n == *name && *h == sha_str);
        if exact_match {
            return Ok(BiosCheck::OkCanonical { name: name.to_string(), sha1: sha_str });
        }

        let renamed_as = NEOCD_BIOS_KNOWN_HASHES
            .iter()
            .find(|(_, h, _)| *h == sha_str)
            .map(|(actual_name, _, desc)| format!("{actual_name} — {desc}"));
        if let Some(actual) = renamed_as {
            log::warn!(
                "oa-shell: Neo Geo CD load — {} content matches a DIFFERENT BIOS: {}. Rename or fetch the correct {}.",
                name, actual, name
            );
        }

        return Ok(BiosCheck::OkUnknownHash { name: name.to_string(), sha1: sha_str });
    }
    Err(BiosError::Missing)
}

/// Known-good SHA-1 hashes for 3DO Interactive Multiplayer BIOSes.
/// Opera (the default `3do` core) requires a regional/manufacturer
/// BIOS in `<exe_dir>/system/`. Multiple hardware variants shipped:
/// Panasonic FZ-1 (1993 launch), Panasonic FZ-10 (1994 revision),
/// GoldStar GDO-101M (1995), Sanyo Try IMP-21J (Japan).
const THREEDO_BIOS_KNOWN_HASHES: &[(&str, &str, &str)] = &[
    // (filename,                   SHA-1 uppercase,                            description)
    // All hashes sourced from libretro-database/dat/System.dat.
    //
    // Panasonic FZ-1 (1993 launch model) — most-common dump.
    ("panafz1.bin",            "34BF189111295F74D7B7DFC1F304D98B8D36325A", "Panasonic FZ-1 v1.x (1993 launch, most common)"),
    ("panafz1-kanji.bin",      "ACD39A8FEE1B9D2950D5AB447846C11FB31AF63E", "Panasonic FZ-1 with Kanji ROM (Japan)"),
    ("panafz1j.bin",           "EC7EC62D60EC0459A14ED56EBC66761EF3C80EFC", "Panasonic FZ-1 Japan-region BIOS"),
    ("panafz1j-kanji.bin",     "884515605EE243577AB20767EF8C1A7368E4E407", "Panasonic FZ-1 Japan with Kanji ROM"),
    ("panafz1j-norsa.bin",     "A417587AE3B0B8EF00C830920C21AF8BEE88E419", "Panasonic FZ-1 Japan, RSA-stripped"),
    // Panasonic FZ-10 (1994 revision).
    ("panafz10.bin",           "3C912300775D1AD730DC35757E279C274C0ACAAD", "Panasonic FZ-10 v1.x (1994 revision)"),
    ("panafz10-norsa.bin",     "F05E642322C03694F06A809C0B90FC27AC73C002", "Panasonic FZ-10, RSA-stripped"),
    ("panafz10e-anvil.bin",    "A900371F0CDCDC03F79557F11D406FD71251A5FD", "Panasonic FZ-10 EU 'Anvil' v1.02d"),
    ("panafz10e-anvil-norsa.bin", "2765C7B4557CC838B32567D2428D088980295159", "Panasonic FZ-10 EU 'Anvil', RSA-stripped"),
    ("panafz10ja-anvil-kanji.bin", "2E857B957803D0331FD229328DF01F3FFAB69EEE", "Panasonic FZ-10 JA 'Anvil' with Kanji ROM"),
    // Third-party 3DO licensees.
    ("goldstar.bin",           "C4A2E5336F77FB5F743DE1EEA2CDA43675EE2DE7", "GoldStar GDO-101M (1995)"),
    ("sanyotry.bin",            "B01C53DA256DDE43FFEC4AD3FC3ADFA8D635E943", "Sanyo TRY IMP-21J (Japan)"),
    // 3DO M2 arcade variant (rare — for completeness).
    ("3do_arcade_saot.bin",    "520D3D1B5897800AF47F92EFD2444A26B7A7DEAD", "3DO Arcade SAOT firmware (M2 / arcade variant)"),
];

/// Scan `<system_dir>` for any 3DO BIOS matching a known
/// regional/manufacturer filename. Same shape as the other check_*_bios
/// functions; slots into the CD-launch BIOS dispatch arm.
fn check_3do_bios(system_dir: &Path) -> Result<BiosCheck, BiosError> {
    use sha1::{Digest, Sha1};

    for (name, _, _) in THREEDO_BIOS_KNOWN_HASHES {
        let p = system_dir.join(name);
        if !p.is_file() {
            continue;
        }
        let bytes = std::fs::read(&p).map_err(BiosError::Io)?;
        let hash = Sha1::digest(&bytes);
        let sha_str = hash.iter().map(|b| format!("{:02X}", b)).collect::<String>();

        let exact_match = THREEDO_BIOS_KNOWN_HASHES
            .iter()
            .any(|(n, h, _)| *n == *name && *h == sha_str);
        if exact_match {
            return Ok(BiosCheck::OkCanonical { name: name.to_string(), sha1: sha_str });
        }

        let renamed_as = THREEDO_BIOS_KNOWN_HASHES
            .iter()
            .find(|(_, h, _)| *h == sha_str)
            .map(|(actual_name, _, desc)| format!("{actual_name} — {desc}"));
        if let Some(actual) = renamed_as {
            log::warn!(
                "oa-shell: 3DO load — {} content matches a DIFFERENT BIOS: {}. Rename or fetch the correct {}.",
                name, actual, name
            );
        }

        return Ok(BiosCheck::OkUnknownHash { name: name.to_string(), sha1: sha_str });
    }
    Err(BiosError::Missing)
}

/// Known-good SHA-1 hashes for NEC PC-FX BIOSes. Beetle PC-FX (the
/// default `pcfx` core) requires `pcfx.rom` in `<exe_dir>/system/`.
/// PC-FX was Japan-only with a single canonical BIOS — no regional
/// variants to track.
const PCFX_BIOS_KNOWN_HASHES: &[(&str, &str, &str)] = &[
    // (filename,        SHA-1 uppercase,                            description)
    // All hashes sourced from libretro-database/dat/System.dat.
    ("pcfx.rom",     "1A77FD83E337F906AECAB27A1604DB064CF10074", "NEC PC-FX BIOS v1.00 (canonical, Japan-only platform)"),
    ("pcfxbios.bin", "1A77FD83E337F906AECAB27A1604DB064CF10074", "NEC PC-FX BIOS v1.00 (alternate naming, same content)"),
    ("pcfxv101.bin", "8B662F7548078BE52A871565E19511CCCA28C5C8", "NEC PC-FX BIOS v1.01 (later revision)"),
    ("pcfxga.rom",   "A9372202A5DB302064C994FCDA9B24D29BB1B41C", "NEC PC-FXGA / FX-1 BIOS (PC-FX expansion card)"),
    ("fx-scsi.rom",  "65482A23AC5C10A6095AEE1DB5824CCA54EAD6E5", "NEC PC-FX SCSI BIOS (expansion accessory)"),
];

/// Scan `<system_dir>` for the PC-FX BIOS. Same shape as the other
/// check_*_bios functions; slots into the CD-launch BIOS dispatch arm.
fn check_pcfx_bios(system_dir: &Path) -> Result<BiosCheck, BiosError> {
    use sha1::{Digest, Sha1};

    for (name, _, _) in PCFX_BIOS_KNOWN_HASHES {
        let p = system_dir.join(name);
        if !p.is_file() {
            continue;
        }
        let bytes = std::fs::read(&p).map_err(BiosError::Io)?;
        let hash = Sha1::digest(&bytes);
        let sha_str = hash.iter().map(|b| format!("{:02X}", b)).collect::<String>();

        let exact_match = PCFX_BIOS_KNOWN_HASHES
            .iter()
            .any(|(n, h, _)| *n == *name && *h == sha_str);
        if exact_match {
            return Ok(BiosCheck::OkCanonical { name: name.to_string(), sha1: sha_str });
        }

        return Ok(BiosCheck::OkUnknownHash { name: name.to_string(), sha1: sha_str });
    }
    Err(BiosError::Missing)
}

/// Known-good SHA-1 hashes for Sega Dreamcast BIOSes. Flycast (the
/// default `dreamcast` core) requires:
///   `dc_boot.bin` — boot ROM (2 MB, universal across regions)
///   `dc_flash.bin` — flash RAM (256 KB, region-specific: US/JP/EU)
/// in `<exe_dir>/system/`. The boot ROM is the same for all regions;
/// the flash file carries region-locking + clock-region defaults.
const DREAMCAST_BIOS_KNOWN_HASHES: &[(&str, &str, &str)] = &[
    // (filename,      SHA-1 uppercase,                            description)
    // Hashes sourced from libretro-database/dat/System.dat. Flycast
    // requires `dc_boot.bin` + `dc_flash.bin`. Flash is region-coded
    // but libretro-database tracks a single canonical factory dump —
    // operators sometimes ship regional variants; those land on the
    // OkUnknownHash path (launch proceeds with warn-toast).
    ("dc_boot.bin",  "8951D1BB219AB2FF8583033D2119C899CC81F18C", "Dreamcast Boot ROM (canonical, universal across regions)"),
    ("boot.bin",     "8951D1BB219AB2FF8583033D2119C899CC81F18C", "Dreamcast Boot ROM (alternate naming, same content)"),
    ("dc_flash.bin", "94D44D7F9529EC1642BA3771ED3C5F756D5BC872", "Dreamcast Flash ROM (canonical factory dump)"),
    ("flash.bin",    "94D44D7F9529EC1642BA3771ED3C5F756D5BC872", "Dreamcast Flash ROM (alternate naming, same content)"),
];

/// Scan `<system_dir>` for any Dreamcast BIOS file. Same shape as the
/// other check_*_bios functions; slots into the CD-launch BIOS dispatch
/// arm as the 8th CD-shape system.
fn check_dreamcast_bios(system_dir: &Path) -> Result<BiosCheck, BiosError> {
    use sha1::{Digest, Sha1};

    for (name, _, _) in DREAMCAST_BIOS_KNOWN_HASHES {
        let p = system_dir.join(name);
        if !p.is_file() {
            continue;
        }
        let bytes = std::fs::read(&p).map_err(BiosError::Io)?;
        let hash = Sha1::digest(&bytes);
        let sha_str = hash.iter().map(|b| format!("{:02X}", b)).collect::<String>();

        let exact_match = DREAMCAST_BIOS_KNOWN_HASHES
            .iter()
            .any(|(n, h, _)| *n == *name && *h == sha_str);
        if exact_match {
            return Ok(BiosCheck::OkCanonical { name: name.to_string(), sha1: sha_str });
        }

        let renamed_as = DREAMCAST_BIOS_KNOWN_HASHES
            .iter()
            .find(|(_, h, _)| *h == sha_str)
            .map(|(actual_name, _, desc)| format!("{actual_name} — {desc}"));
        if let Some(actual) = renamed_as {
            log::warn!(
                "oa-shell: Dreamcast load — {} content matches a DIFFERENT BIOS: {}. Rename or fetch the correct {}.",
                name, actual, name
            );
        }

        return Ok(BiosCheck::OkUnknownHash { name: name.to_string(), sha1: sha_str });
    }
    Err(BiosError::Missing)
}

/// Known-good SHA-1 hashes for Sony PlayStation 2 BIOSes. LRPS2
/// (PCSX2) requires a regional BIOS file in `<exe_dir>/system/`.
/// Multiple hardware revisions shipped across the PS2's 2000-2013
/// lifespan; the most commonly-tested set covers the launch / fat /
/// slim eras across JP / US / EU.
const PS2_BIOS_KNOWN_HASHES: &[(&str, &str, &str)] = &[
    // (filename,                  SHA-1 uppercase,                            description)
    // All hashes sourced from libretro-database/dat/System.dat. PCSX2 /
    // LRPS2 uses the `ps2-XXXX-YYYYMMDD.bin` naming convention; many
    // operators have files in the legacy `scphXXXXX.bin` style. Both
    // naming conventions are accepted here — content-by-hash matching
    // catches mis-renames either way.
    //
    // SCPH-style aliases (legacy naming most BIOS distributions use).
    ("scph10000.bin", "AEA061E6E263FDCC1C4FDBD68553EF78DAE74263", "JP PS2 fat v1.00 (SCPH-10000, 2000-03-04 launch; matches ps2-0100j)"),
    ("scph39001.bin", "F9A5D629A036B99128F7CB530C6E3CA016E9C8B7", "US PS2 fat v1.60 (SCPH-39001; matches ps2-0160a-20020207)"),
    ("scph70000.bin", "FBD54BFC020AF34008B317DCB80B812DD29B3759", "JP PS2 slim v2.30 (SCPH-70000; matches ps2-0230j-20080220)"),
    ("scph77001.bin", "8361D615CC895962E0F0838489337574DBDC9173", "US PS2 slim v2.20 (SCPH-77001; matches ps2-0220a-20060905)"),
    ("scph90001.bin", "B9CB5775AF29CD4D1EC5521E8231F8B6636E2E44", "EU PS2 slim v2.50 (SCPH-90001; matches ps2-0250e-20100415)"),
    // PCSX2-style names (libretro-database canonical).
    ("ps2-0100j-20000117.bin",  "AEA061E6E263FDCC1C4FDBD68553EF78DAE74263", "JP PS2 fat v1.00 (2000-01-17 build, SCPH-10000)"),
    ("ps2-0101j-20000217.bin",  "916E02431BCD73140504DA3355C9598143B77E11", "JP PS2 fat v1.01 (2000-02-17)"),
    ("ps2-0110a-20000727.bin",  "20F6CE6693CF97E9494F8F0227F2B7988FFAF961", "US PS2 fat v1.10 (2000-07-27)"),
    ("ps2-0120e-20000902.bin",  "274C05FEC654913A3F698D4B0D592085866A2CBD", "EU PS2 fat v1.20 (2000-09-02)"),
    ("ps2-0120j-20001027-185015.bin", "E481079ECA752225555F0C26D14C9D0F94D9A8E9", "JP PS2 fat v1.20 (2000-10-27)"),
    ("ps2-0150a-20001228.bin",  "5AF5B5077D84A9C037EBE12BFAB8A38B31D8A543", "US PS2 fat v1.50 (2000-12-28)"),
    ("ps2-0150e-20001228.bin",  "E22EF231FAF3661EDD92F2EE449A71297C82A092", "EU PS2 fat v1.50 (2000-12-28)"),
    ("ps2-0150j-20010118.bin",  "D6F365A0F07CD04ED28108E6EC5076E2F81E5F72", "JP PS2 fat v1.50 (2001-01-18)"),
    ("ps2-0160a-20010427.bin",  "7331A40B4B4FEB1B3F0F77B013B6D38483577BAA", "US PS2 fat v1.60 (2001-04-27)"),
    ("ps2-0160a-20020207.bin",  "F9A5D629A036B99128F7CB530C6E3CA016E9C8B7", "US PS2 fat v1.60 (2002-02-07; SCPH-39001 equiv)"),
    ("ps2-0160e-20020319.bin",  "BFF2902BD0CE9729A060581132541E9FD1A9FAB6", "EU PS2 fat v1.60 (2002-03-19)"),
    ("ps2-0160j-20020426.bin",  "003628C137DAE577FF3B04B93CA1787B0C944702", "JP PS2 fat v1.60 (2002-04-26)"),
    ("ps2-0170e-20030227.bin",  "AD15BD7EABD5BD81BA011516A5BE44947D6641AA", "EU PS2 fat v1.70 (2003-02-27)"),
    ("ps2-0170a-20030325.bin",  "D269D1ED513227F3EF7133C76CF1B3A64F97B15D", "US PS2 fat v1.70 (2003-03-25)"),
    ("ps2-0170j-20030206.bin",  "D812AC65C357D392396CA9EDEE812DC41BED8BDE", "JP PS2 fat v1.70 (2003-02-06)"),
    ("ps2-0180j-20031028.bin",  "AA4A35C14EE342CF7A03B1DDE294CA10E64889E1", "JP PS2 fat v1.80 (2003-10-28)"),
    ("ps2-0190a-20030623.bin",  "C74D92A2952A2912B6698CBCF7742ADAC8F784D3", "US PS2 fat v1.90 (2003-06-23)"),
    ("ps2-0190e-20030623.bin",  "18B9BA833C469C4683676CC20DA5124080D980BB", "EU PS2 fat v1.90 (2003-06-23)"),
    ("ps2-0190j-20030623.bin",  "6A6ECFE6C10E42EFF1CA056349DEF799B5629067", "JP PS2 fat v1.90 (2003-06-23)"),
    ("ps2-0200a-20040614.bin",  "7A62E5F48603582707E9898EB055EA3EAEE50D4C", "US PS2 fat v2.00 (2004-06-14)"),
    ("ps2-0200e-20040614.bin",  "434BC0B4EB4827DA0773EC0795AADC5162569A07", "EU PS2 fat v2.00 (2004-06-14)"),
    ("ps2-0200j-20040614.bin",  "224AB5704AB719EDEB05CA1D835812252C97C1B3", "JP PS2 fat v2.00 (2004-06-14, SCPH-50000-series)"),
    ("ps2-0210j-20040917.bin",  "BBB1AF3085E77599691EC430D147810157DA934F", "JP PS2 fat v2.10 (2004-09-17)"),
    ("ps2-0220a-20050620.bin",  "48D0445DFFD1E879C7AE752C5166EC3101921555", "US PS2 fat v2.20 (2005-06-20)"),
    ("ps2-0220e-20050620.bin",  "929A85E974FAF4B40D0A7785023B758402C43BD9", "EU PS2 fat v2.20 (2005-06-20)"),
    ("ps2-0220j-20050620.bin",  "7FFA75D142CB8EEEA6C777DBCF263143655275D5", "JP PS2 fat v2.20 (2005-06-20)"),
    ("ps2-0220a-20060905.bin",  "8361D615CC895962E0F0838489337574DBDC9173", "US PS2 slim v2.20 (2006-09-05, SCPH-77001)"),
    ("ps2-0220e-20060905.bin",  "DA5AACEAD2FB55807D6D4E70B1F10F4FDCFD3281", "EU PS2 slim v2.20 (2006-09-05)"),
    ("ps2-0220j-20060905.bin",  "3BAF847C1C217AA71AC6D298389C88EDB3DB32E2", "JP PS2 slim v2.20 (2006-09-05)"),
    ("ps2-0230a-20080220.bin",  "F9229FE159D0353B9F0632F3FDC66819C9030458", "US PS2 slim v2.30 (2008-02-20, SCPH-79001)"),
    ("ps2-0230e-20080220.bin",  "9915B5BA56798F4027AC1BD8D10ABE0C1C9C326A", "EU PS2 slim v2.30 (2008-02-20)"),
    ("ps2-0230j-20080220.bin",  "FBD54BFC020AF34008B317DCB80B812DD29B3759", "JP PS2 slim v2.30 (2008-02-20, SCPH-90000)"),
    ("ps2-0250e-20100415.bin",  "B9CB5775AF29CD4D1EC5521E8231F8B6636E2E44", "EU PS2 slim v2.50 (2010-04-15, SCPH-90001)"),
    ("ps2-0250j-20100415.bin",  "4B5EF16B67E3B523D28ED2406106CB80470A06D0", "JP PS2 slim v2.50 (2010-04-15)"),
];

/// Scan `<system_dir>` for any PS2 BIOS matching a known revision.
/// Same shape as the other check_*_bios functions; slots into the
/// CD-launch BIOS dispatch arm as the 9th CD-shape system.
fn check_ps2_bios(system_dir: &Path) -> Result<BiosCheck, BiosError> {
    use sha1::{Digest, Sha1};

    for (name, _, _) in PS2_BIOS_KNOWN_HASHES {
        let p = system_dir.join(name);
        if !p.is_file() {
            continue;
        }
        let bytes = std::fs::read(&p).map_err(BiosError::Io)?;
        let hash = Sha1::digest(&bytes);
        let sha_str = hash.iter().map(|b| format!("{:02X}", b)).collect::<String>();

        let exact_match = PS2_BIOS_KNOWN_HASHES
            .iter()
            .any(|(n, h, _)| *n == *name && *h == sha_str);
        if exact_match {
            return Ok(BiosCheck::OkCanonical { name: name.to_string(), sha1: sha_str });
        }

        return Ok(BiosCheck::OkUnknownHash { name: name.to_string(), sha1: sha_str });
    }
    Err(BiosError::Missing)
}

/// Known-good SHA-1 hashes for Nintendo DS BIOSes. melonDS requires
/// **three files** in `<exe_dir>/system/`:
///   `bios7.bin` (ARM7 BIOS, 16 KB) — coprocessor BIOS handling audio +
///                                    wireless + touch screen
///   `bios9.bin` (ARM9 BIOS, 4 KB) — main CPU BIOS handling boot + graphics
///   `firmware.bin` (DS firmware, 256 KB) — user settings + WiFi config
const NDS_BIOS_KNOWN_HASHES: &[(&str, &str, &str)] = &[
    // (filename,        SHA-1 uppercase,                            description)
    // All hashes sourced from libretro-database/dat/System.dat.
    ("bios7.bin",    "24F67BDEA115A2C847C8813A262502EE1607B7DF", "DS ARM7 BIOS (16 KB, coprocessor for audio/wireless/touch)"),
    ("bios9.bin",    "BFAAC75F101C135E32E2AAF541DE6B1BE4C8C62D", "DS ARM9 BIOS (4 KB, main CPU boot + graphics)"),
    ("firmware.bin", "CFE072921EE3FB93F688743F8BEEF89043C3E9AD", "DS Firmware (256 KB, region + user settings)"),
];

/// Scan `<system_dir>` for the three required DS BIOS files. Unlike
/// the single-file BIOS checks (which short-circuit on the first
/// matching filename), the DS check requires ALL THREE files to be
/// present — `bios7.bin` + `bios9.bin` + `firmware.bin` together
/// constitute the DS BIOS. Returns OkCanonical only if all three
/// hash-match; returns OkUnknownHash if all three exist but at least
/// one has an unexpected SHA-1; returns Missing if any are absent.
fn check_nds_bios(system_dir: &Path) -> Result<BiosCheck, BiosError> {
    use sha1::{Digest, Sha1};

    let mut all_canonical = true;
    let mut last_name: String = String::new();
    let mut last_sha: String = String::new();

    for (name, expected_sha, _) in NDS_BIOS_KNOWN_HASHES {
        let p = system_dir.join(name);
        if !p.is_file() {
            // Any of the three missing → BIOS is incomplete.
            return Err(BiosError::Missing);
        }
        let bytes = std::fs::read(&p).map_err(BiosError::Io)?;
        let hash = Sha1::digest(&bytes);
        let sha_str = hash.iter().map(|b| format!("{:02X}", b)).collect::<String>();
        if &sha_str != *expected_sha {
            all_canonical = false;
        }
        last_name = name.to_string();
        last_sha = sha_str;
    }

    // All three files exist; report the canonical (all-three-match) or
    // unknown-hash (at least one mismatches) status. Use the last file
    // checked as the representative entry in the OkCanonical /
    // OkUnknownHash payload.
    if all_canonical {
        Ok(BiosCheck::OkCanonical { name: "bios7.bin + bios9.bin + firmware.bin".to_string(), sha1: "all canonical".to_string() })
    } else {
        Ok(BiosCheck::OkUnknownHash { name: last_name, sha1: last_sha })
    }
}

/// Check for Neo Geo cart BIOS — `neogeo.zip` in `<exe_dir>/system/`.
/// Unlike the CD-shape systems (whose BIOSes are single .bin files
/// with stable SHA-1s), the Neo Geo cart BIOS is a multi-ROM .zip
/// whose content SHA-1 varies by MAME revision + Universe BIOS
/// presence. Phase 0 ships an existence-only check — operator gets a
/// clean "missing neogeo.zip" error when absent, FBNeo handles the
/// content validation internally if present.
///
/// Content peek: we open the zip and look for the canonical Neo Geo
/// BIOS files. The minimum-viable set is:
///   - one System BIOS: `sp-s2.sp1` (most common) OR an older `sp-s.sp1`
///     / `sp1.jipan.1024` variant OR a Universe BIOS (`uni-bios_*.rom`)
///   - the Z80 sound program: `sm1.sm1`
///   - the LO-ROM sprite zoom table: `000-lo.lo`
///
/// All three present → OkCanonical. Zip exists but is missing one or
/// more → OkUnknownHash (launch still proceeds — some sparse BIOS sets
/// still boot FBNeo). Zip absent → Missing.
fn check_neogeo_bios(system_dir: &Path) -> Result<BiosCheck, BiosError> {
    let p = system_dir.join("neogeo.zip");
    if !p.is_file() {
        return Err(BiosError::Missing);
    }
    // Peek inside the zip without extracting. The archive crate's
    // `list_rom_contents` filters by extension allowlist (designed for
    // game-content scanning), which would drop the BIOS files since
    // `.sp1` / `.sm1` / `.lo` aren't playable extensions. Walk the zip
    // directly with the `zip` crate here so we see every inner file.
    let file = std::fs::File::open(&p).map_err(BiosError::Io)?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| {
        BiosError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    })?;
    // Build a lowercase set of basenames so case-quirky dumps (some
    // tools uppercase filenames) match.
    let mut inner_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    for i in 0..zip.len() {
        if let Ok(entry) = zip.by_index(i) {
            if entry.is_dir() {
                continue;
            }
            // Strip any path component — some zips wrap files inside a
            // top-level dir, but the canonical Neo Geo set is flat.
            let name = entry.name().to_string();
            let basename = std::path::Path::new(&name)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&name)
                .to_ascii_lowercase();
            inner_names.insert(basename);
        }
    }
    // Acceptable System BIOS filenames (any one satisfies the system-BIOS
    // requirement). Order matters only for diagnostic output: prefer
    // sp-s2.sp1 as the canonical match.
    const SYSTEM_BIOS_CANDIDATES: &[&str] = &[
        "sp-s2.sp1",        // canonical (Asia/USA AES + MVS, ~98% of dumps)
        "sp-s.sp1",         // older System BIOS revision
        "sp1.jipan.1024",   // Japan AES
        "sp1-1v1.bin",      // Universe BIOS naming variant
        "sp-1v1_3db8c.bin", // Universe BIOS 3 / 4 variant
        "uni-bios_4_0.rom", // Universe BIOS 4.0
        "uni-bios_3_3.rom", // Universe BIOS 3.3
        "uni-bios_3_2.rom", // Universe BIOS 3.2
        "uni-bios_3_1.rom", // Universe BIOS 3.1
        "uni-bios_3_0.rom", // Universe BIOS 3.0
        "uni-bios_2_3.rom", // Universe BIOS 2.3
    ];
    let system_bios = SYSTEM_BIOS_CANDIDATES
        .iter()
        .find(|name| inner_names.contains(**name));
    let has_sm1 = inner_names.contains("sm1.sm1");
    let has_lo = inner_names.contains("000-lo.lo");

    if let (Some(sys), true, true) = (system_bios, has_sm1, has_lo) {
        // Tag the active BIOS variant in the diagnostic so the operator
        // can confirm at a glance which flavour is loaded (stock AES/MVS
        // vs. Universe BIOS). Helps debugging "why isn't this game
        // booting?" — Unibios skips region locks + adds a CD-mode toggle,
        // so behaviour differs from stock.
        let flavour = neogeo_bios_flavour(sys);
        Ok(BiosCheck::OkCanonical {
            name: "neogeo.zip".to_string(),
            sha1: format!("CONTENT PEEK: {sys} [{flavour}] + sm1.sm1 + 000-lo.lo present"),
        })
    } else {
        // Zip exists but content is incomplete — surface what's missing
        // so the operator can see exactly which file to fetch.
        let missing: Vec<&str> = [
            (system_bios.is_none(), "system BIOS (sp-s2.sp1 or Universe BIOS)"),
            (!has_sm1, "sm1.sm1"),
            (!has_lo, "000-lo.lo"),
        ]
        .into_iter()
        .filter_map(|(m, label)| if m { Some(label) } else { None })
        .collect();
        Ok(BiosCheck::OkUnknownHash {
            name: "neogeo.zip".to_string(),
            sha1: format!("CONTENT PEEK: missing {}", missing.join(" + ")),
        })
    }
}

/// Classify a Neo Geo system-BIOS filename as either stock factory BIOS
/// (Asia / USA / Japan AES + MVS variants) or community-developed
/// Universe BIOS (Unibios). FBNeo accepts both shapes; the difference
/// matters at runtime — Unibios adds a built-in soft-reset / region /
/// CD-mode menu and skips region locks, so behaviour can diverge from
/// stock for the same game.
///
/// Returned tag lands in the `OkCanonical` diagnostic so operators
/// can see at a glance which BIOS flavour is active without grepping
/// filenames against documentation.
fn neogeo_bios_flavour(filename: &str) -> &'static str {
    if filename.starts_with("uni-bios") || filename.starts_with("sp1-1v1") || filename.starts_with("sp-1v1") {
        "Universe BIOS"
    } else {
        "stock AES/MVS"
    }
}

/// Known-good SHA-1 hashes for ColecoVision BIOS. BlueMSX (the default
/// `coleco` core per `default_core_dll_for_system`) requires the
/// `colecovision.rom` system BIOS in `<exe_dir>/system/`. All hashes
/// sourced from libretro-database/dat/System.dat.
const COLECO_BIOS_KNOWN_HASHES: &[(&str, &str, &str)] = &[
    // (filename,            SHA-1 uppercase,                            description)
    ("colecovision.rom", "45BEDC4CBDEAC66C7DF59E9E599195C778D86A92", "ColecoVision BIOS (8 KB, canonical)"),
    ("coleco.rom",       "45BEDC4CBDEAC66C7DF59E9E599195C778D86A92", "ColecoVision BIOS (alternate naming, same content)"),
];

/// Scan `<system_dir>` for the ColecoVision BIOS. Same shape as the
/// CD-shape BIOS checks; slots into the cart-shape BIOS dispatch arm
/// alongside Neo Geo / NDS.
fn check_coleco_bios(system_dir: &Path) -> Result<BiosCheck, BiosError> {
    use sha1::{Digest, Sha1};

    for (name, _, _) in COLECO_BIOS_KNOWN_HASHES {
        let p = system_dir.join(name);
        if !p.is_file() {
            continue;
        }
        let bytes = std::fs::read(&p).map_err(BiosError::Io)?;
        let hash = Sha1::digest(&bytes);
        let sha_str = hash.iter().map(|b| format!("{:02X}", b)).collect::<String>();

        let exact_match = COLECO_BIOS_KNOWN_HASHES
            .iter()
            .any(|(n, h, _)| *n == *name && *h == sha_str);
        if exact_match {
            return Ok(BiosCheck::OkCanonical { name: name.to_string(), sha1: sha_str });
        }

        return Ok(BiosCheck::OkUnknownHash { name: name.to_string(), sha1: sha_str });
    }
    Err(BiosError::Missing)
}

/// Known-good SHA-1 hashes for Mattel Intellivision BIOS files. FreeIntv
/// (the default `intv` core) requires BOTH `exec.bin` (8 KB executive
/// ROM, the main CPU BIOS) AND `grom.bin` (2 KB graphics ROM, the
/// character set + graphics primitives) in `<exe_dir>/system/`. All
/// hashes sourced from libretro-database/dat/System.dat.
const INTV_BIOS_KNOWN_HASHES: &[(&str, &str, &str)] = &[
    // (filename,   SHA-1 uppercase,                            description)
    ("exec.bin", "5A65B922B562CB1F57DAB51B73151283F0E20C7A", "Intellivision Executive ROM (8 KB)"),
    ("grom.bin", "F9608BB4AD1CFE3640D02844C7AD8E0BCD974917", "Intellivision Graphics ROM (2 KB)"),
];

/// Scan `<system_dir>` for both required Intellivision BIOS files.
/// Multi-file check — same shape as `check_nds_bios`: requires ALL
/// listed files to be present. OkCanonical only if every file hash
/// matches; OkUnknownHash if all files exist but at least one mismatches;
/// Missing if any are absent.
fn check_intv_bios(system_dir: &Path) -> Result<BiosCheck, BiosError> {
    use sha1::{Digest, Sha1};

    let mut all_canonical = true;
    let mut last_name: String = String::new();
    let mut last_sha: String = String::new();

    for (name, expected_sha, _) in INTV_BIOS_KNOWN_HASHES {
        let p = system_dir.join(name);
        if !p.is_file() {
            return Err(BiosError::Missing);
        }
        let bytes = std::fs::read(&p).map_err(BiosError::Io)?;
        let hash = Sha1::digest(&bytes);
        let sha_str = hash.iter().map(|b| format!("{:02X}", b)).collect::<String>();
        if &sha_str != *expected_sha {
            all_canonical = false;
        }
        last_name = name.to_string();
        last_sha = sha_str;
    }

    if all_canonical {
        Ok(BiosCheck::OkCanonical { name: "exec.bin + grom.bin".to_string(), sha1: "all canonical".to_string() })
    } else {
        Ok(BiosCheck::OkUnknownHash { name: last_name, sha1: last_sha })
    }
}

/// Known-good SHA-1 hashes for Magnavox Odyssey² / Philips Videopac BIOS.
/// O2EM (the default `o2` core) requires `o2rom.bin` in
/// `<exe_dir>/system/`. Three regional variants exist (US Odyssey²,
/// EU Videopac G7400, FR Philips Jopac). All hashes sourced from
/// libretro-database/dat/System.dat.
const O2_BIOS_KNOWN_HASHES: &[(&str, &str, &str)] = &[
    // (filename,   SHA-1 uppercase,                            description)
    ("o2rom.bin",  "B2E1955D957A475DE2411770452EFF4EA19F4CEE", "Magnavox Odyssey² BIOS (US, canonical)"),
    ("c52.bin",    "A6120AED50831C9C0D95DBDF707820F601D9452E", "Philips Videopac G7400 BIOS variant (C52)"),
    ("g7400.bin",  "5130243429B40B01A14E1304D0394B8459A6FBAE", "Philips Videopac+ G7400 BIOS"),
    ("jopac.bin",  "54B8D2C1317628DE51A85FC1C424423A986775E4", "Philips Jopac BIOS (FR Videopac variant)"),
];

/// Scan `<system_dir>` for an Odyssey²/Videopac BIOS. Same shape as
/// the CD-shape BIOS checks; slots into the cart-shape BIOS dispatch arm.
fn check_o2_bios(system_dir: &Path) -> Result<BiosCheck, BiosError> {
    use sha1::{Digest, Sha1};

    for (name, _, _) in O2_BIOS_KNOWN_HASHES {
        let p = system_dir.join(name);
        if !p.is_file() {
            continue;
        }
        let bytes = std::fs::read(&p).map_err(BiosError::Io)?;
        let hash = Sha1::digest(&bytes);
        let sha_str = hash.iter().map(|b| format!("{:02X}", b)).collect::<String>();

        let exact_match = O2_BIOS_KNOWN_HASHES
            .iter()
            .any(|(n, h, _)| *n == *name && *h == sha_str);
        if exact_match {
            return Ok(BiosCheck::OkCanonical { name: name.to_string(), sha1: sha_str });
        }

        return Ok(BiosCheck::OkUnknownHash { name: name.to_string(), sha1: sha_str });
    }
    Err(BiosError::Missing)
}

/// Known-good SHA-1 hashes for Fairchild Channel F BIOS files. FreeChaF
/// (the default `channelf` core) requires `sl31253.bin` + `sl31254.bin`
/// (the original 1976 launch ROM pair) in `<exe_dir>/system/`. The
/// optional `sl90025.bin` ships with the 1978 Channel F II revision —
/// recognized but not required. All hashes sourced from libretro-database
/// /dat/System.dat.
const CHANNELF_BIOS_KNOWN_HASHES: &[(&str, &str, &str)] = &[
    // (filename,    SHA-1 uppercase,                            description)
    ("sl31253.bin", "81193965A374D77B99B4743D317824B53C3E3C78", "Channel F Cartridge ROM 1 (1976 launch)"),
    ("sl31254.bin", "8F70D1B74483BA3A37E86CF16C849D601A8C3D2C", "Channel F Cartridge ROM 2 (1976 launch)"),
    ("sl90025.bin", "759E2ED31FBDE4A2D8DAF8B9F3E0DFFEBC90DAE2", "Channel F II Cartridge ROM (1978 revision, optional)"),
];

/// Scan `<system_dir>` for the Channel F BIOS files. Multi-file check
/// requiring sl31253.bin + sl31254.bin (the launch pair); sl90025.bin
/// is optional and only validated if present.
fn check_channelf_bios(system_dir: &Path) -> Result<BiosCheck, BiosError> {
    use sha1::{Digest, Sha1};

    // Required pair: sl31253.bin + sl31254.bin
    let required = ["sl31253.bin", "sl31254.bin"];
    let mut all_canonical = true;
    let mut last_name: String = String::new();
    let mut last_sha: String = String::new();

    for name in &required {
        let p = system_dir.join(name);
        if !p.is_file() {
            return Err(BiosError::Missing);
        }
        let bytes = std::fs::read(&p).map_err(BiosError::Io)?;
        let hash = Sha1::digest(&bytes);
        let sha_str = hash.iter().map(|b| format!("{:02X}", b)).collect::<String>();
        let expected = CHANNELF_BIOS_KNOWN_HASHES
            .iter()
            .find(|(n, _, _)| n == name)
            .map(|(_, h, _)| *h)
            .unwrap_or("");
        if sha_str != expected {
            all_canonical = false;
        }
        last_name = name.to_string();
        last_sha = sha_str;
    }

    // Optional sl90025.bin — validated if present, ignored if not.
    let opt = system_dir.join("sl90025.bin");
    if opt.is_file() {
        let bytes = std::fs::read(&opt).map_err(BiosError::Io)?;
        let hash = Sha1::digest(&bytes);
        let sha_str = hash.iter().map(|b| format!("{:02X}", b)).collect::<String>();
        if sha_str != "759E2ED31FBDE4A2D8DAF8B9F3E0DFFEBC90DAE2" {
            all_canonical = false;
            last_name = "sl90025.bin".to_string();
            last_sha = sha_str;
        }
    }

    if all_canonical {
        Ok(BiosCheck::OkCanonical { name: "sl31253.bin + sl31254.bin".to_string(), sha1: "all canonical".to_string() })
    } else {
        Ok(BiosCheck::OkUnknownHash { name: last_name, sha1: last_sha })
    }
}

/// Known-good SHA-1 hashes for Atari 5200 BIOS. Pre-staged for the
/// `atari5200` onboarding (planned). a5200 / atari800 cores require
/// `5200.rom` in `<exe_dir>/system/`. Sourced from libretro-database
/// /dat/System.dat.
const ATARI5200_BIOS_KNOWN_HASHES: &[(&str, &str, &str)] = &[
    // (filename,   SHA-1 uppercase,                            description)
    ("5200.rom", "6AD7A1E8C9FAD486FBEC9498CB48BF5BC3ADC530", "Atari 5200 SuperSystem BIOS (2 KB, canonical)"),
];

/// Pre-staged 5200 BIOS check (used once the `5200` SystemId variant
/// lands in oa-core + the default core .dll is registered).
fn check_atari5200_bios(system_dir: &Path) -> Result<BiosCheck, BiosError> {
    use sha1::{Digest, Sha1};

    for (name, _, _) in ATARI5200_BIOS_KNOWN_HASHES {
        let p = system_dir.join(name);
        if !p.is_file() {
            continue;
        }
        let bytes = std::fs::read(&p).map_err(BiosError::Io)?;
        let hash = Sha1::digest(&bytes);
        let sha_str = hash.iter().map(|b| format!("{:02X}", b)).collect::<String>();

        let exact_match = ATARI5200_BIOS_KNOWN_HASHES
            .iter()
            .any(|(n, h, _)| *n == *name && *h == sha_str);
        if exact_match {
            return Ok(BiosCheck::OkCanonical { name: name.to_string(), sha1: sha_str });
        }

        return Ok(BiosCheck::OkUnknownHash { name: name.to_string(), sha1: sha_str });
    }
    Err(BiosError::Missing)
}

/// Known-good SHA-1 hashes for Pokémon Mini BIOS. Pre-staged for the
/// `pokemini` onboarding (planned). PokeMini libretro core requires
/// `bios.min` in `<exe_dir>/system/`. Sourced from libretro-database
/// /dat/System.dat.
const POKEMINI_BIOS_KNOWN_HASHES: &[(&str, &str, &str)] = &[
    // (filename,   SHA-1 uppercase,                            description)
    ("bios.min", "DAAD4113713ED776FBD47727762BCA81BA74915F", "Pokémon Mini boot ROM (4 KB, canonical)"),
];

/// Pre-staged PokeMini BIOS check (used once the `pokemini` SystemId
/// variant lands in oa-core).
fn check_pokemini_bios(system_dir: &Path) -> Result<BiosCheck, BiosError> {
    use sha1::{Digest, Sha1};

    for (name, _, _) in POKEMINI_BIOS_KNOWN_HASHES {
        let p = system_dir.join(name);
        if !p.is_file() {
            continue;
        }
        let bytes = std::fs::read(&p).map_err(BiosError::Io)?;
        let hash = Sha1::digest(&bytes);
        let sha_str = hash.iter().map(|b| format!("{:02X}", b)).collect::<String>();

        let exact_match = POKEMINI_BIOS_KNOWN_HASHES
            .iter()
            .any(|(n, h, _)| *n == *name && *h == sha_str);
        if exact_match {
            return Ok(BiosCheck::OkCanonical { name: name.to_string(), sha1: sha_str });
        }

        return Ok(BiosCheck::OkUnknownHash { name: name.to_string(), sha1: sha_str });
    }
    Err(BiosError::Missing)
}

/// Map a ROM file path to a save-state directory name: take the filename
/// stem (no extension) and replace any path-unsafe characters with `_`.
/// `Bonk's Adventure (USA).pce` → `Bonk's Adventure (USA)` on most filesystems.
fn sanitize_stem(rom_path: &str) -> String {
    let stem = Path::new(rom_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    let sanitized: String = stem
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    if sanitized.trim().is_empty() {
        "unknown".to_string()
    } else {
        sanitized
    }
}

fn slot_path(app_data_dir: &Path, stem: &str, slot: u32) -> PathBuf {
    app_data_dir.join("saves").join(stem).join(format!("slot-{}.bin", slot))
}

/// RetroArch-parity slice 5 — apply every enabled cheat to the core's
/// memory regions. Called after each NORMAL / FAST-FORWARD / SLOW-MO
/// run_frame so trainers + locked-value cheats stay in effect frame-
/// over-frame. Width-3 / width-other rows silently no-op (defensive
/// against corrupted persisted rows). Skipped during TAS replay — cheats
/// modifying memory would diverge from the recorded inputs' outcome.
fn apply_cheats(core: &mut dyn oa_core::Core, cheats: &[library_db::Cheat]) {
    for c in cheats.iter().filter(|c| c.enabled && c.kind == "memory_poke") {
        let Some(region_id) = oa_core::MemoryRegionId::parse(&c.region) else { continue };
        let Some(mem) = core.memory_region_mut(region_id) else { continue };
        let offset = c.offset as usize;
        match c.width {
            1 => {
                if offset < mem.len() {
                    mem[offset] = c.value as u8;
                }
            }
            2 => {
                if offset + 2 <= mem.len() {
                    let bytes = (c.value as u16).to_le_bytes();
                    mem[offset..offset + 2].copy_from_slice(&bytes);
                }
            }
            4 => {
                if offset + 4 <= mem.len() {
                    let bytes = (c.value as u32).to_le_bytes();
                    mem[offset..offset + 4].copy_from_slice(&bytes);
                }
            }
            _ => {}
        }
    }
}

/// RetroArch-parity slice 3 — write the current framebuffer to a PNG
/// screenshot under `appData/screenshots/<stem>/<timestamp>.png`. Reuses
/// the same png encode path as save-state thumbnails. Returns the
/// absolute path on success so the success toast can name the file.
fn write_screenshot(
    app_data_dir: &Path,
    stem: &str,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> std::io::Result<PathBuf> {
    let dir = app_data_dir.join("screenshots").join(stem);
    std::fs::create_dir_all(&dir)?;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let path = dir.join(format!("{timestamp}.png"));
    write_thumbnail(&path, width, height, rgba)?;
    Ok(path)
}

/// Encode an RGBA8 framebuffer to a PNG file. Full-size (no downsample).
fn write_thumbnail(path: &Path, width: u32, height: u32, rgba: &[u8]) -> std::io::Result<()> {
    if width == 0 || height == 0 || rgba.is_empty() {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "empty framebuffer"));
    }
    let file = std::fs::File::create(path)?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    writer
        .write_image_data(rgba)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellMode {
    TwoWindow,
    SingleWindow,
}

impl ShellMode {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "single-window" | "single" => Some(Self::SingleWindow),
            "two-window" | "two" => Some(Self::TwoWindow),
            _ => None,
        }
    }

    /// Resolve the active shell mode for this launch.
    /// Precedence: `OA_SHELL_MODE` env (dev override) > `appDataDir/shell.json` > two-window default.
    fn resolve(app_data_dir: &Path) -> Self {
        if let Ok(env) = std::env::var("OA_SHELL_MODE") {
            if let Some(mode) = Self::parse(&env) {
                log::info!("oa-shell: shell_mode from OA_SHELL_MODE = {}", mode.as_str());
                return mode;
            }
            log::warn!("oa-shell: OA_SHELL_MODE={env} unrecognized; falling back to file/default");
        }
        if let Some(mode) = read_shell_pref(app_data_dir) {
            log::info!("oa-shell: shell_mode from shell.json = {}", mode.as_str());
            return mode;
        }
        log::info!("oa-shell: shell_mode default = two-window");
        Self::TwoWindow
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::TwoWindow => "two-window",
            Self::SingleWindow => "single-window",
        }
    }
}

fn read_shell_pref(app_data_dir: &Path) -> Option<ShellMode> {
    let path = app_data_dir.join("shell.json");
    let raw = std::fs::read_to_string(&path).ok()?;
    let parsed: std::collections::HashMap<String, String> = serde_json::from_str(&raw).ok()?;
    ShellMode::parse(parsed.get("shellMode")?.as_str())
}

fn write_shell_pref(app_data_dir: &Path, mode: ShellMode) -> std::io::Result<()> {
    std::fs::create_dir_all(app_data_dir)?;
    let path = app_data_dir.join("shell.json");
    let body = serde_json::to_string_pretty(&serde_json::json!({
        "shellMode": mode.as_str(),
    }))
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, body)
}

/// Read the persisted per-system core preferences from `appDataDir/cores.json`.
/// Format: `{ "tg16": "mednafen_pce_fast_libretro.dll", ... }`. Missing or
/// malformed yields an empty map.
fn read_cores_pref(app_data_dir: &Path) -> std::collections::BTreeMap<String, String> {
    let path = app_data_dir.join("cores.json");
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return Default::default(),
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

fn write_cores_pref(
    app_data_dir: &Path,
    prefs: &std::collections::BTreeMap<String, String>,
) -> std::io::Result<()> {
    std::fs::create_dir_all(app_data_dir)?;
    let path = app_data_dir.join("cores.json");
    let body = serde_json::to_string_pretty(prefs)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, body)
}

/// Resolve the cores directory the same way `run_emu_render` does: next to
/// the .exe. Used by the list_cores command.
fn resolve_cores_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("cores")
}

/// Read the persisted audio device name from `appDataDir/audio.json`. `None`
/// (missing file, malformed JSON, or `"deviceName": null`) means "use the
/// system default device".
fn read_audio_pref(app_data_dir: &Path) -> Option<String> {
    let path = app_data_dir.join("audio.json");
    let raw = std::fs::read_to_string(&path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&raw).ok()?;
    parsed.get("deviceName").and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn write_audio_pref(app_data_dir: &Path, device: Option<&str>) -> std::io::Result<()> {
    std::fs::create_dir_all(app_data_dir)?;
    let path = app_data_dir.join("audio.json");
    let body = serde_json::to_string_pretty(&serde_json::json!({ "deviceName": device }))
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, body)
}

/// Abstracts the "game-bearing window" that `launch_rom` focuses and
/// `set_window_mode` fullscreens. In two-window mode this is the native game
/// Window; in single-window mode it's the same WebviewWindow that hosts the
/// library UI on top of the wgpu surface.
enum ShellWindow {
    TwoWindow { game: Arc<tauri::Window> },
    SingleWindow { window: Arc<tauri::WebviewWindow> },
}

#[derive(Debug, Clone, Copy)]
enum WindowModeRequest {
    Windowed,
    Borderless,
    Fullscreen,
}

impl WindowModeRequest {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "windowed"   => Some(Self::Windowed),
            "borderless" => Some(Self::Borderless),
            "fullscreen" => Some(Self::Fullscreen),
            _ => None,
        }
    }
}

impl ShellWindow {
    fn focus_game(&self) {
        match self {
            ShellWindow::TwoWindow { game } => {
                game.show().ok();
                game.unminimize().ok();
                game.set_focus().ok();
            }
            ShellWindow::SingleWindow { window } => {
                window.set_focus().ok();
            }
        }
    }

    fn set_fullscreen(&self, fs: bool) -> tauri::Result<()> {
        match self {
            ShellWindow::TwoWindow { game } => game.set_fullscreen(fs),
            ShellWindow::SingleWindow { window } => window.set_fullscreen(fs),
        }
    }

    fn set_decorations(&self, d: bool) -> tauri::Result<()> {
        match self {
            ShellWindow::TwoWindow { game } => game.set_decorations(d),
            ShellWindow::SingleWindow { window } => window.set_decorations(d),
        }
    }

    /// Returns (position, size) of the monitor the window is currently on.
    fn current_monitor_bounds(&self) -> Option<(tauri::PhysicalPosition<i32>, tauri::PhysicalSize<u32>)> {
        let monitor = match self {
            ShellWindow::TwoWindow { game } => game.current_monitor().ok().flatten()?,
            ShellWindow::SingleWindow { window } => window.current_monitor().ok().flatten()?,
        };
        Some((*monitor.position(), *monitor.size()))
    }

    fn set_position(&self, pos: tauri::PhysicalPosition<i32>) -> tauri::Result<()> {
        match self {
            ShellWindow::TwoWindow { game } => game.set_position(pos),
            ShellWindow::SingleWindow { window } => window.set_position(pos),
        }
    }

    fn set_size(&self, size: tauri::PhysicalSize<u32>) -> tauri::Result<()> {
        match self {
            ShellWindow::TwoWindow { game } => game.set_size(size),
            ShellWindow::SingleWindow { window } => window.set_size(size),
        }
    }

    fn available_monitors(&self) -> tauri::Result<Vec<tauri::Monitor>> {
        match self {
            ShellWindow::TwoWindow { game } => game.available_monitors(),
            ShellWindow::SingleWindow { window } => window.available_monitors(),
        }
    }

    fn apply_window_mode(&self, mode: WindowModeRequest, monitor_index: Option<u32>) -> tauri::Result<()> {
        match mode {
            WindowModeRequest::Windowed => {
                self.set_fullscreen(false)?;
                self.set_decorations(true)?;
            }
            WindowModeRequest::Borderless => {
                self.set_fullscreen(false)?;
                self.set_decorations(false)?;
                let bounds = monitor_index
                    .and_then(|idx| {
                        self.available_monitors()
                            .ok()
                            .and_then(|ms| ms.into_iter().nth(idx as usize))
                            .map(|m| (*m.position(), *m.size()))
                    })
                    .or_else(|| self.current_monitor_bounds());
                if let Some((pos, size)) = bounds {
                    self.set_position(pos)?;
                    self.set_size(size)?;
                }
            }
            WindowModeRequest::Fullscreen => {
                self.set_decorations(true)?;
                self.set_fullscreen(true)?;
            }
        }
        Ok(())
    }
}

struct AppState {
    emu_tx: Mutex<mpsc::Sender<EmuCommand>>,
    shell_window: ShellWindow,
    shell_mode: ShellMode,
    app_data_dir: PathBuf,
    /// Lifted to AppState so Tauri commands (set_ui_intercepting) can flip it.
    /// The emu thread holds a clone and reads it each frame to gate input.
    /// Default `false` — set to `true` while a modal/binding-capture is active.
    ui_intercepting: Arc<AtomicBool>,
    /// Phase 6 Cross-system slice 3 — "Game focus" mode. When `true`, OA
    /// hotkeys (F1/F2/F3/F5/F6/F7/F8/F12/Esc/digits/Backspace) stop firing
    /// inside the emu thread so the libretro keyboard-passthrough pump can
    /// hand those keys to the core unchallenged. Toggled from the Tools
    /// menu checkbox + the Scroll Lock / Ctrl+G hotkeys via
    /// `set_game_focus`. Default `false` (OA owns its hotkeys).
    /// Independent of [`ui_intercepting`]: a modal still suppresses
    /// input regardless of Game-focus state.
    game_focus: Arc<AtomicBool>,
    /// Tracks the entry_id of the currently-loaded game. Set by launch_rom,
    /// consumed by unload_rom for archive::cleanup_temp(). None when no ROM
    /// is loaded or when the loaded ROM isn't archived.
    active_archive_entry_id: Arc<Mutex<Option<String>>>,
    /// Phase 4 slice B — published rewind ring stats. Writer = emu thread
    /// (updated after every capture / pop / scrub op). Reader = Tauri
    /// commands like `get_rewind_state`. Mutex is uncontended in practice.
    rewind_state: Arc<Mutex<SharedRewindState>>,
    /// Phase 4 slice C — published TAS recording/replay state. Same
    /// shape + ownership as `rewind_state`.
    tas_state: Arc<Mutex<SharedTasState>>,
    /// Phase 4 slice D — published video capture state. Same shape +
    /// ownership pattern as the others.
    video_state: Arc<Mutex<SharedVideoState>>,
    /// Phase 4 slice E — per-frame memory snapshot for the inspector
    /// UI. Writer = emu thread; readers = Tauri commands.
    memory_snapshot: Arc<Mutex<MemorySnapshot>>,
    /// Emu-thread perf stats — fps, frame count, audio counters. Writer
    /// = emu thread (updated every ~30 frames in the run loop); readers
    /// = `get_perf_stats` Tauri command driving the Performance HUD.
    perf_stats: Arc<Mutex<SharedPerfStats>>,
    /// Debug-console logger handle — ring buffer + current session
    /// file path. Used by `get_recent_logs` / `get_log_file_path` /
    /// `reveal_logs_folder` / `log_from_frontend`.
    logger_handle: logger::LoggerHandle,
    /// Phase 3 slice D — name of the currently-selected shader preset.
    /// Updated by `set_shader_preset`; read by the shader presets watcher
    /// to re-apply the same preset when its TOML changes on disk. None
    /// when no preset has been set this session (no game loaded yet).
    active_shader_preset: Arc<Mutex<Option<String>>>,
    /// Phase 3 slice D — held to keep the OS watcher alive for the
    /// app lifetime. Dropping it stops the watcher.
    #[allow(dead_code)]
    shader_presets_watcher: Option<shader_presets_watcher::ShaderPresetsWatcher>,
    /// RetroArch-parity slice — cached disc-control snapshot, refreshed
    /// by the emu thread after LoadRom + every successful eject/swap.
    /// `None` = no disc-control interface registered (HuCard / cart) OR
    /// no core loaded. Tauri's `get_disc_state` reads from here without
    /// touching the libretro singleton.
    disc_state: Arc<Mutex<Option<oa_core::DiscInfo>>>,
    /// RetroArch-parity slice — in-flight cheat search session. Holds
    /// the previous snapshot + the current candidate list. None when no
    /// search is active. Lives outside the emu thread because the
    /// memory snapshot is already refreshed there every frame; the
    /// search command just reads from `memory_snapshot`.
    cheat_search: Arc<Mutex<Option<cheat_search::CheatSearchSession>>>,
    /// Resolved direct-launch configuration from CLI args / OA_ROM env.
    /// `None` = library mode (default zero-arg invocation). The frontend
    /// reads this via `get_direct_launch_config` on boot to decide whether
    /// to hide library chrome and auto-launch a ROM.
    direct_launch: Option<cli::DirectLaunchConfig>,
}

/// Hint for the emu thread's startup core load. When `Some`, run_emu_render
/// bootstraps with the target system's core instead of the historical tg16
/// default — direct-launch knows the target system upfront so we can skip
/// the wasted load-PCE-CD-then-drop-and-reload-for-real-system dance.
/// `None` = library mode (no target known, bootstrap with tg16).
#[derive(Clone, Debug)]
struct BootstrapHint {
    system_id: String,
    core_override: Option<String>,
}

/// Window label that drives input gating. Two-window mode wants the native
/// game window's focus; single-window mode wants the main WebView's focus
/// (it IS the game window). Matches the labels passed to the builders.
fn focus_target_label(mode: ShellMode) -> &'static str {
    match mode {
        ShellMode::TwoWindow => "game",
        ShellMode::SingleWindow => "main",
    }
}

fn main() {
    let logger_handle = logger::init_early();
    log::info!("oa-shell starting");

    // Parse CLI args before any other startup work. clap exits with status
    // 0 (for --help / --version) or 2 (for bad flags) on its own; our own
    // validation errors get a multi-line banner and exit 2.
    let direct_launch_cli = match cli::parse_and_resolve() {
        Ok(cfg) => cfg,
        Err(e) => {
            e.emit_banner();
            std::process::exit(2);
        }
    };

    // OA_ROM is the legacy env-var fallback for the dev loop. When BOTH
    // CLI args AND OA_ROM are set, CLI args win and we log the override.
    let env_rom = std::env::var("OA_ROM").ok();
    let direct_launch: Option<cli::DirectLaunchConfig> = match (direct_launch_cli, env_rom.as_deref()) {
        (Some(cfg), Some(env_path)) => {
            log::info!("oa-shell: OA_ROM={env_path} ignored — CLI args supplied");
            Some(cfg)
        }
        (Some(cfg), None) => Some(cfg),
        (None, Some(env_path)) => Some(cli::from_oa_rom_env(env_path)),
        (None, None) => None,
    };

    // Startup-load path: the legacy "load a ROM at startup, no settings
    // cascade" code in run_emu_render reads `rom_path` bytes directly off
    // disk through the bootstrap PCE-CD core. That predates direct-launch
    // and is wrong for any non-tg16 system + can't handle archives.
    //
    // Direct-launch (CLI args OR OA_ROM env) goes through the frontend
    // auto-launch effect instead — it runs the same `handleLaunch`
    // cascade a library tile click does: per-game / per-system / OA-wide
    // settings resolution, the SHA-1-matched library row's overrides,
    // archive::extract_for_launch for .zip/.7z, the right core swap, etc.
    //
    // Letting both paths race caused "black screen" on archive launches:
    // startup-load tried to feed .zip bytes through PCE-CD and failed
    // silently, leaving the core in an undefined state by the time the
    // frontend cascade arrived.
    //
    // Hand the startup-load path nothing — let the frontend cascade own
    // every direct-launch ROM, period. Library-mode (no direct_launch) is
    // unchanged.
    let rom_path: Option<String> = None;
    match &direct_launch {
        Some(cfg) => log::info!(
            "oa-shell: direct-launch ROM = {} (system: {}, archive_inner: {:?}) \u{2014} \
             frontend cascade will load (startup-load path bypassed)",
            cfg.rom_path.display(),
            cfg.system_id,
            cfg.archive_inner_path,
        ),
        None => log::info!("oa-shell: no startup ROM set; waiting for library launch_rom commands"),
    }

    let running = Arc::new(AtomicBool::new(true));
    let tauri_running = running.clone();
    // Focus + UI-intercept atomics: the emu thread reads both each frame and
    // gates input on `game_focused && !ui_intercepting`. Built here so both
    // setup paths and the WindowEvent handler share the same Arcs.
    let game_focused = Arc::new(AtomicBool::new(true));
    let ui_intercepting = Arc::new(AtomicBool::new(false));
    // Phase 6 Cross-system slice 3 — Game focus toggle. Default OFF so OA
    // hotkeys own F-keys / Esc / digits / Backspace until the user opts in.
    let game_focus = Arc::new(AtomicBool::new(false));
    let game_focused_for_event = game_focused.clone();
    let shell_mode_for_event: Arc<std::sync::OnceLock<ShellMode>> = Arc::new(std::sync::OnceLock::new());
    let shell_mode_for_event_evt = shell_mode_for_event.clone();

    // Set of currently-focused oa-shell window labels. Used by the global
    // Ctrl/Cmd+Q handler to ignore the shortcut when our app isn't focused —
    // otherwise pressing Ctrl+Q in Notepad would quit us. Updated by the
    // WindowEvent::Focused handler below.
    let focused_windows: Arc<Mutex<std::collections::HashSet<String>>> =
        Arc::new(Mutex::new(std::collections::HashSet::new()));
    let focused_windows_event = focused_windows.clone();
    let focused_windows_shortcut = focused_windows.clone();

    // Ctrl+Q (Windows/Linux) / Cmd+Q (macOS). Quit isn't gated on the same
    // `enable` flag as F5/F8 — it should fire regardless of whether the game
    // window has focus vs. the library WebView, and regardless of whether a
    // UI element is intercepting (the user wants out).
    use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
    let quit_modifier = if cfg!(target_os = "macos") { Modifiers::SUPER } else { Modifiers::CONTROL };
    let quit_shortcut = Shortcut::new(Some(quit_modifier), Code::KeyQ);

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // Custom protocol for game-media delivery. The handler looks up the
        // active cover variant for a given rom_id and streams the file bytes
        // with Cache-Control: max-age=31536000, immutable. See media.rs.
        .register_asynchronous_uri_scheme_protocol("oa-media", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            std::thread::spawn(move || {
                let state = app.state::<media::MediaState>();
                let response = media::handle_uri_request(state.inner(), &request);
                responder.respond(response);
            });
        })
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed { return; }
                    if shortcut == &quit_shortcut {
                        let any_focused = !focused_windows_shortcut
                            .lock()
                            .map(|s| s.is_empty())
                            .unwrap_or(true);
                        if any_focused {
                            log::info!("oa-shell: quit hotkey fired (oa-shell focused)");
                            graceful_exit(app, 0);
                        } else {
                            log::debug!("oa-shell: quit hotkey suppressed — no oa-shell window focused");
                        }
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            scan_rom_folder,
            launch_rom,
            set_scaling_mode,
            set_shader_preset,
            list_shader_presets,
            set_bloom_amount,
            set_display_aspect_override,
            set_overscan_crop,
            set_bezel_image_override,
            clear_bezel_image_override,
            set_rewind_config,
            get_rewind_state,
            get_perf_stats,
            get_recent_logs,
            get_log_file_path,
            reveal_logs_folder,
            log_from_frontend,
            start_rewind_scrub,
            set_rewind_scrub_position,
            end_rewind_scrub,
            get_tas_state,
            start_tas_recording,
            stop_tas_recording,
            start_tas_replay,
            stop_tas_replay,
            list_tas_recordings,
            delete_tas_recording,
            get_video_state,
            start_video_capture,
            stop_video_capture,
            list_video_clips,
            convert_video_clip_to_webm,
            delete_video_clip,
            open_video_clip_folder,
            list_screenshots,
            delete_screenshot,
            open_screenshot_folder,
            read_memory_region,
            list_milestones,
            add_milestone,
            update_milestone,
            delete_milestone,
            reset_milestone_progress,
            arm_milestones,
            set_window_mode,
            get_shell_mode,
            get_shell_mode_pref,
            set_shell_mode_pref,
            get_direct_launch_config,
            get_game,
            get_layout,
            set_layout,
            get_presentation_mode,
            set_presentation_mode,
            get_system_settings,
            set_system_settings,
            analog_sticks_for_system,
            set_analog_routing,
            set_analog_routing_for_game,
            arm_analog_routing,
            set_libretro_device_for_game,
            arm_libretro_device,
            list_games,
            add_games,
            drop_seed_games,
            update_game_core_override,
            get_game_overrides,
            set_game_overrides,
            delete_game,
            find_game_id_by_path,
            delete_games_for_system,
            delete_all_games,
            list_core_options,
            set_system_core_option,
            set_game_core_option,
            apply_game_core_options,
            get_disc_state,
            set_disc_eject,
            set_disc_image,
            pick_patch_file,
            list_cheats,
            add_cheat,
            update_cheat,
            delete_cheat,
            arm_cheats,
            start_cheat_search,
            filter_cheat_search,
            peek_cheat_search,
            end_cheat_search,
            set_run_ahead,
            search_games,
            list_folders,
            add_folder,
            update_folder,
            remove_folder,
            list_folder_rules,
            set_folder_rules,
            reorder_folders,
            migrate_library_from_local_storage,
            migrate_folders_from_local_storage,
            directory_is_empty,
            start_background_scan,
            cancel_background_scan,
            set_watched_folders,
            list_save_slots,
            delete_save_slot,
            list_monitors,
            get_bindings,
            set_binding,
            reset_bindings,
            list_audio_devices,
            get_audio_device_pref,
            set_audio_device_pref,
            set_ui_intercepting,
            set_game_focus,
            get_game_focus,
            list_cores,
            probe_core_file,
            install_core_from_path,
            remove_installed_core,
            core_installer::available_cores,
            core_installer::download_core,
            get_core_pref,
            set_core_pref,
            quit_app,
            unload_rom,
            media::get_media_index,
            media::get_region_priority,
            media::set_region_priority,
            media::get_media_kinds_to_fetch,
            media::set_media_kinds_to_fetch,
            media::get_only_sync_identified,
            media::set_only_sync_identified,
            media::set_manual_cover,
            media::clear_media,
            media::set_selected_variant,
            media::sync_media_for_system,
            metadata::sync_metadata_for_system,
            media::media_storage_stats,
            media::open_media_folder,
            rom_hashes::sync_rom_hashes_for_system,
            rom_hashes::sync_mame_titles,
            rom_hashes::lookup_mame_title,
            rom_hashes::resolve_rom_hashes_for_system,
            rom_hashes::lookup_rom_hash,
            list_game_groups,
            set_game_group_default,
            clear_game_group_default,
            get_library_prefs,
            set_library_prefs,
        ])
        .setup({
            let running = running.clone();
            let rom_path = rom_path.clone();
            let game_focused = game_focused.clone();
            let ui_intercepting = ui_intercepting.clone();
            let game_focus = game_focus.clone();
            let shell_mode_for_event = shell_mode_for_event.clone();
            let logger_handle = logger_handle.clone();
            let mut direct_launch = direct_launch.clone();
            move |app| {
                let (cmd_tx, cmd_rx) = mpsc::channel::<EmuCommand>();

                let app_data_dir = app.path().app_data_dir().unwrap_or_else(|e| {
                    log::warn!("oa-shell: app_data_dir() failed ({e:?}); save states + prefs will use cwd");
                    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                });
                log::info!("oa-shell: app_data_dir = {}", app_data_dir.display());

                // Now that app_data_dir is resolved, switch the logger's
                // file output on. Earlier log lines (cli arg parse,
                // shell_mode resolution) went to stderr + ring only;
                // from here on they also hit the file.
                match logger::configure_file_output(&app_data_dir) {
                    Ok(path) => log::info!("oa-shell: log file = {}", path.display()),
                    Err(e) => log::warn!("oa-shell: log file setup failed: {e}"),
                }

                // Phase E — multi-core boot. Fan out the four boot-time
                // I/O loads to background workers and let them run while
                // the main thread continues with shell-mode resolution +
                // window setup. None of these results are needed until
                // the MediaState manage / library_db manage calls below;
                // joining at point-of-use overlaps the disk reads with
                // the wgpu surface creation + WebView init, which is
                // typically the longest single step in boot. Net savings
                // on a cold start: 100–400ms depending on disk and
                // library size. Worker panics fall back to default-empty
                // state with a warning — never a hard boot failure.
                let workers_app_data = app_data_dir.clone();
                let sweep_handle = std::thread::spawn({
                    let p = workers_app_data.clone();
                    move || archive::sweep_temp(&p.join("temp"))
                });
                let media_db_handle: std::thread::JoinHandle<media::MediaDb> =
                    std::thread::spawn({
                        let p = workers_app_data.clone();
                        move || media::read_media_db(&p)
                    });
                let media_prefs_handle: std::thread::JoinHandle<media::MediaPrefs> =
                    std::thread::spawn({
                        let p = workers_app_data.clone();
                        move || media::read_media_prefs(&p)
                    });
                let library_db_handle: std::thread::JoinHandle<
                    Result<library_db::LibraryDb, String>,
                > = std::thread::spawn({
                    let p = workers_app_data;
                    move || library_db::LibraryDb::open(&p).map_err(|e| e.to_string())
                });

                // Direct-launch mode unconditionally forces single-window —
                // operator's `OA_SHELL_MODE` / `shell.json` preference stays
                // intact on disk so the next library-mode launch honors it.
                // See plan §2 / DECISIONS for the reasoning (one HWND, wgpu
                // under transparent WebView, library chrome hidden by the
                // frontend, close-window = exit).
                let shell_mode = match &direct_launch {
                    Some(_) => {
                        log::info!(
                            "oa-shell: direct-launch mode \u{2192} forcing single-window \
                             (operator pref preserved on disk)"
                        );
                        ShellMode::SingleWindow
                    }
                    None => ShellMode::resolve(&app_data_dir),
                };
                // Stash for the WindowEvent handler — it needs to know which
                // window label to filter on.
                let _ = shell_mode_for_event.set(shell_mode);

                let app_handle = app.handle().clone();
                // Shared rewind-ring stats. Built here so both AppState
                // (Tauri reader) and the emu thread (writer) hold the same
                // Arc.
                let rewind_state = Arc::new(Mutex::new(SharedRewindState::default()));
                let perf_stats = Arc::new(Mutex::new(SharedPerfStats::default()));
                // Shared TAS recording/replay state — same ownership model.
                let tas_state = Arc::new(Mutex::new(SharedTasState::default()));
                // Shared video-capture state — same.
                let video_state = Arc::new(Mutex::new(SharedVideoState::default()));
                // Per-frame memory snapshot (slice E) — same pattern.
                let memory_snapshot = Arc::new(Mutex::new(MemorySnapshot::default()));
                // Cached disc-control snapshot (RetroArch parity). Refreshed
                // on LoadRom + after each disc swap; read by Tauri commands.
                let disc_state: Arc<Mutex<Option<oa_core::DiscInfo>>> = Arc::new(Mutex::new(None));
                // Direct-launch knows the target system upfront — hand the
                // emu thread a bootstrap hint so it loads the right libretro
                // .dll on first launch instead of loading the tg16 default
                // and then immediately swapping. Library mode leaves it None
                // → tg16 bootstrap (the historical default; cheap because
                // tg16 is also the active dev core).
                let bootstrap_hint: Option<BootstrapHint> = direct_launch.as_ref().map(|c| BootstrapHint {
                    system_id: c.system_id.clone(),
                    core_override: c.core_override.clone(),
                });
                let shell_window = match shell_mode {
                    ShellMode::TwoWindow => setup_two_window(app, running.clone(), rom_path.clone(), cmd_rx, app_data_dir.clone(), game_focused.clone(), ui_intercepting.clone(), game_focus.clone(), rewind_state.clone(), tas_state.clone(), video_state.clone(), memory_snapshot.clone(), disc_state.clone(), perf_stats.clone(), app_handle, bootstrap_hint.clone())?,
                    ShellMode::SingleWindow => setup_single_window(app, running.clone(), rom_path.clone(), cmd_rx, app_data_dir.clone(), game_focused.clone(), ui_intercepting.clone(), game_focus.clone(), rewind_state.clone(), tas_state.clone(), video_state.clone(), memory_snapshot.clone(), disc_state.clone(), perf_stats.clone(), app_handle, bootstrap_hint.clone())?,
                };

                // MediaState: shared in-memory MediaDb + region prefs, hydrated
                // from disk once at startup. The protocol handler reads through
                // the same Arcs without round-tripping through Tauri commands.
                // Hydration ran in parallel during shell window setup (Phase E);
                // join the workers now and fall back to defaults on the
                // (very unlikely) panic case so a single bad worker doesn't
                // wedge boot.
                let media_db_value = media_db_handle.join().unwrap_or_else(|_| {
                    log::warn!("oa-shell: media_db worker panicked; starting with empty MediaDb");
                    media::MediaDb::default()
                });
                let media_prefs_value = media_prefs_handle.join().unwrap_or_else(|_| {
                    log::warn!("oa-shell: media_prefs worker panicked; using defaults");
                    media::MediaPrefs::default()
                });
                let media_db = std::sync::Arc::new(std::sync::RwLock::new(media_db_value));
                let media_prefs = std::sync::Arc::new(std::sync::RwLock::new(media_prefs_value));
                app.manage(media::MediaState {
                    db: media_db.clone(),
                    prefs: media_prefs.clone(),
                    app_data_dir: app_data_dir.clone(),
                });

                // Background scan service state — tracks in-flight scan jobs
                // so cancel_background_scan can flip their cancel flags.
                app.manage(scan_service::ScanServiceState::default());

                // Cores directory pointer — buildbot installer commands need
                // to walk it on every call. Resolved once here so the
                // commands don't re-derive it (which would lock us out of
                // testing relative-path setups in the future).
                app.manage(core_installer::CoresDir(resolve_cores_dir()));

                // Filesystem watcher — the frontend calls set_watched_folders
                // once its settings store is hydrated. Until then this just
                // holds an empty state.
                app.manage(watcher::WatcherState::default());

                // LibraryDb: SQLite-backed game catalog at appDataDir/library/games.sqlite.
                // Replaces the WebView's localStorage[oa.library.v1] from Phase 1-2.
                // Open eagerly so the frontend's first get_library command lands in
                // <10ms instead of paying the open + schema-init cost on demand.
                // Phase E — open ran in parallel during shell setup; join here.
                // Sweep-temp worker also joins (its result isn't consumed; the
                // join keeps it tied to the boot sequence rather than dangling).
                let _ = sweep_handle.join();
                let library_db_open = library_db_handle.join().unwrap_or_else(|_| {
                    log::error!("oa-shell: library_db worker panicked");
                    Err("library_db worker panicked".to_string())
                });
                match library_db_open {
                    Ok(db) => {
                        let count = db.count().unwrap_or(0);
                        log::info!("oa-shell: library_db open ({} games tracked)", count);

                        // Direct-launch hash lookup: if a cart-shaped ROM was
                        // supplied, hash it and see if the library has a row
                        // with the same SHA-1. On hit, the frontend will pull
                        // the matched RomEntry via get_game() and apply that
                        // row's per-game overrides through the standard
                        // launch cascade. CD images skip — their on-disk
                        // hash isn't libretro-database-canonical (and is
                        // expensive for multi-GB files at boot).
                        //
                        // For archive direct-launch (Phase H — single ROM
                        // inside .zip/.7z) we hash the inner ROM bytes, not
                        // the outer archive, so the SHA-1 matches the
                        // library DB's convention (rom_hashes stamps the
                        // inner bytes too — see rom_bytes_for).
                        if let Some(cfg) = direct_launch.as_mut() {
                            let extension_for_cd_check: String = match cfg.archive_inner_path.as_deref() {
                                Some(inner) => std::path::Path::new(inner)
                                    .extension()
                                    .and_then(|e| e.to_str())
                                    .unwrap_or("")
                                    .to_ascii_lowercase(),
                                None => cfg
                                    .rom_path
                                    .extension()
                                    .and_then(|e| e.to_str())
                                    .unwrap_or("")
                                    .to_ascii_lowercase(),
                            };
                            if cfg.matched_entry_id.is_none()
                                && !is_cd_extension(&extension_for_cd_check)
                            {
                                let hash_result = match cfg.archive_inner_path.as_deref() {
                                    Some(inner) => archive::read_inner_to_bytes(&cfg.rom_path, inner)
                                        .map(|bytes| {
                                            use sha1::{Digest, Sha1};
                                            let mut h = Sha1::new();
                                            h.update(&bytes);
                                            format!("{:x}", h.finalize())
                                        }),
                                    None => rom_hashes::stream_sha1_of_file(&cfg.rom_path),
                                };
                                match hash_result {
                                    Ok(sha) => match db.find_game_by_sha1(&sha) {
                                        Ok(Some(row)) => {
                                            log::info!(
                                                "oa-shell: direct-launch SHA-1 matched library row {} ({})",
                                                row.id,
                                                row.title,
                                            );
                                            cfg.matched_entry_id = Some(row.id);
                                        }
                                        Ok(None) => log::info!(
                                            "oa-shell: direct-launch SHA-1 not in library; using ad-hoc settings"
                                        ),
                                        Err(e) => log::warn!(
                                            "oa-shell: direct-launch find_game_by_sha1 failed: {e}"
                                        ),
                                    },
                                    Err(e) => log::warn!(
                                        "oa-shell: direct-launch SHA-1 of {} failed: {e}",
                                        cfg.rom_path.display()
                                    ),
                                }
                            }
                        }

                        app.manage(db);
                    }
                    Err(e) => {
                        // Hard error — without the library DB the UI has nothing to render.
                        log::error!("oa-shell: library_db open failed: {e}");
                        return Err(format!("library_db open: {e}").into());
                    }
                }

                // Slice D — start the shader presets watcher. Held on
                // AppState so it lives as long as the process. cmd_tx is
                // also cloned for the watcher's re-apply path; the original
                // moves into AppState for Tauri commands to use.
                let active_shader_preset = Arc::new(Mutex::new(None::<String>));
                let exe_dir = resolve_exe_dir();
                let shader_presets_watcher = shader_presets_watcher::spawn(
                    app.handle().clone(),
                    exe_dir,
                    cmd_tx.clone(),
                    active_shader_preset.clone(),
                )
                .map_err(|e| {
                    log::warn!("oa-shell: shader presets watcher disabled: {e}");
                    e
                })
                .ok();

                app.manage(AppState {
                    emu_tx: Mutex::new(cmd_tx),
                    shell_window,
                    shell_mode,
                    app_data_dir,
                    ui_intercepting: ui_intercepting.clone(),
                    game_focus: game_focus.clone(),
                    active_archive_entry_id: Arc::new(Mutex::new(None)),
                    rewind_state,
                    tas_state,
                    video_state,
                    memory_snapshot,
                    perf_stats: perf_stats.clone(),
                    logger_handle: logger_handle.clone(),
                    active_shader_preset,
                    shader_presets_watcher,
                    disc_state,
                    cheat_search: Arc::new(Mutex::new(None)),
                    direct_launch,
                });

                // Register the quit shortcut now that the plugin is initialized.
                // Suppression of cross-app fires is handled inside the handler
                // by checking `focused_windows`.
                if let Err(e) = app.global_shortcut().register(quit_shortcut) {
                    log::warn!("oa-shell: register Ctrl/Cmd+Q failed: {e:?}");
                }

                Ok(())
            }
        })
        .on_window_event(move |window, event| {
            match event {
                tauri::WindowEvent::CloseRequested { .. } => {
                    tauri_running.store(false, Ordering::SeqCst);
                }
                tauri::WindowEvent::Focused(focused) => {
                    // Only the game-bearing window's focus drives input gating.
                    // In two-window mode the library can be focused on a second
                    // monitor without that meaning "user is playing the game."
                    if let Some(mode) = shell_mode_for_event_evt.get() {
                        if window.label() == focus_target_label(*mode) {
                            game_focused_for_event.store(*focused, Ordering::SeqCst);
                            log::debug!("oa-shell: game window focused = {focused}");
                        }
                    }
                    // Track which of our windows are currently focused so the
                    // global quit shortcut handler can suppress fires that
                    // originate while oa-shell isn't the focused app.
                    if let Ok(mut set) = focused_windows_event.lock() {
                        let label = window.label().to_string();
                        if *focused { set.insert(label); } else { set.remove(&label); }
                    }
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("tauri run failed");

    log::info!("oa-shell: tauri exited, signalling threads");
    running.store(false, Ordering::SeqCst);
    log::info!("oa-shell: bye");
}

fn setup_two_window(
    app: &mut tauri::App,
    running: Arc<AtomicBool>,
    rom_path: Option<String>,
    cmd_rx: mpsc::Receiver<EmuCommand>,
    app_data_dir: PathBuf,
    game_focused: Arc<AtomicBool>,
    ui_intercepting: Arc<AtomicBool>,
    game_focus: Arc<AtomicBool>,
    rewind_state: Arc<Mutex<SharedRewindState>>,
    tas_state: Arc<Mutex<SharedTasState>>,
    video_state: Arc<Mutex<SharedVideoState>>,
    memory_snapshot: Arc<Mutex<MemorySnapshot>>,
    disc_state: Arc<Mutex<Option<oa_core::DiscInfo>>>,
    perf_stats: Arc<Mutex<SharedPerfStats>>,
    app_handle: tauri::AppHandle,
    bootstrap_hint: Option<BootstrapHint>,
) -> tauri::Result<ShellWindow> {
    let _library = tauri::WebviewWindowBuilder::new(
        app,
        "library",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title("Overlooked Arcade")
    .inner_size(960.0, 640.0)
    .build()?;
    log::info!("oa-shell: library WebviewWindow built (two-window)");

    let game = tauri::WindowBuilder::new(app, "game")
        .title("Overlooked Arcade \u{2014} game")
        .inner_size(768.0, 717.0)
        .build()?;
    log::info!("oa-shell: game Window built (two-window)");
    let game = Arc::new(game);

    // Compute raw handles INSIDE the spawned thread — RawWindowHandle's iOS
    // variant carries `NonNull<c_void>` which makes the whole enum `!Send`,
    // so we can't construct it on the main thread and move it across.
    // Arc<tauri::Window> IS Send/Sync, so we move that and call window_handle()
    // on the emu thread.
    std::thread::Builder::new()
        .name("oa-emu-render".into())
        .spawn({
            let game = game.clone();
            move || {
                let raw_window = match game.window_handle() {
                    Ok(h) => h.as_raw(),
                    Err(e) => { log::error!("game window_handle failed: {e:?}"); return; }
                };
                let raw_display = match game.display_handle() {
                    Ok(h) => h.as_raw(),
                    Err(e) => { log::error!("game display_handle failed: {e:?}"); return; }
                };
                let initial_size = game.inner_size().map(|s| (s.width, s.height)).unwrap_or((768, 717));
                let inner_size_fn: Box<dyn Fn() -> Option<(u32, u32)> + Send> = {
                    let game = game.clone();
                    Box::new(move || game.inner_size().ok().map(|s| (s.width, s.height)))
                };
                let window_position_fn: Box<dyn Fn() -> Option<(i32, i32)> + Send> = {
                    let game = game.clone();
                    Box::new(move || game.inner_position().ok().map(|p| (p.x, p.y)))
                };
                run_emu_render(running, inner_size_fn, window_position_fn, raw_window, raw_display, initial_size, rom_path, cmd_rx, app_data_dir, game_focused, ui_intercepting, game_focus, rewind_state, tas_state, video_state, memory_snapshot, disc_state, perf_stats, app_handle, bootstrap_hint);
            }
        })?;

    Ok(ShellWindow::TwoWindow { game })
}

fn setup_single_window(
    app: &mut tauri::App,
    running: Arc<AtomicBool>,
    rom_path: Option<String>,
    cmd_rx: mpsc::Receiver<EmuCommand>,
    app_data_dir: PathBuf,
    game_focused: Arc<AtomicBool>,
    ui_intercepting: Arc<AtomicBool>,
    game_focus: Arc<AtomicBool>,
    rewind_state: Arc<Mutex<SharedRewindState>>,
    tas_state: Arc<Mutex<SharedTasState>>,
    video_state: Arc<Mutex<SharedVideoState>>,
    memory_snapshot: Arc<Mutex<MemorySnapshot>>,
    disc_state: Arc<Mutex<Option<oa_core::DiscInfo>>>,
    perf_stats: Arc<Mutex<SharedPerfStats>>,
    app_handle: tauri::AppHandle,
    bootstrap_hint: Option<BootstrapHint>,
) -> tauri::Result<ShellWindow> {
    // Single-window mode = one transparent WebviewWindow whose
    // underlying HWND hosts both the WebView UI and the wgpu game
    // surface. Compositing happens via DWM: WebView pixels with alpha
    // < 1 reveal wgpu paint underneath. External drag-drop is known
    // to be unreliable on transparent WebView2 windows (see
    // PARKING_LOT.md 2026-05-19 entry) — the Import Wizard and
    // Settings → Library → Add cover the same ingest flow.
    let window = tauri::WebviewWindowBuilder::new(
        app,
        "main",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title("Overlooked Arcade")
    .inner_size(960.0, 640.0)
    .transparent(true)
    .build()?;
    log::info!("oa-shell: single transparent WebviewWindow built (single-window)");
    let window = Arc::new(window);

    std::thread::Builder::new()
        .name("oa-emu-render".into())
        .spawn({
            let window = window.clone();
            move || {
                let raw_window = match window.window_handle() {
                    Ok(h) => h.as_raw(),
                    Err(e) => { log::error!("single window_handle failed: {e:?}"); return; }
                };
                let raw_display = match window.display_handle() {
                    Ok(h) => h.as_raw(),
                    Err(e) => { log::error!("single display_handle failed: {e:?}"); return; }
                };
                let initial_size = window.inner_size().map(|s| (s.width, s.height)).unwrap_or((960, 640));
                let inner_size_fn: Box<dyn Fn() -> Option<(u32, u32)> + Send> = {
                    let window = window.clone();
                    Box::new(move || window.inner_size().ok().map(|s| (s.width, s.height)))
                };
                let window_position_fn: Box<dyn Fn() -> Option<(i32, i32)> + Send> = {
                    let window = window.clone();
                    Box::new(move || window.inner_position().ok().map(|p| (p.x, p.y)))
                };
                run_emu_render(running, inner_size_fn, window_position_fn, raw_window, raw_display, initial_size, rom_path, cmd_rx, app_data_dir, game_focused, ui_intercepting, game_focus, rewind_state, tas_state, video_state, memory_snapshot, disc_state, perf_stats, app_handle, bootstrap_hint);
            }
        })?;

    Ok(ShellWindow::SingleWindow { window })
}

fn run_emu_render(
    running: Arc<AtomicBool>,
    inner_size_fn: Box<dyn Fn() -> Option<(u32, u32)> + Send>,
    // window_position_fn returns the game window's content-area top-left
    // in screen pixels. None when Tauri can't resolve it (window not yet
    // created etc). Combined with `Renderer::last_viewport` for the
    // game-output rectangle in screen space — drives window-relative
    // pointer mapping (NDS stylus, Dreamcast light-gun, etc.).
    window_position_fn: Box<dyn Fn() -> Option<(i32, i32)> + Send>,
    raw_window: raw_window_handle::RawWindowHandle,
    raw_display: raw_window_handle::RawDisplayHandle,
    initial_size: (u32, u32),
    rom_path: Option<String>,
    cmd_rx: mpsc::Receiver<EmuCommand>,
    app_data_dir: PathBuf,
    game_focused: Arc<AtomicBool>,
    ui_intercepting: Arc<AtomicBool>,
    game_focus: Arc<AtomicBool>,
    rewind_state: Arc<Mutex<SharedRewindState>>,
    tas_state: Arc<Mutex<SharedTasState>>,
    video_state: Arc<Mutex<SharedVideoState>>,
    memory_snapshot: Arc<Mutex<MemorySnapshot>>,
    disc_state: Arc<Mutex<Option<oa_core::DiscInfo>>>,
    perf_stats: Arc<Mutex<SharedPerfStats>>,
    app_handle: tauri::AppHandle,
    bootstrap_hint: Option<BootstrapHint>,
) {
    use oa_core::Core;
    use oa_libretro::LibretroCore;

    // The per-system bindings produce libretro-shape bits via
    // `bindings::to_libretro_bits(system_id, polled.buttons)` — we
    // remap inline at the dispatch site (Phase 4 slice C records the
    // libretro-shape bits into TAS files, so we want them in a local
    // we can both dispatch + log without computing twice).

    // Active system id, mutable so LoadRom can swap it. Initial value
    // matches the bootstrap hint (direct-launch target) or tg16 in
    // library mode. Drives bindings load, keyboard-passthrough init,
    // and the per-system core resolution below.
    let mut current_system_id: String = bootstrap_hint
        .as_ref()
        .map(|h| h.system_id.clone())
        .unwrap_or_else(|| "tg16".to_string());

    let mut renderer = match unsafe { oa_render::Renderer::new(raw_window, raw_display, initial_size) } {
        Ok(r) => r,
        Err(e) => {
            log::error!("oa-render init failed: {e:?}");
            return;
        }
    };

    let mut input = oa_input::InputPoller::with_mappings(
        oa_input::KeyboardMapping::empty(),
        oa_input::GamepadMapping::empty(),
    );
    // Load bindings for the bootstrap system from disk (or defaults if
    // the file is missing) and apply them to the freshly-built poller.
    // Bootstrap system is tg16 in library mode, the direct-launch
    // target in direct-launch mode (BootstrapHint determined upstream).
    // The active set swaps on every LoadRom that carries a different
    // system_id.
    let initial_bindings = bindings::load(&app_data_dir, &current_system_id);
    apply_bindings_to_poller(&mut input, &current_system_id, &initial_bindings);
    log::info!(
        "oa-shell: emu+render thread up; bindings loaded for {} ({} buttons); hotkeys: F1 = reset, F5 = save state, F8 = restore",
        current_system_id,
        initial_bindings.len()
    );

    // Cores + BIOS live in `<exe_dir>/cores/` and `<exe_dir>/system/` — the
    // install is self-contained, no roaming AppData required. User prefs
    // (saves, bindings, audio.json, shell.json) stay in app_data_dir because
    // they're user-specific, not install-specific.
    //
    // Env override `OA_LIBRETRO_CORE` lets a developer point at a specific
    // .dll path (useful before the cores folder UI lands).
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    log::info!("oa-shell: exe_dir = {}", exe_dir.display());

    let cores_dir = exe_dir.join("cores");
    let system_dir = exe_dir.join("system");
    let _ = std::fs::create_dir_all(&cores_dir);
    let _ = std::fs::create_dir_all(&system_dir);

    // Bootstrap-core resolution. Direct-launch supplies a hint so we
    // load the *target* system's core upfront instead of the tg16
    // default-then-swap path. Library mode (None hint) keeps the tg16
    // default — historical behavior, harmless because tg16 is also the
    // active dev core.
    //
    // Priority within either branch: `OA_LIBRETRO_CORE` env (dev override,
    // wins) → `--core` per-launch override (direct-launch only) →
    // `cores.json` per-system pref → hardcoded default-for-system.
    let cores_pref = read_cores_pref(&app_data_dir);
    let (bootstrap_dll_name, bootstrap_system_enum): (String, oa_core::SystemId) =
        match bootstrap_hint.as_ref() {
            Some(hint) => {
                let dll = hint.core_override.clone()
                    .or_else(|| cores_pref.get(&hint.system_id).cloned())
                    .unwrap_or_else(|| default_core_dll_for_system(&hint.system_id).to_string());
                log::info!(
                    "oa-shell: direct-launch bootstrap \u{2192} system={} dll={} (skipping tg16 default)",
                    hint.system_id, dll
                );
                (dll, parse_system_id(&hint.system_id))
            }
            None => {
                let default_dll = if cfg!(windows) {
                    "mednafen_pce_fast_libretro.dll"
                } else if cfg!(target_os = "macos") {
                    "mednafen_pce_fast_libretro.dylib"
                } else {
                    "mednafen_pce_fast_libretro.so"
                };
                let dll = cores_pref.get("tg16").cloned().unwrap_or_else(|| default_dll.to_string());
                if dll != default_dll {
                    log::info!("oa-shell: cores.json pref for tg16 = {dll}");
                }
                (dll, oa_core::SystemId::PcEngine)
            }
        };

    let detected_core: Option<PathBuf> = std::env::var("OA_LIBRETRO_CORE")
        .ok()
        .map(PathBuf::from)
        .or_else(|| {
            let p = cores_dir.join(&bootstrap_dll_name);
            if p.exists() { Some(p) } else { None }
        });

    let save_dir = app_data_dir.join("saves");
    let _ = std::fs::create_dir_all(&save_dir);

    let dll_path = match detected_core {
        Some(p) => p,
        None => {
            log::error!(
                "oa-shell: no libretro core found.\n\
                 ─────────────────────────────────────────────────────────────────\n\
                 Drop a libretro core into:\n  {}\n\
                 Or set the OA_LIBRETRO_CORE env var to point at a .dll directly.\n\
                 \n\
                 Looking for: {}\n\
                 Download from: https://buildbot.libretro.com/nightly/windows/x86_64/latest/{}.zip\n\
                 unzip, and place {} into the cores folder above.\n\
                 ─────────────────────────────────────────────────────────────────",
                cores_dir.display(),
                bootstrap_dll_name,
                bootstrap_dll_name,
                bootstrap_dll_name,
            );
            return;
        }
    };

    log::info!("oa-shell: loading libretro core from {}", dll_path.display());
    let initial_core = match LibretroCore::load(&dll_path, bootstrap_system_enum, &system_dir, &save_dir) {
        Ok(c) => {
            log::info!("oa-shell: libretro core loaded successfully");
            c
        }
        Err(e) => {
            log::error!("oa-shell: libretro core load failed: {e:?}");
            return;
        }
    };
    let mut timing = initial_core.timing();
    let mut current_core_dll: String = bootstrap_dll_name.clone();
    // `current_system_id` was initialized at the top of run_emu_render from
    // the bootstrap hint; LoadRom updates it on each launch.
    // `core` is Option so we can drop + reload it on a per-game core swap
    // (libretro singleton constraint: only one LibretroCore alive per process).
    // A failed swap leaves `core = None` and the frame body skips emulation
    // until the next successful LoadRom — operator can right-click the tile
    // and pick a different core to recover.
    let mut core: Option<LibretroCore> = Some(initial_core);
    log::info!(
        "oa-shell: active core timing = {}x{} @ {:.3} Hz, audio {} Hz",
        timing.width, timing.height, timing.fps, timing.sample_rate
    );

    let initial_audio_device = read_audio_pref(&app_data_dir);
    let mut audio = match oa_audio::AudioSink::with_device(timing.sample_rate, initial_audio_device.as_deref()) {
        Ok(a) => {
            log::info!(
                "oa-shell: audio sink up at {} Hz (device = {:?})",
                a.sample_rate(),
                a.current_device()
            );
            Some(a)
        }
        Err(e) => {
            log::warn!("oa-shell: audio disabled ({e:?}); game will run silent");
            None
        }
    };

    // Active ROM stem (sanitized filename without extension) gates which
    // per-game save-state directory we hit. Updated on every successful
    // load_rom — both the startup OA_ROM path and the runtime LoadRom command.
    let mut current_rom_stem: Option<String> = None;
    if let (Some(path), Some(core_ref)) = (rom_path.as_deref(), core.as_mut()) {
        let ext = Path::new(path)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_else(|| "pce".to_string());
        let stem = sanitize_stem(path);
        let load_result = if is_cd_extension(&ext) {
            // Path-based load: core opens the .cue / .chd / .m3u itself and
            // reads tracks relative to it.
            log::info!("oa-shell: loading CD image (path-based) from {}", path);
            core_ref.load_rom(oa_libretro::RomSource::Path(Path::new(path)), &ext, &stem)
        } else {
            match std::fs::read(path) {
                Ok(bytes) => {
                    log::info!("oa-shell: loaded {} bytes from {}", bytes.len(), path);
                    core_ref.load_rom(oa_libretro::RomSource::Bytes(&bytes), &ext, &stem)
                }
                Err(e) => Err(oa_core::CoreError::InvalidRom(format!("read {}: {e}", path))),
            }
        };
        match load_result {
            Ok(()) => {
                log::info!("oa-shell: ROM accepted by core; emulation will start");
                current_rom_stem = Some(sanitize_stem(path));
            }
            Err(e) => {
                log::error!("oa-shell: ROM rejected: {e:?}");
                toast(&app_handle, ToastLevel::Error, format!("ROM rejected: {e}"));
            }
        }
    }

    // F5 = save state to active slot on disk, F8 = restore from active slot.
    // Slots are per-game (keyed by the ROM filename stem) and persist under
    // app_data_dir/saves/<stem>/slot-N.bin. Number keys 0-9 set the active
    // slot. Edge-detected so held keys only fire once.
    let mut prev_f1 = false;
    let mut prev_f5 = false;
    let mut prev_f8 = false;
    let mut current_slot: u32 = 0;
    let mut prev_digit = [false; 10];

    // RetroArch-parity slice 3 — gameplay hotkeys. F2 toggles pause,
    // F3 advances one frame while paused, F6/F7 are hold-for-effect
    // (fast-forward / slow-motion), F12 captures a screenshot.
    // F1 reset is already wired above.
    let mut prev_f2 = false;
    let mut prev_f3 = false;
    let mut prev_f12 = false;
    let mut paused = false;
    let mut frame_advance_request = false;
    // Slow-motion runs run_frame on every Nth render cycle. 2 = half speed
    // (RetroArch default); the rest of the cycles render the last frame.
    const SLOW_MOTION_DIVISOR: u32 = 2;
    let mut slow_mo_phase: u32 = 0;
    // Fast-forward bursts: call run_frame N times per render cycle.
    // 4 = 4× normal speed.
    const FAST_FORWARD_BURST: u32 = 4;

    // Esc rising-edge detection. Emits `oa://request-quick-settings` to
    // the frontend so two-window mode (where the native game window has
    // no WebView to receive keydown events) gets the same Quick Settings
    // affordance single-window already enjoys via App.tsx's keydown
    // handler. Gated on the same `enable` flag as F1/F5/F8, so Esc fires
    // only when the game window has focus and no modal is intercepting.
    let mut prev_esc = false;

    // Phase 6 Cross-system slice 2 — libretro keyboard passthrough.
    // Tracks the set of keys held last frame so the pump can edge-detect
    // press/release transitions and forward them to the core via
    // `LibretroCore::send_keyboard_event`. The `active` flag combines the
    // per-system `SystemSettings::keyboard_passthrough` override with the
    // system's compiled-in default — refreshed on every successful
    // LoadRom so a system change re-resolves the flag. Initial value
    // matches the bootstrap system (tg16 by default; the direct-launch
    // target in direct-launch mode — matters for systems like mame / msx
    // that want keyboard passthrough on from frame zero).
    let mut keyboard_passthrough_active: bool =
        system_settings::effective_keyboard_passthrough(
            &current_system_id,
            &system_settings::read_system_settings(&app_data_dir, &current_system_id),
        );
    let mut prev_keyboard_keys: HashSet<Keycode> = HashSet::new();

    // Phase 6 Cross-system slice 3 — rising-edge tracking for the Ctrl+G
    // Game-focus toggle hotkey. We can't use Scroll Lock because the
    // `device_query` crate doesn't expose a ScrollLock variant; Ctrl+G is
    // the single binding. Bypasses every input gate (focus / UI-intercept /
    // game-focus / hotkeys_enabled) so the user can always toggle out.
    // Edge state lives outside the loop so a held combo only fires once.
    let mut prev_ctrl_g_held = false;

    // Rewind ring (Phase 4 slice A). Bounded by total bytes; populated only
    // when `rewind_config.enabled` is true. Holding Backspace pops the
    // newest snapshot and load_states it, producing visual rewind at
    // ~capture-interval × render-rate frames per second. Frontend pushes
    // SetRewindConfig before each launch resolving the inheritance chain.
    let mut rewind_config = oa_savestate::RewindConfig::default();
    let mut rewind_ring = oa_savestate::RewindRing::new(rewind_config.max_bytes);
    // Scrubbing (Phase 4 slice B). When `scrubbing` is true the frame body
    // freezes forward play + capture; each frame instead peeks at
    // `scrub_position` and applies that snapshot. End commits or cancels.
    let mut scrubbing = false;
    let mut scrub_position: u32 = 0;
    // Set true on each scrub-mode entry / position change so the frame body
    // re-applies the peek+load+run_frame even if `scrub_position` hasn't
    // changed since the last frame (e.g. opening the overlay should show
    // the live frame frozen, not whatever was painted the prior tick).
    let mut scrub_dirty = false;

    // Helper: push the current ring + scrub stats into the shared Mutex
    // so Tauri commands see fresh values. Cheap (4-byte u32s + a u64).
    // Captures `&rewind_ring`, `&rewind_config`, `&timing`, `scrubbing`,
    // `scrub_position` via the outer scope.
    let publish_rewind_state = |ring: &oa_savestate::RewindRing,
                                cfg: &oa_savestate::RewindConfig,
                                t: &oa_core::Timing,
                                scrubbing: bool,
                                pos: u32,
                                state: &Arc<Mutex<SharedRewindState>>| {
        if let Ok(mut s) = state.lock() {
            *s = SharedRewindState {
                enabled: cfg.enabled,
                snapshot_count: ring.len() as u32,
                byte_size: ring.byte_size() as u64,
                capture_interval_frames: cfg.capture_interval_frames,
                fps: t.fps,
                scrubbing,
                scrub_position: pos,
            };
        }
    };

    // Phase 4 slice C — TAS recording + replay.
    //
    // At most one of these is `Some` at a time (mutually exclusive modes
    // checked when handling command messages). When recording is Some,
    // the frame body's normal forward-play branch logs the dispatched
    // input each frame. When replay is Some, the frame body replaces
    // user input with the recording's frame at `replay_current_frame`,
    // then increments; on EOF, replay is cleared automatically.
    //
    // Both modes suppress hold-Backspace rewind for clean v1 semantics —
    // rewind-during-recording and rewind-during-replay get hairy + ship
    // in a follow-up slice (see DECISIONS).
    let mut tas_recording: Option<oa_savestate::tas::TasRecording> = None;
    let mut tas_replay: Option<oa_savestate::tas::TasRecording> = None;
    let mut tas_replay_current_frame: u64 = 0;
    // Track the loaded ROM's SHA-1 (uppercase hex) so we can stamp it
    // into recordings for replay-safety checking. Computed on LoadRom
    // for the Bytes source (cart / HuCard). Path-source loads (CD .cue)
    // leave this empty — hashing a 600 MB CHD at every load isn't
    // worth the latency.
    let mut current_rom_sha1_hex: String = String::new();

    let publish_tas_state = |mode: TasMode,
                             frame: u64,
                             total_frames: u64,
                             display_name: &str,
                             state: &Arc<Mutex<SharedTasState>>| {
        if let Ok(mut s) = state.lock() {
            *s = SharedTasState {
                mode,
                frame,
                total_frames,
                display_name: display_name.to_string(),
            };
        }
    };

    // Phase 4 slice D — frame-by-frame video capture. `Some` while a
    // capture is active; emu thread copies `core.framebuffer().pixels`
    // into a VideoFrame and pushes via `try_submit` after each forward
    // run_frame (normal + replay branches; not during scrub/rewind).
    // Channel-overflow drops are counted in the worker's metadata so
    // the manifest reports gaps.
    let mut video_capture: Option<video_capture::VideoCaptureWorker> = None;
    // Captured at recording start to stamp into the manifest. Resolution
    // may shift mid-game (PCE 256/352/512 modes) but the manifest's
    // first-frame size is just a hint.
    let mut video_first_size: (u32, u32) = (0, 0);
    let mut video_display_name = String::new();

    let publish_video_state = |worker: Option<&video_capture::VideoCaptureWorker>,
                               frames_submitted: u64,
                               display_name: &str,
                               state: &Arc<Mutex<SharedVideoState>>| {
        if let Ok(mut s) = state.lock() {
            match worker {
                Some(w) => {
                    *s = SharedVideoState {
                        capturing: true,
                        frame_count: frames_submitted,
                        dropped_frame_count: w.dropped_frame_count,
                        display_name: display_name.to_string(),
                        clip_dir: w.clip_dir.to_string_lossy().into_owned(),
                    };
                }
                None => *s = SharedVideoState::default(),
            }
        }
    };
    // Frames the emu thread has submitted this session (drops included).
    // Lives alongside video_capture so we can publish without re-locking
    // worker internals.
    let mut video_frames_submitted: u64 = 0;

    // Phase 4 slice F — runtime milestone evaluator.
    //
    // `runtime` holds the parsed predicates we evaluate every frame
    // against live core memory; `prev_true` tracks whether each
    // predicate evaluated true LAST frame (for edge-detect on rising
    // transitions). Indexes parallel — `runtime[i]` ↔ `prev_true[i]`.
    //
    // `triggered_this_session` is the set of milestone IDs that have
    // already fired in this game session — once edge-triggered, a
    // milestone with `edge_only = true` is suppressed until either
    // (a) the game is reloaded or (b) the operator explicitly resets
    // it via `reset_milestone_progress`.
    struct MilestoneRuntime {
        id: i64,
        name: String,
        region: oa_core::MemoryRegionId,
        offset: u32,
        width: u8,
        op: MilestoneOp,
        target: i64,
        edge_only: bool,
        /// Mirrors `triggered_at_unix_ms IS NOT NULL` at load time so
        /// already-unlocked milestones don't re-fire on next launch.
        already_triggered: bool,
    }
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum MilestoneOp { Eq, Neq, Gt, Lt, Geq, Leq }
    impl MilestoneOp {
        fn parse(s: &str) -> Option<Self> {
            match s {
                "eq" => Some(Self::Eq),
                "neq" => Some(Self::Neq),
                "gt" => Some(Self::Gt),
                "lt" => Some(Self::Lt),
                "geq" => Some(Self::Geq),
                "leq" => Some(Self::Leq),
                _ => None,
            }
        }
        fn eval(self, lhs: i64, rhs: i64) -> bool {
            match self {
                Self::Eq => lhs == rhs,
                Self::Neq => lhs != rhs,
                Self::Gt => lhs > rhs,
                Self::Lt => lhs < rhs,
                Self::Geq => lhs >= rhs,
                Self::Leq => lhs <= rhs,
            }
        }
    }
    /// Read an LE integer of `width` bytes at `offset` from `region`.
    /// Returns None if the read would go out of bounds, the region is
    /// unavailable, or the width is unsupported (1 / 2 / 4 only).
    fn read_memory_le(
        core: &dyn oa_core::Core,
        region: oa_core::MemoryRegionId,
        offset: u32,
        width: u8,
    ) -> Option<i64> {
        let bytes = core.memory_region(region)?;
        let off = offset as usize;
        match width {
            1 => bytes.get(off).map(|b| *b as i64),
            2 => bytes.get(off..off + 2).map(|s| {
                u16::from_le_bytes([s[0], s[1]]) as i64
            }),
            4 => bytes.get(off..off + 4).map(|s| {
                u32::from_le_bytes([s[0], s[1], s[2], s[3]]) as i64
            }),
            _ => None,
        }
    }
    let mut milestone_runtime: Vec<MilestoneRuntime> = Vec::new();
    // RetroArch parity slice 5 — armed cheats. Enabled rows write
    // `value` to `(region, offset)` after every NORMAL/REPLAY run_frame.
    let mut cheat_runtime: Vec<library_db::Cheat> = Vec::new();

    // Run-ahead state. Hoisted buffers avoid per-frame allocation —
    // save_buf holds a state snapshot we'll roll back to after the
    // future-frame peek; audio_buf carries the REAL audio (drained
    // between the real run_frame and the peek frames) since the post-
    // peek drain would mix in samples from frames the user shouldn't
    // hear yet. Disabled when SetRunAhead(0); only fires on the truly-
    // normal play branch (skipped in scrub / TAS replay / TAS record /
    // pause / fast-forward / slow-motion).
    let mut run_ahead_frames: u32 = 0;
    let mut run_ahead_save_buf: Vec<u8> = Vec::new();
    let mut run_ahead_audio_buf: Vec<i16> = Vec::new();
    let mut milestone_prev_true: Vec<bool> = Vec::new();

    // Mutable because a per-game core swap may load a core with different fps
    // (rare but possible — SuperGrafx vs PC Engine timing aren't identical).
    let mut frame_period = Duration::from_secs_f64(1.0 / timing.fps);
    let started = Instant::now();
    let mut next_frame = Instant::now();
    let mut frame_n: u64 = 0;
    let mut last_size = initial_size;

    while running.load(Ordering::SeqCst) {
        // Drain command channel non-blockingly. LoadRom hot-swaps the active
        // ROM; the PCE shim's load_rom calls retro_unload_game first so back-
        // to-back swaps are safe.
        loop {
            match cmd_rx.try_recv() {
                Ok(EmuCommand::LoadRom { path, bytes, restore_slot, restore_state_path, core_override, system_id }) => {
                    log::info!("oa-shell: launch_rom -> {} ({} bytes, restore_slot={:?}, restore_state_path={:?}, override={:?}, system={})", path, bytes.len(), restore_slot, restore_state_path, core_override, system_id);
                    let system_changed = current_system_id != system_id;
                    current_system_id = system_id.clone();

                    // Re-apply the per-system bindings to the input poller
                    // whenever the active system changes. Different systems
                    // use different bit positions (PCE clockwise vs
                    // libretro's UP/DOWN/LEFT/RIGHT order) — without this
                    // the InputPoller's slot table is still pointed at the
                    // previous system's bits, and the new system's identity
                    // remap reads the keys at the wrong libretro positions.
                    // `apply_bindings_to_poller` clears the port first so
                    // stale slots can't leak through.
                    if system_changed {
                        let new_bindings = bindings::load(&app_data_dir, &system_id);
                        apply_bindings_to_poller(&mut input, &system_id, &new_bindings);
                        log::info!(
                            "oa-shell: system swap -> {} ({} buttons rebound on Port0)",
                            system_id, new_bindings.len(),
                        );
                    }

                    // Resolve which DLL this ROM should run on. Precedence:
                    //   per-game override → per-system pref (cores.json) → system-specific hardcoded default
                    // (the per-system pref + the bootstrap-system default were
                    // already resolved at startup into `current_core_dll`; we
                    // re-read cores.json so a pref change in Settings takes
                    // effect without needing a per-game override).
                    let per_system_default = default_core_dll_for_system(&system_id).to_string();
                    let per_system_pref = read_cores_pref(&app_data_dir)
                        .get(&system_id)
                        .cloned()
                        .unwrap_or(per_system_default);
                    let target_dll: String = core_override.clone().unwrap_or(per_system_pref);

                    // Trigger a (re)load when:
                    //   - a different DLL is requested (core swap), OR
                    //   - the core slot is empty (post-UnloadRom — see the
                    //     UnloadRom handler for why we drop the whole core).
                    let is_swap = target_dll != current_core_dll;
                    let needs_core_load = is_swap || core.is_none();
                    if needs_core_load {
                        let target_path = cores_dir.join(&target_dll);
                        let context = if is_swap { "core swap" } else { "core reload (post-unload)" };
                        log::info!("oa-shell: {context} requested {} -> {} ({})", current_core_dll, target_dll, target_path.display());
                        if !target_path.is_file() {
                            log::error!("oa-shell: {context} aborted — {} not found in {}; keeping current core", target_dll, cores_dir.display());
                            toast(&app_handle, ToastLevel::Error, format!("Core not found: {target_dll}"));
                        } else if let Err(e) = oa_libretro::probe(&target_path) {
                            log::error!("oa-shell: {context} aborted — probe {} failed: {e:?}; keeping current core", target_dll);
                            toast(&app_handle, ToastLevel::Error, format!("Core probe failed: {target_dll}"));
                        } else {
                            // Drop current to release the libretro singleton before loading the new one.
                            let _ = core.take();
                            match LibretroCore::load(&target_path, parse_system_id(&system_id), &system_dir, &save_dir) {
                                Ok(new_core) => {
                                    let new_timing = new_core.timing();
                                    log::info!(
                                        "oa-shell: {} loaded {} (timing {}x{} @ {:.3} Hz, audio {} Hz)",
                                        context, target_dll, new_timing.width, new_timing.height, new_timing.fps, new_timing.sample_rate
                                    );
                                    // Only toast on a real swap — the silent
                                    // post-unload reload shouldn't surface a
                                    // "Loaded core: …" toast on every relaunch.
                                    if is_swap {
                                        toast(&app_handle, ToastLevel::Success, format!("Loaded core: {target_dll}"));
                                    }
                                    if new_timing.sample_rate != timing.sample_rate {
                                        let device_pref = read_audio_pref(&app_data_dir);
                                        audio = match oa_audio::AudioSink::with_device(new_timing.sample_rate, device_pref.as_deref()) {
                                            Ok(a) => {
                                                log::info!("oa-shell: audio sink rebuilt for new core ({} Hz)", a.sample_rate());
                                                Some(a)
                                            }
                                            Err(e) => {
                                                log::warn!("oa-shell: audio sink rebuild after core swap failed ({e:?}); silent");
                                                None
                                            }
                                        };
                                    }
                                    frame_period = Duration::from_secs_f64(1.0 / new_timing.fps);
                                    timing = new_timing;
                                    core = Some(new_core);
                                    current_core_dll = target_dll;
                                }
                                Err(e) => {
                                    log::error!("oa-shell: core load failed for {}: {e:?}; emulation halted until next successful LoadRom", target_path.display());
                                    toast(&app_handle, ToastLevel::Error, format!("Core load failed: {target_dll}"));
                                    // core is None — frame body will skip emulation.
                                    continue;
                                }
                            }
                        }
                    }

                    let Some(core_ref) = core.as_mut() else { continue; };

                    let ext = Path::new(&path)
                        .extension()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_ascii_lowercase())
                        .unwrap_or_else(|| "pce".to_string());
                    let source = if is_cd_extension(&ext) {
                        oa_libretro::RomSource::Path(Path::new(&path))
                    } else {
                        oa_libretro::RomSource::Bytes(&bytes)
                    };
                    // Compute ROM SHA-1 for TAS replay-safety stamping
                    // (slice C). Bytes-source only — Path-source CD
                    // images vary too much in layout for a meaningful
                    // single-file hash. Empty string = "unknown" and
                    // TAS recording falls back to filename-only safety.
                    current_rom_sha1_hex = match &source {
                        oa_libretro::RomSource::Bytes(data) => {
                            use sha1::{Digest, Sha1};
                            let hash = Sha1::digest(data);
                            hash.iter().map(|b| format!("{:02X}", b)).collect::<String>()
                        }
                        oa_libretro::RomSource::Path(_) => String::new(),
                    };
                    // CD images need a BIOS in <exe_dir>/system/. Validate
                    // its SHA-1 against known-good Mednafen-canonical dumps
                    // — wrong content (vs same size) typically means the core
                    // crashes during CD init with an access violation rather
                    // than failing cleanly. Refuse the launch with a clear
                    // message instead so the user can fix the BIOS file.
                    if is_cd_extension(&ext) {
                        // Per-system BIOS pre-check. Each CD-shape system
                        // has its own canonical BIOS filenames + hashes +
                        // missing-BIOS guidance, so the dispatch + messages
                        // live next to the system_id check. Other CD-shape
                        // systems (saturn, dreamcast, psx, 3do, pcfx, neocd)
                        // land here as they onboard.
                        let bios_check = match system_id.as_str() {
                            "pce-cd" => Some((
                                "PCE-CD",
                                "syscard3.pce",
                                check_pce_cd_bios(&system_dir),
                                "1F8B161A2DB40DBA2079A87C10C0A3340B56ED3B",
                                "the canonical v3.00 dump",
                            )),
                            "segacd" => Some((
                                "Sega CD",
                                "bios_CD_U.bin / bios_CD_J.bin / bios_CD_E.bin",
                                check_sega_cd_bios(&system_dir),
                                "F4F315ADCEF9B8FEB0364C21AB7F0EAF5457F3ED",
                                "the canonical US Sega CD v1.10 dump (libretro-database)",
                            )),
                            "saturn" => Some((
                                "Saturn",
                                "saturn_bios.bin / sega_100.bin / sega_101.bin / mpr-17933.bin",
                                check_saturn_bios(&system_dir),
                                "2B8CB4F87580683EB4D760E4ED210813D667F0A2",
                                "the canonical JP Saturn v1.00 dump (libretro-database)",
                            )),
                            "psx" => Some((
                                "PSX",
                                "scph5500.bin / scph5501.bin / scph5502.bin (v3.0 regional set)",
                                check_psx_bios(&system_dir),
                                "0555C6FAE8906F3F09BAF5988F00E55F88E9F30B",
                                "the canonical US PSX v3.0 SCPH-5501 dump (libretro-database)",
                            )),
                            "neocd" => Some((
                                "Neo Geo CD",
                                "neocd.bin / neocd_z.rom (top-loader) / neocd_t.rom (front-loader)",
                                check_neocd_bios(&system_dir),
                                "7BB26D1E5D1E930515219CB18BCDE5B7B23E2EDA",
                                "the canonical NeoCD boot ROM (libretro-database)",
                            )),
                            "3do" => Some((
                                "3DO",
                                "panafz1.bin / panafz10.bin / goldstar.bin / sanyotry.bin",
                                check_3do_bios(&system_dir),
                                "34BF189111295F74D7B7DFC1F304D98B8D36325A",
                                "the canonical Panasonic FZ-1 BIOS (libretro-database)",
                            )),
                            "pcfx" => Some((
                                "PC-FX",
                                "pcfx.rom / pcfxbios.bin",
                                check_pcfx_bios(&system_dir),
                                "1A77FD83E337F906AECAB27A1604DB064CF10074",
                                "the canonical PC-FX v1.00 BIOS (libretro-database)",
                            )),
                            "dreamcast" => Some((
                                "Dreamcast",
                                "dc_boot.bin + dc_flash.bin",
                                check_dreamcast_bios(&system_dir),
                                "8951D1BB219AB2FF8583033D2119C899CC81F18C",
                                "the canonical Dreamcast boot ROM (libretro-database)",
                            )),
                            "ps2" => Some((
                                "PS2",
                                "scph10000.bin / scph39001.bin / scph70000.bin / scph77001.bin / scph90001.bin (or ps2-XXXX-YYYYMMDD.bin PCSX2-style names)",
                                check_ps2_bios(&system_dir),
                                "F9A5D629A036B99128F7CB530C6E3CA016E9C8B7",
                                "the canonical US PS2 v1.60 BIOS (libretro-database)",
                            )),
                            _ => None,
                        };
                        if let Some((label, expected_files, result, canonical_sha, canonical_desc)) = bios_check {
                            log::info!(
                                "oa-shell: {} CD load — system_dir = {} (drop BIOS files here, e.g. {})",
                                label, system_dir.display(), expected_files,
                            );
                            match result {
                                Ok(BiosCheck::OkCanonical { name, sha1 }) => {
                                    log::info!("oa-shell: {} CD load — BIOS {} verified (SHA-1 {})", label, name, sha1);
                                }
                                Ok(BiosCheck::OkUnknownHash { name, sha1 }) => {
                                    log::warn!(
                                        "oa-shell: {} CD load — BIOS {} has SHA-1 {} which doesn't match a known-good dump. Game may crash; if so, replace with {} (SHA-1 {}).",
                                        label, name, sha1, canonical_desc, canonical_sha,
                                    );
                                    toast(&app_handle, ToastLevel::Warn, format!("BIOS {name} SHA-1 unverified — game may crash"));
                                }
                                Err(BiosError::Missing) => {
                                    log::error!(
                                        "oa-shell: {} CD load aborted — no BIOS in {}. Drop a canonical {} dump there.",
                                        label, system_dir.display(), expected_files,
                                    );
                                    toast(&app_handle, ToastLevel::Error, format!("No {label} BIOS in system/ — drop {expected_files} there"));
                                    continue;
                                }
                                Err(BiosError::Io(e)) => {
                                    log::error!("oa-shell: {} CD load — BIOS check failed: {e:?}", label);
                                    toast(&app_handle, ToastLevel::Warn, format!("BIOS check failed: {e}"));
                                }
                            }
                        }
                    }
                    // Cart-shape BIOS pre-check for Neo Geo. Unlike the
                    // CD-shape systems, Neo Geo's BIOS is a multi-ROM
                    // .zip (`neogeo.zip`) whose content SHA-1 varies
                    // by MAME revision + Universe BIOS presence —
                    // existence-only check at Phase 0. FBNeo handles
                    // the content validation internally if the file
                    // exists. Other cart-shape systems land here as
                    // they require BIOS pre-checks (currently only
                    // Neo Geo; cart genesis/sega32x/nes etc. don't
                    // need a BIOS).
                    if system_id == "nds" {
                        log::info!(
                            "oa-shell: NDS load — system_dir = {} (drop bios7.bin + bios9.bin + firmware.bin here)",
                            system_dir.display()
                        );
                        match check_nds_bios(&system_dir) {
                            Ok(BiosCheck::OkCanonical { name, sha1 }) => {
                                log::info!("oa-shell: NDS load — BIOS {} verified ({})", name, sha1);
                            }
                            Ok(BiosCheck::OkUnknownHash { name, sha1 }) => {
                                log::warn!(
                                    "oa-shell: NDS load — BIOS file {} has SHA-1 {} which doesn't match a known-good melonDS dump. Game may crash; if so, replace with the canonical v1.0 dumps.",
                                    name, sha1,
                                );
                                toast(&app_handle, ToastLevel::Warn, format!("NDS BIOS {name} unverified — game may crash"));
                            }
                            Err(BiosError::Missing) => {
                                log::error!(
                                    "oa-shell: NDS load aborted — bios7.bin + bios9.bin + firmware.bin must all be present in {}.",
                                    system_dir.display()
                                );
                                toast(&app_handle, ToastLevel::Error, "No NDS BIOS in system/ — drop bios7.bin + bios9.bin + firmware.bin there");
                                continue;
                            }
                            Err(BiosError::Io(e)) => {
                                log::error!("oa-shell: NDS load — BIOS check failed: {e:?}");
                                toast(&app_handle, ToastLevel::Warn, format!("BIOS check failed: {e}"));
                            }
                        }
                    }
                    if system_id == "neogeo" {
                        log::info!(
                            "oa-shell: Neo Geo load — system_dir = {} (drop neogeo.zip here)",
                            system_dir.display()
                        );
                        match check_neogeo_bios(&system_dir) {
                            Ok(BiosCheck::OkCanonical { name, sha1 }) => {
                                log::info!("oa-shell: Neo Geo load — BIOS {} present ({})", name, sha1);
                            }
                            Ok(BiosCheck::OkUnknownHash { name, sha1 }) => {
                                // Existence-only check never returns
                                // OkUnknownHash today, but the arm
                                // stays for forward compat with Phase 2
                                // content-validation polish.
                                log::warn!(
                                    "oa-shell: Neo Geo load — BIOS {} has unexpected content ({}); FBNeo will validate.",
                                    name, sha1,
                                );
                            }
                            Err(BiosError::Missing) => {
                                log::error!(
                                    "oa-shell: Neo Geo load aborted — no neogeo.zip in {}. Drop the canonical Neo Geo BIOS ROM-set there.",
                                    system_dir.display()
                                );
                                toast(&app_handle, ToastLevel::Error, "No Neo Geo BIOS in system/ — drop neogeo.zip there");
                                continue;
                            }
                            Err(BiosError::Io(e)) => {
                                log::error!("oa-shell: Neo Geo load — BIOS check failed: {e:?}");
                                toast(&app_handle, ToastLevel::Warn, format!("BIOS check failed: {e}"));
                            }
                        }
                    }
                    // Cart-shape BIOS pre-checks for systems whose libretro
                    // cores hard-require a BIOS to boot (Coleco / Intv / O2
                    // / Channel F). Same shape as the neogeo / nds checks
                    // above. Each is its own block so the error message can
                    // name exactly which file(s) the operator needs to drop.
                    let cart_bios = match system_id.as_str() {
                        "coleco" => Some((
                            "ColecoVision",
                            "colecovision.rom",
                            check_coleco_bios(&system_dir),
                            "45BEDC4CBDEAC66C7DF59E9E599195C778D86A92",
                            "the canonical 8 KB ColecoVision BIOS (libretro-database)",
                        )),
                        "intv" => Some((
                            "Intellivision",
                            "exec.bin + grom.bin",
                            check_intv_bios(&system_dir),
                            "5A65B922B562CB1F57DAB51B73151283F0E20C7A",
                            "exec.bin + grom.bin canonical dumps (libretro-database)",
                        )),
                        "o2" => Some((
                            "Odyssey²/Videopac",
                            "o2rom.bin (or g7400.bin / c52.bin / jopac.bin)",
                            check_o2_bios(&system_dir),
                            "B2E1955D957A475DE2411770452EFF4EA19F4CEE",
                            "the canonical o2rom.bin dump (libretro-database)",
                        )),
                        "channelf" => Some((
                            "Channel F",
                            "sl31253.bin + sl31254.bin (sl90025.bin optional)",
                            check_channelf_bios(&system_dir),
                            "81193965A374D77B99B4743D317824B53C3E3C78",
                            "sl31253.bin + sl31254.bin canonical dumps (libretro-database)",
                        )),
                        "5200" => Some((
                            "Atari 5200",
                            "5200.rom",
                            check_atari5200_bios(&system_dir),
                            "6AD7A1E8C9FAD486FBEC9498CB48BF5BC3ADC530",
                            "the canonical 5200.rom BIOS (libretro-database)",
                        )),
                        "pokemini" => Some((
                            "Pokémon Mini",
                            "bios.min",
                            check_pokemini_bios(&system_dir),
                            "DAAD4113713ED776FBD47727762BCA81BA74915F",
                            "the canonical bios.min boot ROM (libretro-database)",
                        )),
                        _ => None,
                    };
                    if let Some((label, expected_files, result, canonical_sha, canonical_desc)) = cart_bios {
                        log::info!(
                            "oa-shell: {} load — system_dir = {} (drop BIOS files here, e.g. {})",
                            label, system_dir.display(), expected_files,
                        );
                        match result {
                            Ok(BiosCheck::OkCanonical { name, sha1 }) => {
                                log::info!("oa-shell: {} load — BIOS {} verified (SHA-1 {})", label, name, sha1);
                            }
                            Ok(BiosCheck::OkUnknownHash { name, sha1 }) => {
                                log::warn!(
                                    "oa-shell: {} load — BIOS {} has SHA-1 {} which doesn't match a known-good dump. Game may crash; if so, replace with {} (canonical SHA-1 {}).",
                                    label, name, sha1, canonical_desc, canonical_sha,
                                );
                                toast(&app_handle, ToastLevel::Warn, format!("BIOS {name} SHA-1 unverified — game may crash"));
                            }
                            Err(BiosError::Missing) => {
                                log::error!(
                                    "oa-shell: {} load aborted — no BIOS in {}. Drop a canonical {} dump there.",
                                    label, system_dir.display(), expected_files,
                                );
                                toast(&app_handle, ToastLevel::Error, format!("No {label} BIOS in system/ — drop {expected_files} there"));
                                continue;
                            }
                            Err(BiosError::Io(e)) => {
                                log::error!("oa-shell: {} load — BIOS check failed: {e:?}", label);
                                toast(&app_handle, ToastLevel::Warn, format!("BIOS check failed: {e}"));
                            }
                        }
                    }
                    // Pre-compute the stem — cores read it via info_ext->name
                    // during retro_load_game (e.g. FCEUmm uses it to derive
                    // <save_dir>/<name>.sav). Passing the actual ROM stem
                    // beats the State::new() default `"rom"`.
                    let stem = sanitize_stem(&path);
                    match core_ref.load_rom(source, &ext, &stem) {
                        Ok(()) => {
                            log::info!("oa-shell: ROM swap OK; save-state dir = {}/saves/{}", app_data_dir.display(), stem);
                            current_rom_stem = Some(stem.clone());

                            // Phase 2.5 — push the core's display rotation to
                            // the renderer. Vertical arcade boards (Pac-Man,
                            // Galaxian, Donkey Kong, …) set this via
                            // RETRO_ENVIRONMENT_SET_ROTATION during load;
                            // everything else stays at 0.
                            let rot = core_ref.rotation();
                            renderer.set_rotation(rot);
                            if rot != 0 {
                                log::info!(
                                    "oa-shell: core requested rotation {} (× 90° CW)",
                                    rot
                                );
                            }

                            // Phase 6 Cross-system slice 2 — refresh the
                            // keyboard-passthrough flag now that the active
                            // system may have changed. If the new system
                            // wants passthrough OFF and we have held keys
                            // pending, the next frame's pump will see
                            // active=false and emit release events for
                            // every still-held key so the previous system's
                            // core doesn't think they're stuck (defensive
                            // against the prior core surviving across the
                            // swap, which is rare but possible).
                            let new_settings = system_settings::read_system_settings(
                                &app_data_dir,
                                &current_system_id,
                            );
                            keyboard_passthrough_active = system_settings::effective_keyboard_passthrough(
                                &current_system_id,
                                &new_settings,
                            );
                            log::info!(
                                "oa-shell: keyboard passthrough for {} = {}",
                                current_system_id, keyboard_passthrough_active,
                            );

                            // Phase 2.5 — apply per-system analog routing
                            // baseline for all 5 ports. Per-game overrides
                            // (which stack on top) get layered in by the
                            // frontend's post-launch `arm_analog_routing`
                            // call — that resolves per-game → per-system
                            // → identity per port and fires individual
                            // SetAnalogRouting commands. We don't have the
                            // current game's id here so the per-system pass
                            // is all we can do automatically.
                            //
                            // Resolution: explicit user setting → compiled-in
                            // per-system default (today N64 = WASD on the left
                            // stick) → identity. Same chain as
                            // `arm_analog_routing` so the LoadRom baseline
                            // and the post-launch game-level pass agree on
                            // the per-system layer.
                            let analog_sys = new_settings.analog_routing.clone()
                                .or_else(|| system_settings::default_analog_routing(&current_system_id))
                                .unwrap_or_else(system_settings::AnalogRoutingPrefs::identity);
                            for port_idx in 0u32..5 {
                                let port = match port_idx {
                                    0 => oa_core::PortIndex::Port0,
                                    1 => oa_core::PortIndex::Port1,
                                    2 => oa_core::PortIndex::Port2,
                                    3 => oa_core::PortIndex::Port3,
                                    _ => oa_core::PortIndex::Port4,
                                };
                                input.set_analog_routing(
                                    port,
                                    analog_sys.port_routing(port_idx).to_runtime(),
                                );
                            }

                            // RetroArch-parity slice — refresh the cached
                            // disc-control snapshot. None for HuCard / cart
                            // games; populated for multi-disc CD images.
                            if let Ok(mut s) = disc_state.lock() {
                                *s = core_ref.disc_state();
                            }

                            // RetroArch-parity slice — capture the core's
                            // freshly-registered option schema to disk so
                            // the per-system settings page can render the
                            // option list even when no core is running.
                            // Preserves any existing user values whose keys
                            // still appear in the new schema (a core update
                            // can remove options).
                            let schema = core_ref.options();
                            let categories = core_ref.option_categories();
                            if !schema.is_empty() {
                                // Apply the effective per-system + per-game
                                // overrides BEFORE asking the core which
                                // options it wants hidden — the visibility
                                // hints depend on the current values, and
                                // overrides on disk may differ from the
                                // schema defaults the core saw during init.
                                let file = core_options::read(&app_data_dir, &current_system_id);
                                let merged = core_options::build_effective_values(
                                    &schema,
                                    &file.values,
                                    &std::collections::HashMap::new(),
                                );
                                for (k, v) in &merged {
                                    core_ref.set_option(k, v);
                                }
                                // Now poke the core to re-evaluate visibility
                                // against the overridden values; cores that
                                // registered the update-display callback
                                // fire SET_CORE_OPTIONS_DISPLAY re-entrantly
                                // here, populating State.hidden_options.
                                core_ref.refresh_option_visibility();
                                let hidden_keys = core_ref.hidden_option_keys();
                                if let Err(e) = core_options::refresh_schema(
                                    &app_data_dir,
                                    &current_system_id,
                                    schema.clone(),
                                    categories,
                                    hidden_keys.clone(),
                                ) {
                                    log::warn!("oa-shell: core_options refresh_schema failed: {e}");
                                }
                                log::info!(
                                    "oa-shell: captured {} core option(s) for {} ({} hidden) + applied",
                                    schema.len(),
                                    current_system_id,
                                    hidden_keys.len(),
                                );
                            }
                            current_slot = restore_slot.unwrap_or(0);
                            // Snapshots from any previous game are unsafe to
                            // feed back into a different core — drop them on
                            // every successful load. Also drop any stale
                            // scrub state since the ring is now empty.
                            rewind_ring.clear();
                            scrubbing = false;
                            scrub_position = 0;
                            publish_rewind_state(&rewind_ring, &rewind_config, &timing, scrubbing, scrub_position, &rewind_state);
                            // Phase 4 slice C — also drop any in-progress
                            // TAS recording / replay. A different ROM is
                            // loaded; the recording's initial state would
                            // mean nothing now. We discard rather than
                            // auto-save because the user hasn't explicitly
                            // asked to keep this clip.
                            if tas_recording.take().is_some() {
                                log::warn!("oa-shell: discarded in-progress TAS recording (new ROM loaded)");
                                toast(&app_handle, ToastLevel::Warn, "TAS recording discarded — new ROM loaded");
                            }
                            tas_replay = None;
                            tas_replay_current_frame = 0;
                            publish_tas_state(TasMode::Idle, 0, 0, "", &tas_state);
                            // Phase 4 slice D — same treatment for in-flight
                            // video capture. Drop the clip dir on the way out.
                            if let Some(worker) = video_capture.take() {
                                let stem_now = current_rom_stem.clone().unwrap_or_else(|| "unknown".into());
                                let _ = worker.stop_and_finalize(&current_system_id, &stem_now, &video_display_name, timing.fps, video_first_size.0, video_first_size.1, true);
                                log::warn!("oa-shell: discarded in-progress video capture (new ROM loaded)");
                                toast(&app_handle, ToastLevel::Warn, "Video capture discarded — new ROM loaded");
                            }
                            video_first_size = (0, 0);
                            video_display_name.clear();
                            video_frames_submitted = 0;
                            publish_video_state(None, 0, "", &video_state);
                            // Slice E — clear stale memory snapshot.
                            // The frontend will re-seed by polling
                            // via get_memory_region.
                            if let Ok(mut s) = memory_snapshot.lock() {
                                *s = MemorySnapshot::default();
                            }
                            // Slice F — clear runtime milestones; the
                            // frontend pushes the new game's set via
                            // LoadMilestones right after.
                            milestone_runtime.clear();
                            milestone_prev_true.clear();
                            // Cheats — same lifecycle as milestones.
                            // Frontend re-arms via LoadCheats after
                            // launch resolves the per-game list.
                            cheat_runtime.clear();
                            if let Some(slot) = restore_slot {
                                let p = slot_path(&app_data_dir, &stem, slot);
                                match std::fs::read(&p) {
                                    Ok(buf) => match core_ref.load_state(&mut &buf[..]) {
                                        Ok(()) => log::info!(
                                            "oa-shell: post-load restore — slot {} from {} ({} bytes)",
                                            slot, p.display(), buf.len()
                                        ),
                                        Err(e) => log::warn!("oa-shell: post-load restore deserialize failed: {e:?}"),
                                    },
                                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                                        log::warn!("oa-shell: post-load restore — slot {} not present ({})", slot, p.display());
                                    }
                                    Err(e) => log::warn!("oa-shell: post-load restore read {} failed: {e:?}", p.display()),
                                }
                            } else if let Some(p) = restore_state_path.as_ref() {
                                // Direct-launch --state-file: load an arbitrary
                                // state file (full path, not slot-directory).
                                // CLI parsing already validated the path exists
                                // + that --slot isn't also set, so this branch
                                // only fires when the operator explicitly asked.
                                match std::fs::read(p) {
                                    Ok(buf) => match core_ref.load_state(&mut &buf[..]) {
                                        Ok(()) => log::info!(
                                            "oa-shell: post-load restore — state file {} ({} bytes)",
                                            p.display(), buf.len()
                                        ),
                                        Err(e) => {
                                            log::warn!("oa-shell: post-load state-file deserialize failed: {e:?}");
                                            toast(
                                                &app_handle,
                                                ToastLevel::Warn,
                                                format!("state-file restore failed: {e}"),
                                            );
                                        }
                                    },
                                    Err(e) => {
                                        log::warn!(
                                            "oa-shell: post-load state-file read {} failed: {e:?}",
                                            p.display()
                                        );
                                        toast(
                                            &app_handle,
                                            ToastLevel::Warn,
                                            format!("state-file read failed: {e}"),
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("oa-shell: load_rom failed for {}: {e:?}", path);
                            toast(&app_handle, ToastLevel::Error, format!("ROM rejected: {e}"));
                        }
                    }
                }
                Ok(EmuCommand::UnloadRom { title }) => {
                    let had_rom = core.as_ref().map(|c| c.has_rom()).unwrap_or(false);
                    if had_rom {
                        log::info!("oa-shell: ROM unloaded (was {:?})", current_rom_stem);
                        let msg = match title.as_deref() {
                            Some(t) if !t.is_empty() => format!("Unloaded {t}"),
                            _ => "Unloaded".to_string(),
                        };
                        toast(&app_handle, ToastLevel::Success, msg);
                        current_rom_stem = None;
                        current_slot = 0;
                        rewind_ring.clear();
                        scrubbing = false;
                        scrub_position = 0;
                        publish_rewind_state(&rewind_ring, &rewind_config, &timing, scrubbing, scrub_position, &rewind_state);
                        current_rom_sha1_hex.clear();
                        if tas_recording.take().is_some() {
                            log::warn!("oa-shell: discarded in-progress TAS recording (ROM unloaded)");
                        }
                        tas_replay = None;
                        tas_replay_current_frame = 0;
                        publish_tas_state(TasMode::Idle, 0, 0, "", &tas_state);
                        if let Some(worker) = video_capture.take() {
                            let stem_now = current_rom_stem.clone().unwrap_or_else(|| "unknown".into());
                            let _ = worker.stop_and_finalize(&current_system_id, &stem_now, &video_display_name, timing.fps, video_first_size.0, video_first_size.1, true);
                            log::warn!("oa-shell: discarded in-progress video capture (ROM unloaded)");
                        }
                        video_first_size = (0, 0);
                        video_display_name.clear();
                        video_frames_submitted = 0;
                        publish_video_state(None, 0, "", &video_state);
                        if let Ok(mut s) = memory_snapshot.lock() {
                            *s = MemorySnapshot::default();
                        }
                        if let Ok(mut s) = perf_stats.lock() {
                            *s = SharedPerfStats::default();
                        }
                        milestone_runtime.clear();
                        milestone_prev_true.clear();
                    } else if core.is_none() {
                        log::info!("oa-shell: UnloadRom — no core loaded");
                        toast(&app_handle, ToastLevel::Info, "No core loaded");
                    } else {
                        log::info!("oa-shell: UnloadRom — no ROM loaded");
                        toast(&app_handle, ToastLevel::Info, "No ROM loaded");
                    }
                    // Drop the entire core, not just the ROM. Mednafen-derived
                    // cores don't recover from `retro_unload_game` followed by
                    // a gap of frames before the next `retro_load_game` —
                    // STATUS_ACCESS_VIOLATION in retro_load_game on relaunch.
                    // The next LoadRom rebuilds the core fresh (~50 ms).
                    let _ = core.take();
                    // Notify the frontend that the unload drain has completed.
                    // Direct-launch listens and calls `quit_app` — no library
                    // to return to, so the process exits cleanly after the
                    // emu-thread state has fully reset (saves, temp dirs, etc.
                    // already torn down by the handler above).
                    if let Err(e) = app_handle.emit("oa://rom-unloaded", ()) {
                        log::warn!("oa-shell: emit oa://rom-unloaded failed: {e:?}");
                    }
                }
                Ok(EmuCommand::SetScalingMode(mode)) => {
                    renderer.set_scaling_mode(mode);
                }
                Ok(EmuCommand::ApplyShaderPreset(resolved)) => {
                    renderer.set_shader_preset(resolved.base);
                    if let Some(amt) = resolved.bloom_amount {
                        renderer.set_bloom_amount(amt);
                    }
                    match resolved.bezel {
                        Some(b) => {
                            if let Err(e) = renderer.set_bezel_image(&b.rgba, b.width, b.height) {
                                log::warn!("oa-shell: set_bezel_image failed: {e}");
                                renderer.clear_bezel_image();
                            }
                        }
                        None => renderer.clear_bezel_image(),
                    }
                }
                Ok(EmuCommand::SetBloomAmount(amt)) => {
                    renderer.set_bloom_amount(amt);
                }
                Ok(EmuCommand::SetDisplayAspectOverride(aspect)) => {
                    // Resolution: explicit user value (per-game or
                    // per-system override) wins; otherwise fall through
                    // to the compiled-in per-system default (today:
                    // GBA gets 3:2). `None` after that resolution
                    // means "trust the core's reported aspect" — the
                    // typical case for systems whose libretro core
                    // reports its physical aspect correctly.
                    let resolved = aspect.or_else(|| {
                        system_settings::default_display_aspect(&current_system_id)
                    });
                    renderer.set_display_aspect_override(resolved);
                }
                Ok(EmuCommand::SetOverscanCrop(crop)) => {
                    renderer.set_overscan_crop(crop);
                }
                Ok(EmuCommand::SetBezelOverride(bezel)) => {
                    match bezel {
                        Some(b) => {
                            if let Err(e) = renderer.set_bezel_image(&b.rgba, b.width, b.height) {
                                log::warn!("oa-shell: set_bezel_override failed: {e}");
                                renderer.clear_bezel_image();
                            }
                        }
                        None => {
                            // None here means "reapply whatever the
                            // active preset's TOML carried." We could
                            // re-derive that, but it's simpler to just
                            // clear and let the next ApplyShaderPreset
                            // (typically on launch) repopulate. UX:
                            // clearing the per-game/system override
                            // also clears the bezel until next launch.
                            renderer.clear_bezel_image();
                        }
                    }
                }
                Ok(EmuCommand::SetCoreOption { key, value }) => {
                    if let Some(c) = core.as_mut() {
                        c.set_option(&key, &value);
                        // Cores with dynamic visibility (libretro
                        // SET_CORE_OPTIONS_UPDATE_DISPLAY_CALLBACK) want
                        // a re-evaluation after each value change so
                        // dependent options can show/hide; no-op for
                        // cores without a callback.
                        c.refresh_option_visibility();
                        let hidden = c.hidden_option_keys();
                        if let Err(e) = core_options::refresh_visibility(
                            &app_data_dir,
                            &current_system_id,
                            hidden,
                        ) {
                            log::warn!("oa-shell: refresh_visibility failed: {e}");
                        }
                    }
                }
                Ok(EmuCommand::ApplyCoreOptions(values)) => {
                    if let Some(c) = core.as_mut() {
                        for (k, v) in values.iter() {
                            c.set_option(k, v);
                        }
                        c.refresh_option_visibility();
                        let hidden = c.hidden_option_keys();
                        if let Err(e) = core_options::refresh_visibility(
                            &app_data_dir,
                            &current_system_id,
                            hidden,
                        ) {
                            log::warn!("oa-shell: refresh_visibility failed: {e}");
                        }
                        log::info!(
                            "oa-shell: applied {} core option(s) to {}",
                            values.len(),
                            current_system_id,
                        );
                    }
                }
                Ok(EmuCommand::SetDiscEject(ejected)) => {
                    if let Some(c) = core.as_mut() {
                        c.set_disc_eject(ejected);
                        if let Ok(mut s) = disc_state.lock() {
                            *s = c.disc_state();
                        }
                        log::info!("oa-shell: disc tray eject={ejected}");
                    }
                }
                Ok(EmuCommand::SetDiscImage(idx)) => {
                    if let Some(c) = core.as_mut() {
                        c.set_disc_image(idx);
                        if let Ok(mut s) = disc_state.lock() {
                            *s = c.disc_state();
                        }
                        log::info!("oa-shell: disc swap -> {idx}");
                    }
                }
                Ok(EmuCommand::ApplyBindings(b)) => {
                    apply_bindings_to_poller(&mut input, &current_system_id, &b);
                    log::info!("oa-shell: bindings hot-reloaded ({} buttons) for {}", b.len(), current_system_id);
                }
                Ok(EmuCommand::SetAnalogRouting { port, routing }) => {
                    let p = match port {
                        0 => oa_core::PortIndex::Port0,
                        1 => oa_core::PortIndex::Port1,
                        2 => oa_core::PortIndex::Port2,
                        3 => oa_core::PortIndex::Port3,
                        _ => oa_core::PortIndex::Port4,
                    };
                    input.set_analog_routing(p, routing);
                    log::debug!("oa-shell: analog routing hot-reloaded for port {} ({})", port, current_system_id);
                }
                Ok(EmuCommand::SetPortDevice { port, device }) => {
                    // Forward to the loaded core's set_port_device. The
                    // libretro layer no-ops + logs a warning when no ROM is
                    // currently loaded; we let it absorb the call rather
                    // than gating here, because the frontend pipeline can
                    // race (the user can clear an override before a game
                    // is loaded, and the no-op behaviour is correct).
                    if let Some(core_ref) = core.as_mut() {
                        core_ref.set_port_device(port, device);
                        log::info!(
                            "oa-shell: set_port_device({}, {}) for {}",
                            port, device, current_system_id
                        );
                    } else {
                        log::debug!(
                            "oa-shell: SetPortDevice({}, {}) with no core loaded — ignored",
                            port, device
                        );
                    }
                }
                Ok(EmuCommand::SetRewindConfig(cfg)) => {
                    let prev_enabled = rewind_config.enabled;
                    rewind_config = cfg;
                    rewind_ring.set_max_bytes(cfg.max_bytes);
                    if !cfg.enabled && prev_enabled {
                        rewind_ring.clear();
                        // Drop out of scrub mode too — the user disabled
                        // rewind mid-scrub, the position has no meaning now.
                        scrubbing = false;
                        scrub_position = 0;
                    }
                    log::info!(
                        "oa-shell: rewind reconfigured (enabled={}, interval={} frames, cap={} MiB)",
                        cfg.enabled, cfg.capture_interval_frames, cfg.max_bytes / (1024 * 1024)
                    );
                    publish_rewind_state(&rewind_ring, &rewind_config, &timing, scrubbing, scrub_position, &rewind_state);
                }
                Ok(EmuCommand::StartRewindScrub) => {
                    if !rewind_config.enabled {
                        log::info!("oa-shell: StartRewindScrub ignored — rewind not enabled");
                    } else if rewind_ring.is_empty() {
                        log::info!("oa-shell: StartRewindScrub ignored — ring is empty");
                    } else if !scrubbing {
                        scrubbing = true;
                        scrub_position = 0;
                        // Force a peek+load on the next frame so the user
                        // sees a frozen frame at the live edge instead of
                        // continued forward motion.
                        scrub_dirty = true;
                        log::info!("oa-shell: rewind scrub started ({} snapshots available)", rewind_ring.len());
                        publish_rewind_state(&rewind_ring, &rewind_config, &timing, scrubbing, scrub_position, &rewind_state);
                    }
                }
                Ok(EmuCommand::SetRewindScrubPosition { steps_back }) => {
                    if scrubbing {
                        // Clamp to ring length - 1 (max valid steps_back).
                        let max = rewind_ring.len().saturating_sub(1) as u32;
                        let clamped = steps_back.min(max);
                        if clamped != scrub_position {
                            scrub_position = clamped;
                            scrub_dirty = true;
                            publish_rewind_state(&rewind_ring, &rewind_config, &timing, scrubbing, scrub_position, &rewind_state);
                        }
                    }
                    // If not scrubbing, silently ignore — the frontend may
                    // race a position update against an unrelated overlay
                    // close, and we'd rather not loudly fail on that.
                }
                Ok(EmuCommand::StartTasRecording { display_name }) => {
                    let Some(core_ref) = core.as_ref() else {
                        log::warn!("oa-shell: StartTasRecording ignored — no core loaded");
                        continue;
                    };
                    if !core_ref.has_rom() {
                        log::warn!("oa-shell: StartTasRecording ignored — no ROM loaded");
                        toast(&app_handle, ToastLevel::Warn, "No ROM loaded");
                        continue;
                    }
                    if tas_recording.is_some() {
                        log::warn!("oa-shell: StartTasRecording ignored — already recording");
                        continue;
                    }
                    if tas_replay.is_some() {
                        log::warn!("oa-shell: StartTasRecording ignored — replay in progress");
                        toast(&app_handle, ToastLevel::Warn, "Stop replay before recording");
                        continue;
                    }
                    let mut buf = Vec::new();
                    if let Err(e) = core_ref.save_state(&mut buf) {
                        log::warn!("oa-shell: StartTasRecording — save_state failed: {e:?}");
                        toast(&app_handle, ToastLevel::Error, format!("Recording start failed: {e}"));
                        continue;
                    }
                    let rec = oa_savestate::tas::TasRecording::new(
                        current_system_id.clone(),
                        current_core_dll.clone(),
                        current_rom_sha1_hex.clone(),
                        timing.fps,
                        display_name.clone(),
                        buf,
                    );
                    log::info!(
                        "oa-shell: TAS recording started ({} bytes initial state, system={}, core={})",
                        rec.initial_state.len(), current_system_id, current_core_dll
                    );
                    tas_recording = Some(rec);
                    toast(&app_handle, ToastLevel::Success, "TAS recording started");
                    publish_tas_state(TasMode::Recording, 0, 0, &display_name, &tas_state);
                }
                Ok(EmuCommand::StopTasRecording { discard }) => {
                    let Some(mut rec) = tas_recording.take() else {
                        log::info!("oa-shell: StopTasRecording — nothing to stop");
                        publish_tas_state(TasMode::Idle, 0, 0, "", &tas_state);
                        continue;
                    };
                    if discard {
                        log::info!("oa-shell: TAS recording discarded ({} frames)", rec.input_frames.len());
                        toast(&app_handle, ToastLevel::Info, "Recording discarded");
                        publish_tas_state(TasMode::Idle, 0, 0, "", &tas_state);
                        continue;
                    }
                    // Stamp finalized header fields.
                    rec.header.frame_count = rec.input_frames.len() as u64;
                    rec.header.recorded_at_unix_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as i64)
                        .unwrap_or(0);
                    // Build the on-disk path. Per-ROM directory keyed
                    // by the same `sanitize_stem(path)` the save-state
                    // slots use, so a "list recordings for this game"
                    // UI just lists the directory.
                    let stem = current_rom_stem.clone().unwrap_or_else(|| "unknown".into());
                    let timestamp_ms = rec.header.recorded_at_unix_ms.max(0) as u64;
                    let file_stem = if rec.header.display_name.trim().is_empty() {
                        format!("{}", timestamp_ms)
                    } else {
                        let safe_name = rec.header.display_name
                            .chars()
                            .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
                            .collect::<String>();
                        format!("{}-{}", timestamp_ms, safe_name)
                    };
                    let path = app_data_dir.join("tas").join(&stem).join(format!("{}.tas", file_stem));
                    match rec.write_to(&path) {
                        Ok(()) => {
                            log::info!(
                                "oa-shell: TAS recording saved ({} frames) -> {}",
                                rec.input_frames.len(), path.display()
                            );
                            toast(&app_handle, ToastLevel::Success, format!("Saved {} frames", rec.input_frames.len()));
                            if let Err(e) = app_handle.emit("oa://tas-recording-saved", path.to_string_lossy().to_string()) {
                                log::warn!("oa-shell: emit tas-recording-saved failed: {e:?}");
                            }
                        }
                        Err(e) => {
                            log::warn!("oa-shell: TAS write {} failed: {e:?}", path.display());
                            toast(&app_handle, ToastLevel::Error, format!("Save failed: {e}"));
                        }
                    }
                    publish_tas_state(TasMode::Idle, 0, 0, "", &tas_state);
                }
                Ok(EmuCommand::StartTasReplay(rec)) => {
                    let Some(core_ref) = core.as_mut() else {
                        log::warn!("oa-shell: StartTasReplay ignored — no core loaded");
                        continue;
                    };
                    if !core_ref.has_rom() {
                        log::warn!("oa-shell: StartTasReplay ignored — no ROM loaded");
                        toast(&app_handle, ToastLevel::Warn, "No ROM loaded");
                        continue;
                    }
                    if tas_recording.is_some() {
                        log::warn!("oa-shell: StartTasReplay ignored — recording in progress");
                        toast(&app_handle, ToastLevel::Warn, "Stop recording before replaying");
                        continue;
                    }
                    // Soft replay-safety check — warn on mismatch but
                    // proceed. Core may handle a "close enough" ROM.
                    if !rec.header.rom_sha1_hex.is_empty()
                        && !current_rom_sha1_hex.is_empty()
                        && rec.header.rom_sha1_hex != current_rom_sha1_hex
                    {
                        log::warn!(
                            "oa-shell: TAS replay — ROM hash mismatch (recording={}, current={}); replay may desync",
                            rec.header.rom_sha1_hex, current_rom_sha1_hex
                        );
                        toast(&app_handle, ToastLevel::Warn, "ROM hash differs — replay may desync");
                    }
                    if let Err(e) = core_ref.load_state(&mut &rec.initial_state[..]) {
                        log::warn!("oa-shell: TAS replay — load_state failed: {e:?}");
                        toast(&app_handle, ToastLevel::Error, format!("Replay start failed: {e}"));
                        continue;
                    }
                    let total = rec.input_frames.len() as u64;
                    let display = rec.header.display_name.clone();
                    log::info!(
                        "oa-shell: TAS replay started ({} frames, recording display=\"{}\")",
                        total, display
                    );
                    tas_replay = Some(*rec);
                    tas_replay_current_frame = 0;
                    // Drop the rewind ring on replay start — the ring's
                    // snapshots aren't from this state line.
                    rewind_ring.clear();
                    publish_rewind_state(&rewind_ring, &rewind_config, &timing, scrubbing, scrub_position, &rewind_state);
                    publish_tas_state(TasMode::Replaying, 0, total, &display, &tas_state);
                    toast(&app_handle, ToastLevel::Success, "Replaying");
                }
                Ok(EmuCommand::StopTasReplay) => {
                    if tas_replay.take().is_some() {
                        log::info!("oa-shell: TAS replay stopped at frame {}", tas_replay_current_frame);
                        tas_replay_current_frame = 0;
                        publish_tas_state(TasMode::Idle, 0, 0, "", &tas_state);
                        toast(&app_handle, ToastLevel::Info, "Replay stopped");
                    }
                }
                Ok(EmuCommand::StartVideoCapture { display_name }) => {
                    if video_capture.is_some() {
                        log::warn!("oa-shell: StartVideoCapture ignored — already capturing");
                        continue;
                    }
                    let Some(core_ref) = core.as_ref() else {
                        toast(&app_handle, ToastLevel::Warn, "No core loaded");
                        continue;
                    };
                    if !core_ref.has_rom() {
                        toast(&app_handle, ToastLevel::Warn, "No ROM loaded");
                        continue;
                    }
                    let stem = current_rom_stem.clone().unwrap_or_else(|| "unknown".into());
                    let timestamp_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis())
                        .unwrap_or(0);
                    let safe_name = if display_name.trim().is_empty() {
                        format!("{}", timestamp_ms)
                    } else {
                        let safe = display_name
                            .chars()
                            .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
                            .collect::<String>();
                        format!("{}-{}", timestamp_ms, safe)
                    };
                    let clip_dir = app_data_dir.join("clips").join(&stem).join(&safe_name);
                    match video_capture::start(clip_dir.clone()) {
                        Ok(worker) => {
                            log::info!(
                                "oa-shell: video capture started -> {} (display=\"{}\")",
                                clip_dir.display(), display_name
                            );
                            toast(&app_handle, ToastLevel::Success, "Video capture started");
                            let fb = core_ref.framebuffer();
                            video_first_size = (fb.width, fb.height);
                            video_display_name = display_name.clone();
                            video_frames_submitted = 0;
                            video_capture = Some(worker);
                            publish_video_state(video_capture.as_ref(), 0, &display_name, &video_state);
                        }
                        Err(e) => {
                            log::warn!("oa-shell: video capture start failed: {e:?}");
                            toast(&app_handle, ToastLevel::Error, format!("Capture failed: {e}"));
                        }
                    }
                }
                Ok(EmuCommand::LoadMilestones(list)) => {
                    milestone_runtime.clear();
                    milestone_prev_true.clear();
                    for m in &list {
                        // Tolerate malformed rows by skipping with a warn
                        // — the editor UI should validate, but a corrupt
                        // DB shouldn't crash the emu thread.
                        let Some(region) = oa_core::MemoryRegionId::parse(&m.region) else {
                            log::warn!("oa-shell: milestone {:?} skipped — unknown region {:?}", m.name, m.region);
                            continue;
                        };
                        let Some(op) = MilestoneOp::parse(&m.op) else {
                            log::warn!("oa-shell: milestone {:?} skipped — unknown op {:?}", m.name, m.op);
                            continue;
                        };
                        if !matches!(m.width, 1 | 2 | 4) {
                            log::warn!("oa-shell: milestone {:?} skipped — unsupported width {}", m.name, m.width);
                            continue;
                        }
                        let id = match m.id {
                            Some(i) => i,
                            None => {
                                log::warn!("oa-shell: milestone {:?} skipped — no id", m.name);
                                continue;
                            }
                        };
                        milestone_runtime.push(MilestoneRuntime {
                            id,
                            name: m.name.clone(),
                            region,
                            offset: m.offset,
                            width: m.width,
                            op,
                            target: m.target,
                            edge_only: m.edge_only,
                            already_triggered: m.triggered_at_unix_ms.is_some(),
                        });
                        milestone_prev_true.push(false);
                    }
                    log::info!(
                        "oa-shell: milestones loaded — {} active out of {} configured",
                        milestone_runtime.len(), list.len()
                    );
                }
                Ok(EmuCommand::LoadCheats(list)) => {
                    cheat_runtime = list;
                    log::info!(
                        "oa-shell: cheats loaded — {} total ({} enabled)",
                        cheat_runtime.len(),
                        cheat_runtime.iter().filter(|c| c.enabled).count(),
                    );
                    // Reset + re-register libretro-format cheats with the
                    // core. The core's own decoder handles Game Genie /
                    // GameShark / Action Replay / Pro Action Replay /
                    // raw per-system encodings. Memory-poke cheats stay
                    // out of this path — they're handled by apply_cheats
                    // every frame.
                    if let Some(c) = core.as_mut() {
                        c.cheat_reset();
                        let mut idx: u32 = 0;
                        for cheat in cheat_runtime.iter() {
                            if cheat.kind == "libretro_code" {
                                if let Some(code) = cheat.code.as_deref() {
                                    c.cheat_set(idx, cheat.enabled, code);
                                    idx += 1;
                                }
                            }
                        }
                    }
                }
                Ok(EmuCommand::SetRunAhead(n)) => {
                    let clamped = n.min(5);
                    if clamped != run_ahead_frames {
                        log::info!("oa-shell: run-ahead {} -> {} frame(s)", run_ahead_frames, clamped);
                    }
                    run_ahead_frames = clamped;
                }
                Ok(EmuCommand::StopVideoCapture { discard }) => {
                    let Some(worker) = video_capture.take() else {
                        log::info!("oa-shell: StopVideoCapture — nothing to stop");
                        publish_video_state(None, 0, "", &video_state);
                        continue;
                    };
                    let display_name = video_display_name.clone();
                    let stem = current_rom_stem.clone().unwrap_or_else(|| "unknown".into());
                    let (fw, fh) = video_first_size;
                    let core_fps = timing.fps;
                    let system_id = current_system_id.clone();
                    match worker.stop_and_finalize(&system_id, &stem, &display_name, core_fps, fw, fh, discard) {
                        Ok(r) => {
                            if r.discarded {
                                log::info!("oa-shell: video capture discarded");
                                toast(&app_handle, ToastLevel::Info, "Capture discarded");
                            } else {
                                log::info!(
                                    "oa-shell: video capture saved ({} frames written, {} dropped) -> {}",
                                    r.stats.frames_written, r.dropped, r.manifest_path.display()
                                );
                                let msg = if r.dropped > 0 {
                                    format!("Saved {} frames ({} dropped)", r.stats.frames_written, r.dropped)
                                } else {
                                    format!("Saved {} frames", r.stats.frames_written)
                                };
                                toast(&app_handle, ToastLevel::Success, msg);
                                if let Err(e) = app_handle.emit("oa://video-clip-saved", r.clip_dir.to_string_lossy().to_string()) {
                                    log::warn!("oa-shell: emit video-clip-saved failed: {e:?}");
                                }
                            }
                        }
                        Err(e) => {
                            log::warn!("oa-shell: video capture finalize failed: {e:?}");
                            toast(&app_handle, ToastLevel::Error, format!("Finalize failed: {e}"));
                        }
                    }
                    video_first_size = (0, 0);
                    video_display_name.clear();
                    video_frames_submitted = 0;
                    publish_video_state(None, 0, "", &video_state);
                }
                Ok(EmuCommand::EndRewindScrub { commit }) => {
                    if scrubbing {
                        if let Some(core_ref) = core.as_mut() {
                            if commit {
                                // User picked a point in the past. Drop the
                                // snapshots above (newer than) the target;
                                // the target snapshot is now the new newest.
                                // Forward play resumes from that frame.
                                let dropped = rewind_ring.truncate_above(scrub_position as usize);
                                log::info!(
                                    "oa-shell: rewind scrub committed — dropped {} newer snapshots, position now at newest",
                                    dropped
                                );
                                // The frame body will fall through to normal
                                // forward play; the framebuffer is whatever
                                // the last peek+load painted, which IS the
                                // committed target.
                            } else {
                                // User cancelled — restore the live edge so
                                // no history is lost. Use peek_back so the
                                // ring stays intact for next time. peek_back
                                // now returns owned decompressed bytes
                                // (Phase D — rewind ring zstd-compresses
                                // internally), so no extra clone is needed.
                                if let Some(snap) = rewind_ring.peek_back() {
                                    if let Err(e) = core_ref.load_state(&mut &snap[..]) {
                                        log::warn!("oa-shell: scrub cancel — load_state(newest) failed: {e:?}");
                                    } else {
                                        core_ref.run_frame();
                                    }
                                }
                                log::info!("oa-shell: rewind scrub cancelled — restored live edge");
                            }
                        }
                        scrubbing = false;
                        scrub_position = 0;
                        scrub_dirty = false;
                        publish_rewind_state(&rewind_ring, &rewind_config, &timing, scrubbing, scrub_position, &rewind_state);
                    }
                }
                Ok(EmuCommand::SetAudioDevice(name)) => {
                    // If the previous attempt left audio = None, try to revive
                    // it; otherwise swap the running stream.
                    match audio.as_mut() {
                        Some(sink) => match sink.set_device(name.as_deref()) {
                            Ok(()) => log::info!(
                                "oa-shell: audio device swapped (device = {:?})",
                                sink.current_device()
                            ),
                            Err(e) => log::warn!("oa-shell: audio set_device failed ({e:?}); keeping previous device"),
                        },
                        None => match oa_audio::AudioSink::with_device(timing.sample_rate, name.as_deref()) {
                            Ok(a) => {
                                log::info!(
                                    "oa-shell: audio revived at {} Hz (device = {:?})",
                                    a.sample_rate(),
                                    a.current_device()
                                );
                                audio = Some(a);
                            }
                            Err(e) => log::warn!("oa-shell: audio revive failed ({e:?})"),
                        },
                    }
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    log::warn!("oa-shell: command channel disconnected");
                    break;
                }
            }
        }

        if let Some(s) = inner_size_fn() {
            if s != last_size && s.0 > 0 && s.1 > 0 {
                renderer.resize(s.0, s.1);
                last_size = s;
            }
        }

        // Focus-gated input (Phase 1.5 #3). The polling API `is_focused()`
        // returns false for no-WebView native Tauri windows even when they have
        // user focus (see feedback_tauri_no_webview_is_focused_unreliable). The
        // event-driven `WindowEvent::Focused(bool)` path works reliably though,
        // so on_window_event in main() drives game_focused for us.
        //
        // `ui_intercepting` is flipped from the WebView via set_ui_intercepting
        // while a binding-capture or modal is active, so keystrokes meant for
        // the UI don't leak into the emu thread.
        let enable = game_focused.load(Ordering::SeqCst) && !ui_intercepting.load(Ordering::SeqCst);
        input.set_enabled(enable);

        // Phase 6 Cross-system slice 3 — Ctrl+G toggles Game focus. This
        // edge detector runs UNCONDITIONALLY (no `enable` gate) so the
        // user can always toggle out of Game-focus even when the WebView
        // has stolen focus or a modal is intercepting. The trade is that
        // a Ctrl+G chord typed for any other reason while OA is running
        // also flips the toggle — Ctrl+G isn't bound to anything OA-side
        // and most apps don't use it either, so the collision is rare.
        let ctrl_g_held = (input.is_pressed(Keycode::LControl) || input.is_pressed(Keycode::RControl))
            && input.is_pressed(Keycode::G);
        if ctrl_g_held && !prev_ctrl_g_held {
            let new_state = !game_focus.load(Ordering::SeqCst);
            game_focus.store(new_state, Ordering::SeqCst);
            // Tell the frontend so the Tools-menu checkbox + toolbar chip
            // reflect the live state without polling. The single-window
            // WebView sees the keydown too (and the frontend's keydown
            // handler is a no-op for Ctrl+G), but two-window users have
            // no other way for the UI to know.
            if let Err(e) = app_handle.emit("oa://game-focus-changed", new_state) {
                log::warn!("oa-shell: emit game-focus-changed failed: {e:?}");
            }
            log::info!("oa-shell: Ctrl+G — game_focus = {new_state}");
        }
        prev_ctrl_g_held = ctrl_g_held;

        // Phase 6 Cross-system slice 3 — when Game-focus mode is ON, OA's
        // hotkeys (F1/F2/F3/F5/F6/F7/F8/F12/Esc/digits/Backspace-rewind)
        // stop firing so the keyboard-passthrough pump can hand those keys
        // to the core unchallenged. Gameplay input (the InputPoller-based
        // bindings) still runs — Game-focus only gates *hotkeys*, not the
        // configured controller bindings. UI-intercept still wins (a rebind
        // capture or modal blocks even with Game-focus ON).
        let game_focus_on = game_focus.load(Ordering::SeqCst);
        let hotkeys_enabled = enable && !game_focus_on;

        // Phase 6 Cross-system slice 2 — libretro keyboard-passthrough pump.
        //
        // For computer-shaped systems (MAME, MSX/MSX2) the core registered
        // a `retro_keyboard_event_t` via `RETRO_ENVIRONMENT_SET_KEYBOARD_
        // CALLBACK`; we forward raw key transitions through it. The
        // existing OA hotkey path (F1/F2/F5/F8/Esc/digits below) still
        // fires in parallel — Slice 3's "Game focus" toggle will gate
        // those off when the user wants the core to own the keyboard.
        // Until Slice 3 lands, TAB + letters + numbers reach the core
        // without OA conflict; F-keys + Esc go to both.
        //
        // `should_pump` combines: focused + UI not intercepting + per-
        // system passthrough on + core has actually registered a callback.
        // The last condition short-circuits work when the active core
        // declined keyboard input even though the system defaults to on
        // (e.g. an old MAME build without keyboard support).
        let core_has_kb_cb = core.as_ref().map(|c| c.has_keyboard_callback()).unwrap_or(false);
        let should_pump = enable && keyboard_passthrough_active && core_has_kb_cb;
        let current_keys: HashSet<Keycode> = if should_pump {
            input.pressed_keys().into_iter().collect()
        } else {
            HashSet::new()
        };
        if should_pump {
            // Modifiers carry the CURRENT frame's held set — `device_query`
            // doesn't deliver edges, so a press of `A` while Shift is held
            // sees both keys in `current_keys`. We pass the current
            // modifier mask alongside each transition so a core that
            // remaps Shift+letter sees the right combo.
            let modifiers = oa_libretro::modifiers_from_held(&input.pressed_keys());
            // Presses: keys in `current` but not in `prev`.
            for k in current_keys.difference(&prev_keyboard_keys) {
                let rk = oa_libretro::keycode_to_retro_key(*k);
                if rk == 0 {
                    continue; // RETROK_UNKNOWN — don't waste a callback dispatch.
                }
                if let Some(c) = core.as_mut() {
                    c.send_keyboard_event(true, rk, 0, modifiers);
                }
            }
            // Releases: keys in `prev` but not in `current`.
            for k in prev_keyboard_keys.difference(&current_keys) {
                let rk = oa_libretro::keycode_to_retro_key(*k);
                if rk == 0 {
                    continue;
                }
                if let Some(c) = core.as_mut() {
                    c.send_keyboard_event(false, rk, 0, modifiers);
                }
            }
            prev_keyboard_keys = current_keys;
        } else if !prev_keyboard_keys.is_empty() {
            // Pump stopped (focus lost, passthrough disabled, core dropped).
            // Emit releases for every still-held key so the core doesn't
            // see them as stuck-down. Modifier mask is 0 — by the time we
            // get here we're no longer reading live keyboard state.
            for k in prev_keyboard_keys.iter() {
                let rk = oa_libretro::keycode_to_retro_key(*k);
                if rk == 0 {
                    continue;
                }
                if let Some(c) = core.as_mut() {
                    c.send_keyboard_event(false, rk, 0, 0);
                }
            }
            prev_keyboard_keys.clear();
        }

        // Number keys 0-9 select the active slot (rising-edge). Hotkeys gate
        // on the same focus + UI-intercept flags as gameplay input, so typing
        // in the library or in a rebind capture doesn't accidentally change
        // the active slot or fire a save.
        let digit_keys = [
            Keycode::Key0, Keycode::Key1, Keycode::Key2, Keycode::Key3, Keycode::Key4,
            Keycode::Key5, Keycode::Key6, Keycode::Key7, Keycode::Key8, Keycode::Key9,
        ];
        for (i, key) in digit_keys.iter().enumerate() {
            let pressed = hotkeys_enabled && input.is_pressed(*key);
            if pressed && !prev_digit[i] {
                current_slot = i as u32;
                log::info!("oa-shell: active save slot = {}", current_slot);
            }
            prev_digit[i] = pressed;
        }

        // F5 = save, F8 = restore (rising-edge only). Saves go to disk under
        // app_data_dir/saves/<stem>/slot-N.bin so they survive restarts and
        // are per-game. Gated on `enable` for the same reason as the digits.
        // The whole emulation slice (F5/F8 + run_frame + render + audio drain)
        // only fires when a core is loaded — a failed mid-session core swap
        // leaves `core = None` until the next LoadRom recovers it.
        // F1 = soft reset (Mednafen / RetroArch convention). Calls
        // `Core::reset()` which forwards to `retro_reset`. Toast confirms the
        // reset took effect — single-window game mode hides chrome, so
        // without the toast the user has no UI feedback.
        let f1 = hotkeys_enabled && input.is_pressed(Keycode::F1);
        let f2 = hotkeys_enabled && input.is_pressed(Keycode::F2);
        let f3 = hotkeys_enabled && input.is_pressed(Keycode::F3);
        let f5 = hotkeys_enabled && input.is_pressed(Keycode::F5);
        let f6_held = hotkeys_enabled && input.is_pressed(Keycode::F6);
        let f7_held = hotkeys_enabled && input.is_pressed(Keycode::F7);
        let f8 = hotkeys_enabled && input.is_pressed(Keycode::F8);
        let f12 = hotkeys_enabled && input.is_pressed(Keycode::F12);

        // F2 (edge) — toggle pause. The pause flag short-circuits the
        // NORMAL forward-play branch's run_frame; scrub / replay / rewind
        // paths are unaffected (those have their own time semantics).
        if f2 && !prev_f2 {
            if let Some(c) = core.as_ref() {
                if c.has_rom() {
                    paused = !paused;
                    log::info!("oa-shell: F2 — {}", if paused { "paused" } else { "resumed" });
                    toast(&app_handle, ToastLevel::Info, if paused { "Paused" } else { "Resumed" });
                }
            }
        }
        // F3 (edge) — frame advance. Only meaningful while paused. Sets
        // a one-shot flag the run_frame branch honors then clears.
        if f3 && !prev_f3 && paused {
            frame_advance_request = true;
        }
        prev_f2 = f2;
        prev_f3 = f3;

        // Esc → request Quick Settings overlay. Rising-edge so a held key
        // only fires once. We don't toggle from this side — once the
        // overlay opens, ui_intercepting flips true and `enable` goes
        // false, so this branch can't double-fire. Closing is handled by
        // QuickSettings's own keydown listener in the library WebView.
        let esc = hotkeys_enabled && input.is_pressed(Keycode::Escape);
        if esc && !prev_esc {
            if let Some(core_ref) = core.as_ref() {
                if core_ref.has_rom() {
                    if let Err(e) = app_handle.emit("oa://request-quick-settings", ()) {
                        log::warn!("oa-shell: Esc — emit request-quick-settings failed: {e:?}");
                    }
                    // In two-window mode the library WebView doesn't have
                    // focus during gameplay (the native game window does),
                    // so the overlay would appear on the unfocused library
                    // window and the user couldn't interact with it
                    // keyboard-first. Pull library to the front + focus it.
                    // Single-window mode is a no-op here — the library IS
                    // the focused window.
                    if let Some(lib) = app_handle.get_webview_window("library") {
                        lib.show().ok();
                        lib.unminimize().ok();
                        lib.set_focus().ok();
                    }
                }
            }
        }
        prev_esc = esc;
        if let Some(core_ref) = core.as_mut() {
            if f1 && !prev_f1 {
                if core_ref.has_rom() {
                    core_ref.reset();
                    log::info!("oa-shell: F1 — soft reset");
                    toast(&app_handle, ToastLevel::Success, "Reset");
                } else {
                    log::info!("oa-shell: F1 — no ROM loaded; nothing to reset");
                    toast(&app_handle, ToastLevel::Info, "No ROM loaded");
                }
            }
            if f5 && !prev_f5 && core_ref.has_rom() {
                if let Some(stem) = current_rom_stem.as_deref() {
                    let path = slot_path(&app_data_dir, stem, current_slot);
                    let mut buf = Vec::new();
                    match core_ref.save_state(&mut buf) {
                        Ok(()) => {
                            if let Some(parent) = path.parent() {
                                if let Err(e) = std::fs::create_dir_all(parent) {
                                    log::warn!("oa-shell: F5 — create_dir_all({}) failed: {e:?}", parent.display());
                                }
                            }
                            match std::fs::write(&path, &buf) {
                                Ok(()) => {
                                    log::info!(
                                        "oa-shell: F5 — saved slot {} -> {} ({} bytes)",
                                        current_slot, path.display(), buf.len()
                                    );
                                    toast(&app_handle, ToastLevel::Success, format!("Saved slot {current_slot}"));
                                }
                                Err(e) => {
                                    log::warn!("oa-shell: F5 — write {} failed: {e:?}", path.display());
                                    toast(&app_handle, ToastLevel::Warn, format!("Save failed: {e}"));
                                }
                            }
                            // Thumbnail alongside the save. Failure here doesn't
                            // fail the save — the .bin is the source of truth.
                            let thumb_path = path.with_extension("png");
                            let fb = core_ref.framebuffer();
                            if let Err(e) = write_thumbnail(&thumb_path, fb.width, fb.height, fb.pixels) {
                                log::warn!("oa-shell: F5 — thumbnail write {} failed: {e:?}", thumb_path.display());
                            }
                        }
                        Err(e) => {
                            log::warn!("oa-shell: F5 — serialize failed: {e:?}");
                            toast(&app_handle, ToastLevel::Warn, format!("Save failed: {e}"));
                        }
                    }
                } else {
                    log::info!("oa-shell: F5 — no ROM loaded; nothing to save");
                    toast(&app_handle, ToastLevel::Info, "No ROM loaded");
                }
            }
            if f8 && !prev_f8 {
                if let Some(stem) = current_rom_stem.as_deref() {
                    let path = slot_path(&app_data_dir, stem, current_slot);
                    match std::fs::read(&path) {
                        Ok(buf) => match core_ref.load_state(&mut &buf[..]) {
                            Ok(()) => {
                                log::info!(
                                    "oa-shell: F8 — restored slot {} from {} ({} bytes)",
                                    current_slot, path.display(), buf.len()
                                );
                                toast(&app_handle, ToastLevel::Success, format!("Restored slot {current_slot}"));
                            }
                            Err(e) => {
                                log::warn!("oa-shell: F8 — deserialize failed: {e:?}");
                                toast(&app_handle, ToastLevel::Warn, format!("Load failed: {e}"));
                            }
                        },
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                            log::info!("oa-shell: F8 — slot {} empty ({} not present)", current_slot, path.display());
                            toast(&app_handle, ToastLevel::Info, format!("Slot {current_slot} empty"));
                        }
                        Err(e) => {
                            log::warn!("oa-shell: F8 — read {} failed: {e:?}", path.display());
                            toast(&app_handle, ToastLevel::Warn, format!("Load failed: {e}"));
                        }
                    }
                } else {
                    log::info!("oa-shell: F8 — no ROM loaded; nothing to restore");
                    toast(&app_handle, ToastLevel::Info, "No ROM loaded");
                }
            }

            // F12 (edge) — write the current framebuffer to a PNG under
            // appData/screenshots/<rom-stem>/<timestamp>.png. Uses the
            // `png` crate that already drives save-state thumbnails +
            // video capture, so no new deps. Files-on-disk path because
            // the user typically wants to share / move screenshots; an
            // in-app gallery is a separate follow-up.
            if f12 && !prev_f12 {
                if let Some(stem) = current_rom_stem.as_deref() {
                    let fb = core_ref.framebuffer();
                    match write_screenshot(&app_data_dir, stem, fb.width, fb.height, fb.pixels) {
                        Ok(path) => {
                            log::info!("oa-shell: F12 — screenshot saved to {}", path.display());
                            toast(&app_handle, ToastLevel::Success, format!("Screenshot saved: {}", path.file_name().and_then(|s| s.to_str()).unwrap_or("(unknown)")));
                        }
                        Err(e) => {
                            log::warn!("oa-shell: F12 — screenshot failed: {e}");
                            toast(&app_handle, ToastLevel::Warn, format!("Screenshot failed: {e}"));
                        }
                    }
                } else {
                    toast(&app_handle, ToastLevel::Info, "No ROM loaded");
                }
            }

            // RetroArch parity slice 7 — set true when the normal-play
            // branch's run_frame triggered a run-ahead lookahead that
            // already presented the future framebuffer + pushed real
            // audio. The post-frame present/drain block at the bottom
            // of the `if let Some(core_ref)` block honors this flag to
            // avoid double-presenting + audio duplication.
            let mut ran_ahead = false;
            // Four mutually-exclusive play modes:
            //
            //   1. SCRUB mode (Phase 4 slice B). User opened the rewind
            //      scrubber. Forward play + capture are frozen; we peek at
            //      the requested ring index and apply that snapshot any
            //      time the position changed (`scrub_dirty`).
            //
            //   2. TAS REPLAY (Phase 4 slice C). Dispatch the recorded
            //      input for the current replay frame, run_frame, advance.
            //      At EOF, auto-stop replay. User input + capture +
            //      hold-Backspace rewind all suppressed.
            //
            //   3. HOLD-BACKSPACE rewind (Phase 4 slice A). Pop the
            //      newest ring entry + load_state + one forward frame
            //      to refresh the framebuffer (libretro
            //      retro_video_refresh only fires from retro_run; see
            //      reference_libretro_load_state_needs_run_frame).
            //      Suppressed during recording / replay for clean v1
            //      semantics (rewind-during-recording = v2).
            //
            //   4. NORMAL forward play. Dispatch input, run_frame,
            //      capture a snapshot every `capture_interval_frames`.
            //      When recording, also log the dispatched input frame.
            let rewind_held = hotkeys_enabled
                && rewind_config.enabled
                && core_ref.has_rom()
                && !scrubbing
                && tas_recording.is_none()
                && tas_replay.is_none()
                && input.is_pressed(Keycode::Backspace);

            if scrubbing && core_ref.has_rom() {
                if scrub_dirty {
                    if let Some(snap) = rewind_ring.peek_at(scrub_position as usize) {
                        // peek_at returns owned decompressed bytes since
                        // Phase D — no extra clone needed.
                        if let Err(e) = core_ref.load_state(&mut &snap[..]) {
                            log::warn!("oa-shell: scrub — load_state failed: {e:?}");
                        } else {
                            // One forward frame to repaint. Input is NOT
                            // dispatched — scrubbing freezes the game.
                            core_ref.run_frame();
                        }
                    }
                    scrub_dirty = false;
                }
                // No capture during scrub. The user has paused time.
            } else if tas_replay.is_some() {
                // Replay: dispatch the recorded input for this frame,
                // run, advance. Stop automatically at EOF.
                let rec = tas_replay.as_ref().unwrap();
                let idx = tas_replay_current_frame as usize;
                if idx < rec.input_frames.len() {
                    let f = rec.input_frames[idx];
                    // v2 recordings carry pointer state — NDS stylus,
                    // Saturn / Dreamcast light-gun replays reproduce
                    // the touch/aim coordinate. v1 recordings load with
                    // pointer zeroed (covered by TasInputFrame::default).
                    let pointer = (f.pointer_x, f.pointer_y, f.pointer_pressed);
                    // TAS frames don't carry per-button analog pressure
                    // today (the recording format predates the analog-
                    // button field); zero-fill so the replay matches the
                    // recorded digital state. Future TAS format bump
                    // could capture analog_buttons for pressure-sensitive
                    // titles, mirroring how pointer fields got added.
                    let state = oa_core::InputState {
                        buttons: f.port0,
                        axes: [0; 4],
                        pointer,
                        analog_buttons: [0; 16],
                    };
                    // Recorded input bits are ALREADY libretro-shape
                    // (we record what the core received). Set directly
                    // — bypass the per-system remap that's only for
                    // device_query/gilrs poll output.
                    core_ref.set_input(oa_core::PortIndex::Port0, state);
                    if f.port1 != 0 {
                        core_ref.set_input(
                            oa_core::PortIndex::Port1,
                            oa_core::InputState {
                                buttons: f.port1, axes: [0; 4],
                                pointer: (0, 0, false), analog_buttons: [0; 16],
                            },
                        );
                    }
                    if f.port2 != 0 {
                        core_ref.set_input(
                            oa_core::PortIndex::Port2,
                            oa_core::InputState {
                                buttons: f.port2, axes: [0; 4],
                                pointer: (0, 0, false), analog_buttons: [0; 16],
                            },
                        );
                    }
                    if f.port3 != 0 {
                        core_ref.set_input(
                            oa_core::PortIndex::Port3,
                            oa_core::InputState {
                                buttons: f.port3, axes: [0; 4],
                                pointer: (0, 0, false), analog_buttons: [0; 16],
                            },
                        );
                    }
                    core_ref.run_frame();
                    // Phase 4 slice D — submit framebuffer to the video
                    // encoder if a capture is active. Replay is forward
                    // play, so frames flow normally; capturing a TAS
                    // replay produces a clean canonical video of the
                    // recording.
                    if let Some(cap) = video_capture.as_mut() {
                        let fb = core_ref.framebuffer();
                        let expected = (fb.width as usize)
                            .saturating_mul(fb.height as usize)
                            .saturating_mul(4);
                        if !fb.pixels.is_empty() && fb.pixels.len() == expected {
                            cap.try_submit(video_capture::VideoFrame {
                                frame_idx: video_frames_submitted,
                                width: fb.width,
                                height: fb.height,
                                rgba: fb.pixels.to_vec(),
                            });
                            video_frames_submitted += 1;
                            if video_frames_submitted % 30 == 0 {
                                publish_video_state(Some(&*cap), video_frames_submitted, &video_display_name, &video_state);
                            }
                        }
                    }
                    tas_replay_current_frame += 1;
                    // Publish every 30 frames so the UI's status row
                    // updates without locking the Mutex 60 times/s.
                    if tas_replay_current_frame % 30 == 0 {
                        let total = rec.input_frames.len() as u64;
                        let name = rec.header.display_name.clone();
                        publish_tas_state(TasMode::Replaying, tas_replay_current_frame, total, &name, &tas_state);
                    }
                }
                // EOF check — drop the replay handle so the next frame
                // falls through to normal forward play. Capture the
                // total + name before take() so we can emit the final
                // status update.
                let exhausted = tas_replay_current_frame
                    >= tas_replay.as_ref().map(|r| r.input_frames.len() as u64).unwrap_or(0);
                if exhausted {
                    if let Some(done) = tas_replay.take() {
                        log::info!("oa-shell: TAS replay finished ({} frames)", done.input_frames.len());
                        toast(&app_handle, ToastLevel::Success, "Replay complete");
                        if let Err(e) = app_handle.emit("oa://tas-replay-complete", ()) {
                            log::warn!("oa-shell: emit tas-replay-complete failed: {e:?}");
                        }
                    }
                    tas_replay_current_frame = 0;
                    publish_tas_state(TasMode::Idle, 0, 0, "", &tas_state);
                }
            } else if rewind_held {
                if let Some(snap) = rewind_ring.pop_back() {
                    if let Err(e) = core_ref.load_state(&mut &snap[..]) {
                        log::warn!("oa-shell: rewind — load_state failed: {e:?}");
                    }
                    // Run one frame to repaint the framebuffer + drain
                    // audio at the new state. Input is intentionally NOT
                    // dispatched — we're rewinding, not steering. The
                    // ring shrank, so publish for the UI's stats row.
                    core_ref.run_frame();
                    publish_rewind_state(&rewind_ring, &rewind_config, &timing, scrubbing, scrub_position, &rewind_state);
                }
                // Empty ring: fall through to normal play so the user
                // can build new history forward by releasing the key.
            } else {
                // NORMAL forward play. Poll input + dispatch + (if
                // recording) log it + run + (if rewind enabled) capture.
                //
                // We capture the LIBRETRO-shape input bits for the log
                // (the same bits the core actually received), so replay
                // can short-circuit the per-system remap and dispatch
                // verbatim. This is also why replay works across cores
                // with different bindings — the recording always lives
                // in libretro's joypad bit layout.
                let polled = input.poll(PortIndex::Port0);
                let libretro_bits = bindings::to_libretro_bits(&current_system_id, polled.buttons);
                core_ref.set_input(PortIndex::Port0, oa_core::InputState {
                    buttons: libretro_bits,
                    axes: polled.axes,
                    pointer: polled.pointer,
                    analog_buttons: polled.analog_buttons,
                });
                // Phase G — pump sensor values for cores that enabled
                // accelerometer / gyroscope / illuminance via the
                // sensor interface. Today the values come from the
                // keyboard arrow keys (tilt fallback) so GBA Boktai
                // / Kirby Tilt 'n' Tumble / WarioWare Twisted! are
                // playable without OS-level accelerometer access. The
                // sensors_enabled check skips the work for the 95% of
                // cores that don't use sensors.
                if core_ref.sensors_enabled() {
                    let mut sensors = [[0.0f32; 7]; 5];
                    // Phase G v1: arrow keys → accelerometer tilt on
                    // port 0 only. Up/Down drive the Y axis, Left/
                    // Right drive X. Magnitude ~9.8 m/s² (1g) at full
                    // deflection so games see a "tilt to 45°" gesture.
                    let left = input.is_pressed(Keycode::Left);
                    let right = input.is_pressed(Keycode::Right);
                    let up = input.is_pressed(Keycode::Up);
                    let down = input.is_pressed(Keycode::Down);
                    let tilt_x: f32 = match (left, right) {
                        (true, false) => -9.8,
                        (false, true) => 9.8,
                        _ => 0.0,
                    };
                    let tilt_y: f32 = match (up, down) {
                        (true, false) => 9.8,
                        (false, true) => -9.8,
                        _ => 0.0,
                    };
                    sensors[0][oa_libretro::ffi::RETRO_SENSOR_ACCELEROMETER_X as usize] = tilt_x;
                    sensors[0][oa_libretro::ffi::RETRO_SENSOR_ACCELEROMETER_Y as usize] = tilt_y;
                    // Z stays at 9.8 (gravity) — flat-on-table baseline
                    // so games that read all three axes don't see a
                    // free-falling controller.
                    sensors[0][oa_libretro::ffi::RETRO_SENSOR_ACCELEROMETER_Z as usize] = 9.8;
                    core_ref.set_sensor_values(sensors);
                }
                if let Some(rec) = tas_recording.as_mut() {
                    rec.input_frames.push(oa_savestate::tas::TasInputFrame {
                        port0: libretro_bits,
                        port1: 0,
                        port2: 0,
                        port3: 0,
                        // v2: capture pointer state too so NDS stylus /
                        // Saturn-light-gun / Dreamcast-light-gun replays
                        // reproduce the touch/aim coordinates.
                        pointer_x: polled.pointer.0,
                        pointer_y: polled.pointer.1,
                        pointer_pressed: polled.pointer.2,
                    });
                    // Publish the recording frame count every 30 frames.
                    if rec.input_frames.len() as u64 % 30 == 0 {
                        let n = rec.input_frames.len() as u64;
                        let name = rec.header.display_name.clone();
                        publish_tas_state(TasMode::Recording, n, 0, &name, &tas_state);
                    }
                }

                // RetroArch-parity slice 3 — pause / fast-forward / slow-motion.
                // PAUSED: skip run_frame entirely (or run exactly one frame
                // when F3 requested a frame advance; clear the flag after).
                // FAST-FORWARD (F6 held): run multiple frames per render
                // cycle so wall-clock seconds map to N× game-seconds.
                // SLOW-MOTION (F7 held): run_frame only every Nth render
                // cycle — render stays at full rate, game time slows.
                // None of these affect scrub / replay / rewind branches.
                if paused {
                    if frame_advance_request {
                        core_ref.run_frame();
                        apply_cheats(core_ref, &cheat_runtime);
                        frame_advance_request = false;
                    }
                    // else: hold the last frame; framebuffer is unchanged.
                } else if f6_held {
                    // Fast-forward — single render frame, multiple game frames.
                    for _ in 0..FAST_FORWARD_BURST {
                        core_ref.run_frame();
                        apply_cheats(core_ref, &cheat_runtime);
                    }
                    slow_mo_phase = 0;
                } else if f7_held {
                    // Slow-motion — only run on every Nth render frame.
                    slow_mo_phase = (slow_mo_phase + 1) % SLOW_MOTION_DIVISOR;
                    if slow_mo_phase == 0 {
                        core_ref.run_frame();
                        apply_cheats(core_ref, &cheat_runtime);
                    }
                } else {
                    slow_mo_phase = 0;
                    core_ref.run_frame();
                    apply_cheats(core_ref, &cheat_runtime);

                    // === Run-Ahead lookahead =============================
                    //
                    // Reduce perceived input latency by N frames: after
                    // the real run_frame produces frame X, save the
                    // post-X state, run N more frames (no new input),
                    // present the resulting frame X+N, then rollback to
                    // X. The user sees frame X+N's pixels (effectively
                    // a peek into the future) while the core's "real"
                    // position is still X, so when the next render frame
                    // arrives with input I, the core processes I from
                    // state X — but the visible result is from N frames
                    // beyond that.
                    //
                    // Cost: 1 save_state + N extra run_frames + 1
                    // load_state per render frame. PCE/NES-class cores
                    // sit at ~0.5ms total at N=2; bigger cores can
                    // exceed budget — that's why this is opt-in with a
                    // clamp at 5.
                    //
                    // Skipped during scrubbing / TAS replay / TAS
                    // recording — those branches have their own time
                    // semantics where peeking ahead is wrong or breaks
                    // determinism. Other special modes (paused / FF /
                    // SM) don't reach this branch.
                    if run_ahead_frames > 0
                        && !scrubbing
                        && tas_replay.is_none()
                        && tas_recording.is_none()
                    {
                        // Capture the REAL audio between the real
                        // run_frame and the future-frame run_frames so
                        // the user hears samples from the same frame
                        // they're (notionally) on input-wise, not the
                        // future. drain_audio is &mut self and clears
                        // the internal accumulator so future-frame
                        // audio is naturally discarded by drain after
                        // load_state.
                        run_ahead_audio_buf.clear();
                        run_ahead_audio_buf.extend_from_slice(core_ref.drain_audio());
                        run_ahead_save_buf.clear();
                        if core_ref.save_state(&mut run_ahead_save_buf).is_ok() {
                            for _ in 0..run_ahead_frames {
                                core_ref.run_frame();
                                apply_cheats(core_ref, &cheat_runtime);
                            }
                            renderer.present(core_ref.framebuffer());
                            if let Err(e) = core_ref.load_state(&mut run_ahead_save_buf.as_slice()) {
                                log::warn!("oa-shell: run-ahead rollback failed: {e:?}");
                            } else if let Some(sink) = audio.as_mut() {
                                if !run_ahead_audio_buf.is_empty() {
                                    sink.push(&run_ahead_audio_buf);
                                }
                            }
                            ran_ahead = true;
                        } else {
                            log::warn!("oa-shell: run-ahead save_state failed; disabling for this frame");
                        }
                    }
                }

                // Phase 4 slice E + F — refresh the memory snapshot
                // every frame so the inspector + milestone evaluator
                // see fresh bytes. The snapshot is gated on whether
                // the snapshot Arc has a non-default state inside or
                // milestones are armed; even when neither is true we
                // pay only a single Mutex lock + length check.
                {
                    use oa_core::MemoryRegionId::*;
                    // Cheap pre-check: if neither the inspector poll
                    // path nor the milestone evaluator wants memory,
                    // skip the copy entirely. The inspector polls via
                    // a Tauri command; once at least one poll has
                    // happened the snapshot is non-empty + we keep
                    // refreshing. Reasonable proxy: keep refreshing
                    // whenever there's anything to refresh (post-load
                    // first call seeds it; on UnloadRom we'll reset).
                    // Phase F — forward whatever rumble strength the core
                    // requested during this run_frame to gilrs's
                    // force-feedback API. Cores that don't use rumble
                    // return all-zeros from rumble_snapshot(); the
                    // dispatch_rumble fast-paths the zero case so the
                    // per-frame cost is negligible.
                    let rumble = core_ref.rumble_snapshot();
                    input.dispatch_rumble(rumble);
                    let need_snapshot =
                        !milestone_runtime.is_empty()
                        || memory_snapshot
                            .lock()
                            .map(|s| {
                                s.system_ram.is_some()
                                    || s.save_ram.is_some()
                                    || s.rtc.is_some()
                                    || s.video_ram.is_some()
                            })
                            .unwrap_or(false);
                    if need_snapshot {
                        let snap = MemorySnapshot {
                            save_ram: core_ref.memory_region(SaveRam).map(|s| s.to_vec()),
                            rtc: core_ref.memory_region(Rtc).map(|s| s.to_vec()),
                            system_ram: core_ref.memory_region(SystemRam).map(|s| s.to_vec()),
                            video_ram: core_ref.memory_region(VideoRam).map(|s| s.to_vec()),
                        };
                        if let Ok(mut s) = memory_snapshot.lock() {
                            *s = snap;
                        }
                    }
                }

                // Evaluate milestone predicates. Edge-trigger detection
                // uses milestone_prev_true to spot rising transitions
                // (false → true) and fires once per game session if
                // `edge_only` is set. Triggered milestones get stamped
                // in the DB via `mark_milestone_triggered`.
                if !milestone_runtime.is_empty() {
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as i64)
                        .unwrap_or(0);
                    let db = app_handle.try_state::<library_db::LibraryDb>();
                    for (i, m) in milestone_runtime.iter_mut().enumerate() {
                        let now_true = read_memory_le(core_ref, m.region, m.offset, m.width)
                            .map(|v| m.op.eval(v, m.target))
                            .unwrap_or(false);
                        let prev = milestone_prev_true.get(i).copied().unwrap_or(false);
                        let rising = now_true && !prev;
                        if let Some(slot) = milestone_prev_true.get_mut(i) {
                            *slot = now_true;
                        }
                        let should_fire = if m.edge_only {
                            rising && !m.already_triggered
                        } else {
                            now_true && !prev
                        };
                        if should_fire {
                            log::info!("oa-shell: milestone triggered — \"{}\" (id={})", m.name, m.id);
                            toast(&app_handle, ToastLevel::Success, format!("🏆 {}", m.name));
                            if let Err(e) = app_handle.emit("oa://milestone-triggered", serde_json::json!({
                                "id": m.id,
                                "name": m.name.clone(),
                                "triggeredAtUnixMs": now_ms,
                            })) {
                                log::warn!("oa-shell: emit milestone-triggered failed: {e:?}");
                            }
                            if let Some(db_ref) = db.as_ref() {
                                if let Err(e) = db_ref.mark_milestone_triggered(m.id, now_ms) {
                                    log::warn!("oa-shell: mark_milestone_triggered failed: {e:?}");
                                }
                            }
                            if m.edge_only {
                                m.already_triggered = true;
                            }
                        }
                    }
                }

                // Phase 4 slice D — submit framebuffer to the video
                // encoder if a capture is active. Same try_submit
                // pattern as the replay branch.
                if let Some(cap) = video_capture.as_mut() {
                    let fb = core_ref.framebuffer();
                    let expected = (fb.width as usize)
                        .saturating_mul(fb.height as usize)
                        .saturating_mul(4);
                    if !fb.pixels.is_empty() && fb.pixels.len() == expected {
                        cap.try_submit(video_capture::VideoFrame {
                            frame_idx: video_frames_submitted,
                            width: fb.width,
                            height: fb.height,
                            rgba: fb.pixels.to_vec(),
                        });
                        video_frames_submitted += 1;
                        if video_frames_submitted % 30 == 0 {
                            publish_video_state(Some(&*cap), video_frames_submitted, &video_display_name, &video_state);
                        }
                    }
                }

                // Capture a snapshot every `capture_interval_frames`
                // forward frames. The capture itself can be expensive
                // (retro_serialize for SNES is ~300 KB; for tg16 ~50 KB)
                // — keeping it off the rewind path means we don't pay
                // for it during reverse play, where the ring is the
                // workload anyway.
                if rewind_config.enabled
                    && rewind_config.capture_interval_frames > 0
                    && (frame_n % rewind_config.capture_interval_frames as u64) == 0
                {
                    let mut buf = Vec::new();
                    match core_ref.save_state(&mut buf) {
                        Ok(()) => {
                            rewind_ring.push(buf);
                            // Publish only on capture frames — the ring
                            // count + bytes are unchanged on every other
                            // frame, so polling readers don't miss anything.
                            publish_rewind_state(&rewind_ring, &rewind_config, &timing, scrubbing, scrub_position, &rewind_state);
                        }
                        Err(e) => log::trace!(
                            "oa-shell: rewind capture skipped at frame {frame_n}: {e:?}"
                        ),
                    }
                }
            }
            // Run-ahead already presented the future frame + pushed the
            // real audio; skip both here to avoid double-rendering and
            // duplicated audio samples.
            if !ran_ahead {
                renderer.present(core_ref.framebuffer());

                // Phase 2.5 — push the freshly-computed game-output
                // rectangle (in screen coordinates) into the InputPoller
                // so pointer mapping is pixel-perfect against the
                // letterboxed viewport, not the whole monitor. Cheap:
                // last_viewport is a cached field on the renderer, the
                // window position is a Tauri syscall that runs at
                // 60 Hz max. Falls back to the 1080p approximation when
                // either resolution can't be obtained.
                if let (Some((wx, wy)), Some((vx, vy, vw, vh))) =
                    (window_position_fn(), renderer.last_viewport())
                {
                    input.set_pointer_viewport(Some(oa_input::PointerViewport {
                        screen_x: wx as f32 + vx,
                        screen_y: wy as f32 + vy,
                        width: vw,
                        height: vh,
                    }));
                } else {
                    input.set_pointer_viewport(None);
                }

                // Pump audio: drain whatever the core produced this frame into the sink.
                // `drain_audio` borrows &mut self, so this has to come after `framebuffer()`.
                if let Some(sink) = audio.as_mut() {
                    let samples = core_ref.drain_audio();
                    if !samples.is_empty() {
                        sink.push(samples);
                    }
                } else {
                    let _ = core_ref.drain_audio();
                }
            }

            frame_n += 1;
            // Update perf_stats every 30 frames (~0.5 s at 60 fps). The
            // Tauri-side HUD polls at 250 ms so it'll see a fresh value
            // on every other poll. Cheap: one Mutex acquisition + 7
            // field writes.
            if frame_n % 30 == 0 {
                let elapsed = started.elapsed().as_secs_f64();
                let actual_fps = if elapsed > 0.0 { frame_n as f64 / elapsed } else { 0.0 };
                let (pushed, dropped) = audio.as_ref().map(|a| a.stats()).unwrap_or((0, 0));
                if let Ok(mut s) = perf_stats.lock() {
                    *s = SharedPerfStats {
                        core_loaded: core_ref.has_rom(),
                        fps: actual_fps,
                        frame_count: frame_n,
                        audio_pushed: pushed as u64,
                        audio_dropped: dropped as u64,
                        core_fps_nominal: timing.fps,
                    };
                }
            }
            if frame_n % 120 == 0 {
                let fb = core_ref.framebuffer();
                let elapsed = started.elapsed().as_secs_f32();
                let actual_fps = frame_n as f32 / elapsed;
                let (pushed, dropped) = audio.as_ref().map(|a| a.stats()).unwrap_or((0, 0));
                log::info!(
                    "oa-shell: frame {} (~{:.1} fps); fb {}x{}; rom_loaded = {}; audio {}+{} (pushed+dropped)",
                    frame_n, actual_fps, fb.width, fb.height, core_ref.has_rom(), pushed, dropped
                );
            }
        } else {
            // No core loaded (post-UnloadRom, or initial-load failure).
            // Keep the swap chain ticking so the surface shows blank black
            // instead of the last presented framebuffer of the previous game.
            renderer.present_blank();
        }
        prev_f1 = f1;
        prev_f5 = f5;
        prev_f8 = f8;
        prev_f12 = f12;

        next_frame += frame_period;
        let now = Instant::now();
        if next_frame > now {
            std::thread::sleep(next_frame - now);
        } else {
            next_frame = now;
        }
    }

    log::info!("oa-shell: emu+render thread stopping at frame {frame_n}");
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ScannedRom {
    /// File path the frontend stores as RomEntry.filePath. For raw ROMs this
    /// is the file on disk; for archived ROMs it's the encoded
    /// `<archive>#<inner>` form so the UNIQUE constraint on games.file_path
    /// admits multiple inner ROMs from one archive.
    path: String,
    file_name: String,
    /// Inner-ROM extension (lowercase, no leading dot). Drives the system-id
    /// lookup on the frontend side via systemForExtension.
    extension: String,
    /// Set when this entry came from inside an archive. Routes the launch
    /// flow through archive::extract_for_launch. Frontend stores it as
    /// RomEntry.archiveInnerPath.
    #[serde(skip_serializing_if = "Option::is_none")]
    archive_inner_path: Option<String>,
}

#[tauri::command]
fn scan_rom_folder(path: String, extensions: Vec<String>) -> Result<Vec<ScannedRom>, String> {
    let folder = std::path::PathBuf::from(&path);
    if !folder.is_dir() {
        return Err(format!("not a directory: {path}"));
    }

    let wanted: std::collections::HashSet<String> = extensions
        .into_iter()
        .map(|e| e.trim_start_matches('.').to_ascii_lowercase())
        .collect();

    // Recursive scan up to MAX_DEPTH levels — handles the common case where
    // CD games live in per-game subfolders (`Ys I & II/*.cue` + track .bins,
    // or per-game CHD folders). Bounded so picking a deep tree (your entire
    // home dir, say) doesn't take forever. Hidden dirs (`.git/`, `System
    // Volume Information/` after the dot-skip) are skipped.
    const MAX_DEPTH: u32 = 6;
    let mut out = Vec::new();
    scan_recursive(&folder, 0, MAX_DEPTH, &wanted, &mut out);

    out.sort_by(|a, b| a.file_name.to_ascii_lowercase().cmp(&b.file_name.to_ascii_lowercase()));
    // Sorted list of extensions we were looking for — useful when matches=0
    // so the user can see what was filtered in/out.
    let mut wanted_sorted: Vec<&String> = wanted.iter().collect();
    wanted_sorted.sort();
    let archived_count = out.iter().filter(|r| r.archive_inner_path.is_some()).count();
    log::info!(
        "scan_rom_folder: {} matched ({} archived) in {} (recursive depth {}, extensions {:?})",
        out.len(), archived_count, path, MAX_DEPTH, wanted_sorted
    );
    if archived_count > 0 {
        if let Some(sample) = out.iter().find(|r| r.archive_inner_path.is_some()) {
            log::info!(
                "scan_rom_folder: sample archived entry: path={:?} file_name={:?} extension={:?} archive_inner_path={:?}",
                sample.path, sample.file_name, sample.extension, sample.archive_inner_path
            );
            // Also serialize one row through serde to confirm the JSON shape
            // matches what the frontend expects.
            match serde_json::to_string(sample) {
                Ok(j) => log::info!("scan_rom_folder: sample JSON: {}", j),
                Err(e) => log::warn!("scan_rom_folder: serialize sample failed: {e}"),
            }
        }
    }
    if out.is_empty() {
        log::warn!("scan_rom_folder: 0 matches — verify the folder contains files with one of the listed extensions, and that they're not inside dotted directories");
    } else if log::log_enabled!(log::Level::Debug) {
        for r in &out {
            log::debug!("scan_rom_folder: matched {}", r.path);
        }
    }
    Ok(out)
}

fn scan_recursive(
    dir: &Path,
    depth: u32,
    max_depth: u32,
    wanted: &std::collections::HashSet<String>,
    out: &mut Vec<ScannedRom>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else { continue };
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        // Skip dotfiles + dotdirs (.git, .DS_Store, etc.).
        if name_str.starts_with('.') {
            continue;
        }
        let entry_path = entry.path();

        if file_type.is_dir() {
            if depth + 1 <= max_depth {
                scan_recursive(&entry_path, depth + 1, max_depth, wanted, out);
            }
            continue;
        }
        if !file_type.is_file() {
            continue;
        }

        let ext = entry_path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase());
        let Some(ext) = ext else { continue };

        // Archive case — peek inside and emit one ScannedRom per ROM-like
        // inner entry. The user's wanted set doesn't include "zip"/"7z"
        // themselves (they're not playable extensions), so we check ext
        // against ArchiveKind explicitly and use the wanted set for filtering
        // inner entries.
        if let Some(_kind) = archive::ArchiveKind::from_extension(&ext) {
            match archive::list_rom_contents(&entry_path, wanted) {
                Ok(inner_entries) => {
                    for inner in inner_entries {
                        let encoded_path = archive::encode_file_path(&entry_path, &inner.inner_path);
                        // Inner file name for display — strip directory parts
                        // ("subdir/foo.pce" → "foo.pce").
                        let inner_name = Path::new(&inner.inner_path)
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| inner.inner_path.clone());
                        out.push(ScannedRom {
                            path: encoded_path,
                            file_name: inner_name,
                            extension: inner.extension.clone(),
                            archive_inner_path: Some(inner.inner_path),
                        });
                    }
                }
                Err(e) => log::warn!("scan: failed to peek into {}: {e}", entry_path.display()),
            }
            continue;
        }
        if archive::ArchiveKind::is_unsupported_archive(&ext) {
            log::warn!(
                "scan: skipping {} — {} archives aren't supported; convert to .zip or .7z",
                entry_path.display(), ext
            );
            continue;
        }

        if !wanted.contains(&ext) {
            continue;
        }
        out.push(ScannedRom {
            path: entry_path.to_string_lossy().into_owned(),
            file_name: name_str.into_owned(),
            extension: ext,
            archive_inner_path: None,
        });
    }
}

fn parse_scaling_mode(s: &str) -> Result<oa_render::ScalingMode, String> {
    use oa_render::ScalingMode;
    match s {
        "pixel-perfect"  => Ok(ScalingMode::PixelPerfect),
        "aspect-correct" => Ok(ScalingMode::AspectCorrectFit),
        "stretched"      => Ok(ScalingMode::Stretched),
        "original"       => Ok(ScalingMode::Original),
        other if other.starts_with("integer-") => {
            let n: u32 = other
                .trim_start_matches("integer-")
                .trim_end_matches('x')
                .parse()
                .map_err(|_| format!("bad integer multiple: {s}"))?;
            if n == 0 {
                return Err("integer multiple must be >= 1".into());
            }
            Ok(ScalingMode::IntegerMultiple(n))
        }
        _ => Err(format!("unknown scaling mode: {s}")),
    }
}

#[tauri::command]
fn set_scaling_mode(mode: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let parsed = parse_scaling_mode(&mode)?;
    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    tx.send(EmuCommand::SetScalingMode(parsed))
        .map_err(|e| format!("emu thread closed: {e}"))?;
    log::info!("oa-shell: set_scaling_mode -> {}", mode);
    Ok(())
}

/// Phase 3 slice C — apply a shader preset by name. Looks up the preset
/// in the TOML registry (built-ins + any user files in
/// `<exe_dir>/shaders/presets/`), decodes any referenced bezel PNG, and
/// sends a [`EmuCommand::ApplyShaderPreset`] to the emu thread. Called
/// from the frontend launch path after it resolves the effective preset
/// from the per-game → per-system → OA-wide chain. Unknown preset names
/// fall through to "plain" so a stale persisted name can't crash the
/// renderer (matches the prior parse-fallback behavior).
#[tauri::command]
fn set_shader_preset(preset: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let exe_dir = resolve_exe_dir();
    let defs = shader_presets::load_all(&exe_dir);
    let def = defs.iter().find(|d| d.name == preset).cloned().unwrap_or_else(|| {
        log::warn!("oa-shell: shader preset `{preset}` not found; falling back to `plain`");
        defs.iter()
            .find(|d| d.name == "plain")
            .cloned()
            .or_else(|| shader_presets::builtins().into_iter().find(|d| d.name == "plain"))
            .expect("plain preset always shipped as a built-in")
    });
    let resolved = shader_presets::apply(&def, &exe_dir);
    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    log::info!(
        "oa-shell: set_shader_preset -> {} (base {}, bloom={:?}, bezel={})",
        preset,
        resolved.base.as_str(),
        resolved.bloom_amount,
        resolved.bezel.as_ref().map(|b| format!("{}x{}", b.width, b.height)).unwrap_or_else(|| "none".into()),
    );
    tx.send(EmuCommand::ApplyShaderPreset(resolved))
        .map_err(|e| format!("emu thread closed: {e}"))?;
    // Slice D — record the resolved preset's actual name (`def.name`,
    // which is `"plain"` when the requested name was unknown) so the
    // watcher re-applies the right entry when its TOML changes.
    if let Ok(mut guard) = state.active_shader_preset.lock() {
        *guard = Some(def.name.clone());
    }
    Ok(())
}

/// Phase 3 slice C — list shader presets available to the frontend.
/// Returns the merged registry: built-ins overlaid with any user files
/// in `<exe_dir>/shaders/presets/<name>.preset.toml`. Sorted by name.
#[tauri::command]
fn list_shader_presets() -> Vec<shader_presets::ShaderPresetSummary> {
    let exe_dir = resolve_exe_dir();
    let defs = shader_presets::load_all(&exe_dir);
    shader_presets::summarize(&defs)
}

/// Phase 3 slice C polish — override the Phosphor composite weight.
/// Called from the frontend's launch path after `set_shader_preset` so the
/// per-game / per-system value layers on top of the TOML preset's default.
/// Clamped to [0, 1] on the renderer side; values outside that range get
/// silently clipped rather than rejected (treat as a UI affordance).
#[tauri::command]
fn set_bloom_amount(amount: f32, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    tx.send(EmuCommand::SetBloomAmount(amount))
        .map_err(|e| format!("emu thread closed: {e}"))?;
    log::info!("oa-shell: set_bloom_amount -> {:.3}", amount);
    Ok(())
}

/// Pin a display-aspect override on the running renderer. `aspect = 0`
/// (or any value ≤ 0) is normalised to `None` — "revert to whatever the
/// core reports." Frontend resolves per-game → per-system → null
/// before pushing.
#[tauri::command]
fn set_display_aspect_override(aspect: Option<f32>, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let normalised = aspect.filter(|a| *a > 0.0);
    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    tx.send(EmuCommand::SetDisplayAspectOverride(normalised))
        .map_err(|e| format!("emu thread closed: {e}"))?;
    log::info!("oa-shell: set_display_aspect_override -> {:?}", normalised);
    Ok(())
}

/// Load a bezel image from a local file path, decode it, and push to
/// the renderer as an override on top of the active shader preset's
/// TOML `[bezel]` block. Frontend resolves per-game → per-system →
/// null and calls this with the resolved path; `clear_bezel_override`
/// reverts to "whatever the preset said." PNG/JPEG/WebP all accepted.
#[tauri::command]
fn set_bezel_image_override(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    let (rgba, width, height) =
        shader_presets::load_rgba_image(p).map_err(|e| format!("decode bezel {path}: {e}"))?;
    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    tx.send(EmuCommand::SetBezelOverride(Some(shader_presets::ResolvedBezel {
        rgba,
        width,
        height,
    })))
    .map_err(|e| format!("emu thread closed: {e}"))?;
    log::info!("oa-shell: set_bezel_image_override -> {path} ({width}x{height})");
    Ok(())
}

/// Drop any active bezel override. The renderer clears the bezel
/// immediately; the next `ApplyShaderPreset` (typically the next ROM
/// launch) will repopulate from the preset's TOML default.
#[tauri::command]
fn clear_bezel_image_override(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    tx.send(EmuCommand::SetBezelOverride(None))
        .map_err(|e| format!("emu thread closed: {e}"))?;
    log::debug!("oa-shell: clear_bezel_image_override");
    Ok(())
}

/// Apply an overscan crop. All-zero = no crop. Frontend resolves
/// per-game → per-system → all-zero before pushing.
#[tauri::command]
#[allow(non_snake_case)]
fn set_overscan_crop(
    top: u32,
    bottom: u32,
    left: u32,
    right: u32,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let crop = oa_render::OverscanCrop { top, bottom, left, right };
    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    tx.send(EmuCommand::SetOverscanCrop(crop))
        .map_err(|e| format!("emu thread closed: {e}"))?;
    log::info!("oa-shell: set_overscan_crop -> t={top} b={bottom} l={left} r={right}");
    Ok(())
}

/// Locate `<exe_dir>` the same way `resolve_cores_dir` does — next to
/// the running binary, falling back to CWD if the path can't be derived.
fn resolve_exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

/// Phase 4 slice A — configure the rewind ring. Called from the frontend
/// launch path after resolving `enabled` / `capture_interval_frames` /
/// `max_megabytes` from the per-game → per-system → OA-wide chain. Also
/// called when the operator changes any rewind setting in the UI so the
/// running session picks up the new config without needing a relaunch.
/// `max_megabytes` is multiplied into bytes server-side so the API stays
/// human-readable (the UI surfaces "32 MB" not "33554432 B").
#[tauri::command]
fn set_rewind_config(
    enabled: bool,
    capture_interval_frames: u32,
    max_megabytes: u32,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let interval = capture_interval_frames.max(1);
    let bytes = (max_megabytes.max(1) as usize).saturating_mul(1024 * 1024);
    let cfg = oa_savestate::RewindConfig {
        enabled,
        capture_interval_frames: interval,
        max_bytes: bytes,
    };
    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    tx.send(EmuCommand::SetRewindConfig(cfg))
        .map_err(|e| format!("emu thread closed: {e}"))?;
    log::info!(
        "oa-shell: set_rewind_config -> enabled={enabled} interval={interval} cap={}MB",
        max_megabytes
    );
    Ok(())
}

/// Phase 4 slice B — read the live rewind-ring stats so the scrubbing UI
/// can size its timeline + show "N snapshots / X MB / Y s held". Cheap —
/// just clones a small struct out from under a Mutex; the emu thread
/// updates the same Mutex after every capture / pop / scrub op.
#[tauri::command]
fn get_rewind_state(state: tauri::State<'_, AppState>) -> Result<SharedRewindState, String> {
    let s = state.rewind_state.lock().map_err(|_| "rewind_state poisoned".to_string())?;
    Ok(*s)
}

/// Read the latest published emu-thread perf stats. Used by the Tools
/// → Performance HUD overlay to surface real emulator fps + audio
/// counters (the HUD's UI-side `requestAnimationFrame` counter measures
/// WebView render rate, which can run at host display rate even when
/// the emu is stalled).
#[tauri::command]
fn get_perf_stats(state: tauri::State<'_, AppState>) -> Result<SharedPerfStats, String> {
    let s = state.perf_stats.lock().map_err(|_| "perf_stats poisoned".to_string())?;
    Ok(*s)
}

// ---- Debug console -----------------------------------------------------

/// Snapshot the recent-logs ring. `limit` caps the number of returned
/// entries (oldest dropped); `None` returns the whole ring. Used by the
/// `Help → Debug log…` dialog polling at 1 Hz.
#[tauri::command]
fn get_recent_logs(
    limit: Option<usize>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<logger::LogEntry>, String> {
    let ring = state
        .logger_handle
        .ring
        .lock()
        .map_err(|_| "log ring poisoned".to_string())?;
    Ok(ring.snapshot(limit))
}

/// Absolute path of the current session's `oa-current.log`. Used by
/// the "Copy log path" button so the user can paste it into a chat
/// with the developer (me). Stable across launches: the same path
/// every time, contents truncated on each session start.
#[tauri::command]
fn get_log_file_path(state: tauri::State<'_, AppState>) -> Result<Option<String>, String> {
    let p = state
        .logger_handle
        .file_path
        .lock()
        .map_err(|_| "log path poisoned".to_string())?;
    Ok(p.as_ref().map(|pb| pb.to_string_lossy().into_owned()))
}

/// Open the logs folder in the OS file manager. Same per-OS dispatch
/// as `open_video_clip_folder` / `open_screenshot_folder`.
#[tauri::command]
fn reveal_logs_folder(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let dir = state.app_data_dir.join("logs");
    if !dir.is_dir() {
        return Err("no logs folder yet".into());
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer.exe")
            .arg(&dir)
            .spawn()
            .map_err(|e| format!("spawn explorer: {e}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&dir)
            .spawn()
            .map_err(|e| format!("spawn open: {e}"))?;
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(&dir)
            .spawn()
            .map_err(|e| format!("spawn xdg-open: {e}"))?;
    }
    Ok(())
}

/// Bridge frontend `console.log/warn/error/info` calls into the unified
/// Rust log stream. Lets the debug-log dialog show a single timeline
/// of Rust + frontend events. The frontend `logbridge.ts` wraps each
/// `console.*` method to invoke this; existing call sites in the
/// codebase don't have to change.
#[tauri::command]
fn log_from_frontend(level: String, target: String, message: String) {
    let lvl = match level.as_str() {
        "error" => log::Level::Error,
        "warn" => log::Level::Warn,
        "debug" => log::Level::Debug,
        "trace" => log::Level::Trace,
        _ => log::Level::Info,
    };
    // The `target` prefix marks frontend records visually in the file.
    let scoped_target = if target.is_empty() {
        "frontend".to_string()
    } else {
        format!("frontend::{target}")
    };
    log::logger().log(
        &log::Record::builder()
            .level(lvl)
            .target(&scoped_target)
            .args(format_args!("{}", message))
            .build(),
    );
}

/// Phase 4 slice B — enter scrub mode. Forward play + capture freeze
/// until `end_rewind_scrub` arrives. Idempotent: a second start is a
/// no-op. Ignored when rewind isn't enabled or the ring is empty.
#[tauri::command]
fn start_rewind_scrub(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    tx.send(EmuCommand::StartRewindScrub).map_err(|e| format!("emu thread closed: {e}"))?;
    Ok(())
}

/// Phase 4 slice B — preview the snapshot at `steps_back` from the
/// newest. Spammed during drag (fired per frame on most pointer setups).
/// Clamped server-side to the ring's actual length. No-op outside scrub.
#[tauri::command]
fn set_rewind_scrub_position(steps_back: u32, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    tx.send(EmuCommand::SetRewindScrubPosition { steps_back })
        .map_err(|e| format!("emu thread closed: {e}"))?;
    Ok(())
}

/// Phase 4 slice B — exit scrub mode. `commit = true` truncates the
/// snapshots newer than the current position (the user chose a past
/// point and the future is rewritten); `commit = false` restores the
/// most-recent snapshot so the rewind ring is intact and no game state
/// is lost. Forward play resumes either way.
#[tauri::command]
fn end_rewind_scrub(commit: bool, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    tx.send(EmuCommand::EndRewindScrub { commit }).map_err(|e| format!("emu thread closed: {e}"))?;
    log::info!("oa-shell: end_rewind_scrub (commit={commit})");
    Ok(())
}

// ---- Phase 4 slice C — TAS recording / replay --------------------------

/// Live status of the TAS state machine. Same publish-on-state-change
/// pattern as `get_rewind_state`; cheap to poll from the UI every few
/// hundred ms while the Quick Settings TAS panel is open.
#[tauri::command]
fn get_tas_state(state: tauri::State<'_, AppState>) -> Result<SharedTasState, String> {
    let s = state.tas_state.lock().map_err(|_| "tas_state poisoned".to_string())?;
    Ok(s.clone())
}

#[tauri::command]
fn start_tas_recording(display_name: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    tx.send(EmuCommand::StartTasRecording { display_name })
        .map_err(|e| format!("emu thread closed: {e}"))?;
    Ok(())
}

#[tauri::command]
fn stop_tas_recording(discard: bool, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    tx.send(EmuCommand::StopTasRecording { discard })
        .map_err(|e| format!("emu thread closed: {e}"))?;
    Ok(())
}

#[tauri::command]
fn start_tas_replay(file_path: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    // Decode here so we can return a clean error to the frontend on a
    // malformed file. The emu thread expects the decoded TasRecording.
    let path = std::path::PathBuf::from(&file_path);
    let rec = oa_savestate::tas::TasRecording::read_from(&path)
        .map_err(|e| format!("TAS read {}: {e}", path.display()))?;
    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    tx.send(EmuCommand::StartTasReplay(Box::new(rec)))
        .map_err(|e| format!("emu thread closed: {e}"))?;
    Ok(())
}

#[tauri::command]
fn stop_tas_replay(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    tx.send(EmuCommand::StopTasReplay).map_err(|e| format!("emu thread closed: {e}"))?;
    Ok(())
}

/// One entry in the per-game recording list. Frontend renders the
/// list in the QuickSettings TAS panel.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct TasListEntry {
    file_path: String,
    display_name: String,
    recorded_at_unix_ms: i64,
    frame_count: u64,
    fps: f64,
    /// `(frame_count / fps).round()` cached server-side so the UI
    /// doesn't have to recompute.
    duration_seconds: f64,
}

#[tauri::command]
fn list_tas_recordings(rom_path: String, state: tauri::State<'_, AppState>) -> Result<Vec<TasListEntry>, String> {
    // Sanitize same as save-slots so the directory matches what the
    // recording-write path used in `EmuCommand::StopTasRecording`.
    let stem = sanitize_stem(&rom_path);
    let dir = state.app_data_dir.join("tas").join(&stem);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|e| format!("read_dir({}): {e}", dir.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("tas") {
            continue;
        }
        match oa_savestate::tas::TasRecording::read_header_only(&path) {
            Ok(h) => {
                let fps = if h.fps > 0.0 { h.fps } else { 60.0 };
                let duration = h.frame_count as f64 / fps;
                out.push(TasListEntry {
                    file_path: path.to_string_lossy().into_owned(),
                    display_name: h.display_name,
                    recorded_at_unix_ms: h.recorded_at_unix_ms,
                    frame_count: h.frame_count,
                    fps,
                    duration_seconds: duration,
                });
            }
            Err(e) => log::warn!("oa-shell: list_tas — {} unreadable: {e:?}", path.display()),
        }
    }
    // Newest first.
    out.sort_by(|a, b| b.recorded_at_unix_ms.cmp(&a.recorded_at_unix_ms));
    Ok(out)
}

#[tauri::command]
fn delete_tas_recording(file_path: String) -> Result<(), String> {
    let path = std::path::PathBuf::from(&file_path);
    // Sanity: refuse to delete anything that doesn't end in .tas.
    if path.extension().and_then(|s| s.to_str()) != Some("tas") {
        return Err("not a .tas file".into());
    }
    std::fs::remove_file(&path).map_err(|e| format!("delete {}: {e}", path.display()))?;
    log::info!("oa-shell: deleted TAS recording {}", path.display());
    Ok(())
}

// ---- Phase 4 slice D — video capture -----------------------------------

#[tauri::command]
fn get_video_state(state: tauri::State<'_, AppState>) -> Result<SharedVideoState, String> {
    let s = state.video_state.lock().map_err(|_| "video_state poisoned".to_string())?;
    Ok(s.clone())
}

#[tauri::command]
fn start_video_capture(display_name: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    tx.send(EmuCommand::StartVideoCapture { display_name })
        .map_err(|e| format!("emu thread closed: {e}"))?;
    Ok(())
}

#[tauri::command]
fn stop_video_capture(discard: bool, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    tx.send(EmuCommand::StopVideoCapture { discard })
        .map_err(|e| format!("emu thread closed: {e}"))?;
    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct VideoClipEntry {
    clip_dir: String,
    display_name: String,
    recorded_at_unix_ms: i64,
    frame_count: u64,
    dropped_frame_count: u64,
    fps: f64,
    width: u32,
    height: u32,
    duration_seconds: f64,
}

#[tauri::command]
fn list_video_clips(rom_path: String, state: tauri::State<'_, AppState>) -> Result<Vec<VideoClipEntry>, String> {
    let stem = sanitize_stem(&rom_path);
    let dir = state.app_data_dir.join("clips").join(&stem);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let entries = std::fs::read_dir(&dir).map_err(|e| format!("read_dir({}): {e}", dir.display()))?;
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let manifest_path = path.join("manifest.json");
        let Ok(raw) = std::fs::read_to_string(&manifest_path) else { continue };
        let Ok(manifest): Result<video_capture::VideoManifest, _> = serde_json::from_str(&raw) else {
            log::warn!("oa-shell: list_video_clips — {} unparseable", manifest_path.display());
            continue;
        };
        let fps = if manifest.fps > 0.0 { manifest.fps } else { 60.0 };
        let duration = manifest.frame_count as f64 / fps;
        out.push(VideoClipEntry {
            clip_dir: path.to_string_lossy().into_owned(),
            display_name: manifest.display_name,
            recorded_at_unix_ms: manifest.stopped_at_unix_ms.max(manifest.started_at_unix_ms),
            frame_count: manifest.frame_count,
            dropped_frame_count: manifest.dropped_frame_count,
            fps,
            width: manifest.width,
            height: manifest.height,
            duration_seconds: duration,
        });
    }
    out.sort_by(|a, b| b.recorded_at_unix_ms.cmp(&a.recorded_at_unix_ms));
    Ok(out)
}

#[tauri::command]
fn delete_video_clip(clip_dir: String) -> Result<(), String> {
    let path = std::path::PathBuf::from(&clip_dir);
    // Sanity: only delete dirs that contain a manifest.json — refuses
    // to nuke arbitrary directories if the frontend passes a stale path.
    if !path.join("manifest.json").is_file() {
        return Err("not a video clip directory".into());
    }
    std::fs::remove_dir_all(&path).map_err(|e| format!("delete {}: {e}", path.display()))?;
    log::info!("oa-shell: deleted video clip {}", path.display());
    Ok(())
}

// ---- Screenshot gallery (Tools → Screenshot gallery) ------------------

/// One screenshot file under `appData/screenshots/<stem>/`. The `path`
/// round-trips through the frontend back to `delete_screenshot` /
/// `open_screenshot_folder` so we don't need to recompute it server-side.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ScreenshotEntry {
    /// Absolute path to the PNG, OS-native separators.
    path: String,
    /// `<timestamp>.png` (or whatever the user renamed it to).
    file_name: String,
    /// Bytes on disk; 0 when stat fails.
    size_bytes: u64,
    /// Modified time, ms since epoch; falls back to 0 when unavailable.
    modified_unix_ms: u64,
}

#[tauri::command]
fn list_screenshots(rom_path: String, state: tauri::State<'_, AppState>) -> Result<Vec<ScreenshotEntry>, String> {
    let stem = sanitize_stem(&rom_path);
    let dir = state.app_data_dir.join("screenshots").join(&stem);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let entries = std::fs::read_dir(&dir).map_err(|e| format!("read_dir({}): {e}", dir.display()))?;
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("png")) != Some(true) {
            continue;
        }
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let (size_bytes, modified_unix_ms) = match entry.metadata() {
            Ok(m) => {
                let size = m.len();
                let modified = m
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                (size, modified)
            }
            Err(_) => (0, 0),
        };
        out.push(ScreenshotEntry {
            path: path.to_string_lossy().into_owned(),
            file_name,
            size_bytes,
            modified_unix_ms,
        });
    }
    out.sort_by(|a, b| b.modified_unix_ms.cmp(&a.modified_unix_ms));
    Ok(out)
}

#[tauri::command]
fn delete_screenshot(path: String) -> Result<(), String> {
    let p = std::path::PathBuf::from(&path);
    // Sanity: only delete .png files inside an `appData/screenshots/`
    // subtree. Prevents the frontend from removing arbitrary files if it
    // hands us a bad path.
    if p.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_ascii_case("png")) != Some(true) {
        return Err("not a .png file".into());
    }
    if !p.components().any(|c| c.as_os_str() == "screenshots") {
        return Err("not under screenshots/".into());
    }
    std::fs::remove_file(&p).map_err(|e| format!("delete {}: {e}", p.display()))?;
    log::info!("oa-shell: deleted screenshot {}", p.display());
    Ok(())
}

#[tauri::command]
fn open_screenshot_folder(rom_path: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let stem = sanitize_stem(&rom_path);
    let dir = state.app_data_dir.join("screenshots").join(&stem);
    if !dir.is_dir() {
        return Err("no screenshots folder for this ROM".into());
    }
    // Reuse the same `open` path that `open_video_clip_folder` uses —
    // see that function for the per-OS rationale.
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer.exe")
            .arg(&dir)
            .spawn()
            .map_err(|e| format!("spawn explorer: {e}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&dir)
            .spawn()
            .map_err(|e| format!("spawn open: {e}"))?;
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(&dir)
            .spawn()
            .map_err(|e| format!("spawn xdg-open: {e}"))?;
    }
    Ok(())
}

// ---- Phase 4 slice E — memory inspector --------------------------------

/// Region-tagged memory bytes returned by `read_memory_region`.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct MemoryRegionInfo {
    region: String,
    available: bool,
    /// Total size in bytes; equal to `bytes.len() + offset` for the
    /// region as a whole. The caller asked for a window of `bytes`
    /// starting at `offset`.
    total_size: u32,
    offset: u32,
    bytes: Vec<u8>,
}

/// Read a window of a memory region. Returns the bytes Vec encoded
/// as a Tauri JSON array (Tauri base-encodes Vec<u8> efficiently).
/// `length = 0` is treated as "from `offset` to the end of region".
/// Out-of-bounds reads return the available subrange (no error).
#[tauri::command]
fn read_memory_region(region: String, offset: u32, length: u32, state: tauri::State<'_, AppState>) -> Result<MemoryRegionInfo, String> {
    let region_id = oa_core::MemoryRegionId::parse(&region)
        .ok_or_else(|| format!("unknown region: {region}"))?;
    let snap = state.memory_snapshot.lock().map_err(|_| "memory_snapshot poisoned".to_string())?;
    let bytes_opt = snap.region(region_id);
    let total_size = bytes_opt.map(|b| b.len() as u32).unwrap_or(0);
    if bytes_opt.is_none() {
        return Ok(MemoryRegionInfo {
            region: region.clone(),
            available: false,
            total_size: 0,
            offset,
            bytes: Vec::new(),
        });
    }
    let bytes = bytes_opt.unwrap();
    let start = (offset as usize).min(bytes.len());
    let end = if length == 0 {
        bytes.len()
    } else {
        (start + length as usize).min(bytes.len())
    };
    Ok(MemoryRegionInfo {
        region,
        available: true,
        total_size,
        offset,
        bytes: bytes[start..end].to_vec(),
    })
}

// ---- Phase 4 slice F — per-game milestones -----------------------------

#[tauri::command]
fn list_milestones(game_id: String, db: tauri::State<'_, library_db::LibraryDb>) -> Result<Vec<library_db::Milestone>, String> {
    db.list_milestones(&game_id)
}

#[tauri::command]
fn add_milestone(milestone: library_db::Milestone, db: tauri::State<'_, library_db::LibraryDb>) -> Result<i64, String> {
    db.add_milestone(&milestone)
}

#[tauri::command]
fn update_milestone(milestone: library_db::Milestone, db: tauri::State<'_, library_db::LibraryDb>) -> Result<(), String> {
    db.update_milestone(&milestone)
}

#[tauri::command]
fn delete_milestone(id: i64, db: tauri::State<'_, library_db::LibraryDb>) -> Result<usize, String> {
    db.delete_milestone(id)
}

#[tauri::command]
fn reset_milestone_progress(id: i64, db: tauri::State<'_, library_db::LibraryDb>) -> Result<(), String> {
    db.reset_milestone_progress(id)
}

/// Push the current game's milestone list into the emu thread's
/// runtime evaluator. Frontend calls this after every successful
/// launch (after the LoadRom Tauri command returns + the DB has the
/// canonical list). Idempotent — sending an empty list clears.
#[tauri::command]
fn arm_milestones(game_id: String, state: tauri::State<'_, AppState>, db: tauri::State<'_, library_db::LibraryDb>) -> Result<usize, String> {
    let list = db.list_milestones(&game_id)?;
    let count = list.len();
    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    tx.send(EmuCommand::LoadMilestones(list)).map_err(|e| format!("emu thread closed: {e}"))?;
    Ok(count)
}

// --- Cheats CRUD (RetroArch parity slice 5) -------------------------------

#[allow(non_snake_case)]
#[tauri::command]
fn list_cheats(gameId: String, db: tauri::State<'_, library_db::LibraryDb>) -> Result<Vec<library_db::Cheat>, String> {
    db.list_cheats(&gameId)
}

#[tauri::command]
fn add_cheat(cheat: library_db::Cheat, db: tauri::State<'_, library_db::LibraryDb>) -> Result<i64, String> {
    db.add_cheat(&cheat)
}

#[tauri::command]
fn update_cheat(cheat: library_db::Cheat, db: tauri::State<'_, library_db::LibraryDb>) -> Result<(), String> {
    db.update_cheat(&cheat)
}

#[tauri::command]
fn delete_cheat(id: i64, db: tauri::State<'_, library_db::LibraryDb>) -> Result<usize, String> {
    db.delete_cheat(id)
}

/// Read the current bytes for a memory region from the per-frame snapshot.
/// Used by the cheat-search Tauri commands instead of hitting the libretro
/// singleton directly — the emu thread already maintains a copy at frame
/// rate (via the memory inspector + milestone runtime path).
fn read_region_snapshot(snap: &MemorySnapshot, region: &str) -> Option<Vec<u8>> {
    let id = oa_core::MemoryRegionId::parse(region)?;
    snap.region(id).map(|bytes| bytes.to_vec())
}

/// Start a cheat search. Snapshots the named region's current bytes from
/// the cached `memory_snapshot` and primes the candidate list with every
/// offset.
///
/// The memory snapshot is normally only refreshed when the memory
/// inspector OR a milestone runtime is using it (per-frame copy cost
/// optimization). To make Start work in the common case where the user
/// jumps straight into cheat search without first opening the inspector,
/// we write a sentinel `Some(Vec::new())` into the matching field —
/// that flips the per-frame "need_snapshot" gate on — then poll briefly
/// for the emu thread to seed the real bytes.
#[tauri::command]
fn start_cheat_search(
    region: String,
    state: tauri::State<'_, AppState>,
) -> Result<cheat_search::CheatSearchSummary, String> {
    use oa_core::MemoryRegionId::*;
    let id = oa_core::MemoryRegionId::parse(&region)
        .ok_or_else(|| format!("unknown region: {region}"))?;
    // Flip the snapshot gate by writing a non-None field into the
    // matching region slot. The next emu-thread frame will overwrite
    // with the real bytes.
    {
        let mut snap = state.memory_snapshot.lock().map_err(|_| "memory_snapshot poisoned".to_string())?;
        match id {
            SaveRam if snap.save_ram.is_none() => snap.save_ram = Some(Vec::new()),
            Rtc if snap.rtc.is_none() => snap.rtc = Some(Vec::new()),
            SystemRam if snap.system_ram.is_none() => snap.system_ram = Some(Vec::new()),
            VideoRam if snap.video_ram.is_none() => snap.video_ram = Some(Vec::new()),
            _ => {}
        }
    }
    // Poll briefly for the emu thread to fill the region. At 60 fps a
    // frame is ~16 ms so 5 × 20 ms covers ~6 frames of slack.
    let mut bytes: Vec<u8> = Vec::new();
    for _ in 0..6 {
        std::thread::sleep(std::time::Duration::from_millis(20));
        let snap = state.memory_snapshot.lock().map_err(|_| "memory_snapshot poisoned".to_string())?;
        if let Some(b) = read_region_snapshot(&snap, &region) {
            if !b.is_empty() {
                bytes = b;
                break;
            }
        }
    }
    if bytes.is_empty() {
        return Err(format!(
            "region {region} couldn't be seeded — make sure a ROM is loaded and the core exposes that region"
        ));
    }
    let session = cheat_search::CheatSearchSession {
        region: region.clone(),
        width: 1,
        previous: bytes.clone(),
        candidates: (0..bytes.len() as u32).collect(),
    };
    let summary = cheat_search::summarize(&session, &bytes, 32);
    *state.cheat_search.lock().map_err(|_| "cheat_search poisoned".to_string())? = Some(session);
    Ok(summary)
}

/// Apply a filter to the active search session. Returns the updated
/// candidate count + the top-N entries with current/previous bytes.
#[tauri::command]
fn filter_cheat_search(
    filter: cheat_search::CheatSearchFilter,
    state: tauri::State<'_, AppState>,
) -> Result<cheat_search::CheatSearchSummary, String> {
    // Pull the current snapshot first so the lock-order is consistent
    // with start_cheat_search.
    let current = {
        let snap = state.memory_snapshot.lock().map_err(|_| "memory_snapshot poisoned".to_string())?;
        let region = state
            .cheat_search
            .lock()
            .map_err(|_| "cheat_search poisoned".to_string())?
            .as_ref()
            .map(|s| s.region.clone())
            .ok_or_else(|| "no active cheat search".to_string())?;
        read_region_snapshot(&snap, &region)
            .ok_or_else(|| format!("region {region} no longer available"))?
    };
    let mut guard = state.cheat_search.lock().map_err(|_| "cheat_search poisoned".to_string())?;
    let session = guard.as_mut().ok_or_else(|| "no active cheat search".to_string())?;
    cheat_search::apply_filter(session, &current, filter);
    Ok(cheat_search::summarize(session, &current, 32))
}

/// Snapshot the current candidate list + values without filtering.
/// Lets the UI refresh after the user does something in-game but BEFORE
/// they pick a filter — handy for "what changed since last filter".
#[tauri::command]
fn peek_cheat_search(
    state: tauri::State<'_, AppState>,
) -> Result<cheat_search::CheatSearchSummary, String> {
    let region = state
        .cheat_search
        .lock()
        .map_err(|_| "cheat_search poisoned".to_string())?
        .as_ref()
        .map(|s| s.region.clone())
        .ok_or_else(|| "no active cheat search".to_string())?;
    let current = {
        let snap = state.memory_snapshot.lock().map_err(|_| "memory_snapshot poisoned".to_string())?;
        read_region_snapshot(&snap, &region).ok_or_else(|| format!("region {region} unavailable"))?
    };
    let guard = state.cheat_search.lock().map_err(|_| "cheat_search poisoned".to_string())?;
    let session = guard.as_ref().ok_or_else(|| "no active cheat search".to_string())?;
    Ok(cheat_search::summarize(session, &current, 32))
}

/// End the active search session. Idempotent — calling with no session
/// is a no-op.
#[tauri::command]
fn end_cheat_search(state: tauri::State<'_, AppState>) -> Result<(), String> {
    *state.cheat_search.lock().map_err(|_| "cheat_search poisoned".to_string())? = None;
    Ok(())
}

/// Re-arm the cheat runtime for a game. Called from `handleLaunch` after
/// `arm_milestones` so the emu thread's frame body picks up the freshly-
/// resolved set on its very next pass. Also called by the per-game cheats
/// editor after Add / Update / Toggle / Delete so live edits take effect
/// without a relaunch.
#[allow(non_snake_case)]
#[tauri::command]
fn arm_cheats(gameId: String, state: tauri::State<'_, AppState>, db: tauri::State<'_, library_db::LibraryDb>) -> Result<usize, String> {
    let list = db.list_cheats(&gameId)?;
    let count = list.len();
    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    tx.send(EmuCommand::LoadCheats(list)).map_err(|e| format!("emu thread closed: {e}"))?;
    Ok(count)
}

/// Phase 4 slice D-2 — convert a PNG-sequence video clip to a single WebM
/// file via the system's `ffmpeg`. Reads the clip's manifest for fps + frame
/// pattern, then shells out to `ffmpeg -framerate FPS -i frame_%06d.png
/// -c:v libvpx-vp9 -b:v 2M -y out.webm` inside `clip_dir`. Blocks on the
/// child process and returns the absolute path of the output file on
/// success. If `ffmpeg` isn't on PATH, returns a clear error string
/// pointing at the install page rather than panicking.
///
/// Synchronous + blocking on purpose — Tauri commands run on a tokio
/// blocking thread, the UI stays responsive, and a frontend spinner reads
/// just as well as a streamed progress channel for v1.
#[tauri::command]
fn convert_video_clip_to_webm(clip_dir: String) -> Result<String, String> {
    let path = std::path::PathBuf::from(&clip_dir);
    if !path.is_dir() {
        return Err(format!("clip dir not found: {}", path.display()));
    }
    let manifest_raw = std::fs::read_to_string(path.join("manifest.json"))
        .map_err(|e| format!("read manifest.json: {e}"))?;
    let manifest: video_capture::VideoManifest = serde_json::from_str(&manifest_raw)
        .map_err(|e| format!("parse manifest.json: {e}"))?;
    let out_path = path.join("clip.webm");
    let fps_str = format!("{:.6}", manifest.fps);

    // `ffmpeg -y` overwrites an existing clip.webm without prompting (the
    // user re-clicking Convert should not stall on an interactive prompt).
    // `-loglevel error` cuts the spammy banner — the failure path still
    // surfaces the meaningful stderr lines via the captured output below.
    let output = std::process::Command::new("ffmpeg")
        .arg("-y")
        .arg("-loglevel").arg("error")
        .arg("-framerate").arg(&fps_str)
        .arg("-i").arg(&manifest.frame_pattern)
        .arg("-c:v").arg("libvpx-vp9")
        .arg("-b:v").arg("2M")
        .arg("clip.webm")
        .current_dir(&path)
        .output();
    let output = match output {
        Ok(o) => o,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(format!(
                "ffmpeg not found on PATH. Install from https://ffmpeg.org/download.html, \
                 add it to PATH, and try again. ({e})"
            ));
        }
        Err(e) => return Err(format!("spawn ffmpeg: {e}")),
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Trim to keep the toast readable; the full log is in the console.
        let snippet: String = stderr.lines().take(4).collect::<Vec<_>>().join(" | ");
        log::warn!("oa-shell: ffmpeg failed ({}): {stderr}", output.status);
        return Err(format!("ffmpeg failed: {snippet}"));
    }
    log::info!(
        "oa-shell: video clip converted -> {} ({} frames @ {:.3} fps)",
        out_path.display(), manifest.frame_count, manifest.fps,
    );
    out_path.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "non-utf8 output path".to_string())
}

#[tauri::command]
fn open_video_clip_folder(clip_dir: String) -> Result<(), String> {
    let path = std::path::PathBuf::from(&clip_dir);
    if !path.is_dir() {
        return Err(format!("clip dir not found: {}", path.display()));
    }
    // Best-effort cross-platform file-manager open. No-op on failure.
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer").arg(&path).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&path).spawn();
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open").arg(&path).spawn();
    }
    Ok(())
}

#[tauri::command]
fn set_window_mode(mode: String, monitor_index: Option<u32>, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let parsed = WindowModeRequest::parse(&mode)
        .ok_or_else(|| format!("unknown window mode: {mode}"))?;
    state.shell_window.apply_window_mode(parsed, monitor_index).map_err(|e| format!("apply_window_mode: {e}"))?;
    log::info!("oa-shell: set_window_mode -> {} (monitor: {monitor_index:?})", mode);
    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct MonitorInfo {
    index: u32,
    name: Option<String>,
    width: u32,
    height: u32,
    position_x: i32,
    position_y: i32,
    scale_factor: f64,
}

#[tauri::command]
fn list_monitors(state: tauri::State<'_, AppState>) -> Result<Vec<MonitorInfo>, String> {
    let monitors = state.shell_window.available_monitors().map_err(|e| format!("available_monitors: {e}"))?;
    Ok(monitors
        .into_iter()
        .enumerate()
        .map(|(i, m)| MonitorInfo {
            index: i as u32,
            name: m.name().map(|s| s.to_string()),
            width: m.size().width,
            height: m.size().height,
            position_x: m.position().x,
            position_y: m.position().y,
            scale_factor: m.scale_factor(),
        })
        .collect())
}

#[tauri::command]
fn get_shell_mode(state: tauri::State<'_, AppState>) -> String {
    state.shell_mode.as_str().to_string()
}

/// Direct-launch payload for the frontend. `None` = library mode (default
/// zero-arg invocation). The frontend reads this on boot to decide whether
/// to hide library chrome and auto-launch the supplied ROM.
#[tauri::command]
fn get_direct_launch_config(
    state: tauri::State<'_, AppState>,
) -> Option<cli::DirectLaunchConfigDto> {
    state
        .direct_launch
        .as_ref()
        .map(cli::DirectLaunchConfigDto::from)
}

/// Fetch a single game row by id. Mirrors the data shape of `list_games` so
/// the frontend can hydrate a synthetic launch RomEntry when direct-launch
/// matched a library entry by SHA-1.
#[tauri::command]
fn get_game(
    id: String,
    db: tauri::State<'_, library_db::LibraryDb>,
) -> Result<Option<library_db::GameRow>, String> {
    db.list_games().map(|games| games.into_iter().find(|g| g.id == id))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ButtonBinding {
    button: String,
    keyboard: Option<String>,
    gamepad: Option<String>,
}

/// Apply a full Bindings map to the InputPoller on Port0. Slots not present
/// in the map are left untouched; slots present but with `None` are cleared.
/// `system_id` picks the right per-system button-name → bit-mask table —
/// without it a Lynx binding like "OPT1" would skip via `pce_bit_for(_)
/// is None` and never reach the poller.
fn apply_bindings_to_poller(input: &mut oa_input::InputPoller, system_id: &str, b: &Bindings) {
    let port = oa_core::PortIndex::Port0;
    // Critical: clear the port first. Different systems use different
    // per-button bit positions (PCE's d-pad is clockwise UP/RIGHT/DOWN/LEFT,
    // libretro/NES/SNES/Lynx is the straight UP/DOWN/LEFT/RIGHT). Without
    // this clear, switching from PCE to NES leaves the arrow keys bound at
    // PCE's bit slots, which the NES identity remap then reads as the wrong
    // directions. Same kind of leakage for the action buttons.
    input.clear_port_bindings(port);
    for (name, pair) in b {
        let Some(mask) = bindings::bit_for(system_id, name) else { continue };
        let key = pair.keyboard.as_deref().and_then(bindings::keycode_from_name);
        let pad = pair.gamepad.as_deref().and_then(bindings::gamepad_from_name);
        input.set_keyboard_binding(port, mask, key);
        input.set_gamepad_binding(port, mask, pad);
    }
}

fn bindings_to_response(system_id: &str, b: &Bindings) -> Vec<ButtonBinding> {
    // Use the system's canonical button list as the iteration order so the
    // UI rows match the system's natural button order rather than
    // alphabetical. Unknown system → empty slice → empty response.
    bindings::buttons_for(system_id)
        .iter()
        .map(|(name, _)| {
            let pair = b.get(*name).cloned().unwrap_or_default();
            ButtonBinding {
                button: (*name).to_string(),
                keyboard: pair.keyboard,
                gamepad: pair.gamepad,
            }
        })
        .collect()
}

/// Return the current bindings for the given system, reading from disk and
/// falling back to compiled-in defaults if the file is missing.
#[tauri::command]
fn get_bindings(system_id: String, state: tauri::State<'_, AppState>) -> Result<Vec<ButtonBinding>, String> {
    if bindings::defaults_for(&system_id).is_none() {
        return Err(format!("no bindings registered for system: {system_id}"));
    }
    let b = bindings::load(&state.app_data_dir, &system_id);
    Ok(bindings_to_response(&system_id, &b))
}

/// Set or clear a single binding slot. `kind` is "keyboard" or "gamepad";
/// `value: None` unbinds. Writes the updated bindings to disk and pushes the
/// full new map to the running emu thread.
#[tauri::command]
fn set_binding(
    system_id: String,
    button: String,
    kind: String,
    value: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ButtonBinding>, String> {
    if bindings::defaults_for(&system_id).is_none() {
        return Err(format!("no bindings registered for system: {system_id}"));
    }
    if bindings::bit_for(&system_id, &button).is_none() {
        return Err(format!("unknown button: {button}"));
    }
    // Validate the value resolves to a real key / pad button. Unbinding (None)
    // is always allowed.
    if let Some(v) = value.as_deref() {
        match kind.as_str() {
            "keyboard" => {
                if bindings::keycode_from_name(v).is_none() {
                    return Err(format!("unknown keyboard key: {v}"));
                }
            }
            "gamepad" => {
                if bindings::gamepad_from_name(v).is_none() {
                    return Err(format!("unknown gamepad button: {v}"));
                }
            }
            other => return Err(format!("unknown kind: {other} (expected keyboard or gamepad)")),
        }
    } else if kind != "keyboard" && kind != "gamepad" {
        return Err(format!("unknown kind: {kind} (expected keyboard or gamepad)"));
    }

    let mut b = bindings::load(&state.app_data_dir, &system_id);
    let entry = b.entry(button.clone()).or_default();
    match kind.as_str() {
        "keyboard" => entry.keyboard = value,
        "gamepad" => entry.gamepad = value,
        _ => unreachable!(),
    }
    bindings::save(&state.app_data_dir, &system_id, &b).map_err(|e| format!("save bindings: {e}"))?;

    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    tx.send(EmuCommand::ApplyBindings(b.clone())).map_err(|e| format!("emu thread closed: {e}"))?;

    log::info!("oa-shell: set_binding {} {} {} = {:?}", system_id, button, kind, b.get(&button));
    Ok(bindings_to_response(&system_id, &b))
}

/// Restore all bindings for the given system to their compiled-in defaults.
/// Writes the defaults to disk and pushes the new map to the running emu.
#[tauri::command]
fn reset_bindings(system_id: String, state: tauri::State<'_, AppState>) -> Result<Vec<ButtonBinding>, String> {
    let defaults = bindings::defaults_for(&system_id)
        .ok_or_else(|| format!("no bindings registered for system: {system_id}"))?;
    bindings::save(&state.app_data_dir, &system_id, &defaults).map_err(|e| format!("save bindings: {e}"))?;

    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    tx.send(EmuCommand::ApplyBindings(defaults.clone())).map_err(|e| format!("emu thread closed: {e}"))?;

    log::info!("oa-shell: reset_bindings({system_id})");
    Ok(bindings_to_response(&system_id, &defaults))
}

/// Returns whatever's currently stored in `appDataDir/shell.json`.
/// May differ from the *active* shell_mode if an env-var override won at startup.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveSlotInfo {
    slot: u32,
    exists: bool,
    size_bytes: u64,
    modified_at_ms: Option<u128>,
    /// Base64 data URL for the slot's PNG thumbnail, if present.
    thumbnail_data_url: Option<String>,
}

#[tauri::command]
fn list_save_slots(rom_path: String, state: tauri::State<'_, AppState>) -> Result<Vec<SaveSlotInfo>, String> {
    let stem = sanitize_stem(&rom_path);
    let mut out = Vec::with_capacity(10);
    for slot in 0..10u32 {
        let bin = slot_path(&state.app_data_dir, &stem, slot);
        let png = bin.with_extension("png");
        let bin_meta = std::fs::metadata(&bin).ok();
        let exists = bin_meta.is_some();
        let size_bytes = bin_meta.as_ref().map(|m| m.len()).unwrap_or(0);
        let modified_at_ms = bin_meta
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis());
        let thumbnail_data_url = if png.exists() {
            std::fs::read(&png).ok().map(|bytes| {
                use base64::Engine;
                let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                format!("data:image/png;base64,{}", b64)
            })
        } else {
            None
        };
        out.push(SaveSlotInfo { slot, exists, size_bytes, modified_at_ms, thumbnail_data_url });
    }
    Ok(out)
}

#[tauri::command]
fn delete_save_slot(rom_path: String, slot: u32, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let stem = sanitize_stem(&rom_path);
    let bin = slot_path(&state.app_data_dir, &stem, slot);
    let png = bin.with_extension("png");
    let bin_err = std::fs::remove_file(&bin).err();
    let png_err = std::fs::remove_file(&png).err();
    // NotFound is success-equivalent (nothing to delete).
    if let Some(e) = bin_err {
        if e.kind() != std::io::ErrorKind::NotFound {
            return Err(format!("delete {}: {e}", bin.display()));
        }
    }
    // PNG deletion failure is non-fatal; thumbnails are auxiliary.
    if let Some(e) = png_err {
        if e.kind() != std::io::ErrorKind::NotFound {
            log::warn!("oa-shell: delete thumbnail {} failed: {e:?}", png.display());
        }
    }
    log::info!("oa-shell: deleted slot {} for {}", slot, stem);
    Ok(())
}

#[tauri::command]
fn get_shell_mode_pref(state: tauri::State<'_, AppState>) -> String {
    read_shell_pref(&state.app_data_dir)
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "two-window".to_string())
}

/// Writes `appDataDir/shell.json`. Takes effect on next launch.
#[tauri::command]
fn set_shell_mode_pref(mode: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let parsed = ShellMode::parse(&mode).ok_or_else(|| format!("unknown shell mode: {mode}"))?;
    write_shell_pref(&state.app_data_dir, parsed).map_err(|e| format!("write shell.json: {e}"))?;
    log::info!("oa-shell: set_shell_mode_pref -> {}", parsed.as_str());
    Ok(())
}

/// Hydrates `LayoutPrefs` from `appDataDir/layout.json` (or returns defaults).
#[tauri::command]
fn get_layout(state: tauri::State<'_, AppState>) -> layout::LayoutPrefs {
    layout::read_layout(&state.app_data_dir)
}

/// Writes `appDataDir/layout.json` with the supplied prefs. Effect is immediate
/// from the frontend's perspective (it's the source of truth); persistence is
/// for restart survival.
#[tauri::command]
fn set_layout(prefs: layout::LayoutPrefs, state: tauri::State<'_, AppState>) -> Result<(), String> {
    layout::write_layout(&state.app_data_dir, &prefs).map_err(|e| format!("write layout.json: {e}"))
}

/// Returns the active presentation mode ("desktop" | "theater" | "cabinet"),
/// read from `appDataDir/presentation.json`. Default = "desktop".
#[tauri::command]
fn get_presentation_mode(state: tauri::State<'_, AppState>) -> String {
    layout::read_presentation(&state.app_data_dir).as_str().to_string()
}

// --- Per-system settings (Phase 2.8 slice C) -------------------------
//
// `appDataDir/systems/<system_id>.json` per system. Holds overrides that
// take precedence over OA-wide prefs at runtime. Per-system core override
// stays in `cores.json` (its existing store); the frontend bridges both
// transparently in the per-system settings UI.

#[tauri::command]
fn get_system_settings(
    system_id: String,
    state: tauri::State<'_, AppState>,
) -> system_settings::SystemSettings {
    system_settings::read_system_settings(&state.app_data_dir, &system_id)
}

#[tauri::command]
fn set_system_settings(
    system_id: String,
    settings: system_settings::SystemSettings,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    system_settings::write_system_settings(&state.app_data_dir, &system_id, &settings)
        .map_err(|e| format!("write system settings: {e}"))
}

/// Frontend asks "what analog sticks does this system use?" to decide
/// whether to render the Analog section on the per-system Bindings page,
/// and what to label the panel(s). Returns "none" / "single" / "dual"
/// with the friendly label(s).
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AnalogSticksInfo {
    kind: &'static str,
    left_label: Option<&'static str>,
    right_label: Option<&'static str>,
}

#[tauri::command]
fn analog_sticks_for_system(system_id: String) -> AnalogSticksInfo {
    match bindings::analog_sticks_for(&system_id) {
        bindings::AnalogSticks::None => AnalogSticksInfo {
            kind: "none",
            left_label: None,
            right_label: None,
        },
        bindings::AnalogSticks::Single { left_label } => AnalogSticksInfo {
            kind: "single",
            left_label: Some(left_label),
            right_label: None,
        },
        bindings::AnalogSticks::Dual { left_label, right_label } => AnalogSticksInfo {
            kind: "dual",
            left_label: Some(left_label),
            right_label: Some(right_label),
        },
    }
}

/// Push per-system analog routing for ONE port to the running InputPoller
/// + persist it under the system_settings file. Frontend calls this after
/// each UI change so tuning takes effect mid-game. Per-game overrides
/// stack on top — when a game is loaded, after this writes the per-system
/// value the frontend should also call `arm_analog_routing(game_id)` to
/// re-resolve per-game on top.
#[tauri::command]
#[allow(non_snake_case)]
fn set_analog_routing(
    system_id: String,
    port: u32,
    routing: system_settings::AnalogPortRouting,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // Persist into the per-port slot of the per-system file.
    let mut settings = system_settings::read_system_settings(&state.app_data_dir, &system_id);
    let mut prefs = settings.analog_routing.clone().unwrap_or_default();
    prefs.set_port_routing(port, routing.clone());
    settings.analog_routing = if prefs.ports.is_empty() { None } else { Some(prefs) };
    system_settings::write_system_settings(&state.app_data_dir, &system_id, &settings)
        .map_err(|e| format!("write system settings: {e}"))?;
    // Push to the running emu thread (no-op if no game is loaded).
    if let Ok(tx) = state.emu_tx.lock() {
        let _ = tx.send(EmuCommand::SetAnalogRouting {
            port,
            routing: routing.to_runtime(),
        });
    }
    Ok(())
}

/// Persist a per-game analog routing override for ONE port and push to
/// the running emu. Stores into `GameOverrides::analog_routing` so the
/// per-game value layers on top of per-system at launch.
#[tauri::command]
#[allow(non_snake_case)]
fn set_analog_routing_for_game(
    gameId: String,
    port: u32,
    routing: system_settings::AnalogPortRouting,
    state: tauri::State<'_, AppState>,
    db: tauri::State<'_, library_db::LibraryDb>,
) -> Result<(), String> {
    let mut overrides = db.get_game_overrides(&gameId)?;
    let mut prefs = overrides.analog_routing.clone().unwrap_or_default();
    prefs.set_port_routing(port, routing.clone());
    overrides.analog_routing = if prefs.ports.is_empty() { None } else { Some(prefs) };
    db.set_game_overrides(&gameId, &overrides)?;
    if let Ok(tx) = state.emu_tx.lock() {
        let _ = tx.send(EmuCommand::SetAnalogRouting {
            port,
            routing: routing.to_runtime(),
        });
    }
    Ok(())
}

/// Resolve per-game libretro device type and push to the emu thread.
/// Same shape as `arm_analog_routing` — frontend calls this after every
/// successful launch. Today the resolution is just per-game override →
/// JOYPAD default (no per-system layer; most systems run JOYPAD by
/// default and only specific games request Mouse / Light Gun / etc).
/// Future polish: per-system default for systems where the canonical
/// peripheral isn't JOYPAD.
///
/// The dispatch happens AFTER `retro_load_game` per the
/// `reference_libretro_controller_after_load_game` memory — Mednafen
/// cores clobber `data_ptr[]` during load. Frontend's launch flow
/// awaits launch_rom and only then fires `arm_libretro_device`.
#[tauri::command]
#[allow(non_snake_case)]
fn arm_libretro_device(
    gameId: String,
    state: tauri::State<'_, AppState>,
    db: tauri::State<'_, library_db::LibraryDb>,
) -> Result<(), String> {
    let _ = db.list_games()?
        .into_iter()
        .find(|g| g.id == gameId)
        .ok_or_else(|| format!("game id not found: {gameId}"))?;
    let game_overrides = db.get_game_overrides(&gameId)?;
    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    // RETRO_DEVICE_JOYPAD = 1 is the universal default at every port.
    // Per-game overrides win per-port. Phase E (2026-05-21) extended
    // this from port-0-only to all five ports — closes SNES Mouse on
    // port 2, arcade coop light-gun, 7800 twin-stick scenarios.
    for (port, device_override) in
        game_overrides.libretro_device_ports().iter().enumerate()
    {
        let device = device_override.unwrap_or(1);
        let _ = tx.send(EmuCommand::SetPortDevice {
            port: port as u32,
            device,
        });
    }
    Ok(())
}

/// Persist a per-game libretro device-type override and push to the
/// running emu. `port` selects which RetroPad port (0..=4); the
/// matching field on `GameOverrides` (`libretro_device` for port 0,
/// `libretro_device_portN` for 1..=4) gets the new value.
///
/// `device = None` clears the override at that port (falls back to the
/// libretro default JOYPAD); `device = Some(0)` explicitly disconnects
/// the port (RETRO_DEVICE_NONE).
#[tauri::command]
#[allow(non_snake_case)]
fn set_libretro_device_for_game(
    gameId: String,
    device: Option<u32>,
    port: Option<u32>,
    state: tauri::State<'_, AppState>,
    db: tauri::State<'_, library_db::LibraryDb>,
) -> Result<(), String> {
    let port = port.unwrap_or(0);
    if port >= 5 {
        return Err(format!("libretro port out of range: {port} (max 4)"));
    }
    let mut overrides = db.get_game_overrides(&gameId)?;
    match port {
        0 => overrides.libretro_device = device,
        1 => overrides.libretro_device_port1 = device,
        2 => overrides.libretro_device_port2 = device,
        3 => overrides.libretro_device_port3 = device,
        4 => overrides.libretro_device_port4 = device,
        _ => unreachable!("range checked above"),
    }
    db.set_game_overrides(&gameId, &overrides)?;
    // Push to the running emu (no-op if game isn't currently loaded).
    if let Ok(tx) = state.emu_tx.lock() {
        let _ = tx.send(EmuCommand::SetPortDevice {
            port,
            device: device.unwrap_or(1), // RETRO_DEVICE_JOYPAD default
        });
    }
    Ok(())
}

/// Resolve per-game → per-system → identity analog routing for all 5
/// ports of `gameId` and push the resolved values to the emu thread.
/// Frontend calls this after every successful launch, same shape as
/// `arm_milestones` / `apply_effective_core_options`.
#[tauri::command]
#[allow(non_snake_case)]
fn arm_analog_routing(
    gameId: String,
    state: tauri::State<'_, AppState>,
    db: tauri::State<'_, library_db::LibraryDb>,
) -> Result<(), String> {
    let row = db.list_games()?
        .into_iter()
        .find(|g| g.id == gameId)
        .ok_or_else(|| format!("game id not found: {gameId}"))?;
    let sys = system_settings::read_system_settings(&state.app_data_dir, &row.system_id);
    let game_overrides = db.get_game_overrides(&gameId)?;
    // Resolution chain for the per-system layer: an explicit user
    // setting wins; otherwise fall back to the compiled-in per-system
    // default (today: N64 gets WASD on the left stick); otherwise
    // identity. The per-system default is the difference between a
    // keyboard-only N64 player who has to enable Mupen's d-pad-to-stick
    // hack and one who can use WASD out of the box.
    let sys_routing = sys.analog_routing.clone()
        .or_else(|| system_settings::default_analog_routing(&row.system_id))
        .unwrap_or_else(system_settings::AnalogRoutingPrefs::identity);
    let game_routing = game_overrides.analog_routing.clone();
    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    for port in 0u32..5 {
        // Per-game wins if it has a non-identity entry; else per-system;
        // else identity.
        let game_port = game_routing.as_ref().map(|g| g.port_routing(port));
        let sys_port = sys_routing.port_routing(port);
        let resolved = match game_port {
            Some(g) if g != system_settings::AnalogPortRouting::identity() => g,
            _ => sys_port,
        };
        let _ = tx.send(EmuCommand::SetAnalogRouting {
            port,
            routing: resolved.to_runtime(),
        });
    }
    Ok(())
}

/// Writes `appDataDir/presentation.json`. Effect is immediate via body data-attr
/// flip on the frontend; persistence is for next launch.
#[tauri::command]
fn set_presentation_mode(mode: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let parsed = layout::PresentationMode::parse(&mode)
        .ok_or_else(|| format!("unknown presentation mode: {mode}"))?;
    layout::write_presentation(&state.app_data_dir, parsed)
        .map_err(|e| format!("write presentation.json: {e}"))?;
    log::info!("oa-shell: set_presentation_mode -> {}", parsed.as_str());
    Ok(())
}

// --- Library DB commands ------------------------------------------------
//
// Frontend's library store calls these instead of touching localStorage.
// Same data shape as the old RomEntry (id, title, systemId, filePath,
// addedAt, coverPath?, coreOverride?, seed?) — serde camelCase via the
// LibraryDb::GameRow type.

/// List every game in the library, sorted by title. Frontend hydrates its
/// in-memory store from this once at startup and keeps it warm via mutation
/// commands.
#[tauri::command]
fn list_games(db: tauri::State<'_, library_db::LibraryDb>) -> Result<Vec<library_db::GameRow>, String> {
    db.list_games()
}

/// List every game in the library, grouped by `(system_id, base_title)`
/// — so different regions / revisions of the same game render as one
/// library tile with variants behind it. Frontend uses this for the
/// library view and the right-click "Run version" submenu.
///
/// Priority resolution: OA-wide region+revision prefs from
/// `library_prefs`, with each game's `system_id` consulting its
/// `system_settings.json` for an override. Per-group user pins live in
/// `game_group_defaults` and override the priority rules.
#[tauri::command]
#[allow(non_snake_case)]
fn list_game_groups(
    db: tauri::State<'_, library_db::LibraryDb>,
    state: tauri::State<'_, crate::media::MediaState>,
) -> Result<Vec<library_groups::GameGroup>, String> {
    let app_data_dir = &state.app_data_dir;
    let prefs = library_prefs::read_library_prefs(app_data_dir);
    let games = db.list_games()?;

    // Group the games by system_id first so we read per-system settings
    // once per system, not per-game. The aggregator runs once per
    // system with that system's effective priority prefs, and we
    // concatenate the resulting groups (they never overlap because the
    // group key includes system_id).
    use std::collections::HashMap;
    let mut by_system: HashMap<String, Vec<library_db::GameRow>> = HashMap::new();
    for g in games {
        by_system.entry(g.system_id.clone()).or_default().push(g);
    }

    let mut out = Vec::new();
    for (system_id, games) in by_system {
        let sys = system_settings::read_system_settings(app_data_dir, &system_id);
        let region_priority = sys
            .region_priority_override
            .clone()
            .unwrap_or_else(|| prefs.region_priority.clone());
        let revision_priority = sys
            .revision_priority_override
            .unwrap_or(prefs.revision_priority);
        let defaults_for_system = db.list_game_group_defaults_for_system(&system_id)?;
        // The aggregator wants (system_id, base_key) tuples; the DB
        // already returns just base_key (lowercased) so wrap.
        let defaults: HashMap<(String, String), String> = defaults_for_system
            .into_iter()
            .map(|(base, gid)| ((system_id.clone(), base), gid))
            .collect();
        let mut groups = library_groups::build_groups(
            games,
            &region_priority,
            revision_priority,
            &defaults,
        );
        out.append(&mut groups);
    }

    // Sort the merged output the same way build_groups sorts a
    // single-system batch — base_title ascending, case-insensitive.
    out.sort_by(|a, b| {
        a.display_base_title
            .to_lowercase()
            .cmp(&b.display_base_title.to_lowercase())
    });
    Ok(out)
}

/// Pin a (system_id, base_title) group to a specific variant. Called
/// when the user picks "Set default version → X" from the right-click
/// submenu. base_title is the parsed (un-annotated) title — the
/// frontend has the parsed shape from `list_game_groups` and sends
/// `display_base_title` here.
#[tauri::command]
#[allow(non_snake_case)]
fn set_game_group_default(
    systemId: String,
    baseTitle: String,
    preferredGameId: String,
    db: tauri::State<'_, library_db::LibraryDb>,
) -> Result<(), String> {
    db.set_game_group_default(&systemId, &baseTitle, &preferredGameId)
}

/// Remove a group's pin. The next render falls back to the priority
/// rules.
#[tauri::command]
#[allow(non_snake_case)]
fn clear_game_group_default(
    systemId: String,
    baseTitle: String,
    db: tauri::State<'_, library_db::LibraryDb>,
) -> Result<(), String> {
    db.clear_game_group_default(&systemId, &baseTitle)
}

/// Read the OA-wide library prefs (region + revision priority).
#[tauri::command]
fn get_library_prefs(
    state: tauri::State<'_, crate::media::MediaState>,
) -> Result<library_prefs::LibraryPrefs, String> {
    Ok(library_prefs::read_library_prefs(&state.app_data_dir))
}

/// Persist OA-wide library prefs. Pretty-printed JSON at
/// `appDataDir/library/prefs.json`.
#[tauri::command]
fn set_library_prefs(
    prefs: library_prefs::LibraryPrefs,
    state: tauri::State<'_, crate::media::MediaState>,
) -> Result<(), String> {
    library_prefs::write_library_prefs(&state.app_data_dir, &prefs)
        .map_err(|e| format!("write library prefs: {e}"))
}

/// Bulk-insert. Returns the number of newly-added rows. Existing rows
/// (matched by file_path) are skipped via INSERT OR IGNORE.
#[tauri::command]
fn add_games(
    entries: Vec<library_db::GameRow>,
    db: tauri::State<'_, library_db::LibraryDb>,
) -> Result<usize, String> {
    db.add_games(&entries)
}

/// Drop seed rows. Called by the frontend on first real ingest so the six
/// TG-16 placeholders don't co-exist with real data.
#[tauri::command]
fn drop_seed_games(db: tauri::State<'_, library_db::LibraryDb>) -> Result<usize, String> {
    db.drop_seed_rows()
}

#[tauri::command]
fn get_game_overrides(
    id: String,
    db: tauri::State<'_, library_db::LibraryDb>,
) -> Result<library_db::GameOverrides, String> {
    db.get_game_overrides(&id)
}

#[tauri::command]
fn set_game_overrides(
    id: String,
    overrides: library_db::GameOverrides,
    db: tauri::State<'_, library_db::LibraryDb>,
) -> Result<(), String> {
    db.set_game_overrides(&id, &overrides)
}

#[tauri::command]
fn update_game_core_override(
    id: String,
    value: Option<String>,
    db: tauri::State<'_, library_db::LibraryDb>,
) -> Result<(), String> {
    db.update_core_override(&id, value.as_deref())
}

#[tauri::command]
fn delete_game(id: String, db: tauri::State<'_, library_db::LibraryDb>) -> Result<(), String> {
    db.delete_game(&id)
}

/// Look up a game's id by its file path. Returns null if no row matches.
/// Used by the auto-remove-on-delete watcher path so the frontend can
/// turn a file-system event into a `delete_game(id)` call without having
/// to scan the live library state.
#[tauri::command]
fn find_game_id_by_path(
    path: String,
    db: tauri::State<'_, library_db::LibraryDb>,
) -> Result<Option<String>, String> {
    db.find_id_by_file_path(&path)
}

/// Delete every game tagged with the given system id. Returns the count
/// removed for the success toast. Used by Settings → Library →
/// "Clear games for this system".
#[tauri::command]
fn delete_games_for_system(
    system_id: String,
    db: tauri::State<'_, library_db::LibraryDb>,
) -> Result<usize, String> {
    db.delete_games_for_system(&system_id)
}

/// Delete every game row. Returns the count removed. Settings → Library →
/// "Reset entire library" calls this AFTER a frontend `confirm()` dialog.
#[tauri::command]
fn delete_all_games(db: tauri::State<'_, library_db::LibraryDb>) -> Result<usize, String> {
    db.delete_all_games()
}

/// RetroArch-parity slice — return the cached option schema + per-system
/// values for a system. Optionally overlays a game's per-game values if
/// `game_id` is provided. Schema is captured at every successful core
/// load (see the LoadRom path); empty until first launch of a system.
#[allow(non_snake_case)]
#[tauri::command]
fn list_core_options(
    systemId: String,
    gameId: Option<String>,
    state: tauri::State<'_, AppState>,
    db: tauri::State<'_, library_db::LibraryDb>,
) -> Result<core_options::CoreOptionsSnapshot, String> {
    let file = core_options::read(&state.app_data_dir, &systemId);
    let game_values = if let Some(id) = gameId {
        db.get_game_overrides(&id)
            .map(|o| o.core_options)
            .unwrap_or_default()
    } else {
        std::collections::HashMap::new()
    };
    Ok(core_options::CoreOptionsSnapshot {
        schema: file.schema,
        categories: file.categories,
        system_values: file.values,
        game_values,
        hidden_keys: file.hidden_keys,
    })
}

/// RetroArch-parity slice — set or clear a per-system core option.
/// `value = None` clears the override (the next-tier value — per-game
/// override OR the schema default — takes effect). When a core is
/// currently loaded for this system, also sends `SetCoreOption` to the
/// emu thread so the change is live without a relaunch.
#[allow(non_snake_case)]
#[tauri::command]
fn set_system_core_option(
    systemId: String,
    key: String,
    value: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut file = core_options::read(&state.app_data_dir, &systemId);
    match value.clone() {
        Some(v) => { file.values.insert(key.clone(), v); }
        None => { file.values.remove(&key); }
    }
    core_options::write(&state.app_data_dir, &systemId, &file)
        .map_err(|e| format!("write core-options: {e}"))?;
    // Push to the live core. If `value` is None (clear) we resolve to the
    // schema default — the next launch picks up per-game overrides naturally;
    // mid-session clears are best-effort with the schema default value.
    let effective = value.unwrap_or_else(|| {
        file.schema
            .iter()
            .find(|o| o.key == key)
            .map(|o| o.default_value.clone())
            .unwrap_or_default()
    });
    if !effective.is_empty() {
        let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
        let _ = tx.send(EmuCommand::SetCoreOption { key, value: effective });
    }
    Ok(())
}

/// RetroArch-parity slice — set or clear a per-game core option. Stored
/// in the existing `games.overrides_json` blob (`GameOverrides.core_options`
/// map). When a core is currently loaded, also pushes to the emu thread.
#[allow(non_snake_case)]
#[tauri::command]
fn set_game_core_option(
    gameId: String,
    key: String,
    value: Option<String>,
    state: tauri::State<'_, AppState>,
    db: tauri::State<'_, library_db::LibraryDb>,
) -> Result<(), String> {
    let mut overrides = db.get_game_overrides(&gameId)?;
    match value.clone() {
        Some(v) => { overrides.core_options.insert(key.clone(), v); }
        None => { overrides.core_options.remove(&key); }
    }
    db.set_game_overrides(&gameId, &overrides)?;
    // For the live core, push the effective value (per-game override OR
    // per-system OR schema default). The frontend can also re-apply the
    // whole option map via `apply_game_core_options` if desired.
    if let Some(row) = db.list_games()?.into_iter().find(|g| g.id == gameId) {
        let file = core_options::read(&state.app_data_dir, &row.system_id);
        let effective = value.unwrap_or_else(|| {
            file.values
                .get(&key)
                .cloned()
                .or_else(|| file.schema.iter().find(|o| o.key == key).map(|o| o.default_value.clone()))
                .unwrap_or_default()
        });
        if !effective.is_empty() {
            let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
            let _ = tx.send(EmuCommand::SetCoreOption { key, value: effective });
        }
    }
    Ok(())
}

/// RetroArch parity slice 7 — set the OA-wide run-ahead frame count.
/// 0 = disabled, 1-5 = peek that many frames ahead each render frame
/// (save_state + N extra run_frames + load_state). Reduces perceived
/// input latency by N frames at the cost of (N + 1 + serialize-size)
/// per render frame. Heavier cores may exceed budget; the renderer-
/// thread's frame-budget timer hides it but the audio sink will start
/// dropping if the loop exceeds 16 ms. v1 is OA-wide; per-system /
/// per-game override is a small follow-up.
#[tauri::command]
fn set_run_ahead(frames: u32, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    tx.send(EmuCommand::SetRunAhead(frames))
        .map_err(|e| format!("emu thread closed: {e}"))?;
    Ok(())
}

/// RetroArch-parity slice — file picker for ROM patches. Returns the
/// absolute path of the selected `.ips` / `.ups` / `.bps` file, or null
/// if the user cancelled. The frontend feeds this into the per-game
/// settings drawer's `patch_path` slot.
#[tauri::command]
async fn pick_patch_file(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let path = app
        .dialog()
        .file()
        .add_filter("ROM patch", &["ips", "ups", "bps"])
        .blocking_pick_file();
    Ok(path.and_then(|fp| fp.as_path().map(|p| p.display().to_string())))
}

/// RetroArch-parity slice — read the cached disc-control snapshot.
/// Returns null for cart games / cores without a disc-control interface.
/// The cache is refreshed by the emu thread on LoadRom + after every
/// successful eject/swap so consumer commands stay reactive.
#[tauri::command]
fn get_disc_state(state: tauri::State<'_, AppState>) -> Result<Option<oa_core::DiscInfo>, String> {
    let guard = state.disc_state.lock().map_err(|_| "disc_state poisoned".to_string())?;
    Ok(guard.clone())
}

/// RetroArch-parity slice — open or close the virtual disc tray.
/// Disc-swap protocol: eject → set image → close. UI buttons typically
/// run the full sequence; this command exposes the steps individually so
/// "Eject" / "Insert disc N" can be separate user actions.
#[tauri::command]
fn set_disc_eject(ejected: bool, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    tx.send(EmuCommand::SetDiscEject(ejected))
        .map_err(|e| format!("emu thread closed: {e}"))?;
    Ok(())
}

/// RetroArch-parity slice — swap to disc `index` (0-based). Only effective
/// while the tray is ejected; cores typically refuse + log otherwise. The
/// UI runs eject → set_image → close as a single user action.
#[allow(non_snake_case)]
#[tauri::command]
fn set_disc_image(index: u32, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    tx.send(EmuCommand::SetDiscImage(index))
        .map_err(|e| format!("emu thread closed: {e}"))?;
    Ok(())
}

/// RetroArch-parity slice — bulk-apply the merged per-system + per-game
/// option set for a specific game to the running core. Called from
/// `handleLaunch` after `set_shader_preset` so the core sees user-chosen
/// values from its very first frame.
#[allow(non_snake_case)]
#[tauri::command]
fn apply_game_core_options(
    gameId: String,
    state: tauri::State<'_, AppState>,
    db: tauri::State<'_, library_db::LibraryDb>,
) -> Result<usize, String> {
    let row = db
        .list_games()?
        .into_iter()
        .find(|g| g.id == gameId)
        .ok_or_else(|| format!("game id not found: {gameId}"))?;
    let file = core_options::read(&state.app_data_dir, &row.system_id);
    let game_overrides = db.get_game_overrides(&gameId)?;
    let merged = core_options::build_effective_values(
        &file.schema,
        &file.values,
        &game_overrides.core_options,
    );
    let count = merged.len();
    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    tx.send(EmuCommand::ApplyCoreOptions(merged))
        .map_err(|e| format!("emu thread closed: {e}"))?;
    Ok(count)
}

#[tauri::command]
fn search_games(
    query: String,
    limit: Option<usize>,
    db: tauri::State<'_, library_db::LibraryDb>,
) -> Result<Vec<library_db::GameRow>, String> {
    db.search_games(&query, limit.unwrap_or(500))
}

// --- Folder + folder_rules commands ----------------------------------
//
// Consumed by the Phase 2.7 Import wizard. The `folders` table tracks
// imported folders (and their scan / watch prefs); `folder_rules` holds
// per-folder extension → system mappings. The wizard reads via
// `list_folders(true)` to pre-populate its mapping editor when revisiting
// a known folder, and writes via `add_folder` / `set_folder_rules` on
// commit.

#[tauri::command]
fn list_folders(
    include_rules: Option<bool>,
    db: tauri::State<'_, library_db::LibraryDb>,
) -> Result<Vec<library_db::Folder>, String> {
    db.list_folders(include_rules.unwrap_or(false))
}

#[tauri::command]
fn add_folder(
    path: String,
    scan_subfolders: bool,
    subfolders_are_systems: bool,
    watch_enabled: bool,
    db: tauri::State<'_, library_db::LibraryDb>,
) -> Result<library_db::Folder, String> {
    db.add_folder(&path, scan_subfolders, subfolders_are_systems, watch_enabled)
}

#[tauri::command]
fn update_folder(
    id: String,
    fields: library_db::FolderUpdate,
    db: tauri::State<'_, library_db::LibraryDb>,
) -> Result<(), String> {
    db.update_folder(&id, fields)
}

#[tauri::command]
fn remove_folder(
    id: String,
    db: tauri::State<'_, library_db::LibraryDb>,
) -> Result<(), String> {
    db.remove_folder(&id)
}

#[tauri::command]
fn list_folder_rules(
    folder_id: String,
    db: tauri::State<'_, library_db::LibraryDb>,
) -> Result<Vec<library_db::FolderRule>, String> {
    db.list_folder_rules(&folder_id)
}

#[tauri::command]
fn set_folder_rules(
    folder_id: String,
    rules: Vec<library_db::FolderRule>,
    db: tauri::State<'_, library_db::LibraryDb>,
) -> Result<usize, String> {
    db.set_folder_rules(&folder_id, &rules)
}

/// Persist the user's drag-reorder from the Settings → Library tab. Bulk
/// `UPDATE folders SET display_order = ?` in one transaction so concurrent
/// `list_folders` calls never see a partially-reordered list.
#[allow(non_snake_case)]
#[tauri::command]
fn reorder_folders(
    orderedIds: Vec<String>,
    db: tauri::State<'_, library_db::LibraryDb>,
) -> Result<(), String> {
    db.reorder_folders(&orderedIds)
}

/// One-shot migration from the WebView's localStorage[oa.library.v1].
/// Called by App.tsx on first launch after the SQLite upgrade — once it
/// returns Ok, the frontend clears the localStorage key.
#[tauri::command]
fn migrate_library_from_local_storage(
    entries: Vec<library_db::GameRow>,
    db: tauri::State<'_, library_db::LibraryDb>,
) -> Result<usize, String> {
    db.migrate_from_local_storage(&entries)
}

/// One-shot migration from the WebView's localStorage[oa.settings.v1]
/// `libraryFolders` array. Called by the settings store on first launch
/// after the SQLite-folders unification (2026-05-21) — once it returns Ok,
/// the frontend strips the field from the settings payload. Returns the
/// count of paths actually inserted; paths already present in `folders`
/// are skipped (idempotent across re-runs / crashes).
#[tauri::command]
fn migrate_folders_from_local_storage(
    paths: Vec<String>,
    db: tauri::State<'_, library_db::LibraryDb>,
) -> Result<usize, String> {
    db.migrate_folders_from_local_storage(&paths)
}

// --- Background scan service ------------------------------------------
//
// Async folder walker that emits `oa://library-scan-progress` events as it
// goes and a final `oa://library-scan-complete` summary. Cancellable via
// `cancel_background_scan(jobId)`. Replaces the synchronous scan_rom_folder
// for large folders without breaking the existing call sites (the sync one
// stays for short scans and the ingest fallback path).

/// Top-level empty-directory check. Returns true when the directory exists
/// and has zero entries (no files AND no subdirectories at the top level).
/// Used by the ImportWizard + the quick-add path in App.tsx to short-circuit
/// before a pointless scan and surface a friendly warning to the operator.
///
/// Intentionally NOT recursive: a deep walk would defeat the "cheap
/// pre-flight check" goal, and the existing "no supported ROMs found"
/// fallback already covers the "has subdirs but they're all empty" case
/// post-scan.
///
/// Errors: surfaced as `Err` when the path doesn't exist or isn't a
/// directory — the frontend treats both as "can't scan this".
#[tauri::command]
fn directory_is_empty(path: String) -> Result<bool, String> {
    let p = std::path::Path::new(&path);
    if !p.exists() {
        return Err(format!("path does not exist: {path}"));
    }
    if !p.is_dir() {
        return Err(format!("not a directory: {path}"));
    }
    let mut iter = std::fs::read_dir(p).map_err(|e| format!("read_dir {path}: {e}"))?;
    Ok(iter.next().is_none())
}

#[tauri::command]
async fn start_background_scan(
    folder: String,
    extensions: Vec<String>,
    handle: tauri::AppHandle,
    scan_state: tauri::State<'_, scan_service::ScanServiceState>,
) -> Result<u64, String> {
    let folder_path = std::path::PathBuf::from(&folder);
    if !folder_path.is_dir() {
        return Err(format!("not a directory: {folder}"));
    }
    let wanted: std::collections::HashSet<String> = extensions
        .into_iter()
        .map(|e| e.trim_start_matches('.').to_ascii_lowercase())
        .collect();

    let job_id = scan_service::next_job_id();
    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let mut jobs = scan_state
            .jobs
            .lock()
            .map_err(|_| "scan jobs lock poisoned".to_string())?;
        jobs.insert(job_id, cancel.clone());
    }

    let jobs_handle = scan_state.jobs.clone();
    let emit_handle = handle.clone();
    let folder_for_task = folder_path.clone();
    let folder_str = folder.clone();

    // spawn_blocking — fs walk is blocking. The Tokio runtime is already
    // present via reqwest's transitive use; we share it.
    tokio::task::spawn_blocking(move || {
        let result = scan_service::run_scan_blocking(
            job_id,
            emit_handle.clone(),
            folder_for_task,
            wanted,
            cancel.clone(),
        );
        let cancelled = cancel.load(std::sync::atomic::Ordering::Relaxed);

        // De-register the job regardless of outcome.
        if let Ok(mut jobs) = jobs_handle.lock() {
            jobs.remove(&job_id);
        }

        // Emit the final summary with the rows attached so the frontend can
        // ingest in one round-trip. 5000 rows ≈ 500 KB JSON — well below
        // any meaningful event-channel ceiling.
        let (rows, error_message) = match result {
            Ok(rows) => (rows, None),
            Err(e) => (Vec::new(), Some(e)),
        };
        let archived = rows.iter().filter(|r| r.archive_inner_path.is_some()).count() as u64;
        let payload = scan_service::ScanCompletePayload {
            job_id,
            folder: folder_str,
            matches: rows.len() as u64,
            archived,
            cancelled,
            error_message,
            rows,
        };
        if let Err(e) = emit_handle.emit("oa://library-scan-complete", &payload) {
            log::warn!("scan_service: emit complete failed: {e:?}");
        }
    });

    Ok(job_id)
}

#[tauri::command]
fn cancel_background_scan(
    job_id: u64,
    scan_state: tauri::State<'_, scan_service::ScanServiceState>,
) -> Result<(), String> {
    let jobs = scan_state
        .jobs
        .lock()
        .map_err(|_| "scan jobs lock poisoned".to_string())?;
    if let Some(cancel) = jobs.get(&job_id) {
        cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        log::info!("scan_service: cancel requested for job {job_id}");
    } else {
        log::debug!("scan_service: cancel for unknown job {job_id} (already finished?)");
    }
    Ok(())
}

/// Reconfigure the filesystem watcher. The frontend calls this on startup
/// (with its persisted tracked folder list) and again whenever the user
/// adds or removes a folder via Settings → Library.
#[tauri::command]
fn set_watched_folders(
    folders: Vec<String>,
    extensions: Vec<String>,
    handle: tauri::AppHandle,
    state: tauri::State<'_, watcher::WatcherState>,
) -> Result<(), String> {
    let paths: Vec<std::path::PathBuf> = folders.into_iter().map(std::path::PathBuf::from).collect();
    let wanted: std::collections::HashSet<String> = extensions
        .into_iter()
        .map(|e| e.trim_start_matches('.').to_ascii_lowercase())
        .collect();
    state.reconfigure(handle, paths, wanted)
}


/// Flip the UI-intercept flag from the WebView. Set to `true` while a rebind
/// capture is active (or a modal expects to consume keystrokes); set back to
/// `false` when the UI releases the keyboard. The emu thread reads this each
/// frame to gate both gameplay input AND the F5/F8/digit hotkeys.
#[tauri::command]
fn set_ui_intercepting(intercepting: bool, state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.ui_intercepting.store(intercepting, Ordering::SeqCst);
    log::debug!("oa-shell: ui_intercepting = {intercepting}");
    Ok(())
}

/// Phase 6 Cross-system slice 3 — set the Game-focus toggle. When `true`,
/// OA hotkeys (F1/F2/F3/F5/F6/F7/F8/F12/Esc/digits/Backspace) stop firing
/// inside the emu thread so the keyboard-passthrough pump can deliver
/// those keys to the core unchallenged. Frontend calls this from the
/// Tools menu checkbox + the Scroll Lock / Ctrl+G hotkey handler.
#[tauri::command]
fn set_game_focus(active: bool, state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.game_focus.store(active, Ordering::SeqCst);
    log::info!("oa-shell: game_focus = {active}");
    Ok(())
}

/// Phase 6 Cross-system slice 3 — read the current Game-focus state.
/// Frontend calls this once at mount to hydrate its local signal so the
/// Tools menu checkbox + toolbar chip reflect the live state.
#[tauri::command]
fn get_game_focus(state: tauri::State<'_, AppState>) -> Result<bool, String> {
    Ok(state.game_focus.load(Ordering::SeqCst))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CoreEntry {
    file_name: String,
    library_name: String,
    library_version: String,
    valid_extensions: String,
    /// Bytes on disk. `0` if the file metadata was unreadable.
    size_bytes: u64,
    /// Last-modified unix ms. `0` if unreadable.
    modified_unix_ms: i64,
    /// Set to the `retro_system_info.need_fullpath` flag — for the cores-page
    /// table chip ("path-only" cores can't load archived ROMs in memory).
    need_fullpath: bool,
    /// `retro_system_info.block_extract` — cores that handle their own
    /// archive contents (mostly MAME-derived).
    block_extract: bool,
    /// Probe error message, if any. When set, the other library_* fields
    /// will be empty; the row still renders so the user can fix the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn core_extension_for_host() -> &'static str {
    if cfg!(windows)          { "dll"   }
    else if cfg!(target_os = "macos") { "dylib" }
    else                              { "so"    }
}

/// Scan `<exe_dir>/cores/` for libretro cores. For each `.dll`/`.so`/`.dylib`,
/// open it briefly via libloading + call `retro_get_system_info` to read
/// display info. Broken files surface with `error` set so the user can see
/// + remove them; valid cores appear with full metadata.
#[tauri::command]
fn list_cores() -> Vec<CoreEntry> {
    let dir = resolve_cores_dir();
    let Ok(rd) = std::fs::read_dir(&dir) else { return Vec::new(); };
    let valid_ext = core_extension_for_host();
    let mut out = Vec::new();
    for entry in rd.flatten() {
        let p = entry.path();
        if !p.is_file() { continue; }
        if p.extension().and_then(|s| s.to_str()) != Some(valid_ext) { continue; }
        let file_name = p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let (size_bytes, modified_unix_ms) = match std::fs::metadata(&p) {
            Ok(m) => {
                let size = m.len();
                let modified = m.modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0);
                (size, modified)
            }
            Err(_) => (0, 0),
        };
        match oa_libretro::probe(&p) {
            Ok(info) => out.push(CoreEntry {
                file_name: info.file_name,
                library_name: info.library_name,
                library_version: info.library_version,
                valid_extensions: info.valid_extensions,
                size_bytes,
                modified_unix_ms,
                need_fullpath: info.need_fullpath,
                block_extract: info.block_extract,
                error: None,
            }),
            Err(e) => {
                log::warn!("oa-shell: probe {} failed: {e:?}", p.display());
                out.push(CoreEntry {
                    file_name,
                    library_name: String::new(),
                    library_version: String::new(),
                    valid_extensions: String::new(),
                    size_bytes,
                    modified_unix_ms,
                    need_fullpath: false,
                    block_extract: false,
                    error: Some(format!("{e}")),
                });
            }
        }
    }
    out.sort_by(|a, b| {
        let an = if a.library_name.is_empty() { &a.file_name } else { &a.library_name };
        let bn = if b.library_name.is_empty() { &b.file_name } else { &b.library_name };
        an.to_lowercase().cmp(&bn.to_lowercase())
    });
    out
}

/// Validate a candidate libretro .dll/.so/.dylib (e.g. from a file picker)
/// without copying it anywhere. Returns the probed metadata on success.
#[tauri::command]
fn probe_core_file(path: String) -> Result<CoreEntry, String> {
    let p = std::path::Path::new(&path);
    let valid_ext = core_extension_for_host();
    if p.extension().and_then(|s| s.to_str()) != Some(valid_ext) {
        return Err(format!("not a {valid_ext} file: {}", p.display()));
    }
    let (size_bytes, modified_unix_ms) = match std::fs::metadata(p) {
        Ok(m) => (
            m.len(),
            m.modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0),
        ),
        Err(e) => return Err(format!("stat {}: {e}", p.display())),
    };
    match oa_libretro::probe(p) {
        Ok(info) => Ok(CoreEntry {
            file_name: info.file_name,
            library_name: info.library_name,
            library_version: info.library_version,
            valid_extensions: info.valid_extensions,
            size_bytes,
            modified_unix_ms,
            need_fullpath: info.need_fullpath,
            block_extract: info.block_extract,
            error: None,
        }),
        Err(e) => Err(format!("probe failed: {e}")),
    }
}

/// Copy a picked .dll/.so/.dylib into `<exe_dir>/cores/`. Validates via
/// `oa_libretro::probe` before writing — broken or non-libretro files are
/// rejected up front. Returns the destination path so the UI can confirm.
/// Refuses to clobber an existing file with the same name; the user must
/// remove the old one first.
#[tauri::command]
fn install_core_from_path(path: String) -> Result<String, String> {
    let src = std::path::PathBuf::from(&path);
    let valid_ext = core_extension_for_host();
    if src.extension().and_then(|s| s.to_str()) != Some(valid_ext) {
        return Err(format!("not a {valid_ext} file"));
    }
    let file_name = src.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .ok_or_else(|| "no filename in path".to_string())?;
    oa_libretro::probe(&src).map_err(|e| format!("probe failed: {e}"))?;

    let cores_dir = resolve_cores_dir();
    if let Err(e) = std::fs::create_dir_all(&cores_dir) {
        return Err(format!("create cores dir: {e}"));
    }
    let dest = cores_dir.join(&file_name);
    if dest.exists() {
        return Err(format!("{file_name} already exists; remove the existing core first"));
    }
    std::fs::copy(&src, &dest).map_err(|e| format!("copy {} -> {}: {e}", src.display(), dest.display()))?;
    log::info!("oa-shell: installed core {} -> {}", src.display(), dest.display());
    Ok(dest.to_string_lossy().into_owned())
}

/// Delete a libretro core .dll/.so/.dylib from `<exe_dir>/cores/`. Refuses
/// if a Path-traversal attempt slips through. Caller is responsible for
/// confirming the destructive action in the UI.
///
/// On Windows the file is held with a shared lock if some other process
/// (including a prior crashed instance) is still loaded. We let the OS
/// error bubble back through so the UI surfaces "in use" cleanly.
#[tauri::command]
fn remove_installed_core(file_name: String) -> Result<(), String> {
    if file_name.contains('/') || file_name.contains('\\') || file_name == ".." || file_name.is_empty() {
        return Err("invalid file_name".into());
    }
    let path = resolve_cores_dir().join(&file_name);
    if !path.exists() {
        return Err(format!("{file_name} not found in cores dir"));
    }
    std::fs::remove_file(&path).map_err(|e| format!("remove {}: {e}", path.display()))?;
    log::info!("oa-shell: removed core {}", path.display());
    Ok(())
}

/// Read the per-system core preference: which `.dll` filename to load for
/// the given system. `None` = use the shell's default (first detected core).
#[tauri::command]
fn get_core_pref(system_id: String, state: tauri::State<'_, AppState>) -> Option<String> {
    read_cores_pref(&state.app_data_dir).get(&system_id).cloned()
}

/// Persist the per-system core preference. Takes effect on next launch.
/// `file_name = None` clears the preference (use the shell's default again).
#[tauri::command]
fn set_core_pref(
    system_id: String,
    file_name: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut prefs = read_cores_pref(&state.app_data_dir);
    match file_name {
        Some(name) => { prefs.insert(system_id.clone(), name); }
        None => { prefs.remove(&system_id); }
    }
    write_cores_pref(&state.app_data_dir, &prefs).map_err(|e| format!("write cores.json: {e}"))?;
    log::info!("oa-shell: set_core_pref({system_id}) -> {:?}", prefs.get(&system_id));
    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AudioDeviceInfo {
    name: String,
    is_default: bool,
    is_active: bool,
}

#[tauri::command]
fn list_audio_devices(state: tauri::State<'_, AppState>) -> Vec<AudioDeviceInfo> {
    let active = read_audio_pref(&state.app_data_dir);
    oa_audio::list_devices()
        .into_iter()
        .map(|d| AudioDeviceInfo {
            is_active: active.as_deref() == Some(d.name.as_str()),
            name: d.name,
            is_default: d.is_default,
        })
        .collect()
}

/// Return the persisted device name (`None` = system default).
#[tauri::command]
fn get_audio_device_pref(state: tauri::State<'_, AppState>) -> Option<String> {
    read_audio_pref(&state.app_data_dir)
}

/// Persist a device choice and hot-swap the running stream. `name = None`
/// selects the system default.
#[tauri::command]
fn set_audio_device_pref(name: Option<String>, state: tauri::State<'_, AppState>) -> Result<(), String> {
    write_audio_pref(&state.app_data_dir, name.as_deref())
        .map_err(|e| format!("write audio.json: {e}"))?;
    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    tx.send(EmuCommand::SetAudioDevice(name.clone()))
        .map_err(|e| format!("emu thread closed: {e}"))?;
    log::info!("oa-shell: set_audio_device_pref -> {:?}", name);
    Ok(())
}

/// Close every WebView window before calling `AppHandle::exit(0)`. Without the
/// explicit close pass, Chromium/WebView2 logs
/// `Failed to unregister class Chrome_WidgetWin_0. Error = 1412` at ERROR
/// level during shutdown — `ERROR_CLASS_HAS_WINDOWS`, the standard teardown
/// race in Chromium-based runtimes. Closing windows first gives the
/// Chromium widget HWNDs a chance to drain before the process exits.
fn graceful_exit(app: &tauri::AppHandle, code: i32) {
    // Sweep any leftover archive-extraction temp dirs before exit. Failure
    // is logged + ignored — the startup sweep will mop up anything missed
    // on next launch.
    if let Some(state) = app.try_state::<AppState>() {
        let temp_root = state.app_data_dir.join("temp");
        archive::sweep_temp(&temp_root);
    }
    for (label, window) in app.webview_windows() {
        if let Err(e) = window.close() {
            log::warn!("oa-shell: close webview window {label}: {e:?}");
        }
    }
    app.exit(code);
}

/// Exit the application. Wired to the header Quit button. The Ctrl/Cmd+Q
/// hotkey path goes through the global-shortcut plugin handler and calls
/// `graceful_exit` directly without round-tripping through this command.
#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    log::info!("oa-shell: quit_app command — exiting");
    graceful_exit(&app, 0);
}

/// Release the currently-loaded ROM. The core stays initialised — next
/// `launch_rom` re-uses it without a full reload. `title` is the
/// operator-visible name for the success toast (e.g. "Bonk's Adventure");
/// pass `None` for a generic "Unloaded" toast. The renderer keeps presenting
/// the last framebuffer until the next ROM loads.
#[tauri::command]
fn unload_rom(title: Option<String>, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    tx.send(EmuCommand::UnloadRom { title })
        .map_err(|e| format!("emu thread closed: {e}"))?;
    drop(tx);
    // If the unloading game was an archived CD set, clean its temp dir now.
    // Archived cart games never created a temp dir (they ran from in-memory
    // bytes) so the cleanup_temp call is a no-op for those.
    if let Ok(mut active) = state.active_archive_entry_id.lock() {
        if let Some(entry_id) = active.take() {
            let temp_root = state.app_data_dir.join("temp");
            archive::cleanup_temp(&temp_root, &entry_id);
        }
    }
    Ok(())
}

#[tauri::command]
#[allow(non_snake_case)]
// Direct-launch `--state-file PATH` (the `stateFile` arg below) restores an
// arbitrary state file after the ROM loads, bypassing the per-game slot
// directory. Library launches leave it None. Mutually exclusive with `slot`.
fn launch_rom(
    path: String,
    slot: Option<u32>,
    stateFile: Option<String>,
    coreOverride: Option<String>,
    archiveInnerPath: Option<String>,
    entryId: Option<String>,
    // The frontend's `SystemId` for this ROM (e.g. "tg16", "lynx"). Older
    // builds may omit it — we default to "tg16" so the long-tail of upstream
    // callers (launchRom-with-old-args) still works.
    systemId: Option<String>,
    state: tauri::State<'_, AppState>,
    db: tauri::State<'_, library_db::LibraryDb>,
) -> Result<(), String> {
    let system_id = systemId.unwrap_or_else(|| "tg16".to_string());
    // Three launch shapes:
    //   1. Raw cart ROM     — read bytes off disk → RomSource::Bytes.
    //   2. Raw CD container — pass path, core opens it → RomSource::Path.
    //   3. Archived ROM     — decoded via archive::decode_file_path; cart
    //      inner → bytes from in-memory zip read; CD inner → extract to
    //      appData/temp/<entry_id>/ → path at the extracted entry point.
    //
    // archive::encode_file_path() format is `<archive>#<inner>`. The frontend
    // sends `path = entry.filePath` (encoded) AND `archiveInnerPath = inner`
    // separately. We could decode from path alone, but the explicit field
    // lets us keep file_path unique in SQLite without parsing-by-suffix
    // ambiguity (some filenames contain `#`).

    let (resolved_path, mut resolved_bytes);
    let is_archived = archiveInnerPath.is_some();

    if let Some(inner) = archiveInnerPath.as_deref() {
        let (archive_path, _) = archive::decode_file_path(&path);
        if !archive_path.is_file() {
            return Err(format!("archive not found: {}", archive_path.display()));
        }
        let inner_ext = Path::new(inner)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();

        if archive::is_cd_entry_extension(&inner_ext) {
            // CD set — extract whole archive into appData/temp/<entry_id>/
            // and point the core at the extracted entry path. Cleanup ties
            // to unload_rom + the startup sweep.
            let id = entryId.clone().ok_or_else(|| "entryId required for archived CD set".to_string())?;
            let temp_dir = state.app_data_dir.join("temp").join(&id);
            // If a previous run of THIS game left a temp dir behind (crashed
            // mid-session), wipe it before re-extracting to avoid stale tracks.
            let _ = std::fs::remove_dir_all(&temp_dir);
            let entry_path = archive::extract_to_temp(&archive_path, inner, &temp_dir)
                .map_err(|e| format!("extract CD set: {e}"))?;
            resolved_path = entry_path.to_string_lossy().into_owned();
            resolved_bytes = Vec::new();
            // Remember the entry_id so unload_rom can clean it up.
            if let Ok(mut active) = state.active_archive_entry_id.lock() {
                *active = Some(id);
            }
            log::info!(
                "oa-shell: launch_rom archived CD set extracted to {} (entry: {})",
                temp_dir.display(), inner
            );
        } else {
            // Cart format — read inner bytes straight into memory. No temp
            // dir, but clear any stale archive tracking from a previous
            // launch so unload doesn't try to clean someone else's temp.
            if let Ok(mut active) = state.active_archive_entry_id.lock() {
                *active = None;
            }
            let bytes = archive::read_inner_to_bytes(&archive_path, inner)
                .map_err(|e| format!("read inner: {e}"))?;
            // Save-state stem derivation uses Path::file_stem() against this
            // string. Using only the inner filename (no archive prefix, no
            // `#` separator) keeps save-state directory names readable —
            // "Bonk's Adventure (USA)" rather than "games.zip#Bonk".
            let inner_name = Path::new(inner)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(inner);
            resolved_path = inner_name.to_string();
            resolved_bytes = bytes;
            log::info!(
                "oa-shell: launch_rom archived cart {} bytes from {}#{}",
                resolved_bytes.len(), archive_path.display(), inner
            );
        }
    } else {
        // Raw path. Existing behavior. Also clear any stale archived-entry
        // tracking so an old archive's temp dir doesn't get cleaned on the
        // next unload of THIS (non-archived) launch.
        if let Ok(mut active) = state.active_archive_entry_id.lock() {
            *active = None;
        }
        let ext = Path::new(&path)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();

        let bytes = if is_cd_extension(&ext) {
            if !Path::new(&path).is_file() {
                return Err(format!("not a file: {path}"));
            }
            Vec::new()
        } else {
            std::fs::read(&path).map_err(|e| format!("read {path}: {e}"))?
        };
        resolved_path = path.clone();
        resolved_bytes = bytes;
    }

    // RetroArch-parity slice — soft patches (IPS / UPS / BPS). Applied to
    // byte-source ROMs only (cart formats); CD images skip since the core
    // opens the .cue/.chd/.m3u directly and we have no shadow-mount path.
    // Per-game patch_path lives in GameOverrides; missing = no patching.
    if !resolved_bytes.is_empty() {
        if let Some(id) = entryId.as_deref() {
            if let Ok(overrides) = db.get_game_overrides(id) {
                if let Some(patch_path) = overrides.patch_path.as_deref() {
                    let pp = std::path::Path::new(patch_path);
                    match patch::apply_from_path(pp, &resolved_bytes) {
                        Ok(patched) => {
                            log::info!(
                                "oa-shell: soft patch applied {} ({} -> {} bytes)",
                                patch_path, resolved_bytes.len(), patched.len()
                            );
                            resolved_bytes = patched;
                        }
                        Err(e) => {
                            log::warn!("oa-shell: soft patch failed ({patch_path}): {e}");
                            return Err(format!("patch {patch_path}: {e}"));
                        }
                    }
                }
            }
        }
    }

    let restore_state_path: Option<PathBuf> = stateFile.as_deref().map(PathBuf::from);
    log::info!(
        "oa-shell: launch_rom dispatch ({}, {} bytes, slot={:?}, stateFile={:?}, coreOverride={:?}, archived={})",
        resolved_path, resolved_bytes.len(), slot, restore_state_path, coreOverride, is_archived
    );

    let tx = state.emu_tx.lock().map_err(|_| "emu_tx poisoned".to_string())?;
    tx.send(EmuCommand::LoadRom {
        path: resolved_path,
        bytes: resolved_bytes,
        restore_slot: slot,
        restore_state_path,
        core_override: coreOverride,
        system_id,
    })
        .map_err(|e| format!("emu thread closed: {e}"))?;

    state.shell_window.focus_game();
    Ok(())
}
