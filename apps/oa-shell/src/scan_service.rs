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
use std::time::Duration;

use rayon::prelude::*;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::archive;
use crate::library_db::LibraryDb;
use crate::rom_header::HeaderRule;
use crate::title_clean::title_from_file_name;

/// Per-archive deadline for the cancellable peek wrappers. A clean .zip
/// peek is single-digit milliseconds; a typical .7z is tens of ms. 15 s
/// is long enough that no honest archive hits it, short enough that the
/// scan can recover + log + skip a malformed one without the operator
/// staring at a frozen progress bar for minutes. Tuned for "user
/// patience" not "worst-case archive size."
const ARCHIVE_PEEK_TIMEOUT: Duration = Duration::from_secs(15);

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

/// How a row's `system_id` was determined. Read from highest to lowest
/// confidence. The Slice 2 per-ROM results table renders this as a
/// per-row badge (Hash beats Header beats Extension beats Hint).
///
/// Confidence is `None` on rows that never classified (extension not in
/// the registry mapping AND no `system_hint`). The frontend treats those
/// as "unmatched" and prompts the operator to pick a system.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    /// SHA-1 of the canonical (unheadered) bytes matched a libretro-
    /// database entry. Highest confidence — the canonical title and
    /// system are authoritative.
    Hash,
    /// SHA-1 of a header-stripped or byte-swapped view of the file
    /// matched a libretro-database entry. Same canonical accuracy as
    /// `Hash`, lower confidence only in the sense that we needed to
    /// apply a transform to the bytes first (catalog assumed a
    /// different on-disk shape).
    Header,
    /// File extension mapped to a registered system but no hash hit
    /// landed. Suggested title is the cleaned filename, not canonical.
    Extension,
    /// System came from a scan-time content peek (Neo Geo .zip ROM-set
    /// detection) or directory-mode hint (dosbox subdirs). Hash lookup
    /// not applicable for these row shapes.
    Hint,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannedRom {
    /// Encoded `<archive>#<inner>` for archived entries; raw path otherwise.
    pub path: String,
    pub file_name: String,
    pub extension: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive_inner_path: Option<String>,
    /// Optional system_id classification hint emitted by content-peek
    /// disambiguation (e.g. a .zip whose inner files match the SNK Neo
    /// Geo ROM-set signature gets `Some("neogeo")` so the frontend
    /// ingest path classifies it as `neogeo` rather than `mame` even
    /// though `.zip` is shared between the two systems).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_hint: Option<String>,
    /// Smart-classification result (Phase 1B Slice 1): the system the
    /// row was classified to. Populated by [`apply_smart_classification`]
    /// after the walk; the walk itself leaves this `None`. Prefers
    /// `system_hint` first, then the dispatcher-supplied
    /// `extension_to_system` mapping, finally any hash-hit canonical
    /// system_id override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_id: Option<String>,
    /// Title suggestion shown to the operator before commit. Defaults
    /// to the cleaned filename (`title_from_file_name`); a SHA-1 hash
    /// hit overrides it with the canonical `game_name`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_title: Option<String>,
    /// See [`Confidence`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<Confidence>,
    /// Stamped when a candidate SHA-1 hit the `rom_hashes` table.
    /// Lower-case hex. Slice 2 surfaces this as a tooltip; Phase 4
    /// folds it into the post-commit ingest write so the canonical
    /// identify pass doesn't have to re-hash everything.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha1: Option<String>,
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
    extension_to_system: HashMap<String, String>,
    db: &LibraryDb,
    cancel: Arc<AtomicBool>,
) -> Result<Vec<ScannedRom>, String> {
    let folder_str = folder.to_string_lossy().into_owned();
    log::info!(
        "scan_service: start job {job_id} folder={folder_str} extensions={} mappings={} cancel={:p}",
        wanted.len(),
        extension_to_system.len(),
        Arc::as_ptr(&cancel),
    );
    let started = std::time::Instant::now();
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

    // Final walk progress emit so the UI bar lands at 100% / last filename.
    emit_progress(&handle, job_id, &folder_str, files_seen, &out, "");

    // Phase 1B Slice 1: smart-classification pass. Walks the freshly-
    // collected rows, classifies each to a system via the supplied
    // extension→system mapping (overridden by `system_hint` when set),
    // hashes non-CD cart rows in parallel, and stamps the canonical
    // title + SHA-1 on hash hits. Bypasses entirely when the cancel
    // signal is already set — operator-cancelled walks short-circuit
    // before paying for the (potentially long) hash pass.
    if !cancel.load(Ordering::Relaxed) {
        let hash_started = std::time::Instant::now();
        apply_smart_classification(&mut out, &extension_to_system, db, &cancel);
        log::info!(
            "scan_service: smart-classification job {job_id} rows={} elapsed={}ms",
            out.len(),
            hash_started.elapsed().as_millis(),
        );
    }

    out.sort_by(|a, b| a.file_name.to_ascii_lowercase().cmp(&b.file_name.to_ascii_lowercase()));
    let cancelled = cancel.load(Ordering::Relaxed);
    log::info!(
        "scan_service: end job {job_id} files_seen={files_seen} matches={} cancelled={cancelled} elapsed={}ms",
        out.len(),
        started.elapsed().as_millis(),
    );
    Ok(out)
}

/// Highest-priority classification source for a row. Hint > extension map.
/// Returns `None` when neither source resolves — caller leaves the row
/// unclassified (Slice 2 surfaces these as "set a system" rows).
pub(crate) fn classify_row(
    row: &ScannedRom,
    extension_to_system: &HashMap<String, String>,
) -> Option<String> {
    if let Some(hint) = &row.system_hint {
        return Some(hint.clone());
    }
    extension_to_system.get(&row.extension).cloned()
}

/// Map a `candidate_sha1s` rule to the confidence tier we surface to the
/// operator. Raw bytes match → `Hash`; any header-strip / byte-swap
/// transform → `Header`.
pub(crate) fn confidence_from_rule(rule: HeaderRule) -> Confidence {
    match rule {
        HeaderRule::Raw => Confidence::Hash,
        HeaderRule::SkipMagic { .. }
        | HeaderRule::SkipModulo { .. }
        | HeaderRule::ByteSwap { .. } => Confidence::Header,
    }
}

/// Per-row work payload used inside the rayon parallel pass.
struct SmartClassifyResult {
    row_index: usize,
    system_id: Option<String>,
    suggested_title: Option<String>,
    confidence: Option<Confidence>,
    sha1: Option<String>,
}

/// Fold the smart-classification fields (`system_id` / `suggested_title`
/// / `confidence` / `sha1`) onto every row in `rows`. Mutates in place.
///
/// Stages per row:
///   1. Classify system from `system_hint` then `extension_to_system`.
///      Rows with no classification end up with all fields `None`
///      except `suggested_title` (always populated from the cleaned
///      filename so the UI has something to show).
///   2. For non-CD cart rows in a classified system, load bytes, compute
///      header-aware SHA-1 candidates, and look each up against
///      `rom_hashes`. First hit wins; `system_id` is overridden to the
///      canonical row's `system_id` (the DB knows the right system).
///   3. CD-shape rows + dir-mode rows + rows in systems with an empty
///      `rom_hashes` table stay at `Extension` or `Hint` confidence.
///
/// Parallelism mirrors `rom_hashes::resolve_rom_hashes_for_system`'s
/// pre-hash pass: rayon with a CPU-capped thread pool, scoped per call
/// so we don't share state across scans.
pub(crate) fn apply_smart_classification(
    rows: &mut Vec<ScannedRom>,
    extension_to_system: &HashMap<String, String>,
    db: &LibraryDb,
    cancel: &Arc<AtomicBool>,
) {
    if rows.is_empty() {
        return;
    }
    // Per-row classify + cleaned-title pre-pass (cheap, serial). Lets us
    // skip the parallel hash work for rows that can't possibly hit.
    #[derive(Clone)]
    struct Pre {
        row_index: usize,
        system_id: Option<String>,
        suggested_title: String,
        from_hint: bool,
        skip_hash: bool,
    }
    let prelim: Vec<Pre> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let from_hint = row.system_hint.is_some();
            let system_id = classify_row(row, extension_to_system);
            let suggested_title = title_from_file_name(&row.file_name);
            // Skip the hash pass for: unclassified rows, rows derived
            // from a hint (Neo Geo .zip / dosbox dir-mode — both are
            // structurally non-hashable), and CD-container extensions.
            let skip_hash = match &system_id {
                None => true,
                Some(sys) => from_hint || crate::rom_hashes::is_cd_container_ext(&row.extension, sys),
            };
            Pre { row_index: i, system_id, suggested_title, from_hint, skip_hash }
        })
        .collect();

    // Partition into hashable + non-hashable. Hashable rows go through
    // the rayon pool; non-hashable rows write their final state directly
    // without further work.
    let mut hashable: Vec<Pre> = Vec::new();
    let mut results: Vec<SmartClassifyResult> = Vec::with_capacity(rows.len());
    for p in prelim {
        if p.skip_hash {
            let confidence = match &p.system_id {
                Some(_) if p.from_hint => Some(Confidence::Hint),
                Some(_) => Some(Confidence::Extension),
                None => None,
            };
            results.push(SmartClassifyResult {
                row_index: p.row_index,
                system_id: p.system_id,
                suggested_title: Some(p.suggested_title),
                confidence,
                sha1: None,
            });
        } else {
            hashable.push(p);
        }
    }

    // Gate hash work per-system on the `rom_hashes` table being non-
    // empty. A first-time operator without auto-sync (or one whose
    // upstream fetch failed) shouldn't pay the cost of reading + SHA-1ing
    // every file just to look up against an empty table.
    let mut empty_systems: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    let mut populated_systems: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    for p in &hashable {
        if let Some(sys) = &p.system_id {
            if empty_systems.contains(sys) || populated_systems.contains(sys) {
                continue;
            }
            let count = db.count_rom_hashes(sys).unwrap_or(0);
            if count == 0 {
                empty_systems.insert(sys.clone());
            } else {
                populated_systems.insert(sys.clone());
            }
        }
    }

    // Split hashable into has-canonical and no-canonical buckets. No-
    // canonical bucket gets Extension confidence with no hash work.
    let (with_canonical, without_canonical): (Vec<Pre>, Vec<Pre>) = hashable
        .into_iter()
        .partition(|p| match &p.system_id {
            Some(sys) => populated_systems.contains(sys),
            None => false,
        });
    for p in without_canonical {
        results.push(SmartClassifyResult {
            row_index: p.row_index,
            system_id: p.system_id,
            suggested_title: Some(p.suggested_title),
            confidence: Some(Confidence::Extension),
            sha1: None,
        });
    }

    // Parallel hash pass on the rows we expect could hit.
    if !with_canonical.is_empty() {
        let cpu_count = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        let pool_size = cpu_count.min(4).max(1);
        // Snapshot the path + sysid + index per row so the closure
        // doesn't borrow `rows`. Rayon needs `Send + Sync` captures.
        let work: Vec<(usize, String, Option<String>, String, String)> = with_canonical
            .iter()
            .map(|p| {
                let row = &rows[p.row_index];
                (
                    p.row_index,
                    row.path.clone(),
                    row.archive_inner_path.clone(),
                    p.system_id.as_deref().unwrap_or("").to_string(),
                    p.suggested_title.clone(),
                )
            })
            .collect();
        let pool = match rayon::ThreadPoolBuilder::new()
            .num_threads(pool_size)
            .thread_name(|i| format!("oa-scan-hash-{i}"))
            .build()
        {
            Ok(p) => p,
            Err(e) => {
                log::warn!("scan_service: rayon pool build failed ({e}); falling back to serial");
                // Serial fallback — still produces correct results.
                for (row_index, path, inner, system_id, suggested_title) in work {
                    if cancel.load(Ordering::Relaxed) {
                        results.push(SmartClassifyResult {
                            row_index,
                            system_id: Some(system_id),
                            suggested_title: Some(suggested_title),
                            confidence: Some(Confidence::Extension),
                            sha1: None,
                        });
                        continue;
                    }
                    results.push(hash_one_row(
                        row_index,
                        &path,
                        inner.as_deref(),
                        &system_id,
                        &suggested_title,
                        db,
                    ));
                }
                apply_results(rows, results);
                return;
            }
        };
        let hashed: Vec<SmartClassifyResult> = pool.install(|| {
            work.into_par_iter()
                .map(|(row_index, path, inner, system_id, suggested_title)| {
                    if cancel.load(Ordering::Relaxed) {
                        return SmartClassifyResult {
                            row_index,
                            system_id: Some(system_id),
                            suggested_title: Some(suggested_title),
                            confidence: Some(Confidence::Extension),
                            sha1: None,
                        };
                    }
                    hash_one_row(
                        row_index,
                        &path,
                        inner.as_deref(),
                        &system_id,
                        &suggested_title,
                        db,
                    )
                })
                .collect()
        });
        results.extend(hashed);
    }

    apply_results(rows, results);
}

/// One row's hash work — extracted so the parallel + serial paths share
/// the same logic. Loads bytes, computes header-aware SHA-1 candidates,
/// looks up against the DB. First hit wins.
fn hash_one_row(
    row_index: usize,
    path: &str,
    inner: Option<&str>,
    system_id: &str,
    fallback_title: &str,
    db: &LibraryDb,
) -> SmartClassifyResult {
    let bytes = match crate::rom_hashes::rom_bytes_for(path, inner) {
        Ok(b) => b,
        Err(e) => {
            log::debug!("scan_service: hash skip {path}: {e}");
            return SmartClassifyResult {
                row_index,
                system_id: Some(system_id.to_string()),
                suggested_title: Some(fallback_title.to_string()),
                confidence: Some(Confidence::Extension),
                sha1: None,
            };
        }
    };
    let candidates = crate::rom_header::candidate_sha1s(&bytes, system_id);
    for cand in &candidates {
        match db.lookup_rom_hash(&cand.sha1) {
            Ok(Some(hit)) => {
                return SmartClassifyResult {
                    row_index,
                    // Canonical system wins over our extension classification.
                    // The DB row's system_id is authoritative because hash
                    // entries are populated from per-system dat fetches.
                    system_id: Some(hit.system_id),
                    suggested_title: Some(hit.game_name),
                    confidence: Some(confidence_from_rule(cand.rule)),
                    sha1: Some(cand.sha1.clone()),
                };
            }
            Ok(None) => continue,
            Err(e) => {
                log::warn!("scan_service: lookup_rom_hash({}) failed: {e}", cand.sha1);
            }
        }
    }
    SmartClassifyResult {
        row_index,
        system_id: Some(system_id.to_string()),
        suggested_title: Some(fallback_title.to_string()),
        confidence: Some(Confidence::Extension),
        sha1: None,
    }
}

/// Write every result back onto its row. Indices are stable because
/// nothing has reordered `rows` since the parallel pass started.
fn apply_results(rows: &mut [ScannedRom], results: Vec<SmartClassifyResult>) {
    for r in results {
        if let Some(row) = rows.get_mut(r.row_index) {
            row.system_id = r.system_id;
            row.suggested_title = r.suggested_title;
            row.confidence = r.confidence;
            row.sha1 = r.sha1;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn walk(
    dir: &Path,
    depth: u32,
    max_depth: u32,
    wanted: &HashSet<String>,
    cancel: &Arc<AtomicBool>,
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
            // Per-archive diagnostic line — the LAST log entry before
            // a freeze names the file that hung, so an operator-reported
            // "Cancel button doesn't respond" investigation can identify
            // the culprit archive from oa-current.log without having to
            // bisect their library. log::debug would be nicer for
            // signal-to-noise but most users run at INFO level and
            // wouldn't see it; the line is a single ~120 bytes so a
            // 1000-archive folder costs ~120 KB of log.
            log::info!(
                "scan_service: peek archive {} ({} bytes)",
                entry_path.display(),
                std::fs::metadata(&entry_path).map(|m| m.len()).unwrap_or(0),
            );

            // Neo Geo .zip content-peek disambiguation. A .zip whose
            // inner files match the Neo Geo ROM-set signature (.p1 +
            // .s1) gets emitted as a standalone ScannedRom for the
            // whole zip with system_hint="neogeo" — the frontend's
            // ingest path uses the hint to classify ahead of the
            // generic extension-based mapping (which would otherwise
            // route .zip files to MAME by default). MAME zips fall
            // through to the normal archive-enumeration path below.
            //
            // Both peeks run through the cancellable wrappers because
            // either one can hang on a malformed archive (zip-rs's
            // central-directory parser can seek-loop on certain bad
            // ZIP64 entries; sevenz-rust's metadata walk shares the
            // same surface). The wrappers poll cancel every ~100 ms
            // and give up entirely after `ARCHIVE_PEEK_TIMEOUT`, so
            // the operator's Cancel button reaches the walker quickly
            // and one bad file in a 1000-file folder doesn't freeze
            // the whole import.
            let mut handled_as_neogeo = false;
            if ext == "zip" {
                match archive::peek_zip_for_neogeo_cancellable(
                    entry_path.clone(),
                    Arc::clone(cancel),
                    ARCHIVE_PEEK_TIMEOUT,
                ) {
                    Ok(true) => {
                        out.push(ScannedRom {
                            path: entry_path.to_string_lossy().into_owned(),
                            file_name: name_owned.clone(),
                            extension: ext.clone(),
                            archive_inner_path: None,
                            system_hint: Some("neogeo".to_string()),
                            ..ScannedRom::default()
                        });
                        handled_as_neogeo = true;
                    }
                    Ok(false) => {}
                    Err(archive::PeekFailure::Cancelled) => return,
                    Err(archive::PeekFailure::TimedOut) => {
                        log::warn!(
                            "scan_service: neogeo peek timed out after {}s, skipping: {}",
                            ARCHIVE_PEEK_TIMEOUT.as_secs(),
                            entry_path.display(),
                        );
                        continue;
                    }
                    Err(archive::PeekFailure::Failed(e)) => {
                        log::warn!(
                            "scan_service: neogeo peek {} failed: {e}",
                            entry_path.display(),
                        );
                        // Fall through to the generic list_rom_contents
                        // attempt — a peek failure here doesn't mean
                        // the full enumeration will also fail.
                    }
                }
            }
            if !handled_as_neogeo {
                match archive::list_rom_contents_cancellable(
                    entry_path.clone(),
                    wanted.clone(),
                    Arc::clone(cancel),
                    ARCHIVE_PEEK_TIMEOUT,
                ) {
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
                                system_hint: None,
                                ..ScannedRom::default()
                            });
                        }
                    }
                    Err(archive::PeekFailure::Cancelled) => return,
                    Err(archive::PeekFailure::TimedOut) => {
                        log::warn!(
                            "scan_service: archive peek timed out after {}s, skipping: {}",
                            ARCHIVE_PEEK_TIMEOUT.as_secs(),
                            entry_path.display(),
                        );
                    }
                    Err(archive::PeekFailure::Failed(e)) => {
                        log::warn!(
                            "scan_service: peek {} failed: {e}",
                            entry_path.display(),
                        );
                    }
                }
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
                system_hint: None,
                ..ScannedRom::default()
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

/// Directory-mode scan — walk the folder at exactly `depth` levels deep
/// and emit one [`ScannedRom`] per subdirectory found. Used by the
/// dosbox onboarding flow (Phase 2 of `feat/dosbox-and-scummvm`): each
/// top-level subdir of an operator-marked "DOS Games" folder is one
/// game; nested dirs inside are content (game data files), not nested
/// games.
///
/// Currently `depth == 1` is the only supported value — operator points
/// at a parent folder, OA enumerates its direct subdirectories.
/// Different cores (a hypothetical future engine launcher with deeper
/// game-per-folder shape) can lift the depth without rewriting the
/// walker.
///
/// Emitted rows carry `extension = ""` and `archive_inner_path = None`;
/// `system_hint` is filled with the supplied `system_id_hint` so the
/// frontend's ingest path classifies the row to the right system
/// without re-deriving from filename extension (which doesn't exist).
pub fn run_dir_scan_blocking(
    job_id: u64,
    handle: AppHandle,
    folder: PathBuf,
    depth: u32,
    system_id_hint: String,
    db: &LibraryDb,
    cancel: Arc<AtomicBool>,
) -> Result<Vec<ScannedRom>, String> {
    if depth != 1 {
        return Err(format!(
            "run_dir_scan_blocking: only depth = 1 is supported (got {depth})"
        ));
    }
    let folder_str = folder.to_string_lossy().into_owned();
    let mut out = Vec::new();

    let Ok(entries) = std::fs::read_dir(&folder) else {
        return Ok(out);
    };
    let mut seen: u64 = 0;
    for entry in entries.flatten() {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let Ok(file_type) = entry.file_type() else { continue };
        if !file_type.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name_owned: String = name.to_string_lossy().into_owned();
        if name_owned.starts_with('.') {
            continue;
        }
        let dir_path = entry.path();
        out.push(ScannedRom {
            path: dir_path.to_string_lossy().into_owned(),
            file_name: name_owned.clone(),
            extension: String::new(),
            archive_inner_path: None,
            system_hint: Some(system_id_hint.clone()),
            ..ScannedRom::default()
        });
        seen += 1;
        // Throttled progress emit — one per directory found is fine at
        // 1-level-deep scale (operator collections rarely have thousands
        // of top-level subdirs).
        let progress = ScanProgress {
            job_id,
            folder: folder_str.clone(),
            files_seen: seen,
            matches: out.len() as u64,
            archived: 0,
            current_file: name_owned,
        };
        if let Err(e) = handle.emit("oa://library-scan-progress", progress) {
            log::warn!("scan_service: emit dir-scan progress failed: {e:?}");
        }
    }

    // Phase 1B Slice 1: smart-classification pass also runs on dir-mode
    // rows. All rows here carry `system_hint`, so the pass short-circuits
    // the hash work and stamps `Confidence::Hint` + a cleaned title. We
    // pass an empty extension_to_system map because dir-mode rows
    // classify purely via the hint.
    if !cancel.load(Ordering::Relaxed) {
        let empty: HashMap<String, String> = HashMap::new();
        apply_smart_classification(&mut out, &empty, db, &cancel);
    }

    out.sort_by(|a, b| a.file_name.to_ascii_lowercase().cmp(&b.file_name.to_ascii_lowercase()));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rom_header::{ByteSwapKind, HeaderRule};

    fn row(extension: &str, system_hint: Option<&str>) -> ScannedRom {
        ScannedRom {
            path: format!("/fake/{extension}.rom"),
            file_name: format!("game.{extension}"),
            extension: extension.to_string(),
            system_hint: system_hint.map(|s| s.to_string()),
            ..ScannedRom::default()
        }
    }

    #[test]
    fn classify_row_prefers_hint_over_extension_lookup() {
        let mut m = HashMap::new();
        m.insert("zip".to_string(), "mame".to_string());
        let r = row("zip", Some("neogeo"));
        assert_eq!(classify_row(&r, &m), Some("neogeo".to_string()));
    }

    #[test]
    fn classify_row_falls_back_to_extension_map_when_no_hint() {
        let mut m = HashMap::new();
        m.insert("smc".to_string(), "snes".to_string());
        let r = row("smc", None);
        assert_eq!(classify_row(&r, &m), Some("snes".to_string()));
    }

    #[test]
    fn classify_row_unknown_extension_no_hint_returns_none() {
        let m = HashMap::new();
        let r = row("xyz", None);
        assert_eq!(classify_row(&r, &m), None);
    }

    #[test]
    fn classify_row_dir_mode_hint_overrides_empty_extension() {
        // Directory-mode rows (dosbox) carry no extension but always have
        // a `system_hint`. The hint should win.
        let m = HashMap::new();
        let r = row("", Some("dosbox"));
        assert_eq!(classify_row(&r, &m), Some("dosbox".to_string()));
    }

    #[test]
    fn confidence_from_rule_raw_is_hash() {
        assert_eq!(confidence_from_rule(HeaderRule::Raw), Confidence::Hash);
    }

    #[test]
    fn confidence_from_rule_skip_magic_is_header() {
        let rule = HeaderRule::SkipMagic {
            skip: 16,
            magic_offset: 0,
            magic_bytes: b"NES\x1a",
        };
        assert_eq!(confidence_from_rule(rule), Confidence::Header);
    }

    #[test]
    fn confidence_from_rule_skip_modulo_is_header() {
        assert_eq!(
            confidence_from_rule(HeaderRule::SkipModulo { skip: 512, modulus: 1024 }),
            Confidence::Header
        );
    }

    #[test]
    fn confidence_from_rule_byte_swap_is_header() {
        let rule = HeaderRule::ByteSwap {
            magic_at_offset_0: &[0x37, 0x80, 0x40, 0x12],
            swap: ByteSwapKind::Pairs16,
        };
        assert_eq!(confidence_from_rule(rule), Confidence::Header);
    }
}
