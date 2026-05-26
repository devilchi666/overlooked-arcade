# Controller Navigation — Phase 0

Shared foundation for both the **Guided Setup** arc and the **Per-System UI** arc.
Builds the frontend primitives that let an operator navigate OA's UI with a
gamepad (Steam Big Picture style): focus manager, gamepad → UI event layer,
focus-ring component pattern, on-screen hint bar, and a Settings page.

**Source of truth:** `docs/PLANS/guided-setup.md` §10 (controller navigation
model) and §13 Phase 0 (deliverables). This folder records the implementation
slices + decisions, not the design rationale.

## Scope (Phase 0)

| Slice | Deliverable |
| --- | --- |
| A | Web Gamepad API rAF poller → synthetic UI events (button down/up, repeat, DPad/stick direction with deadzone). |
| B | Focus manager + focus-ring CSS pattern (2px outline). Roving-tabindex helpers, focus-group traversal primitives. |
| C | On-screen hint bar. Persistent footer; each screen registers A/B/X/Y/Start/Select labels via context. |
| D | Wire VirtualLibraryGrid + LeftSidebar as POC. Shoulder bumpers move focus between groups. |
| E | Settings → Controller-nav: master toggle, nav source (DPad / left stick / both), A/B swap, animation budget. |

## Out of scope for Phase 0

- Wizard step navigation (Phase 1 — wired once primitives ship).
- Per-game settings drawer, cheat editor, complex multi-pane configuration.
- Per-system audio / boot animation (Per-System UI Stage 1).
- Kiosk-shell theming.

## Design calls locked (2026-05-26)

- **Pad source:** Web Gamepad API in the frontend (rAF poll on
  `navigator.getGamepads()`). No new Rust↔JS plumbing. Existing gilrs poller
  in `oa-input` stays gated to game-window focus, so no conflict.
- **POC scope (Slice D):** VirtualLibraryGrid + LeftSidebar. No Settings
  dialog in Phase 0 — dialogs come in Phase 1 alongside the wizard step
  components.
- **Focus ring:** 2px solid outline in accent color, 8px corner radius
  matching the focused element. Quiet, fits current OA aesthetic.

See [DECISIONS.md](DECISIONS.md) for full rationale.
