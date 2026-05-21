# 2600 — Atari 2600 / VCS

Onboarded 2026-05-19. Drives the Atari 2600 / Video Computer System
(1977-1992 retail, ~30 million units sold) via the libretro **Stella**
core (`stella_libretro.dll`). The 2600 was the first cartridge-based
home console with broad commercial success — MOS 6507 CPU (1.19 MHz, a
6502 derivative) + TIA video/audio chip + RIOT I/O chip. 128 BYTES of
RAM. The granddaddy of the home console era.

The 2600 library spans roughly 500 official releases plus thousands of
unofficial / homebrew / reproduction titles. Stella handles 50+
distinct bankswitching schemes the various publishers invented to
work around the 4 KB cart ROM ceiling.

## Upstream

- **Default core (this onboarding):** Stella — https://github.com/libretro/stella2014-libretro
  - The libretro Atari 2600 default. Mature, comprehensive game compat
    (50+ obscure bankswitching schemes), light CPU.
  - Buildbot path: https://buildbot.libretro.com/nightly/windows/x86_64/latest/stella2014_libretro.dll.zip
  - License: GPL-2.0.
  - Two libretro Stella builds exist upstream: `stella` (the modern
    Stella, tracks active upstream) and `stella2014` (the 2014-era
    fork, considered stable for libretro). OA defaults to whichever
    the buildbot ships as `stella_libretro.dll`.
- **Alternates:** No widely-shipped alternate libretro core for 2600
  in the buildbot.
- **Vendored:** No. Buildbot .dll in `<exe_dir>/cores/`.

## ROM format

- **`.a26`** — canonical headerless raw 2600 dump. Modern dump sets
  (No-Intro) ship `.a26` as primary. Sizes typically 2-32 KB; the
  Supercharger cassette format multi-loads can be larger.
- **`.bin`** — intentionally NOT registered globally despite being
  the *de facto* community standard for 2600 dumps. The collision
  cost (PCE-CD disc tracks + future Sega CD audio + Coleco / Intv /
  O2 / ChannelF / PC-FX) is high enough that we use the import
  wizard's per-folder rule mechanism instead: operators with `.bin`-
  shaped 2600 libraries configure `*.bin → 2600` as a per-folder
  rule on their Atari folder. See DECISIONS for the rationale.

## BIOS

- **None.** The 2600 had no BIOS at all — the cart ROM is the entire
  system firmware. Nothing for the operator to install in
  `<exe_dir>/system/`.

## Native timing

- **NTSC (US):** 59.92 Hz, **160×192** visible.
- **PAL (EU):** 49.86 Hz, **160×228** visible.
- **SECAM (FR):** 49.86 Hz, French SECAM color-encoding variant.
- Stella reports per-loaded-ROM via `retro_system_av_info`. Region
  auto-detect is per-cart-header where possible, otherwise per-game
  override via Stella's core options.
- 160×192 NTSC source on a 4:3 CRT is the canonical aspect — non-square
  pixels (each TIA pixel is ~5× wider than tall on the original). The
  per-system aspect override defaults to 4:3 (`display_aspect_override = 1.333`)
  per the standard 2600 display convention.

## Input

7-button layout defined in `apps/oa-shell/src/bindings.rs::atari2600`.
Identity-mapped to libretro RetroPad bits. The 2600 controller had a
single fire button — by far the simplest layout in OA's lineup. The
SELECT and RESET buttons are the Game Select / Game Reset switches on
the console hardware itself; Stella maps them to libretro SELECT and
START respectively.

| OA Button | libretro bit | Keyboard | Pad | Notes |
|---|---|---|---|---|
| UP/DOWN/LEFT/RIGHT | UP/DOWN/LEFT/RIGHT | Arrows | DPad | 8-way joystick (4-way semantics on most games) |
| FIRE | B (0) | Z | East | The single fire button. **Primary action.** |
| SELECT | SELECT (2) | RShift | Select | Game Select console switch (change game variation / difficulty) |
| RESET | START (3) | Enter | Start | Game Reset console switch (start / restart game) |

**Single-button exception:** the 2600 is the first system in OA's
lineup that's legitimately single-button. The cross-system
`z_is_the_primary_action_button_on_every_system` test (which asserts
both primary AND secondary keyboards land on Z/X) omits the 2600 —
the Z=FIRE half is covered explicitly by
`defaults_cover_every_2600_button`.

**Deferred input:** Paddle controllers (Breakout, Kaboom!, Warlords —
8 paddle-required titles in the main catalog) are NOT covered by the
8-bit joystick bindings. Paddle is analog (a single rotary dial); same
deferred analog-input infrastructure that Atari 7800 Trak-Ball and
Robotron 2084 are waiting on. Without paddle support, paddle-required
games run but are unplayable.

**Console-switch defaults:** Difficulty A/B and Color/B&W switches go
through Stella's core options surface (per-system Settings → Core
Options) rather than the bindings UI — they're hardware toggles, not
input buttons. The operator sets per-game preferences there.

## Current status (2026-05-19)

**Works:**
- Core resolves to `stella_libretro.dll` via
  `default_core_dll_for_system("2600")`.
- 7-button input mapped through `bindings::atari2600_to_libretro_bits`
  (identity).
- Library scanner classifies `.a26` as 2600.
- Theme accent: muted wood-grain brown at hue 60° / chroma 0.07 —
  decisively distinct from TG-16 orange (55° / 0.18) by chroma.

**Not yet validated:**
- Real game launch — needs operator validation. Suggested reference
  set: **Adventure**, **Pitfall!**, **Yars' Revenge**, **River
  Raid**, **Asteroids**, **Combat** (the 1977 pack-in), **E.T.** (the
  infamous one). All joystick-controlled.
- Per-system + per-game launch — Stella has the largest set of game-
  specific compatibility quirks of any OA-onboarded system; expect
  KNOWN_GAME_BUGS to grow.
- libretro-database hash matching against `metadat/no-intro/Atari - 2600.dat`
  — wired but needs operator-run `Settings → Library → Identify ROMs`
  pass. Note: Supercharger / multicart bankswitching schemes may need
  a future header-strip pass to match upstream sha1s.
- Cover sync via libretro-thumbnails `Atari_-_2600` — wired but
  needs operator validation.
- Paddle-required games (Breakout, Kaboom!, Warlords) — confirmed
  unplayable without analog-input infrastructure; documented in
  KNOWN_GAME_BUGS once the operator hits them.

## Per-core docs

- `ROADMAP.md` — phase tracking for Atari 2600 specifically.
- `SESSION_LOG.md` — Shipped / Almost / Next per session.
- `KNOWN_GAME_BUGS.md` — per-game compatibility issues.
- `DECISIONS.md` — 2600-specific integration choices.
