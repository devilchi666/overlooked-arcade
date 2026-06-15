# Theming Substrate — SURFACES

Surface-by-surface engine-vs-theme territory map. Locked as the
first deliverable of ARC 1 Phase 1 so the refactor that follows
can't quietly shift the boundary.

**Two questions answered for every surface:**

1. **Territory** — does the OA engine always render it (engine
   territory), or does the active `.oatheme` render it (theme
   territory)?
2. **Anchor ownership** — for surfaces that are "shown / hidden"
   rather than "always present," who owns the open/close state?
   Per the 2026-06-06 operator confirmation, **Platform owns
   open/close state; themes pick where dialogs anchor.**

Layers, top to bottom:

- **Engine** — fullscreen takeover summoned via fixed affordance.
  Always engine-rendered. Visually neutral. Survives theme swap.
- **Theme** — the active `.oatheme`'s primary surface (tabs,
  routes, browsing UI). Theme decides layout + visuals.
- **Platform** — the shared substrate themes consume. Owns stores,
  Tauri API wrappers, shared components, signals for cross-cutting
  state like dialog open/close. Themes import from `@oa/platform`
  (Phase 2 alias); engine surfaces consume the same modules.

---

## Engine territory (engine-owned, fullscreen takeover)

These surfaces moved out of Retroverse's SETTINGS tab in Phase 1.
The operator reaches all of them through the same engine summon
(F12 / Select+Start / top-right corner icon). The engine surface
opens as a fullscreen takeover that overlays the active theme; on
close, the operator returns to the same theme view they came from.

The engine surface itself is a single component (`EngineManagerSurface`,
new in Phase 1) with its own internal navigation. The shape it
ships with mirrors today's SETTINGS three-pane layout — operators
already know it.

> **Path note:** the "Today's location" / "After Phase 1" columns below
> (and the Theme-territory table's "Today's location" column) are
> **Phase-1 starting-state snapshots**. After the `platform/` refactor +
> Phase 6, current homes are `platform/`, `engine/`, and
> `themes/retroverse/` (e.g. `engine/SettingsPanel.tsx`,
> `themes/retroverse/*Page.tsx`); the `routes/retroverse/` +
> `layout/retroverse/` dirs are gone.

| Surface | Today's location (pre-Phase 1) | After Phase 1 | Notes |
| --- | --- | --- | --- |
| **Settings — 14 OA-wide / content / system categories** (Display / Audio / Shaders / Gameplay / Performance / Controller nav / Per-system UI / Experimental / Themes / Library / Media / System Health / Profile / About) | `routes/retroverse/SettingsPage.tsx` tab body | `engine/manager/SettingsPanel.tsx` (extracted, identical UX) | Category bodies in `engine/SettingsSections.tsx` are engine-owned and are reused as-is. |
| **Per-system drill-in** (45-system picker → Display / Rewind / Shaders / Default core inline + Bindings / Core options launchers) | `routes/retroverse/SettingsPage.tsx` Per-system group + `PerSystemSettingsBody.tsx` | Same files, mounted inside `EngineManagerSurface` | `platform/components/perSystemSections.tsx` (operator-blessed shared component) is the engine-owned canonical reference per plan §10. |
| **Library Manager** (folders / views / game media tabs) | Embedded in SETTINGS → Library category via `LibrarySettings` → `engine/LibraryManagerPage.tsx` | Same file, mounted inside engine surface's Library category | Same path. The `LibrarySettings` wrapper that decides what to show inside the Library card stays. |
| **Import Wizard** (4-step modal) | Modal — opened via `wizardOpen` signal from SETTINGS → Library "Re-scan with smart detection" card | Modal still — open state migrated to Platform store, summon path stays through engine Library card | The wizard itself is a modal; engine card is the *entry point*. Themes never summon the wizard. |
| **BIOS pre-checks** | Inside System Health page, BIOS tab — `routes/retroverse/SystemHealthPage.tsx` + `BiosSettings` body | Same files inside engine surface's System Health category | No file moves; just changes container. |
| **Core installer** (CoresPage) | Inside System Health page, Cores tab — `engine/CoresPage.tsx` mounted via `CoresCategorySettings` | Same files inside engine surface | Same. |
| **System Health overview** (Overview / BIOS / Cores / Storage / Jobs internal tabs) | `routes/retroverse/SystemHealthPage.tsx` | Same file inside engine surface | Internal tab state (`oa.systemHealth.activeTab` in localStorage) survives. |
| **Background Jobs editor** (live + recent activity) | System Health → Jobs tab — `BackgroundJobsSettings` + `RecentActivityPanel` | Same files inside engine surface | The persistent **progress bar** at the bottom of every theme stays theme-territory (see §Theme territory below). |
| **Help dialogs** (Shortcuts / About / Debug log) | Opened via `helpDialog` signal from `onOpenDebugLog` / `onOpenKeyboardShortcuts` callbacks on RetroverseContext, surfaced inside SETTINGS → About | Same dialogs — open state migrated to Platform store, summon paths preserved | Themes don't summon these directly today; the engine About category does. |

### Why these surfaces are engine territory

- **None of them are "the fun part."** Settings, Library Manager,
  BIOS checks, Core installer — these are the operational
  scaffolding. Themers shouldn't redesign them; doing so degrades
  the boring-necessary parts under bad themes.
- **They survive theme swap.** Per D5 a theme swap is a restart in
  ARC 1, so this is partly moot. But the engine surface is the
  natural place to host the Appearance picker that triggers the
  swap (Phase 5 work) — putting Settings inside theme territory
  would mean the very surface used to swap themes is itself
  theme-rendered.
- **The plan locks them in §1 + DECISIONS D2.** This table is the
  enumeration of that decision, not a new judgment call.

---

## Theme territory (active `.oatheme` renders)

These surfaces ARE the theme. Retroverse-as-it-stands fills them
today. After Phase 6 (Retroverse rebuilt as a theme), they're
formally Retroverse's responsibility; the Wheel pilot theme
delivers a different shape of the same surfaces.

| Surface | Today's location | Theme's responsibility |
| --- | --- | --- |
| **Top-level navigation IA** | RetroverseShell's 6-tab strip | Theme picks IA shape (tabs / wheel / sidebar / radial / etc.) via the 5 engine nav primitives (Phase 3). |
| **Library browsing** | LIBRARY tab — `routes/retroverse/LibraryPage.tsx` + `LeftSidebar` + grid + `GameDetailPanel` | Theme decides layout (grid / wheel / list), tile design, detail-pane shape. Consumes `library` store from Platform. |
| **Home / spotlight surface** | HOME tab — `HomePage.tsx` | Theme decides whether it has a home surface at all + what it shows. |
| **Collections** | COLLECTIONS tab — `CollectionsPage.tsx` | Theme decides 3-pane vs flat vs sidebar. Consumes `customCollections` + `library` stores from Platform. |
| **Play Now / discovery** | PLAY NOW + DISCOVER tabs | Theme decides whether to expose curated discovery, in what shape. |
| **Game launch ceremony** | Tile-click → handleLaunch → boot animation + transition | Theme decides ceremony (cut-to-black vs boot-anim vs disc-spin) within the boot animation framework Per-System UI Stage 1 established. |
| **Now-playing chip** | HintBar's now-playing chip with platform-music equalizer | Theme decides whether/where to show, with what animation. |
| **Quick-settings overlay** | `QuickSettings.tsx` summoned via gamepad chord during gameplay | Theme decides anchor (overlay / sidebar / drawer). Engine OWNS the wired controls (display / audio / shaders / rewind state) — theme picks where they render. |
| **Search affordance** | Top toolbar search input | Theme decides whether search lives in a toolbar / dedicated tab / overlay. |
| **Top-right corner icon slot** | Today empty in Retroverse (lives next to profile chip) | Theme MUST reserve top-right for the `<EngineSummonIcon />` Platform component (per D3). Theme chooses surrounding visuals. |
| **Per-game dialog anchors** (saves / info / context-menu) | Centrally mounted from App.tsx via `savesEntry` / `gameInfoFor` / `contextMenuFor` signals | Open/close state lives in Platform; **theme picks the anchor element** (next to tile / inside detail pane / center-overlay). Phase 1 migrates the state; anchor flexibility lands when Phase 6 ports Retroverse onto the SDK. |

### Why these surfaces are theme territory

- **They're the look and feel.** What a theme creator opens OA to
  redesign. Constraining them to engine layout defeats the
  substrate's purpose.
- **They're optional in shape.** A Wheel theme might not have a
  HOME surface; a kiosk attract-mode theme might collapse
  LIBRARY into a single attract reel. Engine forcing a tab strip
  would break those.
- **Per-game dialogs benefit from theme-chosen anchors.** In a
  grid theme, a context menu anchored at the cursor is natural.
  In a wheel theme, the same menu wants to anchor next to the
  focused wheel slot. Theme picks; engine owns the state.

---

## Platform layer (shared substrate, both engine and theme consume)

Not a surface — the shared code both engine and theme import from.
Phase 2 formalizes the `@oa/platform` alias. Phase 1 doesn't move
files yet; this column documents intent so Phase 2's file moves
don't surprise anyone.

| Concern | Today's location | Phase 2 target |
| --- | --- | --- |
| Library store | `library/store.ts` | `platform/library/store.ts` |
| Custom collections | `library/customCollections.ts` | `platform/library/customCollections.ts` |
| Settings store | `settings/store.ts` | `platform/settings/store.ts` |
| Layout state | `layout/state.ts` | `platform/layout/state.ts` |
| Views store | `views/store.ts` | `platform/views/store.ts` |
| Media | `library/gameInfo.ts` etc. | `platform/media/` |
| Per-system theme registry | `themes/registry.ts` + `themes/systemUIConfigs.ts` | `platform/per-system/` |
| Tauri API wrappers (Phase 4 work) | Direct `invoke()` everywhere | `platform/api/{library,settings,media,…}Api.ts` |
| Shared dialog open/close signals (this phase) | App.tsx createSignals | `platform/dialogs/store.ts` (new in Phase 1) |
| Shared nav primitives (grid / wheel / list / carousel / custom) | `platform/components/LibraryView.tsx` is the grid today | `platform/nav/` (Phase 3) |
| Shared scoped event helper | `lib/listenScoped.ts` | `platform/lib/listenScoped.ts` |

---

## Layer boundary contract (ENFORCED — see `frontend/eslint.config.mjs`)

The whole point of the engine/theme split is that **a new feature or fix
can't silently re-wire the layers back together.** Naming the layers isn't
enough — the boundary has to be machine-checked, or it rots (the
`SystemHeader → useTheme` reverse leak introduced with the 2026-06-09
bootless feature is proof: a small, well-intentioned change quietly made
platform depend on theme). So the boundary is a build-checked lint, not a
convention.

### The layers (foundation → up)

| Layer | Dirs (`frontend/src/`) | May import | Must NOT import |
| --- | --- | --- | --- |
| **Platform** (foundation SDK) | `platform/**`, `nav/**` | external libs, Tauri, `oa-core` types, other platform/nav | `engine/`, `themes/**`, `components/` (grab-bag), `App.tsx` |
| **Engine** (engine-owned UI) | `engine/**` | platform, nav | `themes/**` (theme) |
| **Theme** (the active theme) | `themes/**` (e.g. `themes/retroverse/**`) | platform, nav, **engine-public components** (e.g. `<EngineSummonIcon/>`) | other themes' internals, `App.tsx` |
| **Composition root** | `App.tsx`, `main.tsx` | everything (it wires the layers) | — (nothing imports it) |

Rule of thumb: **dependencies point DOWN** (theme → engine → platform).
Platform is pure foundation; it never knows a theme or the engine exists.
The theme mounting engine-owned components (theme → engine-public) is the
one intended upward edge.

### Enforced today (Slice 1 — `feat/theming-boundary-enforcement`, 2026-06-09)

`npm run lint` runs an ESLint flat config that is a **boundary linter only**
(no style rules — `tsc` + review own those). Slice 1 enforces the two
invariants that hold cleanly today, via `import/no-restricted-paths` zones:

- ✅ **`platform/**` ↛ `routes/**`** (platform must not import theme). The
  only violation was the SystemHeader leak — fixed by passing
  `onBootWithoutGame` down as a prop (LibraryPage → LibraryView →
  SystemHeader) instead of calling `useTheme()` inside a platform component.
- ✅ **`platform/**` ↛ `engine/**`** (platform must not import engine). Zero
  violations — added preventively.
- ✅ **`platform/**` ↛ `components/**`** (platform must not import the
  unclassified grab-bag) — added in Slice 2 (2026-06-09). Closing it
  required: relocating `SystemCoresStrip` + `CatalogCoreCard` into
  `platform/components/` (they're platform UI), and extracting the dialog-
  *state* types (`GameDialogKind`/`GameDialogState`/`CollectionDialogMode`/
  `SystemDialogSection`) out of the grab-bag dialog files into
  `platform/dialogs.ts` (platform owns dialog state; the components import
  them back). Platform is now pure of theme, engine, AND the grab-bag.
- ✅ **`engine/**` ↛ `routes/**`** (engine must not import theme) — added in
  Slice 2 batch 3 (2026-06-09). Closing it required the **store-context
  split** (`usePlatform()`, DECISIONS D11) so engine components read stores
  without the theme context, then relocating the engine surface's Settings
  *content* — `PerSystemSettingsBody`, `PerSystemInfoSection`,
  `SystemHealthPage` — out of `routes/retroverse/` into `engine/` (migrating
  `SystemHealthPage`'s `useTheme().library` → `usePlatform()`). The engine
  surface (`SettingsPanel` + content) no longer touches theme code.

### Enforced after the grab-bag drain (`feat/theming-grabbag-drain`, 2026-06-09)

- ✅ **`engine/**` ↛ `components/**`** + ✅ **`routes/**` ↛ `components/**`** —
  the `src/components/` grab-bag is **fully drained and the directory
  removed**. The 38 top-level files + 2 subtrees split by layer: in-game /
  per-game / shared UI → `platform/components/` (incl. the shared leaves
  CoreOptionsPanel / AnalogBindingsSection / reference cards — see DECISIONS
  D12 on why dual-consumed leaves go to the lower layer), engine-manager
  surfaces → `engine/`. `SettingsSections` shed its last `useTheme()` (stores
  → `usePlatform()`; the 5 app-action handlers → `@oa/platform/dialogs`
  setters + the new `platform/libraryAdmin.ts` registry, DECISIONS D13). Both
  `components/**` zones are now ratchets preventing a new unclassified bucket.
  **Six zones enforced + green:** platform↛{routes,engine,components},
  engine↛{routes,components}, routes↛components.

### Known-violating edges deferred to later slices

All enforced — nothing deferred.

- ✅ **Raw `invoke()` outside `platform/api/`** — ENFORCED 2026-06-10 (Phase 4
  Slice 6, the closer). All ~351 raw calls (the census grew to 14 typed
  `platform/api/<domain>Api.ts` modules — settings/library/collections/views/
  media/cores/input/emulator/rewindTas/cheats/milestones/capture/jobs/system/
  shell) are behind typed wrappers; `no-restricted-imports` bans the
  `@tauri-apps/api/core` `invoke` import everywhere except `src/platform/api/**`
  (`convertFileSrc` from the same module stays allowed). Probe-verified the rule
  fires. The command-name string for every backend command now lives in exactly
  one file.

- ✅ **Raw Tauri events (`listen`/`emit`/`once`) outside `platform/api/`** —
  ENFORCED 2026-06-10 (Phase 4.5, sibling to the invoke ban). All ~30 sites route
  through `platform/api/eventsApi` (the `OA_EVENTS` registry +
  `listenScoped`/`listenTo`/`emitEvent`); every `oa://…` channel string lives in
  one place; `no-restricted-imports` bans the raw event value-imports everywhere
  except `src/platform/api/**` (type-only imports stay allowed). Probe-verified.
  See DECISIONS D17.

### The endpoint — REACHED 2026-06-10

Every file in exactly one layer; the lint makes a cross-layer import a build
failure; **both** backend-contract surfaces — `invoke()` command names AND
Tauri event names — are corralled into `platform/api/`. Platform and theme
**physically cannot** be re-coupled by a new feature without ESLint stopping
the commit — at the file level (six boundary zones) and the API level (the
invoke ban + the event ban). **The decoupling track is done.**

---

## Dialog open/close ownership map (Phase 1 migration target)

Per operator confirmation: **Platform owns open/close state; themes
pick where dialogs anchor.** The 5 dialog signals living in App.tsx
today migrate to a new `platform/dialogs/store.ts` (file path
canonical in Phase 2, signals can live anywhere stable in Phase 1).

| Signal | Today | Phase 1 target | Anchor decision |
| --- | --- | --- | --- |
| `savesEntry` (saves picker per game) | `App.tsx:215` | Platform `dialogs.savesEntry` | Theme picks anchor — Retroverse anchors center-overlay today; future Wheel theme could anchor next to focused wheel slot. |
| `contextMenuFor` (tile context menu per game) | `App.tsx:217` | Platform `dialogs.contextMenuFor` | Theme picks anchor — Retroverse positions at cursor (`x` / `y` from event); future themes can position relative to focused tile geometry. |
| `gameInfoFor` (Game Info modal per game) | `App.tsx:220` | Platform `dialogs.gameInfoFor` | Theme picks anchor — Retroverse anchors as center modal; Wheel could anchor as a sidecar panel. |
| `helpDialog` (Shortcuts / About / Debug log) | `App.tsx:254` | Platform `dialogs.helpDialog` | Engine-anchored — the engine surface's About category mounts the dialog. Themes don't summon these directly. |
| `wizardOpen` (Import Wizard) | `App.tsx:480` | Platform `dialogs.wizardOpen` | Engine-anchored — engine surface's Library category surfaces the entry point. |

The two "engine-anchored" dialogs (Help, Import Wizard) still keep
their open-state in Platform — that way the engine surface code
doesn't reach into theme-side stores. The two surfaces (engine /
theme) communicate only through Platform.

### What about the other dialog-like signals in App.tsx?

App.tsx today has another ~12 createSignals for dialog-shaped state
(coreMenuFor, regionPickerFor, propertiesFor, collectionDialog,
gameDialog, quickSettingsOpen, screenshotGalleryFor, systemContextFor,
containerContextFor, settingsDialog, systemDialog, etc.). Phase 1
**only migrates the 5 named in the plan** to lock scope. The rest
follow in Phase 2's platform extraction. Tracking the residue here
so it isn't forgotten:

- `coreMenuFor`, `regionPickerFor`, `propertiesFor` — per-game,
  theme-anchored. Phase 2 candidates.
- `collectionDialog`, `gameDialog` — modal forms, engine-anchored
  candidates. Phase 2.
- `quickSettingsOpen` — engine controls + theme anchor (per
  "Quick-settings overlay" row above). Phase 2.
- `screenshotGalleryFor` — per-game viewer. Theme-anchored. Phase 2.
- `systemContextFor`, `containerContextFor` — per-sidebar-row
  context menus. Theme-anchored. Phase 2.
- `settingsDialog`, `systemDialog` — historical pre-Retroverse
  modals. Audit for dead-letter in Phase 2.

---

## Engine summon affordances (Phase 1)

All three reach the same engine surface. None are theme-configurable
in ARC 1 (theme manifest can rename / hide later if we want — not
yet).

| Affordance | Default | Implementation hook | Theme requirements |
| --- | --- | --- | --- |
| Keyboard hotkey | `F12` | App.tsx keydown handler toggles `platform.engineSurface.open()` signal | None — engine listener at App.tsx level catches before theme. |
| Controller chord | `Select` + `Start` | Existing controller-nav pipeline (`onNavEvent`) recognizes the chord, fires the same signal | None — engine listener at App.tsx level. |
| Top-right corner icon | Always visible | `<EngineSummonIcon />` Platform component themes mount in their top-right corner | Theme MUST reserve a slot for it; manifest's `reserves_corner = "top-right"` is the contract. |

**Close behavior:** `Escape` key, `B` controller button, or click
outside the engine surface all close it. Engine surface remembers
no in-surface state across open/close (intentional — Settings tab
strip + per-system picker reset to defaults; localStorage already
preserves "active tab" in System Health internal nav).

---

## What stays unchanged

- All per-core, per-system, and per-game functionality. Phase 1 is
  a UI plumbing refactor — zero behavior change for emulators,
  cores, save states, controllers, audio.
- All 660+ `oa-shell` Rust tests. Phase 1 is frontend-only.
- All controller-nav contracts. The L1/R1 tab cycling in
  RetroverseShell stays; cycling only walks the 5 remaining tabs.
- Background Jobs **persistent progress bar** at the bottom of
  every theme. That bar is a theme-territory surface today
  (mounted from RetroverseShell or its equivalent) and stays
  theme-territory after Phase 1 — themes decide where to anchor
  it. Only the *editor* (live job list, recent activity, cancel
  controls) lives in the engine surface's System Health → Jobs
  tab.
- `LibraryPrefs.discTrackExperimentalEnabled` and other shipped
  prefs. Phase 1 doesn't touch any persisted state schema.

---

## Open boundary questions (resolve in Phase 2, not Phase 1)

These were raised during the audit but don't block Phase 1. Logged
here so Phase 2 isn't surprised:

1. **`HOTSPOT_SYSTEMS = new Set(["nds"])` triplicate** —
   TouchHotspotOverlay, StylusOverlay, QuickSettings. Plan §6
   Phase 2 collapses into a single `touchInputSupported: boolean`
   on `SystemUIConfig`. Surface: theme territory (overlays only
   render when a game is running, fully theme-side).
2. **`customComponent: "vectrex"` orphan field** — exists on
   `SystemUIConfig`, nothing reads it. Plan §6 Phase 2 either
   wires a consumer or deletes in favor of Phase 3's `custom` nav
   primitive.
3. **`QuickSettings.tsx` is currently theme-coupled** — directly
   imports stores + invokes Tauri commands. Phase 4 (Tauri bridge
   hardening) drains the `invoke()` calls; Phase 6 splits the
   "controls" half (engine-owned, fixed UX) from the "anchor"
   half (theme-positioned).

---

## Phase 1 acceptance gate (from plan §6)

This SURFACES.md doc + the refactor it scopes ship together. The
acceptance gate the plan locks:

1. Operator hits `F12` from anywhere in Retroverse → engine
   Settings surface opens.
2. Operator closes it → returns to the same library view + scroll
   position they came from.
3. Per-system drill-in works identically (sidebar picker → 4
   inline cards + Bindings/Core options launchers).
4. All existing tests green (`cargo test -p oa-shell`).
5. Frontend `npm run typecheck` silent.
6. Visual regression: Retroverse with 5 tabs (HOME / LIBRARY /
   COLLECTIONS / PLAY NOW / DISCOVER) is functionally identical
   to today's 6-tab Retroverse, minus the SETTINGS tab itself.
