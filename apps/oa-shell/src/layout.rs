// Layout + presentation persistence. Same pattern as shell.json / audio.json /
// cores.json: simple file-backed JSON next to the other app prefs in
// appDataDir. Layout state survives WebView cache clears and is portable
// across machines.
//
// Two files:
//   appDataDir/layout.json        — sidebar widths, collapse state, widget
//                                    config. Per-presentation-mode is left
//                                    for a follow-up (this file persists the
//                                    currently-active mode's settings).
//   appDataDir/presentation.json  — active presentation mode.
//
// The frontend's `layout/state.ts` hydrates from these on mount and writes
// through on every change. Both commands are tolerant of missing / malformed
// files: return defaults so first launches don't surprise the UI.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutPrefs {
    pub left_sidebar_width: u32,
    pub left_sidebar_collapsed: bool,
    pub right_sidebar_width: u32,
    pub right_sidebar_visible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right_sidebar_pinned_game_id: Option<String>,
    pub widget_order: Vec<String>,
    pub widget_hidden: Vec<String>,
    // Phase 2.6 additions — defaulted so older layout.json files still parse.
    #[serde(default = "default_view_mode")]
    pub view_mode: String,
    #[serde(default = "default_sort_key")]
    pub sort_key: String,
    #[serde(default = "default_group_by")]
    pub group_by: String,
    /// User-defined ordering of system ids in the left sidebar. Empty = use
    /// the registry's natural order (Object.keys on the SystemId record).
    /// Drag-to-reorder writes here.
    #[serde(default)]
    pub system_order: Vec<String>,
    /// User-hidden system ids — these don't render in the left sidebar at
    /// all even though they're still in the registry. Mirrors widgetHidden.
    /// LaunchBox-equivalent surface: Settings → Library → Sidebar systems
    /// checkbox list. Right-click → "Hide system" pushes into this list.
    #[serde(default)]
    pub hidden_systems: Vec<String>,
    /// When true, the sidebar auto-hides any system with zero games in the
    /// library (on top of `hidden_systems`). Matches LaunchBox's "Empty
    /// Platforms" filter. Default true — fresh installs only show systems
    /// the user actually has games for; flipping off shows the full
    /// registry.
    #[serde(default = "default_auto_hide_empty")]
    pub auto_hide_empty_systems: bool,
    /// Per-window persisted geometry, keyed by Tauri window label
    /// ("library" / "game"). Empty map on first launch — the absence of
    /// a slot is the signal to maximize the library window. Game window
    /// stays at its compiled default size on first launch and only starts
    /// remembering after the operator resizes/moves it.
    #[serde(default)]
    pub windows: HashMap<String, WindowGeometry>,
    /// Target library tile width in pixels. Slider in the library header
    /// toolbar writes here. VirtualLibraryGrid applies hybrid ±20%
    /// scaling at render time so tiles fill the container cleanly at
    /// any window width.
    #[serde(default = "default_library_tile_size")]
    pub library_tile_size: u32,
}

/// Persisted window geometry. `x`/`y` are Option so first-launch / DPI-
/// change scenarios that don't have a known position can fall back to
/// Tauri's centering. `maximized` is captured separately so restore
/// preserves the maximize state across launches.
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WindowGeometry {
    pub width: u32,
    pub height: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<i32>,
    #[serde(default)]
    pub maximized: bool,
}

fn default_library_tile_size() -> u32 { 220 }

fn default_auto_hide_empty() -> bool { true }

fn default_view_mode() -> String { "capsule".to_string() }
fn default_sort_key() -> String { "title".to_string() }
fn default_group_by() -> String { "none".to_string() }

impl Default for LayoutPrefs {
    fn default() -> Self {
        Self {
            left_sidebar_width: 280,
            left_sidebar_collapsed: false,
            right_sidebar_width: 320,
            right_sidebar_visible: true,
            right_sidebar_pinned_game_id: None,
            widget_order: vec!["hero".into(), "title".into(), "metadata".into()],
            widget_hidden: Vec::new(),
            view_mode: default_view_mode(),
            sort_key: default_sort_key(),
            group_by: default_group_by(),
            system_order: Vec::new(),
            hidden_systems: Vec::new(),
            auto_hide_empty_systems: default_auto_hide_empty(),
            windows: HashMap::new(),
            library_tile_size: default_library_tile_size(),
        }
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PresentationMode {
    Desktop,
    Theater,
    Cabinet,
}

impl Default for PresentationMode {
    fn default() -> Self {
        Self::Desktop
    }
}

impl PresentationMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Desktop => "desktop",
            Self::Theater => "theater",
            Self::Cabinet => "cabinet",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "desktop" => Some(Self::Desktop),
            "theater" => Some(Self::Theater),
            "cabinet" => Some(Self::Cabinet),
            _ => None,
        }
    }
}

const LAYOUT_FILE: &str = "layout.json";
const PRESENTATION_FILE: &str = "presentation.json";

pub fn read_layout(app_data_dir: &Path) -> LayoutPrefs {
    let path = app_data_dir.join(LAYOUT_FILE);
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return LayoutPrefs::default(),
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn write_layout(app_data_dir: &Path, prefs: &LayoutPrefs) -> std::io::Result<()> {
    std::fs::create_dir_all(app_data_dir)?;
    let path = app_data_dir.join(LAYOUT_FILE);
    let body = serde_json::to_string_pretty(prefs)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, body)
}

/// Merge a freshly-captured window geometry into a previously-persisted
/// one. The key subtlety: when `current.maximized` is true, the inner
/// size we just captured is the maximized dimensions — not what we want
/// to "restore" to when the user unmaximizes. So we preserve the prior
/// restore size (width/height) and only flip the maximized flag.
///
/// When `current.maximized` is false, current IS the new restore baseline
/// — overwrite everything.
pub fn merge_geometry(prev: WindowGeometry, current: WindowGeometry) -> WindowGeometry {
    if current.maximized {
        WindowGeometry {
            width: prev.width,
            height: prev.height,
            x: current.x.or(prev.x),
            y: current.y.or(prev.y),
            maximized: true,
        }
    } else {
        current
    }
}

/// Read layout.json, merge a new window's geometry into the existing
/// slot for that label, write back. Logs and swallows I/O errors —
/// geometry persistence is best-effort and must not crash the shell.
pub fn persist_window_geometry(app_data_dir: &Path, label: &str, new_geom: WindowGeometry) {
    let mut prefs = read_layout(app_data_dir);
    let merged = match prefs.windows.get(label).cloned() {
        Some(existing) => merge_geometry(existing, new_geom),
        None => new_geom,
    };
    prefs.windows.insert(label.to_string(), merged);
    if let Err(e) = write_layout(app_data_dir, &prefs) {
        log::warn!("oa-shell: persist window geometry for {label} failed: {e}");
    }
}

pub fn read_presentation(app_data_dir: &Path) -> PresentationMode {
    let path = app_data_dir.join(PRESENTATION_FILE);
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return PresentationMode::default(),
    };
    // Stored as { "mode": "desktop" } for forward compat with future fields.
    let parsed: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return PresentationMode::default(),
    };
    parsed
        .get("mode")
        .and_then(|v| v.as_str())
        .and_then(PresentationMode::parse)
        .unwrap_or_default()
}

pub fn write_presentation(app_data_dir: &Path, mode: PresentationMode) -> std::io::Result<()> {
    std::fs::create_dir_all(app_data_dir)?;
    let path = app_data_dir.join(PRESENTATION_FILE);
    let body = serde_json::to_string_pretty(&serde_json::json!({
        "mode": mode.as_str(),
    }))
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presentation_round_trips() {
        assert_eq!(PresentationMode::parse("desktop"), Some(PresentationMode::Desktop));
        assert_eq!(PresentationMode::parse("theater"), Some(PresentationMode::Theater));
        assert_eq!(PresentationMode::parse("cabinet"), Some(PresentationMode::Cabinet));
        assert_eq!(PresentationMode::parse("nope"), None);
        assert_eq!(PresentationMode::Theater.as_str(), "theater");
    }

    #[test]
    fn layout_default_widgets() {
        let d = LayoutPrefs::default();
        assert_eq!(d.widget_order, vec!["hero", "title", "metadata"]);
        assert_eq!(d.left_sidebar_width, 280);
        assert_eq!(d.right_sidebar_width, 320);
        assert!(d.right_sidebar_visible);
        assert!(!d.left_sidebar_collapsed);
    }

    #[test]
    fn layout_missing_file_returns_default() {
        let tmp = std::env::temp_dir().join(format!("oa-layout-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let prefs = read_layout(&tmp);
        assert_eq!(prefs.left_sidebar_width, 280);
    }

    #[test]
    fn layout_round_trip_through_disk() {
        let tmp = std::env::temp_dir().join(format!("oa-layout-roundtrip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let mut prefs = LayoutPrefs::default();
        prefs.left_sidebar_width = 333;
        prefs.right_sidebar_visible = false;
        prefs.widget_hidden = vec!["metadata".into()];
        write_layout(&tmp, &prefs).expect("write");
        let read = read_layout(&tmp);
        assert_eq!(read.left_sidebar_width, 333);
        assert!(!read.right_sidebar_visible);
        assert_eq!(read.widget_hidden, vec!["metadata"]);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn default_library_tile_size_is_220() {
        assert_eq!(LayoutPrefs::default().library_tile_size, 220);
    }

    #[test]
    fn default_windows_map_is_empty() {
        assert!(LayoutPrefs::default().windows.is_empty());
    }

    #[test]
    fn windows_and_tile_size_round_trip() {
        let tmp = std::env::temp_dir().join(format!("oa-layout-windows-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let mut prefs = LayoutPrefs::default();
        prefs.library_tile_size = 280;
        prefs.windows.insert(
            "library".into(),
            WindowGeometry { width: 1600, height: 900, x: Some(100), y: Some(50), maximized: false },
        );
        prefs.windows.insert(
            "game".into(),
            WindowGeometry { width: 1024, height: 768, x: None, y: None, maximized: true },
        );
        write_layout(&tmp, &prefs).expect("write");
        let read = read_layout(&tmp);
        assert_eq!(read.library_tile_size, 280);
        assert_eq!(read.windows.len(), 2);
        let lib = read.windows.get("library").expect("library slot");
        assert_eq!(lib.width, 1600);
        assert_eq!(lib.x, Some(100));
        assert!(!lib.maximized);
        let game = read.windows.get("game").expect("game slot");
        assert!(game.maximized);
        assert_eq!(game.x, None);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn merge_keeps_restore_size_when_current_maximized() {
        let prev = WindowGeometry { width: 1000, height: 700, x: Some(50), y: Some(60), maximized: false };
        let current = WindowGeometry { width: 1920, height: 1080, x: Some(0), y: Some(0), maximized: true };
        let merged = merge_geometry(prev, current);
        assert_eq!(merged.width, 1000);
        assert_eq!(merged.height, 700);
        assert!(merged.maximized);
        // Position carried from current (the maximized position)
        assert_eq!(merged.x, Some(0));
    }

    #[test]
    fn merge_overwrites_when_current_unmaximized() {
        let prev = WindowGeometry { width: 1000, height: 700, x: Some(50), y: Some(60), maximized: true };
        let current = WindowGeometry { width: 1200, height: 800, x: Some(100), y: Some(120), maximized: false };
        let merged = merge_geometry(prev, current);
        assert_eq!(merged.width, 1200);
        assert_eq!(merged.height, 800);
        assert!(!merged.maximized);
        assert_eq!(merged.x, Some(100));
    }

    #[test]
    fn persist_window_geometry_writes_through() {
        let tmp = std::env::temp_dir().join(format!("oa-layout-persist-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        persist_window_geometry(
            &tmp,
            "library",
            WindowGeometry { width: 1280, height: 720, x: Some(10), y: Some(20), maximized: false },
        );
        let prefs = read_layout(&tmp);
        let lib = prefs.windows.get("library").expect("library slot");
        assert_eq!(lib.width, 1280);
        // Now persist a maximized state — restore size should NOT be clobbered
        persist_window_geometry(
            &tmp,
            "library",
            WindowGeometry { width: 1920, height: 1080, x: Some(0), y: Some(0), maximized: true },
        );
        let prefs = read_layout(&tmp);
        let lib = prefs.windows.get("library").expect("library slot");
        assert_eq!(lib.width, 1280);  // preserved
        assert!(lib.maximized);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn legacy_layout_json_without_new_fields_parses() {
        // Older installs have layout.json files written before these fields
        // existed. Verify they still parse with sensible defaults.
        let legacy = r#"{
            "leftSidebarWidth": 280,
            "leftSidebarCollapsed": false,
            "rightSidebarWidth": 320,
            "rightSidebarVisible": true,
            "widgetOrder": ["hero","title"],
            "widgetHidden": []
        }"#;
        let prefs: LayoutPrefs = serde_json::from_str(legacy).expect("legacy parse");
        assert_eq!(prefs.library_tile_size, 220);
        assert!(prefs.windows.is_empty());
        // sanity-check that the legacy fields still landed
        assert_eq!(prefs.left_sidebar_width, 280);
    }
}
