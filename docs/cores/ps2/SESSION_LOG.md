# ps2 Session Log

---

## 2026-05-20 — Phase 0 onboarding (paired with psp + nds)

- **Shipped:** SystemId variant (Ps2), parse_system_id arm,
  `bindings.rs::ps2` module (16 digital buttons — DualShock 2: PSX
  shape + L3/R3 stick clicks; dual analog sticks via shared infra).
  Default core LRPS2 (PCSX2). Media + rom_hashes arms (NO_DAT_SYSTEMS
  for DVD images). `check_ps2_bios` + `PS2_BIOS_KNOWN_HASHES` (6
  entries covering JP launch / US fat / US-EU slim variants). **CD-
  launch BIOS dispatch arm now covers 9 CD-shape systems** (pce-cd /
  segacd / saturn / psx / neocd / 3do / pcfx / dreamcast / ps2).
  CSS theme: deep PS2 cobalt `oklch(0.45 0.22 215)`. Per-core docs
  scaffold.
- **Almost:** Phase 1 operator validation.
- **Next:** Operator drops `pcsx2_libretro.dll` + a regional PS2 BIOS,
  marks a PS2 folder via Import Wizard, launches Shadow of the
  Colossus / MGS2 / GTA III.
