# gba — Nintendo Game Boy Advance

Onboarded 2026-05-19. Drives the Nintendo Game Boy Advance (2001-2010
retail) via the libretro **mGBA** core (`mgba_libretro.dll`) by default.
The GBA was Nintendo's third-generation handheld — 32-bit ARM7TDMI CPU
(16.78 MHz) + Nintendo-custom PPU + a 240×160 reflective TFT LCD (the
later GBA SP added a frontlit/backlit variant).

The GBA is a distinct slug from `gb` despite the family name — different
CPU architecture (ARM vs Sharp LR35902), different libretro cores
(mGBA vs Gambatte), different cartridge format. Backward compatibility
with `.gb` / `.gbc` games was a hardware feature of the GBA itself but
in OA terms those still load via the `gb` slug + Gambatte.

## Upstream

- **Default core (this onboarding):** mGBA — https://github.com/libretro/mgba
  - Mature, near-universal GBA compatibility. Light CPU, broadly
    considered the libretro GBA gold standard.
  - Buildbot path: https://buildbot.libretro.com/nightly/windows/x86_64/latest/mgba_libretro.dll.zip
  - License: MPL-2.0.
- **Alternates (per-system Cores override):**
  - `vba_next_libretro.dll` — VBA-Next, lighter / less accurate. Useful
    on lower-spec hosts.
  - `vbam_libretro.dll` — VBA-M, also viable. Less actively developed.
- **Vendored:** No. Buildbot .dll in `<exe_dir>/cores/`.

## ROM format

- **`.gba`** — canonical raw GBA dump. Headerless binary; ROM header
  at fixed offset 0xA0-0xBF includes game title (12 ASCII bytes),
  game code (4 bytes), maker code, fixed values, and complement /
  checksum bytes. Modern dump sets (No-Intro) ship `.gba` as primary.
  Sizes range from 4 MB (small early titles) to 32 MB (later RPGs).
- **`.bin`** — intentionally NOT registered. Same collision rationale
  as every prior system; users with `.bin` GBA dumps rename to `.gba`.

## BIOS

- **Optional.** mGBA runs without `gba_bios.bin` — most games launch
  fine with mGBA's internal BIOS replacement.
- A small number of games refuse to boot without the real BIOS:
  notable cases are **Splinter Cell**, **Hi-Hi Puffy AmiYumi**, and a
  handful of early licensed titles. Operators hitting those drop the
  canonical `gba_bios.bin` (16 KB) into `<exe_dir>/system/`.
- The accurate boot logo (Game Boy Advance splash + jingle) is also
  BIOS-driven, so operators wanting era-correct boots need the BIOS
  regardless of game requirements.

## Native timing

- **All regions:** 59.73 Hz, **240×160 visible**.
- mGBA reports per-loaded-ROM via `retro_system_av_info`. The renderer
  takes whatever dimensions the core hands it.
- 240×160 on a CRT-class modern display upscales clean to common
  aspects; per-system aspect override defaults to 3:2 (the GBA's
  native LCD ratio).

## Input

10-button layout defined in `apps/oa-shell/src/bindings.rs::gba`.
Identity-mapped to libretro RetroPad bits. The GBA controller extends
the Game Boy face layout with two shoulder buttons (L + R) — same
shoulder convention as SNES.

| OA Button | libretro bit | Keyboard | Pad | Notes |
|---|---|---|---|---|
| UP/DOWN/LEFT/RIGHT | UP/DOWN/LEFT/RIGHT | Arrows | DPad | 4-way d-pad |
| A | A (8) | Z | East | **Primary action** (matches the cross-system "Z + East = primary" rule) |
| B | B (0) | X | South | Secondary action |
| L | L (10) | Q | LeftTrigger | Left shoulder |
| R | R (11) | W | RightTrigger | Right shoulder |
| START | START (3) | Enter | Start | Pause / menu |
| SELECT | SELECT (2) | RShift | Select | Sub-screen / map |

Q/W shoulder convention matches SNES + Genesis 6-button. Z=A primary
keybinding matches GB, NES, and every other console-shape system in OA.

## Current status (2026-05-19)

**Works:**
- Core resolves to `mgba_libretro.dll` via
  `default_core_dll_for_system("gba")`.
- 10-button input mapped through `bindings::gba_to_libretro_bits` (identity).
- Library scanner classifies `.gba` as gba.
- Theme accent: deep indigo at hue 285° / lightness 0.55 / chroma 0.20
  — period-correct GBA-launch color, with the deep lightness separating
  it from SNES violet (L=0.62) and Lynx purple (L=0.65) in the same
  general hue family.

**Not yet validated:**
- Real game launch — needs operator validation. Suggested reference
  set: **The Legend of Zelda: The Minish Cap**, **Pokémon FireRed /
  LeafGreen / Emerald**, **Metroid: Zero Mission**, **Advance Wars**,
  **Castlevania: Aria of Sorrow**, **Final Fantasy Tactics Advance**.
- libretro-database hash matching against `metadat/no-intro/Nintendo - Game Boy Advance.dat`
  — wired but needs operator-run `Settings → Library → Identify ROMs`.
- Cover sync via libretro-thumbnails `Nintendo_-_Game_Boy_Advance` —
  wired but needs operator validation.
- BIOS-required title spot-check — if the operator has a BIOS-required
  title in their library, confirm the BIOS-less default surfaces a
  clear error (rather than mGBA's silent-hang behavior on some titles).

## Per-core docs

- `ROADMAP.md` — phase tracking for GBA specifically.
- `SESSION_LOG.md` — Shipped / Almost / Next per session.
- `KNOWN_GAME_BUGS.md` — per-game compatibility issues.
- `DECISIONS.md` — GBA-specific integration choices.
