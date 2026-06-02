//! Per-system descriptor types — the runtime schema for `config/systems/<id>/`.
//!
//! Phase 1 of the per-system descriptor consolidation arc (Slice 1 of the
//! plan at `docs/PLANS/per-system-descriptors.md`). The arc unifies ~8
//! scattered per-system data sources (hardcoded Rust const tables for
//! BIOS hashes / core catalog / libretro-dat refs / light-gun systems
//! plus the in-tree `docs/cores/<id>/system-info.yaml` + `games-info.md`)
//! into a single per-system YAML triple:
//!
//! - `config/systems/<id>/system.yaml` — descriptor (cores, dat refs,
//!   extensions, default core, optional embedded System Info Panel L2
//!   block).
//! - `config/systems/<id>/bios.yaml` — BIOS hash list + `any_of` /
//!   `all_required` semantics. Optional (systems without BIOS omit it).
//! - `config/systems/<id>/games.yaml` — per-game records, same shape as
//!   the existing [`crate::game_info::GameInfo`]. Optional.
//!
//! Layer model (formalized in Slice 1, wired across slices):
//!
//! | Layer | Source | Editable by |
//! | --- | --- | --- |
//! | **L1** | Rust const fallback (existing const tables) | Code change |
//! | **L2** | `<repo>/config/systems/<id>/` — what these types describe | OA dev + PRs |
//! | **L3** | `<appDataDir>/content-packs/<pack>/systems/<id>/` (Slice 3) | Pack publisher |
//! | **L4** | SQLite operator overrides (existing) | Operator via UI |
//!
//! Slice 1 ships L2 + keeps L1 as the fallback for the 38 unmigrated
//! systems. Slice 2 sweeps the remaining systems + deletes the L1
//! const tables. Slice 3 adds L3 layer-merge + JSON Schema generation.
//!
//! This module owns the *types* and pure-string parsers. The loader
//! (directory walk + hot-fail + lookup surface) lives in
//! [`crate::system_registry`].
//!
//! ## Design notes
//!
//! - **`deny_unknown_fields`** on every struct so contributors get a
//!   loud serde error at load time when a YAML key is misspelled —
//!   silent fallthrough would be a long-tail source of bugs.
//! - **`schema_version`** on every top-level file (default 1) so future
//!   breaking changes can bump it without re-parsing existing files.
//! - **Optional `light_gun` + `input`** blocks are intentionally NOT in
//!   the v1 schema — those data sources (`light_gun_systems::LIGHT_GUN_SYSTEMS`
//!   + `main.rs::DEVICE_ID_OPTIONS_*`) migrate in Slice 2 alongside the
//!   38 remaining systems, at which point we have empirical pilot
//!   feedback on field naming before locking the shape.
//! - **Embedded [`SystemInfoCurated`](crate::system_info::SystemInfoCurated)**
//!   for the L2 System Info Panel block — same struct the existing
//!   `docs/cores/<id>/system-info.yaml` files deserialize into, just
//!   nested under `system_info:` in the new file. The redundant
//!   `system_id` field inside the embedded block is validated at load
//!   time against the descriptor's `id` field; mismatch is a hot-fail
//!   (catches operator-renamed folders that forgot to update the
//!   nested key).

#![allow(dead_code)] // scaffolding for Slice 1; consumers wire in Phases B-D

use serde::{Deserialize, Serialize};

use crate::game_info::GameInfo;
use crate::system_info::SystemInfoCurated;

/// Schema version embedded in every top-level file (`system.yaml`,
/// `bios.yaml`, `games.yaml`). Bump only on a breaking field change;
/// consumers should reject `schema_version > CURRENT_SCHEMA_VERSION`
/// rather than silently rendering possibly-malformed data.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

// =====================================================================
// system.yaml — top-level system descriptor
// =====================================================================

/// One system's authoritative descriptor — the shape
/// `config/systems/<id>/system.yaml` deserializes into.
///
/// Field grouping mirrors the consumers that read each chunk: identity
/// (`id` / `display_name` / `short_name`); engine defaults
/// (`default_core` / `default_shader_preset`); ROM discovery
/// (`extensions` / `libretro_dat_refs`); installer (`cores`); panel
/// data (`system_info`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct SystemDescriptor {
    /// OA system slug — must match the parent folder name + the
    /// `parse_system_id` arms in `bindings.rs`.
    pub id: String,

    /// Operator-facing display name shown in the sidebar / library
    /// header / per-system Settings drill-in ("Game Boy", "PlayStation").
    pub display_name: String,

    /// Optional short / abbreviated form ("GB", "PSX") for tight UI
    /// surfaces. Defaults to `display_name` when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_name: Option<String>,

    /// File-level schema version. Defaults to 1 when absent so handwritten
    /// YAMLs without an explicit version parse cleanly.
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,

    /// Default libretro core .dll / .so / .dylib filename for this
    /// system. Matches the entry the operator can override via per-system
    /// Settings → Cores. None for systems whose default lives outside
    /// the registry (engine launchers like ScummVM until Slice 2
    /// migrates them).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_core: Option<String>,

    /// Default shader-preset slug (matches frontend
    /// `themes/registry.ts::defaultShaderPreset`). Reserved for the
    /// kiosk-mode theme migration; not consumed by Slice 1 (the
    /// frontend theme registry stays as TS const for now per plan
    /// §"Out of scope").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_shader_preset: Option<String>,

    /// File extensions OA accepts for this system's ROMs (lowercase,
    /// no leading dot). Mirrors what the smart-scan classifier reads
    /// for its Extension confidence tier. Empty `Vec` means "no raw
    /// ROM files — engine-launcher shape (ScummVM `.scummvm` files /
    /// DOSBox directory paths)."
    #[serde(default)]
    pub extensions: Vec<String>,

    /// libretro-database `.dat` references. Pairs of (subdir, basename)
    /// — the URL gets composed at fetch time by
    /// [`crate::rom_hashes::fetch_libretro_dat`]. Empty `Vec` means
    /// "no upstream dat for this system" (engine launchers, arcade
    /// platforms that match by zip basename rather than file hash).
    #[serde(default)]
    pub libretro_dat_refs: Vec<LibretroDatRef>,

    /// Cores compatible with this system. Source of truth for the
    /// per-system installer + the future Phase 2 CPU-tier picks.
    /// Mirror of the entries the old global `core_installer::CATALOG`
    /// array filtered for this system_id. Multi-system cores (e.g.
    /// Genesis Plus GX serves sms + gamegear + genesis + segacd)
    /// appear in each of their systems' descriptors — duplication is
    /// accepted in v1 of the schema; Slice 3 may revisit if the
    /// authoring cost becomes painful.
    #[serde(default)]
    pub cores: Vec<CoreEntryDescriptor>,

    /// Embedded L2 System Info Panel block — the same shape as the
    /// existing `docs/cores/<id>/system-info.yaml` files, nested under
    /// the `system_info:` key in the new file. `None` for systems
    /// without hand-authored panel data (~36 of 41 systems pre-Slice
    /// 1); those still get MAME L1 + operator L4 via the existing
    /// `system_info` module.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_info: Option<SystemInfoCurated>,
}

/// One libretro-database `.dat` reference — pair of subdir +
/// basename. Mirrors the runtime [`crate::rom_hashes::DatRef`] shape
/// (just owned `String`s instead of `&'static str` since these come
/// from YAML).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct LibretroDatRef {
    /// Folder under `libretro/libretro-database/` ("metadat/no-intro",
    /// "metadat/redump", "metadat/headered"). See
    /// `crate::rom_hashes::libretro_dat_refs_for_system` for the
    /// existing arm-by-arm assignments.
    pub subdir: String,
    /// `.dat` basename without extension ("Sony - PlayStation",
    /// "Nintendo - Game Boy", "Atari - Lynx").
    pub basename: String,
}

/// One core installer catalog entry — mirror of the existing
/// [`crate::core_installer::CatalogEntry`] (just owned `String`s).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct CoreEntryDescriptor {
    /// Buildbot basename (`mednafen_pce_fast_libretro`,
    /// `gambatte_libretro`). The runtime filename is derived from this
    /// plus the host's dylib extension.
    pub base: String,
    /// Operator-facing display name ("Beetle PCE Fast", "Gambatte").
    pub display_name: String,
    /// Short blurb shown under the title in the installer UI.
    pub blurb: String,
    /// True for OA-tested / first-pick cores for the system. The UI
    /// renders a "recommended" chip when set.
    #[serde(default)]
    pub recommended: bool,
    /// Required BIOS / firmware filename(s) (relative to
    /// `<exe_dir>/system/`) when the core needs one. Surfaced as a
    /// warning chip before install. None for self-contained cores.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bios_required: Option<String>,
}

// =====================================================================
// bios.yaml — per-system BIOS table
// =====================================================================

/// One system's BIOS file inventory + semantics. Optional file in the
/// per-system folder — systems without BIOS (GB, NES, SNES) omit it.
///
/// Replaces the per-system `*_BIOS_KNOWN_HASHES` const tables in
/// `apps/oa-shell/src/main.rs`. Same data + same semantics, just
/// editable + content-packable.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct BiosDescriptor {
    /// File-level schema version.
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    /// How the loader translates the per-file inventory into the
    /// overall readiness verdict. See [`BiosSemanticsYaml`].
    pub semantics: BiosSemanticsYaml,
    /// True when missing BIOS hard-blocks launch (PSX). False when
    /// optional / nice-to-have (some Atari 7800 / GBA configurations).
    /// Defaults to true since most systems with a `bios.yaml` need it
    /// for normal operation.
    #[serde(default = "default_true")]
    pub required_for_launch: bool,
    /// Canonical BIOS files for the system. Order matters for the
    /// readiness-checklist row order in the per-file pill expansion
    /// (Slice 5 of Phase 1B).
    pub files: Vec<BiosFileEntry>,
    /// Short prose pointing operators at where to legally acquire the
    /// BIOS ("Dump from your own PSX"). Renders in the BIOS pill
    /// tooltip; OA never ships BIOSes itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sourcing_hint: Option<String>,
}

/// Per-system semantics for translating the per-file inventory into an
/// overall readiness verdict. Mirror of the private
/// [`BiosSemantics`](crate::BiosSemantics) enum in `main.rs`; defined
/// here as a public YAML-facing enum because the runtime enum stays
/// crate-private until Slice 2 swaps it out wholesale.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BiosSemanticsYaml {
    /// Any one canonical-hash match satisfies the system (regional
    /// variants — PSX, Saturn, Sega CD, PC Engine CD, 3DO, Neo Geo CD,
    /// most cart-BIOS systems).
    AnyOf,
    /// Every entry must be present + canonical (NDS bios7 + bios9 +
    /// firmware; Intellivision exec + grom).
    AllRequired,
}

/// One canonical BIOS file entry. The runtime
/// [`crate::BiosFile`](crate::BiosFile) struct carries additional
/// on-disk state (`on_disk: BiosFileStatus`) that's computed by
/// `scan_bios_table`; this YAML shape is the static descriptor only.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct BiosFileEntry {
    /// Filename OA expects at `<exe_dir>/system/<name>`.
    pub name: String,
    /// Canonical SHA-1, uppercase hex (40 chars). Matches the format
    /// `scan_bios_table` compares against.
    pub sha1: String,
    /// Free-form description shown next to the row in the readiness
    /// checklist's per-file pill expansion ("US PSX BIOS v3.0
    /// (SCPH-5501, 1995, most common NA)"). Empty string when no
    /// description.
    #[serde(default)]
    pub description: String,
    /// When true, `derive_bios_overall` treats this file as satisfied
    /// when Missing on disk (only its hash-match state contributes to
    /// the OkCanonical vs OkUnknownHash distinction). Used by Channel
    /// F's `sl90025.bin` (the 1978 Channel F II revision ROM —
    /// recognized but not required for the launch ROM pair to count
    /// as valid).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
}

// =====================================================================
// games.yaml — per-game records
// =====================================================================

/// One system's per-game records. Wraps the same
/// [`crate::game_info::GameInfo`] shape the existing
/// `docs/cores/<id>/games-info.md` files parse into; the migration is
/// a file move + a schema_version envelope, NOT a reshape.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct GamesDescriptor {
    /// File-level schema version.
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    /// Per-game records. Order matches the source file; consumers
    /// re-index by `id_key.rom_hash` / `id_key.rom_title` for lookup.
    #[serde(default)]
    pub games: Vec<GameInfo>,
}

// =====================================================================
// Defaults helpers — referenced by `#[serde(default = "…")]`
// =====================================================================

fn default_schema_version() -> u32 {
    CURRENT_SCHEMA_VERSION
}

fn default_true() -> bool {
    true
}

// =====================================================================
// Parsers
// =====================================================================

/// Parse `system.yaml` content. Returns a descriptive error including
/// the serde path on failure so contributors can find the offending
/// field quickly.
pub fn parse_system_yaml(content: &str) -> Result<SystemDescriptor, String> {
    serde_yaml::from_str(content).map_err(|e| format!("parse system.yaml: {e}"))
}

/// Parse `bios.yaml` content.
pub fn parse_bios_yaml(content: &str) -> Result<BiosDescriptor, String> {
    serde_yaml::from_str(content).map_err(|e| format!("parse bios.yaml: {e}"))
}

/// Parse `games.yaml` content.
pub fn parse_games_yaml(content: &str) -> Result<GamesDescriptor, String> {
    serde_yaml::from_str(content).map_err(|e| format!("parse games.yaml: {e}"))
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- system.yaml -----------------------------------------------

    #[test]
    fn parse_system_yaml_minimal_record() {
        // Bare-minimum YAML — only the two mandatory fields. Everything
        // else should default cleanly.
        let yaml = r#"
id: gb
display_name: Game Boy
"#;
        let desc = parse_system_yaml(yaml).unwrap();
        assert_eq!(desc.id, "gb");
        assert_eq!(desc.display_name, "Game Boy");
        assert!(desc.short_name.is_none());
        assert_eq!(desc.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(desc.default_core.is_none());
        assert!(desc.extensions.is_empty());
        assert!(desc.cores.is_empty());
        assert!(desc.system_info.is_none());
    }

    #[test]
    fn parse_system_yaml_full_record_with_embedded_panel_data() {
        // Realistic-shape YAML matching what the GB pilot will write.
        // Exercises embedded SystemInfoCurated via the `system_info:`
        // nested block.
        let yaml = r#"
id: gb
display_name: Game Boy
short_name: GB
default_core: gambatte_libretro.dll
extensions:
  - gb
libretro_dat_refs:
  - subdir: metadat/no-intro
    basename: Nintendo - Game Boy
cores:
  - base: gambatte_libretro
    display_name: Gambatte
    blurb: "GB / GBC — very high accuracy."
    recommended: true
  - base: sameboy_libretro
    display_name: SameBoy
    blurb: "Highest-accuracy GB / GBC core."
    recommended: false
system_info:
  system_id: gb
  manufacturer: Nintendo
  type: Handheld
  blurb: "The handheld that took gaming on the road."
"#;
        let desc = parse_system_yaml(yaml).unwrap();
        assert_eq!(desc.id, "gb");
        assert_eq!(desc.short_name.as_deref(), Some("GB"));
        assert_eq!(desc.default_core.as_deref(), Some("gambatte_libretro.dll"));
        assert_eq!(desc.extensions, vec!["gb".to_string()]);
        assert_eq!(desc.libretro_dat_refs.len(), 1);
        assert_eq!(desc.libretro_dat_refs[0].basename, "Nintendo - Game Boy");
        assert_eq!(desc.cores.len(), 2);
        assert!(desc.cores[0].recommended);
        assert!(!desc.cores[1].recommended);
        let panel = desc.system_info.as_ref().expect("system_info populated");
        assert_eq!(panel.manufacturer.as_deref(), Some("Nintendo"));
        assert_eq!(panel.system_type.as_deref(), Some("Handheld"));
    }

    #[test]
    fn parse_system_yaml_rejects_unknown_field() {
        // deny_unknown_fields guard — a typo in `extentions` (sic) must
        // produce a serde error rather than silently populating an empty
        // extensions list.
        let yaml = r#"
id: gb
display_name: Game Boy
extentions: [gb]
"#;
        let err = parse_system_yaml(yaml).unwrap_err();
        assert!(
            err.contains("extentions") || err.contains("unknown field"),
            "expected serde unknown-field error, got: {err}"
        );
    }

    #[test]
    fn parse_system_yaml_rejects_missing_mandatory_fields() {
        // Missing `display_name` — must error.
        let yaml = "id: gb\n";
        assert!(parse_system_yaml(yaml).is_err());
        // Missing `id` — must error.
        let yaml = "display_name: Game Boy\n";
        assert!(parse_system_yaml(yaml).is_err());
    }

    // ---- bios.yaml -------------------------------------------------

    #[test]
    fn parse_bios_yaml_any_of_realistic() {
        // PSX-shaped: any_of semantics with multiple regional variants.
        let yaml = r#"
semantics: any_of
required_for_launch: true
files:
  - name: scph5500.bin
    sha1: B05DEF971D8EC59F346F2D9AC21FB742E3EB6917
    description: "JP PSX BIOS v3.0 (SCPH-5500, 1995)"
  - name: scph5501.bin
    sha1: 0555C6FAE8906F3F09BAF5988F00E55F88E9F30B
    description: "US PSX BIOS v3.0 (SCPH-5501)"
sourcing_hint: "Dump from your own PSX."
"#;
        let bios = parse_bios_yaml(yaml).unwrap();
        assert_eq!(bios.semantics, BiosSemanticsYaml::AnyOf);
        assert!(bios.required_for_launch);
        assert_eq!(bios.files.len(), 2);
        assert_eq!(bios.files[0].name, "scph5500.bin");
        assert_eq!(bios.files[0].sha1.len(), 40);
        assert!(!bios.files[0].optional);
        assert_eq!(bios.sourcing_hint.as_deref(), Some("Dump from your own PSX."));
    }

    #[test]
    fn parse_bios_yaml_all_required_with_optional_flag() {
        // NDS-shape (3 required files) + an optional bonus file with
        // the optional flag set (mirrors Channel F's sl90025.bin
        // pattern).
        let yaml = r#"
semantics: all_required
files:
  - name: bios7.bin
    sha1: 24F67BDEA115A2C847C8813A262502EE1607B7DF
    description: "DS ARM7 BIOS (16 KB)"
  - name: bios9.bin
    sha1: BFAAC75F101C135E32E2AAF541DE6B1BE4C8C62D
    description: "DS ARM9 BIOS (4 KB)"
  - name: firmware.bin
    sha1: CFE072921EE3FB93F688743F8BEEF89043C3E9AD
    description: "DS Firmware (256 KB)"
  - name: optional_extras.bin
    sha1: 0000000000000000000000000000000000000000
    description: "hypothetical optional bonus"
    optional: true
"#;
        let bios = parse_bios_yaml(yaml).unwrap();
        assert_eq!(bios.semantics, BiosSemanticsYaml::AllRequired);
        // required_for_launch defaults to true when absent.
        assert!(bios.required_for_launch);
        assert_eq!(bios.files.len(), 4);
        assert!(!bios.files[0].optional);
        assert!(bios.files[3].optional);
    }

    #[test]
    fn parse_bios_yaml_rejects_unknown_semantics() {
        let yaml = r#"
semantics: maybe_one
files: []
"#;
        let err = parse_bios_yaml(yaml).unwrap_err();
        assert!(
            err.contains("unknown variant") || err.contains("maybe_one"),
            "expected serde variant error, got: {err}"
        );
    }

    // ---- games.yaml ------------------------------------------------

    #[test]
    fn parse_games_yaml_with_one_record() {
        // GameInfo shape is owned + tested by `game_info`; here we just
        // confirm the envelope deserializes and propagates the inner
        // record.
        let yaml = r#"
schema_version: 1
games:
  - id_key:
      system_id: psx
      rom_title: "Test Game"
    date: 1999
"#;
        let games = parse_games_yaml(yaml).unwrap();
        assert_eq!(games.schema_version, 1);
        assert_eq!(games.games.len(), 1);
        assert_eq!(games.games[0].id_key.system_id, "psx");
        assert_eq!(games.games[0].date, Some(1999));
    }

    #[test]
    fn parse_games_yaml_empty_list() {
        // System with no curated games yet — empty games list parses
        // cleanly. (Distinct from "no games.yaml file at all", which
        // the loader handles by treating the file as absent.)
        let yaml = "games: []\n";
        let games = parse_games_yaml(yaml).unwrap();
        assert_eq!(games.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(games.games.is_empty());
    }
}
