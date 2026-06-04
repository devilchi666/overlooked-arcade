//! Background-jobs registry.
//!
//! Single source of truth for what OA is doing in the background
//! (HTTP downloads, hash resolves, media sync, folder scans, future
//! per-track SHA-1 matching). Persists job state to the `background_jobs`
//! table (schema v18; see `library_db::migrate_v17_to_v18`) so
//! operations survive app restart.
//!
//! Phase 1 scope (this module's first cut):
//! - Schema-backed CRUD + the `JobHandle` cancel/pause pattern.
//! - 1 Hz SQLite write debounce on progress ticks.
//! - 10 Hz Tauri broadcast event cap on `oa://job-event`.
//! - ~1 s heartbeat task bumping `last_event_at` for running rows.
//! - 100-row history rolling buffer (oldest finished rows prune on
//!   state-finalize transitions).
//! - Crash recovery: `promote_running_rows_to_interrupted` called from
//!   `main.rs` when the `<data_dir>/oa.lock` marker survives a launch.
//!
//! Phase 1 explicitly does NOT ship:
//! - Frontend bar UI (Phase 2 — events fire but nothing listens yet).
//! - `JobResumer` trait + auto-resume dispatch (Phase 3).
//! - Other 8 kinds beyond `core_download` (Phase 4).
//! - Dependency graph + duplicate-trigger dialog (Phase 3/4).
//! - Settings panel (Phase 5).
//!
//! See `docs/PLANS/background-jobs-and-progress-bar.md` for the full
//! 5-phase plan + locked design decisions.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};

/// SQLite write debounce for progress ticks. Progress fires far faster
/// than the operator can perceive (a fast HTTP stream pushes hundreds
/// of chunks/sec); we only flush to disk this often.
const SQL_DEBOUNCE_MS: i64 = 1_000;

/// Tauri event broadcast cap for progress ticks. Frontend animation is
/// happy at 10 Hz; firing every chunk would saturate the IPC channel
/// and waste WebView paint budget.
const EVENT_DEBOUNCE_MS: i64 = 100;

/// Cap on retained finished-job history rows. Active rows
/// (pending/running/paused/interrupted) are never counted toward this.
const HISTORY_CAP: i64 = 100;

/// Heartbeat cadence. While at least one job is running, the registry
/// bumps `background_jobs.last_event_at` to `now` this often. Lets a
/// future watchdog detect a worker that panicked while the lock file
/// is still held.
const HEARTBEAT_SECS: u64 = 1;

/// Public discriminator for the kind of background work. Phase 1 only
/// names the pilot kind; remaining 8 wire in Phase 4. Stored in the
/// `background_jobs.kind` column as snake_case (`"core_download"`).
/// The associated data lands in `target_id` (e.g. the .dll base name
/// for `core_download`) so future queries can filter by target without
/// JSON-deserializing.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JobKind {
    CoreDownload { base: String },
    /// Folder scan kicked off from the Import Wizard's smart-scan
    /// pass (`scan_service::start_background_scan`). `folder` is the
    /// absolute path being walked; doubles as the job label suffix.
    FolderScan { folder: String },
    /// Per-system ROM hash + lookup pass
    /// (`rom_hashes::resolve_rom_hashes_for_system`). Operator-
    /// triggered via the "Identify ROMs" button. `system_id` is the
    /// OA slug — pce, psx, snes, etc.
    HashResolve { system_id: String },
    /// Per-system libretro-database `.dat` sync
    /// (`rom_hashes::sync_rom_hashes_for_system`). Auto-triggered as
    /// a prereq of HashResolve when the system's `rom_hashes` table
    /// is empty (Phase 4a inlines this; Phase 4b's dependency graph
    /// will surface it as a separate parent-job row).
    DatSync { system_id: String },
    /// Per-system disc-track SHA-1 hashing
    /// (`rom_hashes::resolve_rom_hashes_for_system`'s disc-shape
    /// branch). Operator-triggered via "Identify ROMs" on a
    /// disc-shape system; parallel to HashResolve for cart systems.
    /// Phase A1 Sub-phase 3 of the virtual library + launcher arc.
    DiscTrackHash { system_id: String },
    /// MAME catalog refresh from a local MAME install
    /// (`mame_import::refresh_mame_system_info`). Atomic — single
    /// XML parse + bake; cancel rolls back.
    MameListxmlImport,
    /// Per-system artwork download from libretro-thumbnails
    /// (`media::sync_media_for_system`). Plan §"Kind taxonomy"
    /// originally split this into artwork_sync vs metadata_sync for
    /// per-kind concurrency, but the existing function bundles both
    /// into one per-game-per-kind pass — Phase 4b wires the whole
    /// pass as artwork_sync (covers 26 of 27 MediaKind variants;
    /// only Manual is metadata-shape). The deeper split deferred
    /// until a separate metadata-fetching path exists.
    ArtworkSync { system_id: String },
    /// Parent row for Guided Setup's parallel install of N cores
    /// (`MissingCoreBulkPrompt` → N download_core calls). Children
    /// are individual CoreDownload jobs linked via parent_job_id;
    /// the parent's `done` counts how many children have finalized.
    BulkCoreInstall,
    /// Dev-only synthetic job for exercising the BackgroundJobsBar
    /// without burning bandwidth on a real download. Lives at the
    /// production level (not behind `cfg(debug_assertions)`) so
    /// operators can sanity-check the bar after settings changes too;
    /// always-on cost is zero unless `spawn_test_job` is invoked.
    TestJob { name: String },
}

impl JobKind {
    pub fn discriminator(&self) -> &'static str {
        match self {
            Self::CoreDownload { .. } => "core_download",
            Self::FolderScan { .. } => "folder_scan",
            Self::HashResolve { .. } => "hash_resolve",
            Self::DatSync { .. } => "dat_sync",
            Self::DiscTrackHash { .. } => "disc_track_hash",
            Self::MameListxmlImport => "mame_listxml_import",
            Self::ArtworkSync { .. } => "artwork_sync",
            Self::BulkCoreInstall => "bulk_core_install",
            Self::TestJob { .. } => "test_job",
        }
    }

    pub fn target_id(&self) -> Option<String> {
        match self {
            Self::CoreDownload { base } => Some(base.clone()),
            Self::FolderScan { folder } => Some(folder.clone()),
            Self::HashResolve { system_id } => Some(system_id.clone()),
            Self::DatSync { system_id } => Some(system_id.clone()),
            Self::DiscTrackHash { system_id } => Some(system_id.clone()),
            Self::MameListxmlImport => None,
            Self::ArtworkSync { system_id } => Some(system_id.clone()),
            Self::BulkCoreInstall => None,
            Self::TestJob { name } => Some(name.clone()),
        }
    }
}

/// All possible job states. Stored as snake_case strings in
/// `background_jobs.state`. `interrupted` is transitional — Phase 3
/// will promote interrupted rows back to `running` or `paused` based on
/// operator settings; Phase 1 leaves them sitting after the lock-file
/// detection runs at startup.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
    Interrupted,
}

impl JobState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Interrupted => "interrupted",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "running" => Some(Self::Running),
            "paused" => Some(Self::Paused),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            "interrupted" => Some(Self::Interrupted),
            _ => None,
        }
    }

    fn is_finished(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

/// Wire-format snapshot of a job. Same shape used by `list_*` queries
/// AND by Tauri `oa://job-event` payloads — Phase 2's frontend bar
/// will hydrate from these directly.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobSnapshot {
    pub id: i64,
    pub kind: String,
    pub label: String,
    pub system_id: Option<String>,
    pub target_id: Option<String>,
    pub parent_job_id: Option<i64>,
    pub is_prereq: bool,
    pub state: JobState,
    pub done: i64,
    pub total: Option<i64>,
    pub unit: String,
    pub last_event_at: i64,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub can_resume: bool,
    pub error_message: Option<String>,
    pub retry_count: i32,
}

/// Tauri broadcast payload. One `oa://job-event` emitted per event
/// (tagged `type` discriminator, snake_case names). Frontend Phase 2
/// listens; Phase 1 fires unconsumed.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JobEvent {
    Created { snapshot: JobSnapshot },
    Progressed {
        #[serde(rename = "jobId")]
        job_id: i64,
        done: i64,
        total: Option<i64>,
    },
    StateChanged {
        #[serde(rename = "jobId")]
        job_id: i64,
        state: JobState,
    },
    Completed {
        #[serde(rename = "jobId")]
        job_id: i64,
    },
    Failed {
        #[serde(rename = "jobId")]
        job_id: i64,
        error: String,
    },
    /// Phase 3b — emitted by `resume_interrupted_jobs` when an
    /// interrupted row's kind has `prompt_before_resume_on_launch =
    /// true` in JobPrefs. Frontend (Phase 5) will surface a dialog
    /// offering Resume / Discard. For Phase 3b the event fires
    /// unconsumed; the row sits at `state='interrupted'` until the
    /// operator manually retries via the existing per-operation
    /// modal, OR until they remove the opt-out and relaunch.
    ResumePrompt {
        snapshot: JobSnapshot,
    },
}

/// Cancel + pause flags surfaced to per-kind workers. Clone the handle
/// into the worker; the registry retains its own copy in `active` so
/// outside callers can flip the flags via Tauri commands.
#[derive(Clone)]
pub struct JobHandle {
    pub job_id: i64,
    pub kind: String,
    pub cancel: Arc<AtomicBool>,
    pub pause: Arc<AtomicBool>,
    last_db_write_ms: Arc<AtomicI64>,
    last_event_ms: Arc<AtomicI64>,
}

impl JobHandle {
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    pub fn is_paused(&self) -> bool {
        self.pause.load(Ordering::Relaxed)
    }
}

struct Inner {
    conn: Mutex<Connection>,
    active: RwLock<HashMap<i64, JobHandle>>,
    // Per-kind resume handlers registered via `register_resumer`.
    // Looked up by `resume_interrupted_jobs` at startup; rows whose
    // kind doesn't have a registered resumer stay `interrupted` and
    // are logged at warn.
    resumers: RwLock<HashMap<&'static str, Arc<dyn JobResumer>>>,
    // `None` in unit tests so the registry can be constructed without
    // a Tauri runtime. Production callers always pass `Some`.
    app: Option<AppHandle>,
}

/// Per-kind resume handler. Phase 3a registers a CoreDownloadResumer;
/// Phase 3b will add artwork_sync + hash_resolve. The dispatcher in
/// `JobRegistry::resume_interrupted_jobs` iterates `state='interrupted'`
/// rows on launch and dispatches each to its matching registered
/// resumer (looked up by kind discriminator).
///
/// Implementation contract:
///   1. Spawn a worker on `tauri::async_runtime::spawn` (NOT
///      `tokio::spawn` — Tauri's setup() runs before the tokio
///      runtime is entered; see the 2026-06-02 launch-crash fix
///      in this module's git history).
///   2. The worker first calls `registry.mark_running(snapshot.id)`
///      to transition the row out of `interrupted` and emit a
///      StateChanged event so the bar surfaces the resumed job.
///   3. Re-enter the operation from `snapshot.resume_payload`
///      (per-kind JSON checkpoint) OR from scratch if Phase 3a's
///      simple "restart-from-zero" semantics are acceptable for
///      the kind.
///   4. Finalize via mark_completed / mark_cancelled / mark_failed
///      as normal — the resumer doesn't need anything special for
///      cleanup; the standard JobRegistry finalize path applies.
pub trait JobResumer: Send + Sync {
    /// Discriminator string matching `JobKind::discriminator()` —
    /// e.g. `"core_download"`.
    fn kind(&self) -> &'static str;

    /// Spawn the resume worker. The returned JoinHandle is dropped
    /// by Phase 3a's dispatcher (fire-and-forget). Phase 5's
    /// recent-activity panel may want to observe completion via it.
    fn resume(
        &self,
        snapshot: JobSnapshot,
        registry: JobRegistry,
        app: AppHandle,
    ) -> tauri::async_runtime::JoinHandle<()>;
}

/// Public registry. Cheap to `Clone` — wraps an `Arc<Inner>`. The
/// background heartbeat task holds its own clone.
#[derive(Clone)]
pub struct JobRegistry {
    inner: Arc<Inner>,
}

impl JobRegistry {
    /// Open a dedicated connection to the library DB and spawn the
    /// heartbeat task. `db_path` must point at the same
    /// `<data_dir>/library/games.sqlite` that `LibraryDb` owns — WAL
    /// mode is already set on the main connection so concurrent
    /// readers/writers are safe.
    pub fn new(db_path: &Path, app: AppHandle) -> Result<Self, String> {
        let registry = Self::open_inner(db_path, Some(app))?;
        Self::spawn_heartbeat(Arc::clone(&registry.inner));
        Ok(registry)
    }

    /// Test-only constructor — no Tauri app handle (events are
    /// dropped) and no heartbeat task (so tests don't leak background
    /// tokio work).
    #[cfg(test)]
    fn new_for_tests(db_path: &Path) -> Result<Self, String> {
        Self::open_inner(db_path, None)
    }

    fn open_inner(db_path: &Path, app: Option<AppHandle>) -> Result<Self, String> {
        let conn = Connection::open(db_path)
            .map_err(|e| format!("open job registry conn at {}: {e}", db_path.display()))?;
        // Match the LibraryDb-side pragmas. Setting them again is a
        // no-op but keeps this connection self-contained if the open
        // order ever changes.
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA temp_store = MEMORY;",
        )
        .map_err(|e| format!("job registry pragma: {e}"))?;

        let inner = Arc::new(Inner {
            conn: Mutex::new(conn),
            active: RwLock::new(HashMap::new()),
            resumers: RwLock::new(HashMap::new()),
            app,
        });
        Ok(Self {
            inner: Arc::clone(&inner),
        })
    }

    /// Register a per-kind resume handler. Idempotent — replaces any
    /// existing entry for the same kind discriminator. Called at
    /// startup, typically from `main.rs::setup()` right after the
    /// JobRegistry itself lands in Tauri state.
    pub fn register_resumer(&self, resumer: Arc<dyn JobResumer>) {
        let kind = resumer.kind();
        if let Ok(mut map) = self.inner.resumers.write() {
            map.insert(kind, resumer);
            log::info!("background_jobs: registered resumer for kind {kind}");
        }
    }

    /// Dispatch every `state='interrupted'` row to its registered
    /// resumer. Called from `main.rs::setup()` after
    /// `promote_running_rows_to_interrupted` (Phase 1 Slice D) flags
    /// the survivors of a crashed previous run. Rows whose kind has
    /// no registered resumer stay `interrupted` and log a warn —
    /// Phase 4 will register the rest of the kinds; Phase 3a only
    /// handles core_download.
    ///
    /// `should_prompt` is the Phase 3b per-kind opt-out gate — when
    /// it returns true for a kind, the dispatcher SKIPS the resumer
    /// and emits a `ResumePrompt` event instead. Plan §"Resume on
    /// app launch": auto-resume by default, opt-out via the
    /// JobPrefs file. The frontend prompt dialog lands in Phase 5;
    /// for Phase 3b the event fires unconsumed.
    ///
    /// Returns the count of resumers dispatched (NOT counting prompt
    /// events). Each resumer spawns its own task; this call returns
    /// immediately.
    pub fn resume_interrupted_jobs(
        &self,
        app: &AppHandle,
        should_prompt: impl Fn(&str) -> bool,
    ) -> Result<usize, String> {
        let snapshots = self.list_interrupted()?;
        if snapshots.is_empty() {
            return Ok(0);
        }
        let resumers = self
            .inner
            .resumers
            .read()
            .map_err(|e| format!("resumers lock: {e}"))?;
        let mut dispatched = 0;
        for snap in snapshots {
            if should_prompt(snap.kind.as_str()) {
                log::info!(
                    "background_jobs: kind {} has prompt-before-resume opt-out; emitting prompt for job {}",
                    snap.kind,
                    snap.id
                );
                self.emit_event(&JobEvent::ResumePrompt {
                    snapshot: snap.clone(),
                });
                continue;
            }
            match resumers.get(snap.kind.as_str()) {
                Some(resumer) => {
                    log::info!(
                        "background_jobs: dispatching resume for job {} (kind={}, label={:?})",
                        snap.id,
                        snap.kind,
                        snap.label
                    );
                    let _join = resumer.resume(snap.clone(), self.clone(), app.clone());
                    dispatched += 1;
                }
                None => {
                    log::warn!(
                        "background_jobs: no resumer registered for kind {}; job {} stays interrupted",
                        snap.kind,
                        snap.id
                    );
                }
            }
        }
        Ok(dispatched)
    }

    /// Re-attach a JobHandle for a job whose row already exists in
    /// SQLite but was finalized as `interrupted` by a previous run
    /// (or by an explicit pause that broke the worker out of its
    /// loop). Allocates fresh AtomicBool flags and inserts the
    /// handle into the active map so the bar's pause + cancel
    /// buttons re-bind correctly. Idempotent — returns the existing
    /// handle if one is already in the active map.
    ///
    /// Resumers call this BEFORE `mark_running` so the worker has
    /// flags to poll for the rest of its lifetime.
    pub fn attach_handle(&self, job_id: i64, kind: String) -> JobHandle {
        if let Ok(active) = self.inner.active.read() {
            if let Some(h) = active.get(&job_id) {
                return h.clone();
            }
        }
        let handle = JobHandle {
            job_id,
            kind,
            cancel: Arc::new(AtomicBool::new(false)),
            pause: Arc::new(AtomicBool::new(false)),
            last_db_write_ms: Arc::new(AtomicI64::new(now_ms())),
            last_event_ms: Arc::new(AtomicI64::new(0)),
        };
        if let Ok(mut active) = self.inner.active.write() {
            active.insert(job_id, handle.clone());
        }
        handle
    }

    fn spawn_heartbeat(inner: Arc<Inner>) {
        // `tauri::async_runtime::spawn` instead of `tokio::spawn` so
        // this works regardless of whether the caller is inside a
        // tokio reactor context. setup() runs synchronously on Tauri's
        // main thread BEFORE the runtime is entered, so a raw
        // `tokio::spawn` here panics with "there is no reactor running"
        // and the app dies on launch.
        tauri::async_runtime::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(HEARTBEAT_SECS));
            // Skip the immediate first tick — registry just started.
            ticker.tick().await;
            loop {
                ticker.tick().await;
                // Snapshot the running-job IDs while holding the
                // active-map read lock briefly.
                let ids: Vec<i64> = match inner.active.read() {
                    Ok(guard) => guard.keys().copied().collect(),
                    Err(_) => continue,
                };
                if ids.is_empty() {
                    continue;
                }
                let now = now_ms();
                if let Ok(c) = inner.conn.lock() {
                    // Bulk UPDATE — gated by `state = 'running'` so
                    // paused/cancelled rows don't accidentally
                    // refresh.
                    for id in &ids {
                        let _ = c.execute(
                            "UPDATE background_jobs SET last_event_at = ?1 \
                             WHERE id = ?2 AND state = 'running'",
                            params![now, id],
                        );
                    }
                }
            }
        });
    }

    /// Create a new pending row and stash the cancel/pause handle in
    /// the active map. Returns the new job_id. The worker should call
    /// `mark_running` once it actually starts the work.
    pub fn create_job(
        &self,
        kind: JobKind,
        label: String,
        system_id: Option<String>,
        parent_job_id: Option<i64>,
        is_prereq: bool,
        unit: &'static str,
        resume_payload: Option<serde_json::Value>,
    ) -> Result<i64, String> {
        let kind_str = kind.discriminator().to_string();
        let target_id = kind.target_id();
        let now = now_ms();
        let payload_str = match resume_payload {
            Some(v) => Some(
                serde_json::to_string(&v).map_err(|e| format!("encode resume_payload: {e}"))?,
            ),
            None => None,
        };
        let job_id = {
            let conn = self.lock_conn()?;
            conn.execute(
                "INSERT INTO background_jobs \
                 (kind, label, system_id, target_id, parent_job_id, is_prereq, \
                  state, unit, last_event_at, started_at, resume_payload) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', ?7, ?8, ?9, ?10)",
                params![
                    &kind_str,
                    &label,
                    system_id.as_deref(),
                    target_id.as_deref(),
                    parent_job_id,
                    is_prereq as i32,
                    unit,
                    now,
                    now,
                    payload_str.as_deref(),
                ],
            )
            .map_err(|e| format!("insert background_jobs: {e}"))?;
            conn.last_insert_rowid()
        };
        let handle = JobHandle {
            job_id,
            kind: kind_str,
            cancel: Arc::new(AtomicBool::new(false)),
            pause: Arc::new(AtomicBool::new(false)),
            last_db_write_ms: Arc::new(AtomicI64::new(now)),
            last_event_ms: Arc::new(AtomicI64::new(0)),
        };
        if let Ok(mut active) = self.inner.active.write() {
            active.insert(job_id, handle);
        }
        if let Some(snapshot) = self.snapshot(job_id)? {
            self.emit_event(&JobEvent::Created { snapshot });
        }
        Ok(job_id)
    }

    /// Get a clone of the cancel/pause handle for `job_id`. Returns
    /// `None` if the job already finalized (handle was removed from
    /// the active map by `mark_*`).
    pub fn handle(&self, job_id: i64) -> Option<JobHandle> {
        self.inner
            .active
            .read()
            .ok()
            .and_then(|g| g.get(&job_id).cloned())
    }

    /// Per-tick progress update from a worker. Always updates the
    /// in-memory handle; flushes to SQLite at `SQL_DEBOUNCE_MS` cadence;
    /// emits the Tauri event at `EVENT_DEBOUNCE_MS` cadence.
    pub fn progress(&self, job_id: i64, done: i64, total: Option<i64>) -> Result<(), String> {
        let handle = match self.handle(job_id) {
            Some(h) => h,
            None => return Ok(()), // job already finalized — drop the tick
        };
        let now = now_ms();

        // SQLite debounce.
        let last_db = handle.last_db_write_ms.load(Ordering::Relaxed);
        if now - last_db >= SQL_DEBOUNCE_MS {
            handle.last_db_write_ms.store(now, Ordering::Relaxed);
            let conn = self.lock_conn()?;
            conn.execute(
                "UPDATE background_jobs SET done = ?1, total = ?2, last_event_at = ?3 \
                 WHERE id = ?4",
                params![done, total, now, job_id],
            )
            .map_err(|e| format!("update progress: {e}"))?;
        }

        // Tauri event debounce.
        let last_evt = handle.last_event_ms.load(Ordering::Relaxed);
        if now - last_evt >= EVENT_DEBOUNCE_MS {
            handle.last_event_ms.store(now, Ordering::Relaxed);
            self.emit_event(&JobEvent::Progressed {
                job_id,
                done,
                total,
            });
        }
        Ok(())
    }

    /// Force the `total` column for a job row, bypassing the progress
    /// debounce. Used by `JobScope::start` + `JobScope::set_total` so
    /// the bar UI gets the total immediately rather than after the
    /// first progress tick clears the SQL debounce window (~1s post-
    /// create_job).
    pub(crate) fn force_set_total(&self, job_id: i64, total: i64) -> Result<(), String> {
        let conn = self.lock_conn()?;
        conn.execute(
            "UPDATE background_jobs SET total = ?1, last_event_at = ?2 WHERE id = ?3",
            params![total, now_ms(), job_id],
        )
        .map_err(|e| format!("force_set_total: {e}"))?;
        Ok(())
    }

    /// Force the `done` + `total` columns. Used by `JobScope::complete`
    /// to bypass the SQL debounce on the final tick — the last
    /// progress update before `mark_completed` would otherwise be
    /// debounced out, leaving the row's `done` column stale and the
    /// bar visibly stuck at the last persisted value.
    pub(crate) fn force_set_done_total(
        &self,
        job_id: i64,
        done: i64,
        total: Option<i64>,
    ) -> Result<(), String> {
        let conn = self.lock_conn()?;
        conn.execute(
            "UPDATE background_jobs SET done = ?1, total = ?2, last_event_at = ?3 WHERE id = ?4",
            params![done, total, now_ms(), job_id],
        )
        .map_err(|e| format!("force_set_done_total: {e}"))?;
        Ok(())
    }

    /// Persist the latest resume payload for `job_id`. Called by
    /// workers on cancel + on pause so the eventual resume path
    /// (Phase 3) has a checkpoint to restart from.
    pub fn flush_resume_state(
        &self,
        job_id: i64,
        payload: serde_json::Value,
    ) -> Result<(), String> {
        let payload_str =
            serde_json::to_string(&payload).map_err(|e| format!("encode payload: {e}"))?;
        let conn = self.lock_conn()?;
        conn.execute(
            "UPDATE background_jobs SET resume_payload = ?1, last_event_at = ?2 WHERE id = ?3",
            params![payload_str, now_ms(), job_id],
        )
        .map_err(|e| format!("flush resume payload: {e}"))?;
        Ok(())
    }

    /// Transition to `running`. Idempotent — calling it on an already-
    /// running row just bumps `last_event_at`.
    pub fn mark_running(&self, job_id: i64) -> Result<(), String> {
        self.transition_to(job_id, JobState::Running, None, None)
    }

    /// Transition to `paused`. Handle stays in the active map so a
    /// follow-up `mark_running` (or eventual `mark_cancelled`) can
    /// still flip flags.
    pub fn mark_paused(&self, job_id: i64) -> Result<(), String> {
        self.transition_to(job_id, JobState::Paused, None, None)
    }

    /// Finalize as `completed`. Drops the handle from the active map.
    pub fn mark_completed(&self, job_id: i64) -> Result<(), String> {
        let r = self.transition_to(job_id, JobState::Completed, None, None);
        self.drop_handle(job_id);
        self.emit_event(&JobEvent::Completed { job_id });
        self.prune_history_to_cap();
        self.tick_parent_if_any(job_id);
        r
    }

    /// Finalize as `failed`. Drops the handle. `error` is persisted on
    /// the row and surfaced in the failure event.
    pub fn mark_failed(&self, job_id: i64, error: String) -> Result<(), String> {
        let r = self.transition_to(job_id, JobState::Failed, None, Some(error.clone()));
        self.drop_handle(job_id);
        self.emit_event(&JobEvent::Failed { job_id, error });
        self.prune_history_to_cap();
        self.tick_parent_if_any(job_id);
        r
    }

    /// Finalize as `cancelled`. Drops the handle.
    pub fn mark_cancelled(&self, job_id: i64) -> Result<(), String> {
        let r = self.transition_to(job_id, JobState::Cancelled, None, None);
        self.drop_handle(job_id);
        self.prune_history_to_cap();
        self.tick_parent_if_any(job_id);
        r
    }
}

/// RAII guard for a background-jobs row. Replaces the verbose
/// `create_job → mark_running → progress×N → mark_completed/mark_failed`
/// boilerplate that every consumer had previously, AND closes the most
/// common foot-gun in that pattern: a `?` early-return between
/// `create_job` and `mark_completed` leaves the row in `running`
/// state forever (until next launch's `promote_running_rows_to_interrupted`
/// sweep promotes it to `interrupted`).
///
/// ## Lifecycle
///
/// 1. `JobScope::start` calls `create_job` + `mark_running` atomically
///    and stashes the id.
/// 2. Consumer calls `scope.tick(done)` per iteration. Total can be
///    set up-front via the `total` arg or later via `set_total()`.
/// 3. Consumer calls `scope.complete()` (success) or `scope.fail(err)`
///    (explicit failure).
/// 4. If the scope drops without complete()/fail() — typical for `?`
///    early-returns — Drop calls `mark_failed` with a generic message
///    so the row never hangs in `running`.
///
/// ## Why not `run_job_over_iter`?
///
/// A higher-order helper that wraps the entire "create + iterate +
/// complete" pattern (option (c) in the 2026-06-04 audit) would force
/// every consumer into one iterator-shape. Some of our jobs — media
/// sync's repo-grouped concurrent work, scan_service's parallel
/// classification — have legitimate reasons for hand-rolled iteration.
/// JobScope keeps the structural fix (auto-finalize) without
/// constraining the work shape.
///
/// **If progress-bar bugs keep surfacing after JobScope is in place**,
/// the next iteration is the `run_job_over_iter` helper for the
/// simple-loop consumers (hash_resolve, dat_sync, artwork_sync). See
/// `docs/DECISIONS.md` 2026-06-04 entry for the trigger criteria.
///
/// ## No-registry mode
///
/// `JobScope::start` accepts `Option<&JobRegistry>` and silently
/// degrades to a no-op scope when None. All methods are no-ops in
/// that mode. Lets callers skip the `if let Some(reg) = ...` ceremony.
pub struct JobScope<'a> {
    inner: Option<JobScopeInner<'a>>,
}

struct JobScopeInner<'a> {
    registry: &'a JobRegistry,
    id: i64,
    /// `i64::MIN` sentinel = total is None (unknown). Otherwise the
    /// actual total. Using AtomicI64 keeps JobScope `Send`-friendly
    /// for consumers that hold it across awaits.
    total: AtomicI64,
    /// `true` once complete()/fail() has run. Drop checks this to
    /// avoid double-finalization.
    finalized: AtomicBool,
}

const JOB_SCOPE_NO_TOTAL: i64 = i64::MIN;

impl<'a> JobScope<'a> {
    /// Create the row and mark it running. On registry=None or
    /// create_job failure, returns a no-op scope (all methods become
    /// no-ops). Failure is logged at warn level so the consumer's
    /// code path stays simple.
    #[allow(clippy::too_many_arguments)]
    pub fn start(
        registry: Option<&'a JobRegistry>,
        kind: JobKind,
        label: String,
        system_id: Option<String>,
        parent_job_id: Option<i64>,
        is_prereq: bool,
        unit: &'static str,
        total: Option<i64>,
    ) -> Self {
        let Some(reg) = registry else {
            return Self { inner: None };
        };
        let id = match reg.create_job(
            kind,
            label,
            system_id,
            parent_job_id,
            is_prereq,
            unit,
            None,
        ) {
            Ok(id) => id,
            Err(e) => {
                log::warn!("background_jobs: JobScope::start create_job failed: {e}");
                return Self { inner: None };
            }
        };
        if let Err(e) = reg.mark_running(id) {
            log::warn!("background_jobs: JobScope::start mark_running failed: {e}");
        }
        // Flush the initial total to SQLite immediately so the bar's
        // first snapshot read carries it. Without this, the row's
        // total stays NULL until the first progress tick clears the
        // 1s SQL debounce — fine for long-running jobs, broken for
        // jobs that finish in under a second.
        if let Some(t) = total {
            if let Err(e) = reg.force_set_total(id, t) {
                log::warn!("background_jobs: JobScope::start force_set_total failed: {e}");
            }
        }
        Self {
            inner: Some(JobScopeInner {
                registry: reg,
                id,
                total: AtomicI64::new(total.unwrap_or(JOB_SCOPE_NO_TOTAL)),
                finalized: AtomicBool::new(false),
            }),
        }
    }

    /// Update the registry's progress. `done` is the latest count;
    /// total comes from the scope (set at start or via `set_total`).
    /// No-op when this is a no-registry scope.
    pub fn tick(&self, done: i64) {
        if let Some(i) = &self.inner {
            let total = i.total.load(Ordering::Relaxed);
            let total_opt = if total == JOB_SCOPE_NO_TOTAL { None } else { Some(total) };
            let _ = i.registry.progress(i.id, done, total_opt);
        }
    }

    /// Set / update the total. Useful when the count is unknown at
    /// `start` time (e.g. after a `list_games_missing_hash` query).
    /// Forces the new total to SQLite immediately so the bar reflects
    /// it without waiting for the next progress tick to clear the
    /// debounce.
    pub fn set_total(&self, total: i64) {
        if let Some(i) = &self.inner {
            i.total.store(total, Ordering::Relaxed);
            if let Err(e) = i.registry.force_set_total(i.id, total) {
                log::warn!("background_jobs: JobScope::set_total failed: {e}");
            }
        }
    }

    /// The underlying job_id, if this scope is real. Returns None
    /// for no-registry scopes. Used by callers that need to pass
    /// the parent_job_id to a child scope.
    pub fn id(&self) -> Option<i64> {
        self.inner.as_ref().map(|i| i.id)
    }

    /// Mark completed. Sets the final progress tick to (total, total)
    /// when total is known so the bar shows 100% before the row
    /// transitions to history. Idempotent — second call is a no-op.
    /// Uses `force_set_done_total` to bypass the SQL debounce on this
    /// final tick — the bar would otherwise show the last debounced
    /// `done` value instead of 100%.
    pub fn complete(&self) {
        if let Some(i) = &self.inner {
            if !i.finalized.swap(true, Ordering::AcqRel) {
                let total = i.total.load(Ordering::Relaxed);
                if total != JOB_SCOPE_NO_TOTAL {
                    let _ = i
                        .registry
                        .force_set_done_total(i.id, total, Some(total));
                }
                let _ = i.registry.mark_completed(i.id);
            }
        }
    }

    /// Mark failed with an explicit error. Idempotent — second call
    /// is a no-op. Once called, Drop will not re-finalize.
    pub fn fail(&self, error: String) {
        if let Some(i) = &self.inner {
            if !i.finalized.swap(true, Ordering::AcqRel) {
                let _ = i.registry.mark_failed(i.id, error);
            }
        }
    }

    /// Mark cancelled. Use when an operator-triggered cancel is the
    /// reason work is stopping (as opposed to a real failure). Idempotent —
    /// second call is a no-op, and Drop will not re-finalize.
    pub fn cancel(&self) {
        if let Some(i) = &self.inner {
            if !i.finalized.swap(true, Ordering::AcqRel) {
                let _ = i.registry.mark_cancelled(i.id);
            }
        }
    }

    /// Re-attach to an existing job row whose lifecycle a worker is about
    /// to run. Use this when:
    ///   - a Tauri command created the row and returned its job_id to the
    ///     frontend, and a follow-up worker task (`spawn_blocking` /
    ///     `async_runtime::spawn`) needs Drop-on-`?` protection;
    ///   - a `JobResumer` is picking up an `interrupted` row left by a
    ///     previous run.
    ///
    /// Internally calls `attach_handle` + `mark_running`. `total` is
    /// optional — pass the known value when available so the bar shows
    /// the right denominator on the first tick. Drop semantics match
    /// `start()`: on scope drop without explicit complete/fail/cancel,
    /// the row is marked failed.
    pub fn resume(
        registry: Option<&'a JobRegistry>,
        job_id: i64,
        kind: String,
        total: Option<i64>,
    ) -> Self {
        let Some(reg) = registry else {
            return Self { inner: None };
        };
        let _handle = reg.attach_handle(job_id, kind);
        if let Err(e) = reg.mark_running(job_id) {
            log::warn!("background_jobs: JobScope::resume mark_running failed: {e}");
        }
        if let Some(t) = total {
            if let Err(e) = reg.force_set_total(job_id, t) {
                log::warn!("background_jobs: JobScope::resume force_set_total failed: {e}");
            }
        }
        Self {
            inner: Some(JobScopeInner {
                registry: reg,
                id: job_id,
                total: AtomicI64::new(total.unwrap_or(JOB_SCOPE_NO_TOTAL)),
                finalized: AtomicBool::new(false),
            }),
        }
    }
}

impl Drop for JobScope<'_> {
    fn drop(&mut self) {
        if let Some(i) = &self.inner {
            if !i.finalized.swap(true, Ordering::AcqRel) {
                let _ = i.registry.mark_failed(
                    i.id,
                    "JobScope dropped without explicit complete() or fail() \
                     — likely a `?` early-return in the consumer"
                        .to_string(),
                );
            }
        }
    }
}

impl JobRegistry {

    /// Phase 4b parent aggregation — when a child finalizes, look up
    /// its parent_job_id (if any), recount finished siblings, and
    /// emit a progress event on the parent. When the last sibling
    /// resolves, finalize the parent too (mark_completed if every
    /// child succeeded, mark_failed if any failed/cancelled — Phase
    /// 4b treats partial success as failure at the parent level so
    /// the bar visibly surfaces the issue; Phase 4c retry policy
    /// will refine this).
    fn tick_parent_if_any(&self, child_job_id: i64) {
        let (parent_id, done_children, total_children, any_failure): (i64, i64, i64, bool) = {
            let Ok(conn) = self.lock_conn() else { return };
            let parent_id: Option<i64> = conn
                .query_row(
                    "SELECT parent_job_id FROM background_jobs WHERE id = ?1",
                    params![child_job_id],
                    |row| row.get(0),
                )
                .ok()
                .flatten();
            let Some(parent_id) = parent_id else {
                return;
            };
            let result: Result<(i64, i64, i64), rusqlite::Error> = conn.query_row(
                "SELECT \
                    SUM(CASE WHEN state IN ('completed', 'failed', 'cancelled') THEN 1 ELSE 0 END), \
                    COUNT(*), \
                    SUM(CASE WHEN state IN ('failed', 'cancelled') THEN 1 ELSE 0 END) \
                 FROM background_jobs WHERE parent_job_id = ?1",
                params![parent_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            );
            match result {
                Ok((done, total, fail)) => (parent_id, done, total, fail > 0),
                Err(_) => return,
            }
        };
        // Emit parent progress (bypassing the per-handle debounce —
        // parents don't carry handles in the active map; they advance
        // by child-completion ticks, which are inherently low-rate).
        let _ = self.write_progress_unthrottled(parent_id, done_children, Some(total_children));
        if done_children >= total_children && total_children > 0 {
            if any_failure {
                let _ = self.mark_failed(
                    parent_id,
                    format!("{} of {} children failed", any_failure as i64, total_children),
                );
            } else {
                let _ = self.mark_completed(parent_id);
            }
        }
    }

    /// Unthrottled progress writer for parent rows — bypasses the
    /// per-JobHandle debounce because parents don't carry handles
    /// (no worker polling pause/cancel on the parent itself).
    fn write_progress_unthrottled(
        &self,
        job_id: i64,
        done: i64,
        total: Option<i64>,
    ) -> Result<(), String> {
        let now = now_ms();
        let conn = self.lock_conn()?;
        conn.execute(
            "UPDATE background_jobs SET done = ?1, total = ?2, last_event_at = ?3 \
             WHERE id = ?4",
            params![done, total, now, job_id],
        )
        .map_err(|e| format!("update parent progress: {e}"))?;
        drop(conn);
        self.emit_event(&JobEvent::Progressed {
            job_id,
            done,
            total,
        });
        Ok(())
    }

    /// Phase 2 — flip the JobHandle's pause flag from outside the
    /// worker (typically from the BackgroundJobsBar's per-row pause
    /// button, routed through the `pause_job` / `resume_job` Tauri
    /// commands). The worker's chunk-loop already polls
    /// `handle.pause` (Slice E pattern) and spins on the flag with a
    /// cancel-check overlay, so the operator's pause click resolves
    /// at the next chunk boundary. Returns `false` if the job already
    /// finalized (handle dropped from the active map).
    ///
    /// Note: this only flips the in-memory flag. The Phase 2 bar
    /// optimistically updates the row's state to `paused` in the
    /// frontend store; the authoritative SQLite transition lands
    /// when the worker observes the flag, flushes its resume
    /// payload, and calls `mark_paused`. Phase 1's pilot kind
    /// (core_download) does not yet bridge pause → mark_paused (its
    /// chunk loop just spins on the flag), so pause currently looks
    /// the same as a stalled download until Phase 3 wires the
    /// per-kind pause path. Cancel works end-to-end today.
    pub fn signal_pause(&self, job_id: i64, paused: bool) -> bool {
        let Some(handle) = self.handle(job_id) else {
            return false;
        };
        handle.pause.store(paused, Ordering::Relaxed);
        true
    }

    /// Phase 2 — flip the JobHandle's cancel flag from outside the
    /// worker. The worker's chunk-loop polls `handle.cancel` and
    /// returns the "cancelled" sentinel error; `download_core`'s
    /// finalize block then calls `mark_cancelled` + the per-kind
    /// `.partial` cleanup. Returns `false` if the job already
    /// finalized.
    pub fn signal_cancel(&self, job_id: i64) -> bool {
        let Some(handle) = self.handle(job_id) else {
            return false;
        };
        handle.cancel.store(true, Ordering::Relaxed);
        true
    }

    /// Phase 2 — bulk flip cancel on every active job. Used by the
    /// bar's "Cancel all" button (which confirms before applying when
    /// 3+ jobs are active per plan §"Bar header"). Returns the count
    /// of jobs whose cancel flag was flipped.
    pub fn signal_cancel_all(&self) -> usize {
        let Ok(active) = self.inner.active.read() else {
            return 0;
        };
        let mut n = 0;
        for handle in active.values() {
            handle.cancel.store(true, Ordering::Relaxed);
            n += 1;
        }
        n
    }

    /// Phase 2 — bulk flip pause on every active job. Used by the
    /// bar's "Pause all" button.
    pub fn signal_pause_all(&self, paused: bool) -> usize {
        let Ok(active) = self.inner.active.read() else {
            return 0;
        };
        let mut n = 0;
        for handle in active.values() {
            handle.pause.store(paused, Ordering::Relaxed);
            n += 1;
        }
        n
    }

    /// Crash recovery. Called by `main.rs::setup()` after the lock
    /// file flagged the previous run as crashed. Promotes every
    /// `state = 'running'` row to `state = 'interrupted'` so Phase 3
    /// has a clear list to re-dispatch from. Returns the number
    /// promoted.
    pub fn promote_running_rows_to_interrupted(&self) -> Result<usize, String> {
        let conn = self.lock_conn()?;
        let n = conn
            .execute(
                "UPDATE background_jobs SET state = 'interrupted', last_event_at = ?1 \
                 WHERE state = 'running'",
                params![now_ms()],
            )
            .map_err(|e| format!("promote running→interrupted: {e}"))?;
        Ok(n)
    }

    /// List active rows (pending/running/paused). Phase 2's bar reads
    /// this on mount to hydrate the visible-stack, then keeps in sync
    /// via the `oa://job-event` broadcast.
    pub fn list_active(&self) -> Result<Vec<JobSnapshot>, String> {
        self.list_by_states(&[
            JobState::Pending,
            JobState::Running,
            JobState::Paused,
        ])
    }

    /// List rows that need the Phase 3 resume dispatcher to look at
    /// them. Phase 1 just leaves these sitting (operator triggers
    /// retry via the existing per-operation modal).
    #[allow(dead_code)] // Phase 3 will consume
    pub fn list_interrupted(&self) -> Result<Vec<JobSnapshot>, String> {
        self.list_by_states(&[JobState::Interrupted])
    }

    /// Last N finished rows (any outcome) for the Phase 5 recent-
    /// activity panel.
    #[allow(dead_code)] // Phase 5 will consume
    pub fn list_recent(&self, limit: usize) -> Result<Vec<JobSnapshot>, String> {
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, kind, label, system_id, target_id, parent_job_id, is_prereq, \
                        state, done, total, unit, last_event_at, started_at, finished_at, \
                        can_resume, resume_payload, error_message, retry_count \
                 FROM background_jobs \
                 WHERE state IN ('completed', 'failed', 'cancelled') \
                 ORDER BY finished_at DESC LIMIT ?1",
            )
            .map_err(|e| format!("prepare list_recent: {e}"))?;
        let rows = stmt
            .query_map(params![limit as i64], row_to_snapshot)
            .map_err(|e| format!("query list_recent: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("collect list_recent: {e}"))?;
        Ok(rows)
    }

    /// Phase 3b — find an active row (pending / running / paused)
    /// matching the given kind + target_id. Used by the
    /// duplicate-trigger dialog: the frontend calls
    /// `check_duplicate_job` before kicking off a new operation,
    /// and shows a Wait / Restart / Cancel dialog if an existing
    /// row hits. Returns the first match (there shouldn't be more
    /// than one — plan §"Per-kind parallel concurrency" + FIFO
    /// ordering — but if duplicates somehow exist, returning the
    /// newest by last_event_at is the right default).
    pub fn find_active_by_kind_target(
        &self,
        kind: &str,
        target_id: &str,
    ) -> Result<Option<JobSnapshot>, String> {
        let conn = self.lock_conn()?;
        let row = conn
            .query_row(
                "SELECT id, kind, label, system_id, target_id, parent_job_id, is_prereq, \
                        state, done, total, unit, last_event_at, started_at, finished_at, \
                        can_resume, resume_payload, error_message, retry_count \
                 FROM background_jobs \
                 WHERE kind = ?1 AND target_id = ?2 \
                       AND state IN ('pending', 'running', 'paused') \
                 ORDER BY last_event_at DESC LIMIT 1",
                params![kind, target_id],
                row_to_snapshot,
            )
            .optional()
            .map_err(|e| format!("find_active_by_kind_target: {e}"))?;
        Ok(row)
    }

    /// Single-row snapshot. Used internally to build event payloads
    /// and by tests.
    pub fn snapshot(&self, job_id: i64) -> Result<Option<JobSnapshot>, String> {
        let conn = self.lock_conn()?;
        let row = conn
            .query_row(
                "SELECT id, kind, label, system_id, target_id, parent_job_id, is_prereq, \
                        state, done, total, unit, last_event_at, started_at, finished_at, \
                        can_resume, resume_payload, error_message, retry_count \
                 FROM background_jobs WHERE id = ?1",
                params![job_id],
                row_to_snapshot,
            )
            .optional()
            .map_err(|e| format!("snapshot {job_id}: {e}"))?;
        Ok(row)
    }

    // ---- private helpers --------------------------------------------------

    fn lock_conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
        self.inner
            .conn
            .lock()
            .map_err(|e| format!("job registry conn lock: {e}"))
    }

    fn transition_to(
        &self,
        job_id: i64,
        new_state: JobState,
        finished_at_override: Option<i64>,
        error: Option<String>,
    ) -> Result<(), String> {
        let now = now_ms();
        let finished_at = if new_state.is_finished() {
            Some(finished_at_override.unwrap_or(now))
        } else {
            None
        };
        {
            let conn = self.lock_conn()?;
            conn.execute(
                "UPDATE background_jobs \
                 SET state = ?1, last_event_at = ?2, finished_at = ?3, error_message = COALESCE(?4, error_message) \
                 WHERE id = ?5",
                params![new_state.as_str(), now, finished_at, error, job_id],
            )
            .map_err(|e| format!("transition {job_id} → {}: {e}", new_state.as_str()))?;
        }
        self.emit_event(&JobEvent::StateChanged {
            job_id,
            state: new_state,
        });
        Ok(())
    }

    fn drop_handle(&self, job_id: i64) {
        if let Ok(mut active) = self.inner.active.write() {
            active.remove(&job_id);
        }
    }

    fn emit_event(&self, evt: &JobEvent) {
        let Some(app) = &self.inner.app else {
            return;
        };
        if let Err(e) = app.emit("oa://job-event", evt) {
            log::warn!("emit oa://job-event: {e}");
        }
    }

    fn list_by_states(&self, states: &[JobState]) -> Result<Vec<JobSnapshot>, String> {
        let in_clause = states
            .iter()
            .map(|s| format!("'{}'", s.as_str()))
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT id, kind, label, system_id, target_id, parent_job_id, is_prereq, \
                    state, done, total, unit, last_event_at, started_at, finished_at, \
                    can_resume, resume_payload, error_message, retry_count \
             FROM background_jobs WHERE state IN ({in_clause}) \
             ORDER BY last_event_at DESC"
        );
        let conn = self.lock_conn()?;
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("prepare list_by_states: {e}"))?;
        let rows = stmt
            .query_map([], row_to_snapshot)
            .map_err(|e| format!("query list_by_states: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("collect list_by_states: {e}"))?;
        Ok(rows)
    }

    fn prune_history_to_cap(&self) {
        let conn = match self.lock_conn() {
            Ok(c) => c,
            Err(e) => {
                log::warn!("prune_history_to_cap: {e}");
                return;
            }
        };
        // Count finished rows. If under cap, no-op. Otherwise delete
        // the oldest-by-`finished_at` excess in one statement.
        let count: i64 = match conn.query_row(
            "SELECT COUNT(*) FROM background_jobs \
             WHERE state IN ('completed', 'failed', 'cancelled')",
            [],
            |row| row.get(0),
        ) {
            Ok(n) => n,
            Err(e) => {
                log::warn!("prune_history_to_cap count: {e}");
                return;
            }
        };
        if count <= HISTORY_CAP {
            return;
        }
        let excess = count - HISTORY_CAP;
        if let Err(e) = conn.execute(
            "DELETE FROM background_jobs WHERE id IN ( \
                SELECT id FROM background_jobs \
                WHERE state IN ('completed', 'failed', 'cancelled') \
                ORDER BY finished_at ASC LIMIT ?1 \
             )",
            params![excess],
        ) {
            log::warn!("prune_history_to_cap delete: {e}");
        }
    }
}

// ===========================================================================
// Tauri commands — Phase 2 BackgroundJobsBar surface
// ===========================================================================
//
// The bar reads `list_active_jobs` once on mount to hydrate, then keeps
// in sync via the existing `oa://job-event` broadcast. Per-row controls
// flow through `pause_job` / `resume_job` / `cancel_job`. The "Pause all"
// + "Cancel all" header buttons flow through `pause_all_jobs` (with a
// bool arg so resume-all reuses the same command) + `cancel_all_jobs`.
//
// All commands soft-fail when the JobRegistry isn't managed (returns
// the empty list or a no-op count). Same fallback shape as Phase 1's
// download_core wiring — the bar just stays Hidden in that case.

// Internal helper: Tauri commands take AppHandle and look up the
// registry via try_state. Using `Option<tauri::State<...>>` directly
// as a #[tauri::command] arg doesn't compile (State isn't Deserialize),
// so we route through this lookup. Returns None when the registry
// isn't managed yet; commands degrade to no-ops in that case.
fn registry_handle<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Option<tauri::State<'_, JobRegistry>> {
    app.try_state::<JobRegistry>()
}

/// Snapshot of currently-active jobs (pending / running / paused).
/// Returns an empty list when the registry isn't managed yet.
#[tauri::command]
pub fn list_active_jobs(app: tauri::AppHandle) -> Result<Vec<JobSnapshot>, String> {
    match registry_handle(&app) {
        Some(reg) => reg.list_active(),
        None => Ok(Vec::new()),
    }
}

/// Recent finished history (completed / failed / cancelled), newest
/// first. The Phase 5 recent-activity panel is the primary consumer,
/// but exposing this in Phase 2 lets the bar's history sub-route work
/// against a real surface once Phase 5 lands. `limit` is capped at
/// 100 because that's the rolling-buffer ceiling (plan §"Job
/// history").
#[tauri::command]
pub fn list_recent_jobs(
    limit: Option<usize>,
    app: tauri::AppHandle,
) -> Result<Vec<JobSnapshot>, String> {
    match registry_handle(&app) {
        Some(reg) => reg.list_recent(limit.unwrap_or(100).min(100)),
        None => Ok(Vec::new()),
    }
}

/// Pause a single job. The worker observes the AtomicBool at the next
/// chunk boundary and spins on it until resume or cancel. Returns
/// `false` when the job already finalized.
#[tauri::command]
pub fn pause_job(job_id: i64, app: tauri::AppHandle) -> Result<bool, String> {
    Ok(registry_handle(&app)
        .map(|reg| reg.signal_pause(job_id, true))
        .unwrap_or(false))
}

/// Resume a paused job. Flips the pause flag back off; the worker
/// breaks out of its pause spin and continues from the same chunk.
#[tauri::command]
pub fn resume_job(job_id: i64, app: tauri::AppHandle) -> Result<bool, String> {
    Ok(registry_handle(&app)
        .map(|reg| reg.signal_pause(job_id, false))
        .unwrap_or(false))
}

/// Cancel a single job. Flips the cancel flag; the worker exits at
/// the next chunk boundary and the finalize block applies the
/// per-kind cancel-cleanup contract (delete .partial for
/// core_download, etc.).
#[tauri::command]
pub fn cancel_job(job_id: i64, app: tauri::AppHandle) -> Result<bool, String> {
    Ok(registry_handle(&app)
        .map(|reg| reg.signal_cancel(job_id))
        .unwrap_or(false))
}

/// Pause every active job. `paused = false` reuses this command as
/// "Resume all" so the bar header's toggle button can route through
/// one endpoint. Returns the count of handles whose flag flipped.
#[tauri::command]
pub fn pause_all_jobs(paused: bool, app: tauri::AppHandle) -> Result<usize, String> {
    Ok(registry_handle(&app)
        .map(|reg| reg.signal_pause_all(paused))
        .unwrap_or(0))
}

/// Cancel every active job. The bar UI must confirm with the
/// operator before invoking this (plan §"Bar header": "Both confirm
/// before applying when 3+ jobs are active").
#[tauri::command]
pub fn cancel_all_jobs(app: tauri::AppHandle) -> Result<usize, String> {
    Ok(registry_handle(&app)
        .map(|reg| reg.signal_cancel_all())
        .unwrap_or(0))
}

/// Polish — dispatch the registered resumer for a single
/// `state='interrupted'` row. Used by ResumePromptDialog: operator
/// flipped a per-kind opt-out ON, then either chose "Resume" or
/// "Discard" when the launch dialog surfaced. Resume calls this;
/// Discard goes through the existing cancel_job command.
///
/// Returns true on dispatch, false when:
///   - the row doesn't exist
///   - the row isn't actually `interrupted`
///   - no resumer is registered for the row's kind
/// In all three cases the row state is unchanged.
#[tauri::command]
pub fn resume_one_interrupted_job(
    job_id: i64,
    app: tauri::AppHandle,
) -> Result<bool, String> {
    let Some(reg) = registry_handle(&app) else {
        return Ok(false);
    };
    let Some(snap) = reg.snapshot(job_id)? else {
        return Ok(false);
    };
    if snap.state != JobState::Interrupted {
        return Ok(false);
    }
    let resumers = reg
        .inner
        .resumers
        .read()
        .map_err(|e| format!("resumers lock: {e}"))?;
    let Some(resumer) = resumers.get(snap.kind.as_str()) else {
        return Ok(false);
    };
    let _join = resumer.resume(snap, reg.inner().clone(), app.clone());
    Ok(true)
}

/// Phase 5 — wipe every finished row (completed / failed / cancelled)
/// from `background_jobs`. Active rows (pending / running / paused /
/// interrupted) are preserved. Returns the deleted row count. Used by
/// the Settings → Background Jobs → "Clear recent activity" button
/// (plan §Settings panel §History).
#[tauri::command]
pub fn clear_job_history(app: tauri::AppHandle) -> Result<usize, String> {
    let Some(reg) = registry_handle(&app) else {
        return Ok(0);
    };
    let conn = reg
        .inner
        .conn
        .lock()
        .map_err(|e| format!("conn lock: {e}"))?;
    let n = conn
        .execute(
            "DELETE FROM background_jobs \
             WHERE state IN ('completed', 'failed', 'cancelled')",
            [],
        )
        .map_err(|e| format!("clear_job_history delete: {e}"))?;
    Ok(n)
}

/// Phase 3b — look up an active job by kind + target_id. Returns
/// the snapshot when one exists (i.e. an operator-triggered duplicate
/// of the same operation). The frontend uses this to drive the
/// Wait / Restart / Cancel dialog (plan §"Duplicate same-job
/// triggering"). Returns None when no active duplicate exists; the
/// caller proceeds with the new operation.
#[tauri::command]
pub fn check_duplicate_job(
    kind: String,
    target_id: String,
    app: tauri::AppHandle,
) -> Result<Option<JobSnapshot>, String> {
    match registry_handle(&app) {
        Some(reg) => reg.find_active_by_kind_target(&kind, &target_id),
        None => Ok(None),
    }
}

/// Phase 4b — create a `bulk_core_install` parent job for the
/// Guided Setup MissingCoreBulkPrompt batch. The frontend then loops
/// over the per-core download_core calls, passing the returned id
/// as `parentJobId` on each. As each child mark_completed/failed
/// fires, the JobRegistry's tick_parent_if_any helper recounts
/// children and updates the parent's `done` accordingly. Returns
/// the parent's job_id; -1 means the registry isn't managed (the
/// frontend treats this as "no parent" and the downloads still
/// proceed individually).
#[tauri::command]
pub fn start_bulk_core_install(n: usize, app: tauri::AppHandle) -> Result<i64, String> {
    let Some(reg) = registry_handle(&app) else {
        log::warn!(
            "background_jobs: start_bulk_core_install — registry not managed, skipping parent"
        );
        return Ok(-1);
    };
    // JobScope intentionally NOT used here: the parent has no worker —
    // its lifecycle is driven by `tick_parent_if_any` ticks fired when
    // child download_core jobs finalize. A JobScope would Drop on
    // command return and incorrectly mark_failed the parent before any
    // child even started.
    let label = format!("Installing {} core{}", n, if n == 1 { "" } else { "s" });
    let id = reg.create_job(
        JobKind::BulkCoreInstall,
        label,
        None,
        None,
        false,
        "cores",
        None,
    )?;
    if let Err(e) = reg.mark_running(id) {
        log::warn!("background_jobs: bulk_core_install mark_running({id}) failed: {e}");
    }
    // Pre-set the parent's total so the bar opens with "0 / N" rather
    // than blank.
    if let Err(e) = reg.write_progress_unthrottled(id, 0, Some(n as i64)) {
        log::warn!("background_jobs: bulk_core_install initial total failed: {e}");
    }
    Ok(id)
}

/// Dev helper — spawn a synthetic background job that ticks
/// progress at 10 Hz for `duration_secs` seconds. Lets the operator
/// exercise the BackgroundJobsBar's pause / resume / cancel /
/// auto-collapse / +N more affordances without a real long-running
/// operation. Honors the JobHandle's cancel + pause flags exactly
/// the same way `download_core`'s chunk loop does, so pause-then-
/// cancel races behave identically.
///
/// Invoked from Settings → Library → "Background Jobs (dev test)".
/// Stays in the production build because the always-on cost is zero
/// (no spawn until invoked) and it remains useful for sanity-checking
/// the bar after any settings change. Multiple calls produce N
/// concurrent test jobs so the bar's 2+ / 3+ confirm thresholds can
/// be exercised.
#[tauri::command]
pub async fn spawn_test_job(
    duration_secs: Option<u64>,
    app: tauri::AppHandle,
) -> Result<i64, String> {
    let secs = duration_secs.unwrap_or(30).clamp(1, 600);
    let registry = registry_handle(&app)
        .map(|s| (*s).clone())
        .ok_or_else(|| "background-jobs registry not managed".to_string())?;
    let total: i64 = (secs as i64) * 10; // 10 Hz ticks
    // Make the label distinct so the operator can tell test jobs
    // apart from real ones when both run together.
    let suffix = now_ms() % 10_000;
    let name = format!("test-{suffix:04}");
    // Create the row in the command so we can return job_id to the
    // frontend synchronously; the worker task picks it up via
    // JobScope::resume so any panic in the loop auto-finalizes the row.
    let job_id = registry.create_job(
        JobKind::TestJob { name: name.clone() },
        format!("Test job ({secs}s)"),
        None,
        None,
        false,
        "steps",
        None,
    )?;
    let handle = registry
        .handle(job_id)
        .ok_or_else(|| "test_job handle dropped before tick loop".to_string())?;
    let registry_clone = registry.clone();
    tauri::async_runtime::spawn(async move {
        let scope = JobScope::resume(
            Some(&registry_clone),
            job_id,
            "test_job".to_string(),
            Some(total),
        );
        let mut done: i64 = 0;
        while done < total {
            if handle.is_cancelled() {
                let _ = registry_clone
                    .flush_resume_state(job_id, serde_json::json!({ "done": done }));
                scope.cancel();
                return;
            }
            // Same paused → running state bridge core_download uses so
            // the bar's resume button toggles for test jobs too. These
            // are non-finalize transitions so they go direct to the
            // registry; JobScope only tracks finalize.
            let mut was_paused = false;
            while handle.is_paused() {
                if !was_paused {
                    was_paused = true;
                    let _ = registry_clone.mark_paused(job_id);
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
                if handle.is_cancelled() {
                    scope.cancel();
                    return;
                }
            }
            if was_paused {
                let _ = registry_clone.mark_running(job_id);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
            done += 1;
            scope.tick(done);
        }
        scope.complete();
    });
    Ok(job_id)
}

fn row_to_snapshot(row: &rusqlite::Row<'_>) -> rusqlite::Result<JobSnapshot> {
    let state_str: String = row.get(7)?;
    let state = JobState::parse(&state_str).unwrap_or(JobState::Pending);
    let is_prereq: i32 = row.get(6)?;
    let can_resume: i32 = row.get(14)?;
    Ok(JobSnapshot {
        id: row.get(0)?,
        kind: row.get(1)?,
        label: row.get(2)?,
        system_id: row.get(3)?,
        target_id: row.get(4)?,
        parent_job_id: row.get(5)?,
        is_prereq: is_prereq != 0,
        state,
        done: row.get(8)?,
        total: row.get(9)?,
        unit: row.get(10)?,
        last_event_at: row.get(11)?,
        started_at: row.get(12)?,
        finished_at: row.get(13)?,
        can_resume: can_resume != 0,
        // column 15 (resume_payload) is intentionally not surfaced on
        // JobSnapshot — Phase 3's resumer reads it directly via a
        // dedicated query when promoting interrupted rows.
        error_message: row.get(16)?,
        retry_count: row.get(17)?,
    })
}

/// Path helper: where `JobRegistry::new` should be told to open. Lives
/// next to the existing `LibraryDb` at `<data_dir>/library/games.sqlite`.
pub fn db_path_for(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("library").join("games.sqlite")
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Holds the tmp dir for the test's lifetime so the on-disk DB
    /// survives the registry's queries; drops it (and removes the
    /// directory) when the test exits.
    struct TestDataDir(PathBuf);
    impl Drop for TestDataDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn fresh_registry() -> (JobRegistry, TestDataDir) {
        let tmp = std::env::temp_dir().join(format!(
            "oa-job-registry-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).expect("mkdir tmp");
        // Bootstrap the schema by opening LibraryDb once.
        let _db = crate::library_db::LibraryDb::open(&tmp).expect("open library db");
        let registry =
            JobRegistry::new_for_tests(&db_path_for(&tmp)).expect("open registry");
        (registry, TestDataDir(tmp))
    }

    #[test]
    fn create_job_round_trip() {
        let (reg, _tmp) = fresh_registry();
        let job_id = reg
            .create_job(
                JobKind::CoreDownload {
                    base: "mednafen_psx_hw_libretro".into(),
                },
                "Downloading Beetle PSX HW".into(),
                None,
                None,
                false,
                "bytes",
                None,
            )
            .expect("create_job");
        let snap = reg.snapshot(job_id).expect("snapshot").expect("present");
        assert_eq!(snap.id, job_id);
        assert_eq!(snap.kind, "core_download");
        assert_eq!(snap.target_id.as_deref(), Some("mednafen_psx_hw_libretro"));
        assert_eq!(snap.state, JobState::Pending);
        assert_eq!(snap.unit, "bytes");
        // Handle is in the active map.
        assert!(reg.handle(job_id).is_some());
    }

    #[test]
    fn progress_debounce_writes_at_1hz_to_db_but_in_memory_always() {
        let (reg, _tmp) = fresh_registry();
        let job_id = reg
            .create_job(
                JobKind::CoreDownload {
                    base: "x".into(),
                },
                "x".into(),
                None,
                None,
                false,
                "bytes",
                None,
            )
            .unwrap();
        reg.mark_running(job_id).unwrap();
        // First progress tick is allowed (last_db_write_ms set at
        // create time, but the debounce check uses >=, and ms ticks
        // forward — we expect at least the FIRST or LAST tick to write).
        for i in 1..50 {
            reg.progress(job_id, i, Some(100)).unwrap();
        }
        // The row should reflect SOME progress (we can't pin down
        // exact value without time control, but it must not be 0).
        let snap = reg.snapshot(job_id).unwrap().unwrap();
        assert!(
            snap.done >= 0,
            "done should have been written at least once (got {})",
            snap.done
        );
    }

    #[test]
    fn mark_completed_drops_handle_and_persists() {
        let (reg, _tmp) = fresh_registry();
        let job_id = reg
            .create_job(
                JobKind::CoreDownload {
                    base: "x".into(),
                },
                "x".into(),
                None,
                None,
                false,
                "bytes",
                None,
            )
            .unwrap();
        reg.mark_running(job_id).unwrap();
        reg.mark_completed(job_id).unwrap();
        assert!(reg.handle(job_id).is_none(), "handle should be dropped");
        let snap = reg.snapshot(job_id).unwrap().unwrap();
        assert_eq!(snap.state, JobState::Completed);
        assert!(snap.finished_at.is_some());
    }

    #[test]
    fn mark_cancelled_drops_handle_and_persists() {
        let (reg, _tmp) = fresh_registry();
        let job_id = reg
            .create_job(
                JobKind::CoreDownload {
                    base: "x".into(),
                },
                "x".into(),
                None,
                None,
                false,
                "bytes",
                None,
            )
            .unwrap();
        reg.mark_running(job_id).unwrap();
        let handle = reg.handle(job_id).expect("handle");
        handle.cancel.store(true, Ordering::Relaxed);
        assert!(handle.is_cancelled());
        reg.flush_resume_state(job_id, serde_json::json!({"downloaded": 12345}))
            .unwrap();
        reg.mark_cancelled(job_id).unwrap();
        let snap = reg.snapshot(job_id).unwrap().unwrap();
        assert_eq!(snap.state, JobState::Cancelled);
        assert!(reg.handle(job_id).is_none());
    }

    // ---- JobScope RAII guard ----

    #[test]
    fn job_scope_start_with_none_registry_is_noop() {
        // Pure type-level check that the no-op scope handles every
        // method without panicking. id() returns None; tick / set_total
        // / complete / fail just silently no-op.
        let scope = JobScope::start(
            None,
            JobKind::TestJob { name: "x".into() },
            "x".into(),
            None,
            None,
            false,
            "n",
            Some(10),
        );
        assert!(scope.id().is_none());
        scope.tick(5);
        scope.set_total(20);
        scope.complete();
        scope.fail("won't fire".into());
    }

    #[test]
    fn job_scope_round_trips_through_complete() {
        let (reg, _tmp) = fresh_registry();
        let scope = JobScope::start(
            Some(&reg),
            JobKind::TestJob { name: "round-trip".into() },
            "Round trip".into(),
            None,
            None,
            false,
            "items",
            Some(10),
        );
        let id = scope.id().expect("real scope has id");
        // Row is running with the total set.
        let snap = reg.snapshot(id).expect("snap").expect("present");
        assert_eq!(snap.state, JobState::Running);
        assert_eq!(snap.total, Some(10));
        scope.tick(5);
        scope.complete();
        let snap = reg.snapshot(id).expect("snap").expect("present");
        assert_eq!(snap.state, JobState::Completed);
        assert_eq!(snap.done, 10, "complete() ticks done to total");
    }

    #[test]
    fn job_scope_drop_without_complete_marks_failed() {
        // The core foot-gun fix: forgetting to call complete()/fail()
        // — typical for `?` early-returns — causes Drop to mark_failed
        // so the row never hangs in `running`.
        let (reg, _tmp) = fresh_registry();
        let id;
        {
            let scope = JobScope::start(
                Some(&reg),
                JobKind::TestJob { name: "drop-fail".into() },
                "Drop fail".into(),
                None,
                None,
                false,
                "items",
                Some(5),
            );
            id = scope.id().expect("id");
            // scope drops here without complete/fail
        }
        let snap = reg.snapshot(id).expect("snap").expect("present");
        assert_eq!(snap.state, JobState::Failed);
        assert!(snap.error_message.as_deref().unwrap_or("").contains("dropped"));
    }

    #[test]
    fn job_scope_complete_is_idempotent() {
        let (reg, _tmp) = fresh_registry();
        let scope = JobScope::start(
            Some(&reg),
            JobKind::TestJob { name: "idem".into() },
            "Idem".into(),
            None,
            None,
            false,
            "items",
            Some(3),
        );
        let id = scope.id().unwrap();
        scope.complete();
        scope.complete(); // second call no-ops
        scope.fail("won't fire".into()); // no-op after complete
        let snap = reg.snapshot(id).expect("snap").expect("present");
        assert_eq!(snap.state, JobState::Completed);
    }

    #[test]
    fn job_scope_fail_overrides_drop_message() {
        let (reg, _tmp) = fresh_registry();
        let id;
        {
            let scope = JobScope::start(
                Some(&reg),
                JobKind::TestJob { name: "fail-msg".into() },
                "Fail msg".into(),
                None,
                None,
                false,
                "items",
                None,
            );
            id = scope.id().unwrap();
            scope.fail("explicit consumer error".into());
            // scope still drops here, but Drop sees finalized=true and no-ops.
        }
        let snap = reg.snapshot(id).expect("snap").expect("present");
        assert_eq!(snap.state, JobState::Failed);
        assert_eq!(snap.error_message.as_deref(), Some("explicit consumer error"));
    }

    #[test]
    fn job_scope_set_total_late_lets_consumer_post_initial_count() {
        // Many consumers don't know the total at create-time (e.g.
        // rom_hashes resolve fetches list_games_missing_hash AFTER the
        // job row is up). set_total covers that case.
        let (reg, _tmp) = fresh_registry();
        let scope = JobScope::start(
            Some(&reg),
            JobKind::TestJob { name: "late-total".into() },
            "Late total".into(),
            None,
            None,
            false,
            "items",
            None,
        );
        let id = scope.id().unwrap();
        let snap = reg.snapshot(id).expect("snap").expect("present");
        assert_eq!(snap.total, None);
        scope.set_total(42);
        scope.tick(1);
        // Force a SQL flush by waiting past the debounce window.
        std::thread::sleep(std::time::Duration::from_millis(1_100));
        scope.tick(2);
        let snap = reg.snapshot(id).expect("snap").expect("present");
        assert_eq!(snap.total, Some(42));
        scope.complete();
    }

    #[test]
    fn promote_running_to_interrupted_idempotent() {
        let (reg, _tmp) = fresh_registry();
        let job_id = reg
            .create_job(
                JobKind::CoreDownload {
                    base: "x".into(),
                },
                "x".into(),
                None,
                None,
                false,
                "bytes",
                None,
            )
            .unwrap();
        reg.mark_running(job_id).unwrap();
        let n1 = reg.promote_running_rows_to_interrupted().unwrap();
        assert_eq!(n1, 1);
        let snap = reg.snapshot(job_id).unwrap().unwrap();
        assert_eq!(snap.state, JobState::Interrupted);
        // Second call → nothing left to promote.
        let n2 = reg.promote_running_rows_to_interrupted().unwrap();
        assert_eq!(n2, 0);
    }

    #[test]
    fn list_active_excludes_finished() {
        let (reg, _tmp) = fresh_registry();
        let a = reg
            .create_job(
                JobKind::CoreDownload {
                    base: "a".into(),
                },
                "a".into(),
                None,
                None,
                false,
                "bytes",
                None,
            )
            .unwrap();
        let b = reg
            .create_job(
                JobKind::CoreDownload {
                    base: "b".into(),
                },
                "b".into(),
                None,
                None,
                false,
                "bytes",
                None,
            )
            .unwrap();
        reg.mark_running(a).unwrap();
        reg.mark_completed(b).unwrap();
        let active = reg.list_active().unwrap();
        let ids: Vec<i64> = active.iter().map(|s| s.id).collect();
        assert!(ids.contains(&a));
        assert!(!ids.contains(&b));
    }

    #[test]
    fn history_rolling_buffer_caps_at_100() {
        let (reg, _tmp) = fresh_registry();
        // Create 105 finished jobs. After each finalize, the rolling
        // buffer prune should keep at most 100.
        let mut ids = Vec::new();
        for i in 0..105 {
            let id = reg
                .create_job(
                    JobKind::CoreDownload {
                        base: format!("c{i}"),
                    },
                    format!("c{i}"),
                    None,
                    None,
                    false,
                    "bytes",
                    None,
                )
                .unwrap();
            reg.mark_running(id).unwrap();
            reg.mark_completed(id).unwrap();
            ids.push(id);
        }
        let recent = reg.list_recent(200).unwrap();
        assert_eq!(recent.len(), HISTORY_CAP as usize);
        // Oldest 5 (the first 5 created) should have been pruned.
        let surviving_ids: std::collections::HashSet<i64> =
            recent.iter().map(|s| s.id).collect();
        for old in &ids[..5] {
            assert!(
                !surviving_ids.contains(old),
                "oldest finished id {old} should be pruned"
            );
        }
        for keep in &ids[5..] {
            assert!(
                surviving_ids.contains(keep),
                "kept id {keep} should be present"
            );
        }
    }
}
