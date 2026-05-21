# virtualboy Session Log

Per-core Shipped / Almost / Next log for Nintendo Virtual Boy. Project-wide log lives at `docs/SESSION_LOG.md`.

---

## 2026-05-20 — Phase 0 onboarding

- **Shipped:** `bindings.rs::virtualboy` 10-button module (LEFT D-pad + A + B + L + R + START + SELECT). `default_core_dll_for_system("virtualboy") → "mednafen_vb_libretro.dll"`. `rom_hashes` arm. Frontend `systemThemes.virtualboy` (extension `["vb"]`, landscape 4/3, **`plain` shader** — see DECISIONS) + `[data-system="virtualboy"]` block (deep VB red 7° / L=0.55 / C=0.26 — period-correct LED red, distinct from MAME scarlet + NES red by lightness + chroma). Per-core docs.
- **Almost:** Phase 1 operator validation — single-D-pad VB titles (Mario's Tennis, V-Tetris, Wario Cruise). Anaglyph 3D mode spot-check via Beetle VB core options.
- **Next:** Operator installs `mednafen_vb_libretro.dll` (no BIOS needed — VB never had one), scans VB folder, confirms VB-red themed tiles, launches a known-good single-D-pad ROM. Dual-D-pad games (Mario Clash, Wario Land VB, Teleroboxer, Red Alarm, Vertical Force) playable single-D-pad-only until Phase 2 right-D-pad work lands.
