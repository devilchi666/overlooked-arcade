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

## Asset bundle layout

Per-system assets ship at `<exe_dir>/assets/system-ui/<systemId>/`.
Slice 2 added the bundled-asset tier to the existing per-system
`SystemSettings.ui_sound_<event>` operator-override path; the
resolver cascades:

```
1. Operator override — SystemSettings.ui_sound_<event> (any absolute path)
2. Per-system bundle — <exe_dir>/assets/system-ui/<systemId>/sounds/<event>.<ext>
3. Universal baseline — <exe_dir>/assets/system-ui/_baseline/sounds/<event>.<ext>
4. Silence
```

Supported extensions in priority order: `ogg`, `opus`, `wav`, `mp3`,
`flac`, `m4a` (matches rodio's `symphonia-all` decoder set).

Full asset directory shape (planned across stage 1; ships per slice):

```
<exe_dir>/assets/system-ui/<systemId>/
  ├─ sounds/
  │   ├─ navigate.ogg     (cursor tile-to-tile, Slice 2)
  │   ├─ click.ogg        (tile picked / confirm, Slice 2)
  │   ├─ back.ogg         (cancel, Slice 2)
  │   ├─ launch.ogg       (game starts loading, Slice 2)
  │   ├─ boot-intro.ogg   (boot-animation accompaniment, Slice 4)
  │   └─ boot-outro.ogg   (exit-system, Stage 3)
  ├─ backgrounds/
  │   ├─ default.png      (static, Slice 3)
  │   ├─ animated.webm    (animated, Slice 3)
  │   └─ shader.wgsl      (shader, Slice 3 + pilot 3)
  └─ boot-animation/
      ├─ keyframes.css    (CSS animation, Slice 4)
      └─ effects.wgsl     (optional shader-based, Slice 4)
```

`_baseline` is the universal fallback for systems without a
per-system asset bank. Stage 1 ships a single CC0 click pack at
`_baseline/sounds/` so every system has at least a soft click for
nav / select / back / launch.

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
