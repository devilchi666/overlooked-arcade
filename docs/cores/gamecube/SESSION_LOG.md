# gamecube Session Log

---

## 2026-05-20 — Phase 0 onboarding (paired with n64)

- **Shipped:** SystemId variant (GameCube), parse_system_id arm
  (single slug covers both GC + Wii via Dolphin auto-detect),
  `bindings.rs::gamecube` module (12 digital buttons — d-pad + A/B/X/Y
  face + L/R/Z + START; main stick + C-stick flow through analog
  axes). Default core Dolphin. Media + rom_hashes arms
  (NO_DAT_SYSTEMS for large multi-format disc images). CSS theme:
  Indigo GameCube oklch(0.48 0.22 280) — deep premium indigo in the
  Nintendo home-console violet cluster. Per-core docs scaffold.
- **Almost:** Phase 1 operator validation. Gamepad with dual analog
  sticks essential for proper GC experience (main stick + C-stick).
  Wii Remote / motion-controls not yet wired.
- **Next:** Operator drops `dolphin_libretro.dll`, scans GC + Wii
  ISOs, launches Super Smash Bros. Melee. The first test is whether
  the LeftStick smoothly moves the character AND the RightStick fires
  smash attacks via C-stick — that confirms both analog axes flow
  end-to-end.
