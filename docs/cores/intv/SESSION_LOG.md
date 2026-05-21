# intv Session Log

Per-core Shipped / Almost / Next log for Mattel Intellivision. Project-wide log lives at `docs/SESSION_LOG.md`.

---

## 2026-05-19 — Phase 0 onboarding

- **Shipped:** `oa_core::SystemId::Intellivision` variant. `bindings.rs::intv` 10-button module (D-pad disc-as-8-way + 4 side action buttons + START/SELECT). `default_core_dll_for_system("intv") → "freeintv_libretro.dll"`. `media::repo_for_system_id` + `rom_hashes::libretro_dat_refs_for_system` arms. Frontend `systemThemes.intv` (extension `["int"]`, portrait 3/4, crt-lite) + `[data-system="intv"]` block (deep Mattel navy 260° / L=0.50 / C=0.17 — period-correct, lightness-axis separation from SNES violet + Genesis cobalt). Per-core docs.
- **Almost:** Phase 1 operator validation. Needs `.int` ROM launched (Astrosmash, Utopia, Snafu good test cases). BIOS pre-check workflow.
- **Next:** Operator installs `freeintv_libretro.dll` + `exec.bin` + `grom.bin`, scans Intv folder, confirms navy-themed tiles, launches a known-good ROM.
