# Settings IA Redesign

**Status:** Planned 2026-06-14; execution deferred. Slice 1 queued in
`docs/NEXT.md` HIGH band.

## What this is

Re-cuts the engine **Settings** information architecture around *user intent*
instead of legacy groupings. The trigger: Settings → "Library" conflates
library **management** (folders/scan/cleanup), library **appearance**
(tile/sort/group — actually theme territory), and library **organization**
(sidebar trees + collections) into one dense three-tab admin surface that new
users land in with no framing.

New top-level Settings groups:

- **Themes / Appearance** — pick the active shell + configure *its* options,
  per-theme, from a **declarative schema** each theme declares (so community
  themes surface options OA never hardcoded).
- **Library** — directory custodian: folders, scan options, **re-point a moved
  directory** (relink; no on-disk file ops), cleanup.
- **Organize My Collection** — sidebar trees (today's "Views", renamed) +
  Collections + sidebar-systems visibility.
- **Import & Setup** — Import Wizard + guided first-run.
- **External Emulators** — single home for standalone-emulator binaries/profiles.

Card primitives (`HubCard`/`HubGrid`/`PanelScaffold`) + spatial-nav everywhere.

## Source of truth

Plan + slices + boundaries + verification:
**[../../PLANS/settings-ia-redesign.md](../../PLANS/settings-ia-redesign.md)**.
Decisions: [DECISIONS.md](DECISIONS.md). Progress: [SESSION_LOG.md](SESSION_LOG.md).
Slice status: [ROADMAP.md](ROADMAP.md).

## Relationships

- **Appearance schema (Slice 3)** co-designs with **theming-substrate Phase 5**
  (the `.oatheme` loader — last open ARC-1 piece).
- **External Emulators (Slice 4)** rides **Virtual-Library Phase D** (install
  pipeline).
- Builds on the merged **Unified Navigation Phase 1** spatial engine + the
  **Per-System Settings Hub** card primitives.
