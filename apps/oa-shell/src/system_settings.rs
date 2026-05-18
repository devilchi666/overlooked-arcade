// Per-system settings persistence.
//
// Phase 2.8 slice C introduces the per-system settings page reached via the
// system page header `⚙` button. Each system gets its own JSON file under
// `appDataDir/systems/<system_id>.json` holding overrides that, when set,
// take precedence over the OA-wide preferences. Per-game overrides (slice D)
// will sit on top of these.
//
// Today's surface: scaling / window / monitor overrides. The per-system
// core override stays in `cores.json` (its own pre-existing store); the
// frontend bridges both stores transparently in the UI. Future overrides
// (audio device, region priority, shader preset) land as additional Option
// fields on `SystemSettings` — old files still parse because every field is
// already Option-wrapped and `#[serde(default)]`.
//
// Same file pattern as layout.rs / shell.rs / cores.rs: tolerant of missing
// or malformed files (returns defaults), pretty-print on write, one file per
// logical settings surface.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct SystemSettings {
    /// One of the ScalingMode strings from settings/store.ts. None = inherit
    /// the OA-wide default. Stored as a String here (not an enum) so we
    /// don't have to keep the Rust enum in sync with the frontend's union
    /// type — the frontend is the source of truth for the valid set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scaling_override: Option<String>,
    /// One of "windowed" | "borderless" | "fullscreen". None = inherit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_mode_override: Option<String>,
    /// 0-based monitor index. None = inherit (== "Current monitor" in OA UI).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monitor_index_override: Option<i32>,
    /// Phase 3 slice A — per-system shader preset name (looked up against
    /// the TOML registry; slice C). None = inherit the OA-wide default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shader_preset: Option<String>,
    /// Phase 3 slice C polish — per-system override for the Phosphor
    /// composite weight. None = use whatever the active preset's TOML
    /// `[params].bloom_amount` field carries (or whichever value the
    /// renderer currently holds if the preset doesn't specify). The
    /// override applies at launch time AFTER `set_shader_preset` so the
    /// override always wins over the TOML's default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bloom_amount: Option<f32>,
    /// Phase 4 slice A — per-system rewind enabled toggle. None = inherit
    /// the OA-wide default. Disabled by default since rewind has a non-zero
    /// RAM + CPU cost per frame.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rewind_enabled: Option<bool>,
    /// Phase 4 slice A — frames between snapshots. None = inherit.
    /// 1 = capture every frame; 6 = ~100 ms at 60 fps (default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rewind_capture_interval_frames: Option<u32>,
    /// Phase 4 slice A — rewind ring memory cap in MB. None = inherit.
    /// Cores with large states (SNES ~300 KB, Saturn ~3 MB) hold fewer
    /// seconds of history per MB; the UI surfaces seconds_held live.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rewind_buffer_megabytes: Option<u32>,
}

fn system_settings_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("systems")
}

fn system_settings_path(app_data_dir: &Path, system_id: &str) -> PathBuf {
    // System ids in the registry are short ASCII slugs (tg16, lynx, …). If
    // a future id ever contains path separators we'd want to sanitize; for
    // now the registry guarantees safe filenames.
    system_settings_dir(app_data_dir).join(format!("{system_id}.json"))
}

pub fn read_system_settings(app_data_dir: &Path, system_id: &str) -> SystemSettings {
    let path = system_settings_path(app_data_dir, system_id);
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return SystemSettings::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn write_system_settings(
    app_data_dir: &Path,
    system_id: &str,
    settings: &SystemSettings,
) -> std::io::Result<()> {
    let dir = system_settings_dir(app_data_dir);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{system_id}.json"));
    let body = serde_json::to_string_pretty(settings)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_all_none() {
        let d = SystemSettings::default();
        assert!(d.scaling_override.is_none());
        assert!(d.window_mode_override.is_none());
        assert!(d.monitor_index_override.is_none());
        assert!(d.shader_preset.is_none());
        assert!(d.bloom_amount.is_none());
        assert!(d.rewind_enabled.is_none());
        assert!(d.rewind_capture_interval_frames.is_none());
        assert!(d.rewind_buffer_megabytes.is_none());
    }

    #[test]
    fn missing_file_returns_default() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-syssettings-missing-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        let s = read_system_settings(&tmp, "tg16");
        assert_eq!(s, SystemSettings::default());
    }

    #[test]
    fn round_trip_through_disk() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-syssettings-roundtrip-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        let prefs = SystemSettings {
            scaling_override: Some("pixel-perfect".to_string()),
            window_mode_override: None,
            monitor_index_override: Some(1),
            shader_preset: Some("scanlines".to_string()),
            bloom_amount: Some(0.42),
            rewind_enabled: Some(true),
            rewind_capture_interval_frames: Some(6),
            rewind_buffer_megabytes: Some(32),
        };
        write_system_settings(&tmp, "tg16", &prefs).expect("write");
        let read = read_system_settings(&tmp, "tg16");
        assert_eq!(read, prefs);
        // Untouched system uses defaults.
        let untouched = read_system_settings(&tmp, "lynx");
        assert_eq!(untouched, SystemSettings::default());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn malformed_json_falls_back_to_default() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-syssettings-malformed-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("systems")).expect("mkdir");
        std::fs::write(tmp.join("systems").join("tg16.json"), "{not json").expect("write");
        assert_eq!(read_system_settings(&tmp, "tg16"), SystemSettings::default());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
