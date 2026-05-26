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
- ⬜ ≤120ms animated label changes (current: instant — will revisit in Slice E once animation budget setting is wired)

## Slice D — POC wiring

- ⬜ VirtualLibraryGrid: DPad moves tile selection, A launches, X opens TileContextMenu
- ⬜ LeftSidebar tree: DPad navigates nodes, A activates view
- ⬜ Shoulder bumpers (L/R) toggle focus between sidebar and grid
- ⬜ B exits any open menu / cancels selection
- ⬜ Mouse + keyboard still work alongside controller (no input mode lock-out)

## Slice E — Settings → Controller-nav

- ⬜ `Settings → Display → Controller navigation` panel
- ⬜ Master toggle (controller-nav on/off entirely)
- ⬜ Nav source: DPad / left stick / both (default: both)
- ⬜ A/B swap (for Nintendo-convention users)
- ⬜ Animation budget: snappy (0ms) / subtle (120ms, default) / animated (250ms)
- ⬜ Persisted in settings store

## Gate to Phase 1

When all five slices land + operator validates the POC with a real controller,
Phase 0 closes and Phase 1 (wizard upgrade) can start.
