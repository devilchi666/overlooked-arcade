# Unified Navigation & Panel System — Session Log

Newest first. Three lines per entry: **Shipped / Almost / Next**.

---

## 2026-06-14 — Arc planned (pivot from per-panel wiring)

- **Shipped:** Paperwork only. Operator surfaced that per-panel nav wiring
  doesn't scale (most engine panels inert) and asked for a unified system + a
  panel structure/look restructure for keyboard / controller / kiosk. Captured
  the two-pillar design (spatial engine + unified panel structure) + 4 phases +
  the fold-in of Controller-Nav Coverage Slices 1–3 in
  [../../PLANS/unified-navigation-and-panels.md](../../PLANS/unified-navigation-and-panels.md).
  Rollout: build + prove on Settings first (operator choice). Queued in NEXT.md
  HIGH band; ACTIVE_WORK + INDEX updated.
  - **Control-floor invariant added (operator, 2026-06-14):** every action must
    be reachable with ONLY direction + Confirm + Back (arcade cabs / 2-button
    pads); all other buttons are accelerators, never the sole path. The spatial
    engine makes this structural (every actionable element auto-discovered →
    Direction+Confirm reaches it) provided Pillar B gives every action a visible,
    focusable affordance. See the plan's "Minimum control set" invariant.
- **Almost:** n/a — design only, no code.
- **Next:** **Phase 1 — spatial engine + universal discovery, proven on the
  whole Settings surface** (incl. embedded sub-pages: Library Manager tabs,
  Media, Metadata, System Health, Profile, About), with the Pillar-B panel
  scaffold applied to Settings. Open forks to resolve on the prototype: movement
  model (pure spatial vs hybrid), Pillar-B depth in Phase 1, grid/carousel
  adapter vs spatial-native. Fold in Slice 3's `selector` override + bare-button
  dispatch; keep Slices 1–2 markers as harmless no-ops.
