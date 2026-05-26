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

---

## 2026-05-25 — Per-system keypad-event dispatch (not generic passthrough)

**Decision:** Wire jaguar's KP8 / KP9 / KP_STAR / KP0 / KP_HASH dispatch
as a system-specific path in the emu thread, NOT as an extension of the
existing keyboard-passthrough pump that MAME / MSX / 5200 use.

**Why:** The two paths look similar but solve different problems.

- The generic pump (`main.rs` ~line 5365, gated on
  `system_settings::default_keyboard_passthrough(system_id)`) forwards
  every raw key press the operator types to the core. MAME / MSX want
  this because the core IS the keyboard owner — TAB opens the MAME
  menu, BASIC accepts typing, etc.
- Jaguar is a gamepad-shaped system where only 5 specific BINDINGS
  happen to need keyboard-event transport because they're above bit 15
  in the bindings layout. Operators with jaguar loaded don't want the
  core to suddenly eat all their typing — they want the 5 specific
  bound bits to reach the keypad and that's it.

So jaguar's dispatch is gated on `current_system_id == "jaguar"` rather
than on the per-system keyboard-passthrough flag, and reads `polled.
buttons` (post-bindings, pre-mask) rather than raw `pressed_keys()`.
The trade-off is one branch per frame for non-jaguar systems and a
small (5-bit) loop for jaguar — well under the 4 ms render budget.

**Why dispatch fires regardless of `keyboard_passthrough_active`:** the
flag means "the core owns the keyboard." Jaguar doesn't own the
keyboard — it owns the specific bindings the operator assigned to
KP8-KP_HASH. Tying the dispatch to the flag would force operators to
toggle the system into "core-owns-keyboard" mode just to get a single
keypad binding through, which is the wrong UX.

**Generalization for intv / o2:** both systems carry the same "high
bits need keyboard transport" shape per `bindings.rs::intv` and `o2`
comments. When their Phase 2 lands, the natural refactor is to extract
the jaguar dispatcher into a generic helper that consults a per-system
`high_bit_to_retro_key` function. Not extracted now — scope discipline
per CLAUDE.md: three similar lines is better than a premature
abstraction.

**Why `#` maps to `RETROK_HASH` (35), not a keypad keycode:** libretro
defines `RETROK_KP_MULTIPLY` for `*` but no `RETROK_KP_HASH`. Virtual
Jaguar reads keypad input through standard keyboard scancodes; titles
that watch for `#` parse the keycode without caring whether it came
from the numeric keypad or the main row.

**Considered and rejected:**
- **Extend `keyboard_passthrough_active` to jaguar.** Would let the
  existing pump forward keys naturally, but introduces the "core eats
  all typing" UX problem above.
- **Add a "secondary pad bit-set" abstraction** to give KP8-KP_HASH
  proper RetroPad bits in a hypothetical extended layout. Heavier
  refactor (would touch every system's remap function); the keyboard-
  event path is the libretro-blessed way to handle keys above bit 15.
