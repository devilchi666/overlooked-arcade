# Theming Substrate — Session Log

Each session that touches this feature appends a 3-line entry:
**Shipped / Almost / Next.** Older entries roll to
`SESSION_LOG_ARCHIVE.md` when this file passes ~150 lines.

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
