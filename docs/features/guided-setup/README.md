# Guided Setup

Upgrade the existing Import Wizard into a guided-setup flow that handles
smart ROM/system matching, per-system readiness, curated core
recommendations by CPU tier, optional canonical folder layout, and
controller-navigable UI from day one.

**Source of truth:** [docs/PLANS/guided-setup.md](../../PLANS/guided-setup.md)
— 488-line locked plan from the 2026-05-25 advisor + operator planning
session. Read that for goals / audience / voice / wizard flow / phase
breakdown / open questions. This folder records implementation slices +
decisions, not design rationale.

## Phase structure

| Phase | Scope | Status |
| --- | --- | --- |
| Phase 0 | Controller-nav primitives (shared with Per-System UI) | ✅ shipped 2026-05-26 — see [features/controller-nav/](../controller-nav/) |
| Phase 1B | Wizard upgrade (~3-4 weeks) | 🔄 in flight — see below |
| Phase 2B | Curated core selection (~1 week) | ⬜ pending |
| Phase 2C | Folder management (~1 week) | ⬜ pending |
| Phase 2D | First-system bindings + KNOWN_GAME_BUGS overrides (~1 week) | ⬜ pending |
| Phase 2E | Help suppression registry (~3-4 days) | ⬜ pending |
| Phase 2F | Existing-operator re-entry (~3-4 days) | ⬜ pending |

## Phase 1B — Wizard upgrade

Centerpiece: a per-ROM results table with confidence-tiered classification.
Today's wizard knows nothing per-row — classification is extension-only on
the frontend, SHA-1 identification only happens post-commit. Phase 1B moves
the smart classification into the scan path and surfaces it in a
LaunchBox-inspired table.

| Slice | Deliverable | Status |
| --- | --- | --- |
| 1 | Smart-scan emission (backend) + Settings → Library entry point | ✅ shipped 2026-06-01 |
| 2 | Per-ROM results table in the wizard (consumes Slice 1 fields) | ✅ shipped 2026-06-01 |
| 3 | Per-system readiness checklist component (also Settings → System Readiness) | ✅ shipped 2026-06-01 |
| 4 | Bulk-prompt missing-core download (wires `core_installer.rs`) | ✅ shipped 2026-06-01 |
| 5 | Guided BIOS resolution UI | ⬜ next |
| 6 | Voice/tone copy pass + first-launch empty-state | ⬜ pending |

Slice 1's plan + execution notes lived off-tree at
`C:\Users\Devilchi\.claude\plans\spicy-shimmying-crescent.md`. Once Phase
1B wraps, that plan's design notes get folded into this folder's
DECISIONS.md if any non-obvious calls warrant capture.

## Operator-locked decisions (Slice 1, 2026-06-01)

- **Approach for Slice 1:** backend-first. Build the smart-scan data
  layer once; UI consumes it in later slices.
- **Hashing strategy:** foreground, hash everything during scan. Stream
  progress so a large library still looks alive.
- **Wizard relation across slices:** replace in place — mutate
  `ImportWizard.tsx` across slices rather than build a V2 alongside.
- **Wizard entry point:** Settings → Library → "Re-scan with smart
  detection" card. Re-establishes a path to the wizard (orphaned after
  the 2026-05-31 legacy Shell deletion) and matches plan §12 IA.
- **Empty-state entry point:** deferred to Slice 6 — lands once the
  table + readiness checklist are wired so first-time operators see
  the full new flow, not a half-built one.

## Out of scope (parked)

See `docs/PLANS/guided-setup.md` §15 "v2 / future additions" — per-ROM
table actions deferred to v2, watcher-triggered scan toast, theme
ecosystem (WAIT lock), Netplay + RetroAchievements (separate strategic
decisions).
