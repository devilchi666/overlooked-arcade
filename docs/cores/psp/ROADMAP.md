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

- ⬜ Operator validation: God of War: Chains of Olympus / Crisis
  Core: FFVII / Patapon / Metal Gear Solid: Peace Walker.
- ⬜ Analog stick smoke-test.
- ⬜ Save state F5/F8 round-trip.

---

## ⬜ Phase 2 — Polish

- ⬜ PSP Go second analog stick support (rare hardware).
- ⬜ PPSSPP-specific core options surface (texture upscaling,
  internal resolution multiplier).

---

## 2026-05-21 — Stale-cleanup audit

The Phase 1+ items above were written when this system onboarded, before cross-system infrastructure (Phases 1.5 / 2.5–2.8 / 3 / 4 + direct-launch CLI) landed. Many `⬜` items are actually shipped — see `docs/cores/AUDIT_2026-05-21.md` for the per-item breakdown (stale vs open-code vs open-operator) for this system.
