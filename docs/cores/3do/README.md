# 3do — 3DO Interactive Multiplayer

Onboarded 2026-05-20 (paired with jaguar + pcfx). Drives the 3DO
Interactive Multiplayer via the libretro **Opera** core (formerly 4DO,
`opera_libretro.dll`).

The 3DO Interactive Multiplayer was the first console of the 5th
generation (1993 launch, ~6 months before Saturn/PSX). Manufactured by
multiple licensees under a unique business model: Panasonic FZ-1
(1993 launch), Panasonic FZ-10 (1994 revision), GoldStar GDO-101M
(1995 LG-branded), Sanyo Try IMP-21J (Japan-only). ~300 retail
releases over 1993-1996 before The 3DO Company exited the console
business. Standout titles: Star Control II, Road Rash, The Need for
Speed, Lemmings 3DO, Crash 'n Burn, Killing Time, Burning Soldier,
D, Gex.

## Upstream

- **Default core:** Opera — https://github.com/libretro/opera-libretro
- **Vendored:** No.

## ROM format

Standard libretro CD container set (`.cue` + `.bin` / `.chd` / `.iso` /
`.m3u` / `.ccd` / `.toc`); per-folder Import Wizard disambiguation.

## BIOS

Required regional/manufacturer BIOS in `<exe_dir>/system/`. Pre-checked
by `check_3do_bios`; slots into the CD-launch BIOS dispatch arm.

| Filename | Description |
|---|---|
| `panafz1.bin` | Panasonic FZ-1 v1.0 (1993 launch, most common) |
| `panafz10.bin` | Panasonic FZ-10 v1.x (1994 revision) |
| `goldstar.bin` | GoldStar GDO-101M (1995) |
| `sanyotry.bin` | Sanyo Try IMP-21J (Japan) |

The four BIOSes are functionally interchangeable for most games; some
late-1995/96 titles benefit from FZ-10 BIOS specifically.

## Input

11-button controller — d-pad + A (red, primary) + B (green, secondary)
+ C (yellow, tertiary) + L/R shoulders + STOP (X) + PLAY (P) + START.
No SELECT on the 3DO standard pad.

Keyboard defaults: Z=A primary, X=B secondary, A=C tertiary, Q/W=L/R
shoulders, Key1/Key2=STOP/PLAY, Enter=START.

## Current status (2026-05-20)

Phase 0 onboarded. Awaits operator validation.

**Test discs:** Star Control II, Road Rash, The Need for Speed,
Lemmings 3DO, Crash 'n Burn. Pick one matching the operator's
installed BIOS.

## Per-core docs

- `ROADMAP.md`, `SESSION_LOG.md`, `KNOWN_GAME_BUGS.md`, `DECISIONS.md`.
