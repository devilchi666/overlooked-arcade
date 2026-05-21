# nds Session Log

---

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
