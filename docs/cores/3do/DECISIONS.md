# 3do Decisions Log

Append-only. Newest at the bottom.

---

## 2026-05-20 — Opera as default

**Decision:** `opera_libretro.dll`. No practical alternate.

---

## 2026-05-20 — Four-entry BIOS table

**Decision:** Cover all four canonical 3DO BIOSes (Panasonic FZ-1,
Panasonic FZ-10, GoldStar GDO-101M, Sanyo Try). Operator picked
"Broader 4-6 entries" during onboarding.

**Why:** 3DO's licensee-built hardware shipped multiple regional/
manufacturer variants, all with distinct BIOSes. Most operators
have FZ-1 or FZ-10; covering all four catches the GoldStar/Sanyo
edge cases with OkCanonical rather than OkUnknownHash warnings.

---

## 2026-05-20 — Deep 3DO purple-magenta 297° theme

**Decision:** `[data-system="3do"]` ships `oklch(0.55 0.22 297)` —
deep purple-magenta in the tight Lynx 290° / WS 305° gap.

**Why:** Period-correct to the 3DO swirl logo's purple-magenta
gradient. Tight cluster placement; L/C profile separates from Lynx
(brighter L=0.65) and WS (pearl L=0.70).

---

## 2026-05-20 — Separate 3DO SystemId

**Decision:** `3do` is its own slug. Standard "every console gets its
own home" pattern.

**Why:** No shared plumbing with prior systems (distinct controller,
distinct BIOS set, distinct library). Slug name uses `3do` per the
canonical operator-typed shorthand; Rust enum variant is `ThreeDo`
(Rust identifiers can't start with a digit).
