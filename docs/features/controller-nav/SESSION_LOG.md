# Controller Navigation — Session Log

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
- **Almost:** Operator playtest with a real controller across every
  newly-wired surface — the branch is local-only; nothing pushed yet.
  Branch tree is clean.
- **Next:** Push `feat/controller-nav-completion`, operator playtest,
  then merge `--no-ff` to main and update `NEXT.md` / `ACTIVE_WORK.md`
  to reflect the closed completion pass. After merge, the next arc in
  the pipelined sequence is Per-System UI Stage 1.

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
