//! Content-pack preferences — the runtime config the pack manager reads
//! before it ever touches the network.
//!
//! Two things live here, and the *why* matters:
//!
//! - **`registry_url`** — where OA fetches `registry.json` from. Per
//!   decision **CP1** this is a **runtime config value, never a
//!   compile-time constant**: OA seeds it with the content-packs.md §4
//!   default but the operator (or a future federation UI) can repoint it
//!   at any host without a code change. Hosting is the most deferrable
//!   decision; treating the URL as data keeps every distribution choice
//!   reversible.
//! - **`allow_network`** — the master "Allow network calls" toggle
//!   (content-packs.md §9). Defaults ON but is one flip from OFF; when OFF
//!   every network-touching pack command refuses synchronously before any
//!   call goes out. The Privacy panel that surfaces this lands in Slice 4;
//!   the pref + the gate it drives live here from Slice 2 so the gate is
//!   testable now.
//!
//! Same file pattern as `library_prefs.rs` / `layout.rs`: one JSON file at
//! `appDataDir/packs/prefs.json`, tolerant of a missing or malformed file
//! (returns defaults), pretty-printed on write, serde-default on every
//! field added after v1 so older files migrate cleanly.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The content-packs.md §4 default registry location. **A seed, not a
/// constant the code depends on** (CP1) — it's only ever used to populate
/// a fresh `prefs.json`; once written, the persisted value wins and the
/// operator can change it. The `overlooked-arcade` org need not exist yet
/// (CP1); nothing fetches this until the operator clicks Browse.
pub const DEFAULT_REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/overlooked-arcade/oa-pack-registry/main/registry.json";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PacksPrefs {
    /// Where `registry.json` is fetched from. Seeded with
    /// [`DEFAULT_REGISTRY_URL`]; operator-overridable (CP1).
    pub registry_url: String,
    /// Master network toggle (content-packs.md §9). ON by default.
    pub allow_network: bool,
    /// ISO-8601 timestamp of the last successful registry fetch, for the
    /// "Last checked: …" line in the Packs panel (Slice 3). `None` until
    /// the first fetch.
    #[serde(default)]
    pub last_checked: Option<String>,
}

impl Default for PacksPrefs {
    fn default() -> Self {
        Self {
            registry_url: DEFAULT_REGISTRY_URL.to_string(),
            allow_network: true,
            last_checked: None,
        }
    }
}

fn packs_prefs_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("packs").join("prefs.json")
}

pub fn read_packs_prefs(app_data_dir: &Path) -> PacksPrefs {
    let path = packs_prefs_path(app_data_dir);
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return PacksPrefs::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn write_packs_prefs(app_data_dir: &Path, prefs: &PacksPrefs) -> std::io::Result<()> {
    let path = packs_prefs_path(app_data_dir);
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

    fn tmp(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "oa-packsprefs-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn default_seeds_registry_url_and_network_on() {
        let prefs = PacksPrefs::default();
        assert_eq!(prefs.registry_url, DEFAULT_REGISTRY_URL);
        assert!(prefs.allow_network);
        assert!(prefs.last_checked.is_none());
    }

    #[test]
    fn missing_file_returns_default() {
        let dir = tmp("missing");
        assert_eq!(read_packs_prefs(&dir), PacksPrefs::default());
    }

    #[test]
    fn round_trip_through_disk() {
        let dir = tmp("roundtrip");
        let prefs = PacksPrefs {
            registry_url: "https://example.invalid/my-registry.json".into(),
            allow_network: false,
            last_checked: Some("2026-06-16T00:00:00Z".into()),
        };
        write_packs_prefs(&dir, &prefs).expect("write");
        assert_eq!(read_packs_prefs(&dir), prefs);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn legacy_file_without_last_checked_migrates() {
        let dir = tmp("legacy");
        std::fs::create_dir_all(dir.join("packs")).unwrap();
        let legacy = r#"{"registryUrl":"https://example.invalid/r.json","allowNetwork":true}"#;
        std::fs::write(dir.join("packs").join("prefs.json"), legacy).unwrap();
        let loaded = read_packs_prefs(&dir);
        assert_eq!(loaded.registry_url, "https://example.invalid/r.json");
        assert!(loaded.last_checked.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn malformed_json_falls_back_to_default() {
        let dir = tmp("malformed");
        std::fs::create_dir_all(dir.join("packs")).unwrap();
        std::fs::write(dir.join("packs").join("prefs.json"), "{not json").unwrap();
        assert_eq!(read_packs_prefs(&dir), PacksPrefs::default());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
