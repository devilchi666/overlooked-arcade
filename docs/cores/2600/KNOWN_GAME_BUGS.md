# 2600 Known Game Bugs

Per-game compatibility issues observed on the configured Atari 2600
core (Stella by default; no widely-shipped alternate libretro core).
Append-only; newest at the bottom.

Format:

```
## <Title> (<Region>) — <Date>
- **Symptom:** <what user sees>
- **Trigger:** <what makes it happen>
- **Workaround:** <what to do — per-game core override, etc.>
- **Status:** <reported / confirmed / fixed upstream / works on alternate core>
```

---

## Paddle-required games (Phase 2 deferred)

The following titles use the 2600 paddle controller (analog rotary
dial) — they LOAD and run, but are unplayable with the 8-bit joystick
bindings since the paddle is analog input. Resolution waits on the
shared analog-input infrastructure (same as Atari 7800 Trak-Ball and
Robotron 2084 twin-stick).

- **Breakout** — paddle moves the paddle.
- **Kaboom!** — paddle moves the bucket-catcher.
- **Warlords** — 4 simultaneous paddle players.
- **Super Breakout** — same as Breakout.
- **Night Driver** — paddle steers.
- **Indy 500** — driving-controller hybrid (paddle-shape, but spinner
  semantics). Same gating.
- **Casino** — paddle as selector.
- **Backgammon** — paddle as selector.

(Documented up-front rather than waiting for the operator to discover
each one individually. Add new paddle-required titles here as found.)

---

(No per-game bugs reported yet beyond the paddle-deferral list above.
Phase 1 operator validation will populate this file with whatever
per-game quirks show up.)
