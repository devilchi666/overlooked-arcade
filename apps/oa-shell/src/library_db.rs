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

const SCHEMA_VERSION: i32 = 13;

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
/// Game Genie / Action Replay / GameShark codes are not first-class —
/// they're system-specific encodings. Users translate to raw
/// address+value via online tables for now; per-system decoders are a
/// follow-up.
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

        Ok(())
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
                        sha1, serial, disc_id
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
            }))
        } else {
            Ok(None)
        }
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
    ) -> Result<Vec<GameRow>, String> {
        let conn = self.inner.lock().map_err(|_| "library_db: lock poisoned".to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, system_id, file_path, title, added_at,
                        core_override, cover_path, seed, archive_inner_path,
                        sha1, serial, disc_id
                 FROM games
                 WHERE system_id = ?1 AND (sha1 IS NULL OR sha1 = '')
                   AND (disc_id IS NULL OR disc_id = '')",
            )
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
                })
            })
            .map_err(|e| format!("query list_games_missing_hash: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("collect list_games_missing_hash: {e}"))?;
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
        let is_empty = overrides.scaling_override.is_none()
            && overrides.window_mode_override.is_none()
            && overrides.monitor_index_override.is_none()
            && overrides.region_override.is_none()
            && overrides.shader_preset.is_none()
            && overrides.bloom_amount.is_none()
            && overrides.core_options.is_empty()
            && overrides.patch_path.is_none()
            && overrides.rewind_enabled.is_none()
            && overrides.rewind_capture_interval_frames.is_none()
            && overrides.rewind_buffer_megabytes.is_none();
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
        let missing = db.list_games_missing_hash("tg16").expect("missing");
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
        assert!(db.list_games_missing_hash("tg16").expect("missing").is_empty());
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

        // list_games_missing_hash should now exclude both — neither has
        // a sha1 but BOTH have a disc_id (the new WHERE clause excludes
        // them).
        assert!(db
            .list_games_missing_hash("tg16")
            .expect("missing-hash")
            .iter()
            .all(|r| r.id != "g1" && r.id != "g2"));
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
}
