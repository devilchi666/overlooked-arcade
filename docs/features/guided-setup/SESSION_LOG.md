# Guided Setup — Session Log

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
