# snes — Super Nintendo Entertainment System / Super Famicom

System #4 (onboarded 2026-05-18 alongside NES). Runs via the **Snes9x** libretro core loaded as a `.dll` from `<exe_dir>/cores/`. bsnes is the higher-accuracy alternative.

## Upstream core

- **Default:** `snes9x_libretro.dll` from libretro buildbot — the standard libretro SNES core. Broad compatibility, reasonable CPU.
- **Accuracy alternative:** `bsnes_libretro.dll` (or `bsnes_mercury_*` variants for different speed-accuracy trade-offs). Cycle-accurate, heavier.
- **License:** Snes9x non-commercial freeware / bsnes GPL-3.0.
- **Source:** https://github.com/libretro/snes9x + https://github.com/libretro/bsnes-libretro

## Operator setup

1. Download `snes9x_libretro.dll` from buildbot.libretro.com.
2. Drop into `<exe_dir>/cores/`.
3. **No BIOS required** for most cart games. Satellaview needs `BS-X.bin` + supporting files in `<exe_dir>/system/`. Sufami Turbo needs `STBIOS.bin`. Special chip games (SuperFX / SA-1 / DSP) work without any BIOS — the chip behavior is implemented in the core.
4. Run OA — the cores-folder scan picks up the new .dll automatically. SNES ROMs (`.smc`, `.sfc`, `.fig`, `.swc`) in any tracked folder appear under the SNES system in the sidebar.

## Native timing

- **Resolution:** 256×224 (most common) or 256×239 (overscan-heavy games). Hi-res mode 512×448 for games that use it (Secret of Mana 2-player split, RPM Racing menus).
- **Frame rate:** 60.098 Hz NTSC. PAL Super Famicom would be 50.007 Hz.
- **Audio:** 32 kHz stereo (S-DSP native), upmixed by the core to 48 kHz libretro standard.

## Input

- **SNES native layout:** 12 buttons — A + B + X + Y (diamond) + L + R (shoulders) + START + SELECT + 4-way d-pad.
- **Bit layout:** identity remap (matches `RETRO_DEVICE_ID_JOYPAD_*` directly — see `apps/oa-shell/src/bindings.rs::snes`).
- **Default keyboard:** arrows = d-pad, Z = B, X = A, A = Y, S = X, Q = L, W = R, Enter = START, RShift = SELECT.
- **Default gamepad:** d-pad / East = A / South = B / North = X / West = Y / LeftTrigger = L / RightTrigger = R / Start = START / Select = SELECT.

## ROM format

- `.sfc` — canonical Super Famicom / SNES dump.
- `.smc` — with a 512-byte copier header (some old dumps).
- `.fig` / `.swc` — less common copier formats. Snes9x + bsnes both handle them.

## Per-core docs

- `ROADMAP.md` / `SESSION_LOG.md` / `KNOWN_GAME_BUGS.md` / `DECISIONS.md` — same shape as the other cores.
