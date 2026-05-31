# dreamcast — Roadmap

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-20)

- ✅ `oa_core::SystemId::Dreamcast` variant + `parse_system_id` arm
  (`dreamcast | dc | sega-dreamcast`).
- ✅ `bindings.rs::dreamcast` module — 11-button (d-pad + A/B/X/Y +
  L/R + START; no SELECT). Single analog stick flows via shared
  analog infra.
- ✅ `default_core_dll_for_system("dreamcast") → "flycast_libretro.dll"`.
- ✅ `rom_hashes` → `&[]` with NO_DAT_SYSTEMS entry (GD-ROM CD images
  not single-file matched).
- ✅ `media::repo_for_system_id` → `Sega_-_Dreamcast`.
- ✅ `check_dreamcast_bios` + `DREAMCAST_BIOS_KNOWN_HASHES` (4
  entries: dc_boot.bin universal + dc_flash.bin US/JP/EU). Slotted
  into the CD-launch BIOS dispatch arm as the 8th CD-shape system.
- ✅ Frontend SystemId / systemThemes / CSS (Plan A: DC orange
  swirl `oklch(0.55 0.27 32)` — highest chroma in the warm zone).
- ✅ Per-core docs scaffold.

**Acceptance gate:** Operator drops `flycast_libretro.dll` +
`dc_boot.bin` + a regional `dc_flash.bin`, marks a Dreamcast folder
via Import Wizard, sees DC-orange tiles, launches a known-good disc
with gamepad analog stick driving movement.

---

## ⬜ Phase 1 — First Dreamcast game running

- ⬜ Operator validation: Sonic Adventure / Crazy Taxi / Jet Set Radio / Power Stone / Soulcalibur — operator playtest.
- ✅ Analog stick smoke-test — closed by cross-system analog axes (`InputState.axes` + `compute_stick_output` with keyboard fallback + deadzone + sensitivity).
- ✅ L/R analog trigger validation (Crazy Taxi, Daytona USA gas/brake) — closed by Phase B per-button analog pressure (`InputState.analog_buttons` slots 12/13, gilrs trigger axes flow through `cb_input_state` RETRO_DEVICE_INDEX_ANALOG_BUTTON). Operator playtest pending.
- ✅ Save state F5/F8 round-trip mid-disc — closed by cross-system save-state infra (`oa_libretro::LibretroCore::save_state / load_state`).
- ⬜ Multi-region testing (US/JP/EU) — operator playtest.
- ✅ Cover sync via libretro-thumbnails Sega_-_Dreamcast — closed by cross-system media sync (`media::sync_media_for_system`).

---

## ⬜ Phase 2 — Polish

- ⬜ **VMU peripheral support** — the iconic memory-card-with-screen doubled as a peripheral for some titles — gated on Phase 2.5 secondary-screen plumbing.
- ⬜ **Light gun support** (House of the Dead 2, Confidential Mission, Death Crimson OX, Maze of the Kings) — operator validation. LIGHTGUN dispatch shipped 2026-05-25 on `feat/light-gun-harness`; IS_OFFSCREEN reload-by-aim flag plumbed 2026-05-27 via the new `in_viewport` field on `InputState.pointer` (`crates/oa-libretro/src/state.rs::lightgun_field_value`). Flycast polls RETRO_DEVICE_LIGHTGUN for the DC light gun + arcade-cabinet ports; SCREEN_X/Y/TRIGGER/IS_OFFSCREEN reach the core. House of the Dead 2 + Confidential Mission's reload-by-aiming-off-screen gesture now functional. Catalogued in `apps/oa-shell/src/light_gun_systems.rs`.
- ✅ **Light-gun gun-side buttons** (AUX_A/B/C + START + SELECT + DPAD + RELOAD) — shipped 2026-05-30 via Phase 4 of `feat/gameplay-fixes-batch`. New `oa_core::InputState.lightgun_buttons: u32` + State mirror + bit-keyed `lightgun_field_value` dispatch. Bindings derive from per-port RetroPad bits via `oa_input::lightgun_buttons_from_joypad_bits`. The DC Sega Saturn-style light gun has a single trigger + a small START button on the side (used for HotD 2 continue prompts); Confidential Mission's lock-on button maps to AUX_A.
- ✅ **Disc-id extraction** — shipped via `apps/oa-shell/src/cd_id.rs::extractors::dreamcast` (reads IP.BIN HDR serial); `rom_hashes` points at `metadat/redump/Sega - Dreamcast`.
- ✅ **L/R analog trigger pressure-sensitivity** — closed by Phase B (see above). Same shared infra closed GC + PS2.
- ✅ **DC Jump Pack rumble** — closed by Phase F rumble interface (`RETRO_ENVIRONMENT_GET_RUMBLE_INTERFACE` wired through to gilrs). Operator playtest pending.

---

## ⬜ Phase 3+ — Stretch

- ⬜ **Naomi arcade hardware** — deferred (potential separate `naomi` slug).
- ⬜ **DC keyboard/mouse peripherals** (Phantasy Star Online text chat, Quake III DC mouse aim) — keyboard passthrough infra shipped cross-system; libretro KEYBOARD/MOUSE device dispatch for DC peripherals still ⬜.
