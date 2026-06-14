# Settings IA Redesign — Session Log

Newest first. Three lines per entry: **Shipped / Almost / Next**.

---

## 2026-06-14 — Slice 1 implemented (code-complete, pending playtest)

- **Shipped (branch `feat/settings-ia-slice-1`, not yet merged):** the IA
  re-skeleton + Library/Organize split. `engine/SettingsPanel.tsx` gained three
  new CONTENT categories — **Import & Setup**, **Organize My Collection**,
  **External Emulators** (shell) — Themes relabelled **Themes / Appearance**,
  Library reworded to management-only; content sidebar order is now Import &
  Setup → Library → Organize → Systems → External Emulators. `LibraryManagerPage`
  lost its Views tab + the sidebar-systems visibility block and its `layout`/
  `views` props (now a 2-tab Library/Game-media management surface). New engine
  components: `OrganizeLanding` (mounts `ViewsManagerTab` under the renamed
  "Sidebar layouts" + new `CollectionsManager` + extracted `SidebarSystemsCard`),
  `ImportSetupLanding` (Wizard CTA + folder add/rescan `HubCard`s),
  `ExternalEmulatorsLanding` (honest placeholder pointing at the current
  System-Health → Cores location; real consolidation is Slice 4),
  `CollectionsManager` (CRUD over `customCollections`), `SidebarSystemsCard`.
  `LibrarySettings` slimmed to just embed the management surface (CTA moved to
  Import & Setup). typecheck + lint (six boundary zones green) + vitest(98) pass;
  frontend-only (cargo unaffected).
- **Almost:** External Emulators is a shell (Slice 4 rides VL Phase D); Themes /
  Appearance is relabel-only (the declarative per-theme schema is Slice 3).
- **Next:** operator playtest the new Settings IA (spatial-nav across the new
  card landings; confirm Organize/Import/Library all work) → merge to main. Then
  Slice 2 (Library re-point).

## 2026-06-14 — Planning session (design locked, execution deferred)

- **Shipped:** the design. Explored the current Settings → Library surface
  (`engine/SettingsSections.tsx::LibrarySettings` → `LibraryManagerPage` 3-tab
  manager) and confirmed the key finding — today's Settings → Library is ~100%
  *management*; appearance (`sortKey`/`groupBy`/`viewMode`) is wired into
  `LibraryView` but has **no UI** (only tile-size, via `GridControls`), and is
  theme territory per SURFACES/D19. Operator re-cut the whole Settings IA around
  user intent: new top-level **Themes/Appearance · Library · Organize My
  Collection · Import & Setup · External Emulators** groups. Decisions D1–D7
  locked (verify on re-point; declarative per-theme settings schema; start at the
  re-skeleton). Plan written to
  [../../PLANS/settings-ia-redesign.md](../../PLANS/settings-ia-redesign.md);
  feature folder + NEXT.md Slice 1 + ACTIVE_WORK + INDEX updated.
- **Almost:** nothing built — planning only.
- **Next:** **Slice 1 — IA re-skeleton + Library/Organize split** (frontend-only;
  `engine/SettingsPanel.tsx` groups/categories + split `LibraryManagerPage` into
  Library + Organize card hubs reusing `engine/systemsHub/` primitives). Queued
  in NEXT.md HIGH band.
