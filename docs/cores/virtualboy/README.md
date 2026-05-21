# virtualboy — Nintendo Virtual Boy

Onboarded 2026-05-20. Drives the Nintendo Virtual Boy (1995-1996
retail, ~770,000 units — Nintendo's biggest commercial flop) via the
libretro **Beetle VB** core (`mednafen_vb_libretro.dll`). NEC V810 CPU
(32-bit RISC at 20 MHz) + dual mirror-array LED projectors for
stereoscopic 3D + monochrome red-only LED display.

The Virtual Boy was Gunpei Yokoi's final Nintendo project before he
left the company. The "console" was a head-mounted display on a tabletop
stand with a unique dual-D-pad controller. Despite the commercial
failure and brief lifespan, the 22-title library includes some of the
era's most-creative games (Virtual Boy Wario Land, Mario Clash, Mario's
Tennis, Teleroboxer, Jack Bros).

## Upstream

- **Default core:** Beetle VB — https://github.com/libretro/beetle-vb-libretro
  - Mednafen-derived libretro VB core. Mature, near-universal compat.
- **Alternates:** No widely-shipped alternate.
- **Vendored:** No.

## ROM format

- **`.vb`** — canonical Virtual Boy dump extension (No-Intro standard).
  Headerless raw cart bytes.

## BIOS

- **None.** The Virtual Boy had no BIOS — the cart ROM is the entire
  firmware. Nothing for the operator to install.

## Native timing

- **All regions (JP + US):** 50.27 Hz, **384×224** visible per eye
  (the dual mirror-arrays display the same scene from two perspectives
  to create stereoscopic 3D).

## Stereoscopic 3D output

Beetle VB renders the dual-perspective scene in configurable modes:
- **Anaglyph** (red/cyan glasses) — Phase 0 default, requires
  inexpensive glasses to see the 3D effect on a normal monitor.
- **Side-by-side** — for VR-style display via Cardboard / DK1 / DK2
  (the original mid-2010s VR hardware era; modern VR via OpenXR
  bridges is Phase 2+ polish).
- **2D flat** — single-eye output, loses the 3D effect but works on
  any display without glasses.
- **Cyclic 2D** — alternates eyes per frame (only useful on 120 Hz+
  displays with shutter glasses).

The mode is per-system Core Options (Beetle VB exposes
`vb_3dmode`). OA's launch path doesn't pin a default mode; operators
configure via Settings → Core Options.

## Input

10-button layout — LEFT D-pad + A + B + L + R + START + SELECT.
Identity libretro remap. **Right D-pad is Phase 2 polish** — see
DECISIONS for the deferral rationale.

| OA Button | libretro bit | Keyboard | Pad |
|---|---|---|---|
| UP/DOWN/LEFT/RIGHT | UP/DOWN/LEFT/RIGHT | Arrows | DPad |
| A | A (8) | Z | East — primary |
| B | B (0) | X | South — secondary |
| L | L (10) | Q | LeftTrigger |
| R | R (11) | W | RightTrigger |
| START | START (3) | Enter | Start |
| SELECT | SELECT (2) | RShift | Select |

## Current status (2026-05-20)

**Works:** Core resolution, 10-button bindings (single D-pad), library
scanner classifies `.vb`, theme accent deep VB red 7°/L=0.55/C=0.26
(period-correct LED red, distinct from MAME scarlet + NES red by
lightness + chroma).

**Not yet validated:** Operator launch (suggested single-D-pad titles:
**Mario's Tennis**, **V-Tetris**, **Wario Cruise**, **Jack Bros**,
**Galactic Pinball**, **Virtual Boy Wario Land**). Anaglyph 3D mode
spot-check. Cover sync.

**Deferred (Phase 2):**
- **Right D-pad** — Beetle VB exposes it via the right analog stick
  by default; binding it as digital input needs both Beetle VB core
  option config AND OA analog-input infra. Documented in
  KNOWN_GAME_BUGS for the ~5 dual-D-pad games (Mario Clash, Wario
  Land VB, Teleroboxer, Red Alarm, Vertical Force) which are playable
  single-D-pad but lose authentic feel.

## Per-core docs

- `ROADMAP.md` / `SESSION_LOG.md` / `KNOWN_GAME_BUGS.md` / `DECISIONS.md`.
