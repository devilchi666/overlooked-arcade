# dosbox Session Log

Date + Shipped / Almost / Next.

---

## 2026-05-24 — Phase 0 onboarding

- **Shipped:** Branch `feat/dosbox-onboarding` Phase 2 of the `feat/dosbox-and-scummvm` plan — `oa_core::SystemId::DosBox` + parse + bindings module (12 buttons: d-pad + A/B/X/Y face diamond + L/R shoulders + START + SELECT) + default core dll + repo mapping + empty rom_hashes refs + LaunchBox `MS-DOS`/`DOS` art-pack mappings + keyboard passthrough on + per-core `system_dir` override (`<exe_dir>/system/dosbox/`) + new `is_directory_path_system` helper routing directory launches through `RomSource::Path` (all three launch sites) + new `GameOverrides.dosbox_entry_point` field wired at the `launch_rom` Tauri command + new `scan_service::run_dir_scan_blocking` directory-mode walker + new `start_background_directory_scan` Tauri command + frontend `DIR_MODE_SYSTEMS` constant with `(folder)` sentinel pattern + Import Wizard's `startScan` firing both extension-mode + directory-mode scans concurrently and merging results + classification preferring backend `systemHint` over extension lookup + frontend theme (amber-on-black 55° at L=0.65 — DOS prompt CRT amber palette) + per-core docs scaffold. Cross-system bindings test fixtures extended (default_keys_round_trip, dpad_lands, z_is_primary, to_libretro_bits_dispatches). New dosbox-specific tests: `defaults_cover_every_dosbox_button`, `dosbox_remap_is_identity`, `launchbox_platform_maps_dosbox`.
- **Almost:** Phase 1 operator validation — drop `dosbox_pure_libretro.dll` into `<exe_dir>/cores/`, drop a game directory (e.g. `Doom/` with `DOOM.EXE` + `DOOM.WAD`) into a library folder, mark that folder with a `dosbox` rule via the Import Wizard, scan, launch.
- **Next:** After dosbox validates, the final cleanup phase = ACTIVE_WORK / NEXT / VISION docs updates, then merge feat/dosbox-onboarding to main.
