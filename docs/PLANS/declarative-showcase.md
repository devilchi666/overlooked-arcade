# Declarative Showcase — make the file-theme path capable & beautiful

**Planned 2026-06-18. Part of Theming ARC 3 (Cinematic). Branch:
`feat/motion-selection-ambient-hook` (broadened from the selection/ambient hook).**

## Why

OA has two kinds of theme:

- **Code themes** (`retroverse`, `coverflow`, `bare`, the **Graphics Lab**) — bespoke
  TSX. The Lab is our cinematic showcase, but it proves what the **engine** can do
  *via code*. A distributable file theme **can't ship code** (PD1), so the Lab proves
  nothing about what a theme *author* can achieve.
- **Declarative / file themes** (`neon-list` on disk; `bare-declarative` the built-in
  twin) — pure data (`theme.toml` + `tokens.toml` + `per-system.toml`) rendered by the
  one built-in `DeclarativeShell`. **This is the only path a distributable `.oatheme`
  uses.**

So the honest proof that "loadable file themes can use everything correctly" must be a
showcase built on the **declarative path** — not the code Lab. That's this arc: grow
the `DeclarativeShell` renderer + a flagship on-disk showcase theme (**Aurora**) that
exercises the full vocabulary, so file themes can be beautiful.

The constraint that makes this real: every time the showcase wants to look better, the
fix is in `DeclarativeShell` (data-driven), which *unlocks that capability for every
file theme* — not a one-off in a code theme.

## Honest ceiling

`DeclarativeShell` is a **single-surface flat browse** (list/grid/carousel/wheel +
background). It cannot express Retroverse's multi-tab / detail-panel structure — that
stays compiled-in or waits for ARC 3 scripting. "Pretty" here means making the browse
surface itself rich (cover art, motion, per-system theming, background), not adding new
surface structure.

## Vehicle

- **`bare-declarative` + `neon-list`** stay the **minimal floor** examples (mirror
  `bare`).
- **`aurora`** (`themes/community/aurora/`) is the new **on-disk showcase** — the
  declarative counterpart to the Graphics Lab. Being on disk, it also exercises the
  real file-load path end to end.

## Slices

- **S1 — cover art + Aurora + motion ✅ (this branch).** `DeclarativeShell` renders
  box-art covers on cards (was text-only) with a system-tinted fallback + a legibility
  scrim + a focus glow. New `aurora` theme: carousel layout, premium dark palette, vivid
  per-system accents, glyph set, full motion (slide transition + lift selection + breathe
  ambient). The selection/ambient hook (built earlier on this branch) is exercised here
  on box-art cards (where scale presets belong). `bare-declarative` reverted to its
  minimal floor.
- **S2 — background + now-focused detail (next).** A theme-supplied background through
  `ThemeBackground` (needs the disk-asset `basePath` → `convertFileSrc` plumbing — the
  S5.1 cascade already resolves theme/system asset bases); optionally a focused-game
  detail strip (title/metadata/logo) so the browse reads less like a bare grid.
- **S3 — list/row polish + metadata.** Cover thumbnails + metadata chips
  (year/genre/players) on list rows; richer settings_schema vocabulary the
  `DeclarativeShell` recognizes (density, card size, reflections…).
- **S4 — fit & finish.** Whatever the playtests of S1–S3 surface; promote keepers into
  the recognized declarative vocabulary so every file theme inherits them.

Slices are re-scoped after each playtest — the point is to keep pulling the built
subsystems (motion · per-system palettes · glyph sets · layouts · media · background ·
settings) together on the declarative path until a file theme can stand next to the
code Lab.

## Acceptance

Select **Aurora** in Settings → Appearance (`cargo tauri build`): a coverflow of real
box art, each focused cover lifting + breathing with a system-coloured glow, the whole
shell re-tinting per system — all from data, no theme code. That's the proof.
