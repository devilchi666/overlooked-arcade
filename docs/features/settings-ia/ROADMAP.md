# Settings IA Redesign — Roadmap

Slice status. Plan: [../../PLANS/settings-ia-redesign.md](../../PLANS/settings-ia-redesign.md).

- ⬜ **Slice 1 — IA re-skeleton + Library/Organize split** *(start here;
  frontend-only)* — new top-level groups (Organize / Import & Setup / External
  Emulators), reshape Library, expand Themes → Themes/Appearance; split
  `LibraryManagerPage` into Library (folders/scan/region/cleanup) + Organize
  (Views renamed + Collections + sidebar visibility); rebuild landings as card
  hubs reusing `engine/systemsHub/` primitives. (in `engine/SettingsPanel.tsx`,
  `engine/SettingsSections.tsx`, `engine/LibraryManagerPage.tsx`)
- ⬜ **Slice 2 — Library re-point** *(backend + UI)* — `repoint_folder(folder_id,
  new_path)` Rust command (verify same-ROMs → rebase paths in place → update
  watcher) + `repointFolder` api wrapper + per-folder "Move / relink…" UI.
- ⬜ **Slice 3 — Themes/Appearance + declarative theme-settings schema** *(rides
  theming Phase 5)* — `ThemeManifest.settings_schema` + validator + generic
  `AppearancePanel` renderer; migrate global tile/sort/group/view-mode into
  per-theme namespaces.
- ⬜ **Slice 4 — External Emulators consolidation** *(rides VL Phase D)* —
  promote the CoresPage external-emulator section into the new tab; retire the
  duplicate.
- ⬜ **Slice 5 — Import & Setup depth** *(onboarding)* — Wizard + guided
  first-run (ties to `docs/features/guided-setup/` Phase 2).
