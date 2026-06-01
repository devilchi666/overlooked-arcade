# `games-info.md` — Schema reference

The authoritative reference for the per-system `docs/cores/<id>/games-info.md`
files that drive the Game Info Panel.

The Rust types live at `apps/oa-shell/src/game_info.rs`; the parser there is
the source of truth for "what's a valid file." This document describes the
format for human contributors.

## File format

Each file is **multi-document YAML** with the standard `---` separator. One
YAML document per game record. The `.md` extension is for editor / GitHub
display friendliness; the content itself must be pure YAML.

**Every record MUST start with a `---` line on its own**, including the first
one. The parser uses the first `^---` line as a defensive anchor: anything
before it is treated as prose and skipped. Without leading `---` on the first
doc, a file's opening record may be silently dropped.

```yaml
---
id_key:
  system_id: psx
  rom_hash: 7a4b...
  rom_title: "Tomb Raider (USA)"
date: 1996
# ...
---
id_key:
  system_id: psx
  rom_hash: 1111...
  rom_title: "Final Fantasy VII (USA)"
date: 1997
# ...
```

YAML comments (lines starting with `#`) are fine anywhere. Lines that are
NOT comments and NOT valid YAML (raw markdown prose, backtick spans,
em-dashes outside quoted strings) will either parse as scalar strings —
producing a malformed record that's logged and skipped — or in pathological
cases hang the upstream tokenizer. The "skip everything before the first
`---`" rule protects against accidental prose at the top of the file.

Records that fail to parse are logged and skipped; one bad record doesn't
poison the file. Adjacent valid records continue to load.

## Field reference

### `id_key` (required)

The match key against the operator's local library.

| Field | Type | Required | Notes |
|---|---|---|---|
| `system_id` | string | yes | OA system slug — matches the directory name (`psx`, `n64`, `snes`, …). |
| `rom_hash` | string | no | SHA-1 of the canonical ROM, hex-encoded lowercase. Match priority 1. |
| `rom_title` | string | no | No-Intro / Redump canonical title. Match priority 2 (fallback). |

Match order: hash first, title second. At least one of `rom_hash` or
`rom_title` should be present, otherwise the record matches nothing in the
operator's library.

### Factual fields (all optional, sourced from metadata sync in v1)

| Field | Type | Example |
|---|---|---|
| `date` | integer | `1996` |
| `publisher` | string | `Eidos Interactive` |
| `region` | string | `USA` / `Europe` / `Japan` / `World` |
| `version` | string | `"1.0"` / `"Rev A"` / `"Prototype 1996-03-12"` |
| `player_count` | integer | `1` / `2` / `4` |
| `genre` | string | `Action-Adventure` |

Quote strings that look like numbers (`version: "1.0"` not `version: 1.0`)
so YAML doesn't coerce them to floats.

### Narrative fields (all optional, operator-editable / hand-curated)

#### `short_summary`

```yaml
short_summary: "A 3D action-adventure that helped define the genre."
```

One- or two-sentence editorial summary. When present, the UI renders this in
place of the existing `metadata.description` from the libretro-database sync,
with an `(operator note)` mini-label. Empty string (`""`) is distinct from
missing — both render as nothing in the UI, but `""` signals "intentionally
blank, no info."

#### `controls_supported`

```yaml
controls_supported:
  - "Standard gamepad"
  - "DualShock vibration"
  - "Mouse"
```

Free-form short strings for v1. Suggested categories: `Standard gamepad`,
`Light gun`, `Mouse`, `Trackball`, `Touchscreen`, `Multitap`, `Keyboard`,
`Analog stick`, `Pressure-sensitive triggers`, `Vibration`. v2 may evolve
to `RETRO_DEVICE_*` id strings for deeper picker integration.

#### `best_emulator`

```yaml
best_emulator:
  recommended: "beetle_psx_hw_libretro.dll"
  reason: "PGXP + Vulkan renderer eliminates the depth-buffering glitches of the SW renderer."
```

`recommended` is the libretro core filename (the same shape OA's core
installer uses). `reason` is optional but recommended — it's what appears
under the recommendation in the panel.

The "Apply best emulator" button in the panel writes `recommended` into the
operator's `GameOverrides.libretro_core` for this game.

#### `bugs`

```yaml
bugs:
  - description: "Crashes when entering Caves of Kaliya without prior save."
    severity: blocker
    workaround: "Save in the previous room first."
  - description: "Audio cuts in pre-rendered cutscene at start of Egypt level."
    severity: minor
```

Each entry has:

- `description` (string, required) — user-visible bug text.
- `severity` (enum, required) — one of `blocker` / `major` / `minor` / `cosmetic`.
- `workaround` (string, optional) — short workaround. Omit when none exists.

Severity drives the panel's emphasis and the tile badge's tint. The tile
badge shows the maximum severity across all bugs.

#### `touch_hotspots`

```yaml
touch_hotspots:
  - label: "Map"
    x: 8
    y: 16
    w: 80
    h: 80
  - label: "Inventory"
    x: 200
    y: 12
    w: 40
    h: 24
```

Game-specific tappable regions worth showing as labelled outlines while a
stylus-using game runs (Phantom Hourglass's map, Brain Age's letter zones,
Trauma Center's incision sites). The `TouchHotspotOverlay` frontend
component reads these and renders thin accent-coloured rectangles with
labels over the bottom-screen area.

Each entry has:

- `label` (string, required) — short label shown next to the outlined
  rectangle. Keep terse so it doesn't crowd small render sizes.
- `x` / `y` (integer, required) — top-left corner of the rectangle in
  **NDS bottom-screen native space** (0..256 × 0..192).
- `w` / `h` (integer, required) — width / height in the same native space.

NDS-specific in practice today; the schema is generic enough that future
stylus / pointer systems (PSP touch in some titles, Vita rear-touch, etc.)
can adopt it without schema changes — coordinates would be in that
system's primary touchscreen space.

**V1 limitation:** the overlay assumes the default melonDS stacked-screen
layout (top above bottom, bottom screen occupying y[192..384] of the
256×384 framebuffer). Non-default layouts (side-by-side, top-only)
misplace the hotspots until v2 reads the core option.

Toggle via Esc → QuickSettings → "Show touch hints" while a stylus-using
game runs.

### `meta` (optional, defaults populated)

```yaml
meta:
  schema_version: 1
  last_updated: "2026-05-26"
  contributors:
    - "operator-handle"
```

| Field | Type | Default |
|---|---|---|
| `schema_version` | integer | `1` |
| `last_updated` | string (ISO date) | _absent_ |
| `contributors` | string array | `[]` |

Bump `schema_version` only on breaking field changes; consumers reject
records with a `schema_version` higher than they understand.

## Missing / null / empty-string semantics

All three render as "nothing" in the UI but preserve distinct provenance in
the source file:

- **Missing** (field absent entirely): "not yet considered."
- **`null`** (explicit YAML null, e.g. `publisher: null`): "data source had no value."
- **Empty string** (`publisher: ""`): "intentionally blank, no info."

Use whichever signals the right intent for future contributors.

## Authoring tips

- One YAML document per game; separate with `---` on its own line.
- Order documents however you want — alphabetical, by release date, by
  region. The parser preserves source order.
- Lines starting with `#` are YAML comments — use them to group records
  by region or note authoring context.
- Keep `description` and `reason` strings short. The panel wraps long text
  but multi-paragraph entries crowd the layout.
- Quote any string that might be confused for a number, boolean, or date.

## Validation

Run `cargo test -p oa-shell --bin oa-shell game_info::` to validate the
parser against all the unit-test fixtures. A future task may add a
`cargo run --bin oa-shell -- validate-game-info` command for whole-file
linting.

## Migrating from `KNOWN_GAME_BUGS.md`

The per-core `KNOWN_GAME_BUGS.md` files are the legacy free-form markdown
notes that pre-date the structured schema. v1 migrates these into
`games-info.md` records — operator-driven, one system at a time:

1. **Identify per-game blocks** — typically delimited by an h2/h3 heading
   like `## Tomb Raider (USA)`. Some files use bulleted lists per game;
   judgement call.
2. **Build the `id_key`** — pull the canonical title from the operator's
   library (matches the No-Intro / Redump rom_title). The `rom_hash` is
   optional; look it up in `library_db.games.sha1` if available.
3. **Extract bug entries** — each "this game has issue X" sentence becomes
   one entry under `bugs:`. Pick the severity by reading the impact:
   - `blocker` — uncompletable / hard crash with no workaround
   - `major` — significant degradation, completable with effort
   - `minor` — noticeable but doesn't disrupt
   - `cosmetic` — visual glitch only
   Workaround text → `workaround:` field when the legacy notes mention one.
4. **Surface recommended-core mentions** — if the legacy notes say "use
   core X for this game," extract into `best_emulator.recommended` +
   `reason`.
5. **Spot-check** — `cargo test --bin oa-shell game_info::` validates the
   parser ate everything you wrote.
6. **Drop the legacy file** — once the YAML mirrors the markdown, the
   `KNOWN_GAME_BUGS.md` can be deleted. Until then both exist side by side;
   the panel reads YAML, the README still links the legacy file.

Per-system migration is independent — no need to do them all at once.

## Related plans

- `docs/PLANS/game-info-panel.md` — the full v1 plan + future evolution
- `apps/oa-shell/src/game_info.rs` — Rust types + parser
- `docs/cores/<id>/KNOWN_GAME_BUGS.md` — legacy free-form bug notes; being
  migrated into `games-info.md` per the workflow above

---

# `system-info.yaml` — Schema reference

The authoritative reference for the per-system
`docs/cores/<id>/system-info.yaml` files that drive the System Info
Panel v1 — the Retroverse HOME tab's right pane + system spotlight
hero. Three-layer cake:

- **L1 — MAME baseline.** Baked at launch from
  `assets/mame-source/listxml-slim.json` + `history-slim.xml` (slim
  artifacts emitted by `tools/mame-extractor/`). Carries the per-
  system data MAME's own tables already encode: manufacturer, year,
  CPU + sound chip + clocks, resolution, refresh rate, max players,
  raw peripheral hints, history.xml prose description.
- **L2 — OA curated YAML.** *This file.* One per system, one record
  per file (single-document YAML). Polishes L1 fields where MAME
  emits awkward strings, supplies fields MAME can't (release date
  with month/day, units sold, generation, operator-facing peripheral
  names + glyphs, hero blurb).
- **L3 — operator overrides.** SQLite `system_info_overrides`, edited
  via SETTINGS → per-system drill-in → "System info" section.

Field-typed precedence at read time: L3 wins per-field > L2 per-field
> L1 per-field. Same merge shape `games-info.md` uses for per-game data.

The Rust types live at `apps/oa-shell/src/system_info.rs`; the parser
there is the source of truth for "what's a valid file." This document
describes the format for human contributors.

## File format

Each file is a **single YAML document** (no `---` separator needed,
no multi-document support — one system per file). YAML comments
(lines starting with `#`) are fine anywhere.

```yaml
# SNES — system-info.yaml
system_id: snes
manufacturer: Nintendo
type: Home Console
generation: 4th Generation
release_date: "August 23, 1990"
# … rest of the fields …

meta:
  schema_version: 1
  last_updated: "2026-05-31"
```

Every field except `system_id` is optional. A YAML containing nothing
but `system_id` parses cleanly — every other field falls through to
L1 (MAME) or stays "—" in the UI.

The parser accepts both snake_case (canonical for YAML) and camelCase
keys via serde aliases — the wire format from Tauri is camelCase to
match the rest of OA's frontend conventions, but the YAML files stay
snake_case. Don't mix conventions within one file.

## Field reference

### Top-level — required

| Field | Type | Notes |
|---|---|---|
| `system_id` | string | OA system slug — must match the directory under `docs/cores/` (`snes`, `nes`, `psx`, …). |

### SYSTEM INFORMATION section

| Field | Type | Source notes |
|---|---|---|
| `manufacturer` | string | Cleaner form than MAME's (e.g. "Nintendo" instead of "NEC / Hudson Soft"). Overrides L1. |
| `type` | string | "Home Console" / "Handheld" / "Arcade" — editorial. |
| `generation` | string | "3rd Generation" / "4th Generation" — editorial. |
| `release_date` | string | Full date with month + day. L1 has year only. |
| `discontinued` | string | Never in MAME. |
| `units_sold` | string | Never in MAME. |
| `media` | string | "Cartridge" / "CD-ROM" / "DVD-ROM". |
| `cpu` | string | Polished CPU string ("Ricoh 5A22 @ 3.58 MHz"). Overrides L1. |
| `sound` | string | Polished sound chip ("Sony SPC700"). Overrides L1. |
| `resolution` | string | Polished resolution ("256 x 224"). Overrides L1's sometimes-pixel-clock emit. |
| `color_palette` | string | Not in MAME for home consoles; entirely L2. |
| `display_ratio` | string | Source pixel ratio framing ("8:7 (4:3)"). Entirely L2. |

### TECHNICAL DETAILS section

| Field | Type | Source notes |
|---|---|---|
| `architecture` | string | "8-Bit" / "16-Bit" — editorial. |
| `max_players` | string | Polished string ("1-2 Players"). Overrides L1's integer. |
| `multiplayer` | string | Free-form ("2 local; up to 5 via Super Multitap"). Entirely L2. |
| `region` | string | "NTSC / PAL". |
| `storage` | string | "Cartridge (4MB Max)". Entirely L2. |
| `ram` | string | "128KB". |
| `video_output` | string | "Composite / RGB". Entirely L2. |
| `aspect_ratio` | string | "4:3". Entirely L2. |
| `refresh_rate` | string | Polished form ("60 Hz" without decimals). Overrides L1's "60.10 Hz" emit. |

### `peripherals` (list)

```yaml
peripherals:
  - name: "SNES Controller"
    glyph: "🎮"
  - name: "Super Scope"
    glyph: "🔫"
  - name: "Multitap (X4)"
    glyph: "🔗"
```

| Sub-field | Type | Notes |
|---|---|---|
| `name` | string | Operator-facing display name. |
| `glyph` | string | Single emoji or short symbol (1-4 chars). Renders inline next to the name in SUPPORTED PERIPHERALS. |

The L1 layer's `peripheral_hints` (raw MAME control kinds: `joy`,
`lightgun`, `trackball`) doesn't feed the UI directly — only this
curated list does. L1 hints are kept around for future filters
("show only systems with lightgun support").

### Hero extras (L2-only)

| Field | Type | Notes |
|---|---|---|
| `release_flag` | string | Country-of-origin emoji ("🇺🇸" / "🇯🇵") next to the release date in the hero. |
| `tagline` | string | "16-BIT HOME CONSOLE" — short subtitle in the hero. |
| `blurb` | string | 2-4 sentence curator description. |
| `sidebar_subline` | string | "16-BIT · 1990" — compact subline under the system name in the SYSTEMS sidebar. |

### `meta` (optional)

```yaml
meta:
  schema_version: 1
  last_updated: "2026-05-31"
  contributors: []
```

| Sub-field | Type | Notes |
|---|---|---|
| `schema_version` | u32 | Defaults to 1. Bump only on breaking field changes. |
| `last_updated` | string | ISO 8601 date ("2026-05-31"). |
| `contributors` | string[] | Attribution list for v2's hand-curated content layer. Empty in v1. |

## Authoring workflow

1. Check whether `docs/cores/<system>/system-info.yaml` already
   exists. v1 ships entries for snes / nes / genesis / psx / gb.
2. Copy one of the existing files as a template — they're all the
   same shape.
3. Fill in the values you can verify. Drop fields you can't —
   missing fields fall through to L1 (MAME) or stay "—".
4. Spot-check: `cargo test --bin oa-shell system_info::` validates
   the parser ate everything you wrote. The
   `load_curated_records_parses_all_shipped_yamls` test asserts
   every file under `docs/cores/<id>/system-info.yaml` parses.
5. Restart `cargo tauri dev` — the bake-on-launch hash detects the
   new content and writes the row to SQLite.

Per-system entries are independent — no need to do them all at once.

## Related plans

- `docs/PLANS/system-info-panel-v1.md` — the full v1 plan
- `apps/oa-shell/src/system_info.rs` — Rust types + parser + merge
- `apps/oa-shell/src/mame_import.rs` — operator-driven L1 re-import
- `tools/mame-extractor/README.md` — slim-file generator + driver map
