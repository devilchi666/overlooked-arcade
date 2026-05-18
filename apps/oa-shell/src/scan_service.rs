// Background folder-scan service.
//
// The original `scan_rom_folder` is synchronous: it walks the directory tree
// inline, then returns the full list. That's fine for a few hundred ROMs but
// freezes the WebView for several seconds on a 10K-game LaunchBox tree. This
// module owns the async version — a tokio task walks the tree, emits per-
// folder + per-file progress events, and returns the final list when done.
//
// Two events:
//   oa://library-scan-progress  { jobId, folder, filesSeen, matches, archived, currentFile }
//   oa://library-scan-complete  { jobId, folder, matches, archived, cancelled, errorMessage }
//
// Cancellation is via `cancel_background_scan(jobId)` — the running task
// checks an AtomicBool each iteration and bails out cleanly.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::archive;

static NEXT_JOB_ID: AtomicU64 = AtomicU64::new(1);

/// Per-process registry of in-flight scan jobs. Each entry carries the
/// cancel flag the running task polls.
pub struct ScanServiceState {
    pub jobs: Arc<Mutex<HashMap<u64, Arc<AtomicBool>>>>,
}

impl Default for ScanServiceState {
    fn default() -> Self {
        Self { jobs: Arc::new(Mutex::new(HashMap::new())) }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannedRom {
    /// Encoded `<archive>#<inner>` for archived entries; raw path otherwise.
    pub path: String,
    pub file_name: String,
    pub extension: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive_inner_path: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub job_id: u64,
    pub folder: String,
    pub files_seen: u64,
    pub matches: u64,
    pub archived: u64,
    pub current_file: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanCompletePayload {
    pub job_id: u64,
    pub folder: String,
    pub matches: u64,
    pub archived: u64,
    pub cancelled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// Inline rows so the frontend can ingest without an extra round-trip.
    pub rows: Vec<ScannedRom>,
}

const MAX_DEPTH: u32 = 6;
const PROGRESS_THROTTLE_MS: u64 = 80;

/// Walk the folder tree in a tokio blocking task, emit progress events, and
/// return the final list. Caller-supplied `cancel` flag short-circuits the
/// walk; we check it at each directory entry + each archive peek.
pub fn run_scan_blocking(
    job_id: u64,
    handle: AppHandle,
    folder: PathBuf,
    wanted: HashSet<String>,
    cancel: Arc<AtomicBool>,
) -> Result<Vec<ScannedRom>, String> {
    let folder_str = folder.to_string_lossy().into_owned();
    let mut out = Vec::new();
    let mut files_seen: u64 = 0;
    let mut last_emit = std::time::Instant::now();

    fn emit_progress(
        handle: &AppHandle,
        job_id: u64,
        folder: &str,
        files_seen: u64,
        out: &[ScannedRom],
        current_file: &str,
    ) {
        let archived = out.iter().filter(|r| r.archive_inner_path.is_some()).count() as u64;
        let progress = ScanProgress {
            job_id,
            folder: folder.to_string(),
            files_seen,
            matches: out.len() as u64,
            archived,
            current_file: current_file.to_string(),
        };
        if let Err(e) = handle.emit("oa://library-scan-progress", progress) {
            log::warn!("scan_service: emit progress failed: {e:?}");
        }
    }

    walk(
        &folder,
        0,
        MAX_DEPTH,
        &wanted,
        &cancel,
        &mut out,
        &mut files_seen,
        &mut last_emit,
        &handle,
        job_id,
        &folder_str,
    );

    // Final progress emit so the UI bar lands at 100% / last filename.
    emit_progress(&handle, job_id, &folder_str, files_seen, &out, "");

    out.sort_by(|a, b| a.file_name.to_ascii_lowercase().cmp(&b.file_name.to_ascii_lowercase()));
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn walk(
    dir: &Path,
    depth: u32,
    max_depth: u32,
    wanted: &HashSet<String>,
    cancel: &AtomicBool,
    out: &mut Vec<ScannedRom>,
    files_seen: &mut u64,
    last_emit: &mut std::time::Instant,
    handle: &AppHandle,
    job_id: u64,
    folder_str: &str,
) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        let Ok(file_type) = entry.file_type() else { continue };
        let name = entry.file_name();
        let name_owned: String = name.to_string_lossy().into_owned();
        if name_owned.starts_with('.') {
            continue;
        }
        let entry_path = entry.path();
        if file_type.is_dir() {
            if depth + 1 <= max_depth {
                walk(
                    &entry_path,
                    depth + 1,
                    max_depth,
                    wanted,
                    cancel,
                    out,
                    files_seen,
                    last_emit,
                    handle,
                    job_id,
                    folder_str,
                );
            }
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        *files_seen += 1;
        let ext = entry_path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase());
        let Some(ext) = ext else { continue };

        if archive::ArchiveKind::from_extension(&ext).is_some() {
            match archive::list_rom_contents(&entry_path, wanted) {
                Ok(inner_entries) => {
                    for inner in inner_entries {
                        if cancel.load(Ordering::Relaxed) {
                            return;
                        }
                        let encoded_path =
                            archive::encode_file_path(&entry_path, &inner.inner_path);
                        let inner_name = Path::new(&inner.inner_path)
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| inner.inner_path.clone());
                        out.push(ScannedRom {
                            path: encoded_path,
                            file_name: inner_name,
                            extension: inner.extension.clone(),
                            archive_inner_path: Some(inner.inner_path),
                        });
                    }
                }
                Err(e) => log::warn!("scan_service: peek {} failed: {e}", entry_path.display()),
            }
        } else if archive::ArchiveKind::is_unsupported_archive(&ext) {
            log::warn!(
                "scan_service: skipping {} ({} not supported)",
                entry_path.display(), ext,
            );
        } else if wanted.contains(&ext) {
            out.push(ScannedRom {
                path: entry_path.to_string_lossy().into_owned(),
                file_name: name_owned.clone(),
                extension: ext,
                archive_inner_path: None,
            });
        }

        // Throttled progress emit — keep below 12 Hz so a 50K-file scan
        // doesn't flood the IPC channel.
        let now = std::time::Instant::now();
        if now.duration_since(*last_emit).as_millis() as u64 >= PROGRESS_THROTTLE_MS {
            *last_emit = now;
            let archived =
                out.iter().filter(|r| r.archive_inner_path.is_some()).count() as u64;
            let progress = ScanProgress {
                job_id,
                folder: folder_str.to_string(),
                files_seen: *files_seen,
                matches: out.len() as u64,
                archived,
                current_file: name_owned,
            };
            if let Err(e) = handle.emit("oa://library-scan-progress", progress) {
                log::warn!("scan_service: emit progress failed: {e:?}");
            }
        }
    }
}

pub fn next_job_id() -> u64 {
    NEXT_JOB_ID.fetch_add(1, Ordering::Relaxed)
}
