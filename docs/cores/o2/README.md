# o2 — Magnavox Odyssey² / Videopac

Onboarded 2026-05-19. Drives the Magnavox Odyssey² (US, 1978-1983) and
its European equivalent Videopac G7000 (Philips, 1978-1983) plus the
later Videopac+ G7400 via the libretro **O2EM** core
(`o2em_libretro.dll`). Intel 8048 CPU + Intel 8244/8245 graphics chip.

The Odyssey² launched in the same year as the Atari 2600 (1978) but
shipped with a unique design choice — a full 47-key alphanumeric
keyboard integrated into the console. This let it hybrid as a "home
computer" for educational titles (Type & Tell, Computer Programmer)
while also playing arcade-style games (KC Munchkin, Pick Axe Pete).

## Upstream

- **Default core:** O2EM — https://github.com/libretro/libretro-o2em
- **Alternates:** No widely-shipped libretro alternate.
- **Vendored:** No.

## ROM format

- **`.o2`** — synthetic extension used by some OpenEmu / RetroPie sets.
  The Odyssey² community NEVER standardized — almost every real
  library is `.bin`. Operators with `.bin`-shaped libraries configure
  per-folder `*.bin → o2` rules in the Import Wizard.
- **`.bin`** — intentionally NOT registered globally; per-folder rule.

## BIOS

- **REQUIRED:** `o2rom.bin` for US Odyssey², and/or `c52.bin` for
  EU Videopac G7000 + Videopac+ G7400. Both in `<exe_dir>/system/`.
- O2EM picks based on the loaded ROM's region marker.

## Native timing

- **NTSC (US Odyssey²):** 59.92 Hz, **160×192** visible.
- **PAL (EU Videopac G7000):** 49.86 Hz, **160×240** visible.

## Input

5-button layout — D-pad + single ACTION button. The Odyssey² is the
second single-action system in OA's lineup (after 2600). The 47-key
alphanumeric keyboard for game-specific input goes through libretro
RETRO_DEVICE_KEYBOARD via OA's keyboard passthrough (same path MAME
uses).

| OA Button | libretro bit | Keyboard | Pad |
|---|---|---|---|
| UP/DOWN/LEFT/RIGHT | UP/DOWN/LEFT/RIGHT | Arrows | DPad |
| ACTION | B (0) | Z | East (single primary action) |

**Single-button exception:** Like 2600, O2 doesn't appear in the
`z_is_the_primary_action_button_on_every_system` fixture; the Z=ACTION
assertion lives in `defaults_cover_every_o2_button` instead.

## Current status (2026-05-19)

**Works:** Core resolution, 5-button bindings, library scanner
classifies `.o2`, theme accent rose-fuchsia 325°/L=0.62/C=0.18.

**Not yet validated:** Operator launch (suggested: **KC Munchkin**,
**Pick Axe Pete**, **Quest for the Rings** — the latter needs the
keyboard for character-class selection). Cover sync. BIOS workflow.
