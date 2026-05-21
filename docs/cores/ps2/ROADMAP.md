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

- ⬜ Operator validation: Shadow of the Colossus / MGS2 / GTA III / FFX — operator playtest.
- ✅ Dual analog stick test — closed by cross-system analog axes (`InputState.axes` + `compute_stick_output` with keyboard fallback + deadzone + sensitivity).
- ✅ Save state F5/F8 round-trip — closed by cross-system save-state infra (`oa_libretro::LibretroCore::save_state / load_state`).

---

## ⬜ Phase 2 — Polish

- ✅ Pressure-sensitive face buttons + analog L2/R2 — closed by Phase B per-button analog pressure. Same shared infra closed GC + DC. Operator playtest pending (Gran Turismo 3/4 brake-feel, Metal Gear Solid 2/3 prone-walk).
- ✅ DualShock 2 rumble — closed by Phase F rumble interface.
- ⬜ PS2 memory card UX — operator-driven validation.
- ✅ Disc-id extraction via SYSTEM.CNF — shipped via `apps/oa-shell/src/cd_id.rs::dispatch_extractor` routing `ps2` → `extractors::psx_family`; `rom_hashes` points at `metadat/redump/Sony - PlayStation 2`.
- ⬜ LRPS2 core-options surface (graphics renderer, FPS limit) — operator-driven curation via per-system settings page (per-system Core Options shipped cross-system).
