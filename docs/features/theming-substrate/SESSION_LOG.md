# Theming Substrate — Session Log

Each session that touches this feature appends a 3-line entry:
**Shipped / Almost / Next.** Older entries roll to
`SESSION_LOG_ARCHIVE.md` when this file passes ~150 lines
(rolled 2026-06-09 — entries before the grab-bag drain live there).

---

## 2026-06-09 — Grab-bag drain: components/ emptied, engine↛components enforced

- **Shipped:** drained the entire `src/components/` grab-bag (38 top-level +
  2 subtrees) in two commits on `feat/theming-grabbag-drain`, then removed the
  directory. **Batch 1** (`0286dae`): in-game / per-game / shared UI →
  `platform/components/` — QuickSettings, GameDialogs (+ the per-game dialog
  family), GamePropertiesDialog, GameInfoModal, SaveSlotsModal, RegionPicker,
  CorePickerMenu, ScreenshotGalleryDialog, ToastStack, PerformanceHud, the
  three context menus, NewCollectionDialog, the overlays (Stylus /
  TouchHotspot / SystemBackground / SystemBootAnimation), the reference cards
  (Keypad / LightGun / GenesisPad), AnalogBindingsSection, CoreOptionsPanel,
  and the whole `background-jobs/` subtree. **Batch 2** (`aba360b`): the
  engine-manager surfaces → `engine/` — SettingsSections, CoresPage,
  LibraryManagerPage, ImportWizard, ImportArtPackDialog, ScummvmDetectDialog,
  DebugLogDialog, HelpDialogs, PlatformMediaDialog, GameMediaManagePanel,
  UnidentifiedGamesDialog, ViewsManagerTab, ViewEditorPane, SystemDialogs,
  SystemBindingsEditor, `import-wizard/*`. Added + verified the
  `engine/** ↛ components/**` lint zone (the batch's goal) plus a
  `routes/** ↛ components/**` ratchet; the eslint header now documents **six
  enforced zones**, only raw `invoke()` (Phase 4) left. typecheck + lint green
  after each commit.
- **Two judgment calls** (both deviate from the plan's literal lists, both
  forced by the import graph — see DECISIONS D12/D13):
  - *Shared leaves go to the LOWER layer.* AnalogBindingsSection +
    CoreOptionsPanel + the reference cards are imported by BOTH engine
    surfaces (SystemBindingsEditor / SystemDialogs) AND per-game platform UI
    (GameDialogs); putting them in engine would force a platform→engine edge,
    so they land in `platform/components/`. GameDialogs's only other engine
    coupling — a re-export block from SystemDialogs — was redirected to its
    true source `@oa/platform/components/perSystemSections`, severing the edge.
    background-jobs/* likewise → platform (the persistent bar is
    theme-territory per SURFACES; RecentActivityPanel / ResumePromptDialog are
    consumed only by the bar / App).
  - *SettingsSections' 5 app-action handlers.* 3 → direct
    `@oa/platform/dialogs` setters (`setWizardOpen` / `setHelpDialog`); the 2
    library-admin actions → a new `platform/libraryAdmin.ts` registry (App
    registers its App-scoped handlers on mount; engine calls them without
    importing App/theme). Chosen over 3-layer prop drilling (D10/D13).
- **Almost:** pure relocations + store-source swaps, no behavior change
  intended — awaiting operator playtest (Settings all categories + per-system
  drill-in + System Health; per-game dialogs; Library Manager; Import Wizard)
  before merge to main.
- **Next:** Phase 4 — typed `platform/api/` Tauri bridge (corral 157 raw
  `invoke()` calls + a `no raw invoke() outside platform/api/` rule). The
  platform/engine/theme separation is now fully lint-enforced; raw `invoke()`
  is the last coupling.

---

## 2026-06-09 — Phase 4 Slice 1: `settingsApi` (typed Tauri bridge)

- **Shipped:** Created `frontend/src/platform/api/settingsApi.ts` — the first
  `platform/api/<domain>Api.ts` module (28 typed wrappers, one named export per
  command, command string lives only here) — and migrated the
  video-display / audio / system-settings / per-game-overrides / shell-mode +
  kiosk + presentation-mode cluster across **13 files** on
  `feat/theming-platform-api-settings` (off main): App.tsx (launch AV-override
  push + revert-to-defaults + direct-launch fullscreen + shell-mode hydrate),
  `settings/store.ts`, `lib/audio.ts`, `layout/state.ts`, `shader_presets.ts`,
  QuickSettings, GameDialogs, GamePropertiesDialog, perSystemSections,
  AnalogBindingsSection, SettingsSections, SystemDialogs, PerSystemSettingsBody.
  Import style: named imports where no collision; `import * as settingsApi`
  namespace alias in the four files whose local signal setters / exports shadow
  the wrapper names (App.tsx, settings/store, audio, layout/state). `VideoState`
  (the `get_video_state` return) lifted into the api module as its canonical
  home; QuickSettings imports it now. typecheck + lint green; every migrated
  command string greps to **only** `settingsApi.ts` (the sole other matches are
  one index.css doc comment — updated — and 6 diagnostic label strings in
  App.tsx's revert array, which are `[label, promise]` tuples, not `invoke()`
  bindings). The Slice 6 lint rule is NOT on yet (per plan).
- **Three judgment calls:**
  - *Shape-divergent getters stay generic.* `get_game_overrides` /
    `get_system_settings` are typed differently at each call site (each file
    declares only the fields it reads — `SysSettings` / `GameOver` /
    `PerSystemOverrides` / `{analogRouting?}` / the GameDialogs supersets).
    Rather than force one canonical type (which would over/under-constrain
    callers), the wrappers are `getGameOverrides<T = GameOverrides>(id)` /
    `getSystemSettings<T = SystemSettings>(systemId)` — every call site keeps
    its exact local view via the type arg, zero type churn, no `any`. Canonical
    `GameOverrides` / `SystemSettings` / `OverscanCropPrefs` / `VideoState`
    defined + exported in the api module as the backend-contract home.
  - *Views/layout stay out of Slice 1.* `layout/state.ts` calls both the
    presentation/kiosk commands (settingsApi) and `get/set_layout` (viewsApi,
    Slice 2) — but they're **separate call sites**, not entangled, so only the
    presentation/kiosk trio migrated; `get/set_layout` left raw for Slice 2.
  - *Surfaced + fixed a latent bug.* AnalogBindingsSection's `get_game_overrides`
    passed `{ gameId }`, but the backend command's arg is `id` — so the call
    silently errored (caught + logged "analog prefs fetch failed") and per-game
    analog routing fell back to empty `{ ports: [] }`. The typed wrapper sends
    the correct `{ id }`, fixing it. Flagged to operator (the one place this
    slice changes runtime behavior; everything else is a pure pass-through).
- **Almost:** behavior-preserving everywhere except the AnalogBindingsSection
  bug fix above — awaiting operator smoke-test (QuickSettings during a game:
  window mode / scaling / shader / audio device / volume; per-game Display +
  Shaders + Audio dialogs; Settings → Display / Audio / Shaders; per-system
  drill-in) before merge to main.
- **Next:** Slice 2 — `libraryApi` + `collectionsApi` + `viewsApi` (the
  store-heavy core: library/store, customCollections, ingest, views/store +
  the `get/set_layout` calls left in layout/state, App.tsx library paths;
  ~55 sites). Same convention.
