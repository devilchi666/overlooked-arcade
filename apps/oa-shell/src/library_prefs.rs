//! OA-wide preferences that drive the library aggregation + default
//! variant resolution. Region + revision priority lists are consulted
//! by `library_groups::resolve_default_variant` to pick the default
//! variant of a multi-dump game group.
//!
//! Per-system overrides live on `SystemSettings` (region_priority_override
//! / revision_priority_override fields). Per-group overrides live in the
//! `game_group_defaults` SQLite table.
//!
//! Same file pattern as `shell.rs` / `audio.rs` / `layout.rs`: one JSON
//! file at `appDataDir/library/prefs.json`, tolerant of missing or
//! malformed files (returns defaults), pretty-printed on write.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Newest- vs oldest-revision-wins tie-breaker between two variants of
/// the same group + region.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RevisionPriority {
    /// `Castlevania (USA) (Rev 1)` beats `Castlevania (USA)`. Default —
    /// most users want the latest official revision.
    #[default]
    Newest,
    /// `Castlevania (USA)` beats `Castlevania (USA) (Rev 1)`. Useful
    /// for preservation-minded libraries that prefer the original
    /// release.
    Oldest,
}

/// Match strictness for disc-shape per-track SHA-1 identification.
/// Plan-locked at three steps; default Strict per the plan §"Match
/// strictness". File-persisted alongside the rest of LibraryPrefs.
/// Maps to [`crate::disc_track_hash::Strictness`] at the call site.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DiscTrackStrictness {
    /// Every data track must match. Default — silent green
    /// confidence pill on success.
    #[default]
    Strict,
    /// At least 80% of data tracks must match. Shows the ⚠ partial-
    /// match badge on the library tile when it passes via this mode
    /// rather than Strict.
    Threshold80,
    /// At least one data track matches. Most permissive; same partial-
    /// match badge.
    Lenient,
}

impl DiscTrackStrictness {
    /// Translate the persistence enum into the matcher's runtime enum.
    /// Threshold80 fixes the percentage at 80 per the plan; if v2 ever
    /// exposes a custom percentage, add a `Threshold(u8)` variant
    /// here with a `pct: u8` field.
    pub fn to_engine(self) -> crate::disc_track_hash::Strictness {
        match self {
            Self::Strict => crate::disc_track_hash::Strictness::Strict,
            Self::Threshold80 => crate::disc_track_hash::Strictness::Threshold(80),
            Self::Lenient => crate::disc_track_hash::Strictness::Lenient,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPrefs {
    /// Ordered list of region names (matching `title_parse`'s canonical
    /// region strings). Default order is USA > World > Europe > Japan >
    /// Asia > Other (the catch-all). The resolver scans this list to
    /// find the first region that any variant of a group provides.
    pub region_priority: Vec<String>,
    pub revision_priority: RevisionPriority,
    /// Phase A1 — strictness mode for disc-shape per-track SHA-1
    /// identification. Default [`DiscTrackStrictness::Strict`]; serde
    /// default makes prefs files written before this field shipped
    /// migrate cleanly.
    #[serde(default)]
    pub disc_track_strictness: DiscTrackStrictness,
    /// Phase A1 pivot 2026-06-03 — gate per-track SHA-1 disc
    /// identification behind an experimental flag. Default OFF after
    /// operator playtest revealed two architectural limitations:
    ///
    /// 1. `chdman extractcd` produces .bin files that differ from
    ///    redump's source DiscImageCreator dumps (verified 225-frame
    ///    offset on Dreamcast GD-ROM). Per-track SHA-1s computed via
    ///    the CHD path never match redump's published catalog.
    /// 2. Archived disc images (.cue inside .zip) skip per-track
    ///    entirely (Sub-phase 3 deferral). Most operator libraries
    ///    are archived to save space.
    ///
    /// Result: 0% per-track match rate on real-world libraries.
    /// The architecture is correct for raw, unarchived
    /// DiscImageCreator-shape dumps — a niche of operators. Filename-
    /// based fuzzy matching is the new primary identification path
    /// (cheap, works on any container). Per-track stays available
    /// for the niche behind this flag. See `docs/PLANS/disc-track-
    /// sha1-matching.md` for the pivot decision.
    #[serde(default)]
    pub disc_track_experimental_enabled: bool,
}

impl Default for LibraryPrefs {
    fn default() -> Self {
        Self {
            region_priority: default_region_priority(),
            revision_priority: RevisionPriority::Newest,
            disc_track_strictness: DiscTrackStrictness::default(),
            disc_track_experimental_enabled: false,
        }
    }
}

/// Canonical default region priority. Confirmed in design discussion:
/// USA > World > Europe > Japan > Asia > Other.
pub fn default_region_priority() -> Vec<String> {
    vec![
        "USA".to_string(),
        "World".to_string(),
        "Europe".to_string(),
        "Japan".to_string(),
        "Asia".to_string(),
        "Other".to_string(),
    ]
}

fn library_prefs_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("library").join("prefs.json")
}

pub fn read_library_prefs(app_data_dir: &Path) -> LibraryPrefs {
    let path = library_prefs_path(app_data_dir);
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return LibraryPrefs::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn write_library_prefs(
    app_data_dir: &Path,
    prefs: &LibraryPrefs,
) -> std::io::Result<()> {
    let path = library_prefs_path(app_data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_string_pretty(prefs)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_region_order_is_usa_first() {
        let prefs = LibraryPrefs::default();
        assert_eq!(prefs.region_priority[0], "USA");
        assert_eq!(prefs.region_priority[1], "World");
        assert_eq!(prefs.region_priority[2], "Europe");
        assert_eq!(prefs.region_priority[3], "Japan");
        assert_eq!(prefs.revision_priority, RevisionPriority::Newest);
    }

    #[test]
    fn missing_file_returns_default() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-libprefs-missing-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        assert_eq!(read_library_prefs(&tmp), LibraryPrefs::default());
    }

    #[test]
    fn round_trip_through_disk() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-libprefs-roundtrip-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        let prefs = LibraryPrefs {
            region_priority: vec!["Japan".to_string(), "USA".to_string()],
            revision_priority: RevisionPriority::Oldest,
            disc_track_strictness: DiscTrackStrictness::Threshold80,
            disc_track_experimental_enabled: true,
        };
        write_library_prefs(&tmp, &prefs).expect("write");
        assert_eq!(read_library_prefs(&tmp), prefs);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn disc_track_experimental_default_is_off_and_serde_round_trips() {
        // Per the Phase A1 pivot (2026-06-03), per-track SHA-1 is
        // OFF by default. Legacy prefs.json files (without this field)
        // must hydrate to false via serde default.
        let tmp = std::env::temp_dir().join(format!(
            "oa-libprefs-experimental-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        // Default is OFF.
        assert!(!LibraryPrefs::default().disc_track_experimental_enabled);
        // Legacy prefs.json without this field hydrates to false.
        std::fs::create_dir_all(tmp.join("library")).expect("mkdir");
        let legacy = r#"{"regionPriority":["USA"],"revisionPriority":"newest","discTrackStrictness":"strict"}"#;
        std::fs::write(tmp.join("library").join("prefs.json"), legacy).expect("write");
        let loaded = read_library_prefs(&tmp);
        assert!(!loaded.disc_track_experimental_enabled);
        // Round-trip ON survives.
        let prefs_on = LibraryPrefs {
            disc_track_experimental_enabled: true,
            ..LibraryPrefs::default()
        };
        write_library_prefs(&tmp, &prefs_on).expect("write on");
        let reloaded = read_library_prefs(&tmp);
        assert!(reloaded.disc_track_experimental_enabled);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn disc_track_strictness_serde_round_trip() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-libprefs-strictness-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        // Default is Strict.
        let defaults = LibraryPrefs::default();
        assert_eq!(defaults.disc_track_strictness, DiscTrackStrictness::Strict);
        // Round-trip each variant through disk.
        for variant in [
            DiscTrackStrictness::Strict,
            DiscTrackStrictness::Threshold80,
            DiscTrackStrictness::Lenient,
        ] {
            let _ = std::fs::remove_dir_all(&tmp);
            let prefs = LibraryPrefs {
                disc_track_strictness: variant,
                ..LibraryPrefs::default()
            };
            write_library_prefs(&tmp, &prefs).expect("write");
            assert_eq!(read_library_prefs(&tmp).disc_track_strictness, variant);
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn legacy_prefs_without_strictness_field_deserializes_with_default() {
        // Simulate a prefs.json written by a build before this field
        // shipped. The serde default should hydrate disc_track_strictness
        // to Strict without erroring.
        let tmp = std::env::temp_dir().join(format!(
            "oa-libprefs-legacy-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("library")).expect("mkdir");
        let legacy = r#"{"regionPriority":["USA","Europe"],"revisionPriority":"newest"}"#;
        std::fs::write(tmp.join("library").join("prefs.json"), legacy).expect("write");
        let loaded = read_library_prefs(&tmp);
        assert_eq!(loaded.region_priority, vec!["USA", "Europe"]);
        assert_eq!(loaded.revision_priority, RevisionPriority::Newest);
        assert_eq!(loaded.disc_track_strictness, DiscTrackStrictness::Strict);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn malformed_json_falls_back_to_default() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-libprefs-malformed-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("library")).expect("mkdir");
        std::fs::write(tmp.join("library").join("prefs.json"), "{not json").expect("write");
        assert_eq!(read_library_prefs(&tmp), LibraryPrefs::default());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
