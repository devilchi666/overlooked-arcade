# coleco Session Log

Per-core Shipped / Almost / Next log for ColecoVision. Project-wide log lives at `docs/SESSION_LOG.md`.

---

## 2026-05-19 — Phase 0 onboarding

- **Shipped:** `bindings.rs::coleco` 16-button module (D-pad + L_FIRE + R_FIRE + KP0..KP9), identity remap, all dispatch arms. `default_core_dll_for_system("coleco") → "bluemsx_libretro.dll"`. `media::repo_for_system_id` + `rom_hashes::libretro_dat_refs_for_system` arms. Frontend `systemThemes.coleco` (extensions `["col", "cv"]`, portrait 3/4, crt-lite) + `[data-system="coleco"]` block (bright cyan 195° / L=0.72 / C=0.16). Per-core docs scaffold.
- **Almost:** Phase 1 operator validation — needs `.col` ROM launched end-to-end (Donkey Kong, Zaxxon, Lady Bug). BIOS pre-check workflow.
- **Next:** Operator installs `bluemsx_libretro.dll` + `coleco.rom` BIOS, scans Coleco folder, confirms cyan-themed tiles, launches a known-good ROM.
