# dosbox Decisions

Per-core architectural decisions. Date entries.

---

## 2026-05-24 — Ship as an ordinary OA system, not a separate launcher

Locked in the [features/dosbox-and-scummvm/README.md](../../features/dosbox-and-scummvm/README.md) plan. DOSBox appears in the same sidebar / library / per-game settings model as console systems. No separate "PC games" surface, no new launcher app, no new top-level concept. The directory-based scan + path-based launch is contained inside the engine-launcher abstraction (parallels the scummvm path-based launch); everything downstream — cover art, save states, per-game overrides, audio overrides — works the same as for console games.

## 2026-05-24 — Directory = game; scan 1-level deep

DOSBox games are directories of multi-file content (executable + data files + optional `dosbox.conf`), not single ROM files. OA scans `<library_folder>` at exactly 1 nesting level deep — each direct subdirectory is one game; nested directories inside are content (game data), not nested games. This is the canonical "LaunchBox MS-DOS layout" — operators dropping LaunchBox-curated libraries into OA get the right shape automatically. The 1-level constraint sidesteps `.exe` collisions with Windows installers: the OA scanner never enumerates files at all in directory mode, only top-level subdirectories.

## 2026-05-24 — dosbox-pure as the default core

DOSBox-pure is the only mature libretro DOS option. Active development. Auto-config baked in (cycles, sound card, mount, expanded memory) so operators don't need to write per-game config from scratch. Ships its own DOSBox runtime — no external dependency on a standalone DOSBox install. Alternates (DOSBox Daum, DOSBox Staging) aren't packaged for the libretro buildbot; operator-driven core swap via the per-system Cores dialog still works if a community libretro port of those lands later.

## 2026-05-24 — Per-core `system_dir` subdirectory

DOSBox-pure caches per-game state (save states, config snapshots, screenshots) in its system dir. The install-wide `<exe_dir>/system/` is reserved for console BIOSes; mixing engine-launcher per-game caches there would clutter the BIOS folder. DOSBox gets `<exe_dir>/system/dosbox/` (created on first launch by `system_dir_for` in `apps/oa-shell/src/main.rs`), parallel to scummvm's `<exe_dir>/system/scummvm/`.

## 2026-05-24 — `RomSource::Path` with directory path

dosbox-pure accepts a directory path via `retro_load_game` and auto-detects the boot path by walking the directory. The shell's launch dispatch passes the directory string verbatim through `RomSource::Path` via the new `is_directory_path_system` helper — parallel to how CD images and `.scummvm` descriptors flow through `RomSource::Path` (the libretro core sets `need_fullpath = true` either way, so the OA dispatch is uniform path-based). No file-bytes round-trip; the core walks the disk directly.

## 2026-05-24 — `GameOverrides.dosbox_entry_point` for the auto-detect misses

dosbox-pure's auto-detect picks the right .exe ~90% of the time. The remaining ~10% are games where an install utility (`INSTALL.EXE`), DOS shell, or non-game .exe sits next to the real game binary — auto-detect picks the wrong one. The per-game override is a simple `Option<String>` relative path; when set, the launch path resolves to `<game_dir>/<dosbox_entry_point>` and dosbox-pure treats the explicit path as the boot target. Covers the long tail without needing operators to hand-write `dosbox.conf`.

## 2026-05-24 — Per-game `dosbox.conf` honored automatically

dosbox-pure reads `dosbox.conf` from the game directory at load time and applies its `[autoexec]` + per-game tuning (sound card, cpu cycles, memory). No OA wiring needed; the file just exists in the game directory and gets picked up. This is the LaunchBox / DOSBox Daum / DOSBox Staging migration path — operators with existing `dosbox.conf` tuning don't need to migrate it anywhere.

## 2026-05-24 — No BIOS check, no SHA-1 dat

DOSBox has no BIOS — the core ships its own runtime. Game data files vary enormously by media (floppy / CD / GOG re-release) + fan patches; libretro-database doesn't ship a canonical SHA-1 set for DOS games. Cover sync falls back to fuzzy filename matching at the shared 0.95 threshold against the game-directory basename. Both `rom_hashes::libretro_dat_refs_for_system` and the cart-shape BIOS dispatch return no-op for `"dosbox"`.

## 2026-05-24 — Keyboard passthrough on by default

`default_keyboard_passthrough("dosbox") = true`. Many DOS games are keyboard-driven (typed commands in the Sierra adventures, ASCII menu navigation in roguelikes, text-mode strategy titles). Without passthrough those games are unplayable. Same precedent as `scummvm` / `mame` / `msx` — systems where keyboard input is part of the game.

## 2026-05-24 — Action-DOS-game face-button defaults

The default RetroPad bindings target action DOS games (Doom, Wolfenstein 3D, Commander Keen, Jazz Jackrabbit) — the kind of game most players first launch on DOSBox. **A = jump/use (PRIMARY)**, **B = shoot/attack** with Z/X muscle memory per the cross-system rule. L/R shoulders = strafe-left/right (Doom muscle memory). START = ESC / pause, SELECT = TAB / map / inventory. Mouse-driven sims (X-COM, SimCity, Civilization) flow through OA's shared POINTER infra and barely touch the RetroPad; operators rebind freely for non-action DOS games via the per-system Bindings UI.

## 2026-05-24 — Theme: amber-on-black 55° L=0.65 C=0.18

Period-correct to the DOS era's amber-phosphor CRT aesthetic — the iconic monochrome terminal palette + the orange-on-black BIOS POST + DOS prompt `C:\>`. Hue 55° aligns with the TG-16 orange (also 55°) but DOSBox sits at lower lightness (L=0.65 vs TG-16 L=0.74) and slightly higher chroma; together with its `formFactor: "computer"` and the engine-launcher pairing with ScummVM's teal-cyan it reads as the "DOS amber" most players associate with the platform.
