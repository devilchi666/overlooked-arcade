# pcfx Decisions Log

Append-only. Newest at the bottom.

---

## 2026-05-20 — Beetle PC-FX as default

**Decision:** `mednafen_pcfx_libretro.dll`. No practical alternate.

**Why:** Beetle PC-FX is the canonical libretro PC-FX core; Mednafen-
derived, same lineage as the other Beetle cores OA ships
(consistency across the Mednafen-emulated subset).

---

## 2026-05-20 — Separate `pcfx` bindings module (not shared with `pce::*`)

**Decision:** PCFX gets its own bindings module with 12 entries
(d-pad + I-VI + RUN + SELECT). NOT shared via dispatch arm with the
existing `pce::*` module.

**Why:** `pce::*` is a 2-button I/II layout sized for TG-16 / PCE-CD's
HuCard / CD library which is mostly 2-button. PCFX uses the
post-1993 PCE 6-button pad with III/IV/V/VI on additional libretro
bits (L/R/L2/R2). Sharing the pce module would mean extending it with
III-VI, which would either:
1. Make those bits unused for tg16/pce-cd defaults (wasteful), OR
2. Break tg16/pce-cd defaults (the 2-button pad doesn't have III-VI).

Separate module keeps each system's defaults clean.

**Considered and rejected:**
- **Extend pce module with III-VI.** Would force tg16/pce-cd to either
  acknowledge the 6-button extras (incorrect for their 2-button-pad
  reality) or skip them (wasted bits). Defeated.

---

## 2026-05-20 — Anime pink-magenta 320° theme

**Decision:** `[data-system="pcfx"]` ships `oklch(0.62 0.24 320)` —
saturated pink-magenta in the tight WonderSwan 305° → O2 325° gap.

**Why:** Period-correct to PC-FX's identity as a Japan-only anime/VN/
dating-sim platform — the marketing palette leaned heavily into
vivid pinks. L=0.62 + C=0.24 reads as "saturated anime pink",
visually distinct from WS pearl lavender (lower C) and O2 rose-
fuchsia (slightly different hue).

---

## 2026-05-20 — Single-entry BIOS table

**Decision:** `PCFX_BIOS_KNOWN_HASHES` ships with one entry — the
canonical `pcfx.rom`.

**Why:** PC-FX was Japan-only; no regional or revision variants
shipped. The platform's commercial failure (62 retail releases over
4 years) means no third-party "alternative" BIOS dumps either —
unlike PSX which has scph5500/5501/5502 regional triplets, PCFX has
exactly one canonical BIOS.
