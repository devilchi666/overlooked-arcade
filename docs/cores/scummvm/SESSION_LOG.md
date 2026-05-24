# scummvm Session Log

Date + Shipped / Almost / Next.

---

## 2026-05-24 — Phase 0 onboarding

- **Shipped:** Branch `feat/dosbox-and-scummvm` Phase 1 — `oa_core::SystemId::ScummVm` + parse + bindings module (8 buttons; d-pad + LMB + RMB + ESCAPE + PAUSE) + default core dll + repo mapping + empty rom_hashes refs + LaunchBox art-pack mapping + keyboard passthrough on + per-core `system_dir` override (`<exe_dir>/system/scummvm/`) + `is_descriptor_extension` helper routing `.scummvm` through `RomSource::Path` + frontend theme (teal-cyan 195° at L=0.62 — adventure-game ocean / dialogue-box palette) + per-core docs scaffold. Cross-system bindings test fixtures extended (default_keys_round_trip, dpad_lands, z_is_primary, to_libretro_bits_dispatches). New scummvm-specific tests: `defaults_cover_every_scummvm_button`, `scummvm_remap_is_identity`, `launchbox_platform_maps_scummvm`.
- **Almost:** Phase 1 operator validation — drop `scummvm_libretro.dll` into `<exe_dir>/cores/`, drop a `Monkey Island/` directory + sibling `Monkey Island.scummvm` (line: `monkey:scumm`) into a library folder, scan, launch.
- **Next:** After scummvm validates, Phase 2 = DOSBox onboarding (Phase 2 of the `feat/dosbox-and-scummvm` branch).
