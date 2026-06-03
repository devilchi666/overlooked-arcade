# DISCOVER tab — Retroverse default theme

**Status:** Design sketch. No code.

**Author:** Operator + Claude (LLM pair).

**Owner-of-decisions:** the operator. This document records what was sketched.

**Reference:** [features/per-system-ui/assets/default-theme-mockup.png](../features/per-system-ui/assets/default-theme-mockup.png) (HOME) +
[features/per-system-ui/assets/library-default-mockup.png](../features/per-system-ui/assets/library-default-mockup.png)
(LIBRARY) + [settings-tab-retroverse.md](settings-tab-retroverse.md) +
[play-now-tab-retroverse.md](play-now-tab-retroverse.md).
This doc is the DISCOVER companion.

---

## 1. TL;DR

DISCOVER's job is the one the other tabs don't cover: **explore
the retro world itself** — editorial, historical context,
anniversaries, era / genre / developer / region browsing.

- LIBRARY = browse what you own
- HOME = browse by system
- COLLECTIONS = curated groupings of your library
- PLAY NOW = pick something to play right now
- **DISCOVER = learn and wander**

Think Apple News for retro games, not a store. For a non-
commercial preservation project this becomes the *"learn and
wander"* surface — never commerce-adjacent.

Same three-pane shell shape as the other tabs (consistency).
Center pane is magazine-style instead of grid- or rail-style.
The right pane is polymorphic — different focused-card type =
different detail shape (article preview / game detail / system
spotlight summary / era summary).

---

## 2. Layout shape

```
┌───────────────────────────────────────────────────────────────────────────────────┐
│ ◤ RETROVERSE      HOME  LIBRARY  COLLECTIONS  PLAY NOW  ▣DISCOVER▣  SETTINGS  ⌕  │
│   EMULATION FRONTEND                                              09:47 PM · 👤  │
├───────────────────────────────────────────────────────────────────────────────────┤
│ ┌─ EXPLORE ────────┐ ┌─ ▣ FEATURED ─────────────────────┐ ┌─ FOCUS ────────────┐ │
│ │                  │ │                                  │ │  ARTICLE           │ │
│ │ ▣ Featured     ◀ │ │ ╔══════════════════════════════╗ │ │                    │ │
│ │ 📅 On this day   │ │ ║                              ║ │ │ ┌──────────────┐   │ │
│ │ ⏳ By era        │ │ ║   [ FEATURE OF THE WEEK ]    ║ │ │ │ [hero image] │   │ │
│ │ 🎮 By genre      │ │ ║                              ║ │ │ │              │   │ │
│ │ 🌏 By region     │ │ ║   THE TG-16 THAT ALMOST WAS  ║ │ │ └──────────────┘   │ │
│ │ 🏢 By developer  │ │ ║   How NEC's underdog console ║ │ │                    │ │
│ │ 🎯 System dive   │ │ ║   almost beat Sega in '89.   ║ │ │ THE TG-16 THAT     │ │
│ │ 💎 Cult classics │ │ ║                              ║ │ │ ALMOST WAS         │ │
│ │ 👻 Lost games    │ │ ║       [ ▶ READ ARTICLE ]    ║ │ │                    │ │
│ │                  │ │ ║                              ║ │ │ ARTICLE · 12 min   │ │
│ │ ─────────────    │ │ ╚══════════════════════════════╝ │ │ By the OA editors  │ │
│ │ ⓘ Editorial is   │ │                                  │ │                    │ │
│ │   curated by OA  │ │ ─ TODAY'S HIGHLIGHTS ─────────   │ │ Released in '87 in │ │
│ │   + the          │ │                                  │ │ Japan as the PC    │ │
│ │   community —    │ │ ┌────────┐ ┌────────┐ ┌────────┐│ │ Engine, NEC's      │ │
│ │   no web fetch.  │ │ │ ◤ANNIV │ │ ◤SPOT  │ │ ◤ART   ││ │ first console…     │ │
│ │                  │ │ │        │ │        │ │        ││ │                    │ │
│ │                  │ │ │ Castle-│ │ Game   │ │ Konami │ │ [▶ OPEN ARTICLE]   │ │
│ │                  │ │ │ vania  │ │ Boy    │ │ years  │ │                    │ │
│ │                  │ │ │ SOTN   │ │ at 35  │ │ that   │ │ [♥ SAVE FOR LATER] │ │
│ │                  │ │ │ turns  │ │ — a    │ │ shook  │ │                    │ │
│ │                  │ │ │ 29     │ │ retro- │ │ retro  │ │ TAGS               │ │
│ │                  │ │ │ today  │ │ spect  │ │        │ │ #tg16 #history     │ │
│ │                  │ │ └────────┘ └────────┘ └────────┘│ │ #editorial #nec    │ │
│ │                  │ │                                  │ │                    │ │
│ │                  │ │ ─ SYSTEM SPOTLIGHT — TG-16 ────  │ │ RELATED IN LIBRARY │ │
│ │                  │ │                                  │ │ ✓ Bonk's Adventure │ │
│ │                  │ │ ┌──┐┌──┐┌──┐┌──┐┌──┐┌──┐  ▸    │ │ ✓ Splatterhouse    │ │
│ │                  │ │ │  ││  ││  ││  ││  ││  │       │ │ ✗ Magical Chase    │ │
│ │                  │ │ │  ││  ││  ││  ││  ││  │       │ │   (not in library) │ │
│ │                  │ │ └──┘└──┘└──┘└──┘└──┘└──┘       │ │                    │ │
│ │                  │ │ TG-16 essentials — curated      │ │                    │ │
│ │                  │ │                                  │ │                    │ │
│ │                  │ │ ─ ANNIVERSARIES THIS WEEK ────  │ │                    │ │
│ │                  │ │ ┌──┐┌──┐┌──┐┌──┐┌──┐┌──┐  ▸    │ │                    │ │
│ │                  │ │ │  ││  ││  ││  ││  ││  │       │ │                    │ │
│ │                  │ │ │  ││  ││  ││  ││  ││  │       │ │                    │ │
│ │                  │ │ └──┘└──┘└──┘└──┘└──┘└──┘       │ │                    │ │
│ │                  │ │ 29y  35y  18y  22y  14y  30y    │ │                    │ │
│ └──────────────────┘ └──────────────────────────────────┘ └────────────────────┘ │
├───────────────────────────────────────────────────────────────────────────────────┤
│  Ⓐ OPEN   Ⓑ BACK   Ⓧ SEARCH   Ⓨ SAVE FOR LATER    L1/R1 SECTION   RS AXIS    │
└───────────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Sidebar — EXPLORE axes

9 axes, organized as one group (vs Settings' three groups) since
they're all peers:

- **Featured** (default) — rotating editorial picks, mixed card
  grid
- **On this day** — anniversaries from `release_date` data in
  `library_db`, filtered MM-DD across years
- **By era** — 70s arcade · 8-bit · 16-bit war · handheld ·
  32-bit · etc.
- **By genre** — shmups · JRPGs · platformers · metroidvanias ·
  puzzle
- **By region** — JP-exclusive · EU PAL · NA-only · worldwide
- **By developer** — Capcom · Konami · Treasure · Squaresoft ·
  SNK · Compile
- **System dive** — deep-history per system (reuses per-system-
  ui identity work)
- **Cult classics** — under-celebrated games with editorial
  framing
- **Lost games** — cancelled / unreleased / prototypes
  (preservation angle)

---

## 4. Center pane — magazine layout

Departs intentionally from LIBRARY's clinical grid. Breathing
room, big editorial cards, contextual section labels. Per axis,
different framing:

- **Featured** → big "Feature of the Week" hero card + a mixed
  grid below (TODAY'S HIGHLIGHTS — articles, anniversaries,
  system spotlights together with type chips `◤ANNIV` / `◤SPOT`
  / `◤ART`).
- **On this day** → big "X years ago today" hero + chronological
  strip of releases on this date across years.
- **By era** → era timeline strip (1972 → 2000s) selectable like
  a tab bar, then era hero + key systems + defining games.
- **By genre / region / developer** → tile grid; click into one
  = filtered editorial view.
- **System dive** → reuses HOME's per-system hero but with
  editorial overlays (release timeline, sales history, key
  staff, signature games).

Each card carries a type chip (top-left corner) so the operator
knows what they're clicking into. Type chips:

- `◤ARTICLE` — long-form editorial
- `◤ANNIVERSARY` — "X years ago today" entry
- `◤SPOTLIGHT` — system / developer / studio deep-dive
- `◤ESSAY` — shorter opinion / overview piece
- `◤GAME` — direct game card (links to LIBRARY detail / launch)

---

## 5. Right pane — focused-card detail (polymorphic)

Different focused-card type renders a different right-pane shape:

- **Article card** → article preview (hero image, byline, "X min
  read," lead paragraph, OPEN ARTICLE + SAVE FOR LATER buttons,
  tags, **RELATED IN LIBRARY** block with in-library check
  marks).
- **Game card** → same as LIBRARY's focused-game detail, **plus
  a status badge**: `✓ In your library` or `✗ Not in your
  library — drop ROM in <path> to add`. Never a fake "buy" CTA.
- **System spotlight card** → system hero summary (year founded,
  units sold, key staff, signature games).
- **Era card** → era summary (years covered, defining systems,
  key transitions).
- **Developer card** → studio summary (founded, key franchises,
  status today).

The **RELATED IN LIBRARY** block on article detail is the
operator-facing payoff: editorial mentions games, the panel
shows which the operator already owns and which they don't.
Connects exploration back to actual collection.

---

## 6. Footer hint bar

- `Ⓐ OPEN` — context-sensitive: open article / play game / open
  system spotlight
- `Ⓑ BACK` — backs out to HOME
- `Ⓧ SEARCH` — search across all editorial content
- `Ⓨ SAVE FOR LATER` — bookmark articles for a "Saved"
  collection
- `L1 / R1 SECTION` — moves focus up/down between center-pane
  sections
- `RS AXIS` — quick-cycle through the explore axes

---

## 7. Notable deltas vs PLAY NOW / LIBRARY / HOME

- **No recommendation engine.** Editorial is curated, not
  algorithmic. Operator picks an axis; content shows.
- **No "play" as the dominant CTA.** READ ARTICLE / OPEN / SAVE
  are the primary actions; PLAY exists for games but isn't the
  headline.
- **Type-mixed cards.** Articles, anniversaries, system
  spotlights, and games all live in the same grid (with type
  chips) rather than being segregated into rails. Feels like
  browsing a magazine, not a queue.
- **Offline-first.** No web fetches per the locked rule.
  Editorial content lives on disk (built-in + community-supplied
  packs). DISCOVER works the same on a plane as at home.
- **Operator-library aware.** The RELATED IN LIBRARY block ties
  exploration back to ownership. The other tabs assume you only
  see games you own; DISCOVER intentionally shows games you
  don't and tells you so explicitly.

---

## 8. Content distribution model

Editorial content lives in `<exe_dir>/discover/`:

```
<exe_dir>/discover/
  builtin/                          # ships with OA installer
    articles/<id>.md
    spotlights/<system_id>.md
    axes/{eras,genres,regions,developers}.yml
    assets/<id>/hero.png
  community/<pack_id>/              # operator-installed packs
    manifest.yml
    articles/...
    spotlights/...
  overrides/                        # operator-edited overrides
    articles/<id>.md                # wins over builtin/community
```

Each article is markdown with front-matter:

```yaml
---
id: tg16-almost-was
title: "The TG-16 That Almost Was"
byline: "OA editors"
hero: assets/tg16-almost-was/hero.png
read_time_minutes: 12
tags: [tg16, history, nec]
related_games: [bonk-adventure, splatterhouse, magical-chase]
published: 2026-05-01
pack_id: builtin
---

Article body in markdown…
```

Layered loading order: `builtin/` → `community/<pack_id>/` →
`overrides/`. Last wins. Lets operators ship their own edits
without forking a pack.

Pack distribution + update mechanism lives in its own planning
doc (cross-cutting — also covers core updates, theme packs,
asset packs). See
[`content-packs.md`](content-packs.md).

---

## 9. Implementation sketch (not committed)

Not a green-lit implementation plan — rough mapping in case it
ever ships:

- New `DiscoverPage` route at `frontend/src/routes/Discover.tsx`.
- A `ContentIndex` service walks `<exe_dir>/discover/` at startup
  and on demand, merges layered packs, exposes a typed catalog
  (`articles[]`, `spotlights[]`, `anniversaries[]`).
- Markdown rendering via a sandboxed renderer
  (`marked` + DOMPurify, or `mdsvex`-style; no scripts, no
  external URLs).
- Hero images served via the existing Tauri asset protocol scope
  (the portable-install scope already covers `<exe_dir>/discover/`
  by extension).
- Anniversaries computed live from `library_db.release_date` —
  no separate dataset needed for the first pass; editorial
  packs can override / supplement.
- "Related in library" check = exists-by-ID lookup against the
  library DB at render time.
- Saved-for-later list stored in a per-operator `saved.json` in
  the OA data dir.

Status: idea, not in `ACTIVE_WORK.md`. Implementation will
follow once the operator green-lights the design and the
content-pack plumbing is in place.

---

## 10. Open questions for future planning passes

- **Editorial bootstrap.** Who writes the first 10-20 articles?
  Operator-curated baseline, or RFP'd from the retro-community
  writing scene with attribution + permissive license?
- **Markdown extensions.** Plain markdown for now, or extended
  syntax (figure captions, side-by-side comparisons, embedded
  game cards inline)?
- **Translation / locale.** English-only at first; structure for
  per-locale packs later (`<pack_id>/locales/<lang>/articles/`)?
- **Article reader chrome.** Full-screen reader vs sidebar
  reader vs modal. Reader experience is its own design pass.
- **Community pack verification.** Should OA verify pack
  signatures, or trust on first install with operator
  confirmation?
