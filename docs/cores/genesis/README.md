# genesis — Sega Mega Drive / Genesis

Onboarded 2026-05-19. Drives the Sega Mega Drive / Genesis via the
libretro **ClownMDEmu** core (`clownmdemu_libretro.dll`) by default.
The Mega Drive (JP / EU naming) / Genesis (NA naming) was Sega's 16-bit
home console (1988-97) — Motorola 68000 main CPU + Z80 sound CPU + VDP +
Yamaha YM2612 + SN76489 PSG. The console launched against the PC Engine
and went on to make Sega the dominant US console brand of the early
'90s before SNES caught up.

OA wires the Mega Drive cart path only. Sega CD / 32X expansion
hardware uses different libretro cores (`picodrive_libretro`,
`genesis_plus_gx_libretro` for Sega CD) and would land as separate
`segacd` / `sega32x` slugs.

## Upstream

- **Default core (this onboarding):** ClownMDEmu — https://github.com/Clownacy/clownmdemu-libretro-frontend
  - Modern active-development MD core (v1.6.11 as of 2026-05-19).
  - Buildbot path: https://buildbot.libretro.com/nightly/windows/x86_64/latest/clownmdemu_libretro.dll.zip
  - License: AGPL-3.0+.
- **Alternates (per-system Cores override):**
  - `genesis_plus_gx_libretro.dll` — long-standing libretro multi-Sega
    core (SMS / GG / MD / Sega CD). Catalog-recommended for general use;
    the broader Sega family lives behind one .dll.
  - `picodrive_libretro.dll` — lighter MD core, also covers 32X + Sega CD.
  - `blastem_libretro.dll` — higher-accuracy MD, chip-level focus.
- **Vendored:** No. ClownMDEmu is small and active; we install the
  prebuilt buildbot .dll into `<exe_dir>/cores/` and treat it as a
  black box. If we ever need to fork (e.g. for an OA-specific extension),
  we maintain our own libretro-frontend build per the project DECISIONS
  2026-05-16 pivot.

## ROM format

- **`.md`** — canonical raw MD dump. Headerless binary; first 256 bytes
  are the system header (ASCII strings, region byte, ROM checksum).
  Every modern dump set (No-Intro, TOSEC) ships .md as the primary.
- **`.smd`** — Super Magic Drive-format dump. 512-byte header + interleaved
  16 KB blocks. Older dumpers used this when SMD was the dominant retail
  copier. ClownMDEmu / Genesis Plus GX deinterleave SMD format internally.
- **`.gen`** — alternate raw extension some old Genesis dumpers used
  (notably some BinSplit / Kega Fusion era sets). Same byte content as .md.
- **`.68k`** — rare; some homebrew dumps lean on the CPU name. ClownMDEmu
  handles it as raw binary.
- **`.bin`** — intentionally NOT registered. Collides with PCE-CD track
  files + future Atari 2600 / SMS sets. Users with `.bin` MD dumps can
  rename to `.md`.

## BIOS

- **None required** for stock Mega Drive cart playback. Sega's MD
  shipped with a tiny TMSS (Trademark Security System) check ROM
  embedded in the console hardware — cores emulate that internally
  rather than loading an external file.
- Sega CD / 32X add-on hardware **would** need BIOSes (`bios_CD_E.bin`
  / `bios_CD_U.bin` / `bios_CD_J.bin` etc. for Sega CD), but those are
  per-system and live behind a future `segacd` slug.

## Native timing

- **NTSC:** 59.92 Hz, 320×224 visible (some games drop to 256×224 in
  H32 mode).
- **PAL:** 49.70 Hz, 320×240 visible.
- ClownMDEmu reports timing per-loaded-ROM via `retro_system_av_info` —
  the renderer takes whatever dimensions the core hands it.
- Mega Drive has non-square pixels in H40 mode (320×224 on a 4:3 CRT),
  so per-system Display → Aspect override defaults to 4:3 (`display_aspect_override = 1.333`).
  H32 mode (256×224) shipped in some early games (Phantasy Star II,
  Altered Beast) — the operator can override per-game if a specific
  title looks stretched.

## Input

10-button layout defined in `apps/oa-shell/src/bindings.rs::genesis`.
Identity-mapped to libretro RetroPad bits. The default is the **6-button
Mega Drive controller** (A/B/C + X/Y/Z + Start + Mode + d-pad) — Street
Fighter II Champion Edition and Capcom's MD fighters all ship support
for it, and most modern dumps assume 6-button is available.

| OA Button | libretro bit | Keyboard | Pad | Notes |
|---|---|---|---|---|
| UP/DOWN/LEFT/RIGHT | UP/DOWN/LEFT/RIGHT | Arrows | DPad | 8-way stick |
| A | Y (1) | A | West | Lower-left of 3-button face row |
| B | B (0) | Z | East | Middle of 3-button face row — **primary action** (matches the cross-system "Z + East = primary" rule) |
| C | A (8) | X | South | Lower-right of 3-button face row — secondary |
| X | L (10) | Q | LeftTrigger | Top-left of 6-button extra row |
| Y | X (9) | S | North | Top-middle of 6-button extra row |
| Z | R (11) | W | RightTrigger | Top-right of 6-button extra row |
| START | START (3) | Enter | Start | Pause / menu |
| MODE | SELECT (2) | RShift | Select | 3- vs 6-button mode toggle on real hardware; here it's just bound to libretro SELECT for any game that reads it |

Per the cross-system "Z is primary" rule (locked by the
`z_is_the_primary_action_button_on_every_system` test), keyboard **Z**
fires the MD's **B** button (libretro bit 0) — the middle face button.
Most MD games use B for the main action (jump in Sonic; attack in
Streets of Rage). Keyboard **X** is C (jump-while-running / kick /
secondary). Keyboard **A** is A (tertiary, e.g. special move). The
6-button shoulder triplet (X/Y/Z) follows the SNES pattern: Q/S/W on
the top QWERTY row + LeftTrigger / North / RightTrigger on the pad.

3-button-only games (Sonic 3D Blast, World of Illusion) ignore X/Y/Z;
the user can toggle to 3-button mode via the per-system Bindings page
if a specific game misbehaves with the 6-button pad announce. ClownMDEmu
defaults to 6-button mode.

## Current status (2026-05-19)

**Works:**
- Core loads via `clownmdemu_libretro.dll`.
- 10-button input mapped through `bindings::genesis_to_libretro_bits` (identity).
- Library scanner classifies `.md` / `.smd` / `.gen` / `.68k` as genesis.
- Theme accent: cobalt blue at hue 245°, distinct from PCE-CD's cyan-blue (220°).

**Not yet validated:**
- Real game launch — needs operator validation against a known-good
  ROM. Suggested test ROMs: **Sonic the Hedgehog**, **Streets of Rage 2**,
  **Phantasy Star IV**, **Gunstar Heroes**, **Castlevania: Bloodlines**.
- `.smd` deinterleaving — ClownMDEmu should handle this transparently;
  needs an SMD-format dump to confirm.
- PAL game compatibility — most US-region NTSC dumps work; PAL games
  may need the operator to confirm 50 Hz timing isn't dropping frames.
- libretro-database hash matching against `metadat/no-intro/Sega - Mega Drive - Genesis.dat`
  — wired but needs operator-run `Settings → Library → Identify ROMs`
  pass to confirm canonical title lookup.
- Cover sync via libretro-thumbnails `Sega_-_Mega_Drive_-_Genesis` — wired
  but needs operator validation.

## Per-core docs

- `ROADMAP.md` — phase tracking for Mega Drive / Genesis specifically.
- `SESSION_LOG.md` — Shipped / Almost / Next per session.
- `KNOWN_GAME_BUGS.md` — per-game compatibility issues.
- `DECISIONS.md` — Genesis-specific integration choices.
