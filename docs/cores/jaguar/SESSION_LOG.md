# jaguar Session Log

---

## 2026-05-20 — Phase 0 onboarding (paired with 3do + pcfx)

- **Shipped:** SystemId variant, parse_system_id arm (`jaguar | jag |
  atari-jaguar`), `bindings.rs::jaguar` module (**21-button** including
  full 12-key numpad — operator overrode the recommended 8-button
  Phase 0 in favor of full numpad coverage; KP1-KP7 in RetroPad bits,
  KP8/KP9/KP_STAR/KP0/KP_HASH in shell-reserved high bits for keyboard
  binding via per-system page, Phase 2 wires the libretro KEYBOARD
  device dispatch for those). All 4 dispatch arms wired; 3 tests lock
  the dispatch including the high-bit-masking case. Default core
  Virtual Jaguar. Media + rom_hashes arms (no-intro Atari Jaguar dat).
  CSS theme: saturated gold 65° L=0.65 C=0.22 in the open 65-75°
  Atari-warm-zone band. Per-core docs scaffold.
- **Almost:** Phase 1 operator validation.
- **Next:** Operator drops `virtualjaguar_libretro.dll`, scans Jaguar
  ROMs, launches Iron Soldier / Tempest 2000 / Rayman. Numpad weapon-
  select on Iron Soldier is the canonical "does the full keypad work"
  test (KP1-KP7 via pad shoulders or keyboard Key1-Key7; KP8-KP_HASH
  via keyboard direct only at Phase 0).
