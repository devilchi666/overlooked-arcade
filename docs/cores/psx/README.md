# psx — Sony PlayStation (PS1)

Onboarded 2026-05-20 (paired with Saturn). Drives the Sony PlayStation
via the libretro **Beetle PSX HW** core (`mednafen_psx_hw_libretro.dll`)
by default, with **Beetle PSX SW** (`mednafen_psx_libretro.dll`)
pre-registered as a recommended catalog peer for hosts where the
hardware-accelerated renderer can't obtain a Vulkan/OpenGL surface.

The Sony PlayStation (PS1) was Sony's 1994 (JP) / 1995 (US/EU) 32-bit
CD-ROM console — the highest-selling console of its generation
(~100M units lifetime) and the platform with OA's largest CD-shape
library (~3000 retail titles). Home of Final Fantasy VII/VIII/IX,
Metal Gear Solid, Crash Bandicoot, Spyro, Castlevania: Symphony of the
Night, Resident Evil, Silent Hill, Tony Hawk's Pro Skater, the canonical
JRPG + survival-horror + early-3D platformer library.

OA wires the PS1 cart-shape — disc images only. The PocketStation
memory-card peripheral and PS2-via-PSX backwards-compat live outside
this slug.

## Upstream

- **Default core (this onboarding):** Beetle PSX HW — https://github.com/libretro/beetle-psx-libretro
  - Mednafen-derived; hardware-accelerated Vulkan/OpenGL renderer
    with upscaling, texture filtering, PGXP geometry correction.
  - Buildbot path: https://buildbot.libretro.com/nightly/windows/x86_64/latest/mednafen_psx_hw_libretro.dll.zip
  - License: GPL-2.0+.
- **Catalog peer (same Beetle PSX lineage, software renderer):**
  - `mednafen_psx_libretro.dll` (Beetle PSX SW) — same BIOS file set,
    same compatibility profile, software-only renderer. Pre-registered
    as a recommended catalog peer so operators can swap without manual
    .dll install if HW fails to obtain a GL/Vulkan surface from our
    wgpu DX12 host on their machine.
- **Other alternates (per-system Cores override):**
  - `swanstation_libretro.dll` — DuckStation-derived libretro fork
    (active, modern PSX core with similar HW upscaling).
- **Vendored:** No. Operator drops the buildbot .dll into
  `<exe_dir>/cores/`.

## ROM format

PS1 games are CD images — the libretro CD container set plus the
PSP-converted PS1 EBOOT container:

- **`.cue` + `.bin`** — canonical multi-track CD layout.
- **`.chd`** — single-file MAME-derived compressed CD container.
- **`.iso`** — single-track data-only ISO (loses CDDA).
- **`.m3u`** — multi-disc playlist (FFVII / FFVIII / FFIX 3-disc /
  4-disc / 4-disc, Final Fantasy Tactics 1-disc but homebrew patches
  swap discs).
- **`.ccd` / `.toc`** — CloneCD / cdrdao metadata.
- **`.pbp`** — PSP-format PS1 EBOOT container. Used by libraries
  imported from PSP / PSone Classics releases. Beetle PSX HW + SW
  both read `.pbp` directly. **PSX-unique extension** — no collision
  with PCE-CD / segacd / saturn.

The `.cue/.chd/.iso/.m3u/.ccd/.toc` set collides with PCE-CD / segacd /
saturn; disambiguation via per-folder Import Wizard rule.

## BIOS

PS1 playback **requires** a regional BIOS in `<exe_dir>/system/`
matching the disc's region. PSX region-locking enforced at the BIOS
level. The shell pre-checks SHA-1 against canonical Mednafen-blessed
dumps (`PSX_BIOS_KNOWN_HASHES` in `apps/oa-shell/src/main.rs`).

| Filename       | SHA-1                                      | Description |
|----------------|--------------------------------------------|-------------|
| `scph5500.bin` | `B05DEF971D8EC59F346F2D9AC21FB742E3EB6917` | JP PSX BIOS v3.0 (1995 launch) |
| `scph5501.bin` | `0555C6FAE8906F3F09BAF5988F00E55F88E9F30B` | US PSX BIOS v3.0 (1995, most common) |
| `scph5502.bin` | `F6BC2D1F5EB6593DE7D089C425AC681D6FFFD3F0` | EU PSX BIOS v3.0 (PAL) |
| `scph7001.bin` | `1E68C231D0896B7EADCAD1D7D8E76129824A48D3` | US PSX BIOS v4.1 (1997 revision) |
| `scph7501.bin` | `1B05CE49AB0E6A7C9C28F0F49B7F03B2DC6F5C2C` | US PSX BIOS v4.4 (1998 revision) |
| `scph1001.bin` | `10155D8D6E6E832D6EA66DB9BC098321FB5E8EBF` | US PSX BIOS v2.2 / SCPH-100x PSone alias |

Missing BIOS surfaces a clear error toast naming the expected filenames;
unknown-hash files still load with a warn-level toast.

## Native timing

- **NTSC:** 59.94 Hz, 320×240 / 640×480 (most common, depending on
  the game's framebuffer mode) visible.
- **PAL:** 49.92 Hz, 320×256 / 640×512 visible.
- Beetle PSX HW reports timing per-loaded-image via
  `retro_system_av_info`. Hardware-renderer upscaling multiplies the
  framebuffer dimensions; the OA renderer takes the upscaled output
  as-is.

## Input

14-button digital DualPad layout — defined in
`apps/oa-shell/src/bindings.rs::psx`:

- 4-way d-pad (UP/DOWN/LEFT/RIGHT)
- 4-button face diamond: Triangle (top), Circle (right), Cross
  (bottom, primary), Square (left)
- L1/R1 front shoulders + L2/R2 rear triggers
- START + SELECT

Identity-mapped to libretro RetroPad bits. The DualShock analog sticks
(Left/Right) + L3/R3 stick clicks ship as Phase 2 work alongside
shared analog-input infra.

| PSX button | libretro bit | Keyboard | Gamepad |
|---|---|---|---|
| Cross (×, primary) | B (0) | Z | East |
| Circle (○, secondary) | A (8) | X | South |
| Square (□) | Y (1) | A | West |
| Triangle (△) | X (9) | S | North |
| L1 | L (10) | Q | LeftTrigger |
| R1 | R (11) | W | RightTrigger |
| L2 | L2 (12) | E | LeftTrigger2 |
| R2 | R2 (13) | R | RightTrigger2 |
| START | START (3) | Enter | Start |
| SELECT | SELECT (2) | RShift | Select |
| UP/DOWN/LEFT/RIGHT | (4-7) | Arrows | DPad |

Per the cross-system "Z is primary" rule, keyboard **Z** → Cross
(libretro B bit 0, primary action in Western releases). Keyboard
**X** → Circle (secondary). This intentionally overrides the PSX
physical-layout convention (where Cross is bottom-south and Circle
is right-east) — OA's cross-system "primary on East" convention wins
over period-correct PSX muscle memory. Operators with strong PSX
muscle memory remap via the per-system Bindings dialog.

## Current status (2026-05-20)

**Works:**
- Core resolves via `default_core_dll_for_system("psx") →
  "mednafen_psx_hw_libretro.dll"`.
- Beetle PSX SW pre-registered as a recommended catalog peer
  alternate.
- 14-button digital DualPad input mapped via `psx_to_libretro_bits`
  (identity remap).
- Library scanner classifies `.cue / .chd / .iso / .m3u / .ccd /
  .toc / .pbp` as `psx` once the operator marks the folder via
  Import Wizard (`.pbp` is PSX-unique, no disambiguation needed).
- Theme accent: teal cyan at hue 180° + L=0.65 + C=0.16 — open band,
  evokes PS1 launch palette's cool blue/cyan/silver.
- BIOS pre-check via `check_psx_bios` in main.rs — six canonical
  SHA-1s spanning JP / US / EU regional + revision variants.

**Not yet validated:**
- Real game launch — needs operator validation against a known-good
  ROM + matching regional BIOS. Suggested test discs: **Final Fantasy
  VII** (US, 3-disc — also tests .m3u), **Metal Gear Solid** (US,
  2-disc), **Castlevania: Symphony of the Night** (US single-disc),
  **Crash Bandicoot** (US), **Resident Evil** (US/EU).
- Save state F5/F8 round-trip mid-disc.
- Multi-disc title via `.m3u` — **Final Fantasy VII** (3 discs) is the
  canonical PSX multi-disc test.
- HW renderer surface obtainment — Beetle PSX HW needs to obtain a
  Vulkan/OpenGL surface from the wgpu host. If it fails on a given
  operator's machine, the SW catalog peer is the fallback.
- libretro-database hash matching — `&[]` at onboarding; PSX disc-id
  extraction via `cd_id.rs` (reads SYSTEM.CNF off the data track for
  the boot binary serial) is Phase 2 polish.
- Cover sync via libretro-thumbnails `Sony_-_PlayStation` — wired
  but needs operator validation.

## Per-core docs

- `ROADMAP.md` — phase tracking for PSX specifically.
- `SESSION_LOG.md` — Shipped / Almost / Next per session.
- `KNOWN_GAME_BUGS.md` — per-game compatibility issues.
- `DECISIONS.md` — PSX-specific integration choices.
