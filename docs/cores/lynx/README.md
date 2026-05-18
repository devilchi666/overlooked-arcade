# lynx — Atari Lynx

Second core online. Runs via the **mednafen_lynx** libretro core loaded as a `.dll` from `<exe_dir>/cores/` — same dynamic-loading architecture as tg16 since the 2026-05-16 libretro pivot.

## Upstream core

- **Recommended build:** `mednafen_lynx_libretro.dll` from the libretro nightlies at https://buildbot.libretro.com/nightly/ (or the latest official stable release).
- **License:** GPL-2.0-or-later (Mednafen).
- **Source:** https://github.com/libretro/beetle-lynx-libretro

OA does not vendor or build the Lynx core in-tree as of this writing — the libretro pivot lets us ship the .dll alongside our binary instead. If we later need a forked Lynx build with custom patches, the recipe (separate libretro-frontend build that emits a .dll we drop in `cores/`) mirrors the planned approach for the modified Beetle PCE Fast — see project `DECISIONS.md` 2026-05-16 entry.

## Operator setup

1. Download `mednafen_lynx_libretro.dll` from buildbot.libretro.com.
2. Drop it into `<exe_dir>/cores/` (the directory next to `oa-shell.exe`).
3. Drop the **Lynx BIOS** (`lynxboot.img`, 512 bytes, SHA-1 `e4ed47fae31693e016b081c6bda48da5b70d7ccb`) into `<exe_dir>/system/`. Without it the core refuses to boot.
4. Run OA — the cores-folder scan picks up the new .dll automatically. Lynx ROMs (`.lnx` / `.lyx`) in any tracked folder appear in the library under the Lynx system.

## Native timing

- **Resolution:** 160×102 native framebuffer (portrait when the handheld is held landscape with stylized box art; some games rotate). The renderer's scaling modes apply unchanged; the per-system `tileAspect: "4/3"` matches the box-art shape, not the framebuffer's 160:102.
- **Frame rate:** 75 Hz native (Lynx ran at 75 Hz, faster than NTSC TVs). The emu thread frame period derives from the core's reported timing; `oa-render` presents under FIFO vsync so the host display caps to its refresh rate.
- **Audio:** 16 kHz mono in the native chip, upsampled by the core to 44.1 kHz stereo before `retro_audio_sample_batch`. `oa-audio`'s linear-interp resampler handles the rest.

## Input

- **Lynx native layout:** D-pad + A + B + Option 1 + Option 2 + dedicated Pause button.
- **libretro mapping** (per RetroArch convention, mirrored in `apps/oa-shell/src/bindings.rs::lynx::*`):
  - Lynx A → libretro A (bit 8)
  - Lynx B → libretro B (bit 0)
  - Lynx Option 1 → libretro START (bit 3)
  - Lynx Option 2 → libretro SELECT (bit 2)
  - Lynx Pause → libretro L (bit 10)
- **Default keyboard:** arrows = d-pad, X = A, Z = B, Enter = OPT1, RShift = OPT2, Space = PAUSE.
- **Default gamepad:** d-pad / East = A / South = B / Start = OPT1 / Select = OPT2 / LeftTrigger = PAUSE.

Because the Lynx bit layout is laid out to match libretro's positions directly, the `lynx_to_libretro_bits` function is identity — `bindings::to_libretro_bits` dispatches to it from `set_input_remapped` based on the active `system_id`.

## ROM format

- `.lnx` — Handy-style headered dumps. Standard format.
- `.lyx` — Variant some dumpers wrote; the core handles both via identical framing.
- The scanner picks up both extensions; no special handling needed.

## Current status (2026-05-18)

**Wired:**
- System registry entry + theme (purple accent — Epyx '89 box family palette).
- Per-system button bits + default keyboard / gamepad bindings.
- `system_id` threaded through `EmuCommand::LoadRom` + `launch_rom` Tauri command; emu thread dispatches input remap and per-system core preference by id.
- `default_core_dll_for_system("lynx")` returns `"mednafen_lynx_libretro.dll"` as the fallback when no per-system pref is set.

**Not yet validated by the operator:**
- End-to-end launch of a real Lynx ROM (needs the operator to drop the core .dll + BIOS into the install).
- Save state behavior — Mednafen Lynx supports `retro_serialize` so this should "just work" via our existing F5/F8 path, but it hasn't been live-tested.
- Whether the OA per-game shader presets (Phase 3 slice A — plain / scanlines / crt-lite) read well at the Lynx's 160×102 framebuffer; the scanline period locks to source `fb_height` so a 102-row source paints noticeably denser scanlines than tg16's 239-row source. May want a per-system scanline-intensity tweak later — falls under Phase 3 slice C's TOML preset work.

## Per-core docs

- `ROADMAP.md` — phase tracking for lynx specifically.
- `SESSION_LOG.md` — Shipped / Almost / Next per session.
- `KNOWN_GAME_BUGS.md` — per-game compatibility issues as they surface.
- `DECISIONS.md` — lynx-specific integration choices and the *why* behind them.

Project-wide context (license, stack, multi-core architecture) lives in `docs/DECISIONS.md`, `docs/ROADMAP.md`, and `CLAUDE.md`.
