# Unified Navigation & Panel System — Session Log

Newest first. Three lines per entry: **Shipped / Almost / Next**.

---

## 2026-06-14 — Phase 1: spatial engine, proven on Settings (Pillar A)

- **Shipped (branch `feat/unified-nav-phase-1`, merged to main):** The
  spatial-navigation engine + adoption on the whole Settings surface.
  - **Engine** (`platform/nav/spatial.tsx` + pure `spatialGeometry.ts` +
    `sliderStep.ts`): a global **layer stack**; **native focusable
    auto-discovery** (no markers, visibility-filtered, scoped to the top
    layer); **geometry movement**; the Slice-1/2/3 **activate layer reused**
    (checkbox/radio flip · select→overlay picker · slider→adjust mode ·
    button/link click · Y-reset accelerator); gamepad **and** keyboard routing;
    layer scoping/trap. `focus.ts` bypasses the legacy index manager whenever a
    spatial layer is active (exactly one model per event) + suppresses ghost
    rings from inert embedded index groups.
  - **Movement model = region-bias hybrid** (resolved the open fork via
    playtest, DECISIONS D1): UP/DOWN move *within* a region, LEFT/RIGHT cross
    *between* regions — matching the operator's locked nav spec. Regions derive
    from existing landmarks (`<aside>`/`<nav>`) + a `data-nav-region` override
    hook (added to the Settings center pane). Pure-flat-plane darting (Down
    diving to the far-left categories) is gone.
  - **Adoption:** `EngineManagerSurface` pushes ONE spatial layer over the
    takeover; `SettingsPanel` dropped both index focus groups; the platform
    `Dialog` routes to a spatial layer when active (`SpatialDialogLayer` —
    Bindings/Core-options now navigable); custom modals (Import Wizard,
    Game-media panel, Missing-cores prompt) wrapped in `SpatialModalScope` +
    z-lifted above the takeover.
  - **Three playtest rounds fixed:** (1) hidden `<select>` inside a collapsed
    `<details>` trapped Down → skip collapsed-details content + make the
    engine's `lastFocused` the source of truth (focusin-synced) so movement
    can't pin to an element that refused focus; (2) one-flat-plane darting →
    region-bias hybrid; (3) content stranded in the layer catch-all →
    `data-nav-region` on the center pane (sub-nav now reachable).
  - Verify: typecheck + lint + vitest (97, +12 geometry) + build all green.
- **Almost:** Pillar B (PanelScaffold + unified control-row vocabulary + stale
  HintBar labels on the Settings surface) — engine-first by design; deferred so
  feel could be validated first.
- **Next:** **Pillar B — unified panel structure/look on Settings** (the
  reference adopter), then Phase 2 (roll the scope formally into `Dialog` +
  retire `Dialog.navigate` markers) and Phase 3 (themes/grid adapters).

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
