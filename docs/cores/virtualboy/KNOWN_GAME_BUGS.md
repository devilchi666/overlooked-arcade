# virtualboy Known Game Bugs

Per-game compatibility issues on the configured Virtual Boy core (Beetle VB by default). Append-only.

Format:

```
## <Title> (<Region>) — <Date>
- **Symptom:** <what user sees>
- **Trigger:** <what makes it happen>
- **Workaround:** <what to do — per-game core override, 3D mode, etc.>
- **Status:** <reported / confirmed / fixed upstream / works on alternate core>
```

---

## Dual-D-pad games (Phase 2 deferred)

Five Virtual Boy titles were designed around the controller's UNIQUE dual D-pad layout — the left D-pad and right D-pad operate independently. Phase 0 ships single-D-pad bindings; these games are playable but lose authentic feel:

- **Mario Clash** — left D-pad moves, right D-pad jumps independently. Without right D-pad, jump shares the left D-pad's UP direction (less precise).
- **Virtual Boy Wario Land** — right D-pad triggers special moves. Without it, those moves are unreachable in default config (per-game key remap can route them to A/B).
- **Teleroboxer** — left D-pad = left arm, right D-pad = right arm. Critically dual-handed; the game is essentially unplayable without both D-pads.
- **Red Alarm** — flight sim; right D-pad rotates camera. Playable single-D-pad but disorienting.
- **Vertical Force** — vertical shmup; right D-pad is secondary fire control.

Resolution waits on Phase 2 right-D-pad bindings work (see ROADMAP).

---

(No specific per-game bugs reported yet. Phase 1 operator validation will populate.)
