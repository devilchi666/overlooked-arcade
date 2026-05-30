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

## Related plans

- `docs/PLANS/game-info-panel.md` — the full v1 plan + future evolution
- `apps/oa-shell/src/game_info.rs` — Rust types + parser
- `docs/cores/<id>/KNOWN_GAME_BUGS.md` — legacy free-form bug notes; being
  migrated into `games-info.md` in Phase 10 of the panel plan
