# Parking Lot

Out-of-scope ideas worth keeping but not pursuing now. Anything that isn't current-phase work for the active core goes here.

Append-only. Date entries. When an item moves into scope, link the deciding entry in `docs/DECISIONS.md` and strike the parking-lot entry (don't delete — history is reference).

---

## Format

```
- YYYY-MM-DD — short title
  Why it came up: <one line>
  Why deferred: <one line>
```

---

## Items

- 2026-05-15 — Per-system overscan / safe-area / aspect-correction quirks
  Why it came up: Phase 2 scaling modes need a per-system "true aspect ratio" + overscan crop config to make "aspect-correct fit" really accurate (PCE's non-square pixels, NTSC overscan, etc.).
  Why deferred: implement the basic scaling modes first; system-specific aspect tuning becomes per-core polish in each system's bring-up.

- 2026-05-15 — Per-game scaling-mode override
  Why it came up: some games look right pixel-perfect, others (text-heavy or 240p artwork) look better stretched. Phase 2's per-game default is enough to start; full per-game override UI is more.
  Why deferred: cover the basic global default first; per-game UX comes with the library + save-state work.
