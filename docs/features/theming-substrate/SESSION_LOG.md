# Theming Substrate — Session Log

Each session that touches this feature appends a 3-line entry:
**Shipped / Almost / Next.** Older entries roll to
`SESSION_LOG_ARCHIVE.md` when this file passes ~150 lines
(rolled 2026-06-10 — Phase 4 Slices 1-2 and everything before the
grab-bag drain live there; live file keeps Slices 3-5 + the drain).

---

## 2026-06-10 — Phase 4.5: the EVENT corral (sibling to the invoke ban)

- **Shipped:** Closed the one coupling the Phase 4 audit surfaced as still-open —
  Tauri **event names**. New `platform/api/eventsApi.ts` is the sole module
  allowed to touch `@tauri-apps/api/event`: it owns `OA_EVENTS` (a 23-entry
  registry of every `oa://…` channel — the single source of truth for event-name
  strings), the moved `listenScoped` (auto-cleanup), `listenTo` (manual
  lifecycle), and `emitEvent`. `platform/lib/eventListener` now just re-exports
  `listenScoped` for back-compat. Migrated **~30 listen/emit/listenScoped sites
  across 16 files** (App.tsx, backgroundJobs, audio, media, platformMedia,
  ingest, ToastStack, toast, CoresPage, SystemCoresStrip, GameDialogs,
  LibraryManagerPage, SystemReadinessChecklist, ImportWizard, MissingCoreBulkPrompt,
  PlatformMediaDialog) + the theme file that emitted `oa://toast` directly
  (`routes/retroverse/GameDetailPanel`). Every `oa://…` string now lives ONLY in
  `OA_EVENTS` (grep-verified: 0 outside eventsApi).
- **The ratchet:** extended the existing `no-restricted-imports` rule with a
  second entry banning `listen` / `emit` / `once` from `@tauri-apps/api/event`
  outside `platform/api/**` (type-only imports like `type UnlistenFn` stay
  allowed). Probe-verified it fires. typecheck + lint green.
- **Decision D17** — events are a backend-contract surface like commands; corral
  them the same way. Payloads stay generic on `<T>` (call site declares its view).
- **Merged:** ✅ operator playtested + merged to main 2026-06-10.
- **Two playtest fixes rode along on the same branch (operator-confirmed):**
  - *Jobs bar invisible.* The persistent BackgroundJobsBar rendered at z-55,
    *behind* the opaque engine surface (z-60) — and jobs are spawned from inside
    that surface (Settings/Cores/Import), so it was hidden exactly when needed.
    Lifted to z-65 (above the engine takeover, below platform modals at z-70).
    NOT a Phase-4 regression — pre-existing since the Phase-1 engine-surface
    split; the jobs store + events were fine (verified).
  - *Native confirm/alert broken under Tauri 2.* `window.confirm` is intercepted
    + ACL-gated AND async (returns a Promise), but all 13 call sites treated it
    as a sync boolean (`if (!window.confirm(x))`) — a Promise is truthy, so the
    guard NEVER fired and destructive actions ran unconfirmed. Replaced all 13
    confirm + 3 alert sites with an in-app awaitable `confirm()`
    (`platform/lib/confirm.ts` + `ConfirmHost`, rendered via the Dialog
    primitive: z-70, themeable, controller-navigable). Zero native dialogs left.
    Follows the May-2026 "move off native dialogs" precedent.
- **Next:** the foundation is now clean on BOTH backend-contract channels
  (commands + events). Theming work resumes the *enable-other-themes* track —
  ARC 1 Phase 3 (shared nav primitives) first.

## 2026-06-10 — Phase 4 Slice 6 (THE CLOSER): jobs/system/shell + invoke-ban lint rule → **Phase 4 COMPLETE**

- **Shipped:** The final three `platform/api/` modules + the ratchet that closes
  the whole decoupling track. On `feat/theming-platform-api-jobs-system-shell`:
  **jobsApi** (18 — active/recent lists, per-job + bulk pause/resume/cancel,
  history clear, duplicate pre-flight, resume prefs + the two job-toggle prefs,
  test job, library/directory scan kick-off + cancel), **systemApi** (9 — the
  System Info v1 L3-override CRUD moved here from systemInfo.ts per D15 +
  status/cpu-tier/perf), **shellApi** (19 — quit, data dir, ui-intercept, the
  log bridge + ring + reveal, reveal-in-folder, game-focus + toggle,
  direct-launch config, ScummVM detection CLI, the sound/music resolvers). Plus
  stragglers folded into existing modules (libraryApi `getLibraryPrefs` /
  `setLibraryPrefs` / `listUnidentifiedGames`; mediaApi `clearMetadataForSystem`).
  **~90 call sites across 21 files** migrated; **every non-`platform/api/` file
  is now free of the raw `invoke` import** (verified: 0).
- **The ratchet:** added `no-restricted-imports` to `frontend/eslint.config.mjs`
  banning the `@tauri-apps/api/core` `invoke` import everywhere **except**
  `src/platform/api/**` (a second flat-config block re-allows it there;
  `convertFileSrc` from the same module stays allowed). Probe-verified the rule
  fires on a planted raw-invoke import. `npm run lint` + `npm run typecheck`
  both green.
- **Notable migrations:**
  - *systemInfo.ts → D15 move + re-export.* It was a pure typed-binding module;
    its six System Info wrappers moved into systemApi and it now re-exports them
    (+ the existing mame re-export). Its shared TYPES stay put, pulled into
    systemApi via `import type`. Zero consumer churn.
  - *Logic modules keep their behavior, route through wrappers.* backgroundJobs.ts
    (store + dedup + event handling), audio.ts (cascade + cache), dataDir.ts
    (cached promise), logbridge.ts (the console bridge) are NOT pure
    pass-throughs (D15 caveat) — they keep their logic and call the thin
    jobsApi/shellApi wrappers internally. backgroundJobs uses `import * as jobsApi`
    so its own exported `pauseJob`/`cancelJob`/… don't collide with the wrappers.
  - *Circular-but-safe.* jobsApi pulls `JobSnapshot`/`JobPrefs` from
    backgroundJobs via `import type` (erased); backgroundJobs imports jobsApi
    values. No runtime cycle.
  - *Census grew under the lint pressure.* The closer surfaced commands no
    earlier slice's census caught (`get_direct_launch_config`, `get_game_focus`,
    `get/set_game_focus_toggle`, `set_job_always_show_bar`/`_sound_on_completion`,
    `clear_metadata_for_system`, `list_unidentified_games`,
    `start_background_directory_scan`) — all now wrapped. The lint rule is what
    guarantees none were missed.
- **Phase 4 totals:** 14 `platform/api/<domain>Api.ts` modules; the command-name
  string for every backend command lives in exactly one file. Decisions D14
  (generic getters) / D15 (typed-binding move + re-export) / D16
  (platform↛components forces types into the api layer) all applied throughout.
- **Merged:** ⏳ awaiting operator playtest + merge. Broad smoke surface (this
  slice touches host/jobs/system plumbing across the whole app): boot, library
  scan + import wizard, background-jobs bar (pause/cancel/resume + history),
  Settings (game-focus chord capture, perf tier, job prefs toggles, test job,
  storage health), Debug Log dialog, ScummVM detect dialog, per-system info edit,
  performance HUD, quit. If boot + a scan + the jobs bar work, the risky parts
  are covered.
- **Next:** **Phase 4 is DONE — the platform/theme decoupling track is closed**
  (file boundary: six lint zones; API boundary: the invoke ban). Theming work
  now shifts to the *enable-other-themes* track: ARC 1 Phase 3 (shared nav
  primitives) → Phase 5 (`.oatheme` packaging) → Phase 6 (rebuild Retroverse as
  a theme on the SDK) → ARCs 2-3 (Rhai behaviors + WGSL shaders + Theme Studio).

## 2026-06-10 — Phase 4 Slice 5: the in-game / gameplay cluster (5 modules, 2 PRs on one branch)

- **Shipped:** Five `platform/api/` modules on `feat/theming-platform-api-gameplay`,
  as two commits (the plan's two-PR split, landed on one branch). **PR A**
  (commit `878e435`): **emulatorApi** (17 wrappers — launch/unload/bootless,
  external-launcher registry + per-system pref + active-launcher caps, multi-disc
  swap/eject/state, disc-set members, selected-variant, region priority) +
  **rewindTasApi** (15 — rewind config/state/scrub, TAS record/replay/list/delete,
  save slots). **PR B** (commit `7e83226`): **cheatsApi** (12 — cheat CRUD + arm,
  formats, live memory search, read_memory_region, pick_patch_file), **milestonesApi**
  (6 — milestone CRUD + arm + reset), **captureApi** (9 — screenshots + video clips).
  ~75 call sites across ~14 files (App.tsx, QuickSettings, GameDialogs, CoresPage,
  PerSystemSettingsBody, media.tsx, launch.ts, settings/store, perSystemSections,
  DiscPickerDialog, SaveSlotsModal, GameInfoModal, ScreenshotGalleryDialog,
  GamePropertiesDialog). typecheck + lint green; every one of the 56 command
  strings greps to **only** its api module.
- **Notable migrations:**
  - *launch.ts stays a rich helper, routes through emulatorApi.* It builds the
    launch args, logs, returns a `LaunchResult` union, toasts on failure — NOT a
    pure pass-through (D15 doesn't apply). Its three exported helpers keep their
    behavior; only the internal raw `invoke()` calls move to the thin
    emulatorApi wrappers (namespace import dodges the `launchRom` name clash).
  - *Namespace imports where wrapper names shadow local functions.* QuickSettings
    has local `startTasRecording` / `startVideoCapture` / `deleteVideoClip` /
    `stopTasReplay` handlers; `import * as rewindTasApi` / `* as captureApi` keeps
    them distinct from the same-named wrappers. Same for GameDialogs (cheats /
    milestones namespaces).
  - *GameDialogs fully drained of raw invoke* (import removed) — its display/
    shader/audio (Slice 1), controller-devices (Slice 4), and now cheats/
    milestones calls are all behind api modules.
  - *Two files emptied their raw invokes this slice and dropped the import:*
    CoresPage and PerSystemSettingsBody (launcher prefs were their last raw
    calls); settings/store (set_rewind_config); perSystemSections
    (get_rewind_state); media.tsx (region + selected-variant); plus the
    single-call leaf files (DiscPickerDialog, SaveSlotsModal,
    ScreenshotGalleryDialog).
- **Judgment calls (carry D14/D16 forward):** generic getters for the duplicated
  reads (`getRewindState` / `listSaveSlots` / `listEmulatorProfiles`); canonical
  contract types defined in the api modules (D16) with single consumers keeping
  structurally-identical local copies; two opaque forwarded blobs kept generic —
  analog-style `routing` already done in Slice 4, and the cheat-search `filter`
  discriminated union (`filterCheatSearch<F>`) here.
- **One behavior touch:** none beyond Slice 4's GameDialogs guard — all
  pass-throughs.
- **Merged:** ⏳ awaiting operator playtest + merge (the whole branch, both PRs).
  Smoke surface: launch/unload, disc swap on a multi-disc game, rewind scrub +
  commit/cancel, TAS record/replay, save-slot load/delete, cheats (add/toggle/
  search), milestones, screenshot gallery, video capture/convert, external
  launcher prefs, region priority.
- **Next:** Slice 6 — `jobsApi` + `systemApi` + `shellApi` (~85 sites:
  backgroundJobs.ts, background-jobs/*, SystemHealthPage, systemInfo.ts, App.tsx
  shell paths, logbridge.ts, scummvm, sounds) **+ turn on the `no raw invoke()
  outside platform/api/` ESLint rule.** That rule flipping green closes the
  entire Phase 4 decoupling track.

## 2026-06-10 — Phase 4 Slice 4: `coresApi` + `inputApi` (cores / bios / core-options + bindings / analog)

- **Shipped:** Two new `platform/api/` modules on a **fresh branch off main**
  (`feat/theming-platform-api-cores-input` — Slices 1-3 already merged, so this
  arc gets its own branch per operator instruction). **coresApi** (18 wrappers:
  installed-core inventory + buildbot catalog/download/install/remove + per-system
  default-core pref + RetroArch-parity core-options table + BIOS inventory/install)
  and **inputApi** (11 wrappers: button bindings + input descriptors + controller
  devices + analog routing + light-gun flag). Migrated **~50 call sites across 18
  files**: CoresPage, CoreOptionsPanel, perSystemSections, PerSystemSettingsBody,
  CorePickerMenu, SystemDialogs, GamePropertiesDialog, SystemCoresStrip,
  MissingCoreBulkPrompt, BiosResolutionDetail, SystemReadinessChecklist,
  SystemHealthPage, SettingsSections, ImportWizard, ingest.ts, backgroundJobs.ts
  (cores side); SystemBindingsEditor, KeypadReference, GenesisPadReference,
  AnalogBindingsSection, GameDialogs, App.tsx (input side). typecheck + lint
  green; every one of the 29 command strings greps to **only** its api module.
- **Three judgment calls:**
  - *The `platform↛components` boundary forces canonical types into the api
    layer (new decision D16).* Several backend-contract types were defined
    component-locally (AvailableCore in CatalogCoreCard, CoreOptionsSnapshot in
    CoreOptionsPanel, etc.). The api module **cannot import them** (the enforced
    six-zone lint forbids platform→components), so the canonical shapes are
    defined IN coresApi/inputApi and the (one) consumer imports them back
    (components→platform/api is allowed). Where a shape is single-consumer the
    local def is deleted and re-homed (CoreOptionsSnapshot/CoreOption,
    InstallResult, CoreRecommendation re-used structurally, InputDescriptor,
    ControllerDeviceDescriptor, AnalogSticksInfo, ButtonBinding canonical).
  - *Generic getters (D14) for the shape-divergent multi-consumer reads.*
    `listCores`, `availableCores`, `getBiosStatus`, `getBindings`, `setBinding`
    are each read with 2-3 different local views (e.g. CoreEntry full vs the
    `{ validExtensions }` one-field view in ImportWizard; ButtonBinding with vs
    without `libretroId`; BiosStatusResponse rich-entries vs `{ slug, status,
    label }` rollup). Wrappers are generic with a canonical default; call sites
    keep their exact local view via the type arg — zero type churn.
  - *The analog-routing `routing` blob stays generic on `R`.* `setAnalogRouting`
    / `setAnalogRoutingForGame` forward an opaque `AnalogPortRouting` whose type
    family lives in AnalogBindingsSection; rather than relocate the whole
    AnalogStickPrefs/AnalogPortRouting/AnalogRoutingPrefs cluster (and bump into
    the same platform↛components wall), the wrappers take `routing: R` and infer
    it at the call site. No `any`, no boundary violation.
- **One behavior touch:** GameDialogs' `get_controller_devices` resource now
  guards `!src.systemId` (was reachable as `string | null`; the typed wrapper
  takes `string`). Equivalent — the guard already required `entryId`, which
  tracks the same entry's systemId; null never reached the backend in practice.
  Everything else is a pure pass-through.
- **Almost:** nothing left in scope for this slice — all 29 commands migrated.
- **Next:** Slice 5 — the in-game / gameplay cluster (`emulatorApi` +
  `rewindTasApi` + `cheatsApi` + `milestonesApi` + `captureApi`): launch.ts,
  QuickSettings gameplay controls, GameDialogs cheats/milestones, SaveSlotsModal,
  ScreenshotGalleryDialog. ~70 sites — may split into two PRs. Then Slice 6
  (jobs/system/shell + turn on the `no raw invoke() outside platform/api/` lint
  rule — the ratchet closes).

## 2026-06-09 — Phase 4 Slice 3: `mediaApi` (art / metadata / game-info / mame / hashes)

- **Shipped:** Created `frontend/src/platform/api/mediaApi.ts` (28 typed
  wrappers, one named export per command, command string lives only here) on a
  **fresh branch off main** (`feat/theming-platform-api-media` — Slices 1-2
  already merged in `a5997e3`, so this arc gets its own branch per operator
  instruction). Migrated the art/metadata cluster across **11 files**:
  `media.tsx` (7 commands via `import * as mediaApi` — its store methods
  `setManualCover`/`clearMedia` shadow wrapper names), `platformMedia.tsx`
  (get_platform_media_index), `PlatformMediaDialog.tsx` (get/set/clear platform
  media), `SystemBackground.tsx` (resolve_background_asset),
  `ImportArtPackDialog.tsx` (import_art_pack), `LibraryManagerPage.tsx` (8:
  sync media/metadata/hashes + only_sync_identified + storage_stats +
  open_folder), `App.tsx` (resolve + sync media/metadata via the existing
  `import * as mediaApi` alias), `ImportWizard.tsx` (resolve + sync media/
  metadata), `ingest.ts` (the mame trio lookup_mame_game/title +
  set_game_mame_metadata, deferred here from Slice 2), `gameInfo.ts` +
  `systemInfo.ts` (see judgment call). typecheck + lint green; every one of the
  28 command strings greps to **only** `mediaApi.ts`.
- **Three judgment calls:**
  - *Existing typed-binding modules → move + re-export.* `gameInfo.ts` (6
    game-info wrappers) and `systemInfo.ts` (`refreshMameSystemInfo`) were
    already thin typed binding modules predating `platform/api/`. Rather than
    double-wrap (delegate) or repoint their 3+1 consumers, the functions MOVE
    into `mediaApi.ts` (their proper home) and the domain modules **re-export**
    them for backward compat — the command string ends up in exactly one place,
    consumers' import paths are untouched, and the shared TYPES (MergedGameInfo
    / GameInfoOverride / MameRefreshReport / …) stay in the domain module,
    pulled into mediaApi via `import type` (erased — no runtime cycle). New
    decision **D15**.
  - *Generic getter only where shapes diverge (D14).* `get_platform_media_index`
    is read in two files with two local `PlatformMediaIndex` views →
    `getPlatformMediaIndex<T = PlatformMediaIndex>()` with a canonical default
    in mediaApi. Single-call-site getters returned concrete contract types
    (`MediaStorageStats`, `ArtPackImportReport`, `MediaIndex`); the callers'
    local copies are structurally identical so they kept them (assignable) —
    zero churn, no forced repoint.
  - *`set_game_mame_metadata` confirmed mediaApi.* It writes MediaDb
    `GameMetadata` (year/publisher) as the store half of the MAME
    resolve-and-store ingest flow → metadata concern, mediaApi. Slice 2's
    grouping holds.
- **Scope discipline:** left raw (other slices, same files) —
  get/set_region_priority + set_selected_variant (emulatorApi/Slice 5, media.tsx),
  start_background_scan + list_cores (jobsApi/coresApi, ingest.ts),
  get/set_system_info* (systemApi/Slice 6, systemInfo.ts), get/set_library_prefs
  (LibraryManagerPage). Their `invoke` imports stayed (verified each file still
  has a live raw call — no Slice-2-style unused-import trap).
- **Merged:** ✅ operator playtested (cover art + manual cover, platform-media
  slots, sync media/metadata, Identify ROMs / resolve hashes, game-info
  overrides + badges, MAME title resolution, per-system backgrounds) + merged
  to main 2026-06-09 (merge `f5657c2`). Behavior-preserving throughout.
- **Next:** Slice 4 — `coresApi` + `inputApi` (cores/bios/core-options +
  bindings/analog; CoresPage, CoreOptionsPanel, SystemBindingsEditor,
  AnalogBindingsSection, ImportWizard core paths). ~45 sites.

## 2026-06-09 — Grab-bag drain: components/ emptied, engine↛components enforced

- **Shipped:** drained the entire `src/components/` grab-bag (38 top-level +
  2 subtrees) in two commits on `feat/theming-grabbag-drain`, then removed the
  directory. **Batch 1** (`0286dae`): in-game / per-game / shared UI →
  `platform/components/` — QuickSettings, GameDialogs (+ the per-game dialog
  family), GamePropertiesDialog, GameInfoModal, SaveSlotsModal, RegionPicker,
  CorePickerMenu, ScreenshotGalleryDialog, ToastStack, PerformanceHud, the
  three context menus, NewCollectionDialog, the overlays (Stylus /
  TouchHotspot / SystemBackground / SystemBootAnimation), the reference cards
  (Keypad / LightGun / GenesisPad), AnalogBindingsSection, CoreOptionsPanel,
  and the whole `background-jobs/` subtree. **Batch 2** (`aba360b`): the
  engine-manager surfaces → `engine/` — SettingsSections, CoresPage,
  LibraryManagerPage, ImportWizard, ImportArtPackDialog, ScummvmDetectDialog,
  DebugLogDialog, HelpDialogs, PlatformMediaDialog, GameMediaManagePanel,
  UnidentifiedGamesDialog, ViewsManagerTab, ViewEditorPane, SystemDialogs,
  SystemBindingsEditor, `import-wizard/*`. Added + verified the
  `engine/** ↛ components/**` lint zone (the batch's goal) plus a
  `routes/** ↛ components/**` ratchet; the eslint header now documents **six
  enforced zones**, only raw `invoke()` (Phase 4) left. typecheck + lint green
  after each commit.
- **Two judgment calls** (both deviate from the plan's literal lists, both
  forced by the import graph — see DECISIONS D12/D13):
  - *Shared leaves go to the LOWER layer.* AnalogBindingsSection +
    CoreOptionsPanel + the reference cards are imported by BOTH engine
    surfaces (SystemBindingsEditor / SystemDialogs) AND per-game platform UI
    (GameDialogs); putting them in engine would force a platform→engine edge,
    so they land in `platform/components/`. GameDialogs's only other engine
    coupling — a re-export block from SystemDialogs — was redirected to its
    true source `@oa/platform/components/perSystemSections`, severing the edge.
    background-jobs/* likewise → platform (the persistent bar is
    theme-territory per SURFACES; RecentActivityPanel / ResumePromptDialog are
    consumed only by the bar / App).
  - *SettingsSections' 5 app-action handlers.* 3 → direct
    `@oa/platform/dialogs` setters (`setWizardOpen` / `setHelpDialog`); the 2
    library-admin actions → a new `platform/libraryAdmin.ts` registry (App
    registers its App-scoped handlers on mount; engine calls them without
    importing App/theme). Chosen over 3-layer prop drilling (D10/D13).
- **Almost:** pure relocations + store-source swaps, no behavior change
  intended — awaiting operator playtest (Settings all categories + per-system
  drill-in + System Health; per-game dialogs; Library Manager; Import Wizard)
  before merge to main.
- **Next:** Phase 4 — typed `platform/api/` Tauri bridge (corral 157 raw
  `invoke()` calls + a `no raw invoke() outside platform/api/` rule). The
  platform/engine/theme separation is now fully lint-enforced; raw `invoke()`
  is the last coupling.
