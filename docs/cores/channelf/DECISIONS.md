# channelf Decisions Log

Fairchild Channel F-specific integration choices. Project-wide decisions live in `docs/DECISIONS.md`. Append-only.

---

## 2026-05-19 — FreeChaF as the default Channel F core

**Decision:** `default_core_dll_for_system("channelf") → "freechaf_libretro.dll"`. No widely-shipped alternate.

---

## 2026-05-19 — 9-button layout; plunger 3-axis stick maps to D-pad 4-axis

**Decision:** `CHANNELF_BUTTONS` ships D-pad (4) + FIRE + 4 console switches (MODE / TIME / START / HOLD). Plunger 3-axis analog mapping deferred to Phase 2.

**Why:** Same shared analog-input infrastructure dependency as 2600 paddles + Intv disc + 7800 Trak-Ball. D-pad approximation is playable for most Channel F titles; the precision-aim subset waits.

The 4 console switches get hardware-label keyboard bindings (M / T / Enter / H) per the original 1976 button labels — operator muscle memory matches the physical console.

---

## 2026-05-19 — Cedar-brown accent at hue 25° / L=0.45 / C=0.06; sibling to 2600 wood-grain

**Decision:** `[data-system="channelf"]` ships `oklch(0.45 0.06 25)`.

**Why:** The Channel F was the FIRST wood-grain console (1976; the 2600's wood-veneer in 1977 was inspired by it). Both deserve the wood-grain visual identity; the lineup gets two related-but-distinct earth tones:

- **2600** — yellow-pine wood (60°, L=0.60, C=0.07) — lighter, more yellow
- **Channel F** — darker red-cedar (25°, L=0.45, C=0.06) — darker, more red

The 35° hue gap + 0.15 lightness gap separate them clearly in mixed library tiles while preserving the family resemblance ("here are the two wood-grain pioneer systems").

---

## 2026-05-19 — Effectively-single-action system; documented in z_is_primary exception

**Decision:** Channel F is omitted from the `z_is_the_primary_action_button_on_every_system` test fixture. Z=FIRE is asserted explicitly inside `defaults_cover_every_channelf_button`.

**Why:** Only FIRE is a game-action button; MODE / TIME / START / HOLD are CONSOLE switches with hardware-label keyboards (M, T, Enter, H — matching the labels printed on the original 1976 console). Treating MODE as "secondary action" and asserting it lands on X would misrepresent both the hardware AND the operator's intuition.

The exception is documented in the z_is_primary test's comment block alongside the 2600 + O2 exceptions.

---

## 2026-05-19 — `.chf` only; exclude `.bin` globally

**Decision:** `extensions = ["chf"]`. `.bin` per-folder rule for older dumps.

**Why:** Same cross-system `.bin` policy.

---

## 2026-05-19 — BIOS optional

**Decision:** Document `sl31253.bin` / `sl31254.bin` / `sl90025.bin` as OPTIONAL.

**Why:** FreeChaF has an internal BIOS replacement; games run without external BIOS files. The original BIOS provides the title-menu / game-selection screen that Channel F games used; with FreeChaF's replacement, the operator goes straight to gameplay — slightly less period-correct but fully playable.
