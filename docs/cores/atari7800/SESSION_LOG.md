# atari7800 Session Log

Append-only. Newest at bottom. Three lines per entry: **Shipped / Almost / Next.**

---

## 2026-05-19 — Onboarded

- **Shipped:** Atari 7800 / ProSystem wired as a first-class system. `atari7800` slug + theme in the frontend registry (extension `.a78`, amber/gold accent at hue 80°), `atari7800` button module in `bindings.rs` (8 buttons: B1/B2 + 4-way d-pad + PAUSE + SELECT, identity-mapped to libretro RetroPad), `default_atari7800_bindings()` (Z/X for primary/secondary fire per the cross-system rule; Enter for Pause, RShift for Select per the 7800's hardware switches), dispatch arms in `bit_for` / `buttons_for` / `to_libretro_bits` / `defaults_for`, `default_core_dll_for_system("atari7800") → "prosystem_libretro.dll"`. Three new atari7800 tests + four expanded cross-system tests; 17/17 bindings tests green. `SystemId::Atari7800` + `parse_system_id("atari7800")` + the catalog `prosystem_libretro` entry were already in place from earlier scaffolding — this commit fills in the recipe gaps the previous pass left behind.
- **Almost:** Real ROM launch validation. Per-game ROM-set name resolution (today's tile shows `Asteroids (USA).a78`, not `Asteroids`).
- **Next:** Operator validation against a known-good ROM set. Suggested no-BIOS test path: drop `Asteroids (USA).a78` / `Centipede (USA).a78` / `Ms. Pac-Man (USA).a78` into the library, confirm launch + input. Then validate a BIOS-required title (Choplifter / Robotron 2084) with `7800 BIOS (U).rom` in `<exe_dir>/system/`.
