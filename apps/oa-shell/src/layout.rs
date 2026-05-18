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
}

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
}
