# coleco — Roadmap

Per-core phase tracking for ColecoVision. Status: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-19)

- ✅ `oa_core::SystemId::Colecovision` (already existed from Phase 0 placeholder).
- ✅ `parse_system_id` arm covers `"coleco" | "colecovision"`.
- ✅ `default_core_dll_for_system("coleco") → "bluemsx_libretro.dll"`.
- ✅ `bindings.rs::coleco` — 16-button layout (D-pad + 2 fires + 10 keypad numbers), identity remap, `COLECO_BUTTONS` table, `default_coleco_bindings()`, all 4 dispatch arms.
- ✅ `media::repo_for_system_id("coleco") → "Coleco_-_ColecoVision"`.
- ✅ `rom_hashes::libretro_dat_refs_for_system("coleco") → metadat/no-intro/Coleco - ColecoVision`.
- ✅ Frontend `systemThemes.coleco` (extensions `["col", "cv"]`, portrait 3/4, crt-lite).
- ✅ Theme CSS — bright cyan hue 195° / L=0.72 / C=0.16.
- ✅ Per-core docs scaffold.

**Acceptance gate:** Operator drops `bluemsx_libretro.dll` + `coleco.rom` BIOS into the install, scans a Coleco folder, sees cyan-themed tiles, launches a game.

---

## ⬜ Phase 1 — First Coleco ROM running

- ⬜ Operator validation: launch a real `.col` ROM. Suggested: **Donkey Kong**, **Zaxxon**, **Lady Bug**, **Carnival**, **Cosmic Avenger**, **Frenzy**, **Mouse Trap** — operator playtest.
- ⬜ BIOS pre-check workflow — operator validation that the existing cart-shape pre-check surfaces a clear error when `coleco.rom` is missing (the BIOS-validation infra itself is shipped cross-system).
- ⬜ Keypad input validation — operator playtest of keypad bindings against keypad-required titles.
- ✅ Save state F5/F8 round-trip — closed by cross-system save-state infra (`oa_libretro::LibretroCore::save_state / load_state`).
- ✅ libretro-database hash matching — closed by cross-system hash ID (`rom_hashes::resolve_rom_hashes_for_system`).
- ✅ Cover sync — closed by cross-system media sync (`media::sync_media_for_system`).

---

## ⬜ Phase 2 — Polish

- ⬜ Per-game keypad mode override — **partial**: per-game `keypad_layout_note` field + drawer UI shipped in `GameOverrides`; full per-game bindings override still gated on per-game keypad-bindings work.
- ✅ Super Action Controller (the Coleco's deluxe controller with spinner + extra buttons) — closed by Phase A PADDLE / ANALOG device-type + Phase C mouse-as-stick X for the spinner. Operator playtest pending.
- ⬜ Coleco Adam computer mode — deferred (separate `adam` slug if ever onboarded).

---

## ⬜ Phase 3+ — Stretch

- ⬜ Cheat support via libretro cheat path — operator-driven validation.
- ✅ Roller Controller / Coleco-specific peripherals — closed by the same PADDLE + mouse-as-stick path as Super Action Controller.
