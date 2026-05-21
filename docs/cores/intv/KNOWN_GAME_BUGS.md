# intv Known Game Bugs

Per-game compatibility issues on the configured Intellivision core (FreeIntv by default). Append-only; newest at the bottom.

Format:

```
## <Title> (<Region>) — <Date>
- **Symptom:** <what user sees>
- **Trigger:** <what makes it happen>
- **Workaround:** <what to do — per-game core override, BIOS install, keypad mapping, etc.>
- **Status:** <reported / confirmed / fixed upstream / works on alternate core>
```

---

## 16-direction disc games (Phase 2 deferred)

Titles that rely on the 16-direction Intellivision disc controller for precise movement — these LOAD and run, but the 8-way D-pad mapping is lossy for them. Resolution waits on the shared analog-input infrastructure.

- **Astrosmash** — ship aiming is angle-based; 8-way feels stiff.
- **Tron Deadly Discs** — 16-direction throw + dodge.
- **Star Strike** — angle-based bombing.

(Most Intv games are playable with the 8-way mapping; only the precision-aim subset feels noticeably worse.)

---

## Keypad-required games (Phase 2 deferred)

Titles that require keypad number input during gameplay (game-mode selection at start is usually doable via the 2 START/SELECT bindings; the issue is mid-game keypad input). These load but get stuck once they reach the keypad-input phase.

- **Utopia** — uses keypad heavily for managing the island economy.
- **B-17 Bomber** — keypad for crew commands.
- **Bomb Squad** — keypad for defuse-the-bomb input.

Workaround until Phase 2 keypad coverage lands: per-game core options surface in FreeIntv allows manual key remapping.

---

(No per-game bugs reported yet beyond the categorical lists above. Phase 1 operator validation will populate.)
