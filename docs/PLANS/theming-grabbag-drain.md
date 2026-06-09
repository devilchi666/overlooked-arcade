# Theming boundary — engine-manager grab-bag drain (deferred batch)

**Status:** queued. The boundary-enforcement track's high-value work is
**done + merged** (4 enforced lint zones; `usePlatform()` keystone — see
below). This doc is the self-contained plan for the remaining `components/`
grab-bag drain, deferred out of the 2026-06-09 session because it's a single
~15-file interconnected refactor that deserves a fresh, focused session
rather than a marathon-tail push.

Owning feature: [features/theming-substrate/](../features/theming-substrate/).
Layer contract: [features/theming-substrate/SURFACES.md](../features/theming-substrate/SURFACES.md)
§"Layer boundary contract". Decisions: that folder's `DECISIONS.md` D8–D11.

## What's already done (merged to main, 2026-06-09)

The boundary is a **build-checked ESLint linter** (`frontend/eslint.config.mjs`,
run by `npm run lint` + CI) — boundary rules only, no style. Four zones are
enforced + green:

- ✅ `platform/** ↛ routes/**` (platform never imports theme)
- ✅ `platform/** ↛ engine/**`
- ✅ `platform/** ↛ components/**` (the grab-bag)
- ✅ `engine/** ↛ routes/**` (engine never imports theme)

Keystone shipped (DECISIONS D11): **`platform/platformContext.tsx`** —
`PlatformProvider` + `usePlatform()` exposing the stores
(library / customCollections / layout / views / settings) + shared state
(searchQuery / focusedEntry / currentView), theme-agnostic. App.tsx provides
BOTH `PlatformProvider` and `ThemeProvider` from the **same instances**, so
theme code's `useTheme().settings` is untouched while engine/platform code
reads stores via `usePlatform()`. **This is the tool that unblocks the drain:
when a grab-bag file moves into engine/, switch its `useTheme()` store reads
to `usePlatform()`.**

## Goal of this batch

Close the **`engine/** ↛ components/**`** edge (then add + enforce that lint
zone), i.e. relocate the engine-manager UI out of the unclassified
`frontend/src/components/` grab-bag into `frontend/src/engine/`. This finishes
the platform/engine/theme separation — after it, the only remaining coupling
is raw `invoke()` (Phase 4, separate).

## The cluster (must move as ONE coherent batch — it's interconnected)

The current `engine/ → components/` edges and the web they pull in:

```
engine/SettingsPanel        → components/SettingsSections
engine/SystemHealthPage     → components/SettingsSections
                            → components/import-wizard/SystemReadinessChecklist
engine/PerSystemSettingsBody → components/SystemDialogs

components/SettingsSections  → CoresPage, LibraryManagerPage, PlatformMediaDialog, useTheme
components/LibraryManagerPage → ViewsManagerTab, ImportArtPackDialog,
                                PlatformMediaDialog, GameMediaManagePanel,
                                UnidentifiedGamesDialog
components/SystemDialogs     → SystemBindingsEditor, CoreOptionsPanel
components/SystemBindingsEditor → AnalogBindingsSection, GenesisPadReference,
                                  LightGunHelp, KeypadReference, systems/keymap
components/CoreOptionsPanel, CoresPage, PlatformMediaDialog, DebugLogDialog → (leaves)
```

Also note these are imported by code that must NOT end up importing engine
illegally:
- `components/GameDialogs` imports `SystemDialogs` + `CoreOptionsPanel`.
  GameDialogs is itself a grab-bag file — it moves in this batch too (it's
  theme/per-game UI; decide engine vs platform vs theme — likely platform,
  since per-game dialogs are a shared surface a theme mounts).
- `App.tsx` imports `SystemDialogs` (fine — App is the composition root).
- `platform/components/perSystemSections.tsx` is imported BY `SystemDialogs`
  (components→platform, allowed) — not the reverse. Verify no platform file
  imports a grab-bag file after the move (lint will catch it).

**Moving any single leaf alone is wrong** — it creates backwards
`components → engine` cross-imports while the parents stay behind. Move the
whole engine-manager set together.

## Classification (where each file lands)

Engine-manager surfaces (the F12 takeover content) → **`engine/`**:
`SettingsSections`, `CoresPage`, `LibraryManagerPage`, `ImportWizard`,
`ImportArtPackDialog`, `ScummvmDetectDialog`, `DebugLogDialog`, `HelpDialogs`,
`PlatformMediaDialog`, `GameMediaManagePanel`, `UnidentifiedGamesDialog`,
`ViewsManagerTab`, `ViewEditorPane`, `SystemDialogs`, `SystemBindingsEditor`,
`CoreOptionsPanel`, `AnalogBindingsSection`, `import-wizard/*`,
`background-jobs/*` (the manager surfaces).

Shared in-game / per-game UI a THEME mounts → **`platform/components/`**:
`QuickSettings`, `GameDialogs`, `GamePropertiesDialog`, `SaveSlotsModal`,
`RegionPicker`, `CorePickerMenu`, `ScreenshotGalleryDialog`, `ToastStack`,
`PerformanceHud`, `TileContextMenu`, `ContainerContextMenu`,
`SystemContextMenu`, `NewCollectionDialog`, the overlays (`StylusOverlay`,
`TouchHotspotOverlay`, `SystemBackground`, `SystemBootAnimation`), reference
cards (`KeypadReference`, `LightGunHelp`, `GenesisPadReference`).

(Use judgment per file — the litmus: "does a THEME need to render this?"
→ platform; "is it part of the engine settings/management takeover?"
→ engine. When unsure, check who renders it: App/EngineManagerSurface
→ engine; RetroverseShell/tiles → platform.)

## Recommended order (smaller, verify between each)

Do it as 2–3 commits, `npm run typecheck` + `npm run lint` green between each:

1. **Platform-bound shared UI first** (the in-game/per-game cluster:
   `GameDialogs`, `QuickSettings`, `SaveSlotsModal`, `RegionPicker`,
   `CorePickerMenu`, overlays, reference cards, context menus). These move to
   `platform/components/`; migrate any `useTheme()` store reads to
   `usePlatform()`. They have no engine deps. This shrinks what the engine
   cluster drags.
2. **Engine-manager cluster** (`SettingsSections` + `CoresPage` +
   `LibraryManagerPage` + `SystemDialogs` + `SystemBindingsEditor` +
   `CoreOptionsPanel` + the import-wizard/media/views manager files) → `engine/`.
   Migrate every `useTheme()` store read → `usePlatform()`.
3. **Fix `SettingsPanel` / `SystemHealthPage` / `PerSystemSettingsBody`** import
   paths to the new `engine/` locations (drop the `../components/` paths).

## The one judgment call — SettingsSections' 5 app-action handlers

`components/SettingsSections` reads 5 handlers off `useTheme()`:
`onAddLibraryFolder`, `onRescanLibraryFolders`, `onOpenImportWizard`,
`onOpenDebugLog`, `onOpenKeyboardShortcuts`. Once it's in `engine/` it can't
use `useTheme()`. Resolution:

- **3 are thin platform/dialogs setters** — `onOpenImportWizard` =
  `setWizardOpen(true)`, `onOpenDebugLog` = `setHelpDialog("debug-log")`,
  `onOpenKeyboardShortcuts` = `setHelpDialog("shortcuts")`. In `engine/`,
  call `@oa/platform/dialogs` setters directly. No handler needed.
- **2 are library-admin actions** — `onAddLibraryFolder` (opens an OS folder
  picker then `library.addLibraryFolderPath`) + `onRescanLibraryFolders`.
  Cleanest: add them to a small platform/engine service the engine surface
  exposes, OR thread them as props from `App → EngineManagerSurface →
  SettingsPanel → SettingsSections`. Recommend: a tiny
  `platform/api/libraryAdmin.ts` (or extend `usePlatform()` with these two
  service fns) so engine code calls them without props. Check `App.tsx`
  `handleAddLibraryFolder` / `handleRescanLibraryFolders` — they're mostly
  `library` store calls + a Tauri picker; the picker is the only App-ish bit.

## Finish

- Add the `engine/** ↛ components/**` zone to `eslint.config.mjs`; confirm
  `npm run lint` green (everything engine imported from `components/` is now
  in `engine/` or `platform/`).
- Consider a `routes/** ↛ components/**` zone too (theme must not import the
  grab-bag) once the grab-bag is empty/near-empty.
- Update SURFACES.md (move `engine↛components` to enforced; shrink the
  grab-bag tally toward zero) + a SESSION_LOG entry + DECISIONS if a handler
  decision is made.

## After this batch

The grab-bag is drained; platform/engine/theme are fully separated +
lint-enforced. The last coupling is **Phase 4 — the typed `platform/api/`
Tauri bridge** (corral the 157 raw `invoke()` calls behind typed wrappers +
a `no raw invoke() outside platform/api/` lint rule). That's its own arc
(see the main theming plan §Phase 4).

## Verification (every commit)

```
cd frontend && npm run typecheck && npm run lint
```
typecheck catches broken import paths; lint enforces the boundary. Operator
playtest after the batch: open Settings (all categories + per-system
drill-in + System Health), open per-game dialogs (Input/Display/etc.),
Library Manager, Import Wizard — confirm they still render + work (pure
relocations + store-source swaps; no behavior change intended).
