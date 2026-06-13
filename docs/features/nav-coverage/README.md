# Controller-Nav Coverage

**Status:** Queued 2026-06-13 (operator's chosen next stream). A cross-cutting
nav-wiring sweep — bring controller navigation to the engine-surface screens
that the audit found keyboard/mouse-only.

## Why this exists

The 2026-06-12 nav-coverage audit (run during the controller-identity arc)
found that the five Retroverse tabs (Home / Library / Collections / Play Now /
Discover) + modals/menus already have **solid, consistent** controller nav via
the 3-region focus-group pattern. The gap is the **engine surface**: Settings
category bodies, the metadata editor, and engine dialogs are form-heavy and
**keyboard/mouse-only** — no focus-group row navigation.

This is **Layer 2** (nav wiring) — distinct from the controller-identity arc,
which was **Layer 1** (physical-pad identity + button mapping, shipped 2026-06-13).
A controller now maps correctly; this stream makes more *screens* respond to it.

## The audit (source of truth)

Full coverage table + per-pane evidence + the gap analysis:
[../controller-identity/NAV_COVERAGE_AUDIT_2026-06-12.md](../controller-identity/NAV_COVERAGE_AUDIT_2026-06-12.md).
**Re-verify before acting** — anchors are approximate and the code moves.

## Prioritized gaps (highest traffic first)

1. **Settings category bodies** — `frontend/src/engine/SettingsSections.tsx`,
   `PerSystemSettingsBody.tsx`, `MetadataSettingsBody.tsx`. The category sidebar
   is navigable (`SettingsPanel.tsx` `useDomQueryFocusGroup`), but the
   per-category form bodies aren't. Highest traffic (operators live here).
2. **Engine dialogs / complex forms** — import wizard steps, per-game
   properties, Debug/Help dialogs. No nav at all; need modal-scoped focus
   groups.
3. **Carousels** — Home + Play Now rails: arrows are mouse-only, no focus group.
4. **GameDetailPanel (panel mode)** — read-only; PLAY / MORE buttons ungrouped.

## Standard recipe (how panes opt in)

Form / list pane:
```ts
import { useDomQueryFocusGroup, useBackHandler, captureFocusReturn } from "@oa/platform/nav";
const restore = captureFocusReturn();        // modal scope only
useBackHandler(() => props.onClose());        // modal scope only
useDomQueryFocusGroup({
  id: "my-section",
  containerRef: () => el,
  selector: "[data-setting-row]",            // or "button", ".row", …
  orientation: "vertical",
  onActivate: (i, node) => node.click(),
  onCancel: () => props.onClose(),
});
onCleanup(restore);
```
Carousel: horizontal `useFocusGroup({ id, orientation: "horizontal", itemCount,
focusedIndex, setFocusedIndex, onActivate, onCancel })`.

## Approach

Incremental — **one surface per slice**, highest-traffic first (Settings
bodies). Each slice: wire the focus group(s), **verify with the controller test
window (Settings → Controllers) + an actual pad**, keep the nav test suite
green. Mirror the working patterns the audit documents (3-region pages, the
modal inert-takeover in `Dialog.tsx`, context-menu vertical lists).

## Out of scope

- The already-covered Retroverse tabs + modals (don't re-wire).
- Controller identity / button mapping / label families (shipped arc).
- Glyph icons, reduced-layout schemes, arcade encoders (parked — see
  PARKING_LOT.md).
