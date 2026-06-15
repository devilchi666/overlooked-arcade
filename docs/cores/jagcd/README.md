# Atari Jaguar CD — Core integration

## Hardware

The Jaguar CD (1995) was Atari's last-ditch CD-ROM expansion sitting on
top of the original Jaguar cart slot. Same dual-Motorola/GPU hardware
underneath; the CD added FMV-capable storage + CDDA audio. ~13 retail
releases (Battlemorph, Highlander I, Hover Strike: Unconquered Lands,
Vid Grid, Dragon's Lair, etc.) plus a growing homebrew + indie CD
scene.

## Core

`virtualjaguar_libretro.dll` — same core that drives cart Jaguar.
Virtual Jaguar auto-detects CD vs cart from the supplied file
container. No widely-shipped alternate libretro core.

## BIOS

**Both required** in `<exe_dir>/system/`:

- `jagboot.rom` — cart-side boot ROM (~8 KB). Already required by the
  cart `jaguar` system; the same file serves both.
- `jagcd.rom` — CD-side boot ROM (~262 KB). Specific to Jaguar CD;
  cart-only sessions don't need it.

Pre-checked by `check_jaguar_bios` (cart side) + `check_jagcd_bios`
(CD side) before each launch. Missing CD-side BIOS blocks launch
with an actionable error toast.

## Load shape

CD-shape. Accepts `.cue`, `.chd`, `.iso`, `.m3u`, `.ccd`. Disambiguation
via per-folder Import Wizard rule — operator marks a folder as
`jagcd` and matching extensions inside route here. Otherwise the
extensions would collide with every other CD-shape system in the
library.

## Sidebar / theme

Sidebar slug: `jagcd`. ShortName "Jag CD". Form factor "console";
manufacturer "atari". Accent palette is the `jagcd` entry in
[`frontend/src/platform/themes/systemPalettes.ts`](../../frontend/src/platform/themes/systemPalettes.ts)
(typed `SYSTEM_PALETTES`, injected as the `[data-system="jagcd"]` CSS
block at boot) — sits in the gold-orange Jaguar family but a shade
deeper (L 0.58, hue 75°) to read distinctly from cart Jaguar at a glance.

## Controller

Identical to cart Jaguar — the Pro Controller (d-pad + A/B/C +
OPTION/PAUSE + 12-key keypad). Bindings share the
`default_jaguar_bindings()` table; no jagcd-specific module.

## Sibling docs

- [ROADMAP.md](ROADMAP.md) — open work + onboarding status
- [SESSION_LOG.md](SESSION_LOG.md) — chronological operator validation
- [KNOWN_GAME_BUGS.md](KNOWN_GAME_BUGS.md) — per-title quirks
