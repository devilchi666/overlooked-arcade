# Theming Substrate — Session Log (Archive)

Rolled-over entries from `SESSION_LOG.md` (it keeps only the most
recent few per its ~150-line policy). Reference-only; newest history
lives in `SESSION_LOG.md`. Entries here are oldest-first.

---

## 2026-06-06 — Planning locked

- **Shipped:** Full plan + feature folder scaffold. Plan at
  [docs/PLANS/theming-substrate.md](../../PLANS/theming-substrate.md);
  feature folder this directory. Operator decisions: one unified
  premium frontend (no LaunchBox/BigBox split); engine vs theme
  territory inside one window; engine summon = fullscreen takeover,
  top-right corner, `F12` / `Select+Start`; manifest = TOML; Kiosk
  plan's 4-layer substrate absorbed (renamed). 3-arc structure
  (ARC 1 = Minimum Viable Substrate, ~22-26 weeks; ARCs 2-3 add
  Rhai + WGSL + Theme Studio).
- **Almost:** Nothing — pure planning session, no code touched.
- **Next:** Phase 1 of ARC 1 — engine/theme surface separation.
  Extract SETTINGS + Library Manager + Import Wizard + BIOS +
  Core installer + System Health + Background Jobs from
  Retroverse into engine-owned fullscreen takeover. Write
  SURFACES.md as part of the phase. See plan §6 Phase 1 for the
  full deliverable list + acceptance gate.

---

## 2026-06-06 — ARC 1 Phase 1 shipped

- **Shipped:** Engine/theme surface separation per plan §6 Phase 1.
  Branch `feat/theming-substrate-phase-1` cut from
  `5695adb`; snapshot tag `v0.x-pre-theming-substrate` + branch
  `pre-theming-substrate` both at the same commit as restore points.
  - **SURFACES.md** written first (scope-lock checkpoint) — surface-by-
    surface engine/theme/platform territory map + 5-dialog migration
    table + 3 summon affordances + residual ~12 dialog signals
    deferred to Phase 2.
  - **`platform/engineSurface.ts`** — engine surface visibility signal
    (`engineSurfaceOpen`, `openEngineSurface`, `closeEngineSurface`,
    `toggleEngineSurface`) + `wireEngineSummonChord()` for the
    Select+Start chord recognizer (600ms window, respects setNavEnabled).
  - **`platform/dialogs.ts`** — 5 dialog signals migrated out of
    App.tsx createSignals: `savesEntry`, `contextMenuFor`,
    `gameInfoFor`, `helpDialog`, `wizardOpen`. Each exports a Solid
    Accessor + a value-form setter. App.tsx destructures them so all
    existing call sites read + write through identical names.
    Per operator decision 2026-06-06: Platform owns open/close; themes
    pick where dialogs anchor. Phase 1 ships the state migration;
    theme-chosen anchors land in Phase 6 with Retroverse-as-theme.
  - **`engine/SettingsPanel.tsx`** lifted from
    `routes/retroverse/SettingsPage.tsx` — identical UX, same
    three-pane layout, same 14 category bodies + per-system drill-in
    picker. Still pulls `settings` via `useRetroverse()` for Phase 1
    (engine surface mounts inside RetroverseProvider while only one
    theme exists; Phase 2 splits to PlatformProvider).
  - **`engine/EngineManagerSurface.tsx`** — z-[60] fullscreen takeover
    rendered when `engineSurfaceOpen()`, with header bar (back button
    + "OA Settings" label) + body hosting `SettingsPanel`. Escape /
    back button / F12 close.
  - **`engine/EngineSummonIcon.tsx`** — gear-icon button themes mount
    in their top-right slot per D3.
  - **App.tsx** wired all three summon affordances: F12 hotkey in
    existing keydown handler (toggles when engine-open OR not gaming;
    falls through to emu-thread screenshot when game is running and
    surface is closed); Select+Start chord via
    `wireEngineSummonChord()` at mount; `<EngineManagerSurface />`
    mounted inside `<RetroverseProvider>` after the conditional Show so
    the surface stays summonable across gameplay.
  - **RetroverseShell** dropped SETTINGS tab (6 → 5 tabs:
    HOME / LIBRARY / COLLECTIONS / PLAY NOW / DISCOVER), mounted
    `<EngineSummonIcon />` in top-right cluster next to clock + quit +
    profile chip, gated L1/R1 tab-cycler on `!engineSurfaceOpen()` so
    L1/R1 inside Settings doesn't bleed through, profile chip click now
    opens engine surface (was: routed to SETTINGS tab).
  - **`routing/currentRoute.ts`** dropped the `"settings"` arm from
    `RetroverseRoute` + `RETROVERSE_ROUTES`. Header comment updated
    to "5 top-toolbar tabs."
  - `routes/retroverse/SettingsPage.tsx` deleted (orphaned after the
    lift).
  - Acceptance gate green: `cargo test -p oa-shell` 744 pass /
    0 fail; frontend `npm run typecheck` silent; SURFACES.md
    locked the boundary before refactor started.
- **Almost:** Operator playtest — F12 / chord / corner-icon round-trip
  + per-system drill-in equivalence + visual regression vs old SETTINGS
  tab is the pending validation step before merging to main.
- **Next:** Phase 2 of ARC 1 — Platform/Theme SDK foundation. Pull
  `frontend/src/platform/` out as a top-level dir with the
  `@oa/platform` Vite alias; move the stores + lib helpers + theme
  registry in; carve `ThemeContext` (rename of `RetroverseContext`);
  cleanup `HOTSPOT_SYSTEMS` triplicate + `customComponent` orphan;
  migrate residual ~12 dialog signals listed in SURFACES.md "Open
  boundary questions" section. Phase 1-2 run parallel with VL Phase A
  per plan §7; pause at end of Phase 2 for VL Phase E + C.

---

## 2026-06-06 — ARC 1 Phase 2 Slice A shipped

- **Shipped:** Platform foundation + cleanup half of Phase 2 per plan
  §6 Phase 2. Branch `feat/theming-substrate-phase-2-slice-a` cut from
  main post-Phase-1 merge. Five tightly-scoped changes:
  - **Dead-letter purge:** `settingsDialog` signal in App.tsx had zero
    open call sites (verified via grep) — only the 4 close handlers +
    4 dialog mounts fired. Pre-Retroverse leftover. Removed the
    signal, the 4 mount blocks, and deleted
    `frontend/src/components/SettingsDialogs.tsx` (376 lines). App.tsx's
    `ShellMode` import re-routed from the deleted file straight to
    `settings/store.ts` where the type actually lives.
  - **HOTSPOT_SYSTEMS triplicate collapse:** added
    `touchInputSupported?: boolean` to `SystemUIConfig`; set `true`
    on `nds`. Three inline `Set<SystemId>(["nds"])` sites
    (TouchHotspotOverlay:50, QuickSettings:1672, StylusOverlay:39 —
    last under name `STYLUS_SYSTEMS`) replaced with a shared
    `isTouchSystem(id)` helper reading from
    `systemUIConfigs[id]?.touchInputSupported`. Single source of
    truth; future stylus-vs-aim splits add a finer field then.
  - **customComponent orphan delete:** `SystemUIConfig.customComponent`
    field at systemUIConfigs.ts:119 had zero consumers (verified
    via grep). Plan §6 Phase 2 said delete in favor of Phase 3's
    `custom` nav primitive — done. Field + the Vectrex assignment
    removed.
  - **`@oa/platform` alias:** added `resolve.alias` in vite.config.ts
    + `baseUrl` / `paths` in tsconfig.json. New
    `frontend/src/platform/index.ts` barrel re-exports
    `engineSurface` + `dialogs` as namespaces. ESLint boundary
    enforcement deferred to Phase 4 per plan §6 Phase 4.
  - **Residual 10 dialog signals migrated:** extended
    `platform/dialogs.ts` with `coreMenuFor`, `regionPickerFor`,
    `propertiesFor`, `collectionDialogMode`, `gameDialog`,
    `quickSettingsOpen`, `screenshotGalleryFor`, `systemContextFor`,
    `containerContextFor`, `systemDialog` (+ `openSystemDialog`
    convenience preserving the existing call shape). All 15 dialog
    signals from SURFACES.md now Platform-owned; dialog COMPONENTS
    still mount from App.tsx (Phase 6 splits mount per theme). Type
    imports from components/ into platform/ are pragmatic — Phase 4
    ESLint catches any runtime escape; Slice B / later phases that
    move dialog components into platform/ eliminate the type-only
    crossing.
- **Almost:** Operator playtest — sanity-check that Settings,
  Library Manager, Import Wizard, BIOS, Cores, System Health,
  Background Jobs editor, per-game dialogs (saves / info / context-
  menu / properties / region picker / core picker / screenshot
  gallery), sidebar context menus (system + container), per-system
  bindings/core-options, quick settings overlay, and the new-
  collection / rename-collection dialogs all still open + close
  unchanged. Migration is mechanical (signal-creation site moved,
  type unchanged) but the surface area is wide.
- **Next:** Phase 2 Slice B — store + component moves into
  `platform/` (settings/store.ts, library/store.ts, layout/state.ts,
  views/store.ts, library/customCollections.ts, lib/* helpers,
  themes/registry.ts, themes/systemUIConfigs.ts, LibraryTile,
  LibraryView, LeftSidebar, perSystemSections). Mass import-path
  rewrites — coordinated as one commit to keep the diff reviewable.
  Then Slice C: ThemeContext rename (RetroverseContext →
  ThemeContext), Theme manifest TOML schema, ESLint boundary rule
  (lands in Phase 4 alongside Tauri-bridge work). Phases 1-2 still
  on track to run parallel with VL Phase A.

---

## 2026-06-07 — ARC 1 Phase 2 Slice B shipped

- **Shipped:** Store + lib + themes batch moves into `platform/` per
  plan §6 Phase 2's mid-section. Branch `feat/theming-substrate-phase-2-slice-b`
  cut from main post-Slice-A merge. Component moves + ThemeContext
  rename + Theme manifest deferred to Slice C to keep this PR's diff
  reviewable.
  - **Batch 1: `lib/` → `platform/lib/`** — 8 files (audio,
    backgroundJobs, dataDir, eventListener, logbridge, reducedMotion,
    retroverseFlag, toast); ~22 importers rewritten via `sed`
    pattern, 4 missed sites (the `./lib/...` same-dir form in App.tsx
    + index.tsx + a dynamic `import()` in SettingsSections) fixed
    by hand.
  - **Batch 2: `themes/` → `platform/themes/`** — 4 files (registry,
    systemUIConfigs, systemUiSound, systemBootAnimation; `systems.css`
    stays in `src/themes/` because it's a CSS bundle imported by
    `index.css`, not a module). 60 importers rewritten — biggest
    single-move blast radius in the slice.
  - **Batch 3: stores** — `settings/` (2 files), `library/` (11
    files), `layout/state.ts`, `views/` (5 files). 45 importer files
    rewritten via a combined `sed` pass covering both `(\.\./)+`
    and `./` relative-import forms. Empty source dirs removed via
    `rmdir`.
  - **`SidebarView` type extracted** — was defined in
    `layout/LeftSidebar.tsx` (component file) and consumed by 6
    files including platform code (`platform/library/filter.ts`).
    Extracted to new `platform/layout/types.ts` per the "platform
    types live in platform" principle; 4 importer files updated to
    the new path; LeftSidebar.tsx now re-imports the type from
    platform (component itself moves in Slice C).
  - All imports rewritten to `@oa/platform/*` alias (no relative
    paths like `../../../platform/foo`). Consistent destination
    regardless of importer depth — easier to bulk-rewrite, easier
    to read.
  - Acceptance gate green: `cargo test -p oa-shell` 790 pass / 0
    fail (no Rust changes; Slice B is frontend-only); frontend
    `npm run typecheck` silent.
- **Almost:** Operator playtest — sanity-check that everything in
  the LIBRARY tab still works (the LeftSidebar's path to the
  platform-located LayoutStore + SidebarView type touches the
  hottest UI surface in the app). Touches every store consumer so
  there's no "isolated change" — every dialog, every page, every
  setting flow exercises the move.
- **Next:** Phase 2 Slice C — shared component moves + ThemeContext
  rename + Theme manifest TOML schema:
  - Move `components/LibraryTile`, `components/LibraryView`,
    `layout/LeftSidebar`, `layout/SidebarTreeNode`, `layout/Dialog`,
    `components/perSystemSections` into `platform/components/`.
  - Rename `RetroverseContext` → `ThemeContext`, `RetroverseProvider`
    → `ThemeProvider`, `useRetroverse()` → `useTheme()`. Pure rename
    + import-path update.
  - Define `ThemeManifest` TypeScript type in `platform/theme/`
    (the SDK layer). No actual `.oatheme` files ship in ARC 1 —
    Phase 5 wires the loader.
  - ESLint boundary rule deferred to Phase 4 alongside Tauri-bridge
    hardening per plan §6 Phase 4.

## 2026-06-07 — Phase 2 Slice C: component moves + ThemeContext rename + ThemeManifest

- **Shipped:** ARC 1 Phase 2 Slice C on
  `feat/theming-substrate-phase-2-slice-c`. Phase 2 is now complete
  (modulo the ESLint boundary rule, operator-deferred to Phase 4).
  - **13 component moves into `platform/components/`** — the 6
    declared (`LibraryTile`, `LibraryView`, `perSystemSections`,
    `Dialog`, `LeftSidebar`, `SidebarTreeNode`) plus the 7-file
    private sub-component cluster (`DiscPickerDialog`,
    `DetailListView`, `GridControls`, `VirtualLibraryGrid`,
    `SystemHeader`, `SidebarMigrationBanner`, `SettingRow`) per
    operator decision this session: the cluster was already
    platform-clean (imports only `@oa/platform/*`), and moving it
    avoids ~7 wrong-direction `../../components/*` reach-backs from
    platform code. All external importers rewritten to
    `@oa/platform/components/<Name>` (sed sweep, ~23 import sites).
  - **Residual wrong-direction edges (known, deliberate):**
    `platform/components/*` → `../../nav/{focus,back,HintBar}` (nav
    becomes platform-owned when Phase 3 builds the nav primitives)
    and `SystemHeader` → `../../components/SystemCoresStrip`
    (drags the core-installer surface — engine territory, stays
    out). Grep `../../` under `platform/components/` to find them
    all at Phase 3/4 cleanup time.
  - **`RetroverseContext` → `ThemeContext` rename** —
    `ThemeContextValue` / `ThemeProvider` / `useTheme()` across
    11 files (context.tsx + App.tsx + 9 consumers). Pure identifier
    rename; file stays at `routes/retroverse/context.tsx` until
    Phase 5/6 extracts the theme entry point (header comment
    documents the lineage).
  - **`ThemeManifest` type** at `platform/theme/manifest.ts` —
    snake_case fields mirroring the `theme.toml` schema from plan
    §Phase 2 verbatim (id / name / version / schema_version /
    oa_version / entry / entry_export / default_route / routes /
    context_slots / required_engine_capabilities / reserves_corner),
    plus `ThemeContextSlot` + `ReservedCorner` unions. Type-only —
    no loader until Phase 5. Re-exported from the `@oa/platform`
    barrel; barrel header comment refreshed.
  - Acceptance gate green: `cargo test -p oa-shell` 790 pass / 0
    fail (frontend-only slice); `npm run typecheck` silent.
- **Almost:** Operator playtest — same blast radius as Slice B
  (LIBRARY tab + every Dialog-based modal + per-system settings
  sections). Then merge closes Phase 2.
- **Next:** Per plan §7 sequencing, ARC 1 **pauses** at end of
  Phase 2: VL Phase E (game_identities schema) + VL Phase C
  (Launcher trait) land before theming Phase 3 resumes. Next
  theming work when resumed: Phase 3 — palette JSON extraction +
  asset resolver generalization + 5 nav primitives + toy second
  theme.

## 2026-06-09 — Boundary enforcement Slice 1 (lint + first fix)

Operator's explicit goal: a **clear, enforced** platform/theme separation
"so we can add to the platform or theme without accidentally wiring them
back together with new features or fixes." Reframed the remaining ARC-1
decoupling work around enforcement-first (DECISIONS D8/D9/D10), inverting
the plan's Phase-3-first order. A code audit found the structure isn't yet
clean enough to draw the full line — `components/` is a 48-file grab-bag
mixing engine/platform/theme — so this slice locks the already-clean edges
and tracks the rest. Branch `feat/theming-boundary-enforcement`.

- **Shipped:** ESLint boundary linter stood up (frontend had none).
  `frontend/eslint.config.mjs` — boundary-ONLY (no style rules), flat
  config, `import/no-restricted-paths` zones enforcing **platform↛routes**
  + **platform↛engine** (the platform foundation never depends on anything
  above it). `npm run lint` script added. Fixed the one live violation: the
  bootless feature's `SystemHeader → useTheme` reverse leak — SystemHeader
  now takes `onBootWithoutGame?` as a prop, threaded LibraryPage →
  LibraryView → SystemHeader (D10 prop-driven pattern). Lint + typecheck
  both green. Full layer contract + the deferred edges documented in
  [SURFACES.md](SURFACES.md) §"Layer boundary contract".
- **Almost:** Slice 1 complete + green; awaiting an operator playtest that
  the "Boot without game" button still works (behavior unchanged — pure
  refactor of where the handler comes from).
- **Next (Slice 2):** drain the `components/` grab-bag (48 files) into
  `engine/` vs `platform/` vs theme, batch by batch, tightening the lint as
  it shrinks — first targets: relocate Settings content
  (`PerSystemSettingsBody`/`SystemHealthPage`/`SettingsSections`) into
  `engine/` to close the `engine↛routes` edge, and move `SystemCoresStrip`
  into `platform/components/` to close `platform↛components`. Then Phase 4
  (typed `platform/api/` Tauri bridge) corrals raw `invoke()`.

## 2026-06-09 — CI wiring + Slice 2 batch 1 (close platform↛components)

- **Shipped:**
  - **CI now runs the boundary lint.** `.github/workflows/ci.yml` runs
    `npm run lint` (before the cargo build, fast-fail) so a cross-layer
    import fails CI on every push to main + PR — the enforcement actually
    bites now, not just locally.
  - **`platform/**` ↛ `components/**` enforced** (third zone). Closing it:
    `git mv` `SystemCoresStrip` + `CatalogCoreCard` → `platform/components/`
    (platform UI that was still in the grab-bag; updated the 2 importers),
    and extracted the dialog-*state* types
    (`GameDialogKind`/`GameDialogState` from GameDialogs,
    `CollectionDialogMode` from NewCollectionDialog, `SystemDialogSection`
    from SystemDialogs) into `platform/dialogs.ts` — platform owns dialog
    state, so the state-shape types belong there; the component files import
    them back (components→platform, allowed). This removed platform/dialogs.ts's
    last type-only crossing into the grab-bag. **Platform is now pure of
    theme, engine, AND the grab-bag.** lint + typecheck green.
- **Almost:** nothing half-done. Operator playtest is the usual
  same-behavior check (the cores strip + the create-collection / per-system
  / per-game dialogs are pure relocations).
- **Next (Slice 2 batch 2 — the keystone):** close `engine↛routes`. The
  blocker is the **store-context split**: `engine/SettingsPanel` pulls its
  content from `routes/retroverse/{PerSystemSettingsBody,SystemHealthPage}`
  + `components/SettingsSections`, and those read platform stores via
  `useTheme()` (the ThemeContext bundles platform stores + theme concerns).
  To relocate that content into `engine/` cleanly, split a
  platform-level `usePlatform()` provider (settings/library/layout/views
  stores) out of `useTheme()` so engine + platform components get stores
  without importing the theme context. That's the next major batch; it
  also unblocks the bulk of the remaining grab-bag drain (any
  ctx-store-reading component can then move out of theme). Then Phase 4
  (typed `platform/api/`).

## 2026-06-09 — Slice 2 batch 2: store-context split (the keystone)

- **Shipped (DECISIONS D11):** `platform/platformContext.tsx` —
  `PlatformProvider` + `usePlatform()` exposing the platform stores
  (library / customCollections / layout / views / settings) + shared state
  (searchQuery / focusedEntry / currentView), theme-agnostic. App.tsx now
  wraps the tree in `<PlatformProvider>` (around ThemeProvider + the engine
  surface + trailing modals) from the SAME store instances ThemeProvider
  uses — so theme code's `useTheme().settings` is untouched while engine/
  platform code can read stores without importing the theme context.
  First consumer migrated: `engine/SettingsPanel` (`useTheme()` →
  `usePlatform()`; it read only `ctx.settings`). That engine file no longer
  imports `routes/retroverse/context`. lint + typecheck green.
- **Almost:** the keystone is in, but `engine↛routes` isn't enforced YET —
  SettingsPanel still imports its CONTENT (`PerSystemSettingsBody` +
  `SystemHealthPage` from routes/, `SettingsSections` from components/).
  Those relocate next.
- **Next (Slice 2 batch 3):** relocate the Settings content into `engine/`,
  migrating each file's store reads to `usePlatform()`:
  `PerSystemSettingsBody` (0 useTheme — trivial), `SystemHealthPage`
  (1 × ctx.library), `SettingsSections` (stores + 5 app-action handlers —
  the handlers either come as props from the engine surface or call
  platform/dialogs setters directly). Then add + enforce the `engine↛routes`
  lint zone. After that, the rest of the grab-bag drains the same way
  (usePlatform unblocks every ctx-store reader), then Phase 4 (typed
  `platform/api/`).

## 2026-06-09 — Slice 2 batch 3: close engine↛routes (4th enforced zone)

- **Shipped:** relocated the engine surface's Settings *content* out of
  `routes/retroverse/` into `engine/` — `PerSystemSettingsBody`,
  `PerSystemInfoSection`, `SystemHealthPage` (all confirmed engine-only
  content, not Retroverse router pages). `SystemHealthPage`'s
  `useTheme().library` → `usePlatform()`; `SettingsPanel` imports the trio
  from `./` now. Their remaining cross-layer imports are `components/`
  (SettingsSections / SystemDialogs / SystemReadinessChecklist) — a
  separate, not-yet-enforced edge — NOT routes/, so `engine↛routes` is
  clean. Added + enforced the `engine/** ↛ routes/**` lint zone.
  **Four zones now enforced + green:** platform↛routes, platform↛engine,
  platform↛components, engine↛routes. lint + typecheck silent.
- **Almost:** engine surface is now theme-free; the engine↛components edge
  remains (the engine-manager surfaces still in the grab-bag).
- **Next:** continue the grab-bag drain — relocate the engine-manager
  surfaces (`SettingsSections`, `CoresPage`, `LibraryManagerPage`,
  `ImportWizard`, `DebugLogDialog`, `SystemDialogs`, the dialogs cluster…)
  into `engine/`, migrating each `useTheme()` store read to `usePlatform()`
  and resolving SettingsSections' 5 app-action handlers (props from the
  engine surface, or direct platform/dialogs setters). Then add the
  `engine↛components` + classify-the-rest zones. Finally Phase 4 (typed
  `platform/api/` Tauri bridge corrals the 157 raw invoke() calls).

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

---

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

## 2026-06-10 — Phase 3 design conversation: two vision corrections + skeleton-first resequence (no code)

- **Shipped (design only):** locked the Phase 3 shape before any code. Three
  outcomes, recorded as DECISIONS **D19** + **D20** and plan **§13**:
  - **D19 — per-system theming is a Retroverse feature, NOT a substrate
    contract.** Operator correction: the substrate's whole job is **swappable
    whole-shells** (BigBox-style), not a per-system-identity mandate. Per-system
    data stays platform-provided; *consuming* it is each theme's choice. Palette
    pillar becomes theme-first; the §6 "theme vs per-system precedence" question
    drops from thorny to low-stakes plumbing.
  - **D20 — kiosk/cabinet capabilities are deferred platform features.** Attract,
    CRT/shader chrome, multi-monitor (marquee/manuals/second-controls) are
    engine-owned platform toggles a shell opts into via the manifest
    `required_engine_capabilities` field — out of scope until ARC 2-3. **Two cheap
    seams reserved in ARC 1:** (a) theme-host lifecycle written as a *general*
    "platform preempts + restores the theme" pattern (so attract slots in free —
    same lifecycle as the F12 engine takeover), (b) manifest declares named
    **surfaces**, ARC 1 honoring exactly one (`main`). CRT/shaders need nothing.
  - **Skeleton-first resequence** of ARC-1 execution (ARC boundaries unchanged):
    pull the vertical slice forward — S1 nav foundation → **S2 walking skeleton**
    (Retroverse + rough **Wheel** switchable, the morale/de-risk swap gate) → S3
    token layer (+ `THEME_CONTRACT.md`) → S4 versioned manifest + validator → S5
    substrate depth. Replaces §6's save-the-proof-for-Phase-6 order.
  - Forward-looking scope calls table (plan §13.2): build-now = token layer
    (#1+#3, engine-scoped) + manifest/validator (#2+#7); seam-now = glyph (#4),
    audio category (#6), declarative props (#8), settings namespace (#9);
    decide-now = precedence (#5, done via D19); defer = hot-reload (#10).
- **Almost:** nothing — design only. No code, no branch.
- **Next:** **S1 — nav foundation.** Lock the verb vocabulary (start set:
  `Confirm`/`Back`/directional/`NextSection`/`PrevSection`/`OpenQuickSettings`/
  `Menu`; reserved `Search`/`Favorite`/`Page`), relocate `src/nav/` (back/focus/
  gamepad/HintBar/types) → `platform/nav/`, build the input→verb `navBindings`
  layer (OA-wide tier + `platform/api/` wrapper), ship `list`/`grid` primitives
  verb-native with declarative-config props. Defaults = operator-locked
  controller-nav spec.

