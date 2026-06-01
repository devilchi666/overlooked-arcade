# Guided Setup — Session Log

## 2026-06-01 — Phase 1B Slice 2: per-ROM results table

Merged to main 2026-06-01 (`--no-ff` from `feat/guided-setup-phase-1b-slice-2`,
merge `04fa975`). One phase commit (`f5e2527`) replacing the wizard's
Step 2 (extension→system mapping editor) + Step 3 (progress + per-system
tally) with a single LaunchBox-inspired per-ROM table that consumes the
Slice 1 backend payload.

- **Shipped:**
  - **Phase 1 — per-ROM results table (`f5e2527`):** new
    `frontend/src/components/import-wizard/ResultsTable.tsx` (~700
    lines) — virtualized table via `@tanstack/solid-virtual` (mirrors
    `DetailListView` pattern). Columns: checkbox / file / detected
    system / suggested title / confidence / status / skip toggle.
    Confidence badge via new `CONFIDENCE_PILL_STYLES` record (mirrors
    `BIOS_PILL_STYLES`). Status derived per row (Ready / Needs
    system / Skipped). Inline-edit for system + title via
    ViewsManagerTab signal-pair pattern (per-cell `editingCell` +
    `editDrafts`; commit on blur/Enter; revert on Escape;
    in-progress draft survives virtualization remount via
    keyed signal). Click-to-sort headers with ▲/▼ glyph (keys:
    fileName, systemId, title, confidence rank, status rank).
    Filter input (case-insensitive substring on fileName + title)
    + "Show skipped" toggle. Bulk-select with tri-state header
    checkbox and Gmail-style appearing toolbar (`N selected ·
    Change system ▾ · Skip · Unskip · Clear`). Row-level
    controller-nav focus group (DPad UP/DOWN walks rows; A toggles
    selection). Show-path popover via ⓘ icon button. Wizard
    integration: Step type `1|2|3|4 → 1|2|3`; STEP_LABELS
    `{Folder / Review / Confirm}`; step-indicator render +
    counter both flipped; new `tableRows` signal + `createEffect`
    builds rows from `scanRows` preferring backend `system_id` /
    `suggestedTitle` / `confidence` / `sha1` from Slice 1 (with
    `classifyScanRow` + `titleFromFileName` as fallback);
    `onRowChange` + `onBulkChange` handlers; new
    `commitRowsToEntries()` replaces `bucketScanned()` honoring
    per-row overrides; `needsSystemCount()` drives an inline
    warning banner above the table and the Next button's
    disabled state; old `bucketTally` + `unmatchedTally` gone;
    auto-start scan effect retargets `step()===3 → step()===2`;
    mapping-editor body extracted into `MappingRulesAdvanced`
    helper rendered inside `<details>` "Advanced — extension
    overrides" below the table (persistent per-folder rules
    editor stays available for power users); ScummVM detect
    banner moves into the expander; footer button branches
    re-routed (Step 2 = Cancel scan / Rescan / Next; Step 3 =
    Skip sync / Import + sync). Frontend `npm run typecheck`
    silent.

- **Almost:** Operator playtest. The table is now the primary UI
  consumer of the Slice 1 smart-scan emission — confidence badges
  should light up reliably for known-canonical ROMs (Hash green,
  Header cyan), CD-shape stays Ext muted, Neo Geo .zips Hint
  amber. Per-row Change-system + Edit-title + Skip apply at
  commit via `commitRowsToEntries()`.

- **Next:** Slice 3 — per-system readiness checklist. Per plan
  §5 Step 5: one row per system found in this scan, status
  pills for Core installed / BIOS present / Default bindings
  ready / Core options pre-tuned / Per-game overrides from
  KNOWN_GAME_BUGS. Same component reused as Settings →
  Library → System Readiness (a second SettingsCard alongside
  "Re-scan with smart detection"). Wizard structure expands
  back to 4 steps with the checklist between Review (Step 2)
  and Confirm (now Step 4). Estimated 1-2 weeks.

## 2026-06-01 — Phase 1B Slice 1: smart-scan emission + Settings entry point

Merged to main 2026-06-01 (`--no-ff` from `feat/guided-setup-phase-1b`,
merge `5ef8062`). Three phase commits + a doc-only fixup closing
Slice 1 of the wizard-upgrade phase. The plan lives off-tree at
`C:\Users\Devilchi\.claude\plans\spicy-shimmying-crescent.md`.

- **Shipped:**
  - **Phase 1 — Backend smart-scan emission (`14f267d`):** new
    `apps/oa-shell/src/title_clean.rs` ports `titleFromFileName` to
    Rust (3 lines; 9 unit tests). `ScannedRom` grows four optional
    fields (`system_id` / `suggested_title` / `confidence` / `sha1`)
    + new `Confidence` enum (`Hash` / `Header` / `Extension` /
    `Hint`). New `apply_smart_classification` runs after the
    directory walk: classifies via `system_hint` → `extension_to_system`
    map; for non-CD cart rows in a system with a populated `rom_hashes`
    table, loads bytes via `rom_bytes_for`, computes header-aware
    SHA-1 candidates via `candidate_sha1s`, looks up against
    `LibraryDb::lookup_rom_hash`. Hash hits override `system_id` to
    the canonical row's value and replace the suggested title with
    `game_name` from libretro-database. Rayon-parallel hash pass
    with a CPU-capped (≤4) pool mirroring
    `resolve_rom_hashes_for_system`'s pre-hash pattern. Pure helpers
    `classify_row` + `confidence_from_rule` extracted for unit
    testing (8 new scan_service tests). `rom_hashes.rs` extracted
    `auto_sync_rom_hashes_if_empty` as a `pub(crate)` helper out of
    `resolve_rom_hashes_for_system`'s inline block;
    `start_background_scan` calls it pre-spawn for every distinct
    system in `extensionToSystem` so first-time operators light up
    the Hash tier on their first scan. `start_background_scan` +
    `start_background_directory_scan` dispatchers grow the
    `extensionToSystem` parameter + `app.state::<LibraryDb>()` borrow
    inside `spawn_blocking`. 615 oa-shell tests green (was 598; +17
    from title_clean + scan_service helpers).
  - **Phase 2 — Frontend smart-scan plumbing (`6803d08`):** TS
    `ScannedRom` extended with the four new fields + `ScanConfidence`
    union. `runBackgroundScan` grows an `extensionToSystem`
    parameter accepting either `Map<string, SystemId>` or
    `Record<string, string>` (Maps don't serialize through Tauri
    invoke as plain objects → `Object.fromEntries`).
    `ingestFolderPath` + `rescanFolders` pass `coreSystemMap` from
    `resolveScannableExtensions` directly through.
    `ImportWizard.startScan` builds the map from its `ruleMap()`
    output. Frontend `npm run typecheck` silent.
  - **Phase 3 — Settings → Library entry point (`df49a8c`):**
    re-establishes a path to the orphaned wizard. The 2026-05-31
    legacy-Shell deletion removed the toolbar that previously
    opened it; the wizard component stayed mounted in
    `App.tsx:1544` with nothing flipping `wizardOpen` to true.
    `RetroverseContextValue` grows `onOpenImportWizard: () =>
    void`; App.tsx wires `() => setWizardOpen(true)`;
    `LibrarySettings` prepends a new `SettingsCard` titled
    "Re-scan with smart detection" with curator-voice copy per
    plan §4 and a primary button calling
    `ctx.onOpenImportWizard()`. Matches plan §12 IA exactly:
    `Settings → Library → "Re-scan with smart detection" ← NEW:
    opens the wizard`.
  - **Fixup — drag-drop reference sweep (`74f6cef`):** three
    docstrings I introduced cited drag-drop as a live ingest path;
    drag-drop was decided Won't fix on 2026-05-20
    (`docs/PARKING_LOT.md`). `onDragDropEvent` listener stays wired
    but isn't operator-facing. Swept references in `App.tsx`,
    `ImportWizard.tsx`, `library/ingest.ts` to name the actual
    live paths (Settings → Library → Re-scan, Settings → Library →
    Add folder, LibraryView empty-state button).

- **Almost:** Operator playtest of the smart-scan emission. UI
  doesn't surface the new fields yet (Slice 2's per-ROM table is
  the first reader); verification needs reading
  `<data_dir>/logs/oa-current.log` for the `oa://library-scan-complete`
  payload shape. Plan verification steps 4-8 cover the lookups
  (auto-sync triggers on fresh DB, hash hits, header-stripped hits,
  CD-shape stays Extension).

- **Next:** Slice 2 — per-ROM results table inside the wizard.
  Replaces the Step 2 mapping editor + Step 3 progress-only view
  with a single table showing one row per scanned ROM (File /
  Detected system / Suggested title / Confidence / Status). v1
  per-row actions: Change system / Edit title / Skip / Show path.
  Bulk-select via checkboxes. Sort/filter by column. Consumes the
  Slice 1 backend payload directly. Estimated 1-2 weeks.

**Plan note:** `C:\Users\Devilchi\.claude\plans\spicy-shimmying-crescent.md`
holds the Slice 1 design notes (scope, hashing strategy, replace-in-
place decision, verification recipes). If Slice 2 surfaces any
non-obvious calls that aren't already captured in the plan, fold
them into a new `DECISIONS.md` in this folder.
