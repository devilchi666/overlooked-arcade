# Unified Navigation & Panel System — arc plan

Planned 2026-06-14. Supersedes the per-panel wiring approach of the
Controller-Nav Coverage stream (Slices 1–3) for the engine surface. Operator
directive: stop hand-wiring each panel; build a **unified system so any panel
just works** with keyboard / controller / (eventually) kiosk-arcade controls,
**and** restructure the panels/windows themselves into a consistent,
input-agnostic structure and look.

## Why the current approach failed

Nav coverage was **opt-in twice over**: every surface had to (1) mount a focus
group AND (2) tag each control with `data-setting-row` / `data-setting-action`.
A panel only responded if someone hand-wired it. Result: Library's Views /
Game-media sub-tabs, content Media, Metadata, System Health, Profile, About,
and most dialogs are inert; a few embedded sub-pages mount their *own* groups
that fight for `active`. There are dozens of panels and the list grows — manual
wiring is a treadmill. (Slices 1–3 proved the *activate* layer; they did not
solve *discovery* or *structure*.)

## Two pillars

**Pillar A — Spatial navigation engine (the substrate).** One global navigator
that:
1. **Auto-discovers** focusable elements via the *native* set (`button, a[href],
   input, select, textarea, [tabindex]:not([tabindex="-1"]), [role=button|tab]`),
   visibility-filtered. No markers, ever. Scope-limited query (per active layer)
   so it's cheap.
2. **Moves by geometry** — DPad/stick → nearest focusable in that direction
   (directional-overlap + distance scoring; DOM-order tiebreak). Columns, rows,
   grids, tab-strips, mixed layouts all work with zero per-surface config. (webOS
   / Tizen / Steam-TV model.)
3. **Acts by control type on Confirm** — REUSES the Slice-1/2/3 dispatch
   verbatim: button→click, checkbox→flip, radio→select, select→**overlay
   picker**, slider→**adjust mode**, Y→**reset**, text→**OSK** (deferred,
   [../features/nav-coverage/OSK_PLAN.md](../features/nav-coverage/OSK_PLAN.md)).
4. **Scopes to the top layer** — a modal traps focus; closing restores. Wire the
   scope ONCE into the containers (`Dialog`, `EngineManagerSurface`) and
   everything rendered inside inherits it.
5. **Reuses** the existing input bus, back stack, HintBar, focus-ring CSS.
6. **Exclusion** via `tabindex="-1"` / a `data-nav-skip` opt-out (inverts the
   old opt-in marker model).

**Pillar B — Unified panel structure & look.** A standard, input-agnostic page
scaffold so panels are consistent, predictable to traverse, and kiosk-ready:
- A shared **PanelScaffold** (consistent header/title/back, region layout,
  scroll container, footer actions) every engine panel adopts.
- A consistent **control-row vocabulary** (generalize `SettingRow` into the
  shared row set all panels use) so spacing / hit-targets / focus affordances
  are uniform.
- A standard **tab / sub-tab** pattern (Library Manager Views / Game media,
  Metadata systems/games) so nested navigation is uniform.
- Kiosk/arcade readiness: large focus targets, no hover-only affordances, a
  linear-friendly fallback for limited-button cabinets.

The pillars reinforce: a consistent structure makes spatial movement
predictable; the engine makes the structure pay off across every input.

## Phases (operator chose: build + prove on Settings first)

1. **Engine + prove on Settings.** Build the spatial engine + universal
   discovery behind the existing manager. Land it on the **whole Settings
   surface** including the embedded sub-pages that are inert today (Library
   Manager tabs, Media, Metadata, System Health, Profile, About). Apply the
   Pillar-B scaffold to the Settings panels as the reference adopter.
   **Acceptance:** every Settings sub-surface is fully controller-navigable with
   zero per-control wiring, and reads consistently.
2. **Containers.** Roll the scope into `Dialog` + `EngineManagerSurface` so every
   dialog inherits nav for free; retire `Dialog.navigate`'s marker dependence.
3. **Themes/grid.** Migrate Retroverse tabs; build thin **adapters** for the
   virtualized library grid + carousels (geometry over virtualized/looping
   content needs help) so they cooperate with the engine.
4. **Kiosk/arcade input pass.** Validate + tune for limited-button cabinets
   (ties into [../features/kiosk-shell/](../features/kiosk-shell/)).

## Fold-in of Slices 1–3 (operator decision: keep 1–2, fold 3)

- **Keep** Slices 1–2 (merged to main). The `data-setting-row` /
  `data-setting-action` markers become **harmless no-ops** under universal
  discovery (native controls are discovered regardless); cleaned up opportun-
  istically, not urgently.
- **Fold** Slice 3's reusable infra into the engine: the `useSettingsRowFocus
  Group` `selector` override and the dispatch generalization (click a matched
  bare `button`/`a`). The wizard's per-panel markers are superseded by discovery.
- The **activate layer** (dispatch + select-overlay + slider-adjust + Y-reset +
  the deferred OSK) is reused wholesale — that work was not wasted.
- Slice 3 stays unmerged on `nav-coverage-slice3` as reference; do **not** merge
  it as-is (its wizard "no movement" report is moot — the engine replaces that
  wiring).

## Risks (all tractable, well-trodden)

Spatial scoring needs tuning (occasional surprising neighbor); virtualized-grid
+ modal-trap integration need care; discovery cost on large DOM (mitigate by
scoping the query to the active layer); text entry still needs the OSK. None are
blockers.

## Open design decisions (resolve during Phase 1 on the Settings prototype)

- **Movement model:** pure spatial geometry vs. a hybrid (DOM-order spine +
  spatial for left/right and grids). Validate feel on Settings before committing.
- **Pillar-B depth in Phase 1:** how much panel restructure to do up front vs.
  incrementally behind the engine.
- **Grid/carousel:** thin adapter vs. spatial-native handling of virtualized /
  looping content.

## What we keep vs. replace

- **Keep:** input bus, back stack, HintBar, control-dispatch/overlay/adjust/
  reset/OSK layer, focus-ring CSS, the per-system theming cascade.
- **Replace:** marker-based discovery → universal discovery; index +
  orientation + neighbour movement → spatial geometry.
- **Adapters:** virtualized grid + carousel keep thin specialized handlers.
