# wonderswan — Bandai WonderSwan + WonderSwan Color

Onboarded 2026-05-20. Drives the Bandai WonderSwan (1999, mono) and
WonderSwan Color (2000, color refresh) — plus the late SwanCrystal
(2002, improved screen) — via the libretro **Beetle WonderSwan** core
(`mednafen_wswan_libretro.dll`). NEC V30MZ CPU (16-bit, ~3.07 MHz) +
custom LCD controller + small mono speaker.

The WonderSwan was Gunpei Yokoi's post-Nintendo handheld design at
Bandai. Japan-only retail. The library is small but distinctive —
heavy on Bandai franchise tie-ins (Gundam, Digimon, One Piece, Naruto)
plus original gems (Final Fantasy I + II remakes, GunPey, Riviera
prototype, the Klonoa series).

**Unique design:** the WS shipped with DUAL 4-way D-pads (X-pad mounted
in the lower-left, Y-pad mounted in the lower-right) so the operator
could rotate the device 90° for vertical games — the active D-pad
becomes whichever one points "down" in the chosen orientation.

## Upstream

- **Default core:** Beetle WonderSwan — https://github.com/libretro/beetle-wswan-libretro
- **Alternates:** No widely-shipped alternate.
- **Vendored:** No.

## ROM format

- **`.ws`** — original mono WonderSwan dump.
- **`.wsc`** — WonderSwan Color dump.

Both routed to the same `wonderswan` slug — Beetle WS auto-detects
mono vs color from the ROM header (same single-slug-multi-hardware
pattern as `gb` covering DMG + CGB).

## BIOS

- **Optional.** Beetle WS has internal BIOS replacement. The original
  WS BIOS (`bios.ws`) + WSC BIOS (`bios.wsc`) handled boot splash +
  name-entry / clock-init screens. Without them, Beetle WS goes
  straight to gameplay. Drop into `<exe_dir>/system/` for era-correct
  boot.

## Native timing

- **All regions (JP only retail):** ~75.47 Hz refresh, **224×144**
  visible.

## Display rotation

WS games come in two physical orientations:
- **Horizontal** — the device held with X-pad on the lower-left,
  Y-pad on the upper-right (Y-pad becomes 4 face buttons).
- **Vertical** — the device rotated 90°, Y-pad on the lower-left
  (becomes the active D-pad), X-pad on the upper-right (becomes face
  buttons).

Beetle WS reads the ROM header's orientation flag and rotates the
framebuffer + auto-swaps the active D-pad accordingly. From the OA
bindings layer this is invisible — the player just gets a single
4-way D-pad that "works correctly" regardless of game orientation.

**Per-game framebuffer rotation override** is a Phase 2 polish item —
some operators may want to keep the screen orientation fixed and let
the game render rotated (90° physical rotation simulation) vs the
default of auto-rotating to match the monitor's natural axis.

## Input

7-button layout — D-pad + A + B + START. Identity libretro remap.

| OA Button | libretro bit | Keyboard | Pad |
|---|---|---|---|
| UP/DOWN/LEFT/RIGHT | UP/DOWN/LEFT/RIGHT | Arrows | DPad |
| A | A (8) | Z | East — primary |
| B | B (0) | X | South — secondary |
| START | START (3) | Enter | Start |

No SELECT button on the WS hardware. The hardware "Sound" volume
button doesn't surface as a RetroPad bit; it lives in Beetle WS's
core options.

## Current status (2026-05-20)

**Works:** Core resolution, 7-button bindings, library scanner
classifies `.ws` + `.wsc`, theme accent pearl-lavender
305°/L=0.70/C=0.14.

**Not yet validated:** Operator launch (suggested: **Final Fantasy I**
[WS Color], **Klonoa: Moonlight Museum**, **GunPey**, **Riviera:
The Promised Land** [WS Color prototype], **Rockman + Forte**, **One
Piece: Treasure Wars**). Cover sync. Per-game vertical-rotation
spot-check (Riviera is vertical-mode if I recall correctly).

**Deferred (Phase 2):**
- **Multi-repo cover sync** — same gap as `gb` (single slug covers
  mono + color; the libretro-thumbnails repo for WSC is separate
  from the WS repo). Phase 0 ships primary repo `Bandai_-_WonderSwan`;
  WSC-specific covers via `Bandai_-_WonderSwan_Color` is the
  multi-repo follow-up.
- **Per-game framebuffer rotation override** for the few cases where
  operators want fixed monitor orientation + rotated game render.

## Per-core docs

- `ROADMAP.md` / `SESSION_LOG.md` / `KNOWN_GAME_BUGS.md` / `DECISIONS.md`.
