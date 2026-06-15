# Sega Titan Video (ST-V) — Core integration

## Hardware

Sega Titan Video (1994-1998) — Saturn-derived arcade board.
Shared the Saturn's twin SH-2 CPUs + VDP1/VDP2 video pipeline +
68k sound co-processor; ST-V was Sega's vehicle for porting
arcade games to the consumer Saturn (and vice versa). ~80 retail
ST-V games — Astra Superstars, Decathlete, Sport Fishing 2,
Cotton 2, Cotton Boomerang, Funky Head Boxers, Golden Axe: The
Duel, Hanagumi Taisen Columns, Maru-Chan de Goo, Princess
Clara Daisakusen, Radiant Silvergun (Japan-only arcade
original), Steep Slope Sliders, Tecmo World Cup '98, Virtua
Fighter Remix, Winter Heat, etc.

## Core

`mame_libretro.dll` — same MAME .dll OA uses for the parent
`mame` system. MAME has the mature `stv` driver covering the
full ST-V library. No new libretro core; ST-V games are
loaded as MAME ROM sets (`.zip` files containing per-board
ROMs) by name lookup against MAME's internal ROM dat.

Alternates available per-game via core override:
- `mednafen_saturn_libretro.dll` — Beetle Saturn has experimental
  STV mode but the libretro support is rougher than MAME's
  driver. Pick this only if MAME's accuracy diverges on a
  specific title.

## BIOS

MAME handles ST-V BIOS lookup internally — the BIOS ROMs are
part of MAME's standard ROM-set distribution (`stvbios.zip` or
embedded inside the game `.zip` depending on the operator's
MAME-set version). No separate BIOS pre-check function in OA;
MAME's launch path surfaces missing-BIOS errors directly.

Operators wanting to drop a BIOS for ST-V manually: place
`stvbios.zip` in MAME's standard BIOS scan path
(`<exe_dir>/system/`) — MAME picks it up there alongside other
arcade BIOSes.

## Load shape

Cart-shape — ROM-set `.zip` / `.7z` files, same as plain MAME.
Disambiguation via per-folder Import Wizard rule (operator
marks a folder as `stv`; matching ROM sets inside route here).
Without the per-folder rule, ST-V `.zip` files would land in
the general `mame` library.

## Sidebar / theme

Sidebar slug: `stv`. ShortName "ST-V". Form factor "arcade";
manufacturer "sega". Accent palette is the `stv` entry in
[`frontend/src/platform/themes/systemPalettes.ts`](../../frontend/src/platform/themes/systemPalettes.ts)
(typed `SYSTEM_PALETTES`, injected as the `[data-system="stv"]` CSS
block at boot) — cyan-blue at hue 220° / L 0.55 — matches ST-V's 1994-1998
cabinet artwork + marketing palette. Sits in the same cyan
cluster as lynx but darker, reading as an arcade-weight cool
blue rather than handheld brightness.

## Controller

Reuses `default_mame_bindings()` — same arcade-style 6-button
panel layout MAME ships. ST-V cabinets typically used standard
6-button joystick configurations (some Sega-specific 6-button
panels with extra start/select buttons, all covered by MAME's
default mapping).

## oa-core routing

The `stv` frontend slug routes to `oa_core::SystemId::Mame` in
the Rust enum (no new SystemId variant). The library shows ST-V
as its own sidebar entry but the launch path, save-state
organization, and per-system settings all flow through the
existing MAME infrastructure. Pure alias pattern — the slug
exists at the frontend layer only.

## Sibling docs

- [ROADMAP.md](ROADMAP.md) — open work + onboarding status
- [SESSION_LOG.md](SESSION_LOG.md) — chronological operator validation
- [KNOWN_GAME_BUGS.md](KNOWN_GAME_BUGS.md) — per-title quirks
