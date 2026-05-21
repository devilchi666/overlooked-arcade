# n64 — Nintendo 64

Onboarded 2026-05-20 (paired with gamecube). Drives the Nintendo 64 via
the libretro **Mupen64Plus-Next** core (`mupen64plus_next_libretro.dll`)
with the GLideN64 video plugin.

The Nintendo 64 was Nintendo's 1996 64-bit cartridge console — the
first console where the analog stick was the PRIMARY directional input
(prior consoles used d-pads or arcade sticks for movement). ~390
retail releases worldwide. Standout titles: Super Mario 64,
GoldenEye 007, The Legend of Zelda: Ocarina of Time / Majora's Mask,
Super Smash Bros, Mario Kart 64, Star Fox 64, Banjo-Kazooie, Perfect
Dark.

## Upstream

- **Default core:** Mupen64Plus-Next — https://github.com/libretro/mupen64plus-libretro-nx
- **Alternates:** `parallel_n64_libretro.dll` (more accurate, heavier).
- **Vendored:** No.

## ROM format

- **`.z64`** — canonical Big-Endian dump (No-Intro standard).
- **`.n64`** — Little-Endian byte-swap (legacy dumper convention).
- **`.v64`** — Half-word byte-swap (older copier format).

Mupen64Plus-Next auto-detects byte order. libretro-database matching
keys against `.z64` sha1; `.n64` and `.v64` dumps need a byte-swap
pass in `rom_header.rs` (Phase 2 polish).

## BIOS

**None required.** The N64's CIC boot ROM is emulated internally by
the core (CIC chip lived on each cart, not externally).

## Native timing

- NTSC: 59.94 Hz, 320×240 / 640×480 visible (mode varies per game).
- PAL: 49.92 Hz.
- High-resolution mode (640×480) used by Conker's Bad Fur Day,
  Perfect Dark menu, etc.

## Input

14-button digital layout + analog stick (via `InputState.axes`).
Defined in `apps/oa-shell/src/bindings.rs::n64`.

| N64 button | libretro bit | Keyboard | Gamepad |
|---|---|---|---|
| A (primary) | B (0) | Z | East |
| B (secondary) | Y (1) | X | South |
| START | START (3) | Enter | Start |
| L | L (10) | Q | LeftTrigger |
| R | R (11) | W | RightTrigger |
| Z (under-controller "use") | L2 (12) | Space | LeftTrigger2 |
| C-Up | X (9) | T | North |
| C-Down | R2 (13) | G | — |
| C-Left | L3 (14) | F | — |
| C-Right | A (8) | H | — |
| UP/DOWN/LEFT/RIGHT | (4-7) | Arrows | DPad |

**Analog stick** = `InputState.axes[0..2]` (gamepad LeftStick).
Keyboard-only users enable Mupen64Plus-Next's "Map d-pad to analog
stick" core option to get full-tilt movement from the digital arrow
keys.

## Current status (2026-05-20)

Phase 0 onboarded. **First system to use the new analog-input infra**
plumbed in this session (libretro RETRO_DEVICE_ANALOG dispatch in
oa-libretro + analog-axis polling in oa-input).

**Test ROMs:** Super Mario 64 (analog stick essential), GoldenEye
(C-buttons for camera), Ocarina of Time, Mario Kart 64,
Smash Bros 64.

## Per-core docs

- `ROADMAP.md`, `SESSION_LOG.md`, `KNOWN_GAME_BUGS.md`, `DECISIONS.md`.
