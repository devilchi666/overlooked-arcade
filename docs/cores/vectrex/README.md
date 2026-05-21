# vectrex — GCE Vectrex

Onboarded 2026-05-20. Drives the GCE Vectrex (1982-1984 retail) via the
libretro **vecx** core (`vecx_libretro.dll`). Motorola 6809 CPU + AY-3-8912
sound + DAC-driven X/Y vector display + integrated 9-inch CRT.

**The Vectrex is unique:** the ONLY home console with a built-in vector
CRT (no raster), a portable form factor (with a carrying handle), and a
library designed around vector graphics (Mine Storm, Berzerk, Star Trek,
Star Castle). The system also shipped with translucent plastic overlays
that clipped onto the screen to add color tinting to specific games —
since the monitor itself was monochrome green-phosphor.

## Upstream

- **Default core:** vecx — https://github.com/libretro/libretro-vecx
- **Alternates:** No widely-shipped libretro alternate.
- **Vendored:** No.

## ROM format

- **`.vec`** — canonical Vectrex dump extension.
- **`.gam`** — alternate extension used by some dump sets.

## BIOS

- **Optional:** `vectrex.bin` (~8 KB) in `<exe_dir>/system/`. The
  Vectrex BIOS contains the boot routines + the **Mine Storm** pack-in
  game built into the system ROM (Mine Storm was always present
  without any cart inserted). Without the BIOS, vecx has an internal
  replacement; games still run, just no Mine Storm and no era-correct
  splash.

## Native timing

- **Vector refresh:** ~50 Hz nominal vector-beam refresh, rendered to
  a raster framebuffer by vecx at the OA renderer's framerate. The
  resolution is virtual since vector displays don't have native pixel
  dimensions; vecx exposes a configurable raster output (typically
  330×410 to match the integrated CRT's portrait aspect ratio).

## Input

8-button layout — 4-direction joystick + 4 face buttons (B1 leftmost,
B2, B3, B4 rightmost in a horizontal row on the controller). Identity
libretro remap; B1/B2/B3/B4 map to libretro B/A/Y/X respectively per
the vecx convention.

| OA Button | libretro bit | Keyboard | Pad |
|---|---|---|---|
| UP/DOWN/LEFT/RIGHT | UP/DOWN/LEFT/RIGHT | Arrows | DPad |
| B1 | B (0) | Z | East — primary |
| B2 | A (8) | X | South — secondary |
| B3 | Y (1) | A | West — tertiary |
| B4 | X (9) | S | North — quaternary |

The 4 face buttons followed the SF-fighter "primary action on the
right" convention even before that was a convention — vecx maps them
in a way that the "Z = primary" rule lines up naturally with B1 (the
leftmost button being the first-finger primary).

## Current status (2026-05-20)

**Works:** Core resolution, 8-button bindings, library scanner
classifies `.vec` + `.gam`, theme accent bright phosphor-green
165°/L=0.80/C=0.16.

**Not yet validated:** Operator launch (suggested: **Mine Storm**
[built into BIOS], **Berzerk**, **Star Trek: The Motion Picture**,
**Spike**, **Bedlam**, **Pole Position**, **Solar Quest**), BIOS
install, cover sync.

**Deferred (Phase 2):**
- **Vector-phosphor shader preset** — `crt-lite` is the temporary
  default; a dedicated `vector-phosphor` preset (Gaussian glow on
  vector lines, no scanlines, optional persistence trail) would be
  era-correct.
- **Translucent overlay rendering** — the Vectrex's plastic-overlay
  color system is a tactile feature with no perfect emulation
  equivalent. A future polish item could ship overlay PNGs per-game
  composited over the framebuffer.

## Per-core docs

- `ROADMAP.md` — phase tracking.
- `SESSION_LOG.md` — Shipped / Almost / Next.
- `KNOWN_GAME_BUGS.md` — per-game compatibility.
- `DECISIONS.md` — Vectrex-specific integration choices.
