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
holds the locked design (the 4-arc structure, ARC 1's 6 phases,
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

Mirrors plan §5 (post-D35 renumber).

| Arc | Focus | Estimate | Status |
| --- | --- | --- | --- |
| 1 | Minimum Viable Substrate — layout + assets + palette, no scripting/shaders. Engine/theme separation + platform layer + Tauri hardening + Retroverse rebuilt as theme + 2nd pilot. | ~22-26 weeks | **complete** bar the `.oatheme` loader (folded into ARC 2's tail) |
| 2 | Per-System Layout Substrate — D32 per-system layout/view capability + D33 consumption opt-in + Per-System UI re-home + the `.oatheme` runtime loader. Declarative; no scripting/shaders. | TBD | planned |
| 3 | Cinematic & Scripting (was ARC 2) — declarative motion/transitions + `<video>`/attract + Rhai behaviors + WGSL shaders per KIOSK_PLAN §2.2. | ~7 weeks | future |
| 4 | Theme Studio (was ARC 3) — in-engine visual + code editor per KIOSK_PLAN §2.3. | ~5-7 weeks | future |

## Cross-arc relationships

Most of what this feature delivers depends on or feeds other arcs:

- **Virtual Library arc** — see plan §7 for sequencing. Phases 1-2
  run parallel with VL Phase A; ARC 1 pauses at end of Phase 2 for
  VL Phase E + C to land first; resumes for Phases 3-6.
- **Per-System UI** — per D34, per-system UI is now an **ARC-2
  substrate capability** themes opt into (a theme declares per-system
  layout/view defaults; the content of each per-system world is
  theme-owned), not a paused Retroverse-only side-stream. The
  platform/theme boundary already formalizes the registry + cascade
  patterns the old Stage 1 prototyped; ARC 2 re-homes the rest.
- **Kiosk Shell** — KIOSK_PLAN.md §2.2-2.5 is the source spec for
  ARCs 3-4. Kiosk-mode capabilities (attract mode, multi-monitor,
  5-bus mixer) become substrate features themes opt into; the
  Kiosk plan's standalone existence is no longer needed.

## Status

**ARC 1 complete bar the `.oatheme` loader.** Phases 1-6 shipped +
merged (engine/theme separation, platform layer, Tauri-bridge
hardening, the skeleton-first substrate-depth slices S1-S5, the
nav-remap Settings UI, and Retroverse-as-theme — see ROADMAP.md +
SESSION_LOG.md). The only ARC-1 remainder is the original §6 Phase 5
`.oatheme` distribution/loader, which D35 folded into **ARC 2's tail**.
ARC 2 (Per-System Layout Substrate) is **planned**; ARCs 3-4
(Cinematic & Scripting / Theme Studio) are future.

DECISIONS G WAIT lock on the public theme ecosystem still holds —
ARC 1 ships operator-loaded themes only (drop a folder in
`<exe_dir>/themes/`). Public SDK + contribution funnel + gallery
deferred until OA hits user mass.
