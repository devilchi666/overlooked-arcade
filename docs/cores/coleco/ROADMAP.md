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

- 🟨 Per-game keypad reference + note — split:
  - ✅ `keypad_layout_note` text-area in the per-game Input dialog
    ("KP1=climb-up, KP2=climb-down, …") — operator-recorded
    documentation for the active game's overlay.
  - ✅ Visual keypad reference panel
    (`frontend/src/platform/components/KeypadReference.tsx`, 2026-05-27) —
    renders the physical 3×4 button layout next to the note with
    each KP labeled by its current per-system keyboard / gamepad
    mapping. Bridges "the note says KP1" to "my physical key 'Q'
    fires KP1." Coleco-only today; Intv shares the 3×4 shape and
    can adopt the same component with different button names.
  - ⬜ **Per-game bindings override** — stretch. Existing design
    intent (see `GameOverrides.keypad_layout_note` doc comment) is
    that key-to-keypad bindings stay system-wide; per-game
    documentation rides on the note + visual reference. Per-game
    bindings would need parallel `set_game_binding(gameId, …)`
    Tauri commands + per-game storage + launch-dispatch layering.
    Defer until an operator surfaces a real need for different
    keyboard mappings per game.
- ✅ Super Action Controller (the Coleco's deluxe controller with spinner + extra buttons) — closed by Phase A PADDLE / ANALOG device-type + Phase C mouse-as-stick X for the spinner. Operator playtest pending.
- ⬜ Coleco Adam computer mode — deferred (separate `adam` slug if ever onboarded).

---

## ⬜ Phase 3+ — Stretch

- ⬜ Cheat support via libretro cheat path — operator-driven validation.
- ✅ Roller Controller / Coleco-specific peripherals — closed by the same PADDLE + mouse-as-stick path as Super Action Controller.
