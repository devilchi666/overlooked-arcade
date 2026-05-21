# psp — Roadmap

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-20)

- ✅ `oa_core::SystemId::Psp` variant + parse_system_id arm.
- ✅ `bindings.rs::psp` module — 12 digital buttons + analog stick via
  shared infra. No L2/R2 (PSP hardware lacks them).
- ✅ `default_core_dll_for_system("psp") → "ppsspp_libretro.dll"`.
- ✅ `rom_hashes` → no-intro PSP dat (.iso/.cso/.pbp single-file).
- ✅ `media::repo_for_system_id` → `Sony_-_PlayStation_Portable`.
- ✅ Frontend SystemId / systemThemes / CSS (Plan A: Sony cool cyan
  `oklch(0.65 0.18 200)`).
- ✅ No BIOS pre-check needed (PPSSPP is BIOS-free).
- ✅ Per-core docs scaffold.

---

## ⬜ Phase 1 — First PSP game running

- ⬜ Operator validation: God of War: Chains of Olympus / Crisis Core: FFVII / Patapon / Metal Gear Solid: Peace Walker — operator playtest.
- ✅ Analog stick smoke-test — closed by cross-system analog axes (`InputState.axes` + `compute_stick_output` with keyboard fallback + deadzone + sensitivity).
- ✅ Save state F5/F8 round-trip — closed by cross-system save-state infra (`oa_libretro::LibretroCore::save_state / load_state`).

---

## ⬜ Phase 2 — Polish

- ⬜ PSP Go second analog stick support — deferred (rare hardware).
- ⬜ PPSSPP-specific core options surface (texture upscaling, internal resolution multiplier) — operator-driven curation via per-system Core Options page (per-system Core Options shipped cross-system).
