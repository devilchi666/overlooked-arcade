# intv — Roadmap

Per-core phase tracking for Mattel Intellivision. Status: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-19)

- ✅ `oa_core::SystemId::Intellivision` variant.
- ✅ `parse_system_id("intv" | "intellivision") → Intellivision`.
- ✅ `default_core_dll_for_system("intv") → "freeintv_libretro.dll"`.
- ✅ `bindings.rs::intv` — 10-button layout, identity remap, dispatch.
- ✅ `media::repo_for_system_id("intv") → "Mattel_-_Intellivision"`.
- ✅ `rom_hashes::libretro_dat_refs_for_system("intv") → metadat/no-intro/Mattel - Intellivision`.
- ✅ Frontend `systemThemes.intv` (extension `["int"]`, portrait 3/4, crt-lite).
- ✅ Theme CSS — deep Mattel navy hue 260° / L=0.50 / C=0.17.
- ✅ Per-core docs scaffold.

**Acceptance gate:** Operator drops `freeintv_libretro.dll` + `exec.bin` + `grom.bin` BIOS files, scans Intv folder, sees navy-themed tiles, launches a game.

---

## ⬜ Phase 1 — First Intv ROM running

- ⬜ Operator validation: **Astrosmash**, **Utopia**, **Snafu**, **Star Strike**, **Major League Baseball**, **B-17 Bomber** — operator playtest.
- ⬜ BIOS pre-check workflow — operator validation (cart-shape BIOS-check infra shipped cross-system).
- ⬜ 4 side-button bindings — operator playtest of UPPER vs LOWER mapping per-game.
- ✅ Save state F5/F8 round-trip — closed by cross-system save-state infra (`oa_libretro::LibretroCore::save_state / load_state`).
- ✅ libretro-database hash matching + cover sync — closed by cross-system hash ID (`rom_hashes::resolve_rom_hashes_for_system`) + media sync (`media::sync_media_for_system`).

---

## ✅ Phase 2 — Polish

- ✅ Full 12-button keypad coverage — shipped via `bindings.rs::intv` (KP0-KP9 occupy bits 1-19); `INTV_BUTTONS` surfaces all 20 entries; `default_intv_bindings` binds KP0-KP9 to Key0-Key9.
- ✅ 16-direction disc analog input — closed by Phase A PADDLE / ANALOG device-type + Phase C mouse-as-stick XY routing (`MouseSource::Xy`). Operator picks "Analog / Paddle" + mouse XY in per-game Input. Operator playtest pending.
- ⬜ Intellivoice voice-synthesis module support — operator validation of FreeIntv's voice module.
- ⬜ Intellivision Computer Module / ECS expansion — deferred (niche).

---

## ⬜ Phase 3+ — Stretch

- ⬜ Cheat support via libretro cheat path — operator-driven validation.
- ⬜ INTV Music Synthesizer (the ECS music synthesizer peripheral) — deferred.
