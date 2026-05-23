# Sidebar — library tree + view editor

Cross-cutting work stream for the left sidebar's tree-of-platforms model,
the named-views infrastructure backing it, and the View Editor that lets
operators edit views in Library Manager.

**Status as of 2026-05-22:** Substantially complete. Tier plan (PR-α/β/γ)
fully shipped 2026-05-21; v2.1–v3.5 (Manufacturers view, cross-container
drag, un-hide UX, Views tab, ViewEditorPane, accent picker, schema v2 with
explicitlyRemoved) shipped 2026-05-22. v3.4 per-container art slots
intentionally parked in `docs/PARKING_LOT.md` pending storage + format
design.

## Files in this folder (after Step B of the 2026-05-22 reorg)

- `SIDEBAR_TIER_PLAN.md` — design + execution spec for the tier rework
  (✅ fully shipped). Historical reference.
- `VIEW_EDITOR_PLAN.md` — design + execution spec for v3 view editing
  (✅ substantially shipped; v3.4 parked). Historical reference.
- `SESSION_LOG.md` — entries for sidebar work (extracted from
  `docs/cores/nds/SESSION_LOG.md` in Step C).
- `SESSION_LOG_ARCHIVE.md` — older entries (after Step D capping).

## Why this lives under features/ instead of cores/

Sidebar work spans every system in the registry — the tree, accent picker,
View Editor, and reconciler all behave identically across cores. Filing
under a per-core SESSION_LOG (as happened pre-reorg) misrepresented the
scope and made the work hard to find later.

## Related

- Kiosk shell ([../kiosk-shell/](../kiosk-shell/)) — Phase 1+ will consume
  the same views data model.
- UI polish ([../ui-polish/](../ui-polish/)) — preceding desktop polish that
  the sidebar rewrite assumed.
