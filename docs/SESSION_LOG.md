# Session Log

Project-wide milestone log. Per-core day-to-day work goes in `docs/cores/<core>/SESSION_LOG.md`. This file is for cross-cutting milestones (phase boundaries, shell-level shipped features, new systems coming online).

Format: date + three lines — **Shipped / Almost / Next**.

---

## 2026-06-05 — Dynamic controller-info arc — light guns + peripherals across every core

Branch `feat/dynamic-controller-info` — 5 commits closing the
hardcoded-device-id-table era. Cores publish their per-port supported
devices via `RETRO_ENVIRONMENT_SET_CONTROLLER_INFO`; the frontend
dropdown now reads each core's authored advertisement directly instead
of shipping a `Light Gun = 4` row that doesn't match FCEUmm's Zapper
(258), snes9x's Super Scope (260), Genesis Plus GX's Light Phaser
(260), Beetle PSX's GunCon (260), or any other core that subclasses.
Plan in `docs/PLANS/dynamic-controller-info.md`. Triggered by a Duck
Hunt validation attempt that exposed the wrong-device-id bug; chosen
over a per-system band-aid per `feedback_no_bandaid_fixes`.

- **Shipped:** Slice 1 — `crates/oa-libretro/src/state.rs::parse_controller_info` walks the null-terminated `retro_controller_info` array, clones strings to owned `String` (decouples from .dll text-segment lifetime), stores per-port `Vec<DeviceDescriptor>` in singleton state; `LibretroCore::controller_devices(port)` + free `loaded_core_controller_devices(port)` accessors; 7 parser tests covering null top pointer / sentinel walking / empty-port / NULL desc / string ownership / subclass-id preservation / log-formatter truncation. Slice 2 — `get_controller_devices` Tauri command; `GameInputDialog`'s `createResource` fetches all 5 ports' lists on open and refactors both port-0 and per-port-1-4 dropdowns to render the live advertisement. Slice 2.5 — Input dialog added to QuickSettings (Esc overlay) so the operator doesn't have to exit-configure-relaunch. Slice 3 — schema v20→v21 `core_controller_info(core_filename, port, devices_json, captured_at, core_mtime)`; populated on every core load; mtime-invalidated; Tauri command falls through to cache when no core is live; cache round-trip + mtime-invalidation + per-core-isolation tests. Slice 4 — `DEVICE_ID_OPTIONS_BASE` + `_GAMECUBE` + `_SNES` + `systemSpecificDeviceLabel` + `deviceOptionsForSystem` + `deviceOptionLabel` deleted; `LIGHT_GUN_SYSTEM_IDS` hand-list + `isLightGunSystem(systemId)` deleted; replaced by `system_has_light_gun` Tauri command that derives from the cache via two patterns (`id & 0xFF == LIGHTGUN` OR label keyword); legacy-id label for stale saves; `LightGunMappingHelp` gate switched from system-list to per-selected-device heuristic. Slice 5 — Duck Hunt validated by operator on FCEUmm; NES ROADMAP Zapper bullet flipped ⬜→✅; SMS / PSX / Saturn / Dreamcast bullets updated to reflect arc-shipped wiring (operator validation still pending per system).
- **Almost:** Hogan's Alley + Wild Gunman as smoke tests; SMS Light Phaser via Genesis Plus GX as the easiest second-system validation (same code path, no per-system work needed).
- **Next:** Per-core operator validation of remaining light-gun systems (SMS / SNES / PSX / Saturn / Dreamcast). The hard work is done — each one is a `launch → Esc → Input → pick the gun → fire` test loop.

---

## 2026-06-03 — Settings declutter arc — merged to main

Branch `feat/settings-declutter-system-health` merged via `--no-ff`
(merge `dd430e4`) after operator playtest signed off. Branch deleted
local + remote. The 5 phase commits + 2 polish commits + 2 docs
commits all carry forward.

---

## 2026-06-03 — Settings declutter arc — System Health hub + Game-media cards

Six phase commits on `feat/settings-declutter-system-health` shipping
the SETTINGS-page declutter the operator asked for: System Readiness
gets its own home, BIOS / Cores / Storage / Background Jobs absorb
into a new System Health hub with an internal tab strip, and the
Game-media tab's 225-button per-system grid collapses to a status-
first card grid + a per-system Manage… side panel.

- **Shipped (Phase 1, `21d803d` — System Health scaffolding):**
  New `system-health` category in the SETTINGS sidebar's SYSTEM
  group. Four previously-standalone categories (BIOS / Cores /
  Storage / Background Jobs) absorbed as internal tabs of the new
  category's body; sidebar shrinks from 16 entries to 12. New
  `frontend/src/routes/retroverse/SystemHealthPage.tsx` with the
  tab strip (Overview / BIOS / Cores / Storage / Jobs). Tab bodies
  render the existing settings components verbatim — no behavior
  change inside each. System Readiness card removed from Settings →
  Library; lives in System Health → Overview now.

- **Shipped (Phase 2, `9bfbc96` — Overview rollup cards):** The
  Overview tab gets a live status rollup with 5 horizontal rows
  (Cores · BIOS · Library readiness · Background jobs · Storage).
  Each row computes a green/amber/red dot + one-line summary against
  existing Tauri commands (list_cores / available_cores /
  get_bios_status / get_job_prefs + activeJobs() /
  get_system_status), and a CTA button deep-links the parent tab
  strip into the relevant deep-dive tab. Per-system readiness
  checklist (lifted in Phase 1) renders below the rollup grid.

- **Shipped (Phase 3, `cb33089` — Dev test relocation):** The
  "Background Jobs — test spawner" affordance moves out of
  Settings → Library (where it sat as visible clutter for every
  operator) into a collapsed-by-default disclosure inside
  Experimental → Dev tools. Same `invoke("spawn_test_job", …)`
  under the hood; ordinary operators never see the buttons.

- **Shipped (Phase 4, `910ba27` — Game media card grid + Freshen):**
  Replaces the per-system action row layout (45 systems × 5 buttons =
  ~225 buttons in one scroll) with a status-first card grid.
  Cards alphabetical, 3-col on xl / 2-col on md / 1-col on small.
  Per-card status rows (identified / covers / metadata) with ✓ / ⚠ /
  ✗ glyphs computed against library entries + MediaDb. Single
  [Freshen] button per card runs the smallest set of ops needed
  (Identify → Sync media → Sync metadata) routed through Background
  Jobs. Top-right [Freshen all systems] CTA chains every library
  system. Preferences (Only sync identified + Kinds to fetch +
  Region priority) hoisted into a top Preferences card with the
  region list collapsed in a details disclosure.

- **Shipped (Phase 5, `e087caf` — Manage side panel):** New
  `GameMediaManagePanel` slides in from the right when the operator
  clicks [Manage…] on a system card. Five op cards (Identify ROMs /
  Sync covers / Sync metadata / Clear metadata / Refresh hash
  database) with one-line descriptions + current counts + single
  [Run] buttons. Operator can switch active card while the panel is
  open; the panel re-targets without close + reopen. Esc + backdrop
  click + ✕ + Done all close. z-45 sits above page chrome and below
  modals / BackgroundJobsBar so the bar stays visible while the
  panel is open.

- **Tests:** 660 of 660 oa-shell tests green (no Rust touched).
  `npm run typecheck` silent across all five phase commits.

- **Almost:** operator playtest of the full arc on Phase 5 build —
  visual review of the new System Health page, the Game-media cards,
  and the Manage panel before merging.

- **Next:** Phase 6 — operator playtest + merge to main; `--no-ff`
  per the locked feature-branch workflow.

- **Plan:** [docs/PLANS/settings-declutter-system-health.md](PLANS/settings-declutter-system-health.md)
  (six phases; one back-and-forth Q&A round on 2026-06-03 locked the
  shape).

---

## 2026-06-02 — Per-system descriptor consolidation — Slice 2 (mass migration + L1 const deletion)

Second slice of the per-system descriptor consolidation arc, closing
the cycle that Slice 1 opened earlier the same day. Took ~2,750 LOC
out of the codebase + replaced it with 46 YAMLs under
`config/systems/<id>/`. Five phase commits on
`feat/per-system-descriptors-slice-2`.

- **Shipped (Phase A, `d4553d1` — migrator tool):**
  `tools/migrate-systems/` standalone Cargo project (own workspace
  per the `tools/mame-extractor/` pattern). Five regex-based parsers
  read OA's Rust sources (default_core_dll_for_system arms, every
  `*_BIOS_KNOWN_HASHES` const + per-system semantics derived from
  the check_*_bios bodies, CATALOG, libretro_dat_refs_for_system
  arms) and join them against an embedded `SYSTEM_THEMES` mirror of
  `frontend/src/themes/registry.ts::systemThemes` to emit the
  three-file YAML triple per system. CLI flags --check (diff against
  existing; exit 1 on drift), --dry-run (print to stdout), --systems
  (subset filter), --output-dir, --repo-root.

- **Shipped (Phase B, `d4e5b89` — mass emit):** ran the migrator
  to emit YAMLs for all 41 frontend-registered systems (3 already
  existed from Slice 1, rewritten by the migrator to the canonical
  shape). 46 system.yaml + 19 bios.yaml emitted; 2 games.yaml
  unchanged. Channel F's `sl90025.bin` hand-flagged with
  `optional: true` (only special-cased system — the migrator can't
  detect the post-scan `f.optional = true` adjustment that
  check_channelf_bios applies). Legacy docs/cores/{snes,nes,
  genesis}/system-info.yaml deleted (content embedded in
  config/systems/<id>/system.yaml::system_info). Test
  `load_curated_records_legacy_docs_cores_is_now_empty` asserts
  the legacy walk produces zero records; new
  `registry_load_finds_all_v1_panel_systems` covers the v1 lineup
  via the registry path.

- **Shipped (Phase C, `368b81c` — consumer shim sweep):** wired
  the 17 remaining check_*_bios functions (pce-cd / segacd /
  saturn / neocd / 3do / pcfx / dreamcast / ps2 / coleco / intv /
  o2 / channelf / 5200 / pokemini / gba / jaguar / jagcd) through
  the `check_bios_from_registry` shim. New
  `libretro_dat_refs_for_system_resolved` + 2 call site updates +
  new `default_core_dll_for_system_resolved` + 2 call site
  updates. Both resolved fns use a `Box::leak` + `OnceLock` cache
  to adapt the registry's owned `String` to the `&'static str`
  return type ~50 downstream callers expect (~2 KB lifetime leak
  bounded by `systems_count × ref_count`). Three new parametric
  tests:
  `all_bios_systems_via_registry_match_legacy_const`,
  `all_bios_systems_via_registry_have_expected_semantics`,
  `libretro_dat_refs_resolved_matches_legacy_for_all_systems`.

- **Shipped (Phase D — L1 const deletion, ~2,750 LOC removed):**
  the surgery. Deletions:
  - 19 `*_BIOS_KNOWN_HASHES` const tables in main.rs (~700 LOC
    total — the largest single chunk)
  - 45-arm `libretro_dat_refs_for_system` match in rom_hashes.rs
    (~315 LOC)
  - 41-arm `default_core_dll_for_system` match in main.rs (~315
    LOC)
  - `known_hashes_for_system` dispatcher (~29 LOC)
  - `scan_bios_table` helper (~33 LOC)
  - legacy `hash_l1_l2_inputs` in system_info.rs (~31 LOC)
  - `apps/oa-shell/src/light_gun_systems.rs` (~230 LOC) — reference
    table with no production consumer
  - `tools/migrate-systems/` (~1,277 LOC) — one-shot tool, mission
    accomplished
  - 5 const-matches-registry tests now invalid
  Simplifications: `check_bios_from_registry` returns Result
  directly; 19 `check_*_bios` functions become one-line wrappers;
  `check_channelf_bios` drops its post-scan optional flag
  adjustment (the flag lives in bios.yaml); resolved-variant
  helpers all drop their const fallbacks. Net source diff: +143
  lines simplifications / -1,387 lines source removed + 1,507 LOC
  deleted in standalone files = **~2,750 LOC net reduction.** Plan
  estimated ~1,800 LOC; we went further because the migrator
  tool's own 1,277 LOC also went away.

- **Shipped (Phase E — docs):** `docs/PLANS/per-system-descriptors.md`
  status flipped to "Slice 2 SHIPPED"; `docs/ACTIVE_WORK.md` +
  `docs/NEXT.md` updates; this SESSION_LOG entry.

- **Almost:** Operator playtest end-to-end on the registry-only
  state — `cargo tauri build` + smoke test that BIOS resolution
  + ROM hash lookup still works for a system NOT migrated in
  Slice 1 (e.g. NES / SNES / Genesis cart launch; Dreamcast disc
  launch; Coleco BIOS check). The behavioral tests are all green
  (646 oa-shell tests; was 615 pre-Slice-1 baseline) so the runtime
  paths are intact — but a Real Game Real BIOS playtest closes the
  arc cleanly.

- **Next:** Slice 3 — L3 content-packs layer +
  `<appDataDir>/content-packs/<pack>/systems/<id>/` deep-merge +
  schemars-generated JSON Schema for external validators + CI
  guard. Designed in the plan §"Slice 3"; ~1 week of focused work.
  Queued in [docs/NEXT.md](NEXT.md). Awaiting fresh operator
  green-light after the Slice 2 playtest pass.

---

## 2026-06-02 — Per-system descriptor consolidation — Slice 1 (pilot: GB + PSX + NDS)

First slice of the per-system descriptor consolidation arc planned
2026-06-01 ([docs/PLANS/per-system-descriptors.md](PLANS/per-system-descriptors.md)).
Replaces the ad-hoc scatter of per-system data across ~8 sources
(hardcoded Rust const tables for BIOS hashes, core catalog,
libretro-dat refs + in-tree `docs/cores/<id>/system-info.yaml` +
`games-info.md`) with a unified `config/systems/<id>/{system,bios,games}.yaml`
triple. Five phase commits on `feat/per-system-descriptors-slice-1`;
ends with 3 systems running off the registry and 38 unchanged
(prefer-registry, fall-back-to-const shim pattern). Operator-editable
YAMLs ship next to the binary so restart-to-reload works without a
recompile (Slice 2 Verification #3 requirement).

- **Shipped (Phase A, `0dd1e8c` — schema + loader):**
  `apps/oa-shell/src/system_descriptor.rs` + `system_registry.rs`
  scaffolding. serde-derived `SystemDescriptor` + `BiosDescriptor` +
  `GamesDescriptor` with `deny_unknown_fields` for loud typo errors.
  `SystemRegistry::load_from_in_tree` hot-fails on missing
  `system.yaml`, id-folder mismatch, embedded system_info id
  mismatch, malformed YAML, or duplicate id. `global_registry()`
  OnceLock singleton mirrors `game_info::global_index()` pattern.
  Resolver mirrors `system_info::resolve_docs_cores_dir` (exe-dir +
  source-tree fallback). 21 new tests (9 descriptor parser + 12
  registry loader).

- **Shipped (Phase B, `5544390` — GB pilot, no-BIOS shape):**
  `config/systems/gb/system.yaml` with the entire
  `docs/cores/gb/system-info.yaml` content embedded under the
  `system_info:` key. New `load_curated_records_with_registry` +
  `hash_l1_l2_inputs_with_registry` in `system_info.rs`;
  `bake_system_info_on_launch` constructs `SystemRegistry::load_default()`
  inline + calls the registry-aware variants. Legacy
  `docs/cores/gb/system-info.yaml` deleted. One new test —
  `registry_load_finds_gb_via_config_systems_path` — validates
  end-to-end + regression-guards against accidental docs/cores
  re-creation.

- **Shipped (Phase C, `edc6bc4` — PSX pilot, any_of BIOS):**
  `config/systems/psx/{system,bios,games}.yaml`. 18 candidate
  regional BIOS files with `semantics: any_of`; 2 seed game records
  (Tomb Raider + Final Fantasy VII) migrated from the deleted
  `games-info.md`. New `scan_bios_entries(system_dir, &[BiosFileEntry])`
  — owned-string mirror of `scan_bios_table` that propagates the
  `optional` flag. New `check_bios_from_registry(system_id,
  system_dir) -> Option<Result<BiosCheck, BiosError>>` — returns
  `Some(verdict)` when the registry has `bios.yaml`, `None` when the
  caller should fall through to its const. `check_psx_bios` +
  `install_bios_file` (via new `is_canonical_bios_hash`) +
  `GameInfoIndex::load_default` (via new `add_records`) all consume
  the registry first. Legacy `docs/cores/psx/system-info.yaml` +
  `games-info.md` deleted. Three new tests: bios.yaml set-equivalence
  with `PSX_BIOS_KNOWN_HASHES`, end-to-end check_psx_bios via
  registry, registry-load finds psx via config/systems.

- **Shipped (Phase D, `e01d851` — NDS pilot, all_required BIOS):**
  `config/systems/nds/{system,bios,games}.yaml`. 3 required BIOS
  files (bios7.bin + bios9.bin + firmware.bin) with `semantics:
  all_required`; 3 seed game records (Phantom Hourglass + Brain Age
  + Trauma Center) each carrying `touch_hotspots` inline. NDS had
  no pre-existing `docs/cores/nds/system-info.yaml` so no embedded
  `system_info:` block (operator can hand-author when ready).
  `check_nds_bios` wired through the same `check_bios_from_registry`
  shim as PSX. Three new tests: nds bios set-equivalence, partial-
  present returns Missing (validates all_required propagation),
  NDS records (with touch_hotspots) surface via registry merge.

- **Shipped (Phase E — docs + verification):**
  `docs/PLANS/per-system-descriptors.md` flipped to "Slice 1 SHIPPED";
  `docs/ACTIVE_WORK.md` adds the Recently-completed entry;
  `docs/NEXT.md` strikes Slice 1 from HIGH band + queues Slice 2 as
  the next slice. This SESSION_LOG entry.

- **Architecture decision (resolved during Slice 1):** Sibling
  `config/` folder + source-tree fallback for the YAML location,
  rejecting `include_dir!` embedding. Reason: Slice 2 Verification
  #3 explicitly requires operators to be able to edit
  `config/systems/<id>/bios.yaml` directly and restart OA to see new
  known-hashes without a recompile. With `include_dir!`, that
  edit-loop would require a rebuild — defeating one of the arc's
  stated values. The bundling step (Slice 2 or Tauri config update)
  will copy the in-tree `config/` next to `oa-shell.exe` at install
  time, mirroring the existing `<exe_dir>/cores/` + `<exe_dir>/system/`
  convention.

- **Almost:** Operator playtest — `cargo tauri dev`; launch PSX with
  canonical BIOS in `<exe_dir>/system/`; drop only `bios7.bin`
  (NDS) and confirm pill expands inline showing per-file ✓/⚠;
  drop PSX BIOS via "Pick BIOS file…"; open GB readiness checklist;
  confirm `Help → Debug log…` shows
  `system_registry: loaded 3 systems from config/systems/ in Xms`
  at startup. All code-side verification (643 oa-shell tests, was
  615 pre-branch; +28 new) is in.

- **Next:** Slice 2 — bulk migration of remaining 38 systems via a
  new `migrate_systems` dev binary, plus deletion of ~1,800 LOC of
  L1 const tables (replaced by ~80 LOC of accessor methods already
  shipped in Slice 1). Queued in HIGH band of
  [docs/NEXT.md](NEXT.md). May want to split the 38 into two batches
  for operator-playtest scopability.

---

## 2026-05-31 — NDS per-game touch hotspots overlay

Closes the second half of `docs/cores/nds/ROADMAP.md` "Per-game
touch overlay UI" — visual stylus reticle landed 2026-05-27 with
the system-fixes branch; per-game touch-area indicator overlay
ships here. Three-phase feature branch
`feat/nds-touch-hotspots`, merged `--no-ff`. ~600 lines across
9 files (parser + tests + component + seed content + docs).

- **Shipped (Phase 1, `2483a9e` — schema + Rust):** New
  optional `touch_hotspots: [{ label, x, y, w, h }]` field on
  `GameInfo` in `apps/oa-shell/src/game_info.rs`. Coordinates
  in NDS bottom-screen native space (0..256 × 0..192). New
  `TouchHotspot` struct; threaded through `MergedGameInfo` +
  `merge_game_info` (file-only in v1, no override path).
  Schema doc section in `docs/cores/SCHEMA.md`. Three new tests
  (parse roundtrip + merge with file + merge without file) push
  oa-shell to 540 tests green.
- **Shipped (Phase 2, `775ef25` — frontend overlay + toggle):**
  New `frontend/src/components/TouchHotspotOverlay.tsx` (~165
  lines) — contain-fits the NDS combined-frame aspect (256×384
  portrait) into the viewport, maps each hotspot to the bottom
  half of the fitted rectangle, renders thin accent-coloured
  outlined rectangles with floating uppercase label chips
  (operator pick — outline-style over filled-translucent /
  numbered-dot alternatives). Per-session `touchHintsEnabled`
  signal in App.tsx; toggle row "Show touch hints" / "Hide
  touch hints" in QuickSettings ActionsPanel, gated to the
  `HOTSPOT_SYSTEMS` set (NDS today). Per-session ergonomics by
  design — resets on process restart, not a sticky preference.
  Toggle thread: App.tsx → QuickSettings Props → ActionsPanel
  Props. Mount alongside StylusOverlay in App.tsx. Live
  re-fit on window resize.
- **Shipped (Phase 3, `a0a9661` — seed content + ROADMAP
  flips):** New `docs/cores/nds/games-info.md` with three
  flagship entries:
  - **Phantom Hourglass** — 4 corner widgets (Map, Items,
    Menu, Speak/Action).
  - **Brain Age: Train Your Brain in Minutes a Day** — answer
    zone (right pane) + Menu/Done.
  - **Trauma Center: Under the Knife** — vertical tool palette
    (left edge) + patient view (rest).
  NDS ROADMAP "Per-game touch overlay UI" bullet flipped
  🟨→✅. NEXT.md Game Info Panel inventory entry updated to
  record the schema extension + the new overlay component.
- **Almost:** v1 layout assumption — overlay positions
  hotspots against the default melonDS stacked-vertical screen
  layout (top above bottom; bottom screen at y[192..384] of
  the 256×384 framebuffer). Non-default melonDS layouts
  (side-by-side, top-only, hybrid) misplace hotspots until v2
  reads the core option. Documented across SCHEMA + overlay
  header + ROADMAP. Operators using non-default layouts can
  flip the toggle off; visual reticle still works.
- **Next:** Toggle is per-session (resets on restart by
  design). Operators wanting a sticky preference can request a
  v2 promotion to `LayoutStore` or `GameOverrides` later.
  Seed content (Phantom Hourglass / Brain Age / Trauma Center)
  is illustrative; operator-driven expansion to other
  stylus-heavy titles (Mario Kart DS course-select, Pokemon
  Pokétch panel, Nintendogs interaction zones) is a content
  workstream. Schema extension is generic enough that future
  pointer/touch systems (PSP touch in some titles, Vita
  rear-touch) can adopt without changes.

---

## 2026-05-31 — Per-System UI visual overlays routed to future Kiosk shell (two-shell decision)

Single-commit branch `feat/retroverse-per-system-overlay-fix`,
merged `--no-ff`. Resolves the visual conflict the operator
surfaced after the legacy Shell deletion: SystemBackground's
50%-opaque radial gradient was painting on top of the Retroverse
chrome (root-level sibling, later in DOM order, no positioned
ancestor).

- **Shipped (App.tsx surgery — net -22 lines):**
  `SystemBackground` + `SystemBootAnimation` dropped from the
  Retroverse render path. `hoveredSystemId` signal + its
  mouseover tracker dropped (only consumer was
  `SystemBackground`'s source chain). `pinnedEntry` memo dropped
  (same reason; LayoutStore field `rightSidebarPinnedGameId`
  stays untouched since it may yet have a Retroverse home).
  `StylusOverlay` retained — `fixed` positioning + small
  footprint + NDS-only gate, no z-conflict with the Retroverse
  chrome. The dropped components themselves stay in-tree
  (`frontend/src/components/SystemBackground.tsx`,
  `SystemBootAnimation.tsx`) as ready-to-consume building blocks
  for the future Kiosk shell.
- **Shipped (architectural decision — see `docs/PARKING_LOT.md`
  2026-05-31 entry, now expanded):** Two-shell future locked.
  Retroverse stays opinionated + clean (Heroic Games Launcher
  peer); future **Kiosk** shell hosts the themable /
  customizable experience (BigBox peer; name matches existing
  `docs/features/kiosk-shell/`). Per-System UI Stage 1 splits:
  audio + accent colors + tile flourishes stay in Retroverse
  (no visual conflict); `SystemBackground` + `SystemBootAnimation`
  become Kiosk-only; `StylusOverlay` works in either. Stage 2's
  visual/layout parts (per-system navigation: wheels / carousels
  / lists) are Kiosk-only; the audio sub-part (per-system
  in-game SFX) ships in both shells when picked up. Stage 3
  routing case-by-case. **Kiosk stays back-burner** per the
  operator — no demand for the cabinet / customizer use case
  yet; Retroverse covers the daily-driver case completely.
- **Almost:** The PARKING_LOT entry now captures the full
  architectural decision (table-shaped split + cross-refs to
  the Kiosk plan + per-system-ui plan). The Kiosk shell at
  `docs/features/kiosk-shell/` keeps its existing scope; no
  edits needed there. The visual-overlay components stay in
  `frontend/src/components/` dormant until Kiosk picks them up.
- **Next:** Operator can flip Settings → Display → Per-system
  experiences back ON — audio + accents + tile flourishes work
  cleanly in Retroverse with no chrome conflict. The
  deprecation-cycle flag mechanism at `frontend/src/lib/retroverseFlag.ts`
  remains in place through one more release cycle per the
  legacy-Shell-deletion plan §4.

---

## 2026-05-31 — Legacy Shell deleted; Retroverse is the only shell

Closes the multi-week deprecation arc. Single feature branch
`feat/retroverse-legacy-deletion`, three phase commits, merged
`--no-ff`. Net **-1860 lines** across 13 files (close to the
~1900-line estimate in `docs/PLANS/retroverse-flag-deprecation.md`
§8). Operator playtest cycle that started with the 2026-05-31
flag-default flip passed clean enough to greenlight deletion.

- **Shipped (Phase 1, `a28fefa` — App.tsx surgery):** Dropped the
  entire `<Show when={isRetroverseUiEnabled()} fallback={<Shell>...
  </Shell>}>` flag-gate wrapper; `RetroverseShell` now renders
  unconditionally (still gated on the existing `!(isDirectLaunch()
  || gameMode())` fullBleed check). Stripped 76 legacy MenuBar
  items (`toolbarLeft` ~230L + `toolbarCenter` ~20L + `toolbarRight`
  ~63L = ~315 lines of toolbar consts). Each menu item documented
  in the commit message as routed to its Retroverse equivalent
  (SETTINGS categories / LIBRARY GridControls / TileContextMenu /
  QuickSettings / AboutSettings buttons). Dropped legacy signals +
  helpers (`widgetCustomizerOpen`, `overflowOpen`,
  `libraryManagerInitialTab`, `openLibraryManager`,
  `openProperties`, `toggleGameFocus`, `openQuickSettings`,
  `activeGameEntry`, `setPerfHudVisible`, `TOOLBAR_BTN`,
  `requestOpenFirstMenu` import + global Start-button handler).
  Dropped the keyboard handler's `currentView().kind ===
  "library-manager"` gates + the legacy `<HintRegion>` fallback
  that mapped `left-sidebar` / `library-grid` / `right-sidebar`
  focus-group ids to hardcoded hints. Dropped `library-manager`
  + `cores` variants from `SidebarView` in
  `frontend/src/layout/LeftSidebar.tsx:36`; swept doc comments in
  4 files. **Discovered + closed a pre-existing gap:**
  `SystemBackground` / `SystemBootAnimation` / `StylusOverlay`
  were rendered ONLY inside the legacy `<Shell>`'s `<main>` before
  this PR — they were already missing in Retroverse mode during
  the playtest cycle (the operator hadn't noticed the subtle
  per-system art / boot fade / stylus reticle). Restored as
  siblings of `RetroverseProvider` so the Per-System UI Stage 1
  master-toggle intent is preserved now that there's only one
  shell.
- **Shipped (Phase 2, `5fbc6f0` — file deletions):** Six legacy
  chrome files gone — `Shell.tsx` (85L), `TopToolbar.tsx` (38L),
  `RightSidebar.tsx` (246L), `MenuBar.tsx` (604L),
  `widgets/index.tsx` (120L), `WidgetCustomizerDialog.tsx`
  (175L). The empty `frontend/src/layout/widgets/` directory also
  removed. Total -1268 lines from disk.
- **Shipped (Phase 3, `16707ec` — variant collapse):**
  `LibraryManagerPage` Props lost `variant: "page" | "panel"` +
  `onBack` (no surviving "page" caller). Page-mode header + Esc
  handler + classList branching + `TAB_HINTS` map all dropped.
  `LibrarySettings` caller in SettingsSections.tsx updated to
  drop the no-op `onBack` + `variant` props. -55 lines.
- **Almost:** Per-System UI overlay visual conflict in Retroverse
  mode — the restored `SystemBackground` / `SystemBootAnimation`
  compete with Retroverse's own theming. Operator's interim
  workaround is turning Settings → Display → Per-system
  experiences master toggle OFF. Filed in `docs/PARKING_LOT.md`
  2026-05-31 entry as a real follow-up (z-index / opacity /
  theming alignment OR per-shell exemption needed before the
  next Per-System UI stage so the design language stays
  coherent). The flag accessor at `frontend/src/lib/retroverseFlag.ts`
  + the bridge `createEffect` + the Settings → Display →
  Experimental → Retroverse UI toggle UI are all still in place;
  flag mechanism deletion is one more release-cycle out per the
  deprecation plan §4.
- **Next:** Small operator-driven follow-ups documented in the
  Phase 1 commit message — Performance HUD needs a Retroverse
  home (HUD currently always-off, no UI toggle), QuickSettings
  deep-link entries (rewind / TAS / video / memory / disc)
  dropped (operators navigate inside QuickSettings via Esc),
  status-message reader gone (writes still happen, plumbing
  ready for a Retroverse home — likely a toast or LIBRARY header
  status row). PARKING_LOT 2026-05-31 entry for the per-system
  UI overlay conflict is the next branch in this lineage. With
  the legacy Shell gone, the deletion plan in
  `docs/PLANS/retroverse-flag-deprecation.md` can move to "done"
  status; the doc itself can be archived or kept as historical
  reference. `npm run typecheck` silent throughout the deletion;
  no Rust changes.

---

## 2026-05-31 — Retroverse flag default flipped OFF → ON; deprecation cycle starts

Single one-line change at `frontend/src/settings/store.ts:139`
(`DEFAULT_EXPERIMENTAL_RETROVERSE_UI: false → true`) plus the
adjacent doc comment. Merged `--no-ff` from
`feat/retroverse-flag-default-on`. Paired merge of the standing
audit branch `feat/retroverse-flag-deprecation-audit` landed
`docs/PLANS/retroverse-flag-deprecation.md` on main.

- **Shipped:** Fresh installs land in Retroverse on first launch.
  Existing operators with a stored `experimentalRetroverseUi`
  value keep their value (silent migration rejected — worse for
  the "I opted out" expectation than a manual re-flip surprise).
  The Settings → Display → Experimental → Retroverse UI toggle
  stays as the documented escape hatch through one release cycle
  of playtest.
- **Almost:** The escape-hatch toggle is intentionally still in
  place. It drops together with the legacy Shell in the eventual
  deletion PR per `docs/PLANS/retroverse-flag-deprecation.md` §7.
- **Next:** Operator playtest cycle (~one release window).
  Anything that surfaces as missing / broken vs the legacy Shell
  gets fixed in small follow-up branches like the recent
  feat/retroverse-migration-followups (Quit + Game-focus + drop
  overlay + Help dialogs). After the cycle passes, the deletion
  PR removes ~1900 lines of dead code.

---

## 2026-05-30 — Retroverse migration follow-ups (drop overlay + header affordances + Help-dialog home)

Three of the six migration items from §5 of
`docs/PLANS/retroverse-flag-deprecation.md`. Single feature branch
`feat/retroverse-migration-followups`, three phase commits, merged
`--no-ff`. Pre-conditions toward the eventual legacy-Shell deletion
PR; no legacy code removed yet.

- **Shipped (Phase 1, `c0bcacb` — folder-drop overlay relocates
  outside the flag gate):** The `<Show when={dropOverlayVisible()}>`
  block moved from inside the legacy `<Shell>` (App.tsx:1973-1989)
  to a sibling of the flag-gate `<Show>`. The window-global drop
  listener at App.tsx:1748-1769 already fired the ingest in both
  modes — only the visual cue was Retroverse-blind. Now the dashed
  drop card overlays both shells uniformly.
- **Shipped (Phase 2, `494d1da` — RetroverseShell Quit + Game-focus
  indicator):** New `gameFocus: Accessor<boolean>` + `onQuit: () =>
  void` on `RetroverseContext`. Header gains two new elements
  between the clock and the profile chip: a `<Show>`-gated
  accent-colored "Game focus" pill (visible only while keyboard
  passthrough is ON) + a 9×9 ✕ Quit button (muted styling so the
  profile chip stays the visual anchor). Mirrors the legacy
  toolbarRight affordances at App.tsx:1457-1515. Ctrl+G / Ctrl+Q
  keyboard shortcuts continue to work in both modes via App.tsx's
  keydown handler.
- **Shipped (Phase 3, `d8ce7b6` — stale prose sweep + Help-dialog
  Retroverse home):** Two pieces. Prose sweep: rephrased the
  themes / library / fallback strings in `SettingsPage.tsx:144 /
  :154 / :455` plus the `LeftSidebar.tsx:568` comment to drop
  "(legacy Shell only)" / "menu bar" tail references. Discovered
  gap: `Help → Debug log…` and `Keyboard shortcuts…` dialogs were
  reachable only from the legacy MenuBar (App.tsx:1424-1427) — the
  pre-edit prose at `SettingsSections.tsx:796` even pointed at
  "(legacy menu bar)" as the bug-report recipe. New
  `onOpenDebugLog` + `onOpenKeyboardShortcuts` handlers on
  `RetroverseContext`, plus two buttons in AboutSettings → Report a
  bug card ("Open debug log…" primary-styled + "Keyboard
  shortcuts…" secondary-styled). The dialogs themselves (existing
  `helpDialog` signal + `DebugLogDialog` + `KeyboardShortcutsDialog`
  components) remain unchanged.
- **Almost:** The Phase 3 commit message notes the deprecation plan
  doc needs a follow-up tick recording that the Help-dialog gap was
  closed in this branch rather than the deletion PR. Documentation
  tick deferred — non-blocking.
- **Next:** Three migration items remain (drop `WidgetCustomizerDialog`,
  drop right-sidebar toggle button, operator-side verify of
  Hide/Show library + Ctrl+W). The first two are deletions, cleaner
  to do in the deletion PR itself alongside dropping the legacy
  `toolbarLeft` menu items that own them. The verification is
  operator playtest. After playtest passes, the flag default can
  flip ON and the deletion PR follows the ordered checklist in §7
  of `docs/PLANS/retroverse-flag-deprecation.md`. `npm run
  typecheck` silent throughout this branch. No Rust changes.

---

## 2026-05-30 — Gameplay fixes batch (NDS multi-touch + lightgun gun-side buttons + SNES Super Multitap)

Four small-to-medium gameplay-completion fixes shipped as ordered
phase commits on `feat/gameplay-fixes-batch`, merged `--no-ff`. Closes
multiple ROADMAP gaps across the per-core surface in one focused
branch.

- **Shipped (Phase 1, `47cd9ef` — docs only):** NEXT.md stale-entry
  audit. Struck through MEDIUM #4 (Jaguar KP8–KP_HASH dispatch, was
  fully shipped at `bindings.rs::jaguar_high_bit_to_retro_key` +
  edge-detect loop in `main.rs:6134-6148`) and LOWER #9 (SMS Light
  Phaser, dispatch shipped 2026-05-25 via light-gun-harness +
  catalogued at `light_gun_systems.rs:102`). Narrowed LOWER #2 to
  Super Multitap only (Mouse half already shipped via per-game
  device-type override id=2 + label at `GameDialogs.tsx:646`).
  Narrowed LOWER #6 to per-game touch-hotspot configuration only
  (visual stylus reticle already shipped via `StylusOverlay.tsx`).
- **Shipped (Phase 2, `552fd79` — SNES Super Multitap subclass):**
  Mirrors the GameCube Wii peripheral subclass pattern. New
  `DEVICE_ID_OPTIONS_SNES` table + `deviceOptionsForSystem` arm +
  `systemSpecificDeviceLabel` case + per-game hint block in
  `GameDialogs.tsx`. Subclass id 257 (= `(1 << 8) | RETRO_DEVICE_JOYPAD`)
  verified against upstream snes9x `libretro/libretro.cpp` via
  WebFetch before committing — hand-encoded same as Dolphin's Wii
  ids, not the canonical RETRO_DEVICE_SUBCLASS macro. No Rust
  changes — `arm_libretro_device` already dispatches arbitrary u32s.
  Closes `snes/ROADMAP.md:31`.
- **Shipped (Phase 3, `2d13533` — NDS multi-touch):** `oa_core::InputState`
  gains `pointer_secondary: (i16, i16, bool, bool)` companion field;
  `State.input_pointer_secondary[port]` mirror in oa-libretro;
  `pointer_field_value(primary, secondary, index, id)` signature
  widened. cb_input_state POINTER arm dispatches on `index`: 0 →
  primary, 1 → secondary, ≥2 → zero. `POINTER_COUNT` reports total
  pressed across both slots (0/1/2) independent of `index`. v1
  plumbing only — `InputPoller::poll` leaves secondary at
  `(0, 0, false, false)` until a real second-finger source is wired
  (operator-driven follow-up). Closes `nds/ROADMAP.md:50` +
  `NEXT.md` LOWER #7. 4 new tests in `state::tests`
  (`pointer_field_value_index_1_returns_secondary_coords` +
  `_index_out_of_range_returns_zero` + `_count_sums_pressed_slots` +
  `_count_unaffected_by_out_of_range_index`).
- **Shipped (Phase 4, `7795359` — Light-gun gun-side buttons):** New
  `oa_core::InputState.lightgun_buttons: u32` field (bit position
  matches libretro `RETRO_DEVICE_ID_LIGHTGUN_*` id directly; **u32
  rather than the originally-specced u16** because RELOAD is id 16
  which doesn't fit in u16). `State.input_lightgun_buttons[port]`
  mirror; `lightgun_field_value(pointer, buttons, id)` signature
  widened, AUX_A/B/C + START + SELECT + DPAD_{UP,DOWN,LEFT,RIGHT} +
  RELOAD ids now read `(buttons >> id) & 1`. TRIGGER stays driven
  by `pointer.pressed` (mouse left-click) so it fires without
  per-system gun-side bindings. New
  `oa_input::lightgun_buttons_from_joypad_bits(joypad: u32) -> u32`
  derives the bitmask from per-port RetroPad bindings via a fixed
  mapping (Y→AUX_A, A→AUX_B, X→AUX_C, START→START, SELECT→SELECT,
  DPAD→DPAD, R-shoulder→RELOAD) — **no new bindings UI surface**,
  operator rebinds the existing per-system JOYPAD bits to change
  which physical input fires which gun-side button. 4 new tests
  cover the bit dispatch + the u16-vs-u32 RELOAD regression guard.
  Time Crisis pedal-reload + Wild Gunman alt-fire + Hogan's Alley
  pause + Justifier's 3-button gun-side row + HotD 2 START all
  reach the core. Per-core ROADMAPs flipped ✅ for nes / sms /
  saturn / psx / dreamcast / atari7800 (each gains a new
  "Light-gun gun-side buttons" line under the existing operator-
  validation ⬜ bullet). `light_gun_systems.rs` snes notes updated
  to reflect AUX wiring (previously read "return 0 today — Phase 2
  Bindings UI work to wire them"). Cross-system POINTER+LIGHTGUN
  inventory entry in `docs/NEXT.md` updated to record both Phase 3
  + Phase 4.
- **Almost:** Phase 4's u16-vs-u32 deviation from the original spec
  noted in the commit message — operator pre-approved when this
  came up at the planning check. The fixed JOYPAD→LIGHTGUN mapping
  is the other deviation (spec said "InputPoller reads bindings";
  derived-from-existing was the cleanest minimum-viable v1).
- **Next:** Operator playtest of the 8-player Bomberman titles
  (snes Super Multitap), Time Crisis 1/2 / Lethal Enforcers / HotD
  series (gun-side buttons across psx + dreamcast + saturn), Wild
  Gunman + Hogan's Alley (NES gun-side START + AUX_A). NDS
  multi-touch needs a real second-finger source before Hotel Dusk
  3D mode / Glory of Heracles two-finger gestures actually exercise
  the index 1 path; surface is plumbed for the additive follow-up
  PR. `cargo test --workspace` green throughout: oa-libretro 23 →
  30 tests (+7 new), 539 oa-shell tests stable. `npm run typecheck`
  silent.

---

## 2026-05-30 — Game Info Panel v1 ship

Closes the full 11-phase arc from `docs/PLANS/game-info-panel.md`. Single
feature branch `feat/game-info-panel-v1`, nine phase commits, merged
`--no-ff` as `1caa4bc`. 33 new backend tests (506 → 539).

- **Shipped (backend, Phases 1-4):**
  - `apps/oa-shell/src/game_info.rs` — types (GameInfo / GameIdKey /
    BestEmulator / GameBug / BugSeverity / GameInfoMeta / GameInfoOverride
    / MergedGameInfo / GameInfoBadge), multi-document YAML parser with
    defensive `skip_pre_yaml` pre-document trimming.
  - GameInfoIndex with two-key lookup (hash priority, title fallback) +
    runtime `resolve_docs_cores_dir` that tries `<exe_dir>/docs/cores/`
    first then falls back to the source tree.
  - SQLite migration v15 adds `game_info_overrides` (columnar; scalar
    fields per column, array fields as JSON blobs).
  - Field-typed precedence merge per plan §8 — facts file-only, narrative
    operator-wins. `merge_game_info` returns None when both layers empty.
  - Six Tauri commands: `get_game_info`, `get_game_info_override`,
    `set_game_info_override`, `delete_game_info_override`,
    `list_game_info_overridden`, `list_game_info_badges`.
- **Shipped (UI, Phases 5-9):**
  - Retroverse `GameDetailPanel` gains four conditional sections:
    operator-note swap (replaces description with "(operator note)"
    mini-label when local), Controls (chip strip), Recommended core
    (Apply button writes per-game `libretro_core` override + provenance
    flag), Known issues (severity-sorted with red/amber/neutral tints).
    LIBRARY / COLLECTIONS / PLAY NOW only — HOME keeps SystemInfoPanel.
  - `LibraryTile` bottom-right badges: `⚠ N` (severity-tinted) + `✎`
    (operator local-edits indicator). Bulk-fetch via
    `list_game_info_badges` so a 10k-entry library is single-digit ms.
    Context-based store in `frontend/src/library/gameInfoBadges.tsx`
    refreshes on library entry changes.
  - `GameInfoModal` gains a 4th "Game info" tab with inline editor:
    short summary, controls supported (newline-separated), recommended
    core + reason, bugs add/remove with severity dropdown + workaround.
    Save / Reset to default / Submit correction (Phase 9 stub: clipboard
    copy + informational toast).
- **Shipped (docs, Phases 10-11):**
  - `docs/cores/SCHEMA.md` — full schema reference + worked migration
    workflow for `KNOWN_GAME_BUGS.md → games-info.md` per-system.
  - `docs/cores/psx/games-info.md` seed with Tomb Raider + FF7 entries.
  - `docs/cores/psx/README.md` updated as the worked example.
  - `docs/INDEX.md` SCHEMA pointer + `docs/NEXT.md` cross-system
    infrastructure inventory entry.
- **Almost:** The `i` keyboard shortcut (Q3 — open modal directly to the
  Game Info tab) deferred to a follow-up; today the operator clicks MORE
  → Game info. Apply controls action also deferred — v1 controls strings
  are free-form ("Standard gamepad" / "Light gun") and need a
  strings→`RETRO_DEVICE_*` mapping that belongs in v2.
- **Next:** Per-core README touch-ups for the other 42 systems (single-
  line mention of games-info.md next to KNOWN_GAME_BUGS.md, no behavior
  change). Per-system migration of `KNOWN_GAME_BUGS.md` content into
  `games-info.md` records — operator-driven, one system at a time,
  workflow documented in `SCHEMA.md`. v2 evolution (separate data repo +
  scheduled scraper + GitHub-Issue submission flow) fully designed in
  plan §11 but blocked on deciding when to start.

---

## 2026-05-30 — libretro env-callback batch (four gaps closed)

Closes four high-leverage libretro `cb_environment` arms that were previously
unhandled or accept-ignored. Single feature branch
`feat/libretro-env-callbacks-batch`, four phase commits, merged `--no-ff` as
`3b35a41`.

- **Shipped (`SET_MEMORY_MAPS` storage):**
  - New `oa_core::MemoryDescriptor` (metadata only — `flags` / `offset` /
    `start` / `select` / `disconnect` / `len` / `addrspace`) + `Core::memory_map()`
    trait method.
  - `crates/oa-libretro/src/state.rs` parses the descriptor array; metadata
    stored in `State.memory_descriptors`, host base pointers separately in
    `State.memory_map_ptrs` as `usize` so State stays `Send`.
  - Cleared on `load_rom` alongside rotation so back-to-back swaps don't
    inherit stale state.
  - 3 unit tests cover null pointer / zero count / 2-region NES-shape map.
  - Unblocks future RetroAchievements rcheevos integration, cheat-search
    address translation, AI/scripting memory reads.
- **Shipped (`SET_MESSAGE` / `SET_MESSAGE_EXT` → toast):**
  - New `oa_core::CoreMessage` + `CoreMessageLevel` + `Core::drain_messages()`.
  - Env arms for env 6 (legacy frames-based) + env 60 (modern with level /
    target / priority); `GET_MESSAGE_INTERFACE_VERSION` (env 59) returns v1
    so modern cores prefer the richer path.
  - Shell drains per render frame in `run_emu_render`, emits each entry as
    `oa://toast` via existing `emit_toast(level, system, text)`.
  - `target=LOG` messages log-only (skip toast); cores' OSD on save state /
    disc swap / cheat apply / BIOS fallback now surface visually.
- **Shipped (`SET_SUPPORT_NO_GAME` + `load_no_rom()`):**
  - Env arm 18 captures the bool into `State.supports_no_game`;
    `LibretroCore::supports_no_game()` accessor + `LibretroCore::load_no_rom()`
    calls `retro_load_game(NULL)` for DOSBox-Pure / ScummVM bootless mode.
  - Refactored shared post-load work into `finish_load()` so `load_rom` and
    `load_no_rom` stay in lockstep.
- **Shipped (disc-control v2 extras):**
  - `LibretroCore::add_disc_image()`, `replace_disc_image(idx, path)`,
    `set_initial_disc_image(idx, path)`, `disc_image_path(idx)`.
  - `oa_core::DiscInfo` gains `paths: Vec<String>` populated from
    `get_image_path` for v2 cores; v1 fallback returns empty.
  - `read_disc_string_field` helper collapses label / path buffer-fill
    duplication.
  - Frontend `QuickSettings.tsx` `DiscInfo` type extended with `paths`
    field for future tooltip polish.
- **Almost:** UI hook for `load_no_rom()` — bootless launch button for DOSBox
  / ScummVM. Infrastructure is in; operator-facing wiring is its own ~30-line
  follow-up if the bootless workflow becomes a real ask.
- **Next:** the remaining big libretro infra gap is `SET_HW_RENDER` — the
  multi-week task that unblocks Beetle PSX HW / Mupen64Plus-Next /
  PPSSPP / Beetle Saturn HW / Flycast at their real quality tier.

---

## 2026-05-21 — Direct-launch Phase I — explicit #inner, CD-in-archive, --state-file restore

Three follow-ups to direct-launch shipped on top of `main`. Closes
out the load-bearing PARKING_LOT items for the CLI feature.

- **Shipped (explicit `<archive>#<inner>` syntax):**
  - `resolve_explicit_archive_inner` in cli.rs — bypasses Phase H's
    single-ROM requirement; the operator can address one ROM out of a
    multi-game archive without scanning the library first.
  - Inner is validated against `archive::list_rom_contents`; typos
    error with the available-inner list (new
    `CliError::ArchiveInnerNotFound`).
  - Cart inners auto-infer the system via `slug_for_ext`; CD inners
    require `--system`.
- **Shipped (CD-in-archive auto-extract):**
  - `resolve_archive` peek filter extended to accept .cue / .ccd /
    .toc / .m3u in the accepted-extensions set.
  - Single CD inner with `--system` → `archive_inner_path` set;
    `launch_rom`'s existing `is_cd_entry_extension` branch fires
    `archive::extract_to_temp` to `appData/temp/<entryId>/`.
  - Synthesized RomEntry's id + filePath fold the inner path in
    (`<archive>#<inner>` encoding) so different CDs in the same
    archive get distinct entryIds and reuse-then-clean their own
    temp dirs.
- **Shipped (`--state-file PATH` actual restore):**
  - `EmuCommand::LoadRom.restore_state_path: Option<PathBuf>` added.
  - `launch_rom` Tauri command takes `stateFile: Option<String>`,
    threaded through `launchRom` JS → `handleLaunch` → cascade.
  - Emu thread's LoadRom handler reads + `core.load_state` from the
    absolute path after the rom load completes, atomically. Toast on
    read/deserialize failure.
  - CLI parse: `--slot` and `--state-file` mutually exclusive
    (RetroArch convention). State-file existence validated upfront
    so a missing file errors before any subprocess work
    (new `CliError::StateFileMissing`).
- **PARKING_LOT swept:** five direct-launch items closed (Phase H
  + Phase I); three new deferrals added for the CLI v2 batches the
  operator chose to skip (launcher-parity flags, kiosk / arcade,
  diagnostics).
- 309/309 tests green. tsc --noEmit clean.

---

## 2026-05-21 — Direct-launch Phase H — archive auto-extract + Windows-release error visibility

Two same-week fast follow-ups on the direct-launch branch driven by
operator real-world testing.

- **Shipped (Windows release error visibility):**
  - `windows_subsystem = "windows"` (release builds) means stderr is
    silently dropped — operators spawning the .exe from cmd / LaunchBox
    / a double-click saw "nothing happens" on CLI validation errors.
  - New `win_msgbox::error` Windows-FFI shim (linked against user32) +
    `CliError::emit_banner` always pops a native MessageBox on Windows
    release. Debug builds keep using the stderr banner.
- **Shipped (Phase H — single-ROM archive auto-extract):**
  - `.zip` / `.7z` direct-launch now peeks inside. Exactly one cart-ROM
    file → transparently used; system inferred from inner extension
    (or honored from `--system`). MAME / Neo Geo pass the archive
    through as-is via `--system mame` (or `neogeo`) or the `.p1+.s1`
    Neo Geo signature auto-detection.
  - Empty / multi-ROM archives error out with a list (and remediation
    hint pointing at the Import Wizard).
  - `DirectLaunchConfig.archive_inner_path` + DTO mirror flow through
    to the frontend's synthesized RomEntry, which forwards it to
    `launch_rom` so the existing `archive::extract_for_launch`
    plumbing runs identically to a library launch.
  - Hash-lookup hashes the inner ROM bytes (via
    `archive::read_inner_to_bytes`) to match the library DB's sha1
    convention — per-game overrides apply for scanned archived games.
  - `accepted_rom_extensions()` restricted to cart shapes only —
    CD-in-archive support is a separate v2 enhancement.
- **Almost:** Multi-ROM-archive launching via explicit
  `<path>#<inner>` syntax. CD images inside archives.
- **Next:** Operator plays through end-to-end on
  `feat/direct-launch-cli` (positional .nes, positional .zip wrapping
  a .sfc, --system mame on a MAME romset, explicit-error paths). Merge
  to main after thumbs-up.

---

## 2026-05-20 — Direct-launch CLI mode (LaunchBox / BigBox / EmulationStation compat)

External-frontend integration ships. `oa-shell.exe "C:\ROMs\game.nes"`
boots straight into the game with no library UI, the way standalone
emulators do. Default zero-arg behavior unchanged.

- **Shipped:**
  - New `apps/oa-shell/src/cli.rs` module (clap derive) parsing
    positional ROM / `--rom` / `--core` / `--system` / `--slot` /
    `--state-file` / `--tas-replay` / `--fullscreen`. Unambiguous
    cart extensions auto-infer the system; CD-shaped extensions
    require `--system`. Error banners + `process::exit(2)` on
    validation failures.
  - `DirectLaunchConfig` on `AppState`; new Tauri commands
    `get_direct_launch_config` + `get_game(id)`.
  - Forced single-window at runtime when direct-launch is set;
    operator's `OA_SHELL_MODE` / `shell.json` preference preserved
    on disk.
  - `library_db::find_game_by_sha1` (uses existing `idx_games_sha1`)
    + boot-time SHA-1 lookup for cart-shaped ROMs. Matched library
    rows carry their per-game overrides (patches, custom core options,
    shader, rewind config, analog routing, bezel) through the
    standard launch cascade.
  - Frontend: `directLaunchConfig` resource + `isDirectLaunch` memo
    + `Shell.fullBleed` wiring + JSX `<Show>` guards collapse chrome
    to game surface + Quick Settings / Save Slots / Game Info /
    Performance HUD / Toast Stack.
  - `createLibraryStore({ shouldBootstrap })` short-circuits
    `list_games` / `list_game_groups` / migration / seed insertion
    in direct-launch.
  - Auto-launch effect re-uses existing `handleLaunch` cascade so
    per-game / per-system / OA-wide settings, milestones, cheats,
    analog routing all arm normally.
  - Exit-on-unload: emu thread emits `oa://rom-unloaded` after the
    UnloadRom drain; frontend listener calls `quit_app` in
    direct-launch. Quick Settings "Exit to library" relabels to
    "Quit".
  - `OA_ROM` env-var still honored as silent fallback; CLI args win
    when both set.
  - Pre-existing build blocker fixed: removed stale `#[cfg(test)]`
    gate on `sha1::Sha1` import in `rom_hashes.rs`.
  - `docs/direct-launch.md` operator usage doc.
  - 9 new cli.rs unit tests; `cargo test -p oa-shell` 309/309 green.
- **Almost:** `--state-file PATH` accepted by clap but not wired
  yet (frontend logs a warning; operators should use `--slot`).
  Future work: a `restore_state_file` Tauri command, then plumb.
- **Next:** Operator play-tests the branch (`feat/direct-launch-cli`)
  end-to-end — positional launch, --system + CD launch, hash-matched
  per-game overrides applying, Quick Settings overlays working,
  close-window-exits, LaunchBox / EmulationStation real-world
  invocation. Merge to main after thumbs-up.

---

## 2026-05-20 — Sony+Nintendo handheld pass + POINTER infra (systems #34-36: psp + ps2 + nds)

Seventh paired pass of the day. **Second cross-cutting input
infrastructure** of the session — the POINTER device dispatch (mouse-
as-touch) joins this morning's analog input infra to round out OA's
modern-controller input model. NDS, the platform that requires it
most, ships immediately playable. PS2 slots into the CD-launch BIOS
dispatch arm as the 9th system; PSP is BIOS-free.

- **Shipped (cross-cutting POINTER infra):**
  - `oa_core::InputState` extended with `pointer: (i16, i16, bool)`
    field — x, y normalized to libretro POINTER range
    (-32768..32767), plus the pressed flag.
  - `oa-libretro::ffi` — new RETRO_DEVICE_POINTER (6) constant +
    RETRO_DEVICE_INDEX_ANALOG_POINTER_LEFT/RIGHT/BUTTON +
    RETRO_DEVICE_ID_POINTER_X/Y/PRESSED/COUNT.
  - `oa-libretro::state::State` — new `input_pointer: [(i16, i16, bool); 5]`
    field (per-port pointer state).
  - `cb_input_state` extended to dispatch RETRO_DEVICE_POINTER
    queries to the stored pointer state per port/id (X/Y axes,
    pressed flag, count).
  - `LibretroCore::set_input` stores `input.pointer`.
  - `oa-input::InputPoller::poll` — new `poll_pointer()` helper reads
    mouse position via device_query (the same DeviceState the
    keyboard polling uses) + left-button state. Normalizes screen
    coordinates to libretro range (assumes 1920×1080 at Phase 0;
    window-relative pixel-perfect mapping is Phase 2.5).
  - Emu thread updated to plumb `polled.pointer` through to
    `core.set_input`; TAS replay paths set pointer to defaults
    (TAS pointer recording is Phase 2.5).
- **Shipped (Rust core):** Three new `oa_core::SystemId` variants
  (`Psp`, `Ps2`, `Nds`) + parse_system_id arms with aliases.
- **Shipped (bindings):** Three new modules.
  - `psp` — 12 digital buttons (PSX-shape: d-pad + 4 face diamond +
    L/R + START + SELECT). **No L2/R2** — PSP hardware lacks them.
    Single analog stick via shared infra.
  - `ps2` — 16 digital buttons (DualShock 2: PSX-shape + L3/R3 stick
    clicks). Dual analog sticks via shared infra. Pressure-sensitive
    face buttons + analog L2/R2 = Phase 2.5.
  - `nds` — 12 digital buttons (Nintendo diamond: A east PRIMARY,
    B south secondary, X north, Y west; matches nes/snes/gb/gba
    precedent). Touch screen via new POINTER infra.
  Nine new tests lock the dispatch.
- **Shipped (default cores):** psp → `ppsspp_libretro.dll` (BIOS-
  free). ps2 → `pcsx2_libretro.dll` (LRPS2, BIOS-required). nds →
  `melonds_libretro.dll`.
- **Shipped (BIOS pre-checks):**
  - `check_ps2_bios` + `PS2_BIOS_KNOWN_HASHES` (6 entries covering
    JP launch / US fat / US-EU slim variants). Slotted into CD-launch
    dispatch as 9th CD-shape system (pce-cd / segacd / saturn / psx /
    neocd / 3do / pcfx / dreamcast / ps2).
  - `check_nds_bios` + `NDS_BIOS_KNOWN_HASHES` (**new multi-file
    BIOS shape** — requires ALL THREE files: bios7.bin + bios9.bin +
    firmware.bin). Cart-shape pre-check arm in main.rs next to neogeo.
    First multi-file BIOS check in OA's lineup.
  - psp is BIOS-free.
- **Shipped (media + rom_hashes):** Three new repo arms
  (`Sony_-_PlayStation_Portable`, `Sony_-_PlayStation_2`,
  `Nintendo_-_Nintendo_DS`). rom_hashes: psp + nds → single-file
  no-intro dats (.iso/.cso/.pbp for psp; .nds for nds); ps2 → `&[]`
  with NO_DAT_SYSTEMS entry (DVD images deferred).
- **Shipped (frontend):** SystemId union extended. Three new
  `systemThemes` entries. Three new CSS blocks (Plan A — Sony
  cool cluster + Nintendo handheld pearl):
  - **psp:** cool cyan `oklch(0.65 0.18 200)` — middle of the new
    Sony cluster (psx 180° / psp 200° / ps2 215°).
  - **ps2:** deep cobalt `oklch(0.45 0.22 215)` — bottom of the
    cool cluster lightness ladder; period-correct to PS2 blue logo
    + dark-hardware-era marketing.
  - **nds:** pearl yellow-green `oklch(0.78 0.14 95)` — Nintendo
    handheld pearl pattern (matches ngp 105° / WS 305°).
- **Shipped (docs):** ACTIVE_CORE → `nds` (POINTER infra leadership).
  Three full per-core docs sets at `docs/cores/{psp,ps2,nds}/` (15
  doc files). Decisions captured: PPSSPP/LRPS2/melonDS defaults,
  Sony cool cluster theme, Nintendo handheld pearl theme, NDS A is
  PRIMARY per Nintendo convention, multi-file BIOS check pattern
  (3-file NDS check) + 6-entry PS2 BIOS table + POINTER-infra
  rationale.
- **Plan:** Flipped 3 rows ⬜ → ✅; bumped count from 33 to 36.
  **Wave 4 (Nintendo handhelds) COMPLETE + Wave 5 (Sony) COMPLETE.**
  Order-of-attack reduced to 3 groups remaining (scummvm+dosbox,
  5200, pokemini).
- **Validation:** `cargo test -p oa-shell --bin oa-shell` 269/269
  green (was 260; +9 tests across 3 systems × 3 each). `cargo check`
  on workspace clean (POINTER trait changes affect oa-core /
  oa-libretro / oa-input cleanly). Frontend `npm run typecheck`
  silent.
- **Almost:** Phase 1 operator validation. **psp** — drop `ppsspp_libretro.dll`
  + gamepad → God of War: Chains of Olympus. **ps2** — drop
  `pcsx2_libretro.dll` + regional BIOS → Shadow of the Colossus.
  **nds** — drop `melonds_libretro.dll` + 3 BIOS files → Phantom
  Hourglass (canonical "POINTER infra works" test — mouse should
  control Link's stylus).
- **Next:** 36 systems shipped (over 100% of original 34-plan —
  scope expansion landed faster than anticipated). Order-of-attack
  next pick is **`scummvm` + `dosbox`** — engine cores that need a
  folder-as-game scanner extension before they slot in cleanly.

---

---

Older entries (everything 2026-05-20 and earlier) live in [SESSION_LOG_ARCHIVE.md](SESSION_LOG_ARCHIVE.md).
