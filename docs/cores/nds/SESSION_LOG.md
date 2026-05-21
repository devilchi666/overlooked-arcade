# nds Session Log

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
