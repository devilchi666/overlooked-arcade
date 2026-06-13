# Controller-Nav Coverage Audit — 2026-06-12

Read-only audit (Explore subagent) of which frontend screens/panes are
controller-navigable vs mouse/keyboard-only. Commissioned because the operator
couldn't tell which surfaces respond to a gamepad. Anchors are approximate —
verify before acting.

## Headline

- **The five Retroverse tabs + modals/menus have solid, consistent nav.** The
  3-region pattern (`useDomQueryFocusGroup` LEFT/CENTER/RIGHT + DPad transfer +
  delegation to `library-grid`) is used uniformly.
- **The big gap is the ENGINE SURFACE** — Settings category bodies, the
  metadata editor, import/per-game dialogs are form-heavy and keyboard/mouse
  only (no focus-group row nav).

## Coverage (condensed)

| Area | Navigable | Evidence |
| --- | --- | --- |
| RetroverseShell tab bar (L1/R1) | ✅ | `themes/retroverse/RetroverseShell.tsx` onNavEvent |
| Home / Library / Collections / Play Now / Discover | ✅ (Library fullest) | each page's `useDomQueryFocusGroup` 3-region + delegate |
| VirtualLibraryGrid / DetailListView | ✅ | `useFocusGroup` id `library-grid` (2D / vertical) |
| GameInfoModal, SaveSlotsModal, RegionPicker | ✅ | `useFocusGroup` + `captureFocusReturn` + back stack |
| Core/System/Tile context menus | ✅ | `useFocusGroup` vertical lists |
| Dialog wrapper (inert takeover) | ✅ | `Dialog.tsx` inert group claims active, suppresses leaks |
| QuickSettings (in-game) | ⚠️ partial | actions grid wired; rewind scrubber custom/unclear |
| Home / Play Now carousels | ❌ | arrows keyboard-focusable, no focus group |
| GameDetailPanel (panel mode) | ⚠️ partial | read-only; PLAY/MORE buttons ungrouped |
| **SettingsPanel bodies** | ⚠️ sidebar only | sidebar nav works; category forms keyboard-only |
| **MetadataSettingsBody / PerSystemSettingsBody** | ❌/⚠️ | form-heavy, no row focus-group |
| **Engine dialogs** (DebugLog, Help, Import steps, per-game props) | ❌ | native keyboard focus only |

## Biggest gaps (priority order)

1. **Settings category bodies** (high traffic) — sidebar navigable, but the
   per-category forms need row-by-row nav. Recipe: wrap each body in
   `useDomQueryFocusGroup` with a `[data-setting-row]`-style selector.
2. **Engine dialogs / complex forms** (metadata editor, import, per-game
   properties) — no nav at all. Same recipe + `useBackHandler` +
   `captureFocusReturn` for modal scope.
3. **Carousels** (Home/Play Now rails) — horizontal `useFocusGroup` per rail.
4. **GameDetailPanel panel-mode action buttons** — group PLAY/MORE as a RIGHT
   neighbour.

## Standard recipe (how panes opt in)

- Form/list pane: `useDomQueryFocusGroup({ id, containerRef, selector,
  orientation, onActivate, onCancel })`; for modal scope add
  `captureFocusReturn()` + `useBackHandler()`.
- Carousel: `useFocusGroup({ id, orientation: "horizontal", itemCount,
  focusedIndex, setFocusedIndex, onActivate, onCancel })`.

## Note

This is **Layer 2** (nav logic / coverage), distinct from this arc's **Layer 1**
(mapping). The gaps above are not controller-identity bugs — they're nav-wiring
work that could be its own slice/arc once mapping is proven. Filed here because
the audit surfaced during the controller-identity work.
