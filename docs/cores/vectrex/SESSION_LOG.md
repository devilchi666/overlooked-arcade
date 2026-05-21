# vectrex Session Log

Per-core Shipped / Almost / Next log for GCE Vectrex. Project-wide log lives at `docs/SESSION_LOG.md`.

---

## 2026-05-20 — Phase 0 onboarding

- **Shipped:** `bindings.rs::vectrex` 8-button module (D-pad + B1/B2/B3/B4 in a horizontal-row 4-face-button layout), identity remap, all dispatch arms. `default_core_dll_for_system("vectrex") → "vecx_libretro.dll"`. `rom_hashes::libretro_dat_refs_for_system` arm. Frontend `systemThemes.vectrex` (extensions `["vec", "gam"]`, landscape 4/3, crt-lite) + `[data-system="vectrex"]` block (bright phosphor-green 165° / L=0.80 / C=0.16 — period-correct for the vector-display CRT). Per-core docs scaffold.
- **Almost:** Phase 1 operator validation. Mine Storm (BIOS pack-in), Berzerk, Star Trek good test cases.
- **Next:** Operator installs `vecx_libretro.dll` (+ optional `vectrex.bin` BIOS for Mine Storm pack-in), scans Vectrex folder, confirms phosphor-green themed tiles, launches a known-good ROM.
