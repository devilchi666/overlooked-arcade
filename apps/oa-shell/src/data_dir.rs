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
            Ok(_) => return Ok(DataDir { path: portable_dir, portable: true }),
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
}
