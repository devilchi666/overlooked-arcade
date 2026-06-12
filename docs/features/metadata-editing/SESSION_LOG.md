# Metadata Curation — Session Log

Newest first. Three lines per entry: **Shipped / Almost / Next**.

---

## 2026-06-12 — Wave 1 close-out (3 of 4 items)

- **Shipped:** (1) **Hide empty systems** — `MetadataSettingsBody` loads
  `list_game_groups` once, derives the systems-with-games set, filters
  the Systems list, and passes the groups + filtered system list down to
  `MetadataGamePane` (no double fetch; until groups resolve it shows all,
  then filters). (2) **Narrative game-info folded in** — a "Notes &
  guidance" section in the game editor edits summary / controls (chips) /
  best-emulator (+ why) via the existing `get/set/delete_game_info_override`
  commands keyed by (systemId, default-variant rom_id), with its own
  baseline/draft + debounced autosave sharing the save-status; Reset-all
  now clears both the factual + narrative overrides. (3) **Per-system
  drill-in pointer** (operator chose Option A) — the drill-in's flat
  "System info" editor is replaced by a note + "Edit in Metadata →"
  button (`onOpenMetadata` → `selectCategory("metadata")`); the old
  `PerSystemInfoSection.tsx` is now orphaned (left on disk, removable).
  `cargo test` 837 green; frontend typecheck + lint silent.
- **Deferred (item 4 — controller-native nav):** proper DPad
  region-traversal needs `useDomQueryFocusGroup` wiring **and** extracting
  the system pane into its own component for clean mount/unmount with the
  mode switch — and it's untestable without controller hardware. Native
  Tab focus works today. Held for a focused controller-playtest pass with
  the operator in the loop rather than shipping untestable focus plumbing
  (the focus framework has known subtle bugs). Still queued in NEXT.md.
- **Other open:** narrative section has no file-layer provenance baseline
  (operator-note fields, treated as override-only); structured bugs list
  deferred; picker "edited" dot is factual-only (narrative edits don't
  flip it); orphaned `PerSystemInfoSection.tsx` removal.
- **Next:** the controller-nav pass (with playtest), then Wave 2 (S4 undo
  stack, S5 merge-mode bulk edit + find-replace).

## 2026-06-12 — Wave 1 / S3: GAME editor + entity-list picker + typed controls

- **Shipped:** The **Games** half of the metadata editor, behind a
  **Systems/Games segmented switch** in the takeover top bar (system pane
  untouched). New `MetadataGamePane.tsx`: a **searchable game picker**
  (`list_game_groups`, system-filter dropdown, cover thumbs, edited dots,
  capped at 500 rows) + a **typed editor** over the S1
  `game_metadata_overrides` backend + a **game tile/hero live preview** +
  optimistic debounced autosave (same race-guarded pattern as the system
  pane). **Typed controls** (`metadataControls.tsx`, reusable): year /
  players / max-players **NumberStepper**, **StarRating** (0–5),
  **ChipInput** for genres with `<datalist>` typeahead from the library
  corpus, **SegmentedPills** for region + release type, TextField (with
  developer/publisher datalist autocomplete) + TextArea. Quiet provenance
  reused via a shared `ProvenanceField` (Default-on-hover + per-field
  reset), baseline = the pristine `game_identities` row. Backend: exposed
  the dormant **`get_identity`** (provenance baseline) + added
  **`list_game_metadata_overridden`** (picker dots/filter), both
  registered commands; new **`platform/api/gameMetadataApi.ts`** wrappers
  + **`platform/library/gameMetadata.ts`** types (invoke-ban preserved).
  `cargo test -p oa-shell` **837 green**; frontend typecheck + lint
  silent.
- **Almost / open:** (1) the narrative game-info fields
  (summary/controls/best-emu/bugs, keyed by rom_id) are NOT folded in yet
  — they need the default-variant rom_id and a small section; deferred to
  keep S3 shippable. (2) Controller-native nav within the takeover still
  isn't wired (native focus only) — applies to both panes. (3) Genre
  default flattens the identity's single-TEXT genre by splitting on
  commas; multi-word genres containing commas would mis-split (rare).
  (4) Picker caps at 500 rows (logged via slice, not silent) — fine until
  virtualization lands. (5) Per-system drill-in still hosts the old flat
  `PerSystemInfoSection` (redirect/deprecate still open). (6) **TODO
  (operator, 2026-06-12):** both the Systems list and the Games picker's
  system-filter dropdown list all ~45 registry systems regardless of whether
  the library has any games for them — filter both to systems with ≥1 game.
  Queued in NEXT.md close-out.
- **GATE:** operator playtest of the **game editor feel** + the
  Systems/Games switch (D5 premium sign-off).
- **Next:** Wave-1 close-out — fold narrative game-info in; controller-nav
  port; Per-system-drill-in redirect decision. Then Wave 2 (undo stack,
  merge-mode bulk edit + find-replace).

## 2026-06-12 — S2 layout rework (post-playtest: "very very busy")

- **Shipped:** Reworked the metadata editor per the operator's playtest
  feedback + Concept-A pick (DECISIONS D6–D9). `MetadataSettingsBody` is
  now a **full-screen takeover** with a `‹ Settings` back button
  (SettingsPanel renders it in place of the 3-pane grid when the
  `metadata` category is active; `prevCategory` records where to return).
  **Quiet provenance:** rows are clean label+value at rest, an edited
  field carries a thin accent bar, and the "Default: <value> · Reset"
  affordance only appears on row **hover / focus-within** (keyboard-
  reachable) — the MAME-vs-curated distinction dropped to a hover
  tooltip; the visible word is just "Default". **Data-driven expander
  groups** (`FIELD_GROUPS` config): "Identity & hero" leads (open),
  "Technical details" + "Peripherals" start collapsed with an
  edited-count badge so hidden edits stay visible. **Collapsible preview
  panel** toggled from the top bar. Dropped the always-on `SettingRow`
  chips that caused the clutter. typecheck + lint silent (no backend
  change this round).
- **Almost / open:** controller-native nav *within* the takeover isn't
  wired yet (relies on native Tab focus + the focus-within reveal) — the
  HintRegion/focus-group treatment the normal Settings pane has needs
  porting in. The Per-system drill-in still hosts the old flat
  `PerSystemInfoSection` (redirect/deprecate still deferred to S3).
- **GATE (still open):** operator re-playtest of the new layout for the
  premium *feel* sign-off (D5).
- **Next:** S3 — game editor + entity-list picker (Systems/Games switch
  in the same takeover), typed controls (genre chips, year stepper,
  rating stars), narrative game-info fields folded in, new
  `platform/api/` game-metadata wrappers. Plus the controller-nav port +
  the Per-system-drill-in redirect decision.

## 2026-06-12 — Wave 1 / S2: metadata Settings category + premium SYSTEM editor

- **Audit finding (the plan's S2 premise was stale):** the system-info
  override **editor already shipped** — `PerSystemInfoSection.tsx`, live
  in Settings → Per-system drill-in → "System info" (all 21 fields,
  provenance badges, save + reset-all, peripheral editor, wired to the
  shipped `*_system_info_override` commands). But it's a flat
  label:input grid — no live preview, no typed controls, no per-field
  reset — closer to the "Windows-98 tab" D5 rejects than premium.
  Operator chose **Option A**: register the new `metadata` category and
  build a premium system editor there (reuse the data layer, lift the
  UX), per-system-drill-in copy's fate decided later.
- **Shipped:** New **`metadata` Settings category** (CONTENT group,
  `SettingsPanel.tsx` `CategoryId` + `CATEGORIES` + `Match` arm) →
  **`MetadataSettingsBody.tsx`** (engine). Premium system editor hitting
  the §UX-pillar bar: (1) **live preview hero** (self-contained engine
  render, no theme import) that updates in real time from the draft;
  (2) **per-field provenance + one-click reset** via the existing
  `SettingRow` (inherited curated-L2 / MAME-L1 value shown, struck
  through when overridden, Reset chip); (3) **optimistic debounced
  autosave** (600ms) with a quiet Saving…/Saved status + race-guarded
  against fast system switches; (4) **search-as-you-type system list**
  with an "Edited only" filter chip + per-system "edited" dots; (5)
  peripheral editor + Reset-all. Backend: exposed the dormant
  `list_system_info_overridden` (dead_code removed → registered command
  + `systemApi.listSystemInfoOverridden` wrapper + `systemInfo`
  re-export) so the list badges/filter take one query, not 45 fetches.
  `cargo test -p oa-shell` 837 green; frontend `typecheck` + `lint`
  silent (the platform/api invoke-ban holds — the body calls only API
  wrappers).
- **Almost / known v1 limit:** the inherited chip shows the curated (L2)
  value reliably; for fields that exist ONLY at the MAME (L1) layer it
  shows the baseline only while un-overridden (the merge backend exposes
  no L1-without-L3 read). Documented in the file header. Also: the
  Per-system drill-in still hosts the OLD flat `PerSystemInfoSection` —
  whether to redirect it to the new editor / deprecate it is an open
  operator call (Option A deferred that).
- **GATE:** Wave-1 exit needs **operator playtest sign-off on the
  premium *feel*** (live preview, typed controls, controller-nav), not
  just function — this is the D5 gated criterion, not polish-later.
  Please open Settings → Metadata, pick a system, and judge the feel.
- **Next:** S3 — the **GAME editor + searchable entity-list picker**
  over the S1 `game_metadata_overrides` backend, in the same `metadata`
  category (add a Systems/Games segmented switch at the top of the
  body). Typed controls matter more here (genre chips w/ typeahead, year
  stepper, rating stars, region/release-type segmented pills). Fold the
  narrative game-info fields (summary/controls/best-emu/bugs) in
  alongside the factual ones. New `platform/api/` wrappers for the S1
  game-metadata commands. Then the Per-system-drill-in redirect decision.

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
