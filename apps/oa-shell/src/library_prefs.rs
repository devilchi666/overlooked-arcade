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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPrefs {
    /// Ordered list of region names (matching `title_parse`'s canonical
    /// region strings). Default order is USA > World > Europe > Japan >
    /// Asia > Other (the catch-all). The resolver scans this list to
    /// find the first region that any variant of a group provides.
    pub region_priority: Vec<String>,
    pub revision_priority: RevisionPriority,
}

impl Default for LibraryPrefs {
    fn default() -> Self {
        Self {
            region_priority: default_region_priority(),
            revision_priority: RevisionPriority::Newest,
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
        };
        write_library_prefs(&tmp, &prefs).expect("write");
        assert_eq!(read_library_prefs(&tmp), prefs);
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
