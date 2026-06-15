# Metadata Curation — Settings → "Metadata" editing surface

**Status:** Planning locked 2026-06-11 (operator Q&A this session). Execution
queued; starts next session (Wave 1 / S1). Owner-of-decisions: the operator.

**One-line goal:** a first-class, *premium* operator surface in engine Settings
to edit **game** and **system** metadata, stored as an **override layer** over
synced/shipped facts — with per-field reset + provenance, and a UI that matches
OA's Heroic-ceiling aesthetic (NOT a Windows-98 property grid).

**Origin:** LaunchBox competitive research
[features/guided-setup/LAUNCHBOX_RESEARCH_2026-06-11.md](../features/guided-setup/LAUNCHBOX_RESEARCH_2026-06-11.md)
§4 + §10 Q2 (RESOLVED: "its own dedicated arc, soon"). Metadata editing is
called out as **OA's single biggest greenfield interaction win** over
LaunchBox — design it as the inverse of LaunchBox's §4.2 editing pain points.

> All `file:line` anchors below came from a 2026-06-11 read-only code sweep
> (subagent exploration). They're approximate — **verify at execution time**,
> the surrounding code is the source of truth.

---

## Context — what exists today (audited 2026-06-11)

A useful **asymmetry**: the backend is half-built, and unevenly.

| Layer | Backend override store + commands | Edit UI |
| --- | --- | --- |
| **System** factual metadata | ✅ **COMPLETE** — `system_info_overrides` table (`library_db.rs` ~`:1634`), `SystemInfoOverride` 25 fields (`system_info.rs` ~`:344`), `get/set/delete/reset_system_info_override` commands (`main.rs` ~`:8195–8276`), 3-layer merge L1 MAME → L2 curated YAML → L3 override (`merge_system_info` ~`:536`) | ❌ **missing** |
| **Game** narrative metadata (summary / controls / best-emu / bugs) | ✅ complete — `game_info_overrides` (`library_db.rs` ~`:1684`), `GameInfoOverride` (`game_info.rs` ~`:330`), `set/delete_game_info_override` (`main.rs` ~`:8106`) | ✅ exists — GameInfoModal "Game Info" tab |
| **Game** factual metadata (year / genre / developer / publisher / players / description / region / rating) | ❌ **MISSING** — read-only except `set_game_mame_metadata` (year + publisher, MAME-only, `media.rs` ~`:3361`); `update_identity_metadata` exists but `#[allow(dead_code)]` (`library_db.rs` ~`:3769`), no override table | ❌ **missing** |

**Where game-factual metadata lives:** `GameMetadata` in MediaDb / `media.json`
(per-ROM: year, genre, developer, publisher, players, description —
`media.rs` ~`:238`), enriched into `game_identities` columns (per-identity, +
rating; `library_db.rs` ~`:489`) via a COALESCE-never-overwrite pass
(`metadata::enrich_identities_from_media`). The `games` table's metadata
columns are read-only fallback, never user-written.

**Tiles/detail render from the identity** (canonical title + metadata),
per-file second — so the canonical edit target is the identity.

**Media mutations already plumbed (no UI):** `set_manual_cover` (`media.rs`
~`:3289`), `set_selected_variant` ("set as default image", ~`:3532`),
`clear_media` (~`:3409`). Store methods exist in `useMedia`.

**Settings category registry (stable, easy to extend):** `CATEGORIES` array +
`Match` arms in `engine/SettingsPanel.tsx` (~`:74–201` / `:429–492`); body
components in `engine/SettingsSections.tsx`. Existing groups: OA-WIDE / CONTENT
(library, media) / SYSTEM. A new `metadata` category plugs into CONTENT.

**Read-only view surfaces that already exist** (reference, not edit targets):
GameInfoModal (metadata grid + boxart gallery + editable narrative tab),
GameDetailPanel (Retroverse), DetailListView (read-only spreadsheet),
GamePropertiesDialog — which literally notes *"rich metadata editing lands in
Phase 4."* **This arc is that promise.**

---

## Locked decisions

### D1 — Location: its own `metadata` Settings category
The editor lives as a new `metadata` category in the engine Settings registry
(group: `content`, beside `library` / `media`) — **not** on the library tile,
**not** a per-game modal popped from the grid. A category registers a body
component, so the surface is **layout-agnostic** and survives the upcoming
settings-layout rework (see Coordination). Inline-in-library editing is
explicitly **deferred** (Wave 3 / "maybe later", per operator).

### D2 — Override layer everywhere (mirror the shipped pattern)
Operator edits are stored as a **per-field override layer** applied at read
time (`override → enriched/baked → source`); reset = drop the override row.
- **System** edits reuse the **already-shipped** `system_info_overrides`.
- **Game-factual** edits get a NEW `game_metadata_overrides` table built to
  mirror `game_info_overrides` / `system_info_overrides` exactly: sparse,
  `is_empty()`-deletes the row, every field `Option<_>`, field-typed merge,
  `created_at` / `updated_at`.
Re-sync never clobbers an edit; an edit never destroys source data. The
override is also the clean **diffable unit** for the future v2 "submit
correction" community flow (parallels Game Info Panel v2).

### D3 — Game edits key on the identity
`game_metadata_overrides` is keyed by `identity_id` (what tiles/detail render).
Per-variant (per-ROM) editing is a later Preservation-mode tier (composes with
virtual-library Phase B Variants tab), NOT in Wave 1.

### D4 — The Settings tab owns its own entity list (the picker)
Because editing no longer lives on the tile, the `metadata` category carries
its own **searchable, filterable game + system list**. This is OA's answer to
LaunchBox's "library spreadsheet," just housed in Settings — and the natural
host for Wave-2 multi-select bulk edit.

### D5 — Premium UX is a first-class requirement, not polish-later
The editor must look and feel top-tier (see §"UX pillar"). A flat
label:input property grid is an explicit non-goal. This is gated in the exit
criteria, not deferred.

---

## UX pillar — a premium curation surface (NOT a Windows-98 tab)

The visual ceiling is Heroic / the rest of OA: dark, spacious, strong
typographic hierarchy, tasteful motion. Concretely, Wave 1 ships:

- **Live preview.** A representative library tile + detail-hero preview pane
  that updates in **real time** as you edit — you see the exact library result
  of every change before you commit.
- **Typed, delightful field controls** (never bare text boxes):
  - Genres / controls / tags → **chip/token multi-select with typeahead from
    the existing library corpus** (keeps values consistent; pre-empts the
    LaunchBox typo/merge mess).
  - Developer / publisher → **autocomplete** from existing DB values.
  - Year / release date → year stepper / date control.
  - Rating → **star widget** (+ ESRB/PEGI segmented picker).
  - Region / release-type → segmented pill picker.
  - Players → numeric stepper.
  - Description → roomy auto-grow textarea with character count.
- **Per-field provenance.** A subtle "overridden" dot per field; hover/expand
  shows the underlying source value + a one-click **reset this field**.
  Always-visible answer to "what have I changed vs what was synced?"
- **Instant, optimistic feedback** — save state inline, **undo toast**, no
  modal save-blocking ceremony.
- **Search-as-you-type entity list** with cover thumbnails + filter chips
  (system, missing-a-field, overridden-only) + **Previous/Next** entity
  cycling (LaunchBox parity).
- **Keyboard + controller native** — full focus-ring + OA nav-verb support
  (`@oa/platform/nav` verbs), always-reachable escape hatch, respects
  `prefers-reduced-motion`.
- **Designed empty/edge states** — no-metadata-yet, unidentified game,
  system with only MAME baseline facts.

---

## Slices — three waves

### Wave 1 — the editor (MVP, mergeable on its own)

- **S1 — Game-factual override backend.** New `game_metadata_overrides` table
  (schema bump v23→v24) + `GameMetadataOverride` struct (mirror
  `GameInfoOverride`) + `get_game_metadata_override` / `set_game_metadata` /
  `delete_game_metadata_override` / `reset_game_metadata_field` commands.
  Merge into the identity read path (`build_groups` / `list_game_groups`):
  `override → enriched identity → per-file`. Expose the dormant
  `update_identity_metadata`. Field set aligned to LaunchBox §4.1 IGame
  (subset OA renders + a sensible superset): title / sort_title, year,
  developer, publisher, genre[], players, max_players, region, rating,
  release_type, series, description. Tests: sparse delete, merge precedence,
  reset, idempotent re-write. **Backend only; no UI change.**

- **S2 — `metadata` Settings category + SYSTEM editor.** Register the category
  (`SettingsPanel.tsx` `CATEGORIES` + `Match` arm) + a system-metadata editor
  body wiring the **already-shipped** `*_system_info_override` commands.
  Fastest visible win (no backend needed) and proves the premium-UX shell
  (live preview, provenance dots, typed controls) on a complete data layer.

- **S3 — GAME editor + entity list.** The searchable game list (picker, D4) +
  per-game editor over S1, with per-field provenance + reset, full premium-UX
  treatment, Previous/Next cycling. Fold the existing narrative game-info
  fields (summary / controls / best-emu / bugs) in alongside the factual ones
  so one surface edits the whole game record.

**Wave 1 exit criteria:**
- Settings → Metadata edits every factual field for a game AND a system,
  persisted as overrides, surviving restart + re-sync.
- Each field shows source-vs-override provenance + one-click reset.
- The surface meets the §UX-pillar bar (live preview, typed controls,
  controller-navigable) — operator playtest sign-off on *feel*, not just
  function.
- oa-shell tests green (822+ at arc start); frontend typecheck + lint silent.

### Wave 2 — depth (the LaunchBox-beating differentiators)

- **S4 — Undo stack.** Per-edit undo (SQLite-transaction journal + visible undo
  affordance). Inverse of LaunchBox §4.2 #4 (no undo → backup-restore only).
- **S5 — Merge-mode bulk edit + find-and-replace.** Multi-select in the entity
  list → add/remove genres etc. **merge, not overwrite** (inverse of §4.2 #3)
  + find-and-replace across the schema. Trivial on SQLite.

### Wave 3 — deferred / "maybe later"

- **S6 — "Fix wrong match" flow.** Re-key a game's identity directly
  (search/select the correct canonical game) — leverages hash identity +
  variant model. Recommendation #3 / §4.2 #5.
- **S7 — Media picker / "set as default image".** Surface the already-plumbed
  `setManualCover` + `setSelectedVariant` (click-to-pin) + download-only-missing.
  Recommendation #5.
- **S8 — Inline-in-library editing** (operator's "maybe later") + configurable,
  persistent, custom-field-aware list columns.

---

## Coordination & dependencies

- **Settings-layout rework (operator: "soon").** Wave 1 has **no hard
  dependency** — it registers a category like every other. Flag for the
  settings-layout arc: account for a new CONTENT-group `metadata` category.
  Design the body component layout-agnostic (no assumption the current 3-pane
  SettingsPanel survives).
- **Virtual-library Phase B/E.** Identity model (Phase E, shipped) is the edit
  target; per-variant editing (D3 deferral) composes with Phase B's Variants
  tab + Preservation mode.
- **Game Info Panel v2.** The override layer (D2) is the diffable unit the
  future "submit correction" → community-PR flow consumes (parallels v2).
- **Theming substrate.** Editor lives in **engine** territory (Settings),
  theme-free — no conflict with the in-flight ARC-1 work.

## Critical files (anchor points — verify at execution)

- `apps/oa-shell/src/library_db.rs` — schema bump + `game_metadata_overrides` + `game_identities` CRUD (`update_identity_metadata` ~`:3769`)
- `apps/oa-shell/src/media.rs` — `GameMetadata` (~`:238`), `set_game_mame_metadata` (~`:3361`, generalize), media mutations
- `apps/oa-shell/src/metadata.rs` — `enrich_identities_from_media` (merge order)
- `apps/oa-shell/src/system_info.rs` — `SystemInfoOverride` (~`:344`), `merge_system_info` (~`:536`) — **system side already done**
- `apps/oa-shell/src/game_info.rs` — `GameInfoOverride` (~`:330`) — the pattern to mirror
- `apps/oa-shell/src/main.rs` — command registration (`:8106` game-info, `:8195–8276` system-info)
- `frontend/src/engine/SettingsPanel.tsx` — `CATEGORIES` (~`:74`) + `Match` arms (~`:429`)
- `frontend/src/engine/SettingsSections.tsx` — body components
- `frontend/src/platform/api/` — new typed wrapper module(s) (Phase-4 invoke ban: all backend calls go through `platform/api/`)
- `frontend/src/platform/library/media.tsx` — `useMedia` (read + mutation methods)
- reference UIs: `GameInfoModal.tsx`, `GameDetailPanel.tsx`, `DetailListView.tsx`, `GamePropertiesDialog.tsx`

## Verification

- After each slice: `cargo test -p oa-shell` + frontend `npm run typecheck` +
  `npm run lint` (the `platform/api` invoke ban is enforced) + operator smoke
  playtest of the visible surface before merge.
- S1: SQL fixture test — set override, read merged, reset, confirm sparse-delete
  + that re-sync doesn't clobber an override.
- Wave-1 gate: operator sign-off on the §UX-pillar *feel*, not just function.

## Reference

- LaunchBox research §4 (the field model + the §4.2 anti-pattern list this arc
  inverts), §9 rec #2/#3/#5, §10 Q2.
- `docs/PLANS/game-identities-schema.md` — identity model (edit target).
- `docs/PLANS/virtual-library-and-launcher-arc.md` — Phase B (Variants /
  Preservation) the per-variant tier composes with.
- `docs/cores/SCHEMA.md` — the games-info YAML schema (factual + narrative
  reference the Game Info Panel reads; the override layer sits over it).
