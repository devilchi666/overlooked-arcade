# ps2 Decisions Log

Append-only.

---

## 2026-05-20 — LRPS2 (PCSX2) as default

`pcsx2_libretro.dll`. No practical libretro alternate.

---

## 2026-05-20 — Deep PS2 cobalt 215° theme

`oklch(0.45 0.22 215)` — period-correct to the PS2's iconic blue
PlayStation logo + dark-hardware-era marketing. Slots into the
Sony cluster (psx 180° / psp 200° / ps2 215°) with the deepest
lightness in that cluster.

---

## 2026-05-20 — DualShock 2 with L3/R3, analog via shared infra

16 digital buttons covering DS2's PSX-shape + L3/R3 stick clicks.
Dual analog sticks flow through the cross-cutting analog input infra
shipped earlier today (n64+gamecube session). Pressure-sensitive face
buttons + analog L2/R2 triggers deferred to Phase 2.5 (shared
deferral with GameCube's analog L/R triggers).

---

## 2026-05-20 — Six-entry BIOS table

`PS2_BIOS_KNOWN_HASHES` ships with 6 entries covering JP launch /
US fat / US-EU slim variants spanning 2000-2010. The PS2 had many
revisions over its 13-year lifespan; 6 entries cover ~95% of operator-
installed BIOSes. Less-common revisions get OkUnknownHash + warn-toast.
