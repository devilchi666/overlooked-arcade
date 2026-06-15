# Sega 32X CD — Core integration

## Hardware

Sega 32X CD games run on the stacked combo of three Sega add-ons: the
Genesis / Mega Drive base console, the Sega CD attachment underneath,
and the 32X cart-slot expansion on top — Sega's notorious 1994-1995
"tower of power" configuration. ~6 retail releases all FMV-heavy
(Corpse Killer, Fahrenheit, Night Trap, Slam City with Scottie
Pippen, Supreme Warrior, Surgical Strike).

## Core

`picodrive_libretro.dll` — the only mainstream libretro core with
32X+CD combined-mode support. Genesis Plus GX (segacd's normal
default) doesn't do 32X at all; ClownMDEmu is MD-only. PicoDrive
auto-detects 32X+CD mode from the supplied disc image + the cart
BIOSes' presence.

## BIOS

**Required** in `<exe_dir>/system/`:

- Regional Sega CD BIOS: `bios_CD_U.bin` (US v1.10) / `bios_CD_J.bin`
  (JP) / `bios_CD_E.bin` (EU). Same files the plain `segacd` system
  already uses; one drop covers both systems.

**Optional**: 32X cart BIOSes (`32x_*.bin` files) are not required —
PicoDrive runs 32X-CD titles without them. Some operators ship them
anyway for parity with cart 32X playback.

Pre-checked by `check_sega_cd_bios` (shared with the segacd
launch gate). Missing BIOS blocks launch with an actionable error
toast.

## Load shape

CD-shape. Accepts `.cue`, `.chd`, `.iso`, `.m3u`, `.ccd`. Same
disambiguation pattern as other CD systems — per-folder Import
Wizard rule marks a folder as `sega32xcd`; matching extensions
inside route here.

## Sidebar / theme

Sidebar slug: `sega32xcd`. ShortName "32X-CD". Form factor "console";
manufacturer "sega". Per-system palette in
[`frontend/src/platform/themes/systemPalettes.ts`](../../../frontend/src/platform/themes/systemPalettes.ts)
(typed `SYSTEM_PALETTES` map, injected as `[data-system]` CSS at boot)
sits in the orange-red 32X family (hue 42°) but slightly deeper
(L 0.60) to read distinctly from cart 32X at a glance.

## Controller

Shares the Mega Drive 6-button layout with genesis / segacd / sega32x —
single `default_genesis_bindings()` table covers all four.

## oa-core routing

The `sega32xcd` frontend slug routes to `oa_core::SystemId::SegaCd`
in the Rust enum (no new SystemId variant). The CD-shape parent
covers shared behaviour (CD container handling, save-state organization);
the slug-level override in `default_core_dll_for_system` is what
diverges sega32xcd from plain segacd — different libretro core
(PicoDrive vs Genesis Plus GX).

This is the "stacked override" pattern documented on the
`oa_core::SystemId::Sega32X` doc comment.

## Sibling docs

- [ROADMAP.md](ROADMAP.md) — open work + onboarding status
- [SESSION_LOG.md](SESSION_LOG.md) — chronological operator validation
- [KNOWN_GAME_BUGS.md](KNOWN_GAME_BUGS.md) — per-title quirks
