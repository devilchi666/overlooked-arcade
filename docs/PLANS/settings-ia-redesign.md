# Settings IA Redesign — Library · Appearance · Organize · Import · External Emulators

> Source of truth for the Settings IA redesign arc. Mirrors the approved plan
> (operator design session 2026-06-14). Execution deferred; Slice 1 queued in
> `docs/NEXT.md` HIGH band.

## Context

The engine **Settings → "Library"** category conflates two unrelated jobs and
buries a third, so a new user (who lands here to get ROMs in) hits a dense
three-tab admin surface with no clear "what am I doing" framing:

- **Management** — folders, scan options, cleanup (engine/admin territory).
- **Appearance** — how the library *looks/acts* while browsing (tile size,
  sort, group, grid/list). Per `docs/features/theming-substrate/SURFACES.md` +
  DECISIONS **D19**, this is **theme territory** — yet today it's mostly
  *dormant* (only tile-size has UI, in `GridControls`; `sortKey`/`groupBy`/
  `viewMode` are read by `LibraryView` but have no control anywhere).
- **Organization** — the "Views" tab (custom *sidebar trees*) + Collections
  (flat sets). These read like appearance but are navigation structure.

The operator re-cut the **whole Settings IA around user intent** instead of
legacy groupings. This plan captures that design and slices it into
playtestable milestones. Execution is **deferred** — Slice 1 is queued, later
slices ride their parent arcs (theming Phase 5, VL Phase D).

## Current state (verified in code)

| Surface | Where | Notes |
| --- | --- | --- |
| `LibrarySettings` body | `engine/SettingsSections.tsx:1066` | CTA card (opens Import Wizard via `setWizardOpen`) → embeds `LibraryManagerPage`. |
| Library Manager (3 tabs) | `engine/LibraryManagerPage.tsx` | **Library** (folders/scan, sidebar-systems visibility, region/version priority, cleanup/danger-zone) · **Views** (`ViewsManagerTab`/`ViewEditorPane`) · **Game media** (cover/art sync, art packs, platform media, region priority, disk usage). |
| Settings categories + groups | `engine/SettingsPanel.tsx` (`CATEGORIES`, `GROUP_ORDER` = oa-wide/content/system) | `Themes` category is today a near-stub picker (`ThemesSettings`). `Library` lives under `content`. |
| Appearance state | `platform/layout/state.ts` (`LayoutPrefs`) | `viewMode`/`sortKey`/`groupBy`/`libraryTileSize` are **global** (one set, all themes). Consumed in `platform/components/LibraryView.tsx`; only tile-size surfaced (`GridControls.tsx`). |
| Per-theme settings storage | `platform/theme/themeSettings.ts` (`useThemeSettings()`, S5.4) | Storage half exists; no *declarative schema* yet (bare's "Compact" is hand-coded in bare). |
| Theme manifest + validator | `platform/theme/manifest.ts`, `platform/theme/validate.ts`, `THEME_CONTRACT.md` | Where a declarative settings schema must live. |
| External emulators | `engine/CoresPage.tsx` "External emulators" section **+** Systems-hub "Launcher" card | Config split across two places; VL Phase D builds the install pipeline. |
| Card primitives | `engine/systemsHub/` — `SystemsHubRoot`, `HubGrid`, `HubCard`, `DomainCard`, `SystemHubDetail`, `PanelScaffold` | The card language to reuse everywhere. Spatial-nav (`platform/nav/spatial.tsx`) auto-discovers native controls — no per-control wiring. |
| Folder commands | `platform/api/libraryApi.ts` (`addFolder`/`removeFolder`/`updateFolder`/`reorderFolders`/`setFolderRules`) | No "re-point a moved folder" command exists. |

OA **never mutates files on disk today** — cleanup is DB-only ("Files on disk
are NOT touched"). The operator is building a separate external ROM organizer,
so OA stays hands-off the filesystem.

## Target Settings IA

Top-level groups in the engine Settings sidebar (card landings throughout):

- **Themes / Appearance** — pick the active shell **+** configure *its* options,
  **per-theme**, rendered from a **declarative schema** the theme declares
  (so community themes surface options OA never hardcoded). Home for the
  formerly-dormant tile/sort/group/view-mode (for grid themes).
- **Library** — directory custodian: tracked folders, scan options, **re-point a
  moved directory** (same ROMs, new path), DB cleanup/danger-zone. *No* on-disk
  file ops.
- **Organize My Collection** *(new top-level)* — sidebar trees (today's "Views",
  renamed) + Collections + sidebar-systems visibility.
- **Import & Setup** *(new top-level)* — the Import Wizard + guided first-run.
- **External Emulators** *(new top-level)* — the single home for standalone
  emulator binaries/profiles + launch config.
- Unchanged: Systems, System Health, Controllers, Display, Audio, Shaders,
  Gameplay, Performance, Controller-nav, Per-system UI, Experimental, Profile,
  About.

### Ownership boundaries (same data, different intent — avoid duplicate homes)

- **Cores / BIOS:** *Import & Setup* owns the **guided flow** (wizard installs
  cores / resolves BIOS at import); *Systems* owns **per-system deep config**;
  *System Health* owns **operational status**.
- **External emulators:** *External Emulators* tab owns **binaries/profiles**;
  the per-system "which launcher" pick stays in *Systems*. Consolidate the
  CoresPage section away.
- **Sidebar-systems visibility** (hide/show, auto-hide-empty) moves from Library
  → *Organize* (it's organization, not files).
- **Region/version priority** (variant collapse) + **Game media** stay with
  *Library* for now (data behavior + asset management); revisit if they want
  their own Media home.

## Decisions locked (2026-06-14 operator session)

See [../features/settings-ia/DECISIONS.md](../features/settings-ia/DECISIONS.md)
for the authoritative D1–D7 with rationale. Summary:

- **D1** Appearance = theme territory (D19), surfaced in an engine
  **Themes/Appearance** tab — discoverable *and* boundary-correct.
- **D2** Library gets **re-point** (relink a moved folder), **verify** the new
  path holds the same ROMs (fast filename pass) before commit, mutate paths
  **in place** (preserve game ids → covers/metadata/favorites/play-time). **No**
  on-disk move/rename/delete — the external organizer owns that.
- **D3** Per-theme options via a **declarative theme-settings schema** in the
  manifest; OA renders it generically. Belongs with theming-substrate Phase 5.
- **D4** **Organize My Collection** new top-level (Views renamed + Collections +
  sidebar visibility).
- **D5** **Import & Setup** + **External Emulators** new top-level groups;
  External Emulators coordinates with VL **Phase D**.
- **D6** Card primitives + spatial-nav everywhere.
- **D7** Sliced milestones; start at the IA re-skeleton + Library/Organize split.

## Slices (sequencing)

### Slice 1 — IA re-skeleton + Library/Organize split  *(start here; frontend-only)*
- `engine/SettingsPanel.tsx`: add new groups/categories — **Organize**,
  **Import & Setup**, **External Emulators**; reshape **Library**; expand
  **Themes → Themes/Appearance**. Wire `CATEGORIES` + `GROUP_ORDER` + `Match`
  arms.
- Split `LibraryManagerPage.tsx`: **Library** category keeps folders + scan +
  region/version priority + cleanup; **Organize** category gets a new landing
  that mounts `ViewsManagerTab`/`ViewEditorPane` + a Collections manager (new,
  over `platform/library/customCollections.ts`) + the sidebar-systems visibility
  block (moved out of Library).
- Rebuild each landing as a **card hub** modeled on `engine/systemsHub/`
  (`HubGrid` of domain cards → `PanelScaffold` editor). Reuse `HubCard`,
  `DomainCard`, `PanelScaffold` directly where they fit.
- Rename user-facing "Views" (e.g. "Sidebar layouts"); keep the `views` store +
  files as-is internally.
- Import & Setup / External Emulators land as **card shells** (CTA into the
  existing Import Wizard; a card that re-homes the CoresPage external-emulator
  section) — depth fills in Slices 4–5.
- Verify: typecheck + lint (boundary zones) + vitest + `cargo test -p oa-shell`
  unaffected (frontend-only); spatial-nav drives every new card landing;
  operator playtest of the new sidebar IA.

### Slice 2 — Library re-point (relink a moved directory)  *(backend + UI)*
- New Rust command `repoint_folder(folder_id, new_path)`: **verify** the new dir
  contains the matching ROM set (filename pass, optional hash spot-check), then
  **rebase in place** — rewrite `folders.path` + every child ROM row's path by
  prefix-swap (UPDATE, never delete+reinsert) + update the watcher target.
- `platform/api/libraryApi.ts`: typed `repointFolder` wrapper.
- UI: Library folders card → per-folder "Move / relink…" → pick dir → verify
  summary (matched/missing counts) → confirm. Confirm covers/metadata survive
  (game ids unchanged).
- Verify: real re-point preserves all per-game state + the watcher; new Rust
  tests for rebase + verify.

### Slice 3 — Themes/Appearance + declarative theme-settings schema  *(rides theming Phase 5)*
- Extend `ThemeManifest` (`platform/theme/manifest.ts`) with a declarative
  `settings_schema` (typed control descriptors: slider/select/toggle + range/
  options/default). Validate in `platform/theme/validate.ts`; document in
  `THEME_CONTRACT.md`.
- New generic renderer (engine, e.g. `AppearancePanel`) reads the active theme's
  `settings_schema` and renders cards bound to `useThemeSettings()`.
- Migrate the global `viewMode`/`sortKey`/`groupBy`/`libraryTileSize` into
  **Retroverse's** per-theme namespace + its `settings_schema`; rewire
  `LibraryView`/`GridControls` to read per-theme. CoverFlow/bare declare their
  own (speed/spacing; Compact).
- Co-design with the `.oatheme` loader so the manifest format ships once.
- Verify: switching themes shows *that theme's* options; a community-style
  fixture theme with novel options renders; appearance persists per-theme.

### Slice 4 — External Emulators consolidation  *(rides VL Phase D)*
- Promote the CoresPage external-emulator section into the External Emulators
  tab (binaries/profiles + Phase D install pipeline). Per-system launcher pick
  stays in Systems. Retire the duplicate CoresPage section.

### Slice 5 — Import & Setup depth  *(onboarding)*
- Flesh the Import & Setup group: Wizard entry + guided first-run (ties to
  `docs/features/guided-setup/` Phase 2). Empty-library CTA points here.

## Verification (per slice)

- `npm run typecheck`, `npm run lint` (six boundary zones stay green),
  `npm run test` (vitest), and `cargo test -p oa-shell` for any Rust slice.
- Operator playtest: open the engine surface (F12 / Select+Start / corner icon),
  confirm the new top-level IA, card landings are spatial-nav-able (DPad/stick +
  Confirm/Back), each relocated surface still works.
- Slice 2: real re-point preserves per-game state + watcher.
- Slice 3: per-theme appearance + a novel-option fixture theme.
