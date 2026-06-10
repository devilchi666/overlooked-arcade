# Theming Substrate — Session Log

Each session that touches this feature appends a 3-line entry:
**Shipped / Almost / Next.** Older entries roll to
`SESSION_LOG_ARCHIVE.md` when this file passes ~150 lines
(rolled 2026-06-09 — entries before the grab-bag drain live there).

---

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

---

## 2026-06-09 — Phase 4 Slice 1: `settingsApi` (typed Tauri bridge)

- **Shipped:** Created `frontend/src/platform/api/settingsApi.ts` — the first
  `platform/api/<domain>Api.ts` module (28 typed wrappers, one named export per
  command, command string lives only here) — and migrated the
  video-display / audio / system-settings / per-game-overrides / shell-mode +
  kiosk + presentation-mode cluster across **13 files** on
  `feat/theming-platform-api-settings` (off main): App.tsx (launch AV-override
  push + revert-to-defaults + direct-launch fullscreen + shell-mode hydrate),
  `settings/store.ts`, `lib/audio.ts`, `layout/state.ts`, `shader_presets.ts`,
  QuickSettings, GameDialogs, GamePropertiesDialog, perSystemSections,
  AnalogBindingsSection, SettingsSections, SystemDialogs, PerSystemSettingsBody.
  Import style: named imports where no collision; `import * as settingsApi`
  namespace alias in the four files whose local signal setters / exports shadow
  the wrapper names (App.tsx, settings/store, audio, layout/state). `VideoState`
  (the `get_video_state` return) lifted into the api module as its canonical
  home; QuickSettings imports it now. typecheck + lint green; every migrated
  command string greps to **only** `settingsApi.ts` (the sole other matches are
  one index.css doc comment — updated — and 6 diagnostic label strings in
  App.tsx's revert array, which are `[label, promise]` tuples, not `invoke()`
  bindings). The Slice 6 lint rule is NOT on yet (per plan).
- **Three judgment calls:**
  - *Shape-divergent getters stay generic.* `get_game_overrides` /
    `get_system_settings` are typed differently at each call site (each file
    declares only the fields it reads — `SysSettings` / `GameOver` /
    `PerSystemOverrides` / `{analogRouting?}` / the GameDialogs supersets).
    Rather than force one canonical type (which would over/under-constrain
    callers), the wrappers are `getGameOverrides<T = GameOverrides>(id)` /
    `getSystemSettings<T = SystemSettings>(systemId)` — every call site keeps
    its exact local view via the type arg, zero type churn, no `any`. Canonical
    `GameOverrides` / `SystemSettings` / `OverscanCropPrefs` / `VideoState`
    defined + exported in the api module as the backend-contract home.
  - *Views/layout stay out of Slice 1.* `layout/state.ts` calls both the
    presentation/kiosk commands (settingsApi) and `get/set_layout` (viewsApi,
    Slice 2) — but they're **separate call sites**, not entangled, so only the
    presentation/kiosk trio migrated; `get/set_layout` left raw for Slice 2.
  - *Surfaced + fixed a latent bug.* AnalogBindingsSection's `get_game_overrides`
    passed `{ gameId }`, but the backend command's arg is `id` — so the call
    silently errored (caught + logged "analog prefs fetch failed") and per-game
    analog routing fell back to empty `{ ports: [] }`. The typed wrapper sends
    the correct `{ id }`, fixing it. Flagged to operator (the one place this
    slice changes runtime behavior; everything else is a pure pass-through).
- **Merged:** ✅ operator playtested + merged to main 2026-06-09 (merge
  `a5997e3`, together with Slice 2). Behavior-preserving everywhere except the
  AnalogBindingsSection bug fix above.
- **Next:** Slice 2 — `libraryApi` + `collectionsApi` + `viewsApi` (the
  store-heavy core: library/store, customCollections, ingest, views/store +
  the `get/set_layout` calls left in layout/state, App.tsx library paths;
  ~55 sites). Same convention.

---

## 2026-06-09 — Phase 4 Slice 2: `libraryApi` + `collectionsApi` + `viewsApi`

- **Shipped:** Three more `platform/api/` modules on the same branch
  (`feat/theming-platform-api-settings` — the branch holds the whole Phase 4
  arc, one-branch-per-arc). **libraryApi** (games / folders / groups /
  migration — 26 wrappers), **collectionsApi** (7 wrappers), **viewsApi**
  (4 wrappers: get/set_views + get/set_layout). Migrated **8 files**:
  `library/store.ts` (13 commands — `invoke` import fully removed),
  `customCollections.ts` (7 — removed), `views/store.ts` (4 — removed),
  `layout/state.ts` (the get/set_layout left from Slice 1 — `invoke` now gone
  there too), `settings/store.ts` (5 folder commands; `invoke` stays for the
  Slice-5 `set_rewind_config`), App.tsx (get_game / directory_is_empty /
  set_watched_folders / find_game_id_by_path via `import * as libraryApi`),
  ImportWizard (6 folder commands), routes/GameDetailPanel
  (update_game_core_override — confirms routes->platform/api is allowed).
  Named imports throughout (no collisions this slice); namespace alias only in
  App.tsx (consistency with its settingsApi alias). typecheck + lint green;
  every migrated command string greps to ONLY its api module (zero leaks —
  cleaner than Slice 1, no stray labels/comments).
- **Two judgment calls:**
  - *Same generic-getter pattern (D14) for two more shape-divergent commands.*
    `list_folders` / `add_folder` return `LibraryFolderRow` in the settings
    store but the richer `Folder` in the import wizard; `get_layout` returns
    `LayoutPrefs` in layout/state but a narrow `{ systemOrder }` in views/store.
    Wrappers are generic with a canonical default (`listFolders<T = LibraryFolderRow>`,
    `getLayout<T = LayoutPrefs>`), each call site keeps its view via the type arg.
  - *`ingest.ts` left untouched* despite the plan's Slice-2 file list. Its
    commands are `start_background_scan` (jobsApi/Slice 6), `list_cores`
    (coresApi/Slice 4), and the mame trio `lookup_mame_game` /
    `lookup_mame_title` / `set_game_mame_metadata` (mediaApi/Slice 3) — none
    belong to library/collections/views. Assign-by-concern wins over the file
    list (same discipline as Slice 1's `set_rewind_config` etc.).
- **Merged:** ✅ operator playtested + merged to main 2026-06-09 (merge
  `a5997e3`, together with Slice 1). Behavior-preserving throughout (no
  AnalogBindingsSection-style latent bug surfaced this slice — all arg names
  were already consistent across call sites).
- **Next:** Slice 3 — `mediaApi` (art/metadata sync + game-info + mame + hashes;
  media.tsx, platformMedia, gameInfo, MediaSettings, ImportWizard art paths,
  and ingest.ts's mame trio). ~45 sites.
