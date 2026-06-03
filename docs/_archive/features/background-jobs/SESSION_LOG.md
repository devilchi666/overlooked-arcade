# Background jobs + persistent progress bar — Session Log

Three-line format per session entry: **Shipped / Almost / Next.**
Cross-stream context goes in the project-wide
[`docs/SESSION_LOG.md`](../../SESSION_LOG.md) instead. Source-of-truth
for the arc design is [`docs/PLANS/background-jobs-and-progress-bar.md`](../../PLANS/background-jobs-and-progress-bar.md).

---

## 2026-06-03 — Polish branch closes the deferred items (`feat/background-jobs-polish`)

**Shipped:** Phase 5 deferred three polish items (always-show-bar
toggle, completion chime, ResumePrompt dialog). All three land in
this branch as a follow-up to the closed arc. Four phase commits.

- **Polish A** (`79e9248`) JobPrefs gains `sound_on_completion`
  (default true) + `always_show_bar` (default false) fields. Two
  new Tauri commands `set_job_sound_on_completion` +
  `set_job_always_show_bar`. JobPrefs gets an explicit Default
  impl since `#[derive(Default)]` doesn't compose with the
  non-default `sound_on_completion` field's `#[serde(default =
  "fn")]`.
- **Polish B** (`0f746dd`) Completion chime — new Tauri command
  `audio_player::resolve_completion_chime` looks up
  `<exe_dir>/assets/oa-ui/sounds/job-complete.<ext>` (same
  extension priority as the per-system UI sound resolver) and
  returns Some(path) or None. Frontend `maybePlayCompletionChime`
  fires on the `oa://job-event` completed handler when
  `jobPrefs.soundOnCompletion` is true, dispatching through the
  existing `playAudio("ui-sounds", path)` path. Silent fail when
  the asset isn't bundled. New doc
  `docs/features/background-jobs/ASSETS.md` explains exact
  placement + supported formats + sourcing tips + the deliberate
  silent-fallback semantics. Operator can drop the file whenever
  they source it.
- **Polish C** (`6555f56`) Always-show toggle — BackgroundJobsBar
  reads `jobPrefs.alwaysShowBar`; when ON, renders the handle
  even with no active jobs (label "No active jobs", pulse dot
  static + dim). Settings → Background Jobs → Bar behavior card's
  two toggles (Always show + Sound on completion) unstubbed and
  wired through `set_job_always_show_bar` /
  `set_job_sound_on_completion` Tauri commands. Each toggle
  handler calls `refreshJobPrefs()` so the module-level live
  signal syncs and the bar reacts immediately.
- **Polish D** (`33be3fe`) ResumePrompt dialog — new
  `resume_one_interrupted_job(job_id)` Tauri command dispatches
  the registered resumer for one specific interrupted row. New
  `ResumePromptDialog.tsx` consumes the `resumePromptQueue`
  signal (added in polish B), shows a modal for the first row
  with Resume / Discard buttons. Resume invokes the new command;
  Discard invokes `cancel_job`. Escape + backdrop close without
  action (the row stays interrupted for next launch). z-[70] so
  it sits above the bar + HintBar + Recent activity panel.
  Mounted in App.tsx as a sibling of BackgroundJobsBar.

660 of 660 oa-shell tests green; frontend `npm run typecheck`
silent.

**Almost:** End-to-end smoke test:
1. Settings → Background Jobs → Bar behavior → "Always show the
   bar" ON → the handle stays visible at the bottom of the
   viewport with "No active jobs" label even when nothing is
   running.
2. Same panel → "Sound on completion" ON (default) → trigger a
   job → on success, no chime plays UNLESS the operator has
   dropped a `job-complete.<ext>` into the assets folder. With
   the asset present, the chime plays through the ui-sounds bus.
3. Per-kind opt-out flow end-to-end: tick "Prompt before
   resuming core downloads" → trigger a download → kill app
   mid-stream → relaunch → ResumePromptDialog surfaces with the
   download's label and Resume / Discard buttons. Resume
   re-enters the Range-resumed download.

**Asset placement reference:**
`docs/features/background-jobs/ASSETS.md` is the operator-facing
file. tldr: drop one file at
`<exe_dir>/assets/oa-ui/sounds/job-complete.<ext>` where ext is
one of ogg/opus/wav/mp3/flac/m4a (first match wins). Operator
can source whenever; OA stays silent until then.

---

## 2026-06-03 — Phase 5 closes the arc (`feat/background-jobs-phase-5`)

**Shipped:** Phase 5 — the operator-facing polish that closes the
7-phase arc as a finished feature. Three phase commits.

- **Slice A — resumers for the remaining six kinds** (`a827c86`).
  Generic `ReinvokeOperatorResumer` (a tiny inline impl in main.rs)
  registered against artwork_sync, hash_resolve, dat_sync,
  folder_scan, mame_listxml_import, bulk_core_install. Strategy:
  mark_cancelled the orphan interrupted row + log a breadcrumb
  with the operator's re-trigger path. The underlying operations
  are all internally idempotent (artwork skips already-downloaded,
  hash_resolve skips already-stamped, etc.) so an operator-
  triggered re-run picks up where the crash left off. Distinct
  from core_download's Phase 3a/3b byte-level Range resume — that
  kind has on-disk state worth a true resume; the others don't.
  Closes the "no resumer registered" warns the Phase 4a/4b kinds
  were logging at every startup.
- **Slice B — Settings → Background Jobs category** (`7c97639`).
  New top-level System-group category in SettingsPage.tsx (glyph
  ⟳). Four cards:
  - **Auto-resume on launch** — per-kind opt-out toggles for the
    seven operator-facing kinds, backed by Phase 3b's
    get_job_prefs / set_job_resume_prompt commands.
  - **Bar behavior** — Always-show + Sound-on-completion toggles,
    stubbed-disabled (Phase 6 polish — neither the always-show
    signal nor the completion chime asset exists yet).
  - **Failure handling** — read-only summary of Phase 4c's
    1s/5s/30s retry policy.
  - **History** — live "X of 100 history rows used" counter + a
    new `clear_job_history` Tauri command + button (wipes
    finished rows, preserves active state).
- **Slice C — Recent activity panel** (`ef543c0`). Full-viewport
  overlay (z-65) tabbed by outcome (Running / Completed / Failed
  / Cancelled). Per-row: kind icon + label + duration +
  finished-at timestamp + outcome glyph. Failed rows show
  error_message inline. Triggered from a new "Recent activity →"
  link in the BackgroundJobsBar's expanded header. Reads
  activeJobs() live for the Running tab; invokes list_recent_jobs
  on open + click-Refresh for the three finished tabs. Escape +
  backdrop-click close.

660 of 660 oa-shell tests green; frontend `npm run typecheck`
silent.

**Almost:** End-to-end smoke test:
1. Settings → Background Jobs → tick "Prompt before resuming
   core downloads" → trigger a download → kill the app
   mid-stream → relaunch → log shows "emitting prompt for job N"
   instead of "dispatching resume"; the bar doesn't surface the
   interrupted row (Phase 6 polish: surface ResumePrompt as a
   bar dialog).
2. Trigger several core downloads + a folder scan + an Identify
   ROMs pass → expand the bar → click "Recent activity →" →
   panel opens with Running tab showing 3+ rows; tab over to
   Completed/Failed/Cancelled (will be empty on first launch);
   click "Clear recent activity" in Settings to wipe finished
   rows.
3. Kill the app during an Identify ROMs pass → relaunch → log
   shows "background_jobs: hash_resolve job N interrupted by
   previous run; auto-cancelling — re-trigger via Settings →
   Library → Identify ROMs". The row is cancelled, doesn't sit
   in interrupted forever.

**Arc closed.** Seven phases shipped end-to-end:
  - Phase 1 (backend pilot)
  - Phase 2 (BackgroundJobsBar + dev affordance)
  - Phase 3a (JobResumer + pause/resume bridge + crash recovery)
  - Phase 3b (byte-level Range resume + per-kind opt-out + dup-
    trigger)
  - Phase 4a (folder_scan + hash_resolve + dat_sync + MAME)
  - Phase 4b (artwork_sync + bulk_core_install parent + z-fix)
  - Phase 4c (dependency graph + retry policy)
  - Phase 5 (resumers + Settings panel + Recent activity)

The bar surfaces every long-running operation the operator can
kick off; pause/resume + cancel work end-to-end; byte-level Range
resume survives process exit for core downloads; per-kind opt-out
gives operators control over the auto-resume default; recent
activity panel + Settings → Background Jobs give the operator a
permanent home for everything the system does behind the scenes.
Plan §"Out of scope" items remain deferred (cross-machine sync,
scheduled cron-style work, per-kind completion chime variants).

---

## 2026-06-03 — Phase 4c ships dependency graph + retry policy (`feat/background-jobs-phase-4c`)

**Shipped:** Phase 4c — the two main pieces of the original Phase 4
orchestration scope. Two phase commits; the third planned slice
(artwork_sync + hash_resolve resumers) scoped out as a follow-up.

- **Slice A — dependency graph (HashResolve → DatSync prereq)**
  (`9713d7d`). `auto_sync_rom_hashes_if_empty` gains a
  `parent_job_id: Option<i64>` argument. When the caller passes
  `Some(id)`, the auto-sync registers itself as a visible DatSync
  row with `parent_job_id = id` + `is_prereq = true` and a
  " (prereq)" label suffix. Plan §"Job dependencies": auto-trigger
  prereqs. `resolve_rom_hashes_for_system` passes its HashResolve
  id so the auto-sync is linked to its parent; the Phase 4b
  `tick_parent_if_any` machinery handles the rest.
  `scan_service::start_background_scan` passes None (pre-scan
  auto-sync runs before any HashResolve, so the DatSync appears
  as a top-level row).
- **Slice B — retry policy** (`8116c2a`). Plan §"Failure handling":
  3 attempts with 1s/5s/30s backoff on 5xx + network errors,
  invisible at warn-log level. Inline retry loop in
  `run_download_core_inner`'s initial GET. Cancel flag polled
  between attempts via new `sleep_with_cancel_check` helper so
  operator-triggered cancel during the 30 s backoff takes effect
  within 100 ms. Permanent 4xx (other than 416) + 2xx/206 break
  out immediately; only 5xx and network errors trigger retry.

660 of 660 oa-shell tests green.

**Scoped out:** Slice C (artwork_sync + hash_resolve resumers).
These operations need their inner logic refactored to attach to
an existing job_id at resume time rather than create a new one
(the snapshot only carries system_id, not the entries list each
function needs). That's a bigger refactor than fits Phase 4c.
Today's behavior is unchanged: interrupted rows for kinds without
registered resumers stay in the `interrupted` state and log a
warn at startup. Phase 5's Settings panel may surface a "clear
interrupted history" affordance.

**Almost:** End-to-end manual smoke test:
1. Trigger an "Identify ROMs" on a system with an empty
   rom_hashes table → bar shows TWO rows: parent "Identifying
   {system} ROMs" + visible child "Updating ROM database —
   {system} (prereq)." Child finalizes first; parent then proceeds.
2. Disconnect network mid-download → retry-policy log lines fire
   with the 1s/5s/30s sequence; on final attempt the row
   mark_failed with "after N attempts" suffix. Reconnect during
   the 30s window → the third attempt succeeds and the bar
   continues normally.
3. Cancel from the bar during a retry backoff sleep → cancel
   takes effect within 100ms instead of waiting for the sleep
   to complete.

**Next:** Phase 5 — Settings panel + Recent activity panel +
polish. Closes the arc as an operator-facing feature.

---

## 2026-06-02 — Phase 3b ships Range resume + opt-out infra + dup-trigger (`feat/background-jobs-phase-3b`)

**Shipped:** Phase 3b — the resume + UX polish half of the original
Phase 3 scope. Three phase commits.

- **Slice A — byte-level Range resume** (`f08c09d`). Refactors
  `run_download_core_inner` from "buffer the whole .zip in RAM,
  write to disk after" to "stream chunks through to .zip.partial
  as they arrive."
  - Two partial paths now: `<base>.dll.zip.partial` (streaming
    write), `<base>.dll.partial` (post-extract).
  - Existing .zip.partial → potential resume. Read size up front;
    if non-zero, GET with Range: bytes={size}-. Handle 206
    (partial content, append to existing), 200 (server ignored
    Range, truncate + restart), 416 (stale partial, drop + surface
    explicit error to operator).
  - tokio::fs::File via AsyncWriteExt. Flushes before pause spin /
    cancel return so kill-during-pause still resumes from the
    latest byte boundary.
  - CoreDownloadResumer no longer drops the .zip.partial up front
    (would defeat byte-level resume). Drops the .dll.partial only
    (a fresh extract runs after the Range-resumed zip lands).
- **Slice B — per-kind opt-out infrastructure** (`dc2ef06`). Plan
  §"Resume on app launch" auto-resume default + per-kind opt-out.
  - New `apps/oa-shell/src/job_prefs.rs` module. `JobPrefs` carries
    `prompt_before_resume_on_launch: HashMap<String, bool>` at
    `<data_dir>/library/job-prefs.json`. Read/write helpers
    mirroring library_prefs.rs.
  - Tauri commands `get_job_prefs` + `set_job_resume_prompt(kind,
    prompt)` — UI surface for these lands in Phase 5; for Phase 3b
    operators can hand-edit job-prefs.json.
  - JobEvent gains `ResumePrompt { snapshot }` variant.
  - `JobRegistry::resume_interrupted_jobs` signature now takes a
    `should_prompt: impl Fn(&str) -> bool`. When it returns true
    for a row's kind, the dispatcher emits ResumePrompt instead of
    calling the resumer. main.rs::setup() reads job-prefs.json and
    passes the closure.
- **Slice C — duplicate-trigger dialog** (`c236ce0`). Plan
  §"Duplicate same-job triggering" — Wait/Restart/Cancel collapses
  to a 2-option window.confirm for Phase 3b (Wait + Cancel are
  operationally identical at the call-site level). Phase 5 may
  upgrade to a richer 3-option Solid dialog.
  - JobRegistry.find_active_by_kind_target +
    `check_duplicate_job(kind, target_id)` Tauri command.
  - lib/backgroundJobs.ts gains
    `downloadCoreWithDuplicateCheck(base, parentJobId?)` helper
    that pre-flights check_duplicate_job → window.confirm if hit
    → cancelJob + 250ms wait + invoke download_core. Returns
    null when the operator chose Wait.
  - CoresPage migrated to the helper; MissingCoreBulkPrompt +
    SystemCoresStrip still call download_core directly (lower
    duplicate-trigger risk in those flows — can migrate later if
    operators surface dupes in real use).

660 of 660 oa-shell tests green; frontend `npm run typecheck`
silent.

**Almost:** End-to-end smoke test for the three new behaviors:
1. Range resume: trigger a core download → kill the app
   mid-stream → relaunch → log shows
   `resuming download of {base} ({N} bytes already on disk)`
   AND the download picks up from where it left off (faster
   second download = working). Range header in the request can
   be verified via Charles/Fiddler if needed.
2. Opt-out: hand-edit `<data_dir>/library/job-prefs.json` to add
   `{"promptBeforeResumeOnLaunch": {"core_download": true}}`,
   trigger a download, kill mid-stream, relaunch → log shows
   "emitting prompt for job N" instead of "dispatching resume."
   Row stays interrupted.
3. Duplicate trigger: start a download from Settings → Cores;
   while it's running, click Install again on the same row →
   confirm dialog appears with "Restart it? (Cancel keeps the
   current download running.)" copy.

**Next:** Phase 4c (dependency graph + retry policy) or Phase 5
(Settings panel + Recent activity panel + polish — closes the
arc as an operator-facing feature). Phase 3b's opt-out toggles
are the most likely Phase 5 work-item.

---

## 2026-06-02 — Phase 4b wires artwork_sync + bulk_core_install parent (`feat/background-jobs-phase-4b`)

**Shipped:** Phase 4b (scoped down from the original Phase 4
"remaining kinds + orchestration" to the two highest-value pieces;
dependency graph + per-kind retry policy queued for Phase 4c).
Two phase commits.

- **Slice A — artwork_sync wiring** (`e04430e`). New
  `ArtworkSync { system_id }` + `BulkCoreInstall` variants on
  `JobKind`. `sync_media_for_system` gains create_job +
  mark_running at entry, initial `progress(0, Some(total))` so the
  bar opens with a determinate 0% bar, per-repo boundary tick
  (1-3 advances per system; fine granularity per inner emit was
  overkill), final tick + mark_completed before return. Plan
  §"Kind taxonomy" originally split this into artwork_sync vs
  metadata_sync for per-kind concurrency; the existing function
  bundles both into one per-game-per-kind pass, so Phase 4b wires
  the whole pass as artwork_sync (26 of 27 MediaKind variants are
  art-shape) and defers the deeper split until a separate
  metadata-fetching path exists.
- **Slice B — bulk_core_install parent aggregation** (`b9e81a4`).
  Guided Setup's "Install N missing cores" batch now surfaces a
  parent row in the bar that aggregates the children.
  - JobRegistry.tick_parent_if_any(child_id) hooks into the three
    mark_* finalizers. When a child finalizes, the helper finds
    its parent_job_id, COUNTs finished siblings, emits an
    unthrottled progress event on the parent, and finalizes the
    parent when the last sibling resolves. Phase 4b treats any
    child failure as parent failure (mark_failed); Phase 4c retry
    policy will refine.
  - New `write_progress_unthrottled` helper — parents don't carry
    a JobHandle in the active map, so they bypass the per-handle
    1 Hz SQLite debounce.
  - New `start_bulk_core_install(n)` Tauri command creates the
    BulkCoreInstall parent (with `0 / n` initial progress) and
    returns the id. Soft-fail returns -1 when the registry isn't
    managed; the frontend treats that as "no parent" and
    downloads proceed individually.
  - `download_core` gains `parentJobId: Option<i64>` arg routed
    through to create_job's parent_job_id slot. Standalone
    callers (CoresPage, SystemCoresStrip) omit the field;
    Tauri's serde deserializes the missing field as None.
  - MissingCoreBulkPrompt.downloadAll invokes
    start_bulk_core_install up front and passes the returned id
    as parentJobId on each per-core download_core. Soft-fail
    silent-on-error.

660 of 660 oa-shell tests green; frontend `npm run typecheck`
silent.

**Almost:** End-to-end smoke test:
1. Settings → Library → identify ROMs → wait for the HashResolve
   pass to land; click "Sync media" → bar shows "Syncing {system}
   artwork" with done / total ticking per repo. Cancel-via-bar
   doesn't work yet (the per-repo loop doesn't poll the JobHandle
   cancel flag — Phase 4c can add it).
2. Import Wizard → "N missing cores" banner → "Install N cores"
   button → bar surfaces a parent "Installing N cores" row at
   0/N PLUS N child "Downloading {core}" rows. As each child
   finishes the parent's done ticks. When the last child resolves
   the parent finalizes (mark_completed if all succeeded, mark_failed
   if any child failed).
3. Standalone "Install core…" from CoresPage / SystemCoresStrip →
   single child row (no parent), behavior unchanged from Phase 1.

**Next:** Phase 4c — dependency graph (parent_job_id chain for
auto-prereqs so HashResolve auto-spawns DatSync as a visible
child rather than inlining it silently); per-kind retry policy
(transient network errors 1s/5s/30s exponential backoff for
3 attempts, refined by the Phase 5 Settings panel). Or Phase 3b /
Phase 5 as alternatives.

---

## 2026-06-02 — Phase 4a wires 4 more kinds (`feat/background-jobs-phase-4a`)

**Shipped:** Phase 4a (the first half of the original Phase 4 scope —
matching the Phase 3 split convention). Four phase commits.

- **Slice A — JobKind variants** (`4bbb40b`). New enum arms in
  `apps/oa-shell/src/job_registry.rs`: `FolderScan { folder }`,
  `HashResolve { system_id }`, `DatSync { system_id }`,
  `MameListxmlImport`. `discriminator` + `target_id` impls so the
  SQL row's `kind` + `target_id` columns get the right values per
  plan §"Kind taxonomy." The frontend `KIND_GLYPH` map already
  covers all nine plan kinds (seeded back in Phase 2 Slice C), so
  no frontend change.
- **Slice B — folder_scan wired** (`cdb17e0`).
  `scan_service::run_scan_blocking` + the inner `walk` now take
  `Option<&JobRegistry>` + `Option<i64>` and tick `progress(files_seen,
  None)` at every throttled emit point (~12 Hz; registry debounces to
  1 Hz internally). Total stays None because the file count is
  unknown until the walk finishes — the bar's indeterminate stripe
  kicks in. `main.rs::start_background_scan` registers the
  FolderScan job, uses the JobHandle's `cancel` AtomicBool as the
  shared cancel flag (bar cancel button + `cancel_background_scan`
  Tauri command both flip the same flag), and finalizes via
  mark_cancelled / mark_completed / mark_failed at the end of
  spawn_blocking.
- **Slice C — hash_resolve + dat_sync wired** (`0088ae4`). Both
  Tauri commands in `rom_hashes.rs`. DatSync is atomic — create_job
  + mark_running, mark_completed at each return point (success,
  empty-refs, cache-hit, 404-dat). HashResolve ticks
  `progress(done, Some(total))` after each per-game iteration's
  `done += 1`. Labels follow plan §"Kind taxonomy" — "Updating ROM
  database — {system_id}" + "Identifying {system_id} ROMs."
- **Slice D — mame_listxml_import wired** (`4d99382`). The
  `refresh_mame_system_info` Tauri command wraps the underlying
  sync `mame_import::refresh_mame_system_info` body. Atomic-retry
  kind: bar shows running → indeterminate stripe → done. On
  success, finalize emits a synthetic `progress(N, Some(N))` so
  the row's final state shows the total records refreshed.

660 of 660 oa-shell tests green.

**Almost:** End-to-end manual smoke test for the new surfaces:
1. Import wizard → start a folder scan → bar surfaces "Scanning
   {folder}" with files_seen ticking up. Cancel from the bar →
   walk stops at the next entry, row vanishes. Cancel from the
   wizard's existing button → same outcome (they share the
   AtomicBool now).
2. Settings → Library → identify ROMs button on a system →
   bar surfaces "Updating ROM database — {system}" briefly
   (DatSync) then "Identifying {system} ROMs" (HashResolve) with
   n_hashed / n_total ticking up smoothly.
3. Settings → MAME → Refresh MAME system info → bar surfaces
   "Refreshing MAME catalog" with indeterminate stripe; lands on
   done with the systems+games count.
4. Kill the app mid-scan → relaunch → log warns "no resumer
   registered for kind folder_scan" + the row stays interrupted
   (Phase 3b adds the resumers).

**Next:** Phase 4b — the remaining kinds + orchestration:
artwork_sync + metadata_sync (the giant sync_media_for_system
body), bulk_core_install with parent-row aggregation, the
dependency graph (parent_job_id chain + auto-trigger prereqs so
HashResolve auto-spawns DatSync as a visible child rather than
inlining it silently), per-kind retry policy. Then the bigger
Phase 4 stuff folds back into the arc plan.

---

## 2026-06-02 — Phase 3a ships the JobResumer + pause/resume bridge (`feat/background-jobs-phase-3a`)

**Shipped:** Phase 3a (the first half of the original Phase 3 scope,
split per the 2026-06-02 operator call to make the largest single
phase more manageable). Four phase commits.

- **Slice A — JobResumer trait + dispatcher** (`0a07a6c`). New
  trait in `apps/oa-shell/src/job_registry.rs` with `kind()` and
  `resume(snapshot, registry, app)` methods returning a
  `tauri::async_runtime::JoinHandle<()>`. JobRegistry stores
  resumers in `RwLock<HashMap<&'static str, Arc<dyn JobResumer>>>`.
  New methods: `register_resumer` (idempotent per-kind
  registration), `resume_interrupted_jobs(&app)` (iterates
  `list_interrupted()` + dispatches; rows whose kind has no
  resumer stay interrupted + log a warn), and `attach_handle`
  (re-creates the AtomicBool flags for a job whose row exists in
  SQLite but isn't in the active map — resumers call this BEFORE
  mark_running so the bar's pause + cancel buttons rebind to the
  resumed worker).
- **Slice B — pause/resume state bridge** (`8deb7e2`). Both
  `core_download` and `spawn_test_job` now call `mark_paused` when
  the pause flag flips true (entering the spin) and `mark_running`
  when it flips false (exiting). A `was_paused` flag gates both
  calls so a momentary check doesn't fire spurious transitions.
  Fixes the operator-reported "pause stops streaming but resume
  button never appears" issue from Phase 2 smoke-test — the row
  state now goes running → paused → running and the bar's per-row
  button toggles ⏸ ↔ ▶ correctly.
- **Slice C — CoreDownloadResumer + inner-fn refactor** (`fb2b290`).
  Extracts the ~200-line download/extract/install body of
  `download_core` into a shared `run_download_core_inner` so both
  the Tauri command AND the resumer share one code path. New
  `CoreDownloadResumer` impl reads `target_id` from the snapshot,
  drops any leftover `.partial`, attaches a fresh handle, calls
  `mark_running`, runs the inner, finalizes via the same
  mark_completed / mark_cancelled (+ .partial cleanup) / mark_failed
  shape as `download_core`'s tail. Phase 3a strategy is
  **restart-from-zero**: the current chunk loop buffers the entire
  .zip in RAM before writing .partial, so byte-level Range resume
  needs a streaming-write refactor (queued for Phase 3b). Cores are
  <10 MB; the re-download cost is acceptable for now.
- **Slice D — register resumer + dispatch in setup()** (`3a4e4dc`).
  In `main.rs::setup()` right after `promote_running_rows_to_interrupted`
  and before `app.manage(registry)`: register the
  `CoreDownloadResumer` (constructed with `resolve_cores_dir()`),
  then call `registry.resume_interrupted_jobs(&app_handle)`. The
  dispatch returns immediately because each per-kind resumer spawns
  its own `tauri::async_runtime` worker.

End-to-end crash-recovery path now works:
1. App crashes mid-download → `<data_dir>/oa.lock` stays behind.
2. Next launch: lock file detected → `running` rows promoted to
   `interrupted`.
3. `resume_interrupted_jobs` dispatches the `CoreDownloadResumer`
   for each interrupted `core_download` row.
4. Resumer drops `.partial`, attaches handle, marks running,
   re-runs the full inner flow from the buildbot URL, finalizes.
5. Bar surfaces the resumed download as soon as the StateChanged
   event for `mark_running` fires.

660 of 660 oa-shell tests green.

**Almost:** End-to-end manual smoke test for the new surfaces:
1. Spawn a test job → pause → confirm the row state pill flips to
   "Paused" + the per-row button shows ▶ resume. Click resume →
   pill clears, button returns to ⏸, ticks continue.
2. Start a real `core_download` → kill the app (Task Manager →
   End Task on `oa-shell.exe`) → relaunch → confirm the log shows
   `dispatched 1 resume worker(s) for interrupted rows` and the
   download restarts from zero (bar shows progress climbing
   again).
3. Spawn a test job → kill the app → relaunch → confirm the log
   warns `no resumer registered for kind test_job; job N stays
   interrupted`. (Phase 3a only resumes core_download.)

**Next:** Phase 3b — byte-level Range resume for `core_download`
(streaming-write refactor + HTTP Range request when `.partial`
exists); `artwork_sync` + `hash_resolve` resumers; per-kind
opt-out infrastructure (settings.json fields, Settings panel UI
stays in Phase 5); duplicate-trigger Wait/Restart/Cancel dialog
for second-click-while-running on the same kind+target. See plan
§Sizing Phase 3.

---

## 2026-06-02 — Phase 2 ships the BackgroundJobsBar (`feat/background-jobs-phase-2`)

**Shipped:** Phase 2 of the 5-phase arc landed in four phase commits
on `feat/background-jobs-phase-2`, pending operator end-to-end smoke
test before merge.

- **Slice A — Tauri commands for the bar** (`cff3dbc`). New
  JobRegistry methods (`signal_pause` / `signal_cancel` /
  `signal_pause_all` / `signal_cancel_all`) flipping the
  AtomicBool flags on the active-map handles. New
  `#[tauri::command]` wrappers: `list_active_jobs`, `list_recent_jobs`,
  `pause_job`, `resume_job`, `cancel_job`, `pause_all_jobs`,
  `cancel_all_jobs`. All take AppHandle + `try_state` lookup so they
  soft-fail when the registry isn't managed (matches the Phase 1
  graceful-degradation shape).
- **Slice B — frontend backgroundJobs store** (`bcd1498`). New
  `frontend/src/lib/backgroundJobs.ts` mirroring the Rust JobState /
  JobSnapshot / JobEvent types. Module-level `activeJobs()` signal
  hydrates from `list_active_jobs` and stays in sync via
  `oa://job-event`. Race-safe ordering — listener attaches BEFORE
  the hydrate invoke so a Created event fired between them is
  queued and replayed. Mutation helpers (pauseJob / resumeJob /
  cancelJob / pauseAllJobs / cancelAllJobs) are best-effort
  silent-on-error.
- **Slice C — BackgroundJobsBar Solid component** (`2944c84`). New
  `frontend/src/components/background-jobs/BackgroundJobsBar.tsx`
  (~370 LOC) with the Hidden / HandleVisible / Expanded state
  machine, max-3-rows + "+N more" stack layout, per-row kind icon +
  label + state pill + done/total numbers + pause / cancel buttons,
  header with Pause-all / Cancel-all (3+ jobs gets an extra confirm
  per plan §"Bar header"; cancels always confirm because they're
  destructive). 2 s bar-pointer-idle auto-collapse from Expanded
  back to HandleVisible. Inline @keyframes for the handle's pulse
  dot keyed off an activeJobs subscription so it re-animates on
  every Progressed event. Fixed-position overlay at z-30 (below
  HintBar's z-40 per plan §"HintBar takes priority").
- **Slice D — mount in App.tsx** (`2765b9c`). Inserted between
  ToastStack and HintBar in the App.tsx return tree. Reachable from
  any Retroverse tab + the legacy fallback paths because App.tsx
  mounts above the RetroverseShell vs Shell branch. Component
  returns null when no active jobs, so the always-mounted cost is
  near-zero when nothing is happening.

660 of 660 oa-shell tests green; frontend `npm run typecheck` silent.

**Almost:** End-to-end manual smoke test still pending operator
validation:
1. Trigger a core download → bar's handle appears at the bottom of
   the viewport with the job count + pulse dot. Click the handle →
   bar expands showing the one row with label, MB/MB progress
   numbers, percentage bar, pause + cancel buttons.
2. Click pause → the row's progress numbers freeze (chunk loop
   spins on the flag). Click resume → progress resumes. Phase 1
   caveat: the row state pill stays empty (i.e. "running") rather
   than showing "Paused" because core_download doesn't yet bridge
   the flag back to `mark_paused`. This is documented in the tooltip
   strings + lands properly in Phase 3.
3. Click cancel → confirm dialog → cancel applies, row vanishes
   from the bar within ~100 ms, `.partial` cleanup runs per the
   per-kind contract.
4. Wait 2 s without hovering the bar → it auto-collapses to the
   handle. Click handle → expands again.
5. Start 2+ downloads → header gains Pause-all / Cancel-all
   buttons. 3+ → both actions confirm before applying.

**Next:** Phase 3 — `JobResumer` trait + per-kind handlers for
`core_download` / `artwork_sync` / `hash_resolve` + auto-resume-on-
launch flow with per-kind opt-out + duplicate-trigger
Wait/Restart/Cancel dialog. Most importantly: wire `core_download`'s
pause spin to flush its resume payload + call `mark_paused` so the
operator's pause click produces a visible state change. See plan
§Sizing Phase 3 for the ~1.5-week scope.

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
