# System Info Panel v1 — Plan

Per-system metadata for the Retroverse HOME tab's right pane, populated
from MAME's `-listxml` + `history.dat` as the baseline, layered with
OA-curated YAML overrides + per-install operator edits.

Replaces the current `frontend/src/routes/retroverse/systemMetadataStubs.ts`
(hardcoded TypeScript constant covering 5 of 45 systems; the other
40 render em-dashes everywhere). Lifts the three-layer pattern Game
Info Panel v1 introduced for *per-game* data up to the *per-system*
surface.

Planning conversation: 2026-05-31. This doc is the spec; treat
anything else as commentary.

---

## 1. The problem today

`SystemInfoPanel.tsx` renders four cards (SYSTEM INFORMATION /
TECHNICAL DETAILS / SUPPORTED PERIPHERALS / ACHIEVEMENTS) sourced
entirely from `systemMetadataStubs.ts`:

- **Coverage:** 5 of 45 systems have curated entries (snes / nes /
  genesis / psx / gb). The other 40 fall through to `DEFAULT_SPECS = {}`
  and render `—` for all 22 rows.
- **Provenance:** Hardcoded TypeScript object literals. No upstream
  data source; the operator who hand-typed the 5 entries is the only
  author.
- **Edit surface:** None. Operators wanting to fix / refine values
  must edit TypeScript source.
- **Achievements card:** Hardcoded placeholder numbers (`68/147`,
  `46%`, etc.) — same constants for every system. Not real data.

The achievements gap is a separate stream (RetroAchievements
integration); v1 leaves it as-is. The other three cards are this
plan's scope.

---

## 2. Solution shape — three-layer cake

Mirrors Game Info Panel v1's `merge_game_info` precedence pattern,
applied to per-system records.

| Layer | Source | Storage | Update cadence |
|---|---|---|---|
| **L1 — MAME baseline** | MAME's `-listxml` + `history.dat` for ~40 OA systems that map to a MAME machine driver | SQLite (`system_info_mame` table) hydrated from shipped slim files at first launch | Per OA release; operator can re-import from their MAME install |
| **L2 — OA curated** | Hand-edited YAML per system at `docs/cores/<id>/system-info.yaml`; mirrors the existing `games-info.md` pattern | SQLite (`system_info_curated` table) baked at first launch from in-tree YAML files | Per OA release |
| **L3 — Operator edits** | Per-install local overrides set via the per-system Settings drill-in | SQLite (`system_info_overrides` table); per-field columnar storage | Whenever the operator clicks Save |

**Precedence at read time:** L3 wins per-field > L2 per-field > L1
per-field. Same field-typed merge shape Game Info Panel v1 uses
(`merge_game_info` in `apps/oa-shell/src/game_info.rs`).

---

## 3. Schema

21 fields total, three sections + four hero extras.

### SYSTEM INFORMATION (12 fields)

| Field | Source | Notes |
|---|---|---|
| `manufacturer` | MAME | Direct field. |
| `type` | L2 | "Home Console" / "Handheld" / "Arcade" — editorial. |
| `generation` | L2 | "3rd Generation" / "4th Generation" — editorial. |
| `release_date` | MAME (year) → L2 (full date) | MAME has the year; L2 promotes to "October 29, 1988". |
| `discontinued` | L2 | Never in MAME. |
| `units_sold` | L2 | Never in MAME. |
| `media` | MAME → L2 | Cartridge / CD-ROM / floppy / etc. |
| `cpu` | MAME → L2 polish | Extractor formats "Ricoh 2A03 N2A03 1789773 Hz" → "Ricoh 2A03 @ 1.79 MHz". |
| `sound` | MAME → L2 polish | Same formatting treatment. |
| `resolution` | MAME | Direct from `<display>`. |
| `color_palette` | MAME | Palette size from `<display>` when present. |
| `display_ratio` | L2 | Source pixel ratio framing ("8:7 (4:3)"); MAME has horizontal aspect math but not this framing. |

### TECHNICAL DETAILS (9 fields)

| Field | Source | Notes |
|---|---|---|
| `architecture` | L2 | "8-Bit" / "16-Bit" — derivable from CPU but editorial. |
| `max_players` | MAME → L2 | MAME `<input>` `players` attribute. |
| `multiplayer` | L2 | **Refined from "Co-Op Support: Yes/No"** to free-form. Examples: "2 local; 4 with multitap"; "Up to 8 via link cable"; "Single player only". |
| `region` | MAME | Direct. |
| `storage` | L2 | "Cartridge (4MB max)" — slightly redundant with `media` but kept distinct. |
| `ram` | L2 | Sometimes in MAME, often not — treat as editorial. |
| `video_output` | L2 | "Composite / RGB" / "Built-in LCD" — editorial. |
| `aspect_ratio` | L2 | Display aspect ("4:3"). |
| `refresh_rate` | MAME | **New field.** Vertical refresh from MAME's `<display>`. "60 Hz" / "59.94 Hz" / "50 Hz". |

**Dropped from current schema:**
- `inputLatency` — every system had "Low" hardcoded; meaningless (real latency depends on operator hardware, not metadata).
- `emulatorCore` — OA's per-system core setting, not system metadata. Already lives in SETTINGS → per-system → Default core.

### PERIPHERALS

`peripherals: Array<{ name: string; glyph: string }>` — JSON column.

- **L2 authoritative** for the operator-facing list. The OA team
  curates names ("Zapper" / "Super Scope") + glyphs (emoji or short
  symbols) per system.
- **L1 carries raw MAME hints** in a separate column: an array of
  unique `<control type="...">` values per system (`["lightgun", "joy",
  "trackball"]`). Not shown to operators directly; reserved for future
  filters ("show only systems with lightgun support") + cross-checks
  with `apps/oa-shell/src/light_gun_systems.rs`-style tables.

### Hero extras (L2-only)

Used by `HomePage.tsx`'s system spotlight, not the right pane:

- `release_flag` — country-of-origin emoji ("🇺🇸" / "🇯🇵")
- `tagline` — "16-BIT HOME CONSOLE"
- `blurb` — 2-4 sentence curator description
- `sidebar_subline` — "16-BIT · 1990" for the SYSTEMS sidebar entries

MAME has none of these — entirely L2.

### Schema metadata

Per-record `schema_version: u8` (default 1, bump on breaking changes).
Same evolution pattern as Game Info Panel v1's `GameInfoMeta`.

---

## 4. File layout

### In-tree (committed to repo)

```
assets/mame-source/
  listxml-slim.json         # ~5MB, OA-relevant machines only
  history-slim.dat          # ~5MB, OA-relevant entries only
  mame-version.txt          # e.g. "0.262" — what bump-mame.sh used

docs/cores/<id>/
  system-info.yaml          # L2 curated overrides, single YAML doc per file

tools/
  bump-mame.sh              # maintainer script — slims MAME data
                            # Requires MAME installed locally
```

### Operator install (shipped with OA)

```
<exe_dir>/assets/mame-source/
  listxml-slim.json         # copied from in-tree
  history-slim.dat          # copied from in-tree
  mame-version.txt

<exe_dir>/docs/cores/<id>/
  system-info.yaml          # copied from in-tree (same resolver pattern
                            # as games-info.md uses today)
```

### Operator data dir (per-install, mutable)

```
<data_dir>/library.db
  system_info_mame          # L1 — rebaked from slim on hash mismatch;
                            # OR overwritten by re-import action
  system_info_curated       # L2 — rebaked from YAML files on hash mismatch
  system_info_overrides     # L3 — written by edit UI
  system_info_meta          # holds the L1+L2 content hashes for
                            # detect-rebake-needed logic
```

The slim MAME files + L2 YAMLs are treated identically to the
existing `docs/cores/<id>/games-info.md` resolver pattern — at
runtime, OA tries `<exe_dir>/...` first, falls back to the source
tree (so dev builds work without an install step).

---

## 5. Bake-on-launch mechanics

1. At startup, OA reads `system_info_meta` table to fetch the stored
   content hash of (slim MAME files + every L2 YAML).
2. OA hashes the current bundled slim MAME files + every L2 YAML.
3. If the hashes match: no work; SQLite L1 + L2 tables are current.
4. If mismatched (or `system_info_meta` empty): rebake.
   - Parse `listxml-slim.json` → write `system_info_mame` rows.
   - Parse `history-slim.dat` → enrich the `system_info_mame` rows.
   - Parse every `docs/cores/<id>/system-info.yaml` → write
     `system_info_curated` rows.
   - Store new content hash in `system_info_meta`.
   - **`system_info_overrides` table is NEVER touched** by rebake.

Hash cost: ~5ms total. Rebake cost: ~50-100ms cold, only on
hash-mismatch. Acceptable launch overhead.

### Re-import-from-MAME path

Operator clicks SETTINGS → Storage → "Refresh MAME system info":

1. OA attempts to locate MAME at OS-typical paths
   (`/usr/bin/mame`, `C:\mame\mame.exe`, etc.) + offers a folder
   picker.
2. If MAME missing: toast "MAME install not found. Point me at your
   MAME folder, or install MAME to refresh."
3. If MAME found: OA invokes `mame -listxml` + reads MAME's bundled
   `history.dat` (typically under `<mame>/history/`).
4. Runs the same slim + extract pipeline but against the operator's
   MAME data.
5. Overwrites `system_info_mame` rows for every MAME-known system.
   **Does NOT touch L2 or L3.**
6. Shows toast: "Refreshed N systems from MAME 0.265."

### Re-import vs OA-update interaction (deliberately simple in v1)

- Operator's re-imported L1 is **session-scoped**. Next OA update
  rebakes from the bundled slim source, overwriting the operator's
  re-imported data.
- Documented in the re-import button's tooltip: *"Refreshes from
  your local MAME install. Next OA update will reset to the bundled
  data."*
- L3 (operator edits) are NOT session-scoped — those always survive
  OA updates.

Per-row provenance tracking (sticky operator re-imports) is out of
scope for v1. If multiple operators ask, v2 adds it.

### Systems MAME doesn't cover

DOSBox, ScummVM, and any future OA system with no MAME machine
driver: extractor emits an L1 stub row (system_id + empty fields).
The L2 YAML carries all values. Merge behaves identically — file
layer just happens to be empty for these systems.

---

## 6. Operator surfaces

### Edit UI

**Location:** SETTINGS → per-system drill-in → new "System info"
section, alongside the existing Display / Bindings / Default core
sections in `PerSystemSettingsBody.tsx`.

**Shape:** Form with the ~21 refined fields. Each field shows the
current MERGED value (from L1+L2+L3) with a hint badge indicating
provenance:
- No badge → showing L1 (MAME)
- `curated` → showing L2 (OA curated)
- `edited` → showing L3 (operator override)

Operator edits write to `system_info_overrides` per-field. Empty /
cleared field → row deleted (falls back through the precedence
chain). "Reset all overrides for this system" button clears the
operator's entire L3 row for that system.

Peripherals section: editable list with add/remove rows + glyph
picker (text field — operators paste emoji).

### Re-import UI

**Location:** SETTINGS → Storage → "Refresh MAME system info"
button.

**Behavior:** Single click → all MAME-known systems re-imported at
once. Progress indicator while running. Per-system failures
reported in a toast: "Refreshed 38 systems. 2 failed: ..."

L2 + L3 untouched.

### HOME tab

`SystemInfoPanel.tsx` + `HomePage.tsx` hero both consume a new
`get_system_info(systemId)` Tauri command returning a
`MergedSystemInfo` shape. No visual changes from operator
perspective unless we hit a YAML row that fills a previously-empty
field — at which point the panel renders real data instead of "—".

---

## 7. Migration of existing stubs

The 5 hand-authored entries in `systemMetadataStubs.ts` (snes / nes
/ genesis / psx / gb) become the seed `system-info.yaml` files:

- Mechanical 1:1 conversion of each `SystemSpecs` object to YAML.
- `releaseFlag` / `tagline` / `blurb` / `sidebarSubline` carry over
  as hero fields in the new schema.
- `inputLatency` + `emulatorCore` dropped (per schema refinement).
- `coOpSupport: "Yes"` re-expressed as free-form `multiplayer:
  "Yes — 2 local"` etc.
- `peripherals` carry as-is.

Post-migration: `systemMetadataStubs.ts` deleted entirely.
`SystemInfoPanel` + `HomePage` hero rewritten to read via the new
Tauri command.

---

## 8. Phase commit sequence

Single branch `feat/system-info-panel-v1`, 6 phase commits:

1. **MAME extractor tool + slim source files.** Rust binary at
   `tools/mame-extractor/` (separate from `oa-shell`); reads MAME's
   `-listxml` output + `history.dat`; emits slim JSON. Shell wrapper
   `tools/bump-mame.sh` for maintainer workflow. Initial commit
   includes `assets/mame-source/listxml-slim.json` +
   `history-slim.dat` + `mame-version.txt` generated against a
   pinned MAME release.

2. **Rust types + SQLite migration + bake-on-launch + Tauri
   commands.** New module `apps/oa-shell/src/system_info.rs` mirrors
   `game_info.rs` shape (`SystemInfo` / `SystemInfoOverride` /
   `MergedSystemInfo` / `merge_system_info`). SQLite migration vNN
   adds the four tables. Bake-on-launch logic + content-hash
   detection. Tauri commands: `get_system_info`,
   `get_system_info_override`, `set_system_info_override`,
   `delete_system_info_override`, `reset_system_info_to_default`.
   L2 YAMLs created from the 5 stub entries.

3. **Frontend cutover.** `SystemInfoPanel.tsx` + `HomePage.tsx`
   read the new `get_system_info` command. Delete
   `systemMetadataStubs.ts`. Drop the dropped fields (Input
   Latency, Emulator Core) from the panel rendering. Add Refresh
   Rate row.

4. **Per-system drill-in edit UI.** New "System info" section in
   `PerSystemSettingsBody.tsx`. Form rendering + per-field
   provenance badges + Reset All Overrides button.

5. **Re-import UI.** New row in SETTINGS → Storage category.
   MAME-detection logic (path probing + folder picker). Progress +
   per-system failure toast.

6. **Docs + ROADMAP + NEXT.md + SCHEMA.md + About credits.** Doc
   tick on this plan ("v1 shipped"). New section in
   `docs/cores/SCHEMA.md` for the `system-info.yaml` schema. New
   inventory entry in `docs/NEXT.md`. Credit line in About →
   Credits card.

### Acceptance per phase

`cargo test --workspace` green; `npm run typecheck` silent. New
Rust tests for the merge function (per-field precedence + dropped
fields + empty-record cases) follow the Game Info Panel v1 test
shape.

---

## 9. Attribution + licensing

MAME is BSD-3-Clause; `-listxml` output and `history.dat` are MAME's
own data and redistributable under the same terms.

Add a line to `AboutSettings` → "Credits" card:

> System metadata derived in part from MAME (BSD-3-Clause) —
> `-listxml` machine data + `history.dat`.

The slim MAME source files in `assets/mame-source/` get a header
comment pointing at the upstream license file (`assets/mame-source/LICENSE`
or similar).

---

## 10. Known limitations / v2 candidates

- **Re-import is session-scoped.** Operator's re-imported L1 gets
  overwritten on next OA release. v2 candidate: per-row provenance
  tracking so re-imports survive OA updates.
- **L1 is bundled-only at read time.** Once baked into SQLite, the
  bundled slim files are the source of truth until OA's next
  release or an operator re-import. v2 candidate: scheduled
  refresh from a separate `overlooked-arcade-system-info` repo
  (matches the Game Info Panel v2 vision).
- **Edit UI is per-system only.** No bulk "edit all systems"
  surface. Probably never wanted.
- **Achievements stays a stub.** Real wiring is the RetroAchievements
  integration stream.
- **No L1 override path for the achievements card.** Achievements
  data conceptually belongs in a per-game aggregate, not per-system
  metadata.
- **`bump-mame.sh` requires MAME installed.** Maintainers without
  MAME can't bump the slim data. Mitigation: GitHub Actions
  workflow that runs `bump-mame.sh` against a known MAME release.

---

## 11. Cross-references

- `apps/oa-shell/src/game_info.rs` — the field-typed merge pattern
  + Rust struct shape this plan mirrors.
- `docs/PLANS/game-info-panel.md` — sibling per-game plan; same
  three-layer model.
- `docs/cores/SCHEMA.md` — `games-info.md` schema doc; will get a
  new sibling section for `system-info.yaml`.
- `frontend/src/routes/retroverse/SystemInfoPanel.tsx` — current
  consumer; will be rewired to the new Tauri command.
- `frontend/src/routes/retroverse/HomePage.tsx` — hero consumer;
  same rewire.
- `frontend/src/routes/retroverse/systemMetadataStubs.ts` —
  deleted in phase 3.
- `frontend/src/routes/retroverse/PerSystemSettingsBody.tsx` —
  gains a "System info" section in phase 4.
- `frontend/src/components/SettingsSections.tsx` — `StorageSettings`
  gains the re-import button in phase 5.
