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

- ⬜ Operator validation: launch a real `.col` ROM. Suggested: **Donkey Kong**, **Zaxxon**, **Lady Bug**, **Carnival**, **Cosmic Avenger**, **Frenzy**, **Mouse Trap**.
- ⬜ BIOS pre-check workflow — confirm operator sees a clear error when `coleco.rom` is missing (similar shape to the PCE-CD syscard pre-check).
- ⬜ Keypad input validation — load a keypad-required title (Donkey Kong needs 1 for one-player, etc.) and confirm keypad keyboard bindings work.
- ⬜ Save state F5/F8 round-trip.
- ⬜ libretro-database hash matching.
- ⬜ Cover sync.

---

## ⬜ Phase 2 — Polish

- ⬜ Per-game keypad mode override — some games map keypad numbers to game actions; per-game core options surface for the operator.
- ⬜ Super Action Controller (the Coleco's deluxe controller with spinner + extra buttons). Niche, deferred.
- ⬜ Coleco Adam computer mode — blueMSX supports it but it's a different system; treat as a separate `adam` slug if ever onboarded.

---

## ⬜ Phase 3+ — Stretch

- ⬜ Cheat support via libretro cheat path.
- ⬜ Roller Controller / Coleco-specific peripherals — analog input dependency.

---

## 2026-05-21 — Stale-cleanup audit

The Phase 1+ items above were written when this system onboarded, before cross-system infrastructure (Phases 1.5 / 2.5–2.8 / 3 / 4 + direct-launch CLI) landed. Many `⬜` items are actually shipped — see `docs/cores/AUDIT_2026-05-21.md` for the per-item breakdown (stale vs open-code vs open-operator) for this system.
