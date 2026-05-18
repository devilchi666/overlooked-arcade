// Filesystem watcher for tracked library folders.
//
// Lifecycle:
//   - Frontend calls `set_watched_folders(folders)` whenever the user adds
//     or removes a tracked folder. We tear down the previous watcher and
//     re-register with the new set.
//   - Each watched folder gets a recursive `notify::RecommendedWatcher`.
//   - File-create events that match a ROM/archive extension fire a single-
//     file scan and emit `oa://library-watch-found` for the frontend to
//     ingest. The frontend hashes-by-file-path same as the existing flow,
//     so duplicates from a re-scan are dropped by INSERT OR IGNORE.
//   - File-remove events emit `oa://library-watch-removed` (frontend can
//     decide whether to drop from library or just flag as missing).
//
// We deliberately don't try to be clever about archives — a newly-dropped
// .zip is treated like any other file, peeked, and its inner ROMs emitted.
// CD games (folders of .cue + .bin) might trigger multiple events; the
// frontend de-dupes by ROM id.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use notify::{Event, EventKind, RecursiveMode, Watcher};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::archive;

pub struct WatcherState {
    inner: Mutex<Option<WatcherInner>>,
}

struct WatcherInner {
    /// Held to keep the OS-level file watcher alive. Dropping this struct
    /// is what stops the underlying notify::RecommendedWatcher.
    #[allow(dead_code)]
    watcher: Box<dyn Watcher + Send>,
    #[allow(dead_code)] // diagnostics / future `folders()` accessor
    folders: Vec<PathBuf>,
}

impl Default for WatcherState {
    fn default() -> Self {
        Self { inner: Mutex::new(None) }
    }
}

impl WatcherState {
    /// Tear down the previous watcher (if any) and start a new one watching
    /// the given folders. Called on app start with the persisted folder
    /// list and again whenever the user adds/removes folders.
    pub fn reconfigure(
        &self,
        handle: AppHandle,
        folders: Vec<PathBuf>,
        wanted: HashSet<String>,
    ) -> Result<(), String> {
        let mut guard = self.inner.lock().map_err(|_| "watcher lock poisoned".to_string())?;
        *guard = None; // drop the old watcher, releasing OS handles

        if folders.is_empty() {
            log::info!("watcher: no folders to watch");
            return Ok(());
        }

        let folders_copy = folders.clone();
        let mut watcher: notify::RecommendedWatcher = match notify::Watcher::new(
            move |res: Result<Event, notify::Error>| {
                let Ok(event) = res else { return };
                handle_event(&handle, &event, &wanted);
            },
            notify::Config::default().with_poll_interval(Duration::from_secs(2)),
        ) {
            Ok(w) => w,
            Err(e) => return Err(format!("notify::Watcher::new: {e}")),
        };

        for folder in &folders {
            if let Err(e) = watcher.watch(folder, RecursiveMode::Recursive) {
                log::warn!("watcher: watch {} failed: {e}", folder.display());
            } else {
                log::info!("watcher: watching {}", folder.display());
            }
        }

        *guard = Some(WatcherInner {
            watcher: Box::new(watcher),
            folders: folders_copy,
        });
        Ok(())
    }

    #[allow(dead_code)] // exposed for future "Folders" settings panel that reads back the live set
    pub fn folders(&self) -> Vec<PathBuf> {
        self.inner
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|i| i.folders.clone()))
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WatchFoundPayload {
    path: String,
    file_name: String,
    extension: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    archive_inner_path: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WatchRemovedPayload {
    path: String,
}

fn handle_event(handle: &AppHandle, event: &Event, wanted: &HashSet<String>) {
    match event.kind {
        EventKind::Create(_) | EventKind::Modify(notify::event::ModifyKind::Name(_)) => {
            for path in &event.paths {
                if path.is_dir() {
                    // A directory was created; the recursive watcher will
                    // pick up file events inside it. No work for us.
                    continue;
                }
                emit_path_found(handle, path, wanted);
            }
        }
        EventKind::Remove(_) => {
            for path in &event.paths {
                let ext = path_extension(path);
                if archive::ArchiveKind::from_extension(&ext).is_some() || wanted.contains(&ext) {
                    let payload = WatchRemovedPayload {
                        path: path.to_string_lossy().into_owned(),
                    };
                    if let Err(e) = handle.emit("oa://library-watch-removed", payload) {
                        log::warn!("watcher: emit removed failed: {e:?}");
                    }
                }
            }
        }
        _ => {}
    }
}

fn path_extension(path: &Path) -> String {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default()
}

fn emit_path_found(handle: &AppHandle, path: &Path, wanted: &HashSet<String>) {
    let ext = path_extension(path);
    if ext.is_empty() {
        return;
    }
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    if archive::ArchiveKind::from_extension(&ext).is_some() {
        match archive::list_rom_contents(path, wanted) {
            Ok(inners) => {
                for inner in inners {
                    let encoded = archive::encode_file_path(path, &inner.inner_path);
                    let inner_name = Path::new(&inner.inner_path)
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| inner.inner_path.clone());
                    let payload = WatchFoundPayload {
                        path: encoded,
                        file_name: inner_name,
                        extension: inner.extension.clone(),
                        archive_inner_path: Some(inner.inner_path),
                    };
                    if let Err(e) = handle.emit("oa://library-watch-found", payload) {
                        log::warn!("watcher: emit found failed: {e:?}");
                    }
                }
            }
            Err(e) => log::warn!("watcher: peek {} failed: {e}", path.display()),
        }
        return;
    }

    if !wanted.contains(&ext) {
        return;
    }
    let payload = WatchFoundPayload {
        path: path.to_string_lossy().into_owned(),
        file_name,
        extension: ext,
        archive_inner_path: None,
    };
    if let Err(e) = handle.emit("oa://library-watch-found", payload) {
        log::warn!("watcher: emit found failed: {e:?}");
    }
}
