//! Game Info Panel v1 — data model + multi-document YAML parser.
//!
//! Per-system files at `docs/cores/<id>/games-info.md` describe per-game
//! structured reference data: facts (release year, publisher, region,
//! version, player count), recommendations (best emulator, controls
//! supported), known issues, and an operator-editable short summary.
//! The format is multi-document YAML with `---` separators, wrapped in
//! a `.md` extension for editor / GitHub display friendliness.
//!
//! This module owns:
//!
//! - The Rust types ([`GameInfo`] + nested struct + enum).
//! - The parser ([`parse_games_info_file`]) that walks a file body and
//!   yields successfully-parsed records; malformed records log at warn
//!   level and skip so one bad entry doesn't poison the whole file.
//! - The schema version constant ([`CURRENT_SCHEMA_VERSION`]).
//!
//! In-memory indexing (Phase 2), SQLite override storage (Phase 3),
//! query layer with field-typed precedence (Phase 4), and the various
//! UI surfaces (Phases 5–9) build on top of these primitives.
//!
//! Plan: `docs/PLANS/game-info-panel.md`. Schema reference:
//! `docs/cores/SCHEMA.md`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// Schema version embedded in every record's `meta.schema_version`
/// field. Bump only on breaking field changes; consumers should
/// reject records with `schema_version > CURRENT_SCHEMA_VERSION`
/// rather than silently rendering possibly-malformed data.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// One game's structured info record. Maps 1:1 onto a YAML document
/// in a `games-info.md` file.
///
/// All fields except [`id_key`](Self::id_key) are optional — missing /
/// null / empty-string all render as "nothing" in the UI but preserve
/// distinct provenance in the source file:
///
/// - **Missing** (field absent): "not yet considered."
/// - **`null`** (explicit YAML null): "data source had no value."
/// - **Empty string** (`""`): "intentionally blank, no info."
///
/// All three deserialize to `None` here; the source file is the
/// authoritative record of which sentinel was chosen.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct GameInfo {
    /// Mandatory identifier. See [`GameIdKey`] for the match rules.
    pub id_key: GameIdKey,

    /// Release year (4-digit). Sourced from existing metadata sync in
    /// v1; long-term scraper-managed in v2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<u32>,

    /// Publisher name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,

    /// Region tag — typically a No-Intro / Redump-style abbreviation
    /// ("USA", "Europe", "Japan", "World", "USA, Europe").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,

    /// Version string ("1.0", "Rev A", "Prototype 1996-03-12").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Number of players supported simultaneously (1 for single-
    /// player, 2 for couch co-op / VS, 4 for multitap titles, etc.).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_count: Option<u32>,

    /// Genre tag — free-form short string ("Action-Adventure",
    /// "Platformer", "RPG", "Shoot-em-up").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,

    /// Operator-editable short summary. Empty by default; in v2
    /// hand-curated content fills this for popular titles. When
    /// present, the UI shows this in place of the existing
    /// `metadata.description` with an "(operator note)" mini-label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_summary: Option<String>,

    /// Generic device categories this game uses ("Standard gamepad",
    /// "Light gun", "Mouse", "Multitap", "Touchscreen"). v1 uses
    /// free-form strings for authoring ergonomics; v2 may evolve to
    /// `RETRO_DEVICE_*` id strings for deeper integration with the
    /// per-game device-type picker.
    #[serde(default)]
    pub controls_supported: Vec<String>,

    /// Recommended core for this game (overriding the system default).
    /// Often used to surface a specific quirk: PSX titles that benefit
    /// from PGXP, MD titles where Genesis Plus GX outperforms PicoDrive
    /// on a specific game, etc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub best_emulator: Option<BestEmulator>,

    /// Known issues — game-specific quirks worth surfacing to the
    /// operator before launch. Migrated from `KNOWN_GAME_BUGS.md`
    /// free-form markdown in Phase 10 of the plan.
    #[serde(default)]
    pub bugs: Vec<GameBug>,

    /// Stylus / touch hotspots — game-specific tappable regions worth
    /// showing as labelled outlines while a stylus-using game runs
    /// (Phantom Hourglass's map, Brain Age's letter zones, Trauma
    /// Center's incision sites). Coordinates are in NDS bottom-screen
    /// native space (0..256 × 0..192). The `TouchHotspotOverlay`
    /// frontend component reads these via `get_game_info` and maps
    /// them to the WebView viewport via the renderer's last_viewport
    /// + the melonDS default stacked-screen layout assumption (bottom
    /// screen at y[192..384] of the 256×384 framebuffer; non-default
    /// layouts misplace hotspots until v2 reads the core option).
    /// NDS-specific in practice; the schema is generic so future
    /// stylus / pointer systems can adopt it.
    #[serde(default)]
    pub touch_hotspots: Vec<TouchHotspot>,

    /// File-level metadata (schema version, last-updated, contributors).
    /// Defaults populated when fields are absent (schema_version = 1).
    #[serde(default)]
    pub meta: GameInfoMeta,
}

/// One labelled tappable region. Coordinates in NDS bottom-screen
/// native space (0..256 × 0..192); the overlay component handles
/// mapping into WebView viewport coords.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct TouchHotspot {
    /// Short label shown next to the outlined rectangle ("Map",
    /// "Inventory", "Letter zone"). Kept terse so it doesn't crowd
    /// the screen at small render sizes.
    pub label: String,

    /// X coordinate of the rectangle's top-left corner, NDS bottom-
    /// screen native space (0..256).
    pub x: u16,

    /// Y coordinate of the rectangle's top-left corner, NDS bottom-
    /// screen native space (0..192).
    pub y: u16,

    /// Width of the rectangle in NDS bottom-screen native space.
    pub w: u16,

    /// Height of the rectangle in NDS bottom-screen native space.
    pub h: u16,
}

/// Identifies which game in the operator's library this record
/// describes. Match order: `(system_id, rom_hash)` first when the
/// operator's ROM has a known hash; fall back to
/// `(system_id, rom_title)` for unhashed homebrew / prototypes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct GameIdKey {
    /// OA system slug ("psx", "n64", "snes", etc.). Matches the
    /// directory under `docs/cores/<id>/`.
    pub system_id: String,

    /// SHA-1 of the canonical ROM, hex-encoded lowercase. Optional
    /// for homebrew / prototypes that aren't catalogued in
    /// libretro-database DATs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rom_hash: Option<String>,

    /// Canonical No-Intro / Redump game title — used as a fallback
    /// match key when `rom_hash` is absent. Operator's local library
    /// matches by hash first; falls through to title compare here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rom_title: Option<String>,
}

/// Recommendation block for the system default's per-game override.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct BestEmulator {
    /// libretro core filename (`beetle_psx_hw_libretro.dll`,
    /// `mupen64plus_next_libretro.dll`, etc.). The "Apply best
    /// emulator" action in the panel writes this into
    /// `GameOverrides.libretro_core` so the next launch picks it up.
    pub recommended: String,

    /// Short justification ("PGXP + Vulkan renderer eliminates the
    /// depth-buffering glitches of the SW renderer"). Renders as
    /// the body text under the recommendation in the panel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// One known issue. Captures enough for the panel's bug list +
/// (eventually) the tile badge's severity colour.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct GameBug {
    /// User-visible description ("Crashes when entering Caves of
    /// Kaliya without prior save.").
    pub description: String,

    /// How bad the bug is. Drives the panel's icon + colour and the
    /// tile badge's emphasis.
    pub severity: BugSeverity,

    /// Optional workaround ("Save in the previous room first.").
    /// When present, renders under the description as a Workaround
    /// line in the panel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workaround: Option<String>,
}

/// Bug severity scale — four levels matching how operators colloquially
/// rank issues. Ordered most-severe first; the panel sorts blockers to
/// the top and the tile badge picks the maximum severity for its tint.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum BugSeverity {
    /// Cosmetic glitch — looks slightly off but doesn't affect play.
    Cosmetic,
    /// Minor issue — noticeable but a workaround exists or it's
    /// rare enough not to disrupt a playthrough.
    Minor,
    /// Major issue — significant gameplay degradation but the game
    /// remains completable.
    Major,
    /// Blocker — game is uncompletable or crashes hard. Surfaces with
    /// the alert icon on the tile + a red emphasis in the panel.
    Blocker,
}

/// Per-record bookkeeping. All fields default-populated so a record
/// authored without an explicit `meta:` block still parses cleanly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct GameInfoMeta {
    /// Schema version for forward compatibility. Defaults to 1 (the
    /// current shipped version) when the field is absent.
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    /// ISO 8601 date string ("2026-05-30") of the last edit. None
    /// when the record has never been touched after initial creation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,
    /// Attribution list for v2's hand-curated content layer. Empty
    /// in v1 (no community pipeline yet).
    #[serde(default)]
    pub contributors: Vec<String>,
}

impl Default for GameInfoMeta {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            last_updated: None,
            contributors: Vec::new(),
        }
    }
}

fn default_schema_version() -> u32 {
    CURRENT_SCHEMA_VERSION
}

/// Parse the body of a `games-info.md` file — multi-document YAML
/// separated by `---` per the YAML spec.
///
/// Records that fail to deserialize log at warn level and are
/// skipped, so a single malformed entry doesn't drop the whole file's
/// contents. The first malformed record's serde error message goes to
/// the log so the operator (or a contributor running `cargo test`)
/// can see what went wrong.
///
/// Returns successfully-parsed records in source order.
pub fn parse_games_info_file(content: &str) -> Vec<GameInfo> {
    // Defensive: skip everything before the first line beginning with
    // `---`. The schema spec says contributors must write pure YAML
    // with `#` comments only, but markdown prose at the top of a `.md`
    // file is a natural authoring mistake — and serde_yaml's tokenizer
    // can hang indefinitely on certain raw-text patterns (backticks,
    // em-dashes, mixed quoting). Stripping the pre-document section
    // makes the parser tolerant without changing the supported schema.
    let content = skip_pre_yaml(content);
    let mut out = Vec::new();
    for (doc_idx, doc) in serde_yaml::Deserializer::from_str(content).enumerate() {
        match GameInfo::deserialize(doc) {
            Ok(gi) => out.push(gi),
            Err(e) => {
                log::warn!(
                    "game_info: skipping malformed record #{doc_idx} ({e})"
                );
            }
        }
    }
    out
}

/// Trim any content before the first line beginning with `---`. Used
/// by [`parse_games_info_file`] to guard against the markdown-prose-
/// at-top authoring mistake.
///
/// Three branches:
/// 1. Content starts (after leading whitespace) with `---` → standard
///    multi-doc YAML, return as-is.
/// 2. Content has a `\n---` token somewhere → markdown prose at top
///    is the likely shape; skip to the separator. (This intentionally
///    drops a leading document that lacks its own `---`, which the
///    schema requires anyway.)
/// 3. No `---` line at all → treat the whole content as one document.
///    Handles single-record files + the `serde_yaml::to_string`
///    roundtrip case where the serializer omits the leading separator.
fn skip_pre_yaml(content: &str) -> &str {
    let trimmed = content.trim_start();
    if trimmed.starts_with("---") {
        return trimmed;
    }
    if let Some(pos) = content.find("\n---") {
        return &content[pos + 1..];
    }
    content
}

// ---- operator local overrides ----------------------------------------
//
// Layer 3 of the three-layer data model (plan §4). Operator's per-game
// edits live in the local SQLite `game_info_overrides` table, keyed by
// `(system_id, rom_id)`. Scalar fields get individual columns;
// array fields (controls_supported, bugs) stay as JSON blobs because
// the cardinality is small + the queries don't need to filter on
// array contents.
//
// `applied_best_emulator` / `applied_controls` are provenance flags
// the "Apply" buttons set when they write into `GameOverrides`. The
// "Reset to default" affordance reads these to know which overrides
// the operator explicitly applied (so resetting takes them back to
// the file/scraper defaults rather than blanking everything).

/// Operator's local edits for one game. All scalar fields are
/// `Option` so the caller can distinguish "no override" (use file/
/// scraper value) from "intentionally cleared" (operator set it to
/// empty string).
///
/// Array fields use `Option<Vec<_>>` for the same reason — `None`
/// means "no override," `Some(vec![])` means "operator cleared the
/// list."
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameInfoOverride {
    /// Operator's short summary. None = no override; Some("") = blank.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_summary: Option<String>,
    /// Operator's controls-supported list override. None = use file
    /// value; Some(vec) replaces the file value entirely.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controls_supported: Option<Vec<String>>,
    /// Operator's best-emulator override (`recommended` field).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub best_emulator: Option<String>,
    /// Operator's best-emulator reason override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub best_emulator_reason: Option<String>,
    /// Operator's bug-list override. None = use file value; Some(vec)
    /// replaces. Operators can add their own findings or remove
    /// entries that don't repro on their setup.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bugs: Option<Vec<GameBug>>,
    /// Set true when the "Apply best emulator" panel button writes
    /// into `GameOverrides.libretro_core`. The "Reset to default"
    /// affordance reads this to know the override came from the
    /// Game Info Panel (vs the operator setting it manually in
    /// per-game settings).
    #[serde(default)]
    pub applied_best_emulator: bool,
    /// Same provenance marker for the "Apply controls" panel button
    /// that writes `GameOverrides.libretro_device_port1..4`.
    #[serde(default)]
    pub applied_controls: bool,
}

impl GameInfoOverride {
    /// True when no field carries a meaningful edit — used to decide
    /// whether to DELETE the override row vs UPSERT it. Keeps the
    /// table sparse: a default-constructed override doesn't pollute
    /// the DB.
    pub fn is_empty(&self) -> bool {
        self.short_summary.is_none()
            && self.controls_supported.is_none()
            && self.best_emulator.is_none()
            && self.best_emulator_reason.is_none()
            && self.bugs.is_none()
            && !self.applied_best_emulator
            && !self.applied_controls
    }
}

// ---- merged record + field-typed precedence (Phase 4) ----------------
//
// The query layer combines file-layer data (layer 1, the `GameInfo`
// records loaded from `docs/cores/<id>/games-info.md`) with operator
// local overrides (layer 3, the SQLite `game_info_overrides` table).
// Field-typed precedence per plan §8:
//
//   - Facts (date / publisher / region / version / player_count /
//     genre) → file always wins. The operator can't override factual
//     fields in v1; the UI shows them as read-only.
//   - Narrative (short_summary / controls_supported / best_emulator /
//     bugs) → operator override wins when present. Falls back to file
//     value when the override is None / absent.
//
// `MergedGameInfo` is the shape the UI consumes — already flattened
// per the precedence rules so the frontend doesn't repeat the merge
// logic. The few `_is_local` flags surface provenance for tooltips
// / mini-labels (plan §7 "Edit locally" surface).

/// Final per-game record after field-typed merge — the shape the UI
/// renders in tile-hover card / GameDetailPanel sections / modal Game
/// Info tab. All narrative fields already resolved through precedence;
/// frontend doesn't reapply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergedGameInfo {
    /// OA system slug — echoed back so the frontend doesn't have to
    /// thread it separately.
    pub system_id: String,

    // --- Facts (file always wins) ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,

    // --- Narrative (operator wins when present) ---
    /// Final summary text. Empty / missing → UI hides the section.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_summary: Option<String>,
    /// True when `short_summary` came from the operator's local
    /// override. Drives the "(operator note)" mini-label per Q5.
    #[serde(default)]
    pub short_summary_is_local: bool,
    /// Controls supported. Empty vec → UI hides the section.
    #[serde(default)]
    pub controls_supported: Vec<String>,
    /// Best emulator recommendation (with optional reason).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub best_emulator: Option<BestEmulator>,
    /// Known issues. Empty vec → UI hides the section.
    #[serde(default)]
    pub bugs: Vec<GameBug>,
    /// Touch hotspots. File-only in v1 (no operator override path);
    /// empty vec → UI overlay doesn't render. See [`TouchHotspot`].
    #[serde(default)]
    pub touch_hotspots: Vec<TouchHotspot>,

    // --- Status flags (provenance for the tile badge + Apply state) ---
    /// True when the operator has at least one local override on this
    /// game. Drives the `✎` tile-badge indicator.
    #[serde(default)]
    pub has_local_edits: bool,
    /// True when the "Apply best emulator" panel button was used —
    /// the per-game `libretro_core` override came from this surface,
    /// not manual entry in per-game settings. Reset-to-default reads
    /// this to know whether to unwind.
    #[serde(default)]
    pub applied_best_emulator: bool,
    /// Same provenance marker for the "Apply controls" action against
    /// `libretro_device_port1..4`.
    #[serde(default)]
    pub applied_controls: bool,
}

/// Merge file-layer + local override per the field-typed precedence
/// rules. Returns `None` when neither layer has any content (the UI
/// hides the panel sections entirely in that case).
///
/// `system_id` is required because the operator can author a local
/// override against a game that has no file-layer record — common
/// for unhashed homebrew. In that case `file` is None and the
/// merged record exists solely on the override.
pub fn merge_game_info(
    system_id: &str,
    file: Option<&GameInfo>,
    ov: &GameInfoOverride,
) -> Option<MergedGameInfo> {
    // Nothing to render — short-circuit. UI shows the existing
    // metadata-only chip strip without the new sections.
    if file.is_none() && ov.is_empty() {
        return None;
    }

    // Resolve narrative fields with override-wins semantics. None on
    // both sides → None on the merged record.
    let short_summary_is_local = ov.short_summary.is_some();
    let short_summary = ov
        .short_summary
        .clone()
        .or_else(|| file.and_then(|f| f.short_summary.clone()));

    let controls_supported = ov
        .controls_supported
        .clone()
        .unwrap_or_else(|| file.map(|f| f.controls_supported.clone()).unwrap_or_default());

    // best_emulator override: when the operator set EITHER the
    // recommended core OR the reason, the override wins (both fields
    // become the merged record's BestEmulator). File-layer value is
    // dropped in that case. When neither is set, fall through to file.
    let best_emulator = if ov.best_emulator.is_some() || ov.best_emulator_reason.is_some() {
        Some(BestEmulator {
            recommended: ov.best_emulator.clone().unwrap_or_default(),
            reason: ov.best_emulator_reason.clone(),
        })
    } else {
        file.and_then(|f| f.best_emulator.clone())
    };

    let bugs = ov
        .bugs
        .clone()
        .unwrap_or_else(|| file.map(|f| f.bugs.clone()).unwrap_or_default());

    // Touch hotspots are file-only in v1 — no operator override path.
    // If we add one in v2, slot the override read here.
    let touch_hotspots = file
        .map(|f| f.touch_hotspots.clone())
        .unwrap_or_default();

    Some(MergedGameInfo {
        system_id: system_id.to_string(),
        date: file.and_then(|f| f.date),
        publisher: file.and_then(|f| f.publisher.clone()),
        region: file.and_then(|f| f.region.clone()),
        version: file.and_then(|f| f.version.clone()),
        player_count: file.and_then(|f| f.player_count),
        genre: file.and_then(|f| f.genre.clone()),
        short_summary,
        short_summary_is_local,
        controls_supported,
        best_emulator,
        bugs,
        touch_hotspots,
        has_local_edits: !ov.is_empty(),
        applied_best_emulator: ov.applied_best_emulator,
        applied_controls: ov.applied_controls,
    })
}

// ---- tile-badge bulk computation (Phase 6) ---------------------------
//
// The tile-badge layer needs per-(system_id, rom_id) signals at scale
// — a typical library can be thousands of entries. Per-tile `getGameInfo`
// invocations would be N round-trips, so we compute badges in bulk:
//
//   1. Frontend hands us the operator's library list (id, system_id,
//      title, sha1) via the existing list_games command.
//   2. We bulk-load all overrides into a HashMap.
//   3. We walk the library list, doing in-memory file-layer + override
//      merges, and emit only entries that have a non-trivial badge
//      (at least one bug OR has_local_edits).
//
// Sized for ~10k entries with ~100 indexed file-layer records — runs in
// single-digit milliseconds. Frontend caches the result once per
// library refresh.

/// Reduced shape the tile-badge UI consumes — only the fields needed
/// to render the `⚠ N` / `✎` indicators. Skips the full merge result
/// (description, controls, etc.) so the payload over the Tauri bridge
/// stays small even for libraries with thousands of badges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameInfoBadge {
    pub system_id: String,
    pub rom_id: String,
    /// Count of bugs after the file + override merge. Zero is filtered
    /// out by the producer (compute_game_info_badges only emits entries
    /// with bugs OR local edits).
    pub bug_count: u32,
    /// Max severity across the merged bug list. None when bug_count is
    /// zero — drives the warning glyph's tint (red for blocker, amber
    /// for major, neutral for minor / cosmetic).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_severity: Option<BugSeverity>,
    /// True when the operator has at least one local override on this
    /// game. Drives the `✎` indicator alongside the bug badge.
    pub has_local_edits: bool,
}

/// Minimal view of a library entry the badge computer needs. Frontend
/// passes this from its already-loaded library cache; Rust doesn't
/// re-query the games table.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryEntryForBadges {
    pub id: String,
    pub system_id: String,
    pub title: String,
    #[serde(default)]
    pub sha1: Option<String>,
}

/// Walk a library entry list against the global file-layer index +
/// pre-loaded override map; emit badges for entries that have at least
/// one bug OR one local edit. Sort order matches the input order — the
/// frontend already has the entries in display order from list_games.
pub fn compute_game_info_badges(
    entries: &[LibraryEntryForBadges],
    overrides: &std::collections::HashMap<(String, String), GameInfoOverride>,
) -> Vec<GameInfoBadge> {
    let index = global_index();
    let mut out = Vec::new();
    for entry in entries {
        let file = index.lookup(&entry.system_id, entry.sha1.as_deref(), Some(&entry.title));
        let key = (entry.system_id.clone(), entry.id.clone());
        let ov = overrides.get(&key).cloned().unwrap_or_default();
        let Some(merged) = merge_game_info(&entry.system_id, file, &ov) else {
            continue;
        };
        let has_local = merged.has_local_edits;
        let bug_count = merged.bugs.len() as u32;
        if bug_count == 0 && !has_local {
            continue;
        }
        let max_severity = merged.bugs.iter().map(|b| b.severity).max();
        out.push(GameInfoBadge {
            system_id: entry.system_id.clone(),
            rom_id: entry.id.clone(),
            bug_count,
            max_severity,
            has_local_edits: has_local,
        });
    }
    out
}

// ---- in-memory index --------------------------------------------------
//
// Phase 2 lifts the parser into a load-at-startup index. Files live at
// `docs/cores/<id>/games-info.md` in the repo. At runtime the loader
// resolves the docs/cores directory via [`resolve_docs_cores_dir`]:
//
// - `cargo run` / `cargo tauri dev`: walks up from `CARGO_MANIFEST_DIR`
//   to the source-tree `docs/cores/` directory.
// - Production install: looks for `<exe_dir>/docs/cores/` next to the
//   shipped binary. (Bundling at install time is a Phase 11 task.)
//
// Per-system files are parsed in parallel via rayon. The lookup API
// matches the plan's §5 priority: hash first, title fallback.
//
// v2's switch to a separate `overlooked-arcade-game-info` data repo
// with daily sync (plan §11.2) replaces the source-tree path with a
// cached fetch — the `GameInfoIndex` API stays the same.

/// Resolve the `docs/cores/` directory at runtime. Walks two candidate
/// locations in priority order; returns the first one that exists.
/// `None` when neither resolves — the loader falls back to an empty
/// index in that case so a missing dev tree never crashes the app.
///
/// Order:
/// 1. `<exe_dir>/docs/cores/` — production install path (the build
///    system is expected to copy the docs tree alongside the binary;
///    Phase 11 wires this).
/// 2. `<CARGO_MANIFEST_DIR>/../../docs/cores/` — source-tree path for
///    `cargo run` + `cargo tauri dev`. Hard-coded relative to the
///    oa-shell crate, which lives at `apps/oa-shell/` two levels under
///    the repo root.
pub fn resolve_docs_cores_dir() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let p = exe_dir.join("docs").join("cores");
            if p.is_dir() {
                return Some(p);
            }
        }
    }
    // Source-tree fallback. `env!("CARGO_MANIFEST_DIR")` evaluates at
    // compile time to the absolute path of this crate's directory.
    let dev = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("docs")
        .join("cores");
    if dev.is_dir() {
        return Some(dev);
    }
    None
}

/// Two-key index over all loaded [`GameInfo`] records.
///
/// Match priority (Phase 4's query layer enforces this):
/// 1. `(system_id, rom_hash)` exact match — most specific.
/// 2. `(system_id, rom_title)` exact match — fallback for records
///    without a hash and operator libraries that haven't been hash-
///    resolved yet.
///
/// Records that publish both keys appear in both maps pointing at the
/// same cloned `GameInfo`. The `all` vector preserves source order for
/// callers that want to enumerate ("list all games with bugs", etc.).
pub struct GameInfoIndex {
    by_hash: HashMap<(String, String), GameInfo>,
    by_title: HashMap<(String, String), GameInfo>,
    all: Vec<GameInfo>,
}

impl GameInfoIndex {
    /// Build the index from a directory laid out as
    /// `<dir>/<system_slug>/games-info.md`. Each per-system file is
    /// read + parsed in parallel via rayon; missing files for a system
    /// dir are silently skipped (not every system ships a games-info
    /// file yet, especially during v1 rollout).
    ///
    /// Returns an empty index if `dir` doesn't exist or is unreadable;
    /// callers should treat that as "no game info data" rather than
    /// an error.
    pub fn load_from_dir(dir: &Path) -> Self {
        // Cheap walk: list subdirectories that contain a games-info.md.
        // The actual file reads + parse run in parallel below.
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                log::info!(
                    "game_info: docs/cores dir not readable at {} ({e}); index is empty",
                    dir.display()
                );
                return Self::empty();
            }
        };

        let candidates: Vec<(String, PathBuf)> = entries
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                // Use the file_type from DirEntry instead of calling
                // is_dir on the path — DirEntry caches the type without
                // an extra stat syscall. On Windows this skips a per-
                // entry NTFS attribute fetch that can be surprisingly
                // slow under AV scanning or indexer churn.
                let file_type = e.file_type().ok()?;
                if !file_type.is_dir() {
                    return None;
                }
                let p = e.path();
                let slug = p.file_name()?.to_str()?.to_string();
                let file = p.join("games-info.md");
                // metadata().ok().map(|m| m.is_file()) avoids the
                // is_file convenience method's syscall doubling on
                // some Rust toolchains (fixed upstream but cheap to
                // sidestep).
                let exists = std::fs::metadata(&file).map(|m| m.is_file()).unwrap_or(false);
                if !exists {
                    return None;
                }
                Some((slug, file))
            })
            .collect();

        // Sequential read + parse. Per-system files are typically a
        // few KB each; the overall workload is bounded by the system
        // count (~40) so parallelism would buy single-digit
        // milliseconds at the cost of spinning rayon's thread pool.
        // The Phase 2 trade-off favors a fast cold path on the
        // first-boot critical section.
        let mut per_system: Vec<(String, Vec<GameInfo>)> = candidates
            .iter()
            .map(|(slug, path)| {
                let body = std::fs::read_to_string(path).unwrap_or_else(|e| {
                    log::warn!(
                        "game_info: read failed for {}: {e}; treating as empty",
                        path.display()
                    );
                    String::new()
                });
                (slug.clone(), parse_games_info_file(&body))
            })
            .collect();
        per_system.sort_by(|a, b| a.0.cmp(&b.0));

        let mut by_hash: HashMap<(String, String), GameInfo> = HashMap::new();
        let mut by_title: HashMap<(String, String), GameInfo> = HashMap::new();
        let mut all: Vec<GameInfo> = Vec::new();

        for (_, records) in per_system {
            for gi in records {
                let system_id = gi.id_key.system_id.clone();
                if let Some(hash) = gi.id_key.rom_hash.as_ref() {
                    by_hash.insert((system_id.clone(), hash.clone()), gi.clone());
                }
                if let Some(title) = gi.id_key.rom_title.as_ref() {
                    by_title.insert((system_id.clone(), title.clone()), gi.clone());
                }
                all.push(gi);
            }
        }

        log::info!(
            "game_info: indexed {} records across {} systems (hash keys: {}, title keys: {}) from {}",
            all.len(),
            candidates.len(),
            by_hash.len(),
            by_title.len(),
            dir.display(),
        );

        Self { by_hash, by_title, all }
    }

    /// Convenience: resolve `docs/cores/` per [`resolve_docs_cores_dir`]
    /// and call [`load_from_dir`](Self::load_from_dir), then merge in
    /// any per-system `games.yaml` records that the
    /// [`crate::system_registry`] carries. Falls back to
    /// [`empty`](Self::empty) when neither source resolves anything.
    ///
    /// Slice 1 of the per-system descriptor consolidation
    /// (`docs/PLANS/per-system-descriptors.md`): pilot systems' game
    /// records moved from `docs/cores/<id>/games-info.md` into
    /// `config/systems/<id>/games.yaml`; the registry path merges
    /// after the legacy docs/cores walk, so registry records overwrite
    /// docs/cores records on the same `(system_id, rom_hash)` /
    /// `(system_id, rom_title)` key (matches the "registry wins per
    /// conflict" Slice 1 contract).
    pub fn load_default() -> Self {
        let mut idx = match resolve_docs_cores_dir() {
            Some(dir) => Self::load_from_dir(&dir),
            None => {
                log::warn!(
                    "game_info: could not resolve docs/cores directory \
                     (checked <exe>/docs/cores and <repo>/docs/cores); falling back to registry-only"
                );
                Self::empty()
            }
        };
        let registry = crate::system_registry::global_registry();
        let mut registry_records = 0usize;
        for sys_id in registry.system_ids() {
            if let Some(loaded) = registry.get(sys_id) {
                if let Some(games) = &loaded.games {
                    registry_records += games.games.len();
                    idx.add_records(games.games.iter().cloned());
                }
            }
        }
        if registry_records > 0 {
            log::info!(
                "game_info: merged {registry_records} additional records from config/systems/*/games.yaml"
            );
        }
        idx
    }

    /// Ingest a stream of [`GameInfo`] records into the index. New
    /// hash + title keys overwrite existing entries — used by the
    /// registry merge path in [`load_default`].
    pub fn add_records(&mut self, records: impl IntoIterator<Item = GameInfo>) {
        for gi in records {
            let system_id = gi.id_key.system_id.clone();
            if let Some(hash) = gi.id_key.rom_hash.as_ref() {
                self.by_hash
                    .insert((system_id.clone(), hash.clone()), gi.clone());
            }
            if let Some(title) = gi.id_key.rom_title.as_ref() {
                self.by_title
                    .insert((system_id.clone(), title.clone()), gi.clone());
            }
            self.all.push(gi);
        }
    }

    /// Build an empty index. Useful for tests + the fallback path when
    /// the embedded tree is somehow absent.
    pub fn empty() -> Self {
        Self {
            by_hash: HashMap::new(),
            by_title: HashMap::new(),
            all: Vec::new(),
        }
    }

    /// Lookup priority: hash first, title fallback. Returns the first
    /// match, or None when neither key resolves.
    ///
    /// `rom_hash` should be SHA-1 hex-encoded lowercase to match the
    /// schema. `rom_title` should be the canonical No-Intro / Redump
    /// title (the same shape the games-info.md author would have used).
    pub fn lookup(
        &self,
        system_id: &str,
        rom_hash: Option<&str>,
        rom_title: Option<&str>,
    ) -> Option<&GameInfo> {
        if let Some(hash) = rom_hash {
            if let Some(gi) = self.by_hash.get(&(system_id.to_string(), hash.to_string())) {
                return Some(gi);
            }
        }
        if let Some(title) = rom_title {
            if let Some(gi) = self.by_title.get(&(system_id.to_string(), title.to_string())) {
                return Some(gi);
            }
        }
        None
    }

    /// All records in stable order — system_id alphabetical, then
    /// source-file order within each system. Useful for "show every
    /// game with a Blocker bug" reports.
    pub fn all(&self) -> &[GameInfo] {
        &self.all
    }

    /// Number of indexed records — for diagnostic / debug-log surfaces.
    pub fn len(&self) -> usize {
        self.all.len()
    }

    /// True when no records loaded — useful for tests + the empty()
    /// fallback path.
    pub fn is_empty(&self) -> bool {
        self.all.is_empty()
    }
}

/// Process-wide singleton. Initialized lazily on first access so the
/// startup path doesn't pay the parse cost when no UI surface has
/// asked yet. Subsequent calls reuse the cached index.
static INDEX: OnceLock<GameInfoIndex> = OnceLock::new();

/// Access the global game-info index, building it on first call via
/// [`GameInfoIndex::load_default`]. Thread-safe; all callers see the
/// same instance for the lifetime of the process.
pub fn global_index() -> &'static GameInfoIndex {
    INDEX.get_or_init(GameInfoIndex::load_default)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sample matching the Tomb Raider example in
    /// `docs/PLANS/game-info-panel.md` §5. Used as the golden record
    /// for "all fields populated parses cleanly." Every document
    /// starts with a leading `---` per schema (so the parser's
    /// pre-YAML-skip logic doesn't mistake the document for prose).
    const TOMB_RAIDER_YAML: &str = r#"---
id_key:
  system_id: psx
  rom_hash: 7a4b00112233445566778899aabbccddeeff0011
  rom_title: "Tomb Raider (USA)"

date: 1996
publisher: Eidos Interactive
region: USA
version: "1.0"
player_count: 1
genre: Action-Adventure

short_summary: ""
controls_supported:
  - "Standard gamepad"
  - "DualShock vibration"
best_emulator:
  recommended: "beetle_psx_hw_libretro.dll"
  reason: "PGXP + Vulkan renderer eliminates the depth-buffering glitches of the SW renderer."

bugs:
  - description: "Crashes when entering Caves of Kaliya without prior save."
    severity: blocker
    workaround: "Save in the previous room first."
  - description: "Audio cuts in pre-rendered cutscene at start of Egypt level."
    severity: minor

meta:
  schema_version: 1
  last_updated: "2026-05-26"
  contributors: []
"#;

    #[test]
    fn parse_empty_string_returns_empty_vec() {
        let out = parse_games_info_file("");
        assert!(out.is_empty(), "empty input must produce empty vec");
    }

    #[test]
    fn parse_whitespace_only_returns_empty_vec() {
        // A file containing only `---` separators or whitespace
        // produces zero documents. serde_yaml treats this as "no
        // YAML content" and yields nothing — the parser passes
        // through.
        let out = parse_games_info_file("\n\n\n");
        assert!(out.is_empty());
    }

    #[test]
    fn parse_tomb_raider_golden_record() {
        let out = parse_games_info_file(TOMB_RAIDER_YAML);
        assert_eq!(out.len(), 1, "TOMB_RAIDER_YAML has one document");
        let gi = &out[0];

        assert_eq!(gi.id_key.system_id, "psx");
        assert_eq!(
            gi.id_key.rom_hash.as_deref(),
            Some("7a4b00112233445566778899aabbccddeeff0011")
        );
        assert_eq!(gi.id_key.rom_title.as_deref(), Some("Tomb Raider (USA)"));

        assert_eq!(gi.date, Some(1996));
        assert_eq!(gi.publisher.as_deref(), Some("Eidos Interactive"));
        assert_eq!(gi.region.as_deref(), Some("USA"));
        assert_eq!(gi.version.as_deref(), Some("1.0"));
        assert_eq!(gi.player_count, Some(1));
        assert_eq!(gi.genre.as_deref(), Some("Action-Adventure"));

        // Empty-string short_summary parses as Some("") — distinct
        // from the missing-field case (None). The UI treats both as
        // "render nothing," but the source file's distinction is
        // preserved here so a future authoring tool can tell them
        // apart.
        assert_eq!(gi.short_summary.as_deref(), Some(""));

        assert_eq!(
            gi.controls_supported,
            vec!["Standard gamepad".to_string(), "DualShock vibration".to_string()]
        );

        let be = gi.best_emulator.as_ref().expect("best_emulator populated");
        assert_eq!(be.recommended, "beetle_psx_hw_libretro.dll");
        assert!(be.reason.as_deref().unwrap().contains("PGXP"));

        assert_eq!(gi.bugs.len(), 2);
        assert_eq!(gi.bugs[0].severity, BugSeverity::Blocker);
        assert!(gi.bugs[0].workaround.is_some());
        assert_eq!(gi.bugs[1].severity, BugSeverity::Minor);
        assert!(gi.bugs[1].workaround.is_none(), "missing workaround → None");

        assert_eq!(gi.meta.schema_version, 1);
        assert_eq!(gi.meta.last_updated.as_deref(), Some("2026-05-26"));
        assert!(gi.meta.contributors.is_empty());
    }

    #[test]
    fn parse_minimal_record_only_id_key() {
        // Only id_key + system_id present; everything else falls back
        // to defaults. Validates that the field-optional contract
        // works for unhashed homebrew / prototypes.
        let yaml = r#"---
id_key:
  system_id: nes
  rom_title: "My Homebrew Demo"
"#;
        let out = parse_games_info_file(yaml);
        assert_eq!(out.len(), 1);
        let gi = &out[0];
        assert_eq!(gi.id_key.system_id, "nes");
        assert_eq!(gi.id_key.rom_title.as_deref(), Some("My Homebrew Demo"));
        assert!(gi.id_key.rom_hash.is_none());
        assert!(gi.date.is_none());
        assert!(gi.controls_supported.is_empty());
        assert!(gi.bugs.is_empty());
        assert!(gi.touch_hotspots.is_empty());
        // meta defaults populated even when absent.
        assert_eq!(gi.meta.schema_version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn parse_touch_hotspots_populates_vec() {
        // Phantom-Hourglass-shaped record exercising the new field.
        // Three hotspots with different positions/sizes to confirm
        // each tuple roundtrips through serde correctly.
        let yaml = r#"---
id_key:
  system_id: nds
  rom_title: "The Legend of Zelda: Phantom Hourglass (USA)"

touch_hotspots:
  - label: "Map"
    x: 8
    y: 16
    w: 80
    h: 80
  - label: "Inventory"
    x: 200
    y: 12
    w: 40
    h: 24
  - label: "Quest log"
    x: 96
    y: 120
    w: 64
    h: 56
"#;
        let out = parse_games_info_file(yaml);
        assert_eq!(out.len(), 1);
        let gi = &out[0];
        assert_eq!(gi.touch_hotspots.len(), 3);
        assert_eq!(gi.touch_hotspots[0].label, "Map");
        assert_eq!(gi.touch_hotspots[0].x, 8);
        assert_eq!(gi.touch_hotspots[0].y, 16);
        assert_eq!(gi.touch_hotspots[0].w, 80);
        assert_eq!(gi.touch_hotspots[0].h, 80);
        assert_eq!(gi.touch_hotspots[1].label, "Inventory");
        assert_eq!(gi.touch_hotspots[2].label, "Quest log");
    }


    #[test]
    fn parse_multi_document_yields_all_records_in_order() {
        // Two real documents separated by the YAML `---` marker.
        // Order matters for callers that want stable iteration.
        let yaml = format!(
            "{}\n{}",
            TOMB_RAIDER_YAML,
            r#"---
id_key:
  system_id: psx
  rom_hash: 1111222233334444555566667777888899990000
  rom_title: "Final Fantasy VII (USA)"

date: 1997
publisher: SCEA
"#
        );
        let out = parse_games_info_file(&yaml);
        assert_eq!(out.len(), 2, "two documents → two records");
        assert_eq!(out[0].id_key.rom_title.as_deref(), Some("Tomb Raider (USA)"));
        assert_eq!(out[1].id_key.rom_title.as_deref(), Some("Final Fantasy VII (USA)"));
    }

    #[test]
    fn parse_skips_malformed_record_keeps_valid_neighbours() {
        // Middle document is missing the mandatory id_key field. The
        // parser must log + skip it, and yield the two flanking
        // valid records — one bad entry should never poison the file.
        let yaml = format!(
            "{}\n---\nthis_is_not_a_game_record: true\n{}",
            TOMB_RAIDER_YAML,
            r#"---
id_key:
  system_id: snes
  rom_title: "Super Metroid (USA)"
"#
        );
        let out = parse_games_info_file(&yaml);
        assert_eq!(
            out.len(),
            2,
            "1 valid + 1 malformed + 1 valid → 2 records yielded"
        );
        assert_eq!(out[0].id_key.system_id, "psx");
        assert_eq!(out[1].id_key.system_id, "snes");
    }

    #[test]
    fn bug_severity_ordering_blocker_is_highest() {
        // Tile badge picks the max severity for its tint. Ord on
        // BugSeverity must put Blocker > Major > Minor > Cosmetic.
        let severities = [
            BugSeverity::Cosmetic,
            BugSeverity::Blocker,
            BugSeverity::Minor,
            BugSeverity::Major,
        ];
        let max = severities.iter().max().copied().unwrap();
        assert_eq!(max, BugSeverity::Blocker);
        assert!(BugSeverity::Major > BugSeverity::Minor);
        assert!(BugSeverity::Minor > BugSeverity::Cosmetic);
    }

    // ---- GameInfoIndex ------------------------------------------------

    /// Construct a small index by hand for lookup tests — avoids
    /// depending on the embedded tree's contents, so tests stay
    /// stable when the psx sample file evolves.
    fn fixture_index() -> GameInfoIndex {
        let tr: GameInfo = parse_games_info_file(TOMB_RAIDER_YAML)
            .into_iter()
            .next()
            .unwrap();
        let homebrew = parse_games_info_file(
            r#"
id_key:
  system_id: nes
  rom_title: "My Homebrew Demo"
date: 2024
"#,
        )
        .into_iter()
        .next()
        .unwrap();
        let mut idx = GameInfoIndex::empty();
        idx.by_hash.insert(
            (tr.id_key.system_id.clone(), tr.id_key.rom_hash.clone().unwrap()),
            tr.clone(),
        );
        idx.by_title.insert(
            (tr.id_key.system_id.clone(), tr.id_key.rom_title.clone().unwrap()),
            tr.clone(),
        );
        idx.by_title.insert(
            (homebrew.id_key.system_id.clone(), homebrew.id_key.rom_title.clone().unwrap()),
            homebrew.clone(),
        );
        idx.all.push(tr);
        idx.all.push(homebrew);
        idx
    }

    #[test]
    fn index_lookup_prefers_hash_match_over_title() {
        // When both hash and title resolve, hash wins per the §5
        // priority order — hash is the more specific key.
        let idx = fixture_index();
        let hit = idx.lookup(
            "psx",
            Some("7a4b00112233445566778899aabbccddeeff0011"),
            Some("Some Other Title"),
        );
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().id_key.rom_title.as_deref(), Some("Tomb Raider (USA)"));
    }

    #[test]
    fn index_lookup_falls_back_to_title_when_hash_misses() {
        // Hash doesn't resolve → fall through to title. Common case
        // for unhashed homebrew where the operator has the ROM but
        // hash isn't in the local cache yet.
        let idx = fixture_index();
        let hit = idx.lookup("nes", Some("ffff_unknown_hash"), Some("My Homebrew Demo"));
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().id_key.system_id, "nes");
    }

    #[test]
    fn index_lookup_returns_none_when_neither_key_matches() {
        let idx = fixture_index();
        assert!(idx.lookup("snes", Some("aaaa"), Some("Nonexistent")).is_none());
    }

    #[test]
    fn index_lookup_scopes_by_system_id() {
        // Same title in two different systems would be two different
        // records. Looking up by the wrong system_id misses even when
        // the title is correct — prevents cross-system collisions.
        let idx = fixture_index();
        assert!(
            idx.lookup("nes", None, Some("Tomb Raider (USA)")).is_none(),
            "wrong system_id must miss the title lookup"
        );
    }

    #[test]
    fn index_empty_constructs_cleanly() {
        let idx = GameInfoIndex::empty();
        assert!(idx.is_empty());
        assert_eq!(idx.len(), 0);
        assert!(idx.lookup("psx", Some("anything"), Some("anything")).is_none());
    }

    #[test]
    fn load_default_loads_psx_sample_via_registry_after_slice1_migration() {
        // Slice 1 Phase C (2026-06-02) — `psx` game records moved from
        // docs/cores/psx/games-info.md into config/systems/psx/games.yaml.
        // The legacy `load_from_dir` walk no longer finds them; the
        // registry merge in `load_default` does.
        //
        // Tomb Raider (USA) + Final Fantasy VII (USA) are the two seed
        // records carried through the migration.
        let idx = GameInfoIndex::load_default();
        assert!(
            idx.lookup(
                "psx",
                Some("7a4b00112233445566778899aabbccddeeff0011"),
                None,
            )
            .is_some(),
            "load_default must include Tomb Raider record from config/systems/psx/games.yaml via the registry merge"
        );
        assert!(
            idx.lookup("psx", None, Some("Final Fantasy VII (USA)"))
                .is_some(),
            "load_default must include FF7 record by title fallback (registry-merged)"
        );

        // Regression guard against accidental re-creation of
        // docs/cores/psx/games-info.md — the legacy walk alone must NOT
        // produce these records anymore.
        let dir = resolve_docs_cores_dir().expect(
            "resolve_docs_cores_dir must find the source-tree docs/cores under cargo test",
        );
        let legacy_only = GameInfoIndex::load_from_dir(&dir);
        assert!(
            legacy_only
                .lookup(
                    "psx",
                    Some("7a4b00112233445566778899aabbccddeeff0011"),
                    None,
                )
                .is_none(),
            "Tomb Raider must not be in docs/cores/ after the Slice 1 Phase C migration"
        );
    }

    #[test]
    fn parse_skips_markdown_prose_before_first_document_marker() {
        // Regression: an earlier version hung on serde_yaml's tokenizer
        // when contributors wrote markdown prose (backticks, em-dashes,
        // raw text) above the first `---` line. The parser now skips
        // everything up to the first `^---` line and yields only the
        // YAML documents.
        let yaml = r#"# System — Title

Per-game structured reference data for the Game Info Panel.
Format reference: `docs/cores/SCHEMA.md`. v1 seed entries —
most coverage arrives in Phase 10 via the
`KNOWN_GAME_BUGS.md` migration pass.

# ============================================================================

---
id_key:
  system_id: psx
  rom_title: "Test Game"
date: 1999
"#;
        let out = parse_games_info_file(yaml);
        assert_eq!(out.len(), 1, "parser must skip pre-YAML prose and yield the one record");
        assert_eq!(out[0].id_key.rom_title.as_deref(), Some("Test Game"));
        assert_eq!(out[0].date, Some(1999));
    }

    #[test]
    fn parse_handles_content_starting_directly_with_separator() {
        // When the file starts with `---` (no leading whitespace or
        // prose), the parser must still yield the record. Common case
        // for clean machine-generated files.
        let yaml = r#"---
id_key:
  system_id: nes
  rom_title: "Direct Start"
"#;
        let out = parse_games_info_file(yaml);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id_key.rom_title.as_deref(), Some("Direct Start"));
    }

    #[test]
    fn parse_returns_empty_when_file_has_no_document_marker() {
        // Content with no `---` line at all — pure prose, no YAML
        // documents — yields zero records rather than hanging or
        // producing a spurious entry.
        let prose = "This is not a YAML file. No documents here.";
        assert!(parse_games_info_file(prose).is_empty());
    }

    #[test]
    fn load_from_dir_missing_path_returns_empty() {
        // Pointing at a non-existent dir produces an empty index, not
        // a crash. The "no game info data" fallback covers production
        // installs that haven't shipped docs/cores/ yet.
        let idx = GameInfoIndex::load_from_dir(Path::new("Z:/nonexistent/path/docs/cores"));
        assert!(idx.is_empty());
    }

    // ---- merge_game_info (Phase 4) -----------------------------------

    fn tomb_raider_file_record() -> GameInfo {
        parse_games_info_file(TOMB_RAIDER_YAML)
            .into_iter()
            .next()
            .expect("TOMB_RAIDER_YAML must parse")
    }

    #[test]
    fn merge_returns_none_when_both_layers_empty() {
        // No file record + empty override → None. UI hides the new
        // panel sections entirely.
        let merged = merge_game_info("psx", None, &GameInfoOverride::default());
        assert!(merged.is_none());
    }

    #[test]
    fn merge_facts_always_from_file_layer() {
        // Operator override can never set date/publisher/region/version/
        // player_count/genre in v1 (no fields on GameInfoOverride for
        // them). File-layer facts must surface unchanged when the
        // override is empty.
        let file = tomb_raider_file_record();
        let merged = merge_game_info("psx", Some(&file), &GameInfoOverride::default())
            .expect("file-only merge yields a record");
        assert_eq!(merged.date, Some(1996));
        assert_eq!(merged.publisher.as_deref(), Some("Eidos Interactive"));
        assert_eq!(merged.region.as_deref(), Some("USA"));
        assert_eq!(merged.version.as_deref(), Some("1.0"));
        assert_eq!(merged.player_count, Some(1));
        assert_eq!(merged.genre.as_deref(), Some("Action-Adventure"));
        // No local edits → status flags all false.
        assert!(!merged.has_local_edits);
        assert!(!merged.short_summary_is_local);
    }

    #[test]
    fn merge_narrative_override_wins_short_summary() {
        let file = tomb_raider_file_record();
        let ov = GameInfoOverride {
            short_summary: Some("Operator's own take.".into()),
            ..Default::default()
        };
        let merged = merge_game_info("psx", Some(&file), &ov).unwrap();
        assert_eq!(merged.short_summary.as_deref(), Some("Operator's own take."));
        assert!(merged.short_summary_is_local, "operator-set → is_local flag flips");
        assert!(merged.has_local_edits);
    }

    #[test]
    fn merge_narrative_falls_back_to_file_when_no_override() {
        // File has short_summary = ""; operator override is None for
        // the field. Merged value is Some("") (the file's empty string
        // surfaces — provenance preserved per the missing/null/empty
        // rule in plan §5). is_local stays false.
        let file = tomb_raider_file_record();
        assert_eq!(file.short_summary.as_deref(), Some(""));
        let merged =
            merge_game_info("psx", Some(&file), &GameInfoOverride::default()).unwrap();
        assert_eq!(merged.short_summary.as_deref(), Some(""));
        assert!(!merged.short_summary_is_local);
    }

    #[test]
    fn merge_controls_supported_override_replaces_file_list() {
        let file = tomb_raider_file_record();
        let ov = GameInfoOverride {
            controls_supported: Some(vec!["Mouse".into()]),
            ..Default::default()
        };
        let merged = merge_game_info("psx", Some(&file), &ov).unwrap();
        assert_eq!(merged.controls_supported, vec!["Mouse".to_string()]);
        // File-layer's "Standard gamepad" + "DualShock vibration" do
        // NOT show through — operator replaces, doesn't merge.
        assert!(!merged.controls_supported.contains(&"Standard gamepad".to_string()));
    }

    #[test]
    fn merge_best_emulator_override_supplies_both_fields() {
        // When the operator sets a recommended core in their override,
        // the file-layer recommendation is dropped entirely (override
        // owns both `recommended` + `reason`, even if reason is None).
        let file = tomb_raider_file_record();
        let ov = GameInfoOverride {
            best_emulator: Some("pcsx_rearmed_libretro.dll".into()),
            best_emulator_reason: Some("Operator prefers pcsx_rearmed on this title".into()),
            ..Default::default()
        };
        let merged = merge_game_info("psx", Some(&file), &ov).unwrap();
        let be = merged.best_emulator.expect("override surfaces best_emulator");
        assert_eq!(be.recommended, "pcsx_rearmed_libretro.dll");
        assert_eq!(
            be.reason.as_deref(),
            Some("Operator prefers pcsx_rearmed on this title")
        );
    }

    #[test]
    fn merge_bugs_override_replaces_file_list() {
        let file = tomb_raider_file_record();
        assert_eq!(file.bugs.len(), 2, "fixture has 2 bugs");
        let ov = GameInfoOverride {
            bugs: Some(vec![]), // operator says "no bugs reproduce on my install"
            ..Default::default()
        };
        let merged = merge_game_info("psx", Some(&file), &ov).unwrap();
        assert!(merged.bugs.is_empty(), "operator's empty list replaces file's bugs");
    }

    #[test]
    fn merge_works_with_override_only_no_file_record() {
        // Operator authors notes for a game with no file-layer entry
        // (homebrew, unindexed prototype). Merged record exists; facts
        // are all None.
        let ov = GameInfoOverride {
            short_summary: Some("Notes for this homebrew.".into()),
            applied_best_emulator: true,
            ..Default::default()
        };
        let merged = merge_game_info("nes", None, &ov).unwrap();
        assert_eq!(merged.system_id, "nes");
        assert!(merged.date.is_none());
        assert!(merged.publisher.is_none());
        assert_eq!(merged.short_summary.as_deref(), Some("Notes for this homebrew."));
        assert!(merged.short_summary_is_local);
        assert!(merged.has_local_edits);
        assert!(merged.applied_best_emulator);
        assert!(!merged.applied_controls);
    }

    #[test]
    fn merge_propagates_file_touch_hotspots() {
        // File has hotspots; override has none. V1 has no override
        // path for hotspots — the merged record carries the file's
        // hotspots unchanged. (Override path can land in v2 if the
        // operator wants per-install custom hotspots.)
        let mut file = tomb_raider_file_record();
        file.touch_hotspots = vec![TouchHotspot {
            label: "Map".to_string(),
            x: 8,
            y: 16,
            w: 80,
            h: 80,
        }];
        let merged = merge_game_info("psx", Some(&file), &GameInfoOverride::default())
            .expect("file-only merge yields a record");
        assert_eq!(merged.touch_hotspots.len(), 1);
        assert_eq!(merged.touch_hotspots[0].label, "Map");
        assert_eq!(merged.touch_hotspots[0].x, 8);
        assert_eq!(merged.touch_hotspots[0].w, 80);
    }

    #[test]
    fn merge_no_file_yields_empty_touch_hotspots() {
        // Override-only record (homebrew the operator annotated
        // locally). touch_hotspots stays empty because there's no
        // override path for it in v1.
        let ov = GameInfoOverride {
            short_summary: Some("Local note".into()),
            ..Default::default()
        };
        let merged = merge_game_info("nds", None, &ov).expect("merged record");
        assert!(merged.touch_hotspots.is_empty());
    }

    #[test]
    fn merge_applied_flags_pass_through_provenance() {
        // The applied_* flags on the override must echo into the merged
        // record without modification — the panel reads them to know
        // which Apply actions were used.
        let file = tomb_raider_file_record();
        let ov = GameInfoOverride {
            applied_best_emulator: true,
            applied_controls: true,
            ..Default::default()
        };
        let merged = merge_game_info("psx", Some(&file), &ov).unwrap();
        assert!(merged.applied_best_emulator);
        assert!(merged.applied_controls);
        // Setting the flags alone (without an explicit override value
        // for a field) still counts as "has_local_edits" — the
        // provenance marker is meaningful state.
        assert!(merged.has_local_edits);
    }

    #[test]
    fn roundtrip_serializes_then_parses_back_to_same() {
        // Authoring tools eventually emit YAML; the parser must round-
        // trip. Confirms the serde tags + skip_serializing_if rules
        // line up so an emitted file re-parses identically.
        let out = parse_games_info_file(TOMB_RAIDER_YAML);
        assert_eq!(out.len(), 1);
        let emitted = serde_yaml::to_string(&out[0]).expect("serialize ok");
        let reparsed = parse_games_info_file(&emitted);
        assert_eq!(reparsed.len(), 1);
        assert_eq!(reparsed[0], out[0]);
    }
}
