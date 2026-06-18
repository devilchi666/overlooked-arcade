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

- **S1 — cover art + Aurora + motion ✅ MERGED to main 2026-06-18** (operator
  playtested — premium). `DeclarativeShell` renders box-art covers on cards (was
  text-only) with a system-tinted fallback + a legibility scrim + a focus glow. New
  `aurora` theme: carousel layout, premium dark palette, vivid per-system accents, glyph
  set, full motion (slide transition + lift selection + breathe ambient). The
  selection/ambient hook (`SelectionMotion`, built earlier on this branch) is exercised
  here on box-art cards (where scale presets belong). **BIOS files hidden** from the
  declarative browse (a title rule mirroring `title_parse` — `RomEntry` doesn't carry
  `is_bios`, and `casual_view_defaults` is unwired till VL Phase F). `bare-declarative`
  reverted to its minimal floor.
  - **Footgun found:** the disk loader prefers `<exe_dir>/themes` over the source-tree
    fallback, so a **stale `target/release/themes/`** silently shadowed the repo themes
    (Aurora invisible). Removed it (a one-off leftover; the build doesn't create it). If
    it recurs, harden the loader to merge both candidates (deduped) instead of first-wins.
- **S2a — self-contained theme asset packages ✅ (branch `feat/self-contained-theme-assets`,
  needs rebuild).** A disk theme pulls its assets from its OWN package dir first
  (operator's point 1): a **tier-0** `themes/<community|dev>/<id>/system-ui/…` was added
  to the asset cascade (above the bundled `<exe_dir>/assets/themes/<id>/` tier),
  `theme_loader::theme_package_dir` resolves it, the asset-protocol scope covers the
  themes dir, and `svg` joined `STATIC_EXTS` so a theme can ship a text-authored vector
  backdrop. Aurora ships `system-ui/_baseline/backgrounds/default.svg`.
- **S2b — author-controlled layout/motion: bounded element slots ✅ (branch
  `feat/self-contained-theme-assets`, needs rebuild).** The focused-game composition is
  author-declared (operator's point 2), designed as a **canvas subset** (chosen over
  jumping straight to the free-form canvas — DECISIONS D59). `ThemeElement`
  (`kind` + `motion`, RESERVED `position`/`size`/`ambient`) + `ThemeManifest.detail` +
  `ELEMENT_KINDS`; Rust loose `detail` pass-through; validator (`INVALID_DETAIL` /
  `UNKNOWN_ELEMENT_KIND` / `UNKNOWN_ELEMENT_MOTION`). `DeclarativeShell` renders an
  engine-arranged focused-detail overlay on carousel/wheel, bound to game data, keyed
  entrance motion. Aurora authors logo/system/title/year/genre/developer + a reserved
  `position` proving the canvas stub round-trips inert.
- **Free-form canvas ⬜ FUTURE (additive — Theme Studio / ARC 4).** The element
  descriptor + reserved `position`/`size` + the loose round-trip are now in place, so
  the canvas is: honor `position`, add a layout engine, add the Studio authoring UI. No
  contract rewrite. (Per D59 — the stub + contract shipped with the floor.)
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
