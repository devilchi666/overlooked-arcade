# Controller Navigation — Session Log

## 2026-05-26 — v2 polish: QuickSettings sub-views + sidebar widgets + MenuBar identity

Follow-on branch `feat/controller-nav-v2-polish` closes the three
LOWER-band items NEXT.md carried as "Controller-nav v2 polish (operator-
driven)." The completion-pass merge left these surfaces on mouse +
keyboard so operator playtest could decide whether to invest the time;
this branch ships all three as one batch.

- **Shipped:**
  - **Slice 1 — QuickSettings sub-views (`b87493d`):** rewind / TAS /
    video / memory / disc panels each gain a focus group + back
    handler. A new `nav/focus.ts::useDomQueryFocusGroup` helper
    generalizes the MenuBar pattern (DOM-query + MutationObserver +
    `data-oa-focus` mirror) with identity-tracked focused element, so
    dynamic content like the TAS recordings list re-binds
    automatically when rows appear / disappear. The rewind scrubber
    is a fixed three-item group [strip, cancel, commit] with an
    `onDirection` override so DPad left/right scrubs the timeline
    when the slider is focused. TasPanel + VideoPanel mirror the
    disabled Back button on B (operator must stop the operation
    first, same as the mouse path). Text inputs (recording name,
    capture name, hex offset) and the region `<select>` stay mouse +
    keyboard — a gamepad can't usefully drive them.
  - **Slice 2 — Right-sidebar widgets DPad browse (`c883af3`):** the
    right-sidebar body becomes one DOM-query group keyed by
    `data-oa-sidebar-row`. Read-only widget sections (cover / title /
    metadata) get focusable wrappers; the existing Play / Saves /
    Game-info action row keeps `data-oa-action="…"` markers and
    `onActivate` routes by the attribute. R1 from the library grid
    still lands on Play — a `createEffect` snaps `focusedIndex` to
    `widgetCount()` while the group is inactive, so the next R1 hits
    the first action rather than whichever row was last on. Operators
    DPad up through widget rows to glance at cover / metadata; A on a
    widget row is a no-op (read-only). Right-sidebar header utility
    chrome (pin toggle, sidebar-hide button) stays mouse-only by
    design — those aren't in the play path.
  - **Slice 3 — MenuBar identity-tracked focus (`567d0de`):** the
    `MutationObserver` rebind in `frontend/src/layout/MenuBar.tsx::Menu`
    now captures the focused button element identity each time the
    mirror effect lands the ring; on the next rebind it looks the
    button up by `indexOf(lastFocusedBtn)` and updates `focusedIndex`
    if its position shifted. Closes Slice K's known limitation —
    when a disabled→enabled flip inserts a row before the focused
    index, the ring follows the same logical button rather than
    staying at the same numeric position. `lastFocusedBtn` clears on
    menu close so the next open starts fresh.
- **Almost:** Operator playtest with a real controller across all three
  surfaces. Branch is local-only and clean.
- **Next:** After operator validation, merge `feat/controller-nav-v2-polish`
  `--no-ff` to main and close NEXT.md LOWER band #1 (three of its four
  bullets — pin toggle + sidebar-hide button stay mouse-only by design).

## 2026-05-26 — Completion pass: Slices F–M cover the rest of the UI

Phase 0 (A–E) merged earlier today; this entry covers the follow-on
branch `feat/controller-nav-completion`, which extends focus + back-stack
coverage to every interactive surface left on the table by the POC.

- **Shipped:**
  - Slice F — critical polish + missing primitives (`102eef8`): global
    back-stack in `nav/back.ts` (mount order = stack order; B pops the
    innermost handler before falling through to the active group's
    onCancel); sidebar containers now take focus and DPad left/right
    collapses/expands them; Y on a tile opens `GameInfoModal`; HintBar
    swaps A↔B glyphs when the A/B-swap setting is on.
  - Slice G — context + overlay menus (`8254aa1`): TileContextMenu,
    SystemContextMenu, SaveSlotsModal and the QuickSettings actions
    panel all consume `useFocusGroup` + back-stack. Both context menus
    were refactored to a data-driven `items()` memo so the flat focus
    index maps 1:1 onto rendered rows regardless of conditional
    sections. QuickSettings sub-views (rewind / TAS / memory / video)
    stay mouse + keyboard for now.
  - Slice H — game info modal + Dialog primitive auto-back
    (`6cb86d9`): `GameInfoModal` routes A→Launch / B→Close / Y→Resume
    / L1+R1→cycle tabs via new `onShoulderL` / `onShoulderR` overrides
    in `focus.ts`. The `Dialog` primitive itself auto-mounts a
    `useBackHandler` and publishes a baseline `HintRegion ({ b: "Close" })`
    for every Show-branch instance, so every modal in the app
    (Display / Audio / Gameplay / Shaders / System Settings / System
    Bindings / Core Options / Game Properties / Game Display / Game
    Input / Game Rewind / Game Shaders / Cheats / About / Keyboard
    Shortcuts / Debug Log / Platform Media / Widget Customizer /
    Screenshot Gallery / etc.) closes on B without further wiring.
  - Fix — suppress Web Gamepad poll while gilrs owns input (`662cd5a`):
    operator reported menus opening mid-gameplay. New gate in App.tsx
    walks four cases (nav disabled, no game, single-window+game,
    two-window+game) and keys the two-window case on DOM focus events
    on the library WebView's `window` rather than Tauri 2's
    `is_focused` (see existing feedback memory). Seeded from
    `document.hasFocus()` so the first frame is correct.
  - Slice K — top toolbar menu bar (`d68ab7f`): Start opens the first
    menu globally; DPad navigates items; L1/R1 cycle between menus.
    `MenuBar` exposes `requestOpenFirstMenu()` plus a counter signal so
    repeat Start presses re-open the bar. Item discovery is a DOM
    query of `[role^="menuitem"]` after each open via
    `queueMicrotask`; dynamic-during-open menu contents would need
    re-binding (deferred).
  - Slice L — chained popovers (`8180a0e`): `CorePickerMenu` and
    `RegionPicker` (opened from `TileContextMenu`) consume the focus
    group + back stack so the X-menu chain has no dead ends. Both
    publish their own `HintRegion`; cleanup re-activates `library-grid`
    so backing out lands the operator on the originating tile.
  - Slice M — right sidebar widget actions (`e721e7d`): the action row
    (Play / Saves / Game info) is a focus group with R1 from the
    library grid as its right neighbour; read-only widgets above the
    row and utility controls (pin toggle, sidebar-hide button) stay
    mouse-only for v1 — the play path is "pick a tile → R1 → activate."
    Library-grid HintRegion gains `r1: Widgets` so the operator sees
    the binding.
- **Operator-found bugs + fixes (post-push, same session):**
  - Library grid DPad left/right hit a wall at row edges — the grid
    branch in `nav/focus.ts::applyDirection` was clamping to column-
    only movement. Fixed in `792f17d` so left/right walk the flat
    index linearly across rows; up/down still jump by `cols`. Same
    fix flows through to `SaveSlotsModal`.
  - Menu toolbar up/down "didn't work" — focus.ts was routing correctly
    and even calling `.focus()` on each new button, but Tailwind's
    preflight strips the default browser outline and `Menu` wasn't
    writing OA's `data-oa-focus` pattern. Fixed in `dc25ab4` (mirror
    `focusedIndex` into `data-oa-focus` + `data-oa-focus-active` on
    bound buttons), plus three audit-driven follow-ups in the same
    commit: queryButtons filters disabled rows so DPad skips them;
    a `MutationObserver` on the popover re-binds on `disabled`-attr
    flips or content changes mid-open (race-guarded so an open-then-
    close faster than a microtask can't leak); and `index.css:242`
    broadens the dashed-dim inactive ring rule to
    `:not([data-oa-focus-active="true"])` so it matches both the
    literal "false" and the absent-attribute case used by seven
    completion-pass components.
- **Shipped (merge close-out):** Branch merged `--no-ff` to main as
  `feat/controller-nav-completion`. With Phase 0 + the completion pass
  in, Per-System UI Stage 1 is the next major arc.

## 2026-05-26 — Stream opened + all five slices landed

- **Shipped:**
  - Plan locked, feature folder created, branch `feat/controller-nav-primitives` cut from main.
  - Three design calls confirmed with operator (pad source = Web Gamepad API; POC scope = grid + sidebar; focus ring = 2px subtle outline).
  - Slice A — `nav/gamepad.ts` Web Gamepad API rAF poller + synthetic NavEvents (commit `ca3dff9`).
  - Slice B — `nav/focus.ts` useFocusGroup hook with vertical / horizontal / grid orientations + shoulder-bumper neighbour transfer (`d8a5ffb`).
  - Slice C — `nav/HintBar.tsx` persistent footer + module-stack `HintRegion` provider (`a3a54b3`).
  - Slice D — VirtualLibraryGrid + LeftSidebar consume the focus group; HintRegion at App root publishes labels per active group (`49522ab`).
  - Slice E — `Settings → Display → Controller navigation` panel with master toggle, nav source picker, A/B swap, animation budget. Settings push live to the gamepad poller + focus manager + CSS var via three `createEffect`s in App.tsx.
- **Almost:** Operator playtest with a real gamepad. Branch is pushed and ready to merge once verified.
- **Next:** After operator validation, merge `feat/controller-nav-primitives` to main with `--no-ff`, close Phase 0 in `docs/NEXT.md`, and pick up Per-System UI Stage 1 (next arc in the pipelined sequence).
