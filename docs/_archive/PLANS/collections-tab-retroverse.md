# COLLECTIONS tab — Retroverse default theme

**Status:** Design sketch. No code.

**Author:** Operator + Claude (LLM pair).

**Owner-of-decisions:** the operator. This document records what was sketched.

**Reference:** [features/per-system-ui/assets/default-theme-mockup.png](../features/per-system-ui/assets/default-theme-mockup.png) (HOME) +
[features/per-system-ui/assets/library-default-mockup.png](../features/per-system-ui/assets/library-default-mockup.png)
(LIBRARY) + [settings-tab-retroverse.md](settings-tab-retroverse.md) +
[play-now-tab-retroverse.md](play-now-tab-retroverse.md) +
[discover-tab-retroverse.md](discover-tab-retroverse.md) +
[content-packs.md](content-packs.md). This doc is the COLLECTIONS
companion.

---

## 1. TL;DR

COLLECTIONS in the LIBRARY mockup's sidebar was a *quick filter*
(Favorites, Recently Played, Completed, etc.). As its own top-
toolbar tab it gets dedicated curatorial chops: **browse +
manage + create + edit** lists, with editorial pack support.

Different from the other tabs:

- **LIBRARY** = browse the whole catalog
- **HOME** = browse by system
- **PLAY NOW** = algorithmic recommendation
- **DISCOVER** = external editorial content
- **COLLECTIONS = your own curated lists, plus curated packs**

Same three-pane shell. Sidebar holds three groups (MY
COLLECTIONS / SMART LISTS / CURATED); center is a header card +
tile grid; right is the LIBRARY focused-game detail with a new
collection-membership block.

---

## 2. Layout shape

```
┌───────────────────────────────────────────────────────────────────────────────────┐
│ ◤ RETROVERSE      HOME  LIBRARY  ▣COLLECTIONS▣  PLAY NOW  DISCOVER  SETTINGS  ⌕ │
│   EMULATION FRONTEND                                              09:47 PM · 👤  │
├───────────────────────────────────────────────────────────────────────────────────┤
│ ┌─ COLLECTIONS ────┐ ┌─ ❤ FAVORITES — 128 games ────────┐ ┌─ FOCUS ────────────┐ │
│ │                  │ │                                  │ │  BURNOUT 3         │ │
│ │ MY COLLECTIONS   │ │ ┌─ HEADER ─────────────────────┐ │ │  Takedown          │ │
│ │ 📚 Weekend       │ │ │ ❤ FAVORITES                  │ │ │ ┌──────────────┐   │ │
│ │   rotation  · 14 │ │ │ 128 games · auto-updated      │ │ │ │ [cover art]  │   │ │
│ │ 🎮 Mid-playthru  │ │ │ Total time 462h · Cleared 38  │ │ │ │              │   │ │
│ │   rotation  · 7  │ │ │ Built-in · read-only          │ │ │ └──────────────┘   │ │
│ │ 🌃 Late-night    │ │ │                              │ │ │                    │ │
│ │   shmups   · 23  │ │ │ Games you've ♥-tagged. Toggle │ │ │ ★★★★★ 4.6 · 512   │ │
│ │ + New collection │ │ │ heart on any tile to update.  │ │ │                    │ │
│ │                  │ │ │                              │ │ │ Criterion · EA     │ │
│ │ SMART LISTS      │ │ │ Sort [Recently added ▾]       │ │ │ Sep 7 2004 · 1–2P  │ │
│ │ ❤ Favorites  128 │ │ │ View [ ▦  ▣  ☰ ]             │ │ │ Played: 8h 12m     │ │
│ │ 🕘 Recent     42 │ │ └──────────────────────────────┘ │ │ ♥ In Favorites     │ │
│ │ ✓ Completed   37 │ │                                  │ │ 📚 In: Weekend     │ │
│ │ 💎 Hidden    88 │ │ ┌──┐┌──┐┌──┐┌──┐┌──┐┌──┐┌──┐    │ │   rotation         │ │
│ │ 👥 Multi-P  213 │ │ │  ││  ││  ││  ││  ││  ││  │    │ │                    │ │
│ │ 🏁 Last       42│ │ │  ││  ││  ││  ││  ││  ││  │    │ │ The ultimate high- │ │
│ │                  │ │ └──┘└──┘└──┘└──┘└──┘└──┘└──┘    │ │ speed driver — big │ │
│ │ CURATED          │ │  PS2  GC  Wii GBA NES SNES PS1  │ │ crashes, takedowns,│ │
│ │ ⓘ No curated     │ │                                  │ │ multi-route mayhem.│ │
│ │   collections    │ │ ┌──┐┌──┐┌──┐┌──┐┌──┐┌──┐┌──┐    │ │                    │ │
│ │   installed.     │ │ │  ││  ││  ││  ││  ││  ││  │    │ │ ─ SCREENSHOTS ─    │ │
│ │ [▶ Browse packs] │ │ │  ││  ││  ││  ││  ││  ││  │    │ │ ┌────┐┌────┐┌────┐ │ │
│ │                  │ │ └──┘└──┘└──┘└──┘└──┘└──┘└──┘    │ │ │ ▓▓ ││ ▓▓ ││ ▓▓ │ │ │
│ │                  │ │  N64 PSP DC  GG  TG-16 SMS MD   │ │ └────┘└────┘└────┘ │ │
│ │                  │ │                                  │ │                    │ │
│ │                  │ │ ┌──┐┌──┐┌──┐┌──┐┌──┐┌──┐┌──┐    │ │ YOUR PROGRESS      │ │
│ │                  │ │ │  ││  ││  ││  ││  ││  ││  │    │ │ Achievements 12/38 │ │
│ │                  │ │ │  ││  ││  ││  ││  ││  ││  │    │ │ Last played May 18 │ │
│ │                  │ │ └──┘└──┘└──┘└──┘└──┘└──┘└──┘    │ │                    │ │
│ │                  │ │ … 107 more ▾                     │ │ [▶ PLAY] [⋯ MORE]  │ │
│ └──────────────────┘ └──────────────────────────────────┘ └────────────────────┘ │
├───────────────────────────────────────────────────────────────────────────────────┤
│ Ⓐ PLAY  Ⓑ BACK  Ⓧ SEARCH IN LIST  Ⓨ REMOVE FROM LIST  L1/R1 LIST   RS GROUP    │
└───────────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Sidebar — three groups

### MY COLLECTIONS

Operator-created custom lists. Reorderable (DPad + Ⓨ to grab and
move). Each row shows the collection name + item count.

`+ New collection` row at the bottom opens a new-collection
dialog: name, optional description, and a **Manual / Smart
query** picker.

### SMART LISTS

Built-in dynamic collections, always present:

- **Favorites** — heart-toggled games (auto-updates as the
  operator hearts/unhearts elsewhere)
- **Recently Played** — rolling 30-day window
- **Completed** — manually marked-complete games
- **Hidden Gems** — high-rated games with low play time
- **Multi-Player** — games tagged 2P+
- **Last Played** — chronological play-order surface

Read-only — operator can't delete or rename these. Update
automatically.

### CURATED

Read-only collections supplied by editorial-style packs of type
`collections` (see [content-packs.md](content-packs.md) for the
pack mechanism).

Empty by default. The empty-state row uses the same shape as
DISCOVER's empty states: a friendly explanation + a
`▶ Browse packs` CTA pointing at SETTINGS → Content → Packs.

---

## 4. Center pane — header + tile grid

### Header card

Sits above the grid carrying everything *about* the collection:

- Title + count + flavor badge (`Built-in · read-only` /
  `Custom · manual` / `Custom · smart query` / `Curated · from <pack>`)
- Stats: total play time across the list, total cleared,
  average rating
- Description (editable for custom; readonly otherwise)
- Sort selector + view toggle (Grid / Large grid / List)
- Edit affordances appear only for custom lists (rename, change
  cover, grab-to-reorder)

### Tile grid

Same shape as LIBRARY's grid — mini system-label header, star,
favorite icon. Stays consistent so operators don't relearn.

Pagination at the bottom if the list is large (matching LIBRARY's
existing pattern).

---

## 5. Right pane — focused-game detail

Same shape as LIBRARY's right pane, with one addition: a
**collection-membership block** below the cover:

```
♥ In Favorites
📚 In: Weekend rotation
📚 In: Late-night shmups
```

Operator sees at a glance which lists this game is part of and
can DPad over to manage. Click any badge → jumps to that
collection.

---

## 6. Footer hint bar

- `Ⓐ PLAY` — launches focused game
- `Ⓑ BACK`
- `Ⓧ SEARCH IN LIST` — search within active collection (distinct
  from global LIBRARY search)
- `Ⓨ REMOVE FROM LIST` — toggle membership for custom lists;
  disabled with a tooltip on smart / curated
- `L1 / R1 LIST` — cycle to prev/next collection in sidebar
- `RS GROUP` — jump between MY / SMART / CURATED groups

---

## 7. Three flavors of "collection"

| Flavor | Editable? | Auto-update? | Examples |
|---|---|---|---|
| **Smart built-in** | No | Yes | Favorites, Recently Played |
| **Custom — manual** | Yes (drag games in/out) | No | "Weekend rotation," "Mid-playthrough" |
| **Custom — smart query** | Yes (edit rules) | Yes | "All PS1 RPGs released < 1998" |
| **Curated (from pack)** | No (but "Copy to mine" supported) | No | "Top 50 SNES RPGs" |

The smart-query variant is the bridge between fully manual lists
and built-in smart lists. Operator defines rules like:

```
system IN [snes, gbc] AND year < 1995 AND tags CONTAINS rpg
```

OA computes membership live. Anything expressible as a LIBRARY
filter can be saved as a smart collection.

---

## 8. Editing a custom collection

- **Rename / re-describe** — header click
- **Cover art** — default auto-generated 2×2 mosaic of first 4
  covers; "Pick game art" override picks any one member's
  cover; "Custom image" lets operator drop in a hero
- **Add games** — `+ ADD GAMES` button opens an inline LIBRARY
  mini-grid (search + filter + multi-select with Ⓐ) without
  leaving the collection
- **Remove** — Ⓨ on a tile, or right-side detail's
  `MORE → Remove from Weekend rotation`
- **Reorder (manual lists)** — Ⓨ grabs the focused tile, DPad
  moves it, Ⓐ drops it
- **Sort mode** — Manual order / Title / Play time / Date added
  / Rating

For smart-query lists, the edit affordance is a **Rules** button
that opens a query builder (same predicates the LIBRARY filter
popover already uses, just persisted).

---

## 9. Empty states

- **No custom collections** — MY COLLECTIONS shows just the
  `+ New collection` row + a friendly hint card in the center
  pane: *"Build your first list — try 'Weekend rotation' or
  'Want to play next.'"*
- **No curated pack installed** — CURATED group shows the
  explanatory shape from DISCOVER, pointing at the pack
  browser:
  ```
  ⓘ No curated collections installed.
  Curated collection packs add operator-curated lists like
  "Top 50 SNES RPGs" or "Hidden gems of the Saturn library."
  [ ▶ Browse available packs ]
  ```

Smart Lists never go empty — they're always present even if 0
games match (Favorites shows `0 games · heart any tile to add`).

---

## 10. Notable deltas vs LIBRARY / PLAY NOW

- **Header card on every list.** LIBRARY has a section title;
  COLLECTIONS has a full info card with stats, sort, view,
  edit affordances. **The list is the subject**, not just the
  filter applied.
- **Curatorial Ⓨ.** In LIBRARY, Ⓨ is "favorite"; here it's
  "remove from current list." Mapping shifts because the verb
  shifts.
- **Cross-collection visibility in right pane.** Membership
  badges tell you a game lives in N lists — a thing LIBRARY
  hides because LIBRARY is a single grid.
- **Smart-query as first-class.** PLAY NOW's recommendation
  engine is invisible; here, the operator can hand-tune the
  same kind of query logic and save it as a reusable
  collection.

---

## 11. Pack-shaped curated collections

New pack type `collections` lands in the
[content-packs.md](content-packs.md) registry. Pack zip layout:

```
oa-snes-rpg-top-50-1.0.0.zip
├── manifest.yml (type: collections)
├── collections/
│   ├── snes-rpg-top-50.yml
│   └── snes-rpg-hidden-gems.yml
└── assets/
    └── snes-rpg-top-50/hero.jpg
```

Each `.yml` lists collection metadata + ordered game IDs:

```yaml
id: snes-rpg-top-50
title: "Top 50 SNES RPGs"
author: "the OA editors"
description: |
  The genre-defining JRPGs of the 16-bit era…
cover: assets/snes-rpg-top-50/hero.jpg
ordered: true
games:
  - id: chrono-trigger
  - id: ff6
  - id: secret-of-mana
  # …
```

OA matches IDs against the operator's library and silently drops
missing entries — showing `12 / 50 in your library` on the
collection header. Never tells the operator to go buy anything;
just notes the overlap.

Curated collections are read-only. **Copy to my collections**
clones the current state (intersected with operator's library)
into a mutable custom collection, so the operator can then
add/remove freely.

---

## 12. Implementation sketch (not committed)

Not a green-lit implementation plan — rough mapping:

- New `CollectionsPage` route at
  `frontend/src/routes/Collections.tsx`.
- Rust side: extend the existing collections model in
  `library_db` to support the four flavors. Smart-query
  collections persist as a predicate AST; OA evaluates against
  library state on render.
- Curated collections loaded by the content-pack loader
  (already designed in [content-packs.md](content-packs.md));
  same loader handles editorial packs.
- Header card + tile grid reuse LIBRARY components; only the
  header is new.
- Inline `+ ADD GAMES` mini-grid is a stripped LIBRARY view in
  a modal with multi-select state.
- Drag-to-reorder uses the existing focus-group primitive from
  controller-nav v2 with a `data-oa-drag-handle` attribute.

Status: idea, not in `ACTIVE_WORK.md`. Implementation order
(when greenlit):

1. Collections model extension in `library_db` (manual + smart
   query + curated flavors).
2. SMART LISTS shipped first — Favorites already exists; the
   other built-ins are auto-computed predicates.
3. Custom manual collections + sidebar group + header card.
4. Smart-query builder + saved smart collections.
5. Curated pack consumption (depends on content-packs v1).

---

## 13. Open questions for future planning passes

- **Export / import.** Sharable file format for collections so
  operators can hand each other lists. Likely JSON listing game
  IDs + metadata. Defer to v2.
- **Operator-published collection packs.** Workflow for an
  operator to publish their curated list as a community pack
  via the [content-packs.md](content-packs.md) registry.
- **Sort vs order.** Manual collections support manual order;
  smart collections compute order from the sort field. UI needs
  to make this distinction obvious so operators don't expect
  manual-order on smart lists.
- **Collection of collections.** Nested collections (a "JRPG
  Marathon" collection containing "SNES JRPGs" + "PS1 JRPGs"
  sub-collections) — interesting but adds complexity. Defer.
- **Statistics drill-in.** Header stats (total time, cleared
  count) are great at a glance — should clicking them open a
  detail surface? Maybe in v2.
