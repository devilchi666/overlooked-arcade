# psp Session Log

---

## 2026-05-20 — Phase 0 onboarding (paired with ps2 + nds)

- **Shipped:** SystemId variant (Psp), parse_system_id arm,
  `bindings.rs::psp` module (12 digital buttons; no L2/R2 — PSP
  hardware lacks them; single analog stick via shared analog infra).
  Default core PPSSPP. Media + rom_hashes arms (no-intro PSP dat).
  CSS theme: Sony cool cyan `oklch(0.65 0.18 200)` (member of the
  Sony cluster psx 180° / psp 200° / ps2 215°). No BIOS pre-check
  (PPSSPP is BIOS-free). Per-core docs scaffold.
- **Almost:** Phase 1 operator validation.
- **Next:** Operator drops `ppsspp_libretro.dll`, connects a gamepad
  with analog stick, launches God of War: Chains of Olympus.
