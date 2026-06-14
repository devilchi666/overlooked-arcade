# Settings IA Redesign — Decisions

Append-only. Newest at the bottom. Each entry: date + the *why*.

---

## D1 — Appearance lives in an engine "Themes/Appearance" tab, not in-theme only (2026-06-14)

Library browsing appearance (tile size, sort, group, grid/list view mode) is
**theme territory** per `theming-substrate/SURFACES.md` + DECISIONS D19. The
tension: a pure in-theme "View" menu is boundary-correct but costs discoverability
for new users. Resolution: surface appearance inside the **engine** Settings under
an expanded **Themes/Appearance** tab — you pick the active shell and configure
*its* appearance in one place. This is both discoverable AND boundary-correct
(the engine renders the picker; the theme owns the values). Rejected: a separate
"Library View" engine category (bends the boundary — puts appearance *defaults* in
engine territory) and an in-theme-only View menu (poor discoverability).

## D2 — Library = directory custodian; re-point not file-ops (2026-06-14)

OA never touches files on disk today (cleanup is DB-only). The operator is
building a **separate external ROM organizer**, so OA stays hands-off the
filesystem — no in-app move/rename/delete. Library instead gains **re-point**:
relink a folder whose ROMs moved to a new path (same ROMs). Two sub-decisions:
**(a) verify** the new path holds the matching ROM set (fast filename pass,
optional hash spot-check) before committing — chosen over blind-trust because
pointing at the wrong folder would silently break the whole library; **(b)
rebase in place** — UPDATE `folders.path` + child ROM paths by prefix-swap, never
delete+reinsert, so game ids stay stable and covers/metadata/favorites/play-time
survive the move.

## D3 — Per-theme options via a declarative theme-settings schema (2026-06-14)

The operator's own framing — "community themes will have options I never thought
of" — rules out hardcoding tile/sort/group into the Themes tab. Each theme must
**declare** its configurable options in its manifest (a typed `settings_schema`),
and OA renders them generically with card primitives. Chosen over
theme-draws-its-own-panel: once `.oatheme` packs are untrusted, a declarative
schema is the only path that stays safe (validated, sandboxed) and visually
consistent, and it lets the future Theme Studio author options. Consequence: this
work belongs with **theming-substrate Phase 5** (the `.oatheme` loader) so the
manifest format ships once. Storage half already exists (`useThemeSettings()`,
S5.4); the missing half is declaration + generic renderer.

## D4 — "Organize My Collection" is its own top-level group (2026-06-14)

OA's "Views" (custom sidebar trees) and "Collections" (flat sets) are navigation
*structure* — not appearance, not files — yet "Views" reads like appearance. They
get their own top-level **Organize My Collection** group (operator choice over
folding them into Library), the user-facing "Views" is renamed (e.g. "Sidebar
layouts") to kill the confusion, and the sidebar-systems visibility block moves
here from Library. Internal `views` store + files stay as-is.

## D5 — Import & Setup and External Emulators are new top-level groups (2026-06-14)

Importing/first-run setup and external-emulator management each get a dedicated
top-level Settings group rather than hiding inside Library / Cores. Ownership
lines to prevent duplicate homes: **Import & Setup** owns the guided flow;
**Systems** owns per-system deep config; **System Health** owns operational
status; **External Emulators** owns binaries/profiles (per-system launcher pick
stays in Systems). External Emulators coordinates with Virtual-Library **Phase D**
(install pipeline) — build the home now, wire the plumbing as that arc lands.

## D6 — Card primitives + spatial-nav everywhere (2026-06-14)

Every new landing uses the `HubGrid`/`HubCard`/`PanelScaffold` card language from
`engine/systemsHub/` and relies on the Phase-1 spatial engine to drive native
controls by geometry — zero per-control wiring. Consistency with the just-shipped
Per-System Settings Hub the operator already likes.

## D7 — Ship as sliced milestones, start at the re-skeleton (2026-06-14)

This is several arcs, not one PR. Sequence: IA re-skeleton + Library/Organize
split (frontend-only, low risk, immediately visible) **first**; then re-point
(backend), Appearance schema (with theming Phase 5), External Emulators (with VL
Phase D), Import & Setup depth. Per the operator's branch rhythm: one branch per
slice, merge at playtestable milestones.
