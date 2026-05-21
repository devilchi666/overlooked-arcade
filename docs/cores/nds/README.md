# nds — Nintendo DS

Onboarded 2026-05-20 (paired with psp + ps2). Drives the Nintendo DS
via the libretro **melonDS** core (`melonds_libretro.dll`). **Ships
with the new POINTER input infra** (mouse-as-touch dispatch) — DS
touch-screen titles are playable at Phase 0.

The Nintendo DS was Nintendo's 2004 dual-screen handheld with a
touch-screen bottom display + microphone — the second-highest-
selling console in history (~154M units, behind only PS2). Library
spans ~2000 retail releases over 2004-2014. Standout titles: New
Super Mario Bros. DS, Pokémon HeartGold/SoulSilver, Mario Kart DS,
The Legend of Zelda: Phantom Hourglass / Spirit Tracks (stylus-
driven), Brain Age, Animal Crossing: Wild World, Castlevania:
Dawn of Sorrow / Portrait of Ruin / Order of Ecclesia, Phoenix
Wright trilogy.

## Upstream

- **Default core:** melonDS — https://github.com/libretro/melonds
- **Alternates:** `desmume_libretro.dll` (older, less accurate).

## ROM format

- **`.nds`** — headerless raw NDS dump (No-Intro standard).

## BIOS

**Required — 3 files** (different shape from single-file BIOS systems).
All three must be in `<exe_dir>/system/`. Pre-checked by
`check_nds_bios` (multi-file shape).

| Filename | Description |
|---|---|
| `bios7.bin` | ARM7 BIOS (16 KB) — audio + wireless + touch screen |
| `bios9.bin` | ARM9 BIOS (4 KB) — main CPU + graphics + boot |
| `firmware.bin` | DS firmware (256 KB) — user settings + WiFi config |

melonDS can also run with HLE (high-level emulation) instead of
real BIOS for some titles, but real BIOS gives broader compatibility.

## Native timing

- 60 Hz, dual 256×192 LCD screens (top + bottom — melonDS composites
  to 256×384 by default).

## Input

12-button digital + touch screen. Defined in
`apps/oa-shell/src/bindings.rs::nds`.

Nintendo diamond layout — A is east face (PRIMARY per Nintendo
convention, matches nes/snes/gb/gba precedent), B is south face,
X is north, Y is west.

| NDS button | libretro bit | Keyboard | Gamepad |
|---|---|---|---|
| A (east, PRIMARY) | A (8) | Z | East |
| B (south, secondary) | B (0) | X | South |
| X (north) | X (9) | S | North |
| Y (west) | Y (1) | A | West |
| L | L (10) | Q | LeftTrigger |
| R | R (11) | W | RightTrigger |
| START | START (3) | Enter | Start |
| SELECT | SELECT (2) | RShift | Select |
| UP/DOWN/LEFT/RIGHT | (4-7) | Arrows | DPad |

**Touch screen** flows via the new RETRO_DEVICE_POINTER dispatch
(`InputState.pointer`). Phase 0: mouse-as-touch with screen-relative
coordinates (Phase 2.5 polish for window-relative pixel-perfect
mapping). Left-mouse-button held = stylus down; release = stylus up.

Microphone input (Phantom Hourglass blow-puzzles, Brain Age
spoken-word, Hotel Dusk voice puzzles) deferred to Phase 2.5.

## Current status (2026-05-20)

Phase 0 onboarded. **First system using the new POINTER input
infra.** Awaits operator validation.

**Test ROMs:**
- **Button-only:** New Super Mario Bros. DS, Mario Kart DS.
- **Stylus-driven** (touch test): The Legend of Zelda: Phantom
  Hourglass, Pokémon HeartGold (touch UI), Brain Age, Picross DS.

## Per-core docs

- `ROADMAP.md`, `SESSION_LOG.md`, `KNOWN_GAME_BUGS.md`, `DECISIONS.md`.
