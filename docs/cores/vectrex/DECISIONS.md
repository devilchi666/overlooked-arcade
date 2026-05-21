# vectrex Decisions Log

GCE Vectrex-specific integration choices. Project-wide decisions live in `docs/DECISIONS.md`. Append-only.

---

## 2026-05-20 — vecx as the default Vectrex core

**Decision:** `default_core_dll_for_system("vectrex") → "vecx_libretro.dll"`. No widely-shipped alternate.

---

## 2026-05-20 — 8-button layout (D-pad + 4 face buttons)

**Decision:** `VECTREX_BUTTONS` ships D-pad + B1/B2/B3/B4 in declaration order. B1 is leftmost-face primary; B2/B3/B4 follow in horizontal order on the physical controller.

**Why:** The Vectrex controller had a 4-direction joystick (digital despite being analog hardware — most games used it digitally) + a horizontal row of 4 face buttons. The vecx libretro core maps the 4 face buttons to libretro B/A/Y/X in that order; our identity remap follows.

The cross-system "Z is primary" rule satisfies B1 = Z naturally — the leftmost button is the first-finger primary for most Vectrex games. B3/B4 use A/S keyboard slots (matching SNES diamond layout) for the less-used buttons.

---

## 2026-05-20 — Bright phosphor-green accent at hue 165° / L=0.80 / C=0.16

**Decision:** `[data-system="vectrex"]` ships `oklch(0.80 0.16 165)`.

**Why:** Period-correct for the Vectrex's iconic green-phosphor vector display. The system's entire visual identity is "bright green vector lines on black CRT"; nothing else in the OA library evokes this aesthetic.

The hue 165° sits in the open 155-185° range — 20° from GB pea-green (145°) and 30° from Coleco cyan (195°). The lightness 0.80 (highest of any system in the lineup) makes it read as bright luminescent phosphor rather than a saturated mid-tone green; the other green-family systems (GG at 130°/L=0.72, GB at 145°/L=0.62) sit at lower lightness, so the family separates as: Vectrex = bright phosphor, GG = mid yellow-green, GB = muted pea-green.

---

## 2026-05-20 — Vector display rendered via crt-lite shader as Phase 0 compromise

**Decision:** `defaultShaderPreset: "crt-lite"`. A dedicated `vector-phosphor` shader (Gaussian glow on vector beams, no scanlines, optional persistence) is a documented Phase 2 polish item.

**Why:** The Vectrex's vector CRT is fundamentally different from raster CRT — there are no scanlines, no pixel grid, no shadow mask. Standard `crt-lite` shader (with its sample-rate scanlines + soft vignette) is technically wrong for vector content, but it provides the soft-glow + slight bloom feel that approximates the phosphor aesthetic acceptably for Phase 0.

The dedicated `vector-phosphor` preset would require:
- Gaussian / radial blur on bright pixels (vector beam glow)
- No scanline pattern
- Optional brightness-persistence (afterimage trail)

Out of Phase 0 scope; tracked in ROADMAP.

---

## 2026-05-20 — Translucent plastic overlays deferred indefinitely

**Decision:** The Vectrex's clip-on plastic color overlays are NOT emulated in Phase 0. Games render in monochrome green throughout.

**Why:** The overlay system was a tactile feature — operators clipped a physical plastic sheet onto the CRT screen to add red/blue/yellow color regions. There's no perfect emulation equivalent; the best approximation would be per-game PNG overlays composited over the framebuffer at fixed positions.

A future polish item could ship overlay PNGs per-game, but the artwork-licensing + per-game-positioning + per-game-opacity work is substantial. Documented in ROADMAP but not committed.
