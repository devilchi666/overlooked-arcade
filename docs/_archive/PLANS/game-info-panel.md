# Game Info Panel — Plan

**Status:** Planning. No code. Locked design after the 2026-05-26 planning session.

**Author:** Operator + Claude (LLM pair).

**Owner-of-decisions:** the operator. This document records what was decided.

---

## 1. TL;DR

Surface **structured reference data per game** in OA's library — date, publisher, region, version, player count, controls supported, known bugs, best-emulator recommendations, and an operator-editable short summary. **Not editorial, not recommendations** ("if you like X try this" is out of scope; that's closer to a future Play History Intelligence feature).

Progressive-disclosure UI: compact essentials on tile hover/focus → full panel on long-press / `i` hotkey. Actions in the panel: read-only for factual fields, "Apply best emulator" + "Apply controls" buttons for recommendations, "Edit locally" + "Submit correction" for operator contributions.

**v1 is intentionally tight** (~3-4 weeks): builds the data model + UI + KNOWN_GAME_BUGS migration + operator local edits, using supplied `.dat` files that OA already syncs. The full distribution + scraper + community contribution architecture is **locked in design but deferred to v2**.

Positioning: the practical answer to "should I launch this game right now?" — version / region / works-with-my-controller / known-issues / pick-the-right-core, surfaced in one place.

---

## 2. Goals + non-goals

### Goals

- **Help operators decide whether to launch a specific game.** All the information needed to make that call in one panel: version + region (am I picking the version I want?), player count + controls (can I play this how I want?), known bugs + best emulator (will it work?).
- **One source of truth per game.** Replace the current "is it in the metadata sync? KNOWN_GAME_BUGS file? per-game GameOverrides?" scattered-state with a coherent structured record.
- **Operator can contribute back.** Local edits + "Submit correction" surface that flows back to a maintained master list (eventually; v1 stubs the submit action).
- **Visible without being intrusive.** Progressive disclosure: tile hover for quick scan; long-press for depth. Operator doesn't pay attention cost they don't choose.
- **Ship in 3-4 weeks for v1.** Full vision is multi-stage; v1 is the foundation that subsequent stages enrich.

### Non-goals

- **No editorial content for v1.** No "why this game is important." No fun facts. No curated cultural commentary. (Operator can write any of this in the short-summary field if they want; OA doesn't source it.)
- **No recommendations engine.** "If you like X try this" is out of scope. That's similarity-graph work that overlaps with the deferred Play History Intelligence feature.
- **No external API integration for v1.** No Wikipedia / IGDB / TheGamesDB / ScreenScraper. The v1 source is supplied `.dat` files OA already uses for metadata sync. Richer sources land in v2 (see §11).
- **No scraper running in v1.** The scraper infrastructure is fully designed (see §11) but operators get static data from the DATs OA ships with, not a live-updating remote source.
- **No community contribution pipeline in v1.** The submission flow is designed (GitHub Issue with auto-PR; field-typed precedence; trusted-contributor queue) but lives behind a "Submit correction" button that's stubbed for v1.

---

## 3. Relationship to other plans

### Shared infrastructure with Guided Setup Phase 4 — KNOWN_GAME_BUGS structured format

Guided Setup Phase 4 (`docs/PLANS/guided-setup.md` §8) needs structured per-game data to auto-apply per-game core overrides from KNOWN_GAME_BUGS at import commit. This plan **defines that format** — YAML front-matter in per-system markdown files — and migrates the existing free-form markdown into it.

When this plan ships v1, Guided Setup Phase 4 can immediately consume the same data. Two features, one structured source.

### Shared infrastructure with Per-System UI Stage 3 — `metadataPriority`

Per-System UI Stage 3 (`docs/PLANS/per-system-ui.md` §13) introduces a `metadataPriority` field on `SystemUIConfig` that drives which game metadata surfaces are visible per system. The data behind those surfaces is what this plan defines.

### Distinct from the theme ecosystem WAIT lock (DECISIONS G, 2026-05-25)

The "no community ecosystem" decision applied to creative themes. Game info is a **factual database**, not a creative ecosystem (per the comparison table in this plan's history). The dead-ecosystem trap doesn't apply — the data has value at v1 even with zero community contributions because OA ships with seed data from existing `.dat` sources.

### Distinct from Play History Intelligence — deferred ChatGPT pitch

"If you like X, try this" recommendations and "you haven't played this in 2 weeks" prompts are Play History territory. This plan **explicitly excludes** that surface; recommendations live in the user's Play History layer (future, separate plan), not the per-game info layer.

---

## 4. Three-layer data model (architecture)

The full architecture is three-layered, even though v1 only ships layer 1 + layer 3:

```
LAYER 1 — SCRAPER OUTPUT (deferred to v2)
   Auto-generated objective fields from libretro-database DATs
   + KNOWN_GAME_BUGS migration
        ↓
LAYER 2 — HAND-CURATED CONTENT (deferred to v2)
   Project maintainer edits + community contributions
   Short summaries, refined best-emulator notes, controls supported,
   narrative bug entries
        ↓
LAYER 3 — OPERATOR LOCAL OVERRIDES (v1)
   Per-install SQLite table; operator's edits never leave their machine
   unless they explicitly "Submit correction"
```

**At runtime**, OA queries all three layers and applies **field-typed precedence** (see §8) to produce the final per-game record shown in the UI.

**For v1**, layer 1 is "what OA's metadata sync already produces" (no separate scraper), layer 2 is "the KNOWN_GAME_BUGS files in the OA repo, parsed", and layer 3 is the operator's SQLite edits.

---

## 5. Field schema + v1 sources

Nine structured fields. Each has a source for v1 and a long-term source for v2 evolution.

| Field | v1 source | v2 source | Type |
| --- | --- | --- | --- |
| **Date** (release year) | Existing metadata sync (libretro-database DATs) | Same | Factual |
| **Publisher** | Existing metadata sync | Same | Factual |
| **Region** | Existing metadata sync + DAT tags | Same | Factual |
| **Version** | Existing metadata sync + DAT rev tags (Rev A, Rev B, prototype tags) | Same | Factual |
| **Player count** | Existing metadata sync | Same | Factual |
| **Short summary** | Empty by default; operator-editable | Wikipedia / curator-written; field-typed merge with operator edit | Narrative |
| **Controls supported** | Empty by default in v1; can be migrated from KNOWN_GAME_BUGS notes if mentioned | Hand-curated in v2 data repo; potentially scraped from cores' valid devices | Structured list |
| **Game bugs** | Migrated from `docs/cores/<id>/KNOWN_GAME_BUGS.md` free-form markdown into structured entries | Maintained in v2 data repo with community contributions | Structured list |
| **Best emulator per game** | Migrated from KNOWN_GAME_BUGS (where mentioned) | Hand-curated in v2 data repo; field-typed merge with operator override | Recommendation |

### Schema (YAML front-matter in per-system markdown)

Per-system file at `docs/cores/<id>/games-info.md`. Each game entry is one YAML block separated by `---`:

```markdown
---
id_key:
  system_id: psx
  rom_hash: 7a4b...               # SHA-1 of the canonical ROM
  rom_title: "Tomb Raider (USA)"  # canonical no-intro title

# Auto-generated fields (would be scraper-managed in v2; manually maintained today)
date: 1996
publisher: Eidos Interactive
region: USA
version: "1.0"
player_count: 1
genre: Action-Adventure          # added when known; optional

# Hand-curated fields (v1: empty by default; manually filled over time)
short_summary: ""                # operator-editable; default empty
controls_supported:              # array of generic categories
  - "Standard gamepad"
  - "DualShock vibration"
best_emulator:
  recommended: "beetle_psx_hw_libretro.dll"
  reason: "PGXP + Vulkan renderer eliminates the depth-buffering glitches of the SW renderer."

# Bug entries (v1: migrated from KNOWN_GAME_BUGS.md narrative)
bugs:
  - description: "Crashes when entering Caves of Kaliya without prior save."
    severity: "blocker"
    workaround: "Save in the previous room first."
  - description: "Audio cuts in pre-rendered cutscene at start of Egypt level."
    severity: "minor"
    workaround: ""

# Metadata (for future scraper + community attribution layers)
meta:
  schema_version: 1
  last_updated: "2026-05-26"
  contributors: []
---
```

### Schema rules

- **`id_key`** is mandatory. Match operator's local game by `rom_hash` first; fall back to `(system_id, rom_title)` for unhashed homebrew / prototypes.
- **All fields are optional except `id_key`.** Missing field = OA shows nothing for that slot in the panel.
- **Empty string vs null vs missing** are distinct: empty string = "intentionally blank, no info"; null = "data source had no value"; missing = "field not yet considered." All three render as nothing in the panel; provenance preserved in the data file.
- **Arrays default to empty** (`bugs: []`, `controls_supported: []`).
- **`meta.schema_version`** lets later OA versions parse old data; bump only on breaking field changes.

---

## 6. UI surfaces (progressive disclosure)

Three distinct surfaces, each with a different role:

### Surface 1 — Tile hover / focus card (compact, transient)

When cursor / controller focus lands on a library tile, a small card appears (anchored to the tile, dismisses on focus-leave). Shows the highest-priority essentials:

```
┌─────────────────────────────────────────┐
│ Tomb Raider (USA)                       │
│ 1996  ·  Eidos Interactive  ·  1 Player │
│                                         │
│ ⚠ 2 known issues                        │
└─────────────────────────────────────────┘
```

- Fields: title, date, publisher, player count, bug-count indicator if any bugs exist
- Layout: 4-6 lines max, fits next to the tile
- Visibility: appears after ~200ms hover/focus; dismisses on focus-leave or input

### Surface 2 — Full panel (long-press / `i` hotkey / right-click)

The complete game record. Opens as a side panel or modal (implementation choice; lean side-panel that pushes the library tiles aside).

Sections, in order:

```
┌──────────────────────────────────────────────────────────┐
│ Tomb Raider (USA)                                   [×]  │
├──────────────────────────────────────────────────────────┤
│                                                          │
│ FACTS                                                    │
│   Released:        1996                                  │
│   Publisher:       Eidos Interactive                     │
│   Region:          USA                                   │
│   Version:         1.0                                   │
│   Players:         1                                     │
│                                                          │
│ SUMMARY                                                  │
│   [empty — operator-editable]              [Edit]        │
│                                                          │
│ CONTROLS SUPPORTED                                       │
│   • Standard gamepad                                     │
│   • DualShock vibration                    [Apply]       │
│                                                          │
│ RECOMMENDED CORE                                         │
│   beetle_psx_hw_libretro.dll               [Apply]       │
│   PGXP + Vulkan renderer eliminates depth-buffering      │
│   glitches of the SW renderer.                           │
│                                                          │
│ KNOWN ISSUES (2)                                         │
│   ⚠ BLOCKER — Crashes when entering Caves of Kaliya      │
│      without prior save.                                 │
│      Workaround: Save in the previous room first.        │
│                                                          │
│   • minor — Audio cuts in pre-rendered cutscene at       │
│      start of Egypt level.                               │
│                                                          │
│ ──────────────────────────────────────────────────────── │
│         [Edit locally]    [Submit correction]            │
└──────────────────────────────────────────────────────────┘
```

- Open: long-press on tile (mouse + controller), `i` hotkey when tile focused, right-click → "Game info"
- Close: B button (controller), Escape (keyboard), click outside (mouse), `i` again

### Surface 3 — Tile badge (subtle, always visible)

Tiny indicator on the tile itself when relevant signals exist:

- `⚠ N` badge when there are known issues — operator sees at a glance which games have known bugs without hovering
- A small "✓" or "⚙" mark if operator has local edits to that game (so they know they've personalized it)

Subtle; doesn't crowd the tile. Single icon + small number max.

---

## 7. Actions in the panel

Hybrid model: facts are read-only; recommendations have inline apply; submission paths surface at the panel bottom.

### Read-only fields

Date, publisher, region, version, player count, bugs section. The panel shows them; operator can't change them in-place (except via "Edit locally").

### Inline apply buttons

- **`[Apply]` next to "Recommended Core"** — sets `GameOverrides.libretro_core` for this game. One click sets the per-game override; next launch uses the recommended core.
- **`[Apply]` next to "Controls Supported"** — wires `GameOverrides.libretro_device_port1..4` to match the suggested device. For most games this is a no-op (already on standard gamepad); for light-gun titles or multitap titles, it sets the right per-port device type.

**Confirmation behavior:** apply buttons commit immediately, no confirmation dialog. Toast notification: "Best emulator applied — Beetle PSX HW will be used on next launch." Undo available via Settings → Per-game → Cores.

### Edit locally

Bottom-of-panel button. Opens an inline editor (Solid component, not a modal-on-top-of-modal) for the editable fields:
- Short summary (free text)
- Controls supported (multi-select from a known list)
- Best emulator (dropdown of installed cores)
- Known bugs (add / remove entries)

Operator's edits write to a local SQLite table (`game_info_overrides` keyed by `(system_id, rom_id)`). Edits never leave the operator's machine unless they explicitly "Submit correction" (see below).

### Submit correction

Bottom-of-panel button. **v1 behavior: stubbed.** For v1, this button copies a JSON dump of the operator's local edits for this game to the clipboard with a toast: "Your changes are copied. We're not yet set up to receive submissions automatically — coming soon." This makes the surface visible without committing to backend infrastructure.

**v2 behavior:** generates a pre-populated GitHub Issue URL on the `overlooked-arcade-game-info` data repo with the operator's edits as a structured form body. Operator clicks → GitHub Issue opens → operator hits submit. Maintainer reviews + (via auto-PR action) merges. See §11 future evolution.

---

## 8. Conflict resolution — field-typed precedence

The architecture supports three data layers (scraper / hand-curated / local edits). Even in v1 where only layers 1 + 3 are populated, the policy is locked.

When the layers disagree on a field, precedence depends on the field's nature:

### Always local wins (operator's edits sacred)

- Short summary — operator wrote it; their words stay
- Bugs (added by operator) — operator's testing knowledge stays
- Controls supported — operator's specific setup knowledge stays
- Best emulator — operator's discovered preference stays

### Always project / scraper wins (objective facts)

- Date — release year is a fact
- Publisher — fact
- Region — fact
- Version — fact
- Player count — fact

These fields are read-only in the UI anyway. If a scraper update finds a corrected publisher name, it overrides any stale local data.

### Three-way merge with operator approval

Currently no fields fall in this bucket. Reserved for fields where neither precedence rule is right (e.g., a recommendation that operator partially agreed with but added their own caveat).

### How precedence applies in code

```
read_game_info(system_id, rom_id):
  scraper_data = layer_1_data(system_id, rom_id)
  curated_data = layer_2_data(system_id, rom_id)
  local_data   = layer_3_data(system_id, rom_id)

  for each field:
    if field in always_local: prefer local
    if field in always_master: prefer (curated || scraper)
    if field in three_way: ... (none yet)

  return merged_record
```

---

## 9. v1 scope (the tight ship)

Two paths from "no Game Info Panel exists" to "v1 ships":

### Code work (v1)

1. **YAML front-matter parser** for `docs/cores/<id>/games-info.md` files. One-time load at OA startup; in-memory index by `(system_id, rom_hash)` and `(system_id, rom_title)`.
2. **SQLite migration** for the `game_info_overrides` local-edits table.
3. **Game info query layer** that returns merged records per the field-typed precedence in §8.
4. **Tile-hover card component** — appears after ~200ms focus, shows essentials.
5. **Full panel component** (side panel or modal; lean side panel).
6. **Tile badge component** for the `⚠ N` indicator + local-edits indicator.
7. **Inline editor component** for "Edit locally" (Solid form bound to SQLite override table).
8. **"Apply best emulator" + "Apply controls" actions** wired to `GameOverrides.libretro_core` + `libretro_device_port1..4`.
9. **"Submit correction" stub** — clipboard copy + informational toast for v1.
10. **Keyboard shortcuts** — `i` to open panel when tile focused; controller long-press equivalent.

### Content work (v1)

11. **KNOWN_GAME_BUGS migration** — parse the existing `docs/cores/<id>/KNOWN_GAME_BUGS.md` free-form files and emit structured entries into `games-info.md`. Each `KNOWN_GAME_BUGS` entry becomes one game record with the `bugs` array populated. Where the existing markdown mentions a recommended emulator, populate `best_emulator`. Where it mentions controls, populate `controls_supported`. Imperfect but better than ignoring the existing knowledge.
12. **Schema versioning** — `meta.schema_version: 1` on every entry. Documented in `docs/cores/SCHEMA.md`.

### Doc work (v1)

13. **`docs/cores/SCHEMA.md`** — defines the YAML front-matter schema for `games-info.md` files. Authoritative reference.
14. **Update `docs/cores/<id>/README.md`** to mention the new `games-info.md` companion file.

### Ship criteria — "v1 done" means

- `docs/cores/<id>/games-info.md` files exist for all 40 systems with at least the migrated bug entries from KNOWN_GAME_BUGS.
- For any game in the operator's library that has metadata sync data (date, publisher, region, version, player count), the tile hover card shows those fields.
- Long-press / `i` opens the full panel with all sections rendering.
- "Edit locally" works — operator can write a short summary; it persists in SQLite; next launch the summary still appears.
- "Apply best emulator" wires the per-game core override and the change is reflected in Settings → Per-game.
- "Submit correction" copies JSON to clipboard with the informational toast.
- `cargo test --workspace` green.

### v1 effort estimate

- Code work (1-10): ~2.5-3 weeks frontend + backend.
- Content work (11-12): ~0.5-1 week (KNOWN_GAME_BUGS migration via scripted pass + spot-check).
- Doc work (13-14): ~2-3 days.
- **Total v1:** ~3-4 weeks.

---

## 10. Coverage in v1

| What | Coverage |
| --- | --- |
| Date / Publisher / Region / Version / Player count | Every game with libretro-database DAT entry (most of every major library) |
| Short summary | Empty by default; operator can fill any game they care about |
| Controls supported | Empty by default in v1; can be backfilled in v2 |
| Game bugs | Migrated from KNOWN_GAME_BUGS files (current coverage = whatever those files have today; varies per system) |
| Best emulator | Migrated from KNOWN_GAME_BUGS where mentioned (sparse coverage in v1) |
| Local edits | Anywhere operator chooses to edit; no coverage minimum |

**The honest read:** in v1 the panel is most useful for games that already have metadata sync + are flagged in KNOWN_GAME_BUGS. For obscure / un-bugged games, the panel shows the basics + empty slots. Operator can fill in the empty slots for their favorites.

---

## 11. Future evolution (DEFERRED — designed, not built)

Locked architectural decisions for when v2+ work begins. **None of this is in v1.** All of it is captured so future-Claude or future-you doesn't relitigate.

### 11.1 Scraper infrastructure

A scheduled scraper produces layer-1 data from external sources:

- **Sources for v2:** libretro-database DAT files + KNOWN_GAME_BUGS markdown migration (already in OA repo). v3+ may add Wikipedia/Wikidata (CC-BY-SA, attribution required) and possibly TheGamesDB or ScreenScraper (license review needed).
- **Location:** likely GitHub Actions on the `overlooked-arcade-game-info` data repo. Cron-scheduled (weekly initially).
- **Output format:** updates the same YAML front-matter in `games-info.md` files. Scraper only touches its own fields (per §11.3 source tagging).

### 11.2 Separate data repo: `overlooked-arcade-game-info`

The `games-info.md` files move from the main OA repo to a dedicated `overlooked-arcade-game-info` GitHub repo when v2 starts. Reasons:

- Lower contribution bar — non-technical contributors don't need to clone the Rust codebase
- Cleaner versioning (data version distinct from OA version)
- Could be reused by other projects (community-good)
- Smaller diff surface for OA releases

### 11.3 Sync mechanics

OA installs pull from the data repo on a schedule:

- **Daily auto-check** — quiet, current.
- **Manual "Check now" button** in Settings → Library → Game info data.
- **Off toggle** for fully-offline operators.
- **Update strategy** — fetch the latest commit's `games-info.md` files; apply field-typed precedence with operator's local edits (per §8).

### 11.4 GitHub Issue → auto-PR submission flow

Operator clicks "Submit correction" → OA generates a URL to a pre-populated GitHub Issue on the data repo with a structured form (operator's edits as the issue body). Operator hits submit → GitHub Action parses the issue + opens a PR with the proposed change → maintainer reviews + merges.

Initial trust model: every submission needs maintainer review. Long-term evolution to trusted-contributor queue once submission volume justifies it.

### 11.5 Field-source tagging evolution

Initial v2 ships PR-only scraper updates (every scraper run produces a PR). Long-term evolution to per-field source tags (`_source: { date: "scraper", short_summary: "manual", best_emulator: "manual" }`) so the scraper can auto-merge non-conflicting updates while leaving hand-curated fields alone.

### 11.6 Layer 2 — hand-curated content

Project maintainer (and eventually trusted volunteers) write short summaries, refined best-emulator notes, narrative bug descriptions, controls-supported entries. Lives in the data repo. Layered between scraper output and operator local edits per §4.

### 11.7 What's NOT in the future plan

These were considered and rejected (see §3 for the rationale on each):

- **External commercial APIs** that prohibit redistribution (some ScreenScraper terms).
- **Recommendations engine** — belongs in Play History Intelligence, not Game Info Panel.
- **Editorial content** — fun facts, cultural commentary, "why it's important." Operator can write this in the short summary field if they want; OA doesn't source it from anywhere.
- **Auto-merge of community submissions without review.**

---

## 12. Open implementation questions

Don't need answers to plan. Need answers during the build.

1. **Tile-hover card placement** — anchored to the tile? Floating top-right? Always-visible side panel that updates with focus? Probably anchored-to-tile with smart positioning to avoid the screen edge.
2. **Full panel: side panel vs modal vs full-page** — side panel reads cleanest. Modal feels heavy for "I want to glance at info." Full page is overkill.
3. **Tile badge styling** — corner-of-tile vs overlay-on-tile? Pixel-sized? Color (yellow ⚠ for any bug, red for blockers)? Tune in implementation.
4. **`controls_supported` granularity** — generic categories ("Standard gamepad", "Light gun", "Mouse") for v1; specific device-type names (RETRO_DEVICE_LIGHTGUN with sub-class) for v2 with deeper integration.
5. **Match priority** — when operator's library has a game that matches the data file by both hash AND title-but-different-region, which wins? Probably hash (more specific); spell it out clearly.
6. **Local edits storage** — SQLite table is the plan; column shape is implementation choice. JSON blob keyed by game vs columnar?
7. **`Edit locally` UX for arrays (bugs, controls_supported)** — add/remove rows in the inline editor? Bulk import from text? Implementation choice.
8. **`Apply best emulator` reversibility** — does the action set a flag that the operator can later "Reset to default" without manually picking the system default? Probably yes — a "this came from Game Info Panel" provenance marker.
9. **What does the tile badge show when operator has CLOSED a known bug?** (E.g., they tested it and it doesn't reproduce.) Probably operator can "Acknowledge" a bug in their local override — the bug stays in master data but hides on their install.
10. **First-launch onboarding** — does OA do anything special the first time operators see the new panel? Probably no — the panel just appears when relevant. No tour.

---

## 13. v2 / future additions (post-v1, post-distribution-layer)

Documented now; not in v1 or v2 first-cut.

- **Bulk-edit operator local edits.** Apply a change across multiple games at once (mark all games in this series as needing the same core; add a tag to a curated subset).
- **Per-game cover art override surfaced in the panel.** Inline thumbnail; click to swap from local file.
- **Play count / last-played surfaced** (would need the Play History Intelligence feature to source it).
- **Achievements / completion state surfaced** if RetroAchievements integration ever ships.
- **Side-by-side region/version comparison.** When operator has multiple regions of the same game, an option to see them side by side with deltas highlighted.
- **Export local edits as a "personal collection" sharable file.** Operator can hand someone their `.oa-collection` file with all their per-game notes and they can import it.
- **Per-system info-panel layout customization.** Stage 3 of per-system UI may want some systems to surface different metadata priority orderings (e.g., arcade systems show "control panel art" prominently).

---

## 14. Related plans + dependencies

- **`docs/PLANS/guided-setup.md`** — Guided Setup Phase 4 needs the structured per-game data format defined in this plan. When this plan ships v1, Phase 4 of guided setup can immediately consume the same data (auto-apply per-game core overrides at import commit).
- **`docs/PLANS/per-system-ui.md`** — Per-System UI Stage 3's `metadataPriority` config field drives which fields are most prominent per system. The data behind those fields is what this plan defines.
- **`apps/oa-shell/src/library_db.rs`** — existing `GameOverrides` struct. Local edits + "Apply" actions write into existing override fields (`libretro_core`, `libretro_device_port1..4`, etc.). No new override fields needed for v1.
- **`apps/oa-shell/src/metadata.rs` + `apps/oa-shell/src/rom_hashes.rs`** — existing metadata sync from libretro-database DATs. v1 reads from the games table that this code populates. No changes needed; just consumption.
- **`docs/cores/<id>/KNOWN_GAME_BUGS.md` files** — current free-form markdown. v1 migration script reads these and emits structured entries.
- **`docs/cores/<id>/README.md` files** — get a new "Game info" section referencing the companion `games-info.md`.
- **DECISIONS G (2026-05-25) theme-ecosystem WAIT** — separate concern from this plan; the comparison is documented in §3 to avoid future confusion.
- **DECISIONS Q (2026-05-26) kiosk-shell positioning** — kiosk shell consumes the Game Info Panel as-is for its in-mode game info displays.

---

## 15. What "ready to start" looks like

Before v1 code is written:

- Confirm the field schema (§5) matches the operator's actual mental model. If a field is missing or wrong, easier to fix now than after data files exist.
- Confirm `docs/cores/<id>/games-info.md` is the right location for v1 (in OA main repo). v2 will move to the separate `overlooked-arcade-game-info` data repo; v1 starts in main.
- Decide whether to update `docs/NEXT.md` to add Game Info Panel as a third major arc alongside Guided Setup and Per-System UI. (Game Info Panel is smaller — ~3-4 weeks v1 — so it could ride alongside Phase 0 of the other two arcs, or as a polish pass after Per-System UI Stage 1.)
- Optionally capture the strategic decisions from this planning session in `docs/DECISIONS.md`.

None block writing this plan; they're natural next steps after planning closes.

---

## 16. Decision summary (the locks made during planning)

For quick reference:

- **Scope:** Game Info Panel — factual reference data, not editorial / recommendations / "context system."
- **Fields:** 9 fields locked (§5). Five from existing metadata sync, four needing new structure or content.
- **Data format:** YAML front-matter in per-system markdown.
- **Storage location v1:** `docs/cores/<id>/games-info.md` in OA main repo.
- **Storage location v2:** separate `overlooked-arcade-game-info` data repo.
- **Sync mechanics (v2):** daily auto-check + manual "Check now" button.
- **Conflict resolution:** field-typed precedence — always-local for narrative + operator preferences; always-master for objective facts.
- **Three-layer model:** scraper output (v2) / hand-curated (v2) / operator local edits (v1).
- **UI:** progressive disclosure — tile-hover card + long-press / `i` full panel + tile badge.
- **Actions:** read-only facts; "Apply best emulator" + "Apply controls" inline; "Edit locally" + "Submit correction" at panel bottom.
- **"Submit correction" v1 behavior:** clipboard copy + informational toast (stub for v1).
- **"Submit correction" v2 behavior:** auto-pre-populated GitHub Issue → auto-PR → maintainer review + merge.
- **Scraper architecture (v2):** scheduled job in the data repo's GitHub Actions; PR-only updates initially; evolves to per-field source tagging.
- **Scraper sources (v2):** libretro-database DATs + KNOWN_GAME_BUGS migration; Wikipedia/etc later only if v2 proves insufficient.
- **v1 scope:** supplied DAT files only; no scraper running; no separate data repo; no community contribution pipeline. Tight 3-4 week ship.
