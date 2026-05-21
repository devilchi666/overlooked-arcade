# 5200 Roadmap

Per-core phase tracking for the Atari 5200 SuperSystem.

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-20)

- ✅ `oa_core::SystemId::Atari5200` variant + `parse_system_id` arm.
- ✅ `apps/oa-shell/src/bindings.rs` — `atari5200` button module (9 buttons: d-pad + FIRE1 + FIRE2 + START + SELECT + RESET), `ATARI5200_BUTTONS` table, dispatch arms in `bit_for` / `buttons_for` / `to_libretro_bits` / `defaults_for`.
- ✅ `default_core_dll_for_system("5200") → "atari800_libretro.dll"`.
- ✅ `media::repo_for_system_id("5200") → "Atari_-_5200"`.
- ✅ `rom_hashes::libretro_dat_refs_for_system("5200")` — points at the no-intro Atari - 5200 dat.
- ✅ `check_atari5200_bios` wired into the cart-shape BIOS dispatch arm (was pre-staged from the BIOS audit session; activated this session).
- ✅ Frontend `SystemId` union extended, `systemThemes` entry added, CSS theme block (saturated red `oklch(0.62 0.20 18)` matching the 5200's iconic black-and-red faceplate).

**Acceptance gate:** Phase 1 operator validation pending — drop `atari800_libretro.dll` + `5200.rom` into the install, scan a `.a52` library, launch a representative game.

---

## ⬜ Phase 1 — Operator validation

- ⬜ Drop `atari800_libretro.dll v3.1` (from buildbot) into `<exe_dir>/cores/` — operator-driven.
- ⬜ Drop `5200.rom` (SHA-1 `6AD7A1E8C9FAD486FBEC9498CB48BF5BC3ADC530`) into `<exe_dir>/system/` — operator-driven.
- ⬜ Mark a `.a52` folder via the Import Wizard — operator-driven.
- ⬜ Launch (operator playtest):
  - Star Raiders (5200) — flagship pack-in title
  - Missile Command — high-scoring trackball candidate (5200 used keypad)
  - Pac-Man — Atari's 5200 port (improved over the infamous 2600 version)

---

## ⬜ Phase 2 — Hardening

- ✅ **12-key keypad via libretro KEYBOARD device (2026-05-20).** `system_settings::default_keyboard_passthrough("5200") = true` flips the keyboard-passthrough infra on by default — operators press numeric / symbol keys directly while the game window is focused and Atari800 receives them through `retro_keyboard_event`. Same routing infrastructure MAME / MSX use. Missile Command coord-shooting and RealSports Football play selection now work.
- ✅ Full analog routing via the shared analog-input infra. The self-centering joystick was genuinely analog (each axis 0-228 native value); games like Pole Position II read continuous values for steering. Closed by Phase A (PADDLE device-type) + Phase C (mouse-X-as-stick) — operator picks "Analog / Paddle" + mouse X in per-game Input. Operator playtest pending.
- ⬜ Per-game core option templates for the dual-controller "PROLINE" titles (Robotron 2084 on 5200 used both controllers stacked) — operator-driven curation.
