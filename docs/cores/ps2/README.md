# ps2 — Sony PlayStation 2

Onboarded 2026-05-20 (paired with psp + nds). Drives the Sony
PlayStation 2 via the libretro **LRPS2** core (`pcsx2_libretro.dll`).

The PlayStation 2 was Sony's 2000 6th-gen DVD-ROM console — the
highest-selling console in history (~155M units lifetime). Library
spans ~3800 retail releases over the 2000-2013 lifespan. Standout
titles: Shadow of the Colossus, Ico, Metal Gear Solid 2/3, Final
Fantasy X/XII, Grand Theft Auto III/Vice City/San Andreas, God of
War 1/2, Devil May Cry 1/3, Persona 3/4, Kingdom Hearts 1/2.

## Upstream

- **Default core:** LRPS2 (libretro PCSX2 build) — https://github.com/libretro/pcsx2
- **Vendored:** No. Heavy — needs a strong 64-bit host.

## ROM format

- **`.iso`** — raw DVD dump (most PS2 games shipped on DVD).
- **`.chd`** — MAME-derived compressed CD/DVD container.

Per-folder Import Wizard disambiguates `.iso` collisions against
PSP/3DO/Saturn/Dreamcast/GameCube libraries.

## BIOS

Required regional BIOS in `<exe_dir>/system/`. Pre-checked by
`check_ps2_bios`; slots into the CD-launch BIOS dispatch arm as the
**9th CD-shape system**.

| Filename | Description |
|---|---|
| `scph10000.bin` | JP launch (2000-03-04) |
| `scph39001.bin` | US fat v1.60 |
| `scph70000.bin` | US/EU slim v1.90 |
| `scph90001.bin` | US/EU slim v2.20 |
| `ps2-0230j-20080220.bin` | JP slim v2.30 |
| `ps2-0250a-20100415.bin` | US slim v2.50 |

## Native timing

- NTSC: 59.94 Hz, 640×448 / 640×480 / higher (LRPS2 internal upscale).
- PAL: 50 Hz.

## Input

DualShock 2 — 16 digital buttons + dual analog sticks (via shared
analog infra; LeftStick → `axes[0..2]`, RightStick → `axes[2..4]`).
Defined in `apps/oa-shell/src/bindings.rs::ps2`.

PSX-shape layout + L3/R3 stick clicks (libretro L3/R3 bits). DS2's
real pressure-sensitive face buttons + analog L2/R2 triggers are
Phase 2.5 polish (same deferral as GameCube's analog L/R).

Keyboard defaults match PSX shape: Z=Cross primary, X=Circle
secondary, A=Square, S=Triangle, Q/W=L1/R1, E/R=L2/R2, Enter=Start,
RShift=Select. L3/R3 keyboard unbound by default (gamepad
LeftThumb/RightThumb).

## Current status (2026-05-20)

Phase 0 onboarded. Awaits operator validation.

**Test discs:** Shadow of the Colossus (analog sticks + Cross
combos), Metal Gear Solid 2, Grand Theft Auto III, FFX.

## Per-core docs

- `ROADMAP.md`, `SESSION_LOG.md`, `KNOWN_GAME_BUGS.md`, `DECISIONS.md`.
