# gamegear — Sega Game Gear

Onboarded 2026-05-19. Drives the Sega Game Gear via the libretro
**Genesis Plus GX** core (`genesis_plus_gx_libretro.dll`) — the same
.dll that services `sms`, so a single install covers both. The Game
Gear was Sega's handheld counterpart to the Master System (1990-97
retail), built on the same Z80-A + VDP architecture but in a portable
landscape form factor with a 160×144 backlit LCD.

The Game Gear is essentially a portable Master System with a different
screen and a few extra colors — Genesis Plus GX handles both from a
single core via the loaded ROM's header signature.

## Upstream

- **Default core (this onboarding):** Genesis Plus GX — https://github.com/libretro/Genesis-Plus-GX
  - Same .dll used by `sms` and `genesis` (alternate). One install,
    three systems.
  - Buildbot path: https://buildbot.libretro.com/nightly/windows/x86_64/latest/genesis_plus_gx_libretro.dll.zip
- **Alternates (per-system Cores override):**
  - `picodrive_libretro.dll` — lighter, MD-first but also handles GG.
- **Vendored:** No. Same convention as SMS / Genesis — buildbot .dll
  in `<exe_dir>/cores/`.

## ROM format

- **`.gg`** — canonical raw Game Gear dump. Headerless binary; ROM
  signature distinguishes GG from SMS internally (Genesis Plus GX picks
  the right hardware mode automatically).
- **`.bin`** — intentionally NOT registered. Same collision rationale
  as SMS, Genesis, Atari 7800: users with `.bin` GG dumps rename to
  `.gg`.

## BIOS

- **Optional.** Genesis Plus GX runs without `bios.gg` — the
  era-correct boot logo gets skipped, but games launch normally. The
  canonical `bios.gg` lives in `<exe_dir>/system/` if the operator
  wants the period-correct boot behavior.

## Native timing

- **Game Gear (all regions):** 59.92 Hz, **160×144 visible** (smaller
  than SMS's 256×192 — the LCD physical resolution).
- Some Game Gear titles announce as SMS-mode via the ROM signature and
  render at 256×192; GPGX scales those down to 160×144 with letterbox,
  which the OA renderer accepts as-is per the per-frame display aspect
  the core hands it.

## Input

7-button layout defined in `apps/oa-shell/src/bindings.rs::gamegear`.
Identity-mapped to libretro RetroPad bits. Same shape as SMS — D-pad +
Button 1 + Button 2 — but the Game Gear has its Start button on the
unit itself (top-left edge), so the third binding is labeled "START"
rather than "PAUSE" for operator clarity.

| OA Button | libretro bit | Keyboard | Pad | Notes |
|---|---|---|---|---|
| UP/DOWN/LEFT/RIGHT | UP/DOWN/LEFT/RIGHT | Arrows | DPad | 8-way d-pad |
| B1 | B (0) | Z | East | **Primary action** (matches the cross-system "Z + East = primary" rule) |
| B2 | A (8) | X | South | Secondary action |
| START | START (3) | Enter | Start | GG hardware Start button (top-left edge of the unit) |

Same primary/secondary keyboard convention as SMS — Z fires Button 1
(jump / attack / select), X fires Button 2 (secondary / cancel / use).

## Current status (2026-05-19)

**Works:**
- Core resolves to `genesis_plus_gx_libretro.dll` via
  `default_core_dll_for_system("gamegear")`.
- 7-button input mapped through `bindings::gamegear_to_libretro_bits` (identity).
- Library scanner classifies `.gg` as gamegear.
- Theme accent: yellow-green at hue 130°, distinct from every other
  claimed hue (closest neighbor on the wheel: SMS magenta 340° at the
  far side; no near-collisions).

**Not yet validated:**
- Real game launch — needs operator validation against a known-good
  ROM. Suggested test ROMs: **Sonic the Hedgehog (Game Gear)**,
  **Shinobi**, **Tails Adventure**, **Streets of Rage** (Game Gear
  port), **Columns**.
- libretro-database hash matching against `metadat/no-intro/Sega - Game Gear.dat`
  — wired but needs operator-run `Settings → Library → Identify ROMs`
  pass to confirm canonical title lookup.
- Cover sync via libretro-thumbnails `Sega_-_Game_Gear` — wired
  but needs operator validation.

## Per-core docs

- `ROADMAP.md` — phase tracking for Game Gear specifically.
- `SESSION_LOG.md` — Shipped / Almost / Next per session.
- `KNOWN_GAME_BUGS.md` — per-game compatibility issues.
- `DECISIONS.md` — Game Gear-specific integration choices.
