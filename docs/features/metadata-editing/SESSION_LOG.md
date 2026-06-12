# Metadata Curation — Session Log

Newest first. Three lines per entry: **Shipped / Almost / Next**.

---

## 2026-06-11 — Wave 1 / S1: game-factual override backend

- **Shipped:** `game_metadata_overrides` table (schema v23→v24,
  `migrate_v23_to_v24`), keyed by `identity_id` (D3), sparse +
  `is_empty()`-deletes like its `game_info_overrides` /
  `system_info_overrides` siblings. New `GameMetadataOverride` struct
  (`library_db.rs`, all-`Option`, camelCase, `genre` as JSON array;
  `PartialEq` not `Eq` because `rating: f64`) with `is_empty()` +
  `apply_to_identity()` (the read-time overlay). CRUD on `LibraryDb`:
  `get` / `set` / `reset_game_metadata_field` /
  `list_all_game_metadata_overrides`. Tauri commands
  `get_game_metadata_override` / `set_game_metadata` /
  `delete_game_metadata_override` / `reset_game_metadata_field` +
  exposed the formerly-dormant `update_identity_metadata` (its
  `#[allow(dead_code)]` removed, now a registered command). Read-path
  merge wired into `list_game_groups` (`main.rs`): one bulk override
  load → overlay onto the in-memory identity map before `build_groups`,
  so override → enriched identity → per-file, DB columns stay pristine.
  7 new SQL fixture tests (default-absent, round-trip + sparse-delete,
  upsert-replace, merge precedence over enriched, reset-field +
  sparse-delete, **survives re-sync**, corrupt-genre-JSON degrades).
  `cargo test -p oa-shell` → **837 passed / 0 failed** (was 822+ at arc
  start). Backend only, no UI. Branch `feat/metadata-curation`.
- **Almost:** the override stores the full LaunchBox §4.1 field set
  (title/sort_title/year/developer/publisher/genre[]/players/max_players/
  region/rating/release_type/series/description), but the read-time
  merge only surfaces the 7 fields the `game_identities` row carries
  (title→canonical_title, year, developer, publisher, genre, players,
  rating). sort_title / max_players / region / release_type / series /
  description are persisted + read back via the CRUD for the S3 editor,
  but have no render surface yet — intentional for backend-only S1.
- **Architecture note (D2 vs "expose update_identity_metadata"):** the
  override layer (`set_game_metadata`) is the operator-edit path — it
  keeps the identity columns as the pristine enriched/synced source so
  a reset just drops the row. `update_identity_metadata` is a DIRECT
  canonical write (COALESCE onto `game_identities`) that can't be reset
  back to source; it's exposed as the enrichment-adjacent primitive but
  is NOT the editor path. Flag for S3: edits go through the override
  layer, not the direct writer.
- **Next:** S2 — register the `metadata` Settings category
  (`SettingsPanel.tsx` `CATEGORIES` + `Match` arm) + the SYSTEM editor
  body over the already-shipped `*_system_info_override` commands.
  Fastest visible win; proves the premium-UX shell (live preview,
  provenance dots, typed controls) on a complete data layer. Then S3
  (game editor + entity-list picker over this S1 backend). Both gated on
  the §UX-pillar exit criterion (premium feel, not a property grid).

## 2026-06-11 — Arc planned + scoped

- **Shipped:** Planning locked. Read-only code sweep mapped the metadata
  storage + edit surfaces; plan written
  ([../../PLANS/metadata-editing.md](../../PLANS/metadata-editing.md)) with
  decisions D1–D5; feature folder + NEXT.md HIGH-band Wave-1/S1 queue +
  ACTIVE_WORK + INDEX updated. Operator answered the three forks: editor lives
  as its own **Settings tab** (covers systems too), **override layer**,
  inline-in-library **deferred**; plus an explicit ask for a **premium editor
  (not a Windows-98 tab)** — now a gated UX pillar + exit criterion.
- **Almost:** nothing in code yet — paperwork only.
- **Next:** Wave 1 / **S1** — `game_metadata_overrides` table (v23→v24) +
  `GameMetadataOverride` struct + get/set/delete/reset commands mirroring
  `game_info_overrides`, merged into the identity read path; expose the dormant
  `update_identity_metadata`. Backend only, no UI. Then S2 (Settings category +
  system editor — fastest visible win, backend already exists) → S3 (game
  editor + entity-list picker).
