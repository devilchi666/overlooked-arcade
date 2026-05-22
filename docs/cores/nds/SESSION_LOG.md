# nds Session Log

---

## 2026-05-22 — Sidebar tier PR-β: views.json + ViewsStore + SidebarView fold (cross-system)

Second PR of `docs/SIDEBAR_TIER_PLAN.md`. Pure-plumbing PR — zero
user-visible UI change. The sidebar still renders as a flat list of
platform leaves; the data model underneath is now a tree of containers
+ platform nodes driven by the new ViewsStore. PR-γ will replace the
flat render with the recursive tree + count badges + migration banner.

Pre-session note: previous attempt at PR-β was lost to a power outage
mid-implementation. Restarted fresh from `e6e8f4b`; the discarded WIP
was structurally similar to what shipped here (caught up on lessons
learned from the inspection rather than blindly re-deriving).

- **Shipped:** PR-β of 3 from `docs/SIDEBAR_TIER_PLAN.md`. Three
  phase commits squashed via `--no-ff` merge into main as `a049dd4`.
  - **β.1 Rust persistence (`461d29b`)**: new
    `apps/oa-shell/src/views.rs` with the full ViewsConfig schema
    (ViewsConfig / View / ViewKind / ViewNode / ContainerNode /
    PlatformNode / ContainerRule). Atomic writes (tempfile + rename)
    — views.json carries enough operator state that a power-loss
    mid-write would actually hurt, unlike layout.json. `read_views`
    returns `Option<ViewsConfig>` (not a defaulted struct) so the
    frontend can distinguish "file present" from "first launch" and
    pick the right migration path. Two new Tauri commands
    `get_views` / `set_views`, registered in `invoke_handler!` next
    to layout's. Five unit tests cover round-trip, atomic-write
    cleanup, missing-file → None, camelCase + tagged-enum JSON
    shape (must match frontend types), and forward-compat parse of
    older configs lacking `bannerDismissed`.
  - **β.2 Frontend views/ module (`00a2f00`)**: five new files
    under `frontend/src/views/` totalling 638 LOC.
    - `types.ts` mirrors the Rust serde shape one-for-one
      (internally-tagged ViewNode discriminant).
    - `defaults.ts` auto-buckets every registered system into 5
      form-factor containers (Consoles / Handhelds / Computers /
      Arcade / Other), leaves sorted by displayName within each
      bucket. Node-id helpers (`platformNodeIdFor`,
      `parsePlatformNodeId`) keep the encoding stable across
      producer / consumer / synth-fallback.
    - `resolver.ts` ships `findNode` (DFS), `flattenLeaves` (PR-β's
      flat render source), `resolveNodeSystemIds` (container rule
      eval against the registry, with synth-leaf fallback for
      deep-links outside the active view), `countGamesUnder`
      (PR-γ count badges), `synthesizeLeafForSystem`.
    - `migration.ts` builds the "Flat (Legacy)" view from the
      operator's pre-views `layout.systemOrder` (preserves their
      drag-reorder work as a selectable view). `reorderForFormFactor`
      implements Option C — preserves operator's relative ordering
      within each form-factor bucket when they pick "Try Form Factor
      view" in PR-γ's banner.
    - `store.ts` — `createViewsStore()` Solid composable matching
      LayoutStore's shape. Hydrates from `get_views`; seeds defaults
      + optional Flat-Legacy on first launch (migration path
      decided via `get_layout`'s `systemOrder`); write-through to
      `set_views` on every mutation, gated on `hydrated()`.
      Mutations: setActiveView, setBannerDismissed, toggleExpanded,
      setNodeHidden, reorderChildren, reorderTopLevel, replaceView,
      commitTryFormFactor (batched).
  - **β.3 SidebarView fold (`6ccf22d`)**: hard rewrite of the
    SidebarView discriminant — `{ kind: "system"; id }` removed
    entirely (no alias period; TypeScript catches every missed
    call site), replaced with `{ kind: "view-node"; viewId; nodeId }`.
    Five call sites in App.tsx + LeftSidebar + LibraryView updated
    via two new helpers (viewForSystem / viewToSystemId). LeftSidebar
    renders from `flattenLeaves(activeView.root)` with per-leaf
    parent-id tracking; drag-reorder writes to
    `ViewsStore.reorderChildren` for same-parent drops, silently
    no-ops cross-parent drops (cross-container drag is PR-γ
    surface). `library/filter.ts` `filterEntries` now takes a
    resolved `viewSystemIds: ReadonlyArray<SystemId> | null` —
    pure module stays ViewsStore-unaware. LibraryView accepts
    `views` prop and resolves the selection via
    `resolveNodeSystemIds` (full filter) + a single-system
    accessor for page chrome (header / data-system / title).

- **Almost:** N/A — clean three-phase landing. Operator confirmed
  visual no-change + builds green; pushed to origin/main as
  `a049dd4`.

- **Next:** PR-γ from `docs/SIDEBAR_TIER_PLAN.md` §3 — the
  visible-change PR. New `SidebarTreeNode` recursive component
  with twisties + recursive count badges, replaces LeftSidebar's
  flat render with the tree. Migration banner for upgrade installs
  (showing "Try Form Factor view" vs "Stay on Flat (Legacy)").
  Right-click container "Hide from sidebar." Cascade auto-hide-empty
  for containers. Within-container + top-level drag-reorder via
  solid-dnd SortableProvider scopes. LibraryManagerPage
  system-visibility checkboxes rewired from `layout.hiddenSystems`
  to `viewsStore.setNodeHidden(platform_node_id, ...)`.

---

## 2026-05-22 — Sidebar tier PR-α: registry tagging + gb/gbc split + msx/msx2 stubs (cross-system)

First PR of the sidebar hierarchy execution sheet
(`docs/SIDEBAR_TIER_PLAN.md`). Frontend registry now carries the
`formFactor` + `manufacturer` tags the upcoming default Platforms view
consumes, plus the three system splits/adds the plan's bucket lists
call for. Logged under `nds` (still ACTIVE_CORE) because the work is
cross-system UI infra, same pattern the UI polish PRs followed.

- **Shipped:** PR-α of 3 from `docs/SIDEBAR_TIER_PLAN.md`.
  - **Type plumbing:** `FormFactorTag` + `ManufacturerTag` union types
    added to `frontend/src/themes/registry.ts`; `SystemTheme` extended
    with required `formFactor` + `manufacturer` fields.
  - **Registry audit:** all 38 existing entries tagged per the plan
    §1.2 tables (28 console / 9 handheld pre-split / 1 arcade; MAME →
    manufacturer `other` per operator confirmation pending MAME-import
    metadata work).
  - **gb/gbc split:** existing combined `gb` entry split into
    `gb` (.gb, DMG only, displayName "Nintendo Game Boy") and the new
    `gbc` (.gbc, CGB only, displayName "Nintendo Game Boy Color").
    Both share the Gambatte core via `oa_core::SystemId::Gb`. New
    `[data-system="gbc"]` block in `systems.css` (translucent-cart
    magenta at hue 320°, distinct from gb pea-green / Lynx purple
    290° / SNES violet 270°). Rust dispatch arms in `bindings.rs`
    (bit_for, buttons_for, to_libretro_bits, defaults_for) all extend
    `"gb" =>` to `"gb" | "gbc" =>` since gbc reuses gb's hardware
    layout. Thumbnail-repo split in `media.rs` + libretro-database
    split in `rom_hashes.rs` + metadata-source split in `metadata.rs`
    so each slug resolves to its own dat / repo. `cli.rs`
    `slug_for_ext` no longer aliases `.gbc → "gb"`; each extension
    routes to its own slug.
  - **MSX / MSX2 stubs:** new `msx` + `msx2` entries in
    `frontend/src/themes/registry.ts` with `formFactor: "computer"`
    + `manufacturer: "microsoft"`. Extensions list empty by design
    so the library scanner produces no entries for them today.
    `[data-system="msx"]` + `[data-system="msx2"]` CSS blocks (royal
    blue at hue 250°, MSX2 brighter at L=0.65 vs MSX L=0.55).
    `main.rs` `parse_system_id` already had `"msx" | "msx2" → SystemId::Msx`;
    `default_core_dll_for_system` extended with the same arm pointing
    at `bluemsx_libretro.dll`. Bindings / BIOS pre-check / per-folder
    `.rom` disambiguation **deferred to a dedicated "MSX system add"
    follow-up PR** — declared-but-not-yet-runnable in the spirit of
    the plan's PR scope discipline.
  - Frontend roster: 41 systems (28 console + 11 handheld with the
    gbc split + 2 computer + 1 arcade). `npm run typecheck` clean.
  - Rust: 405 tests passing across the workspace (oa-shell 342 +
    other crates 63), including the bindings test loops which now
    iterate gbc as well.
- **Almost:** PR-β (views infrastructure + SidebarView fold) — depends
  on operator validation of PR-α first.
- **Next:** After operator thumbs-up + merge, branch `feat/sidebar-tier-beta`
  and start §2 of the plan (views.json schema, Rust commands, ViewsStore,
  default + legacy view construction, migration seeding logic,
  SidebarView discriminant rewrite, library filter extension). Sidebar
  still renders flat in PR-β; PR-γ does the tree UI.

Follow-ups noted for future PRs (filed here so they're not lost):
- **Full MSX/MSX2 wiring** — pick canonical extensions (likely .mx1
  / .mx2 / .dsk + per-folder rule for .rom), populate `MSX_BUTTONS`
  + remap + `default_msx_bindings`, plumb through bindings.rs test
  arrays, add BIOS pre-check shape, document in
  `docs/cores/msx/README.md`. Probably a single dedicated branch
  rather than rolling into PR-β.
- **Library Manager visibility** — the system-visibility checkboxes
  in `LibraryManagerPage.tsx` don't yet know about msx/msx2. Will be
  handled by PR-γ's wholesale Settings reconciliation against the
  active view's leaves.

---

## 2026-05-22 — UI polish PR 3 + PR 4 (Phases D + E, cross-system)

Final two PRs of the polish-plan execution, bundled per operator request.
Plan now fully shipped — `docs/UI_POLISH_PLAN.md` complete.

- **Shipped:** PR 3 + PR 4 of 4 from `docs/UI_POLISH_PLAN.md`.
  - **Phase D — drawer shrink + Game-menu dialog extraction:**
    - New `GameDialogs.tsx` (~1733 lines) with seven focused
      single-purpose dialogs: `GameCoreOptionsDialog`,
      `GameDisplayDialog`, `GameInputDialog`, `GameRewindDialog`,
      `GameShadersDialog`, `MilestonesDialog`, `CheatsDialog`. Shared
      `useGameOverrides()` composable owns hydration + the patch
      helper; each dialog uses the appropriate Dialog size from PR 2
      (Cheats / Milestones / Input / Core options / Display at xl).
    - `PerGameSettingsDrawer.tsx` (1933 lines, 10 tabs) collapsed to
      `GamePropertiesDialog.tsx` (~225 lines) with only Overview +
      Core in two `<DialogSection>`s at xl. Region tab deleted
      entirely (no runtime effect; duplicated boxart RegionPicker
      semantically). Drawer chrome (slide-in, tab strip, custom Esc
      handler, custom backdrop) all gone — Dialog primitive handles
      them uniformly.
    - `App.tsx` Game ▾ menu rewires the 7 deep-link items to a single
      discriminated `gameDialog` signal `{ kind, target }`. Properties
      keeps opening the slim Properties dialog. Old "ROM patch…" menu
      item retired (folded into Properties → Core); "Input…" takes
      its slot.
    - Cheats + Milestones implementations carried over largely
      verbatim (4-stage cheat-search state machine + MilestoneEditor
      draft pattern are fiddly enough that mechanical extraction is
      the right risk profile). New behavior: `CheatsDialog`
      auto-ends an in-flight cheat search if the dialog closes
      mid-search (avoids orphaned Rust-side session).
  - **Phase E — kiosk shell hooks:**
    - `--kiosk` CLI flag added to `Cli` (clap). `parse_and_resolve()`
      now returns `CliConfig { direct_launch, kiosk }` rather than
      `Option<DirectLaunchConfig>` — keeps kiosk orthogonal to ROM
      presence so the flag works alone for testing.
    - `AppState.kiosk` + `get_kiosk_mode` Tauri command surface the
      flag to the frontend.
    - `LayoutStore` onMount: after hydrating `presentation.json` but
      before `setHydrated(true)`, reads `get_kiosk_mode`; if true
      forces `setPresentationMode("cabinet")`. The write-through
      effect is gated on `hydrated()`, so this runtime override
      doesn't persist to disk. Operator's on-disk preference is
      preserved for the next library-mode launch.
    - `chromeVisible()` memo added in `App.tsx` —
      `!isDirectLaunch() && !gameMode()`. Zero behavior change today;
      pre-wired so Phase 1 of the kiosk plan only has to extend the
      memo body to gate menu bar + toolbar + sidebars off when a
      future PresentationMode variant lands.
- **Almost:** —
- **Next:** Polish plan complete. Kiosk Phase 1 (the actual kiosk
  shell — `docs/KIOSK_PLAN.md`) is the next polish-adjacent block but
  not next-up; we return to per-core work. `docs/ACTIVE_CORE.md` is
  nds — pick up operator validation for melonDS (Phase 1 BIOS
  pre-check + cart-shape, NSMB DS + Phantom Hourglass stylus test).

## 2026-05-22 — UI polish PR 2 (Phases B + C, cross-system, not core-specific)

Continues the polish-plan execution. PR 1 (Phase A) landed earlier today.

- **Shipped:** PR 2 of 4 from `docs/UI_POLISH_PLAN.md`.
  - Dialog primitive: size scale widens to sm/md/lg/xl/2xl; new
    `<DialogSection>` for row grouping; type ramp + spacing + SVG
    close-button glyph.
  - SettingRow: built-in `select` / `slider` / `toggle` controls;
    typed `inherited` + new `description`, `disabled`, `onReset`
    props; exports `selectClass(tone)` as the canonical select-
    styling helper. Legacy `inheritedValue` / `inheritedFrom`
    pair kept as a passthrough during migration.
  - DisplayDialog migrates at xl as the reference (three sections:
    Scaling / Window / Run-ahead).
  - Audio / Gameplay / Shaders dialogs adopt built-in controls.
  - SystemDialogs + PerGameSettingsDrawer bloom sliders collapse to
    `SettingRow.slider` + `onReset`.
  - 3 of 4 LibraryManagerPage row candidates migrated (only-sync,
    auto-remove, revision-tiebreaker). Action-select for
    "Clear games for" stays raw + uses the new `selectClass("oa")`
    helper — the DOM-reset idiom after each pick doesn't fit a
    controlled built-in.
  - Three duplicate SELECT_CLASS constants deleted (SettingsDialogs,
    SystemDialogs, LibraryManagerPage); single source of styling
    is now `selectClass()` in SettingRow.
- **Almost:** —
- **Next:** PR 3 from `UI_POLISH_PLAN.md` — Phase D, the biggest PR.
  Shrink `PerGameSettingsDrawer` (10 tabs → 2: Overview + Core) and
  extract 7 Game-menu dialogs (`GameCoreOptionsDialog`,
  `GameDisplayDialog`, `GameInputDialog`, `GameRewindDialog`,
  `GameShadersDialog`, `MilestonesDialog`, `CheatsDialog`). Delete the
  Region tab (no runtime effect; duplicates boxart RegionPicker
  semantically). Depends on the `xl` size from PR 2 — Cheats /
  Milestones / Input / Core options / Display all want the room.

## 2026-05-22 — UI polish PR 1 (Phase A cleanup, cross-system, not core-specific)

Logged here because nds is the active core; the work itself is cross-cutting
UI shaped by `docs/UI_POLISH_PLAN.md` (Phase 0 of the kiosk plan). See
`docs/UI_POLISH_PLAN.md` §1 for the full Phase A spec.

- **Shipped:** PR 1 of 4 from the polish plan. `SettingsPage.tsx` →
  `LibraryManagerPage.tsx` (heading, localStorage key, warn prefixes,
  dead `moveRegion` helper removed). `SidebarView` discriminant
  `"settings"` → `"library-manager"` across `App.tsx`, `LeftSidebar`,
  `LibraryView`, `filter.ts`. Bottom Cores + Settings buttons on the
  left sidebar deleted (collapse toggle preserved). Stale
  `PerSystemSettingsPage` doc-comments repointed across `CoresPage`,
  `PerGameSettingsDrawer`, `SystemBindingsEditor`. `docs/UI_AUDIT.md`
  gains a staleness header pointing at the polish plan.
- **Almost:** —
- **Next:** PR 2 from `UI_POLISH_PLAN.md` — Phase B + C bundled
  (Dialog primitive polish: new `sm/md/lg/xl/2xl` size scale,
  `<DialogSection>` component, type-ramp updates; plus `SettingRow`
  canonicalization: built-in `select/slider/toggle` controls,
  `description` prop, `disabled` + `onReset` props, delete three
  duplicate `SELECT_CLASS` constants. `DisplayDialog` migrates as
  the reference).

## 2026-05-21 — Shared analog input infra Phases E + F + G (cross-system infra)

- **Audit:** Operator asked "I thought analog input infra was done."
  Checked: Phases A-D shipped substantively (per-game device-type
  override, per-button analog pressure, mouse-as-stick, per-game UI)
  but NEXT.md DEFERRED + per-core ROADMAPs still listed the umbrella
  as open. Three genuinely-still-open siblings: multi-port
  device-type (port-0-only today), rumble interface (declined),
  sensor interface (declined).
- **Shipped (Phase E — multi-port device-type):** `GameOverrides`
  gains `libretro_device_port1..4: Option<u32>` siblings to the
  existing `libretro_device` (port 0 kept for back-compat).
  `arm_libretro_device` walks all 5 ports.
  `set_libretro_device_for_game` takes optional `port` so the same
  Tauri command writes any port. `PerGameSettingsDrawer` Input tab
  adds a collapsible "+ Additional ports (1–4)" section that
  auto-expands when any port-1..4 override is non-null.
- **Shipped (Phase F — rumble interface):** New FFI types
  (`retro_rumble_effect`, `retro_rumble_interface`,
  `retro_set_rumble_state_t`). `State.rumble: [[u16; 2]; 5]`.
  `cb_set_rumble_state` trampoline + env 23 handler.
  `LibretroCore::rumble_snapshot()` accessor.
  `InputPoller::dispatch_rumble(strengths)` builds long-lived
  gilrs `Effect` per (port × kind) lazily, varies magnitude via
  `set_gain` (continuous-rumble polls stay cheap), stops on
  strength=0, rebuilds on gamepad rotation. Shell's emu thread
  calls dispatch after each NORMAL forward-play `run_frame`.
- **Shipped (Phase G — sensor interface):** FFI types
  (`retro_sensor_interface`, `retro_set_sensor_state_t`,
  `retro_sensor_get_input_t`, RETRO_SENSOR_* constants).
  `State.sensor_enabled: [[bool; 3]; 5]` +
  `State.sensor_values: [[f32; 7]; 5]`.
  `cb_set_sensor_state` + `cb_get_sensor_input` trampolines.
  Phase 1 fallback: keyboard arrow keys feed accelerometer X/Y on
  port 0 (Z = 1g gravity baseline) so GBA Boktai / Kirby Tilt 'n'
  Tumble / WarioWare Twisted! are playable without OS-level
  accelerometer. `core_ref.sensors_enabled()` guard skips the
  per-frame pump for the 95% of cores that don't use sensors.
- **Doc sweep:** Flipped ⬜→✅ across 11 per-core ROADMAPs (2600
  paddle/driving; 5200 full analog; 7800 twin-stick/light-gun/
  trakball; channelf plunger; coleco super-action/roller;
  dreamcast triggers/jump-pack; gamecube triggers/vibration;
  ps2 pressure/rumble; psx DualShock/rumble; intv 16-dir disc;
  gba tilt/solar/rumble; mame steering/trackball/paddle/yoke;
  n64 Rumble Pak). Updated NEXT.md DEFERRED to remove the umbrella
  entry; added Phase E/F/G to cross-system infra inventory.
- **Tests:** All workspace tests green (cargo test --workspace —
  333+ across 19 crates). Frontend tsc --noEmit clean.
- **Almost:** Operator validation across the unlocked features.
  Canonical tests: Beetle PSX DualShock (Ape Escape), N64 Rumble
  Pak (Star Fox 64), GameCube triggers (RE4 brake-feel), GBA tilt
  (Kirby Tilt 'n' Tumble with keyboard fallback), Atari 2600
  paddle (Breakout / Kaboom! with mouse-X).
- **Next:** Operator playtest of the unlocked features per the
  canonical tests above. Trackball-delta verification (MAME
  Marble Madness) listed in NEXT.md DEFERRED for now since
  RETRO_DEVICE_MOUSE may already work via existing pointer
  dispatch — verify-as-needed.

---

## 2026-05-21 — Library folders: SQLite single source of truth (cross-system infra)

- **Diagnosis:** Operator reported "no folders tracked" in Settings →
  Library despite 5 folders + ~4500 games imported. SQLite `folders`
  table held all 5 paths correctly; the localStorage
  `oa.settings.v1.libraryFolders` mirror was empty. Two parallel stores
  had drifted (last log entries that would have showed the drift were
  already rotated out — the 5-archive cap loses ~3 days of history).
- **Shipped (Schema v12):** New `folders.display_order INTEGER NOT NULL`
  column, backfilled from `rowid`. `list_folders` orders by
  `display_order, rowid`. `add_folder` inserts at `MAX+1` so new rows
  go to the end of the user's order. New `reorder_folders(ordered_ids)`
  bulk-update for drag-reorder.
- **Shipped (Tauri):** `reorder_folders` + `migrate_folders_from_local_storage`
  commands. Migration is idempotent (paths already in `folders` are
  skipped) so the strip-and-save step is crash-safe.
- **Shipped (frontend settings store):** Removed `libraryFolders` from
  `Persisted`. Replaced with SQLite-backed `libraryFolderRows` signal
  populated via `list_folders`; `libraryFolders()` getter returns paths
  for backward compatibility with the watcher + Rescan-all. New
  `addLibraryFolderPath`, `removeLibraryFolderById`,
  `reorderLibraryFolderIds`, `refreshLibraryFolders` setters write
  through to SQLite then refresh. One-shot localStorage migration runs
  on init.
- **Shipped (App.tsx + SettingsPage + ImportWizard):** All `setLibraryFolders`
  callers migrated. SettingsPage drag-drop now uses folder ids as
  sortable keys (stable across reorder). ImportWizard drops the mirror
  line and calls `refreshLibraryFolders` after commit.
- **Tests:** `folders_display_order_persists_and_reorders` +
  `migrate_folders_from_local_storage_idempotent` alongside the
  existing `folders_crud_roundtrip`. `cargo test --workspace` green
  (333+ tests). Frontend `tsc --noEmit` clean.
- **Almost:** Operator validation. First launch after upgrade should
  auto-migrate any operator who has localStorage paths into SQLite +
  populate the Settings list from the now-authoritative store.
- **Next:** The operator's previously-imported 5 folders will appear
  in Settings on next launch (SQLite already has them; no migration
  needed for that case — the empty localStorage was the bug).

---

## 2026-05-21 — Honor libretro option-visibility envs (cross-system infra)

- **Shipped (libretro envs 55 + 69):** Wired
  `SET_CORE_OPTIONS_DISPLAY` and `SET_CORE_OPTIONS_UPDATE_DISPLAY_CALLBACK`,
  the two accept-and-ignore stubs the panel was leaving on the floor.
  Cores can now hide options that don't apply given the current
  configuration (Beetle PSX's "Lightgun crosshair color" goes away
  when "Lightgun" is off; ditto Dolphin's GC-vs-Wii overlay options,
  Mupen64Plus-Next's HW-renderer-only options when SW is selected, etc.).
- **Shipped (`oa-libretro`):** `retro_core_option_display` +
  `retro_core_options_update_display_callback` FFI types;
  `State.hidden_options: HashSet<String>` + `State.update_display_cb:
  Option<retro_core_options_update_display_callback_t>` fields;
  env-callback handlers populate them; schema-replacing envs clear
  the hidden set so a re-init starts fresh.
- **Shipped (`oa-core::Core`):** Two new default-empty trait methods —
  `hidden_option_keys()` + `refresh_option_visibility()`. Cores
  without dynamic visibility (everything non-libretro, or libretro
  cores that don't register the callback) inherit the no-op defaults.
- **Shipped (`LibretroCore` impl):** `hidden_option_keys()` returns
  the State's set; `refresh_option_visibility()` lifts the cb pointer
  out from under the State mutex (so the core's re-entry into
  `cb_environment` doesn't deadlock), then invokes it.
- **Shipped (shell):** `CoreOptionsFile` gains `hidden_keys: Vec<String>`
  on disk; `refresh_schema` captures the initial set post-load AFTER
  pushing effective overrides (visibility is value-dependent); a new
  `refresh_visibility` mutates only the hidden set; the emu-thread
  handlers for `SetCoreOption` + `ApplyCoreOptions` invoke
  `refresh_option_visibility` then write the updated set back.
  `list_core_options` surfaces `hiddenKeys` to the frontend.
- **Shipped (frontend):** `CoreOptionsPanel` filters hidden keys out
  of `filteredOptions`; option count denominator shows
  `schema.length - hiddenKeys.length`.
- **Tests:** Added `refresh_visibility_replaces_hidden_keys_only`
  alongside the existing `refresh_schema_drops_stale_keys`.
  `cargo test --workspace` green (271/271; was 269/269).
- **Almost:** Operator validation on a core that actually exercises
  the dynamic path. Beetle PSX's lightgun-color toggle is the
  canonical test. NDS itself (melonDS) doesn't use the dynamic
  visibility callback, but its schema is captured + filtered
  through the same plumbing.
- **Next:** Existing nds onboarding path — operator drops
  `melonds_libretro.dll` + 3 BIOS files; first stylus-game launch.

---

## 2026-05-20 — Phase 0 onboarding (paired with psp + ps2; POINTER infra shipped)

- **Shipped (Rust core):** SystemId variant (Nds), parse_system_id
  arm (`nds | ds | nintendo-ds`), `bindings.rs::nds` module
  (12 digital buttons; Nintendo diamond layout — A east PRIMARY per
  Nintendo convention, B south secondary, X north, Y west).
- **Shipped (POINTER input infra — cross-cutting):**
  - `oa_core::InputState` extended with `pointer: (i16, i16, bool)`
    field (x, y normalized to libretro POINTER range; pressed flag).
  - `oa-libretro::ffi` adds RETRO_DEVICE_POINTER (6) +
    RETRO_DEVICE_INDEX_ANALOG_POINTER_* + RETRO_DEVICE_ID_POINTER_*
    constants.
  - `oa-libretro::state::State` extended with
    `input_pointer: [(i16, i16, bool); 5]`.
  - `cb_input_state` dispatches RETRO_DEVICE_POINTER queries to the
    stored pointer state per port/id.
  - `LibretroCore::set_input` stores `input.pointer`.
  - `oa-input::InputPoller::poll` samples device_query mouse via the
    existing DeviceState handle — normalizes screen coordinates to
    libretro POINTER range; reads left mouse button as the pressed
    flag.
  - End-to-end mouse-as-touch dispatch.
- **Shipped (BIOS pre-check — new multi-file shape):** `check_nds_bios`
  + `NDS_BIOS_KNOWN_HASHES`. Unlike single-file BIOS checks, requires
  ALL THREE files (bios7.bin + bios9.bin + firmware.bin) to be
  present. Cart-shape pre-check arm in main.rs (next to neogeo).
- **Shipped (default core, media, rom_hashes):** melonDS default.
  `Nintendo_-_Nintendo_DS` thumbnails repo. no-intro NDS dat.
- **Shipped (frontend):** SystemId union + systemThemes
  (`.nds` extension, 3/4 portrait tile, crt-lite). CSS: pearl
  yellow-green `oklch(0.78 0.14 95)` (Nintendo handheld pearl pattern
  matching ngp 105° / WS 305°).
- **Shipped (docs):** Per-core scaffold.
- **Almost:** Phase 1 operator validation. Stylus games are the
  canonical "POINTER infra works" test.
- **Next:** Operator drops `melonds_libretro.dll` + 3 BIOS files
  (`bios7.bin` + `bios9.bin` + `firmware.bin`), scans NDS ROMs,
  launches NSMB DS (button-only) + Phantom Hourglass (stylus test).
