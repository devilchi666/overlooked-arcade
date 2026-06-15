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
- ✅ **Slice 3 — Themes/Appearance + declarative theme-settings schema**
  *(MERGED to main 2026-06-14, operator playtested clean — merge `5386305`)* —
  `ThemeSettingControl`/`ThemeSettingsSchema` + `settings_schema` on
  `ThemeManifest`; S4 validator extended (`SETTING_KEY_INVALID`/
  `SETTING_CONTROL_INVALID`, +9 vitest cases); generic `engine/AppearancePanel`
  renders the active theme's schema via `SettingRow` bound to `useThemeSettings`
  (per-control reset), mounted in Themes / Appearance. `bare`'s Compact toggle
  moved hand-coded → declarative. **Retroverse** migrated: declares
  tile/sort/group/view in its schema, reads them per-theme via a new
  `LibraryAppearance` prop on the shared `LibraryView` (platform grid stays
  config-agnostic — boundary-clean), one-time seed from the global layout so
  nothing jumps. Manifest `settings_schema` format ships now; theming Phase 5's
  `.oatheme` loader reuses it. typecheck/lint/vitest(107) green; THEME_CONTRACT
  §8 added.
- 🟡 **Slice 4 — External Emulators consolidation** *(IN PROGRESS)* — relocate
  the existing CoresPage "External emulators" section (binary paths + per-system
  default-launcher) into the `ExternalEmulatorsLanding`, retire the duplicate.
  The one-click *install pipeline* depth still rides **VL Phase D** (unbuilt) —
  this slice ships the consolidation + a hook for the installer. Paired with a
  research-need doc cataloguing the external-emulator roster +
  command-line-launch details we must gather (both systems we DON'T support via
  cores AND ones we DO, so users have options): [RESEARCH/external-emulators.md](../../RESEARCH/external-emulators.md).
- ⬜ **Slice 5 — Import & Setup depth** *(onboarding)* — Wizard + guided
  first-run (ties to `docs/features/guided-setup/` Phase 2).
