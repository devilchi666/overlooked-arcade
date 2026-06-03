# Active Work Streams

Free-form list of what's in flight. Read the linked stream's README + recent
SESSION_LOG entry to pick up where the last session left off.

Replaces the older `docs/ACTIVE_CORE.md` (single-string "which core is active")
because cross-cutting work didn't fit that model — the 2026-05-22 sidebar work
spanned every system but was filed under whichever core happened to be active.

---

## In flight

- **Retroverse UI rollout** — all six top-toolbar tabs operator-
  facing with real bodies. 2026-05-28 shipped Phases A-C4 + HOME v2
  + SETTINGS expansion; 2026-05-29 closed the unified controller
  pipeline + menu/dialog polish + Slice 12 custom collections +
  Per-system SETTINGS drill-in + Now-playing chip + DISCOVER body.
  Toggle Settings → Display → Experimental → Retroverse UI ON to
  enter; flag OFF stays byte-identical with the legacy Shell apart
  from the heart overlay on tiles + the custom-collections submenu
  in TileContextMenu (Retroverse-only).

  **Current state (2026-05-29 end of day):**
  - HOME — v2 dense mockup (system spotlight + carousel arrows +
    dot pagination + Recently Played panel; right pane = SYSTEM
    INFORMATION / TECHNICAL DETAILS / PERIPHERALS / ACHIEVEMENTS).
  - LIBRARY — header card + system-label tile headers; reuses
    LeftSidebar + VirtualLibraryGrid + GameDetailPanel.
  - COLLECTIONS — 3-pane; ALL 6 smart-lists wired (Favorites /
    Recently played / Completed / Multi-player / Hidden gems /
    Last played) PLUS Slice 12 custom collections (create / rename
    / delete / membership submenu in TileContextMenu).
  - PLAY NOW — hero + WHY-line + 3 rails + 9-mood sidebar (For
    you / Continue / With a friend / Nostalgia / Quick / Marathon
    / Challenge / Surprise me / Daily roulette with UTC-day lock).
  - DISCOVER — 3-pane with 4 data-driven axes (By era / By genre /
    By publisher / By developer) reading from `useMedia().media(
    romId)?.metadata`; 5 editorial axes (Featured / On this day /
    System dive / Cult classics / Lost games) render empty-state
    cards pointing at Phase C6 content-packs.
  - SETTINGS — ALL 15 top-level categories have real bodies PLUS
    the Per-system drill-in (sidebar group expands to 45-system
    picker; center pane composes Display / Rewind / Shaders /
    Default core inline + Bindings / Core options launchers).
    Section bodies shared with legacy SystemSettingsDialog via
    `components/perSystemSections.tsx`.
  - Now-playing chip in HintBar shows current platform-music
    system with animated equalizer bars.
  - Operator-locked controller-nav: L1/R1 cycle tabs; DPad
    LEFT/RIGHT transfers regions; stick walks within (LIBRARY
    sidebar containers expand/collapse on stick L/R via
    source-gated `onDirection`). See
    [features/retroverse-ui/DECISIONS.md](features/retroverse-ui/DECISIONS.md).
  - 506 oa-shell tests green; frontend `npm run typecheck` silent.

  **Genuinely open work** (full §10 list in
  [PLANS/retroverse-ui-rollout.md](PLANS/retroverse-ui-rollout.md)
  §10, audited 2026-05-29):
  - Phase C6 — content-packs infrastructure (substantial; unlocks
    DISCOVER's 5 stub axes + curated COLLECTIONS + theme packs).
  - RetroAchievements integration OR local milestone tracking
    (HOME ACHIEVEMENTS card + GameDetailPanel / SystemInfoPanel
    sections are placeholders).
  - Per-System UI Stage 2 + Stage 3 — separate plan.
  - Flag deprecation endpoint — eventual.

  **Content workstream (operator-side):** per-system hero art (drop
  console + fanart into existing PlatformMedia slots),
  `systemMetadataStubs.ts` refinement for ~38 systems beyond the 7
  priority stubs, per-system blurbs.

- **ColecoVision keypad reference + GameCube Wii peripherals** —
  branch `feat/coleco-keypad-and-gamecube-wii`, two commits.
  - **Coleco visual keypad reference** (`c4746d9`): new
    `KeypadReference` component renders the physical 3×4 Coleco
    controller keypad in the per-game Input dialog with each KP
    labeled by its current per-system keyboard / gamepad mapping.
    Bridges "keypad_layout_note says KP1" to "physical key 'Q'
    fires KP1." Visible only for Coleco today; Intv shares the 3×4
    shape and can adopt later. Per-game bindings override stays
    ⬜ stretch (`coleco/ROADMAP.md:36`); current design keeps
    bindings system-wide per the `GameOverrides.keypad_layout_note`
    doc comment.
  - **GameCube Wii peripheral subclasses**: un-deferred after
    Dolphin libretro source research confirmed the 5 selectable
    SUBCLASS values from `Source/Core/DolphinLibretro/Input.cpp:48-54`.
    New `DEVICE_ID_OPTIONS_GAMECUBE` table in `GameDialogs.tsx`
    adds Wii Remote (sideways) / + Nunchuk / + Classic Controller /
    + Classic Pro / GC Controller (Wii mode). `deviceOptionsForSystem`
    helper gates them so they only show in the dropdown for
    GameCube system games. Per-game hint block names the Wii titles
    each peripheral is for. No Rust changes — `arm_libretro_device`
    already dispatches arbitrary u32s. `library_db.rs::GameOverrides::libretro_device`
    doc comment extended with the Dolphin subclass table.
    `gamecube/ROADMAP.md:44` flipped ⬜→✅.

  **Dreamcast VMU peripheral** — still deferred to its own run
  (Phase 2.5 secondary-screen plumbing).

- **System fixes pass — MAME / light-gun IS_OFFSCREEN / Saturn 3D Pad +
  Atari 7800 twin-stick labels / NDS stylus reticle** — branch
  `feat/system-fixes-mame-lightgun-analog`, five commits closing a
  handful of small-to-medium code gaps across the per-core ROADMAPs:
  - **MAME Phase 1.5 clarification**: ROADMAP line 30 reframed from
    "verify dispatch" ⬜ to clarified ✅ — MAME consumes Service /
    Tab / P2 inputs via the keyboard-passthrough pump, not the
    RetroPad bits the four MAME_BUTTONS entries map to. RetroPad
    mappings stay as future-proofing; no code change.
  - **Light-gun IS_OFFSCREEN flag (`696dc87`)**: real code gap. New
    `in_viewport` field on `InputState.pointer` flows from
    `InputPoller::poll_pointer` → `cb_input_state` →
    `lightgun_field_value` → `RETRO_DEVICE_ID_LIGHTGUN_IS_OFFSCREEN`.
    Shoot-off-screen-to-reload now works across all 6 light-gun
    systems (NES Zapper, SMS Phaser, Saturn Virtua Gun, PS1 GunCon
    / Justifier, Dreamcast HotD / Confidential Mission, Atari 7800
    XEGS Light Gun). Closes the 2026-05-25 light-gun arc's
    remaining ⬜ note; 577 workspace tests green.
  - **System-specific device-type labels (`5e60ae6`)**: cosmetic
    polish — the per-game Input dialog dropdown now shows
    "3D Pad / Analog" for Saturn, "Virtua Gun" for Saturn/STV
    light-gun device, "Zapper" for NES, "Super Scope" for SNES,
    "DualShock / Analog" + "GunCon / Justifier" for PSX, etc.
    Plus help-text blocks in the Additional Ports section for
    atari7800 (Robotron twin-stick recipe) + saturn / stv
    (3D Pad setup). Underlying dispatch unchanged.
  - **NDS stylus reticle (`9d04815`)**: visual cursor + tap
    feedback overlay. Hollow accent-colored ring follows the OS
    cursor while an NDS game is running; fills in + scales down
    on left-mouse-down so the operator sees explicit stylus-tap
    feedback the OS cursor doesn't provide. Per-game touch-area
    hotspots remain ⬜ as a separate larger feature.

  Per-core ROADMAP flips (saturn / atari7800 / nds / dreamcast / psx /
  mame) reflect each of these closures. NEXT.md cross-system
  inventory updated to record the in_viewport plumbing.

- **Three new systems — jagcd / sega32xcd / stv** — merged to
  main 2026-05-27 (`--no-ff` from `feat/new-systems-jagcd-32xcd-stv`,
  merge `189c448`). Three phase commits + a docs commit lifting
  all three from the `docs/NEXT.md` DEFERRED band.
  - **jagcd** (`5ec0a57`): new oa-core `SystemId::JaguarCd`
    variant + frontend slug + theme + BIOS check (jagboot.rom +
    jagcd.rom) + bindings shared with cart Jaguar + Atari_-_Jaguar_CD
    thumbnails repo.
  - **sega32xcd** (`de4335b`): new frontend slug routing to
    `oa_core::SystemId::SegaCd` (stacked-override pattern, no new
    Rust variant). Default core swapped to PicoDrive — the only
    libretro core with 32X+CD combined-mode support. BIOS check
    reuses `check_sega_cd_bios`.
  - **stv** (`f700f64`): pure alias slug routing to
    `oa_core::SystemId::Mame`. MAME's stv driver handles BIOS
    lookup + ROM-set loading internally; no separate BIOS check
    function in OA. Bindings + media sync share MAME's.

  Phase 0 (slug wiring) is ✅ for all three. Phase 1 (operator
  playtest) is ⬜ for all three — operator needs to legally
  acquire BIOSes + ROMs first. Per-core ROADMAPs flip Phase 1 ✅
  as each is validated end-to-end.

- **Per-System Custom UI Stage 1 — code arc complete; content-side
  pause** ([features/per-system-ui/](features/per-system-ui/)).
  Slices 1-5 merged to main 2026-05-26 / 2026-05-27: the foundation
  + the four consumer-side mechanisms (per-system SFX wiring,
  background renderer, boot animation framework, tile flourish
  system). Master toggle ON gives every system a visibly distinct
  feel via the registry alone — operator playtested across the
  Stage 1 pilots and confirmed the per-system differences read.
  Remaining slices 6-9 are content-heavy: GB / NES / Vectrex
  full pilot builds (SFX recordings, background assets, boot
  animation keyframes, plus a Vectrex custom-component escape
  hatch) + per-core README "Per-system UI" sections. Held pending
  operator content production (CC0 audio curation, DMG gradient,
  AI-generated Vectrex vector blips, etc. — see plan §9 for
  sourcing strategy). Resumes when operator green-lights with
  content in hand. See
  [features/per-system-ui/ROADMAP.md](features/per-system-ui/ROADMAP.md)
  for the slice breakdown and
  [features/per-system-ui/ASSETS.md](features/per-system-ui/ASSETS.md)
  for the operator-facing asset catalog (where every sound /
  background / boot animation file goes on disk).

## Recently completed (this session)

- **SETTINGS declutter — System Health hub + Game-media cards** —
  merged to main 2026-06-03 (`--no-ff` from
  `feat/settings-declutter-system-health`, merge `dd430e4`). Five
  phase commits + two polish commits + two docs commits closing the
  operator's 2026-06-03 ask end-to-end.
  - **Phase 1** (`21d803d`) — new System Health category absorbs
    BIOS / Cores / Storage / Background Jobs as internal tabs;
    sidebar shrinks 16 → 12 entries.
  - **Phase 2** (`9bfbc96`) — Overview rollup cards (5 status rows
    with green/amber/red dots + CTAs deep-linking into tabs) +
    lifted readiness checklist.
  - **Phase 3** (`cb33089`) — dev test spawner relocated from
    Settings → Library into a collapsed disclosure in
    Experimental → Dev tools.
  - **Phase 4** (`910ba27`) — Game-media tab refactored into a
    status-first card grid (alphabetical, 3-col grid, per-card
    Freshen smart-runner) + Preferences card + top-right
    [Freshen all systems] CTA.
  - **Phase 5** (`e087caf`) — `GameMediaManagePanel` side panel
    surfaces the 5 granular ops (Identify / Sync covers / Sync
    metadata / Clear metadata / Refresh hash DB) when the operator
    clicks [Manage…] on a system card.
  - **Polish** (`3d5f6c5`) — Readiness rollup card lists incomplete
    system names ("Dreamcast, NES, PS1 + 2 more need attention") +
    Jump-to-details CTA that smooth-scrolls to the per-system grid.
  - **Polish** (`cbb249b`) — per-system readiness rows alphabetical;
    BIOS status splits into two sections (⚠ Missing & issues / ✓
    Ready), each alphabetical.
  - 660 of 660 oa-shell tests green at branch tip. Frontend
    `npm run typecheck` silent across every phase. Operator
    playtest signed off before merge.
  - Plan: [PLANS/settings-declutter-system-health.md](PLANS/settings-declutter-system-health.md).

- **Background jobs + persistent progress bar — polish (always-show + chime + ResumePromptDialog + ASSETS.md)**
  ([features/background-jobs/](features/background-jobs/)) — merged to
  main 2026-06-03 (`--no-ff` from `feat/background-jobs-polish`).
  Five phase commits closing the three items Phase 5 deferred.
  - **Polish A** (`79e9248`) JobPrefs gains `sound_on_completion`
    (default true) + `always_show_bar` (default false) +
    explicit Default impl + two new Tauri commands.
  - **Polish B** (`0f746dd`) `resolve_completion_chime` Tauri
    command resolves `<exe_dir>/assets/oa-ui/sounds/job-complete.<ext>`.
    Frontend `maybePlayCompletionChime` fires on Completed
    events when the toggle is on; silent fallback when asset
    missing. New `docs/features/background-jobs/ASSETS.md`
    documents exact path + supported formats + silent-fallback
    semantics for operators sourcing the chime.
  - **Polish C** (`6555f56`) `BackgroundJobsBar` reads
    `jobPrefs.alwaysShowBar`; ON keeps handle visible with "No
    active jobs" label. Settings → Background Jobs → Bar
    behavior card unstubbed and wired through the new commands;
    each handler calls `refreshJobPrefs()` so the live signal
    syncs and the bar reacts immediately.
  - **Polish D** (`33be3fe`) `resume_one_interrupted_job(job_id)`
    Tauri command + new `ResumePromptDialog.tsx`. Closes the
    Phase 3b opt-out loop end-to-end: tick the per-kind toggle
    → kill app mid-op → relaunch → dialog surfaces with Resume
    (re-enters via the registered resumer) or Discard
    (cancel_job). Mounted in App.tsx at z-[70].
  - 660 of 660 oa-shell tests green. Operator-confirmed all
    three behaviors (always-show, chime toggle path, ResumePrompt
    flow) before merge.

- **Background jobs + persistent progress bar — Phase 5 (arc-close: Settings + Recent activity + final resumers)**
  ([features/background-jobs/](features/background-jobs/)) — merged to
  main 2026-06-03 (`--no-ff` from `feat/background-jobs-phase-5`).
  Four phase commits closing the 7-phase arc as a complete
  operator-facing feature.
  - **Slice A** (`a827c86`) Generic `ReinvokeOperatorResumer`
    registered for the six remaining kinds (artwork_sync,
    hash_resolve, dat_sync, folder_scan, mame_listxml_import,
    bulk_core_install). mark_cancelled the orphan interrupted row
    + log re-trigger path. Internally-idempotent operations make
    operator re-trigger functionally equivalent to resume.
    core_download keeps its Phase 3a/3b byte-level Range resume.
  - **Slice B** (`7c97639`) New "Background Jobs" Settings
    category (glyph ⟳). Per-kind auto-resume toggles (Phase 3b
    backend) + bar behavior stubs (Phase 6 polish target) +
    failure-handling summary + history counter + `clear_job_history`
    Tauri command + button.
  - **Slice C** (`ef543c0`) `RecentActivityPanel.tsx` — full-
    viewport overlay (z-65) tabbed by outcome. Triggered from a
    new "Recent activity →" link in the BackgroundJobsBar's
    expanded header. Reads activeJobs() live for the Running tab;
    invokes list_recent_jobs for finished tabs. Escape +
    backdrop close.
  - **Wrap** (`e32ec65`) SESSION_LOG arc-close entry.
  - 660 of 660 oa-shell tests green. Operator-confirmed all four
    surfaces (Settings, Recent activity panel, resumer log lines,
    clear-history button) before merge.

  **Arc summary** — seven phases shipped end-to-end in two
  consecutive sessions:
  - Phase 1 (backend pilot) / Phase 2 (BackgroundJobsBar + dev
    affordance) / Phase 3a (JobResumer + pause bridge + crash
    recovery) / Phase 3b (Range resume + opt-out + dup-trigger) /
    Phase 4a (folder/hash/dat/MAME) / Phase 4b (artwork +
    bulk-install parent + z-fix) / Phase 4c (deps graph + retry
    policy) / Phase 5 (final resumers + Settings + Recent
    activity).
  - Bar surfaces every long-running operation operators can
    trigger; pause/resume + cancel work end-to-end; byte-level
    Range resume survives crashes for core_download; per-kind
    opt-out gives operators control over auto-resume; recent
    activity + Settings → Background Jobs give the operator a
    permanent home for everything OA does behind the scenes.
  - Plan §"Out of scope" items remain deferred (cross-machine
    sync, scheduled cron-style work, per-kind completion chime
    variants). Phase 6 polish targets: always-show bar toggle,
    completion chime asset + wiring, ResumePrompt dialog
    surfacing.

- **Background jobs + persistent progress bar — Phase 4c (dependency graph + retry policy)**
  ([features/background-jobs/](features/background-jobs/)) — merged to
  main 2026-06-03 (`--no-ff` from `feat/background-jobs-phase-4c`).
  Three phase commits implementing two of the three originally-
  planned Phase 4c pieces; the third (artwork/hash resumers) scoped
  out as a follow-up because the inner operation functions need
  refactoring to attach to an existing job_id at resume time.
  - **Slice A** (`9713d7d`) Dependency graph —
    `auto_sync_rom_hashes_if_empty` gains a `parent_job_id`
    argument. When called from `resolve_rom_hashes_for_system` it
    registers a visible DatSync prereq row with `is_prereq=true` +
    " (prereq)" label suffix; Phase 4b's `tick_parent_if_any`
    handles the parent updates as the prereq finalizes.
    `scan_service::start_background_scan` passes None (no parent —
    DatSync appears top-level for pre-scan auto-sync).
  - **Slice B** (`8116c2a`) Retry policy — Plan §"Failure handling"
    3 attempts with 1s/5s/30s backoff on 5xx + network errors in
    core_download's GET. Inline retry loop in
    `run_download_core_inner`. New `sleep_with_cancel_check`
    helper polls the cancel flag every 100 ms during backoff so
    operator-triggered cancel during the 30 s sleep takes effect
    within 100 ms instead of waiting for the sleep.
  - **Scoped out** — Slice C (artwork_sync + hash_resolve
    resumers). Snapshot carries only system_id, but
    sync_media_for_system + resolve_rom_hashes_for_system both
    need entries lists and create their own job_id on each call;
    a real resume requires extracting the inner logic into
    `*_with_existing_job_id` variants. Deferred to Phase 5 or
    a separate follow-up.
  - 660 of 660 oa-shell tests green. Operator-confirmed smoke test
    of the prereq + retry behavior before merge.

- **Background jobs + persistent progress bar — Phase 3b (Range resume + opt-out + dup-trigger)**
  ([features/background-jobs/](features/background-jobs/)) — merged to
  main 2026-06-03 (`--no-ff` from `feat/background-jobs-phase-3b`).
  Four phase commits implementing the second half of the original
  Phase 3 scope (Phase 3a was the trait + pause bridge).
  - **Slice A** (`f08c09d`) Byte-level Range resume —
    `run_download_core_inner` refactored from "buffer the whole .zip
    in RAM" to "stream chunks through to `<base>.dll.zip.partial`
    via `tokio::fs::File::write_all` as they arrive." On restart,
    the partial's size tells us the resume point; we send
    `Range: bytes={size}-` and append. Handles 206 (resume), 200
    (server ignored Range — truncate + restart), 416 (stale partial
    — surface error). Flushes before pause spin / cancel return so
    kill-during-pause still resumes from a valid byte boundary.
    CoreDownloadResumer no longer drops the .zip.partial up front
    (would defeat byte-level resume); drops the .dll.partial only.
  - **Slice B** (`dc2ef06`) Per-kind opt-out infrastructure —
    plan §"Resume on app launch" auto-resume default + per-kind
    opt-out. New `apps/oa-shell/src/job_prefs.rs` module with
    `JobPrefs.prompt_before_resume_on_launch: HashMap<String, bool>`
    at `<data_dir>/library/job-prefs.json`. Tauri commands
    `get_job_prefs` + `set_job_resume_prompt(kind, prompt)`. New
    `JobEvent::ResumePrompt { snapshot }` variant fires when a
    kind has opt-out set. `JobRegistry::resume_interrupted_jobs`
    signature gains a `should_prompt: impl Fn(&str) -> bool` arg;
    main.rs::setup() reads job-prefs.json and threads the closure
    through. UI surface for the toggles lands in Phase 5.
  - **Slice C** (`c236ce0`) Duplicate-trigger dialog — plan
    §"Duplicate same-job triggering" 3-option Wait/Restart/Cancel
    collapses to a 2-option window.confirm (Wait + Cancel are
    operationally identical at the call-site level). New
    `JobRegistry::find_active_by_kind_target` + `check_duplicate_job`
    Tauri command. Frontend helper
    `downloadCoreWithDuplicateCheck(base, parentJobId?)` in
    lib/backgroundJobs.ts pre-flights the check → window.confirm →
    cancelJob + 250 ms wait + retry on Restart. CoresPage migrated.
  - 660 of 660 oa-shell tests green. Operator smoke-tested before
    merge.

- **Background jobs + persistent progress bar — Phase 4b (artwork_sync + bulk parent + z-fix)**
  ([features/background-jobs/](features/background-jobs/)) — merged to
  main 2026-06-02 (`--no-ff` from `feat/background-jobs-phase-4b`).
  Three phase commits scoping Phase 4 down to artwork_sync wiring +
  bulk_core_install parent aggregation + a z-index fix that
  unblocked the bar across modal surfaces.
  - **Slice A** (`e04430e`) `ArtworkSync { system_id }` +
    `BulkCoreInstall` JobKind variants. `sync_media_for_system`
    registers + ticks per-repo boundary + finalizes at end. Locked
    artwork-vs-metadata split deferred (existing function bundles
    both; needs a separate metadata-fetching path before the split
    is meaningful).
  - **Slice B** (`b9e81a4`) `JobRegistry::tick_parent_if_any` +
    `write_progress_unthrottled` + new `start_bulk_core_install(n)`
    Tauri command. download_core gains `parentJobId: Option<i64>`
    routed through to create_job. MissingCoreBulkPrompt invokes the
    new command up front and threads the parent id into each per-
    core invocation. When the last child finalizes, the parent
    auto-finalizes (mark_completed if all succeeded, mark_failed
    if any child failed/cancelled — Phase 4c retry policy will
    refine).
  - **z-index fix** (`b0c4036`) Bar 30→55, HintBar 40→60. Modals
    at z-50 were covering both, hiding the bar across every
    operation-kickoff surface (Import Wizard, MissingCoreBulkPrompt,
    Settings dialogs). Plan §"Hybrid coexistence" locks the bar
    must stay visible alongside modals; this restores that.
  - 660 of 660 oa-shell tests green. Operator-confirmed smoke test
    on the post-fix build.

- **Background jobs + persistent progress bar — Phase 4a (4 more kinds wired)**
  ([features/background-jobs/](features/background-jobs/)) — merged to
  main 2026-06-02 (`--no-ff` from `feat/background-jobs-phase-4a`).
  Five phase commits surfacing four more operations in the
  BackgroundJobsBar — every operator-triggered long-running
  operation that was already firing its own `oa://*-progress` events
  now ALSO appears in the bottom-of-viewport bar.
  - **Slice A** (`4bbb40b`) `JobKind` enum variants:
    `FolderScan { folder }`, `HashResolve { system_id }`,
    `DatSync { system_id }`, `MameListxmlImport`.
  - **Slice B** (`cdb17e0`) folder_scan via `scan_service.rs` +
    `main.rs::start_background_scan`. JobHandle.cancel
    AtomicBool shared with the existing
    `scan_service::ScanServiceState` so bar Cancel + the existing
    `cancel_background_scan` Tauri command flip the same flag.
    Total stays None (file count unknown until walk completes); bar
    shows indeterminate stripe.
  - **Slice C** (`0088ae4`) hash_resolve + dat_sync in
    `rom_hashes.rs`. HashResolve ticks per game with the smooth
    n_hashed / n_total fill; DatSync is atomic with finalize at
    each return point (success, empty-refs, cache-hit, 404-dat).
  - **Slice D** (`4d99382`) mame_listxml_import at the Tauri
    command wrapper around the sync `mame_import::refresh_mame_system_info`
    body. Atomic-retry kind with a synthetic final tick showing
    systems_refreshed + games_refreshed count.
  - 660 of 660 oa-shell tests green. Operator confirmed all four
    kinds surface end-to-end after rebuild.

- **Background jobs + persistent progress bar — Phase 3a (JobResumer + pause bridge)**
  ([features/background-jobs/](features/background-jobs/)) — merged to
  main 2026-06-02 (`--no-ff` from `feat/background-jobs-phase-3a`).
  Five phase commits implementing the first half of the original
  Phase 3 scope, split per operator call to keep the biggest single
  phase manageable.
  - **Slice A** (`0a07a6c`) `JobResumer` trait + JobRegistry
    plumbing — `register_resumer`, `resume_interrupted_jobs(&app)`
    dispatcher, `attach_handle(job_id, kind)` for re-creating the
    AtomicBool flags on a row that came back as `interrupted`.
  - **Slice B** (`8deb7e2`) pause/resume state bridge in both
    `core_download` and `spawn_test_job` chunk loops. `was_paused`
    flag drives `mark_paused` on spin entry + `mark_running` on
    spin exit, so the bar's per-row button correctly toggles
    ⏸ ↔ ▶ — closes the operator-reported "pause stops streaming
    but resume button never appears" issue from Phase 2.
  - **Slice C** (`fb2b290`) extracted the ~200-line
    download/extract/install body into `run_download_core_inner`
    shared by both the `download_core` Tauri command and the new
    `CoreDownloadResumer`. Phase 3a resumer strategy is
    restart-from-zero (drops `.partial`, re-runs from buildbot
    URL); byte-level Range resume needs a streaming-write
    refactor queued for Phase 3b.
  - **Slice D** (`3a4e4dc`) wired registration + dispatch in
    `main.rs::setup()` right after `promote_running_rows_to_interrupted`.
    End-to-end crash-recovery path now works: kill the app
    mid-download → relaunch → `dispatched 1 resume worker(s)`
    log line → download restarts from scratch and the bar
    surfaces the resumed row as soon as `mark_running` fires.
  - 660 of 660 oa-shell tests green. Operator smoke-tested the
    pause/resume toggle + crash-recovery dispatch before merge.

- **Background jobs + persistent progress bar — Phase 2 (BackgroundJobsBar)**
  ([features/background-jobs/](features/background-jobs/)) — merged to
  main 2026-06-02 (`--no-ff` from `feat/background-jobs-phase-2`).
  Five phase commits + a DOM-stability fix + a dev test affordance
  shipping the persistent UI surface on top of Phase 1's backend.
  - **Slice A** (`cff3dbc`) Tauri commands: `list_active_jobs`,
    `list_recent_jobs`, `pause_job`, `resume_job`, `cancel_job`,
    `pause_all_jobs`, `cancel_all_jobs`. JobRegistry gains
    `signal_pause` / `signal_cancel` / `signal_pause_all` /
    `signal_cancel_all` to flip the AtomicBool flags from outside
    the worker. All commands soft-fail through `try_state` lookup.
  - **Slice B** (`bcd1498`) `frontend/src/lib/backgroundJobs.ts` —
    module-level reactive store mirroring JobState / JobSnapshot /
    JobEvent. Race-safe hydration (listener attaches before the
    `list_active_jobs` invoke; events queue until hydration lands).
    Mutation helpers wrapping the Slice A commands.
  - **Slice C** (`2944c84`) `BackgroundJobsBar.tsx` (~370 LOC) —
    Hidden / HandleVisible / Expanded state machine, max-3-rows +
    "+N more", per-row controls (pause / cancel + status pill +
    formatted done/total), header with Pause-all / Cancel-all
    (confirm when 3+ jobs active; cancels always confirm because
    destructive). 2 s bar-pointer-idle auto-collapse. Inline
    @keyframes pulse dot.
  - **Slice D** (`2765b9c`) Mounted in App.tsx between ToastStack and
    HintBar — z-30 vs HintBar's z-40 so mid-modal hint contexts win.
  - **Dev affordance** (`3f1376a`) `JobKind::TestJob` +
    `spawn_test_job(durationSecs)` Tauri command + a "Background Jobs
    — dev test" SettingsCard in Settings → Library with Spawn 30 s /
    Spawn 10 s buttons. Lets the operator exercise the bar without
    burning a real download (cores download in well under a second on
    fast internet, too short to inspect the UI).
  - **DOM-stability fix** (`d75cbd8`) Switching the store from
    `createSignal` + `s.map(...)` to `createStore` + `produce`. The
    map approach swapped in a new object reference for the row on
    every Progressed event, which Solid's `<For>` keys by identity →
    the per-row DOM was destroyed and recreated 10×/sec. Two symptoms
    fell out: the pause + cancel buttons flickered (visible DOM
    churn) and clicks never landed (mousedown landed on a node that
    was about to be destroyed before mouseup). createStore + produce
    mutates fields in place; DOM stays stable; buttons retain
    identity and clicks work. Added a dedicated `progressTick`
    signal because store-field mutations don't surface through the
    array-identity subscription the pulse animation was using.
  - 660 of 660 oa-shell tests green. Operator smoke-tested pause +
    cancel via the dev affordance before merge. Phase 1 pause
    caveat carries through: the pause button stops the chunk loop
    streaming but the row state stays `running` because Phase 1's
    pilot kind doesn't yet bridge the pause flag back to
    `mark_paused` — Phase 3 wires that, and the button will start
    toggling to "▶ resume" then.

- **Background jobs + persistent progress bar — Phase 1 (backend pilot)**
  ([features/background-jobs/](features/background-jobs/)) — merged to
  main 2026-06-02 (`--no-ff` from `feat/background-jobs-phase-1`).
  Six phase commits + a launch-crash fix landing the backend half of
  the 5-phase arc locked earlier the same day in
  [docs/PLANS/background-jobs-and-progress-bar.md](PLANS/background-jobs-and-progress-bar.md).
  - **Slice A** (`7add49c`) feature-folder skeleton + ACTIVE_WORK entry
    + INDEX cross-cutting link.
  - **Slice B** (`5c734d5`) schema migration v17→v18: new
    `background_jobs` table + 3 indexes per plan §Schema.
    `parent_job_id` uses `ON DELETE SET NULL` so the rolling-buffer
    prune of finished parents doesn't cascade and drop in-flight
    children.
  - **Slice C** (`e3ac548`) `apps/oa-shell/src/job_registry.rs`
    (~700 LOC): JobKind + JobState + JobSnapshot + JobEvent +
    JobHandle (cancel + pause AtomicBool + last-write rate-limit
    cells) + the JobRegistry wrapping `Arc<Inner>` for cheap Clone.
    1 Hz SQLite write debounce, 10 Hz Tauri event cap, ~1 s heartbeat,
    100-row history rolling buffer pruned on each finalize. 7 unit
    tests cover the lifecycle + invariants.
  - **Slice D** (`916cd31`) `<data_dir>/oa.lock` lifecycle + crash
    detection wired in main.rs: lock file present at startup →
    `promote_running_rows_to_interrupted` runs on registry
    construction. Path shuttles from setup() to the post-`.run()`
    cleanup via the same `Arc<OnceLock<PathBuf>>` pattern the
    window-geometry flusher uses.
  - **Slice E** (`86c9a96`) `core_installer::download_core` wired
    via JobRegistry: cancel + pause polled inside the chunk loop,
    per-chunk progress through `registry.progress()`, finalize block
    handles mark_completed / mark_cancelled (with .partial cleanup
    per plan §"Cancel cleanup") / mark_failed. Existing
    `oa://core-download-progress` emit stays intact so Guided Setup's
    listener doesn't break.
  - **Wrap + fix** (`7ce47b1` + `71b24cc`) SESSION_LOG entry + a
    launch-crash fix: setup() runs synchronously on Tauri's main
    thread BEFORE the async runtime is entered, so raw `tokio::spawn`
    in `JobRegistry::new` panicked with "no reactor running."
    Switched to `tauri::async_runtime::spawn` which queues onto
    Tauri's managed runtime regardless of caller context. Operator
    smoke-tested before merge.
  - 660 of 660 oa-shell tests green. Phase 2 (BackgroundJobsBar Solid
    component) + Phase 3 (auto-resume dispatch) + Phase 4 (remaining
    8 kinds + dependency graph) + Phase 5 (Settings + Recent activity
    panel) queued. Phase 2 is the natural next step but stays in
    NEXT.md HIGH band rather than auto-starting; the arc's plan
    explicitly allows pipelining around other work.

- **Per-system descriptor consolidation — Slice 2 (mass migration + L1 const deletion)**
  ([docs/PLANS/per-system-descriptors.md](PLANS/per-system-descriptors.md))
  — shipped 2026-06-02 across five phase commits on
  `feat/per-system-descriptors-slice-2`. Completes the consolidation
  arc: every per-system data point that used to live in a Rust const
  now lives in `config/systems/<id>/{system,bios,games}.yaml`.
  - **Phase A** (`d4553d1`): `tools/migrate-systems/` standalone dev
    tool. Five regex-based parsers feed an embedded SYSTEM_THEMES
    mirror of frontend systemThemes to emit the YAML triple per
    system. CLI with --check / --dry-run / --systems subset.
  - **Phase B** (`d4e5b89`): 46 system.yaml + 19 bios.yaml emitted.
    Channel F's sl90025.bin hand-flagged `optional: true` (only
    special-cased system). docs/cores/{snes,nes,genesis}/system-info.yaml
    deleted (content migrated).
  - **Phase C** (`368b81c`): 17 more check_*_bios functions + new
    resolved variants for libretro_dat_refs +
    default_core_dll wired through the registry shim pattern. 3
    new parametric tests.
  - **Phase D** (`14e2c41`): deleted ~2,750 LOC of L1 const data.
    19 `*_BIOS_KNOWN_HASHES` consts, 45-arm libretro_dat_refs match,
    41-arm default_core_dll match, known_hashes_for_system,
    scan_bios_table, LIGHT_GUN_SYSTEMS reference module, the
    migrate-systems tool itself. check_*_bios functions become
    one-liners.
  - **Phase E** (this commit): docs flips + SESSION_LOG entry.
  - End state: 46 systems load entirely from `config/systems/<id>/`.
    646 oa-shell tests green. Slice 3 (L3 content packs + L4
    SQLite layer + JSON Schema + CI lint) queued in
    [docs/NEXT.md](NEXT.md).

- **Per-system descriptor consolidation — Slice 1 (pilot: GB + PSX + NDS)**
  ([docs/PLANS/per-system-descriptors.md](PLANS/per-system-descriptors.md))
  — shipped 2026-06-02 across five phase commits on
  `feat/per-system-descriptors-slice-1`. Replaces ~8 scattered
  per-system data sources (hardcoded Rust const tables for BIOS
  hashes, core catalog, libretro-dat refs + the in-tree
  `docs/cores/<id>/system-info.yaml` + `games-info.md`) for 3 pilot
  systems with a unified per-folder YAML triple
  (`config/systems/<id>/system.yaml` + `bios.yaml` + `games.yaml`).
  - **Phase A** (`0dd1e8c`): `apps/oa-shell/src/system_descriptor.rs`
    + `system_registry.rs` scaffolding. serde-derived types with
    `deny_unknown_fields`, runtime loader with hot-fail on malformed
    YAML / id-folder mismatch / embedded system_info id mismatch /
    duplicate id; `global_registry()` OnceLock singleton; resolver
    mirrors `system_info::resolve_docs_cores_dir`. 21 new tests.
  - **Phase B** (`5544390`): `config/systems/gb/system.yaml` with
    embedded `SystemInfoCurated`. New `load_curated_records_with_registry`
    + `hash_l1_l2_inputs_with_registry`; `bake_system_info_on_launch`
    swapped through registry. Legacy `docs/cores/gb/system-info.yaml`
    deleted.
  - **Phase C** (`edc6bc4`): `config/systems/psx/{system,bios,games}.yaml`
    (any_of 18 BIOS files). New `scan_bios_entries` +
    `check_bios_from_registry` + `is_canonical_bios_hash` shims;
    `check_psx_bios` + `install_bios_file` + `GameInfoIndex::load_default`
    consume them. Legacy `docs/cores/psx/system-info.yaml` +
    `games-info.md` deleted.
  - **Phase D** (`e01d851`): `config/systems/nds/{system,bios,games}.yaml`
    (all_required 3 BIOS files). `check_nds_bios` wired through the
    same shim. Legacy `docs/cores/nds/games-info.md` deleted.
  - **Phase E** (this commit): docs flips + SESSION_LOG entry.
  - **Loader path decision** (resolved Slice 1): sibling
    `<exe_dir>/config/systems/` with source-tree fallback for dev +
    test. Chosen over `include_dir!` because Slice 2 Verification #3
    requires operator-editable YAMLs without recompile. Bundling will
    copy in-tree `config/` next to `oa-shell.exe` at install time.
  - End state: 3 systems run off the registry; 38 unmigrated systems
    unchanged (keep reading hardcoded const via the
    "prefer-registry, fall back" shim pattern). 643 oa-shell tests
    green (was 615 pre-branch; +28 new). Slice 2 (mass migration of
    remaining 38 + const-table deletion) queued in HIGH band of
    `docs/NEXT.md`.

- **Guided Setup Phase 1B — wizard upgrade (CLOSED)** —
  ([features/guided-setup/](features/guided-setup/)). Six slices
  shipped 2026-06-01 (`5ef8062` / `04fa975` / `b57f3e7` / `923ea7b`
  / `e3092b8` / `bf77117` merges to main, ~1,800 lines of new code
  total). The orphaned wizard (legacy-Shell toolbar entry point
  deleted 2026-05-31) is now reachable via Settings → Library, with
  a smart-scan classifier emitting per-row Hash/Header/Extension/
  Hint confidence + canonical titles; a LaunchBox-inspired per-ROM
  results table with inline edits + bulk-select + sort + filter; a
  per-system readiness checklist surfaced in both the wizard Step 3
  AND as a Settings → Library card; a bulk missing-core install
  modal calling `core_installer::download_core` in parallel via the
  buildbot; structured per-file BIOS resolution with a `Pick BIOS
  file…` picker calling the new `install_bios_file` Tauri command;
  warmed copy across the surfaces per plan §4 voice; and a
  first-launch hero in `LibraryView::EmptyState`. 615 oa-shell
  tests stayed green throughout. Phase 1B is feature-complete; the
  next major guided-setup work-item (Phase 2 — curated CPU-tier
  core selection per plan §13) is queued in
  [NEXT.md](NEXT.md) HIGH band, awaiting fresh operator green-light.
  Full per-slice ship log in
  [features/guided-setup/SESSION_LOG.md](features/guided-setup/SESSION_LOG.md).

- **MAME ROM-set name resolution (listxml metadata pass)** —
  branch `feat/mame-rom-set-name-resolution`, five phase commits
  closing `docs/cores/mame/ROADMAP.md` line 56's listxml-deferral.
  Library tiles now show "Donkey Kong (US set 1)" instead of `dkong`
  on first launch; GameDetailPanel auto-surfaces year + manufacturer
  via MediaDb GameMetadata enrichment.
  - **Phase 1a** (`0e13bb5`) — `tools/mame-extractor` emits
    `assets/mame-source/mame-games-slim.json` (name + description +
    year + manufacturer + cloneof per machine) alongside the
    existing per-system slim; single-pass walk with both outputs.
    Filter: `runnable!=no` AND `isbios!=yes` AND `isdevice!=yes`
    AND has `<rom>`. Both parents + clones emit own descriptions.
  - **Phase 1b** (`4957a19`) — operator regenerated bundle via
    `tools/bump-mame.sh` against MAME 0.288. 42,612 records, 5.4 MB
    minified.
  - **Phase 2** (`e68def2`) — SQLite migration v17 (`mame_games`
    L1 + `mame_games_overrides` L3 + `mame_games_meta` KV); new
    `apps/oa-shell/src/mame_games.rs` module; 5 Tauri commands
    (`lookup_mame_game` / `get_mame_game_override` /
    `set_mame_game_override` / `reset_mame_game_override` plus
    `media::set_game_mame_metadata` MediaDb writer); bake-on-launch
    wired next to `bake_system_info_on_launch`.
  - **Phase 3** (`8e98972`) — `frontend/src/library/ingest.ts::resolveMameTitles`
    cutover with the supersede-don't-replace lookup chain
    (`lookup_mame_game` → legacy `lookup_mame_title` → filename);
    writes year + publisher to MediaDb GameMetadata at ingest so
    GameDetailPanel renders enriched without per-system UI work.
  - **Phase 4** (`4dd3b74`) — `mame_import.rs::parse_listxml`
    mirrors the Phase 1a refactor so "Refresh MAME system info"
    bakes both tables from one MAME run; `games_refreshed` count
    surfaces in the existing toast.
  - **Phase 5** (this commit) — docs flips.

  598 oa-shell tests passing (+21 from pre-branch 577). Frontend
  typecheck silent. Legacy `mame_titles` table (v11 libretro-database
  MAME.dat) preserved as 2nd-tier fallback; L3 edit UI deferred to v2.

- **System Info Panel v1** — merged to main 2026-06-01 (`--no-ff`
  from `feat/system-info-panel-v1`). Six phase commits + a
  SCHEMA_VERSION trap fix closing `docs/PLANS/system-info-panel-v1.md`
  end-to-end.
  - **Phase 1a + 1b** (`8491bd5` + `13300a7`) — maintainer-time
    `tools/mame-extractor/` Rust binary + `tools/bump-mame.sh`
    wrapper + slim artifacts in `assets/mame-source/` (12.5 KB
    listxml-slim.json with 36 of 39 OA slugs populated, 40 KB
    history-slim.xml filtered from arcade-history.com's
    history.xml v2.87a, mame-version.txt). Discovered
    history.dat → history.xml migration mid-stream
    (MAME deprecated the text format in 2023; arcade-history.com
    publishes XML). Project-wide `Emulators/` convention also
    introduced here (canonical home for third-party emulator
    binaries OA shells out to; `/Emulators/` gitignored).
  - **Phase 2** (`d59879b`) — Rust types + SQLite migration v16
    (4 tables: system_info_mame / _curated / _overrides / _meta)
    + bake-on-launch with hash-based dirty detection + 5 Tauri
    commands + 5 L2 YAMLs migrated from systemMetadataStubs.ts.
  - **SCHEMA_VERSION trap fix** (`3a65a77`) — pre-existing
    library_db.rs bug where the early-return short-circuit
    skipped the v15+v16 migrations on installs at user_version=14.
    Constant bumped to 16 with a long inline comment calling out
    the trap. Side-fixed Game Info Panel v1's silent
    `game_info_overrides` absence on the same installs.
  - **Phase 3** (`3131d13`) — frontend cutover. SystemInfoPanel
    + HomePage hero read the merged record via getSystemInfo;
    systemMetadataStubs.ts deleted (-328 lines). Schema
    refinements applied: dropped Input Latency + Emulator Core
    rows; Co-Op Support → Multiplayer (free-form); added
    Refresh Rate row.
  - **Phase 4** (`ee60ad3`) — per-system Settings drill-in edit
    UI. New PerSystemInfoSection with form-row-per-field input
    + provenance badges (slate "curated" = L2, accent "edited" =
    L3, no badge = L1 default) + dedicated peripherals editor
    + Reset All Overrides button. SystemInfoCurated flipped to
    rename_all="camelCase" with snake_case aliases so the wire
    format matches the rest of the API while YAMLs stay
    snake_case.
  - **Phase 5** (`c717d1a`) — operator-driven MAME re-import.
    New apps/oa-shell/src/mame_import.rs ports the maintainer-
    time extractor in-process (shared MAME_DRIVER_MAP +
    parse_listxml + format_clock). New "Refresh MAME system
    info" card in StorageSettings with folder-picker fallback.
    L2 + L3 untouched on refresh.
  - **Phase 6** (`<this commit>`) — docs + About credits.
  Workspace tests: 577 oa-shell green at branch tip (was 539
  pre-branch; +38 from system_info module + library_db tests
  + mame_import tests). Frontend `npm run typecheck` silent.

- **NDS per-game touch hotspots overlay** — merged to main
  2026-05-31 (`--no-ff` from `feat/nds-touch-hotspots`). Three
  phase commits closing the second half of `docs/cores/nds/ROADMAP.md`
  "Per-game touch overlay UI" (visual stylus reticle was first
  half, 2026-05-27). Schema extension: new
  `touch_hotspots: [{ label, x, y, w, h }]` optional field on
  `GameInfo` in `apps/oa-shell/src/game_info.rs`; coords in
  NDS bottom-screen native space. New
  `frontend/src/components/TouchHotspotOverlay.tsx` renders
  accent-outlined labelled rectangles via contain-fit math
  against the standard NDS combined-frame aspect. Per-session
  "Show touch hints" toggle in QuickSettings ActionsPanel,
  NDS-gated. Seed entries for Phantom Hourglass + Brain Age +
  Trauma Center in new `docs/cores/nds/games-info.md`. V1
  limitation: assumes default melonDS stacked-vertical screen
  layout; non-default layouts misplace hotspots until v2 reads
  the core option. 540 oa-shell tests green (+3 new). Full
  per-phase summary in [docs/SESSION_LOG.md](SESSION_LOG.md)
  2026-05-31 entry.

- **Legacy Shell deletion** — merged to main 2026-05-31 (`--no-ff`
  from `feat/retroverse-legacy-deletion`). Three phase commits
  closing the deletion plan in
  `docs/PLANS/retroverse-flag-deprecation.md` §7. Net **-1860
  lines** across 13 files. Phase 1 stripped the entire
  `<Show fallback={<Shell>}>` flag-gate wrapper + 76 legacy
  MenuBar items + supporting signals/handlers/imports + the
  `library-manager` + `cores` variants from `SidebarView`.
  Phase 2 deleted six legacy chrome files (Shell / TopToolbar /
  RightSidebar / MenuBar / widgets/index / WidgetCustomizerDialog,
  -1268 lines on disk). Phase 3 collapsed `LibraryManagerPage`'s
  `variant` + `onBack` props now that only the panel-mode caller
  survived. `SystemBackground` / `SystemBootAnimation` /
  `StylusOverlay` restored as siblings of `RetroverseProvider`
  (these had only rendered inside the legacy `<main>`
  pre-deletion); the restoration surfaced a Per-System UI vs
  Retroverse theme visual conflict that's filed for follow-up in
  `docs/PARKING_LOT.md` 2026-05-31 entry. Operator's interim
  workaround is turning Settings → Display → Per-system
  experiences OFF. The flag accessor + Settings toggle remain in
  place through one more release cycle per the deprecation
  plan §4. Full per-phase summary in
  [docs/SESSION_LOG.md](SESSION_LOG.md) 2026-05-31 entry.

- **Retroverse migration follow-ups — drop overlay + header
  affordances + Help-dialog Retroverse home** — merged to main
  2026-05-30 (`--no-ff` from `feat/retroverse-migration-followups`).
  Three phase commits closing items from §5 of
  `docs/PLANS/retroverse-flag-deprecation.md` ahead of the eventual
  legacy-Shell deletion PR. Phase 1 lifts the folder-drop overlay
  out of the legacy Shell so Retroverse mode shows the visual cue
  too. Phase 2 wires Quit + Game-focus indicator into the
  RetroverseShell header next to the profile chip via new
  `gameFocus` + `onQuit` context handlers. Phase 3 sweeps stale
  "menu bar / legacy Shell only" prose from category helpText, and
  closes a discovered gap — Debug log + Keyboard shortcuts dialogs
  were only reachable from the legacy MenuBar — by surfacing
  buttons in AboutSettings → Report a bug card via new
  `onOpenDebugLog` + `onOpenKeyboardShortcuts` handlers. No legacy
  code removed yet (that's the deletion PR). `npm run typecheck`
  silent. Three migration items remain (drop
  WidgetCustomizerDialog, drop right-sidebar toggle, operator
  playtest of Hide/Show + Ctrl+W) and fold into the deletion PR
  itself. Full per-phase summary in
  [docs/SESSION_LOG.md](SESSION_LOG.md) 2026-05-30 entry.

- **Gameplay fixes batch — NDS multi-touch + lightgun gun-side
  buttons + SNES Super Multitap** — merged to main 2026-05-30
  (`--no-ff` from `feat/gameplay-fixes-batch`). Four ordered phase
  commits closing small-to-medium per-core ROADMAP gaps in one
  focused branch. Phase 1 NEXT.md cleanup (Jaguar high-bit + SMS
  Phaser struck; SNES + NDS touch-overlay bullets narrowed). Phase 2
  SNES Super Multitap subclass id 257 in the per-game device-type
  dropdown (verified against upstream snes9x source). Phase 3 NDS
  multi-touch — `pointer_secondary` companion field +
  `pointer_field_value(primary, secondary, index, id)` dispatch;
  POINTER_COUNT reports 0/1/2 total pressed; v1 plumbing only,
  InputPoller leaves secondary at zero until a real second-finger
  source is wired. Phase 4 light-gun gun-side buttons —
  `lightgun_buttons: u32` (u32 deviation from u16 because RELOAD is
  id 16) + State mirror + bit-keyed `lightgun_field_value` dispatch;
  AUX_A/B/C + START + SELECT + DPAD + RELOAD wired through. Bindings
  derive from per-port RetroPad bits via
  `oa_input::lightgun_buttons_from_joypad_bits` — no new bindings
  UI surface. Per-core ROADMAP flips: snes / nds / nes / sms /
  saturn / psx / dreamcast / atari7800. Cross-system POINTER+LIGHTGUN
  inventory entry in NEXT.md updated. oa-libretro 23→30 tests (+7
  new); 539 oa-shell tests stable. Full per-phase summary in
  [docs/SESSION_LOG.md](SESSION_LOG.md) 2026-05-30 entry.

- **Game Info Panel v1** — merged to main 2026-05-30 (`--no-ff` from
  `feat/game-info-panel-v1`, merge `1caa4bc`). 11-phase arc across nine
  phase commits; full per-phase summary in
  [docs/SESSION_LOG.md](SESSION_LOG.md) 2026-05-30 entry. Backend ships
  the three-layer data model (file layer at `docs/cores/<id>/games-info.md`
  + SQLite `game_info_overrides` v15 migration + field-typed precedence
  merge) and six Tauri commands; UI extends Retroverse `GameDetailPanel`
  with operator-note + Controls + Recommended core (+wired Apply action)
  + Known issues sections, adds `⚠ N` + `✎` tile badges, and gains a
  4th "Game info" tab in `GameInfoModal` with the inline editor + Submit
  correction stub. Plan: `docs/PLANS/game-info-panel.md`. Schema:
  `docs/cores/SCHEMA.md`. v1 seed entries live in `docs/cores/psx/games-info.md`
  (Tomb Raider + Final Fantasy VII). 539 oa-shell tests green (up 33 from
  506). Per-system `KNOWN_GAME_BUGS → games-info.md` migration + per-core
  README touch-ups (42 systems) stay operator-driven follow-up.

- **libretro env-callback batch (four gaps closed)** — merged to main
  2026-05-30 (`--no-ff` from `feat/libretro-env-callbacks-batch`, merge
  `3b35a41`). SET_MEMORY_MAPS storage + Core::memory_map() trait method
  (unblocks RetroAchievements rcheevos); SET_MESSAGE / SET_MESSAGE_EXT
  routed to existing oa://toast layer with per-system theming;
  SET_SUPPORT_NO_GAME flag + LibretroCore::load_no_rom() (DOSBox-Pure /
  ScummVM bootless mode infrastructure); disc-control v2 extras
  (add_image_index / replace_image_index / set_initial_image /
  get_image_path) — DiscInfo gains `paths` Vec. Full summary in
  [docs/SESSION_LOG.md](SESSION_LOG.md) 2026-05-30 entry. Cross-system
  inventory in `docs/NEXT.md` records each gap.

- **Controller navigation v2 polish**
  ([features/controller-nav/](features/controller-nav/)) — merged to
  main 2026-05-26 (`--no-ff` from `feat/controller-nav-v2-polish`).
  Three phase commits + a docs commit closing three of the four
  LOWER-band #1 items NEXT.md carried as "Controller-nav v2 polish
  (operator-driven)." Slice 1 QuickSettings sub-views (`b87493d`):
  rewind / TAS / video / memory / disc panels each gain a focus group
  + back handler; most via a new `useDomQueryFocusGroup` helper in
  `frontend/src/nav/focus.ts` (DOM-query + MutationObserver +
  identity-tracked focused element, generalized from the MenuBar
  pattern); the rewind scrubber uses an `onDirection` override so DPad
  left/right scrubs the timeline. Slice 2 right-sidebar widget DPad
  browse (`c883af3`): sidebar body becomes one DOM-query group keyed
  by `data-oa-sidebar-row`, widget wrappers participate alongside the
  action row, R1 from grid still lands on Play (createEffect snaps
  focus to first action while inactive). Slice 3 MenuBar identity-
  tracked focus (`567d0de`): closes Slice K's known limitation by
  tracking the focused button by element identity through rebinds.
  Pin toggle + sidebar-hide button in the right-sidebar header stay
  mouse-only by design (utility chrome, not the play path) — one
  of the four LOWER-band #1 bullets remains ⬜ for that reason. With
  controller-nav now fully shipped (Phase 0 + completion pass + v2
  polish), Per-System UI Stage 1 is the next major arc per the
  pipelined sequence in `docs/NEXT.md`.

- **Controller navigation completion pass**
  ([features/controller-nav/](features/controller-nav/)) — merged to
  main 2026-05-26 (`--no-ff` from `feat/controller-nav-completion`).
  Extends Phase 0 (A–E, merged earlier today) to cover every remaining
  interactive surface so the operator can run the whole shell from a
  pad. Ten commits on the branch: seven feature slices plus three
  post-test fixes. Slices: F critical polish + global back-stack
  (`102eef8`); G context + overlay menus — TileContextMenu /
  SystemContextMenu / SaveSlotsModal / QuickSettings (`8254aa1`);
  H GameInfoModal + universal Dialog B-close via the primitive
  (`6cb86d9`); a fix that gates the frontend Web Gamepad poller while
  gilrs owns input via DOM focus on the library WebView (`662cd5a`);
  K top toolbar menu bar with Start-to-open + L1/R1 menu cycling
  (`d68ab7f`); L chained CorePicker + RegionPicker popovers
  (`8180a0e`); M right sidebar widget actions row with R1-from-grid
  transfer (`e721e7d`). Post-test fixes: docs (`4079d20`), library
  grid DPad left/right wrap-across-rows (`792f17d`), and menu bar
  focus ring + disabled filter + dynamic content + a cross-cutting
  `data-oa-focus-active` CSS broadening (`dc25ab4`). Read-only
  widgets, utility chrome, and QuickSettings sub-views deliberately
  stay mouse + keyboard in v1. With Phase 0 + the completion pass
  shipped, Per-System UI Stage 1 unblocks next per the pipelined
  sequence in `docs/NEXT.md`.

- **Controller navigation primitives (Phase 0)**
  ([features/controller-nav/](features/controller-nav/)) — merged to
  main 2026-05-26 (`--no-ff` from `feat/controller-nav-primitives`).
  5 phase commits shipping the shared foundation for the Guided Setup
  + Per-System UI arcs: Slice A `nav/gamepad.ts` Web Gamepad API
  poller (`ca3dff9`); Slice B `nav/focus.ts` useFocusGroup hook with
  vertical/horizontal/grid orientations + L1/R1 neighbour transfer
  (`d8a5ffb`); Slice C `nav/HintBar.tsx` persistent footer + module-
  stack HintRegion (`a3a54b3`); Slice D wired VirtualLibraryGrid +
  LeftSidebar with DPad nav + A/X buttons + shoulder bumpers
  (`49522ab`); Slice E `Settings → Display → Controller navigation`
  panel with master toggle / source picker / A↔B swap / animation
  budget (`f2501fb`). Operator-confirmed working before merge.
  Phase 0 closes; Per-System UI Stage 1 unblocks per the pipelined
  sequence in `docs/NEXT.md`.

- **DOSBox + ScummVM onboarding** ([features/dosbox-and-scummvm/](features/dosbox-and-scummvm/))
  — shipped 2026-05-24 across two `--no-ff` merges. Phase 1
  scummvm (`0b56bd8`, branch `feat/dosbox-and-scummvm`) wired
  the descriptor-file engine launcher; Phase 2 dosbox (`b6fea2c`,
  branch `feat/dosbox-onboarding`) wired the directory-path engine
  launcher and added new infrastructure that future engine cores
  will reuse: `is_directory_path_system` helper, `run_dir_scan_blocking`
  + `start_background_directory_scan`, `systemHint`-aware classification
  in the Import Wizard, `GameOverrides.dosbox_entry_point` field.
  Cross-stream SESSION_LOG + commit shas at
  [docs/features/dosbox-and-scummvm/SESSION_LOG.md](features/dosbox-and-scummvm/SESSION_LOG.md).
  Per-core ROADMAP Phase 1 entries flip ✅ when operator playtest
  validates each (gated on having game data on hand).

- **Media taxonomy** ([features/media-taxonomy/](features/media-taxonomy/)) —
  merged to main 2026-05-24 (`--no-ff` from `feat/media-taxonomy`).
  7 phase commits implementing the full LaunchBox-shape art/audio
  taxonomy locked in the 2026-05-23 plan:
  - **Phase 1** (`7c1b0e9`) data model + folder layout: MediaKind
    5 → 27 variants, GameMedia/SelectedMedia per-slot fields,
    sanitize/path-builder helpers, set_manual_cover writes to new
    layout via library_db.find_game_by_id rom_stem lookup.
  - **Phase 2** (`c2d0976`) libretro-thumbnails sync to new layout
    + operator-art-wins guard (sha-based cache, next-variant
    suffix), ingest_manual_for_slot eviction logic.
  - **Phase 3** (`2edfc1d`) LaunchBox/EmuMovies art-pack importer
    (auto-detects single- vs multi-platform layouts, fuzzy
    matches against library titles at 0.95 threshold) +
    ImportArtPackDialog UI.
  - **Phase 4** (`b71057c`) 4-bus audio mixer over rodio/symphonia
    (platform-music / ui-sounds / ceremony / snap-audio) +
    SystemSettings audio override fields + GameOverrides
    platform_music_path + frontend audio dispatch service.
  - **Phase 5** (`92c2403`) existing-install migration: walks
    pre-Phase-1 MediaDb, moves manual covers / copies synced art
    to canonical kind dirs, sentinel-guarded one-shot pass.
  - **Phase 6** (`d8dd98a`) per-system PlatformMedia (9 slots —
    banner, clear-logo, console, controller, fanart, marquee,
    photo, wheel, background) + PlatformMediaDialog UI.
  - **Phase 7** docs + SESSION_LOG (this entry).
  cargo test workspace 430 oa-shell + 64 others all green.
- **Window geometry persistence + tile-size slider**
  ([features/ui-polish/](features/ui-polish/)) — merged to main
  2026-05-23 as `6cf6acb`. 3 phase commits on
  `feat/window-and-tile-prefs`: `LayoutPrefs.windows` map with
  per-label geometry + first-launch maximize + debounced flusher
  thread; `library_tile_size` + GridControls slider + hybrid ±20%
  scaling in VirtualLibraryGrid; SESSION_LOG entry.
- **Portable install** ([features/portable-install/](features/portable-install/)) —
  merged to main 2026-05-23 as `993ca6a`. 5 commits: data_dir
  resolver + marker file, asset-protocol runtime scope + frontend
  getDataDir helper, AppData→portable auto-migration with sentinel,
  CLAUDE.md + docs, and a follow-up `npm --prefix` fix to
  tauri.conf.json so `cargo tauri build` works end-to-end.
- **Docs audit + reorg** — branch `feat/docs-audit-and-reorg`, 5 commits.
  Phase 1 fixed stale references across the docs tree; Phase 2 introduced
  `INDEX.md` + `ACTIVE_WORK.md` + `docs/features/<name>/` skeleton, moved
  executed plans into their feature folders, re-filed cross-cutting
  session entries out of per-core SESSION_LOGs, and capped the long
  SESSION_LOGs with sibling ARCHIVE files. Merged to main.
- **Sidebar v3.4 PARKING_LOT entry** — small doc-cleanup PR merged
  to main 2026-05-23 as `c700641`.

## Recently completed (last 1–2 sessions; reference for context)

- **Sidebar tier + view editor** ([features/sidebar/](features/sidebar/)) —
  PR-α/β/γ shipped 2026-05-21; v2.1–v3.5 shipped 2026-05-22. Tier plan and
  View Editor plan are now historical reference under features/sidebar/.
  Outstanding: v3.4 per-container art slots (parked in PARKING_LOT.md).
- **UI polish** ([features/ui-polish/](features/ui-polish/)) — Phases A–E
  shipped 2026-05-22. Menu-bar IA operationalized via dialog refactor.

## Cores

No core is in active deep-integration work today. The 2026-05-20 POINTER
infrastructure batch (psp + ps2 + nds) was the most recent cross-core focus.

Per-core status surfaces:
- High-priority next work — [NEXT.md](NEXT.md) HIGH/MEDIUM bands
- Per-system status — `docs/cores/<id>/ROADMAP.md`
- **5200 + pokemini** Phase 0 fully wired 2026-05-20 (default core,
  BIOS check, bindings, registry, theme). Phase 1 = operator
  playtest only (drop .dll + BIOS, scan library, launch flagship
  titles per the ROADMAP). No more code work on these two from this
  side until playtest surfaces a Phase 2 polish need.
- **scummvm + dosbox** — engine cores, plan locked 2026-05-24
  ([features/dosbox-and-scummvm/](features/dosbox-and-scummvm/)).
  5-phase implementation pending operator green-light. Both ship as
  ordinary OA systems alongside consoles; scummvm scans for
  `.scummvm` descriptor files, dosbox scans for one-level-deep
  subdirectories. No new UI surface beyond the existing sidebar.

## Picking next work

When this stream wraps and there's no clear next ask: read [NEXT.md](NEXT.md)
HIGH/MEDIUM bands first, then [PARKING_LOT.md](PARKING_LOT.md). Confirm the
pick with the operator before starting.
