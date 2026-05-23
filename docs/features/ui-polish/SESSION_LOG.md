# UI Polish — Session Log

Entries for the menu-bar IA redesign + dialog reorganization work.
Originally filed under whichever per-core SESSION_LOG was active at the
time (mostly `docs/cores/nds/SESSION_LOG.md`, with two 2026-05-18 entries
under `docs/cores/tg16/SESSION_LOG.md`) — re-filed here 2026-05-22 as part
of the docs reorg so cross-cutting work has a proper home.

---

## 2026-05-22 — UI polish PR 3 + PR 4 (Phases D + E, cross-system)

Final two PRs of the polish-plan execution, bundled per operator request.
Plan now fully shipped — sibling `UI_POLISH_PLAN.md` complete.

- **Shipped:** PR 3 + PR 4 of 4 from sibling `UI_POLISH_PLAN.md`.
  - **Phase D — drawer shrink + Game-menu dialog extraction:**
    - New `GameDialogs.tsx` (~1733 lines) with seven focused
      single-purpose dialogs: `GameCoreOptionsDialog`,
      `GameDisplayDialog`, `GameInputDialog`, `GameRewindDialog`,
      `GameShadersDialog`, `MilestonesDialog`, `CheatsDialog`. Shared
      `useGameOverrides()` composable owns hydration + the patch
      helper; each dialog uses the appropriate Dialog size from PR 2
      (Cheats / Milestones / Input / Core options / Display at xl).
    - `PerGameSettingsDrawer.tsx` (1933 lines, 10 tabs) collapsed to
      `GamePropertiesDialog.tsx` (~225 lines) with only Overview +
      Core in two `<DialogSection>`s at xl. Region tab deleted
      entirely (no runtime effect; duplicated boxart RegionPicker
      semantically). Drawer chrome (slide-in, tab strip, custom Esc
      handler, custom backdrop) all gone — Dialog primitive handles
      them uniformly.
    - `App.tsx` Game ▾ menu rewires the 7 deep-link items to a single
      discriminated `gameDialog` signal `{ kind, target }`. Properties
      keeps opening the slim Properties dialog. Old "ROM patch…" menu
      item retired (folded into Properties → Core); "Input…" takes
      its slot.
    - Cheats + Milestones implementations carried over largely
      verbatim (4-stage cheat-search state machine + MilestoneEditor
      draft pattern are fiddly enough that mechanical extraction is
      the right risk profile). New behavior: `CheatsDialog`
      auto-ends an in-flight cheat search if the dialog closes
      mid-search (avoids orphaned Rust-side session).
  - **Phase E — kiosk shell hooks:**
    - `--kiosk` CLI flag added to `Cli` (clap). `parse_and_resolve()`
      now returns `CliConfig { direct_launch, kiosk }` rather than
      `Option<DirectLaunchConfig>` — keeps kiosk orthogonal to ROM
      presence so the flag works alone for testing.
    - `AppState.kiosk` + `get_kiosk_mode` Tauri command surface the
      flag to the frontend.
    - `LayoutStore` onMount: after hydrating `presentation.json` but
      before `setHydrated(true)`, reads `get_kiosk_mode`; if true
      forces `setPresentationMode("cabinet")`. The write-through
      effect is gated on `hydrated()`, so this runtime override
      doesn't persist to disk. Operator's on-disk preference is
      preserved for the next library-mode launch.
    - `chromeVisible()` memo added in `App.tsx` —
      `!isDirectLaunch() && !gameMode()`. Zero behavior change today;
      pre-wired so Phase 1 of the kiosk plan only has to extend the
      memo body to gate menu bar + toolbar + sidebars off when a
      future PresentationMode variant lands.
- **Almost:** —
- **Next:** Polish plan complete. Kiosk Phase 1 (the actual kiosk
  shell — `../kiosk-shell/KIOSK_PLAN.md`) is the next polish-adjacent
  block but not next-up; we return to per-core work.

---

## 2026-05-22 — UI polish PR 2 (Phases B + C, cross-system, not core-specific)

Continues the polish-plan execution. PR 1 (Phase A) landed earlier today.

- **Shipped:** PR 2 of 4 from sibling `UI_POLISH_PLAN.md`.
  - Dialog primitive: size scale widens to sm/md/lg/xl/2xl; new
    `<DialogSection>` for row grouping; type ramp + spacing + SVG
    close-button glyph.
  - SettingRow: built-in `select` / `slider` / `toggle` controls;
    typed `inherited` + new `description`, `disabled`, `onReset`
    props; exports `selectClass(tone)` as the canonical select-
    styling helper. Legacy `inheritedValue` / `inheritedFrom`
    pair kept as a passthrough during migration.
  - DisplayDialog migrates at xl as the reference (three sections:
    Scaling / Window / Run-ahead).
  - Audio / Gameplay / Shaders dialogs adopt built-in controls.
  - SystemDialogs + PerGameSettingsDrawer bloom sliders collapse to
    `SettingRow.slider` + `onReset`.
  - 3 of 4 LibraryManagerPage row candidates migrated (only-sync,
    auto-remove, revision-tiebreaker). Action-select for
    "Clear games for" stays raw + uses the new `selectClass("oa")`
    helper — the DOM-reset idiom after each pick doesn't fit a
    controlled built-in.
  - Three duplicate SELECT_CLASS constants deleted (SettingsDialogs,
    SystemDialogs, LibraryManagerPage); single source of styling
    is now `selectClass()` in SettingRow.
- **Almost:** —
- **Next:** PR 3 from sibling `UI_POLISH_PLAN.md` — Phase D, the biggest PR.
  Shrink `PerGameSettingsDrawer` (10 tabs → 2: Overview + Core) and
  extract 7 Game-menu dialogs (`GameCoreOptionsDialog`,
  `GameDisplayDialog`, `GameInputDialog`, `GameRewindDialog`,
  `GameShadersDialog`, `MilestonesDialog`, `CheatsDialog`). Delete the
  Region tab (no runtime effect; duplicates boxart RegionPicker
  semantically). Depends on the `xl` size from PR 2 — Cheats /
  Milestones / Input / Core options / Display all want the room.

---

## 2026-05-22 — UI polish PR 1 (Phase A cleanup, cross-system, not core-specific)

Originally logged under nds (the active core); the work itself is cross-
cutting UI shaped by sibling `UI_POLISH_PLAN.md` (Phase 0 of the kiosk
plan). See sibling `UI_POLISH_PLAN.md` §1 for the full Phase A spec.

- **Shipped:** PR 1 of 4 from the polish plan. `SettingsPage.tsx` →
  `LibraryManagerPage.tsx` (heading, localStorage key, warn prefixes,
  dead `moveRegion` helper removed). `SidebarView` discriminant
  `"settings"` → `"library-manager"` across `App.tsx`, `LeftSidebar`,
  `LibraryView`, `filter.ts`. Bottom Cores + Settings buttons on the
  left sidebar deleted (collapse toggle preserved). Stale
  `PerSystemSettingsPage` doc-comments repointed across `CoresPage`,
  `PerGameSettingsDrawer`, `SystemBindingsEditor`. Sibling `UI_AUDIT.md`
  gains a staleness header pointing at the polish plan.
- **Almost:** —
- **Next:** PR 2 from sibling `UI_POLISH_PLAN.md` — Phase B + C bundled
  (Dialog primitive polish: new `sm/md/lg/xl/2xl` size scale,
  `<DialogSection>` component, type-ramp updates; plus `SettingRow`
  canonicalization: built-in `select/slider/toggle` controls,
  `description` prop, `disabled` + `onReset` props, delete three
  duplicate `SELECT_CLASS` constants. `DisplayDialog` migrates as
  the reference).

---

## 2026-05-18 — Menu-bar redesign (project-wide IA shift)

- **Shipped:** Top-bar menu-bar redesign (sibling `UI_AUDIT.md` + sibling `UI_MENU_BAR_PLAN.md` drafted first, then 9 of 10 steps shipped). Seven named menus replace the scattered Settings/Cores/⚙ entry points: Library · View · System ▾ · Game ▾ · Tools · Settings · Help. New primitives: `MenuBar` + `Menu` + `MenuItem` + `MenuRadio` + `MenuCheckbox` + `MenuLabel` + `MenuDivider` (`frontend/src/layout/MenuBar.tsx`), `Dialog` (`frontend/src/layout/Dialog.tsx`). 16 settings dialogs across `SettingsDialogs.tsx` (OA-wide Display/Audio/Gameplay/Shaders), `SystemDialogs.tsx` (per-system Bindings/Display/Rewind/Shaders/DefaultCore/CoreOptions), `HelpDialogs.tsx` (Shortcuts/About). Retired `PerSystemSettingsPage.tsx` entirely; trimmed `SettingsPage` from 7 tabs → 2 (Library + Game media only). Trimmed `QuickSettings` action grid from 10 rows → 4 verbs (Resume / Saves / Game info / Exit); the drill-in panels are now reached via the Tools menu which can deep-link the overlay to a specific view. View menu owns sort/group/view-mode/sidebar toggles/presentation mode — `GridControls` collapsed to title + count. Trimmed 4 disabled "soon" Quick Destinations from the left sidebar, dropped 4 placeholder tabs (Audio/Theme in per-system, Audio/Input in per-game). Surfaced 2 orphans (Keyboard shortcuts cheatsheet, About) under Help ▾.
- **Almost:** Step 10 visual polish (icon set, type ramp, accent usage) not done — needs visual eye on the live dev server. Remaining orphans (Screenshot gallery, Performance HUD, Right-sidebar widget customizer) not surfaced — they need new backend wiring / new Rust commands.
- **Next:** Sanity-check the bar in dev server, commit slice, then polish pass + orphan wiring.

## 2026-05-18 — Menu-bar redesign: deferred work shipped

- **Shipped:** Widget customizer dialog (`View → Customize widgets…`) — drives the existing `widgetOrder` + `widgetHidden` layout fields, finally surfacing the right-sidebar reorder/hide UI that the store had supported since Phase 2. Screenshot gallery (`Tools → Screenshot gallery`) — added Rust commands `list_screenshots` / `delete_screenshot` / `open_screenshot_folder` following the existing `list_video_clips` pattern; frontend dialog grids thumbnails with delete + open-folder actions. Performance HUD overlay (`Tools → Performance HUD` checkbox) — pure frontend `requestAnimationFrame` counter that surfaces UI render FPS + frame time as a small fixed-position chip in the top-right; honest about scope (says "UI" not "FPS") so future emu-side telemetry can plug in without changing the HUD shell. Polish: removed `▶`/`🚪` emoji from `Game ▾` Launch/Exit items to stay consistent with the "stylized text" decision; tightened `MenuItem` gap from `gap-3` to `gap-2` to match `MenuRadio` + `MenuCheckbox`. `cargo check -p oa-shell` green; `tsc --noEmit` green.
- **Almost:** Emu-side performance telemetry not exposed yet — `SharedPerfStats` Rust struct + `get_perf_stats` command would unlock displaying real emulator fps in the HUD. Decided against threading the change through the EmuLoopArgs structs in a no-live-verify session.
- **Next:** Visual check the new dialogs (widget customizer, screenshot gallery, perf HUD) in dev server, commit slice.
