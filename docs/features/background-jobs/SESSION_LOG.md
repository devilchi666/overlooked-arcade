# Background jobs + persistent progress bar — Session Log

Three-line format per session entry: **Shipped / Almost / Next.**
Cross-stream context goes in the project-wide
[`docs/SESSION_LOG.md`](../../SESSION_LOG.md) instead. Source-of-truth
for the arc design is [`docs/PLANS/background-jobs-and-progress-bar.md`](../../PLANS/background-jobs-and-progress-bar.md).

---

## 2026-06-02 — Phase 1 ships the backend pilot (`feat/background-jobs-phase-1`)

**Shipped:** Phase 1 of the 5-phase arc landed in five phase commits
on `feat/background-jobs-phase-1`, pending operator end-to-end smoke
test before merge.

- **Slice A — docs scaffold** (`7add49c`). `docs/features/background-jobs/README.md`
  + ACTIVE_WORK in-flight entry + INDEX cross-cutting link.
- **Slice B — schema migration v17→v18** (`5c734d5`). New
  `background_jobs` table + 3 indexes per plan §Schema. SCHEMA_VERSION
  bumped to 18; `migrate_v17_to_v18` arm added; new
  `schema_v17_to_v18_migration` test verifies table + indexes + row
  round-trip. `parent_job_id` uses ON DELETE SET NULL so the 100-row
  rolling-buffer prune of finished parents doesn't cascade and drop
  their in-flight children.
- **Slice C — JobRegistry module** (`e3ac548`). New
  `apps/oa-shell/src/job_registry.rs` (~700 LOC) containing the full
  backend surface: `JobKind` enum (Phase 1 only names CoreDownload),
  `JobState` + parse/as_str, `JobSnapshot` wire-format, `JobEvent`
  broadcast payload, `JobHandle` (cancel + pause AtomicBool +
  last_db_write_ms + last_event_ms rate-limit cells), `JobRegistry`
  itself wrapping `Arc<Inner>` for cheap Clone. Performance contracts
  per plan §Sizing: 1 Hz SQLite write debounce, 10 Hz Tauri broadcast
  event cap, ~1 s heartbeat task bumping `last_event_at` for running
  rows. History rolling buffer at HISTORY_CAP=100 finished rows
  pruned via single DELETE on each finalize. Test-only constructor
  `new_for_tests` bypasses the AppHandle requirement and skips the
  heartbeat. 7 unit tests cover create round-trip, debounce write
  semantics, mark-completed handle-drop invariant, mark-cancelled +
  flush_resume_state, crash-promotion idempotence, list_active state
  filter, and the 100-row rolling buffer cap.
- **Slice D — lock-file + crash-detection plumbing** (`916cd31`).
  `<data_dir>/oa.lock` written in `setup()` and removed AFTER `.run()`
  returns. Lock file present at startup → previous run crashed →
  `JobRegistry::promote_running_rows_to_interrupted` runs on
  registry construction. Path shuttles from inside setup() to the
  post-run cleanup via the same `Arc<OnceLock<PathBuf>>` pattern the
  window-geometry flusher uses. Registry construction is soft-fail:
  if it errors at startup the download path still works, only the
  new tracking degrades.
- **Slice E — core_download pilot wiring** (`86c9a96`).
  `core_installer::download_core` registers a `core_download` job at
  entry (label "Downloading {catalog display_name}" with bare-base
  fallback for ad-hoc downloads), polls JobHandle cancel + pause from
  inside the chunk loop, pipes per-chunk progress through
  `registry.progress()` (registry rate-limits internally). The
  existing `oa://core-download-progress` emit stays intact so Guided
  Setup's listener doesn't break (operator approved double-emit per
  plan §"Out of scope"). Body wrapped in `async { ... }.await` so
  the finalize block (mark_completed / mark_cancelled with .partial
  cleanup / mark_failed) runs exactly once regardless of which error
  path took us out. `use tauri::Manager` added for `try_state`.

660 of 660 oa-shell tests green at branch tip (was 645 pre-branch;
+1 schema migration test + 7 job_registry tests + 7 net deltas from
unrelated tests passing newly).

**Almost:** End-to-end manual smoke test still pending operator
validation:
1. trigger a core download → confirm progress events flow in the
   `background_jobs` row (`done` + `total` advance, `state='running'`)
   AND the existing `oa://core-download-progress` toast/Guided-Setup
   modal still works.
2. cancel mid-download via flipping the JobHandle.cancel flag from a
   future Phase 2 UI affordance (Phase 1 has no UI; operator can
   test via a temporary Tauri command if desired) → confirm row goes
   `state='cancelled'` + `.partial` file deleted.
3. kill the app mid-download (close window forcibly / kill -9) →
   relaunch → confirm lock-file detection logs the warn line +
   row's `state` flipped from `running` to `interrupted`.

**Next:** Phase 2 — frontend `BackgroundJobsBar` Solid component.
Mounted in `RetroverseShell` between the main content area and the
HintBar. Handle / collapsed / expanded state machine, max 3 visible
rows + "+N more", per-row pause + cancel controls, bar header with
Pause-all / Cancel-all for 2+ jobs. Hooks to `oa://job-event`. See
plan §Sizing Phase 2 for the ~1-week scope.
