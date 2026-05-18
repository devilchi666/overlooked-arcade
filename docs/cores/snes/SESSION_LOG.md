# snes Session Log

Format: date + three lines — **Shipped / Almost / Next**. Cross-cutting milestones go in the project-wide `docs/SESSION_LOG.md`.

---

## 2026-05-18 — SNES onboarding (Phase 0)

System #4. Onboarded alongside NES in the same session — the modern recipe handled 8-button NES + 12-button SNES in one set of changes.

- **Shipped:** Registry entry (`.sfc` / `.smc` / `.fig` / `.swc`, 4/3 landscape tile aspect). CSS theme (`oklch(0.62 0.18 270)` violet — SNES diamond-button family palette, distinct from Lynx's purer 290° purple). `snes::*` button bits laid out to match libretro positions directly (12 buttons: A/B/X/Y diamond + L/R shoulders + START/SELECT + d-pad, identity remap). `SNES_BUTTONS` table + `snes_bit_for` + `snes_to_libretro_bits` + `default_snes_bindings()` (Z/X/A/S diamond, Q/W shoulders, Enter/RShift, arrows). `defaults_for("snes")` arm + dispatch arms. `oa_core::SystemId::Snes` variant. `parse_system_id("snes" | "super-famicom") → SystemId::Snes`. `default_core_dll_for_system("snes") → "snes9x_libretro.dll"`. Per-core docs scaffold.
- **Almost:** Operator runtime validation. Drop `snes9x_libretro.dll` from buildbot.libretro.com into `<exe_dir>/cores/`, scan a folder of `.sfc` ROMs, launch Super Mario World as the reference game.
- **Next:** Once Phase 0 operator-validates, Phase 1 (reference set including special-chip games — SMRPG / Star Fox / Yoshi's Island exercise the SA-1 / SuperFX paths). Phase 2 polish picks up bsnes swap + hi-res game validation + SNES Mouse + Super Multitap.
