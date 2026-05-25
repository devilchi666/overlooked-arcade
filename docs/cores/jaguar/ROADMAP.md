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

- ✅ **Keyboard-event dispatch for KP8-KP_HASH** — bits 16-20 in the polled mask (KP8 / KP9 / KP_STAR / KP0 / KP_HASH) now dispatch to Virtual Jaguar through `retro_keyboard_event_t` as RETROK_KP8 / KP9 / KP_MULTIPLY / KP0 / HASH respectively. Per-port edge detection in the emu thread (`apps/oa-shell/src/main.rs` Port0 input slice) only fires on transitions; gated on `current_system_id == "jaguar"` + `core.has_keyboard_callback()` so the path costs ~one mask compare per frame for other systems. Translation lives in `bindings::jaguar_high_bit_to_retro_key`; 4 new unit tests lock the mapping table + bit-mask hygiene. **Operator validation pending:** confirm Virtual Jaguar registers a keyboard callback and that Iron Soldier weapon-select (KP1-KP9) round-trips end-to-end.
- ✅ `jagboot.rom` BIOS pre-check — closed by `check_jaguar_bios` in `apps/oa-shell/src/main.rs` + dispatch arm for `jaguar` system_id. Recognizes `jagboot.rom` (and `jaguar_boot.rom` alt name) with libretro-database canonical SHA-1 (10B36AE9B3942D2B7BD5F77F61E51E16AA1B5DE5); blocks launch + toasts when missing (Virtual Jaguar won't initialize without it).
- ✅ Per-system shader override — closed by cross-system per-system shader override (slice 2.8.C + shader pipeline). Jaguar-specific scanline profile is operator-driven preset choice.

---

## ⬜ Phase 3+ — Stretch

- ⬜ **Jaguar CD support** — deferred (separate load path + BIOS, potentially `jaguar-cd` split).
