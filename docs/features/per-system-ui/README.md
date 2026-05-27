# Per-System Custom UI — Feature

Each system in OA's library carries its own mini-experience — audio,
boot animation, tile flourishes, background, eventually navigation
shape + in-game overlay theming. Default OA experience (not opt-in);
a Settings → Display master toggle ships a uniform plain library for
operators who want it.

Stages are sequential, each shippable on its own:

| Stage | Focus | Estimate |
| --- | --- | --- |
| 1 | Polish layer — `SystemUIConfig` registry, per-system SFX + boot animations + tile flourishes + backgrounds, 3 full pilots (GB → NES → Vectrex), baseline config for the rest | ~5-7 weeks |
| 2 | Behavior layer — per-system layout (carousel / list / wheel), interaction feel, tile emphasis, 5-10 more showcase systems | ~4-6 weeks |
| 3 | Experience layer — per-system in-game overlays, library ↔ game transitions, per-system metadata priorities, all ~40 systems tuned | ~6-10 weeks |

**Source of truth:** [docs/PLANS/per-system-ui.md](../../PLANS/per-system-ui.md)
holds the locked design (modes, architecture, audio sourcing strategy,
pilot specs, open questions). This folder records what's actually
implemented vs what's still on paper.

## Sibling docs

- [ROADMAP.md](ROADMAP.md) — slice-by-slice surface for the active stage
- [SESSION_LOG.md](SESSION_LOG.md) — Shipped / Almost / Next per session
- [DECISIONS.md](DECISIONS.md) — implementation decisions made during
  the build (separate from the plan's strategic decisions)
- [ASSETS.md](ASSETS.md) — operator-facing catalog of where every
  sound / image / shader file goes on disk, with per-event filenames,
  per-pilot specs, file-size targets, and CC0 sourcing guidance

## Asset bundle layout (quick reference)

Per-system assets ship at `<exe_dir>/assets/system-ui/<systemId>/`
with a `_baseline/` universal fallback alongside. Full path catalog,
per-event filename map, and per-pilot specs live in
[ASSETS.md](ASSETS.md); the high-level shape is:

```
<exe_dir>/assets/system-ui/
├── _baseline/<sounds|backgrounds|boot-animation>/   (universal fallback)
└── <systemId>/<sounds|backgrounds|boot-animation>/  (per-system bank)
```

Resolver cascade: operator override → per-system bundle →
`_baseline` → built-in default (e.g. accent-color gradient for
backgrounds) → silence / no-op. Slice 2 ships the sounds cascade;
backgrounds + boot-animation cascades land in Slices 3 + 4.

## Cross-arc relationships

- **Kiosk shell** — eventually becomes the theme editor for power
  users that consumes these built-in per-system experiences as
  starting defaults. See plan §3 for the relationship.
- **Guided setup** — the controller-nav primitives + on-screen hint
  bar that Per-System UI relies on shipped under
  [features/controller-nav/](../controller-nav/) (Phase 0 + completion
  pass + v2 polish, all merged to main 2026-05-26). Stage 1 builds
  directly on top of those primitives.
- **Media taxonomy** — the 4-bus audio mixer (shipped 2026-05-24)
  carries per-system SFX on the `ui-sounds` bus. No new audio
  infrastructure needed for Stage 1.
