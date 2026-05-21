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

- ⬜ Operator validation: Iron Soldier / Tempest 2000 / Rayman / Alien vs Predator / Doom Jaguar — operator playtest.
- ⬜ Numpad-using game validation (Iron Soldier weapon select) — operator playtest.
- ✅ Save state F5/F8 round-trip — closed by cross-system save-state infra (`oa_libretro::LibretroCore::save_state / load_state`).

---

## ⬜ Phase 2 — Polish

- ⬜ **Keyboard-passthrough dispatch for KP8-KP_HASH** — the upper 5 keypad keys surface in the per-system Bindings page but don't reach Virtual Jaguar (keyboard-passthrough infra is shipped cross-system; the libretro KEYBOARD device dispatch for these high-bit entries is still ⬜).
- ⬜ `jagboot.rom` BIOS pre-check — Jaguar-specific cart-shape BIOS pre-check still ⬜ (cart-shape BIOS-check infra is shipped cross-system).
- ✅ Per-system shader override — closed by cross-system per-system shader override (slice 2.8.C + shader pipeline). Jaguar-specific scanline profile is operator-driven preset choice.

---

## ⬜ Phase 3+ — Stretch

- ⬜ **Jaguar CD support** — deferred (separate load path + BIOS, potentially `jaguar-cd` split).
