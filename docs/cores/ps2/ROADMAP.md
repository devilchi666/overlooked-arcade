# ps2 — Roadmap

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-20)

- ✅ `oa_core::SystemId::Ps2` variant + parse_system_id arm.
- ✅ `bindings.rs::ps2` module — 16 digital buttons (DualShock 2
  shape: PSX + L3/R3). Dual analog sticks via shared infra.
- ✅ `default_core_dll_for_system("ps2") → "pcsx2_libretro.dll"`.
- ✅ `rom_hashes` → `&[]` with NO_DAT_SYSTEMS entry.
- ✅ `media::repo_for_system_id` → `Sony_-_PlayStation_2`.
- ✅ `check_ps2_bios` + `PS2_BIOS_KNOWN_HASHES` (6 canonical entries
  spanning JP launch / US fat / US-EU slim variants). Slotted into
  CD-launch dispatch as 9th CD-shape system.
- ✅ Frontend SystemId / systemThemes / CSS (Plan A: deep PS2
  cobalt `oklch(0.45 0.22 215)`).
- ✅ Per-core docs scaffold.

---

## ⬜ Phase 1 — First PS2 game running

- ⬜ Operator validation: Shadow of the Colossus / MGS2 / GTA III / FFX.
- ⬜ Dual analog stick test.
- ⬜ Save state F5/F8 round-trip.

---

## ⬜ Phase 2 — Polish

- ⬜ Pressure-sensitive face buttons + analog L2/R2 (Phase 2.5 work
  shared with GameCube's analog L/R triggers).
- ⬜ PS2 memory card UX.
- ⬜ Disc-id extraction via SYSTEM.CNF (same as PSX).
- ⬜ LRPS2 core-options surface (graphics renderer, FPS limit).
