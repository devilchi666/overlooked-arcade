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

- ⬜ Operator validation: Sonic Adventure / Crazy Taxi / Jet Set
  Radio / Power Stone / Soulcalibur. Pick a disc matching the BIOS
  region.
- ⬜ Analog stick smoke-test (Sonic's full 3D platforming needs
  smooth LeftStick → analog axis).
- ⬜ L/R analog trigger validation (most racing games — Crazy Taxi,
  Daytona USA — use them as gas/brake).
- ⬜ Save state F5/F8 round-trip mid-disc.
- ⬜ Multi-region testing (US/JP/EU).
- ⬜ Cover sync via libretro-thumbnails Sega_-_Dreamcast.

---

## ⬜ Phase 2 — Polish

- ⬜ **VMU peripheral support** — the iconic memory-card-with-screen
  doubled as a peripheral for some titles (Sonic Chao raising,
  Skies of Arcadia, Sonic Adventure mini-games). Phase 2.5 work
  alongside per-system Bindings UI for the VMU's secondary screen.
- ⬜ **Light gun support** (House of the Dead 2, Confidential Mission,
  Death Crimson 2). Flycast supports the DC light gun via libretro
  pointer device.
- ⬜ **Disc-id extraction** — DC discs key on IP.BIN signature in
  the data track header. Extend cd_id.rs.
- ⬜ **L/R analog trigger pressure-sensitivity** — Phase 2.5 polish.
  Flycast supports analog triggers via RETRO_DEVICE_INDEX_ANALOG_BUTTON
  which OA's cb_input_state currently returns 0 for (deferred along
  with GC's analog L/R).

---

## ⬜ Phase 3+ — Stretch

- ⬜ **Naomi arcade hardware** — DC's arcade sibling running the
  same hardware. Flycast handles Naomi via core options; potential
  separate `naomi` slug if shipped.
- ⬜ **DC keyboard/mouse peripherals** (Phantasy Star Online text
  chat, Quake III Arena DC mouse aim). Phase 2.5 polish via
  libretro KEYBOARD/MOUSE device dispatch.
