# nes — Nintendo Entertainment System / Famicom

System #3 (onboarded 2026-05-18 alongside SNES). Runs via the **FCEUmm** libretro core loaded as a `.dll` from `<exe_dir>/cores/`. Mesen is the higher-accuracy alternative the user can swap to via per-system Cores settings.

## Upstream core

- **Default:** `fceumm_libretro.dll` from libretro buildbot — long-standing libretro NES core, broad compatibility, light CPU.
- **Accuracy alternative:** `mesen_libretro.dll` (Mesen). Cycle-accurate, slightly heavier. Recommended for cycle-edge homebrew + obscure mappers.
- **License:** GPL-2.0-or-later (FCEUmm) / GPL-3.0 (Mesen).
- **Source:** https://github.com/libretro/libretro-fceumm + https://github.com/SourMesen/Mesen2

## Operator setup

1. Download `fceumm_libretro.dll` (or `mesen_libretro.dll`) from buildbot.libretro.com.
2. Drop into `<exe_dir>/cores/`.
3. **No BIOS required** for cart games. **FDS games need `disksys.rom`** in `<exe_dir>/system/` (SHA-1 `5C891EB05680B61438EDBC4C3D77F9C7DC4E8FCA`).
4. Run OA — the cores-folder scan picks up the new .dll automatically. NES ROMs (`.nes`, `.fds`, `.unf`, `.unif`) in any tracked folder appear under the NES system in the sidebar.

## Native timing

- **Resolution:** 256×240 NTSC (top + bottom 8 rows are commonly cropped by the core; visible area 256×224).
- **Frame rate:** 60.099 Hz NTSC (the canonical "59.94 + bit" NTSC NES rate). PAL Famicom would be 50.007 Hz — the libretro core handles region switching internally.
- **Audio:** 48 kHz stereo (libretro standard), upmixed from the NES's 2A03 chip.

## Input

- **NES native layout:** A + B + SELECT + START + 4-way d-pad (8 buttons).
- **Bit layout:** identity remap (matches `RETRO_DEVICE_ID_JOYPAD_*` directly — see `apps/oa-shell/src/bindings.rs::nes`).
- **Default keyboard:** arrows = d-pad, X = A, Z = B, Enter = START, RShift = SELECT.
- **Default gamepad:** d-pad / East = A / South = B / Start = START / Select = SELECT.

## ROM format

- `.nes` — iNES headered dump (the standard).
- `.fds` — Famicom Disk System (needs the `disksys.rom` BIOS).
- `.unf` / `.unif` — UNIF format (some homebrew + obscure mappers).
- NSF audio-only files are intentionally NOT scanned — they aren't games.

## Per-core docs

- `ROADMAP.md` / `SESSION_LOG.md` / `KNOWN_GAME_BUGS.md` / `DECISIONS.md` — same shape as the other cores. Mostly empty at onboarding.
