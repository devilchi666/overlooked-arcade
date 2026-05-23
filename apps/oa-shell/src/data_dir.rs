//! Data-dir resolver — single source of truth for where OA user state lives.
//!
//! Two modes:
//! - **Portable**: a `portable.txt` marker file next to `oa-shell.exe` opts in;
//!   all user state lives under `<exe_dir>/settings/`. The whole install can
//!   be copied between machines or onto a USB stick as a unit. Matches the
//!   existing cores/BIOS-next-to-exe pattern.
//! - **AppData** (default): falls back to Tauri's `app_data_dir()` — on
//!   Windows `%APPDATA%\dev.overlookedarcade.shell\`.
//!
//! Same binary supports both modes. The marker file is the entire opt-in.
//! AppData → portable migration is handled separately (see Phase 3).

use std::path::{Path, PathBuf};

use tauri::Manager;

/// Marker filename next to oa-shell.exe that opts the install into portable mode.
pub const MARKER_FILENAME: &str = "portable.txt";

/// Subdirectory of <exe_dir> where user state lives in portable mode.
pub const PORTABLE_SUBDIR: &str = "settings";

/// Sentinel filename written inside the portable dir after a successful
/// migration from AppData. Presence means "migration already ran; do not
/// re-overwrite portable state from AppData even if portable looks empty."
const MIGRATION_SENTINEL: &str = ".migrated-from-appdata";

/// Tauri 2 bundle identifier — must match `tauri.conf.json` `identifier`.
/// Used to derive the AppData fallback path when no Tauri app handle is
/// available (headless CLI mode).
pub(crate) const TAURI_IDENTIFIER: &str = "dev.overlookedarcade.shell";

/// Resolved data-dir + which mode produced it. Callers that need to widen
/// the asset-protocol scope or log differently key off `portable`.
#[derive(Debug, Clone)]
pub struct DataDir {
    pub path: PathBuf,
    pub portable: bool,
}

/// Resolve where OA reads/writes user state. Tries portable mode first;
/// falls back to AppData if no marker is present.
///
/// `app: Some(&tauri::App)` — GUI mode; uses Tauri's `app_data_dir()` for
/// the AppData fallback (handles per-platform conventions natively).
///
/// `app: None` — headless / CLI mode; uses the manual env-var fallback
/// (same per-OS logic Tauri uses internally).
pub fn resolve_data_dir(app: Option<&tauri::App>) -> Result<DataDir, String> {
    if let Some(portable_dir) = detect_portable_dir() {
        // Ensure the portable settings/ dir exists. If we can't create it,
        // the install folder isn't writable (e.g. Program Files, read-only
        // network share) — log loudly and fall back to AppData rather than
        // silently corrupt state.
        match std::fs::create_dir_all(&portable_dir) {
            Ok(_) => {
                migrate_from_appdata_if_needed(&portable_dir);
                return Ok(DataDir { path: portable_dir, portable: true });
            }
            Err(e) => log::error!(
                "oa-shell: portable.txt detected but {} not writable ({e}); \
                 falling back to AppData. Move the install to a writable \
                 folder (Documents, Desktop, USB drive, etc.) to enable \
                 portable mode.",
                portable_dir.display()
            ),
        }
    }

    let appdata = match app {
        Some(app) => app
            .path()
            .app_data_dir()
            .map_err(|e| format!("Tauri app_data_dir() failed: {e:?}"))?,
        None => resolve_appdata_manually()?,
    };
    Ok(DataDir { path: appdata, portable: false })
}

/// Returns `Some(<exe_dir>/settings)` if the portable marker is present
/// next to the running .exe.
fn detect_portable_dir() -> Option<PathBuf> {
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    portable_dir_for_exe_dir(&exe_dir)
}

/// Pure function — given an exe_dir, return the portable settings dir if
/// the marker is present. Split out from `detect_portable_dir` so unit
/// tests can pass a tempdir.
fn portable_dir_for_exe_dir(exe_dir: &Path) -> Option<PathBuf> {
    if exe_dir.join(MARKER_FILENAME).exists() {
        Some(exe_dir.join(PORTABLE_SUBDIR))
    } else {
        None
    }
}

/// One-shot migration from the would-be AppData location → portable dir.
///
/// Runs on every portable-mode launch but exits early if any of:
/// - migration sentinel already exists (we ran before)
/// - AppData dir doesn't exist or is empty (nothing to migrate)
/// - portable dir already has user content (operator was already using
///   portable mode independently; don't overwrite)
///
/// On success, writes `.migrated-from-appdata` so subsequent launches skip
/// even if the operator manually empties the portable dir later. The
/// AppData dir is left in place — operator can delete it once they're
/// confident the migration took.
///
/// All errors logged, never propagated — migration failure should not
/// block app launch.
fn migrate_from_appdata_if_needed(portable_dir: &Path) {
    let sentinel = portable_dir.join(MIGRATION_SENTINEL);
    if sentinel.exists() {
        return;
    }
    if portable_has_user_content(portable_dir) {
        // First-portable-launch but operator already populated it without
        // running migration. Write the sentinel so we don't trip on this
        // every launch.
        if let Err(e) = std::fs::write(&sentinel, "skipped: portable dir already populated\n") {
            log::warn!("oa-shell: failed to write migration sentinel: {e}");
        }
        return;
    }
    let Ok(appdata) = resolve_appdata_manually() else {
        return;
    };
    if !appdata.is_dir() {
        return;
    }
    let Ok(mut entries) = std::fs::read_dir(&appdata) else {
        return;
    };
    if entries.next().is_none() {
        return; // empty AppData
    }

    log::info!(
        "oa-shell: migrating AppData → portable: {} → {}",
        appdata.display(),
        portable_dir.display()
    );
    match copy_dir_recursive(&appdata, portable_dir) {
        Ok((files, bytes)) => {
            log::info!(
                "oa-shell: migration complete: {files} files, {bytes} bytes copied. \
                 Original AppData left intact at {}; delete manually once verified.",
                appdata.display()
            );
            if let Err(e) = std::fs::write(&sentinel, format!("migrated {files} files / {bytes} bytes\n")) {
                log::warn!("oa-shell: failed to write migration sentinel: {e}");
            }
        }
        Err(e) => {
            log::error!(
                "oa-shell: migration failed mid-copy: {e}. Portable dir left as-is; \
                 next launch will retry. Investigate before continuing."
            );
        }
    }
}

/// True if `portable_dir` contains anything other than the marker file's
/// sibling (we don't expect any files in `portable_dir` itself on first
/// portable launch — the marker lives in the parent `<exe_dir>`).
fn portable_has_user_content(portable_dir: &Path) -> bool {
    match std::fs::read_dir(portable_dir) {
        Ok(mut entries) => entries.any(|e| {
            e.ok()
                .map(|e| e.file_name() != MIGRATION_SENTINEL)
                .unwrap_or(false)
        }),
        Err(_) => false,
    }
}

/// Recursive directory copy. Returns (file count, total bytes copied) on
/// success. Pure std::fs — no walkdir dep needed for a one-shot migration.
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<(u64, u64)> {
    std::fs::create_dir_all(dst)?;
    let mut files = 0u64;
    let mut bytes = 0u64;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let ft = entry.file_type()?;
        if ft.is_dir() {
            let (f, b) = copy_dir_recursive(&src_path, &dst_path)?;
            files += f;
            bytes += b;
        } else if ft.is_file() {
            let copied = std::fs::copy(&src_path, &dst_path)?;
            files += 1;
            bytes += copied;
        }
        // Symlinks and other special files are skipped — none of our
        // user-state subsystems write them.
    }
    Ok((files, bytes))
}

/// Per-OS manual AppData resolution. Mirrors what Tauri does internally,
/// used in headless mode where no Tauri app handle is available.
pub(crate) fn resolve_appdata_manually() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var("APPDATA")
            .map_err(|e| format!("APPDATA env not set: {e}"))?;
        Ok(PathBuf::from(appdata).join(TAURI_IDENTIFIER))
    }
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME")
            .map_err(|e| format!("HOME env not set: {e}"))?;
        Ok(PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join(TAURI_IDENTIFIER))
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            return Ok(PathBuf::from(xdg).join(TAURI_IDENTIFIER));
        }
        let home = std::env::var("HOME")
            .map_err(|e| format!("HOME env not set: {e}"))?;
        Ok(PathBuf::from(home)
            .join(".local")
            .join("share")
            .join(TAURI_IDENTIFIER))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_appdata_path_ends_with_identifier() {
        // Skip on platforms where the required env var isn't set (CI minimal
        // environments). The point of the test is that when resolution
        // succeeds, the suffix is correct.
        if let Ok(path) = resolve_appdata_manually() {
            assert!(
                path.ends_with(TAURI_IDENTIFIER),
                "manual appdata path should end with {TAURI_IDENTIFIER}, got {}",
                path.display()
            );
        }
    }

    #[test]
    fn portable_dir_returns_none_without_marker() {
        let tmp = tempdir_for("no-marker");
        assert!(portable_dir_for_exe_dir(&tmp).is_none());
        cleanup(&tmp);
    }

    #[test]
    fn portable_dir_returns_settings_subdir_with_marker() {
        let tmp = tempdir_for("with-marker");
        std::fs::write(tmp.join(MARKER_FILENAME), "").unwrap();
        let resolved = portable_dir_for_exe_dir(&tmp);
        assert_eq!(resolved, Some(tmp.join(PORTABLE_SUBDIR)));
        cleanup(&tmp);
    }

    fn tempdir_for(tag: &str) -> PathBuf {
        let tmp = std::env::temp_dir().join(format!("oa-shell-data-dir-{tag}"));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        tmp
    }

    fn cleanup(p: &Path) {
        let _ = std::fs::remove_dir_all(p);
    }

    #[test]
    fn copy_dir_recursive_copies_nested_files() {
        let src = tempdir_for("copy-src");
        let dst = tempdir_for("copy-dst");
        let _ = std::fs::remove_dir_all(&dst); // start with no dst
        std::fs::create_dir_all(src.join("sub/deep")).unwrap();
        std::fs::write(src.join("top.txt"), b"hello").unwrap();
        std::fs::write(src.join("sub/mid.txt"), b"world!!").unwrap();
        std::fs::write(src.join("sub/deep/leaf.bin"), b"\x00\x01\x02").unwrap();

        let (files, bytes) = copy_dir_recursive(&src, &dst).unwrap();
        assert_eq!(files, 3);
        assert_eq!(bytes, 5 + 7 + 3);
        assert_eq!(std::fs::read(dst.join("top.txt")).unwrap(), b"hello");
        assert_eq!(std::fs::read(dst.join("sub/mid.txt")).unwrap(), b"world!!");
        assert_eq!(std::fs::read(dst.join("sub/deep/leaf.bin")).unwrap(), b"\x00\x01\x02");

        cleanup(&src);
        cleanup(&dst);
    }

    #[test]
    fn portable_has_user_content_ignores_sentinel() {
        let dir = tempdir_for("has-content-sentinel-only");
        std::fs::write(dir.join(MIGRATION_SENTINEL), "").unwrap();
        assert!(!portable_has_user_content(&dir));
        std::fs::write(dir.join("audio.json"), "{}").unwrap();
        assert!(portable_has_user_content(&dir));
        cleanup(&dir);
    }

    #[test]
    fn portable_has_user_content_false_on_empty() {
        let dir = tempdir_for("has-content-empty");
        assert!(!portable_has_user_content(&dir));
        cleanup(&dir);
    }
}
