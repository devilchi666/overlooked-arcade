//! Phase 3 slice C — TOML preset registry.
//!
//! Replaces the slice-A hardcoded preset string list with a data-driven
//! registry. Built-in presets are compiled in via `include_str!` so the
//! app always has them; user-provided presets in
//! `<exe_dir>/shaders/presets/<name>.preset.toml` overlay by name (a user
//! file named `phosphor.preset.toml` replaces the built-in Phosphor entry).
//!
//! Schema:
//! ```toml
//! display_name = "Phosphor (soft bloom)"
//! description  = "Separable Gaussian blur composited with the source."
//! base         = "phosphor"   # picks the renderer's built-in pipeline
//!
//! [params]
//! bloom_amount = 0.6          # 0..1; only meaningful for `base = "phosphor"`
//!
//! [bezel]
//! image = "bezels/crt-shell.png"  # relative to <exe_dir>/shaders/
//! ```
//!
//! Resolution flow: `set_shader_preset(name)` looks up the def, calls
//! [`apply`] to decode any referenced bezel PNG (`image` crate), then
//! sends a [`ResolvedPreset`] over the existing emu-thread channel which
//! pushes `base` to the renderer, then `bloom_amount`, then the bezel.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Parsed preset file. `name` is filled in from the file stem by the loader
/// — it's not present in the TOML body.
#[derive(Debug, Clone, Deserialize)]
pub struct ShaderPresetDef {
    #[serde(skip)]
    pub name: String,
    pub display_name: String,
    #[serde(default)]
    pub description: String,
    pub base: String,
    #[serde(default)]
    pub params: PresetParams,
    #[serde(default)]
    pub bezel: Option<BezelSpec>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PresetParams {
    #[serde(default)]
    pub bloom_amount: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BezelSpec {
    /// Image path. Absolute paths are used verbatim; relative paths
    /// resolve relative to `<exe_dir>/shaders/`. PNG (RGBA recommended)
    /// — anything `image::open` accepts works, but bezels need an alpha
    /// channel to composite correctly.
    pub image: String,
}

/// Trimmed-down view of [`ShaderPresetDef`] returned by `list_shader_presets`.
/// Frontend only needs name + display_name + description + a hint about which
/// renderer pipeline backs this preset. Full params + bezel resolution
/// happen Rust-side via `set_shader_preset`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShaderPresetSummary {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub base: String,
}

/// Result of resolving a [`ShaderPresetDef`] — base preset + optional
/// param overrides + optional decoded bezel image. The emu thread applies
/// each Some() field in turn; None means "leave the renderer's current
/// value alone" (which is the right semantic when a preset has no opinion
/// on a parameter — selecting Plain shouldn't clobber the bloom_amount
/// the user set in Phosphor, for instance, but it SHOULD clear the bezel
/// since Plain explicitly has no bezel block).
pub struct ResolvedPreset {
    pub base: oa_render::ShaderPreset,
    pub bloom_amount: Option<f32>,
    /// `Some(rgba)` = load this bezel. `None` = clear any active bezel.
    /// (The TOML's absence-of-bezel-block always means "clear" — only an
    /// explicit `[bezel]` section keeps the current bezel active.)
    pub bezel: Option<ResolvedBezel>,
}

pub struct ResolvedBezel {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Compiled-in built-in presets. Ships with every build; user files in
/// `<exe_dir>/shaders/presets/` can override by name. The built-ins
/// match the renderer's `ShaderPreset` enum 1:1 — adding a new renderer
/// pipeline means adding a matching .preset.toml file here AND
/// shipping it as an asset, since `include_str!` is compile-time.
const BUILTIN_PRESETS: &[(&str, &str)] = &[
    ("plain",        include_str!("../../../shaders/presets/plain.preset.toml")),
    ("scanlines",    include_str!("../../../shaders/presets/scanlines.preset.toml")),
    ("crt-lite",     include_str!("../../../shaders/presets/crt-lite.preset.toml")),
    ("phosphor",     include_str!("../../../shaders/presets/phosphor.preset.toml")),
    // LcdHandheld — slice E. Subpixel triplet + matrix grid. Default
    // for the handheld panels (gb / gba / gg / ngp / ws); also
    // selectable globally via the Presets dropdown.
    ("lcd-handheld", include_str!("../../../shaders/presets/lcd-handheld.preset.toml")),
];

/// Built-ins only — used by tests + as the fallback path if exe_dir
/// resolution somehow fails.
pub fn builtins() -> Vec<ShaderPresetDef> {
    BUILTIN_PRESETS
        .iter()
        .map(|(name, raw)| {
            let mut def: ShaderPresetDef = toml::from_str(raw)
                .unwrap_or_else(|e| panic!("built-in preset `{name}` failed to parse: {e}"));
            def.name = (*name).to_string();
            def
        })
        .collect()
}

/// Load the full registry: built-ins first, then any user files at
/// `<exe_dir>/shaders/presets/<name>.preset.toml` overlaying by name.
/// Sorted alphabetically by name in the returned Vec — gives a stable
/// dropdown order.
pub fn load_all(exe_dir: &Path) -> Vec<ShaderPresetDef> {
    let mut by_name: BTreeMap<String, ShaderPresetDef> = BTreeMap::new();
    for def in builtins() {
        by_name.insert(def.name.clone(), def);
    }
    let user_dir = exe_dir.join("shaders").join("presets");
    if let Ok(entries) = std::fs::read_dir(&user_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = match path.file_name().and_then(|s| s.to_str()) {
                Some(s) => s,
                None => continue,
            };
            if !file_name.ends_with(".preset.toml") {
                continue;
            }
            let name = file_name.trim_end_matches(".preset.toml").to_string();
            if name.is_empty() {
                continue;
            }
            let raw = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("oa-shell: shader preset {}: read failed: {e}", path.display());
                    continue;
                }
            };
            match toml::from_str::<ShaderPresetDef>(&raw) {
                Ok(mut def) => {
                    def.name = name.clone();
                    log::info!("oa-shell: loaded user shader preset `{name}` from {}", path.display());
                    by_name.insert(name, def);
                }
                Err(e) => {
                    log::warn!("oa-shell: shader preset {}: parse failed: {e}", path.display());
                }
            }
        }
    }
    by_name.into_values().collect()
}

/// Summary list for the Tauri `list_shader_presets` command.
pub fn summarize(defs: &[ShaderPresetDef]) -> Vec<ShaderPresetSummary> {
    defs.iter()
        .map(|d| ShaderPresetSummary {
            name: d.name.clone(),
            display_name: d.display_name.clone(),
            description: d.description.clone(),
            base: d.base.clone(),
        })
        .collect()
}

/// Resolve a preset definition into renderer-ready values. PNG decode
/// happens here so the emu thread doesn't touch the filesystem. A bezel
/// that fails to decode is logged + dropped — the rest of the preset
/// still applies.
pub fn apply(def: &ShaderPresetDef, exe_dir: &Path) -> ResolvedPreset {
    let base = oa_render::ShaderPreset::parse(&def.base);
    let bloom_amount = def.params.bloom_amount;
    let bezel = def.bezel.as_ref().and_then(|spec| {
        let path = resolve_bezel_path(&spec.image, exe_dir);
        match load_rgba_image(&path) {
            Ok((rgba, w, h)) => Some(ResolvedBezel { rgba, width: w, height: h }),
            Err(e) => {
                log::warn!("oa-shell: bezel image {}: {e}; ignoring", path.display());
                None
            }
        }
    });
    ResolvedPreset { base, bloom_amount, bezel }
}

fn resolve_bezel_path(image: &str, exe_dir: &Path) -> PathBuf {
    let p = Path::new(image);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        exe_dir.join("shaders").join(image)
    }
}

/// Decode an image file to raw RGBA bytes + dimensions. Used by the
/// preset TOML loader for `[bezel]` images and by the per-system /
/// per-game bezel override path (`set_bezel_image_override` Tauri
/// command). PNG/JPEG/WebP are all accepted by the `image` crate.
pub fn load_rgba_image(path: &Path) -> Result<(Vec<u8>, u32, u32), String> {
    let img = image::open(path).map_err(|e| format!("open: {e}"))?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Ok((rgba.into_raw(), w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_parse_and_have_unique_names() {
        let defs = builtins();
        assert_eq!(defs.len(), 5, "shipping 5 built-in presets (incl. lcd-handheld)");
        let names: std::collections::HashSet<_> = defs.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names.len(), 5, "built-in names must be unique");
        // Every built-in base must be a valid renderer preset string (round-
        // trip through `ShaderPreset::parse` returns the same value as
        // re-parsing the .as_str() output).
        for d in &defs {
            let parsed = oa_render::ShaderPreset::parse(&d.base);
            assert_eq!(parsed.as_str(), d.base, "preset `{}` base `{}` doesn't round-trip", d.name, d.base);
        }
    }

    #[test]
    fn phosphor_carries_bloom_amount_param() {
        let defs = builtins();
        let phos = defs.iter().find(|d| d.name == "phosphor").expect("phosphor present");
        assert_eq!(phos.params.bloom_amount, Some(0.6));
        // Non-phosphor presets don't declare a bloom_amount — they inherit
        // whatever the renderer has set (which is the slice B-2 default).
        let plain = defs.iter().find(|d| d.name == "plain").expect("plain present");
        assert_eq!(plain.params.bloom_amount, None);
    }

    #[test]
    fn no_builtin_specifies_a_bezel() {
        // We don't ship a default bezel asset in slice C — the bezel field
        // is opt-in per-preset and unused until users provide their own.
        for d in builtins() {
            assert!(d.bezel.is_none(), "preset `{}` shouldn't ship a bezel", d.name);
        }
    }

    #[test]
    fn user_overlay_replaces_builtin_by_name() {
        // Drop a `phosphor.preset.toml` in a fake exe_dir/shaders/presets
        // dir with a different display_name + bloom_amount; load_all should
        // see the user version, not the built-in.
        let tmp = std::env::temp_dir().join(format!(
            "oa-shader-preset-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let presets_dir = tmp.join("shaders").join("presets");
        std::fs::create_dir_all(&presets_dir).expect("mkdir");
        let override_body = r#"
            display_name = "Phosphor (custom)"
            description  = "User override."
            base         = "phosphor"

            [params]
            bloom_amount = 0.3
        "#;
        std::fs::write(presets_dir.join("phosphor.preset.toml"), override_body).expect("write");

        let defs = load_all(&tmp);
        let phos = defs.iter().find(|d| d.name == "phosphor").expect("phosphor present");
        assert_eq!(phos.display_name, "Phosphor (custom)");
        assert_eq!(phos.params.bloom_amount, Some(0.3));
        // Other built-ins still appear.
        assert_eq!(defs.len(), 5, "user override doesn't add or drop entries");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn malformed_user_file_is_skipped_not_fatal() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-shader-preset-test-bad-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let presets_dir = tmp.join("shaders").join("presets");
        std::fs::create_dir_all(&presets_dir).expect("mkdir");
        // Missing required `base` field → toml::from_str errors.
        std::fs::write(
            presets_dir.join("broken.preset.toml"),
            "display_name = \"Broken\"\n",
        )
        .expect("write");

        let defs = load_all(&tmp);
        // Built-ins still load; broken file is dropped silently (with a
        // warning logged — not asserted here).
        assert_eq!(defs.len(), 5);
        assert!(defs.iter().all(|d| d.name != "broken"));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
