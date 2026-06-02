# Background jobs + persistent progress bar

**Status:** Planning locked 2026-06-02 (6 rounds of operator Q&A; all design questions answered). Execution deferred to a future session.

**Owner-of-decisions:** the operator. This document records the
decisions that came out of the refinement Q&A. Implementation
should follow them unless a code-time issue forces a revisit (in
which case: check back in here first).

---

## Why this matters

OA already runs a half-dozen long-running operations: core
downloads, libretro-database dat sync, ROM hash resolution, media
sync (thumbnails / metadata), MAME ROM-set extraction, the
upcoming per-track SHA-1 matching for disc-shape systems
([docs/PLANS/disc-track-sha1-matching.md](disc-track-sha1-matching.md)),
folder scanning, the wizard's scan + ingest pass. Each of these
operations does its OWN thing for progress: some emit toast
notifications, some have inline progress bars in their dialogs,
some print to the debug log only. The result:

- Operator starts a core download → toast pops, fills, dismisses
- Operator clicks Identify ROMs → progress modal opens, runs, closes
- Media sync runs in background → no visible UI; only the debug log knows
- App closes mid-operation → ALL state lost; restart picks up nothing

That's three problems wrapped together:

1. **No single surface.** Operations announce themselves
   independently. The operator can't see "what is OA doing right
   now?" at a glance.
2. **Fake progress in some places.** Some operations report
   "Processing..." or fake percentages because the operation
   doesn't actually know its total cost up front. Operator hates
   this; says it explicitly.
3. **No persistence.** Close the app mid-download, restart, and
   the download is gone (or worse, leaves a `.partial` file the
   operator has to clean up manually). Same for half-completed
   scans, half-resolved hash passes, etc.

**The operator's pitch:** "A real progress bar at the bottom of
the UI that says exactly what OA is doing, with real numbers, and
that remembers what it was doing when I close the app — so when I
relaunch it picks up where it left off."

---

## Locked design decisions (from 6 rounds of refinement, 2026-06-02)

### Concurrency + scheduling

- **Per-kind parallel concurrency.** One job per kind can run at
  any time; multiple kinds can run concurrently (a network
  download + a CPU-bound hash resolve + a folder scan all happen
  at once). Same-kind jobs queue.
- **FIFO ordering within a kind.** No per-system priority, no
  promotion (operator considered + rejected per-system priority in
  Round 4). Whoever clicked first goes first.
- **Duplicate same-job triggering:** second click while a
  same-kind same-target job is running opens a
  `Wait / Restart / Cancel` dialog:
    - **Wait** — queue the second instance behind the first
    - **Restart** — cancel current + start a fresh one
    - **Cancel** — close the dialog, do nothing

### Visibility scope

- **Everything OA does in the background appears in the bar.**
  Operator-initiated work (clicked a button) AND auto-triggered
  work (post-scan media sync that fires implicitly) both show.
  Maximal "I can see exactly what OA is doing."

### Pause + cancel

- **Pause semantics: cancel-and-remember.** Pause and "app closed
  mid-job" are the same code path: flush state to SQLite,
  terminate the worker cleanly, free resources. Resume re-enters
  from the persisted state (HTTP Range request, scan-from-stamp,
  per-track-cache lookup, etc.). One mechanism handles both pause
  and restart.
- **Cancel cleanup: per-kind contract.** Each kind defines what
  "clean" means for cancel:
    - `core_download` cancel = delete `.partial` file
    - `folder_scan` cancel = discard partial rows
    - `artwork_sync` / `metadata_sync` cancel = keep
      already-downloaded items (idempotent, useful next run)
    - `hash_resolve` cancel = keep already-stamped rows
    - etc.

### Job dependencies

- **Explicit dependency graph.** Jobs auto-trigger their
  prerequisites. Click "Identify PSX hashes" → if the dat table is
  empty for psx, the system enqueues "Sync PSX dat" first as a
  prereq, then "Identify PSX hashes" runs once the dat lands.
  Schema models the chain via a `parent_job_id` + `prereq_of` link.
- **Dependency cancellation: prompt the operator.** Cancel a job
  that has in-flight prereqs → dialog: "Also cancel its
  prereqs?" with `[Just this one] / [The whole chain]`. Operator
  decides per cancel — never waste data the operator wanted; never
  leave orphans the operator didn't expect.

### Resume on app launch

- **Auto-resume everything by default.** Interrupted jobs silently
  resume on app launch. No prompt dialog at launch.
- **Per-kind opt-out in Settings.** The "Download Settings" panel
  (see §"Settings panel" below) exposes a per-kind toggle: "Prompt
  before resuming on launch." All checkboxes unchecked by default.
  Operator who wants their hash-resolve work to ask first checks
  that one box.

### Crash detection

- **Lock file + heartbeat (belt + braces).**
  - Lock file: OA writes `<data_dir>/oa.lock` at startup; deletes
    on clean shutdown. Lock file present at launch = previous run
    crashed = mark any `state = 'running'` rows as
    `state = 'interrupted'`.
  - Heartbeat: app updates `background_jobs.last_event_at` every
    ~1s while jobs are running. Catches the case where the app is
    alive but a worker thread died (panicked) — stale
    `last_event_at` while the lock file is still held → row's
    worker is gone, mark interrupted + re-queue for resume.

### Failure handling

- **Auto-retry transient network errors with exponential backoff.**
  Network timeouts, 5xx HTTP, connection drops: retry 3 times with
  backoff (1s, 5s, 30s). Each retry is invisible to the operator
  (logged at warn).
- **Persistent failures surface in the bar.** HTTP 404, file-not-
  found, parse errors, hash mismatch: don't retry; mark the job
  `state = 'failed'`; bar shows the failed row with
  `[Retry] [Discard]` buttons.
- **Retry from failed state re-runs the job from the last persisted
  checkpoint.** Same code path as the auto-resume path.

### Job history

- **Keep the last 100 jobs across all kinds.** Rolling buffer.
  When the 101st completes, the oldest finished row gets DELETE'd.
- **Recent activity panel** — bar exposes a link to a panel that
  shows the last-100 view. **Tabbed by outcome:** `Running /
  Completed / Failed / Cancelled`. Each row: timestamp, kind icon,
  label, duration, outcome glyph (✓ / ✗ / cancelled-X / paused).

### Kind taxonomy (9 kinds)

| Kind | What it is | Per-row label shape | Resumable | Default auto-resume |
| --- | --- | --- | --- | --- |
| `core_download` | Single libretro .dll download from buildbot | "Downloading {core display_name}" | yes (HTTP Range + .partial) | auto |
| `bulk_core_install` | Guided Setup's parallel install of N cores | "Installing {n} cores" + children | parent yes; children individually resume | auto |
| `dat_sync` | Fetch + parse a system's libretro-database dat | "Updating ROM database — {system}" | atomic retry | auto |
| `hash_resolve` | Per-system cart ROM hash + lookup | "Identifying {system} ROMs" | yes (per-game cache) | auto |
| `disc_track_hash` | Per-disc per-track SHA-1 + lookup (future arc) | "Identifying {system} discs" | yes (per-track cache) | auto |
| `artwork_sync` | Cover / box / screenshot / banner / fanart / wheel art per system | "Syncing {system} artwork" | yes (per-art cache) | auto |
| `metadata_sync` | Year + publisher + genre + descriptions per system | "Syncing {system} metadata" | yes (per-game cache) | auto |
| `folder_scan` | Wizard's folder walk + classification | "Scanning {folder}" | partial (re-walks from start; cheap) | auto |
| `mame_listxml_import` | Refresh MAME catalog from local MAME install | "Refreshing MAME catalog" | atomic retry | auto |

Note on the artwork+metadata split: Round 3 picked "split into
artwork vs metadata" rather than the bundled `media_sync` kind.
Operator value: per-kind concurrency means an artwork sync for
PSX + a metadata sync for the same PSX can run in parallel
(different resource paths — artwork is downloads, metadata is
local parsing of imported files for many sources).

`thumbnail_repo_sync` (libretro-thumbnails GitHub repo sync) folds
into `artwork_sync` — it's one of several artwork sources, not its
own kind.

### Notification on completion

- **Subtle chime + bar slides out.** No completion toast. The
  audio cue carries "something finished, check the bar if you
  care"; the bar visual reflects the new state. Operator who
  multitasked while OA worked in the background hears the chime.
- **Per-kind chime variation:** all kinds use the same chime in
  v1. Per-kind / per-system chime variants are a
  PARKING_LOT-worthy polish item for later (mostly because the
  audio bus already supports per-system theming if we want it).

### Bar UI shape

- **Auto-hide with persistent handle when work is active.**
    - Idle (no jobs): nothing visible.
    - Active jobs present, bar collapsed: a slim handle
      (~12-16px) sits at the bottom of the window, always
      clickable to expand. The handle pulses subtly when a job
      progresses (so the operator's eye catches "still working").
    - Active jobs present, bar expanded: the stack-visible bar
      slides up from the bottom; auto-hide reactivates after 2s
      of operator-input idle.
- **Stack visible: max 3 visible rows + "+N more"** below.
  Each running job gets its own thin row. Most-recently-started
  jobs at the top (so a freshly-clicked operation appears right
  away). 4+ concurrent jobs (rare) get truncated to 3 visible +
  a "+N more" affordance that expands the full list.
- **Bar header (visible when 2+ jobs run):** `Pause all` +
  `Cancel all` buttons. Both confirm before applying when 3+ jobs
  are active. Single-job and zero-job states hide these.
- **Per-row controls:** label (kind icon + system + summary) /
  progress bar / done-of-total numbers / per-row pause + cancel
  buttons.
- **Bar theming: neutral always.** OA accent (warm orange from the
  Retroverse chrome) regardless of which system's jobs are
  running. No per-system colors on the bar — keeps it
  predictable and matches the rest of the Retroverse chrome.

### Bar placement in the Retroverse layout

- Anchored at the bottom of the main content area, above the
  existing HintBar. When the bar is expanded, it slides into the
  content area (pushing content up briefly). When collapsed, the
  bottom-of-screen handle sits in HintBar-adjacent space without
  competing.
- **HintBar takes priority** when both want to show (e.g. operator
  is mid-modal with a hint context). The bar can still be expanded
  via the handle.

### Existing per-operation UI — hybrid coexistence

The architectural rule: **the job is the model; the modal is one
optional view of it; the bar is the persistent view.** Operator can
close the modal without losing the job; reopening reattaches.

- **Toasts (per-operation completion notifications) RETIRE.**
  The bar + completion chime carry the role today's
  `✓ Downloaded Beetle PSX HW` toast does. Less visual noise.
- **Modals KEEP their inline progress.** Identify ROMs modal,
  the Import Wizard's scan-review step, the bulk core install
  prompt all keep their dedicated UX. They register a job in the
  bar AND show the same progress inline.
- **Closing a modal mid-job does NOT cancel the job.** The job
  continues in the background; the bar surfaces it.
  Reopening the modal reattaches and shows the live state.

### Settings panel

- **Top-level Settings category: "Download Settings."** Sits in
  the Retroverse SETTINGS sidebar alongside Display / Audio /
  Library / Cores / etc.
- **Panel contents:**
    - **Per-kind auto-resume on launch** — 9 toggles
      (one per kind). All unchecked by default = auto-resume
      everything. Checked = prompt on launch for that kind.
    - **Bar behavior:**
        - "Always show the bar" toggle (default OFF; ON = bar
          handle never auto-hides even when idle).
        - "Sound on completion" toggle (default ON).
    - **Failure handling:**
        - "Auto-retry transient network errors" toggle
          (default ON).
        - Retry attempts (default 3; range 0-10).
    - **History:**
        - Read-only counter: "X of 100 history rows used."
        - "Clear recent activity" button.

---

## Schema

New SQLite table:

```sql
CREATE TABLE background_jobs (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    kind            TEXT NOT NULL,            -- "core_download", "scan", "artwork_sync", ...
    label           TEXT NOT NULL,            -- operator-facing string
    system_id       TEXT,                     -- nullable: jobs scoped to a single system
    target_id       TEXT,                     -- nullable: kind-specific (core base name, game rom_id, etc.)
    parent_job_id   INTEGER,                  -- nullable: for bulk children or dep chains; FK to id
    is_prereq       INTEGER NOT NULL DEFAULT 0, -- bool: auto-triggered prereq vs operator-initiated
    state           TEXT NOT NULL,            -- pending | running | paused | completed | failed | cancelled | interrupted
    done            INTEGER NOT NULL DEFAULT 0,
    total           INTEGER,                  -- nullable: some kinds don't know up front
    unit            TEXT NOT NULL,            -- "bytes" | "files" | "games" | "tracks" | "cores"
    last_event_at   INTEGER NOT NULL,         -- unix ms; for heartbeat / stale detection
    started_at      INTEGER NOT NULL,
    finished_at     INTEGER,
    can_resume      INTEGER NOT NULL DEFAULT 1,
    resume_payload  TEXT,                     -- JSON: kind-specific resume state
    error_message   TEXT,                     -- nullable: last error
    retry_count     INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_background_jobs_active ON background_jobs (state, last_event_at);
CREATE INDEX idx_background_jobs_history ON background_jobs (state, finished_at);
CREATE INDEX idx_background_jobs_parent ON background_jobs (parent_job_id);
```

`state` machine:
- `pending` (queued but not started)
- `running` (active worker)
- `paused` (operator paused or app restart interrupted)
- `completed`
- `failed`
- `cancelled` (operator cancelled)
- `interrupted` (crash detection; transitional state → auto-resume
  flow promotes to `running` or `paused` based on settings)

History rolling buffer: when total finished rows (completed +
failed + cancelled) exceeds 100, DELETE the oldest by
`finished_at`. Active rows (pending / running / paused /
interrupted) are never counted toward the 100.

---

## JobRegistry — Tauri-managed state

```rust
pub struct JobRegistry {
    db: Arc<LibraryDb>,
    active: Arc<Mutex<HashMap<i64, JobHandle>>>,
    event_tx: tokio::sync::broadcast::Sender<JobEvent>,
}

pub struct JobHandle {
    job_id: i64,
    cancel: Arc<AtomicBool>,
    pause: Arc<AtomicBool>,   // checked by long-running loops; flipping = pause request
    last_progress: AtomicI64,
}

pub enum JobEvent {
    Created { job_id: i64, snapshot: JobSnapshot },
    Progressed { job_id: i64, done: i64, total: Option<i64> },
    StateChanged { job_id: i64, new_state: JobState },
    Completed { job_id: i64 },
    Failed { job_id: i64, error: String },
}
```

Per-kind workers use the handle pattern:

```rust
let job_id = registry.create_job(JobKind::CoreDownload {
    base: "mednafen_psx_hw_libretro".into(),
}, label, payload)?;
let handle = registry.handle(job_id);

while let Some(chunk) = stream.next().await {
    if handle.cancel.load(Ordering::Relaxed) {
        registry.flush_resume_state(job_id, partial_state)?;
        return Err("cancelled");
    }
    while handle.pause.load(Ordering::Relaxed) {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if handle.cancel.load(Ordering::Relaxed) { return Err("cancelled"); }
    }
    // write chunk + emit progress
    registry.progress(job_id, downloaded_bytes, Some(total_bytes))?;
}
```

Performance contracts:
- `registry.progress(...)` debounces SQLite writes to 1 Hz max.
- Tauri broadcast events from `event_tx` cap at 10 Hz max to the
  frontend.
- Worker threads run on tokio's existing runtime; CPU-heavy work
  uses `tokio::task::spawn_blocking` (already the pattern in
  scan_service / rom_hashes).

---

## Per-kind resume handlers

```rust
pub trait JobResumer: Send + Sync {
    fn resume(
        &self,
        job_id: i64,
        payload: serde_json::Value,
        registry: Arc<JobRegistry>,
        app: AppHandle,
    ) -> tokio::task::JoinHandle<Result<(), String>>;

    fn cleanup_on_cancel(
        &self,
        payload: &serde_json::Value,
        app: &AppHandle,
    ) -> Result<(), String>;
}
```

Each kind owns its resumer alongside the operation it manages.
At app start:

```rust
fn resume_interrupted_jobs(registry: &JobRegistry, settings: &Settings) {
    for job in registry.list_interrupted()? {
        let prompt = settings.prompt_before_resume(&job.kind);
        let resumer = registry.resumer_for(&job.kind);
        if prompt {
            // emit event; UI prompts operator
            emit_resume_prompt(job);
        } else {
            resumer.resume(job.id, job.resume_payload, ...);
        }
    }
}
```

---

## Frontend UI

New `BackgroundJobsBar` Solid component, mounted in
`RetroverseShell` between the main content area and the HintBar.

**State machine:**
- `Hidden` — no jobs, no handle visible
- `HandleVisible` — jobs active, bar collapsed; thin handle at
  bottom-of-window pulses on progress
- `Expanded` — full bar visible with stack rows
- `RecentActivity` — full-screen overlay with tabbed history

**Layout (Expanded):**

```
┌─────────────────────────────────────────────────────────────┐
│ [Pause all] [Cancel all]              Recent activity > [×] │ <- header (when 2+ jobs)
├─────────────────────────────────────────────────────────────┤
│ ▣ Downloading Beetle PSX HW        3.2 MB / 8.4 MB  ⏸ ✕    │ <- per-job row
│   ████████████░░░░░░░░░░░░░░░░░░░░░░ 38%                    │
├─────────────────────────────────────────────────────────────┤
│ 🔍 Identifying PSX ROMs            145 / 200 games   ⏸ ✕   │
│   ████████████████████████░░░░░░░ 72%                       │
├─────────────────────────────────────────────────────────────┤
│ 🖼 Syncing PSX artwork              23 / 200 covers   ⏸ ✕   │
│   ██░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ 11%                      │
├─────────────────────────────────────────────────────────────┤
│ +2 more                                            [show all]│
└─────────────────────────────────────────────────────────────┘
```

**Layout (HandleVisible):**

```
                                              ╲ active jobs ╱
                                                 ▔▔▔▔▔▔▔
```
Thin (~12-16px) handle at bottom-of-window. Pulses softly on
progress events. Click to expand.

**Recent activity panel (full-screen overlay or settings drill-in):**

Tabbed by outcome:
- `Running (3)` (active jobs — same as the bar's expanded view)
- `Completed (47)` (finished successfully)
- `Failed (2)` (errored out)
- `Cancelled (12)` (operator-cancelled)

Each row shows timestamp, kind icon, label, duration, outcome
glyph. Operator can click a row to see error details + [Retry]
on failed rows.

---

## Operations to consolidate (existing inventory)

| Operation | Kind | Today's surface (retires) | Real-progress contract |
| --- | --- | --- | --- |
| `download_core` | `core_download` | toast | bytes_downloaded / total_bytes (HTTP Content-Length) |
| Guided Setup bulk install | `bulk_core_install` | modal w/ child progress | n_done / n_requested cores |
| `sync_rom_hashes_for_system` | `dat_sync` | inline; no progress today | single fetch — show indeterminate / done split |
| `resolve_rom_hashes_for_system` | `hash_resolve` | per-system events | n_hashed / n_total |
| `refresh_mame_system_info` | `mame_listxml_import` | dialog progress | n_records / n_records (atomic) |
| `sync_media_for_system` | `artwork_sync` | per-system events | n_arts_done / n_arts_total |
| `sync_media_for_system` (metadata path) | `metadata_sync` | per-system events | n_games_metadata / n_games_total |
| `start_background_scan` | `folder_scan` | inline wizard progress | files_seen (no total until walk completes) |
| (future) per-track SHA-1 | `disc_track_hash` | n/a (planned) | n_tracks / n_total_tracks per disc; bytes / bytes per track |
| Cover / fanart / wheel download | folds into `artwork_sync` | per-art event | per-art |
| Thumbnail repo sync | folds into `artwork_sync` | per-file event | per-file |

---

## Sizing

Rough phasing — ~5-6 weeks total:

- **Phase 1 — schema + JobRegistry + first kind wired** (~1 week):
  `background_jobs` SQLite table + migration. `JobRegistry`
  Tauri-managed state. `JobHandle` shape + event broadcast.
  Lock file + heartbeat infrastructure. Wire `core_download`
  end-to-end (smallest progress contract, easiest verify).
  Smoke test: create → progress → cancel → resume → recover-from-crash.

- **Phase 2 — BackgroundJobsBar UI** (~1 week): new Solid
  component, mounted in RetroverseShell. Handle / collapsed /
  expanded states + animation. Stack-visible layout (max 3 +
  "+N more"). Per-row controls. Bar header for 2+ jobs with
  Pause-all / Cancel-all. Hooked to Tauri event broadcast.

- **Phase 3 — Resume infrastructure + 3 more kinds** (~1.5
  weeks): `JobResumer` trait + per-kind handlers for
  `core_download` / `artwork_sync` / `hash_resolve`. Cancel
  cleanup per kind. Auto-resume-on-launch flow w/ per-kind
  opt-out prompt. Duplicate-trigger Wait/Restart/Cancel dialog.

- **Phase 4 — wire remaining kinds + dependency graph** (~1.5
  weeks): `folder_scan` (with unknown-total / pulsing handle
  variant). `metadata_sync`. `mame_listxml_import`. `dat_sync`.
  `bulk_core_install` with parent-row aggregation. Dependency
  graph: `parent_job_id` chain, auto-trigger prereqs, cancel
  prompt for "Just this one / The whole chain." Per-kind retry
  policy. `disc_track_hash` integration once that arc lands
  (this phase pipelines with the disc-track work).

- **Phase 5 — Settings panel + Recent activity + polish** (~1
  week): "Download Settings" top-level category. Per-kind
  auto-resume toggles. Bar behavior (always-show, sound on
  completion). Retry policy controls. Recent activity full
  panel (tabbed by outcome, last 100). Operator playtest;
  performance check (10Hz event saturation); crash-recovery
  testing.

---

## Risks

- **Resume implementation per kind is real engineering.** Each
  operation needs its handler. Phase 3 ships with 3 kinds (the
  trait + the pattern); Phase 4 wires the rest. Operations that
  already track per-item idempotent state (artwork, metadata,
  hash_resolve) are easy; HTTP downloads need Range support
  (already partly there via `.partial` pattern).

- **State drift between app session + on-disk reality.** Half-
  downloaded files, partial scan state, stale cache. The per-kind
  `cleanup_on_cancel` contract is the answer; needs implementing
  alongside every resumer.

- **Performance.** SQLite write at every progress tick could
  thrash the DB on a fast HTTP loop. The 1Hz debounce + 10Hz
  Tauri event cap handle this. In-memory job state is always
  current.

- **Concurrent jobs of the same kind across different systems.**
  Most operations already use per-system mutexes
  (`MediaState::gate_for`). The job system needs to respect those
  and queue same-(kind, system) tuples even though same-kind
  different-system would otherwise run in parallel. The kind +
  system_id tuple in the schema supports this; the queue dispatch
  needs to check both.

- **Operator confusion when a job "completes" but the underlying
  data still updates.** Example: hash_resolve completes, shows ✓,
  then artwork_sync kicks off automatically as a dependent. Make
  the chain visible — the recent-activity panel shows what
  triggered what (via `parent_job_id`).

- **The bar feels too noisy.** The auto-hide + handle-only-when-
  active design mitigates most of this. If real-world testing
  shows the handle pulsing too often → tune the pulse to only
  fire on state changes (progress ticks below 1% delta skip the
  pulse).

---

## Out of scope (won't do here)

- **Adding new background operations.** Consolidating + persisting
  + UI'ing what's already there + what the disc-track arc adds.
  New ops slot in by implementing the `JobResumer` trait + adding
  an entry in `JobKind`.
- **Replacing the existing progress event protocol entirely.**
  Today's `oa://core-download-progress` /
  `oa://library-scan-progress` etc. emit unchanged; the job
  system listens + maps them onto job rows. Future ops can emit
  `oa://job-progress` directly with the job_id.
- **Cross-machine sync of jobs.** Each install runs its own queue.
- **Scheduled / cron-style background work.** Operator-triggered
  only.
- **Modal-foreground operations** (wizard pre-commit review,
  per-game settings dialog, etc.). The job system is for
  background work that the operator can ignore + come back to.
- **Per-kind / per-system completion chime variants.** v1 ships
  with a single shared chime. Per-kind / per-system variants are
  a PARKING_LOT-worthy polish item.

---

## When this arc starts

This plan is approved + queued (planning locked 2026-06-02) but
deferred. The executing session should:

1. **Re-read this plan in full.**
2. **Re-validate the per-kind inventory** in §"Operations to
   consolidate". New ops may have landed; existing ops may have
   changed shape.
3. **Confirm the schema** with the operator before kicking off
   Phase 1. The `background_jobs` table shape is the foundation
   everything else depends on; any schema drift here ripples
   through every resumer.
4. **Branch as `feat/background-jobs-phase-1`** per the standard
   workflow.
5. **Phase 1 pilot: `core_download`** — simplest progress
   contract, smallest UI surface, easiest to verify end-to-end.

---

*Plan refined 2026-06-02 across 6 rounds of operator Q&A. 24
design decisions locked. Original framing: "a real progress bar
at the bottom of the UI that says exactly what OA is doing, with
real numbers, and that remembers what it was doing when I close
the app."*
