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

## S3 kickoff — start here

**The chosen next slice (operator, 2026-06-18).** S1 + S2 are MERGED to main; this is
the self-contained brief — a fresh session can start from this section alone.

**Startup:** normal session start (CLAUDE.md → docs/INDEX.md → docs/ACTIVE_WORK.md "In
flight"), then load: this plan; SESSION_LOG top entries (S2a/S2b ✅ MERGED, incl. the
metadata-key fix + BigBox polish); DECISIONS **D59** (low-floor-now / high-ceiling-
reserved — the governing principle S3 must follow).

**Code to load:**
- `frontend/src/platform/theme/declarativeShell.tsx` — `RECOGNIZED_SETTINGS` (today
  recognizes ONLY `compactRows`), `renderRow` (LIST layout, currently text-only:
  accent dot + title + system short), `renderCard` (cards already have cover art), the
  `games` memo, `coverFor`/`logoFor`/`focusedMeta`, the S2b detail composition
  (`elementContent`/`renderDetail`/`CHIP_KINDS`).
- `frontend/src/platform/theme/themeSettings.ts` — `useThemeSettings().get<T>(key, default)`.
- `frontend/src/platform/theme/manifest.ts` — `ThemeSettingsSchema`/`ThemeSettingControl`,
  `ELEMENT_KINDS`. `frontend/src/platform/theme/validate.ts` — settings_schema validation.
- `frontend/src/platform/library/media.tsx` — ⚠️ cover/logo ART is identity-keyed
  (`coverUrl(systemId, identityId ?? id, …)`); per-game METADATA is rom-id-keyed
  (`media.media(entry.id)?.metadata`). Use `entry.id` for metadata; don't unify.
- Dogfood themes: `themes/community/neon-list/theme.toml` (LIST — the natural vehicle for
  row thumbnails), `themes/community/aurora/theme.toml` (carousel),
  `frontend/src/themes/declarative-bare/index.ts` (minimal list floor — keep minimal).
- The engine Appearance panel already renders `settings_schema` generically + persists
  per theme (grep `engine/`); S3 adds RECOGNIZED keys the shell ACTS on, not new panel
  plumbing.

**Task — three parts, all DATA-driven on `DeclarativeShell` (no per-theme code), each
built D59-style (recognized/wired now; declared-but-unknown persists + stays inert):**
1. **List-row thumbnails** — `renderRow` gains a small leading cover thumbnail (reuse
   `coverFor`; text fallback when no art).
2. **Row metadata** — compact metadata on rows via `media.media(entry.id)?.metadata`
   (rom-id key!).
3. **Richer recognized settings vocabulary** — turn `RECOGNIZED_SETTINGS` from the single
   `compactRows` into a curated, documented contract of keys the shell honors (candidates:
   row thumbnails on/off, row metadata on/off, density, card/tile size). Other declared
   `settings_schema` keys keep persisting + rendering in Appearance but stay inert.

**Design first (operator workflow):** discuss + push back in PROSE, settle the shape,
only THEN fire AskUserQuestion if a real fork remains — don't jump to code. Forks to
settle: the recognized-settings contract (which keys / types / defaults / how
"recognized vs declared-inert" is expressed — a typed registry?); row anatomy (thumbnail
size/aspect/position; which metadata, how compact; gated by recognized toggles?); any
high-ceiling stub needed per D59; dogfood (neon-list authors thumbnails+metadata on;
bare-declarative stays minimal).

**Landmines / conventions:**
- Metadata = rom id; art = identity id (see media.tsx note).
- DON'T run `cargo fmt -p oa-shell` — it reformats the whole crate (~8000-line churn
  once). Match style by hand or fmt only touched files.
- Operator playtests with `cargo tauri build` → `target/release/oa-shell.exe`. A stale
  `<exe_dir>/themes/community/` SHADOWS the repo themes (first-match-wins in
  `theme_loader`) — if a theme/asset doesn't show, delete the stale exe-dir copy before
  debugging.
- MOTION.md rules #3 (scale↔cards / y·opacity·glow↔rows) + #4 (clipping ancestor) if
  motion is added; reduced-motion is the players' floor (D58.6).

**Verify per step:** `cd frontend && ./node_modules/.bin/tsc --noEmit && npm run lint &&
./node_modules/.bin/vitest run src/platform/theme src/themes`. Rust only if a NEW
manifest field needs round-tripping (settings_schema + `detail` already round-trip — a
curated recognized-key set is frontend-only); if so `cargo test -p oa-shell theme_loader`.

**Branch/merge:** one branch (e.g. `feat/declarative-showcase-s3`); commit each coherent
step; operator playtests on `neon-list` (Settings → Appearance) + Aurora; flip
SESSION_LOG/ACTIVE_WORK/NEXT/this-plan statuses + `--no-ff` merge at a playtestable
milestone (same as S1/S2).

**NOT S3 (deferred):** the free-form **canvas** (honor element `position`/`size` + a
layout engine + Theme Studio = ARC 4 — contract + stubs already in place per D59, so
it's additive); `push-hero`/attract (Thrust V); the per-element `ambient` slot.
