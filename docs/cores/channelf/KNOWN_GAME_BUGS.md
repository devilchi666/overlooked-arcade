# channelf Known Game Bugs

Per-game compatibility issues on the configured Channel F core (FreeChaF by default). Tiny library (~26 titles), so this file is expected to stay short.

Append-only.

Format:

```
## <Title> (<Region>) — <Date>
- **Symptom:** <what user sees>
- **Trigger:** <what makes it happen>
- **Workaround:** <what to do — per-game core override, plunger mapping, etc.>
- **Status:** <reported / confirmed / fixed upstream / works on alternate core>
```

---

## Plunger-precision games (Phase 2 deferred)

The Channel F plunger was a true 3-axis stick. The D-pad 4-direction approximation loses precision for titles that lean on continuous-axis control:

- **Pinball** — paddle position is continuous.
- **Robot War** — twist for direction is angular.
- **Dodge'em** — same.

(These run; they just feel less precise than original hardware. Resolution waits on shared analog-input infra — same gate as 2600 paddles + Intv 16-direction disc.)

---

(No per-game bugs reported. Phase 1 operator validation will populate.)
