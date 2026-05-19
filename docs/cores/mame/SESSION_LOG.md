# mame Session Log

Append-only. Newest at bottom. Three lines per entry: **Shipped / Almost / Next.**

---

## 2026-05-19 — Onboarded

- **Shipped:** MAME / Arcade wired as a first-class system. Added `oa_core::SystemId::Mame`, `mame` slug + theme in the frontend registry (extensions `.zip` + `.chd`, neon arcade red accent at hue 12°), `mame` button module in `bindings.rs` (12 buttons: 6 face buttons B1-B6 + START + COIN + d-pad, identity-mapped to libretro RetroPad), `default_mame_bindings()` (Z/X/A/S/Q/W on the keyboard for B1-B6; Key1/Key5 for Start/Coin per RetroArch convention), dispatch arms in `bit_for` / `buttons_for` / `to_libretro_bits` / `defaults_for`, `default_core_dll_for_system("mame") → "mame_libretro.dll"`, `parse_system_id("mame" | "arcade")`. Renamed catalog slug `arcade-mame` → `mame` (5 MAME variants) and `arcade-fbneo` → `fbneo` for symmetry. Two new bindings tests + four expanded existing tests; 14/14 green. Operator already has `mame_libretro.dll v0.287` installed.
- **Almost:** Real ROM launch validation. Per-game ROM-set name resolution (today's tile shows `pacman.zip`, not `Pac-Man`).
- **Next:** Validate a known-good ROM set against MAME 0.287; suggest pacman.zip or sf2ce.zip.
