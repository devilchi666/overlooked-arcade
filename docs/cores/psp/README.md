# psp — Sony PlayStation Portable

Onboarded 2026-05-20 (paired with ps2 + nds). Drives the Sony PSP via
the libretro **PPSSPP** core (`ppsspp_libretro.dll`).

The PlayStation Portable was Sony's 2004 handheld competitor to the
Nintendo DS — a UMD-based 480×272 LCD device. ~1900 retail releases
over the 2004-2014 lifespan. Standout titles: God of War: Chains of
Olympus / Ghost of Sparta, Crisis Core: FFVII, Metal Gear Solid:
Peace Walker, Patapon 1/2/3, LocoRoco, Daxter, Monster Hunter
Freedom Unite, Ridge Racer.

## Upstream

- **Default core:** PPSSPP — https://github.com/libretro/ppsspp
- **Vendored:** No.

## ROM format

- **`.iso`** — raw UMD dump.
- **`.cso`** — compressed ISO (PSP-specific).
- **`.pbp`** — PSN-format EBOOT (PSone Classics + PSP digital
  releases).

## BIOS

**None required.** PPSSPP synthesizes the PSP firmware behavior
internally.

## Native timing

- 60 Hz, 480×272 LCD source.

## Input

12-button digital + analog stick (via `InputState.axes[0..2]`).
Defined in `apps/oa-shell/src/bindings.rs::psp`.

| PSP button | libretro bit | Keyboard | Gamepad |
|---|---|---|---|
| Cross (×, primary) | B (0) | Z | East |
| Circle (○, secondary) | A (8) | X | South |
| Square (□) | Y (1) | A | West |
| Triangle (△) | X (9) | S | North |
| L | L (10) | Q | LeftTrigger |
| R | R (11) | W | RightTrigger |
| START | START (3) | Enter | Start |
| SELECT | SELECT (2) | RShift | Select |
| UP/DOWN/LEFT/RIGHT | (4-7) | Arrows | DPad |

**No L2/R2** — PSP-1000/2000/3000 hardware has only L and R triggers.
PSP Go's right analog stick is Phase 2.5 polish.

## Current status (2026-05-20)

Phase 0 onboarded. Awaits operator validation.

**Test ROMs:** God of War: Chains of Olympus (analog stick + Cross
combos), Crisis Core, Patapon (rhythm gameplay).

## Per-core docs

- `ROADMAP.md`, `SESSION_LOG.md`, `KNOWN_GAME_BUGS.md`, `DECISIONS.md`.
