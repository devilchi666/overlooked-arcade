# Controller Navigation — Roadmap

Phase 0 of the guided-setup pipeline. Branch: `feat/controller-nav-primitives`.

## Slice A — Gamepad → UI event layer

- ⬜ rAF poller reading `navigator.getGamepads()`
- ⬜ Synthetic event types: `nav-button` (A/B/X/Y/Start/Select/L/R), `nav-direction` (up/down/left/right)
- ⬜ Press / release / repeat (initial delay 400ms, repeat rate 80ms)
- ⬜ Deadzone for analog stick (0.4 default; configurable)
- ⬜ Single event bus consumed by focus manager + hint bar

## Slice B — Focus manager + focus-ring pattern

- ⬜ `<FocusGroup>` Solid component — registers child focusables, owns selection index
- ⬜ Roving-tabindex helper (only current focusable has `tabindex=0`)
- ⬜ `useFocus()` primitive for focusables to bind handlers
- ⬜ Focus-ring CSS class (`.oa-focus-ring` — 2px solid accent color, 8px radius)
- ⬜ Wired to gamepad event bus from Slice A

## Slice C — On-screen hint bar

- ⬜ `<HintBar>` component pinned to bottom of viewport
- ⬜ Per-screen `<HintRegion>` provider — registers A/B/X/Y labels via context
- ⬜ ≤120ms animated label changes
- ⬜ Auto-hide when no gamepad has been seen this session

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
