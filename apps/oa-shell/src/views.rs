// Views config persistence — appDataDir/views.json.
//
// Sister file to layout.json / presentation.json (read on demand, written
// through on every change). Schema is richer: each view defines a tree of
// container + platform nodes that the sidebar projects the library through.
// See docs/SIDEBAR_TIER_PLAN.md §2.1 for the locked schema and
// docs/KIOSK_PLAN.md §3.3 for the future kiosk-shell consumer.
//
// Writes are atomic (tempfile + rename) — layout.json uses plain fs::write
// because a truncated layout file just resets to defaults; views.json
// carries more user state (custom views, expanded sets, per-node hide
// flags) so the atomic-write extra cost is worth paying to guarantee a
// power-loss mid-write can't corrupt the file.
//
// read_views returns `Option<ViewsConfig>` (not a defaulted struct like
// layout.rs) because the frontend distinguishes three states on first
// launch: file present → load it; file absent + legacy `layout.systemOrder`
// → seed default + Flat-Legacy and show migration banner; file absent +
// no legacy order → seed default only. The None signal lets the frontend
// pick the right migration path.

use std::path::Path;

use serde::{Deserialize, Serialize};

const VIEWS_FILE: &str = "views.json";
const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewsConfig {
    pub schema_version: u32,
    pub active_view_id: String,
    pub views: Vec<View>,
    /// PR-γ's migration banner dismissal flag. Optional for forward compat
    /// — older configs without the field default to `false` (banner
    /// eligible if active view is Flat-Legacy).
    #[serde(default)]
    pub banner_dismissed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub id: String,
    pub name: String,
    pub kind: ViewKind,
    /// Node ids the operator has expanded in the sidebar tree. Separate
    /// from `hidden` (which lives inline on each node) so the diff size on
    /// every expand-toggle stays tiny.
    pub expanded_nodes: Vec<String>,
    pub root: ContainerNode,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ViewKind {
    /// Shipped default (e.g. "Platforms" form-factor view) or migration-
    /// seeded view ("Flat (Legacy)"). User-visible but the operator
    /// doesn't own its lifecycle — replaced on app upgrade if the shape
    /// changes. Operator can still hide containers / reorder children,
    /// just not delete the view outright.
    UserBuiltin,
    /// Theme-shipped advisory view — appears while the theme is active,
    /// disappears when the theme switches. Not used in v1; schema slot
    /// reserved for the kiosk theme work.
    ThemeShipped,
    /// User-authored persistent view. v3 surface; no UI to create one in
    /// v1.
    UserBuilt,
}

/// Tagged enum serialized with `kind` discriminant: `{"kind": "container", ...}`
/// or `{"kind": "platform", ...}`. Matches the frontend `ViewNode` type
/// in frontend/src/views/types.ts.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ViewNode {
    Container(ContainerNode),
    Platform(PlatformNode),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerNode {
    pub id: String,
    pub label: String,
    /// `None` (serialized as JSON null) on the root container — "match
    /// everything." Leaf-level container rules drive the runtime filter
    /// when an operator clicks the container row.
    #[serde(default)]
    pub rule: Option<ContainerRule>,
    /// Per-container accent override. v1 ships every container with `None`
    /// (neutral); v3's category-editor UI populates this.
    #[serde(default)]
    pub accent: Option<String>,
    /// Reserved slot for per-container art (banner / logo / clear-logo)
    /// — populated by theme work in vN. Held as a free-form JSON value so
    /// the schema doesn't need a v2 bump when the art shape lands.
    #[serde(default)]
    pub art: Option<serde_json::Value>,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub children: Vec<ViewNode>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformNode {
    pub id: String,
    pub system_id: String,
    #[serde(default)]
    pub hidden: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ContainerRule {
    /// Match systems whose registry `formFactor` tag equals `value`.
    FormFactor { value: String },
    /// Match systems whose registry `manufacturer` tag equals `value`.
    /// Unused in v1 — pre-emptive groundwork for v2's Manufacturer view.
    Manufacturer { value: String },
    /// Match an explicit set of system ids. Used by v3+ custom containers.
    SystemIds { values: Vec<String> },
}

/// Read views.json from `app_data_dir`. Returns `None` when the file
/// doesn't exist or fails to parse — the frontend's ViewsStore seeds
/// defaults on `None` (fresh install vs upgrade path decided by checking
/// `layout.systemOrder` separately).
pub fn read_views(app_data_dir: &Path) -> Option<ViewsConfig> {
    let path = app_data_dir.join(VIEWS_FILE);
    let raw = std::fs::read_to_string(&path).ok()?;
    let mut config: ViewsConfig = serde_json::from_str(&raw).ok()?;
    migrate_inplace(&mut config);
    Some(config)
}

/// Write views.json atomically: serialize to a sibling `.tmp` file then
/// rename onto the target. On both Windows and POSIX, rename is atomic
/// when source + destination share a filesystem (which they always do
/// here — both live under `app_data_dir`).
pub fn write_views(app_data_dir: &Path, config: &ViewsConfig) -> std::io::Result<()> {
    std::fs::create_dir_all(app_data_dir)?;
    let path = app_data_dir.join(VIEWS_FILE);
    let tmp = app_data_dir.join("views.json.tmp");
    let body = serde_json::to_string_pretty(config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&tmp, body)?;
    std::fs::rename(&tmp, &path)
}

/// No-op today; lands real migration steps when the schema bumps past v1.
fn migrate_inplace(config: &mut ViewsConfig) {
    if config.schema_version < CURRENT_SCHEMA_VERSION {
        config.schema_version = CURRENT_SCHEMA_VERSION;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> ViewsConfig {
        ViewsConfig {
            schema_version: 1,
            active_view_id: "default-formfactor".to_string(),
            banner_dismissed: false,
            views: vec![View {
                id: "default-formfactor".to_string(),
                name: "Platforms".to_string(),
                kind: ViewKind::UserBuiltin,
                expanded_nodes: vec!["root".to_string(), "container:console".to_string()],
                root: ContainerNode {
                    id: "root".to_string(),
                    label: "Platforms".to_string(),
                    rule: None,
                    accent: None,
                    art: None,
                    hidden: false,
                    children: vec![ViewNode::Container(ContainerNode {
                        id: "container:console".to_string(),
                        label: "Consoles".to_string(),
                        rule: Some(ContainerRule::FormFactor {
                            value: "console".to_string(),
                        }),
                        accent: None,
                        art: None,
                        hidden: false,
                        children: vec![ViewNode::Platform(PlatformNode {
                            id: "platform:nes".to_string(),
                            system_id: "nes".to_string(),
                            hidden: false,
                        })],
                    })],
                },
            }],
        }
    }

    #[test]
    fn views_missing_file_returns_none() {
        let tmp = std::env::temp_dir().join(format!("oa-views-missing-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        assert!(read_views(&tmp).is_none());
    }

    #[test]
    fn views_round_trips_through_disk() {
        let tmp = std::env::temp_dir().join(format!("oa-views-roundtrip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let cfg = sample_config();
        write_views(&tmp, &cfg).expect("write");
        let read = read_views(&tmp).expect("read");
        assert_eq!(read.active_view_id, "default-formfactor");
        assert_eq!(read.views.len(), 1);
        assert_eq!(read.views[0].name, "Platforms");
        assert_eq!(read.views[0].kind, ViewKind::UserBuiltin);
        match &read.views[0].root.children[0] {
            ViewNode::Container(c) => {
                assert_eq!(c.id, "container:console");
                match &c.rule {
                    Some(ContainerRule::FormFactor { value }) => assert_eq!(value, "console"),
                    other => panic!("unexpected rule: {:?}", other),
                }
            }
            other => panic!("expected container, got {:?}", other),
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn views_serializes_with_camelcase_and_tag_discriminant() {
        let cfg = sample_config();
        let json = serde_json::to_string(&cfg).expect("serialize");
        assert!(json.contains("\"schemaVersion\":1"));
        assert!(json.contains("\"activeViewId\":\"default-formfactor\""));
        assert!(json.contains("\"expandedNodes\""));
        assert!(json.contains("\"user-builtin\""));
        assert!(json.contains("\"kind\":\"container\""));
        assert!(json.contains("\"kind\":\"platform\""));
        assert!(json.contains("\"kind\":\"formFactor\""));
        assert!(json.contains("\"value\":\"console\""));
        assert!(json.contains("\"systemId\":\"nes\""));
    }

    #[test]
    fn views_atomic_write_leaves_no_tmp_file_on_success() {
        let tmp = std::env::temp_dir().join(format!("oa-views-atomic-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        write_views(&tmp, &sample_config()).expect("write");
        assert!(tmp.join("views.json").exists());
        assert!(!tmp.join("views.json.tmp").exists());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn views_missing_banner_dismissed_field_defaults_false() {
        // Forward-compat: older configs lacking bannerDismissed parse OK.
        let json = r#"{
            "schemaVersion": 1,
            "activeViewId": "x",
            "views": []
        }"#;
        let config: ViewsConfig = serde_json::from_str(json).expect("parse");
        assert!(!config.banner_dismissed);
    }
}
