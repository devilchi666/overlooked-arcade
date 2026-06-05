# nes Session Log

Format: date + three lines — **Shipped / Almost / Next**. Cross-cutting milestones go in the project-wide `docs/SESSION_LOG.md`.

---

## 2026-06-05 — Zapper validated on Duck Hunt

- **Shipped:** Duck Hunt operator validation green-light on FCEUmm. Aim via OS mouse cursor, left-click fires `RETRO_DEVICE_ID_LIGHTGUN_TRIGGER`. Path: per-game Input dialog (in QuickSettings now, surfaced mid-game) → port 1 = "Zapper" (id 258 = FCEUmm's `RETRO_DEVICE_SUBCLASS(MOUSE, 0)`) → `arm_libretro_device` dispatches → `set_controller_port_device(1, 258)` → FCEUmm's `update_nes_controllers` matches `RETRO_DEVICE_ZAPPER` case + wires `SI_ZAPPER` → `cb_input_state(port=1, device=LIGHTGUN)` reads `input_pointer[1]` populated by the mirror-pointer-to-ports-1-4 fix on `ee0f813`. ROADMAP Phase 2 Light gun bullet flipped ✅.
- **Almost:** Hogan's Alley + Wild Gunman as smoke tests for the same arc — same code path, should "just work" since FCEUmm's Zapper auto-attaches via `GameInfo->input[1]` for the rest of the canonical Zapper catalog.
- **Next:** Mesen swap validation (Phase 2 polish) — Mesen may or may not advertise Zapper the same way FCEUmm does; the new dynamic-controller-info path will surface whatever Mesen publishes. Operator-driven; not blocking.

---

## 2026-05-18 — NES onboarding (Phase 0)

System #3. Onboarded in the same session as SNES — the modern libretro-pivot recipe (registry + CSS + bindings + system_id arms + per-core docs) lets two systems land together in a single session.

- **Shipped:** Registry entry (`.nes` / `.fds` / `.unf` / `.unif`, 3/4 portrait tile aspect). CSS theme (`oklch(0.62 0.22 28)` crimson — the Big Box NES palette). `nes::*` button bits laid out to match libretro positions directly (8 buttons, identity remap). `NES_BUTTONS` table + `nes_bit_for` + `nes_to_libretro_bits` + `default_nes_bindings()` (Z/X for B/A, Enter/RShift for START/SELECT, arrows for d-pad). `defaults_for("nes")` arm + `bit_for/buttons_for/to_libretro_bits` dispatch arms. `oa_core::SystemId::Nes` variant. `parse_system_id("nes" | "famicom") → SystemId::Nes`. `default_core_dll_for_system("nes") → "fceumm_libretro.dll"`. Per-core docs scaffold (this directory). 3 unit tests cover defaults_cover_every_nes_button + identity remap + dispatch.
- **Almost:** Operator runtime validation. Drop `fceumm_libretro.dll` from buildbot.libretro.com into `<exe_dir>/cores/`, scan a folder of `.nes` ROMs, launch SMB to confirm pixels + audio + controller. FDS validation needs `disksys.rom` in `<exe_dir>/system/`.
- **Next:** Once Phase 0 operator-validates, Phase 1 (reference set of NES games verified). Phase 2 polish picks up Mesen swap + Zapper support.
