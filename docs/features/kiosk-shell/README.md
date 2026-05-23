# Kiosk shell — full-screen cabinet mode

Cross-cutting work stream for the full-screen "BigBox-class" presentation
mode: theming substrate (4-layer + Rhai), in-engine Theme Studio, attract
mode (3 tiers), 5-bus audio mixer, multi-monitor surfaces, launch ceremony,
in-game menu, configurable controller bindings, named views with arbitrary
hierarchies, kid mode, accessibility floor, federated theme distribution.

**Status as of 2026-05-22:** 📐 Design-only. Phase 0 (desktop polish
prereq) = `UI_POLISH_PLAN.md` ✅ shipped. Phase 1+ kiosk shell itself
(`--kiosk` flag, mode switch, theme substrate, attract, in-game menu) has
not begun.

## Files in this folder (after Step B of the 2026-05-22 reorg)

- `KIOSK_PLAN.md` — design spec for the full kiosk feature set
  (📐 design-only). Phase 0–7 implementation plan inside.
- `SESSION_LOG.md` — created when implementation work begins.

## Why this lives under features/ instead of cores/

Kiosk shell is the inverse of per-core work — it's a system-agnostic
presentation tier that wraps every core uniformly.

## Related

- Sidebar ([../sidebar/](../sidebar/)) — kiosk Phase 1+ consumes the views
  data model the sidebar work landed.
- UI polish ([../ui-polish/](../ui-polish/)) — Phase 0 of this kiosk plan.
- Parking lot entry — `docs/PARKING_LOT.md` 2026-05-22 entry that lives
  alongside this plan.
