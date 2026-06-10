# Theming Substrate — Session Log

Each session that touches this feature appends a 3-line entry:
**Shipped / Almost / Next.** Older entries roll to
`SESSION_LOG_ARCHIVE.md` when this file passes ~150 lines
(rolled 2026-06-10 — Phase 4 Slices 1-3 + the grab-bag drain are in the
archive; live file keeps Phase 4 Slices 4-6 + Phase 4.5).

---

## 2026-06-10 — Phase 3 S2: walking skeleton (Retroverse ⇄ Wheel swap gate) — ⏳ awaiting playtest

> **The morale/de-risk milestone — the dream first becomes visible.** Branch
> `feat/theming-walking-skeleton`. Four S2 design decisions signed off
> (AskUserQuestion, all the recommended path) before any code.

- **Shipped** (all 5 S2 scope items + the two D20 seams + the boundary ratchet):
  - **Theme SDK contract** (`platform/theme/types.ts`): a theme = `{ manifest, entry }`;
    the entry is `Component<{ surface: "main" }>` (surface-aware, D20b) consuming ONLY
    platform (usePlatform stores + the host context + `@oa/platform/nav` + `@oa/platform/api`).
  - **Host context → platform** (`platform/theme/host.tsx`): `ThemeContextValue` /
    `ThemeProvider` / `useTheme` moved out of `routes/retroverse/context.tsx` (now a
    re-export shim — D15-style, ~11 importers unchanged) so EVERY theme consumes the
    same launch/saves/info/favorite host services. Adds `themePreempted()` — the
    general D20a preempt/restore seam (= `engineSurfaceOpen()` today; attract reuses it).
  - **Active-theme registry** (`platform/theme/registry.ts`): platform owns the
    `activeThemeId` signal + boot seed + picker list + `setActiveTheme` (persist→restart);
    App injects the concrete `BUILTIN_THEMES` via `registerThemes()` (platform ↛ themes,
    so App is the injection point — D13 pattern). Persisted on
    `LibraryPrefs.active_theme_id` (boot-loaded). App.tsx renders the active theme via
    `<Dynamic component={activeTheme().entry} surface="main"/>` (was hardcoded
    `<RetroverseShell/>`), gated on `activeThemeResolved()` (no default flash).
  - **Restart**: new Rust `restart_app` command via Tauri 2 `AppHandle::restart()`
    (no new plugin; mirrors `quit_app` cleanup) + `shellApi.restartApp()`.
  - **Retroverse = thin wrapper** (`themes/retroverse/index.tsx` → existing
    `RetroverseShell`; layout/routes stay put, full move is Phase 6). Default theme.
  - **Wheel = rough 2nd shell** (`themes/wheel/index.tsx`): full-bleed horizontal
    **coverflow** — centred scaled focused cover, neighbours fanning + dimming, metadata
    strip + Launch button below, Left/Right browse, Confirm launch, Game-info on
    Secondary. System-AGNOSTIC by choice (D19). Built on the S1 `ListNav` primitive
    (horizontal, controlled index) + `usePlatform` + `useMedia` covers. Honest caveat
    baked into the code + picker: layout/feel only — attract/CRT/ceremony is ARC 2-3.
  - **`EngineSummonIcon` re-homed** `engine/` → `platform/components/` (D12 — a leaf
    themes must mount belongs to the lowest consuming layer); both themes mount it, the
    operator's always-available path back to Settings → Themes. RetroverseShell's L1/R1
    gate switched to `themePreempted()`.
  - **Appearance picker**: filled in the existing OA-wide **Themes** Settings category
    (`ThemesSettings` in `engine/SettingsSections.tsx`) — lists registered themes,
    Switch button → in-app confirm → persist + restart. Stale Legacy-Shell card removed.
  - **`surfaces` field** added to `ThemeManifest` (D20b); **6 new lint zones**
    (platform↛themes, engine↛themes, themes↛{engine,routes(except retroverse),
    layout(except retroverse),components}); `themes↛engine` probe-verified to fire.
- **Verified:** `npm run typecheck` + `npm run lint` green; `cargo test -p oa-shell`
  = **822 passed** (incl. the `library_prefs` round-trip now carrying `active_theme_id`).
- **Decision D22** recorded (the 9-point implementation shape + the two
  most-easily-undone constraints: platform-owns-machinery/App-injects, and the
  retroverse lint `except`).
- **Almost:** nothing in S2 scope left. The nav-remap Settings UI is still the
  separate follow-on (after this gate, per D18).
- **Next (operator):** **playtest the swap gate** — boot (lands on Retroverse),
  browse + launch; F12 → Settings → Themes → Switch to Wheel → confirm → app restarts
  into the Wheel coverflow → browse + launch a game → switch back to Retroverse →
  indistinguishable from before. Then merge. **After merge: S3 — token layer**
  (`THEME_CONTRACT.md` + design tokens + a11y/motion baseline + engine-territory token
  isolation), per plan §13.3.

### S2 playtest round 1 (2026-06-10) — fixes + rename

- **Operator playtested; swap gate WORKS** (Retroverse ⇄ CoverFlow, browse + launch
  both). Three bugs found + fixed on the same branch, then re-confirmed working:
  - *Covers painted over the Settings surface* (z-index). The theme mount in App.tsx
    is now wrapped in an `isolation: isolate` stacking context — a theme's internal
    z-indexes can never escape above engine territory / platform modals. Substrate
    guarantee, applies to every theme.
  - *Controls did nothing.* The theme mounts late (async pref seed, behind a Show), so
    its `ListNav` focus group never claimed the active slot. Rebuilt the coverflow on
    `useFocusGroup` **directly** with an explicit `group.activate()` once games load.
  - *Perf:* `ListNav` rendered all 8541 game nodes (no virtualization). Windowed to ±8
    cards on a sliding CSS track, reconciled by stable RomEntry refs. Also added mouse
    click-to-centre + wheel-scroll so it's usable without controller nav.
- **Renamed the second theme `Wheel` → `CoverFlow`** (id `wheel` → `coverflow`, dir
  `themes/wheel/` → `themes/coverflow/`) per operator: what S2 ships is a coverflow
  IA; a true radial/arc **Wheel** is the separate `wheel` nav primitive, parked for S5.
  *Migration note:* a pref persisted as `activeThemeId:"wheel"` is now unknown → the
  registry falls back to the default (Retroverse) on next boot; re-pick CoverFlow once.
- typecheck + lint green throughout; 822 oa-shell tests unaffected (frontend-only).

## 2026-06-10 — Phase 3 S1: nav foundation (verb-native nav layer) — ✅ shipped + merged

> **Merged to main 2026-06-10** — operator playtested ("working as expected").

- **Shipped** on `feat/theming-nav-foundation` (all four S1 scope items + the
  two recommendations the operator approved — persistence-real + HintBar verb
  re-key + arrow-key keyboard):
  - **Relocated `src/nav/` → `platform/nav/`** (git mv: types/gamepad/back/
    focus/HintBar) + new modules; all **24 importers repointed to
    `@oa/platform/nav`** (one barrel `index.ts`). This **closes the Phase-2
    residual wrong-direction edges** (`platform/components/* → ../../nav/*`):
    those imports are now intra-platform. New ratchet lint zone
    **`platform/nav ↛ platform/components`** keeps the nav layer a generic leaf.
  - **Verb vocabulary** (`verbs.ts`): `Confirm`/`Back`/`Secondary`/`Tertiary` +
    directional `Up`/`Down`/`Left`/`Right` + `PrevSection`/`NextSection`/`Menu` +
    reserved-unbound `OpenQuickSettings`/`Search`/`Favorite`/`Page`. (Operator
    sign-off added `Secondary`/`Tertiary` to the plan's headline set — they're
    the X/Y focused-item roles the focus framework already dispatches.)
  - **Input→verb indirection** (`navBindings.ts`): OA-wide `NavBindings`
    (gamepad + keyboard channels) + `DEFAULT_BINDINGS` = the operator-locked
    controller-nav spec verbatim. Persisted in appData (`nav_bindings.json`) via
    new `platform/api/navBindingsApi.ts` + Rust `get/set_nav_bindings`
    (opaque-JSON blob, mirrors `audio.json`). `resolveButtonVerb` /
    `resolveKeyVerb` / `buttonForVerb` resolvers.
  - **A/B swap collapsed into a binding** — the old `swapAB` special-case is gone
    from `focus.ts`/`HintBar`; it's now a resolve-time overlay in `navBindings`
    (`setSwapAB`/`isSwapAB` moved there). `focus.ts` `routeEvent` resolves
    button→verb then dispatches by verb (`dispatchVerb`); focus-group callback
    names (onActivate/onCancel/…) kept stable so the ~15 consumers don't churn.
  - **HintBar is verb-native**: `Hints` re-keyed from physical buttons to verbs
    (`{ Confirm, Back, Secondary, … }` + `dpad`/`stick` descriptors) across **17
    call sites**; glyphs resolve **verb → currently-bound button → glyph** via
    the glyph-set seam (`glyphs.ts`, scope-call #4). Remap / swap re-paints every
    hint for free.
  - **`list` + `grid` primitives** (`primitives/`) — verb-native, declarative
    props (`density`/`focusProminence`/`easing`/data-source/neighbours, surfaced
    as `data-oa-*` seams for the S3 token layer; scope-call #8). Additive — they
    do **not** replace the bespoke VirtualLibraryGrid/LeftSidebar focus usage;
    they're the surface S2's Wheel/Retroverse skeletons consume.
  - **Keyboard**: arrow keys → directional nav at the focus layer (gated:
    nav-enabled, non-editable target, no Ctrl/Meta/Alt). Confirm already works
    natively on focusable buttons; Enter/Back/Esc keyboard verbs deferred to the
    remap follow-on (need a native-control coexistence audit). Schema carries
    both channels now.
- **Verified:** `npm run typecheck` + `npm run lint` green; `cargo test -p
  oa-shell` = **822 passed**. Operator playtested + merged to main 2026-06-10.
- **Decision D21** recorded (focus-group callback names kept; gamepad bus stays
  raw-event so the engine-summon chord + boot-skip are untouched; keyboard arrows
  emit source "dpad").
- **Behavior to watch in playtest:** arrow-key nav is newly live in the shell —
  arrows now move the active focus group (instead of scrolling) when focus isn't
  in an editable field. Everything else should feel identical (defaults = the
  locked spec).
- **Almost:** the remap Settings UI (the verb-rebinding surface) — deliberately
  the follow-on **after** the S2 swap gate, per D18.
- **Next:** **S2 — walking skeleton:** minimal active-theme switch (restart) +
  Retroverse wrapped as default theme + a rough **Wheel** second shell;
  switchable from Settings → Appearance, both browse + launch. The morale/
  de-risk milestone.

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
