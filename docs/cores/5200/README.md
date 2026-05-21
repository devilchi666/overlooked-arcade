# Atari 5200 SuperSystem

The 5200 was Atari's 1982 console — same 6502 + ANTIC + GTIA + POKEY silicon as the 400/800 home computers, paired with a notoriously fragile self-centering analog-joystick controller. Released as Atari's answer to the rising ColecoVision; under-supported by Atari themselves; pulled from the market 1984. ~70 official cart releases (most ported from the home-computer side) + a healthy homebrew scene.

## Default core

`atari800_libretro.dll` — the libretro port of the long-standing Atari800 emulator. Covers the Atari 8-bit family in one .dll (400 / 800 / XL / XE home computers + 5200 console). Mature, broad compat, light CPU.

The 8-bit home computers are deferred from OA's wiring plan (per the 2026-05-19 "consoles only" filter). The 5200 console rides into OA on the Atari800 core's coattails.

## BIOS

Required: `5200.rom` (2 KB) at `<exe_dir>/system/5200.rom`. The 5200's BIOS is the console's boot ROM — Atari800 won't launch a cart without it.

Pre-checked by `check_atari5200_bios` (`apps/oa-shell/src/main.rs`). SHA-1 sourced from libretro-database (`6AD7A1E8C9FAD486FBEC9498CB48BF5BC3ADC530`).

## Extensions

`.a52` — canonical headerless raw cart dump.

`.bin` collides with too many other systems (PCE-CD audio tracks, Sega CD, Coleco/Intv/O2/Channel F). Operators with `.bin`-shaped 5200 libraries should use the Import Wizard's per-folder rule to route them.

## Controller

The 5200 controller is famously one of the worst ever made — a self-centering analog joystick that wasn't actually self-centering (the rubber boot degraded fast), plus 12-key keypad. OA ships a default Phase 0 mapping:

- D-pad → joystick (digital fallback — analog routing via the shared analog-input infra is Phase 2.5 polish)
- Z → FIRE1 (libretro B — bottom-side fire button)
- X → FIRE2 (libretro A — top-side fire button)
- Enter → START (keypad START)
- RShift → SELECT (keypad PAUSE)
- F4 → RESET (keypad RESET)

The 12-key keypad (0-9 + * + #) lives behind the libretro KEYBOARD device — Phase 2 polish (same approach as Jaguar's keypad).

## Status

- Phase 0 onboarding: ✅ 2026-05-20 (this session)
- Phase 1 operator validation: ⬜ — drop `atari800_libretro.dll` + `5200.rom` into the appropriate folders, scan a `.a52` library, launch Star Raiders / Missile Command / Galaxian.

## See also

- `docs/cores/5200/ROADMAP.md` — phase tracking + Phase 2 polish items
- `docs/cores/5200/SESSION_LOG.md` — what last session shipped
- libretro-thumbnails: `Atari_-_5200`
- libretro-database dat: `metadat/no-intro/Atari - 5200.dat`
