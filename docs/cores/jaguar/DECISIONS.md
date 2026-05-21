# jaguar Decisions Log

Append-only. Newest at the bottom.

---

## 2026-05-20 — Virtual Jaguar as default

**Decision:** `virtualjaguar_libretro.dll`. No practical alternate.

---

## 2026-05-20 — Full 21-button bindings with high-bit keypad keys

**Decision:** Ship the full 12-key numpad in the bindings module (21
entries total: 4 d-pad + 3 face + 2 system + 12 keypad). KP1-KP7 use
spare RetroPad bits (libretro X / L / R / L2 / R2 / L3 / R3); KP8 /
KP9 / KP_STAR / KP0 / KP_HASH live in shell-reserved high bits
(1<<16 through 1<<20).

**Why:** Operator overrode the recommended 8-button Phase 0 in favor
of full numpad coverage. Justification: Jaguar games like Iron
Soldier (weapon select 1-9), AvP (full keypad for inventory + map +
weapon switching), Cybermorph (numpad menu navigation) lean heavily
on the numpad. Surfacing the full keypad in the per-system Bindings
page is the right operator UX even if Phase 0 doesn't fully wire the
upper 5 keys to the core.

**Phase 2 polish:** Add libretro KEYBOARD device dispatch for the
high-bit keypad entries so KP8/KP9/KP_STAR/KP0/KP_HASH reach the
core when bound to keyboard keys. RetroPad bit-binding for these 5
remains unsupported (no spare bits) — operators wanting full numpad
on pad will need a Phase 2 "secondary pad bit-set" abstraction.

**Considered and rejected:**
- **Basic 8-button (recommended Phase 0).** Simpler, but operator
  picked the maximalist option.
- **8-button + 4 corner keys.** Compromise; operator went all-in
  instead.

---

## 2026-05-20 — Saturated gold 65° theme

**Decision:** `[data-system="jaguar"]` ships `oklch(0.65 0.22 65)` —
saturated gold in the open 65-75° band between 2600 wood-brown (60°,
L=0.60, C=0.07) and Atari 7800 gold (80°, L=0.78).

**Why:** Three Atari-era systems now share the warm zone, distinct
via lightness ladder: 2600 muted wood, Jaguar saturated mid, 7800
bright top. Period-correct to Atari Corp's 1993-1996 Jaguar
marketing (JAGUAR logotype + jaguar-cat-fur reference).

Operator chose Plan A (open-band gold) over Plan B (deepest red at
L=0.40, would have created a 4th member of the warm-red cluster).
