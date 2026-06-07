# Phase E — `game_identities` schema promotion (sub-phase plan)

Parent arc: [virtual-library-and-launcher-arc.md](virtual-library-and-launcher-arc.md)
§6 Phase E. Branch family: `feat/virtual-library-phase-e*`.
Started 2026-06-07 after the Theming Substrate ARC 1 Phase-2 pause
handed the floor to VL per theming plan §7.

## Goal in one line

The library stops being "a list of files with runtime grouping" and
becomes "a table of *games* (identities) that files point at" — so
play time, favorites, canonical metadata, and artwork attach to the
game, not whichever dump happened to be launched.

## Current state (audited 2026-06-07, schema v22)

- `games` — one row per file. Carries per-file `play_time_secs` /
  `favorite` / `completed` / `cover_path`. The `year/genre/developer/
  publisher` columns exist but are **never written by enrichment** —
  real metadata lives in MediaDb (`media.json`), keyed per game id.
- `list_game_groups` (main.rs) recomputes `library_groups::
  build_groups` in-memory on every call: buckets by `(system_id,
  base_title_key, disc_number)`, ranks variants by release status →
  region priority → revision priority, honors per-group pins from
  `game_group_defaults`.
- `game_group_defaults` — `(system_id, base_title)` → pinned game id.
- MediaDb has no identity-level keyspace; covers are per-file.

## Design decisions

### D1 — Identity granularity: per game, not per disc

`game_identities` is keyed `(system_id, normalized_title)` where
`normalized_title` == `build_groups`' `base_title_key` (lowercased
parsed base). FF7 Disc 1/2/3 share **one identity**. The A1
Sub-phase 4 behavior (per-disc variant buckets so "Run version ▸" on
Disc 1 shows only Disc-1 dumps) is preserved *inside* the identity:
group rendering buckets by `(identity_id, disc_number)`. Identity =
the game; disc structure stays a `disc_sets` concern.

### D2 — Deterministic identity ids

`id = "idn-" + hex(sha1(system_id + "\x1f" + normalized_title))[..16]`.
Re-running the migration (or a rebuild) on the same library produces
the same ids — the exit criterion's idempotency requirement falls out
of the id scheme rather than needing bookkeeping. (sha1 is already a
dependency via disc-track hashing.)

### D3 — One rebuild path for migration AND scans

A single `rebuild_identities_for_system(system_id, ...)` upserts
identity rows from the current games table (reusing `build_groups`
ranking to pick the default variant), reassigns `games.identity_id`,
and deletes orphaned identities. The v23 migration calls it for every
system; scan/identify flows call it for affected systems afterwards.
No second "keep identities fresh" code path to drift.

### D4 — Canonical metadata enrichment is NOT in the migration

The SQLite migration cannot read `media.json` (wrong layer — MediaDb
is owned by `media::MediaState` at the command layer). v23 populates
identities with what SQLite knows: canonical/normalized title,
default variant, and the default variant's `cover_path` as the
starting canonical cover. Year/genre/developer/publisher start NULL;
Sub-phase 3's enrichment pass fills them from the default variant's
MediaDb metadata at the command layer.

### D5 — Pins: dual-write until the read path swaps

Sub-phase 1 copies existing `game_group_defaults` pins into
`game_identities.default_variant_id` and makes
`set_game_group_default` write **both** places. Sub-phase 2 swaps the
read path to identities and retires the legacy writes; the
`game_group_defaults` table is dropped in a later cleanup migration
once nothing reads it (same pattern as the media-taxonomy migration's
sentinel approach — never strand a downgrade mid-arc).

### D6 — Per-variant stats stay; identity stats are derived

`games.play_time_secs/favorite/completed` remain per-file ground
truth. Identity-level numbers are SQL aggregates (SUM of play time;
favorite/completed via `EXISTS` over variants in Sub-phase 2 reads).
Phase B's "toggleable per-variant via Preservation view" needs the
per-file columns intact, so nothing migrates *off* the games table.

## Sub-phases

### Sub-phase 1 — v23 migration + identity CRUD (backend only)

- v22 → v23: `CREATE TABLE game_identities (id TEXT PK, system_id,
  canonical_title, normalized_title, year, genre, developer,
  publisher, players, rating, canonical_cover_path,
  default_variant_id, UNIQUE(system_id, normalized_title))` +
  `games.identity_id` column + indexes.
- `rebuild_identities_for_system` per D3; migration runs it for all
  systems; pins copied per D5.
- CRUD: list/get identities, update identity metadata,
  set default variant. `GameRow` gains `identity_id`.
- Scan/identify completion hooks call the rebuild for touched
  systems.
- Tests: migration populates, re-run idempotent (same ids), pins
  copied, rebuild reassigns on insert/retitle, orphans deleted.
- **No frontend or read-path changes.** App behaves identically.

### Sub-phase 2 — read path swap (backend)

- `list_game_groups` reads identity rows (JOIN games) instead of
  recomputing; variant ranking inside a group preserved verbatim.
  `GameGroup` gains `identity_id` + canonical metadata fields.
- Per-identity stats: aggregate play_time / favorite / completed
  exposed on the group payload.
- `set_game_group_default` → writes `default_variant_id` only;
  legacy `game_group_defaults` writes retired.
- Cross-file search keys on canonical title (FTS over identities or
  JOIN).

### Sub-phase 3 — MediaDb identity keyspace + frontend + enrichment

- `identity_media` keyspace in media.json (canonical/parent artwork;
  per-file media untouched per arc decision S4).
- Enrichment pass per D4: canonical metadata from default variant's
  MediaDb metadata; runs post-identify + on demand.
- Frontend: tiles render identity canonical title/cover;
  GameDetailPanel header shows canonical metadata; search hits
  identities once, not per-file.

### Exit criteria (from the arc plan)

- Existing libraries migrate cleanly; re-run produces same ids.
- Every library tile renders from an identity row.
- Per-identity metadata, per-variant metadata, per-identity stats
  all queryable + editable.
- oa-shell tests stay green throughout (790 at Phase E start).
