# Per-System UI — Reference assets

## `default-theme-mockup.png`

Operator-supplied design reference for the **default OA theme**
(2026-05-27). Shows the target visual + interaction model that
Per-System UI Stage 1+ should converge on:

### Layout shape

- **Left sidebar — system list.** Cover icon + system name + game
  count per row. Selected/hovered row gets an accent ring (PS2 in
  the mockup).
- **Center hero panel** — large system art (PS2 console rendered
  against a neon-city background), system name, star rating,
  short blurb, metadata chips (year / architecture / media), a
  progress bar ("Your progress 78%"), and a horizontal popular-
  games carousel of cover-art tiles.
- **Right sidebar — focused game detail.** Cover art, title,
  publisher / genre / release year, description, game info
  (players / last played / play time / region / size), a
  prominent **PLAY GAME** button.
- **Top toolbar** — HOME / COLLECTION / PLAY NOW / DISCOVER /
  SETTINGS, search input, clock, profile chip.
- **Bottom strip** — QUICK LAUNCH shortcuts (Random Game,
  Favorites, Most Played, Last Played, Multiplayer, Achievements)
  + RECENTLY PLAYED carousel + SYSTEM STATUS gauges (CPU / RAM /
  Storage).
- **Footer hint bar** — NAVIGATE / SELECT / BACK / SEARCH / RANDOM
  GAME / OPTIONS / FAVORITE gamepad glyphs + now-playing track.

### Interaction model

- **Hover a system in the left sidebar** → this rich detail view
  renders (system hero + popular games + focused game detail).
- **Click into a system** → library view styled the same way
  (system hero stays as a banner; the popular-games carousel
  expands into a fuller library grid).

### Relationship to existing Stage 1-5 plan

This design is more ambitious than the Stage 1 "polish layer"
plan currently locked at `docs/PLANS/per-system-ui.md`. Hitting
this mock requires:

- **Per-system hero art** — high-quality system imagery (likely
  AI-generated or curated stock). Asset bundle in
  `<exe_dir>/assets/system-ui/<id>/hero/`.
- **Per-system blurbs** — short marketing copy per system
  (~3 sentences). Data field in `systemThemes` or a sibling
  registry.
- **Per-game richer metadata** — players / play time / last
  played / region / size all surfaced together. Some fields
  already in the library DB; play-time tracking needs a new
  per-game counter.
- **Quick launch shortcuts + system status gauges** — new
  bottom-strip components. CPU/RAM/Storage gauges piggyback on
  the existing `sysinfo` crate.
- **Top toolbar IA refresh** — HOME / COLLECTION / PLAY NOW /
  DISCOVER / SETTINGS top-level mode picker. Distinct from the
  current sidebar-driven nav.
- **Hover-to-preview semantics** — sidebar hover currently does
  NOT render a system detail panel; needs new hover signal +
  panel component.

Effectively this is **Per-System UI Stage 1 + Stage 2 + Stage 3
combined**, plus several net-new features (top toolbar IA, system
status gauges, quick-launch surface, play-time tracking). Mapping
the mock into shippable slices is its own planning pass — see
the parent `ROADMAP.md` once that planning lands.

For now this reference image anchors what "done" looks like so
future slices can converge.
