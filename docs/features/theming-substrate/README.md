# Theming Substrate — Feature

OA gets BigBox-style theming where creators can ship radically
different looks (wheel layouts, tile grids, list views, cabinet
attract modes) on top of one shared engine. **One unified premium
frontend** — no LaunchBox/BigBox-style binary split.

The unlock is splitting OA's UI into two surfaces inside one
window:

- **Engine territory** (always engine-rendered, visually neutral):
  Settings, Library Manager, Import Wizard, BIOS pre-checks, Core
  installer, System Health, Background Jobs. Summoned from any
  theme via fullscreen takeover (hotkey `F12` / controller
  `Select+Start` / top-right corner icon themes reserve).
- **Theme territory** (where creators design): library browsing,
  game launch ceremony, now-playing, quick-settings overlay,
  discovery surfaces.

Retroverse becomes the first theme on the substrate — dogfood
test. If Retroverse can be a `.oatheme`, anything can.

## Source of truth

[docs/PLANS/theming-substrate.md](../../PLANS/theming-substrate.md)
holds the locked design (the 3-arc structure, ARC 1's 6 phases,
manifest schema, sequencing relative to the Virtual Library arc,
risks). This folder records what's actually implemented vs what's
still on paper.

## Sibling docs

- [ROADMAP.md](ROADMAP.md) — phase status board (ARC 1)
- [SESSION_LOG.md](SESSION_LOG.md) — Shipped / Almost / Next per
  session
- [DECISIONS.md](DECISIONS.md) — implementation decisions made
  during the build (separate from the plan's strategic
  decisions)
- [SURFACES.md](SURFACES.md) — engine-vs-theme territory map.
  Written in Phase 1 of ARC 1; placeholder until then.

## Arc structure (quick reference)

| Arc | Focus | Estimate | Status |
| --- | --- | --- | --- |
| 1 | Minimum Viable Substrate — layout + assets + palette, no scripting/shaders. Engine/theme separation + platform layer + Tauri hardening + `.oatheme` distribution + Retroverse rebuilt as theme + 2nd pilot. | ~22-26 weeks | queued |
| 2 | Behaviors + Shaders (Rhai + WGSL per KIOSK_PLAN §2.2). | ~7 weeks | future |
| 3 | Theme Studio (in-engine visual + code editor per KIOSK_PLAN §2.3). | ~5-7 weeks | future |

## Cross-arc relationships

Most of what this feature delivers depends on or feeds other arcs:

- **Virtual Library arc** — see plan §7 for sequencing. Phases 1-2
  run parallel with VL Phase A; ARC 1 pauses at end of Phase 2 for
  VL Phase E + C to land first; resumes for Phases 3-6.
- **Per-System Custom UI Stage 2+3** — currently paused for
  content. Easier to ship after this arc lands because the
  platform/theme boundary formalizes the registry + cascade
  patterns Stage 1 prototyped.
- **Kiosk Shell** — KIOSK_PLAN.md §2.2-2.5 is the source spec for
  ARCs 2-3. Kiosk-mode capabilities (attract mode, multi-monitor,
  5-bus mixer) become substrate features themes opt into; the
  Kiosk plan's standalone existence is no longer needed.

## Status

**Queued.** No code yet. Planning conversation 2026-06-06; full
plan written. Phase 1 (engine/theme surface separation) is the
first slice when the arc starts.

DECISIONS G WAIT lock on the public theme ecosystem still holds —
ARC 1 ships operator-loaded themes only (drop a folder in
`<exe_dir>/themes/`). Public SDK + contribution funnel + gallery
deferred until OA hits user mass.
