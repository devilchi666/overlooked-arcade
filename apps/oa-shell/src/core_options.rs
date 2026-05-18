//! Per-system core-options persistence (RetroArch parity, slice 1).
//!
//! Each system gets a file at `appDataDir/core-options/<system_id>.json`
//! holding both the SCHEMA captured at the last core load (so the
//! per-system settings page can render the option list even when no
//! game is running) AND the user's overridden VALUES. Per-game overrides
//! live in `games.core_options_json` (SQLite, library_db) and overlay
//! this map at launch time.
//!
//! The schema's stored representation mirrors `oa_core::CoreOption`'s
//! shape but uses owned Strings throughout so the file is self-contained.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use oa_core::{CoreOption, CoreOptionCategory, CoreOptionValue};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CoreOptionsFile {
    /// Snapshot of the option definitions captured the last time a core
    /// for this system was loaded. Empty until first launch.
    #[serde(default)]
    pub schema: Vec<CoreOption>,
    /// V2 categories (empty for V1 / legacy schemas).
    #[serde(default)]
    pub categories: Vec<CoreOptionCategory>,
    /// User-overridden values keyed by `CoreOption::key`. Unset keys
    /// fall back to the schema's `default_value`.
    #[serde(default)]
    pub values: HashMap<String, String>,
}

fn core_options_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("core-options")
}

fn core_options_path(app_data_dir: &Path, system_id: &str) -> PathBuf {
    core_options_dir(app_data_dir).join(format!("{system_id}.json"))
}

pub fn read(app_data_dir: &Path, system_id: &str) -> CoreOptionsFile {
    let path = core_options_path(app_data_dir, system_id);
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return CoreOptionsFile::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn write(
    app_data_dir: &Path,
    system_id: &str,
    file: &CoreOptionsFile,
) -> std::io::Result<()> {
    let dir = core_options_dir(app_data_dir);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{system_id}.json"));
    let body = serde_json::to_string_pretty(file)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, body)
}

/// Refresh the schema half of the per-system file from a freshly-loaded
/// core's option list. Preserves any user-set values whose keys still
/// exist in the new schema; drops values whose keys no longer exist (a
/// core update can remove options).
pub fn refresh_schema(
    app_data_dir: &Path,
    system_id: &str,
    schema: Vec<CoreOption>,
    categories: Vec<CoreOptionCategory>,
) -> std::io::Result<()> {
    let mut file = read(app_data_dir, system_id);
    let new_keys: std::collections::HashSet<&str> =
        schema.iter().map(|o| o.key.as_str()).collect();
    file.values.retain(|k, _| new_keys.contains(k.as_str()));
    file.schema = schema;
    file.categories = categories;
    write(app_data_dir, system_id, &file)
}

/// Resolve the effective value for an option using the inheritance
/// chain: per-game override → per-system override → schema default.
/// Returns None if the key isn't in the schema at all.
///
/// Today's production launch path uses [`build_effective_values`] (the
/// bulk form). This single-key resolver is kept around because the
/// inheritance-semantics test exercises it directly and a future
/// "explain why this option is its current value" diagnostic surface
/// would call through here.
#[allow(dead_code)]
pub fn resolve_value(
    schema: &[CoreOption],
    system_values: &HashMap<String, String>,
    game_values: &HashMap<String, String>,
    key: &str,
) -> Option<String> {
    if let Some(v) = game_values.get(key) {
        return Some(v.clone());
    }
    if let Some(v) = system_values.get(key) {
        return Some(v.clone());
    }
    schema.iter().find(|o| o.key == key).map(|o| o.default_value.clone())
}

/// Build the full effective value map for a system/game launch — every
/// key in the schema with the inherited value. The emu thread receives
/// this map and pushes each (key, value) into the core via `set_option`.
pub fn build_effective_values(
    schema: &[CoreOption],
    system_values: &HashMap<String, String>,
    game_values: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut out = HashMap::with_capacity(schema.len());
    for opt in schema {
        let v = game_values
            .get(&opt.key)
            .or_else(|| system_values.get(&opt.key))
            .cloned()
            .unwrap_or_else(|| opt.default_value.clone());
        out.insert(opt.key.clone(), v);
    }
    out
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreOptionsSnapshot {
    pub schema: Vec<CoreOption>,
    pub categories: Vec<CoreOptionCategory>,
    pub system_values: HashMap<String, String>,
    pub game_values: HashMap<String, String>,
}

// Re-export so `serde(default)` round-trips with the same shape oa_core uses.
#[allow(dead_code)] // used implicitly through CoreOption + CoreOptionValue
fn _types_visible(_: CoreOptionValue) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn opt(key: &str, default: &str, values: &[&str]) -> CoreOption {
        CoreOption {
            key: key.into(),
            desc: format!("{key} desc"),
            info: None,
            category_key: None,
            default_value: default.into(),
            values: values.iter().map(|v| CoreOptionValue { value: (*v).into(), label: None }).collect(),
        }
    }

    #[test]
    fn resolve_value_walks_inheritance_chain() {
        let schema = vec![opt("a", "1", &["1", "2", "3"]), opt("b", "x", &["x", "y"])];
        let sys: HashMap<String, String> = [("a", "2")].iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        let game: HashMap<String, String> = [("a", "3")].iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        assert_eq!(resolve_value(&schema, &sys, &game, "a"), Some("3".into()));
        assert_eq!(resolve_value(&schema, &sys, &HashMap::new(), "a"), Some("2".into()));
        assert_eq!(resolve_value(&schema, &HashMap::new(), &HashMap::new(), "a"), Some("1".into()));
        assert_eq!(resolve_value(&schema, &HashMap::new(), &HashMap::new(), "b"), Some("x".into()));
        assert_eq!(resolve_value(&schema, &HashMap::new(), &HashMap::new(), "missing"), None);
    }

    #[test]
    fn build_effective_values_covers_every_schema_key() {
        let schema = vec![opt("a", "1", &["1", "2"]), opt("b", "x", &["x", "y"])];
        let sys: HashMap<String, String> = [("a", "2")].iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        let merged = build_effective_values(&schema, &sys, &HashMap::new());
        assert_eq!(merged.len(), 2);
        assert_eq!(merged.get("a"), Some(&"2".to_string()));
        assert_eq!(merged.get("b"), Some(&"x".to_string())); // default
    }

    #[test]
    fn refresh_schema_drops_stale_keys() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-core-options-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        // Seed with a value for a key that the next schema won't carry.
        let initial = CoreOptionsFile {
            schema: vec![opt("a", "1", &["1", "2"]), opt("old", "x", &["x", "y"])],
            categories: Vec::new(),
            values: [("a", "2"), ("old", "y")]
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        };
        write(&tmp, "tg16", &initial).expect("seed write");

        // New schema drops "old".
        refresh_schema(&tmp, "tg16", vec![opt("a", "1", &["1", "2"])], Vec::new())
            .expect("refresh");
        let after = read(&tmp, "tg16");
        assert_eq!(after.schema.len(), 1);
        assert_eq!(after.values.get("a"), Some(&"2".to_string()));
        assert!(after.values.get("old").is_none(), "stale key dropped");
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
