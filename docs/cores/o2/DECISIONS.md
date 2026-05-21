# o2 Decisions Log

Magnavox Odyssey² / Videopac-specific integration choices. Append-only.

---

## 2026-05-19 — O2EM as the default core

**Decision:** `default_core_dll_for_system("o2") → "o2em_libretro.dll"`. No alternates exist in libretro buildbot.

---

## 2026-05-19 — Single slug covers US Odyssey² + EU Videopac G7000 + Videopac+ G7400

**Decision:** One `o2` slug for all three regional / generational variants.

**Why:** The hardware is functionally identical (same CPU + graphics chip), the games are largely the same library re-released regionally, and O2EM picks the right BIOS + timing from the loaded ROM. Splitting would create three sparse libraries instead of one healthy one.

---

## 2026-05-19 — 5-button layout; second single-action system after 2600

**Decision:** `O2_BUTTONS` ships D-pad + ACTION. Z=ACTION pinned explicitly in `defaults_cover_every_o2_button` since the system can't satisfy the `z_is_the_primary_action_button_on_every_system` fixture's primary+secondary requirement.

---

## 2026-05-19 — Keyboard input via RETRO_DEVICE_KEYBOARD passthrough

**Decision:** The 47-key alphanumeric keyboard is NOT exposed as RetroPad bits. It routes through libretro's RETRO_DEVICE_KEYBOARD device, leveraging OA's existing keyboard-passthrough mechanism (same path MAME uses for its Service / TAB-menu / typing).

**Why:** Exposing 47 keys as RetroPad bits would exhaust the bit budget AND mismatch hardware reality (the O2 keyboard is a TRUE keyboard, not a 4×4 keypad). Routing through keyboard passthrough matches the hardware shape + reuses existing infra.

A Phase 2 polish item adds per-game keyboard-overlay images for the Master Strategy Series titles (Quest for the Rings etc.) that shipped printed keyboard mapping cards.

---

## 2026-05-19 — Rose-fuchsia accent at hue 325° / L=0.62 / C=0.18

**Decision:** `[data-system="o2"]` ships `oklch(0.62 0.18 325)`.

**Why:** O2's branding didn't have a single dominant color identity — the US Odyssey² boxes used red/orange (Magnavox logo), EU Videopac used green-on-black, neither tying strongly to a hue. Picked from the open 305-335° unclaimed range; 15° from SMS magenta (340°) is enough separation given the chroma + lightness placement.

---

## 2026-05-19 — Synthetic `.o2` extension; `.bin` per-folder rule

**Decision:** `extensions = ["o2"]`. The canonical real-world Odyssey² extension is `.bin`; `.o2` is synthetic (used by some OpenEmu / RetroPie sets but not widely standardized).

**Why:** Same `.bin` collision policy as 2600 / Coleco / Intv / Channel F. Operators with `.bin`-shaped O2 libraries (the dominant case) configure per-folder `*.bin → o2` rules in the Import Wizard. Operators who use `.o2`-extension dump sets get auto-classification.

This is the FIRST system where the registered extension is "synthetic but plausible" rather than a widely-shipped real extension — accepted as the consistency-preserving choice over either polluting `.bin` globally or registering zero extensions and forcing manual rules even for the rare `.o2`-shaped library.
