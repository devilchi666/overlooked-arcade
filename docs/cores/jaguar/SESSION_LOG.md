# jaguar Session Log

---

## 2026-05-25 — Phase 2 keypad-event dispatch (KP8-KP_HASH)

- **Shipped:** `bindings::jaguar_high_bit_to_retro_key` + `JAGUAR_HIGH_BIT_MASK`
  const that translates each of the 5 above-RetroPad bits (KP8 / KP9 /
  KP_STAR / KP0 / KP_HASH) to a libretro `RETROK_*` keycode (KP8 / KP9 /
  KP_MULTIPLY / KP0 / HASH — # has no keypad-specific RETROK, so it
  routes through plain RETROK_HASH = 35). Per-port edge-detection loop
  in the emu thread's Port0 input slice (`apps/oa-shell/src/main.rs`
  ~line 5794) snapshots `polled.buttons & JAGUAR_HIGH_BIT_MASK` each
  frame, compares vs prev, dispatches press/release via
  `core.send_keyboard_event` on transitions. Gated on
  `current_system_id == "jaguar" && core.has_keyboard_callback()` — the
  rest of the time the dispatcher just clears prev-state without
  emitting releases (don't poison whatever core is loaded now with
  RETROK presses it never received). `jaguar_to_libretro_bits` still
  masks bits 16-20 off the joypad mask so they don't double-dispatch.
  4 new unit tests in `bindings::tests` lock the mapping table + bit-
  mask hygiene; 6 jaguar tests + 467 others all green.
- **Almost:** Operator validation that Virtual Jaguar actually registers
  a keyboard callback. Without it, `send_keyboard_event` is a no-op
  (the dispatcher detects this via `has_keyboard_callback()` and skips
  the whole branch).
- **Next:** Operator playtest — drop `virtualjaguar_libretro.dll`,
  launch Iron Soldier, bind KP8/KP9/KP_STAR/KP0/KP_HASH to keyboard
  keys in the per-system Bindings page, confirm weapon-select cycles
  past 7. If Virtual Jaguar doesn't expose keyboard input, we'd need
  to either (a) patch our VJ fork to register the callback, or (b)
  reroute KP8/9 through `retro_input_state_t` JOYPAD overrides (which
  would need a "secondary pad" abstraction).

---

## 2026-05-20 — Phase 0 onboarding (paired with 3do + pcfx)

- **Shipped:** SystemId variant, parse_system_id arm (`jaguar | jag |
  atari-jaguar`), `bindings.rs::jaguar` module (**21-button** including
  full 12-key numpad — operator overrode the recommended 8-button
  Phase 0 in favor of full numpad coverage; KP1-KP7 in RetroPad bits,
  KP8/KP9/KP_STAR/KP0/KP_HASH in shell-reserved high bits for keyboard
  binding via per-system page, Phase 2 wires the libretro KEYBOARD
  device dispatch for those). All 4 dispatch arms wired; 3 tests lock
  the dispatch including the high-bit-masking case. Default core
  Virtual Jaguar. Media + rom_hashes arms (no-intro Atari Jaguar dat).
  CSS theme: saturated gold 65° L=0.65 C=0.22 in the open 65-75°
  Atari-warm-zone band. Per-core docs scaffold.
- **Almost:** Phase 1 operator validation.
- **Next:** Operator drops `virtualjaguar_libretro.dll`, scans Jaguar
  ROMs, launches Iron Soldier / Tempest 2000 / Rayman. Numpad weapon-
  select on Iron Soldier is the canonical "does the full keypad work"
  test (KP1-KP7 via pad shoulders or keyboard Key1-Key7; KP8-KP_HASH
  via keyboard direct only at Phase 0).
