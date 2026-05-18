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

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

const SCHEMA_VERSION: i32 = 5;

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

        Ok(())
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
                        core_override, cover_path, seed, archive_inner_path
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
                            core_override, cover_path, seed, archive_inner_path
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
                        g.core_override, g.cover_path, g.seed, g.archive_inner_path
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
            let mut stmt = conn
                .prepare(&format!("{sql} ORDER BY path"))
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
        conn.execute(
            "INSERT INTO folders (id, path, scan_subfolders, subfolders_are_systems, watch_enabled, last_scanned_at)
             VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
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
            rewind_enabled: Some(true),
            rewind_capture_interval_frames: Some(3),
            rewind_buffer_megabytes: Some(48),
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
}
