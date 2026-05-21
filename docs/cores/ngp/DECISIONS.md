# ngp Decisions Log

Append-only. Newest at the bottom.

---

## 2026-05-20 — Beetle NeoPop as the NGP/NGPC core

**Decision:** Default to `mednafen_ngp_libretro.dll` (Beetle NeoPop).
No alternates registered.

**Why:** Mednafen-derived, mature, same upstream lineage as the other
Beetle cores OA ships (PCE Fast, Saturn, PSX, VB, WonderSwan, Lynx).
Consistency wins for the Mednafen-emulated subset of OA's lineup.

---

## 2026-05-20 — NGP + NGPC share one SystemId

**Decision:** Both Neo Geo Pocket (mono, 1998) and Neo Geo Pocket
Color (1999) live under a single `ngp` SystemId. Beetle NeoPop
auto-detects hardware variant from ROM header.

**Why:** Same precedent as `gb` (DMG + CGB via Gambatte auto-detect)
and `wonderswan` (WS + WSC via Beetle WonderSwan auto-detect). Same
hardware family, same controller, same single .dll. Splitting would
create operator confusion about which slug to scan into.

---

## 2026-05-20 — Pearl yellow-green 105° theme

**Decision:** `[data-system="ngp"]` ships `oklch(0.80 0.12 105)` —
pearl yellow-green from the open 95-125° band.

**Why:** Evokes the NGPC's translucent yellow shell variant (one of
the iconic 1999 launch colors alongside platinum, sky blue, clear
purple). L=0.80 + C=0.12 reads as "pearl translucent" rather than
vivid color — handheld pastel aesthetic.

Deliberately breaks free from the SNK arcade red/gold family (cart
neogeo 18°, neocd 50°) to mark NGPC as the handheld outlier — same
precedent WonderSwan's pearl lavender (305°) uses to separate from
its arcade-era Bandai siblings.

**Considered and rejected:**
- **Family-clustered with neogeo/neocd in the warm zone.** Would
  signal "SNK family" visually but lose the handheld-vs-arcade
  distinction. Defeated by the more useful arcade/handheld split.

---

## 2026-05-20 — No BIOS pre-check

**Decision:** No `check_ngp_bios` function. The NGP/NGPC is BIOS-free
(Beetle NeoPop synthesizes the boot firmware).

**Why:** Following Beetle NeoPop's upstream behavior — there's no
external BIOS file an operator could be missing. The launch path
proceeds directly from extension check to retro_load_game.
