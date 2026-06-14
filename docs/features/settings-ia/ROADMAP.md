# Settings IA Redesign — Roadmap

Slice status. Plan: [../../PLANS/settings-ia-redesign.md](../../PLANS/settings-ia-redesign.md).

- ✅ **Slice 1 — IA re-skeleton + Library/Organize split** *(MERGED to main
  2026-06-14, operator playtested clean — merge `e71eef0`)* — added the
  new top-level CONTENT categories **Import & Setup** (`ImportSetupLanding`),
  **Organize My Collection** (`OrganizeLanding`), **External Emulators**
  (`ExternalEmulatorsLanding`, shell only); relabelled Themes → **Themes /
  Appearance**; reshaped **Library** to management-only. Split
  `LibraryManagerPage` (dropped the Views tab + the sidebar-systems block; now a
  2-tab Library/Game-media management surface; `layout`/`views` props removed).
  New Organize landing mounts `ViewsManagerTab` (user-facing "Views" → "Sidebar
  layouts") + new `CollectionsManager` (over `customCollections`) + extracted
  `SidebarSystemsCard`. Import & Setup landing carries the Wizard CTA + folder
  add/rescan as `HubCard`s. typecheck + lint (boundary zones green) + vitest(98)
  all pass; frontend-only. Files: `engine/SettingsPanel.tsx`,
  `engine/SettingsSections.tsx`, `engine/LibraryManagerPage.tsx`, +
  `engine/{OrganizeLanding,CollectionsManager,SidebarSystemsCard,ImportSetupLanding,ExternalEmulatorsLanding}.tsx`.
- ✅ **Slice 2 — Library re-point** *(MERGED to main 2026-06-14, operator
  playtested clean — merge `59b0d52`)* — Rust
  `preview_repoint_folder` + `repoint_folder` commands in `library_db.rs`
  (filename-existence verify → in-place path rebase in a tx; folder + game ids
  stay stable so covers/metadata/favorites/play-time survive; sibling-folder
  boundary respected) + `previewRepointFolder`/`repointFolder` api wrappers +
  per-folder **"Relink…"** button → pick dir → preview (matched/missing confirm)
  → commit → `refreshLibraryFolders` (re-registers the watcher via App.tsx's
  effect) + `refreshGroups`. 2 new Rust tests (rebase+preserve, preview
  matched/missing). cargo 839 + typecheck/lint/vitest(98) green.
- ⬜ **Slice 3 — Themes/Appearance + declarative theme-settings schema** *(rides
  theming Phase 5)* — `ThemeManifest.settings_schema` + validator + generic
  `AppearancePanel` renderer; migrate global tile/sort/group/view-mode into
  per-theme namespaces.
- ⬜ **Slice 4 — External Emulators consolidation** *(rides VL Phase D)* —
  promote the CoresPage external-emulator section into the new tab; retire the
  duplicate.
- ⬜ **Slice 5 — Import & Setup depth** *(onboarding)* — Wizard + guided
  first-run (ties to `docs/features/guided-setup/` Phase 2).
