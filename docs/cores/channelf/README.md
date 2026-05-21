# channelf — Fairchild Channel F

Onboarded 2026-05-19. Drives the Fairchild Channel F (1976-1983) via
the libretro **FreeChaF** core (`freechaf_libretro.dll`). Fairchild
F8 CPU (the FIRST microprocessor designed for a home computer use case)
+ custom video controller.

**Historical note:** The Channel F was the FIRST cartridge-based home
video game console. It predates the Atari 2600 by a year. Atari's
wood-veneer VCS design was a direct response to the Channel F's
wood-grain wedge form factor. The Channel F's library is tiny
(~26 official titles + a small homebrew scene) but every game is a
piece of console history.

## Upstream

- **Default core:** FreeChaF — https://github.com/libretro/FreeChaF
- **Alternates:** No widely-shipped libretro alternate.
- **Vendored:** No.

## ROM format

- **`.chf`** — Channel F community / FreeChaF homebrew extension.
  Exists in modern fan-made sets.
- **`.bin`** — older dump sets use this; intentionally NOT registered
  globally. Per-folder rule for `.bin`-shaped libraries.

## BIOS

- **OPTIONAL:** `sl31253.bin` + `sl31254.bin` + `sl90025.bin` in
  `<exe_dir>/system/`. The Channel F's BIOS contains the title-menu
  selection + per-game menu code. Games will run without it
  (FreeChaF has an internal BIOS replacement) but with less period
  authenticity.

## Native timing

- **NTSC:** 59.92 Hz, **128×64** visible (tiny native resolution; the
  TV-encoded image stretched to 4:3).

## Input

9-button layout — 4-axis plunger controller (mapped to D-pad: UP =
plunger pull-up, DOWN = plunger push-down, LEFT/RIGHT = plunger twist)
+ FIRE (plunger push-in) + 4 console switches (MODE = game-mode
select, TIME = game-select, START = begin game, HOLD = pause).

| OA Button | libretro bit | Keyboard | Pad |
|---|---|---|---|
| UP/DOWN/LEFT/RIGHT | UP/DOWN/LEFT/RIGHT | Arrows | DPad (plunger 4-axis) |
| FIRE | B (0) | Z | East (plunger push-in, primary) |
| MODE | Y (1) | M | North (game-mode select) |
| TIME | SELECT (2) | T | Select (game-select) |
| START | START (3) | Enter | Start |
| HOLD | L (10) | H | LeftTrigger (pause) |

The Channel F joins 2600 + O2 as a Z=primary-only system: FIRE is the
single game-action button; MODE/TIME/START/HOLD are CONSOLE switches
with hardware-label keyboard bindings (M, T, Enter, H), not
secondary game actions. Channel F's exception is documented in the
`z_is_the_primary_action_button_on_every_system` test header.

## Current status (2026-05-19)

**Works:** Core resolution, 9-button bindings, library scanner
classifies `.chf`, theme accent cedar-brown 25°/L=0.45/C=0.06 —
sibling wood-grain to 2600.

**Not yet validated:** Operator launch (suggested: **Video Whizball**,
**Spitfire**, **Dodge It**, **Memory Match** — among the few titles
that exist for this system). Cover sync. BIOS optional install.
