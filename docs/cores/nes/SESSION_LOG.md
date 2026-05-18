# nes Session Log

Format: date + three lines — **Shipped / Almost / Next**. Cross-cutting milestones go in the project-wide `docs/SESSION_LOG.md`.

---

## 2026-05-18 — NES onboarding (Phase 0)

System #3. Onboarded in the same session as SNES — the modern libretro-pivot recipe (registry + CSS + bindings + system_id arms + per-core docs) lets two systems land together in a single session.

- **Shipped:** Registry entry (`.nes` / `.fds` / `.unf` / `.unif`, 3/4 portrait tile aspect). CSS theme (`oklch(0.62 0.22 28)` crimson — the Big Box NES palette). `nes::*` button bits laid out to match libretro positions directly (8 buttons, identity remap). `NES_BUTTONS` table + `nes_bit_for` + `nes_to_libretro_bits` + `default_nes_bindings()` (Z/X for B/A, Enter/RShift for START/SELECT, arrows for d-pad). `defaults_for("nes")` arm + `bit_for/buttons_for/to_libretro_bits` dispatch arms. `oa_core::SystemId::Nes` variant. `parse_system_id("nes" | "famicom") → SystemId::Nes`. `default_core_dll_for_system("nes") → "fceumm_libretro.dll"`. Per-core docs scaffold (this directory). 3 unit tests cover defaults_cover_every_nes_button + identity remap + dispatch.
- **Almost:** Operator runtime validation. Drop `fceumm_libretro.dll` from buildbot.libretro.com into `<exe_dir>/cores/`, scan a folder of `.nes` ROMs, launch SMB to confirm pixels + audio + controller. FDS validation needs `disksys.rom` in `<exe_dir>/system/`.
- **Next:** Once Phase 0 operator-validates, Phase 1 (reference set of NES games verified). Phase 2 polish picks up Mesen swap + Zapper support.
