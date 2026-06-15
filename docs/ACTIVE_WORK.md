# Active Work Streams

Free-form list of what's in flight. Read the linked stream's README + recent
SESSION_LOG entry to pick up where the last session left off.

Replaces the older `docs/ACTIVE_CORE.md` (single-string "which core is active")
because cross-cutting work didn't fit that model — the 2026-05-22 sidebar work
spanned every system but was filed under whichever core happened to be active.

---

## In flight

- **Unified Navigation & Panel System** — pivoted 2026-06-14 from the per-panel
  Controller-Nav Coverage sweep (operator: per-panel wiring doesn't scale; most
  engine panels stayed inert). Two pillars: a **spatial-navigation engine**
  (universal focusable auto-discovery + geometry movement + layer scoping,
  reusing the Slice-1/2/3 activate layer) **+** a unified, input-agnostic panel
  structure/look (keyboard / controller / kiosk-arcade). Plan:
  [PLANS/unified-navigation-and-panels.md](PLANS/unified-navigation-and-panels.md);
  feature folder [features/unified-nav/](features/unified-nav/).
  - **Phase 1 ✅ shipped + merged to main 2026-06-14**
    (`feat/unified-nav-phase-1`). Pillar A spatial engine
    (`platform/nav/spatial.tsx` + pure `spatialGeometry.ts` + `sliderStep.ts`):
    global layer stack, native focusable auto-discovery (no markers),
    **region-bias hybrid** movement (UP/DOWN within a region, LEFT/RIGHT
    between — derived from `<aside>`/`<nav>` landmarks + a `data-nav-region`
    hook; matches the locked nav spec, DECISIONS D1), the Slice-1/2/3 activate
    layer reused, gamepad+keyboard routing, layer scoping. Proven on the whole
    Settings surface: `EngineManagerSurface` pushes one layer; `SettingsPanel`
    dropped its index groups; `Dialog` → `SpatialDialogLayer` when active;
    custom modals (Import Wizard / Game-media / Missing-cores) wrapped in
    `SpatialModalScope` + z-lifted. `focus.ts` bypasses the legacy index
    manager while a spatial layer is active. Three playtest rounds
    (collapsed-`<details>` focus trap, flat-plane darting, content-region
    stranding) resolved; D1–D4 in
    [features/unified-nav/DECISIONS.md](features/unified-nav/DECISIONS.md).
    typecheck/lint/vitest(97)/build green.
  - **Pillar B ✅ delivered** by the Per-System Settings Hub arc
    (`HubCard`/`HubGrid`/`PanelScaffold` + the engine-surface HintBar fix).
    **Remaining phases (not yet started):** Phase 2 — formalize the spatial
    scope into `Dialog` + `EngineManagerSurface` so every dialog inherits nav
    for free + retire `Dialog.navigate`'s marker dependence; Phase 3 — Retroverse
    tabs + thin adapters for the virtualized library grid + carousels; Phase 4 —
    kiosk/arcade limited-button input pass. Queued in [NEXT.md](NEXT.md).
  - **Predecessor — Controller-Nav Coverage (Slices 1–3, 2026-06-13/14):**
    Slice 1 (Settings row-nav: `useSettingsRowFocusGroup` + select-overlay +
    slider-adjust + Y-reset) and Slice 2 (engine dialogs via `Dialog.navigate`)
    **shipped + merged to main**. Slice 3 (ImportWizard) on
    `nav-coverage-slice3`, **folded** into the engine (reusable `selector`
    override + bare-button dispatch kept; per-panel markers superseded). OSK for
    text entry deferred ([features/nav-coverage/OSK_PLAN.md](features/nav-coverage/OSK_PLAN.md)).
    History: [features/nav-coverage/](features/nav-coverage/).

- **Theming Substrate (BigBox-style themes + engine/theme territory
  split)** — major multi-arc planned 2026-06-06. **4 arcs (D35 renumber):**
  ARC 1 (Minimum Viable Substrate) **complete** bar the `.oatheme` loader;
  **ARC 2 = Per-System Layout Substrate** (planned, L1 queued); ARC 3 =
  Cinematic & Scripting (Rhai + WGSL + declarative motion); ARC 4 = Theme
  Studio. Plans at [PLANS/theming-substrate.md](PLANS/theming-substrate.md)
  + [PLANS/theming-arc-2-per-system-layout.md](PLANS/theming-arc-2-per-system-layout.md);
  feature folder [features/theming-substrate/](features/theming-substrate/).
  - **Phase 1 ✅ shipped + merged 2026-06-06** (merge `870edb2`).
    Engine/theme surface separation: SETTINGS + Library Manager +
    Import Wizard + BIOS + Core installer + System Health +
    Background Jobs moved out of Retroverse's tab list (6 → 5 tabs)
    into an engine-owned fullscreen takeover summoned via F12 /
    Select+Start / top-right corner icon. New `engine/` + `platform/`
    directories; `EngineManagerSurface` + `SettingsPanel` +
    `EngineSummonIcon` + `platform/engineSurface.ts` +
    `platform/dialogs.ts`. 5 dialog signals migrated from App.tsx
    createSignals to Platform-owned store per operator decision
    (Platform owns open/close; themes pick anchors). Snapshot
    restore: tag `v0.x-pre-theming-substrate` + branch
    `pre-theming-substrate` both at `5695adb`. SURFACES.md is the
    boundary-doc deliverable.
  - **Phase 2 Slice A ✅ shipped + merged 2026-06-06** (branch
    `feat/theming-substrate-phase-2-slice-a`, merge `e400274`).
    Platform foundation + cleanup half of
    Phase 2: `@oa/platform` Vite alias + tsconfig paths +
    `platform/index.ts` barrel; HOTSPOT_SYSTEMS / STYLUS_SYSTEMS
    triplicate collapsed via new `touchInputSupported?: boolean`
    on `SystemUIConfig`; `customComponent` orphan field deleted;
    dead-letter `settingsDialog` signal + `SettingsDialogs.tsx`
    (376 lines) removed; 10 residual dialog signals migrated to
    `platform/dialogs.ts` (all 15 SURFACES.md-listed dialog
    signals now Platform-owned). 744 oa-shell tests pass;
    frontend typecheck silent. Snapshot restore: same
    `pre-theming-substrate` branch + tag still apply.
  - **Phase 2 Slice B ✅ shipped + merged 2026-06-07** (merge
    `0eb2f56`; operator playtest passed — boot / LIBRARY /
    theming / settings persistence / launch all clean).
    Foundational moves into `platform/`:
    `lib/` (8 files), `themes/` (4 of 5 — `systems.css` stays as a
    CSS bundle), settings/library/layout/state/views stores (19
    files). ~130 import sites rewritten to the `@oa/platform/*`
    alias. `SidebarView` type extracted from `layout/LeftSidebar.tsx`
    (component file) to new `platform/layout/types.ts` so platform
    code (`platform/library/filter.ts`) doesn't depend on a
    component module. 790 oa-shell tests pass; typecheck silent.
  - **Phase 2 Slice C ✅ shipped + merged 2026-06-07** (merge
    `e6b6568`; operator playtest passed). 13 component moves into
    `platform/components/` — the 6 declared (`LibraryTile`,
    `LibraryView`, `perSystemSections`, `Dialog`, `LeftSidebar`,
    `SidebarTreeNode`) + the 7-file private sub-component cluster
    (`DiscPickerDialog`, `DetailListView`, `GridControls`,
    `VirtualLibraryGrid`, `SystemHeader`, `SidebarMigrationBanner`,
    `SettingRow`) per operator decision; `RetroverseContext` →
    `ThemeContext` / `useTheme()` rename across 11 files;
    `ThemeManifest` type in `platform/theme/manifest.ts`. 790
    oa-shell tests pass; typecheck silent. **Phase 2 complete**
    modulo the ESLint boundary rule (deferred to Phase 4).
    **ARC 1 unblocked 2026-06-08** — plan §7's pause condition (VL
    Phase E + C land first) is now satisfied: both merged to main.
    Theming Phase 3 is ready to resume.
  - **Boundary enforcement track ✅ MERGED to main 2026-06-09**
    (`feat/theming-boundary-enforcement`; operator playtested each
    batch). Per operator ask for a *clear enforced* platform/theme
    separation, reframed the remaining ARC-1 decoupling
    enforcement-first (DECISIONS D8–D11 — inverts plan's Phase-3-first).
    Shipped: an ESLint **boundary-only** linter
    (`frontend/eslint.config.mjs` + `npm run lint`, wired into CI) +
    the **`usePlatform()` store-context split** keystone
    (`platform/platformContext.tsx`, D11). **Four zones enforced +
    green:** `platform↛routes`, `platform↛engine`, `platform↛components`,
    `engine↛routes`. Platform is fully isolated from theme/engine/
    grab-bag; the engine surface is fully theme-free (Settings content
    relocated into `engine/`). Layer contract +
    enforced/deferred edges:
    [features/theming-substrate/SURFACES.md](features/theming-substrate/SURFACES.md)
    §"Layer boundary contract".
    - **Grab-bag drain ✅ SHIPPED on `feat/theming-grabbag-drain`
      (2026-06-09) — awaiting operator playtest + merge.** The
      `src/components/` grab-bag is **fully drained and removed**: 38
      files + 2 subtrees split into `engine/` (manager surfaces) and
      `platform/components/` (shared per-game / in-game UI).
      SettingsSections shed its last `useTheme()` (stores →
      `usePlatform()`; 5 handlers → `@oa/platform/dialogs` setters +
      new `platform/libraryAdmin.ts` registry). **Six zones now
      enforced + green:** `platform↛{routes,engine,components}`,
      `engine↛{routes,components}`, `routes↛components`. Two judgment
      calls (DECISIONS D12/D13): shared leaves + background-jobs →
      platform; library-admin handlers → registry not props. Plan +
      log: **[PLANS/theming-grabbag-drain.md](PLANS/theming-grabbag-drain.md)**,
      theming-substrate SESSION_LOG 2026-06-09.
    - **Phase 4 — typed `platform/api/` Tauri bridge — IN FLIGHT
      (Slices 1-2 ✅ merged 2026-06-09, merge `a5997e3`; operator
      playtested).** The last platform/theme decoupling step: corral the
      raw `invoke()` calls (351 / 54 files / 222 command names at the
      start) behind typed `platform/api/<domain>Api.ts` wrappers + a
      `no raw invoke() outside platform/api/` lint rule (the rule turns on
      in the final slice). Plan + 6-slice order:
      **[PLANS/theming-platform-api-bridge.md](PLANS/theming-platform-api-bridge.md)**.
      - **Slice 1 (`settingsApi`)** + **Slice 2 (`libraryApi` +
        `collectionsApi` + `viewsApi`)** shipped on
        `feat/theming-platform-api-settings` (one-branch-per-arc) and
        merged together: **4 modules, 65 typed wrappers, ~95 call sites**
        across display/audio/settings + library/folders/groups/collections/
        views. Convention locked in DECISIONS **D14** (generic getters for
        shape-divergent commands; api layer owns the backend-contract type;
        assign-by-concern not by file). Surfaced + fixed one latent bug
        (AnalogBindingsSection `get_game_overrides` arg name). Also rolled
        the theming SESSION_LOG to `SESSION_LOG_ARCHIVE.md` (487→109 lines).
      - **Slice 3 (`mediaApi`)** ✅ merged 2026-06-09 (merge `f5657c2`;
        operator playtested). 28 wrappers across art/metadata sync +
        game-info + mame + hashes (11 files). DECISIONS **D15**
        (typed-binding modules move + re-export).
      - **Slice 4 (`coresApi` + `inputApi`)** ✅ merged 2026-06-10
        (`feat/theming-platform-api-cores-input`). 29 wrappers / ~50 call sites / 18
        files: installed cores + buildbot catalog + core-options + BIOS
        (coresApi), bindings + input descriptors + controller devices +
        analog routing + light-gun (inputApi). DECISIONS **D16** (the
        `platform↛components` boundary forces component-local
        backend-contract types to re-home INTO the api layer; generic
        `routing: R` blob for the analog cluster). typecheck + lint green;
        frontend-only (no Rust). One behavior touch: GameDialogs
        `get_controller_devices` now guards `!systemId` (was reachable as
        null; equivalent).
      - **Slice 5 (gameplay cluster)** ✅ merged 2026-06-10
        (`feat/theming-platform-api-gameplay`). Five modules / ~75 call sites / ~14
        files, landed as two commits (the planned two-PR split on one
        branch): **PR A** = `emulatorApi` (17) + `rewindTasApi` (15);
        **PR B** = `cheatsApi` (12) + `milestonesApi` (6) + `captureApi`
        (9). launch.ts stays a rich helper but routes through emulatorApi;
        GameDialogs fully drained of raw invoke; namespace imports where
        wrapper names shadow local handlers (QuickSettings TAS/video,
        GameDialogs cheats/milestones). typecheck + lint green; frontend
        only. 56 command strings each grep to only their api module. See
        SESSION_LOG 2026-06-10.
      - **Slice 6 (THE CLOSER)** ✅ merged 2026-06-10
        (`feat/theming-platform-api-jobs-system-shell`; operator playtested).
        `jobsApi` (18) + `systemApi`
        (9) + `shellApi` (19) + straggler folds (libraryApi prefs/unidentified,
        mediaApi clear-metadata); ~90 sites / 21 files. systemInfo.ts D15
        move+re-export; logic modules (backgroundJobs/audio/dataDir/logbridge)
        route through wrappers. **Turned ON the `no-restricted-imports` rule
        banning raw `invoke` outside `platform/api/`** (probe-verified it
        fires); every non-api file is now invoke-free. typecheck + lint green.
      - **✅ PHASE 4 COMPLETE (2026-06-10).** 14 typed `platform/api/` modules;
        the platform/theme decoupling track is closed at BOTH the file level
        (six boundary lint zones) and the API level (the invoke ban). A new
        feature physically cannot re-couple the layers without ESLint stopping
        the commit.
      - **Phase 4.5 — the EVENT corral** ✅ merged 2026-06-10
        (`feat/theming-platform-api-events`; operator playtested). Closed the
        symmetric coupling the Phase 4 audit flagged: Tauri event names. New
        `platform/api/eventsApi.ts` (`OA_EVENTS` registry + `listenScoped`/
        `listenTo`/`emitEvent`); ~30 sites / 16 files migrated (incl. a theme
        file that emitted `oa://toast` raw); a second `no-restricted-imports`
        entry bans raw `listen`/`emit`/`once` outside `platform/api/`
        (probe-verified). Every `oa://…` string now lives only in `OA_EVENTS`.
        DECISIONS **D17**. **The foundation is now clean on BOTH backend-contract
        channels (commands + events).**
      - **Two playtest fixes rode along (merged 2026-06-10, operator
        confirmed):** (1) the persistent BackgroundJobsBar was rendering at z-55
        *behind* the opaque engine surface (z-60) — invisible exactly where jobs
        are spawned (Settings/Cores/Import); lifted to z-65. (2) Native
        `window.confirm`/`alert` were both ACL-gated AND async-but-treated-as-sync
        under Tauri 2 — every `if (!window.confirm())` guard silently never fired,
        so destructive actions ran unconfirmed. Replaced all 13 confirm + 3 alert
        sites with an in-app awaitable `confirm()` (`platform/lib/confirm.ts` +
        `ConfirmHost`, themeable/controller-navigable via the Dialog primitive).
      - **Phase 3 design locked 2026-06-10** (design conversation, no code yet):
        DECISIONS **D19** (per-system theming is Retroverse-only, NOT a substrate
        contract — the substrate's job is swappable whole-shells; **superseded
        2026-06-15 by D32/D33 — per-system UI is now an ARC-2 substrate capability
        themes opt into**) + **D20**
        (kiosk/cabinet capabilities — attract / CRT-shaders / multi-monitor — are
        deferred platform features, with two cheap seams reserved: general
        preempt/restore theme lifecycle + named manifest `surfaces`). **ARC-1
        execution resequenced skeleton-first** (plan §13.3): S1 nav foundation →
        S2 walking skeleton (Retroverse + rough Wheel switchable — the swap gate)
        → S3 token layer → S4 manifest + validator → S5 substrate depth.
        Follow-on after S2: nav-remap Settings UI.
      - **Phase 3 S1 — nav foundation ✅ shipped + merged 2026-06-10
        (`feat/theming-nav-foundation`; operator playtested).** Verb-native
        nav layer: `src/nav/` relocated to `platform/nav/` (24 importers repointed
        to `@oa/platform/nav`; closes the Phase-2 residual wrong-direction edges) +
        verb vocabulary (`verbs.ts`) + OA-wide input→verb `navBindings`
        (`nav_bindings.json` persistence via `platform/api/navBindingsApi` + Rust
        `get/set_nav_bindings`) + `focus.ts`/`HintBar` dispatch/render by **verb**
        (A/B swap is now a binding overlay) + glyph-set seam (`glyphs.ts`) +
        declarative `list`/`grid` primitives + arrow-key keyboard nav + new
        `platform/nav ↛ platform/components` lint zone. DECISIONS **D21**. 822
        oa-shell tests pass; typecheck + lint green. **S2 (walking skeleton) is
        next.**
      - **Phase 3 S2 — walking skeleton (Retroverse ⇄ CoverFlow swap gate) ✅
        shipped + merged 2026-06-10 (operator playtested;
        `feat/theming-walking-skeleton`, merge `1fbec3e`).** The morale/de-risk
        milestone — two switchable whole-shells. Four design decisions signed off before code (all the
        recommended path). Shipped: a minimal **Theme SDK contract**
        (`platform/theme/{types,host,registry}` — a theme = `{manifest, entry}`,
        the entry a surface-aware `Component<{surface:"main"}>` consuming only
        platform); the **host context relocated** `routes/retroverse/context.tsx`
        → `platform/theme/host.tsx` (move+re-export, D15-style) so every theme
        shares the launch/saves/info host services; an **active-theme registry**
        (platform owns the signal + boot-seed + restart-switch, App injects the
        concrete `BUILTIN_THEMES` — platform↛themes, D13 pattern); `activeThemeId`
        persisted on **`LibraryPrefs.active_theme_id`**; **restart** via a new Rust
        `restart_app` (Tauri `AppHandle::restart()`, no plugin); **Retroverse** as
        the default theme (thin wrapper → existing `RetroverseShell`); a rough
        **CoverFlow** 2nd shell (full-bleed horizontal coverflow, system-agnostic
        per D19, on `useFocusGroup` + a windowed sliding track — renamed from
        "Wheel" per operator; a true radial Wheel is the S5 `wheel` primitive;
        honest caveat: layout/feel only, the cinematic layer is ARC 2-3);
        `EngineSummonIcon` **re-homed** to `platform/components/` (D12) so both
        themes mount it; `themePreempted()` (the general D20a preempt/restore
        seam); a `surfaces` manifest field (D20b); the active-theme mount isolated
        in its own stacking context (engine territory always overlays the theme);
        the **Appearance picker** filled into the existing OA-wide Themes Settings
        category. 6 new lint zones (incl. `themes↛engine` probe-verified). 822
        oa-shell tests pass; typecheck + lint green. DECISIONS **D22**. Playtest
        round-1 fixes rode along (z-index isolation, focus-claim, windowing,
        mouse/scroll). The nav-remap Settings UI stays the separate D18 follow-on.
      - **Phase 3 S3 — token layer (design-token contract) ✅ shipped + merged
        2026-06-10 (operator playtested; `feat/theming-token-layer`, merge
        `340c3fe`, DECISIONS D23).** Preceded by a **BigBox research round** (operator asked
        to align on what BigBox themes actually do before S3 — the cinematic/motion
        axis = animation engine / transitions / video snaps / attract / Theme
        Creator = **ARC 2-3**, not the token layer; operator kept S3 static).
        Shipped: a typed **`ThemeTokens`** contract (`platform/theme/tokens.ts`)
        **formalizing the existing** `index.css` CSS-var system (palette /
        typography / geometry; **motion reserved** for ARC 2); `ThemePackage.tokens`
        **scoped-injected** on the S2 theme-mount wrapper so engine territory always
        reads `:root` (the **D2 guarantee is structural** — sibling-scope, no
        namespace split); an a11y **`prefers-reduced-motion`** baseline +
        **`--oa-focus-ring`** token (NOT theme-overridable); **CoverFlow re-skinned**
        via a cool token set (same code, different tokens = different shell);
        Retroverse ships no tokens. New doc **`THEME_CONTRACT.md`** (theme-facing
        peer of SURFACES.md). typecheck + lint green; frontend-only. **Acceptance
        gate (operator):** CoverFlow reads cooler/cyan while Settings looks identical
        under both themes (D2); reduce-motion stills the UI. **After merge: S4 —
        versioned manifest + validator** (`bare` theme fixture). Remap Settings UI
        stays the D18 follow-on.
      - **Phase 3 S4 — versioned manifest + load-time validator (`bare` fixture) ✅
        shipped + merged 2026-06-10 (`feat/theming-manifest-validator`, merge `6fb0653`,
        DECISIONS D24; operator playtested).** Turns THEME_CONTRACT.md §6 from
        documented to machine-checked. Four design forks signed off (AskUserQuestion,
        all recommended) before code. Shipped: pure **`validateTheme(pkg)`**
        (`platform/theme/validate.ts`) over the **declarative** manifest+tokens surface
        — required fields / `schema_version` ∈ supported-set `{1}` ("newer schema —
        update OA" vs "unsupported" messages) / `surfaces` non-empty ⊆ `["main"]` /
        `required_engine_capabilities` ⊆ `ENGINE_CAPABILITIES` (**empty in ARC 1** →
        only `[]` validates) / `tokens` keys ∈ `TOKEN_VAR` + non-empty values (the data
        half of the §4 no-override rule); warnings for non-dir-safe `id` + `default_route`
        ∉ `routes`. **Registry gate** (`registry.ts`): validates-on-register, **excludes
        invalid themes** from the picker + `activeTheme()`, errors logged always /
        warnings DEV-only, + a **fallback toast** when a persisted `active_theme_id` is no
        longer a valid choice (e.g. wheel→coverflow). **`bare` theme** (`themes/bare/`, in
        `BUILTIN_THEMES`) — minimal valid whole-shell (ListNav of games + launch + engine
        icon, **no tokens**, ~110 LOC, system-agnostic): operator-selectable lowest-floor
        reference AND the validator's canonical fixture. **Vitest stood up** — the
        frontend's **first test runner** (the gate had to be TS: manifests are TS objects
        with no Rust visibility, D6); `vitest` + `jsdom` + `vitest.config.ts` (reuses
        vite-plugin-solid + the `@oa/platform` alias) + `npm run test` wired into CI; an
        `overrides:{vite}` pin dedupes vitest's nested vite. **25 tests pass** across
        `platform/theme/validate.test.ts` (15 pure unit, every code) +
        `themes/builtin-themes.test.ts` (10 — real themes validate clean; lives in
        `themes/` because validating real themes means importing them and `platform ↛
        themes` is forbidden). THEME_CONTRACT.md §6 rewritten (enforced-data /
        structurally-backed / deferred-gap). The no-`:root` rule stays **structurally**
        backed (sibling-scope + boundary lint), not runtime-enforced — a `<style>:root` /
        `document.head` / global-CSS bypass is invisible to a package-object validator
        (Phase-5/untrusted-author concern). typecheck + lint + test + build green;
        frontend-only (822 oa-shell tests unaffected). **Next: S5 — substrate depth.**
      - **Phase 3 S5 — substrate depth — sliced into 5 per-sub-area micro-slices**
        (operator choice 2026-06-11; design forks signed off via AskUserQuestion;
        order = contracts first). Plan §13.3 S5. **S5.1 (resolver theme tier) ✅
        shipped + merged 2026-06-11** (`feat/theming-s5-1-resolver-theme-tier`, merge
        `783da2e`, DECISIONS **D25**). Adds the active-theme tier to the two existing
        per-system resolvers — generalize/connect the shipped Per-System-UI machinery
        into the theme cascade, NOT rebuild it: `resolve_background_asset` +
        `resolve_ui_sound` gain a leading `themeId` + walk a shared theme→platform
        cascade (`system_ui_assets::candidate_asset_bases`: theme/<system> →
        theme/_baseline → system/<system> → system/_baseline → null; ui-sound keeps
        operator-override on top). Theme overrides home at
        `<exe_dir>/assets/themes/<id>/system-ui/<system>/<category>/` (operator-droppable
        now — no Phase-5 loader); a theme `_baseline` tier lets a system-agnostic theme
        (D19) ship one library-wide asset. Frontend api wrappers take `themeId`;
        dispatchers (`lib/audio.ts`, `SystemBackground`) resolve `activeThemeId()`
        ambiently → zero consumer churn. **830 oa-shell tests** (+8 cascade) +
        typecheck/lint/vitest(25)/build green. **Merged on the test basis** — the
        **background** consumer `SystemBackground` is unmounted since 2026-05-31 (dropped
        over a Retroverse visual conflict; zero JSX mount), so **reviving a theme-owned
        background surface folds into S5.5**; the **ui-sound** half is live (grid-nav
        sounds). **S5.2 (palette substrate) ✅ shipped + merged 2026-06-11**
        (`feat/theming-s5-2-palette-substrate`, merge `f5b9b61`, DECISIONS **D26**):
        retired hand-authored `systems.css` for a typed `SYSTEM_PALETTES` single-source
        map (`platform/themes/systemPalettes.ts`), global `[data-system]` baseline derived
        + injected at boot (`ensureSystemPaletteBaseline`, no flash); per-theme
        `perSystemTokens` scoped override seam (App.tsx `.oa-theme-mount [data-system]{}` —
        D19 sub-cascade, D2 sibling-scope); validator extended; THEME_CONTRACT.md §4/§6.
        Data-home = typed map not `config/*.json`+build-step (frontend-only data). `bare`
        reframed as the substrate **test bed** + the seam's live consumer (per-system accent
        dots + scoped NES→cyan/PSX→magenta demo). typecheck/lint/vitest(37)/build green;
        frontend-only. **S5.3 (glyph-set seam) ✅ shipped + merged 2026-06-11**
        (`feat/theming-s5-3-glyph-set`, merge `af13cb7`, DECISIONS **D27**): S1 verb→glyph
        indirection made theme-choosable — `PLAYSTATION_GLYPH_SET` + `GLYPH_SETS` + the
        `activeGlyphSet()` signal (HintBar reactive); loose manifest `glyph_set` bridged from
        App; validator `UNKNOWN_GLYPH_SET` warning; `bare` ships `glyph_set:"playstation"`.
        **S5.4 (per-theme settings namespace) ✅ shipped + merged 2026-06-11**
        (`feat/theming-s5-4-theme-settings`, merge `895f8c0`, DECISIONS **D28**): collision-free
        per-theme prefs (`themeSettings.ts` + `useThemeSettings()` auto-bound to the active
        theme's id; localStorage; reactive); `bare`'s "Compact" toggle the live consumer;
        THEME_CONTRACT §7. **S5.5 (primitives + background revival) ✅ shipped + merged
        2026-06-11** (`feat/theming-s5-5-primitives`, merge `105fad8`, DECISIONS **D29**):
        `CarouselNav` (CoverFlow dogfooded onto it) + `CustomNav` escape hatch + reserved
        `WheelNav` contract + `onNavSound` hook (#6) + `ThemeBackground` (revived dead
        `SystemBackground` → the S5.1 background tier's live consumer); shared `useLateClaim`
        + ListNav/GridNav scroll-into-view from the 3-round playtest. vitest 51.
        **✅ S5 COMPLETE — the Phase-3 substrate-depth arc is closed.** **D18 nav-remap
        Settings UI (gamepad) ✅ shipped + merged 2026-06-11** (`feat/theming-nav-remap-settings`,
        merge `2b40e3e`, DECISIONS **D30**): `NavRemapCard` in Settings → Controls rebinds the
        shell-nav verbs to standard-layout gamepad buttons (live/no-restart; conflict-by-steal;
        per-row + global Reset; F12/keyboard escape hatch). The **keyboard** nav-remap UI + a
        default keyboard map + the future **per-controller-id gameplay-binding auto-config** are
        queued in **PARKING_LOT.md** (2026-06-11) — the keyboard dispatch infra is already wired
        (`focus.ts:214`), only the editing UI is deferred. **Phase 3 is now fully closed.**
        Remaining ARC-1: original §6 Phase 5 (`.oatheme` on-disk distribution/loader) + Phase 6
        (full Retroverse-as-theme move).
      - **Phase 6 — Retroverse rebuilt as a real theme (the ARC-1 acceptance gate) ✅ shipped +
        merged 2026-06-11** (`feat/theming-retroverse-as-theme`, merge `711f337`, DECISIONS **D31**;
        operator playtested — indistinguishable). The dogfood: moved Retroverse from the S2 thin wrapper (D22.8) into a REAL theme
        physically under `themes/retroverse/` (RetroverseShell + 8 route files + theme-private
        `currentRoute.ts`), consuming ONLY platform, and **removed the last two boundary exceptions**
        (`themes↛routes` + `themes↛layout` `except: ['./retroverse']`, probe-verified to fire). The
        reverse-import audit found **zero files needing to hoist to platform** — the S2/Phase-4/
        grab-bag work had already hoisted everything shared, so Phase 6 was a pure relocation + one
        shim deletion. Deleted along the way: the `context.tsx` re-export shim (repointed to
        `@oa/platform/theme/host`), dead `StubPage.tsx`, and App.tsx's obsolete `__retroverse_debug`
        DevTools block (a platform-owned dev-console seam queued in PARKING_LOT). 3 green sub-commits
        (C1 sever shim · C2 relocate + delete dead code · C3 drop exceptions + probe);
        typecheck+lint+vitest(58)+build green; cargo `oa-shell` green; frontend-only. **Only §6
        Phase 5 (`.oatheme` distribution/loader) now remains open in ARC 1.**
      - **ARC 2 — Per-System Layout Substrate — L1 + L2a + L2b ✅ MERGED 2026-06-15.**
        **L2a (view/layout manifest contract)** (D37): `ViewType`/`LayoutPrimitive`
        enums + `views?` manifest field + validator; no consumer yet. **L2b (D34
        migration)** (D38): experiential per-system config → `themes/retroverse/`
        via `ThemePackage.perSystemUiConfigs` + App bridge; `touchInputSupported`
        split to platform-factual `systemSupportsTouch`; `uiConfigFor` merges over
        `BASELINE_UI`. Operator playtested visual-identical. **L3a (layout
        resolver + persisted override store) ✅ MERGED to main, CI-green** (D39): pure
        `resolveLayout` cascade + `useResolvedLayout` hook + `(theme,system,view)→
        layout` localStorage override store; no consumer, no visual change. **L3b
        (per-system layout wired into game-browse) ✅ MERGED, operator
        playtested (NES→list, slider hides on list)** (D40):
        coexist model — `layout` made optional, `useDeclaredLayout` keyed on
        `selectedSystemId()`, `LibraryView` renders the declared primitive
        (grid/list; carousel/wheel/custom→grid fallback) else the global viewMode;
        Retroverse demo NES→list. **L4a (render carousel in game-browse) ✅ shipped
        on `feat/theming-arc2-l4-wheelnav`, pending visual playtest** (D41):
        `LibraryView` renders a per-system `carousel` via `CarouselNav` (CoverFlow's
        pattern); Retroverse demo SNES→carousel; wheel/custom still grid-fallback.
        typecheck/lint/vitest(145)/build green. **Next: L4b** (fresh session) — the
        radial WheelNav primitive + render `wheel`. Then L5 override UI → L6 → P.
        **L1 (D33 consumption opt-in) ✅ MERGED to main, operator playtested**
        (DECISIONS **D36**): per-theme `per_system_ui?:
        {tiles?,sfx?}` manifest opt-in, App-bridged into the `LibraryTile`/grid/SFX
        gate (`userMaster AND themeOptsIn`); Retroverse opts in, CoverFlow/bare go
        uniform; user master toggle kept as global off-switch. typecheck/lint/
        vitest(114)/build green, frontend-only. Acceptance gate: Retroverse
        per-system as before, CoverFlow/bare uniform, master-off forces uniform.
        **Next after playtest+merge: L2** (view/layout manifest contract +
        systemUIConfigs experiential→Retroverse split, D34).
        With ARC 1 complete (Phase 6 gate passed), planned ARC 2 from the fixed
        inputs **D32** (per-system layout becomes a substrate contract +
        persisted end-user override) + **D33** (per-system UI is a platform
        capability themes opt INTO). Two new decisions: **D34** (factual data +
        machinery = platform; experiential design + content = theme — the
        shipped global `systemUIConfigs.ts` experiential config migrates to
        Retroverse) + **D35** (arc split/renumber: **ARC 2 = layout**, **ARC 3 =
        cinematic/scripting** [old ARC-2 Rhai+WGSL+motion], **ARC 4 = Theme
        Studio**; the `.oatheme` loader absorbed as ARC 2's tail). Slice order
        (contracts-first, D33 fix pulled forward): **L1** D33 consumption opt-in
        · **L2** view/layout manifest contract + systemUIConfigs split · **L3**
        resolver + persisted `(theme,system,view)→layout` user override · **L4**
        WheelNav body · **L5** end-user override UI (engine Per-System Hub) ·
        **L6** re-home Per-System UI Stage 2/3 as Retroverse content · **P**
        `.oatheme` runtime loader. Plan:
        [PLANS/theming-arc-2-per-system-layout.md](PLANS/theming-arc-2-per-system-layout.md).
        Cinematic/scripting axis = ARC 3 (own plan when scheduled). **L1 queued
        in [NEXT.md](NEXT.md) HIGH band.** Supersedes per-system-ui Stage 2/3.
  - ESLint boundary rule defers to Phase 4 alongside Tauri-bridge
    work. Operator decisions locked 2026-06-06: one unified
    premium frontend (no LaunchBox/BigBox split); manifest = TOML;
    theme swap = restart (ARC 1); build-time bundling only
    (ARC 1); Kiosk plan's 4-layer substrate absorbed. Sequencing:
    Phases 1-2 parallel with VL Phase A; pause at end of Phase 2
    for VL Phase E + C; resume Phases 3-6 after both VL phases
    ship. Retroverse becomes the first theme on the substrate
    (Phase 6 dogfood test).

- **Virtual library + preservation architecture + launcher-agnostic frontend**
  — major multi-month arc planned 2026-06-03. 8 phases (A → E → B → C
  → D → F → G; Phase H deferred). Plan at
  [PLANS/virtual-library-and-launcher-arc.md](PLANS/virtual-library-and-launcher-arc.md).
  **Phase C COMPLETE ✅ — C3 merged + playtest passed 2026-06-08**
  (merge `28875d5`; C1 + C2 + C3 all on main). Operator picked C
  over B 2026-06-07 to unblock the theming arc's Phase-2 pause;
  theming plan §7 wanted VL E + C before theming Phase 3 resumes —
  **both are now merged, so Theming Phase 3 is unblocked.** Next VL
  slice = **Phase B** (two-mode UX + Collection Health). Sub-phase
  plan +
  operator-locked decisions:
  [PLANS/launcher-abstraction.md](PLANS/launcher-abstraction.md).
  Key decisions: Launcher = lifecycle trait ABOVE the untouched
  `oa_core::Core`; pilot = **Dolphin standalone against the
  existing `gamecube` system** (no new systems in C — wiiu/ps3/3ds
  ride Phase D with the installer); minimize-OA-while-running;
  C1 (trait + LibretroLauncher refactor, invisible) → C2 (profile
  registry + ExternalProcessLauncher + first Dolphin launch) →
  C3 (capability gating + session polish, merged + playtest
  passed).
  - **Phase 0 ✅ shipped 2026-06-03** (merged from
    `feat/virtual-library-arc-foundation`, merge `dd430e4`-ish — Phase 0
    + the subsequent docs cleanup merged to main together). DECISIONS
    reversal of the 2026-05-16 libretro-only stance + partial un-park
    of the 2026-06-02 plugin-API entry + CLAUDE.md softening + plan
    committed at the path above.
  - **Phase A — identification depth (~3–4 weeks, in flight via A1):** disc-track SHA-1
    (A1) + filename tag decode (A2, hacks/translations/bad dumps) +
    Tier 5 deep-dive (A3) + MAME parent/clone bridge (A4).
    - **A1 Sub-phase 1 ✅ shipped 2026-06-03** (merge `1c319f8`).
      Schema v18→v19 (rom_hashes_tracks + game_disc_tracks +
      disc_sets + games.disc_set_id/disc_number), parser per-track
      + multi-disc-parent emission, sync flow dispatch, full
      disc-shape helper surface in library_db. 665 oa-shell tests
      pass (660 baseline + 5 new); frontend typecheck silent.
      Cart-shape `rom_hashes` path untouched. Plan + design
      decisions in [PLANS/disc-track-sha1-matching.md](PLANS/disc-track-sha1-matching.md).
    - **A1 Sub-phase 2 ✅ shipped 2026-06-03** (merge `dc2a257`).
      Per-track hashing engine in `apps/oa-shell/src/disc_track_hash.rs`:
      `.iso` / `.cue` (split-bin + merged-bin via INDEX 01) / `.gdi` /
      `.chd` (CHT2 parse + 4-frame padding + subchannel strip).
      Streaming SHA-1 with 1 MiB cancel-check cadence. `evaluate_match`
      across Strict / Threshold / Lenient. `cd_id::cue::parse` extended
      to capture INDEX 01 positions for merged-bin slicing. 687 tests
      pass (665 baseline + 22 new); frontend typecheck silent.
    - **A1 Sub-phase 3 ✅ shipped 2026-06-03** (merge `d2bf7db`).
      Backend identify flow + game_disc_tracks cache + mtime/size
      invalidation. `JobKind::DiscTrackHash`, `DiscTrackStrictness`,
      per-track try block in `resolve_rom_hashes_for_system`.
    - **A1 Pivot ✅ shipped 2026-06-03** (merge `c4aec19`).
      Per-track moved behind `LibraryPrefs.disc_track_experimental_enabled`
      (default OFF) after operator playtest measured 0% match rate
      on real library (Dreamcast CHD: chdman extract is 225 sectors
      short of redump's DiscImageCreator source dump; archived PSX
      ZIP: per-track skipped per Sub-phase 3 deferral). New primary
      identification: filename-fuzzy match against canonical disc
      titles in `rom_hashes_tracks` — cheap, works on any container
      shape. 697 tests pass.
    - **A1 follow-up — frontend experimental checkbox ✅ shipped.**
      `frontend/src/components/SettingsSections.tsx:322` renders the
      "Per-track SHA-1 disc identification" toggle in Settings →
      Display → Experimental, wired to
      `LibraryPrefs.discTrackExperimentalEnabled` via
      `set_library_prefs`. Bullet retained for the historical pointer.
    - **A1 hit-rate measurement ✅ good enough — closed 2026-06-08.**
      Operator ran Identify ROMs on a disc system and judged the
      filename-fuzzy primary (matched against canonical disc titles
      in `rom_hashes_tracks`, misses falling to `peek_disc_id`)
      sufficient for now. Sub-phase 4 (multi-disc grouping) already
      shipped on top of the fuzzy primary (2026-06-04). No further
      identification architecture work needed at this time; revisit
      only if a real library surfaces a systematic miss pattern.
    - **A1 Sub-phase 4 ✅ shipped 2026-06-04** (backend `b6b4ae6` +
      frontend `f42c567`). Multi-disc disc-set wiring on top of the
      fuzzy primary. Backend: `maybe_stamp_disc_set_membership` at
      identify time stamps `games.disc_set_id` + `games.disc_number`
      when a canonical title's `(Disc N)` suffix matches a disc_sets
      row. Frontend: `collapseDiscSets` representative-per-set tile
      collapse + DiscPickerDialog overlay (lists members via
      `list_disc_set_members`, click → launch that disc). Works for
      multi-disc games on PSX / Saturn / Sega CD / PCE-CD where
      redump catalogs the parent group. Single-disc games and cart
      libraries unchanged.
    - **A2 — filename tag decode ✅ shipped + merged 2026-06-06**
      (branch `feat/virtual-library-phase-a2`, merge `91e8e04`).
      7 new fields on `ParsedTitle` (and mirrored
      onto `GameVariant` + frontend `VariantInfo`): `dump_status`
      enum (Verified / BadDump / OverDump / Fixed / Unknown),
      `is_hack`, `is_translation`, `is_pirate`, `is_bios`,
      `is_homebrew`, `translation_languages` Vec. Decoder covers
      GoodTools brackets (`[!]`, `[b]`/`[b1]`/`[b2]`, `[o]`/`[o1]`,
      `[f]`/`[f1]`, `[h]`/`[h1]`/`[hI]`/`[hIR]`, `[p]`/`[p1]`,
      `[T+Eng]`/`[T-Eng]`/`[T+Eng,Fra]`/`[T+Eng1.0_Aeon]`) +
      No-Intro/TOSEC paren forms (`(Hack)`, `(Pirate)`, `(Cracked)`,
      `(BIOS)`, `(Homebrew)`, `(Aftermarket)`). `Unl` /
      `Unlicensed` intentionally NOT folded into `is_homebrew`
      (unlicensed commercial vs amateur homebrew is a real
      preservation distinction). `library_groups.rs` ships
      `VariantFilters` + `variant_passes_filters` pure-function
      predicate, plus `casual_view_defaults()` (excludes bad dumps /
      over-dumps / BIOS / prerelease; keeps hacks / translations /
      pirate / homebrew opt-in) and `preservation_view_defaults()`
      (everything passes). Foundation for VL Phase F's Preservation
      Vault filter ribbon. 790 oa-shell tests pass (744 baseline + 46
      new); frontend typecheck silent.
    - **A3 — Tier-5 deep-dive — DEFERRED 2026-06-07.** Structural
      matching (internal header title extraction, archive intro-
      spection, trimmed-CRC32 fallback) was the natural ~1-week
      next slice after A2; operator chose to park it. Reason for
      deferral: the current 4-tier chain (Hash / Header / Extension
      / Hint) plus A1's fuzzy-filename fallback plus A2's typed
      tags already covers the bulk of real-world library hit-rate;
      Tier-5 yield isn't quantified yet, so it'd be ~1 week of
      build for unclear payoff. Reconsider when: (a) operator's
      hit-rate measurement on a real library surfaces a meaningful
      un-matched chunk that structural matching could close, or
      (b) a specific system class (e.g. archived MAME variants,
      trimmed GBA dumps) starts hitting the unidentified queue
      visibly. Plan source: [PLANS/virtual-library-and-launcher-arc.md](PLANS/virtual-library-and-launcher-arc.md)
      §6 Phase A (A3 sub-bullet).
  - **Phase E — schema promotion (~3–4 weeks):** new
    `game_identities` SQLite table; per-group MediaDb keys; per-group
    metadata + play_time + favorites. Sub-phase plan + design
    decisions: [PLANS/game-identities-schema.md](PLANS/game-identities-schema.md).
    - **E Sub-phase 1 ✅ shipped + merged 2026-06-07** (merge
      `e13b7e7`; operator playtest passed — live library migrated
      to v23 cleanly). Schema v22→v23: `game_identities` table
      (deterministic `idn-<sha1[..16]>` ids over
      `(system_id, lowercased parsed base title)`) +
      `games.identity_id` FK + population at migration + existing
      `game_group_defaults` pins copied into
      `default_variant_id`. One `rebuild_identities_for_system`
      path serves migration, `add_games`, `delete_game`, seed
      drops, and the end of Identify ROMs (regroups canonical
      retitles, sweeps orphans; operator metadata survives
      rebuilds via upsert-preserving SQL). Pin set/clear
      dual-writes both stores until Sub-phase 2 swaps the read
      path. Backend-only — zero behavior change visible in the
      UI. 796 oa-shell tests pass (790 baseline + 6 new).
    - **E Sub-phase 2 ✅ shipped + merged 2026-06-07** (merge
      `8d59b7e`; operator playtest passed). Read-path swap:
      `build_groups` is
      identity-backed (group key = `games.identity_id`; identity
      supplies canonical title / metadata / cover fallback / pin;
      ranking preserved verbatim; unstamped rows fall back to
      parse grouping + lazy heal in `list_game_groups`).
      `GameGroup` gains identity_id + canonical metadata +
      per-identity stats (play_time sum, any-favorite,
      any-completed, max last_played) aggregated across discs;
      `GameVariant` gains per-file stats for Phase B's Variants
      tab. Pins write ONLY `game_identities.default_variant_id`
      now (dangling-pin sweep replaces the legacy FK cascade);
      `game_group_defaults` is dead — dropped in a later cleanup
      migration. Canonical-title search moved to Sub-phase 3.
      801 oa-shell tests pass (796 baseline + 5 net new).
    - **E Sub-phase 3 ✅ shipped + merged 2026-06-07** (merge
      `2bcdcf5`; operator playtest passed — canonical tile titles
      + metadata backfill + canonical search all confirmed).
      **PHASE E COMPLETE** (identity editor UI rides Phase B).
      Identity media keyspace = same media.json map
      keyed by `idn-…` ids (existing MediaDb machinery free; cover
      resolution tries identity key → per-file; no writer yet —
      canonical-art UI lands Phase B/F). Enrichment pass merges
      member variants' MediaDb metadata onto identity rows
      (fill-NULL-only; startup backfill + post-metadata-sync
      hook). Frontend: tiles show canonical titles (singleton
      groups now in `groupsByVariantId` — every tile
      identity-backed), sort runs post-collapse, search matches
      canonical titles, GameDetailPanel header prefers identity
      metadata. 803 oa-shell tests pass; typecheck silent.
      **Merging this completes Phase E** (identity editor UI
      rides Phase B).
  - **Phase B — two-mode UX + Collection Health (~2 weeks):** global
    Casual / Preservation toggle; Variants tab on GameDetailPanel;
    System Health Overview gains % verified / % covers / % metadata
    rollups.
    - ⚠️ **RE-DERIVE, don't merge (reassessed 2026-06-15):** Phase B is
      **code-complete (Slices 1–4)** on branch `feat/virtual-library-phase-b`
      (tip `a475b29`, also on `origin`) — ~647 lines / 14 files, forked
      2026-06-09 and now **171 commits behind main**. The files it needs have
      all since **moved or been rewritten**, so a merge would be a conflict
      slog. The clean path is to **re-implement on current main, using the
      branch as a design+code reference** (the branch does not rot as a
      reference, so there is **no time pressure** — slot it after Theming
      ARC 2 L1). What it adds + the re-home work:
      - **Slice 1 — Casual ⇄ Preservation mode + Settings toggle.** Built
        against the OLD `SettingsSections` + settings store; **must re-home
        into the new Settings IA "Library" group** (`engine/SettingsSections.tsx`
        + the rewritten `platform/settings/store.ts`).
      - **Slice 2 — preservation variant ribbon** on tiles/list. Paints chrome
        on the *shared* `LibraryTile`/grid unconditionally — **reconcile with
        D33** (make it theme-opt-in, not forced cross-theme).
      - **Slice 3 — Variants tab** — re-apply onto the relocated+rewritten
        `platform/components/GameInfoModal.tsx`.
      - **Slice 4 — Collection Health rollups** — re-home onto
        `engine/SystemHealthPage.tsx` (moved from `routes/retroverse/`).
      No *conceptual* conflict with locked decisions (two-mode UX is core to
      this arc; orthogonal to theming). Parked intentionally; do not delete the
      branch (it's the reference). The branch's `a475b29` docs commit
      (re-categorize Preservation Vault / Phase F as theme territory) predates
      D31–D34 — re-evaluate against current theme/engine territory when Phase F
      is planned.
  - **Phase C — launcher abstraction (~2–3 weeks):** `Launcher`
    lifecycle trait ABOVE the untouched `oa_core::Core`;
    `LibretroLauncher` + `ExternalProcessLauncher` impls;
    `config/emulators/<id>.yaml` profile registry. Sub-phase plan:
    [PLANS/launcher-abstraction.md](PLANS/launcher-abstraction.md).
    - **C1 — Launcher trait + LibretroLauncher refactor ✅ shipped +
      merged 2026-06-07** (operator playtest passed — launches
      identical to pre-C1). `oa-core` gains `Launcher`
      (`prepare → launch → is_alive → terminate` + capabilities) +
      `LaunchRequest` / `LaunchPrepared` / `LaunchedSession` /
      `LauncherCapabilities` / `LaunchError` (transparent Display so
      pre-C1 error strings survive byte-identically). New
      `apps/oa-shell/src/launcher.rs`: `LibretroLauncher` maps
      launch → `EmuCommand::LoadRom`, terminate →
      `EmuCommand::UnloadRom`, same fields + error strings.
      `launch_rom` / `unload_rom` route through
      `AppState.launcher: Arc<dyn Launcher>` + new `active_launch`
      session slot; content resolution / session bookkeeping /
      focus_game / archive cleanup stay in the commands unchanged.
      No external launching yet. 807 oa-shell tests pass (803
      baseline + 4 new); typecheck silent.
    - **C2 — profile registry + `ExternalProcessLauncher` + first
      Dolphin launch ✅ shipped + merged 2026-06-08** (branch
      `feat/virtual-library-phase-c2`; operator playtest passed —
      external Dolphin launches + exits cleanly back to the library,
      launcher picker hints correct). New
      `apps/oa-shell/src/emulator_profiles.rs`:
      `config/emulators/<id>.yaml` registry (D4 fields; shipped
      pilot `dolphin.yaml`, supported_systems `[gamecube]`,
      `--batch --exec={content}`) + two appData pref files
      mirroring cores.json — `emulators.json` (per-profile binary
      path, operator-set; wins over the profile's optional
      `binary_path`) and `launchers.json` (per-system default
      launcher; no entry = libretro, byte-for-byte today's path).
      `ExternalProcessLauncher` in launcher.rs: spawn via expanded
      template, child stdout/stderr pumped into the OA debug log
      (`oa_shell::external` target), `is_alive` = try_wait,
      terminate = graceful close (Windows taskkill WM_CLOSE) → 5s
      grace → kill fallback. `launch_rom` branches per-system
      BEFORE the bytes/patch machinery (externals never read a
      1.4 GB image into RAM; archived entries get a clear "extract
      or clear the pref" error — C3+ territory); `active_launch`
      slot now pairs session + owning launcher; supersede rules:
      external→anything and in-process→external terminate through
      the owning launcher first (libretro→libretro keeps
      LoadRom-replace). D3: OA minimizes all windows on spawn; an
      exit-watcher thread (1 Hz `is_alive` poll, slot-ownership
      guard) persists play time via `close_active_session`,
      restores + focuses the shell, toasts, and emits
      `oa://external-session-ended`. Settings surface: CoresPage
      "External emulators" section — per-profile binary-path
      picker (validated file-exists; name-mismatch accepted with
      WARN) + per-system "Default launcher" select + download
      link. New commands: `list_emulator_profiles` /
      `set_emulator_binary_path` / `get_launcher_pref` /
      `set_launcher_pref`. 818 oa-shell tests pass (803 baseline +
      15 new); typecheck silent. Playtest gate: GameCube game
      launches through the operator's Dolphin install from the
      same tile that launched it via libretro yesterday; pref
      unset = identical to yesterday.
      - **C2 playtest fixes 2026-06-07** (commit `ba80d8c`, on the
        same branch). Two bugs surfaced in the first Dolphin
        playtest: (1) **black inescapable screen after the external
        emulator exits** — the Rust exit-watcher's
        `oa://external-session-ended` event had no frontend
        listener, so the shell un-minimized still in its in-game
        view (`gameRunning=true`, library hidden) onto a wgpu
        surface with no core rendering. Added the listener in
        `App.tsx`; mirrors `handleUnload`'s UI reset (leave in-game
        view, reveal library, revert renderer overrides) but with NO
        `unload_rom` call since the session is already torn down;
        direct-launch mode quits like `rom-unloaded`. Refactored the
        revert block into shared `revertRendererToDefaults()`. (2)
        **no discoverable core-install path from the launcher
        picker** — added a soft amber "no core installed — see
        Browse cores below" hint beside the per-system Default
        launcher dropdown when libretro is selected but no installed
        catalog core claims the system (mirrors the existing "set
        the binary path first" hint). Frontend typecheck green; no
        Rust changes. Also rode along: z-index fix lifting platform
        modals to `z-[70]` above the engine takeover (commit
        `5d0ac97`).
    - **C3 — capability gating + session polish ✅ shipped + merged
      2026-06-08** (merge `28875d5`; operator playtest passed —
      **Phase C complete**). Three slices. **(1) Capability gating**
      (commit `5fd92ab`): new `get_active_launcher_capabilities`
      command + `ActiveLauncherInfo` (launcher id / display name /
      isExternal / `LauncherCapabilities`) reads the `active_launch`
      slot — full libretro set when in-process/no session, the
      external launcher's caps (v1 all-false) + profile display name
      otherwise. QuickSettings fetches it on open (fail-open to the
      full set) and grays the governed action rows (saves / shaders /
      core options / input / bindings / screenshots / rewind / tas /
      video / memory / disc) with a "Managed by <name>" hint; Resume /
      Game info / toggles / Exit always available. Relevant in the
      OA-restored-during-external-session state (D3 minimizes OA in
      normal play). **(2) Per-system drill-in launcher pref** +
      **(3) force-quit affordance** (commit `ff9e86b`): Settings →
      Per-system gains a "Launcher" card (renders only when an
      external profile covers the system) mirroring the CoresPage
      dropdown, with a binary-not-set warning + a note that external
      launchers bypass the libretro-only cards; the QuickSettings Exit
      row names the emulator ("Close Dolphin") for external sessions
      (the unload_rom path already does graceful→5s→kill from C2).
      818 oa-shell tests pass (unchanged — additive command);
      frontend typecheck silent. Play-time tracking + graceful
      terminate/hang fallback were already delivered in C2.
  - **Phase D — external emulator install pipeline (~2–3 weeks):**
    download + setup for v1 pilot trio (Cemu / RPCS3 / Lime3DS) from
    official release endpoints; plugin-style updater; legal posture
    locked (zero ROMs / zero BIOS, ever).
  - **Phase F — Preservation Vault polish (~1–2 weeks):** dedicated
    surface with deep variant tree filter ribbon.
  - **Phase G — `crates/oa-preserve` workspace split (~1–2 weeks):**
    refactor identification + grouping + DAT parsing into a
    standalone crate.
  - **Phase H — `oa-preserve-cli`:** deferred — back burner.
  - Two strategic shifts: (1) virtual library moves from runtime
    grouping to SQLite schema; (2) external standalone emulators
    (Cemu / RPCS3 / Lime3DS / Ryujinx / Suyu / Dolphin standalone)
    join libretro cores via the `Launcher` trait. Reverses the
    2026-05-16 libretro-only DECISIONS entry; partially un-parks the
    2026-06-02 plugin-API PARKING_LOT entry. Driven by 2026-06-03
    advisor proposal (ChatGPT + Gemini) + three rounds of operator
    Q&A. Total estimate ~14–22 weeks.

- **HW-Render Pipeline (GPU-rendered libretro cores)** — engine arc
  planned 2026-06-07 after the first internal Dolphin playtest
  crashed. Implements the libretro HW render interface (`SET_HW_RENDER`
  + Vulkan HW negotiation) so GPU-emulator cores — Dolphin
  (GameCube/Wii), paraLLEl-N64, Beetle PSX HW, Flycast (Dreamcast),
  PPSSPP, Beetle Saturn HW — run in-process instead of falling to a
  Null video backend and crashing. Plan at
  [PLANS/hw-render-pipeline.md](PLANS/hw-render-pipeline.md); feature
  folder [features/hw-render/](features/hw-render/). Operator decisions
  (2026-06-07): Vulkan-first multi-backend abstraction (RetroArch
  video-driver model — one backend active at a time, matched to the
  core — built on wgpu's existing backends + `texture_from_raw`, NOT
  four hand-written renderers); shared-device zero-copy as the end
  state (the "OA runs better itself" win); commit wgpu to its Vulkan
  backend for the HW path; capability tiering + software-peer fallback;
  **no HW-render guard** — make the cores work. DX12/GL contexts added
  later only if a core runs better on them or operators hit Vulkan
  issues. Four milestones: M1 (handshake + core on screen) → M2
  (zero-copy import) → M3 (full HW lineup + reinit-on-core-switch +
  tiering) → M4 (more backends if needed + MoltenVK/macOS).
  **Touches the Rust engine layer only** (`oa-render` +
  `oa-libretro/{state,ffi}.rs` + `main.rs` run loop) — near-zero
  overlap with the frontend/schema/launcher-lifecycle work in the
  Theming + Virtual Library arcs. **Sequencing:** slot **after VL
  Phase C3** (both edit `main.rs`'s `LoadRom` handler — different
  regions; let C3 land first to avoid a double-edit, and C3 + HW-render
  together finish the GameCube launch story) and **before Theming ARC 2
  (WGSL shaders)** (so shader hooks build on the GPU-resident-texture
  renderer instead of being rewritten). Theming ARC 1's frontend resume
  can interleave safely (different layer).
  - **M1 ✅ PROVEN 2026-06-08** (branch `feat/hw-render-m1`, NOT merged —
    8 commits, pushed). A GPU-rendering libretro core (**paraLLEl-N64 /
    paraLLEl-RDP Vulkan**) runs in-process via a standalone `ash`
    VkDevice + the core's `create_device` + the 8 Vulkan interface
    callbacks + CPU readback into `fb_rgba` — wgpu untouched, 46 software
    cores unaffected. Full chain confirmed in the log
    (`first readback OK — 640x240 … frame on screen`). Along the way also
    fixed a real OA bug: per-system **core options were applied AFTER
    `retro_load_game`** so cores couldn't pick their Vulkan renderer —
    now pre-applied before load (`main.rs`). Protocol learnings recorded
    in feature DECISIONS **D7**. Dolphin/GameCube (the plan's literal M1
    gate) is **parked** — it builds its own windowless Vulkan context and
    silent-crashes; needs ground-truth Vulkan validation, separate harder
    problem. **Known M1 limitation:** ~half speed (synchronous readback
    full-drains the GPU each frame; audio sounds off purely because emu
    runs at ~half rate) — speed is M2's mandate by design.
  - **M2 (zero-copy) — IN PROGRESS** (branch `feat/hw-render-m2`, off M1;
    M1 stays bankable at tag `hw-render-m1-proven`). Architecture confirmed
    (DECISIONS **D9** reinit-per-core, **D10** HwContext-trait-for-later).
    **Foundation DONE + working on hardware:** wgpu adopts the core's
    Vulkan device (`from_raw`/`device_from_raw`, verified runtime), the
    renderer rebuild-on-core-switch lifecycle is crash-free (swap/restore/
    re-adopt; R1 holds), and display settings (shader/scaling/etc.) are
    preserved across the rebuild. **Measured (cont. 14):** the M1 readback
    is ~31ms/frame of pure serialization (NOT upscale — confirmed 1×), so
    zero-copy is justified. **Zero-copy import SHIPPED (cont. 15, task 1 —
    compiles clean + 49 tests pass, awaiting playtest):** `present_hw_image`
    GPU-blits the core's `set_image` `VkImage` into a wgpu-native fb_texture
    (raw `vkCmdBlitImage` via `as_hal_mut`, hand-managed barriers leaving it
    in `SHADER_READ_ONLY` where wgpu's tracker expects it — sidesteps the
    confirmed `texture_from_raw` UNDEFINED-discard wall) and runs the existing
    shader/scale/bezel chain on it. Queue sync via `hw_queue_lock/unlock` (the
    core's `lock_queue`). oa-shell sets import mode on adopt + routes both
    present sites through `present_current` (readback stays the fallback).
    **PROVEN at 60 fps (cont. 15b):** playtest log shows readback GONE, steady
    60.0 fps (the on-screen ~55 was a cumulative-since-launch average), audio
    0 dropped, CrtLite intact. Tasks 2 & 3 (multi-buffer `get_sync_index` /
    lock-narrowing) are NOT needed for paraLLEl-N64 — deferred indefinitely.
    The playtest's "small centered image" was the per-game `scaling: original`
    override (native 640×240 centered), not a render bug; one follow-up fix
    refreshes fb dims in import mode (was a stale stat). Also fixed en route: a
    general libretro audio gap — `SET_SYSTEM_AV_INFO` (env 32) timing revisions
    were dropped, so cores that revise their rate post-load (paraLLEl-N64)
    underfed the sink → buzz; now honored (sink rebuild + limiter retime).
  - **M2 ✅ SHIPPED + MERGED to main 2026-06-08** (tag `hw-render-m2-proven`).
    Operator validated on hardware: paraLLEl-N64 zero-copy at steady 60 fps,
    aspect/flip correct, CrtLite intact, audio good, AND software-core render +
    HW⇄software swap-back confirmed. Tasks 2 & 3 (multi-buffer `get_sync_index`
    / lock-narrowing) deferred indefinitely — not needed for paraLLEl-N64.
    **HW-render arc now at a checkpoint** (M1 + M2 + M3 Half 2 done — see the
    M3 bullet below). Remaining: M3 **Half 1** (operator-driven per-core HW
    validation — Beetle PSX HW, Flycast, PPSSPP, Saturn HW) + M4 (DX12/GL
    backends, cross-platform), **future stretch**. Known
    separate to-do: NES (+ maybe others) audio clipping/clicking — filed in
    `NEXT.md` MEDIUM, independent of the HW path. See the hw-render
    SESSION_LOG cont.15/15b/15c entries.
    - **M3 "Half 2" ✅ MERGED to main 2026-06-15** (cherry-picked clean off the
      stale `feat/hw-render-m3` onto current main; 846 oa-shell tests green).
      Ships the D4 observability + fallback signal: `hw_render.rs` (HW-render
      status log line every launch + `software_peer_core()` table +
      decline-toast) + `oa-libretro` `HwRenderStatus` enum. **Not** an
      auto-core-swap. **M3 "Half 1" (the actual per-core HW validation —
      Beetle PSX HW / Flycast / PPSSPP / Saturn HW) is operator-driven
      playtesting, still open.** M4 (DX12/GL backends) future stretch.

- **Retroverse UI rollout** — all six top-toolbar tabs operator-
  facing with real bodies. 2026-05-28 shipped Phases A-C4 + HOME v2
  + SETTINGS expansion; 2026-05-29 closed the unified controller
  pipeline + menu/dialog polish + Slice 12 custom collections +
  Per-system SETTINGS drill-in + Now-playing chip + DISCOVER body.
  (The Retroverse experimental flag + legacy Shell were both removed
  2026-05-31; Retroverse is now the default shell, rebuilt as a real
  theme under `themes/retroverse/` — see theming Phase 6.)

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
  - 660+ oa-shell tests green; frontend `npm run typecheck` silent.

  **Genuinely open work** (full §10 list in
  [PLANS/retroverse-ui-rollout.md](PLANS/retroverse-ui-rollout.md)
  §10, audited 2026-05-29):
  - Phase C6 — content-packs infrastructure (substantial; unlocks
    DISCOVER's 5 stub axes + curated COLLECTIONS + theme packs).
  - RetroAchievements integration OR local milestone tracking
    (HOME ACHIEVEMENTS card + GameDetailPanel / SystemInfoPanel
    sections are placeholders).
  - Per-System UI Stage 2 + Stage 3 — now Theming ARC 2 (slice L6).
  - Flag deprecation — done (legacy Shell removed 2026-05-31).

  **Content workstream (operator-side):** per-system hero art (drop
  console + fanart into existing PlatformMedia slots),
  `systemMetadataStubs.ts` refinement for ~38 systems beyond the 7
  priority stubs, per-system blurbs.

- **Per-System Custom UI — ⇒ merged into Theming ARC 2 (2026-06-15).**
  Stage 1 machinery (Slices 1-5: SFX wiring, background renderer, boot
  animation framework, tile flourish) shipped + now lives in `platform/`.
  The architecture is **superseded by D32/D33/D34** — per-system UI is a
  platform capability themes opt into; pilots (GB/NES/Vectrex) + Stage 2/3
  re-home as Retroverse content under
  [PLANS/theming-arc-2-per-system-layout.md](PLANS/theming-arc-2-per-system-layout.md)
  (slice L6). Folder banner-reconciled:
  [features/per-system-ui/](features/per-system-ui/).

---

## Recently completed

Compressed log. Full per-arc detail lives in `docs/_archive/` — see
[_archive/INDEX.md](_archive/INDEX.md) for the searchable manifest.

**2026-06-13 → 06-15** (moved out of In-flight by the 2026-06-15 doc audit)
- **Settings IA Redesign** — Slices 1–4 merged (re-cut Settings around intent:
  Import & Setup · Library · Organize · Systems · External Emulators ·
  Themes/Appearance; declarative per-theme Appearance schema; Library re-point) +
  external-emulator batch-1 profiles (9 `config/emulators/*.yaml`). Slice 5
  deferred into guided-setup Phase 2; external-emulator depth parked for a
  planning arc. Plan archived (`_archive/PLANS/settings-ia-redesign.md`).
- **Per-System Settings Hub** — ✅ COMPLETE + merged 2026-06-14. Card-based
  **Systems** hub (grid → per-system domain cards → editor) consolidating all
  per-system settings, **including the Game/Platform Metadata editors** (absorbed
  the standalone Metadata Curation arc). `HubCard`/`HubGrid`/`PanelScaffold` ARE
  the unified-nav Pillar-B deliverable; shipped a dev-only DevTools panel + HintBar
  fix alongside. Parked: terminology audit, re-gate DevTools before public release.
- **Controller Identity & Auto-Config** — ✅ full arc shipped + merged 2026-06-13
  (`808fc0b`): VID/PID identity, replug-stable ports, non-standard-pad
  normalization + SDL `gamecontrollerdb` import (813 pads), label families, live
  test window. Fixed the operator's wired Faceoff Switch pad end-to-end. Parked:
  Phase-3 wizard, glyph icons, data-file update mechanism. Plan archived.
- **Metadata Curation** — ✅ closed 2026-06-15. Override backend shipped
  (`game_metadata_overrides`, schema v24); editor UI **absorbed into the
  Per-System Hub** (Game/Platform Metadata domain cards) — standalone `metadata`
  Settings category removed. Plan + feature folder archived.

**2026-06-08**
- Libretro plumbing audit (`docs/cores/AUDIT_2026-06-08.md`) + fixes +
  Game-focus keyboard switch — branch `feat/libretro-plumbing-fixes`
  (commits `1ed8efb` audit, `d98417b` fixes, `ce2034d` game-focus; merged to
  main 2026-06-08). Polled keyboard/mouse state (H1/H2) unblocks the computer-
  core tier (MSX/DOS/5200/O2) + arcade trackball/spinner games; eight more
  env/parser/lifecycle fixes (M1-M7, L1/L2). Game focus is now a clean
  RetroArch-style keyboard switch (OFF=shell, ON=machine) with a configurable
  toggle chord (Settings › Controls). oa-shell 821 tests green; frontend
  typecheck clean. Not yet operator-playtested. Follow-ups: bootless `load_no_rom`
  launch; audit M6 (subsystems) + L3 (microphone).

**2026-06-04**
- Audit-derived structural sweep — three tiers landed in sequence:
  silent-bug (frontend `.catch(() => {})` swallows replaced with
  toast/console surfacing; merge `78cbd13`), fragile (JobScope
  adoption extended via new `cancel()` + `resume()` constructors,
  covering `core_installer`, `start_background_scan`, `spawn_test_job`,
  `CoreDownloadResumer`; merge `ede2473`), boilerplate (frontend
  `listenScoped` helper retiring 8 manual `listen`/`onCleanup`
  ceremonies; merge `4091804`). Backend `JobScope::tick_and_emit`
  intentionally NOT shipped — emit and tick fire at different cadences
  by design. Dual-channel retirement parked. See
  [DECISIONS.md](DECISIONS.md) 2026-06-04 "Audit-derived sweep" entry.
- Unidentified-games audit surface + tiered disc-filename matcher
  (v1→v2). Bundle merge `902ecf2`. Three deliverables: (a) per-system
  "View N unidentified ▸" dialog from LibraryManagerPage's Manage
  panel with reveal-in-folder per row + Re-run Identify ROMs footer;
  (b) v1 tiered matcher (strict + relaxed-fallback) bridging
  No-Intro `(v1.1)`/Redump `(Rev 1)` + single-region `(USA)`/
  multi-country `(USA, Canada)`; (c) v2 TOSEC-vs-Redump bridge adding
  language-code paren strip + `(Disc N of M)`→`(Disc N)` + `)( `→`) (`
  spacing + `(Unl)` strip. Operator-library outcome: PSX 98→13 (87%),
  Dreamcast 105→27 (74%), PCE-CD 59→24 with Identify re-run. 33
  rom_hashes tests cover both tiers + regression-guard preserved
  distinctions (Beta/Proto/Demo, Disc 1≠Disc 2, USA≠Japan≠Europe).
  See [DECISIONS.md](DECISIONS.md) 2026-06-04 "Unidentified-games
  audit surface" + "Tiered disc-filename fuzzy matcher" entries.
- Spot-audit derived fast wins. Merge `1bb9489`: `tg16` + `pce-cd`
  default_core wiring (BROKEN — both YAMLs lacked the field) + N+1
  batched cart lookup via new `library_db::lookup_rom_hashes_batch`
  (~1500 → ~500 SELECTs per Identify ROMs pass on a 500-game cart
  library). Plus `perf/async-fs-and-render-measurement` (merge
  `23b4f50`): tokio::fs cutover for `metadata.rs` + `rom_hashes.rs`
  cache I/O (4 sync-fs-in-async sites), and measure-before-build
  instrumentation for the renderer bind-group pool which the data
  rejected at 0.10% of frame budget (DECISIONS.md 2026-06-04
  "Bind-group pool" entry). Parked: async-fs Phase 2 for
  `core_installer.rs` (PARKING_LOT 2026-06-04 with trigger criteria).
- System info content arc — Wave 1 + Pass A + Pass B shipped. Bundle
  merges `4d2c41d` (Wave 1: 12 overlooked-focus systems written from
  training knowledge), `94845e4` (Pass A: 5 pre-Wave-1 entries
  re-verified against Wikipedia — caught a 1-year SNES release-date
  error, NES discontinue mix-up, PSX manufacturer precision, units_sold
  caveats), `49e311b` (Pass B: 11 Wave 1 entries re-verified —
  caught wrong `virtualboy` RAM by ~3×, wrong `tg16` units_sold by
  ~2.5×, wrong `pce-cd` Arcade Card size by ~8×, plus several
  manufacturer + release-date + storage-cap fixes). Methodology:
  WebFetch Wikipedia per system → identify verifiable fields → drop
  unsourced numbers → write with established blurb voice →
  `meta.contributors` carries source URLs. 17 of 46 systems now
  Wikipedia-sourced. Remaining 29 (Pass C1/C2/C3) tracked at
  [NEXT.md](NEXT.md) DOC / DATA / TRIAGE section.

**2026-06-03**
- SETTINGS declutter — System Health hub + Game-media status-first
  cards. Sidebar shrank 16 → 12. Merge `dd430e4`. Plan:
  [_archive/PLANS/settings-declutter-system-health.md](_archive/PLANS/settings-declutter-system-health.md).
- Background jobs registry + persistent progress bar — 7-phase arc
  + polish complete. Plan:
  [_archive/PLANS/background-jobs-and-progress-bar.md](_archive/PLANS/background-jobs-and-progress-bar.md);
  feature folder:
  [_archive/features/background-jobs/](_archive/features/background-jobs/).

**2026-06-02**
- Per-system descriptor consolidation — Slices 1+2 shipped (~2,750
  LOC removed; 46 systems load from `config/systems/<id>/`). Slice 3
  (L3 content packs + L4 SQLite + JSON Schema + CI lint) still
  queued. Plan: [PLANS/per-system-descriptors.md](PLANS/per-system-descriptors.md).

**2026-06-01**
- Guided Setup Phase 1B — wizard upgrade closed (6 slices in a
  single day). Phase 2 (curated CPU-tier core selection) queued.
  Feature: [features/guided-setup/](features/guided-setup/).
- MAME ROM-set name resolution (listxml metadata pass).
- System Info Panel v1 — 4-table SQLite schema + per-system YAML
  curation. Plan:
  [_archive/PLANS/system-info-panel-v1.md](_archive/PLANS/system-info-panel-v1.md).

**2026-05-30 / 2026-05-31**
- NDS per-game touch hotspots overlay.
- Legacy Shell deletion (-1,860 lines across 13 files).
- Retroverse migration follow-ups (drop overlay + header
  affordances + Help-dialog Retroverse home).
- Gameplay fixes batch — NDS multi-touch + lightgun gun-side
  buttons + SNES Super Multitap.
- Game Info Panel v1 — 3-layer data model + 6 Tauri commands +
  4-tab modal. Plan:
  [_archive/PLANS/game-info-panel.md](_archive/PLANS/game-info-panel.md).
- libretro env-callback batch (four gaps closed).

**2026-05-26 / 2026-05-27**
- Three new systems — jagcd / sega32xcd / stv (Phase 0 wiring done;
  operator playtest pending BIOS + ROM acquisition).
- Per-System Custom UI Stage 1 Slices 1-5 (code arc complete;
  Slices 6-9 await operator content).
- Controller-nav v2 polish + completion pass + Phase 0 primitives.
  Feature: [_archive/features/controller-nav/](_archive/features/controller-nav/).

**Earlier (2026-05-20 → 2026-05-25)**
- ColecoVision keypad reference + GameCube Wii peripherals.
- System fixes pass — MAME / light-gun IS_OFFSCREEN / Saturn 3D Pad
  + Atari 7800 twin-stick labels / NDS stylus reticle.
- DOSBox + ScummVM onboarding plan locked.
  Feature: [features/dosbox-and-scummvm/](features/dosbox-and-scummvm/).
- Media taxonomy (5-slot → 26-slot LaunchBox shape).
  Feature: [_archive/features/media-taxonomy/](_archive/features/media-taxonomy/).
- Window geometry persistence + tile-size slider.
- Portable install (`<exe_dir>/settings/` via `portable.txt`).
  Feature: [_archive/features/portable-install/](_archive/features/portable-install/).
- Docs audit + reorg (the originating cleanup arc).
- Sidebar tier + view editor + UI polish.
  Features:
  [_archive/features/sidebar/](_archive/features/sidebar/),
  [_archive/features/ui-polish/](_archive/features/ui-polish/).

For full per-arc detail, see [_archive/INDEX.md](_archive/INDEX.md).

---

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
