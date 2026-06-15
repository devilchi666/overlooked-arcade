# Settings IA Redesign — Session Log

Newest first. Three lines per entry: **Shipped / Almost / Next**.

---

## 2026-06-14 — Slice 4 implemented (code-complete, pending playtest)

- **Shipped (branch `feat/settings-ia-slice-4`, not yet merged):** External
  Emulators consolidation. Extracted the standalone-emulator config (profiles +
  binary-path picker + per-system default-launcher) out of `CoresPage` into a
  self-contained `engine/ExternalEmulatorsSection`, rendered in
  `ExternalEmulatorsLanding` — the tab is now real, not a placeholder shell.
  Retired the duplicate from CoresPage (removed its emulator state/handlers/type
  + the now-unused `emulatorApi` import; dropped the catalog-coupled "no core
  installed — see Browse cores below" hint, Cores-page-specific). Per-system
  launcher choice still also lives on the system's card in Systems (per the
  ownership lines). Bring-your-own-binary today; the install pipeline rides VL
  Phase D (a "Coming" hook notes it). Frontend-only; typecheck/lint/vitest(107)
  green.
- **Also shipped — the research-need doc** the operator asked for:
  `docs/RESEARCH/external-emulators.md` scopes the major research into
  command-line launching of a broad emulator roster — both systems we can't run
  via cores (Cemu/RPCS3/Ryujinx/Lime3DS/Vita3K/Xenia/xemu/…) AND ones we DO
  support but where users may prefer a standalone (PCSX2/DuckStation/PPSSPP/
  Dolphin/…). Includes a per-emulator research template + draft (needs-verify)
  CLI roster + the grow-over-time philosophy (simple CLI now → options → scripts/
  plugins → Phase-D installer).
- **Almost:** the research itself isn't done — the doc is the scope + template;
  CLI columns are draft and need per-emulator verification.
- **Next:** operator playtest (External Emulators tab shows the Dolphin profile +
  binary-path + per-system launcher; Cores page no longer duplicates it) → merge.
  Then Slice 5 (Import & Setup depth) — or pull the external-emulator research
  forward as its own pass.

## 2026-06-14 — Slice 3 implemented (code-complete, pending playtest)

- **Shipped (branch `feat/settings-ia-slice-3`, not yet merged):** declarative
  per-theme appearance. `ThemeManifest` gains `settings_schema` (toggle/slider/
  select control descriptors); the S4 validator checks them (unique keys, valid
  type/label, slider min<max + default-in-range, select default∈options — a
  malformed control is a disqualifying error; +9 vitest cases). New generic
  `engine/AppearancePanel` reads the active theme's schema and renders each
  control via `SettingRow` bound to `useThemeSettings()` (per-control reset),
  mounted in Settings → Themes / Appearance below the picker. `bare`'s "Compact"
  toggle moved from hand-coded JSX to a declared control (engine renders it; bare
  just consumes the value). **Retroverse** migrated off the global layout store:
  it declares `tileSize`/`sortKey`/`groupBy`/`viewMode` in its schema and reads
  them per-theme; the shared platform `LibraryView` swapped its `layout` prop for
  a focused `LibraryAppearance` prop (Retroverse builds it from `useThemeSettings`
  and passes it in — the platform grid never reaches into theme settings, keeping
  the layer boundary clean). A one-time seed in `RetroverseEntry` copies the
  current global values into Retroverse's namespace so nothing jumps; this also
  finally gives the previously-dormant sort/group/view-mode real UI. The
  `settings_schema` manifest format ships now; theming Phase 5's `.oatheme` loader
  reuses it (no rework). typecheck/lint/vitest(107) green; THEME_CONTRACT.md §8
  added.
- **Almost:** the global `layout` sortKey/groupBy/viewMode/libraryTileSize fields
  are now vestigial for Retroverse (kept as the migration source; safe to retire
  in a later cleanup). CoverFlow declares no schema yet (shows "no options").
- **Next:** operator playtest (Themes / Appearance shows Retroverse's tile/sort/
  group/layout + bare's Compact; changing them works + persists per-theme; the
  in-grid tile slider stays in sync; switching themes shows that theme's options)
  → merge. Then Slice 4 (External Emulators → VL Phase D).

## 2026-06-14 — Slice 2 implemented (code-complete, pending playtest)

- **Shipped (branch `feat/settings-ia-slice-2`, not yet merged):** Library
  **re-point** (relink a moved folder). Rust: `RepointPreview`/`RepointResult` +
  `preview_repoint_folder` (read-only relative-path existence verify) +
  `repoint_folder` (one-tx in-place rebase of `folders.path` + every child
  game's `file_path` by prefix; folder id + game ids stay stable; refuses
  same-path + collision; sibling-folder boundary respected) in `library_db.rs`,
  plus two Tauri commands in `main.rs`. Because covers/metadata/favorites/
  play-time are keyed by game id (not path), the in-place UPDATE preserves them
  all — proven by a test that seeds favorite + play-time via the real setters
  and asserts they survive. Frontend: `previewRepointFolder`/`repointFolder`
  wrappers + a per-folder **"Relink…"** button (pick dir → preview confirm with
  matched/missing → commit → `refreshLibraryFolders` re-registers the watcher
  via App.tsx's folder effect + `refreshGroups`). cargo `oa-shell` 839 (+2) +
  typecheck/lint/vitest(98) green.
- **Almost:** verify is filename-existence only (no hash spot-check — deferred;
  enough to catch a wrong-folder pick).
- **Next:** operator playtest (move a real folder → Relink → confirm zero
  cover/favorite/play-time loss + watcher tracks new path) → merge. Then Slice 3
  (declarative Appearance schema, rides theming Phase 5).

## 2026-06-14 — Slice 1 MERGED + Slice 2 started

- **Shipped:** Slice 1 merged to main (`e71eef0`) after operator playtest passed.
  Now starting **Slice 2 — Library re-point** (relink a moved folder).
- **Next:** Rust `repoint_folder` command (verify same-ROMs → in-place path
  rebase → watcher update) + `repointFolder` api wrapper + per-folder "Move /
  relink…" UI in the Library folders card.

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
