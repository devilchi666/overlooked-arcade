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
const CURRENT_SCHEMA_VERSION: u32 = 2;

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
    /// SystemIds the operator has deleted from this view via the editor.
    /// v3.5.3's reconciler consults this list before auto-extending
    /// shipped views with newly-registered systems — operator-deleted
    /// systems stay deleted instead of coming back on next launch.
    /// Optional + serde-default for forward/back compat: v1 configs
    /// without the field hydrate cleanly with an empty list.
    #[serde(default)]
    pub explicitly_removed: Vec<String>,
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
    /// Unified Navigation Tree Slice 1 — membership is an explicit set of
    /// games, keyed by `collection_id` into the custom-collections store.
    /// The frontend resolver turns this into a `{ games, romIds }`
    /// membership; Rust only stores + round-trips it. The enum-level
    /// `rename_all` renames the variant tag ("collection"); the field is
    /// renamed explicitly so the JSON key is `collectionId` to match the
    /// TS mirror.
    Collection {
        #[serde(rename = "collectionId")]
        collection_id: String,
    },
    /// Reserved (D59) — predicate over games (Favorites / Recently Played
    /// / dump-quality). Inert in Slice 1; the shape settles in Slice 2.
    /// `spec` is opaque JSON so the predicate AST can land later without a
    /// schema bump.
    Filter { spec: serde_json::Value },
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

/// Schema migration ladder.
///
/// v1 → v2: every view gains `explicitly_removed: Vec<String>`. Serde
/// default on the field handles the on-disk migration silently — old
/// configs deserialize with an empty list. The version bump here is
/// the explicit semantic flag the reconciler consults to know that
/// post-migration code can rely on the field's presence.
fn migrate_inplace(config: &mut ViewsConfig) {
    if config.schema_version < 2 {
        // Field already populated via serde default on read; nothing
        // structural to do beyond bumping the version flag.
        config.schema_version = 2;
    }
    if config.schema_version < CURRENT_SCHEMA_VERSION {
        // Future migrations slot in here.
        config.schema_version = CURRENT_SCHEMA_VERSION;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> ViewsConfig {
        ViewsConfig {
            schema_version: CURRENT_SCHEMA_VERSION,
            active_view_id: "default-formfactor".to_string(),
            banner_dismissed: false,
            views: vec![View {
                id: "default-formfactor".to_string(),
                name: "Platforms".to_string(),
                kind: ViewKind::UserBuiltin,
                expanded_nodes: vec!["root".to_string(), "container:console".to_string()],
                explicitly_removed: Vec::new(),
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
        assert!(json.contains(&format!("\"schemaVersion\":{}", CURRENT_SCHEMA_VERSION)));
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

    #[test]
    fn views_v1_config_migrates_to_v2_with_empty_explicitly_removed() {
        // Forward-compat: v1 configs lacking `explicitlyRemoved` on each
        // view deserialize with empty lists (serde default), then
        // migrate_inplace bumps the schema_version flag to 2.
        let json = r#"{
            "schemaVersion": 1,
            "activeViewId": "default-formfactor",
            "views": [
                {
                    "id": "default-formfactor",
                    "name": "Platforms",
                    "kind": "user-builtin",
                    "expandedNodes": ["root"],
                    "root": {
                        "id": "root",
                        "label": "Platforms",
                        "rule": null,
                        "accent": null,
                        "art": null,
                        "hidden": false,
                        "children": []
                    }
                }
            ]
        }"#;
        let mut config: ViewsConfig = serde_json::from_str(json).expect("parse v1");
        assert_eq!(config.schema_version, 1);
        assert_eq!(config.views[0].explicitly_removed.len(), 0);
        migrate_inplace(&mut config);
        assert_eq!(config.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(config.views[0].explicitly_removed.len(), 0);
    }

    #[test]
    fn views_collection_and_filter_rules_round_trip() {
        // Unified Navigation Tree Slice 1 — the new `collection` + reserved
        // `filter` rule kinds survive a write→read cycle and serialize with
        // the camelCase tag + `collectionId` field the TS mirror expects.
        let mut cfg = sample_config();
        cfg.views[0].root.children.push(ViewNode::Container(ContainerNode {
            id: "container:collections".to_string(),
            label: "Collections".to_string(),
            rule: None,
            accent: None,
            art: None,
            hidden: false,
            children: vec![
                ViewNode::Container(ContainerNode {
                    id: "collection:favorites".to_string(),
                    label: "Favorites".to_string(),
                    rule: Some(ContainerRule::Collection {
                        collection_id: "favorites".to_string(),
                    }),
                    accent: None,
                    art: None,
                    hidden: false,
                    children: vec![],
                }),
                ViewNode::Container(ContainerNode {
                    id: "filter:recent".to_string(),
                    label: "Recently Played".to_string(),
                    rule: Some(ContainerRule::Filter {
                        spec: serde_json::json!({ "kind": "recentlyPlayed" }),
                    }),
                    accent: None,
                    art: None,
                    hidden: false,
                    children: vec![],
                }),
            ],
        }));

        // JSON shape matches the TS contract.
        let json = serde_json::to_string(&cfg).expect("serialize");
        assert!(json.contains("\"kind\":\"collection\""));
        assert!(json.contains("\"collectionId\":\"favorites\""));
        assert!(json.contains("\"kind\":\"filter\""));

        // Disk round-trip preserves both rule kinds.
        let tmp =
            std::env::temp_dir().join(format!("oa-views-collection-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        write_views(&tmp, &cfg).expect("write");
        let read = read_views(&tmp).expect("read");
        let collections = match &read.views[0].root.children[1] {
            ViewNode::Container(c) => c,
            other => panic!("expected collections container, got {:?}", other),
        };
        match &collections.children[0] {
            ViewNode::Container(c) => match &c.rule {
                Some(ContainerRule::Collection { collection_id }) => {
                    assert_eq!(collection_id, "favorites")
                }
                other => panic!("expected collection rule, got {:?}", other),
            },
            other => panic!("expected collection node, got {:?}", other),
        }
        match &collections.children[1] {
            ViewNode::Container(c) => match &c.rule {
                Some(ContainerRule::Filter { spec }) => {
                    assert_eq!(spec["kind"], "recentlyPlayed")
                }
                other => panic!("expected filter rule, got {:?}", other),
            },
            other => panic!("expected filter node, got {:?}", other),
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn views_v2_explicitly_removed_round_trips() {
        let mut cfg = sample_config();
        cfg.views[0].explicitly_removed = vec!["nes".to_string(), "snes".to_string()];
        let tmp = std::env::temp_dir().join(format!("oa-views-v2-roundtrip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        write_views(&tmp, &cfg).expect("write");
        let read = read_views(&tmp).expect("read");
        assert_eq!(read.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(read.views[0].explicitly_removed, vec!["nes".to_string(), "snes".to_string()]);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
