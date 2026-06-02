# Background jobs + persistent progress bar

**Status:** Planning (operator-requested 2026-06-02, execution deferred to a future session). No code yet.

**Owner-of-decisions:** the operator. This document records the
shape of the work + design questions still open. Revisit before
kicking off.

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

That's three pieces of work bundled together:
- **Real-progress contract** — every long-running op emits
  authoritative `done / total` numbers, no fake percentages.
- **Persistent job queue** — operations register as jobs that
  survive process restart, with enough state on disk to resume.
- **Single UI surface** — a persistent bar across the bottom of
  the Retroverse shell showing the active job (and possibly a
  queue of pending ones).

---

## What this is + what it isn't

**This arc:**
- A `JobQueue` / `JobRegistry` Tauri-managed state holder that
  every long-running operation registers itself with at start.
- A job-status persistence layer in SQLite — jobs serialize their
  state into `background_jobs` table on creation + at every
  meaningful progress increment + at app close.
- A standardized job-progress shape: `{ id, kind, label, done,
  total, unit, state, last_event_at, can_resume, resume_payload }`.
  `unit` is the unit being counted ("files" / "bytes" / "games" /
  "tracks"); `state` is one of `pending | running | paused |
  completed | failed | cancelled`. `resume_payload` is op-specific
  JSON the kind's resume handler knows how to consume.
- Per-job-kind resume handlers — each operation that wants to
  survive restart implements a handler that takes the persisted
  `resume_payload` + returns to where it left off.
- A persistent progress-bar UI component in the Retroverse shell
  showing the currently-running job + an expandable view of
  pending / paused jobs. Probably at the bottom of the window
  (HintBar adjacent), sized small enough to not steal real estate.
- A pause/resume affordance per job (where resumability makes
  sense — see §"Resumability per operation").
- A cancel affordance per job (always available; cancelled jobs
  leave on-disk state in a known-good shape).
- An app-launch flow that surfaces interrupted jobs:
  - "You had 3 unfinished jobs last session. Resume them now?"
  - Or auto-resume per-job based on a per-kind policy (e.g. core
    downloads auto-resume, media sync prompts).

**Not this arc:**
- New operations. We're consolidating + persisting + UI'ing the
  operations that exist today (plus the disc-track SHA-1 work
  that's queued). No new background work gets added by this arc.
- Foreground-modal operations. Some operations are
  operator-blocking by design (e.g. the wizard's pre-commit
  review). Those stay foreground; the persistent bar surfaces
  background work only.
- Cross-app job scheduling. Each OA install runs its own queue;
  no networked / shared / multi-install coordination.
- The progress-event protocol redesign. Today's
  `oa://core-download-progress` / `oa://library-scan-progress`
  / etc. events stay; the job system listens to them + maps
  them onto job rows. We don't rewrite every emitter.

---

## Operations to consolidate (Phase 1 inventory)

For each, note what the op does today + what its real-progress
contract looks like + what resume-from-last looks like.

| Operation | Today's surface | Real-progress contract | Resume-from-last |
| --- | --- | --- | --- |
| Core download (`download_core`) | per-call toast via `oa://core-download-progress` | bytes_downloaded / total_bytes (HTTP `Content-Length`); zips are typically <10 MB so total is always known | HTTP Range request from the existing `.partial` file's size — already most of the way there; needs the queue to remember WHICH core was being downloaded |
| Bulk core install (Guided Setup) | `MissingCoreBulkPrompt` modal | n_completed / n_requested core IDs | Re-runs the per-system list, skips installed cores |
| libretro-database dat sync (`sync_rom_hashes_for_system`) | inline progress in resolve flow | single HTTP fetch; cheap; no progress emit today | Atomic — retry on next launch |
| ROM hash resolve (`resolve_rom_hashes_for_system`) | per-system progress emit via `oa://rom-hashes-resolved` | n_hashed / n_total per system | Per-game; rows already-stamped are skipped on re-run |
| MAME ROM-set listxml import (`refresh_mame_system_info`) | dialog progress bar | listxml parse is fast; not really "long-running" — borderline | Atomic — retry on next launch |
| Media sync (`sync_media_for_system`) | per-system; emits `oa://media-sync-progress` | n_games_resolved / n_games_total | Per-game; already-resolved games skip |
| Folder scan (`start_background_scan`) | inline progress in wizard | files_seen + matches; total unknown up front (walking the tree) | Re-scans from the start (cheap; whole walk is single-digit seconds for a 10K-file folder) |
| Disc-track SHA-1 (future arc) | not implemented yet | n_tracks_hashed / n_tracks_total per disc; bytes_hashed / bytes_total per track | Per-track cache on the game row |
| Cover-art / fanart download (per game) | inline; `oa://media-sync-progress` carries it | n_arts_downloaded / n_arts_requested | Per-art; already-cached art is skipped |
| Thumbnail repo sync (libretro-thumbnails) | per-system; events | n_files_downloaded / n_files_expected | Per-file; already-downloaded files skip |

Operations marked "Atomic" don't really need resume — they're
short enough that retry-from-scratch is cheaper than tracking
mid-state. They still register as jobs so the operator can SEE
them happening, but `can_resume` is false; cancel just kills the
job without saving state.

Operations marked "Per-game" / "Per-file" / "Per-track" resume
trivially because their work is idempotent at the per-item level
— resume = "iterate the work list, skip items already done."

The HARD case is the single-stream operations (the core download
HTTP fetch). HTTP Range request resume is the standard answer
(already partly in place via the `.partial` file pattern); the
job queue just needs to persist the URL + the destination path
so the kind's resume handler knows where to pick up.

---

## Technical shape

### Schema

New SQLite table:

```sql
CREATE TABLE background_jobs (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    kind            TEXT NOT NULL,    -- "core_download", "scan", "media_sync", ...
    label           TEXT NOT NULL,    -- operator-facing "Downloading Beetle PSX HW"
    state           TEXT NOT NULL,    -- pending | running | paused | completed | failed | cancelled
    done            INTEGER NOT NULL DEFAULT 0,
    total           INTEGER,          -- nullable: some ops genuinely don't know
    unit            TEXT NOT NULL,    -- "bytes" | "files" | "games" | "tracks"
    last_event_at   INTEGER NOT NULL, -- unix ms; for stale-job detection
    started_at      INTEGER NOT NULL,
    finished_at     INTEGER,          -- nullable
    can_resume      INTEGER NOT NULL DEFAULT 0,  -- bool: kind supports resume
    resume_payload  TEXT,             -- JSON: kind-specific resume state
    error_message   TEXT              -- nullable: last error
);

CREATE INDEX idx_background_jobs_active ON background_jobs (state, last_event_at);
```

`state = 'running'` rows at app start are jobs interrupted by a
crash / close. The launch flow looks at these.

### Tauri-managed `JobRegistry`

```rust
pub struct JobRegistry {
    db: Arc<LibraryDb>,
    active: Arc<Mutex<HashMap<i64, JobHandle>>>,
    event_tx: tokio::sync::broadcast::Sender<JobEvent>,
}

pub struct JobHandle {
    job_id: i64,
    cancel: Arc<AtomicBool>,
    pause: Arc<AtomicBool>,  // checked by long-running loops
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

Long-running operations create a job at start:
```rust
let job_id = registry.create_job(JobKind::CoreDownload, label, can_resume: true, payload)?;
let handle = registry.handle(job_id);

while let Some(chunk) = stream.next().await {
    if handle.cancel.load(Ordering::Relaxed) { return Err("cancelled"); }
    while handle.pause.load(Ordering::Relaxed) {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if handle.cancel.load(Ordering::Relaxed) { return Err("cancelled"); }
    }
    // write chunk + emit progress
    registry.progress(job_id, downloaded_bytes, Some(total_bytes))?;
}
```

`registry.progress(...)` debounces writes to SQLite (probably
1 Hz max) so a tight loop doesn't thrash the DB.

Tauri broadcast events from `event_tx` flow to the frontend at
~10 Hz max so the progress bar updates smoothly without saturating
the IPC channel.

### Per-kind resume handlers

```rust
pub trait JobResumer {
    fn resume(
        job_id: i64,
        payload: serde_json::Value,
        registry: &JobRegistry,
        app: AppHandle,
    ) -> tokio::task::JoinHandle<Result<(), String>>;
}

// Per-kind impls live next to the operation they manage:
impl JobResumer for CoreDownloadResumer { ... }
impl JobResumer for MediaSyncResumer { ... }
// ...

// At app start:
fn resume_interrupted_jobs(registry: &JobRegistry) {
    for job in registry.list_resumable_interrupted()? {
        let resumer = pick_resumer(&job.kind);
        resumer.resume(job.id, job.resume_payload, registry, app.clone());
    }
}
```

Each operation owns its own resume handler — no central dispatcher
trying to understand every op's state. The handler knows what the
payload means + how to re-enter the operation at the right point.

### Frontend UI

A new `BackgroundJobsBar` Solid component, mounted in
`RetroverseShell` at the bottom of the window (above the existing
HintBar; HintBar takes priority when shown). Layout:

```
+--------------------------------------------------------------+
| Downloading Beetle PSX HW   3.2 MB / 8.4 MB        [▢] [×]   |
| ████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ 38%  +2 queued |
+--------------------------------------------------------------+
```

- Single-line bar: label + numbers + bar + pause + cancel
  buttons + a "+N queued" affordance
- Clicking the bar expands a panel showing all active + queued
  jobs (max-height; scrollable)
- Auto-hides after 2 seconds of empty-queue
- Listens to `JobEvent` broadcasts via a Tauri event subscription

### Operator-facing resume prompt

At app launch, IF there are any `state = 'running'` rows in
`background_jobs`:

```
Last session had unfinished work
┌─────────────────────────────────────────┐
│ ▶ Downloading Beetle PSX HW             │
│   38% complete (3.2 of 8.4 MB)          │
│   [Resume] [Discard]                    │
│                                         │
│ ▶ Resolving NES ROM hashes              │
│   145 of 200 ROMs hashed                │
│   [Resume] [Discard]                    │
└─────────────────────────────────────────┘
        [Resume all] [Discard all]
```

Alternative — auto-resume per kind based on a policy. Core
downloads auto-resume (low-friction); scans + hash passes
prompt (they're CPU-heavy and the operator might want to defer).
Operator decides which kind goes which way.

---

## Open design questions

1. **Resume prompt vs auto-resume.** First-pass instinct is "always
   prompt" — operator likes control. But for a core download
   that's 90% done, asking "want to finish this?" every launch is
   noise. Per-kind policy may be the answer; needs operator call
   per kind.

2. **Pause semantics — actual pause vs cancel-and-remember.**
   Cleanest is `pause = stop work + persist resume state`. Then
   resume is the same code path the app-restart resume uses. But
   some ops (HTTP downloads) can pause without closing the
   connection if the pause is short. Worth optimizing? Or just
   always close + reopen on resume? Probably the simpler "close
   and reopen" — handles every resume case uniformly.

3. **Concurrent jobs — show one or stack?** Most users only have
   1-2 jobs running at once. Showing the topmost active job
   + "+N queued" feels right for the common case. Power users
   might want a stack. Defer; ship with single-line UI + the
   expandable panel for queue inspection.

4. **What about jobs that genuinely can't measure `total`?**
   Folder scans walk the FS — total file count is unknown until
   the walk completes. Show "Scanning... 1,247 files seen" with a
   pulsing bar (no percent), then switch to percent once the walk
   finishes + the next phase (hashing / classifying) starts. Need
   to handle the unknown-total case in the schema (`total NULL`)
   and the UI (pulsing-bar mode).

5. **Job priority + ordering.** When multiple ops are queued, who
   runs first? FIFO? Per-kind priority (downloads first; sync
   last)? Operator-promoted? Start with FIFO; revisit if a real
   ordering need shows up.

6. **What about the "Identify ROMs" flow's existing inline
   progress?** Today the operator clicks Identify in
   Settings → Library and a modal opens with a progress bar.
   When the job system lands, do we (a) keep the modal but ALSO
   register a job that shows in the bar, (b) drop the modal and
   route everything through the bar? (a) is gentler; (b) is more
   consistent. Probably (a) for the transition + revisit later.

7. **What about toast notifications?** Today some ops emit toasts
   on completion ("✓ Downloaded Beetle PSX HW"). The job system
   could replace these (the bar shows the job; completion just
   removes it). Or both could coexist. Probably both, with the
   toasts as a celebration on completion + the bar for the
   in-progress state.

8. **Crash recovery.** If the app crashes mid-job, the
   `background_jobs` row stays at `state = 'running'`. The
   launch flow detects this via `last_event_at` being old (>5
   seconds before crash) and flips to "interrupted" before
   prompting. What about jobs that legitimately ran for hours
   between events? Probably a stale-detection heuristic per
   kind (some kinds expect frequent updates; others don't).

9. **Disk-resident state outside SQLite.** A core download has a
   `.partial` file on disk. A scan has cached file metadata.
   These need consistent cleanup when a job is "Discard"ed at
   app launch. Per-kind cleanup function alongside the resume
   handler.

10. **What if the user has reorganized their library between
    sessions?** A scan job that was paused mid-folder might have
    file paths that don't exist anymore. The resume handler needs
    to be defensive — skip-missing rather than fail-the-job.

---

## Sizing

Rough phasing — ~5-6 weeks total:

- **Phase 1 — schema + JobRegistry + first kind wired** (~1 week):
  `background_jobs` SQLite table + migration. `JobRegistry`
  Tauri-managed state. `JobHandle` shape + event broadcast
  channel. Wire ONE operation (probably `download_core` — it's
  the simplest with the clearest progress contract). End-to-end
  smoke test: launch download → progress events → cancel works.

- **Phase 2 — BackgroundJobsBar UI** (~1 week): new Solid
  component, mounted in RetroverseShell. Auto-hide / single-line
  / expandable panel. Listens to `JobEvent` broadcast via Tauri
  event. Pause + cancel buttons wired to the JobRegistry.
  Validation: end-to-end with the Phase 1 download kind.

- **Phase 3 — resume infrastructure + 3 more kinds** (~1.5 weeks):
  `JobResumer` trait. Per-kind handlers for `core_download`,
  `media_sync`, `resolve_rom_hashes`. App-launch detect +
  prompt flow. Each kind's resume tested end-to-end (close app
  mid-job; relaunch; resume from where it left off).

- **Phase 4 — wire remaining kinds + edge cases** (~1.5 weeks):
  Folder scan (with unknown-total / pulsing-bar shape).
  Thumbnail repo sync. MAME listxml import. Per-track SHA-1
  matching (once that arc lands; this phase pipelines with it).
  Pause-actually-works tests per kind. Stale-job detection.

- **Phase 5 — operator playtest + polish** (~1 week):
  Real-library testing. Performance check (does the 10 Hz
  event rate saturate IPC on a noisy job?). Crash recovery
  testing (kill app mid-job; verify clean recovery). Documentation
  + per-core README mentions.

---

## Risks

- **Resume implementation per kind is real engineering.** Each
  operation needs its handler. Costs grow with operation count.
  Mitigation: ship Phase 3 with 3 kinds; document the trait so
  future operations follow the pattern from day 1.

- **State drift between app session + on-disk reality.** Half-
  downloaded files, partial scan state, stale cache. Per-kind
  cleanup function is the answer; needs to be wired alongside
  every resume handler.

- **UI noise.** Operator wants to SEE progress, doesn't want to be
  buried in it. The auto-hide + single-line + collapsed-by-default
  design is the answer; needs validation in playtest.

- **Performance.** Persisting to SQLite at every progress tick
  could thrash the DB on a fast loop (HTTP download chunks come
  fast). Debounce at the source (1 Hz max persistence; 10 Hz max
  event broadcast); the in-memory job state is always current.

- **Concurrent jobs of the same kind.** What if the operator
  triggers a media sync for psx while one for nes is already
  running? Currently most operations use per-system mutexes
  (`MediaState::gate_for`); jobs need to respect those + queue
  rather than parallel-run. May require modeling job
  dependencies / mutex relationships.

- **Operator confusion when a job "completes" but the underlying
  data still updates.** Example: ROM hash resolve completes,
  shows ✓, but the operator THEN runs media sync which kicks
  off another job. Maintain the visual difference between
  "this specific operation completed" and "all background work
  is done."

---

## Out of scope (won't do here)

- **Adding new background operations.** Consolidating + persisting
  + UI'ing what's already there. New ops would slot into the
  trait once it exists.
- **Replacing the existing progress event protocol.** Today's
  `oa://core-download-progress` / `oa://library-scan-progress`
  etc. stay; the job system listens + maps. Future ops can
  emit `oa://job-progress` directly.
- **Cross-machine sync of jobs.** Each install runs its own queue.
- **Scheduled / cron-style background work.** Operator-triggered
  only.
- **Modal-foreground operations** (wizard pre-commit review,
  per-game settings dialog, etc.). Job system is for background
  work that the operator can ignore + come back to.

---

## When this arc starts

This plan is approved + queued (2026-06-02) but deferred. The
executing session should:

1. **Re-read this plan in full.**
2. **Re-validate the per-kind inventory** in §"Operations to
   consolidate". New ops may have landed; existing ops may have
   changed shape.
3. **Confirm the schema + persistence model** with the operator
   before kicking off Phase 1. The `background_jobs` table shape
   is the foundation everything else depends on.
4. **Branch as `feat/background-jobs-phase-1`** per the standard
   workflow.
5. **Pick the Phase 1 pilot operation** (default suggestion:
   `download_core` — simplest progress contract, smallest UI
   surface, easiest to verify end-to-end). Operator may prefer a
   different pilot if a specific op's noisy progress bothers
   them more.

---

*Plan written 2026-06-02 after the Slice 2 closure + the disc-track
SHA-1 plan landed. Operator framing: "a real progress bar at the
bottom of the UI that says exactly what OA is doing, with real
numbers, and that remembers what it was doing when I close the app."*
