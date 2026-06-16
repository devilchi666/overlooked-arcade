//! On-disk theme loader — `.oatheme` runtime themes, declarative-first
//! (Theming ARC 2 "P", docs/PLANS/theming-oatheme-loader.md, slice P.1 S1).
//!
//! OA discovers themes that live **on disk** (next to the exe, distributed via
//! the content-pack channel) rather than baked into the Vite bundle. This
//! module is the Rust half: it walks the on-disk layout, parses each theme's
//! `theme.toml` (+ optional `tokens.toml` / `per-system.toml`) into a
//! [`DiskThemeDescriptor`], and exposes the lot through the
//! [`oa_themes_list_disk`] Tauri command. There is **no frontend consumer, no
//! rendering, and no `DeclarativeShell`** yet — that's slice P.1 S2.
//!
//! ## Declarative-only (decision PD1 / D44)
//!
//! Runtime themes are **data, never code**: the structs below mirror the
//! frontend's declarative theme contract (`frontend/src/platform/theme/
//! {manifest,tokens}.ts`) but carry **no `entry`/`entry_export`** — those are
//! implicit (a built-in `DeclarativeShell` renders every disk theme; PD3/D46).
//! The serde field names track the TS keys 1:1 so the parsed document maps
//! straight onto `ThemeManifest` / `ThemeTokens` / `perSystemTokens` on the
//! frontend with no casing transform (manifest keys are snake_case; token keys
//! are camelCase — same split as the TS types).
//!
//! ## On-disk layout (decision PD2 / D45)
//!
//! ```text
//! <exe_dir>/themes/community/<theme_id>/
//! ├── manifest.yml      # oa-packs pack identity (type: themes) — the channel reads this
//! ├── theme.toml        # the theme definition (mirrors ThemeManifest, minus entry/*)
//! ├── tokens.toml       # optional — ThemeTokens overrides
//! ├── per-system.toml   # optional — perSystemTokens (per-system palette overrides)
//! └── system-ui/        # optional — backgrounds / sounds (S5.1 cascade), read in a later slice
//! ```
//!
//! `<type>/community/<id>` is the exact shape the pack channel installs to
//! (CP2), with `themes` as the pack `type` (CP3/PD4) — so a `themes` pack
//! dropped on the channel lands where this loader looks, for free. A loose
//! `<exe_dir>/themes/dev/<id>/` path is **reserved** for hand-dropped dev
//! themes; it is scanned at startup but hot-reload is deliberately NOT wired
//! (swap-by-restart is the shipped model — hot-reload is pure dev ergonomics,
//! deferred until it earns its keep).
//!
//! ## Never fatal
//!
//! Mirrors the [`crate::emulator_profiles`] / [`crate::packs`] loaders: a
//! missing directory yields an empty list, and one malformed `theme.toml` is
//! logged + skipped so it can't take down its siblings or the shell. A
//! malformed *optional* file (`tokens.toml` / `per-system.toml`) is logged and
//! that layer is dropped — the theme still loads without it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// theme.toml — mirrors frontend ThemeManifest (manifest.ts), minus entry/*.
// Keys are snake_case, matching the TS manifest field names verbatim.
// ---------------------------------------------------------------------------

/// The parsed `theme.toml`. Field set mirrors the TS `ThemeManifest` declarative
/// surface — **except `entry`/`entry_export`**, which are implicit for a
/// declarative theme (the loader supplies `DeclarativeShell`; PD1/PD3).
///
/// The six fields with no `#[serde(default)]` are required: a file missing any
/// of them isn't a usable theme manifest and is skipped (logged). Everything
/// else defaults so a minimal theme stays terse; the frontend `validateTheme()`
/// (run on disk themes in S3) is the authority on the looser rules
/// (non-empty `surfaces`, `default_route ∈ routes`, …).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiskThemeManifest {
    /// Stable identifier — directory-safe, lowercase (e.g. `"coverflow"`).
    pub id: String,
    /// Display name shown in Settings → Appearance.
    pub name: String,
    /// Theme's own semver (e.g. `"1.0.0"`).
    pub version: String,
    /// Manifest schema revision this file targets (current is `1`).
    pub schema_version: u32,
    /// Semver range of the OA shell the theme supports (e.g. `"^0.x"`).
    pub oa_version: String,
    /// Route id the shell navigates to on theme mount; must appear in `routes`.
    pub default_route: String,
    /// Route ids the theme registers.
    #[serde(default)]
    pub routes: Vec<String>,
    /// Context slices the theme consumes from the engine-provided ThemeContext.
    #[serde(default)]
    pub context_slots: Vec<String>,
    /// Engine capabilities the theme refuses to run without (`[]` = runs
    /// anywhere — the only valid value while ARC 1 advertises none).
    #[serde(default)]
    pub required_engine_capabilities: Vec<String>,
    /// Corner reserved for the engine summon icon. Always `"top-right"` in
    /// ARC 1; defaulted so a theme needn't restate the one legal value.
    #[serde(default = "default_reserved_corner")]
    pub reserves_corner: String,
    /// Named surfaces the theme renders (ARC 1 themes declare `["main"]`).
    #[serde(default)]
    pub surfaces: Vec<String>,
    /// Controller-glyph set the HintBar paints (`"xbox"` | `"playstation"`);
    /// omit to inherit the default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glyph_set: Option<String>,
    /// Per-system UI consumption opt-in (ARC 2 L1 / D33).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub per_system_ui: Option<PerSystemUiFlags>,
    /// Per-view layout map (ARC 2 L2 / D32): which layout primitive each
    /// library-journey view mounts, optionally varied per system.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub views: Option<BTreeMap<String, ViewLayoutConfig>>,
    /// Declarative appearance/options the engine renders generically.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings_schema: Option<Vec<ThemeSettingControl>>,
}

fn default_reserved_corner() -> String {
    "top-right".to_string()
}

/// `per_system_ui` opt-in flags — `{ tiles?, sfx? }` booleans (D33/D34).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerSystemUiFlags {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tiles: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sfx: Option<bool>,
}

/// A `views` entry — a default `layout` and/or `per_system` overrides
/// (mirrors the TS `ViewLayoutConfig`; both halves optional per L3b/D40).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewLayoutConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub per_system: Option<BTreeMap<String, String>>,
}

/// One `settings_schema` control — the discriminated union mirrors the TS
/// `ThemeSettingControl` (`toggle` | `slider` | `select`). Internally tagged on
/// `type`, so the serialized JSON carries `"type": "toggle"` exactly as the
/// frontend renderer + `validateTheme()` expect.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ThemeSettingControl {
    Toggle {
        key: String,
        label: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hint: Option<String>,
        default: bool,
    },
    Slider {
        key: String,
        label: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hint: Option<String>,
        default: f64,
        min: f64,
        max: f64,
        step: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        unit: Option<String>,
    },
    Select {
        key: String,
        label: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hint: Option<String>,
        default: String,
        options: Vec<SelectOption>,
    },
}

/// One `{ value, label }` choice in a `select` control.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

// ---------------------------------------------------------------------------
// tokens.toml — mirrors frontend ThemeTokens (tokens.ts). Keys are camelCase
// (matching the TS type), so the parsed map drops straight onto a
// Partial<ThemeTokens>. Every field optional — a theme overrides any subset.
// ---------------------------------------------------------------------------

/// Optional `tokens.toml` — design-token overrides mirroring the TS
/// `Partial<ThemeTokens>`. Token keys are camelCase (the TS contract's casing),
/// so e.g. `bgDeep = "..."` in the file deserializes into `bg_deep` and
/// re-serializes to `bgDeep` for the frontend. Omitted keys inherit the
/// `:root` defaults.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DiskThemeTokens {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bg_deep: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ink: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ink_dim: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent_soft: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent_glow: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus_ring: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_display: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tile_radius: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grid_gap: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section_spacing: Option<String>,
}

impl DiskThemeTokens {
    /// True when no token is overridden — used to drop an all-empty
    /// `tokens.toml` to `None` rather than ship a no-op object to the frontend.
    fn is_empty(&self) -> bool {
        *self == DiskThemeTokens::default()
    }
}

// ---------------------------------------------------------------------------
// per-system.toml — perSystemTokens (the D19 per-system palette sub-cascade).
// Per-system *layout* overrides live in theme.toml's `views.<v>.per_system`;
// per-system.toml is the palette layer (mirrors ThemePackage.perSystemTokens).
// ---------------------------------------------------------------------------

/// A system's accent palette overrides — `Partial<SystemPalette>` (accent /
/// soft / glow), mirroring `frontend/src/platform/themes/systemPalettes.ts`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SystemPalettePartial {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub soft: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glow: Option<String>,
}

/// Optional `per-system.toml`. The `per_system_tokens` table maps a system id
/// to its palette overrides, mirroring the TS `perSystemTokens`
/// (`Partial<Record<SystemId, Partial<SystemPalette>>>`). System ids are kept
/// loose `String` here; the frontend `validateTheme()` checks them against the
/// `SYSTEM_PALETTES` registry in S3 (same split as the manifest's loose
/// `glyph_set` / `views` system ids).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DiskPerSystem {
    #[serde(default)]
    pub per_system_tokens: BTreeMap<String, SystemPalettePartial>,
}

// ---------------------------------------------------------------------------
// The descriptor — one on-disk theme, ready to hand to the frontend. The
// wrapper fields are camelCase (perSystemTokens / basePath) to match the TS
// ThemePackage shape; the nested `manifest` keeps its own snake_case keys.
// ---------------------------------------------------------------------------

/// One discovered on-disk theme: its parsed manifest, optional token + palette
/// overrides, and the absolute path of its directory (for resolving the
/// theme's own assets — backgrounds/sounds — via `convertFileSrc` later).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskThemeDescriptor {
    pub manifest: DiskThemeManifest,
    /// Parsed `tokens.toml`, or `None` when absent / all-empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<DiskThemeTokens>,
    /// Parsed `per-system.toml` palette map, or `None` when absent / empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_system_tokens: Option<BTreeMap<String, SystemPalettePartial>>,
    /// Absolute path of the theme's directory (`<…>/themes/community/<id>`),
    /// the base for resolving its bundled assets.
    pub base_path: String,
}

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

/// Theme manifest filename inside a theme directory.
const THEME_MANIFEST: &str = "theme.toml";
/// Optional token-override filename.
const TOKENS_FILE: &str = "tokens.toml";
/// Optional per-system palette filename.
const PER_SYSTEM_FILE: &str = "per-system.toml";

/// Resolve `<…>/themes/<leaf>` with the SAME two-candidate walk as
/// [`crate::system_registry::resolve_config_systems_dir`] /
/// [`crate::emulator_profiles::resolve_config_emulators_dir`]:
///
///   1. `<exe_dir>/themes/<leaf>` — production install path (next to the exe;
///      where the pack channel installs, CP2/PD2).
///   2. `<CARGO_MANIFEST_DIR>/../../themes/<leaf>` — the source tree, for
///      `cargo run` / `cargo tauri dev` / running the workspace `target/` exe,
///      where no resources sit beside the binary (the operator's playtest
///      workflow runs `target/release/oa-shell.exe`, so without this a
///      repo-placed theme is never found). Harmless in production: the baked
///      `CARGO_MANIFEST_DIR` path doesn't exist on a user's machine.
///
/// `None` when neither candidate exists.
fn resolve_themes_subdir(leaf: &str) -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let p = exe_dir.join("themes").join(leaf);
            if p.is_dir() {
                return Some(p);
            }
        }
    }
    let dev = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("themes")
        .join(leaf);
    dev.is_dir().then_some(dev)
}

/// Resolve `themes/community/` — where installed `themes` packs land (the
/// oa-packs `<type>/community/` layout, CP2/PD2) and where hand-placed themes
/// go. See [`resolve_themes_subdir`] for the exe-dir-then-source-tree walk.
pub fn resolve_themes_community_dir() -> Option<PathBuf> {
    resolve_themes_subdir("community")
}

/// Resolve the loose-folder dev path `themes/dev/` — where an author can
/// hand-drop a theme folder outside the pack channel. Scanned at startup
/// alongside `community/`; hot-reload is intentionally NOT wired
/// (swap-by-restart is the shipped model, PD2).
pub fn resolve_themes_dev_dir() -> Option<PathBuf> {
    resolve_themes_subdir("dev")
}

/// Discover every theme directly under `parent` (one subdirectory per theme,
/// each holding a `theme.toml`). Unreadable / malformed themes are logged and
/// skipped — never fatal. A missing `parent` (read_dir fails) yields an empty
/// list. This is the unit-testable core; [`load_default`] wires it to the
/// resolved on-disk paths.
pub fn load_from_parent_dir(parent: &Path) -> Vec<DiskThemeDescriptor> {
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(parent) {
        Ok(e) => e,
        Err(e) => {
            // Common case: the dir simply doesn't exist (no themes). Caller
            // gates on `is_dir`, so reaching here is a genuine read error.
            log::warn!("theme_loader: read_dir {}: {e}", parent.display());
            return out;
        }
    };
    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        if let Some(desc) = load_one_theme(&dir) {
            out.push(desc);
        }
    }
    out
}

/// Parse a single theme directory into a descriptor, or `None` if it isn't a
/// usable theme (no/unparseable `theme.toml`). Optional files that fail to
/// parse are logged and dropped to `None` — the theme still loads without that
/// layer.
fn load_one_theme(dir: &Path) -> Option<DiskThemeDescriptor> {
    let manifest_path = dir.join(THEME_MANIFEST);
    let raw = match std::fs::read_to_string(&manifest_path) {
        Ok(s) => s,
        Err(e) => {
            // No theme.toml → not a theme folder (debug, not warn — a stray
            // dir under themes/ is not an error). Any other read error warns.
            if e.kind() == std::io::ErrorKind::NotFound {
                log::debug!("theme_loader: {} has no {THEME_MANIFEST}, skipping", dir.display());
            } else {
                log::warn!("theme_loader: read {}: {e}", manifest_path.display());
            }
            return None;
        }
    };
    let manifest: DiskThemeManifest = match toml::from_str(&raw) {
        Ok(m) => m,
        Err(e) => {
            log::warn!("theme_loader: parse {}: {e}", manifest_path.display());
            return None;
        }
    };

    // Folder name is authoritative for the on-disk path; the `id` field is
    // authoritative for the registry. Warn (don't reject) on a mismatch, same
    // as emulator_profiles.
    if let Some(folder) = dir.file_name().and_then(|s| s.to_str()) {
        if folder != manifest.id {
            log::warn!(
                "theme_loader: {} declares id '{}' (folder name differs — id field is authoritative)",
                dir.display(),
                manifest.id
            );
        }
    }
    if manifest.schema_version > 1 {
        log::warn!(
            "theme_loader: {} declares schema_version {} — newer than this build understands (1); loading anyway",
            manifest_path.display(),
            manifest.schema_version
        );
    }

    let tokens = load_optional(&dir.join(TOKENS_FILE), "tokens")
        .filter(|t: &DiskThemeTokens| !t.is_empty());
    let per_system_tokens = load_optional::<DiskPerSystem>(&dir.join(PER_SYSTEM_FILE), "per-system")
        .map(|ps| ps.per_system_tokens)
        .filter(|m| !m.is_empty());

    log::info!(
        "theme_loader: loaded theme '{}' ({}) from {}",
        manifest.id,
        manifest.name,
        dir.display()
    );
    Some(DiskThemeDescriptor {
        manifest,
        tokens,
        per_system_tokens,
        base_path: dir.to_string_lossy().into_owned(),
    })
}

/// Read + parse an optional sidecar TOML file. Absent → `None` silently; a
/// read/parse error is logged and dropped to `None` (the theme loads without
/// that layer — never fatal).
fn load_optional<T: serde::de::DeserializeOwned>(path: &Path, label: &str) -> Option<T> {
    let raw = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                log::warn!("theme_loader: read {label} {}: {e}", path.display());
            }
            return None;
        }
    };
    match toml::from_str::<T>(&raw) {
        Ok(v) => Some(v),
        Err(e) => {
            log::warn!("theme_loader: parse {label} {}: {e}", path.display());
            None
        }
    }
}

/// Discover all on-disk themes: the install-time `community/` themes plus any
/// hand-dropped `dev/` themes (the reserved loose-folder path). Returns an
/// empty list when neither directory exists. Dedup / validation / registry
/// merge is the frontend's job in S3 — this returns every descriptor it found.
pub fn load_default() -> Vec<DiskThemeDescriptor> {
    let mut out = Vec::new();
    match resolve_themes_community_dir() {
        Some(dir) => {
            log::info!("theme_loader: scanning {}", dir.display());
            out.extend(load_from_parent_dir(&dir));
        }
        None => {
            // Log WHERE we looked so a mis-placed folder is debuggable without
            // a rebuild (the candidate next to the exe — the production path).
            let candidate = std::env::current_exe()
                .ok()
                .and_then(|e| e.parent().map(|p| p.join("themes").join("community")));
            match candidate {
                Some(p) => log::info!(
                    "theme_loader: no disk themes — neither {} nor the source-tree fallback exists",
                    p.display()
                ),
                None => log::info!("theme_loader: no disk themes — could not resolve <exe_dir>"),
            }
        }
    }
    if let Some(dir) = resolve_themes_dev_dir() {
        log::info!("theme_loader: scanning dev path {}", dir.display());
        out.extend(load_from_parent_dir(&dir));
    }
    log::info!("theme_loader: discovered {} disk theme(s)", out.len());
    out
}

// ---------------------------------------------------------------------------
// Tauri command
// ---------------------------------------------------------------------------

/// List every on-disk theme discovered next to the exe. No network, no state —
/// re-scans on each call (themes change rarely; the swap-by-restart model means
/// a fresh scan per launch is enough). The frontend consumer arrives in S2/S3.
#[tauri::command]
pub fn oa_themes_list_disk() -> Vec<DiskThemeDescriptor> {
    load_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_tmp_dir(label: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "oa-themeloader-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("mkdir tmp");
        p
    }

    /// A complete, valid `theme.toml` exercising every field — required fields,
    /// views (default + per_system), per_system_ui, and all three
    /// settings_schema control kinds.
    const FULL_THEME_TOML: &str = r#"
id = "coverflow"
name = "CoverFlow"
version = "1.2.0"
schema_version = 1
oa_version = "^0.x"
default_route = "library"
routes = ["library", "settings"]
context_slots = ["library", "layout"]
required_engine_capabilities = []
surfaces = ["main"]
glyph_set = "playstation"

[per_system_ui]
tiles = true
sfx = false

[views.game-browse]
layout = "carousel"

[views.game-browse.per_system]
tg16 = "wheel"
lynx = "grid"

[[settings_schema]]
type = "toggle"
key = "reflections"
label = "Cover reflections"
default = true

[[settings_schema]]
type = "slider"
key = "spacing"
label = "Card spacing"
default = 12.0
min = 0.0
max = 40.0
step = 1.0
unit = "px"

[[settings_schema]]
type = "select"
key = "sort"
label = "Sort order"
default = "title"
options = [
    { value = "title", label = "Title" },
    { value = "year", label = "Year" },
]
"#;

    fn write_theme(parent: &Path, id: &str, theme_toml: &str) -> PathBuf {
        let dir = parent.join(id);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(THEME_MANIFEST), theme_toml).unwrap();
        dir
    }

    #[test]
    fn parses_full_theme_with_views_and_settings_schema() {
        let parent = fresh_tmp_dir("full");
        let dir = write_theme(&parent, "coverflow", FULL_THEME_TOML);

        let themes = load_from_parent_dir(&parent);
        assert_eq!(themes.len(), 1, "the one valid theme loads");
        let t = &themes[0];

        // --- required + simple fields ---
        assert_eq!(t.manifest.id, "coverflow");
        assert_eq!(t.manifest.name, "CoverFlow");
        assert_eq!(t.manifest.version, "1.2.0");
        assert_eq!(t.manifest.schema_version, 1);
        assert_eq!(t.manifest.oa_version, "^0.x");
        assert_eq!(t.manifest.default_route, "library");
        assert_eq!(t.manifest.routes, vec!["library", "settings"]);
        assert_eq!(t.manifest.surfaces, vec!["main"]);
        assert_eq!(t.manifest.glyph_set.as_deref(), Some("playstation"));
        // reserves_corner defaults to the one legal ARC 1 value.
        assert_eq!(t.manifest.reserves_corner, "top-right");

        // --- per_system_ui ---
        let psu = t.manifest.per_system_ui.as_ref().expect("per_system_ui parsed");
        assert_eq!(psu.tiles, Some(true));
        assert_eq!(psu.sfx, Some(false));

        // --- views (default layout + per_system override) ---
        let views = t.manifest.views.as_ref().expect("views parsed");
        let gb = views.get("game-browse").expect("game-browse view");
        assert_eq!(gb.layout.as_deref(), Some("carousel"));
        let per_sys = gb.per_system.as_ref().expect("per_system overrides");
        assert_eq!(per_sys.get("tg16").map(String::as_str), Some("wheel"));
        assert_eq!(per_sys.get("lynx").map(String::as_str), Some("grid"));

        // --- settings_schema: all three control kinds, in order ---
        let ss = t.manifest.settings_schema.as_ref().expect("settings_schema parsed");
        assert_eq!(ss.len(), 3);
        match &ss[0] {
            ThemeSettingControl::Toggle { key, label, default, hint } => {
                assert_eq!(key, "reflections");
                assert_eq!(label, "Cover reflections");
                assert!(*default);
                assert!(hint.is_none());
            }
            other => panic!("expected toggle, got {other:?}"),
        }
        match &ss[1] {
            ThemeSettingControl::Slider { key, default, min, max, step, unit, .. } => {
                assert_eq!(key, "spacing");
                assert_eq!(*default, 12.0);
                assert_eq!(*min, 0.0);
                assert_eq!(*max, 40.0);
                assert_eq!(*step, 1.0);
                assert_eq!(unit.as_deref(), Some("px"));
            }
            other => panic!("expected slider, got {other:?}"),
        }
        match &ss[2] {
            ThemeSettingControl::Select { key, default, options, .. } => {
                assert_eq!(key, "sort");
                assert_eq!(default, "title");
                assert_eq!(options.len(), 2);
                assert_eq!(options[0].value, "title");
                assert_eq!(options[1].label, "Year");
            }
            other => panic!("expected select, got {other:?}"),
        }

        // --- base path resolves to the theme's own directory ---
        assert_eq!(t.base_path, dir.to_string_lossy());

        // No sidecar files → no tokens / per-system overrides.
        assert!(t.tokens.is_none());
        assert!(t.per_system_tokens.is_none());

        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn parses_optional_tokens_and_per_system_files() {
        let parent = fresh_tmp_dir("sidecars");
        let dir = write_theme(
            &parent,
            "midnight",
            r#"
id = "midnight"
name = "Midnight"
version = "1.0.0"
schema_version = 1
oa_version = "^0.x"
default_route = "library"
surfaces = ["main"]
"#,
        );
        // tokens.toml uses camelCase keys (the TS ThemeTokens casing).
        std::fs::write(
            dir.join(TOKENS_FILE),
            r#"
bg = "oklch(0.15 0.02 270)"
bgDeep = "oklch(0.10 0.02 270)"
accentSoft = "oklch(0.93 0.05 270)"
tileRadius = "12px"
"#,
        )
        .unwrap();
        // per-system.toml: palette overrides keyed by system id.
        std::fs::write(
            dir.join(PER_SYSTEM_FILE),
            r#"
[per_system_tokens.tg16]
accent = "oklch(0.74 0.18 55)"
glow = "oklch(0.74 0.18 55 / 0.35)"

[per_system_tokens.lynx]
accent = "oklch(0.65 0.22 290)"
"#,
        )
        .unwrap();

        let themes = load_from_parent_dir(&parent);
        assert_eq!(themes.len(), 1);
        let t = &themes[0];

        let tokens = t.tokens.as_ref().expect("tokens.toml parsed");
        assert_eq!(tokens.bg.as_deref(), Some("oklch(0.15 0.02 270)"));
        assert_eq!(tokens.bg_deep.as_deref(), Some("oklch(0.10 0.02 270)"));
        assert_eq!(tokens.accent_soft.as_deref(), Some("oklch(0.93 0.05 270)"));
        assert_eq!(tokens.tile_radius.as_deref(), Some("12px"));
        // Unset keys inherit (None).
        assert!(tokens.ink.is_none());

        let ps = t.per_system_tokens.as_ref().expect("per-system.toml parsed");
        assert_eq!(ps.get("tg16").and_then(|p| p.accent.as_deref()), Some("oklch(0.74 0.18 55)"));
        assert_eq!(ps.get("tg16").and_then(|p| p.glow.as_deref()), Some("oklch(0.74 0.18 55 / 0.35)"));
        assert!(ps.get("tg16").and_then(|p| p.soft.as_deref()).is_none());
        assert_eq!(ps.get("lynx").and_then(|p| p.accent.as_deref()), Some("oklch(0.65 0.22 290)"));

        let _ = std::fs::remove_dir_all(&parent);
    }

    /// Camelcase token keys round-trip to camelCase JSON for the frontend.
    #[test]
    fn token_keys_serialize_camelcase_for_frontend() {
        let tokens = DiskThemeTokens {
            bg_deep: Some("x".into()),
            font_display: Some("y".into()),
            ..Default::default()
        };
        let json = serde_json::to_string(&tokens).unwrap();
        assert!(json.contains("\"bgDeep\""), "got {json}");
        assert!(json.contains("\"fontDisplay\""), "got {json}");
        assert!(!json.contains("bg_deep"), "snake_case leaked: {json}");
    }

    /// The descriptor's wrapper fields serialize camelCase (perSystemTokens /
    /// basePath) while the nested manifest keeps snake_case (schema_version).
    #[test]
    fn descriptor_serializes_with_expected_casing() {
        let parent = fresh_tmp_dir("casing");
        write_theme(
            &parent,
            "t",
            r#"
id = "t"
name = "T"
version = "1.0.0"
schema_version = 1
oa_version = "^0.x"
default_route = "library"
surfaces = ["main"]
"#,
        );
        let themes = load_from_parent_dir(&parent);
        let json = serde_json::to_string(&themes[0]).unwrap();
        assert!(json.contains("\"basePath\""), "got {json}");
        assert!(json.contains("\"schema_version\""), "manifest stays snake_case: {json}");
        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn skips_malformed_theme_but_keeps_siblings() {
        let parent = fresh_tmp_dir("malformed");
        // A good theme…
        write_theme(&parent, "good", FULL_THEME_TOML);
        // …a syntactically broken sibling…
        write_theme(&parent, "broken-syntax", "id = \"broken-syntax\"\nname = ");
        // …and one missing a required field (no `id`).
        write_theme(
            &parent,
            "missing-id",
            r#"
name = "No Id"
version = "1.0.0"
schema_version = 1
oa_version = "^0.x"
default_route = "library"
surfaces = ["main"]
"#,
        );
        // …and a stray dir with no theme.toml at all.
        std::fs::create_dir_all(parent.join("not-a-theme")).unwrap();

        let themes = load_from_parent_dir(&parent);
        assert_eq!(themes.len(), 1, "only the valid theme survives");
        assert_eq!(themes[0].manifest.id, "coverflow");

        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn malformed_sidecar_is_dropped_not_fatal() {
        let parent = fresh_tmp_dir("bad-sidecar");
        let dir = write_theme(
            &parent,
            "ok",
            r#"
id = "ok"
name = "Ok"
version = "1.0.0"
schema_version = 1
oa_version = "^0.x"
default_route = "library"
surfaces = ["main"]
"#,
        );
        // A broken tokens.toml must not disqualify the theme.
        std::fs::write(dir.join(TOKENS_FILE), "bg = ").unwrap();

        let themes = load_from_parent_dir(&parent);
        assert_eq!(themes.len(), 1, "theme still loads without its tokens layer");
        assert!(themes[0].tokens.is_none());

        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn empty_tokens_file_collapses_to_none() {
        let parent = fresh_tmp_dir("empty-tokens");
        let dir = write_theme(
            &parent,
            "ok",
            r#"
id = "ok"
name = "Ok"
version = "1.0.0"
schema_version = 1
oa_version = "^0.x"
default_route = "library"
surfaces = ["main"]
"#,
        );
        // A tokens.toml present but with no overrides → None (no no-op object).
        std::fs::write(dir.join(TOKENS_FILE), "\n").unwrap();
        std::fs::write(dir.join(PER_SYSTEM_FILE), "[per_system_tokens]\n").unwrap();

        let themes = load_from_parent_dir(&parent);
        assert_eq!(themes.len(), 1);
        assert!(themes[0].tokens.is_none(), "all-empty tokens collapses to None");
        assert!(themes[0].per_system_tokens.is_none(), "empty per-system collapses to None");

        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn missing_directory_yields_empty() {
        let parent = fresh_tmp_dir("missing");
        let nonexistent = parent.join("does-not-exist");
        assert!(load_from_parent_dir(&nonexistent).is_empty());
        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn base_path_points_at_theme_dir() {
        let parent = fresh_tmp_dir("basepath");
        let dir = write_theme(&parent, "coverflow", FULL_THEME_TOML);
        let themes = load_from_parent_dir(&parent);
        assert_eq!(themes.len(), 1);
        assert_eq!(themes[0].base_path, dir.to_string_lossy());
        // The path is absolute (asset resolution needs it absolute).
        assert!(Path::new(&themes[0].base_path).is_absolute());
        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn shipped_sample_theme_parses() {
        // The live `neon-list` sample at `<repo>/themes/community/` must
        // round-trip through the loader so it can't drift from the schema
        // (mirrors emulator_profiles' shipped-profile test). Resolving via the
        // real `resolve_themes_community_dir` ALSO proves the source-tree
        // fallback works (the test binary runs from target/, so the exe-dir
        // candidate is absent and the fallback resolves the repo dir).
        let dir = resolve_themes_community_dir()
            .expect("themes/community resolves in-tree via the source-tree fallback");
        let themes = load_from_parent_dir(&dir);
        let neon = themes
            .iter()
            .find(|t| t.manifest.id == "neon-list")
            .expect("neon-list sample parses");
        assert_eq!(neon.manifest.name, "Neon List");
        // Declares the list layout for game-browse.
        assert_eq!(
            neon.manifest
                .views
                .as_ref()
                .and_then(|v| v.get("game-browse"))
                .and_then(|c| c.layout.as_deref()),
            Some("list")
        );
        // Both optional sidecars (tokens.toml + per-system.toml) load.
        assert!(neon.tokens.as_ref().and_then(|t| t.accent.as_deref()).is_some());
        assert!(neon.per_system_tokens.as_ref().and_then(|m| m.get("nes")).is_some());
    }
}
