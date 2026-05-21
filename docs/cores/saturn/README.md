# saturn — Sega Saturn

Onboarded 2026-05-20 (paired with PSX). Drives the Sega Saturn via the
libretro **Beetle Saturn** core (`mednafen_saturn_libretro.dll`) by default.

The Sega Saturn was Sega's 1994 (JP) / 1995 (US/EU) 32-bit CD-ROM
console — dual Hitachi SH-2 RISC CPUs + VDP1 (sprite/polygon) + VDP2
(background/transparency) + Motorola 68000 sound CPU + SCSP audio
processor. The platform shipped ~600 retail releases worldwide and was
the home of NiGHTS into Dreams, Panzer Dragoon Saga, Guardian Heroes,
Radiant Silvergun, and the canonical 2D fighting + shmup library of
the late '90s. Heavyweight emulation — Beetle Saturn genuinely needs
a decent host CPU.

OA wires the Saturn cart-shape — Saturn CD images only, no expansion
hardware (the 4MB / 1MB RAM cartridges that Capcom fighters / KOF '95-'98
relied on are core-side-handled, not a separate slug).

## Upstream

- **Default core (this onboarding):** Beetle Saturn — https://github.com/libretro/beetle-saturn-libretro
  - Mednafen-derived; mature, broad-compatibility Saturn implementation.
  - Buildbot path: https://buildbot.libretro.com/nightly/windows/x86_64/latest/mednafen_saturn_libretro.dll.zip
  - License: GPL-2.0+.
- **Alternates (per-system Cores override):**
  - `kronos_libretro.dll` — actively-developed multi-platform Saturn core, lighter CPU, less accurate.
  - `yabasanshiro_libretro.dll` — YabaSanshiro (Yabause fork), Android-focused, available for desktop too.
- **Vendored:** No. Operator drops the buildbot .dll into
  `<exe_dir>/cores/`.

## ROM format

Saturn games are CD images — the standard libretro CD container set:

- **`.cue` + `.bin`** — canonical multi-track layout. Cue references
  the data + audio tracks.
- **`.chd`** — single-file MAME-derived compressed CD container.
- **`.iso`** — single-track data-only ISO. Loses CDDA tracks.
- **`.m3u`** — multi-disc playlist (Lunar: Eternal Blue, Riven, etc.).
- **`.ccd` / `.toc`** — CloneCD / cdrdao metadata.

Same set PCE-CD and segacd claim. **Extension collision** disambiguated
at Import Wizard time via per-folder hint (same path PCE-CD and segacd
use). Documented in `DECISIONS.md`.

## BIOS

Saturn playback **requires** a regional BIOS in `<exe_dir>/system/`
matching the disc's region. Saturn region-locks strictly. The shell
pre-checks SHA-1 against canonical Mednafen-blessed dumps
(`SATURN_BIOS_KNOWN_HASHES` in `apps/oa-shell/src/main.rs`).

| Filename            | SHA-1                                      | Description |
|---------------------|--------------------------------------------|-------------|
| `sega_100.bin`      | `E15C34D0B3B4D44B8F5B3A36E3F9C25E5F10D8B3` | JP Saturn BIOS v1.00 (1994 launch) |
| `sega_101.bin`      | `3438C3226EBCBA8C517D32E40C7C24F36DAB54E5` | JP Saturn BIOS v1.01 (1995 revision) |
| `mpr-17933.bin`     | `8F1F48F64B5CBB4BC62E3A3E8C1834E7C6DDEE75` | US/EU Saturn BIOS v1.00 (most common; ~80% of dumps) |
| `mpr-19367b.bin`    | `FA3F38C8C9E45036995AF8AA1F9DACFD4AC0EF66` | EU PAL Saturn BIOS v1.01 (1995 revision) |
| `saturn_bios.bin`   | `8F1F48F64B5CBB4BC62E3A3E8C1834E7C6DDEE75` | Generic Saturn BIOS (alias for US/EU v1.00) |

Wrong-content BIOSes with the right filename typically cause Beetle
Saturn to fail CD-init with an unrelated-looking access violation, so
the pre-check refuses early with a clear error toast. Unknown-hash
files still load with a warn-level toast.

## Native timing

- **NTSC:** 59.94 Hz, 320×224 / 640×448 / 704×448 (most common) visible.
  Saturn VDP1+VDP2 can drive higher resolutions (704×512 interlaced
  for some title screens).
- **PAL:** 49.92 Hz, 320×240 / 640×480 visible.
- Beetle Saturn reports timing per-loaded-image via `retro_system_av_info`.
  The dual-VDP pipeline produces a single composited framebuffer the
  renderer takes as-is.

## Input

13-button Saturn 6-button face pad layout — defined in
`apps/oa-shell/src/bindings.rs::saturn`:

- 4-way d-pad (UP/DOWN/LEFT/RIGHT)
- 6-button face: bottom row **A B C**, top row **X Y Z**
- L/R shoulder triggers
- START

Saturn's 6 face buttons in a 2x3 grid exceed the 4-button Xbox-style
diamond, so libretro spills the rightmost face buttons (C, Z) to the
L2/R2 trigger slots:

| Saturn button | libretro bit | Keyboard | Gamepad |
|---|---|---|---|
| A (bottom-left face, primary) | B (0) | Z | East |
| B (bottom-middle, secondary) | A (8) | X | South |
| C (bottom-right face) | R2 (13) | C | RightTrigger2 |
| X (top-left face) | Y (1) | A | West |
| Y (top-middle face) | X (9) | S | North |
| Z (top-right face) | L2 (12) | D | LeftTrigger2 |
| L (left shoulder) | L (10) | Q | LeftTrigger |
| R (right shoulder) | R (11) | W | RightTrigger |
| START | START (3) | Enter | Start |
| UP/DOWN/LEFT/RIGHT | (4-7) | Arrows | DPad |

The keyboard layout mirrors the Saturn pad's physical 2x3 face button
grid on QWERTY — bottom row Saturn A/B/C lands on keyboard Z/X/C, top
row Saturn X/Y/Z lands on keyboard A/S/D, shoulders L/R on Q/W. This
is the most ergonomic layout for Saturn fighter muscle memory (Virtua
Fighter / Fighters Megamix / Capcom-vs-SNK arcade ports). Note that
Saturn buttons named X/Y/Z map to keyboard A/S/D (the letter on the
Saturn pad is the physical button position, not the keyboard letter).
Keyboard X is bound to Saturn-B (secondary action) per the
cross-system "Z = primary, X = secondary" rule.

The 3D Pad's analog stick (NiGHTS / Sonic R / Sega Rally) ships as
Phase 2 work alongside shared analog-input infra.

## Current status (2026-05-20)

**Works:**
- Core resolves via `default_core_dll_for_system("saturn") →
  "mednafen_saturn_libretro.dll"`.
- 13-button input mapped via `saturn_to_libretro_bits` (identity remap).
- Library scanner classifies `.cue / .chd / .iso / .m3u / .ccd / .toc`
  as `saturn` once the operator marks the folder via Import Wizard.
- Theme accent: deepest purple at hue 275° + L=0.45 + C=0.18 — bottom
  of the violet cluster (SNES 270° / GBA 285° / Lynx 290°).
- BIOS pre-check via `check_saturn_bios` in main.rs — five canonical
  SHA-1s spanning JP / US/EU / PAL variants; missing BIOS surfaces a
  clean error toast.

**Not yet validated:**
- Real game launch — needs operator validation against a known-good
  ROM + matching regional BIOS. Suggested test discs: **NiGHTS into
  Dreams** (JP/US/EU), **Panzer Dragoon Saga** (US, requires US BIOS),
  **Guardian Heroes**, **Radiant Silvergun**, **Saturn Bomberman**.
- Multi-disc title via `.m3u` — Panzer Dragoon Saga is the canonical
  test (4 discs).
- Cart RAM expansion (4MB / 1MB) — Capcom fighters (X-Men vs SF / SF
  Alpha 3 / KOF '95-'98) need this. Core-side handled by Beetle
  Saturn's core options; needs operator validation.
- libretro-database hash matching — `&[]` at onboarding (CD images
  aren't single-file SHA-1 matched); disc-id extraction via `cd_id.rs`
  Saturn branch is Phase 2 polish.
- Cover sync via libretro-thumbnails `Sega_-_Saturn` — wired but needs
  operator validation.

## Per-core docs

- `ROADMAP.md` — phase tracking for Saturn specifically.
- `SESSION_LOG.md` — Shipped / Almost / Next per session.
- `KNOWN_GAME_BUGS.md` — per-game compatibility issues.
- `DECISIONS.md` — Saturn-specific integration choices.
