// SQLite-backed game library.
//
// Replaces the WebView's `localStorage[oa.library.v1]` entry from Phase 1-2.
// Source of truth is `appDataDir/library/games.sqlite`. Frontend talks to
// this module only through Tauri commands declared in main.rs — there is no
// per-tile IPC for reads (the entire library is shipped once at startup,
// mutations are individual commands).
//
// Schema is created lazily at first open. Migrations to come (if/when the
// schema changes incompatibly) follow the `PRAGMA user_version` pattern.
//
// FTS5 mirror: `games_fts` is a contentless FTS5 virtual table over
// (title, normalized_title, developer, publisher). Maintained via INSERT/
// UPDATE/DELETE triggers so the application code never has to think about
// keeping the index in sync.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

// Bump this with every new migration. The early-return in
// `bootstrap_schema` (current == SCHEMA_VERSION → Ok) gates the entire
// if-chain, so an unbumped constant silently skips every newer
// migration even when the migration code is present. (We learned this
// the hard way 2026-05-31: Game Info Panel v1's v14→v15 migration
// shipped without the bump, leaving game_info_overrides absent on any
// install that opened the build; System Info Panel v1's v15→v16
// inherited the same hole until the operator caught it via the bake-
// on-launch warn-level log.)
const SCHEMA_VERSION: i32 = 21;

/// Per-game override bag (Phase 2.8 slice D). Lives in `games.overrides_json`
/// as one column rather than dedicated columns because the field set is
/// growing — every new override (region, shader preset, audio profile, …)
/// would otherwise need a schema bump + migration. All fields Option so old
/// rows hydrate as the empty struct. Per-game core override stays in its
/// dedicated `core_override` column (the launch path reads it directly + the
/// existing TileContextMenu / CorePickerMenu pair already write to it).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct GameOverrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scaling_override: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_mode_override: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monitor_index_override: Option<i32>,
    /// Emulator region override (USA / Japan / Europe / …). Distinct from the
    /// per-game cover-art region surface in MediaDb.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region_override: Option<String>,
    /// Phase 3 slice A — per-game shader preset name. Looked up against the
    /// TOML registry (slice C). None = inherit per-system → OA-wide.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shader_preset: Option<String>,
    /// Phase 3 slice C polish — per-game override for the Phosphor composite
    /// weight (`bloom_amount`). None = inherit per-system → preset TOML
    /// default. Applied at launch AFTER `set_shader_preset` so the override
    /// always wins over the TOML's default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bloom_amount: Option<f32>,
    /// RetroArch-parity slice — per-game libretro core-option overrides.
    /// Map of `option_key -> value`. Inherits the per-system override (which
    /// in turn falls back to the schema's `default_value`). Applied at
    /// launch via `set_option` callbacks on the running core.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub core_options: std::collections::HashMap<String, String>,
    /// RetroArch-parity slice — absolute path to an IPS / UPS / BPS patch
    /// applied to the ROM bytes before `retro_load_game`. None = no
    /// patching. Only takes effect for byte-source ROMs (HuCards, NES,
    /// SNES carts, etc.) — CD images are opened by the core directly
    /// and can't be patched in-place from our side.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch_path: Option<String>,
    /// Phase 4 slice A — per-game rewind enabled toggle. None = inherit
    /// the per-system override (or OA-wide).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rewind_enabled: Option<bool>,
    /// Phase 4 slice A — per-game capture interval in frames. None = inherit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rewind_capture_interval_frames: Option<u32>,
    /// Phase 4 slice A — per-game rewind buffer cap in MB. None = inherit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rewind_buffer_megabytes: Option<u32>,
    /// Per-game override for the framebuffer's display_aspect. None =
    /// inherit per-system → core-reported. Mirrors
    /// `SystemSettings::display_aspect_override` semantically.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_aspect_override: Option<f32>,
    /// Per-game per-edge overscan crop. None = inherit per-system →
    /// none. Mirrors `SystemSettings::overscan_crop_override`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overscan_crop_override: Option<crate::system_settings::OverscanCropPrefs>,
    /// Per-game bezel image override. Wins over per-system and over
    /// the active shader preset's TOML default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bezel_image_path: Option<String>,
    /// Phase 2.5 — per-game analog routing override. Stacks on top of
    /// per-system routing: each port's resolution is "per-game wins if
    /// non-identity, else per-system, else identity". Frontend's
    /// `arm_analog_routing` command does the resolution at launch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analog_routing: Option<crate::system_settings::AnalogRoutingPrefs>,
    /// Free-form per-game keypad layout note. Coleco + Intellivision +
    /// O2 (and others with non-game-specific keypads) shipped paper
    /// overlays that told the player what each number meant in the
    /// active game (Donkey Kong: KP1=jump, KP2=climb-up, …). Operators
    /// record those mappings here for the frontend's per-game drawer
    /// to surface as a reference panel — the actual key-to-keypad
    /// bindings still live in the per-system Bindings page; this is
    /// the "what does pressing KP3 in this game DO?" doc string.
    ///
    /// `None` / empty string = no per-game note. Displayed verbatim
    /// in the per-game drawer; no markdown, no structured fields —
    /// freeform so operators can use whichever shorthand they like
    /// ("KP1=climb-up, KP2=climb-down, KP3=jump, KP4=duck").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keypad_layout_note: Option<String>,
    /// Per-game libretro device-type override for port 0. Maps to one
    /// of `oa_libretro::ffi::RETRO_DEVICE_*` values:
    ///
    /// - `Some(1)` (RETRO_DEVICE_JOYPAD) — standard RetroPad (default).
    /// - `Some(2)` (RETRO_DEVICE_MOUSE) — SNES Mouse (Mario Paint),
    ///   GC USB Mouse, generic mouse-as-pointer titles.
    /// - `Some(4)` (RETRO_DEVICE_LIGHTGUN) — NES Zapper, SMS Light
    ///   Phaser, Saturn Stunner, House of the Dead, Time Crisis.
    /// - `Some(5)` (RETRO_DEVICE_ANALOG) — DualShock-shape pad on
    ///   PSX / Saturn 3D Pad / N64 (already polled by some cores
    ///   without an explicit set_port_device).
    /// - `Some(6)` (RETRO_DEVICE_POINTER) — touch / stylus games
    ///   (NDS, Dreamcast pointer-of-the-dead).
    /// - `Some(0)` (RETRO_DEVICE_NONE) — disconnect port 0 entirely.
    /// - `None` — fall through to the system default (JOYPAD at load
    ///   time per `LibretroCore::load_rom`).
    ///
    /// **Dolphin (GameCube / Wii) peripheral subclasses** (sourced from
    /// `libretro/dolphin` `Source/Core/DolphinLibretro/Input.cpp:48-54`):
    /// Dolphin hand-encodes its subclass values as `((N << 8) | base)`
    /// without the canonical libretro `RETRO_DEVICE_SUBCLASS` macro's
    /// `+1` convention — the u32 wire values here are what
    /// `retro_set_controller_port_device` literally receives:
    /// - `Some(513)`  — Wii Remote (sideways grip; NSMB Wii, Excite Truck)
    /// - `Some(769)`  — Wii Remote + Nunchuk (Skyward Sword, Galaxy 1/2, RE4 Wii)
    /// - `Some(1025)` — Wii Remote + Classic Controller (Brawl, Xenoblade)
    /// - `Some(1281)` — Wii Remote + Classic Controller Pro (Monster Hunter Tri G)
    /// - `Some(1537)` — GameCube Controller in Wii mode (MKWii with Wii U adapter)
    ///
    /// Real WiiMote / Bluetooth-passthrough subclass `1536` exists but
    /// is intentionally not surfaced — needs host-side Bluetooth
    /// pairing OA doesn't wire today.
    ///
    /// Frontend's `arm_libretro_device(gameId)` command reads this on
    /// every launch and dispatches a `SetPortDevice` to the emu thread
    /// AFTER `retro_load_game` completes. Mednafen-derived cores
    /// clobber `data_ptr[]` during load, so a pre-load wiring silently
    /// disconnects (see `reference_libretro_controller_after_load_game`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub libretro_device: Option<u32>,
    /// Phase E (2026-05-21) — per-game device-type override for port 1.
    /// Same semantics as `libretro_device` (port 0). Multi-port use
    /// cases: SNES Mouse plugged into port 2 (Mario Paint) alongside
    /// JOYPAD on port 1; arcade coop light-gun games (LIGHTGUN on both
    /// ports); 7800 twin-stick (Robotron mapping its second joystick
    /// onto port 1). `None` falls through to the libretro default
    /// (JOYPAD) — same as port 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub libretro_device_port1: Option<u32>,
    /// Per-game device-type override for port 2. See `libretro_device_port1`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub libretro_device_port2: Option<u32>,
    /// Per-game device-type override for port 3. See `libretro_device_port1`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub libretro_device_port3: Option<u32>,
    /// Per-game device-type override for port 4. See `libretro_device_port1`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub libretro_device_port4: Option<u32>,
    /// Media-taxonomy Phase 4 (2026-05-23) — per-game platform music
    /// override. Plays when this game is highlighted in the library,
    /// overriding the `SystemSettings::platform_music_path` for the
    /// game's system. Stored as a `PathBuf` (absolute or app-data-
    /// relative — frontend resolves). `None` = inherit per-system →
    /// theme default → silence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform_music_path: Option<std::path::PathBuf>,
    /// DOSBox-only — per-game entry-point override. Path is relative
    /// to the game directory (e.g. `"INSTALL.EXE"`, `"DOSBOX/AUTOEXEC.BAT"`).
    /// Covers the ~10% of DOS games where dosbox-pure's auto-detect
    /// picks the wrong .exe — typically install utilities or DOS
    /// shells sitting next to the real game binary. `None` = let
    /// dosbox-pure auto-detect from the directory contents.
    ///
    /// Wired at launch by the `launch_rom` Tauri command: when set,
    /// the resolved path passed to `retro_load_game` becomes
    /// `<game_dir>/<dosbox_entry_point>` instead of just `<game_dir>`.
    /// dosbox-pure interprets the explicit path as the boot target.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dosbox_entry_point: Option<String>,
}

impl GameOverrides {
    /// Per-port device-type override array (port 0..=4). Combines the
    /// `libretro_device` scalar (port 0, kept for back-compat) with the
    /// new `libretro_device_port1..4` siblings. `None` at an index
    /// means "inherit the libretro default" (JOYPAD); explicit
    /// `Some(0)` means RETRO_DEVICE_NONE (disconnect).
    pub fn libretro_device_ports(&self) -> [Option<u32>; 5] {
        [
            self.libretro_device,
            self.libretro_device_port1,
            self.libretro_device_port2,
            self.libretro_device_port3,
            self.libretro_device_port4,
        ]
    }
}

/// Phase 4 slice F — one memory-watching milestone for a game.
///
/// On every emulator frame the emu thread evaluates the predicate
/// `read(region, offset, width) <op> target` against live memory. On
/// rising-edge (predicate was false last frame, true this frame) the
/// milestone is "triggered": an event fires and `triggered_at_unix_ms`
/// gets stamped.
///
/// `edge_only = true` (the default) means the milestone unlocks once
/// per session and stays unlocked until reset (matches "achievement"
/// semantics). `edge_only = false` evaluates fresh each frame — useful
/// for "currently in this state" indicators rather than achievements.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Milestone {
    /// SQLite rowid. None when the client constructs a fresh milestone
    /// for INSERT; populated on read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    pub game_id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Memory region tag matching `oa_core::MemoryRegionId::as_str()`.
    pub region: String,
    pub offset: u32,
    /// Operand width in bytes: 1 / 2 / 4. Larger widths read LE.
    pub width: u8,
    /// Comparison operator: "eq" | "neq" | "gt" | "lt" | "geq" | "leq".
    pub op: String,
    /// Target value to compare against. Stored as i64 to fit any width
    /// + signed/unsigned the operator might want.
    pub target: i64,
    /// Edge-trigger: fire once on transition rather than every frame
    /// the predicate is true. Defaults true (achievement semantics).
    #[serde(default = "default_edge_only")]
    pub edge_only: bool,
    /// Unix ms when the milestone first triggered, or None if not yet.
    /// Reset via `reset_milestone_progress`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triggered_at_unix_ms: Option<i64>,
}

fn default_edge_only() -> bool {
    true
}

/// RetroArch parity slice 5 — per-game cheat. Runtime evaluator writes
/// `value` (little-endian, `width` bytes) into memory at
/// `(region, offset)` every frame the cheat is enabled. Width is
/// constrained to 1 / 2 / 4; rows with other widths silently no-op
/// (defensive against corrupted persisted data).
///
/// `kind` discriminates the dispatch:
/// - `"memory_poke"` — written to core memory every frame via
///   `Core::memory_region_mut` (`main.rs::apply_cheats`).
/// - Any other value (including the generic `"libretro_code"` and the
///   per-system named formats like `"game_genie_nes"`,
///   `"action_replay_gba"`, `"gameshark_gb"` etc.) — `code` string
///   passed verbatim to `Core::cheat_set(idx, enabled, code)` which
///   calls libretro's `retro_cheat_set`. Each core decodes its native
///   formats — `cheat_formats.rs` declares which formats each system's
///   core accepts so the frontend can render a system-aware Type
///   picker + validate input at save time.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Cheat {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    pub game_id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Memory region tag matching `oa_core::MemoryRegionId::as_str()`.
    /// `system_ram` is the typical choice for trainer-style cheats.
    pub region: String,
    pub offset: u32,
    /// Width in bytes: 1 / 2 / 4. Writes are little-endian.
    pub width: u8,
    /// Value to write. Stored as i64 to fit any width + signed flavor.
    pub value: i64,
    /// Apply every frame while enabled. The whole machinery short-
    /// circuits on `!enabled` so this is a hot toggle.
    pub enabled: bool,
    /// "memory_poke" (default) or "libretro_code". Memory pokes write
    /// `value` to `(region, offset)` every frame via memory_region_mut.
    /// Libretro codes pass through `retro_cheat_set(index, enabled, code)`
    /// and let the core decode (Game Genie / GameShark / Action Replay /
    /// raw — per the core's conventions).
    #[serde(default = "default_cheat_kind")]
    pub kind: String,
    /// Raw libretro-format code string. Only meaningful when
    /// `kind == "libretro_code"`; ignored otherwise. For Game Genie
    /// the user-entered code goes in here verbatim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

fn default_cheat_kind() -> String {
    "memory_poke".to_string()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub id: String,
    pub path: String,
    pub scan_subfolders: bool,
    pub subfolders_are_systems: bool,
    pub watch_enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_scanned_at: Option<i64>,
    /// Populated when `list_folders(true)` / `get_folder_by_path(true)` is
    /// called. Empty Vec when none configured; None when the caller didn't
    /// ask for eager-loaded rules.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<FolderRule>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderRule {
    /// Server-side autoincrement id. `None` when the client is constructing
    /// a new rule to insert via `set_folder_rules` — the replace pass
    /// rewrites the whole rule set so client-side ids would be meaningless.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    pub folder_id: String,
    pub match_pattern: String,
    pub system_id: String,
}

/// Partial-update payload for `update_folder`. Any `None` field is left
/// untouched. The wizard's mapping step toggles individual checkboxes; the
/// commit step bumps `last_scanned_at`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderUpdate {
    pub scan_subfolders: Option<bool>,
    pub subfolders_are_systems: Option<bool>,
    pub watch_enabled: Option<bool>,
    pub last_scanned_at: Option<i64>,
}

/// Stable id for a folder by its path. djb2 hash — same family as
/// `romIdFromPath` in the frontend; lets us add-then-remove-then-add the
/// same folder and recover the same id (FK cascade wipes orphan rules
/// between the remove and re-add).
fn folder_id_for_path(path: &str) -> String {
    let mut h: u64 = 5381;
    for byte in path.bytes() {
        h = h.wrapping_mul(33) ^ (byte as u64);
    }
    format!("folder-{:016x}", h)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameRow {
    pub id: String,
    pub title: String,
    pub system_id: String,
    /// The file the user sees in their filesystem. For raw ROMs this is the
    /// ROM itself; for archives this is the .zip/.7z that contains the ROM.
    pub file_path: String,
    pub added_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub core_override: Option<String>,
    #[serde(default)]
    pub seed: bool,
    /// When set, this entry is a ROM living inside the archive at `file_path`.
    /// Format: a posix-style relative path inside the archive, e.g.
    /// `"Bonk's Adventure (USA).pce"` or `"CD-stuff/Castlevania.cue"`. The
    /// launch path passes this to `archive::extract_for_launch` which decides
    /// in-memory-bytes vs extract-to-temp based on the inner extension.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_inner_path: Option<String>,
    /// SHA-1 (40-char lowercase hex) of the ROM bytes, stamped by the
    /// rom_hashes resolve flow. `None` until the user runs Identify ROMs
    /// — `Some` after that, whether or not the hash matched a canonical
    /// entry. The media sync uses this to look the canonical name up
    /// server-side for exact filename matching against libretro-thumbnails.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
    /// Region / catalog serial pulled from libretro-database's rom_hashes
    /// dat on a hash match (e.g. "TGX040080"). Diagnostic for now; surfaced
    /// in the GameInfoModal later.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serial: Option<String>,
    /// Disc identifier extracted from the data track of a CD-shaped
    /// container (Hu7-series code for PCE-CD, SLUS_xxx.xx for PSX,
    /// etc.). Stamped by the `cd_id` resolve flow, parallel to `sha1`
    /// for cart games. `None` until the user runs Identify ROMs — Some
    /// after that, whether or not the disc-id matched a canonical entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disc_id: Option<String>,
    /// Retroverse-UI Phase C3 — operator-tagged favorite. Drives the
    /// Favorites smart-list in the COLLECTIONS tab. Defaults `false`;
    /// toggled via `update_favorite` from the tile heart overlay or
    /// the tile context menu.
    #[serde(default)]
    pub favorite: bool,
    /// Retroverse-UI Phase C3 — operator-marked completed. Drives the
    /// Completed smart-list in the COLLECTIONS tab. Defaults `false`;
    /// toggled via `update_completed` from the tile context menu.
    #[serde(default)]
    pub completed: bool,
    /// Unix-seconds timestamp of the last session for this game.
    /// Written by `update_play_session` in main.rs's close_active_session.
    /// `None` until the operator plays the game at least once.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_played_at: Option<i64>,
    /// Total seconds played across all sessions. Incremented by
    /// `update_play_session`. Defaults 0 for never-played games.
    #[serde(default)]
    pub play_time_secs: i64,
    /// Maximum simultaneous players supported (1, 2, 4, …). Populated
    /// by metadata enrichment; `None` when unknown. Drives the
    /// Multi-Player smart-list in COLLECTIONS.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub players: Option<i64>,
    /// Editorial rating 0.0–5.0 from metadata enrichment. `None`
    /// when unknown. Will drive Hidden Gems smart-list in a follow-up
    /// once a populated source exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rating: Option<f64>,
    /// Phase A1 Sub-phase 4 — FK into `disc_sets.id` when this game
    /// is one disc of a multi-disc set. NULL for single-disc games
    /// and cart games. Stamped by `maybe_stamp_disc_set_membership`
    /// at identify time. Drives library tile grouping (one tile per
    /// set rather than one tile per disc).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disc_set_id: Option<i64>,
    /// Phase A1 Sub-phase 4 — 1-based disc index within the parent
    /// set ("Disc 1" / "Disc 2" / …). NULL for standalone games.
    /// Drives the disc-picker overlay's ordering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disc_number: Option<u32>,
}

/// One canonical rom-hash entry — the source-of-truth shape pulled from
/// libretro-database's `dat/<system>.dat` files. Stored in the
/// `rom_hashes` table keyed on sha1; `apply_rom_hash` resolves a game by
/// hash to the canonical name + serial.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RomHashRow {
    pub sha1: String,
    pub system_id: String,
    pub game_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serial: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crc32: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<i64>,
}

/// Per-track canonical hash entry for disc-shape systems — the
/// source-of-truth shape pulled from libretro-database's redump
/// `metadat/redump/<system>.dat` files. Stored in `rom_hashes_tracks`
/// (parallel to `rom_hashes` for cart shape, per Phase A1 of the
/// virtual library + launcher arc). One row per track of every
/// redump-catalogued disc; the operator's per-track SHA-1 (extracted
/// from .cue+.bin / .chd / .gdi / .iso at identify time) is looked up
/// against this table.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RomTrackRow {
    pub sha1: String,
    pub system_id: String,
    pub game_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serial: Option<String>,
    pub track_number: u32,
    /// Derived heuristic: "DATA" for track 01 (.bin) entries,
    /// "AUDIO" for track 2+ (.bin) entries, "MODE1/2048" for .iso
    /// entries. The dat format doesn't reliably encode mode; the
    /// operator-side hashing path uses the real mode from cue/CHD
    /// metadata, this column is informational only.
    pub track_mode: String,
    pub size_bytes: i64,
}

/// Slim "unidentified game" payload returned by
/// [`LibraryDb::list_unidentified_games_for_system`] for the operator's
/// audit UI. Excludes seed rows. `has_disc_id=true` means the legacy
/// `peek_disc_id` flow stamped a publisher catalog code but the new
/// fuzzy/per-track path hasn't landed a sha1 yet — re-running Identify
/// ROMs will send these through the new path.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnidentifiedGameRow {
    pub id: String,
    pub system_id: String,
    pub title: String,
    /// Operator-visible filesystem path. For archived ROMs this is the
    /// outer .zip/.7z; the inner entry lives on `archive_inner_path`.
    pub file_path: String,
    pub archive_inner_path: Option<String>,
    /// Legacy disc_id stamped without a corresponding sha1 fuzzy match.
    pub has_disc_id: bool,
}

/// Cached per-track hash bundle for one operator-side disc image.
/// Returned by [`LibraryDb::get_game_disc_tracks`]. The mtime/size
/// stamps drive cache invalidation — the caller stat()s the disc
/// file at scan time and compares against these; drift means
/// "operator replaced the dump" → re-hash via
/// [`LibraryDb::clear_game_disc_tracks`] + a fresh
/// [`LibraryDb::write_game_disc_tracks`] call.
#[derive(Clone, Debug)]
pub struct GameDiscTracksCache {
    pub tracks: Vec<crate::disc_track_hash::TrackHash>,
    pub file_mtime: i64,
    pub file_size: i64,
    pub last_hashed_at: i64,
}

/// One MAME ROM-set entry. Keyed by `rom_set` (the .zip basename without
/// extension, e.g. "sf2ce") rather than SHA-1 because MAME's .zip
/// contents drift across MAME versions while the filename stays stable.
/// Populated from libretro-database `metadat/mame/MAME.dat` via the
/// `sync_mame_titles` Tauri command.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MameTitleRow {
    /// .zip basename without extension (lowercased — "sf2ce" for sf2ce.zip).
    pub rom_set: String,
    /// Human-readable title from MAME.dat (e.g. "Street Fighter II: Champion Edition").
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub developer: Option<String>,
}

/// Serial-keyed canonical-title row. One per (system_id, serial). Parsed
/// out of the same libretro-database `.dat` files as `RomHashRow` —
/// every `game (...)` block that carries a `serial "..."` line gets a
/// row here too. CD games consume it via the disc-id extraction path
/// (Phase 2b); cart games can use it as a hash-miss fallback.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GameSerialRow {
    pub system_id: String,
    pub serial: String,
    pub canonical_title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
}

/// Retroverse Phase C3 Slice 12 — operator-built collection metadata.
/// One row per collection in the `custom_collections` table; member
/// rom ids live in the junction table `custom_collection_members`.
/// Returned by `list_custom_collections` so the frontend can render
/// the MY COLLECTIONS sidebar group.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CustomCollectionRow {
    pub id: String,
    pub name: String,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
    /// Live count of rows in `custom_collection_members` whose
    /// `rom_id` still resolves to a row in `games`. Computed at query
    /// time via a JOIN — stale memberships from deleted games don't
    /// inflate the count.
    pub member_count: i64,
}

pub struct LibraryDb {
    inner: Mutex<Connection>,
    #[allow(dead_code)] // diagnostics / future log-on-error
    db_path: PathBuf,
}

impl LibraryDb {
    /// Open (or create) the library DB at `app_data_dir/library/games.sqlite`.
    /// Creates parent directory if missing. Runs schema bootstrap if the DB
    /// is fresh.
    pub fn open(app_data_dir: &Path) -> Result<Self, String> {
        let lib_dir = app_data_dir.join("library");
        std::fs::create_dir_all(&lib_dir).map_err(|e| format!("mkdir library: {e}"))?;
        let db_path = lib_dir.join("games.sqlite");
        let conn = Connection::open(&db_path).map_err(|e| format!("open sqlite: {e}"))?;

        // Reasonable defaults for a desktop launcher DB.
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA temp_store = MEMORY;",
        )
        .map_err(|e| format!("pragma: {e}"))?;

        Self::bootstrap_schema(&conn)?;

        Ok(Self {
            inner: Mutex::new(conn),
            db_path,
        })
    }

    #[allow(dead_code)] // diagnostics; surfaced via a future "open library folder" action
    pub fn path(&self) -> &Path {
        &self.db_path
    }

    fn bootstrap_schema(conn: &Connection) -> Result<(), String> {
        let current: i32 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap_or(0);
        if current == SCHEMA_VERSION {
            return Ok(());
        }
        if current > SCHEMA_VERSION {
            return Err(format!(
                "library DB schema version {current} is newer than this build (expected {SCHEMA_VERSION}); refusing to downgrade",
            ));
        }

        // v0 → v1: full base schema.
        if current < 1 {
            Self::create_v1(conn)?;
            conn.pragma_update(None, "user_version", 1)
                .map_err(|e| format!("set user_version=1: {e}"))?;
            log::info!("library_db: schema v1 initialised");
        }

        // v1 → v2: archive_inner_path column + folder_rules table.
        if current < 2 {
            Self::migrate_v1_to_v2(conn)?;
            conn.pragma_update(None, "user_version", 2)
                .map_err(|e| format!("set user_version=2: {e}"))?;
            log::info!("library_db: schema migrated to v2 (archive support + folder_rules)");
        }

        // v2 → v3: overrides_json column for per-game settings (slice 2.8.D).
        if current < 3 {
            Self::migrate_v2_to_v3(conn)?;
            conn.pragma_update(None, "user_version", 3)
                .map_err(|e| format!("set user_version=3: {e}"))?;
            log::info!("library_db: schema migrated to v3 (per-game overrides_json)");
        }

        // v3 → v4: milestones table (Phase 4 slice F).
        if current < 4 {
            Self::migrate_v3_to_v4(conn)?;
            conn.pragma_update(None, "user_version", 4)
                .map_err(|e| format!("set user_version=4: {e}"))?;
            log::info!("library_db: schema migrated to v4 (per-game milestones)");
        }

        // v4 → v5: retag tg16 rows whose file_path or archive_inner_path ends
        // in a CD container extension as the new `pce-cd` system. Phase 5
        // split — see ROADMAP entry "2026-05-18 — PCE-CD bringup".
        if current < 5 {
            Self::migrate_v4_to_v5(conn)?;
            conn.pragma_update(None, "user_version", 5)
                .map_err(|e| format!("set user_version=5: {e}"))?;
            log::info!("library_db: schema migrated to v5 (split CD games to pce-cd)");
        }

        // v5 → v6: cheats table (RetroArch parity slice 5 — per-game
        // memory-poke cheats).
        if current < 6 {
            Self::migrate_v5_to_v6(conn)?;
            conn.pragma_update(None, "user_version", 6)
                .map_err(|e| format!("set user_version=6: {e}"))?;
            log::info!("library_db: schema migrated to v6 (per-game cheats)");
        }

        // v6 → v7: cheat kind + code columns (slice 8 — Game Genie /
        // Action Replay / GameShark via libretro retro_cheat_set).
        if current < 7 {
            Self::migrate_v6_to_v7(conn)?;
            conn.pragma_update(None, "user_version", 7)
                .map_err(|e| format!("set user_version=7: {e}"))?;
            log::info!("library_db: schema migrated to v7 (cheat kinds)");
        }

        // v7 → v8: hash-based ROM identification. Adds sha1 + serial to
        // games for hits stamped from libretro-database, plus a
        // rom_hashes lookup table keyed on sha1.
        if current < 8 {
            Self::migrate_v7_to_v8(conn)?;
            conn.pragma_update(None, "user_version", 8)
                .map_err(|e| format!("set user_version=8: {e}"))?;
            log::info!("library_db: schema migrated to v8 (rom_hashes + games.sha1/serial)");
        }

        // v8 → v9: serial-keyed canonical-title lookup. Adds
        // games.disc_id (the catalog identifier extracted from disc-based
        // images at Phase 2b) and a game_serials table populated from
        // libretro-database `serial` fields. Together they let the resolve
        // pass identify CD-shaped games (and cart games whose sha1 we
        // didn't match) by publisher serial / disc id.
        if current < 9 {
            Self::migrate_v8_to_v9(conn)?;
            conn.pragma_update(None, "user_version", 9)
                .map_err(|e| format!("set user_version=9: {e}"))?;
            log::info!("library_db: schema migrated to v9 (game_serials + games.disc_id)");
        }

        // v9 → v10: per-group default-variant override. When the user
        // sets "Castlevania (Japan)" as their default variant of the
        // Castlevania group via the right-click menu, we store the
        // chosen game_id here keyed on (system_id, base_title). The
        // priority resolver consults this before falling back to the
        // region+revision priority rules.
        if current < 10 {
            Self::migrate_v9_to_v10(conn)?;
            conn.pragma_update(None, "user_version", 10)
                .map_err(|e| format!("set user_version=10: {e}"))?;
            log::info!("library_db: schema migrated to v10 (game_group_defaults)");
        }

        // v10 → v11: MAME ROM-set → human-title lookup table. Keyed by
        // .zip basename (e.g. "sf2ce") rather than SHA-1 because MAME's
        // .zip contents drift across MAME versions while the filename
        // stays stable. Populated from libretro-database `metadat/mame/
        // MAME.dat` via the new `sync_mame_titles` Tauri command;
        // consulted on game ingest so the library shows
        // "Street Fighter II: Champion Edition" rather than "sf2ce.zip".
        if current < 11 {
            Self::migrate_v10_to_v11(conn)?;
            conn.pragma_update(None, "user_version", 11)
                .map_err(|e| format!("set user_version=11: {e}"))?;
            log::info!("library_db: schema migrated to v11 (mame_titles)");
        }

        // v11 → v12: `display_order` column on `folders` so the Settings →
        // Library tab can persist drag-reorder. Backfill existing rows with
        // their insertion order (rowid) so the migration is a no-op visually.
        // Bumped when finishing the localStorage → SQLite library-folders
        // unification — see DECISIONS.md 2026-05-21 "Library folders: SQLite
        // is the single source of truth".
        if current < 12 {
            Self::migrate_v11_to_v12(conn)?;
            conn.pragma_update(None, "user_version", 12)
                .map_err(|e| format!("set user_version=12: {e}"))?;
            log::info!("library_db: schema migrated to v12 (folders.display_order)");
        }

        // v12 → v13: complementary partial index for list_games_missing_hash.
        // The existing `idx_games_sha1 WHERE sha1 IS NOT NULL` covers the
        // common find-by-sha1 lookup but is the INVERSE of what
        // list_games_missing_hash queries. Pre-fix the missing-hash
        // query (`WHERE sha1 IS NULL OR sha1 = ''`) had no usable
        // index — SQLite did a full table scan, fine at OA's current
        // 5697-row scale but a serious problem at 50K+ row libraries
        // (every Identify ROMs click would scan the whole table).
        if current < 13 {
            Self::migrate_v12_to_v13(conn)?;
            conn.pragma_update(None, "user_version", 13)
                .map_err(|e| format!("set user_version=13: {e}"))?;
            log::info!("library_db: schema migrated to v13 (idx_games_missing_hash)");
        }

        // v13 → v14: Retroverse Phase C3 Slice 12 — custom collections.
        // Operator-built lists alongside the existing smart-lists in the
        // COLLECTIONS tab. Two tables: a parent row per collection + a
        // junction row per (collection, rom) pair. ON DELETE CASCADE on
        // the FK so deleting a collection cleans up its members; the
        // games table doesn't carry a reverse FK because rom rows can
        // come and go (rescan, remove) and we tolerate orphan member
        // rows for a frame — the next `list_collection_members` join
        // filters them out, and a follow-up sweep prunes them on rom
        // delete (see `delete_game`).
        if current < 14 {
            Self::migrate_v13_to_v14(conn)?;
            conn.pragma_update(None, "user_version", 14)
                .map_err(|e| format!("set user_version=14: {e}"))?;
            log::info!("library_db: schema migrated to v14 (custom_collections)");
        }

        // v14 → v15: Game Info Panel v1 — operator local overrides
        // table. Layer 3 of the plan's three-layer data model (file
        // scraper / hand-curated content / local edits). Keyed by
        // (system_id, rom_id) for direct lookup in the query path.
        // Scalar fields get columns; array fields (controls, bugs)
        // stay as JSON blobs because their cardinality is small + no
        // queries need to filter by element.
        if current < 15 {
            Self::migrate_v14_to_v15(conn)?;
            conn.pragma_update(None, "user_version", 15)
                .map_err(|e| format!("set user_version=15: {e}"))?;
            log::info!("library_db: schema migrated to v15 (game_info_overrides)");
        }

        // v15 → v16: System Info Panel v1 — three-layer per-system
        // metadata. L1 (system_info_mame) baked from the slim files
        // shipped under assets/mame-source/; L2 (system_info_curated)
        // baked from docs/cores/<id>/system-info.yaml; L3
        // (system_info_overrides) holds the operator's local edits.
        // system_info_meta carries the content hash that drives the
        // dirty-detection rebake on launch (plan §5).
        if current < 16 {
            Self::migrate_v15_to_v16(conn)?;
            conn.pragma_update(None, "user_version", 16)
                .map_err(|e| format!("set user_version=16: {e}"))?;
            log::info!("library_db: schema migrated to v16 (system_info_* tables)");
        }

        // v16 → v17: MAME ROM-set name resolution — three new tables
        // backing the new listxml-based per-arcade-game catalog.
        // mame_games (L1) holds the bundled slim baked from
        // assets/mame-source/mame-games-slim.json; mame_games_overrides
        // (L3) holds sparse operator edits; mame_games_meta carries
        // the bake-on-launch content hash. The legacy `mame_titles`
        // table (v11) is left in place as a 2nd-tier fallback for
        // operators who synced it via the libretro-database HTTP path
        // before this build shipped.
        if current < 17 {
            Self::migrate_v16_to_v17(conn)?;
            conn.pragma_update(None, "user_version", 17)
                .map_err(|e| format!("set user_version=17: {e}"))?;
            log::info!("library_db: schema migrated to v17 (mame_games* tables)");
        }

        // v17 → v18: background_jobs registry. Persists what OA is
        // doing in the background (HTTP downloads, hash resolves,
        // media sync, folder scans, future per-track SHA-1) so
        // operations survive app restart. First slice of the
        // 5-phase arc in docs/PLANS/background-jobs-and-progress-bar.md.
        // Phase 1 wires `core_download` as the pilot kind; the other
        // 8 kinds in the §"Operations to consolidate" inventory wire
        // in Phase 4. The frontend BackgroundJobsBar lands in Phase 2;
        // auto-resume-on-launch dispatch lands in Phase 3. Phase 1
        // crash detection promotes `state='running'` rows to
        // `state='interrupted'` on next launch (via the lock file +
        // `JobRegistry::promote_running_rows_to_interrupted`) and
        // leaves them there for the operator to retry from the
        // existing per-operation modal.
        if current < 18 {
            Self::migrate_v17_to_v18(conn)?;
            conn.pragma_update(None, "user_version", 18)
                .map_err(|e| format!("set user_version=18: {e}"))?;
            log::info!("library_db: schema migrated to v18 (background_jobs)");
        }

        // v18 → v19: per-track SHA-1 matching for disc-shape systems.
        // Phase A1 of the virtual library + launcher arc — see
        // docs/PLANS/disc-track-sha1-matching.md. Three new tables
        // (rom_hashes_tracks for the canonical redump-synced lookup,
        // game_disc_tracks for the operator's per-game per-track hash
        // cache with mtime+size invalidation, disc_sets for multi-disc
        // grouping) + two nullable columns on `games` (disc_set_id +
        // disc_number). The existing cart-shape `rom_hashes` table is
        // not touched — disc identification is a parallel path
        // dispatched per system_id (matches the v8→v9 game_serials
        // precedent of adding a parallel table rather than mutating
        // rom_hashes' PK).
        if current < 19 {
            Self::migrate_v18_to_v19(conn)?;
            conn.pragma_update(None, "user_version", 19)
                .map_err(|e| format!("set user_version=19: {e}"))?;
            log::info!(
                "library_db: schema migrated to v19 (rom_hashes_tracks + game_disc_tracks + disc_sets + games.disc_set_id/disc_number)"
            );
        }

        // v19 → v20: backfill games.disc_set_id + disc_number for
        // multi-disc games identified BEFORE the Sub-phase 4 backend
        // shipped (2026-06-04 `b6b4ae6`). Those games got their
        // canonical title rewritten via fuzzy match in this session,
        // but list_games_missing_hash excludes already-identified
        // games so the next Identify ROMs run would NOT re-stamp
        // them. This one-shot migration walks every disc game with
        // a `(Disc N)` title and stamps the linkage so the library
        // tile collapse + DiscPickerDialog work for pre-existing
        // identifications too.
        if current < 20 {
            let n = Self::migrate_v19_to_v20(conn)?;
            conn.pragma_update(None, "user_version", 20)
                .map_err(|e| format!("set user_version=20: {e}"))?;
            log::info!(
                "library_db: schema migrated to v20 (backfilled disc_set_id on {n} pre-existing multi-disc games)"
            );
        }

        // v20 → v21: per-core controller-info cache. Populated on every
        // successful core load with the per-port supported-device list
        // the core advertised via RETRO_ENVIRONMENT_SET_CONTROLLER_INFO.
        // Lets the per-game Input dialog render the correct dropdown
        // (FCEUmm Zapper = 258, snes9x Super Scope = 260, etc.) even
        // when no core is currently loaded. Invalidated by .dll mtime
        // change so a core update (or operator-swapped .dll) re-captures
        // on next load. See docs/PLANS/dynamic-controller-info.md Slice 3.
        if current < 21 {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS core_controller_info ( \
                    core_filename TEXT    NOT NULL, \
                    port          INTEGER NOT NULL, \
                    devices_json  TEXT    NOT NULL, \
                    captured_at   INTEGER NOT NULL, \
                    core_mtime    INTEGER NOT NULL, \
                    PRIMARY KEY (core_filename, port) \
                 );",
            )
            .map_err(|e| format!("v21 create core_controller_info: {e}"))?;
            conn.pragma_update(None, "user_version", 21)
                .map_err(|e| format!("set user_version=21: {e}"))?;
            log::info!(
                "library_db: schema migrated to v21 (core_controller_info cache table)"
            );
        }

        Ok(())
    }

    fn migrate_v19_to_v20(conn: &Connection) -> Result<usize, String> {
        // SELECT every game whose title carries a `(Disc N)` suffix
        // AND lacks a disc_set_id stamp. GLOB is faster than LIKE for
        // this pattern and unambiguous about the literal parentheses.
        let mut select_stmt = conn
            .prepare(
                "SELECT id, system_id, title FROM games \
                 WHERE disc_set_id IS NULL AND title GLOB '* (Disc *)*'",
            )
            .map_err(|e| format!("prepare v20 backfill select: {e}"))?;
        let rows: Vec<(String, String, String)> = select_stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
            })
            .map_err(|e| format!("query v20 backfill: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("collect v20 backfill: {e}"))?;
        drop(select_stmt);

        let mut stamped = 0usize;
        for (id, system_id, title) in rows {
            let Some((base_title, disc_n)) =
                crate::rom_hashes::extract_disc_set_candidate(&title)
            else {
                continue;
            };
            let set_id: Option<i64> = conn
                .query_row(
                    "SELECT id FROM disc_sets WHERE system_id = ?1 AND canonical_title = ?2",
                    params![&system_id, &base_title],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| format!("v20 disc_set lookup {id}: {e}"))?;
            let Some(set_id) = set_id else { continue };
            conn.execute(
                "UPDATE games SET disc_set_id = ?1, disc_number = ?2 WHERE id = ?3",
                params![set_id, disc_n as i64, &id],
            )
            .map_err(|e| format!("v20 backfill update {id}: {e}"))?;
            stamped += 1;
        }
        log::debug!("v20 backfill: stamped {stamped} games");
        Ok(stamped)
    }

    fn migrate_v18_to_v19(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            r#"
            -- Phase A1: per-track SHA-1 matching for disc-shape systems.
            -- See docs/PLANS/disc-track-sha1-matching.md for the locked
            -- plan; the 2026-06-03 research pass closed Q1 (full
            -- 2352-byte hash convention), Q2 (separate-table shape vs
            -- extending rom_hashes), and Q3 (chd-crate TOC walk via
            -- manual CHT2 parse + 4-frame TRACK_PADDING accounting).

            -- L1 — canonical SHA-1 lookup. One row per (track of every
            -- redump-catalogued disc-shape game). Synced from the
            -- libretro-database redump dats analogously to the
            -- existing rom_hashes table. PK is (sha1, system_id) to
            -- tighten the cart-shape table's "sha1 is globally unique"
            -- assumption now that we're persisting per-track rows
            -- (multiple disc games legitimately share Track 01 SHA-1s
            -- across regions / revisions). The by_game index supports
            -- the "find candidate from a single matching track, then
            -- verify all tracks for that candidate" lookup pattern.
            CREATE TABLE IF NOT EXISTS rom_hashes_tracks (
                sha1            TEXT NOT NULL,
                system_id       TEXT NOT NULL,
                game_name       TEXT NOT NULL,
                serial          TEXT,
                track_number    INTEGER NOT NULL,
                track_mode      TEXT NOT NULL,
                size_bytes      INTEGER NOT NULL,
                PRIMARY KEY (sha1, system_id)
            );
            CREATE INDEX IF NOT EXISTS idx_rom_hashes_tracks_by_game
                ON rom_hashes_tracks (system_id, game_name);

            -- Operator-side per-game cache. file_mtime + file_size
            -- stamps drive cache invalidation — scan stats the disc
            -- file and compares to the cached stamp; mismatch deletes
            -- cache rows for that game and re-queues disc_track_hash.
            -- ON DELETE CASCADE on the FK so deleting a game cleans
            -- up its track cache. The PK is (game_id, track_number)
            -- which already provides the lookup index for
            -- "find all tracks for this game" (game_id is the prefix);
            -- no extra by-game index needed.
            CREATE TABLE IF NOT EXISTS game_disc_tracks (
                game_id         TEXT NOT NULL,
                track_number    INTEGER NOT NULL,
                sha1            TEXT NOT NULL,
                track_mode      TEXT NOT NULL,
                size_bytes      INTEGER NOT NULL,
                file_mtime      INTEGER NOT NULL,
                file_size       INTEGER NOT NULL,
                last_hashed_at  INTEGER NOT NULL,
                PRIMARY KEY (game_id, track_number),
                FOREIGN KEY (game_id) REFERENCES games (id) ON DELETE CASCADE
            );

            -- Multi-disc grouping. Auto-detected at sync time from the
            -- redump title pattern `Foo (Disc N)` — see the plan's
            -- "Sync flow" section. One row per logical multi-disc
            -- game; individual disc rows in `games` reference it via
            -- the new disc_set_id + disc_number columns added below.
            -- UNIQUE (system_id, canonical_title) enables UPSERT
            -- semantics in `replace_disc_sets_for_system`, which keeps
            -- the autoincrement `id` stable across re-syncs so the
            -- games-table FK references remain valid even when disc
            -- count drifts upstream.
            CREATE TABLE IF NOT EXISTS disc_sets (
                id                  INTEGER PRIMARY KEY AUTOINCREMENT,
                canonical_title     TEXT NOT NULL,
                system_id           TEXT NOT NULL,
                disc_count          INTEGER NOT NULL,
                created_at          INTEGER NOT NULL,
                UNIQUE (system_id, canonical_title)
            );
            CREATE INDEX IF NOT EXISTS idx_disc_sets_system
                ON disc_sets (system_id);
            "#,
        )
        .map_err(|e| format!("v18→v19 create tables: {e}"))?;

        // games.disc_set_id + games.disc_number. SQLite ALTER TABLE has
        // no IF NOT EXISTS for columns; use the PRAGMA table_info
        // pattern from migrate_v7_to_v8 so re-runs after a partial
        // failure are no-ops.
        let existing_cols: std::collections::HashSet<String> = {
            let mut stmt = conn
                .prepare("PRAGMA table_info(games)")
                .map_err(|e| format!("table_info games: {e}"))?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|e| format!("query table_info: {e}"))?;
            let mut out = std::collections::HashSet::new();
            for r in rows {
                out.insert(r.map_err(|e| format!("row table_info: {e}"))?);
            }
            out
        };
        if !existing_cols.contains("disc_set_id") {
            conn.execute("ALTER TABLE games ADD COLUMN disc_set_id INTEGER", [])
                .map_err(|e| format!("alter games add disc_set_id: {e}"))?;
        }
        if !existing_cols.contains("disc_number") {
            conn.execute("ALTER TABLE games ADD COLUMN disc_number INTEGER", [])
                .map_err(|e| format!("alter games add disc_number: {e}"))?;
        }

        // Partial index on the non-NULL disc_set_id rows — most games
        // are not multi-disc-set members, so a full-table index would
        // waste space on NULL rows. Lookup pattern is "find members of
        // a set," which always filters by non-NULL.
        conn.execute_batch(
            r#"
            CREATE INDEX IF NOT EXISTS idx_games_disc_set
                ON games (disc_set_id)
                WHERE disc_set_id IS NOT NULL;
            "#,
        )
        .map_err(|e| format!("v18→v19 disc_set index: {e}"))?;

        Ok(())
    }

    fn migrate_v17_to_v18(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            r#"
            -- Background jobs registry. Schema follows
            -- docs/PLANS/background-jobs-and-progress-bar.md §Schema
            -- verbatim. Active rows (pending/running/paused/
            -- interrupted) are queried via idx_background_jobs_active;
            -- finished rows (completed/failed/cancelled) page through
            -- idx_background_jobs_history for the recent-activity
            -- panel AND for the 100-row rolling-buffer prune.
            --
            -- parent_job_id models two cases:
            --   1. Bulk parents (bulk_core_install w/ N child downloads).
            --   2. Auto-triggered prereqs (Identify hashes → Sync dat).
            -- ON DELETE SET NULL so the rolling-buffer prune of finished
            -- parents doesn't cascade and accidentally drop their
            -- in-flight children.
            --
            -- resume_payload is per-kind JSON: HTTP Range start byte
            -- for core_download; scan stamp for folder_scan; per-track
            -- cache key for disc_track_hash; etc. NULL means "no
            -- checkpoint worth resuming from."
            CREATE TABLE IF NOT EXISTS background_jobs (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                kind            TEXT NOT NULL,
                label           TEXT NOT NULL,
                system_id       TEXT,
                target_id       TEXT,
                parent_job_id   INTEGER REFERENCES background_jobs(id) ON DELETE SET NULL,
                is_prereq       INTEGER NOT NULL DEFAULT 0,
                state           TEXT NOT NULL,
                done            INTEGER NOT NULL DEFAULT 0,
                total           INTEGER,
                unit            TEXT NOT NULL,
                last_event_at   INTEGER NOT NULL,
                started_at      INTEGER NOT NULL,
                finished_at     INTEGER,
                can_resume      INTEGER NOT NULL DEFAULT 1,
                resume_payload  TEXT,
                error_message   TEXT,
                retry_count     INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_background_jobs_active
                ON background_jobs (state, last_event_at);
            CREATE INDEX IF NOT EXISTS idx_background_jobs_history
                ON background_jobs (state, finished_at);
            CREATE INDEX IF NOT EXISTS idx_background_jobs_parent
                ON background_jobs (parent_job_id);
            "#,
        )
        .map_err(|e| format!("v17→v18 migration: {e}"))?;
        Ok(())
    }

    fn migrate_v16_to_v17(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            r#"
            -- L1: per-arcade-game baseline rows baked from the bundled
            -- mame-games-slim.json. `name` is the MAME machine name and
            -- matches the operator's .zip filename stem (lowercase).
            -- `description` is the human title; other fields are
            -- optional. `cloneof` points at the parent ROM-set's name
            -- when the row is a clone (no foreign-key constraint —
            -- parents may not be present in the slim if filtered out).
            CREATE TABLE IF NOT EXISTS mame_games (
                name           TEXT PRIMARY KEY,
                description    TEXT NOT NULL,
                year           TEXT,
                manufacturer   TEXT,
                cloneof        TEXT
            );
            -- Cloneof index supports future parent/clone grouping
            -- queries without a full table scan.
            CREATE INDEX IF NOT EXISTS idx_mame_games_cloneof
                ON mame_games(cloneof) WHERE cloneof IS NOT NULL;

            -- L3: per-install operator overrides. Sparse columnar
            -- shape — every field optional, only rows with at least
            -- one non-NULL override field exist. Mirrors
            -- system_info_overrides' shape. created_at + updated_at
            -- let any future "show my edits" UI sort by recency.
            CREATE TABLE IF NOT EXISTS mame_games_overrides (
                name           TEXT PRIMARY KEY,
                description    TEXT,
                year           TEXT,
                manufacturer   TEXT,
                cloneof        TEXT,
                created_at     INTEGER NOT NULL,
                updated_at     INTEGER NOT NULL
            );

            -- Key-value bag for the bake-on-launch dirty-detection
            -- hash. Single row today (key='l1_hash'); structured as
            -- KV so future cache markers (per-machine rebake stamps,
            -- schema-version migrations, etc.) can land without a
            -- schema bump.
            CREATE TABLE IF NOT EXISTS mame_games_meta (
                key            TEXT PRIMARY KEY,
                value          TEXT NOT NULL
            );
            "#,
        )
        .map_err(|e| format!("v16→v17 migration: {e}"))?;
        Ok(())
    }

    fn migrate_v15_to_v16(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            r#"
            -- L1: MAME baseline rows. system_id is the OA slug
            -- (matches `SystemId` in frontend/src/themes/registry.ts);
            -- one row per slug, even when multiple slugs point at
            -- the same MAME machine (tg16 + pce-cd both → pce).
            -- max_players keeps its integer form here; the merge
            -- layer formats it as a string only when L2 + L3 don't
            -- supply one.
            CREATE TABLE IF NOT EXISTS system_info_mame (
                system_id          TEXT PRIMARY KEY,
                machine_name       TEXT,
                year               TEXT,
                manufacturer       TEXT,
                cpu                TEXT,
                sound              TEXT,
                resolution         TEXT,
                refresh_rate       TEXT,
                max_players        INTEGER,
                peripheral_hints   TEXT,
                description        TEXT
            );

            -- L2: hand-curated YAML rows. Columnar storage (one
            -- column per field) so the per-system Settings drill-in
            -- can query individual fields without parsing a JSON
            -- blob. peripherals is the one Vec field; stored as a
            -- JSON array of {name, glyph}.
            CREATE TABLE IF NOT EXISTS system_info_curated (
                system_id          TEXT PRIMARY KEY,
                manufacturer       TEXT,
                system_type        TEXT,
                generation         TEXT,
                release_date       TEXT,
                discontinued       TEXT,
                units_sold         TEXT,
                media              TEXT,
                cpu                TEXT,
                sound              TEXT,
                resolution         TEXT,
                color_palette      TEXT,
                display_ratio      TEXT,
                architecture       TEXT,
                max_players        TEXT,
                multiplayer        TEXT,
                region             TEXT,
                storage            TEXT,
                ram                TEXT,
                video_output       TEXT,
                aspect_ratio       TEXT,
                refresh_rate       TEXT,
                peripherals        TEXT,
                release_flag       TEXT,
                tagline            TEXT,
                blurb              TEXT,
                sidebar_subline    TEXT,
                schema_version     INTEGER NOT NULL DEFAULT 1,
                last_updated       TEXT
            );

            -- L3: per-install operator overrides. Same columnar
            -- shape as system_info_curated minus the schema/meta
            -- fields (operator edits don't carry their own schema
            -- version — they ride the L2 schema). Sparse: rows only
            -- exist when the operator has at least one non-default
            -- field; a default-constructed SystemInfoOverride
            -- triggers a DELETE rather than an UPSERT.
            CREATE TABLE IF NOT EXISTS system_info_overrides (
                system_id          TEXT PRIMARY KEY,
                manufacturer       TEXT,
                system_type        TEXT,
                generation         TEXT,
                release_date       TEXT,
                discontinued       TEXT,
                units_sold         TEXT,
                media              TEXT,
                cpu                TEXT,
                sound              TEXT,
                resolution         TEXT,
                color_palette      TEXT,
                display_ratio      TEXT,
                architecture       TEXT,
                max_players        TEXT,
                multiplayer        TEXT,
                region             TEXT,
                storage            TEXT,
                ram                TEXT,
                video_output       TEXT,
                aspect_ratio       TEXT,
                refresh_rate       TEXT,
                peripherals        TEXT,
                release_flag       TEXT,
                tagline            TEXT,
                blurb              TEXT,
                sidebar_subline    TEXT,
                created_at         INTEGER NOT NULL,
                updated_at         INTEGER NOT NULL
            );

            -- Key-value bag for the bake-on-launch dirty-detection
            -- hash. Single row today (key='l1_l2_hash'); structured
            -- as KV so future cache markers (per-system rebake
            -- stamps, schema-version migrations, etc.) can land
            -- without a schema bump.
            CREATE TABLE IF NOT EXISTS system_info_meta (
                key                TEXT PRIMARY KEY,
                value              TEXT NOT NULL
            );
            "#,
        )
        .map_err(|e| format!("v15→v16 migration: {e}"))?;
        Ok(())
    }

    fn migrate_v14_to_v15(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS game_info_overrides (
                system_id              TEXT    NOT NULL,
                rom_id                 TEXT    NOT NULL,
                short_summary          TEXT,
                controls_supported     TEXT,
                best_emulator          TEXT,
                best_emulator_reason   TEXT,
                bugs                   TEXT,
                applied_best_emulator  INTEGER NOT NULL DEFAULT 0,
                applied_controls       INTEGER NOT NULL DEFAULT 0,
                created_at             INTEGER NOT NULL,
                updated_at             INTEGER NOT NULL,
                PRIMARY KEY (system_id, rom_id)
            );

            -- Index for the tile-badge "is this game locally-edited?"
            -- query that scans by system_id during a library refresh.
            CREATE INDEX IF NOT EXISTS idx_game_info_overrides_system
                ON game_info_overrides (system_id);
            "#,
        )
        .map_err(|e| format!("v14→v15 migration: {e}"))?;
        Ok(())
    }

    fn migrate_v13_to_v14(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS custom_collections (
                id            TEXT PRIMARY KEY,
                name          TEXT NOT NULL,
                sort_order    INTEGER NOT NULL DEFAULT 0,
                created_at    INTEGER NOT NULL,
                updated_at    INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_custom_collections_sort
                ON custom_collections(sort_order);

            CREATE TABLE IF NOT EXISTS custom_collection_members (
                collection_id TEXT NOT NULL
                    REFERENCES custom_collections(id) ON DELETE CASCADE,
                rom_id        TEXT NOT NULL,
                sort_order    INTEGER NOT NULL DEFAULT 0,
                added_at      INTEGER NOT NULL,
                PRIMARY KEY (collection_id, rom_id)
            );
            CREATE INDEX IF NOT EXISTS idx_custom_collection_members_rom
                ON custom_collection_members(rom_id);
            "#,
        )
        .map_err(|e| format!("create custom_collections schema: {e}"))
    }

    /// Add a complementary partial index for list_games_missing_hash.
    /// SQLite stores partial indexes as compact b-trees keyed on
    /// system_id, so this gives the missing-hash query a per-system
    /// fast path: at 50K rows where ~80% are already stamped, the
    /// index covers the 10K-row remainder without touching the rest.
    fn migrate_v12_to_v13(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            r#"
            CREATE INDEX IF NOT EXISTS idx_games_missing_hash
                ON games(system_id)
                WHERE sha1 IS NULL OR sha1 = '';
            "#,
        )
        .map_err(|e| format!("create idx_games_missing_hash: {e}"))
    }

    fn migrate_v11_to_v12(conn: &Connection) -> Result<(), String> {
        // Idempotent ALTER (mirrors migrate_v1_to_v2's pattern) so re-running
        // on a partially-migrated DB doesn't error on "column already exists."
        let has_column: bool = conn
            .prepare("PRAGMA table_info(folders)")
            .and_then(|mut stmt| {
                let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
                let mut found = false;
                for r in rows {
                    if r? == "display_order" {
                        found = true;
                        break;
                    }
                }
                Ok(found)
            })
            .unwrap_or(false);
        if !has_column {
            conn.execute(
                "ALTER TABLE folders ADD COLUMN display_order INTEGER NOT NULL DEFAULT 0",
                [],
            )
            .map_err(|e| format!("alter folders add display_order: {e}"))?;
            // Seed display_order from rowid so existing folders keep their
            // chronological order on first launch after upgrade.
            conn.execute("UPDATE folders SET display_order = rowid", [])
                .map_err(|e| format!("seed folders.display_order: {e}"))?;
        }
        Ok(())
    }

    fn migrate_v10_to_v11(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS mame_titles (
                rom_set    TEXT PRIMARY KEY,
                title      TEXT NOT NULL,
                year       TEXT,
                developer  TEXT
            );
            "#,
        )
        .map_err(|e| format!("create mame_titles table: {e}"))
    }

    fn migrate_v9_to_v10(conn: &Connection) -> Result<(), String> {
        // base_title is the parsed-and-lowercased title — see
        // `title_parse::parse_canonical_title`. Lowercasing happens at
        // write time so the (system_id, base_title) PK matches across
        // case-quirky upstream titles.
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS game_group_defaults (
                system_id          TEXT NOT NULL,
                base_title         TEXT NOT NULL,
                preferred_game_id  TEXT NOT NULL,
                PRIMARY KEY (system_id, base_title),
                FOREIGN KEY (preferred_game_id) REFERENCES games(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_group_defaults_system
                ON game_group_defaults(system_id);
            "#,
        )
        .map_err(|e| format!("create game_group_defaults table: {e}"))
    }

    fn migrate_v8_to_v9(conn: &Connection) -> Result<(), String> {
        // Idempotent ALTER — re-running on a partially-migrated DB
        // doesn't blow up. Mirrors the pattern in migrate_v7_to_v8.
        let existing_cols: std::collections::HashSet<String> = {
            let mut stmt = conn
                .prepare("PRAGMA table_info(games)")
                .map_err(|e| format!("table_info games: {e}"))?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|e| format!("query table_info: {e}"))?;
            let mut out = std::collections::HashSet::new();
            for r in rows {
                out.insert(r.map_err(|e| format!("row table_info: {e}"))?);
            }
            out
        };
        if !existing_cols.contains("disc_id") {
            conn.execute("ALTER TABLE games ADD COLUMN disc_id TEXT", [])
                .map_err(|e| format!("alter games add disc_id: {e}"))?;
        }
        // The `game_serials` table is a serial-keyed canonical-title
        // lookup, parallel to `rom_hashes` (which is sha1-keyed). One
        // row per game (system_id, serial). Populated from
        // libretro-database `.dat` files alongside the rom_hashes upsert.
        //
        // For CD-based games the resolve pass extracts the on-disc serial
        // (Phase 2b — `cd_id` module) and looks it up here. For cart
        // games this is a fallback when the file sha1 missed — e.g. a
        // user's regional/revision dump shares its catalog serial with
        // the canonical entry even when the bytes differ.
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS game_serials (
                system_id        TEXT NOT NULL,
                serial           TEXT NOT NULL,
                canonical_title  TEXT NOT NULL,
                region           TEXT,
                PRIMARY KEY (system_id, serial)
            );
            CREATE INDEX IF NOT EXISTS idx_game_serials_system
                ON game_serials(system_id);
            CREATE INDEX IF NOT EXISTS idx_games_disc_id
                ON games(disc_id) WHERE disc_id IS NOT NULL;
            "#,
        )
        .map_err(|e| format!("create game_serials table: {e}"))
    }

    fn migrate_v7_to_v8(conn: &Connection) -> Result<(), String> {
        // Add sha1 + serial to games. Idempotent — check via table_info
        // so re-running the migration after a partial failure doesn't
        // ERROR-out on "column already exists."
        let existing_cols: std::collections::HashSet<String> = {
            let mut stmt = conn
                .prepare("PRAGMA table_info(games)")
                .map_err(|e| format!("table_info games: {e}"))?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|e| format!("query table_info: {e}"))?;
            let mut out = std::collections::HashSet::new();
            for r in rows {
                out.insert(r.map_err(|e| format!("row table_info: {e}"))?);
            }
            out
        };
        if !existing_cols.contains("sha1") {
            conn.execute("ALTER TABLE games ADD COLUMN sha1 TEXT", [])
                .map_err(|e| format!("alter games add sha1: {e}"))?;
        }
        if !existing_cols.contains("serial") {
            conn.execute("ALTER TABLE games ADD COLUMN serial TEXT", [])
                .map_err(|e| format!("alter games add serial: {e}"))?;
        }
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS rom_hashes (
                sha1        TEXT PRIMARY KEY,
                system_id   TEXT NOT NULL,
                game_name   TEXT NOT NULL,
                serial      TEXT,
                crc32       TEXT,
                size_bytes  INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_rom_hashes_system ON rom_hashes(system_id);
            CREATE INDEX IF NOT EXISTS idx_games_sha1 ON games(sha1) WHERE sha1 IS NOT NULL;
            -- Complementary partial index for list_games_missing_hash —
            -- see migrate_v12_to_v13 for the rationale. Bootstrapped
            -- here so fresh installs get it without waiting for the
            -- v13 migration to run.
            CREATE INDEX IF NOT EXISTS idx_games_missing_hash
                ON games(system_id)
                WHERE sha1 IS NULL OR sha1 = '';
            "#,
        )
        .map_err(|e| format!("create rom_hashes table: {e}"))
    }

    fn migrate_v6_to_v7(conn: &Connection) -> Result<(), String> {
        // Add `kind` (default 'memory_poke' so v6 rows still apply via
        // memory_region_mut) + `code` (NULL for memory_poke, Some(code)
        // for libretro_code). SQLite ALTER TABLE doesn't support
        // IF NOT EXISTS, so we check the column list first — same
        // pattern as migrate_v1_to_v2's archive_inner_path. Re-running
        // the migration (e.g. tests that rewind user_version) must be
        // a no-op.
        let existing_cols: std::collections::HashSet<String> = {
            let mut stmt = conn
                .prepare("PRAGMA table_info(cheats)")
                .map_err(|e| format!("table_info cheats: {e}"))?;
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|e| format!("query table_info: {e}"))?;
            let mut out = std::collections::HashSet::new();
            for r in rows {
                out.insert(r.map_err(|e| format!("row table_info: {e}"))?);
            }
            out
        };
        if !existing_cols.contains("kind") {
            conn.execute(
                "ALTER TABLE cheats ADD COLUMN kind TEXT NOT NULL DEFAULT 'memory_poke'",
                [],
            )
            .map_err(|e| format!("alter cheats add kind: {e}"))?;
        }
        if !existing_cols.contains("code") {
            conn.execute("ALTER TABLE cheats ADD COLUMN code TEXT", [])
                .map_err(|e| format!("alter cheats add code: {e}"))?;
        }
        Ok(())
    }

    /// Cheats table — one row per cheat. The runtime evaluator (emu
    /// thread) writes `value` (little-endian, `width` bytes) to the
    /// memory at `(region, offset)` every frame the cheat is enabled.
    /// Same shape as the milestones table — region tags match
    /// `MemoryRegionId::as_str()`. Width is stored u8 but constrained
    /// 1 / 2 / 4 (matches what we write LE).
    fn migrate_v5_to_v6(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS cheats (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                game_id      TEXT NOT NULL,
                name         TEXT NOT NULL,
                description  TEXT NOT NULL DEFAULT '',
                region       TEXT NOT NULL,
                offset       INTEGER NOT NULL,
                width        INTEGER NOT NULL,
                value        INTEGER NOT NULL,
                enabled      INTEGER NOT NULL DEFAULT 1,
                FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_cheats_game ON cheats(game_id);
            "#,
        )
        .map_err(|e| format!("create cheats table: {e}"))
    }

    /// Retag tg16 games with CD-image extensions as `pce-cd`. Idempotent
    /// (re-running on already-split data is a no-op). Looks at both the
    /// outer `file_path` (uncompressed scans) and `archive_inner_path`
    /// (ROMs that live inside a zip/7z — the inner extension is what the
    /// launch path actually keys off).
    fn migrate_v4_to_v5(conn: &Connection) -> Result<(), String> {
        // GLOB is case-sensitive in SQLite by default — match both .CUE and
        // .cue by lowercasing the path inside the predicate. The extension
        // list is the literal mirror of the frontend's `pce-cd` registry
        // entry; keep them in sync if either side adds a container.
        const CD_GLOBS: &[&str] = &[
            "*.cue", "*.chd", "*.ccd", "*.toc", "*.m3u", "*.iso",
        ];
        let mut total: usize = 0;
        for pat in CD_GLOBS {
            let n = conn
                .execute(
                    "UPDATE games
                       SET system_id = 'pce-cd'
                     WHERE system_id = 'tg16'
                       AND (lower(file_path) GLOB ?1
                            OR (archive_inner_path IS NOT NULL
                                AND lower(archive_inner_path) GLOB ?1))",
                    rusqlite::params![pat],
                )
                .map_err(|e| format!("retag tg16→pce-cd ({pat}): {e}"))?;
            total += n;
        }
        if total > 0 {
            log::info!("library_db: v4→v5 retagged {total} CD game(s) tg16 → pce-cd");
        }
        Ok(())
    }

    fn migrate_v3_to_v4(conn: &Connection) -> Result<(), String> {
        // Slice 4.F — per-game memory-watching milestones. Each row is
        // ONE condition; on rising-edge (predicate false → true), the
        // emu thread emits an event AND we stamp `triggered_at_unix_ms`
        // so the UI knows it's been unlocked. Reset zeroes that out.
        //
        // The `region` field is a string tag ("system_ram" etc.) — same
        // shape as `MemoryRegionId::as_str()` in oa-core. We keep it as
        // a string so a future region (e.g. expansion-cart RAM) doesn't
        // need a schema migration. `op` likewise.
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS milestones (
                id                    INTEGER PRIMARY KEY AUTOINCREMENT,
                game_id               TEXT NOT NULL,
                name                  TEXT NOT NULL,
                description           TEXT NOT NULL DEFAULT '',
                region                TEXT NOT NULL,
                offset                INTEGER NOT NULL,
                width                 INTEGER NOT NULL,
                op                    TEXT NOT NULL,
                target                INTEGER NOT NULL,
                edge_only             INTEGER NOT NULL DEFAULT 1,
                triggered_at_unix_ms  INTEGER,
                FOREIGN KEY(game_id) REFERENCES games(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_milestones_game ON milestones(game_id);
            "#,
        )
        .map_err(|e| format!("create milestones table: {e}"))
    }

    fn create_v1(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS games (
                id                  TEXT PRIMARY KEY,
                system_id           TEXT NOT NULL,
                file_path           TEXT NOT NULL UNIQUE,
                title               TEXT NOT NULL,
                normalized_title    TEXT NOT NULL,
                added_at            INTEGER NOT NULL,
                core_override       TEXT,
                cover_path          TEXT,
                year                INTEGER,
                genre               TEXT,
                developer           TEXT,
                publisher           TEXT,
                players             INTEGER,
                rating              REAL,
                play_time_secs      INTEGER NOT NULL DEFAULT 0,
                last_played_at      INTEGER,
                region              TEXT,
                favorite            INTEGER NOT NULL DEFAULT 0,
                completed           INTEGER NOT NULL DEFAULT 0,
                custom_fields_json  TEXT,
                seed                INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_games_system ON games(system_id);
            CREATE INDEX IF NOT EXISTS idx_games_added ON games(added_at);
            CREATE INDEX IF NOT EXISTS idx_games_last_played
                ON games(last_played_at) WHERE last_played_at IS NOT NULL;

            CREATE VIRTUAL TABLE IF NOT EXISTS games_fts USING fts5(
                title, normalized_title, developer, publisher,
                content='games',
                content_rowid='rowid',
                tokenize='unicode61 remove_diacritics 2'
            );
            CREATE TRIGGER IF NOT EXISTS games_ai AFTER INSERT ON games BEGIN
                INSERT INTO games_fts(rowid, title, normalized_title, developer, publisher)
                VALUES (new.rowid, new.title, new.normalized_title, new.developer, new.publisher);
            END;
            CREATE TRIGGER IF NOT EXISTS games_ad AFTER DELETE ON games BEGIN
                INSERT INTO games_fts(games_fts, rowid, title, normalized_title, developer, publisher)
                VALUES('delete', old.rowid, old.title, old.normalized_title, old.developer, old.publisher);
            END;
            CREATE TRIGGER IF NOT EXISTS games_au AFTER UPDATE ON games BEGIN
                INSERT INTO games_fts(games_fts, rowid, title, normalized_title, developer, publisher)
                VALUES('delete', old.rowid, old.title, old.normalized_title, old.developer, old.publisher);
                INSERT INTO games_fts(rowid, title, normalized_title, developer, publisher)
                VALUES (new.rowid, new.title, new.normalized_title, new.developer, new.publisher);
            END;

            CREATE TABLE IF NOT EXISTS folders (
                id                      TEXT PRIMARY KEY,
                path                    TEXT NOT NULL UNIQUE,
                scan_subfolders         INTEGER NOT NULL DEFAULT 1,
                subfolders_are_systems  INTEGER NOT NULL DEFAULT 0,
                watch_enabled           INTEGER NOT NULL DEFAULT 0,
                last_scanned_at         INTEGER
            );
            "#,
        )
        .map_err(|e| format!("create v1 schema: {e}"))
    }

    fn migrate_v2_to_v3(conn: &Connection) -> Result<(), String> {
        // Slice 2.8.D — per-game overrides surface. JSON column rather than
        // typed columns because the override set will grow (scaling, window,
        // monitor, region, shader preset, …) and a JSON bag stays migration-
        // free for new fields. PRAGMA table_info guard so re-running the
        // migration after a mid-flight failure doesn't error.
        let has_column: bool = conn
            .prepare("PRAGMA table_info(games)")
            .and_then(|mut stmt| {
                let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
                let mut found = false;
                for r in rows {
                    if r? == "overrides_json" {
                        found = true;
                        break;
                    }
                }
                Ok(found)
            })
            .unwrap_or(false);
        if !has_column {
            conn.execute("ALTER TABLE games ADD COLUMN overrides_json TEXT", [])
                .map_err(|e| format!("alter games add overrides_json: {e}"))?;
        }
        Ok(())
    }

    fn migrate_v1_to_v2(conn: &Connection) -> Result<(), String> {
        // SQLite ADD COLUMN is in-place + cheap. Defaulting to NULL means
        // every existing row reads as "not an archive" without rewriting.
        // PRAGMA table_info check first so re-running the migration after a
        // mid-flight failure doesn't error on "column already exists."
        let has_column: bool = conn
            .prepare("PRAGMA table_info(games)")
            .and_then(|mut stmt| {
                let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
                let mut found = false;
                for r in rows {
                    if r? == "archive_inner_path" {
                        found = true;
                        break;
                    }
                }
                Ok(found)
            })
            .unwrap_or(false);
        if !has_column {
            conn.execute("ALTER TABLE games ADD COLUMN archive_inner_path TEXT", [])
                .map_err(|e| format!("alter games add archive_inner_path: {e}"))?;
        }
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS folder_rules (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                folder_id     TEXT NOT NULL,
                match_pattern TEXT NOT NULL,
                system_id     TEXT NOT NULL,
                FOREIGN KEY(folder_id) REFERENCES folders(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_folder_rules_folder ON folder_rules(folder_id);
            "#,
        )
        .map_err(|e| format!("create folder_rules: {e}"))
    }

    /// Normalize a title for fuzzy matching + FTS searchability. Same shape as
    /// the existing `normalize::normalize_title` used by the cover sync — keep
    /// these aligned so search results and cover matching surface the same
    /// "this is the same game" decisions.
    fn normalize_title(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let lower = s.to_lowercase();
        let mut prev_was_space = true;
        for ch in lower.chars() {
            if ch.is_alphanumeric() {
                out.push(ch);
                prev_was_space = false;
            } else if !prev_was_space {
                out.push(' ');
                prev_was_space = true;
            }
        }
        out.trim().to_string()
    }

    pub fn list_games(&self) -> Result<Vec<GameRow>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, system_id, file_path, title, added_at,
                        core_override, cover_path, seed, archive_inner_path,
                        sha1, serial, disc_id,
                        favorite, completed, last_played_at, play_time_secs,
                        players, rating, disc_set_id, disc_number
                 FROM games
                 ORDER BY title COLLATE NOCASE",
            )
            .map_err(|e| format!("prepare list_games: {e}"))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(GameRow {
                    id: row.get(0)?,
                    system_id: row.get(1)?,
                    file_path: row.get(2)?,
                    title: row.get(3)?,
                    added_at: row.get(4)?,
                    core_override: row.get(5)?,
                    cover_path: row.get(6)?,
                    seed: row.get::<_, i64>(7)? != 0,
                    archive_inner_path: row.get(8)?,
                    sha1: row.get(9)?,
                    serial: row.get(10)?,
                    disc_id: row.get(11)?,
                    favorite: row.get::<_, i64>(12)? != 0,
                    completed: row.get::<_, i64>(13)? != 0,
                    last_played_at: row.get(14)?,
                    play_time_secs: row.get(15)?,
                    players: row.get(16)?,
                    rating: row.get(17)?,
                    disc_set_id: row.get(18)?,
                    disc_number: row.get::<_, Option<i64>>(19)?.map(|n| n as u32),
                })
            })
            .map_err(|e| format!("query list_games: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("collect list_games: {e}"))?;
        Ok(rows)
    }

    /// Bulk-insert. Returns the number of newly-added rows (entries that
    /// collide on file_path are skipped). Existing seed rows are NOT removed
    /// here — call `drop_seed_rows` separately when a real ingest commits.
    pub fn add_games(&self, entries: &[GameRow]) -> Result<usize, String> {
        let mut conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let tx = conn.transaction().map_err(|e| format!("begin tx: {e}"))?;
        let mut added = 0usize;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT OR IGNORE INTO games
                     (id, system_id, file_path, title, normalized_title, added_at,
                      core_override, cover_path, seed, archive_inner_path)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                )
                .map_err(|e| format!("prepare insert: {e}"))?;
            for g in entries {
                let inserted = stmt
                    .execute(params![
                        g.id,
                        g.system_id,
                        g.file_path,
                        g.title,
                        Self::normalize_title(&g.title),
                        g.added_at,
                        g.core_override,
                        g.cover_path,
                        if g.seed { 1i64 } else { 0i64 },
                        g.archive_inner_path,
                    ])
                    .map_err(|e| format!("insert game {}: {e}", g.id))?;
                added += inserted;
            }
        }
        tx.commit().map_err(|e| format!("commit add_games: {e}"))?;
        Ok(added)
    }

    /// Return every game tagged with `system_id` as a `GameRow`.
    /// Mirrors `list_games` but filtered to one system. Used by the
    /// art-pack importer to scope fuzzy matching — Genesis-folder art
    /// only matches Genesis library entries, never accidentally lands
    /// on a Game Boy title with a similar name.
    pub fn list_games_for_system(&self, system_id: &str) -> Result<Vec<GameRow>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, system_id, file_path, title, added_at,
                        core_override, cover_path, seed, archive_inner_path,
                        sha1, serial, disc_id
                 FROM games
                 WHERE system_id = ?1
                 ORDER BY title",
            )
            .map_err(|e| format!("prepare list_games_for_system: {e}"))?;
        let rows = stmt
            .query_map(params![system_id], |row| {
                Ok(GameRow {
                    id: row.get(0)?,
                    system_id: row.get(1)?,
                    file_path: row.get(2)?,
                    title: row.get(3)?,
                    added_at: row.get(4)?,
                    core_override: row.get(5)?,
                    cover_path: row.get(6)?,
                    seed: row.get::<_, i64>(7)? != 0,
                    archive_inner_path: row.get(8)?,
                    sha1: row.get(9)?,
                    serial: row.get(10)?,
                    disc_id: row.get(11)?,
                    // Phase C3 — these aren't SELECTed by this query; consumer
                    // doesn't need them. list_games is the canonical source for
                    // smart-list data in the COLLECTIONS tab.
                    favorite: false,
                    completed: false,
                    last_played_at: None,
                    play_time_secs: 0,
                    players: None,
                    rating: None,
                    disc_set_id: None,
                    disc_number: None,
                })
            })
            .map_err(|e| format!("query list_games_for_system: {e}"))?;
        let mut out: Vec<GameRow> = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| format!("step list_games_for_system: {e}"))?);
        }
        Ok(out)
    }

    /// Remove seed rows. Called when the first real ingest commits so the
    /// six placeholder TG-16 tiles don't co-exist with real data.
    pub fn drop_seed_rows(&self) -> Result<usize, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let affected = conn
            .execute("DELETE FROM games WHERE seed = 1", [])
            .map_err(|e| format!("delete seeds: {e}"))?;
        Ok(affected)
    }

    pub fn update_core_override(&self, id: &str, value: Option<&str>) -> Result<(), String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        conn.execute(
            "UPDATE games SET core_override = ?1 WHERE id = ?2",
            params![value, id],
        )
        .map_err(|e| format!("update core_override: {e}"))?;
        Ok(())
    }

    /// Retroverse-UI Phase C3 — flip the favorite flag for a single
    /// game. Drives the Favorites smart-list in the COLLECTIONS tab.
    /// Idempotent: writing the same value twice is harmless.
    pub fn update_favorite(&self, id: &str, value: bool) -> Result<(), String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        conn.execute(
            "UPDATE games SET favorite = ?1 WHERE id = ?2",
            params![value as i64, id],
        )
        .map_err(|e| format!("update favorite: {e}"))?;
        Ok(())
    }

    /// Retroverse-UI Phase C3 — flip the completed flag for a single
    /// game. Drives the Completed smart-list in the COLLECTIONS tab.
    /// Idempotent: writing the same value twice is harmless.
    pub fn update_completed(&self, id: &str, value: bool) -> Result<(), String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        conn.execute(
            "UPDATE games SET completed = ?1 WHERE id = ?2",
            params![value as i64, id],
        )
        .map_err(|e| format!("update completed: {e}"))?;
        Ok(())
    }

    /// Bulk-insert hash entries into rom_hashes. Called by the metadata-sync
    /// flow after parsing dat/<system>.dat from libretro-database.
    /// INSERT OR REPLACE so re-syncing with newer upstream data overwrites
    /// the stored game_name + serial in place (filename was already the
    /// key on the upstream side).
    #[allow(dead_code)] // merge-semantics counterpart to replace_rom_hashes_for_system; kept for any future caller that needs append-only behaviour
    pub fn upsert_rom_hashes(&self, entries: &[RomHashRow]) -> Result<usize, String> {
        let mut conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let tx = conn.transaction().map_err(|e| format!("begin tx: {e}"))?;
        let mut written = 0usize;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT OR REPLACE INTO rom_hashes
                       (sha1, system_id, game_name, serial, crc32, size_bytes)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                )
                .map_err(|e| format!("prepare upsert rom_hashes: {e}"))?;
            for r in entries {
                stmt.execute(params![
                    r.sha1.to_ascii_lowercase(),
                    r.system_id,
                    r.game_name,
                    r.serial,
                    r.crc32.as_ref().map(|s| s.to_ascii_lowercase()),
                    r.size_bytes,
                ])
                .map_err(|e| format!("insert rom_hash {}: {e}", r.sha1))?;
                written += 1;
            }
        }
        tx.commit().map_err(|e| format!("commit upsert_rom_hashes: {e}"))?;
        Ok(written)
    }

    /// Look up a single sha1 in the rom_hashes table. Sha1 is matched
    /// case-insensitively (we lowercase on read AND on insert).
    pub fn lookup_rom_hash(&self, sha1: &str) -> Result<Option<RomHashRow>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT sha1, system_id, game_name, serial, crc32, size_bytes
                 FROM rom_hashes WHERE sha1 = ?1",
            )
            .map_err(|e| format!("prepare lookup_rom_hash: {e}"))?;
        let mut rows = stmt
            .query(params![sha1.to_ascii_lowercase()])
            .map_err(|e| format!("query lookup_rom_hash: {e}"))?;
        if let Some(row) = rows.next().map_err(|e| format!("step lookup_rom_hash: {e}"))? {
            Ok(Some(RomHashRow {
                sha1: row.get(0).map_err(|e| format!("col sha1: {e}"))?,
                system_id: row.get(1).map_err(|e| format!("col system_id: {e}"))?,
                game_name: row.get(2).map_err(|e| format!("col game_name: {e}"))?,
                serial: row.get(3).map_err(|e| format!("col serial: {e}"))?,
                crc32: row.get(4).map_err(|e| format!("col crc32: {e}"))?,
                size_bytes: row.get(5).map_err(|e| format!("col size_bytes: {e}"))?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Batched variant of [`lookup_rom_hash`] for the resolve loop's
    /// per-game candidate set. Replaces the per-candidate N+1 pattern
    /// `for c in candidates { db.lookup_rom_hash(&c.sha1)? }` with a
    /// single `WHERE sha1 IN (?,?,?,...)` query whose result is keyed
    /// on the lowercased sha1 for O(1) per-candidate lookup.
    ///
    /// Empty `sha1s` returns an empty map. The lookup is
    /// case-insensitive: input sha1s are lowercased before the query
    /// AND the keys in the returned map are lowercase. Callers can
    /// either lowercase their probe key OR call `.to_ascii_lowercase()`
    /// on it before `get()`.
    ///
    /// SQLite imposes a default variable limit (typically 999 in older
    /// builds, 32766 in modern). For OA's resolve loop this caps at the
    /// header-rule expansion (3–5 candidates per game) which is well
    /// under the limit; if a future caller wants to batch hundreds of
    /// sha1s at once they should chunk into batches of 500.
    pub fn lookup_rom_hashes_batch(
        &self,
        sha1s: &[String],
    ) -> Result<std::collections::HashMap<String, RomHashRow>, String> {
        let mut out: std::collections::HashMap<String, RomHashRow> =
            std::collections::HashMap::with_capacity(sha1s.len());
        if sha1s.is_empty() {
            return Ok(out);
        }
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let lowered: Vec<String> = sha1s.iter().map(|s| s.to_ascii_lowercase()).collect();
        let placeholders = vec!["?"; lowered.len()].join(",");
        let sql = format!(
            "SELECT sha1, system_id, game_name, serial, crc32, size_bytes
             FROM rom_hashes WHERE sha1 IN ({placeholders})"
        );
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("prepare lookup_rom_hashes_batch: {e}"))?;
        let params_iter = rusqlite::params_from_iter(lowered.iter());
        let mut rows = stmt
            .query(params_iter)
            .map_err(|e| format!("query lookup_rom_hashes_batch: {e}"))?;
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("step lookup_rom_hashes_batch: {e}"))?
        {
            let r = RomHashRow {
                sha1: row.get(0).map_err(|e| format!("col sha1: {e}"))?,
                system_id: row.get(1).map_err(|e| format!("col system_id: {e}"))?,
                game_name: row.get(2).map_err(|e| format!("col game_name: {e}"))?,
                serial: row.get(3).map_err(|e| format!("col serial: {e}"))?,
                crc32: row.get(4).map_err(|e| format!("col crc32: {e}"))?,
                size_bytes: row.get(5).map_err(|e| format!("col size_bytes: {e}"))?,
            };
            out.insert(r.sha1.clone(), r);
        }
        Ok(out)
    }

    /// Find a library row by SHA-1 hash. Used by direct-launch to discover
    /// an existing library entry (and its per-game overrides) when the
    /// user spawns oa-shell with a ROM path from an external frontend.
    /// Uses the `idx_games_sha1` index. Matches case-insensitively — sha1
    /// is normalized to lowercase on both sides.
    /// Look up a single game by its row id (the djb2 path-hash key used
    /// throughout the frontend). Returns Ok(None) when the id isn't in
    /// the table. Used by `media::set_manual_cover` to discover the
    /// rom_stem (file_path basename) for the new LaunchBox-shape art
    /// folder layout introduced 2026-05-23.
    pub fn find_game_by_id(&self, id: &str) -> Result<Option<GameRow>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, system_id, file_path, title, added_at,
                        core_override, cover_path, seed, archive_inner_path,
                        sha1, serial, disc_id
                 FROM games
                 WHERE id = ?1
                 LIMIT 1",
            )
            .map_err(|e| format!("prepare find_game_by_id: {e}"))?;
        let mut rows = stmt
            .query(params![id])
            .map_err(|e| format!("query find_game_by_id: {e}"))?;
        if let Some(row) = rows.next().map_err(|e| format!("step find_game_by_id: {e}"))? {
            Ok(Some(GameRow {
                id: row.get(0).map_err(|e| format!("col id: {e}"))?,
                system_id: row.get(1).map_err(|e| format!("col system_id: {e}"))?,
                file_path: row.get(2).map_err(|e| format!("col file_path: {e}"))?,
                title: row.get(3).map_err(|e| format!("col title: {e}"))?,
                added_at: row.get(4).map_err(|e| format!("col added_at: {e}"))?,
                core_override: row.get(5).map_err(|e| format!("col core_override: {e}"))?,
                cover_path: row.get(6).map_err(|e| format!("col cover_path: {e}"))?,
                seed: row.get::<_, i64>(7).map_err(|e| format!("col seed: {e}"))? != 0,
                archive_inner_path: row.get(8).map_err(|e| format!("col archive_inner_path: {e}"))?,
                sha1: row.get(9).map_err(|e| format!("col sha1: {e}"))?,
                serial: row.get(10).map_err(|e| format!("col serial: {e}"))?,
                disc_id: row.get(11).map_err(|e| format!("col disc_id: {e}"))?,
                // Phase C3 — find_game_by_id doesn't SELECT these columns;
                // callers (media path resolution etc.) don't need them.
                favorite: false,
                completed: false,
                last_played_at: None,
                play_time_secs: 0,
                players: None,
                rating: None,
                disc_set_id: None,
                disc_number: None,
            }))
        } else {
            Ok(None)
        }
    }

    /// Record a finished play session against `id`: increment `play_time_secs`
    /// by `delta_secs` and bump `last_played_at` to `last_played_unix_secs`.
    /// Single UPDATE so both fields stay coherent. Idempotent at the row
    /// level — a missing id is a quiet no-op (operator may have deleted the
    /// row mid-session). Used by the Retroverse-UI Phase A Slice 2
    /// `close_active_session` helper in main.rs.
    pub fn update_play_session(
        &self,
        id: &str,
        delta_secs: u64,
        last_played_unix_secs: i64,
    ) -> Result<(), String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        conn.execute(
            "UPDATE games
                SET play_time_secs = play_time_secs + ?1,
                    last_played_at = ?2
              WHERE id = ?3",
            params![delta_secs as i64, last_played_unix_secs, id],
        )
        .map_err(|e| format!("update_play_session: {e}"))?;
        Ok(())
    }

    pub fn find_game_by_sha1(&self, sha1: &str) -> Result<Option<GameRow>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, system_id, file_path, title, added_at,
                        core_override, cover_path, seed, archive_inner_path,
                        sha1, serial, disc_id
                 FROM games
                 WHERE sha1 = ?1
                 LIMIT 1",
            )
            .map_err(|e| format!("prepare find_game_by_sha1: {e}"))?;
        let mut rows = stmt
            .query(params![sha1.to_ascii_lowercase()])
            .map_err(|e| format!("query find_game_by_sha1: {e}"))?;
        if let Some(row) = rows.next().map_err(|e| format!("step find_game_by_sha1: {e}"))? {
            Ok(Some(GameRow {
                id: row.get(0).map_err(|e| format!("col id: {e}"))?,
                system_id: row.get(1).map_err(|e| format!("col system_id: {e}"))?,
                file_path: row.get(2).map_err(|e| format!("col file_path: {e}"))?,
                title: row.get(3).map_err(|e| format!("col title: {e}"))?,
                added_at: row.get(4).map_err(|e| format!("col added_at: {e}"))?,
                core_override: row.get(5).map_err(|e| format!("col core_override: {e}"))?,
                cover_path: row.get(6).map_err(|e| format!("col cover_path: {e}"))?,
                seed: row.get::<_, i64>(7).map_err(|e| format!("col seed: {e}"))? != 0,
                archive_inner_path: row.get(8).map_err(|e| format!("col archive_inner_path: {e}"))?,
                sha1: row.get(9).map_err(|e| format!("col sha1: {e}"))?,
                serial: row.get(10).map_err(|e| format!("col serial: {e}"))?,
                disc_id: row.get(11).map_err(|e| format!("col disc_id: {e}"))?,
                // Phase C3 — find_game_by_sha1 doesn't SELECT these; same
                // pattern as find_game_by_id.
                favorite: false,
                completed: false,
                last_played_at: None,
                play_time_secs: 0,
                players: None,
                rating: None,
                disc_set_id: None,
                disc_number: None,
            }))
        } else {
            Ok(None)
        }
    }

    /// Bulk-hydrate sha1 + canonical-no-intro title for every game tagged
    /// with `system_id`. Returns a `HashMap<id, (sha1, Option<canonical_title>)>`
    /// covering only entries whose `games.sha1` is non-null and whose
    /// sha1 matches a `rom_hashes` row for the same system.
    ///
    /// Single prepared LEFT JOIN — runs in one lock acquisition, one
    /// SQL execution, regardless of how many entries the caller has.
    /// Pre-2026-05-21 the media + metadata sync paths fell back on
    /// `find_sha1_by_id` + `lookup_rom_hash` per entry, which on a
    /// 1160-entry sync did ~11,400 sequential lock cycles before the
    /// network walk could begin. This helper collapses that to one
    /// query.
    ///
    /// Skips rows whose sha1 is empty/null (entries that resolve_rom_hashes
    /// hasn't stamped yet — caller will get those after the next resolve).
    pub fn hydrate_sha1_and_canonical_for_system(
        &self,
        system_id: &str,
    ) -> Result<HashMap<String, (String, Option<String>)>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT g.id, g.sha1, h.game_name
                 FROM games g
                 LEFT JOIN rom_hashes h
                   ON h.sha1 = g.sha1 AND h.system_id = g.system_id
                 WHERE g.system_id = ?1
                   AND g.sha1 IS NOT NULL
                   AND g.sha1 <> ''",
            )
            .map_err(|e| format!("prepare hydrate_sha1_and_canonical_for_system: {e}"))?;
        let rows = stmt
            .query_map(params![system_id], |row| {
                let id: String = row.get(0)?;
                let sha1: String = row.get(1)?;
                let canonical: Option<String> = row.get(2)?;
                Ok((id, sha1, canonical))
            })
            .map_err(|e| format!("query hydrate_sha1_and_canonical_for_system: {e}"))?;
        let mut out: HashMap<String, (String, Option<String>)> = HashMap::new();
        for row in rows {
            let (id, sha1, canonical) = row
                .map_err(|e| format!("step hydrate_sha1_and_canonical_for_system: {e}"))?;
            out.insert(id, (sha1, canonical));
        }
        Ok(out)
    }

    /// Return every game in the given system that doesn't have a sha1 yet
    /// and isn't a multi-file CD image. Caller hashes them and calls
    /// `apply_rom_hash` per-row.
    pub fn list_games_missing_hash(
        &self,
        system_id: &str,
        include_disc_id_stamped: bool,
    ) -> Result<Vec<GameRow>, String> {
        // Phase A1 Sub-phase 3 fix — disc-shape callers pass true so
        // games previously stamped with disc_id by the old peek_disc_id
        // flow (Sub-phase 0/1 — see `apply_disc_id` callers in
        // rom_hashes.rs) get a per-track retry. The per-track path is
        // strictly more accurate than serial-lookup, so re-running it
        // on disc-id-stamped games is the correct semantic. Cart-shape
        // callers pass false to preserve the pre-fix exclusion of
        // disc-id-stamped rows.
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let query = if include_disc_id_stamped {
            "SELECT id, system_id, file_path, title, added_at,
                    core_override, cover_path, seed, archive_inner_path,
                    sha1, serial, disc_id
             FROM games
             WHERE system_id = ?1 AND (sha1 IS NULL OR sha1 = '')"
        } else {
            "SELECT id, system_id, file_path, title, added_at,
                    core_override, cover_path, seed, archive_inner_path,
                    sha1, serial, disc_id
             FROM games
             WHERE system_id = ?1 AND (sha1 IS NULL OR sha1 = '')
               AND (disc_id IS NULL OR disc_id = '')"
        };
        let mut stmt = conn
            .prepare(query)
            .map_err(|e| format!("prepare list_games_missing_hash: {e}"))?;
        let rows = stmt
            .query_map(params![system_id], |row| {
                Ok(GameRow {
                    id: row.get(0)?,
                    system_id: row.get(1)?,
                    file_path: row.get(2)?,
                    title: row.get(3)?,
                    added_at: row.get(4)?,
                    core_override: row.get(5)?,
                    cover_path: row.get(6)?,
                    seed: row.get::<_, i64>(7)? != 0,
                    archive_inner_path: row.get(8)?,
                    sha1: row.get(9)?,
                    serial: row.get(10)?,
                    disc_id: row.get(11)?,
                    // Phase C3 — helper used by the metadata-sync flow;
                    // doesn't need the new fields.
                    favorite: false,
                    completed: false,
                    last_played_at: None,
                    play_time_secs: 0,
                    players: None,
                    rating: None,
                    disc_set_id: None,
                    disc_number: None,
                })
            })
            .map_err(|e| format!("query list_games_missing_hash: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("collect list_games_missing_hash: {e}"))?;
        Ok(rows)
    }

    /// List every TRULY unidentified game in the given system — `sha1` is
    /// NULL/empty. Excludes seed rows (placeholder tiles). Returns the
    /// slim payload the UI needs: id, title, file_path, archive_inner_path,
    /// plus a `has_disc_id` flag so the operator can tell which rows were
    /// stamped by the legacy `peek_disc_id` flow (semi-identified, but
    /// still actionable — re-running Identify ROMs sends them through the
    /// new fuzzy path).
    ///
    /// Cart games: `has_disc_id` is always false.
    /// Disc games: `has_disc_id=true` means a publisher catalog code was
    /// extracted but no fuzzy/per-track match landed; `false` means no
    /// disc-id read either (truly unmatched).
    pub fn list_unidentified_games_for_system(
        &self,
        system_id: &str,
    ) -> Result<Vec<UnidentifiedGameRow>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, system_id, file_path, title, archive_inner_path, disc_id
                 FROM games
                 WHERE system_id = ?1
                   AND (sha1 IS NULL OR sha1 = '')
                   AND seed = 0
                 ORDER BY title ASC",
            )
            .map_err(|e| format!("prepare list_unidentified_games_for_system: {e}"))?;
        let rows = stmt
            .query_map(params![system_id], |row| {
                let disc_id: Option<String> = row.get(5)?;
                Ok(UnidentifiedGameRow {
                    id: row.get(0)?,
                    system_id: row.get(1)?,
                    file_path: row.get(2)?,
                    title: row.get(3)?,
                    archive_inner_path: row.get(4)?,
                    has_disc_id: disc_id.as_deref().is_some_and(|s| !s.is_empty()),
                })
            })
            .map_err(|e| format!("query list_unidentified_games_for_system: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("collect list_unidentified_games_for_system: {e}"))?;
        Ok(rows)
    }

    /// Stamp a game with its sha1 and (optionally) the canonical title +
    /// serial from the matched rom_hashes entry. Pass `title = None` to
    /// only record the sha1; pass `title = Some(canonical)` to overwrite
    /// the stored title with the libretro-database canonical name. The
    /// normalized_title is rebuilt alongside so FTS5 stays consistent.
    pub fn apply_rom_hash(
        &self,
        id: &str,
        sha1: &str,
        canonical_title: Option<&str>,
        serial: Option<&str>,
    ) -> Result<(), String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let lower = sha1.to_ascii_lowercase();
        match canonical_title {
            Some(title) => {
                let normalized = Self::normalize_title(title);
                conn.execute(
                    "UPDATE games
                       SET sha1 = ?1,
                           serial = COALESCE(?2, serial),
                           title = ?3,
                           normalized_title = ?4
                     WHERE id = ?5",
                    params![lower, serial, title, normalized, id],
                )
                .map_err(|e| format!("apply_rom_hash update with title: {e}"))?;
            }
            None => {
                conn.execute(
                    "UPDATE games
                       SET sha1 = ?1,
                           serial = COALESCE(?2, serial)
                     WHERE id = ?3",
                    params![lower, serial, id],
                )
                .map_err(|e| format!("apply_rom_hash update: {e}"))?;
            }
        }
        Ok(())
    }

    /// Replace the entire `rom_hashes` corpus for a given system. DELETE
    /// WHERE system_id = ? then INSERT each row, inside one transaction.
    /// This is the right shape for sync flows where the upstream `.dat`
    /// is the source of truth: entries removed upstream disappear locally
    /// rather than lingering as orphans across resyncs. Use
    /// `upsert_rom_hashes` when you really do want merge semantics.
    ///
    /// Entries whose `system_id` doesn't match the argument are silently
    /// dropped (defensive — the parser stamps every row with the same
    /// system_id, so a mismatch would mean a caller bug we'd want
    /// surfaced via test rather than corrupting another system's table).
    pub fn replace_rom_hashes_for_system(
        &self,
        system_id: &str,
        entries: &[RomHashRow],
    ) -> Result<usize, String> {
        let mut conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let tx = conn.transaction().map_err(|e| format!("begin tx: {e}"))?;
        tx.execute(
            "DELETE FROM rom_hashes WHERE system_id = ?1",
            params![system_id],
        )
        .map_err(|e| format!("delete rom_hashes for {system_id}: {e}"))?;
        let mut written = 0usize;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT OR REPLACE INTO rom_hashes
                       (sha1, system_id, game_name, serial, crc32, size_bytes)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                )
                .map_err(|e| format!("prepare replace rom_hashes: {e}"))?;
            for r in entries {
                if r.system_id != system_id {
                    continue;
                }
                stmt.execute(params![
                    r.sha1.to_ascii_lowercase(),
                    r.system_id,
                    r.game_name,
                    r.serial,
                    r.crc32.as_ref().map(|s| s.to_ascii_lowercase()),
                    r.size_bytes,
                ])
                .map_err(|e| format!("insert rom_hash {}: {e}", r.sha1))?;
                written += 1;
            }
        }
        tx.commit().map_err(|e| format!("commit replace_rom_hashes: {e}"))?;
        Ok(written)
    }

    /// Bulk-replace every entry in `mame_titles`. Used by
    /// `sync_mame_titles` to refresh the full corpus on each pull from
    /// libretro-database. Single transaction so partial fetches don't
    /// leave a half-populated table.
    pub fn replace_mame_titles(&self, entries: &[MameTitleRow]) -> Result<usize, String> {
        let mut conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let tx = conn.transaction().map_err(|e| format!("begin tx: {e}"))?;
        tx.execute("DELETE FROM mame_titles", [])
            .map_err(|e| format!("clear mame_titles: {e}"))?;
        let mut written = 0usize;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT OR REPLACE INTO mame_titles
                       (rom_set, title, year, developer)
                     VALUES (?1, ?2, ?3, ?4)",
                )
                .map_err(|e| format!("prepare insert mame_titles: {e}"))?;
            for r in entries {
                stmt.execute(params![
                    r.rom_set.to_ascii_lowercase(),
                    r.title,
                    r.year,
                    r.developer,
                ])
                .map_err(|e| format!("insert mame_title {}: {e}", r.rom_set))?;
                written += 1;
            }
        }
        tx.commit().map_err(|e| format!("commit replace_mame_titles: {e}"))?;
        Ok(written)
    }

    /// Look up a single MAME ROM-set entry by `.zip` basename. Returns
    /// `Ok(None)` when the rom_set isn't in the catalog (e.g. a homebrew
    /// or hack the operator has that isn't in libretro-database yet).
    pub fn lookup_mame_title(&self, rom_set: &str) -> Result<Option<MameTitleRow>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let key = rom_set.to_ascii_lowercase();
        conn.query_row(
            "SELECT rom_set, title, year, developer FROM mame_titles WHERE rom_set = ?1",
            params![key],
            |row| {
                Ok(MameTitleRow {
                    rom_set: row.get(0)?,
                    title: row.get(1)?,
                    year: row.get(2)?,
                    developer: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(|e| format!("lookup_mame_title {rom_set}: {e}"))
    }

    /// Stamp a CD game with its disc_id and (optionally) the canonical
    /// title from the matched game_serials entry. Parallel to
    /// `apply_rom_hash` for cart games. Pass `title = None` to record
    /// only the disc_id (so re-scans skip re-peeking) without
    /// overwriting the user's title — the "scanned but unmatched" case.
    pub fn apply_disc_id(
        &self,
        id: &str,
        disc_id: &str,
        canonical_title: Option<&str>,
    ) -> Result<(), String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        match canonical_title {
            Some(title) => {
                let normalized = Self::normalize_title(title);
                conn.execute(
                    "UPDATE games
                       SET disc_id = ?1,
                           title = ?2,
                           normalized_title = ?3
                     WHERE id = ?4",
                    params![disc_id, title, normalized, id],
                )
                .map_err(|e| format!("apply_disc_id with title: {e}"))?;
            }
            None => {
                conn.execute(
                    "UPDATE games SET disc_id = ?1 WHERE id = ?2",
                    params![disc_id, id],
                )
                .map_err(|e| format!("apply_disc_id: {e}"))?;
            }
        }
        Ok(())
    }

    /// Diagnostic — how many rom_hashes rows we hold for a given system.
    /// Exercised by the v8 migration test; the running UI doesn't surface
    /// it yet (a future "library health" view would, e.g. "tg16: 705
    /// canonical entries indexed").
    #[allow(dead_code)]
    pub fn count_rom_hashes(&self, system_id: &str) -> Result<i64, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        conn.query_row(
            "SELECT COUNT(*) FROM rom_hashes WHERE system_id = ?1",
            params![system_id],
            |r| r.get(0),
        )
        .map_err(|e| format!("count rom_hashes: {e}"))
    }

    // ---- Disc-shape per-track hashes (Phase A1 of the virtual library
    // + launcher arc). Parallel surface to the cart-shape rom_hashes
    // functions above. ---------------------------------------------------

    /// Look up a single SHA-1 in the disc-shape per-track table.
    /// Returns the matched canonical entry (if any). Mirrors
    /// `lookup_rom_hash`'s shape — sha1 is matched case-insensitively
    /// (lowercased on read AND on insert). Called from Sub-phase 3's
    /// resolve flow once per-track hashing lands.
    #[allow(dead_code)]
    pub fn lookup_rom_hash_track(
        &self,
        sha1: &str,
    ) -> Result<Option<RomTrackRow>, String> {
        let conn = self
            .inner
            .lock()
            .map_err(|_| "library_db: lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT sha1, system_id, game_name, serial, track_number, track_mode, size_bytes
                 FROM rom_hashes_tracks WHERE sha1 = ?1",
            )
            .map_err(|e| format!("prepare lookup_rom_hash_track: {e}"))?;
        let mut rows = stmt
            .query(params![sha1.to_ascii_lowercase()])
            .map_err(|e| format!("query lookup_rom_hash_track: {e}"))?;
        if let Some(row) = rows
            .next()
            .map_err(|e| format!("step lookup_rom_hash_track: {e}"))?
        {
            Ok(Some(RomTrackRow {
                sha1: row.get(0).map_err(|e| format!("col sha1: {e}"))?,
                system_id: row.get(1).map_err(|e| format!("col system_id: {e}"))?,
                game_name: row.get(2).map_err(|e| format!("col game_name: {e}"))?,
                serial: row.get(3).map_err(|e| format!("col serial: {e}"))?,
                track_number: row
                    .get::<_, i64>(4)
                    .map_err(|e| format!("col track_number: {e}"))? as u32,
                track_mode: row.get(5).map_err(|e| format!("col track_mode: {e}"))?,
                size_bytes: row.get(6).map_err(|e| format!("col size_bytes: {e}"))?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Bulk-replace every per-track row for a system. Wipe-and-replace
    /// per system_id (DELETE then INSERT inside one transaction); the
    /// upstream redump dat is the source of truth, so entries removed
    /// upstream disappear locally rather than lingering as orphans.
    ///
    /// Mirrors `replace_rom_hashes_for_system`'s shape — entries whose
    /// `system_id` doesn't match the argument are silently dropped
    /// (defensive against caller bugs).
    pub fn replace_rom_hashes_tracks_for_system(
        &self,
        system_id: &str,
        entries: &[RomTrackRow],
    ) -> Result<usize, String> {
        let mut conn = self
            .inner
            .lock()
            .map_err(|_| "library_db: lock poisoned".to_string())?;
        let tx = conn.transaction().map_err(|e| format!("begin tx: {e}"))?;
        tx.execute(
            "DELETE FROM rom_hashes_tracks WHERE system_id = ?1",
            params![system_id],
        )
        .map_err(|e| format!("delete rom_hashes_tracks for {system_id}: {e}"))?;
        let mut written = 0usize;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT OR REPLACE INTO rom_hashes_tracks
                       (sha1, system_id, game_name, serial, track_number, track_mode, size_bytes)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                )
                .map_err(|e| format!("prepare replace rom_hashes_tracks: {e}"))?;
            for r in entries {
                if r.system_id != system_id {
                    continue;
                }
                stmt.execute(params![
                    r.sha1.to_ascii_lowercase(),
                    r.system_id,
                    r.game_name,
                    r.serial,
                    r.track_number as i64,
                    r.track_mode,
                    r.size_bytes,
                ])
                .map_err(|e| {
                    format!(
                        "insert rom_hashes_track sha1={} track={}: {e}",
                        r.sha1, r.track_number
                    )
                })?;
                written += 1;
            }
        }
        tx.commit()
            .map_err(|e| format!("commit replace_rom_hashes_tracks: {e}"))?;
        Ok(written)
    }

    /// Diagnostic — how many per-track canonical rows we hold for a
    /// given system. Parallel to `count_rom_hashes` for cart-shape
    /// systems. Used by the disc-shape sync flow to surface "psx: 13,526
    /// canonical track entries indexed" style telemetry.
    #[allow(dead_code)]
    pub fn count_rom_hashes_tracks(&self, system_id: &str) -> Result<i64, String> {
        let conn = self
            .inner
            .lock()
            .map_err(|_| "library_db: lock poisoned".to_string())?;
        conn.query_row(
            "SELECT COUNT(*) FROM rom_hashes_tracks WHERE system_id = ?1",
            params![system_id],
            |r| r.get(0),
        )
        .map_err(|e| format!("count rom_hashes_tracks: {e}"))
    }

    /// UPSERT disc-set rows for a system from the latest sync. Uses
    /// `INSERT ... ON CONFLICT(system_id, canonical_title) DO UPDATE`
    /// so the autoincrement `id` stays stable across re-syncs (a
    /// dropped row would orphan any games-table `disc_set_id` FK).
    /// `disc_count` updates in place when upstream adds or removes a
    /// disc from the set.
    ///
    /// Returns the count of rows written (inserted + updated).
    pub fn upsert_disc_sets_for_system(
        &self,
        system_id: &str,
        entries: &[(String, u32)], // (canonical_title, disc_count)
    ) -> Result<usize, String> {
        let mut conn = self
            .inner
            .lock()
            .map_err(|_| "library_db: lock poisoned".to_string())?;
        let tx = conn.transaction().map_err(|e| format!("begin tx: {e}"))?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let mut written = 0usize;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO disc_sets
                       (canonical_title, system_id, disc_count, created_at)
                     VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT(system_id, canonical_title) DO UPDATE
                       SET disc_count = excluded.disc_count",
                )
                .map_err(|e| format!("prepare upsert disc_sets: {e}"))?;
            for (canonical_title, disc_count) in entries {
                stmt.execute(params![
                    canonical_title,
                    system_id,
                    *disc_count as i64,
                    now,
                ])
                .map_err(|e| {
                    format!("upsert disc_set {canonical_title}: {e}")
                })?;
                written += 1;
            }
        }
        tx.commit()
            .map_err(|e| format!("commit upsert_disc_sets: {e}"))?;
        Ok(written)
    }

    /// Look up the auto-incremented `disc_sets.id` for a
    /// (system_id, canonical_title) pair. Used by the
    /// `maybe_stamp_disc_set_membership` helper at identify time:
    /// after the fuzzy / per-track path stamps a multi-disc canonical
    /// game name like "Final Fantasy IX (USA) (Disc 1)", strip the
    /// `(Disc N)` suffix to get "Final Fantasy IX (USA)", call this,
    /// and on hit stamp `games.disc_set_id` + `games.disc_number`.
    ///
    /// Returns `Ok(None)` when no disc-set row exists for that base
    /// title — e.g. the redump parser didn't detect a multi-disc
    /// parent group (most single-disc games), or the operator's
    /// rom_hashes_tracks sync hasn't run.
    pub fn lookup_disc_set_id(
        &self,
        system_id: &str,
        canonical_title: &str,
    ) -> Result<Option<i64>, String> {
        let conn = self
            .inner
            .lock()
            .map_err(|_| "library_db: lock poisoned".to_string())?;
        conn.query_row(
            "SELECT id FROM disc_sets WHERE system_id = ?1 AND canonical_title = ?2",
            params![system_id, canonical_title],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|e| format!("lookup_disc_set_id: {e}"))
    }

    /// Stamp `games.disc_set_id` + `games.disc_number` on a single
    /// game. Called from the resolve flow's disc-set helper after a
    /// successful canonical title match revealed the game belongs to
    /// a multi-disc set. Doesn't validate the disc_set_id is real —
    /// caller's responsibility via [`lookup_disc_set_id`] first.
    pub fn apply_disc_set_membership(
        &self,
        game_id: &str,
        disc_set_id: i64,
        disc_number: u32,
    ) -> Result<(), String> {
        let conn = self
            .inner
            .lock()
            .map_err(|_| "library_db: lock poisoned".to_string())?;
        conn.execute(
            "UPDATE games SET disc_set_id = ?1, disc_number = ?2 WHERE id = ?3",
            params![disc_set_id, disc_number as i64, game_id],
        )
        .map_err(|e| format!("apply_disc_set_membership: {e}"))?;
        Ok(())
    }

    /// List all game rows that belong to a disc-set, sorted by
    /// `disc_number` ascending. Used by the frontend disc-picker
    /// overlay (Sub-phase 4 UI) when the operator clicks a collapsed
    /// set tile.
    pub fn list_disc_set_members(
        &self,
        disc_set_id: i64,
    ) -> Result<Vec<GameRow>, String> {
        let conn = self
            .inner
            .lock()
            .map_err(|_| "library_db: lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, system_id, file_path, title, added_at,
                        core_override, cover_path, seed, archive_inner_path,
                        sha1, serial, disc_id, disc_set_id, disc_number
                 FROM games
                 WHERE disc_set_id = ?1
                 ORDER BY disc_number ASC, title ASC",
            )
            .map_err(|e| format!("prepare list_disc_set_members: {e}"))?;
        let rows = stmt
            .query_map(params![disc_set_id], |row| {
                Ok(GameRow {
                    id: row.get(0)?,
                    system_id: row.get(1)?,
                    file_path: row.get(2)?,
                    title: row.get(3)?,
                    added_at: row.get(4)?,
                    core_override: row.get(5)?,
                    cover_path: row.get(6)?,
                    seed: row.get::<_, i64>(7)? != 0,
                    archive_inner_path: row.get(8)?,
                    sha1: row.get(9)?,
                    serial: row.get(10)?,
                    disc_id: row.get(11)?,
                    disc_set_id: row.get(12)?,
                    // SQLite returns disc_number as i64; downcast to
                    // u32 for the camelCase JSON the frontend reads.
                    disc_number: row.get::<_, Option<i64>>(13)?.map(|n| n as u32),
                    // Sub-phase 4 frontend doesn't need these for the
                    // disc-picker overlay; leave at defaults.
                    favorite: false,
                    completed: false,
                    last_played_at: None,
                    play_time_secs: 0,
                    players: None,
                    rating: None,
                })
            })
            .map_err(|e| format!("query list_disc_set_members: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("collect list_disc_set_members: {e}"))?;
        Ok(rows)
    }

    /// Diagnostic — how many disc-set rows we hold for a system.
    /// Parallel surface to `count_rom_hashes_tracks`.
    #[allow(dead_code)]
    pub fn count_disc_sets(&self, system_id: &str) -> Result<i64, String> {
        let conn = self
            .inner
            .lock()
            .map_err(|_| "library_db: lock poisoned".to_string())?;
        conn.query_row(
            "SELECT COUNT(*) FROM disc_sets WHERE system_id = ?1",
            params![system_id],
            |r| r.get(0),
        )
        .map_err(|e| format!("count disc_sets: {e}"))
    }

    // ---- game_disc_tracks (operator-side per-track hash cache,
    // Phase A1 Sub-phase 3). Stores the per-game per-track SHA-1s
    // the operator's disc image hashed to, plus mtime+size stamps
    // for cache invalidation. -----------------------------------------

    /// Persist a per-game per-track hash cache row. `tracks` is the
    /// output of [`crate::disc_track_hash::hash_disc`] (or pulled from
    /// a cache hit). `file_mtime` / `file_size` stamp the file's
    /// state at hash time so the next scan can stat() the file and
    /// detect "operator replaced the dump" without re-hashing.
    /// Wipes any existing rows for `game_id` then bulk-inserts.
    pub fn write_game_disc_tracks(
        &self,
        game_id: &str,
        tracks: &[crate::disc_track_hash::TrackHash],
        file_mtime: i64,
        file_size: i64,
        last_hashed_at: i64,
    ) -> Result<usize, String> {
        let mut conn = self
            .inner
            .lock()
            .map_err(|_| "library_db: lock poisoned".to_string())?;
        let tx = conn.transaction().map_err(|e| format!("begin tx: {e}"))?;
        tx.execute(
            "DELETE FROM game_disc_tracks WHERE game_id = ?1",
            params![game_id],
        )
        .map_err(|e| format!("clear game_disc_tracks for {game_id}: {e}"))?;
        let mut written = 0usize;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT OR REPLACE INTO game_disc_tracks
                       (game_id, track_number, sha1, track_mode, size_bytes,
                        file_mtime, file_size, last_hashed_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                )
                .map_err(|e| format!("prepare insert game_disc_tracks: {e}"))?;
            for t in tracks {
                stmt.execute(params![
                    game_id,
                    t.track_number as i64,
                    t.sha1.to_ascii_lowercase(),
                    t.track_mode,
                    t.size_bytes as i64,
                    file_mtime,
                    file_size,
                    last_hashed_at,
                ])
                .map_err(|e| {
                    format!(
                        "insert game_disc_tracks game_id={game_id} track={}: {e}",
                        t.track_number
                    )
                })?;
                written += 1;
            }
        }
        tx.commit()
            .map_err(|e| format!("commit write_game_disc_tracks: {e}"))?;
        Ok(written)
    }

    /// Fetch the cached per-track hashes for one game, plus the
    /// mtime/size stamps at hash time. Returns `Ok(None)` when no
    /// cache row exists yet (first identify) — the caller hashes the
    /// disc fresh.
    ///
    /// The cache-validity check is the caller's responsibility: stat()
    /// the disc file, compare its current mtime+size against the
    /// returned stamps, and treat mismatch as "operator replaced the
    /// dump" → delete + re-hash via [`clear_game_disc_tracks`].
    pub fn get_game_disc_tracks(
        &self,
        game_id: &str,
    ) -> Result<Option<GameDiscTracksCache>, String> {
        let conn = self
            .inner
            .lock()
            .map_err(|_| "library_db: lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT track_number, sha1, track_mode, size_bytes,
                        file_mtime, file_size, last_hashed_at
                 FROM game_disc_tracks
                 WHERE game_id = ?1
                 ORDER BY track_number ASC",
            )
            .map_err(|e| format!("prepare get_game_disc_tracks: {e}"))?;
        let mut rows = stmt
            .query(params![game_id])
            .map_err(|e| format!("query get_game_disc_tracks: {e}"))?;
        let mut tracks: Vec<crate::disc_track_hash::TrackHash> = Vec::new();
        let mut stamps: Option<(i64, i64, i64)> = None;
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("step get_game_disc_tracks: {e}"))?
        {
            let track_number: i64 = row.get(0).map_err(|e| format!("col: {e}"))?;
            let sha1: String = row.get(1).map_err(|e| format!("col: {e}"))?;
            let track_mode: String = row.get(2).map_err(|e| format!("col: {e}"))?;
            let size_bytes: i64 = row.get(3).map_err(|e| format!("col: {e}"))?;
            let file_mtime: i64 = row.get(4).map_err(|e| format!("col: {e}"))?;
            let file_size: i64 = row.get(5).map_err(|e| format!("col: {e}"))?;
            let last_hashed_at: i64 = row.get(6).map_err(|e| format!("col: {e}"))?;
            tracks.push(crate::disc_track_hash::TrackHash {
                track_number: track_number as u32,
                track_mode,
                sha1,
                size_bytes: size_bytes as u64,
            });
            if stamps.is_none() {
                stamps = Some((file_mtime, file_size, last_hashed_at));
            }
        }
        if let Some((file_mtime, file_size, last_hashed_at)) = stamps {
            Ok(Some(GameDiscTracksCache {
                tracks,
                file_mtime,
                file_size,
                last_hashed_at,
            }))
        } else {
            Ok(None)
        }
    }

    /// Drop every cached row for a game. Called when the cache-validity
    /// check detects mtime/size drift, or via the games-table ON DELETE
    /// CASCADE when the operator removes a game from the library.
    #[allow(dead_code)]
    pub fn clear_game_disc_tracks(&self, game_id: &str) -> Result<usize, String> {
        let conn = self
            .inner
            .lock()
            .map_err(|_| "library_db: lock poisoned".to_string())?;
        let n = conn
            .execute(
                "DELETE FROM game_disc_tracks WHERE game_id = ?1",
                params![game_id],
            )
            .map_err(|e| format!("clear game_disc_tracks {game_id}: {e}"))?;
        Ok(n)
    }

    /// Return one [`RomTrackRow`] per distinct game_name in
    /// `rom_hashes_tracks` for the system, picking the
    /// lowest-track-number entry as the stamp marker. Used by the
    /// filename-fuzzy identification path (Phase A1 pivot
    /// 2026-06-03) to build an in-memory index of canonical titles
    /// for the system.
    pub fn list_canonical_disc_titles(
        &self,
        system_id: &str,
    ) -> Result<Vec<RomTrackRow>, String> {
        let conn = self
            .inner
            .lock()
            .map_err(|_| "library_db: lock poisoned".to_string())?;
        // Subquery picks min track_number per (system_id, game_name)
        // so we get one row per game, with a deterministic stamp sha1.
        let mut stmt = conn
            .prepare(
                "SELECT t1.sha1, t1.system_id, t1.game_name, t1.serial,
                        t1.track_number, t1.track_mode, t1.size_bytes
                 FROM rom_hashes_tracks t1
                 WHERE t1.system_id = ?1
                   AND t1.track_number = (
                       SELECT MIN(t2.track_number)
                       FROM rom_hashes_tracks t2
                       WHERE t2.system_id = t1.system_id
                         AND t2.game_name = t1.game_name
                   )",
            )
            .map_err(|e| format!("prepare list_canonical_disc_titles: {e}"))?;
        let mut rows = stmt
            .query(params![system_id])
            .map_err(|e| format!("query list_canonical_disc_titles: {e}"))?;
        let mut out = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("step list_canonical_disc_titles: {e}"))?
        {
            out.push(RomTrackRow {
                sha1: row.get(0).map_err(|e| format!("col: {e}"))?,
                system_id: row.get(1).map_err(|e| format!("col: {e}"))?,
                game_name: row.get(2).map_err(|e| format!("col: {e}"))?,
                serial: row.get(3).map_err(|e| format!("col: {e}"))?,
                track_number: row
                    .get::<_, i64>(4)
                    .map_err(|e| format!("col: {e}"))? as u32,
                track_mode: row.get(5).map_err(|e| format!("col: {e}"))?,
                size_bytes: row.get(6).map_err(|e| format!("col: {e}"))?,
            });
        }
        Ok(out)
    }

    /// Look up the canonical-side per-track entries for a game name +
    /// system. Used by the strictness evaluator to verify the
    /// operator's full track set against the candidate's full set
    /// after a single-track SHA-1 hit narrowed down the candidate.
    pub fn lookup_rom_hashes_tracks_for_game(
        &self,
        system_id: &str,
        game_name: &str,
    ) -> Result<Vec<RomTrackRow>, String> {
        let conn = self
            .inner
            .lock()
            .map_err(|_| "library_db: lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT sha1, system_id, game_name, serial, track_number, track_mode, size_bytes
                 FROM rom_hashes_tracks
                 WHERE system_id = ?1 AND game_name = ?2
                 ORDER BY track_number ASC",
            )
            .map_err(|e| format!("prepare lookup_rom_hashes_tracks_for_game: {e}"))?;
        let mut rows = stmt
            .query(params![system_id, game_name])
            .map_err(|e| format!("query lookup_rom_hashes_tracks_for_game: {e}"))?;
        let mut out = Vec::new();
        while let Some(row) = rows
            .next()
            .map_err(|e| format!("step lookup_rom_hashes_tracks_for_game: {e}"))?
        {
            out.push(RomTrackRow {
                sha1: row.get(0).map_err(|e| format!("col: {e}"))?,
                system_id: row.get(1).map_err(|e| format!("col: {e}"))?,
                game_name: row.get(2).map_err(|e| format!("col: {e}"))?,
                serial: row.get(3).map_err(|e| format!("col: {e}"))?,
                track_number: row
                    .get::<_, i64>(4)
                    .map_err(|e| format!("col: {e}"))? as u32,
                track_mode: row.get(5).map_err(|e| format!("col: {e}"))?,
                size_bytes: row.get(6).map_err(|e| format!("col: {e}"))?,
            });
        }
        Ok(out)
    }

    /// Bulk-upsert game_serials rows. Idempotent on (system_id, serial)
    /// — re-running the sync overwrites the canonical_title in place
    /// rather than producing dupes.
    #[allow(dead_code)] // merge-semantics counterpart to replace_game_serials_for_system; kept for any future caller that needs append-only behaviour
    pub fn upsert_game_serials(&self, entries: &[GameSerialRow]) -> Result<usize, String> {
        let mut conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let tx = conn.transaction().map_err(|e| format!("begin tx: {e}"))?;
        let mut written = 0usize;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT OR REPLACE INTO game_serials
                       (system_id, serial, canonical_title, region)
                     VALUES (?1, ?2, ?3, ?4)",
                )
                .map_err(|e| format!("prepare upsert game_serials: {e}"))?;
            for r in entries {
                stmt.execute(params![
                    r.system_id,
                    r.serial,
                    r.canonical_title,
                    r.region,
                ])
                .map_err(|e| format!("insert game_serial {}/{}: {e}", r.system_id, r.serial))?;
                written += 1;
            }
        }
        tx.commit().map_err(|e| format!("commit upsert_game_serials: {e}"))?;
        Ok(written)
    }

    /// Look up a single (system_id, serial) in the game_serials table.
    /// Serials are matched case-sensitively — publisher catalog codes
    /// like "SLUS-00067" / "TGX040080" are uppercase by convention; we
    /// store them exactly as parsed from the upstream `.dat`.
    #[allow(dead_code)] // wired in by the Phase 2b disc-id extractor
    pub fn lookup_game_serial(
        &self,
        system_id: &str,
        serial: &str,
    ) -> Result<Option<GameSerialRow>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT system_id, serial, canonical_title, region
                 FROM game_serials WHERE system_id = ?1 AND serial = ?2",
            )
            .map_err(|e| format!("prepare lookup_game_serial: {e}"))?;
        let mut rows = stmt
            .query(params![system_id, serial])
            .map_err(|e| format!("query lookup_game_serial: {e}"))?;
        if let Some(row) = rows.next().map_err(|e| format!("step lookup_game_serial: {e}"))? {
            Ok(Some(GameSerialRow {
                system_id: row.get(0).map_err(|e| format!("col system_id: {e}"))?,
                serial: row.get(1).map_err(|e| format!("col serial: {e}"))?,
                canonical_title: row.get(2).map_err(|e| format!("col canonical_title: {e}"))?,
                region: row.get(3).map_err(|e| format!("col region: {e}"))?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Replace the entire `game_serials` corpus for a given system —
    /// same shape as `replace_rom_hashes_for_system`, see that for the
    /// rationale.
    pub fn replace_game_serials_for_system(
        &self,
        system_id: &str,
        entries: &[GameSerialRow],
    ) -> Result<usize, String> {
        let mut conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let tx = conn.transaction().map_err(|e| format!("begin tx: {e}"))?;
        tx.execute(
            "DELETE FROM game_serials WHERE system_id = ?1",
            params![system_id],
        )
        .map_err(|e| format!("delete game_serials for {system_id}: {e}"))?;
        let mut written = 0usize;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT OR REPLACE INTO game_serials
                       (system_id, serial, canonical_title, region)
                     VALUES (?1, ?2, ?3, ?4)",
                )
                .map_err(|e| format!("prepare replace game_serials: {e}"))?;
            for r in entries {
                if r.system_id != system_id {
                    continue;
                }
                stmt.execute(params![
                    r.system_id,
                    r.serial,
                    r.canonical_title,
                    r.region,
                ])
                .map_err(|e| format!("insert game_serial {}/{}: {e}", r.system_id, r.serial))?;
                written += 1;
            }
        }
        tx.commit().map_err(|e| format!("commit replace_game_serials: {e}"))?;
        Ok(written)
    }

    /// Pin a (system_id, base_title) group to a specific variant. The
    /// next time the library is rendered, this group's default tile
    /// represents `preferred_game_id`. The base_title is stored
    /// lowercased so case-quirky upstream titles still match.
    pub fn set_game_group_default(
        &self,
        system_id: &str,
        base_title: &str,
        preferred_game_id: &str,
    ) -> Result<(), String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO game_group_defaults
               (system_id, base_title, preferred_game_id)
             VALUES (?1, ?2, ?3)",
            params![system_id, base_title.to_lowercase(), preferred_game_id],
        )
        .map_err(|e| format!("set_game_group_default: {e}"))?;
        Ok(())
    }

    /// Remove a group's variant pin so the priority resolver falls back
    /// to its region/revision rules. Idempotent: clearing a non-existent
    /// row is a no-op.
    pub fn clear_game_group_default(
        &self,
        system_id: &str,
        base_title: &str,
    ) -> Result<(), String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        conn.execute(
            "DELETE FROM game_group_defaults WHERE system_id = ?1 AND base_title = ?2",
            params![system_id, base_title.to_lowercase()],
        )
        .map_err(|e| format!("clear_game_group_default: {e}"))?;
        Ok(())
    }

    /// Return every group→preferred-game-id pin for a system. The
    /// aggregator consults this map when picking the default variant of
    /// each group; an unpinned group falls back to the priority rules.
    pub fn list_game_group_defaults_for_system(
        &self,
        system_id: &str,
    ) -> Result<std::collections::HashMap<String, String>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT base_title, preferred_game_id
                 FROM game_group_defaults WHERE system_id = ?1",
            )
            .map_err(|e| format!("prepare list_group_defaults: {e}"))?;
        let rows = stmt
            .query_map(params![system_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| format!("query list_group_defaults: {e}"))?;
        let mut out = std::collections::HashMap::new();
        for r in rows {
            let (base, game_id) = r.map_err(|e| format!("row list_group_defaults: {e}"))?;
            out.insert(base, game_id);
        }
        Ok(out)
    }

    /// Diagnostic — how many game_serials rows we hold for a given
    /// system. Exercised by the v9 migration test; parallels
    /// `count_rom_hashes`.
    #[allow(dead_code)]
    /// Count how many games in the system have a sha1 stamped (i.e.
    /// have been through a successful Identify ROMs pass). Used by
    /// resolve_rom_hashes_for_system to report "X of Y already
    /// identified, M remaining" in its summary — without this number
    /// the no-op re-run case (everything's already stamped) shows
    /// "0/0 scanned" with no context as to why.
    pub fn count_games_with_hash_for_system(&self, system_id: &str) -> Result<i64, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM games
                 WHERE system_id = ?1
                   AND sha1 IS NOT NULL
                   AND sha1 <> ''",
                params![system_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("count_games_with_hash_for_system: {e}"))?;
        Ok(count)
    }

    /// Total games in the system (no filter on sha1 / disc_id). Used
    /// alongside count_games_with_hash_for_system to derive
    /// "X of Y already identified".
    pub fn count_games_for_system(&self, system_id: &str) -> Result<i64, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM games WHERE system_id = ?1",
                params![system_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("count_games_for_system: {e}"))?;
        Ok(count)
    }

    pub fn count_game_serials(&self, system_id: &str) -> Result<i64, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        conn.query_row(
            "SELECT COUNT(*) FROM game_serials WHERE system_id = ?1",
            params![system_id],
            |r| r.get(0),
        )
        .map_err(|e| format!("count game_serials: {e}"))
    }

    pub fn delete_game(&self, id: &str) -> Result<(), String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        // Sweep dangling custom-collection memberships first so a
        // deleted game doesn't leave orphan rows that survive across
        // sessions. The custom_collections FK only cascades on the
        // collection side; there's no FK from games → members because
        // games come and go independently of operator curation.
        conn.execute(
            "DELETE FROM custom_collection_members WHERE rom_id = ?1",
            params![id],
        )
        .map_err(|e| format!("delete game memberships: {e}"))?;
        conn.execute("DELETE FROM games WHERE id = ?1", params![id])
            .map_err(|e| format!("delete game: {e}"))?;
        Ok(())
    }

    /// Full-text search across title + normalized_title + developer + publisher.
    /// Empty query returns all rows (capped by `limit`). Query string is wrapped
    /// in FTS5 prefix syntax (`"foo bar"*`) so partial typing matches early.
    pub fn search_games(&self, query: &str, limit: usize) -> Result<Vec<GameRow>, String> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            // Fast path: just return the limited list.
            let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
            let mut stmt = conn
                .prepare(
                    "SELECT id, system_id, file_path, title, added_at,
                            core_override, cover_path, seed, archive_inner_path,
                            sha1, serial, disc_id
                     FROM games
                     ORDER BY title COLLATE NOCASE
                     LIMIT ?1",
                )
                .map_err(|e| format!("prepare search empty: {e}"))?;
            let rows = stmt
                .query_map([limit as i64], |row| {
                    Ok(GameRow {
                        id: row.get(0)?,
                        system_id: row.get(1)?,
                        file_path: row.get(2)?,
                        title: row.get(3)?,
                        added_at: row.get(4)?,
                        core_override: row.get(5)?,
                        cover_path: row.get(6)?,
                        seed: row.get::<_, i64>(7)? != 0,
                        archive_inner_path: row.get(8)?,
                        sha1: row.get(9)?,
                        serial: row.get(10)?,
                        disc_id: row.get(11)?,
                        // Phase C3 — search results don't need smart-list data;
                        // tile rendering reads from the LibraryStore which is
                        // hydrated via list_games (which does carry them).
                        favorite: false,
                        completed: false,
                        last_played_at: None,
                        play_time_secs: 0,
                        players: None,
                        rating: None,
                        disc_set_id: None,
                        disc_number: None,
                    })
                })
                .map_err(|e| format!("query search empty: {e}"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("collect search empty: {e}"))?;
            return Ok(rows);
        }

        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        // FTS5 query — escape inner double quotes, wrap as a prefix match.
        let fts_query = format!("\"{}\"*", trimmed.replace('"', "\"\""));
        let mut stmt = conn
            .prepare(
                "SELECT g.id, g.system_id, g.file_path, g.title, g.added_at,
                        g.core_override, g.cover_path, g.seed, g.archive_inner_path,
                        g.sha1, g.serial, g.disc_id
                 FROM games g
                 INNER JOIN games_fts f ON f.rowid = g.rowid
                 WHERE games_fts MATCH ?1
                 ORDER BY rank
                 LIMIT ?2",
            )
            .map_err(|e| format!("prepare search: {e}"))?;
        let rows = stmt
            .query_map(params![fts_query, limit as i64], |row| {
                Ok(GameRow {
                    id: row.get(0)?,
                    system_id: row.get(1)?,
                    file_path: row.get(2)?,
                    title: row.get(3)?,
                    added_at: row.get(4)?,
                    core_override: row.get(5)?,
                    cover_path: row.get(6)?,
                    seed: row.get::<_, i64>(7)? != 0,
                    archive_inner_path: row.get(8)?,
                    sha1: row.get(9)?,
                    serial: row.get(10)?,
                    disc_id: row.get(11)?,
                    // Phase C3 — FTS search hits; smart-list data lives on the
                    // LibraryStore's full entry, not search-result rows.
                    favorite: false,
                    completed: false,
                    last_played_at: None,
                    play_time_secs: 0,
                    players: None,
                    rating: None,
                    disc_set_id: None,
                    disc_number: None,
                })
            })
            .map_err(|e| format!("query search: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("collect search: {e}"))?;
        Ok(rows)
    }

    pub fn count(&self) -> Result<usize, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM games", [], |row| row.get(0))
            .map_err(|e| format!("count games: {e}"))?;
        Ok(n as usize)
    }

    /// Look up a game id by its `file_path`. Returns None when no row
    /// matches. Used by the auto-remove-on-delete path so the watcher
    /// callback can find the id from just the path the OS reported.
    pub fn find_id_by_file_path(&self, file_path: &str) -> Result<Option<String>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let id: Option<String> = conn
            .query_row(
                "SELECT id FROM games WHERE file_path = ?1",
                params![file_path],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("find_id_by_file_path: {e}"))?;
        Ok(id)
    }

    /// Delete every game tagged with the given system id. Returns the
    /// number of rows removed. Used by the Settings → Library "Clear
    /// games for this system" action.
    /// Return every game `id` tagged with the given system. Cheap — just
    /// the id column. Used by the metadata-clear path which needs to
    /// walk media_db entries scoped to one system without materializing
    /// full game rows.
    pub fn list_game_ids_for_system(&self, system_id: &str) -> Result<Vec<String>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare("SELECT id FROM games WHERE system_id = ?1")
            .map_err(|e| format!("prepare list_game_ids_for_system: {e}"))?;
        let rows = stmt
            .query_map(params![system_id], |row| row.get::<_, String>(0))
            .map_err(|e| format!("query list_game_ids_for_system: {e}"))?;
        let mut ids = Vec::new();
        for row in rows {
            ids.push(row.map_err(|e| format!("step list_game_ids_for_system: {e}"))?);
        }
        Ok(ids)
    }

    pub fn delete_games_for_system(&self, system_id: &str) -> Result<usize, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let n = conn
            .execute("DELETE FROM games WHERE system_id = ?1", params![system_id])
            .map_err(|e| format!("delete_games_for_system: {e}"))?;
        Ok(n)
    }

    /// Delete every game row. Returns the count removed. Used by the
    /// Settings → Library "Reset entire library" action (with a
    /// confirmation dialog on the frontend before firing).
    pub fn delete_all_games(&self) -> Result<usize, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let n = conn
            .execute("DELETE FROM games", [])
            .map_err(|e| format!("delete_all_games: {e}"))?;
        Ok(n)
    }

    // --- Per-game overrides (Phase 2.8 slice D) --------------------------
    //
    // Lives in `games.overrides_json`. NULL = no overrides set. Round-trips
    // through serde so reading a malformed JSON blob silently returns the
    // empty struct rather than failing the launch path that depends on it.

    pub fn get_game_overrides(&self, id: &str) -> Result<GameOverrides, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let raw: Option<String> = conn
            .query_row(
                "SELECT overrides_json FROM games WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("get_game_overrides query: {e}"))?
            .flatten();
        let Some(json) = raw else { return Ok(GameOverrides::default()) };
        Ok(serde_json::from_str(&json).unwrap_or_default())
    }

    /// Replace the override bag for one game. Pass `GameOverrides::default()`
    /// (or a struct with every field None) to clear — the JSON serializes
    /// to `{}` which we then write as NULL to keep the column sparse.
    pub fn set_game_overrides(
        &self,
        id: &str,
        overrides: &GameOverrides,
    ) -> Result<(), String> {
        // Compare against the all-default struct so adding a new field
        // to GameOverrides can't silently regress this check. The old
        // hand-listed AND-chain went stale every time a field landed
        // (display_aspect_override / overscan_crop_override / bezel /
        // keypad_layout_note / libretro_device + per-port siblings /
        // analog_routing / platform_music_path / dosbox_entry_point
        // were all missing) — saving an override consisting ONLY of a
        // missing field evaluated "empty" and NULL'd the row, wiping
        // the value the operator just set.
        let is_empty = overrides == &GameOverrides::default();
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        if is_empty {
            conn.execute(
                "UPDATE games SET overrides_json = NULL WHERE id = ?1",
                params![id],
            )
            .map_err(|e| format!("clear overrides: {e}"))?;
            return Ok(());
        }
        let json = serde_json::to_string(overrides)
            .map_err(|e| format!("serialize overrides: {e}"))?;
        conn.execute(
            "UPDATE games SET overrides_json = ?1 WHERE id = ?2",
            params![json, id],
        )
        .map_err(|e| format!("write overrides: {e}"))?;
        Ok(())
    }

    // --- Game info overrides (Game Info Panel v1, Phase 3) ---------------
    //
    // Layer 3 of the data model. Lives in a dedicated columnar table
    // `game_info_overrides` keyed by (system_id, rom_id). Field-typed
    // precedence merging with the file-layer (Phase 4) produces the
    // final per-game record the UI shows.

    /// Read the operator's local overrides for one game. Returns
    /// [`crate::game_info::GameInfoOverride::default`] when no row
    /// exists — the "no overrides" case is the common path, not an
    /// error.
    pub fn get_game_info_override(
        &self,
        system_id: &str,
        rom_id: &str,
    ) -> Result<crate::game_info::GameInfoOverride, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let row = conn
            .query_row(
                "SELECT short_summary, controls_supported, best_emulator,
                        best_emulator_reason, bugs,
                        applied_best_emulator, applied_controls
                 FROM game_info_overrides
                 WHERE system_id = ?1 AND rom_id = ?2",
                params![system_id, rom_id],
                |row| {
                    let short_summary: Option<String> = row.get(0)?;
                    let controls_json: Option<String> = row.get(1)?;
                    let best_emulator: Option<String> = row.get(2)?;
                    let best_emulator_reason: Option<String> = row.get(3)?;
                    let bugs_json: Option<String> = row.get(4)?;
                    let applied_best_emulator: i64 = row.get(5)?;
                    let applied_controls: i64 = row.get(6)?;
                    Ok((
                        short_summary,
                        controls_json,
                        best_emulator,
                        best_emulator_reason,
                        bugs_json,
                        applied_best_emulator != 0,
                        applied_controls != 0,
                    ))
                },
            )
            .optional()
            .map_err(|e| format!("get_game_info_override query: {e}"))?;

        let Some((
            short_summary,
            controls_json,
            best_emulator,
            best_emulator_reason,
            bugs_json,
            applied_best_emulator,
            applied_controls,
        )) = row
        else {
            return Ok(crate::game_info::GameInfoOverride::default());
        };

        // Malformed JSON in either array column degrades to None (no
        // override) rather than failing the read. Matches the
        // game_overrides_json pattern — corrupt overrides shouldn't
        // brick the panel.
        let controls_supported = controls_json
            .as_deref()
            .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok());
        let bugs = bugs_json
            .as_deref()
            .and_then(|s| serde_json::from_str::<Vec<crate::game_info::GameBug>>(s).ok());

        Ok(crate::game_info::GameInfoOverride {
            short_summary,
            controls_supported,
            best_emulator,
            best_emulator_reason,
            bugs,
            applied_best_emulator,
            applied_controls,
        })
    }

    /// Upsert the operator's local overrides for one game. Passing a
    /// default-constructed (empty) override deletes the row so the
    /// table stays sparse.
    pub fn set_game_info_override(
        &self,
        system_id: &str,
        rom_id: &str,
        ov: &crate::game_info::GameInfoOverride,
    ) -> Result<(), String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;

        if ov.is_empty() {
            conn.execute(
                "DELETE FROM game_info_overrides WHERE system_id = ?1 AND rom_id = ?2",
                params![system_id, rom_id],
            )
            .map_err(|e| format!("delete game_info_override: {e}"))?;
            return Ok(());
        }

        let controls_json = ov
            .controls_supported
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "[]".into()));
        let bugs_json = ov
            .bugs
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "[]".into()));
        let now: i64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        conn.execute(
            r#"
            INSERT INTO game_info_overrides (
                system_id, rom_id,
                short_summary, controls_supported,
                best_emulator, best_emulator_reason,
                bugs, applied_best_emulator, applied_controls,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)
            ON CONFLICT(system_id, rom_id) DO UPDATE SET
                short_summary         = excluded.short_summary,
                controls_supported    = excluded.controls_supported,
                best_emulator         = excluded.best_emulator,
                best_emulator_reason  = excluded.best_emulator_reason,
                bugs                  = excluded.bugs,
                applied_best_emulator = excluded.applied_best_emulator,
                applied_controls      = excluded.applied_controls,
                updated_at            = excluded.updated_at
            "#,
            params![
                system_id,
                rom_id,
                ov.short_summary,
                controls_json,
                ov.best_emulator,
                ov.best_emulator_reason,
                bugs_json,
                if ov.applied_best_emulator { 1i64 } else { 0i64 },
                if ov.applied_controls { 1i64 } else { 0i64 },
                now,
            ],
        )
        .map_err(|e| format!("upsert game_info_override: {e}"))?;
        Ok(())
    }

    /// Bulk load every operator override row as full
    /// `(system_id, rom_id, GameInfoOverride)` tuples. Used by the
    /// tile-badge query path to merge file-layer + override-layer in
    /// one pass over the library rather than N queries.
    pub fn list_all_game_info_overrides(
        &self,
    ) -> Result<Vec<(String, String, crate::game_info::GameInfoOverride)>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT system_id, rom_id, short_summary, controls_supported,
                        best_emulator, best_emulator_reason, bugs,
                        applied_best_emulator, applied_controls
                 FROM game_info_overrides",
            )
            .map_err(|e| format!("list_all_game_info_overrides prepare: {e}"))?;
        let rows = stmt
            .query_map([], |row| {
                let system_id: String = row.get(0)?;
                let rom_id: String = row.get(1)?;
                let short_summary: Option<String> = row.get(2)?;
                let controls_json: Option<String> = row.get(3)?;
                let best_emulator: Option<String> = row.get(4)?;
                let best_emulator_reason: Option<String> = row.get(5)?;
                let bugs_json: Option<String> = row.get(6)?;
                let applied_best_emulator: i64 = row.get(7)?;
                let applied_controls: i64 = row.get(8)?;
                let controls_supported = controls_json
                    .as_deref()
                    .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok());
                let bugs = bugs_json
                    .as_deref()
                    .and_then(|s| serde_json::from_str::<Vec<crate::game_info::GameBug>>(s).ok());
                Ok((
                    system_id,
                    rom_id,
                    crate::game_info::GameInfoOverride {
                        short_summary,
                        controls_supported,
                        best_emulator,
                        best_emulator_reason,
                        bugs,
                        applied_best_emulator: applied_best_emulator != 0,
                        applied_controls: applied_controls != 0,
                    },
                ))
            })
            .map_err(|e| format!("list_all_game_info_overrides query: {e}"))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| format!("list_all_game_info_overrides row: {e}"))?);
        }
        Ok(out)
    }

    /// List `(system_id, rom_id)` pairs for every game with at least
    /// one operator override. Used by the tile-badge layer to mark
    /// locally-edited games with the `✎` indicator. Cheap: covers an
    /// index on `system_id` + returns just two columns.
    pub fn list_game_info_overridden(&self) -> Result<Vec<(String, String)>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare("SELECT system_id, rom_id FROM game_info_overrides")
            .map_err(|e| format!("list_game_info_overridden prepare: {e}"))?;
        let rows = stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            .map_err(|e| format!("list_game_info_overridden query: {e}"))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| format!("list_game_info_overridden row: {e}"))?);
        }
        Ok(out)
    }

    // --- System info CRUD (System Info Panel v1, Phase 2) ---------------
    //
    // Three tables back the merge layer: system_info_mame (L1, baked
    // from assets/mame-source/), system_info_curated (L2, baked from
    // docs/cores/<id>/system-info.yaml), and system_info_overrides
    // (L3, written by the per-system Settings drill-in edit UI). The
    // bake-on-launch path (main.rs::bake_system_info_on_launch) writes
    // L1+L2 in bulk; the per-system query path joins all three on
    // system_id via merge_system_info.
    //
    // system_info_meta carries the content hash of the slim files +
    // YAMLs; bake_system_info_on_launch compares against this hash to
    // decide whether a rebake is needed (cheap: ~5ms hash, ~50-100ms
    // rebake on miss).

    /// Read one L1 row by system slug. Returns `None` when the system
    /// has no MAME-baseline data (DOSBox / ScummVM / PSP / PS2 / NDS /
    /// GameCube / 3DO / MSX / MSX2 — anything the extractor's
    /// MAME_DRIVER_MAP doesn't cover or the upstream MAME release
    /// doesn't ship).
    pub fn get_system_info_mame(
        &self,
        system_id: &str,
    ) -> Result<Option<crate::system_info::SystemInfoMame>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let row = conn
            .query_row(
                "SELECT machine_name, year, manufacturer, cpu, sound,
                        resolution, refresh_rate, max_players,
                        peripheral_hints, description
                 FROM system_info_mame WHERE system_id = ?1",
                params![system_id],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, Option<i64>>(7)?,
                        row.get::<_, Option<String>>(8)?,
                        row.get::<_, Option<String>>(9)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| format!("get_system_info_mame: {e}"))?;
        let Some((
            machine_name,
            year,
            manufacturer,
            cpu,
            sound,
            resolution,
            refresh_rate,
            max_players,
            peripheral_hints_json,
            description,
        )) = row
        else {
            return Ok(None);
        };
        // Malformed JSON degrades to an empty list rather than
        // poisoning the read — matches the game_info_overrides bug
        // list parsing convention.
        let peripheral_hints: Vec<String> = peripheral_hints_json
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();
        Ok(Some(crate::system_info::SystemInfoMame {
            system_id: system_id.to_string(),
            machine_name,
            year,
            manufacturer,
            cpu,
            sound,
            resolution,
            refresh_rate,
            max_players: max_players.and_then(|n| u32::try_from(n).ok()),
            peripheral_hints,
            description,
        }))
    }

    /// Read one L2 row by system slug. Returns `None` for systems
    /// without a `docs/cores/<id>/system-info.yaml` file (most of the
    /// 45 systems in v1 — the L2 layer fills in over time).
    pub fn get_system_info_curated(
        &self,
        system_id: &str,
    ) -> Result<Option<crate::system_info::SystemInfoCurated>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let sid = system_id.to_string();
        let row = conn
            .query_row(
                "SELECT manufacturer, system_type, generation, release_date,
                        discontinued, units_sold, media, cpu, sound,
                        resolution, color_palette, display_ratio,
                        architecture, max_players, multiplayer, region,
                        storage, ram, video_output, aspect_ratio, refresh_rate,
                        peripherals, release_flag, tagline, blurb,
                        sidebar_subline, schema_version, last_updated
                 FROM system_info_curated WHERE system_id = ?1",
                params![system_id],
                |row| {
                    let peripherals_json: Option<String> = row.get(21)?;
                    let peripherals = peripherals_json
                        .as_deref()
                        .and_then(|s| serde_json::from_str::<Vec<crate::system_info::Peripheral>>(s).ok())
                        .unwrap_or_default();
                    Ok(crate::system_info::SystemInfoCurated {
                        system_id: sid.clone(),
                        manufacturer: row.get(0)?,
                        system_type: row.get(1)?,
                        generation: row.get(2)?,
                        release_date: row.get(3)?,
                        discontinued: row.get(4)?,
                        units_sold: row.get(5)?,
                        media: row.get(6)?,
                        cpu: row.get(7)?,
                        sound: row.get(8)?,
                        resolution: row.get(9)?,
                        color_palette: row.get(10)?,
                        display_ratio: row.get(11)?,
                        architecture: row.get(12)?,
                        max_players: row.get(13)?,
                        multiplayer: row.get(14)?,
                        region: row.get(15)?,
                        storage: row.get(16)?,
                        ram: row.get(17)?,
                        video_output: row.get(18)?,
                        aspect_ratio: row.get(19)?,
                        refresh_rate: row.get(20)?,
                        peripherals,
                        release_flag: row.get(22)?,
                        tagline: row.get(23)?,
                        blurb: row.get(24)?,
                        sidebar_subline: row.get(25)?,
                        meta: crate::system_info::SystemInfoMeta {
                            schema_version: row.get::<_, i64>(26)? as u32,
                            last_updated: row.get(27)?,
                            contributors: Vec::new(),
                        },
                    })
                },
            )
            .optional()
            .map_err(|e| format!("get_system_info_curated: {e}"))?;
        Ok(row)
    }

    /// Read the operator's L3 overrides for one system. Returns the
    /// default-constructed (empty) override when no row exists — the
    /// "no overrides" case is the common path, not an error.
    pub fn get_system_info_override(
        &self,
        system_id: &str,
    ) -> Result<crate::system_info::SystemInfoOverride, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let row = conn
            .query_row(
                "SELECT manufacturer, system_type, generation, release_date,
                        discontinued, units_sold, media, cpu, sound,
                        resolution, color_palette, display_ratio,
                        architecture, max_players, multiplayer, region,
                        storage, ram, video_output, aspect_ratio, refresh_rate,
                        peripherals, release_flag, tagline, blurb, sidebar_subline
                 FROM system_info_overrides WHERE system_id = ?1",
                params![system_id],
                |row| {
                    let peripherals_json: Option<String> = row.get(21)?;
                    // None = no override; Some(json) = override (possibly
                    // empty vec for "operator cleared the list"). Malformed
                    // JSON downgrades to None — same robustness pattern as
                    // game_info bug list parsing.
                    let peripherals: Option<Vec<crate::system_info::Peripheral>> = peripherals_json
                        .as_deref()
                        .and_then(|s| serde_json::from_str::<Vec<crate::system_info::Peripheral>>(s).ok());
                    Ok(crate::system_info::SystemInfoOverride {
                        manufacturer: row.get(0)?,
                        system_type: row.get(1)?,
                        generation: row.get(2)?,
                        release_date: row.get(3)?,
                        discontinued: row.get(4)?,
                        units_sold: row.get(5)?,
                        media: row.get(6)?,
                        cpu: row.get(7)?,
                        sound: row.get(8)?,
                        resolution: row.get(9)?,
                        color_palette: row.get(10)?,
                        display_ratio: row.get(11)?,
                        architecture: row.get(12)?,
                        max_players: row.get(13)?,
                        multiplayer: row.get(14)?,
                        region: row.get(15)?,
                        storage: row.get(16)?,
                        ram: row.get(17)?,
                        video_output: row.get(18)?,
                        aspect_ratio: row.get(19)?,
                        refresh_rate: row.get(20)?,
                        peripherals,
                        release_flag: row.get(22)?,
                        tagline: row.get(23)?,
                        blurb: row.get(24)?,
                        sidebar_subline: row.get(25)?,
                    })
                },
            )
            .optional()
            .map_err(|e| format!("get_system_info_override: {e}"))?;
        Ok(row.unwrap_or_default())
    }

    /// Upsert the operator's L3 overrides for one system. A default-
    /// constructed (empty) override deletes the row so the table stays
    /// sparse.
    pub fn set_system_info_override(
        &self,
        system_id: &str,
        ov: &crate::system_info::SystemInfoOverride,
    ) -> Result<(), String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        if ov.is_empty() {
            conn.execute(
                "DELETE FROM system_info_overrides WHERE system_id = ?1",
                params![system_id],
            )
            .map_err(|e| format!("delete system_info_override: {e}"))?;
            return Ok(());
        }

        let peripherals_json = ov
            .peripherals
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "[]".into()));
        let now: i64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        conn.execute(
            r#"
            INSERT INTO system_info_overrides (
                system_id,
                manufacturer, system_type, generation, release_date,
                discontinued, units_sold, media, cpu, sound,
                resolution, color_palette, display_ratio,
                architecture, max_players, multiplayer, region,
                storage, ram, video_output, aspect_ratio, refresh_rate,
                peripherals, release_flag, tagline, blurb, sidebar_subline,
                created_at, updated_at
            ) VALUES (
                ?1,
                ?2, ?3, ?4, ?5,
                ?6, ?7, ?8, ?9, ?10,
                ?11, ?12, ?13,
                ?14, ?15, ?16, ?17,
                ?18, ?19, ?20, ?21, ?22,
                ?23, ?24, ?25, ?26, ?27,
                ?28, ?28
            )
            ON CONFLICT(system_id) DO UPDATE SET
                manufacturer    = excluded.manufacturer,
                system_type     = excluded.system_type,
                generation      = excluded.generation,
                release_date    = excluded.release_date,
                discontinued    = excluded.discontinued,
                units_sold      = excluded.units_sold,
                media           = excluded.media,
                cpu             = excluded.cpu,
                sound           = excluded.sound,
                resolution      = excluded.resolution,
                color_palette   = excluded.color_palette,
                display_ratio   = excluded.display_ratio,
                architecture   = excluded.architecture,
                max_players    = excluded.max_players,
                multiplayer    = excluded.multiplayer,
                region         = excluded.region,
                storage        = excluded.storage,
                ram            = excluded.ram,
                video_output   = excluded.video_output,
                aspect_ratio   = excluded.aspect_ratio,
                refresh_rate   = excluded.refresh_rate,
                peripherals    = excluded.peripherals,
                release_flag   = excluded.release_flag,
                tagline        = excluded.tagline,
                blurb          = excluded.blurb,
                sidebar_subline = excluded.sidebar_subline,
                updated_at     = excluded.updated_at
            "#,
            params![
                system_id,
                ov.manufacturer,
                ov.system_type,
                ov.generation,
                ov.release_date,
                ov.discontinued,
                ov.units_sold,
                ov.media,
                ov.cpu,
                ov.sound,
                ov.resolution,
                ov.color_palette,
                ov.display_ratio,
                ov.architecture,
                ov.max_players,
                ov.multiplayer,
                ov.region,
                ov.storage,
                ov.ram,
                ov.video_output,
                ov.aspect_ratio,
                ov.refresh_rate,
                peripherals_json,
                ov.release_flag,
                ov.tagline,
                ov.blurb,
                ov.sidebar_subline,
                now,
            ],
        )
        .map_err(|e| format!("upsert system_info_override: {e}"))?;
        Ok(())
    }

    /// List `system_id`s with at least one operator override — drives
    /// the per-system Settings drill-in "edited" indicator (similar
    /// to the `✎` badge for per-game edits).
    pub fn list_system_info_overridden(&self) -> Result<Vec<String>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare("SELECT system_id FROM system_info_overrides")
            .map_err(|e| format!("list_system_info_overridden prepare: {e}"))?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| format!("list_system_info_overridden query: {e}"))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| format!("list_system_info_overridden row: {e}"))?);
        }
        Ok(out)
    }

    /// Wholesale replace the L1 table. The bake-on-launch path calls
    /// this after parsing `listxml-slim.json` + folding in
    /// `history-slim.xml`'s descriptions. Wrapped in a transaction so
    /// a parse failure mid-rebake doesn't leave the table in a half-
    /// populated state.
    pub fn bake_system_info_mame(
        &self,
        rows: &[crate::system_info::SystemInfoMame],
    ) -> Result<(), String> {
        let mut conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let tx = conn
            .transaction()
            .map_err(|e| format!("bake_system_info_mame tx: {e}"))?;
        tx.execute("DELETE FROM system_info_mame", [])
            .map_err(|e| format!("bake_system_info_mame clear: {e}"))?;
        {
            let mut stmt = tx
                .prepare(
                    r#"
                    INSERT INTO system_info_mame (
                        system_id, machine_name, year, manufacturer,
                        cpu, sound, resolution, refresh_rate,
                        max_players, peripheral_hints, description
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                    "#,
                )
                .map_err(|e| format!("bake_system_info_mame prepare: {e}"))?;
            for r in rows {
                let hints_json = serde_json::to_string(&r.peripheral_hints).unwrap_or_else(|_| "[]".into());
                stmt.execute(params![
                    r.system_id,
                    r.machine_name,
                    r.year,
                    r.manufacturer,
                    r.cpu,
                    r.sound,
                    r.resolution,
                    r.refresh_rate,
                    r.max_players.map(|n| n as i64),
                    hints_json,
                    r.description,
                ])
                .map_err(|e| format!("bake_system_info_mame insert {}: {e}", r.system_id))?;
            }
        }
        tx.commit()
            .map_err(|e| format!("bake_system_info_mame commit: {e}"))?;
        Ok(())
    }

    /// Wholesale replace the L2 table. Same transaction pattern as
    /// `bake_system_info_mame`.
    pub fn bake_system_info_curated(
        &self,
        rows: &[crate::system_info::SystemInfoCurated],
    ) -> Result<(), String> {
        let mut conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let tx = conn
            .transaction()
            .map_err(|e| format!("bake_system_info_curated tx: {e}"))?;
        tx.execute("DELETE FROM system_info_curated", [])
            .map_err(|e| format!("bake_system_info_curated clear: {e}"))?;
        {
            let mut stmt = tx
                .prepare(
                    r#"
                    INSERT INTO system_info_curated (
                        system_id,
                        manufacturer, system_type, generation, release_date,
                        discontinued, units_sold, media, cpu, sound,
                        resolution, color_palette, display_ratio,
                        architecture, max_players, multiplayer, region,
                        storage, ram, video_output, aspect_ratio, refresh_rate,
                        peripherals, release_flag, tagline, blurb, sidebar_subline,
                        schema_version, last_updated
                    ) VALUES (
                        ?1,
                        ?2, ?3, ?4, ?5,
                        ?6, ?7, ?8, ?9, ?10,
                        ?11, ?12, ?13,
                        ?14, ?15, ?16, ?17,
                        ?18, ?19, ?20, ?21, ?22,
                        ?23, ?24, ?25, ?26, ?27,
                        ?28, ?29
                    )
                    "#,
                )
                .map_err(|e| format!("bake_system_info_curated prepare: {e}"))?;
            for r in rows {
                let peripherals_json = if r.peripherals.is_empty() {
                    None
                } else {
                    Some(
                        serde_json::to_string(&r.peripherals)
                            .unwrap_or_else(|_| "[]".into()),
                    )
                };
                stmt.execute(params![
                    r.system_id,
                    r.manufacturer,
                    r.system_type,
                    r.generation,
                    r.release_date,
                    r.discontinued,
                    r.units_sold,
                    r.media,
                    r.cpu,
                    r.sound,
                    r.resolution,
                    r.color_palette,
                    r.display_ratio,
                    r.architecture,
                    r.max_players,
                    r.multiplayer,
                    r.region,
                    r.storage,
                    r.ram,
                    r.video_output,
                    r.aspect_ratio,
                    r.refresh_rate,
                    peripherals_json,
                    r.release_flag,
                    r.tagline,
                    r.blurb,
                    r.sidebar_subline,
                    r.meta.schema_version as i64,
                    r.meta.last_updated,
                ])
                .map_err(|e| {
                    format!("bake_system_info_curated insert {}: {e}", r.system_id)
                })?;
            }
        }
        tx.commit()
            .map_err(|e| format!("bake_system_info_curated commit: {e}"))?;
        Ok(())
    }

    /// Read the stored content hash from `system_info_meta` (key =
    /// `l1_l2_hash`). None when no row exists (first launch, or the
    /// table was just created).
    pub fn get_system_info_meta_hash(&self) -> Result<Option<String>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let row = conn
            .query_row(
                "SELECT value FROM system_info_meta WHERE key = 'l1_l2_hash'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| format!("get_system_info_meta_hash: {e}"))?;
        Ok(row)
    }

    /// Write (or replace) the stored content hash.
    pub fn set_system_info_meta_hash(&self, hash: &str) -> Result<(), String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        conn.execute(
            r#"
            INSERT INTO system_info_meta (key, value) VALUES ('l1_l2_hash', ?1)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value
            "#,
            params![hash],
        )
        .map_err(|e| format!("set_system_info_meta_hash: {e}"))?;
        Ok(())
    }

    // --- Controller info cache (v21 — dynamic-controller-info Slice 3) --
    //
    // Persists what each core's `RETRO_ENVIRONMENT_SET_CONTROLLER_INFO`
    // call advertised on its most recent load. Keyed by
    // (core_filename, port). Written immediately after a successful
    // core load; read by `get_controller_devices` when no core is
    // currently loaded so the per-game Input dialog can still render
    // the right dropdown pre-launch.
    //
    // Invalidation is mtime-based: cached `core_mtime` is compared
    // against the live .dll's current mtime; mismatch → reader returns
    // empty (caller falls back) and the next core load overwrites.

    /// Write or replace the cached per-port device lists for one core.
    /// Pass an array of 5 Vecs (one per libretro port). Empty inner
    /// Vecs are still persisted — `[]` is a valid declaration meaning
    /// "core advertised this port supports nothing." `core_mtime` is
    /// the unix-seconds modification time of the .dll at capture time.
    pub fn upsert_controller_info(
        &self,
        core_filename: &str,
        devices_per_port: &[Vec<oa_core::ControllerDeviceDescriptor>; 5],
        core_mtime: i64,
    ) -> Result<(), String> {
        let now: i64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let mut conn = self
            .inner
            .lock()
            .map_err(|_| "library_db: lock poisoned".to_string())?;
        let tx = conn
            .transaction()
            .map_err(|e| format!("upsert_controller_info tx: {e}"))?;
        for (port, devices) in devices_per_port.iter().enumerate() {
            let json = serde_json::to_string(devices)
                .map_err(|e| format!("serialize controller_info port {port}: {e}"))?;
            tx.execute(
                r#"
                INSERT INTO core_controller_info
                    (core_filename, port, devices_json, captured_at, core_mtime)
                VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(core_filename, port) DO UPDATE SET
                    devices_json = excluded.devices_json,
                    captured_at  = excluded.captured_at,
                    core_mtime   = excluded.core_mtime
                "#,
                params![core_filename, port as i64, json, now, core_mtime],
            )
            .map_err(|e| format!("upsert controller_info port {port}: {e}"))?;
        }
        tx.commit()
            .map_err(|e| format!("upsert_controller_info commit: {e}"))?;
        Ok(())
    }

    /// Read the cached per-port device list. Returns `Ok(None)` when
    /// no row exists OR the cached `core_mtime` doesn't match the
    /// caller-supplied `current_mtime` (stale cache — caller should
    /// fall back to the no-cache path until the core reloads).
    ///
    /// Caller is expected to look up the .dll mtime themselves (it's a
    /// filesystem call; we don't want the DB layer touching disk for
    /// arbitrary paths).
    pub fn cached_controller_devices(
        &self,
        core_filename: &str,
        port: u32,
        current_mtime: i64,
    ) -> Result<Option<Vec<oa_core::ControllerDeviceDescriptor>>, String> {
        let conn = self
            .inner
            .lock()
            .map_err(|_| "library_db: lock poisoned".to_string())?;
        let row: Option<(String, i64)> = conn
            .query_row(
                "SELECT devices_json, core_mtime FROM core_controller_info \
                 WHERE core_filename = ?1 AND port = ?2",
                params![core_filename, port as i64],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|e| format!("cached_controller_devices query: {e}"))?;
        let Some((json, cached_mtime)) = row else {
            return Ok(None);
        };
        if cached_mtime != current_mtime {
            // Stale — caller should fall back to empty + the dialog
            // shows the "launch the game once" hint. Next core load
            // refreshes the row.
            return Ok(None);
        }
        let devices: Vec<oa_core::ControllerDeviceDescriptor> = serde_json::from_str(&json)
            .map_err(|e| format!("cached_controller_devices parse json: {e}"))?;
        Ok(Some(devices))
    }

    // --- MAME games (v17 — listxml-based ROM-set name resolution) -------

    /// Wholesale replace every row in `mame_games`. Single transaction
    /// so a parse failure mid-bake doesn't leave the table half-
    /// populated. Used by the bake-on-launch path (when the bundled
    /// slim's hash changes) and by the operator-driven MAME refresh
    /// (Phase 4) so both paths share the same write logic.
    pub fn bake_mame_games(
        &self,
        rows: &[crate::mame_games::MameGame],
    ) -> Result<(), String> {
        let mut conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let tx = conn
            .transaction()
            .map_err(|e| format!("bake_mame_games tx: {e}"))?;
        tx.execute("DELETE FROM mame_games", [])
            .map_err(|e| format!("bake_mame_games clear: {e}"))?;
        {
            let mut stmt = tx
                .prepare(
                    r#"
                    INSERT INTO mame_games (name, description, year, manufacturer, cloneof)
                    VALUES (?1, ?2, ?3, ?4, ?5)
                    "#,
                )
                .map_err(|e| format!("bake_mame_games prepare: {e}"))?;
            for r in rows {
                stmt.execute(params![
                    r.name.to_ascii_lowercase(),
                    r.description,
                    r.year,
                    r.manufacturer,
                    r.cloneof,
                ])
                .map_err(|e| format!("bake_mame_games insert {}: {e}", r.name))?;
            }
        }
        tx.commit()
            .map_err(|e| format!("bake_mame_games commit: {e}"))?;
        Ok(())
    }

    /// Read one L1 row by machine name. Lowercases the input to match
    /// the storage convention. Returns `Ok(None)` for names not in the
    /// catalog (homebrew, hacks, or ROMs whose machine post-dates the
    /// bundled slim — operators can re-bundle via the Phase 4 refresh).
    pub fn get_mame_game(
        &self,
        name: &str,
    ) -> Result<Option<crate::mame_games::MameGame>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let key = name.to_ascii_lowercase();
        conn.query_row(
            "SELECT name, description, year, manufacturer, cloneof FROM mame_games WHERE name = ?1",
            params![key],
            |row| {
                Ok(crate::mame_games::MameGame {
                    name: row.get(0)?,
                    description: row.get(1)?,
                    year: row.get(2)?,
                    manufacturer: row.get(3)?,
                    cloneof: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(|e| format!("get_mame_game {name}: {e}"))
    }

    /// Read the L3 override row for one machine. Returns `Ok(None)`
    /// when the operator hasn't edited that machine.
    pub fn get_mame_game_override(
        &self,
        name: &str,
    ) -> Result<Option<crate::mame_games::MameGameOverride>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let key = name.to_ascii_lowercase();
        conn.query_row(
            "SELECT name, description, year, manufacturer, cloneof FROM mame_games_overrides WHERE name = ?1",
            params![key],
            |row| {
                Ok(crate::mame_games::MameGameOverride {
                    name: row.get(0)?,
                    description: row.get(1)?,
                    year: row.get(2)?,
                    manufacturer: row.get(3)?,
                    cloneof: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(|e| format!("get_mame_game_override {name}: {e}"))
    }

    /// Merged L1 + L3 lookup. Returns `Ok(None)` when neither tier has
    /// the machine (caller falls through to the legacy `mame_titles`
    /// path and ultimately to the filename).
    pub fn lookup_merged_mame_game(
        &self,
        name: &str,
    ) -> Result<Option<crate::mame_games::MergedMameGame>, String> {
        let l1 = match self.get_mame_game(name)? {
            Some(l1) => l1,
            // Optimisation: L1 absent → L3 has nothing to override.
            // The operator-edit-only case (L3 present, no L1) is
            // intentionally unsupported; overrides are layered ON TOP
            // of a baseline, never standalone.
            None => return Ok(None),
        };
        let l3 = self.get_mame_game_override(name)?;
        Ok(Some(crate::mame_games::merge_mame_game(l1, l3)))
    }

    /// Upsert an L3 override row. Empty overrides (every field None)
    /// trigger a DELETE so the row doesn't waste storage and the
    /// `has_local_edits` flag reads false for that machine.
    pub fn upsert_mame_game_override(
        &self,
        override_record: &crate::mame_games::MameGameOverride,
    ) -> Result<(), String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let key = override_record.name.to_ascii_lowercase();
        if override_record.is_empty() {
            conn.execute(
                "DELETE FROM mame_games_overrides WHERE name = ?1",
                params![key],
            )
            .map_err(|e| format!("upsert_mame_game_override delete {key}: {e}"))?;
            return Ok(());
        }
        let now: i64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        conn.execute(
            r#"
            INSERT INTO mame_games_overrides
                (name, description, year, manufacturer, cloneof, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
            ON CONFLICT(name) DO UPDATE SET
                description  = excluded.description,
                year         = excluded.year,
                manufacturer = excluded.manufacturer,
                cloneof      = excluded.cloneof,
                updated_at   = excluded.updated_at
            "#,
            params![
                key,
                override_record.description,
                override_record.year,
                override_record.manufacturer,
                override_record.cloneof,
                now,
            ],
        )
        .map_err(|e| format!("upsert_mame_game_override {key}: {e}"))?;
        Ok(())
    }

    /// Delete the L3 override for one machine. Idempotent — a DELETE
    /// that matches no row is not an error.
    pub fn reset_mame_game_override(&self, name: &str) -> Result<(), String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let key = name.to_ascii_lowercase();
        conn.execute(
            "DELETE FROM mame_games_overrides WHERE name = ?1",
            params![key],
        )
        .map_err(|e| format!("reset_mame_game_override {key}: {e}"))?;
        Ok(())
    }

    /// Read the stored L1 content hash from `mame_games_meta`. None
    /// means "no row yet" (first launch, or table was just created).
    pub fn get_mame_games_meta_hash(&self) -> Result<Option<String>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let row = conn
            .query_row(
                "SELECT value FROM mame_games_meta WHERE key = 'l1_hash'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| format!("get_mame_games_meta_hash: {e}"))?;
        Ok(row)
    }

    /// Write (or replace) the stored L1 content hash.
    pub fn set_mame_games_meta_hash(&self, hash: &str) -> Result<(), String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        conn.execute(
            r#"
            INSERT INTO mame_games_meta (key, value) VALUES ('l1_hash', ?1)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value
            "#,
            params![hash],
        )
        .map_err(|e| format!("set_mame_games_meta_hash: {e}"))?;
        Ok(())
    }

    // --- Milestones CRUD (Phase 4 slice F) -------------------------------

    /// List every milestone configured for a game, in id order. Returns
    /// empty Vec when the game has none (the typical case until the
    /// operator adds some). Triggered milestones come back with
    /// `triggered_at_unix_ms` populated.
    pub fn list_milestones(&self, game_id: &str) -> Result<Vec<Milestone>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, game_id, name, description, region, offset, width, op, target, edge_only, triggered_at_unix_ms
                 FROM milestones WHERE game_id = ?1 ORDER BY id",
            )
            .map_err(|e| format!("prepare list_milestones: {e}"))?;
        let rows = stmt
            .query_map([game_id], |row| {
                Ok(Milestone {
                    id: Some(row.get::<_, i64>(0)?),
                    game_id: row.get(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    region: row.get(4)?,
                    offset: row.get::<_, i64>(5)? as u32,
                    width: row.get::<_, i64>(6)? as u8,
                    op: row.get(7)?,
                    target: row.get(8)?,
                    edge_only: row.get::<_, i64>(9)? != 0,
                    triggered_at_unix_ms: row.get::<_, Option<i64>>(10)?,
                })
            })
            .map_err(|e| format!("query_map list_milestones: {e}"))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| format!("row list_milestones: {e}"))?);
        }
        Ok(out)
    }

    /// Insert a milestone. Returns the rowid. Caller's `id` field is
    /// ignored — SQLite assigns one. `triggered_at_unix_ms` is forced
    /// to NULL on insert (fresh milestones haven't fired yet).
    pub fn add_milestone(&self, m: &Milestone) -> Result<i64, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        conn.execute(
            "INSERT INTO milestones (game_id, name, description, region, offset, width, op, target, edge_only)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                &m.game_id, &m.name, &m.description, &m.region,
                m.offset as i64, m.width as i64, &m.op, m.target,
                if m.edge_only { 1i64 } else { 0i64 },
            ],
        )
        .map_err(|e| format!("insert milestone: {e}"))?;
        Ok(conn.last_insert_rowid())
    }

    /// Update an existing milestone in place. `triggered_at_unix_ms`
    /// is intentionally NOT writeable here — use
    /// [`reset_milestone_progress`] or [`mark_milestone_triggered`].
    pub fn update_milestone(&self, m: &Milestone) -> Result<(), String> {
        let id = m.id.ok_or("update_milestone: missing id")?;
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let rows = conn
            .execute(
                "UPDATE milestones
                 SET name = ?1, description = ?2, region = ?3, offset = ?4,
                     width = ?5, op = ?6, target = ?7, edge_only = ?8
                 WHERE id = ?9",
                rusqlite::params![
                    &m.name, &m.description, &m.region, m.offset as i64,
                    m.width as i64, &m.op, m.target,
                    if m.edge_only { 1i64 } else { 0i64 },
                    id,
                ],
            )
            .map_err(|e| format!("update milestone: {e}"))?;
        if rows == 0 {
            return Err(format!("update_milestone: no row with id={id}"));
        }
        Ok(())
    }

    /// Remove a milestone. Returns the row-count actually deleted
    /// (0 if id didn't exist).
    pub fn delete_milestone(&self, id: i64) -> Result<usize, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        conn.execute("DELETE FROM milestones WHERE id = ?1", [id])
            .map_err(|e| format!("delete milestone: {e}"))
    }

    /// Stamp `triggered_at_unix_ms` (called by the emu thread when a
    /// rising-edge fires). No-op if id doesn't exist.
    pub fn mark_milestone_triggered(&self, id: i64, ts_ms: i64) -> Result<(), String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        conn.execute(
            "UPDATE milestones SET triggered_at_unix_ms = ?1 WHERE id = ?2 AND triggered_at_unix_ms IS NULL",
            rusqlite::params![ts_ms, id],
        )
        .map_err(|e| format!("mark milestone: {e}"))?;
        Ok(())
    }

    /// Reset progress — clear `triggered_at_unix_ms` so the predicate
    /// can re-fire.
    pub fn reset_milestone_progress(&self, id: i64) -> Result<(), String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        conn.execute(
            "UPDATE milestones SET triggered_at_unix_ms = NULL WHERE id = ?1",
            [id],
        )
        .map_err(|e| format!("reset milestone: {e}"))?;
        Ok(())
    }

    // --- Cheats CRUD (RetroArch parity slice 5) --------------------------

    pub fn list_cheats(&self, game_id: &str) -> Result<Vec<Cheat>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, game_id, name, description, region, offset, width, value, enabled, kind, code
                 FROM cheats WHERE game_id = ?1 ORDER BY id",
            )
            .map_err(|e| format!("prepare list_cheats: {e}"))?;
        let rows = stmt
            .query_map([game_id], |row| {
                Ok(Cheat {
                    id: Some(row.get::<_, i64>(0)?),
                    game_id: row.get(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    region: row.get(4)?,
                    offset: row.get::<_, i64>(5)? as u32,
                    width: row.get::<_, i64>(6)? as u8,
                    value: row.get(7)?,
                    enabled: row.get::<_, i64>(8)? != 0,
                    kind: row.get(9)?,
                    code: row.get::<_, Option<String>>(10)?,
                })
            })
            .map_err(|e| format!("query_map list_cheats: {e}"))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| format!("row list_cheats: {e}"))?);
        }
        Ok(out)
    }

    pub fn add_cheat(&self, c: &Cheat) -> Result<i64, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        conn.execute(
            "INSERT INTO cheats (game_id, name, description, region, offset, width, value, enabled, kind, code)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                &c.game_id, &c.name, &c.description, &c.region,
                c.offset as i64, c.width as i64, c.value,
                if c.enabled { 1i64 } else { 0i64 },
                &c.kind, &c.code,
            ],
        )
        .map_err(|e| format!("insert cheat: {e}"))?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_cheat(&self, c: &Cheat) -> Result<(), String> {
        let id = c.id.ok_or("update_cheat: missing id")?;
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let rows = conn
            .execute(
                "UPDATE cheats
                 SET name = ?1, description = ?2, region = ?3, offset = ?4,
                     width = ?5, value = ?6, enabled = ?7, kind = ?8, code = ?9
                 WHERE id = ?10",
                rusqlite::params![
                    &c.name, &c.description, &c.region, c.offset as i64,
                    c.width as i64, c.value,
                    if c.enabled { 1i64 } else { 0i64 },
                    &c.kind, &c.code,
                    id,
                ],
            )
            .map_err(|e| format!("update cheat: {e}"))?;
        if rows == 0 {
            return Err(format!("update_cheat: no row with id={id}"));
        }
        Ok(())
    }

    pub fn delete_cheat(&self, id: i64) -> Result<usize, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        conn.execute("DELETE FROM cheats WHERE id = ?1", [id])
            .map_err(|e| format!("delete cheat: {e}"))
    }

    // --- Folder + folder_rules CRUD --------------------------------------
    //
    // The `folders` and `folder_rules` tables shipped in schema v1 and v2
    // respectively but had no consumers until the Phase 2.7 Import wizard.
    // The wizard's commit step calls `add_folder` (or `update_folder` if the
    // path already exists), then `set_folder_rules` transactionally replaces
    // the rule set. `list_folders(true)` eager-loads rules so the wizard can
    // pre-populate its mapping editor when re-importing a known folder.

    /// List every tracked folder. When `include_rules` is true, each Folder
    /// arrives with its `rules` field populated (empty Vec if no rules);
    /// otherwise `rules` stays `None` and the caller queries rules per-folder
    /// via `list_folder_rules` when needed.
    pub fn list_folders(&self, include_rules: bool) -> Result<Vec<Folder>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let mut folders = Self::query_folders(&conn, None)?;
        if include_rules {
            // One bulk query, bucket by folder_id. Avoids N+1 on libraries
            // with many tracked folders.
            let mut stmt = conn
                .prepare(
                    "SELECT id, folder_id, match_pattern, system_id
                     FROM folder_rules
                     ORDER BY folder_id, id",
                )
                .map_err(|e| format!("prepare list folder_rules: {e}"))?;
            let rows = stmt
                .query_map([], |row| {
                    Ok(FolderRule {
                        id: Some(row.get::<_, i64>(0)?),
                        folder_id: row.get(1)?,
                        match_pattern: row.get(2)?,
                        system_id: row.get(3)?,
                    })
                })
                .map_err(|e| format!("query folder_rules: {e}"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("collect folder_rules: {e}"))?;
            for folder in &mut folders {
                let rules: Vec<FolderRule> = rows
                    .iter()
                    .filter(|r| r.folder_id == folder.id)
                    .cloned()
                    .collect();
                folder.rules = Some(rules);
            }
        }
        Ok(folders)
    }

    /// Look up a folder by absolute path. Wired for the wizard's "lookup
    /// before insert" path; today the frontend uses `list_folders(true)`
    /// + `.find` instead, so this is only exercised by the unit tests.
    #[allow(dead_code)]
    pub fn get_folder_by_path(
        &self,
        path: &str,
        include_rules: bool,
    ) -> Result<Option<Folder>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let mut folders = Self::query_folders(&conn, Some(path))?;
        let Some(mut folder) = folders.pop() else { return Ok(None) };
        if include_rules {
            folder.rules = Some(Self::query_rules_for(&conn, &folder.id)?);
        }
        Ok(Some(folder))
    }

    fn query_folders(conn: &Connection, by_path: Option<&str>) -> Result<Vec<Folder>, String> {
        let sql = "SELECT id, path, scan_subfolders, subfolders_are_systems,
                          watch_enabled, last_scanned_at
                   FROM folders";
        let map_row = |row: &rusqlite::Row<'_>| -> rusqlite::Result<Folder> {
            Ok(Folder {
                id: row.get(0)?,
                path: row.get(1)?,
                scan_subfolders: row.get::<_, i64>(2)? != 0,
                subfolders_are_systems: row.get::<_, i64>(3)? != 0,
                watch_enabled: row.get::<_, i64>(4)? != 0,
                last_scanned_at: row.get(5)?,
                rules: None,
            })
        };
        if let Some(p) = by_path {
            let mut stmt = conn
                .prepare(&format!("{sql} WHERE path = ?1"))
                .map_err(|e| format!("prepare folders by_path: {e}"))?;
            let rows = stmt
                .query_map([p], map_row)
                .map_err(|e| format!("query folders by_path: {e}"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("collect folders by_path: {e}"))?;
            Ok(rows)
        } else {
            // ORDER BY display_order, then rowid as a tiebreaker so equal
            // display_orders (which can happen when two adds race during
            // bulk migration) still produce a stable order.
            let mut stmt = conn
                .prepare(&format!("{sql} ORDER BY display_order, rowid"))
                .map_err(|e| format!("prepare folders: {e}"))?;
            let rows = stmt
                .query_map([], map_row)
                .map_err(|e| format!("query folders: {e}"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("collect folders: {e}"))?;
            Ok(rows)
        }
    }

    fn query_rules_for(conn: &Connection, folder_id: &str) -> Result<Vec<FolderRule>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT id, folder_id, match_pattern, system_id
                 FROM folder_rules
                 WHERE folder_id = ?1
                 ORDER BY id",
            )
            .map_err(|e| format!("prepare rules for folder: {e}"))?;
        let rows = stmt
            .query_map([folder_id], |row| {
                Ok(FolderRule {
                    id: Some(row.get::<_, i64>(0)?),
                    folder_id: row.get(1)?,
                    match_pattern: row.get(2)?,
                    system_id: row.get(3)?,
                })
            })
            .map_err(|e| format!("query rules for folder: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("collect rules for folder: {e}"))?;
        Ok(rows)
    }

    /// Insert a tracked folder. Errors if `path` already exists — callers
    /// should `get_folder_by_path` first and route to `update_folder` for
    /// edits. Returns the inserted Folder (with `rules: None`).
    pub fn add_folder(
        &self,
        path: &str,
        scan_subfolders: bool,
        subfolders_are_systems: bool,
        watch_enabled: bool,
    ) -> Result<Folder, String> {
        let id = folder_id_for_path(path);
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        // Append to the end of the user's current display order. COALESCE
        // handles the empty-table case where MAX() returns NULL.
        conn.execute(
            "INSERT INTO folders (id, path, scan_subfolders, subfolders_are_systems, watch_enabled, last_scanned_at, display_order)
             VALUES (?1, ?2, ?3, ?4, ?5, NULL, (SELECT COALESCE(MAX(display_order), 0) + 1 FROM folders))",
            params![
                id,
                path,
                if scan_subfolders { 1i64 } else { 0i64 },
                if subfolders_are_systems { 1i64 } else { 0i64 },
                if watch_enabled { 1i64 } else { 0i64 },
            ],
        )
        .map_err(|e| format!("insert folder: {e}"))?;
        Ok(Folder {
            id,
            path: path.to_string(),
            scan_subfolders,
            subfolders_are_systems,
            watch_enabled,
            last_scanned_at: None,
            rules: None,
        })
    }

    /// Bulk-update `display_order` for the given folder ids in one tx so the
    /// SettingsPage drag-reorder persists. Ids absent from `ordered_ids` are
    /// left untouched, but UI callers typically pass the full current set.
    pub fn reorder_folders(&self, ordered_ids: &[String]) -> Result<(), String> {
        let mut conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let tx = conn.transaction().map_err(|e| format!("begin reorder tx: {e}"))?;
        {
            let mut stmt = tx
                .prepare("UPDATE folders SET display_order = ?1 WHERE id = ?2")
                .map_err(|e| format!("prepare reorder update: {e}"))?;
            for (i, id) in ordered_ids.iter().enumerate() {
                stmt.execute(params![(i as i64) + 1, id])
                    .map_err(|e| format!("update display_order for {id}: {e}"))?;
            }
        }
        tx.commit().map_err(|e| format!("commit reorder tx: {e}"))?;
        Ok(())
    }

    /// One-shot import for folders that lived in the WebView's localStorage
    /// `oa.settings.v1.libraryFolders` array before the SQLite migration.
    /// For each `path` not already in `folders`, inserts a row with the
    /// quick-add defaults (scan subfolders, watch enabled, no rules — the
    /// frontend posts default rules in a follow-up `set_folder_rules`).
    /// Returns the count actually inserted (paths already in SQLite are
    /// silently skipped).
    pub fn migrate_folders_from_local_storage(
        &self,
        paths: &[String],
    ) -> Result<usize, String> {
        let mut conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let tx = conn.transaction().map_err(|e| format!("begin folder migration tx: {e}"))?;
        let mut inserted = 0usize;
        for path in paths {
            let id = folder_id_for_path(path);
            // Skip paths already tracked — keeps the migration idempotent
            // across launches if the frontend's "clear localStorage after
            // migrate" step ever races a crash.
            let exists: bool = tx
                .query_row(
                    "SELECT 1 FROM folders WHERE id = ?1",
                    params![id],
                    |_| Ok(true),
                )
                .optional()
                .map_err(|e| format!("check existing folder: {e}"))?
                .unwrap_or(false);
            if exists {
                continue;
            }
            tx.execute(
                "INSERT INTO folders (id, path, scan_subfolders, subfolders_are_systems, watch_enabled, last_scanned_at, display_order)
                 VALUES (?1, ?2, 1, 0, 1, NULL, (SELECT COALESCE(MAX(display_order), 0) + 1 FROM folders))",
                params![id, path],
            )
            .map_err(|e| format!("insert migrated folder {path}: {e}"))?;
            inserted += 1;
        }
        tx.commit().map_err(|e| format!("commit folder migration tx: {e}"))?;
        Ok(inserted)
    }

    /// Apply a partial update to a folder row. Fields left `None` in the
    /// payload are not touched. Returns `Err` if the folder id is unknown.
    pub fn update_folder(&self, id: &str, update: FolderUpdate) -> Result<(), String> {
        // Build a SET clause from the populated fields. rusqlite's named
        // params would clean this up, but the field count is small enough
        // that conditional WHEREs are cheaper than the macro footprint.
        let mut sets: Vec<&'static str> = Vec::new();
        let mut values: Vec<rusqlite::types::Value> = Vec::new();
        if let Some(v) = update.scan_subfolders {
            sets.push("scan_subfolders = ?");
            values.push(rusqlite::types::Value::Integer(if v { 1 } else { 0 }));
        }
        if let Some(v) = update.subfolders_are_systems {
            sets.push("subfolders_are_systems = ?");
            values.push(rusqlite::types::Value::Integer(if v { 1 } else { 0 }));
        }
        if let Some(v) = update.watch_enabled {
            sets.push("watch_enabled = ?");
            values.push(rusqlite::types::Value::Integer(if v { 1 } else { 0 }));
        }
        if let Some(v) = update.last_scanned_at {
            sets.push("last_scanned_at = ?");
            values.push(rusqlite::types::Value::Integer(v));
        }
        if sets.is_empty() {
            return Ok(());
        }
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let sql = format!("UPDATE folders SET {} WHERE id = ?", sets.join(", "));
        values.push(rusqlite::types::Value::Text(id.to_string()));
        let affected = conn
            .execute(&sql, rusqlite::params_from_iter(values.iter()))
            .map_err(|e| format!("update folder: {e}"))?;
        if affected == 0 {
            return Err(format!("unknown folder id: {id}"));
        }
        Ok(())
    }

    /// Drop a folder + cascade-delete its rules (FK ON DELETE CASCADE).
    pub fn remove_folder(&self, id: &str) -> Result<(), String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        conn.execute("DELETE FROM folders WHERE id = ?1", params![id])
            .map_err(|e| format!("delete folder: {e}"))?;
        Ok(())
    }

    /// Return every rule for the given folder, sorted by insertion order.
    pub fn list_folder_rules(&self, folder_id: &str) -> Result<Vec<FolderRule>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        Self::query_rules_for(&conn, folder_id)
    }

    /// Transactional replace: wipe every existing rule for `folder_id` and
    /// insert the supplied set. Returns the number of inserted rules.
    /// Rules' inbound `folder_id` field is ignored — the folder_id parameter
    /// is authoritative so a misconfigured client can't write to the wrong
    /// folder.
    pub fn set_folder_rules(
        &self,
        folder_id: &str,
        rules: &[FolderRule],
    ) -> Result<usize, String> {
        let mut conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let tx = conn.transaction().map_err(|e| format!("begin set_folder_rules tx: {e}"))?;
        let folder_exists: bool = tx
            .query_row(
                "SELECT 1 FROM folders WHERE id = ?1",
                params![folder_id],
                |_| Ok(true),
            )
            .optional()
            .map_err(|e| format!("check folder exists: {e}"))?
            .unwrap_or(false);
        if !folder_exists {
            return Err(format!("unknown folder id: {folder_id}"));
        }
        tx.execute("DELETE FROM folder_rules WHERE folder_id = ?1", params![folder_id])
            .map_err(|e| format!("clear folder_rules: {e}"))?;
        let mut inserted = 0usize;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO folder_rules (folder_id, match_pattern, system_id)
                     VALUES (?1, ?2, ?3)",
                )
                .map_err(|e| format!("prepare insert rule: {e}"))?;
            for rule in rules {
                stmt.execute(params![folder_id, rule.match_pattern, rule.system_id])
                    .map_err(|e| format!("insert rule: {e}"))?;
                inserted += 1;
            }
        }
        tx.commit().map_err(|e| format!("commit set_folder_rules: {e}"))?;
        Ok(inserted)
    }

    /// One-shot migration entry point — called once on first launch after the
    /// SQLite upgrade. Caller is expected to clear localStorage[oa.library.v1]
    /// on success so we don't migrate twice. Idempotent (uses INSERT OR IGNORE)
    /// so re-running it is harmless.
    pub fn migrate_from_local_storage(&self, entries: &[GameRow]) -> Result<usize, String> {
        if entries.is_empty() {
            return Ok(0);
        }
        let added = self.add_games(entries)?;
        log::info!(
            "library_db: migrated {} entries from localStorage ({} new, {} already present)",
            entries.len(),
            added,
            entries.len() - added,
        );
        Ok(added)
    }

    /// Look up cover_path for a single game. Used by the launch path which
    /// previously read coverPath from the localStorage RomEntry — keep that
    /// column populated so we can hydrate it on launch without round-tripping
    /// through the MediaDb.
    #[allow(dead_code)] // wired into launch flow alongside the per-game shader work
    pub fn get_cover_path(&self, id: &str) -> Result<Option<String>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        conn.query_row("SELECT cover_path FROM games WHERE id = ?1", params![id], |row| {
            row.get(0)
        })
        .optional()
        .map_err(|e| format!("get_cover_path: {e}"))
    }

    // --- Custom collections (Phase C3 Slice 12) --------------------------
    //
    // Operator-built lists alongside the smart-list COLLECTIONS shipped
    // in Slice 11. The two-table shape (parent + junction) keeps the
    // membership set independent from the game row so a future drag-
    // reorder lands cleanly on `sort_order` without rewriting the
    // collection row. `list_custom_collections` joins for the live
    // count so stale memberships from deleted games don't inflate the
    // sidebar badge.

    /// All collections, ordered by `sort_order` then created_at. Each
    /// row carries the live member count via a LEFT JOIN against
    /// `games` so a member row whose rom was deleted between sessions
    /// doesn't contribute. The empty case returns an empty Vec.
    pub fn list_custom_collections(&self) -> Result<Vec<CustomCollectionRow>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT cc.id, cc.name, cc.sort_order, cc.created_at, cc.updated_at,
                        COUNT(g.id) AS member_count
                 FROM custom_collections cc
                 LEFT JOIN custom_collection_members ccm
                     ON ccm.collection_id = cc.id
                 LEFT JOIN games g ON g.id = ccm.rom_id
                 GROUP BY cc.id
                 ORDER BY cc.sort_order ASC, cc.created_at ASC",
            )
            .map_err(|e| format!("prepare list_custom_collections: {e}"))?;
        let rows = stmt
            .query_map([], |row| {
                Ok(CustomCollectionRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    sort_order: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                    member_count: row.get(5)?,
                })
            })
            .map_err(|e| format!("query list_custom_collections: {e}"))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| format!("row list_custom_collections: {e}"))?);
        }
        Ok(out)
    }

    /// Create a new collection with the given display name. The id is
    /// generated server-side (Slice 11 pattern uses lowercase nanoid-
    /// style hex; here we use the existing `Self::generate_id` shape).
    /// Returns the new id so the frontend can attach members
    /// immediately. `sort_order` defaults to the max+1 so freshly
    /// created lists land at the bottom of the sidebar; the operator
    /// can drag-reorder in a follow-up slice.
    pub fn create_custom_collection(&self, name: &str) -> Result<String, String> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err("collection name cannot be empty".to_string());
        }
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let next_sort: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM custom_collections",
                [],
                |row| row.get(0),
            )
            .unwrap_or(1);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let id = format!("col-{:x}-{:x}", now, next_sort);
        conn.execute(
            "INSERT INTO custom_collections (id, name, sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)",
            params![id, trimmed, next_sort, now],
        )
        .map_err(|e| format!("insert custom_collections: {e}"))?;
        Ok(id)
    }

    /// Rename an existing collection. Empty name is rejected. Bumps
    /// `updated_at` so a future "Recently edited" surface can sort on
    /// it without a join.
    pub fn rename_custom_collection(&self, id: &str, name: &str) -> Result<(), String> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err("collection name cannot be empty".to_string());
        }
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let n = conn
            .execute(
                "UPDATE custom_collections
                 SET name = ?1, updated_at = ?2
                 WHERE id = ?3",
                params![trimmed, now, id],
            )
            .map_err(|e| format!("update custom_collections name: {e}"))?;
        if n == 0 {
            return Err(format!("collection {id} not found"));
        }
        Ok(())
    }

    /// Delete a collection. ON DELETE CASCADE on the FK in
    /// `custom_collection_members` cleans up the junction rows in the
    /// same SQLite statement (no orphan sweep needed).
    pub fn delete_custom_collection(&self, id: &str) -> Result<(), String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        // SQLite FKs are off by default; enable on this connection so
        // the CASCADE on custom_collection_members fires. Cheap to
        // re-set on each call.
        conn.execute_batch("PRAGMA foreign_keys = ON")
            .map_err(|e| format!("enable foreign_keys: {e}"))?;
        let n = conn
            .execute("DELETE FROM custom_collections WHERE id = ?1", params![id])
            .map_err(|e| format!("delete custom_collections: {e}"))?;
        if n == 0 {
            return Err(format!("collection {id} not found"));
        }
        Ok(())
    }

    /// Add a rom to a collection. INSERT OR IGNORE keeps the call
    /// idempotent — toggling "Add to X" twice doesn't error. `sort_order`
    /// places new members at the bottom of the collection (max+1) so
    /// add order is preserved by default; future drag-reorder lands on
    /// the same column.
    pub fn add_to_custom_collection(
        &self,
        collection_id: &str,
        rom_id: &str,
    ) -> Result<(), String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let next_sort: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), 0) + 1
                 FROM custom_collection_members
                 WHERE collection_id = ?1",
                params![collection_id],
                |row| row.get(0),
            )
            .unwrap_or(1);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        conn.execute(
            "INSERT OR IGNORE INTO custom_collection_members
                (collection_id, rom_id, sort_order, added_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![collection_id, rom_id, next_sort, now],
        )
        .map_err(|e| format!("insert custom_collection_members: {e}"))?;
        // Bump the parent's updated_at so a future "recently edited"
        // sort sees the membership change.
        conn.execute(
            "UPDATE custom_collections SET updated_at = ?1 WHERE id = ?2",
            params![now, collection_id],
        )
        .map_err(|e| format!("touch custom_collections updated_at: {e}"))?;
        Ok(())
    }

    /// Remove a rom from a collection. Silent if the membership didn't
    /// exist — the operator's expectation is "this game is no longer
    /// in X," not "the membership row was deleted."
    pub fn remove_from_custom_collection(
        &self,
        collection_id: &str,
        rom_id: &str,
    ) -> Result<(), String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        conn.execute(
            "DELETE FROM custom_collection_members
             WHERE collection_id = ?1 AND rom_id = ?2",
            params![collection_id, rom_id],
        )
        .map_err(|e| format!("delete custom_collection_members: {e}"))?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        conn.execute(
            "UPDATE custom_collections SET updated_at = ?1 WHERE id = ?2",
            params![now, collection_id],
        )
        .map_err(|e| format!("touch custom_collections updated_at: {e}"))?;
        Ok(())
    }

    /// List the rom ids that belong to a collection, sorted by
    /// `sort_order` then `added_at`. Stale memberships pointing at
    /// deleted games are filtered via an EXISTS check so the frontend
    /// never has to defensively skip missing rows.
    pub fn list_collection_members(&self, collection_id: &str) -> Result<Vec<String>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT ccm.rom_id
                 FROM custom_collection_members ccm
                 INNER JOIN games g ON g.id = ccm.rom_id
                 WHERE ccm.collection_id = ?1
                 ORDER BY ccm.sort_order ASC, ccm.added_at ASC",
            )
            .map_err(|e| format!("prepare list_collection_members: {e}"))?;
        let rows = stmt
            .query_map(params![collection_id], |row| row.get::<_, String>(0))
            .map_err(|e| format!("query list_collection_members: {e}"))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| format!("row list_collection_members: {e}"))?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_db() -> LibraryDb {
        let tmp = std::env::temp_dir().join(format!(
            "oa-library-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        LibraryDb::open(&tmp).expect("open fresh db")
    }

    fn row(id: &str, title: &str) -> GameRow {
        GameRow {
            id: id.to_string(),
            title: title.to_string(),
            system_id: "tg16".to_string(),
            file_path: format!("/roms/{id}.pce"),
            added_at: 0,
            cover_path: None,
            core_override: None,
            seed: false,
            archive_inner_path: None,
            sha1: None,
            serial: None,
            disc_id: None,
            favorite: false,
            completed: false,
            last_played_at: None,
            play_time_secs: 0,
            players: None,
            rating: None,
            disc_set_id: None,
            disc_number: None,
        }
    }

    #[test]
    fn opens_and_lists_empty() {
        let db = fresh_db();
        let games = db.list_games().expect("list");
        assert_eq!(games.len(), 0);
        assert_eq!(db.count().expect("count"), 0);
    }

    #[test]
    fn controller_info_cache_round_trip() {
        // Fresh DB starts with no cached info → cached_controller_devices
        // returns Ok(None) regardless of mtime.
        let db = fresh_db();
        assert_eq!(
            db.cached_controller_devices("fceumm_libretro.dll", 1, 1700000000)
                .expect("query"),
            None,
        );

        // Persist a FCEUmm-shaped advertisement: port 0 + port 1 carry
        // the gamepad + zapper + arkanoid + power pad options; ports 2-4
        // empty (matches FCEUmm's update_nes_controllers port-bucketing).
        let zapper = oa_core::ControllerDeviceDescriptor {
            label: "Zapper".into(),
            id: 258,
        };
        let pad = oa_core::ControllerDeviceDescriptor {
            label: "RetroPad".into(),
            id: 1,
        };
        let per_port: [Vec<oa_core::ControllerDeviceDescriptor>; 5] = [
            vec![pad.clone(), zapper.clone()],
            vec![pad.clone(), zapper.clone()],
            vec![],
            vec![],
            vec![],
        ];
        db.upsert_controller_info("fceumm_libretro.dll", &per_port, 1700000000)
            .expect("upsert");

        // Matching mtime → returns the cached vec.
        let got = db
            .cached_controller_devices("fceumm_libretro.dll", 1, 1700000000)
            .expect("query")
            .expect("Some(devices) for matching mtime");
        assert_eq!(got.len(), 2);
        assert_eq!(got[1].id, 258);
        assert_eq!(got[1].label, "Zapper");

        // Empty per-port persists faithfully — port 3 round-trips as [].
        let port3 = db
            .cached_controller_devices("fceumm_libretro.dll", 3, 1700000000)
            .expect("query")
            .expect("Some([]) for empty port");
        assert!(port3.is_empty(), "empty per-port list should round-trip as Some(vec![])");

        // Mismatched mtime → treats cache as stale and returns None
        // (caller falls back to "Launch the game once" hint).
        assert_eq!(
            db.cached_controller_devices("fceumm_libretro.dll", 1, 1700099999)
                .expect("query stale"),
            None,
            "mtime mismatch must invalidate cache",
        );

        // Re-upsert with new mtime overwrites the row + restores reads.
        let per_port2 = [
            vec![pad.clone()],
            vec![],
            vec![],
            vec![],
            vec![],
        ];
        db.upsert_controller_info("fceumm_libretro.dll", &per_port2, 1700099999)
            .expect("re-upsert");
        let after = db
            .cached_controller_devices("fceumm_libretro.dll", 0, 1700099999)
            .expect("query after re-upsert")
            .expect("Some(devices) after re-upsert");
        assert_eq!(after.len(), 1, "re-upsert should replace, not append");
        assert_eq!(after[0].id, 1);
    }

    #[test]
    fn controller_info_cache_keyed_by_core_filename() {
        // Two different cores' caches should not collide.
        let db = fresh_db();
        let fceumm = [
            vec![oa_core::ControllerDeviceDescriptor {
                label: "Zapper".into(),
                id: 258,
            }],
            vec![],
            vec![],
            vec![],
            vec![],
        ];
        let snes9x = [
            vec![oa_core::ControllerDeviceDescriptor {
                label: "Super Scope".into(),
                id: 260,
            }],
            vec![],
            vec![],
            vec![],
            vec![],
        ];
        db.upsert_controller_info("fceumm_libretro.dll", &fceumm, 100)
            .expect("upsert fceumm");
        db.upsert_controller_info("snes9x_libretro.dll", &snes9x, 200)
            .expect("upsert snes9x");

        let got_nes = db
            .cached_controller_devices("fceumm_libretro.dll", 0, 100)
            .expect("query nes")
            .expect("nes Some");
        assert_eq!(got_nes[0].label, "Zapper");

        let got_snes = db
            .cached_controller_devices("snes9x_libretro.dll", 0, 200)
            .expect("query snes")
            .expect("snes Some");
        assert_eq!(got_snes[0].label, "Super Scope");
    }

    #[test]
    fn add_dedup_by_file_path() {
        let db = fresh_db();
        let a = db.add_games(&[row("a", "Alpha"), row("b", "Bravo")]).expect("add 1");
        assert_eq!(a, 2);
        // Second add of same file_path is ignored.
        let mut c = row("c", "Charlie");
        c.file_path = "/roms/a.pce".to_string();
        let b = db.add_games(&[c]).expect("add 2");
        assert_eq!(b, 0);
        assert_eq!(db.count().expect("count"), 2);
    }

    #[test]
    fn search_finds_by_prefix() {
        let db = fresh_db();
        db.add_games(&[
            row("a", "Bonk's Adventure"),
            row("b", "Blazing Lazers"),
            row("c", "Splatterhouse"),
        ])
        .expect("seed");
        let hits = db.search_games("bonk", 10).expect("search bonk");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "a");
        let hits = db.search_games("bl", 10).expect("search bl");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "b");
        let hits = db.search_games("nonexistent_word", 10).expect("search miss");
        assert_eq!(hits.len(), 0);
    }

    #[test]
    fn find_id_by_file_path_returns_match_or_none() {
        let db = fresh_db();
        let mut a = row("a", "Alpha");
        a.file_path = "/roms/alpha.pce".into();
        db.add_games(&[a]).expect("seed");
        assert_eq!(
            db.find_id_by_file_path("/roms/alpha.pce").expect("hit"),
            Some("a".to_string())
        );
        assert_eq!(db.find_id_by_file_path("/roms/missing.pce").expect("miss"), None);
    }

    #[test]
    fn delete_games_for_system_removes_only_that_system() {
        let db = fresh_db();
        let mut a = row("a", "Alpha");
        a.system_id = "tg16".into();
        let mut b = row("b", "Bravo");
        b.system_id = "nes".into();
        b.file_path = "/roms/b.nes".into();
        let mut c = row("c", "Charlie");
        c.system_id = "nes".into();
        c.file_path = "/roms/c.nes".into();
        db.add_games(&[a, b, c]).expect("seed");

        let removed = db.delete_games_for_system("nes").expect("bulk delete");
        assert_eq!(removed, 2);
        let remaining = db.list_games().expect("list");
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, "a");

        // Idempotent — second call removes nothing.
        let removed = db.delete_games_for_system("nes").expect("bulk delete 2");
        assert_eq!(removed, 0);
    }

    #[test]
    fn delete_all_games_resets_library() {
        let db = fresh_db();
        db.add_games(&[row("a", "Alpha"), row("b", "Bravo"), row("c", "Charlie")])
            .expect("seed");
        assert_eq!(db.count().expect("count"), 3);
        let removed = db.delete_all_games().expect("reset");
        assert_eq!(removed, 3);
        assert_eq!(db.count().expect("count post-reset"), 0);
        // Idempotent — second call is a 0-row no-op, not an error.
        let removed = db.delete_all_games().expect("reset again");
        assert_eq!(removed, 0);
    }

    #[test]
    fn search_empty_returns_all() {
        let db = fresh_db();
        db.add_games(&[row("a", "Alpha"), row("b", "Bravo")]).expect("seed");
        let all = db.search_games("", 10).expect("search empty");
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn update_and_delete() {
        let db = fresh_db();
        db.add_games(&[row("a", "Alpha")]).expect("seed");
        db.update_core_override("a", Some("custom.dll")).expect("update");
        let games = db.list_games().expect("list");
        assert_eq!(games[0].core_override, Some("custom.dll".to_string()));
        db.update_core_override("a", None).expect("clear");
        let games = db.list_games().expect("list 2");
        assert_eq!(games[0].core_override, None);
        db.delete_game("a").expect("delete");
        assert_eq!(db.count().expect("count"), 0);
    }

    #[test]
    fn drop_seed_rows_only_removes_seeds() {
        let db = fresh_db();
        let mut s = row("seed", "Seed");
        s.seed = true;
        db.add_games(&[s, row("real", "Real")]).expect("seed");
        assert_eq!(db.count().expect("count"), 2);
        let removed = db.drop_seed_rows().expect("drop");
        assert_eq!(removed, 1);
        let remaining = db.list_games().expect("list");
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, "real");
    }

    #[test]
    fn cheats_crud_roundtrip() {
        let db = fresh_db();
        db.add_games(&[row("a", "Alpha")]).expect("seed");
        // List on a game with no cheats → empty.
        assert!(db.list_cheats("a").expect("empty").is_empty());

        // Add three cheats.
        let mut c1 = Cheat {
            id: None,
            game_id: "a".into(),
            name: "Infinite lives".into(),
            description: String::new(),
            region: "system_ram".into(),
            offset: 0x1F22,
            width: 1,
            value: 9,
            enabled: true,
            kind: "memory_poke".into(),
            code: None,
        };
        let id1 = db.add_cheat(&c1).expect("add 1");
        c1.id = Some(id1);
        let id2 = db.add_cheat(&Cheat {
            id: None,
            game_id: "a".into(),
            name: "Max score".into(),
            description: "Sets score to 999999 every frame".into(),
            region: "system_ram".into(),
            offset: 0x2000,
            width: 4,
            value: 999999,
            enabled: false,
            kind: "memory_poke".into(),
            code: None,
        }).expect("add 2");
        // Libretro-format Game Genie code — region / offset / width / value
        // unused but the schema requires them; default to harmless values.
        db.add_cheat(&Cheat {
            id: None,
            game_id: "a".into(),
            name: "Infinite health (GG)".into(),
            description: "Castlevania Game Genie code".into(),
            region: "system_ram".into(),
            offset: 0,
            width: 1,
            value: 0,
            enabled: true,
            kind: "libretro_code".into(),
            code: Some("SXIOPO".into()),
        }).expect("add libretro");

        let listed = db.list_cheats("a").expect("list");
        assert_eq!(listed.len(), 3);
        let gg = listed.iter().find(|c| c.kind == "libretro_code").expect("gg row");
        assert_eq!(gg.code.as_deref(), Some("SXIOPO"));

        // Update the first cheat's value + disable.
        c1.value = 5;
        c1.enabled = false;
        db.update_cheat(&c1).expect("update");
        let after = db.list_cheats("a").expect("list after");
        let updated = after.iter().find(|c| c.id == Some(id1)).unwrap();
        assert_eq!(updated.value, 5);
        assert!(!updated.enabled);

        // Delete the second; first + the libretro one still present.
        assert_eq!(db.delete_cheat(id2).expect("delete"), 1);
        assert_eq!(db.list_cheats("a").expect("after delete").len(), 2);

        // FK cascade — deleting the game also drops its cheats.
        db.delete_game("a").expect("delete game");
        assert!(db.list_cheats("a").expect("post-cascade").is_empty());
    }

    #[test]
    fn migrate_from_local_storage_is_idempotent() {
        let db = fresh_db();
        let entries = vec![row("a", "Alpha"), row("b", "Bravo")];
        let n1 = db.migrate_from_local_storage(&entries).expect("first");
        assert_eq!(n1, 2);
        // Second call returns 0 — same file_paths, INSERT OR IGNORE skips them.
        let n2 = db.migrate_from_local_storage(&entries).expect("second");
        assert_eq!(n2, 0);
        assert_eq!(db.count().expect("count"), 2);
    }

    #[test]
    fn archive_inner_path_round_trips() {
        let db = fresh_db();
        let mut a = row("zip-bonk", "Bonk's Adventure");
        a.file_path = "/roms/games.zip".to_string();
        a.archive_inner_path = Some("Bonk's Adventure (USA).pce".to_string());
        let mut b = row("zip-blazing", "Blazing Lazers");
        b.file_path = "/roms/games.zip".to_string();
        // Same archive on disk, different inner — file_path must differ so the
        // UNIQUE constraint on file_path doesn't reject the second insert.
        // The convention the scanner uses is "<archive>#<inner>" for the
        // file_path so each inner entry is unique.
        b.file_path = "/roms/games.zip#blazing.pce".to_string();
        b.archive_inner_path = Some("blazing.pce".to_string());

        assert_eq!(db.add_games(&[a, b]).expect("add"), 2);
        let games = db.list_games().expect("list");
        assert_eq!(games.len(), 2);
        for g in &games {
            assert!(g.archive_inner_path.is_some(), "all entries are archived");
        }
    }

    #[test]
    fn v4_to_v5_retags_cd_games_to_pce_cd() {
        let db = fresh_db();
        // Three tg16 carts (.pce — must stay tg16), a bare-CHD CD image, a
        // CUE+BIN, an archived CD image where the outer file is a .zip but
        // the inner extension is .cue (the launch path keys off the inner
        // extension, so the migration must too), and a stray tg16 row whose
        // .pce filename happens to live next to "cue" in its path — make
        // sure the GLOB anchor on the *.ext suffix isn't fooled.
        let mut cart_a = row("cart-a", "Bonk");
        cart_a.file_path = "/roms/tg16/Bonk.pce".into();
        let mut cd_chd = row("cd-chd", "Rondo of Blood");
        cd_chd.file_path = "/roms/tg-cd/Rondo of Blood.chd".into();
        let mut cd_cue = row("cd-cue", "Ys IV");
        cd_cue.file_path = "/roms/tg-cd/Ys IV.cue".into();
        let mut cd_in_zip = row("cd-zip", "Lords of Thunder");
        cd_in_zip.file_path = "/roms/Lords of Thunder.zip#disc.cue".into();
        cd_in_zip.archive_inner_path = Some("disc.cue".into());
        let mut tricky = row("tricky", "Cue Sports");
        tricky.file_path = "/roms/tg16-cue-folder/Cue Sports.pce".into();

        db.add_games(&[cart_a, cd_chd, cd_cue, cd_in_zip, tricky]).expect("seed");
        // Rewind user_version to v4 and re-run the schema bootstrap — that's
        // what would happen if a v4 DB met this build for the first time.
        let rewind_and_rebootstrap = || {
            let guard = db.inner.lock().expect("lock");
            guard
                .pragma_update(None, "user_version", 4)
                .expect("rewind to v4");
            LibraryDb::bootstrap_schema(&guard).expect("re-bootstrap");
        };
        rewind_and_rebootstrap();

        let by_id = |gid: &str| -> String {
            db.list_games()
                .expect("list")
                .into_iter()
                .find(|g| g.id == gid)
                .expect("row present")
                .system_id
        };
        assert_eq!(by_id("cart-a"), "tg16", "cart must stay tg16");
        assert_eq!(by_id("cd-chd"), "pce-cd", "bare CHD retagged");
        assert_eq!(by_id("cd-cue"), "pce-cd", "CUE retagged");
        assert_eq!(by_id("cd-zip"), "pce-cd", "archived inner-.cue retagged");
        assert_eq!(by_id("tricky"), "tg16", "outer .pce not fooled by 'cue' substring in path");

        // Idempotent — second run leaves things alone.
        rewind_and_rebootstrap();
        assert_eq!(by_id("cd-chd"), "pce-cd");
        assert_eq!(by_id("cart-a"), "tg16");
    }

    fn rule(folder_id: &str, pattern: &str, system: &str) -> FolderRule {
        FolderRule {
            id: None,
            folder_id: folder_id.to_string(),
            match_pattern: pattern.to_string(),
            system_id: system.to_string(),
        }
    }

    #[test]
    fn folders_crud_roundtrip() {
        let db = fresh_db();
        assert!(db.list_folders(false).expect("empty list").is_empty());

        let f = db
            .add_folder("/roms/tg16", true, false, true)
            .expect("add folder");
        assert!(f.id.starts_with("folder-"));
        assert_eq!(f.path, "/roms/tg16");
        assert!(f.scan_subfolders);
        assert!(!f.subfolders_are_systems);
        assert!(f.watch_enabled);
        assert!(f.last_scanned_at.is_none());

        // Stable id: same path produces the same id, so re-add (without first
        // removing) should error on UNIQUE — but `get_folder_by_path` finds it.
        let dup = db.add_folder("/roms/tg16", true, false, true);
        assert!(dup.is_err(), "duplicate path must error");

        let found = db
            .get_folder_by_path("/roms/tg16", false)
            .expect("get")
            .expect("present");
        assert_eq!(found.id, f.id);
        assert!(found.rules.is_none(), "include_rules=false leaves rules None");

        // Partial update — flip subfolders_are_systems, bump last_scanned_at.
        db.update_folder(
            &f.id,
            FolderUpdate {
                subfolders_are_systems: Some(true),
                last_scanned_at: Some(12345),
                ..Default::default()
            },
        )
        .expect("update");
        let after = db
            .get_folder_by_path("/roms/tg16", false)
            .expect("get")
            .expect("present");
        assert!(after.subfolders_are_systems);
        assert_eq!(after.last_scanned_at, Some(12345));
        assert!(after.scan_subfolders, "scan_subfolders untouched");
        assert!(after.watch_enabled, "watch_enabled untouched");

        // Update unknown id surfaces a clean error.
        let err = db
            .update_folder(
                "folder-nope",
                FolderUpdate {
                    watch_enabled: Some(false),
                    ..Default::default()
                },
            )
            .expect_err("unknown id errors");
        assert!(err.contains("unknown folder id"));

        db.remove_folder(&f.id).expect("remove");
        assert!(db.list_folders(false).expect("post-remove list").is_empty());
    }

    #[test]
    fn folder_rules_replace_and_cascade() {
        let db = fresh_db();
        let f = db
            .add_folder("/roms/mixed", true, false, false)
            .expect("add");

        // Seed three rules.
        let n = db
            .set_folder_rules(
                &f.id,
                &[
                    rule(&f.id, "*.pce", "tg16"),
                    rule(&f.id, "*.cue", "tg16"),
                    rule(&f.id, "*.chd", "tg16"),
                ],
            )
            .expect("set initial");
        assert_eq!(n, 3);
        let listed = db.list_folder_rules(&f.id).expect("list rules");
        assert_eq!(listed.len(), 3);
        assert!(listed.iter().all(|r| r.id.is_some()));

        // Replace with two different rules. Existing three must be wiped.
        let n2 = db
            .set_folder_rules(
                &f.id,
                &[
                    rule(&f.id, "*.sgx", "tg16"),
                    rule(&f.id, "*.m3u", "tg16"),
                ],
            )
            .expect("set replace");
        assert_eq!(n2, 2);
        let after = db.list_folder_rules(&f.id).expect("list after replace");
        assert_eq!(after.len(), 2);
        assert!(after.iter().any(|r| r.match_pattern == "*.sgx"));
        assert!(after.iter().any(|r| r.match_pattern == "*.m3u"));
        assert!(after.iter().all(|r| r.match_pattern != "*.pce"));

        // Eager-load via list_folders.
        let eager = db.list_folders(true).expect("list eager");
        assert_eq!(eager.len(), 1);
        let rules = eager[0].rules.as_ref().expect("eager rules");
        assert_eq!(rules.len(), 2);

        // Cascade: removing the folder must drop its rules.
        db.remove_folder(&f.id).expect("remove folder");
        let orphan = db.list_folder_rules(&f.id).expect("list after delete");
        assert_eq!(orphan.len(), 0, "FK ON DELETE CASCADE drops rules");

        // set_folder_rules on a vanished folder returns a clean error.
        let err = db
            .set_folder_rules(&f.id, &[rule(&f.id, "*.pce", "tg16")])
            .expect_err("set on missing folder errors");
        assert!(err.contains("unknown folder id"));
    }

    #[test]
    fn folders_display_order_persists_and_reorders() {
        let db = fresh_db();
        let a = db.add_folder("/roms/aaa", true, false, true).expect("a");
        let b = db.add_folder("/roms/bbb", true, false, true).expect("b");
        let c = db.add_folder("/roms/ccc", true, false, true).expect("c");

        // Insertion order is the default display order.
        let listed: Vec<String> = db
            .list_folders(false)
            .expect("list")
            .into_iter()
            .map(|f| f.path)
            .collect();
        assert_eq!(listed, vec!["/roms/aaa", "/roms/bbb", "/roms/ccc"]);

        // Reorder C, A, B — drag-drop in the Settings UI.
        db.reorder_folders(&[c.id.clone(), a.id.clone(), b.id.clone()])
            .expect("reorder");
        let after: Vec<String> = db
            .list_folders(false)
            .expect("list after reorder")
            .into_iter()
            .map(|f| f.path)
            .collect();
        assert_eq!(after, vec!["/roms/ccc", "/roms/aaa", "/roms/bbb"]);

        // Adding a new folder appends to the end of the user's order.
        let d = db.add_folder("/roms/ddd", true, false, true).expect("d");
        let _ = d;
        let after_add: Vec<String> = db
            .list_folders(false)
            .expect("list after add")
            .into_iter()
            .map(|f| f.path)
            .collect();
        assert_eq!(
            after_add,
            vec!["/roms/ccc", "/roms/aaa", "/roms/bbb", "/roms/ddd"]
        );
    }

    #[test]
    fn migrate_folders_from_local_storage_idempotent() {
        let db = fresh_db();
        // Pre-existing folder via the wizard path — migration should leave
        // it alone, not duplicate.
        db.add_folder("/roms/pre", true, false, true).expect("seed");

        let n = db
            .migrate_folders_from_local_storage(&[
                "/roms/pre".to_string(),
                "/roms/new1".to_string(),
                "/roms/new2".to_string(),
            ])
            .expect("migrate");
        assert_eq!(n, 2, "only the two new paths were inserted");

        let after: Vec<String> = db
            .list_folders(false)
            .expect("list")
            .into_iter()
            .map(|f| f.path)
            .collect();
        assert_eq!(after, vec!["/roms/pre", "/roms/new1", "/roms/new2"]);

        // Re-running with the same set is a no-op.
        let n2 = db
            .migrate_folders_from_local_storage(&[
                "/roms/pre".to_string(),
                "/roms/new1".to_string(),
                "/roms/new2".to_string(),
            ])
            .expect("migrate again");
        assert_eq!(n2, 0);
    }

    #[test]
    fn game_overrides_round_trip_and_clear() {
        let db = fresh_db();
        db.add_games(&[row("a", "Alpha")]).expect("seed");
        // No overrides set yet → default struct.
        let initial = db.get_game_overrides("a").expect("get empty");
        assert_eq!(initial, GameOverrides::default());
        // Set some overrides.
        let pref = GameOverrides {
            scaling_override: Some("pixel-perfect".to_string()),
            window_mode_override: None,
            monitor_index_override: Some(1),
            region_override: Some("japan".to_string()),
            shader_preset: Some("crt-lite".to_string()),
            bloom_amount: Some(0.45),
            core_options: std::collections::HashMap::new(),
            patch_path: None,
            rewind_enabled: Some(true),
            rewind_capture_interval_frames: Some(3),
            rewind_buffer_megabytes: Some(48),
            display_aspect_override: Some(1.333),
            overscan_crop_override: Some(crate::system_settings::OverscanCropPrefs {
                top: 8, bottom: 8, left: 0, right: 0,
            }),
            bezel_image_path: Some("C:/bezels/arcade.png".to_string()),
            analog_routing: Some(crate::system_settings::AnalogRoutingPrefs {
                ports: vec![crate::system_settings::AnalogPortRouting {
                    left: crate::system_settings::AnalogStickPrefs {
                        deadzone: 0.15,
                        ..crate::system_settings::AnalogStickPrefs::default()
                    },
                    right: crate::system_settings::AnalogStickPrefs::default_right(),
                    stick_swap: false,
                }],
            }),
            keypad_layout_note: Some(
                "Donkey Kong: KP1=climb-up, KP2=climb-down, KP3=jump".to_string(),
            ),
            libretro_device: Some(2), // RETRO_DEVICE_MOUSE
            libretro_device_port1: Some(1), // Phase E — JOYPAD on port 1 alongside MOUSE on port 0
            libretro_device_port2: None,
            libretro_device_port3: None,
            libretro_device_port4: None,
            platform_music_path: None,
            dosbox_entry_point: Some("INSTALL.EXE".to_string()),
        };
        db.set_game_overrides("a", &pref).expect("set");
        let after = db.get_game_overrides("a").expect("get after");
        assert_eq!(after, pref);
        // Clear (all None) writes NULL — round-trips back as default.
        db.set_game_overrides("a", &GameOverrides::default()).expect("clear");
        let cleared = db.get_game_overrides("a").expect("get cleared");
        assert_eq!(cleared, GameOverrides::default());
        // Unknown id reads as default (no row → flatten None → default).
        let unknown = db.get_game_overrides("nope").expect("unknown");
        assert_eq!(unknown, GameOverrides::default());
    }

    #[test]
    fn game_overrides_persists_when_only_late_added_field_set() {
        // Regression: the is_empty NULL-out check used to hand-list the
        // first ~11 fields and missed every later addition (libretro_device
        // + per-port siblings, platform_music_path, dosbox_entry_point,
        // analog_routing, keypad_layout_note, display_aspect_override,
        // overscan_crop_override, bezel_image_path). An override consisting
        // ONLY of a missing field evaluated "empty" → row NULL'd → next
        // get returned default → operator's Zapper / SNES Mouse / Light
        // Gun pick disappeared on dialog re-open and never reached the
        // emu at launch.
        let db = fresh_db();
        db.add_games(&[row("duck", "Duck Hunt")]).expect("seed");
        let only_zapper = GameOverrides {
            libretro_device_port1: Some(4), // RETRO_DEVICE_LIGHTGUN on NES port 2
            ..GameOverrides::default()
        };
        db.set_game_overrides("duck", &only_zapper).expect("set");
        let after = db.get_game_overrides("duck").expect("get");
        assert_eq!(after, only_zapper, "single-field override must survive round-trip");
    }

    #[test]
    fn milestones_crud_roundtrip() {
        let db = fresh_db();
        db.add_games(&[row("game", "Bonk")]).expect("seed");
        // Empty list on a fresh game.
        assert!(db.list_milestones("game").expect("empty").is_empty());
        let m = Milestone {
            id: None,
            game_id: "game".into(),
            name: "Boss 1 defeated".into(),
            description: "Defeat the first boss".into(),
            region: "system_ram".into(),
            offset: 0x1234,
            width: 1,
            op: "eq".into(),
            target: 1,
            edge_only: true,
            triggered_at_unix_ms: None,
        };
        let id = db.add_milestone(&m).expect("add");
        let list = db.list_milestones("game").expect("after add");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, Some(id));
        assert_eq!(list[0].name, "Boss 1 defeated");
        assert_eq!(list[0].triggered_at_unix_ms, None);
        // Trigger + reset round-trip.
        db.mark_milestone_triggered(id, 1700000000000).expect("mark");
        let after_trig = db.list_milestones("game").expect("after trig");
        assert_eq!(after_trig[0].triggered_at_unix_ms, Some(1700000000000));
        // Second mark on an already-triggered milestone is a no-op
        // (the WHERE triggered_at_unix_ms IS NULL guard).
        db.mark_milestone_triggered(id, 1800000000000).expect("re-mark");
        let still_trig = db.list_milestones("game").expect("re-mark");
        assert_eq!(still_trig[0].triggered_at_unix_ms, Some(1700000000000));
        // Reset clears.
        db.reset_milestone_progress(id).expect("reset");
        let after_reset = db.list_milestones("game").expect("after reset");
        assert_eq!(after_reset[0].triggered_at_unix_ms, None);
        // Update.
        let mut updated = list[0].clone();
        updated.target = 5;
        updated.op = "geq".into();
        db.update_milestone(&updated).expect("update");
        let after_update = db.list_milestones("game").expect("after update");
        assert_eq!(after_update[0].target, 5);
        assert_eq!(after_update[0].op, "geq");
        // Delete.
        assert_eq!(db.delete_milestone(id).expect("delete"), 1);
        assert!(db.list_milestones("game").expect("after delete").is_empty());
    }

    #[test]
    fn schema_v2_to_v3_migration() {
        // Build a v2 DB by hand, then open through LibraryDb which should
        // migrate it forward to v3 by adding overrides_json.
        let tmp = std::env::temp_dir().join(format!(
            "oa-library-v2-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("library")).expect("mkdir");
        let db_path = tmp.join("library").join("games.sqlite");
        {
            let conn = Connection::open(&db_path).expect("open v2");
            LibraryDb::create_v1(&conn).expect("create v1");
            LibraryDb::migrate_v1_to_v2(&conn).expect("migrate to v2");
            conn.pragma_update(None, "user_version", 2).expect("set v2");
            // Insert one row in the v2 shape (with archive_inner_path, no overrides_json).
            conn.execute(
                "INSERT INTO games (id, system_id, file_path, title, normalized_title, added_at, archive_inner_path)
                 VALUES ('legacy', 'tg16', '/roms/legacy.pce', 'Legacy', 'legacy', 12345, NULL)",
                [],
            )
            .expect("insert legacy");
        }
        // Open through LibraryDb — should migrate v2 → v3.
        let db = LibraryDb::open(&tmp).expect("open and migrate");
        assert_eq!(db.list_games().expect("list").len(), 1);
        // Overrides round-trip on the legacy row.
        let pref = GameOverrides {
            scaling_override: Some("stretched".to_string()),
            ..Default::default()
        };
        db.set_game_overrides("legacy", &pref).expect("set on legacy");
        let got = db.get_game_overrides("legacy").expect("get on legacy");
        assert_eq!(got, pref);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn schema_v1_to_v2_migration() {
        // Build a v1 DB by hand, then open through LibraryDb which should
        // migrate it forward.
        let tmp = std::env::temp_dir().join(format!(
            "oa-library-v1-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("library")).expect("mkdir");
        let db_path = tmp.join("library").join("games.sqlite");

        // Create v1 by hand: use the create_v1 helper, set user_version=1.
        {
            let conn = Connection::open(&db_path).expect("open v1");
            LibraryDb::create_v1(&conn).expect("create v1");
            conn.pragma_update(None, "user_version", 1).expect("set v1");
            // Insert one row in the v1 shape (no archive_inner_path column yet).
            conn.execute(
                "INSERT INTO games (id, system_id, file_path, title, normalized_title, added_at)
                 VALUES ('old', 'tg16', '/roms/old.pce', 'Old Game', 'old game', 12345)",
                [],
            )
            .expect("insert legacy");
        }

        // Now open through LibraryDb — bootstrap_schema should migrate to v2.
        let db = LibraryDb::open(&tmp).expect("open and migrate");
        let games = db.list_games().expect("list after migrate");
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].id, "old");
        assert_eq!(games[0].archive_inner_path, None);
        // Confirm we can now insert a v2-shaped row.
        let mut new_row = row("new", "New Archive");
        new_row.archive_inner_path = Some("inner.pce".to_string());
        new_row.file_path = "/roms/new.zip#inner.pce".to_string();
        assert_eq!(db.add_games(&[new_row]).expect("add v2"), 1);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn rom_hashes_upsert_lookup_and_apply() {
        let db = fresh_db();
        // Seed two games of system tg16.
        db.add_games(&[row("g1", "Old Junk Name (1).pce"), row("g2", "Renamed Garbage (2).pce")])
            .expect("seed games");

        // Bulk-insert two hashes.
        let rows = vec![
            RomHashRow {
                sha1: "AA00aa00aa00aa00aa00aa00aa00aa00aa00aa00".into(),
                system_id: "tg16".into(),
                game_name: "Bonk's Adventure (USA)".into(),
                serial: Some("TGX040080".into()),
                crc32: Some("4f0bb6d2".into()),
                size_bytes: Some(393_216),
            },
            RomHashRow {
                sha1: "BB11bb11bb11bb11bb11bb11bb11bb11bb11bb11".into(),
                system_id: "tg16".into(),
                game_name: "Air Zonk (USA)".into(),
                serial: None,
                crc32: None,
                size_bytes: None,
            },
        ];
        assert_eq!(db.upsert_rom_hashes(&rows).expect("upsert"), 2);
        assert_eq!(db.count_rom_hashes("tg16").expect("count"), 2);

        // Case-insensitive lookup — sha1 stored lowercase, query upper.
        let got = db
            .lookup_rom_hash("AA00AA00AA00AA00AA00AA00AA00AA00AA00AA00")
            .expect("lookup")
            .expect("hit");
        assert_eq!(got.game_name, "Bonk's Adventure (USA)");
        assert_eq!(got.serial.as_deref(), Some("TGX040080"));

        // Apply the match → game title rewritten to canonical, sha1 + serial
        // stamped, normalized_title rebuilt so FTS5 stays consistent.
        db.apply_rom_hash(
            "g1",
            "aa00aa00aa00aa00aa00aa00aa00aa00aa00aa00",
            Some("Bonk's Adventure (USA)"),
            Some("TGX040080"),
        )
        .expect("apply");
        let listed = db.list_games().expect("list");
        let g1 = listed.iter().find(|g| g.id == "g1").expect("g1 present");
        assert_eq!(g1.title, "Bonk's Adventure (USA)");

        // missing-hash query now excludes g1 (it has a sha1) but still
        // returns g2.
        let missing = db.list_games_missing_hash("tg16", false).expect("missing");
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].id, "g2");
    }

    #[test]
    fn rom_hashes_no_match_stamps_sha1_only() {
        // When apply_rom_hash is called with canonical_title = None we
        // record the sha1 (so re-runs don't re-hash) but leave the user's
        // title alone — covers the "scanned but not in DB" case.
        let db = fresh_db();
        db.add_games(&[row("g1", "User Title")]).expect("seed");
        db.apply_rom_hash("g1", "deadbeef".into(), None, None).expect("apply");
        let game = db.list_games().expect("list").into_iter().find(|g| g.id == "g1").unwrap();
        assert_eq!(game.title, "User Title");
        // And the missing-hash query no longer returns it.
        assert!(db.list_games_missing_hash("tg16", false).expect("missing").is_empty());
    }

    #[test]
    fn replace_rom_hashes_for_system_removes_orphans() {
        // Seed two systems with two entries each. Calling
        // replace_rom_hashes_for_system("tg16", &[one_new]) must:
        // - drop both old tg16 entries
        // - leave the snes entries untouched
        // - end with exactly the one_new row for tg16
        let db = fresh_db();
        let initial = vec![
            RomHashRow {
                sha1: "1".repeat(40),
                system_id: "tg16".into(),
                game_name: "Old TG-16 A".into(),
                serial: None,
                crc32: None,
                size_bytes: None,
            },
            RomHashRow {
                sha1: "2".repeat(40),
                system_id: "tg16".into(),
                game_name: "Old TG-16 B (since removed upstream)".into(),
                serial: None,
                crc32: None,
                size_bytes: None,
            },
            RomHashRow {
                sha1: "3".repeat(40),
                system_id: "snes".into(),
                game_name: "Untouched SNES".into(),
                serial: None,
                crc32: None,
                size_bytes: None,
            },
        ];
        assert_eq!(db.upsert_rom_hashes(&initial).expect("seed"), 3);

        let replacement = vec![RomHashRow {
            sha1: "4".repeat(40),
            system_id: "tg16".into(),
            game_name: "Fresh TG-16".into(),
            serial: Some("TGX040080".into()),
            crc32: None,
            size_bytes: None,
        }];
        assert_eq!(
            db.replace_rom_hashes_for_system("tg16", &replacement).expect("replace"),
            1
        );

        assert_eq!(db.count_rom_hashes("tg16").expect("count tg16"), 1);
        assert_eq!(db.count_rom_hashes("snes").expect("count snes"), 1);
        assert!(db.lookup_rom_hash(&"1".repeat(40)).expect("lookup 1").is_none());
        assert!(db.lookup_rom_hash(&"2".repeat(40)).expect("lookup 2").is_none());
        let fresh = db
            .lookup_rom_hash(&"4".repeat(40))
            .expect("lookup 4")
            .expect("hit");
        assert_eq!(fresh.game_name, "Fresh TG-16");
    }

    #[test]
    fn lookup_rom_hashes_batch_returns_hits_and_skips_misses() {
        let db = fresh_db();
        db.upsert_rom_hashes(&[
            RomHashRow {
                sha1: "a".repeat(40),
                system_id: "snes".into(),
                game_name: "Game A".into(),
                serial: None,
                crc32: None,
                size_bytes: None,
            },
            RomHashRow {
                sha1: "b".repeat(40),
                system_id: "snes".into(),
                game_name: "Game B".into(),
                serial: None,
                crc32: None,
                size_bytes: None,
            },
        ])
        .expect("upsert");

        // Mix of two hits, one miss. Batched lookup returns 2 rows.
        let probes = vec!["a".repeat(40), "b".repeat(40), "c".repeat(40)];
        let out = db.lookup_rom_hashes_batch(&probes).expect("batch");
        assert_eq!(out.len(), 2);
        assert_eq!(out.get(&"a".repeat(40)).unwrap().game_name, "Game A");
        assert_eq!(out.get(&"b".repeat(40)).unwrap().game_name, "Game B");
        assert!(out.get(&"c".repeat(40)).is_none());

        // Case-insensitive matching: uppercase probe still hits.
        let upper = vec!["A".repeat(40)];
        let out = db.lookup_rom_hashes_batch(&upper).expect("batch upper");
        assert_eq!(out.len(), 1);
        // Returned map is keyed on the stored (lowercase) sha1.
        assert!(out.get(&"a".repeat(40)).is_some());

        // Empty input returns empty map without locking the connection
        // or hitting SQLite.
        let empty: Vec<String> = Vec::new();
        let out = db.lookup_rom_hashes_batch(&empty).expect("batch empty");
        assert!(out.is_empty());
    }

    #[test]
    fn replace_game_serials_for_system_removes_orphans() {
        let db = fresh_db();
        db.upsert_game_serials(&[
            GameSerialRow {
                system_id: "tg16".into(),
                serial: "TGX040080".into(),
                canonical_title: "Old A".into(),
                region: None,
            },
            GameSerialRow {
                system_id: "tg16".into(),
                serial: "TGXCD1037".into(),
                canonical_title: "Old B".into(),
                region: None,
            },
            GameSerialRow {
                system_id: "snes".into(),
                serial: "SNS-12345".into(),
                canonical_title: "Untouched".into(),
                region: None,
            },
        ])
        .expect("seed");

        db.replace_game_serials_for_system(
            "tg16",
            &[GameSerialRow {
                system_id: "tg16".into(),
                serial: "TGX040080".into(),
                canonical_title: "Fresh A".into(),
                region: Some("USA".into()),
            }],
        )
        .expect("replace");

        assert_eq!(db.count_game_serials("tg16").expect("count tg16"), 1);
        assert_eq!(db.count_game_serials("snes").expect("count snes"), 1);
        let fresh = db
            .lookup_game_serial("tg16", "TGX040080")
            .expect("lookup")
            .expect("hit");
        assert_eq!(fresh.canonical_title, "Fresh A");
        assert!(db
            .lookup_game_serial("tg16", "TGXCD1037")
            .expect("lookup orphan")
            .is_none());
    }

    #[test]
    fn replace_rom_hashes_drops_rows_with_mismatched_system_id() {
        // Defensive — calling replace("tg16", entries_for_snes) must NOT
        // smuggle snes rows into the table. Catches parser-side bugs
        // before they corrupt cross-system data.
        let db = fresh_db();
        let n = db
            .replace_rom_hashes_for_system(
                "tg16",
                &[RomHashRow {
                    sha1: "a".repeat(40),
                    system_id: "snes".into(), // wrong system_id
                    game_name: "Should be dropped".into(),
                    serial: None,
                    crc32: None,
                    size_bytes: None,
                }],
            )
            .expect("replace");
        assert_eq!(n, 0, "row with mismatched system_id should be dropped");
        assert_eq!(db.count_rom_hashes("tg16").expect("count"), 0);
        assert_eq!(db.count_rom_hashes("snes").expect("count snes"), 0);
    }

    #[test]
    fn game_serials_upsert_and_lookup() {
        let db = fresh_db();
        let rows = vec![
            GameSerialRow {
                system_id: "tg16".into(),
                serial: "TGX040080".into(),
                canonical_title: "Bonk's Adventure (USA)".into(),
                region: Some("USA".into()),
            },
            GameSerialRow {
                system_id: "tg16".into(),
                serial: "TGXCD1037".into(),
                canonical_title: "Ys Book I & II (USA)".into(),
                region: Some("USA".into()),
            },
        ];
        assert_eq!(db.upsert_game_serials(&rows).expect("upsert"), 2);
        assert_eq!(db.count_game_serials("tg16").expect("count"), 2);

        let got = db
            .lookup_game_serial("tg16", "TGXCD1037")
            .expect("lookup")
            .expect("hit");
        assert_eq!(got.canonical_title, "Ys Book I & II (USA)");
        assert_eq!(got.region.as_deref(), Some("USA"));

        // Miss on a different system_id even with the same serial.
        assert!(db
            .lookup_game_serial("snes", "TGX040080")
            .expect("lookup")
            .is_none());

        // Idempotent — re-upserting an updated title replaces the row.
        db.upsert_game_serials(&[GameSerialRow {
            system_id: "tg16".into(),
            serial: "TGX040080".into(),
            canonical_title: "Bonk's Adventure (Updated)".into(),
            region: Some("USA".into()),
        }])
        .expect("re-upsert");
        let got = db.lookup_game_serial("tg16", "TGX040080").expect("re-lookup").expect("hit");
        assert_eq!(got.canonical_title, "Bonk's Adventure (Updated)");
        // Total count unchanged.
        assert_eq!(db.count_game_serials("tg16").expect("count"), 2);
    }

    #[test]
    fn apply_disc_id_round_trip() {
        let db = fresh_db();
        db.add_games(&[row("g1", "Castlevania - Rondo of Blood.cue")])
            .expect("seed game");
        // Apply with canonical title — title + disc_id both stamped.
        db.apply_disc_id("g1", "TGXCD1037", Some("Castlevania: Rondo of Blood (USA)"))
            .expect("apply");
        let games = db.list_games().expect("list");
        let g1 = games.iter().find(|g| g.id == "g1").expect("g1");
        assert_eq!(g1.title, "Castlevania: Rondo of Blood (USA)");
        assert_eq!(g1.disc_id.as_deref(), Some("TGXCD1037"));

        // Apply with title = None — stamps disc_id only, leaves title alone.
        db.add_games(&[row("g2", "Mystery Disc.cue")]).expect("seed g2");
        db.apply_disc_id("g2", "UNKNOWN001", None).expect("apply no-match");
        let games = db.list_games().expect("re-list");
        let g2 = games.iter().find(|g| g.id == "g2").expect("g2");
        assert_eq!(g2.title, "Mystery Disc.cue"); // unchanged
        assert_eq!(g2.disc_id.as_deref(), Some("UNKNOWN001"));

        // list_games_missing_hash with include_disc_id_stamped=false
        // excludes both — neither has a sha1 but BOTH have a disc_id.
        assert!(db
            .list_games_missing_hash("tg16", false)
            .expect("missing-hash cart-shape")
            .iter()
            .all(|r| r.id != "g1" && r.id != "g2"));
        // include_disc_id_stamped=true (the disc-shape resolve flow)
        // surfaces them again for per-track retry — the Phase A1
        // Sub-phase 3 fix that unblocked Dreamcast.
        let disc_retry: Vec<String> = db
            .list_games_missing_hash("tg16", true)
            .expect("missing-hash disc-shape")
            .iter()
            .map(|r| r.id.clone())
            .collect();
        assert!(
            disc_retry.contains(&"g1".to_string()) && disc_retry.contains(&"g2".to_string()),
            "disc-id-stamped games must be re-surfaced when include_disc_id_stamped=true \
             (the fix that unblocks per-track identify after a prior disc-id pass)"
        );
    }

    #[test]
    fn game_group_defaults_crud() {
        let db = fresh_db();
        db.add_games(&[row("g1", "Castlevania (USA).nes"), row("g2", "Castlevania (Japan).nes")])
            .expect("seed games");

        // Initially no defaults.
        let m = db
            .list_game_group_defaults_for_system("tg16")
            .expect("list empty");
        assert!(m.is_empty());

        // Set, list, clear.
        db.set_game_group_default("tg16", "Castlevania", "g1").expect("set");
        let m = db.list_game_group_defaults_for_system("tg16").expect("list");
        assert_eq!(m.get("castlevania").map(String::as_str), Some("g1"));

        // Idempotent re-set switches to a different variant.
        db.set_game_group_default("tg16", "Castlevania", "g2").expect("re-set");
        let m = db.list_game_group_defaults_for_system("tg16").expect("re-list");
        assert_eq!(m.get("castlevania").map(String::as_str), Some("g2"));
        assert_eq!(m.len(), 1, "re-set replaces, doesn't dupe");

        // Cascade: deleting the pinned game removes the default row.
        db.delete_game("g2").expect("delete g2");
        let m = db.list_game_group_defaults_for_system("tg16").expect("post-cascade");
        assert!(m.is_empty(), "default cascades on game delete");

        // Clear is idempotent on missing rows.
        db.clear_game_group_default("tg16", "Nothing Set").expect("clear noop");
    }

    #[test]
    fn schema_v18_to_v19_migration() {
        // Build a v18 DB by hand (rom_hashes + background_jobs exist,
        // no per-track tables), open through LibraryDb to migrate
        // forward, exercise every new surface. Mirrors the v17→v18
        // test pattern: pragma_update jumps to the pre-migration
        // version and the bootstrap chain runs only from there.
        let tmp = std::env::temp_dir().join(format!(
            "oa-library-v18-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("library")).expect("mkdir");
        let db_path = tmp.join("library").join("games.sqlite");
        {
            // Run every intermediate migration so the cart-shape
            // rom_hashes table exists (created in v7→v8) — we want to
            // verify it stays untouched by v18→v19. Mirrors the
            // v8→v9 test's full-chain build pattern.
            let conn = Connection::open(&db_path).expect("open v18");
            LibraryDb::create_v1(&conn).expect("create v1");
            LibraryDb::migrate_v1_to_v2(&conn).expect("v2");
            LibraryDb::migrate_v2_to_v3(&conn).expect("v3");
            LibraryDb::migrate_v3_to_v4(&conn).expect("v4");
            LibraryDb::migrate_v4_to_v5(&conn).expect("v5");
            LibraryDb::migrate_v5_to_v6(&conn).expect("v6");
            LibraryDb::migrate_v6_to_v7(&conn).expect("v7");
            LibraryDb::migrate_v7_to_v8(&conn).expect("v8");
            LibraryDb::migrate_v8_to_v9(&conn).expect("v9");
            LibraryDb::migrate_v9_to_v10(&conn).expect("v10");
            LibraryDb::migrate_v10_to_v11(&conn).expect("v11");
            LibraryDb::migrate_v11_to_v12(&conn).expect("v12");
            LibraryDb::migrate_v12_to_v13(&conn).expect("v13");
            LibraryDb::migrate_v13_to_v14(&conn).expect("v14");
            LibraryDb::migrate_v14_to_v15(&conn).expect("v15");
            LibraryDb::migrate_v15_to_v16(&conn).expect("v16");
            LibraryDb::migrate_v16_to_v17(&conn).expect("v17");
            LibraryDb::migrate_v17_to_v18(&conn).expect("v18");
            conn.pragma_update(None, "user_version", 18).expect("set v18");
        }
        let db = LibraryDb::open(&tmp).expect("open and migrate");

        // All three new tables exist after migration.
        let conn2 = Connection::open(&db_path).expect("reopen");
        let table_count: i64 = conn2
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN \
                 ('rom_hashes_tracks', 'game_disc_tracks', 'disc_sets')",
                [],
                |row| row.get(0),
            )
            .expect("query new tables presence");
        assert_eq!(
            table_count, 3,
            "rom_hashes_tracks + game_disc_tracks + disc_sets should all exist after v19 migration"
        );

        // games.disc_set_id + games.disc_number columns exist.
        let cols: std::collections::HashSet<String> = {
            let mut stmt = conn2
                .prepare("PRAGMA table_info(games)")
                .expect("table_info games");
            let mut out = std::collections::HashSet::new();
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .expect("query table_info");
            for r in rows {
                out.insert(r.expect("row"));
            }
            out
        };
        assert!(cols.contains("disc_set_id"), "games.disc_set_id missing post-v19");
        assert!(cols.contains("disc_number"), "games.disc_number missing post-v19");

        // Round-trip a row through the new rom_hashes_tracks surface.
        let track = RomTrackRow {
            sha1: "11111111111111111111111111111111111111aa".into(),
            system_id: "psx".into(),
            game_name: "Tomb Raider (USA)".into(),
            serial: Some("SLUS-00152".into()),
            track_number: 1,
            track_mode: "DATA".into(),
            size_bytes: 622_272,
        };
        let n = db
            .replace_rom_hashes_tracks_for_system("psx", &[track.clone()])
            .expect("replace_rom_hashes_tracks_for_system");
        assert_eq!(n, 1);
        let found = db
            .lookup_rom_hash_track("11111111111111111111111111111111111111AA")
            .expect("lookup_rom_hash_track")
            .expect("hit");
        assert_eq!(found.game_name, "Tomb Raider (USA)");
        assert_eq!(found.track_number, 1);
        assert_eq!(found.track_mode, "DATA");
        assert_eq!(found.size_bytes, 622_272);
        assert_eq!(db.count_rom_hashes_tracks("psx").expect("count"), 1);
        // Cart-shape rom_hashes is untouched — no rows for the disc.
        assert_eq!(
            db.count_rom_hashes("psx").expect("count cart"),
            0,
            "rom_hashes shouldn't be touched by replace_rom_hashes_tracks_for_system"
        );

        // Wipe-and-replace semantics: writing an empty slice clears.
        let n = db
            .replace_rom_hashes_tracks_for_system("psx", &[])
            .expect("clear via empty");
        assert_eq!(n, 0);
        assert_eq!(db.count_rom_hashes_tracks("psx").expect("count post-clear"), 0);

        // disc_sets UPSERT — id stays stable across re-sync; disc_count
        // updates in place.
        let _ = db
            .upsert_disc_sets_for_system(
                "psx",
                &[("Final Fantasy IX (USA)".to_string(), 4)],
            )
            .expect("first upsert");
        let id_v1: i64 = conn2
            .query_row(
                "SELECT id FROM disc_sets WHERE system_id = 'psx' \
                 AND canonical_title = 'Final Fantasy IX (USA)'",
                [],
                |row| row.get(0),
            )
            .expect("read id v1");
        let _ = db
            .upsert_disc_sets_for_system(
                "psx",
                &[("Final Fantasy IX (USA)".to_string(), 4)],
            )
            .expect("re-upsert");
        let id_v2: i64 = conn2
            .query_row(
                "SELECT id FROM disc_sets WHERE system_id = 'psx' \
                 AND canonical_title = 'Final Fantasy IX (USA)'",
                [],
                |row| row.get(0),
            )
            .expect("read id v2");
        assert_eq!(
            id_v1, id_v2,
            "UPSERT must preserve the autoincrement id across re-sync"
        );
        // disc_count changes in place on conflict.
        let _ = db
            .upsert_disc_sets_for_system(
                "psx",
                &[("Final Fantasy IX (USA)".to_string(), 5)],
            )
            .expect("third upsert with new count");
        let (id_v3, count_v3): (i64, i64) = conn2
            .query_row(
                "SELECT id, disc_count FROM disc_sets WHERE system_id = 'psx' \
                 AND canonical_title = 'Final Fantasy IX (USA)'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read after disc_count change");
        assert_eq!(id_v3, id_v1, "id stable through disc_count change");
        assert_eq!(count_v3, 5);
        assert_eq!(db.count_disc_sets("psx").expect("count disc_sets"), 1);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn game_disc_tracks_round_trip_through_cache_helpers() {
        // Phase A1 Sub-phase 3 — verify the operator-side per-track
        // cache writes / reads with stamps intact, clear empties, and
        // mtime/size drift drives the caller (via stamp comparison)
        // to re-hash.
        let tmp = std::env::temp_dir().join(format!(
            "oa-library-disc-cache-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("library")).expect("mkdir");
        let db = LibraryDb::open(&tmp).expect("open");
        // FK requires a games row.
        let mut g = row("g1", "Tomb Raider");
        g.system_id = "psx".into();
        g.file_path = "/roms/psx/tomb.cue".into();
        db.add_games(&[g]).expect("add game");

        let tracks = vec![
            crate::disc_track_hash::TrackHash {
                track_number: 1,
                track_mode: "MODE1/2352".into(),
                sha1: "1111111111111111111111111111111111111111".into(),
                size_bytes: 622_272,
            },
            crate::disc_track_hash::TrackHash {
                track_number: 2,
                track_mode: "AUDIO".into(),
                sha1: "2222222222222222222222222222222222222222".into(),
                size_bytes: 33_840_960,
            },
        ];
        let written = db
            .write_game_disc_tracks("g1", &tracks, 1_700_000_000, 622_272 + 33_840_960, 1_700_000_001)
            .expect("write_game_disc_tracks");
        assert_eq!(written, 2);

        // Read back; tracks come out in track_number order.
        let cached = db
            .get_game_disc_tracks("g1")
            .expect("get_game_disc_tracks")
            .expect("Some");
        assert_eq!(cached.tracks.len(), 2);
        assert_eq!(cached.tracks[0].track_number, 1);
        assert_eq!(cached.tracks[0].track_mode, "MODE1/2352");
        assert_eq!(cached.tracks[1].track_number, 2);
        assert_eq!(cached.file_mtime, 1_700_000_000);
        assert_eq!(cached.file_size, 622_272 + 33_840_960);
        assert_eq!(cached.last_hashed_at, 1_700_000_001);

        // Sha1 lower-cased on write.
        let upper_sha = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let track_upper = vec![crate::disc_track_hash::TrackHash {
            track_number: 1,
            track_mode: "MODE1/2352".into(),
            sha1: upper_sha.into(),
            size_bytes: 1024,
        }];
        let _ = db
            .write_game_disc_tracks("g1", &track_upper, 1_700_000_100, 1024, 1_700_000_101)
            .expect("rewrite");
        let cached = db.get_game_disc_tracks("g1").expect("get").expect("Some");
        assert_eq!(cached.tracks.len(), 1, "rewrite replaces, not appends");
        assert_eq!(cached.tracks[0].sha1, upper_sha.to_ascii_lowercase());

        // Stamp drift simulates "operator replaced the dump."
        // Caller logic: stat() returns different mtime/size → clear +
        // re-hash. Tests the contract: stamps are returned faithfully
        // so the caller can compare.
        assert_eq!(cached.file_mtime, 1_700_000_100);
        assert_eq!(cached.file_size, 1024);

        // Clear empties.
        let n = db.clear_game_disc_tracks("g1").expect("clear");
        assert_eq!(n, 1);
        assert!(db.get_game_disc_tracks("g1").expect("get post-clear").is_none());

        // FK cascade: deleting the game cascades the disc-track rows.
        let _ = db
            .write_game_disc_tracks("g1", &tracks, 1, 1, 1)
            .expect("rewrite-after-clear");
        db.delete_game("g1").expect("delete game");
        assert!(
            db.get_game_disc_tracks("g1").expect("get after game delete").is_none(),
            "FK CASCADE drops disc-track rows when game is deleted"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn lookup_rom_hashes_tracks_for_game_returns_sorted_by_track_number() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-library-lookup-tracks-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("library")).expect("mkdir");
        let db = LibraryDb::open(&tmp).expect("open");
        // Insert canonical tracks in REVERSE order — the lookup should
        // re-sort.
        db.replace_rom_hashes_tracks_for_system(
            "psx",
            &[
                RomTrackRow {
                    sha1: "33".into(),
                    system_id: "psx".into(),
                    game_name: "Foo (USA)".into(),
                    serial: Some("SLUS-00001".into()),
                    track_number: 3,
                    track_mode: "AUDIO".into(),
                    size_bytes: 100,
                },
                RomTrackRow {
                    sha1: "11".into(),
                    system_id: "psx".into(),
                    game_name: "Foo (USA)".into(),
                    serial: Some("SLUS-00001".into()),
                    track_number: 1,
                    track_mode: "DATA".into(),
                    size_bytes: 100,
                },
                RomTrackRow {
                    sha1: "22".into(),
                    system_id: "psx".into(),
                    game_name: "Foo (USA)".into(),
                    serial: Some("SLUS-00001".into()),
                    track_number: 2,
                    track_mode: "AUDIO".into(),
                    size_bytes: 100,
                },
                // Unrelated game — must not surface in this lookup.
                RomTrackRow {
                    sha1: "99".into(),
                    system_id: "psx".into(),
                    game_name: "Bar (USA)".into(),
                    serial: None,
                    track_number: 1,
                    track_mode: "DATA".into(),
                    size_bytes: 100,
                },
            ],
        )
        .expect("replace_rom_hashes_tracks_for_system");

        let tracks = db
            .lookup_rom_hashes_tracks_for_game("psx", "Foo (USA)")
            .expect("lookup");
        assert_eq!(tracks.len(), 3, "only Foo's tracks return");
        assert_eq!(tracks[0].track_number, 1);
        assert_eq!(tracks[1].track_number, 2);
        assert_eq!(tracks[2].track_number, 3);

        let none = db
            .lookup_rom_hashes_tracks_for_game("psx", "Nonexistent")
            .expect("lookup nonexistent");
        assert!(none.is_empty());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn schema_v19_to_v20_backfills_disc_set_id_on_existing_identifications() {
        // Phase A1 Sub-phase 4 hotfix — the v20 migration walks any
        // game that already has its canonical title applied (sha1
        // stamped via fuzzy match BEFORE the Sub-phase 4 backend
        // shipped) and links it to the matching disc_sets row.
        let tmp = std::env::temp_dir().join(format!(
            "oa-library-v20-backfill-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("library")).expect("mkdir");
        let db = LibraryDb::open(&tmp).expect("open");

        // Seed a multi-disc canonical entry + 4 game rows with
        // canonical titles already applied (sha1 NOT NULL) but
        // disc_set_id IS NULL — simulates the pre-fix state.
        db.upsert_disc_sets_for_system(
            "psx",
            &[("Final Fantasy IX (USA)".to_string(), 4)],
        )
        .expect("seed disc_sets");
        // Also a non-multi-disc game to verify it's untouched.
        let mut single = row("single", "Tomb Raider (USA)");
        single.system_id = "psx".into();
        single.sha1 = Some("aa".into());
        // Multi-disc games with `(Disc N)` titles, sha1 stamped, no disc_set_id.
        let mut g1 = row("ff9-d1", "Final Fantasy IX (USA) (Disc 1)");
        g1.system_id = "psx".into();
        g1.sha1 = Some("11".into());
        let mut g2 = row("ff9-d2", "Final Fantasy IX (USA) (Disc 2)");
        g2.system_id = "psx".into();
        g2.sha1 = Some("22".into());
        let mut g3 = row("ff9-d3", "Final Fantasy IX (USA) (Disc 3)");
        g3.system_id = "psx".into();
        g3.sha1 = Some("33".into());
        let mut g4 = row("ff9-d4", "Final Fantasy IX (USA) (Disc 4)");
        g4.system_id = "psx".into();
        g4.sha1 = Some("44".into());
        db.add_games(&[single, g1, g2, g3, g4]).expect("seed games");

        // Acquire the inner connection to run the migration directly.
        // (The bootstrap-on-open path already ran v20 on the fresh
        // DB; this test exercises the actual migration function on a
        // hand-built state.)
        let conn = db.inner.lock().expect("lock");
        let n = LibraryDb::migrate_v19_to_v20(&conn).expect("migrate v20");
        assert_eq!(n, 4, "stamped all 4 multi-disc games");
        drop(conn);

        // Verify disc_set_id + disc_number are stamped on the 4 FF9
        // discs and the standalone game is untouched.
        let conn = db.inner.lock().expect("lock");
        for (id, want_disc) in [("ff9-d1", 1), ("ff9-d2", 2), ("ff9-d3", 3), ("ff9-d4", 4)] {
            let (set_id, disc_n): (Option<i64>, Option<i64>) = conn
                .query_row(
                    "SELECT disc_set_id, disc_number FROM games WHERE id = ?1",
                    params![id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .expect("query");
            assert!(set_id.is_some(), "{id} got disc_set_id stamped");
            assert_eq!(disc_n, Some(want_disc), "{id} disc_number");
        }
        let single_set: Option<i64> = conn
            .query_row(
                "SELECT disc_set_id FROM games WHERE id = 'single'",
                [],
                |row| row.get(0),
            )
            .expect("query single");
        assert!(single_set.is_none(), "standalone game untouched");

        // Idempotent: re-running stamps 0 (all already linked).
        let n2 = LibraryDb::migrate_v19_to_v20(&conn).expect("re-run");
        assert_eq!(n2, 0, "re-run is no-op");
        drop(conn);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn disc_set_membership_round_trips_through_helpers() {
        // Phase A1 Sub-phase 4 — verify the disc-set lookup + membership
        // stamp + member-list helpers compose correctly.
        let tmp = std::env::temp_dir().join(format!(
            "oa-library-disc-set-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("library")).expect("mkdir");
        let db = LibraryDb::open(&tmp).expect("open");

        // Seed a disc_sets row via the existing UPSERT helper.
        db.upsert_disc_sets_for_system(
            "psx",
            &[("Final Fantasy IX (USA)".to_string(), 4)],
        )
        .expect("seed disc_sets");

        // lookup_disc_set_id hits on exact (system_id, canonical_title).
        let ff9_id = db
            .lookup_disc_set_id("psx", "Final Fantasy IX (USA)")
            .expect("lookup")
            .expect("disc set exists");
        // Wrong title returns None.
        assert!(
            db.lookup_disc_set_id("psx", "Final Fantasy IX")
                .expect("lookup")
                .is_none(),
            "exact-match on canonical_title; trailing region tag is part of the key"
        );
        // Wrong system returns None.
        assert!(
            db.lookup_disc_set_id("saturn", "Final Fantasy IX (USA)")
                .expect("lookup")
                .is_none()
        );

        // Seed 4 game rows for FF9 discs 1-4 + 1 standalone game.
        let mut g1 = row("ff9-d1", "Final Fantasy IX (USA) (Disc 1)");
        g1.system_id = "psx".into();
        let mut g2 = row("ff9-d2", "Final Fantasy IX (USA) (Disc 2)");
        g2.system_id = "psx".into();
        let mut g3 = row("ff9-d3", "Final Fantasy IX (USA) (Disc 3)");
        g3.system_id = "psx".into();
        let mut g4 = row("ff9-d4", "Final Fantasy IX (USA) (Disc 4)");
        g4.system_id = "psx".into();
        let mut standalone = row("tomb", "Tomb Raider (USA)");
        standalone.system_id = "psx".into();
        db.add_games(&[g1, g2, g3, g4, standalone]).expect("seed games");

        // Stamp membership on the 4 FF9 disc rows in non-sequential order
        // to verify list_disc_set_members sorts by disc_number ASC.
        db.apply_disc_set_membership("ff9-d3", ff9_id, 3).expect("stamp d3");
        db.apply_disc_set_membership("ff9-d1", ff9_id, 1).expect("stamp d1");
        db.apply_disc_set_membership("ff9-d4", ff9_id, 4).expect("stamp d4");
        db.apply_disc_set_membership("ff9-d2", ff9_id, 2).expect("stamp d2");

        // list_disc_set_members returns all 4 in disc_number ASC order.
        let members = db.list_disc_set_members(ff9_id).expect("list members");
        assert_eq!(members.len(), 4, "all four FF9 discs return");
        assert_eq!(members[0].id, "ff9-d1");
        assert_eq!(members[1].id, "ff9-d2");
        assert_eq!(members[2].id, "ff9-d3");
        assert_eq!(members[3].id, "ff9-d4");
        // Regression guard for the 2026-06-04 "Disc ?" frontend bug —
        // ensure disc_number AND disc_set_id come through populated
        // (the SELECT used to omit both columns, leaving them None
        // and breaking the DiscPickerDialog's button labels).
        assert_eq!(members[0].disc_number, Some(1));
        assert_eq!(members[1].disc_number, Some(2));
        assert_eq!(members[2].disc_number, Some(3));
        assert_eq!(members[3].disc_number, Some(4));
        for m in &members {
            assert_eq!(m.disc_set_id, Some(ff9_id), "disc_set_id surfaced for {}", m.id);
        }
        // Standalone game is NOT in the member list.
        assert!(
            members.iter().all(|g| g.id != "tomb"),
            "standalone game with NULL disc_set_id excluded"
        );

        // Empty result for a nonexistent disc_set_id.
        let empty = db.list_disc_set_members(99_999).expect("empty");
        assert!(empty.is_empty());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn schema_v17_to_v18_migration() {
        // Build a v17 DB by hand (no background_jobs table), then open
        // through LibraryDb so bootstrap_schema migrates it forward to
        // SCHEMA_VERSION. Per the v7→v8 test pattern: skip the
        // intermediate migrations by jumping straight to the target
        // pre-migration version via pragma_update — the bootstrap
        // chain will only run migrations from there onward.
        let tmp = std::env::temp_dir().join(format!(
            "oa-library-v17-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("library")).expect("mkdir");
        let db_path = tmp.join("library").join("games.sqlite");
        {
            let conn = Connection::open(&db_path).expect("open v17");
            LibraryDb::create_v1(&conn).expect("create v1");
            conn.pragma_update(None, "user_version", 17).expect("set v17");
        }
        let _db = LibraryDb::open(&tmp).expect("open and migrate");
        // background_jobs table should exist after migration. Use a
        // second Connection (WAL mode set on the LibraryDb-owned
        // connection makes concurrent readers safe).
        let conn2 = Connection::open(&db_path).expect("reopen");
        let cnt: i64 = conn2
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='background_jobs'",
                [],
                |row| row.get(0),
            )
            .expect("query background_jobs presence");
        assert_eq!(cnt, 1, "background_jobs table should exist after v18 migration");
        // Round-trip a row through the new shape to confirm columns + types.
        let now: i64 = 1_700_000_000_000;
        conn2
            .execute(
                "INSERT INTO background_jobs (kind, label, state, unit, last_event_at, started_at, total) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params!["core_download", "Downloading Beetle PSX HW", "pending", "bytes", now, now, 8_400_000_i64],
            )
            .expect("insert");
        let (kind, state, unit, total): (String, String, String, i64) = conn2
            .query_row(
                "SELECT kind, state, unit, total FROM background_jobs WHERE label = ?1",
                params!["Downloading Beetle PSX HW"],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .expect("read back");
        assert_eq!(kind, "core_download");
        assert_eq!(state, "pending");
        assert_eq!(unit, "bytes");
        assert_eq!(total, 8_400_000);
        // All three indexes should exist.
        let idx_count: i64 = conn2
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' \
                 AND name IN ('idx_background_jobs_active', 'idx_background_jobs_history', 'idx_background_jobs_parent')",
                [],
                |row| row.get(0),
            )
            .expect("query indexes");
        assert_eq!(idx_count, 3, "all three background_jobs indexes should exist");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn schema_v9_to_v10_migration() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-library-v9-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("library")).expect("mkdir");
        let db_path = tmp.join("library").join("games.sqlite");
        {
            let conn = Connection::open(&db_path).expect("open v9");
            LibraryDb::create_v1(&conn).expect("create v1");
            LibraryDb::migrate_v1_to_v2(&conn).expect("v2");
            LibraryDb::migrate_v2_to_v3(&conn).expect("v3");
            LibraryDb::migrate_v3_to_v4(&conn).expect("v4");
            LibraryDb::migrate_v4_to_v5(&conn).expect("v5");
            LibraryDb::migrate_v5_to_v6(&conn).expect("v6");
            LibraryDb::migrate_v6_to_v7(&conn).expect("v7");
            LibraryDb::migrate_v7_to_v8(&conn).expect("v8");
            LibraryDb::migrate_v8_to_v9(&conn).expect("v9");
            conn.pragma_update(None, "user_version", 9).expect("set v9");
        }
        let db = LibraryDb::open(&tmp).expect("open and migrate");
        db.add_games(&[row("g1", "Castlevania.nes")]).expect("add post-migrate");
        db.set_game_group_default("tg16", "Castlevania", "g1")
            .expect("set post-migrate");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn schema_v8_to_v9_migration() {
        // Build a v8 DB by hand (sha1/serial cols + rom_hashes table, but
        // no disc_id col and no game_serials table). Open through
        // LibraryDb to migrate forward, then exercise the new surface.
        let tmp = std::env::temp_dir().join(format!(
            "oa-library-v8-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("library")).expect("mkdir");
        let db_path = tmp.join("library").join("games.sqlite");
        {
            let conn = Connection::open(&db_path).expect("open v8");
            LibraryDb::create_v1(&conn).expect("create v1");
            LibraryDb::migrate_v1_to_v2(&conn).expect("v2");
            LibraryDb::migrate_v2_to_v3(&conn).expect("v3");
            LibraryDb::migrate_v3_to_v4(&conn).expect("v4");
            LibraryDb::migrate_v4_to_v5(&conn).expect("v5");
            LibraryDb::migrate_v5_to_v6(&conn).expect("v6");
            LibraryDb::migrate_v6_to_v7(&conn).expect("v7");
            LibraryDb::migrate_v7_to_v8(&conn).expect("v8");
            conn.pragma_update(None, "user_version", 8).expect("set v8");
        }
        let db = LibraryDb::open(&tmp).expect("open and migrate");
        db.upsert_game_serials(&[GameSerialRow {
            system_id: "tg16".into(),
            serial: "TGX040080".into(),
            canonical_title: "Migrated".into(),
            region: None,
        }])
        .expect("upsert post-migrate");
        assert_eq!(db.count_game_serials("tg16").expect("count"), 1);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn schema_v7_to_v8_migration() {
        // Build a v7 DB by hand (no sha1/serial cols, no rom_hashes table),
        // then open through LibraryDb which should migrate it forward.
        let tmp = std::env::temp_dir().join(format!(
            "oa-library-v7-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("library")).expect("mkdir");
        let db_path = tmp.join("library").join("games.sqlite");
        {
            let conn = Connection::open(&db_path).expect("open v7");
            LibraryDb::create_v1(&conn).expect("create v1");
            LibraryDb::migrate_v1_to_v2(&conn).expect("v2");
            LibraryDb::migrate_v2_to_v3(&conn).expect("v3");
            LibraryDb::migrate_v3_to_v4(&conn).expect("v4");
            // Mirror migrate_v4_to_v5 / v5_to_v6 / v6_to_v7 idempotency
            // by jumping straight to v7 via the bootstrap path.
            conn.pragma_update(None, "user_version", 7).expect("set v7");
        }
        let db = LibraryDb::open(&tmp).expect("open and migrate");
        // sha1 + serial columns should now exist (round-trip a real row).
        db.add_games(&[row("g1", "Migrated Game")]).expect("add post-migrate");
        db.upsert_rom_hashes(&[RomHashRow {
            sha1: "0".repeat(40),
            system_id: "tg16".into(),
            game_name: "Canonical".into(),
            serial: None,
            crc32: None,
            size_bytes: None,
        }])
        .expect("upsert post-migrate");
        assert_eq!(db.count_rom_hashes("tg16").expect("count"), 1);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    // --- Custom collections (Phase C3 Slice 12) ----------------------

    #[test]
    fn custom_collections_empty_by_default() {
        let db = fresh_db();
        let list = db.list_custom_collections().expect("list");
        assert!(list.is_empty());
    }

    #[test]
    fn create_rename_delete_collection() {
        let db = fresh_db();
        let id = db.create_custom_collection("Co-op night").expect("create");
        assert!(!id.is_empty());
        let list = db.list_custom_collections().expect("list");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Co-op night");
        assert_eq!(list[0].member_count, 0);

        db.rename_custom_collection(&id, "Saturday slate").expect("rename");
        let list = db.list_custom_collections().expect("list");
        assert_eq!(list[0].name, "Saturday slate");

        db.delete_custom_collection(&id).expect("delete");
        assert!(db.list_custom_collections().expect("list").is_empty());
    }

    #[test]
    fn create_rejects_empty_or_whitespace_name() {
        let db = fresh_db();
        assert!(db.create_custom_collection("").is_err());
        assert!(db.create_custom_collection("   ").is_err());
        assert!(db.list_custom_collections().expect("list").is_empty());
    }

    #[test]
    fn rename_unknown_collection_errors() {
        let db = fresh_db();
        let err = db.rename_custom_collection("nope", "X").unwrap_err();
        assert!(err.contains("not found"));
    }

    #[test]
    fn add_remove_members_round_trip() {
        let db = fresh_db();
        db.add_games(&[row("a", "Alpha"), row("b", "Bravo")]).expect("seed games");
        let id = db.create_custom_collection("Fav co-op").expect("create");

        db.add_to_custom_collection(&id, "a").expect("add a");
        db.add_to_custom_collection(&id, "b").expect("add b");
        let members = db.list_collection_members(&id).expect("list members");
        assert_eq!(members, vec!["a".to_string(), "b".to_string()]);

        // Count surfaced via list_custom_collections.
        let list = db.list_custom_collections().expect("list");
        assert_eq!(list[0].member_count, 2);

        db.remove_from_custom_collection(&id, "a").expect("remove a");
        let members = db.list_collection_members(&id).expect("list members");
        assert_eq!(members, vec!["b".to_string()]);
    }

    #[test]
    fn add_member_is_idempotent() {
        let db = fresh_db();
        db.add_games(&[row("a", "Alpha")]).expect("seed");
        let id = db.create_custom_collection("Set").expect("create");
        db.add_to_custom_collection(&id, "a").expect("add");
        db.add_to_custom_collection(&id, "a").expect("add again");
        assert_eq!(db.list_collection_members(&id).expect("list").len(), 1);
    }

    #[test]
    fn deleting_collection_cascades_members() {
        let db = fresh_db();
        db.add_games(&[row("a", "Alpha")]).expect("seed");
        let id = db.create_custom_collection("Doomed").expect("create");
        db.add_to_custom_collection(&id, "a").expect("add");
        db.delete_custom_collection(&id).expect("delete");
        // Re-creating a collection and listing members confirms the
        // FK cascade removed the orphan junction row — list_collection_members
        // for the new id returns empty rather than seeing the old member.
        let new_id = db.create_custom_collection("Replacement").expect("recreate");
        assert!(db.list_collection_members(&new_id).expect("list").is_empty());
    }

    #[test]
    fn deleting_game_prunes_memberships() {
        let db = fresh_db();
        db.add_games(&[row("a", "Alpha"), row("b", "Bravo")]).expect("seed");
        let id = db.create_custom_collection("Set").expect("create");
        db.add_to_custom_collection(&id, "a").expect("add a");
        db.add_to_custom_collection(&id, "b").expect("add b");

        // Deleting the game row sweeps the membership junction.
        db.delete_game("a").expect("delete game a");
        let members = db.list_collection_members(&id).expect("list");
        assert_eq!(members, vec!["b".to_string()]);
        // member_count uses an INNER JOIN against games so a stale
        // membership for a deleted game wouldn't count anyway, but the
        // junction sweep keeps the table from growing unbounded.
        let list = db.list_custom_collections().expect("list");
        assert_eq!(list[0].member_count, 1);
    }

    #[test]
    fn list_filters_orphan_memberships_via_join() {
        // Belt + suspenders: even if a future code path bypasses
        // delete_game's sweep and leaves a dangling membership row,
        // list_collection_members must not return ids that don't
        // resolve to a games row.
        let db = fresh_db();
        db.add_games(&[row("a", "Alpha")]).expect("seed");
        let id = db.create_custom_collection("Set").expect("create");
        db.add_to_custom_collection(&id, "a").expect("add");
        // Manually insert an orphan row pointing at a non-existent game.
        {
            let conn = db.inner.lock().expect("lock");
            conn.execute(
                "INSERT INTO custom_collection_members
                    (collection_id, rom_id, sort_order, added_at)
                 VALUES (?1, 'ghost', 99, 0)",
                params![&id],
            )
            .expect("orphan insert");
        }
        let members = db.list_collection_members(&id).expect("list");
        assert_eq!(members, vec!["a".to_string()]);
        let list = db.list_custom_collections().expect("list");
        assert_eq!(list[0].member_count, 1);
    }

    // ---- game info overrides (Phase 3) -------------------------------

    #[test]
    fn game_info_override_default_when_absent() {
        let db = fresh_db();
        let ov = db
            .get_game_info_override("psx", "tomb_raider_usa")
            .expect("get");
        assert_eq!(ov, crate::game_info::GameInfoOverride::default());
    }

    #[test]
    fn game_info_override_roundtrip_and_delete() {
        let db = fresh_db();
        let pref = crate::game_info::GameInfoOverride {
            short_summary: Some("Played it as a kid. Memorable cutscenes.".into()),
            controls_supported: Some(vec![
                "Standard gamepad".into(),
                "DualShock vibration".into(),
                "PSP via PS-on-PSP".into(),
            ]),
            best_emulator: Some("beetle_psx_hw_libretro.dll".into()),
            best_emulator_reason: Some("Vulkan + PGXP".into()),
            bugs: Some(vec![
                crate::game_info::GameBug {
                    description: "Operator-observed: crash on save in Hub 3".into(),
                    severity: crate::game_info::BugSeverity::Major,
                    workaround: Some("Save in Hub 2 instead".into()),
                },
            ]),
            applied_best_emulator: true,
            applied_controls: false,
        };
        db.set_game_info_override("psx", "tomb_raider_usa", &pref)
            .expect("set");
        let after = db
            .get_game_info_override("psx", "tomb_raider_usa")
            .expect("get after");
        assert_eq!(after, pref);

        // Clearing — pass a default-constructed override; row should
        // be DELETEd so the table stays sparse.
        db.set_game_info_override(
            "psx",
            "tomb_raider_usa",
            &crate::game_info::GameInfoOverride::default(),
        )
        .expect("clear");
        let cleared = db
            .get_game_info_override("psx", "tomb_raider_usa")
            .expect("get after clear");
        assert_eq!(cleared, crate::game_info::GameInfoOverride::default());

        // Unknown game id reads as default with no error.
        let unknown = db
            .get_game_info_override("psx", "nonexistent")
            .expect("unknown");
        assert_eq!(unknown, crate::game_info::GameInfoOverride::default());
    }

    #[test]
    fn game_info_override_upsert_updates_existing_row() {
        // Second set against the same (system_id, rom_id) must UPDATE
        // not duplicate; readback shows the new values.
        let db = fresh_db();
        let initial = crate::game_info::GameInfoOverride {
            short_summary: Some("v1 summary".into()),
            ..Default::default()
        };
        db.set_game_info_override("psx", "tomb_raider_usa", &initial)
            .expect("set initial");

        let updated = crate::game_info::GameInfoOverride {
            short_summary: Some("v2 — operator edited".into()),
            applied_best_emulator: true,
            ..Default::default()
        };
        db.set_game_info_override("psx", "tomb_raider_usa", &updated)
            .expect("set updated");

        let after = db
            .get_game_info_override("psx", "tomb_raider_usa")
            .expect("get");
        assert_eq!(after.short_summary.as_deref(), Some("v2 — operator edited"));
        assert!(after.applied_best_emulator);
    }

    #[test]
    fn game_info_override_list_overridden() {
        let db = fresh_db();
        // Two real overrides + one that's empty (won't appear).
        db.set_game_info_override(
            "psx",
            "tr_usa",
            &crate::game_info::GameInfoOverride {
                short_summary: Some("note 1".into()),
                ..Default::default()
            },
        )
        .expect("set 1");
        db.set_game_info_override(
            "nes",
            "smb",
            &crate::game_info::GameInfoOverride {
                applied_best_emulator: true,
                ..Default::default()
            },
        )
        .expect("set 2");
        db.set_game_info_override(
            "snes",
            "smw",
            &crate::game_info::GameInfoOverride::default(),
        )
        .expect("set empty");

        let listed = db.list_game_info_overridden().expect("list");
        assert_eq!(listed.len(), 2);
        assert!(listed.contains(&("psx".into(), "tr_usa".into())));
        assert!(listed.contains(&("nes".into(), "smb".into())));
        // The empty override case must NOT appear in the list — the
        // set_game_info_override path deleted the row at write time.
        assert!(!listed.contains(&("snes".into(), "smw".into())));
    }

    #[test]
    fn game_info_override_scopes_by_system_id() {
        // Same rom_id in two different systems is two separate rows.
        let db = fresh_db();
        let a = crate::game_info::GameInfoOverride {
            short_summary: Some("psx note".into()),
            ..Default::default()
        };
        let b = crate::game_info::GameInfoOverride {
            short_summary: Some("nes note".into()),
            ..Default::default()
        };
        db.set_game_info_override("psx", "same_rom_id", &a).expect("set a");
        db.set_game_info_override("nes", "same_rom_id", &b).expect("set b");

        let from_psx = db.get_game_info_override("psx", "same_rom_id").expect("psx");
        let from_nes = db.get_game_info_override("nes", "same_rom_id").expect("nes");
        assert_eq!(from_psx.short_summary.as_deref(), Some("psx note"));
        assert_eq!(from_nes.short_summary.as_deref(), Some("nes note"));
    }

    #[test]
    fn game_info_override_handles_corrupt_json_gracefully() {
        // Manually plant a corrupt JSON value in the controls_supported
        // column. The reader must degrade to None (no override) rather
        // than failing the whole get_game_info_override call.
        let db = fresh_db();
        {
            let conn = db.inner.lock().expect("lock");
            conn.execute(
                r#"INSERT INTO game_info_overrides
                    (system_id, rom_id, short_summary, controls_supported, bugs,
                     applied_best_emulator, applied_controls, created_at, updated_at)
                   VALUES ('psx', 'broken', NULL, 'not valid json {[', '[also broken',
                           0, 0, 0, 0)"#,
                [],
            )
            .expect("manual insert");
        }
        let ov = db.get_game_info_override("psx", "broken").expect("read");
        // Row exists (we'd return default if it didn't) — short_summary
        // is None per the column. But controls + bugs degraded to None
        // because the JSON parse failed.
        assert!(ov.controls_supported.is_none());
        assert!(ov.bugs.is_none());
    }

    // ---- System Info Panel v1 — Phase 2 DB tests --------------------

    #[test]
    fn system_info_override_default_when_absent() {
        // No row in system_info_overrides → get_system_info_override
        // returns the default-constructed override (every field None).
        let db = fresh_db();
        let ov = db.get_system_info_override("nes").expect("read");
        assert_eq!(ov, crate::system_info::SystemInfoOverride::default());
        assert!(ov.is_empty());
    }

    #[test]
    fn system_info_override_set_and_get_roundtrip() {
        let db = fresh_db();
        let pref = crate::system_info::SystemInfoOverride {
            blurb: Some("My NES blurb.".to_string()),
            cpu: Some("MOS 6502-derivative".to_string()),
            peripherals: Some(vec![crate::system_info::Peripheral {
                name: "Modded Controller".to_string(),
                glyph: "🕹️".to_string(),
            }]),
            ..Default::default()
        };
        db.set_system_info_override("nes", &pref).expect("upsert");

        let read = db.get_system_info_override("nes").expect("read back");
        assert_eq!(read, pref);
    }

    #[test]
    fn system_info_override_empty_deletes_row() {
        // Setting a default-constructed (empty) override deletes the
        // row so the table stays sparse — matches the game_info
        // pattern.
        let db = fresh_db();
        let pref = crate::system_info::SystemInfoOverride {
            blurb: Some("set".to_string()),
            ..Default::default()
        };
        db.set_system_info_override("nes", &pref).expect("upsert");
        // Now wipe it.
        db.set_system_info_override("nes", &crate::system_info::SystemInfoOverride::default())
            .expect("delete via empty");
        // Verify by counting rows directly.
        let conn = db.inner.lock().expect("lock");
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM system_info_overrides WHERE system_id = ?1",
                params!["nes"],
                |r| r.get(0),
            )
            .expect("count");
        assert_eq!(count, 0, "empty override must DELETE the row");
    }

    #[test]
    fn system_info_override_upsert_updates_existing_row() {
        let db = fresh_db();
        let first = crate::system_info::SystemInfoOverride {
            blurb: Some("v1".to_string()),
            ..Default::default()
        };
        db.set_system_info_override("nes", &first).expect("upsert 1");
        let second = crate::system_info::SystemInfoOverride {
            blurb: Some("v2".to_string()),
            cpu: Some("override cpu".to_string()),
            ..Default::default()
        };
        db.set_system_info_override("nes", &second).expect("upsert 2");
        let read = db.get_system_info_override("nes").expect("read back");
        assert_eq!(read.blurb.as_deref(), Some("v2"));
        assert_eq!(read.cpu.as_deref(), Some("override cpu"));
    }

    #[test]
    fn system_info_override_scopes_by_system_id() {
        let db = fresh_db();
        db.set_system_info_override(
            "nes",
            &crate::system_info::SystemInfoOverride {
                blurb: Some("nes blurb".to_string()),
                ..Default::default()
            },
        )
        .expect("upsert nes");
        db.set_system_info_override(
            "snes",
            &crate::system_info::SystemInfoOverride {
                blurb: Some("snes blurb".to_string()),
                ..Default::default()
            },
        )
        .expect("upsert snes");
        let nes = db.get_system_info_override("nes").expect("read nes");
        let snes = db.get_system_info_override("snes").expect("read snes");
        assert_eq!(nes.blurb.as_deref(), Some("nes blurb"));
        assert_eq!(snes.blurb.as_deref(), Some("snes blurb"));
        // Unrelated slug returns default.
        let unrelated = db.get_system_info_override("psx").expect("read psx");
        assert!(unrelated.is_empty());
    }

    #[test]
    fn system_info_list_overridden_returns_only_systems_with_rows() {
        let db = fresh_db();
        db.set_system_info_override(
            "nes",
            &crate::system_info::SystemInfoOverride {
                blurb: Some("set".to_string()),
                ..Default::default()
            },
        )
        .expect("upsert");
        db.set_system_info_override(
            "psx",
            &crate::system_info::SystemInfoOverride {
                blurb: Some("set".to_string()),
                ..Default::default()
            },
        )
        .expect("upsert");
        let listed = db.list_system_info_overridden().expect("list");
        assert_eq!(listed.len(), 2);
        assert!(listed.contains(&"nes".to_string()));
        assert!(listed.contains(&"psx".to_string()));
    }

    #[test]
    fn system_info_meta_hash_roundtrip() {
        let db = fresh_db();
        // Fresh DB → no stored hash.
        assert!(db.get_system_info_meta_hash().expect("read").is_none());
        db.set_system_info_meta_hash("abc123").expect("write");
        assert_eq!(
            db.get_system_info_meta_hash().expect("read"),
            Some("abc123".to_string())
        );
        // Subsequent write overwrites.
        db.set_system_info_meta_hash("def456").expect("write 2");
        assert_eq!(
            db.get_system_info_meta_hash().expect("read"),
            Some("def456".to_string())
        );
    }

    #[test]
    fn system_info_bake_mame_replaces_all_rows() {
        let db = fresh_db();
        let first = vec![crate::system_info::SystemInfoMame {
            system_id: "nes".to_string(),
            machine_name: Some("nes".to_string()),
            year: Some("1985".to_string()),
            max_players: Some(2),
            peripheral_hints: vec!["joy".to_string()],
            description: Some("first description".to_string()),
            ..Default::default()
        }];
        db.bake_system_info_mame(&first).expect("bake 1");
        let read = db.get_system_info_mame("nes").expect("read 1").unwrap();
        assert_eq!(read.year.as_deref(), Some("1985"));
        assert_eq!(read.max_players, Some(2));
        assert_eq!(read.peripheral_hints, vec!["joy"]);

        // Replace with a different set — the bake clears the table
        // first, so the old NES row must be gone afterward.
        let second = vec![crate::system_info::SystemInfoMame {
            system_id: "snes".to_string(),
            year: Some("1990".to_string()),
            ..Default::default()
        }];
        db.bake_system_info_mame(&second).expect("bake 2");
        assert!(
            db.get_system_info_mame("nes").expect("read 2 nes").is_none(),
            "first bake's NES row must be cleared"
        );
        let snes = db.get_system_info_mame("snes").expect("read 2 snes").unwrap();
        assert_eq!(snes.year.as_deref(), Some("1990"));
    }

    #[test]
    fn system_info_bake_curated_roundtrip() {
        let db = fresh_db();
        let recs = vec![crate::system_info::SystemInfoCurated {
            system_id: "snes".to_string(),
            manufacturer: Some("Nintendo".to_string()),
            system_type: Some("Home Console".to_string()),
            blurb: Some("Test blurb.".to_string()),
            peripherals: vec![crate::system_info::Peripheral {
                name: "SNES Controller".to_string(),
                glyph: "🎮".to_string(),
            }],
            meta: crate::system_info::SystemInfoMeta {
                schema_version: 1,
                last_updated: Some("2026-05-31".to_string()),
                contributors: Vec::new(),
            },
            ..Default::default()
        }];
        db.bake_system_info_curated(&recs).expect("bake");
        let read = db.get_system_info_curated("snes").expect("read").unwrap();
        assert_eq!(read.manufacturer.as_deref(), Some("Nintendo"));
        assert_eq!(read.system_type.as_deref(), Some("Home Console"));
        assert_eq!(read.blurb.as_deref(), Some("Test blurb."));
        assert_eq!(read.peripherals.len(), 1);
        assert_eq!(read.peripherals[0].name, "SNES Controller");
        assert_eq!(read.meta.last_updated.as_deref(), Some("2026-05-31"));
    }

    #[test]
    fn system_info_bake_curated_handles_empty_peripherals() {
        // A curated record with no peripherals must store NULL (not
        // a `[]` JSON string) in the peripherals column — so the
        // reader's empty-vec / no-override distinction stays clean.
        let db = fresh_db();
        let recs = vec![crate::system_info::SystemInfoCurated {
            system_id: "gba".to_string(),
            manufacturer: Some("Nintendo".to_string()),
            peripherals: Vec::new(),
            ..Default::default()
        }];
        db.bake_system_info_curated(&recs).expect("bake");
        let read = db.get_system_info_curated("gba").expect("read").unwrap();
        assert!(read.peripherals.is_empty());
    }

    // ---- MAME games (v17) — Phase 2 DB tests ------------------------

    fn sample_mame_games() -> Vec<crate::mame_games::MameGame> {
        vec![
            crate::mame_games::MameGame {
                name: "dkong".into(),
                description: "Donkey Kong (US set 1)".into(),
                year: Some("1981".into()),
                manufacturer: Some("Nintendo".into()),
                cloneof: None,
            },
            crate::mame_games::MameGame {
                name: "sf2".into(),
                description: "Street Fighter II: The World Warrior".into(),
                year: Some("1991".into()),
                manufacturer: Some("Capcom".into()),
                cloneof: None,
            },
            crate::mame_games::MameGame {
                name: "sf2ce".into(),
                description: "Street Fighter II': Champion Edition".into(),
                year: Some("1992".into()),
                manufacturer: Some("Capcom".into()),
                cloneof: Some("sf2".into()),
            },
        ]
    }

    #[test]
    fn mame_games_bake_inserts_all_rows() {
        let db = fresh_db();
        db.bake_mame_games(&sample_mame_games()).expect("bake");
        let read = db.get_mame_game("dkong").expect("read").unwrap();
        assert_eq!(read.description, "Donkey Kong (US set 1)");
        assert_eq!(read.year.as_deref(), Some("1981"));
        assert_eq!(read.cloneof, None);

        let clone = db.get_mame_game("sf2ce").expect("read clone").unwrap();
        assert_eq!(clone.cloneof.as_deref(), Some("sf2"));
    }

    #[test]
    fn mame_games_lookup_lowercases_input() {
        // Operator's ROM filename casing varies (`DKONG.zip` vs
        // `dkong.zip`); storage + lookup must match regardless of
        // caller casing. Bake writes lowercase; lookup lowercases on
        // read.
        let db = fresh_db();
        db.bake_mame_games(&sample_mame_games()).expect("bake");
        assert!(db.get_mame_game("DKONG").expect("read").is_some());
        assert!(db.get_mame_game("dKoNg").expect("read").is_some());
    }

    #[test]
    fn mame_games_bake_replaces_existing_rows() {
        let db = fresh_db();
        db.bake_mame_games(&sample_mame_games()).expect("bake 1");
        // Bake an entirely different set — old rows must be cleared.
        let second = vec![crate::mame_games::MameGame {
            name: "pacman".into(),
            description: "Pac-Man".into(),
            year: Some("1980".into()),
            ..Default::default()
        }];
        db.bake_mame_games(&second).expect("bake 2");
        assert!(db.get_mame_game("dkong").expect("read").is_none());
        assert!(db.get_mame_game("pacman").expect("read").is_some());
    }

    #[test]
    fn mame_games_lookup_merged_falls_through_to_l1() {
        let db = fresh_db();
        db.bake_mame_games(&sample_mame_games()).expect("bake");
        let merged = db.lookup_merged_mame_game("dkong").expect("lookup").unwrap();
        assert_eq!(merged.description, "Donkey Kong (US set 1)");
        assert_eq!(merged.year.as_deref(), Some("1981"));
        assert!(!merged.has_local_edits);
    }

    #[test]
    fn mame_games_lookup_merged_applies_l3_override() {
        let db = fresh_db();
        db.bake_mame_games(&sample_mame_games()).expect("bake");
        let ov = crate::mame_games::MameGameOverride {
            name: "dkong".into(),
            description: Some("Donkey Kong (operator polished)".into()),
            ..Default::default()
        };
        db.upsert_mame_game_override(&ov).expect("upsert");
        let merged = db.lookup_merged_mame_game("dkong").expect("lookup").unwrap();
        assert_eq!(merged.description, "Donkey Kong (operator polished)");
        // Year + manufacturer ride through L1 since L3 left them None.
        assert_eq!(merged.year.as_deref(), Some("1981"));
        assert_eq!(merged.manufacturer.as_deref(), Some("Nintendo"));
        assert!(merged.has_local_edits);
    }

    #[test]
    fn mame_games_lookup_merged_returns_none_when_l1_absent() {
        let db = fresh_db();
        db.bake_mame_games(&sample_mame_games()).expect("bake");
        // Pure-L3 lookup without L1 baseline: returns None (overrides
        // can't exist standalone — they layer ON TOP of L1).
        let ov = crate::mame_games::MameGameOverride {
            name: "homebrew".into(),
            description: Some("Some homebrew title".into()),
            ..Default::default()
        };
        db.upsert_mame_game_override(&ov).expect("upsert");
        assert!(
            db.lookup_merged_mame_game("homebrew").expect("lookup").is_none(),
            "lookup must require L1 — pure-L3 case yields None so the frontend falls through"
        );
    }

    #[test]
    fn mame_games_upsert_override_empty_deletes_row() {
        let db = fresh_db();
        db.bake_mame_games(&sample_mame_games()).expect("bake");
        let ov = crate::mame_games::MameGameOverride {
            name: "dkong".into(),
            description: Some("polished".into()),
            ..Default::default()
        };
        db.upsert_mame_game_override(&ov).expect("upsert");
        assert!(db.get_mame_game_override("dkong").expect("read").is_some());
        // Empty override → row should drop.
        let empty = crate::mame_games::MameGameOverride {
            name: "dkong".into(),
            ..Default::default()
        };
        db.upsert_mame_game_override(&empty).expect("upsert empty");
        assert!(
            db.get_mame_game_override("dkong").expect("read").is_none(),
            "empty override must DELETE the row"
        );
    }

    #[test]
    fn mame_games_reset_override_is_idempotent() {
        let db = fresh_db();
        db.bake_mame_games(&sample_mame_games()).expect("bake");
        // No row yet — reset must not error.
        db.reset_mame_game_override("dkong").expect("reset on empty");
        // Now create one + reset.
        let ov = crate::mame_games::MameGameOverride {
            name: "dkong".into(),
            description: Some("x".into()),
            ..Default::default()
        };
        db.upsert_mame_game_override(&ov).expect("upsert");
        db.reset_mame_game_override("dkong").expect("reset");
        assert!(db.get_mame_game_override("dkong").expect("read").is_none());
    }

    #[test]
    fn mame_games_upsert_override_lowercases_name() {
        let db = fresh_db();
        db.bake_mame_games(&sample_mame_games()).expect("bake");
        let ov = crate::mame_games::MameGameOverride {
            name: "DKONG".into(),  // caller passes mixed case
            description: Some("polished".into()),
            ..Default::default()
        };
        db.upsert_mame_game_override(&ov).expect("upsert");
        // Lookup with the lowercase form succeeds — the upsert
        // normalised the storage key.
        assert!(db.get_mame_game_override("dkong").expect("read").is_some());
    }

    #[test]
    fn mame_games_meta_hash_roundtrip() {
        let db = fresh_db();
        assert!(db.get_mame_games_meta_hash().expect("read").is_none());
        db.set_mame_games_meta_hash("abc").expect("write");
        assert_eq!(
            db.get_mame_games_meta_hash().expect("read"),
            Some("abc".to_string())
        );
        db.set_mame_games_meta_hash("def").expect("write 2");
        assert_eq!(
            db.get_mame_games_meta_hash().expect("read"),
            Some("def".to_string())
        );
    }

    #[test]
    fn mame_games_lookup_missing_returns_none() {
        let db = fresh_db();
        // Empty table — every lookup should return None cleanly.
        assert!(db.get_mame_game("dkong").expect("read").is_none());
        assert!(db.lookup_merged_mame_game("dkong").expect("read").is_none());
        assert!(db.get_mame_game_override("dkong").expect("read").is_none());
    }
}
