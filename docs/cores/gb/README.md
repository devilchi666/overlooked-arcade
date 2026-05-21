# gb — Nintendo Game Boy / Game Boy Color

Onboarded 2026-05-19. Drives both the original Game Boy (DMG, 1989) and
the Game Boy Color (CGB, 1998) via the libretro **Gambatte** core
(`gambatte_libretro.dll`) by default. A single OA SystemId / slug covers
both hardware variants — Gambatte auto-detects DMG vs CGB from the
loaded ROM header. Z80-derived Sharp LR35902 CPU + Nintendo-custom
PPU + DMG/CGB-specific palette hardware.

The Game Boy went on to sell ~118 million units across the DMG + GBC
lifetime, making it one of the highest-volume single-platform libraries
in the OA catalog. The line continues into Wave 4 with `gba` (Game Boy
Advance) — a separate slug using mGBA, since the hardware is different
enough (32-bit ARM CPU vs the 8-bit Sharp) that it gets its own .dll.

## Upstream

- **Default core (this onboarding):** Gambatte — https://github.com/libretro/gambatte-libretro
  - Long-standing libretro Game Boy default. Mature, broad compat,
    light CPU.
  - Buildbot path: https://buildbot.libretro.com/nightly/windows/x86_64/latest/gambatte_libretro.dll.zip
  - License: GPL-2.0.
- **Alternates (per-system Cores override):**
  - `sameboy_libretro.dll` — more accurate (cycle-level), slightly
    heavier. Recommended for users hitting specific timing-sensitive
    games where Gambatte misbehaves.
  - `tgbdual_libretro.dll` — dual-emulation, focused on Link Cable
    multiplayer scenarios (deferred polish work — Phase 2+).
- **Vendored:** No. Buildbot .dll in `<exe_dir>/cores/`, per the
  2026-05-16 libretro pivot.

## ROM format

- **`.gb`** — canonical raw DMG (Game Boy) dump. Headerless binary;
  ROM header at fixed offset 0x100-0x14F includes Nintendo logo check,
  title, cartridge type, ROM/RAM size, region byte. Modern dump sets
  (No-Intro) ship `.gb` as primary.
- **`.gbc`** — canonical raw CGB (Game Boy Color) dump. Same shape as
  `.gb` but with the CGB flag byte set in the header (0x80 = backward-
  compatible, 0xC0 = CGB-only). Gambatte reads the flag and switches
  hardware mode on load.
- **`.cgb`** — alternate extension some old dumpers used. Not commonly
  shipped today; intentionally NOT registered to avoid an over-broad
  match — users with `.cgb` dumps can rename to `.gbc`.
- **`.sgb`** — Super Game Boy enhanced ROMs (SNES adapter palette
  data). Niche; out of scope for Phase 0. SNES + Super Game Boy
  playback would route through the `snes` slug and an SGB-aware core,
  not here.

## BIOS

- **Optional for both DMG + CGB.** Gambatte runs without a boot ROM —
  the Nintendo boot logo + brand-jingle splash get skipped, but games
  launch normally.
- For era-correct boot the operator drops `dmg_boot.bin` (~256 B) and
  `cgb_boot.bin` (~2 KB) into `<exe_dir>/system/`. These BIOSes are
  trivially small and easy to obtain; OA doesn't ship them.

## Native timing

- **DMG (Game Boy):** 59.73 Hz, **160×144** visible (the LCD physical
  resolution).
- **CGB (Game Boy Color):** 59.73 Hz, **160×144** visible — same LCD
  dimensions; the difference is color depth (DMG = 4-shade grayscale,
  CGB = 32,768 colors with per-palette indexing).
- Gambatte reports per-loaded-ROM via `retro_system_av_info`. The
  renderer takes whatever dimensions the core hands it.

## Input

8-button layout defined in `apps/oa-shell/src/bindings.rs::gb`.
Identity-mapped to libretro RetroPad bits. Identical to NES in shape
— 4-way d-pad + A + B + START + SELECT.

| OA Button | libretro bit | Keyboard | Pad | Notes |
|---|---|---|---|---|
| UP/DOWN/LEFT/RIGHT | UP/DOWN/LEFT/RIGHT | Arrows | DPad | 4-way d-pad |
| A | A (8) | Z | East | **Primary action** (matches the cross-system "Z + East = primary" rule) |
| B | B (0) | X | South | Secondary action |
| START | START (3) | Enter | Start | Pause / menu |
| SELECT | SELECT (2) | RShift | Select | Subscreen / map toggle |

Per the cross-system "Z is primary" rule (locked by the
`z_is_the_primary_action_button_on_every_system` test), keyboard **Z**
fires the GB **A** button — the primary action key for most GB titles
(Mario's jump, Link's sword, Pokémon's "confirm"). Keyboard **X** is
the **B** button (secondary — run / cancel / Pokémon "back").

## Current status (2026-05-19)

**Works:**
- Core resolves to `gambatte_libretro.dll` via
  `default_core_dll_for_system("gb")`.
- 8-button input mapped through `bindings::gb_to_libretro_bits` (identity).
- Library scanner classifies `.gb` + `.gbc` as gb.
- Theme accent: muted DMG pea-green at hue 145° / chroma 0.13. Decisively
  distinct from Game Gear's 130° / 0.18 yellow-green (15° hue gap + 0.05
  chroma gap — GG reads as bright yellow-green, GB as muted forest).

**Not yet validated:**
- Real game launch — needs operator validation against a known-good
  ROM. Suggested DMG test ROMs: **Tetris** (the pack-in), **Super
  Mario Land**, **The Legend of Zelda: Link's Awakening**,
  **Pokémon Red/Blue**. CGB test ROMs: **Pokémon Crystal**,
  **The Legend of Zelda: Link's Awakening DX**, **Wario Land 3**.
- libretro-database hash matching against both `metadat/no-intro/Nintendo - Game Boy.dat`
  AND `metadat/no-intro/Nintendo - Game Boy Color.dat` (merged into
  one local corpus per `fetch_and_parse_all`) — wired but needs operator-
  run `Settings → Library → Identify ROMs`.
- Cover sync via libretro-thumbnails `Nintendo_-_Game_Boy` — wired as
  the primary repo. GBC-specific covers from `Nintendo_-_Game_Boy_Color`
  is a documented follow-up gap (single-slug ↔ multi-repo support).

## Per-core docs

- `ROADMAP.md` — phase tracking for Game Boy specifically.
- `SESSION_LOG.md` — Shipped / Almost / Next per session.
- `KNOWN_GAME_BUGS.md` — per-game compatibility issues.
- `DECISIONS.md` — Game Boy-specific integration choices.
