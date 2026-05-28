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

## `library-default-mockup.png`

Operator-supplied design reference for the **library view** in
the default OA theme (2026-05-28). Companion to
`default-theme-mockup.png` — that mock shows the HOME / system-
hero view (hover or select a system in the left sidebar), this
one shows what happens once the operator opens **LIBRARY** from
the top toolbar.

### Layout shape

- **Top toolbar** — same shape as `default-theme-mockup.png`
  (HOME / LIBRARY / COLLECTIONS / PLAY NOW / DISCOVER /
  SETTINGS, search input, clock, profile chip). **LIBRARY** is
  the active tab in this mock.
- **Left sidebar — SYSTEMS filter list.** Cover icon + system
  name + game count per row. Top row "ALL SYSTEMS · 3,074" is
  the active filter; per-system rows below act as filters. A
  `+ More Systems` expander suggests the list is paged.
  **COLLECTIONS** section below the systems list — Favorites,
  Recently Played, Completed, Hidden Gems, Multi-Player. Acts
  like a Steam category list rather than tiered nav.
- **Center — ALL GAMES grid.** Section header with title +
  current-filter game count + Sort / View / Filters controls
  (sort key, grid-density toggle, filter popover). Cover tiles
  carry a mini system-label header row (system glyph + name,
  e.g. "PlayStation.2"), star rating, and a heart favorite
  toggle. Pagination strip at the bottom (page 1 of 62 with
  ellipsis).
- **Right sidebar — focused game detail.** Cover hero (full
  bleed), title + subtitle, star rating + review count, genre
  chip, metadata chip strip (Developer / Publisher / Release /
  Players / Time Played), short description, horizontal
  screenshot carousel, **YOUR PROGRESS** block (Achievements
  count + % + Last Played), prominent **PLAY GAME** button +
  **MORE** secondary action.
- **Footer hint bar** — gamepad glyphs for SELECT / BACK /
  SEARCH / FILTERS / VIEW (density toggle) / CHANGE SYSTEM /
  FAVORITE. Matches the existing controller-nav HintBar
  primitive.

### Interaction model

- **Click LIBRARY in the top toolbar** → this view renders for
  whatever system filter is active (defaults to ALL SYSTEMS).
- **Click a system in the left sidebar** → the grid re-filters
  to that system; the focused-game detail panel updates when
  the user selects a tile.
- **VIEW toggle in the footer** (mapped to a stick / shoulder
  in the mock) → cycles between Grid / Large Grid / List view
  densities. The icon row near the Filters button mirrors this.
- **CHANGE SYSTEM (RS)** → quick-swap focus to the next system
  in the sidebar without leaving LIBRARY.

### Relationship to existing implementation

Same caveats as `default-theme-mockup.png` — this is more
ambitious than today's library. Notable deltas:

- **Cover tile system-label header** — today's `LibraryTile`
  shows star + favorite + flag chips, but no system-glyph
  header row. New addition.
- **Right-side focused-game detail panel** — today the focused
  game opens a modal (`GameInfoModal`); this mock keeps it
  always-visible as a persistent third pane. Conceptually
  closer to Steam's library layout.
- **System Status / Quick Launch strips intentionally absent**
  here — they live on HOME (`default-theme-mockup.png`) but
  not on LIBRARY. Reinforces that the two views serve different
  jobs: HOME is browse-the-shelf, LIBRARY is pick-a-title-and-
  play.
- **Pagination strip vs current virtualized grid** — the
  existing `VirtualLibraryGrid` is virtualized, not paged.
  Whether to keep virtualization or switch to pagination is its
  own decision (likely keep virtualization; treat any paginator
  as a visual element for very large collections only).

Mapping these into shippable slices is its own planning pass.
For now this image anchors the LIBRARY-view target alongside
`default-theme-mockup.png`'s HOME-view target.
