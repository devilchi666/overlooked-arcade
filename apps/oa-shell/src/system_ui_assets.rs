//! Per-System Custom UI asset resolver — non-audio assets.
//!
//! Backgrounds, boot animations, and future per-system visual assets
//! live under `<exe_dir>/assets/system-ui/<systemId>/` with a
//! `_baseline/` universal fallback alongside (same shape as Slice 2's
//! per-system SFX bundle). Audio cascades stay in `audio_player.rs`
//! because they share the rodio mixer state; this module owns the
//! disk-only resolvers.
//!
//! The frontend asks the Rust side for a resolved absolute path
//! (rather than constructing one itself) because Rust knows
//! `<exe_dir>` via `std::env::current_exe()` and can do the
//! extension-priority walk in one place. The frontend then funnels
//! the path through Tauri's asset protocol (`convertFileSrc`) to
//! load it in the WebView.

use std::path::{Path, PathBuf};

/// Background asset kind. Maps to the file basename inside
/// `<systemId>/backgrounds/`:
///
/// - `"default"` → static image, walks
///   `[png, jpg, jpeg, webp]` extensions.
/// - `"animated"` → looping video, walks `[webm, mp4]`.
///
/// Shader backgrounds (`<systemId>/backgrounds/shader.wgsl`) are not
/// resolved through this command — Slice 8 (Vectrex pilot) ships the
/// shader-driven render path and consumes the file directly via the
/// renderer integration. Slice 3 falls back to `static` rendering when
/// `SystemUIConfig.background === "shader"` so non-Vectrex systems
/// that happen to set the shader value stay legible.
const STATIC_EXTS: &[&str] = &["png", "jpg", "jpeg", "webp"];
const ANIMATED_EXTS: &[&str] = &["webm", "mp4"];

/// Resolve a per-system background asset path. Returns `Ok(None)` when
/// no candidate file exists on disk — frontend falls back to the
/// CSS accent-color gradient.
///
/// Resolution order matches Slice 2's per-system SFX cascade:
///   1. `<exe_dir>/assets/system-ui/<systemId>/backgrounds/<basename>.<ext>`
///   2. `<exe_dir>/assets/system-ui/_baseline/backgrounds/<basename>.<ext>`
///   3. None
///
/// `kind` must be one of: `"default"` (static image) or `"animated"`
/// (looping video). Unknown values return an error so the frontend
/// catches typos.
#[tauri::command]
#[allow(non_snake_case)]
pub fn resolve_background_asset(
    systemId: String,
    kind: String,
) -> Result<Option<String>, String> {
    let exts: &[&str] = match kind.as_str() {
        "default" => STATIC_EXTS,
        "animated" => ANIMATED_EXTS,
        _ => return Err(format!("unknown background kind: {kind}")),
    };
    let basename = kind.as_str(); // file basename matches kind name
    let assets_dir = resolve_assets_dir();
    Ok(find_bundled_background_in_dir(&assets_dir, &systemId, basename, exts)
        .map(|pb| pb.to_string_lossy().to_string()))
}

/// Locate the `<exe_dir>/assets` directory the same way
/// `resolve_cores_dir` locates `<exe_dir>/cores`: next to the .exe.
/// Duplicated from `audio_player.rs` because the modules don't share
/// a parent — pulling both into a `paths` module is a follow-up
/// cleanup, not Slice 3 scope.
fn resolve_assets_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .join("assets")
}

/// Pure resolver — given an assets dir, system slug, basename
/// (`"default"` / `"animated"`), and the extension priority list,
/// return the first matching file under
/// `system-ui/<slug>/backgrounds/`, falling back to
/// `system-ui/_baseline/backgrounds/`. Returns None if no candidate
/// exists. Parameterized on `assets_dir` so unit tests can drop a
/// tempdir's contents in and verify the cascade.
fn find_bundled_background_in_dir(
    assets_dir: &Path,
    system_id: &str,
    basename: &str,
    exts: &[&str],
) -> Option<PathBuf> {
    if system_id.contains('/') || system_id.contains('\\') || system_id == ".." {
        return None;
    }
    for slug in [system_id, "_baseline"] {
        let base = assets_dir.join("system-ui").join(slug).join("backgrounds");
        for ext in exts {
            let p = base.join(format!("{basename}.{ext}"));
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempdir_for(tag: &str) -> PathBuf {
        let tmp = std::env::temp_dir().join(format!("oa-shell-bg-asset-{tag}"));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        tmp
    }

    fn drop_bg(assets_dir: &Path, system_slug: &str, basename: &str, ext: &str) -> PathBuf {
        let dir = assets_dir.join("system-ui").join(system_slug).join("backgrounds");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(format!("{basename}.{ext}"));
        std::fs::write(&p, b"fake-asset").unwrap();
        p
    }

    #[test]
    fn bundled_resolver_finds_per_system_first() {
        let tmp = tempdir_for("per-system-wins");
        let per_system = drop_bg(&tmp, "nes", "default", "png");
        let _baseline = drop_bg(&tmp, "_baseline", "default", "png");

        let resolved = find_bundled_background_in_dir(&tmp, "nes", "default", STATIC_EXTS);
        assert_eq!(resolved.as_deref(), Some(per_system.as_path()));
    }

    #[test]
    fn bundled_resolver_falls_back_to_baseline() {
        let tmp = tempdir_for("baseline-fallback");
        let baseline = drop_bg(&tmp, "_baseline", "default", "png");

        let resolved = find_bundled_background_in_dir(&tmp, "nes", "default", STATIC_EXTS);
        assert_eq!(resolved.as_deref(), Some(baseline.as_path()));
    }

    #[test]
    fn bundled_resolver_returns_none_when_nothing_on_disk() {
        let tmp = tempdir_for("empty");
        let resolved = find_bundled_background_in_dir(&tmp, "nes", "default", STATIC_EXTS);
        assert!(resolved.is_none());
    }

    #[test]
    fn bundled_resolver_prefers_png_over_jpg() {
        // Extension priority matters when a system bank ships multiple
        // formats. PNG sits first in STATIC_EXTS so it wins; locks the
        // STATIC_EXTS order so a future reshuffle has to update tests.
        let tmp = tempdir_for("ext-priority-static");
        let png = drop_bg(&tmp, "nes", "default", "png");
        let _jpg = drop_bg(&tmp, "nes", "default", "jpg");

        let resolved = find_bundled_background_in_dir(&tmp, "nes", "default", STATIC_EXTS);
        assert_eq!(resolved.as_deref(), Some(png.as_path()));
    }

    #[test]
    fn bundled_resolver_walks_static_extensions() {
        // Operator might drop webp instead of png; the walk should
        // find it past the first two extension misses.
        let tmp = tempdir_for("ext-walk-static");
        let webp = drop_bg(&tmp, "_baseline", "default", "webp");
        let resolved = find_bundled_background_in_dir(&tmp, "nes", "default", STATIC_EXTS);
        assert_eq!(resolved.as_deref(), Some(webp.as_path()));
    }

    #[test]
    fn bundled_resolver_finds_animated_webm() {
        let tmp = tempdir_for("animated-webm");
        let webm = drop_bg(&tmp, "nes", "animated", "webm");
        let resolved = find_bundled_background_in_dir(&tmp, "nes", "animated", ANIMATED_EXTS);
        assert_eq!(resolved.as_deref(), Some(webm.as_path()));
    }

    #[test]
    fn bundled_resolver_rejects_path_traversal_in_system_id() {
        let tmp = tempdir_for("traversal");
        drop_bg(&tmp, "_baseline", "default", "png");

        for evil in ["../nes", "..\\nes", "..", "nes/../etc", "nes\\..\\etc"] {
            let resolved = find_bundled_background_in_dir(&tmp, evil, "default", STATIC_EXTS);
            assert!(resolved.is_none(), "expected None for slug {evil:?}, got {resolved:?}");
        }
    }

    #[test]
    fn static_and_animated_resolvers_dont_cross_contaminate() {
        // Dropping a static asset shouldn't satisfy a request for an
        // animated asset — each cascade walks its own basename +
        // extensions independently.
        let tmp = tempdir_for("no-cross");
        drop_bg(&tmp, "nes", "default", "png");

        let static_resolved = find_bundled_background_in_dir(&tmp, "nes", "default", STATIC_EXTS);
        assert!(static_resolved.is_some());

        let animated_resolved =
            find_bundled_background_in_dir(&tmp, "nes", "animated", ANIMATED_EXTS);
        assert!(animated_resolved.is_none());
    }
}
