# coleco — ColecoVision

Onboarded 2026-05-19. Drives the ColecoVision (1982-1984 retail, ~2 million
units) via the libretro **blueMSX** core (`bluemsx_libretro.dll`) by default.
Z80A CPU + Texas Instruments TMS9928A VDP + SN76489 PSG. ColecoVision was
Coleco's challenger to the Atari 2600 — competitive 8-bit graphics for
the era + a famously-strong launch library (Donkey Kong arcade port,
Zaxxon, Lady Bug, Carnival).

## Upstream

- **Default core:** blueMSX — https://github.com/libretro/blueMSX-libretro
  - Mature multi-Z80 emulator: handles MSX1/2 + ColecoVision + SVI-3x8.
  - Buildbot path: https://buildbot.libretro.com/nightly/windows/x86_64/latest/bluemsx_libretro.dll.zip
- **Alternates:** `gearcoleco_libretro.dll` (Coleco-only, lighter
  footprint, good fallback).
- **Vendored:** No.

## ROM format

- **`.col`** — canonical headerless ColecoVision dump (No-Intro standard).
- **`.cv`** — alternate extension used by some dump sets.
- **`.bin`** — intentionally NOT registered globally; use per-folder
  rules. Same rationale as 2600.

## BIOS

- **REQUIRED:** `coleco.rom` (8 KB) in `<exe_dir>/system/`. The
  ColecoVision boot screen and title menu are part of the BIOS; games
  refuse to boot without it.

## Native timing

- **NTSC:** 59.92 Hz, **256×192** visible.
- **PAL:** 49.86 Hz, **256×212** visible.

## Input

16-button layout — D-pad + 2 fire buttons (yellow left, red right) +
10 keypad numbers (0-9). Identity libretro remap; keypad numbers spread
across the RetroPad's Y/X/L/R/L2/R2/L3/R3 + Start + Select per blueMSX's
convention.

| OA Button | libretro bit | Keyboard | Pad |
|---|---|---|---|
| UP/DOWN/LEFT/RIGHT | UP/DOWN/LEFT/RIGHT | Arrows | DPad |
| L_FIRE | B (0) | Z | East |
| R_FIRE | A (8) | X | South |
| KP1..KP9 | Y/X/L/R/L2/R2/L3/R3/START | Key1..Key9 | West/North/triggers/thumb-clicks/Start |
| KP0 | SELECT (2) | Key0 | Select |

The keypad is critical — many Coleco games REQUIRE keypad input at the
start screen (game-mode selection, difficulty choice). Without it, those
games can't progress past their menus.

## Current status (2026-05-19)

**Works:** Core resolution, 16-button bindings, library scanner classifies
`.col` + `.cv`, theme accent bright cyan 195°/L=0.72/C=0.16.

**Not yet validated:** Operator launch validation (suggested:
**Donkey Kong**, **Zaxxon**, **Lady Bug**, **Carnival**, **Cosmic
Avenger**). BIOS pre-check workflow. Cover sync.
