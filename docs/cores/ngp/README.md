# ngp — SNK Neo Geo Pocket / Color (NGP + NGPC)

Onboarded 2026-05-20 (paired with neogeo + neocd). Drives both the
mono **Neo Geo Pocket** (1998) and the **Neo Geo Pocket Color** (1999)
via the libretro **Beetle NeoPop** core (`mednafen_ngp_libretro.dll`).
Single slug per the gb / wonderswan pattern — Beetle NeoPop auto-
detects the hardware variant from the ROM header.

The NGP/NGPC was SNK's monochrome (NGP) / 8192-color (NGPC) handheld,
designed as a head-to-head competitor against Game Boy Color. ~80
retail releases combined across both hardware variants over 1998-2000
before SNK discontinued the line. Standout titles: SNK vs. Capcom:
Card Fighter's Clash, SNK vs. Capcom: Match of the Millennium, KOF
R-2, Cool Boarders Pocket, Sonic the Hedgehog Pocket Adventure,
Cardinal Syn (no — actually Card Fighter's Clash).

## Upstream

- **Default core:** Beetle NeoPop — https://github.com/libretro/beetle-ngp-libretro
  - Mednafen-derived; same upstream lineage as the other Beetle cores
    OA ships (PCE Fast, Saturn, PSX, VB, WonderSwan, Lynx).
  - Buildbot path: https://buildbot.libretro.com/nightly/windows/x86_64/latest/mednafen_ngp_libretro.dll.zip
- **Alternates:** None practical. Race is a less-maintained alternative.

## ROM format

- **`.ngp`** — original Neo Geo Pocket mono dump.
- **`.ngc`** — Neo Geo Pocket Color dump.

Both extensions handled by Beetle NeoPop via auto-detect from ROM
header. Headerless raw dumps; library scanner classifies by extension.

## BIOS

**None required.** Beetle NeoPop synthesizes the boot firmware. The
operator doesn't need any NGP/NGPC-specific BIOS in `<exe_dir>/system/`.

## Native timing

- 59.95 Hz refresh.
- NGP: 160×152 monochrome (8 shades).
- NGPC: 160×152 color (8192-color palette, ~12-bit color).

## Input

7-button handheld layout — d-pad + A + B + OPTION. Defined in
`apps/oa-shell/src/bindings.rs::ngp`.

| NGP button | libretro bit | Keyboard | Gamepad |
|---|---|---|---|
| A (primary) | B (0) | Z | East |
| B (secondary) | A (8) | X | South |
| OPTION | START (3) | Enter | Start |
| UP/DOWN/LEFT/RIGHT | (4-7) | Arrows | DPad |

The hardware Sound/Power button doesn't surface as a RetroPad bit.

## Current status (2026-05-20)

Phase 0 onboarded. Awaits operator validation.

**Test ROMs:** SNK vs Capcom: Card Fighter's Clash, Match of the
Millennium, Sonic Pocket Adventure, Magical Drop Pocket.

## Per-core docs

- `ROADMAP.md`, `SESSION_LOG.md`, `KNOWN_GAME_BUGS.md`, `DECISIONS.md` —
  same shape as other per-core directories.
