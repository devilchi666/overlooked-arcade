# Controller Navigation — Roadmap

Phase 0 of the guided-setup pipeline. Branch: `feat/controller-nav-primitives`.

## Slice A — Gamepad → UI event layer

- ⬜ rAF poller reading `navigator.getGamepads()`
- ⬜ Synthetic event types: `nav-button` (A/B/X/Y/Start/Select/L/R), `nav-direction` (up/down/left/right)
- ⬜ Press / release / repeat (initial delay 400ms, repeat rate 80ms)
- ⬜ Deadzone for analog stick (0.4 default; configurable)
- ⬜ Single event bus consumed by focus manager + hint bar

## Slice B — Focus manager + focus-ring pattern

- ✅ `useFocusGroup` hook — registers a group, returns helpers (`isActive`, `activate`, `bind`) (in frontend/src/nav/focus.ts)
- ✅ Index-based focus model: parent owns focusedIndex signal, group reads/writes it; works with virtualized lists
- ✅ Per-orientation direction handling: vertical / horizontal / grid (columns accessor)
- ✅ Shoulder-bumper transfer to neighbour groups (L1/R1)
- ✅ Button routing: A/B/X/Y/Start → onActivate/onCancel/onSecondary/onTertiary/onStart
- ✅ Focus-ring CSS via `[data-oa-focus="true"]` (2px solid system accent, 8px radius, --oa-focus-anim-ms transition)
- ✅ Inactive-group ring style (dashed dim outline) so the operator sees the bumper-back target

## Slice C — On-screen hint bar

- ✅ `<HintBar>` component pinned to bottom of viewport (in frontend/src/nav/HintBar.tsx, mounted at App root)
- ✅ Per-screen `<HintRegion hints={...}>` provider — innermost wins via module-level mount-order stack
- ✅ Auto-hide when no gamepad has been seen this session (via `hasSeenGamepad` reactive accessor)
- ✅ Auto-hide when no HintRegion is mounted (game running unmounts library → empty stack → bar hidden)
- ✅ Focus-ring transition respects `--oa-focus-anim-ms` (Slice E wires the settings → CSS var bridge)

## Slice D — POC wiring

- ✅ VirtualLibraryGrid: DPad moves tile selection, A launches, X opens TileContextMenu (in frontend/src/components/VirtualLibraryGrid.tsx)
- ✅ LeftSidebar: DPad navigates "All Games" + visible leaves, A opens, X opens system context menu (in frontend/src/layout/LeftSidebar.tsx)
- ✅ Shoulder bumpers (L1/R1) transfer focus between sidebar and grid via `neighbours` config
- ✅ HintRegion at App root publishes A/X/L1/R1 labels based on active group
- ✅ Mouse hover + click still work — `onFocus` callback explicitly activates the group + updates focus
- ✅ Focused tile scrolls into view via `virtualizer.scrollToIndex({align:"auto"})` — no-op when already visible
- ⬜ Sidebar containers don't take focus (DPad nav skips them); deferred to a later polish — operators can still toggle expansion with mouse
- ⬜ B (cancel) doesn't yet clear selection or close menus — needs Slice E global semantics decision

## Slice E — Settings → Controller-nav

- ✅ `Settings → Display → Controller navigation` panel (in frontend/src/components/SettingsDialogs.tsx::DisplayDialog)
- ✅ Master toggle — flips `setNavEnabled(...)` on the gamepad poller; all NavEvents suppressed when off
- ✅ Nav source: DPad / left stick / both — `setNavSource(...)` filters poller emit
- ✅ A/B swap — `setSwapAB(true)` in focus.ts renames the A↔B button before routing the switch
- ✅ Animation budget: 0 / 120 / 250 ms — writes `--oa-focus-anim-ms` on documentElement; CSS transition reads it
- ✅ Persisted under existing `oa.settings.v1` localStorage payload with validation in `load()` + emit on `save()`

## Gate to Phase 1

When all five slices land + operator validates the POC with a real controller,
Phase 0 closes and Phase 1 (wizard upgrade) can start.

**Slice status (2026-05-26):** A/B/C/D/E shipped on
`feat/controller-nav-primitives`; operator-confirmed working before
merge to main (`733f8a1`, `--no-ff`, 2026-05-26).

---

# Completion pass (post-Phase 0)

Phase 0 (A–E) covered the **POC** — library grid + sidebar + Settings.
A follow-on branch `feat/controller-nav-completion` extends focus + back
coverage to every remaining interactive surface so the operator can run
the whole shell from a pad. Each slice lives on its own commit on that
branch.

## Slice F — Critical polish + missing primitives (in commit 102eef8)

- ✅ Global back-stack: `nav/back.ts::useBackHandler` registers a close fn for the lifetime of its reactive scope (mount order = stack order); `focus.ts` B handler calls `popBack()` first, falls through to active group's `onCancel` when empty
- ✅ Sidebar containers focusable: DFS-built nav list honors the renderer's expand state (explicit-expanded ∪ active-leaf auto-expand); DPad left collapses, right expands then descends; A on a container navigates to its view-node
- ✅ Y on a tile opens the game info modal (VirtualLibraryGrid `onShowInfo` prop → focus group `onTertiary`)
- ✅ HintBar swaps A↔B glyphs under their labels when the A/B-swap setting is on (cosmetic mirror of the dispatch swap; `isSwapAB` reactive accessor)

## Slice G — Context + overlay menus (in commit 8254aa1)

- ✅ `TileContextMenu` (frontend/src/components/TileContextMenu.tsx): data-driven `items()` memo so conditional sections fold into one ordered list; A activates, X fires per-row secondary (variant pin/unpin), B/Esc/click-outside close; `onCleanup` re-activates `library-grid`
- ✅ `SystemContextMenu` (frontend/src/components/SystemContextMenu.tsx): same refactor; "Move to category" sub-view feeds a different `items()` list; B steps Back from sub-view rather than closing the whole menu
- ✅ `SaveSlotsModal` (frontend/src/components/SaveSlotsModal.tsx): grid focus group at 5 columns; A launches focused slot, X deletes, B closes
- ✅ `QuickSettings` actions view (frontend/src/components/QuickSettings.tsx): new `ActionsPanel` sub-component owns its focus group + back handler
- ⬜ QuickSettings sub-views (rewind / TAS / memory / video) stay mouse + keyboard for now — deferred

## Slice H — Game info modal + Dialog primitive auto-back (in commit 6cb86d9)

- ✅ `GameInfoModal` (frontend/src/components/GameInfoModal.tsx) as a single primary-action surface: A→Launch / B→Close / Y→Resume from latest slot (when one exists) / L1+R1→cycle Screenshots / Title screens / Save states tabs
- ✅ New `onShoulderL` / `onShoulderR` per-group overrides in `focus.ts` that pre-empt the default neighbour-jump — used by GameInfoModal tabs; future overlays can use it for paginated content
- ✅ `Dialog` primitive (frontend/src/layout/Dialog.tsx) auto-mounts a `DialogBackHandler` registering `props.onClose` with the back stack, and publishes a baseline `HintRegion ({ b: "Close" })` — so every Show-branch modal in the app (Display, Audio, Gameplay, Shaders, System Settings, System Bindings, Core Options, Game Properties, Game Display, Game Input, Game Rewind, Game Shaders, Cheats, About, Keyboard Shortcuts, Debug Log, Platform Media, Widget Customizer, Screenshot Gallery, etc.) closes on B for free

## Fix — Suppress Web Gamepad poll while gilrs owns input (in commit 662cd5a)

- ✅ Four-case gate in `App.tsx` controlling whether the frontend Web Gamepad poller fires:
  - Nav disabled in Settings → always off
  - No game running → always on
  - Single-window shell + game running → on only when `libraryVisible()` or `quickSettingsOpen()`
  - Two-window shell + game running → on only when the library WebView holds OS focus, tracked via DOM `focus` / `blur` on its `window` (Tauri 2's `is_focused` is unreliable for the no-WebView game window — see existing feedback memory)
- ✅ `webviewFocused` seeded from `document.hasFocus()` so the first frame is correct before any event fires

## Slice K — Top toolbar menu bar (in commit d68ab7f)

- ✅ `MenuBar` (frontend/src/layout/MenuBar.tsx): module-level open signal + counter so repeat Start presses re-open the first menu; exported `requestOpenFirstMenu()` lets `App` drive open-from-Start globally
- ✅ `MenuBarContext.registerMenu(id)` + `menuIds()` lets each `<Menu>` register in mount order; L1/R1 cycle through the list
- ✅ Each `<Menu>` opens a focus group on mount: item count comes from a DOM query of `[role^="menuitem"]` inside the popover via `queueMicrotask`; A clicks focused button (existing handler closes + fires row action); B closes via back stack
- ✅ HintRegion per menu: A / B / L1 prev menu / R1 next menu; sidebar + library-grid HintRegions add "Menu bar" to their start hint
- ✅ Re-bind on dynamic-during-open menu content changes — shipped in `dc25ab4` via a `MutationObserver` on the popover (childList + subtree + `attributeFilter: ["disabled"]`) calling a `rebind()` closure that re-queries enabled buttons, sets itemCount, re-binds, and bumps a `domRev` signal so the focus-ring mirror repaints. Open-microtask race-guarded; `onCleanup` disconnects on dispose. Covered case: Library menu's "Scanning…" row flipping to enabled "Import folder…" when a background scan finishes.
- ⬜ Known limitation: when the observer fires because a button appears **before** the focused index (e.g. a disabled→enabled flip at position 0), the focused index now points at a different button than it did before — visual ring appears to shift. Solving requires identity tracking, not just index tracking. Acceptable for v1 since the trigger (disabled-attr flips on visible menus) is rare and brief.

## Slice L — Chained popovers (in commit 8180a0e)

- ✅ `CorePickerMenu` (frontend/src/components/CorePickerMenu.tsx): data-driven `items()` memo prepends a "(Default — auto-detect)" entry so flat index maps 1:1 onto rows; vertical orientation; A picks (clears override on the default row); B closes via back stack
- ✅ `RegionPicker` (frontend/src/components/RegionPicker.tsx): horizontal orientation, DPad left/right walks variants linearly regardless of row wrap; A picks, B closes
- ✅ Both publish their own HintRegion from the modal root; `onCleanup` re-activates `library-grid` so backing out lands on the originating tile

## Slice M — Right sidebar widget actions (in commit e721e7d)

- ✅ Right sidebar action row (Play / Saves / Game info) is a focus group (frontend/src/layout/RightSidebar.tsx); R1 from the library grid transfers in, L1 / B step back out to the grid
- ✅ Library-grid HintRegion gains `r1: Widgets`; right-sidebar group publishes A "Activate" / B "Library" / L1 "Library"
- ⬜ Read-only widgets above the action row (recently-played, system blurb, etc.) stay non-focusable in v1 — primary play path is "pick a tile → R1 → activate"
- ⬜ Pin toggle + sidebar-hide button in the header stay mouse-only for v1 — utility / configuration, not part of the play path

## Gate to merge

When the operator has playtested the completion-pass surfaces with a real
controller, the branch merges `--no-ff` to main and `docs/ACTIVE_WORK.md`
+ `docs/NEXT.md` close the completion pass. Per-System UI Stage 1
unblocks next per the pipelined sequence.

**Slice status (2026-05-26):** F/G/H/fix/K/L/M shipped on
`feat/controller-nav-completion`; branch is local-only and tree is clean;
awaiting operator playtest before push + merge.
