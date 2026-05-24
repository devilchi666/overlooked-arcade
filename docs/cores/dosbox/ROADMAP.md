# dosbox Roadmap

Per-core phase tracking for DOSBox (libretro `dosbox_pure_libretro.dll`).

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-24)

- ✅ `oa_core::SystemId::DosBox` variant + `parse_system_id` arm (accepts `"dosbox"` / `"dos"` / `"ms-dos"` / `"msdos"` / `"dosbox-pure"`).
- ✅ `apps/oa-shell/src/bindings.rs` — `dosbox` button module (12 bits: d-pad + A/B/X/Y diamond + L/R shoulders + START + SELECT), `DOSBOX_BUTTONS` table, dispatch arms in `bit_for` / `buttons_for` / `to_libretro_bits` / `defaults_for`. Cross-system fixtures (default_keys_round_trip, dpad_lands, z_is_primary, to_libretro_bits_dispatches) extended.
- ✅ `default_core_dll_for_system("dosbox") → "dosbox_pure_libretro.dll"`.
- ✅ `media::repos_for_system_id("dosbox") → ["DOS"]`.
- ✅ `rom_hashes::libretro_dat_refs_for_system("dosbox") → []` (engine launcher, no canonical SHA-1 set; fuzzy filename match at the 0.95 threshold against directory basename covers cover sync). Listed in `NO_DAT_SYSTEMS` with the reason.
- ✅ `art_pack_importer::launchbox_platform_to_system_id` — both `"MS-DOS"` (modern) and `"DOS"` (legacy) route to `"dosbox"` + dedicated test.
- ✅ `is_directory_path_system("dosbox") → true` — new helper alongside `is_descriptor_extension`. Routes dosbox launches through `RomSource::Path` with the directory path; covers all three launch-dispatch sites (startup OA_ROM, EmuCommand::LoadRom, launch_rom Tauri command).
- ✅ `system_dir_for` extended — dosbox gets `<exe_dir>/system/dosbox/` (config cache, save states, screenshots) instead of the install-wide BIOS folder.
- ✅ Directory-mode scanner: `scan_service::run_dir_scan_blocking` walks 1-level-deep for subdirectories, emits one `ScannedRom` per subdir with `systemHint = "dosbox"`. New Tauri command `start_background_directory_scan(folder, systemId)` parallels the existing extension-mode `start_background_scan`.
- ✅ `GameOverrides.dosbox_entry_point: Option<String>` — per-game entry-point override for the ~10% of DOS games where dosbox-pure's auto-detect picks the wrong .exe. Wired at the `launch_rom` Tauri command: when set, the path passed to `retro_load_game` becomes `<game_dir>/<dosbox_entry_point>` instead of just `<game_dir>`.
- ✅ Frontend `SystemId` union extended, `systemThemes` entry added (extensions `[]` since scan is directory-mode, formFactor `"computer"`, manufacturer `"other"`, tileAspect `"4/3"`, defaultShaderPreset `"crt-lite"`), CSS theme block (amber-on-black `oklch(0.65 0.18 55)` — DOS prompt CRT amber phosphor palette).
- ✅ Import Wizard wiring: `DIR_MODE_SYSTEMS = ["dosbox"]` constant; default rules include a sentinel `(folder)` pattern → `dosbox`. `startScan` fires both extension-mode + directory-mode scans concurrently, merging results. Bucketing logic prefers `systemHint` from the backend over extension-based classification.

**Acceptance gate:** Phase 1 operator validation pending — drop `dosbox_pure_libretro.dll` into the install, drop a real game directory (e.g. `Doom/` with `DOOM.EXE` + `DOOM.WAD`) into a library folder, mark the folder as `dosbox` in the Import Wizard, scan, launch.

---

## ⬜ Phase 1 — Operator validation

- ⬜ Drop `dosbox_pure_libretro.dll` (from buildbot or community builds) into `<exe_dir>/cores/` — operator-driven.
- ⬜ Drop a game directory into a library folder; mark via the Import Wizard rules editor (the `(folder)` row → `dosbox`) — operator-driven. Suggested validation set:
  - **Doom** (auto-detect) — flagship action title; validates the gamepad action layout.
  - **Wing Commander** (auto-detect or via `dosbox_entry_point = "WC.EXE"` if needed) — joystick + keyboard mix.
  - **X-COM UFO Defense** (auto-detect, mouse-driven via shared POINTER infra).
  - **A game where auto-detect picks the wrong .exe** — validates the `dosbox_entry_point` override path. SimCity 2000 (auto-picks `INSTALL.EXE` instead of `SC2000.EXE`) is a classic test case.
- ⬜ Verify cover sync from the `DOS` libretro-thumbnails repo lands at canonical paths (`media/dosbox/box-front/<game_dir_basename>.png`).
- ⬜ Verify keyboard passthrough reaches the engine (typed commands in Sierra's classic AGI/SCI titles, if any DOS-shape SCI games are tested).

---

## ⬜ Phase 2 — Hardening

- ⬜ Per-game core-options templates for the long tail of dosbox-pure tuning knobs (CPU cycle target — `cycles=fixed N` vs `cycles=max`; sound card emulation — SB16 / GUS / MT-32 / Adlib; expanded vs extended memory; cycle-up/cycle-down hotkeys for operators running cycle-sensitive titles). The existing per-game core-options drawer (slice 2.8.D) renders dosbox-pure's option schema automatically; this is a curation pass to ship sensible templates per popular game.
- ⬜ Per-game `dosbox.conf` editor — surface inline editing of the game-directory `dosbox.conf` so operators don't have to alt-tab to a text editor for tuning. Defer: operators can hand-edit today; an in-app conf editor is a stretch UX polish.
- ⬜ Watcher integration — file-watcher currently keys on file events; a new directory added under a dosbox-marked folder should trigger a directory-mode rescan. Today operators trigger a manual rescan via the Import Wizard.
- ⬜ ScanMode enum unification — when more directory-mode engine launchers land, fold the parallel scan paths (`run_scan_blocking` + `run_dir_scan_blocking`) into a single dispatcher. The two functions are independent today (Phase 0 keeps them separate for review simplicity); the consolidation is a follow-up refactor.
