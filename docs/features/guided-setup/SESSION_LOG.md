# Guided Setup — Session Log

## 2026-06-01 — Phase 1B CLOSED — Slice 6: voice/tone copy pass + first-launch hero

Merged to main 2026-06-01 (`--no-ff` from `feat/guided-setup-phase-1b-slice-6`,
merge `bf77117`). One phase commit (`bbc649a`). **Closes Phase 1B —
the entire wizard-upgrade arc is now feature-complete after six
slices shipped today.**

- **Shipped:**
  - **Phase 1 — voice/tone copy pass + first-launch hero (`bbc649a`):**
    Targeted-tone pass per operator decision — ~15 string rewrites
    out of ~70 enumerated across the six surfaces. Universal
    affordance labels (column headers, button labels like Close /
    Refresh, pill labels like "✓ Core installed") stayed put because
    they're already operator-friendly and stretching them risks
    over-cute affordances. The plan file (off-tree) carries the
    per-string table as the single source of truth.

    String rewrites: `ImportWizard.tsx` (Step 1 prompt → "Where are
    your games today?", Step 4 confirm leads with operator validation,
    sync-options header + metadata toggle, empty-scan message,
    needs-system banner with conditional singular/plural);
    `ResultsTable.tsx` (filtered-empty message warmed);
    `SystemReadinessChecklist.tsx` (Core options NA detail,
    two Core-pill details, top-of-list banner heading);
    `BiosResolutionDetail.tsx` (detail intro, per-row install hint);
    `biosHints.ts::DEFAULT_HINT`; `SettingsSections.tsx::LibrarySettings`
    (Card 1 title + button, Card 2 description).

    First-launch hero: `LibraryView::EmptyState` `!hasSeed` branch
    replaced with system-accent ◐ glyph + "Welcome to Overlooked
    Arcade" text-3xl heading + plan §5 Step 0 body copy verbatim +
    "Set up your library" primary CTA → `ctx.onOpenImportWizard()`.
    Muted secondary "Or pick a folder the quick way" link preserves
    the legacy `props.onPickFolder()` path one click away. Drag-drop
    body-copy reference dropped — drag-drop is parking-lotted
    Won't fix per `docs/PARKING_LOT.md` 2026-05-20 and the
    `feedback-code-exists-isnt-live` memory. New REQUIRED
    `onImportWizard: () => void` prop on `LibraryView` (compile
    error if forgotten); `LibraryPage` wires from
    `ctx.onOpenImportWizard()`. `hasSeed` branch keeps the compact
    treatment (operator past first launch).

  Frontend `npm run typecheck` silent. No backend changes; 615
  oa-shell tests stay green.

- **Almost:** Operator playtest of the hero treatment + the warmed
  copy across the wizard / readiness / Settings surfaces.

- **Next:** **Phase 2 — curated CPU-tier core selection** per plan
  §13 Phase 2 (separate from Phase 1B). New `sysinfo` crate
  integration for CPU detection (brand, base clock, physical cores
  → High/Mid/Low tier bucket); per-system tier preference table
  declaring which core to default to per tier (e.g. `psx.high =
  beetle_psx_hw`, `psx.mid = duckstation`, `psx.low = pcsx_rearmed`);
  the tier feeds into existing `cores.json` per-system default
  selection at install-time and gets surfaced on the readiness
  checklist row ("Picked: beetle_psx_hw — high-tier core"). New
  Settings → Performance → CPU tier (Auto / High / Mid / Low
  override). Estimated ~1 week. Awaiting fresh operator green-light
  to start (Phase 1B closure is a natural pause point — operator
  may want to play with the shipped guided setup before kicking off
  the next arc).

## Phase 1B summary — six slices, one day

| Slice | Shipped | Highlights |
| --- | --- | --- |
| 1 | `5ef8062` | Backend smart-scan emission (Confidence / sha1 / suggested_title) + Settings → Library entry point re-wiring the orphan wizard |
| 2 | `04fa975` | Per-ROM results table (virtualized via `@tanstack/solid-virtual`); inline edit; bulk-select; sort + filter; mapping editor collapsed to Advanced expander |
| 3 | `b57f3e7` | Per-system readiness checklist component (wizard Step 3 + Settings card); 5 pills per system; `open_bios_folder` action |
| 4 | `923ea7b` | `MissingCoreBulkPrompt` modal + `has_core_options_schema` Tauri command; banner/modal source-of-truth alignment + CATALOG slug realignment fixes |
| 5 | `e3092b8` | `BiosCheck` deep refactor (18 per-system check functions → 2-line helper calls); `install_bios_file` Tauri command; `BiosResolutionDetail` with per-file rows; window-focus refetch follow-up |
| 6 | `bf77117` | Targeted voice/tone pass; first-launch hero in `LibraryView::EmptyState`; arc closure |

Per-phase ship totals: ~1,800 lines of new code (Slices 2 + 5 the
heaviest); 615 oa-shell tests stayed green throughout; frontend
`npm run typecheck` stayed silent throughout. Phase 1B's wizard
upgrade is feature-complete and ready for end-to-end operator
validation against a real ROM library.

## 2026-06-01 — Phase 1B Slice 5: guided BIOS resolution + window-focus refetch

Merged to main 2026-06-01 (`--no-ff` from `feat/guided-setup-phase-1b-slice-5`,
merge `e3092b8`). Two phase commits — main slice + operator-playtest
follow-up.

- **Shipped:**
  - **Phase 1 — BIOS resolution refactor + Pick-BIOS-file picker (`d2d82c8`):**
    Deep refactor of `BiosCheck` enum + 18 per-system `check_*_bios`
    functions. New types: `BiosFile` / `BiosFileStatus` /
    `BiosOverallVerdict` / `BiosSemantics`. New helpers replacing
    inline scan-and-classify across the 18 functions:
    `sha1_hex_upper`, `scan_bios_table` (walks the const hash table +
    builds per-file inventory), `derive_bios_overall` (translates
    inventory + AnyOf/AllRequired semantics into the verdict),
    `bios_check_from_inventory` (wraps verdict into the appropriate
    BiosCheck variant), `exe_system_dir` (centralizes
    `<exe_dir>/system/` derivation that was duplicated three places).
    Most check functions go from ~25 LOC to 2 lines. ChannelF flags
    `sl90025.bin.optional=true` post-scan so the Channel F II
    revision file is hash-checked when present but doesn't gate the
    launch pair on absence. Neo Geo cart keeps its bespoke zip-
    introspection path wrapped in a single-entry inventory.
    BiosStatusEntry grows `files: Vec<BiosFile>` so the frontend gets
    structured per-file detail. New `install_bios_file` Tauri command:
    reads operator-chosen source path → SHA-1 → looks up canonical
    via new `known_hashes_for_system` dispatcher → atomic `.partial`
    swap into `<exe_dir>/system/`. WARN semantics per operator
    decision (copy regardless of hash; pill flips to ⚠ "unknown hash"
    if mismatch). Frontend: new
    `frontend/src/components/import-wizard/BiosResolutionDetail.tsx`
    with per-file rows + status badges + click-to-pick affordance via
    `@tauri-apps/plugin-dialog`'s file picker → invokes
    `install_bios_file` → triggers `onInstalled` callback. Computed
    SHA-1 displayed alongside expected SHA-1 on Unknown hash.
    "Where to get it" collapsible expander pulling from new
    `biosHints.ts` stub map (operator-driven content over time).
    `SystemReadinessChecklist` auto-expands the detail inline below
    the existing action-buttons row when `biosEntry().status !== "ok"`.
    "Open BIOS folder" stays as the escape hatch.
  - **Phase 2 — focus + manual Refresh refetch (`719112d`):**
    operator playtest caught that manually dropping BIOS files into
    `<exe_dir>/system/` via the OS file manager (or dropping cores
    into `/cores/`) didn't update the readiness pills. The
    `bios` / `cores` / `available` / per-system `optionsBySystem`
    resources only refetched on mount + on download-progress
    `phase=done` events; manual filesystem changes had no signal.
    Two complementary fixes: (a) `window.addEventListener("focus", …)`
    refetches all four resources when OA regains OS focus
    (operator clicks "Open BIOS folder", drops files via File
    Explorer, switches back → pill flips live), per the saved
    `reference_tauri_dom_focus_reliable` memory. (b) Manual
    "Refresh" button next to the BIOS folder path as a backup
    affordance for cases where focus doesn't trigger or when
    operator wants explicit control. Both surfaces (wizard Step 3 +
    Settings → Library card) benefit equally — they share the
    component.

  No drag-drop — per `docs/PARKING_LOT.md` 2026-05-20 won't-fix and
  the `feedback-code-exists-isnt-live` memory; per-file picker
  covers the operator-facing install affordance. 615 oa-shell
  tests stay green throughout. Frontend `npm run typecheck` silent.

- **Almost:** Operator playtest of the BIOS picker flow (both
  canonical-match and unknown-hash paths) + the focus-refetch
  behavior. Plus the ChannelF optional file ↪ semantics with a
  real Channel F II ROM in hand.

- **Next:** Slice 6 — voice/tone copy pass + first-launch empty-
  state entry point. CLOSES Phase 1B. Per plan §4 voice card, every
  user-facing string in the wizard + readiness checklist + bulk-
  install modal + BIOS resolution detail gets reviewed against the
  warm/curator-enthusiast tone ("Found 240 games across 12 systems.
  Quite a collection — let's get them ready.") vs the dry default
  ("240 files scanned, 12 systems detected."). Per plan §5 Step 0,
  the first-launch empty-state lands: OA detects "no library
  configured" on first launch and shows a friendly hero with a
  single "Set up your library" button instead of dumping operators
  straight into the empty library view. Estimated 3-4 days.

## 2026-06-01 — Phase 1B Slice 4: bulk missing-core download + Core options pill + catalog slug realignment

Merged to main 2026-06-01 (`--no-ff` from `feat/guided-setup-phase-1b-slice-4`,
merge `923ea7b`). Three phase commits — the main slice + two operator-
playtest fixes for divergences caught during shipping.

- **Shipped:**
  - **Phase 1 — bulk-install modal + Core options pill (`4f46007`):**
    new `frontend/src/components/import-wizard/MissingCoreBulkPrompt.tsx`
    (~410 lines). Lists one row per system needing a core, with a
    recommended-core dropdown when multiple candidates exist;
    per-row checkbox; live progress bar via the existing
    `oa://core-download-progress` event channel; concurrent
    downloads via parallel `download_core(base)` invokes (the Rust
    side's `.partial` swap means /cores/ writes don't trample).
    New `has_core_options_schema(systemId)` Tauri command in
    `core_options.rs` (~15 LOC) wrapping the existing `read()`
    helper; checklist swaps the placeholder Core options pill to
    real status (✓ when populated, ↪ "Schema populates on first
    game launch" when empty). SystemReadinessChecklist adds a
    top-of-list "Install N missing cores…" banner-button visible
    when ≥1 system has ⚠ Core; per-row "Install core…" stub
    rewired to open the modal scoped to that single system.
    Subscribed to download-progress on phase=done → `refetchCores`
    so the pill flips in real time.
  - **Phase 2 — banner/modal source-of-truth alignment (`47b42d2`):**
    operator-reported "install 1 missing core" → "no missing cores"
    on click. Root cause: banner used extension-overlap heuristic
    (does any installed core's validExtensions cover the system's?),
    modal used catalog-membership (CATALOG entries with
    `systems.includes(systemId) && recommended && !installed`).
    Two heuristics could disagree (systems with no CATALOG entries,
    only non-recommended candidates, or aliased slugs where
    another already-installed core covers extensions). Refactored
    `SystemReadinessChecklist` to fetch both `list_cores` AND
    `available_cores` (the same catalog source the modal uses);
    hybrid `coreInstalledFor` does catalog-check first, extension-
    overlap fallback. Added `catalogHasEntry()` helper. Core pill
    grew a third state ↪ "No catalog core" + "Install manually via
    Settings → Cores" detail, distinct from ⚠ "No core". Modal
    loosened filter to include non-recommended candidates with
    recommended-first sort.
  - **Phase 3 — CATALOG slug realignment (`46c28ed`):** operator
    asked "no catalog core for atari 2600? which other systems?".
    Investigation found 12 systems showing the ↪ fallback — 8 slug
    mismatches between Rust CATALOG and frontend registry, 3
    genuinely-missing rows (jagcd / sega32xcd / stv added
    2026-05-27), 1 entirely missing (3do never had an entry).
    Renamed slugs: `atari2600 → 2600`, `atari5200 → 5200`,
    `gameboy → [gb, gbc]`, `gba + gameboy → [gba, gb, gbc]`,
    `intellivision → intv`, `odyssey2 → o2`, `neogeocd → neocd`,
    `dos → dosbox`. Extended existing entries: Virtual Jaguar
    gained `jagcd`; PicoDrive gained `sega32xcd`; all 5 MAME family
    entries gained `stv`; FBNeo main + FB Alpha 2012 Neo Geo
    gained `neogeo`. New `opera_libretro` entry for 3DO. Every
    registry slug now has at least one CATALOG entry; the ↪
    fallback is now defensive code that shouldn't fire for any
    onboarded system.

- **Almost:** Operator playtest of the realigned catalog. Should
  see ✓ or ⚠ on every Core pill (no ↪ "No catalog core" anywhere)
  for a typical 5-15-system library. Bulk-install banner count
  should equal modal row count exactly.

- **Next:** Slice 5 — guided BIOS resolution. Today the ⚠ BIOS
  pill has an "Open BIOS folder" button (lands the operator at
  `<exe_dir>/system/`). Slice 5 expands that to a richer surface:
  per-BIOS filename + SHA-1 hash + "where to get it" hint
  (operator-supplied list of legal sources per-system), with a
  "Pick BIOS file…" per-file picker button (folder-picker via
  `@tauri-apps/plugin-dialog`) that copies the chosen file into
  `<exe_dir>/system/` after filename + hash verification. The
  existing per-system BIOS check helpers in
  `apps/oa-shell/src/main.rs::get_bios_status` already return
  filename + required string per entry — Slice 5 is mostly UI
  surfacing what's already in the response. Estimated 1 week.
  (External drag-drop is parking-lotted Won't fix per
  `docs/PARKING_LOT.md` — drop targets aren't an OA pattern.)

## 2026-06-01 — Phase 1B Slice 3: per-system readiness checklist

Merged to main 2026-06-01 (`--no-ff` from `feat/guided-setup-phase-1b-slice-3`,
merge `b57f3e7`). One phase commit (`2020b4e`) inserting a readiness
checklist between the per-ROM table (Slice 2) and Confirm. Same
component lives in two surfaces: new wizard Step 3 + a second
`SettingsCard` in Settings → Library alongside the existing
"Re-scan with smart detection" card.

- **Shipped:**
  - **Phase 1 — readiness checklist component + Open BIOS folder
    action (`2020b4e`):** new
    `frontend/src/components/import-wizard/SystemReadinessChecklist.tsx`
    (~300 lines). Props: `systems: Accessor<SystemId[]>` +
    `emptyStateLabel?` for surface-specific empty copy. Fetches
    `list_cores` + `get_bios_status` once on mount via
    `createResource`. Per-row rendering uses `data-system="<id>"`
    so each row picks up its theme accent via the `systems.css`
    CSS cascade; auto-fit grid of 5 pills (Core / BIOS / Bindings
    + 2 placeholder pills `— Coming Slice 4`); inline action
    buttons rendered when a row has any ⚠ state. Pill colors via
    a `PILL_STYLES` record mirroring `BIOS_PILL_STYLES` (`ready` /
    `warning` / `na` / `coming`). Core check uses a cheap
    `list_cores().validExtensions ∩ systemThemes[id].extensions`
    intersection (any installed core that handles the system's
    extensions counts as ✓); a stricter "operator's preferred
    default core for this system is installed" check defers to a
    polish pass (needs the cores.json registry, not in scope).
    BIOS check is per-system via `entries.find(slug)`; absence
    from the response = "not required" (BIOS-required system list
    is curated in `main.rs::get_bios_status`). Bindings pill is
    always ✓ for any registered SystemId since `bindings.rs`
    dispatch covers all 45 onboarded systems. New
    `apps/oa-shell/src/main.rs::open_bios_folder` Tauri command
    mirrors `open_video_clip_folder`'s cross-platform spawn
    pattern (`explorer` / `open` / `xdg-open`); auto-creates
    `<exe_dir>/system/` if missing so first-run operators don't
    hit an error. Wizard integration: `Step` type `1|2|3 →
    1|2|3|4`; `STEP_LABELS` `Folder / Review / Readiness /
    Confirm`; step-indicator + header counter back to 4. New
    `Step3` body renders `<SystemReadinessChecklist>` sourcing
    via new `readinessSystems` memo (unique systemIds from
    `commitRowsToEntries()`); old Step3 (sync toggles) renamed to
    Step4; footer button branches re-routed (Step 2 Next → 3;
    Step 3 Next → 4 disabled when `commitRowsToEntries()` is
    empty; Step 4 keeps the existing Skip-sync / Import + sync
    pair). Settings → Library wired: new "System readiness"
    `SettingsCard` between the existing "Re-scan with smart
    detection" card and the embedded `LibraryManagerPage`;
    `librarySystems` memo derives unique systemIds from
    `ctx.library.state.entries` so the card reactively re-renders
    on library mutations. 615 oa-shell tests green; frontend
    `npm run typecheck` silent.

- **Almost:** Operator playtest. The Install-core stub currently
  dispatches a `window.CustomEvent("oa://readiness-stub-toast")`
  with no UI listener yet — operator can't see the toast unless
  reading devtools console. Slice 4 wires the real toast +
  `core_installer.rs` integration on top of that event channel.

- **Next:** Slice 4 — bulk-prompt missing-core download. Wires
  `apps/oa-shell/src/core_installer.rs` to the Install-core stub
  via a real bulk-prompt UI ("Download these N cores from
  libretro buildbot? [12 MB]"). Same slice can swap the
  `— Coming Slice 4` Core options pill into real per-system
  status by reading the `core_options.rs` catalog (likely needs
  a new Tauri command since the existing `read()` is module-
  private). Estimated 1 week.

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
