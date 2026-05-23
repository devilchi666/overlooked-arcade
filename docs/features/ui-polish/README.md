# UI polish — menu-bar IA + dialog reorganization

Cross-cutting work stream for the menu-bar information architecture
redesign: replacing scattered settings entry points with named menus
(Library / View / System / Game / Tools / Settings / Help) dispatching to
focused dialogs.

**Status as of 2026-05-22:** Fully shipped via `UI_POLISH_PLAN.md` Phases A–E.
The menu-bar architecture is operationalized today through the dialog
refactor (SettingsPage → LibraryManagerPage rename, sidebar Cores/Settings
buttons removed, SystemDialogs organized by intent, PerGameSettingsDrawer
shrunk). Visual top-of-window menu-bar component itself isn't the canonical
home today — the menu-organized dialogs do the work.

## Files in this folder (after Step B of the 2026-05-22 reorg)

- `UI_POLISH_PLAN.md` — design + execution spec for Phases A–E
  (✅ fully shipped). Historical reference.
- `UI_AUDIT.md` — original 2026-05-18 inventory of pre-redesign UI surfaces.
  Stale on specifics, valid as design context.
- `UI_MENU_BAR_PLAN.md` — original 2026-05-18 menu-bar proposal.
  Substantially shipped via UI_POLISH execution. Historical reference.
- `SESSION_LOG.md` — entries for UI polish work (extracted from per-core
  logs in Step C if relevant entries exist there).

## Why this lives under features/ instead of cores/

UI polish is system-agnostic — the dialog reorganization affects every
core's settings surface identically.

## Related

- Sidebar ([../sidebar/](../sidebar/)) — sidebar work assumed the menu-bar
  IA was in place.
- Kiosk shell ([../kiosk-shell/](../kiosk-shell/)) — kiosk Phase 1+ will
  build on the polished desktop foundation.
