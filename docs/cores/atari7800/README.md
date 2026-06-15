# atari7800 — Atari 7800 ProSystem

Onboarded 2026-05-19. Drives the Atari 7800 via the libretro ProSystem
core (`prosystem_libretro.dll`). The 7800 was the last cartridge-only
home console Atari shipped under their own brand (1986-92) — a MARIA
chip + 6502 CPU running both native 7800 titles and (with a backwards-
compatible mode) the entire Atari 2600 library.

OA wires the 7800 path only; 2600 compatibility on the 7800 hardware
is a different libretro core (`stella_libretro`) and lives at the
yet-to-be-onboarded `atari2600` slug.

## Upstream

- **Source:** https://github.com/libretro/prosystem-libretro
- **Buildbot:** https://buildbot.libretro.com/nightly/windows/x86_64/latest/prosystem_libretro.dll.zip
- **License:** GPL-3.0+ (frontend wrapper) + ProSystem core's own permissive
  license. See upstream `LICENSE`.
- **Vendored:** No. ProSystem is small but stable; we install the
  prebuilt buildbot .dll into `<exe_dir>/cores/` and treat it as a
  black box.

## ROM format

- **`.a78`** — canonical Atari 7800 dump. 128-byte header carries
  mapper / region / "needs BIOS" / RAM-bank flags the ProSystem core
  reads at load time. Every modern dump (No-Intro, TOSEC) uses .a78.
- **Headerless `.bin`** — rare; some old dumps lack the .a78 header.
  Intentionally NOT registered in `frontend/src/platform/themes/registry.ts` extension list
  because `.bin` collides with future systems (Atari 2600, Mega Drive,
  etc.). Users with headerless dumps can rename to `.a78` — the
  ProSystem core autodetects mapper from the binary itself when the
  header is absent.

## BIOS

- **`7800 BIOS (U).rom`** — the U.S. NTSC boot ROM. ~4 KB. Goes in
  `<exe_dir>/system/`. The boot ROM displays the "Atari" logo and runs
  a brief cartridge-presence check before handing off to the loaded
  ROM. Most games run fine without it; a small subset (POKEY-backed
  audio cores, the "Atari" splash on Robotron 2084, etc.) want it.
- The 7800 has region-specific BIOSes (PAL `7800 BIOS (E).rom`). The
  ProSystem core picks based on .a78 region flag; ship both if you
  play PAL games.
- BIOS is **optional but recommended** per the catalog entry — the
  install path doesn't refuse to load when missing.

## Native timing

- 60 Hz NTSC (320×240 active area, ~14 colors visible per scanline).
- 50 Hz PAL for European releases. The core reports timing per loaded
  ROM via `retro_system_av_info`.
- MARIA's display list system is unusual — line-by-line DMA can starve
  the 6502 of cycles, so the same game can render at different vertical
  resolutions depending on display-list density. The renderer takes
  whatever dimensions the core hands it; the scaling-mode picker covers
  any aspect oddities.

## Input

8-button layout defined in `apps/oa-shell/src/bindings.rs::atari7800`.
Identity-mapped to libretro RetroPad bits.

| Button | libretro bit | Key | Pad | Notes |
|---|---|---|---|---|
| UP/DOWN/LEFT/RIGHT | UP/DOWN/LEFT/RIGHT | Arrows | DPad | Pro-Line 8-way stick |
| B1 (Button 1) | B | Z | East | Primary fire — most games use this for shoot / jump |
| B2 (Button 2) | A | X | South | Secondary fire — weapon swap / kick / lightning |
| PAUSE | START | Enter | Start | 7800 console Pause switch |
| SELECT | SELECT | RShift | Select | 7800 console Select switch |

Per the cross-system "Z is primary" rule, `B1` (the primary fire
button) lands on Z. Most native 7800 games only use B1, so this works
fine for a casual library. Twin-stick-shooter holdouts (Robotron 2084's
two-joystick mode) need a second port wired which isn't currently
exposed in OA — Phase 2+ work.

The 7800's console also had a **Reset** switch alongside Pause and
Select. ProSystem surfaces that via the libretro hotkey API rather
than a RetroPad bit; OA's Reset is the per-tile context menu →
"Reset" action (or the F1 hotkey when the keyboard pump's Game-focus
mode is OFF).

## Current status (2026-05-19)

**Works:**
- Core loads via `prosystem_libretro.dll`.
- 8-button input mapped through `bindings::atari7800_to_libretro_bits` (identity).
- Library scanner classifies `.a78` files as atari7800.
- Theme accent: gold/amber at hue 80°, distinct from every other system.

**Not yet validated:**
- Real game launch — needs operator validation against a known-good
  ROM. Suggested test ROMs: **Asteroids**, **Centipede**, **Ms.
  Pac-Man** (no BIOS dependency), **Choplifter** (POKEY audio,
  recommends BIOS).
- `.a78` mapper coverage — the ProSystem core handles SuperGame
  (the most common 7800 mapper) cleanly; less common mappers
  (Activision-only, F18A) sometimes drop in or out across core
  versions. The catalog default points at the current libretro
  buildbot.
- PAL game compatibility — most US-region NTSC dumps work; PAL
  games may need the European BIOS alongside.

## Per-core docs

- `ROADMAP.md` — phase tracking for Atari 7800 specifically.
- `SESSION_LOG.md` — Shipped / Almost / Next per session.
- `KNOWN_GAME_BUGS.md` — per-game compatibility issues.
- `DECISIONS.md` — 7800-specific integration choices.
