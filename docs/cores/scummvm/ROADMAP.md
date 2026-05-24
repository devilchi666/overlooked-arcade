# scummvm Roadmap

Per-core phase tracking for the ScummVM engine launcher.

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-24)

- ✅ `oa_core::SystemId::ScummVm` variant + `parse_system_id` arm (accepts `"scummvm"` + `"scumm"` alias).
- ✅ `apps/oa-shell/src/bindings.rs` — `scummvm` button module (8 bits: d-pad + LMB + RMB + ESCAPE + PAUSE), `SCUMMVM_BUTTONS` table, dispatch arms in `bit_for` / `buttons_for` / `to_libretro_bits` / `defaults_for`. Cross-system fixtures (default_keys_round_trip, dpad_lands, z_is_primary) extended.
- ✅ `default_core_dll_for_system("scummvm") → "scummvm_libretro.dll"`.
- ✅ `media::repos_for_system_id("scummvm") → ["ScummVM"]`.
- ✅ `rom_hashes::libretro_dat_refs_for_system("scummvm") → []` (engine launcher, no canonical SHA-1 set; fuzzy filename match at the 0.95 threshold covers cover sync).
- ✅ `art_pack_importer::launchbox_platform_to_system_id("ScummVM") → "scummvm"` + dedicated test.
- ✅ `system_settings::default_keyboard_passthrough("scummvm") → true` (sword-fighting insults, password prompts).
- ✅ Per-core `system_dir` override at `LibretroCore::load` — scummvm gets `<exe_dir>/system/scummvm/` instead of the install-wide `<exe_dir>/system/` (engine plugins / themes / extra-files have a stable home; console BIOS folder stays clean).
- ✅ `is_descriptor_extension(".scummvm") → true` — routes the descriptor through `RomSource::Path` so the core opens game data relative to the descriptor's directory.
- ✅ Frontend `SystemId` union extended, `systemThemes` entry added (extensions `[".scummvm"]`, formFactor `"computer"`, manufacturer `"other"`, tileAspect `"1/1"`, defaultShaderPreset `"plain"`), CSS theme block (teal-cyan `oklch(0.62 0.16 195)` — adventure-game ocean / dialogue-box atmosphere).

**Acceptance gate:** Phase 1 operator validation pending — drop `scummvm_libretro.dll` into the install, drop a real game + `.scummvm` descriptor into a library folder, launch.

---

## ⬜ Phase 1 — Operator validation

- ⬜ Drop `scummvm_libretro.dll` (from buildbot or community builds) into `<exe_dir>/cores/` — operator-driven.
- ⬜ Drop a game data directory + sibling `.scummvm` descriptor into a library folder; scan via the Import Wizard — operator-driven. Suggested validation set:
  - **Monkey Island** (`monkey:scumm`) — flagship SCUMM title; covers the sword-fighting insults path (keyboard passthrough).
  - **Day of the Tentacle** (`tentacle:scumm`) — second SCUMM mainstay; pixel-art adventure shown at native resolution.
  - **Lure of the Temptress** (`lure:lure`) — Revolution Software engine, a non-SCUMM coverage point.
- ⬜ Verify cover sync from the `ScummVM` libretro-thumbnails repo lands at canonical paths (`media/scummvm/box-front/<rom_stem>.png`).
- ⬜ Verify keyboard input reaches the engine (type a Monkey Island sword-fighting insult; expect the engine to recognize the line).

---

## ⬜ Phase 2 — Hardening

- ⬜ Per-game `scummvm_extra_path` core-option override for titles needing engine helper files (FOTAQ DAT, Lure of the Temptress overlays, fan-translation patches). The current implementation relies on the engine finding game data next to the descriptor; some specialty titles want the operator to point at an extra-path explicitly.
- ⬜ ScummVM `--detect` auto-detector wrapper — wraps the upstream tool so operators can drop raw game directories and OA generates the `.scummvm` descriptors automatically. Defer: v1 expects operators (or LaunchBox's ScummVM importer) to create the descriptors manually.
- ⬜ Per-game core-options templates for the long tail of engine tuning knobs (subtitle speed, music driver selection, MT-32 vs AdLib for SCUMM-era titles).
