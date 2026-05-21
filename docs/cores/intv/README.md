# intv — Mattel Intellivision

Onboarded 2026-05-19. Drives the Mattel Intellivision (1979-1990 retail,
~3 million units) via the libretro **FreeIntv** core
(`freeintv_libretro.dll`). General Instrument CP1610 CPU (a 16-bit
chip in 1979 — Mattel marketed "Intelligent Television") + STIC video +
PSG sound. Intellivision launched against the 2600 with substantially
better graphics + sound and a famously-cerebral game library
(SubROC, Astrosmash, Utopia, B-17 Bomber).

## Upstream

- **Default core:** FreeIntv — https://github.com/libretro/FreeIntv
  - Modern, active libretro Intellivision core. The previous-generation
    options (jzIntv, BlissIntv) aren't shipped through the libretro
    buildbot in actively-maintained form.
  - Buildbot path: https://buildbot.libretro.com/nightly/windows/x86_64/latest/freeintv_libretro.dll.zip
- **Alternates:** No widely-shipped alternate.
- **Vendored:** No.

## ROM format

- **`.int`** — canonical headerless Intellivision dump (No-Intro standard).
- **`.bin`** — intentionally NOT registered globally; per-folder rule
  for `.bin`-shaped libraries.

## BIOS

- **REQUIRED:** Both `exec.bin` (4 KB — the Executive ROM, Intv's BIOS)
  AND `grom.bin` (2 KB — the Graphics ROM, sprite + font data). Both
  in `<exe_dir>/system/`.
- Without these, FreeIntv won't boot anything (the BIOS handles the
  title splash + sprite rendering pipeline).

## Native timing

- **NTSC:** 59.92 Hz, **160×96** visible (small native resolution; the
  TV-encoded image stretched to 4:3).
- **PAL:** 49.86 Hz, slight variant.

## Input

10-button layout — D-pad (disc-as-8-way) + 4 side action buttons
(UPPER_L, UPPER_R, LOWER_L, LOWER_R) + START (keypad ENTER) + SELECT
(keypad CLEAR). The 12-button keypad numeric (KP1-KP9, KP0) is a Phase
2 polish item — same shape as Coleco's keypad work.

| OA Button | libretro bit | Keyboard | Pad |
|---|---|---|---|
| UP/DOWN/LEFT/RIGHT | UP/DOWN/LEFT/RIGHT | Arrows | DPad (disc-as-8-way) |
| LOWER_L | B (0) | Z | East (primary fire) |
| LOWER_R | A (8) | X | South (secondary fire) |
| UPPER_L | L (10) | Q | LeftTrigger |
| UPPER_R | R (11) | W | RightTrigger |
| START | START (3) | Enter | Start (keypad ENTER) |
| SELECT | SELECT (2) | RShift | Select (keypad CLEAR) |

The Intv disc controller was 16-direction analog; Phase 0 maps to
libretro D-pad as 8-way (lossy but playable for most titles). Full
16-direction support waits on shared analog-input infrastructure.

## Current status (2026-05-19)

**Works:** Core resolution, 10-button bindings, library scanner
classifies `.int`, theme accent deep Mattel navy 260°/L=0.50/C=0.17.

**Not yet validated:** Operator launch (suggested: **Astrosmash**,
**Utopia**, **Snafu**, **Star Strike**, **Major League Baseball**),
BIOS pre-check, cover sync.

**Deferred:** 16-direction disc analog input (Phase 2 — shared
analog-input infra). 12-button keypad full coverage (Phase 2).
