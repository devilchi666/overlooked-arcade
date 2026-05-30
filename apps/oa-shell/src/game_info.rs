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

    /// File-level metadata (schema version, last-updated, contributors).
    /// Defaults populated when fields are absent (schema_version = 1).
    #[serde(default)]
    pub meta: GameInfoMeta,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Sample matching the Tomb Raider example in
    /// `docs/PLANS/game-info-panel.md` §5. Used as the golden record
    /// for "all fields populated parses cleanly."
    const TOMB_RAIDER_YAML: &str = r#"
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
        let yaml = r#"
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
        // meta defaults populated even when absent.
        assert_eq!(gi.meta.schema_version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn parse_multi_document_yields_all_records_in_order() {
        // Two real documents separated by the YAML `---` marker.
        // Order matters for callers that want stable iteration.
        let yaml = format!(
            "{}\n---\n{}",
            TOMB_RAIDER_YAML,
            r#"
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
            "{}\n---\nthis_is_not_a_game_record: true\n---\n{}",
            TOMB_RAIDER_YAML,
            r#"
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
