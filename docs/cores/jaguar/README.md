# jaguar — Atari Jaguar

Onboarded 2026-05-20 (paired with 3do + pcfx). Drives the Atari Jaguar
via the libretro **Virtual Jaguar** core (`virtualjaguar_libretro.dll`).

The Atari Jaguar was Atari Corp's 1993 64-bit cartridge console — the
last console Atari shipped before exiting the hardware business in
1996. ~50 retail cart releases worldwide; the platform's "overlooked
console" identity is exactly why OA exists. Standout titles: Iron
Soldier 1+2, Tempest 2000, Alien vs Predator, Cybermorph, Rayman,
Doom (Jaguar port), Wolfenstein 3D (Jaguar port), Atari Karts.

OA wires the Jaguar cart-shape path. Jaguar CD games (Battlemorph,
Vid Grid, etc.) are a much smaller subset — typically routed through
the same Virtual Jaguar core with a per-game stacked Jag-CD BIOS;
deferred to Phase 3+.

## Upstream

- **Default core:** Virtual Jaguar — https://github.com/libretro/virtualjaguar-libretro
- **Vendored:** No. Operator drops the buildbot .dll.

## ROM format

- **`.j64`** — canonical No-Intro Atari Jaguar dump.
- **`.jag`** — alternate extension some older dumpers used.
- `.rom` intentionally NOT registered (too generic).

## BIOS

**Optional.** `jagboot.rom` in `<exe_dir>/system/` enables the boot
logo + a small set of titles that touch the BIOS. Most games boot
without it. No OA pre-check at Phase 0 (BIOS-optional systems don't
get pre-checks; Virtual Jaguar handles the absence gracefully).

## Native timing

- NTSC: 59.94 Hz, 320×200/400×200/640×200 visible (mode varies per
  game).
- PAL: 49.92 Hz.

## Input

21-button Pro Controller layout — defined in
`apps/oa-shell/src/bindings.rs::jaguar`:

- 4-way d-pad
- 3 face buttons: A (primary, libretro B), B (secondary, libretro A),
  C (tertiary, libretro Y)
- OPTION + PAUSE (small system buttons)
- 12-key numpad: KP1-KP7 mapped to spare RetroPad bits (libretro X /
  L / R / L2 / R2 / L3 / R3); KP8-KP9 + KP_STAR + KP0 + KP_HASH live
  in shell-reserved high bits — surfaced in the per-system Bindings
  page for keyboard binding but require Phase 2 keyboard-passthrough
  dispatch to reach the core via libretro KEYBOARD device.

Keyboard defaults: Z=A primary, X=B secondary, A=C tertiary; Enter=OPTION,
RShift=PAUSE; numpad keys on Key1-Key9 + Key0. KP_STAR + KP_HASH left
unbound by default (operator assigns via per-system Bindings page).

## Current status (2026-05-20)

Phase 0 onboarded. Awaits operator validation.

**Test ROMs:** Iron Soldier (numpad weapon-select stress test),
Tempest 2000, Rayman, Alien vs Predator (full numpad usage),
Doom Jaguar.

## Per-core docs

- `ROADMAP.md`, `SESSION_LOG.md`, `KNOWN_GAME_BUGS.md`, `DECISIONS.md`.
