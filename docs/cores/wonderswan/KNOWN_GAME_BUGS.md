# wonderswan Known Game Bugs

Per-game compatibility issues on the configured WonderSwan core (Beetle WonderSwan by default). Append-only.

Format:

```
## <Title> (<Region>) — <Date>
- **Symptom:** <what user sees>
- **Trigger:** <what makes it happen>
- **Workaround:** <what to do — per-game core override, rotation, BIOS install, etc.>
- **Status:** <reported / confirmed / fixed upstream / works on alternate core>
```

---

(No entries yet. Phase 1 operator validation will populate.)

Likely categorical entries once validated:
- **Vertical-orientation games** — Beetle WS auto-detects from ROM header; confirm rotation feels correct. If a game ships with the wrong header flag, the workaround is the per-game rotation override (Phase 2).
- **Japan-only titles with text-heavy gameplay** — Final Fantasy I + II remakes, Bandai franchise titles, anything with significant Japanese text. Playable but operator-side translation patches (when available) need per-game ROM swap, not a core issue.
- **Cable-link multiplayer titles** — single-player works; multiplayer not supported.
