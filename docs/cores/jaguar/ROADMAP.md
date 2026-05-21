# jaguar — Roadmap

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-20)

- ✅ `oa_core::SystemId::Jaguar` variant + `parse_system_id` arm.
- ✅ `bindings.rs::jaguar` module — 21-button pad with 12-key numpad,
  upper 5 keypad keys in shell-reserved high bits above RetroPad.
- ✅ `default_core_dll_for_system("jaguar") → "virtualjaguar_libretro.dll"`.
- ✅ `rom_hashes` → no-intro Atari Jaguar dat.
- ✅ `media::repo_for_system_id` → `Atari_-_Jaguar`.
- ✅ Frontend SystemId / systemThemes / CSS (Plan A: saturated gold 65°,
  open 65-75° band between 2600 and A7800).
- ✅ Per-core docs scaffold.

**Acceptance gate:** Operator drops `virtualjaguar_libretro.dll`,
scans Jaguar ROMs, sees gold-themed tiles, launches a known-good ROM.

---

## ⬜ Phase 1 — First Jaguar game running

- ⬜ Operator validation: Iron Soldier / Tempest 2000 / Rayman /
  Alien vs Predator / Doom Jaguar.
- ⬜ Numpad-using game validation (Iron Soldier weapon select).
- ⬜ Save state F5/F8 round-trip.

---

## ⬜ Phase 2 — Polish

- ⬜ **Keyboard-passthrough dispatch for KP8-KP_HASH** — the upper 5
  keypad keys currently surface in the per-system Bindings page but
  don't reach Virtual Jaguar via RetroPad (only KP1-KP7 do). Phase 2
  wires libretro KEYBOARD device dispatch for these high-bit entries.
- ⬜ `jagboot.rom` BIOS pre-check (currently no pre-check; Phase 2
  polish adds one if operators surface BIOS-related issues).
- ⬜ Per-system shader override (Jaguar's polygon-heavy era benefits
  from a different scanline profile than the cart-2D systems).

---

## ⬜ Phase 3+ — Stretch

- ⬜ **Jaguar CD support.** A handful of CD-only Jaguar releases
  (Battlemorph, Vid Grid, Hover Strike CD, etc.). Different load
  path + BIOS requirement; would either share `jaguar` slug via
  per-game CD detection or split to `jaguar-cd`.
