# Theming Substrate — Plan

**Status:** Planning. No code. Locked design after the 2026-06-06
operator + Claude planning session.

**Author:** Operator + Claude (LLM pair).

**Owner-of-decisions:** the operator. This document records what was decided.

---

## 1. TL;DR

Build a BigBox-style theming ecosystem for OA where creators can ship
radically different looks — wheel layouts, tile grids, list views,
cabinet attract modes — that all run on the same engine and consume
the same library data. One unified premium frontend; **no
LaunchBox/BigBox-style split** into separate Studio + Launcher
binaries.

The unlock is splitting OA's UI into two surfaces inside one window:

- **Engine territory** (always engine-rendered, visually neutral):
  Settings, Library Manager, Import Wizard, BIOS pre-checks, Core
  installer, System Health, Background Jobs. Summoned from any theme
  via a fixed affordance.
- **Theme territory** (where creators design): library browsing, game
  launch ceremony, now-playing, quick-settings overlay, discovery
  surfaces.

The Kiosk Shell plan's 4-layer theme substrate spec (in
[docs/features/kiosk-shell/KIOSK_PLAN.md](../features/kiosk-shell/KIOSK_PLAN.md)
§2.2-2.5) is the right model — it's just misnamed. It becomes the
theming substrate for ALL of OA, not just kiosk mode. Kiosk-as-such
(attract mode, multi-monitor, 5-bus mixer) becomes capabilities the
substrate exposes; some themes opt into them, some don't.

**Retroverse becomes the first theme on the substrate.** Dogfood test:
if Retroverse can be a `.oatheme`, anything can.

Four arcs total (D35 renumber), ~34-40 weeks. Arc 1 (Minimum Viable
Substrate, ~22-26 weeks) ships layout + assets + palette overrides
without scripting or shaders — shipped bar the `.oatheme` loader. Arc 2
is the Per-System Layout Substrate (declarative, no scripting). Arcs 3-4
add Rhai behaviors + WGSL shaders + in-engine Theme Studio when the
substrate proves out.

---

## 2. Goals + non-goals

### Goals

1. **Code health now.** Eliminate the 3 copies of
   `HOTSPOT_SYSTEMS = new Set(["nds"])` across overlay components.
   Wire the orphan `customComponent: "vectrex"` escape hatch.
   Drain the ~150 direct `invoke()` leakage sites in components.
   Add a SystemId parity test catching TS-union ↔ Rust-enum drift.
2. **Theme creators get a tight scope.** Themes only design the
   "fun" surfaces; Settings + Library Manager + Import Wizard stay
   engine-owned. Same scope reduction BigBox themers benefit from
   without splitting the binary.
3. **Themes can be radically different.** Wheel layouts, list views,
   cabinet attract modes all become tractable — not just color
   swaps.
4. **Substrate is the same for desktop, kiosk, and everything in
   between.** No separate Kiosk theme system to maintain in
   parallel.

### Non-goals (this arc / ARC 1)

- **Public theme SDK / contribution guide / gallery** — deferred per
  DECISIONS G WAIT lock. Operator-loaded themes only in ARC 1.
- **Two-binary split** — explicitly rejected per 2026-06-06
  conversation.
- **Rhai scripting + WGSL shader hooks** — deferred to ARC 3.
- **In-engine Theme Studio editor** — deferred to ARC 4.
- **Replacing the per-system data layer** — themes inherit
  per-system colors, metadata, and assets from the existing
  `themes/registry.ts` + `config/systems/<id>/system.yaml`
  infrastructure unchanged.

---

## 3. Architectural decisions

Locked in conversation 2026-06-06; live in
[features/theming-substrate/DECISIONS.md](../features/theming-substrate/DECISIONS.md).

1. **One unified premium frontend.** One binary, one window. No
   LaunchBox/BigBox split.
2. **Two surfaces inside the window** (engine vs theme territory —
   see §4 mental model).
3. **Engine summon presentation:** fullscreen takeover (not
   slide-in, modal, or separate window).
4. **Engine summon corner:** top-right. Themes must reserve this
   slot for the engine-summon icon. Default hotkey `F12`, default
   controller chord `Select+Start`.
5. **Theme manifest format:** TOML (matches `system.yaml` peer
   format + supports inline comments).
6. **Theme swap requires app restart** in ARC 1. Hot-swap deferred
   to ARC 4.
7. **Build-time bundling only in ARC 1.** Runtime loading from
   extracted `.oatheme` zips deferred to ARC 2's tail (the `.oatheme`
   loader, post-D35 renumber) — Tauri's `tauri://localhost` origin
   breaks out-of-bundle dynamic imports without explicit CSP
   allowlist work.
8. **Kiosk plan's substrate spec absorbed.** The 4-layer model +
   `.oatheme` zip + federated Index + Theme Studio designed in
   KIOSK_PLAN.md §2 becomes the substrate for ALL of OA. Kiosk
   capabilities (attract mode, multi-monitor, 5-bus mixer) become
   substrate features themes opt into.

---

## 4. Mental model

```
+-----------------------------------------------------------+
|                     OA single binary                       |
|                                                            |
|   +--------------------------------------------------+    |
|   |              Engine territory (always)            |    |
|   |  Settings / Library Mgr / Import Wizard / BIOS    |    |
|   |  Core installer / System Health / Background Jobs |    |
|   |                                                   |    |
|   |  Summoned via: hotkey + controller chord +        |    |
|   |  always-visible top-right corner icon             |    |
|   +--------------------------------------------------+    |
|                                                            |
|   +--------------------------------------------------+    |
|   |        Theme territory (active `.oatheme`)       |    |
|   |  Library browsing / game launch ceremony /       |    |
|   |  now-playing / quick-settings overlay /          |    |
|   |  discovery surfaces                              |    |
|   +--------------------------------------------------+    |
|                                                            |
|   +-- Platform layer (shared, themes consume) ---------+   |
|   |  Tauri bridge (typed API)                         |   |
|   |  Stores (library, settings, layout, media, etc.)  |   |
|   |  Shared components (LibraryTile / Grid / Sidebar) |   |
|   |  Per-system data (registry, palette, assets)      |   |
|   +---------------------------------------------------+   |
+-----------------------------------------------------------+
```

---

## 5. Arc structure

| Arc | Focus | Estimate | Status |
| --- | --- | --- | --- |
| 1 | Minimum Viable Substrate — engine/theme separation + platform layer + Tauri hardening + Retroverse rebuilt as a theme + 2nd pilot. Layout + assets + palette only; no scripting or shaders. | ~22-26 weeks | **complete** bar the `.oatheme` loader (→ ARC 2 tail) |
| 2 | **Per-System Layout Substrate** — D32 per-system layout/view capability + D33 consumption opt-in + Per-System UI Stage 2/3 re-home + the `.oatheme` runtime loader. Fully declarative; no scripting/shaders. | TBD | **planned** — [PLANS/theming-arc-2-per-system-layout.md](theming-arc-2-per-system-layout.md) |
| 3 | **Cinematic & Scripting** (was ARC 2) — declarative motion/transitions + `<video>`/attract + Rhai scripting + WGSL shader hooks per KIOSK_PLAN §2.2. | ~7+ weeks | future |
| 4 | **Theme Studio** (was ARC 3) — in-engine visual + code editor per KIOSK_PLAN §2.3. | ~5-7 weeks | future |

> **Arc renumber (2026-06-15, DECISIONS D35):** the old ARC-2 ("Behaviors +
> Shaders") split — the declarative per-system *layout* capability (D32/D33)
> became its own arc (ARC 2), the cinematic/scripting axis moved to ARC 3, and
> Theme Studio bumped to ARC 4. The `.oatheme` loader (originally §6 Phase 5)
> moved into ARC 2's tail. Older "ARC 2 = Rhai+WGSL" / "Theme Studio (ARC 3)"
> references throughout this file + sibling docs reflect the pre-renumber
> numbering until next touched.

Arc 1 was the focus of this plan; ARC 2 has its own plan (linked above). ARCs
3-4 get their own plans when scheduled.

---

## 6. ARC 1 — phase breakdown

### Phase 1 — Engine/Theme surface separation (~4 weeks)

Extract the "boring necessary" surfaces from Retroverse into an
engine-owned surface that any theme summons via fixed affordance.

- **Define the surface boundary doc:**
  [features/theming-substrate/SURFACES.md](../features/theming-substrate/SURFACES.md)
  lists every surface OA renders, marks each as engine-territory
  or theme-territory.
- Move SETTINGS out of Retroverse's tab list. The 15 categories +
  per-system drill-in stay — they live in an engine-owned
  "Manager" panel rendered as fullscreen takeover when summoned.
- Same treatment for Library Manager + Import Wizard + BIOS
  pre-checks + Core installer + System Health + Background Jobs.
- Define summon affordance: configurable hotkey (default `F12`),
  controller chord (default `Select+Start`), always-visible
  top-right corner icon themes reserve a slot for.
- Retroverse drops from 6 tabs to 5 (HOME / LIBRARY / COLLECTIONS
  / PLAY NOW / DISCOVER). SETTINGS tab removed. Functionally
  indistinguishable — operator still reaches the same settings,
  via the engine surface.

**Critical files:**
- `frontend/src/layout/retroverse/RetroverseShell.tsx`
- `frontend/src/routes/retroverse/SettingsPage.tsx` (content
  moves to `frontend/src/engine/manager/SettingsPanel.tsx`)
- `frontend/src/components/perSystemSections.tsx` (becomes
  engine-owned canonical reference)
- `frontend/src/App.tsx` (engine-surface summon orchestration)

**Acceptance gate:** Operator hits `F12` from anywhere in
Retroverse → Settings opens → closes back to same library view.
Per-system drill-in works identically. All existing tests green.

### Phase 2 — Platform / Theme SDK foundation (~4 weeks)

Carve the platform layer out as an explicit boundary. Themes import
only from the platform.

- New top-level dir: `frontend/src/platform/`. Vite alias
  `@oa/platform`.
- Move into platform: `components/LibraryTile.tsx`,
  `components/LibraryView.tsx`, `layout/LeftSidebar.tsx`,
  `components/perSystemSections.tsx`, all stores
  (`settings/store.ts`, `library/store.ts`, `layout/state.ts`,
  `views/store.ts`, `library/gameInfo.ts`, etc.), all lib helpers
  (`lib/listenScoped.ts`, `lib/logbridge.ts`, `lib/toast.ts`,
  etc.), `themes/registry.ts`, `themes/systemUIConfigs.ts`.
- Define Theme manifest schema (`theme.toml`):
  ```toml
  id = "retroverse"
  name = "Retroverse"
  version = "1.0.0"
  schema_version = 1
  oa_version = "^0.x"
  entry = "./dist/index.js"
  entry_export = "default"
  default_route = "library"
  routes = ["home", "library", "collections", "play-now", "discover"]
  context_slots = ["library", "customCollections", "layout", "views", "settings"]
  required_engine_capabilities = []  # e.g. ["multi-monitor", "attract-mode"]
  reserves_corner = "top-right"  # always top-right in ARC 1
  ```
- Define Theme SDK TS interface:
  - Entry component receives `ThemeContext` (renamed from
    `RetroverseContext` — already shape-generic).
  - Engine-owned components (`<EngineSummonIcon />`,
    `<EngineManagerPanel />`) themes mount where they choose.
- ESLint boundary rule: `themes/*` cannot import from `routes/*`,
  `App.tsx`, or any non-platform path.
- Rename: `RetroverseContext` → `ThemeContext`,
  `RetroverseProvider` → `ThemeProvider`, `useRetroverse()` →
  `useTheme()`. Pure rename + re-export.
- Cleanup debt:
  - Collapse `HOTSPOT_SYSTEMS` triplicate
    (TouchHotspotOverlay.tsx, StylusOverlay.tsx, QuickSettings.tsx)
    into single `touchInputSupported: boolean` field on
    `SystemUIConfig`.
  - Wire `customComponent: "vectrex"` escape hatch consumer (or
    delete the field if Phase 3's `custom` nav primitive
    supersedes it).

**Critical files:**
- `frontend/src/platform/` (new — ~30 files moving in)
- `frontend/vite.config.ts` (alias)
- `frontend/.eslintrc.*` (boundary rule)
- `frontend/src/routes/retroverse/context.tsx` (rename)

**Acceptance gate:** TypeCheck silent. ESLint enforces no
non-platform escapes from theme code. Retroverse works identically;
rename is pure refactor.

### Phase 3 — Theme substrate: layout + palette + assets (~5 weeks)

The theme-level extension surface. No scripting or shaders yet.

- Per-system palette data extracted from `themes/systems.css` (45
  OKLCH blocks) into structured JSON co-located with each system
  at `config/systems/<id>/palette.json`. Build step generates
  `systems.generated.css` from the JSON for fallback consumption.
- Runtime CSS-variable injection: when a theme mounts, engine
  reads the theme's optional `palette_overrides` per-system map
  and injects scoped CSS variables (e.g.
  `[data-theme="cyberpunk"][data-system="nes"] { --color-system-accent: ... }`).
- Asset resolver generalized in
  `apps/oa-shell/src/system_ui_assets.rs`:
  `resolve_per_system_asset(theme_id, system_id, category, basename)`.
  Cascade: active-theme override → system baseline → `_baseline`
  fallback → null.
- Layout DSL choice: **start with hand-written TS components**.
  Themes are Solid component trees in their entry file. Declarative
  TOML/RON layout DSL deferred to ARC 2/4 when Theme Studio needs
  a serializable format.
- 5 engine-owned nav primitives themes pick from (per KIOSK_PLAN
  §3.1): `grid`, `wheel`, `list`, `carousel`, `custom`. Each a
  Solid component in `@oa/platform/nav/`. Theme entry picks +
  configures (orientation, density, focus prominence, easing).
- `customComponent` escape hatch becomes the `custom` nav
  primitive's general form.

**Nav input model — semantic verbs, user-remappable (first-class
principle, locked this arc; DECISIONS D18):**

- **Nav is verb-based, not button-based.** Navigation is a fixed
  vocabulary of abstract verbs: `Confirm`, `Back`, `Up`/`Down`/
  `Left`/`Right`, `NextSection`/`PrevSection` (the L1/R1 role —
  binds to whatever top-level structure the active theme exposes),
  `OpenQuickSettings`, `Menu` (+ room for `Search`/`Favorite`/
  `Page`). The 5 primitives AND the HintBar consume **verbs**, never
  raw buttons. So the HintBar renders glyphs from the *current*
  input→verb map (remap Confirm → Ⓧ and every hint updates for free).
- **A physical-input → verb indirection layer is established in this
  phase**, even before the remap UI ships, because retrofitting it
  after the primitives are written is far more painful than building
  the primitives verb-native from day one. The gamepad/keyboard layer
  translates raw input → verbs through a `navBindings` config.
- **User-remappable, OA-wide tier** (not per-theme, not per-game —
  one config in appData applied identically across all themes, so
  full user control AND muscle-memory consistency; per the three-tier
  settings split). Button *meanings* are a **per-user contract, NOT a
  per-theme** knob — themes restyle hints + pick layouts but never
  redefine `Back`. Default bindings = the existing operator-locked
  controller-nav spec (so out-of-the-box behavior is unchanged;
  remapping is opt-in). Refines, doesn't contradict, the 2026-05
  operator-locked nav spec: that spec becomes the *default* map.
- **Accessibility is the headline win** (rebind for reachability /
  preference; keyboard parity, not gamepad-only).
- **The remap Settings UI is a FOLLOW-ON SLICE**, sequenced *after*
  the toy-theme acceptance gate (the verb layer + default bindings
  alone prove the substrate; the Settings screen is then a small
  addition on top). It lives in the **OA-wide Input/Controls settings
  surface** and MUST include a **"Reset to defaults" button** (restore
  the baseline nav map). Design for: conflict validation (block
  Confirm==Back deadlocks), directional (D-pad/stick) remap, keyboard
  parity, and a guaranteed always-reachable escape hatch (a core
  keyboard binding that can't be fully unbound) so a user can't map
  themselves into a corner.

**Critical files:**
- `config/systems/<id>/palette.json` (45 new files; one-time
  generation)
- `frontend/src/themes/systems.css` (becomes generated)
- `apps/oa-shell/src/system_ui_assets.rs`
- `frontend/src/platform/nav/` (new — 5 nav primitives + the existing
  `src/nav/` focus/gamepad/HintBar/back framework relocated here, so
  the whole nav layer is one platform-owned, theme-shareable unit)
- `frontend/src/platform/nav/navBindings.ts` + `platform/api/` wrapper
  (new — the input→verb map; persisted OA-wide)

**Acceptance gate:** A toy second theme (`themes/bare/`) — sidebar
+ list-nav primitive + neutral palette — switchable from Settings
→ Appearance, renders a working library, **navigable end-to-end via
the semantic verbs** (default bindings). Doesn't need to be pretty;
proves the substrate supports more than Retroverse's IA. The remap
Settings UI + "Reset to defaults" is a separate slice gated after
this.

### Phase 4 — Tauri bridge hardening (~3 weeks)

Drain the direct-invoke leakage so themes only access the backend
via a typed platform API.

- **~150 `invoke(` sites across 37 files** (operator audit said
  73/19; Plan-agent's count is deeper and more accurate). Worst
  offenders: `App.tsx` (~25), `QuickSettings.tsx` (~20),
  `settings/store.ts` (~10), 16 others ≥3 each.
- New: `frontend/src/platform/api/` organized by domain
  (`libraryApi.ts`, `settingsApi.ts`, `mediaApi.ts`,
  `gameInfoApi.ts`, `coreApi.ts`, `jobsApi.ts`, `inputApi.ts`).
  Each function wraps `invoke()` with typed in/out + standard
  error toast handling. Pattern mirrors
  `frontend/src/lib/listenScoped.ts:22`.
- Components + stores call API functions; raw `invoke()` prohibited
  outside `platform/api/`. ESLint rule enforces.
- **SystemId parity test** (build-time CI check): Rust test asserts
  every TS-union `SystemId` arm has a matching Rust
  `oa_core::SystemId` variant AND a matching `parse_system_id`
  match arm at `apps/oa-shell/src/main.rs:578-699`. Fails build on
  drift.
- **`tauri-specta` codegen deferred as stretch** — hand-written
  wrappers sufficient for ~110 commands; annotation burden +
  macro error opacity not worth it at this scale.

**Critical files:**
- `frontend/src/platform/api/*.ts` (new — 7 domain modules)
- 37 frontend files (lose raw `invoke()` calls)
- `apps/oa-shell/tests/system_id_parity.rs` (new)

**Acceptance gate:** Grep for `invoke(` outside `platform/api/`
returns zero. SystemId parity test green. All existing tests +
typecheck green.

### Phase 5 — `.oatheme` distribution + loading (~3 weeks)

The on-disk format + how the engine discovers + loads themes.

- `.oatheme` is a zip containing: `theme.toml` manifest + `dist/`
  (built JS) + `assets/` (per-system + theme-wide) + optional
  `palette/` (per-system palette overrides).
- **Loose-folder dev mode:** drop a folder at
  `<exe_dir>/themes/<id>/` with the same shape; engine hot-reloads
  on file change.
- **Built mode:** drop a `.oatheme` zip at `<exe_dir>/themes/`;
  engine extracts to cache + registers.
- Loader (Rust): `apps/oa-shell/src/theme_loader.rs` discovers,
  validates manifest (schema version + OA version range +
  capability declarations), extracts zip, exposes `list_themes()`
  + `get_active_theme()` + `set_active_theme(id)` Tauri commands.
- Frontend: dynamic `import()` of the active theme's entry.
  **Build-time bundling only in ARC 1** — runtime loading from
  extracted zip deferred to ARC 2 (Tauri's `tauri://localhost`
  origin breaks out-of-bundle dynamic imports without explicit
  CSP allowlist work).
- Theme picker UI in engine Manager → Appearance.
- Conflict + failure policies:
  - Duplicate theme IDs: alphabetical priority + warning toast.
  - Active theme fails to load: fall back to bundled Retroverse +
    persistent banner explaining what's wrong.
  - Theme requires capability the engine doesn't have: refuse to
    load + show install message.

**Critical files:**
- `apps/oa-shell/src/theme_loader.rs` (new)
- `frontend/src/platform/themes/loader.ts` (new)
- `frontend/src/engine/manager/AppearancePanel.tsx` (new)

**Acceptance gate:** Drop a built theme folder into
`<exe_dir>/themes/`, restart, see it in Settings → Appearance,
switch to it, see UI change.

### Phase 6 — Retroverse rebuilt + second pilot theme (~3-4 weeks)

The dogfood test. **ARC 1 ACCEPTANCE GATE.**

> **Status (2026-06-11):** The **second-pilot** half landed early — CoverFlow was
> pulled forward in S2 (§13.3) as the rough 2nd shell, then deepened through S3-S5.
> The **Retroverse-as-theme move** (this section's headline) is **✅ shipped + merged
> 2026-06-11** (`feat/theming-retroverse-as-theme`, merge `711f337`; operator playtested —
> indistinguishable; DECISIONS D31; theming SESSION_LOG 2026-06-11). The reverse-import audit found zero files needing
> to hoist to platform (S2/Phase-4/grab-bag already hoisted everything shared), so the
> move was a pure relocation of Retroverse-private files into `themes/retroverse/` + a
> shim deletion + dropping the two `except: ['./retroverse']` ESLint exceptions
> (probe-verified). After merge, only **Phase 5** (`.oatheme` distribution/loader) remains
> open in ARC 1.

- Rebuild Retroverse against the Theme SDK as `themes/retroverse/`.
  Same UX, same controller-nav, same per-system theming, same
  tests, same screenshot fidelity.
- Build a second pilot theme — recommend a **"Wheel"** layout
  (horizontal coverflow, BigBox-inspired) to prove the SDK
  supports a different IA shape, not just a re-skin. ~1500-2500
  LOC ballpark. **Wheel design spec written as sub-plan when arc
  reaches Phase 5.**
- Both themes ship in the OA binary by default (built into
  `<exe_dir>/themes/`). Operator picks in Manager → Appearance.

**Critical files:**
- `themes/retroverse/` (new — Retroverse-the-theme)
- `themes/wheel/` (new — second pilot)
- `frontend/src/App.tsx` + bootstrap: load default theme on
  first launch; remember active theme in `LibraryPrefs`.

**Acceptance gate:** OA ships with Retroverse + Wheel themes.
Operator switches between them. All 700+ oa-shell tests green.
Frontend typecheck silent. Retroverse feels indistinguishable from
pre-arc behavior. Wheel theme boots, browses, launches games.

---

## 7. Sequencing relative to in-flight Virtual Library arc

- **Phases 1-2 run parallel with VL Phase A.** Engine/theme
  separation + platform extraction are mostly mechanical refactor;
  don't conflict with VL's schema work.
- **PAUSE Theming-Substrate arc at end of Phase 2.** VL Phase E
  (game_identities schema promotion, ~3-4 weeks) and VL Phase C
  (Launcher trait abstraction, ~2-3 weeks) both rewire components
  the Theming arc's later phases consume. Doing Phases 3-6 first
  guarantees a rewrite when VL Phase E reshapes the tile data
  model and VL Phase C reshapes QuickSettings around launcher
  capabilities.
- **Resume Phase 3+ after VL Phase E ✓ and VL Phase C ✓.** Net
  Theming arc slips ~6-9 weeks but avoids guaranteed rewrite cost.

---

## 8. Risks

**R1 (HIGH) — App.tsx is the integration god-component.** Mounts
MediaProvider / PlatformMediaProvider / GameInfoBadgesProvider /
RetroverseProvider, owns 30+ dialog state signals, 1500+ line
value-object that became `RetroverseContext`. Phase 1 calls this a
"pure rename" but isn't — much of App.tsx's state really belongs
in Platform, not in any single theme's context. **Decide in
Phase 1:** does dialog state (savesEntry, gameInfoFor,
contextMenuFor, helpDialog, wizardOpen) live in Platform or in
each theme? Recommend: Platform owns dialog open/close state;
themes pick where dialogs anchor.

**R2 (MEDIUM) — Vite dynamic-import + Tauri `tauri://localhost`
origin.** Production Tauri serves from a non-`file://` origin;
dynamic `import()` of files outside the bundled `dist/` directory
fails without explicit CSP allowlist. **Mitigation:** ARC 1 ships
build-time bundling only. Runtime loading deferred to ARC 2.

**R3 (MEDIUM) — Lifecycle on theme swap.** Tearing down the
active theme's context + remounting cleanly is non-trivial
(gamepad listeners, audio routing, focus state). **Decision for
Phase 5:** theme swap requires app restart in ARC 1. Hot-swap
deferred to ARC 4 alongside Theme Studio.

**R4 (MEDIUM) — SystemId drift silently falls through to PcEngine.**
Confirmed at `main.rs:578-699`. Phase 4 parity test must run in
CI. A theme shipping a custom per-system block for `"3do"` would
silently degrade if Rust hasn't grown that arm.

**R5 (LOW arch) — `customComponent: "vectrex"` is unrealized
indirection.** Field exists, nothing reads it. Phase 2 should
either wire a consumer or delete in favor of Phase 3's `custom`
nav primitive.

---

## 9. Patterns to reuse

- `frontend/src/lib/eventListener.ts:22` (`listenScoped`) — exact
  shape Phase 4 typed-API wrappers follow (Solid `onCleanup`
  integration + cancelled-flag pattern).
- `frontend/src/lib/retroverseFlag.ts:23-30` — bridge pattern
  (`setX` from `App.tsx` createEffect, reactive `useX()` accessor)
  is the right shape for `activeThemeId()` signal.
- `frontend/src/components/perSystemSections.tsx` —
  operator-blessed shared-component pattern. Canonical reference
  for Phase 2's platform component contract.
- `apps/oa-shell/src/system_ui_assets.rs::resolve_background_asset`
  — cascade pattern Phase 3's generalized resolver extends.
- `frontend/src/themes/systemUIConfigs.ts` — `BASELINE_UI` +
  per-system override pattern. Same approach themes use to declare
  per-system tweaks.

---

## 10. Critical files

Representative — pattern repeats across many.

- `frontend/src/App.tsx` (lines 1429-1518 god-component — Phase 1)
- `frontend/src/routes/retroverse/context.tsx` (rename — Phase 2)
- `frontend/src/themes/registry.ts:1` (SystemId parity anchor —
  Phase 4)
- `frontend/src/themes/systems.css` (becomes generated — Phase 3)
- `apps/oa-shell/src/system_ui_assets.rs` (resolver generalization
  — Phase 3)
- `apps/oa-shell/src/main.rs:578-699` (`parse_system_id` parity
  test target — Phase 4)

Files NOT moved into platform: every `routes/retroverse/*Page.tsx`
(becomes Retroverse-theme-private in Phase 6),
`layout/retroverse/RetroverseShell.tsx` (same),
`routing/currentRoute.ts` (Retroverse-specific tab routing).

---

## 11. Verification

Per-phase acceptance gates above. End of arc:

1. `cargo test -p oa-shell` — all 700+ tests green.
2. `cd frontend && npm run typecheck` — silent.
3. `cd frontend && npm run lint` — boundary rules green (no theme
   escapes from `@oa/platform`, no raw `invoke()` outside
   `platform/api/`).
4. SystemId parity test green.
5. **Operator dogfood:** open OA, hit `F12` from Retroverse →
   Settings opens, close → back to library. Manager → Appearance
   → switch to Wheel theme → app reloads → Wheel theme renders →
   browse → launch a game → game runs → exit → back to Wheel.
   Switch back to Retroverse → indistinguishable from pre-arc
   behavior.
6. Drop a third hand-built theme folder at
   `<exe_dir>/themes/test/` → appears in Appearance picker → loads.

---

## 12. Cross-arc relationships

- **Virtual Library arc** ([docs/PLANS/virtual-library-and-launcher-arc.md](virtual-library-and-launcher-arc.md))
  — sequenced together per §7. VL Phase E + C land mid-arc.
- **Per-System Custom UI** ([docs/PLANS/per-system-ui.md](per-system-ui.md))
  — **merged into ARC 2** (D32/D33/D34). Stage 1 machinery shipped (now in
  `platform/`); Stage 2/3 + the GB/NES/Vectrex pilots re-home into ARC 2 as
  **Retroverse content** consumed via the per-system substrate capability —
  not a paused side-stream. See
  [theming-arc-2-per-system-layout.md](theming-arc-2-per-system-layout.md) §3.
- **Kiosk Shell** ([docs/features/kiosk-shell/KIOSK_PLAN.md](../features/kiosk-shell/KIOSK_PLAN.md))
  — KIOSK_PLAN §2.2-2.5 is the source spec for ARCs 3-4 (Rhai
  behaviors + WGSL shaders + Theme Studio). Kiosk-as-mode
  capabilities (attract / multi-monitor / 5-bus mixer) become
  substrate features any theme opts into.

---

## 13. Addendum (2026-06-10) — vision corrections + skeleton-first resequence

A design conversation before Phase 3 surfaced three things that refine
(not replace) the plan above. Decisions live in
[features/theming-substrate/DECISIONS.md](../features/theming-substrate/DECISIONS.md)
D19–D20; the forward-looking scope calls are captured here.

### 13.1 The two vision corrections

1. **Per-system theming is a Retroverse feature, not a substrate contract
   (D19).** The substrate's purpose is **swappable whole-shells** (BigBox-style);
   per-system "worlds" are Retroverse's take, not a platform-mandatory axis.
   Per-system data stays platform-provided; *consuming* it is each theme's choice.
   The Phase 3 palette pillar is therefore **theme-first** — per-system tokens are
   an optional sub-cascade, and the §6 "cascade precedence" question (theme vs
   per-system) drops from a thorny decision to low-stakes plumbing (theme composes
   over per-system; outer-layer cascade as written).
2. **Kiosk/cabinet capabilities are platform features, deferred (D20).** Attract,
   CRT/shader chrome, multi-monitor (marquee / manuals / second-controls) are
   engine-owned platform toggles a shell opts into via the manifest's
   `required_engine_capabilities` field — out of scope for a good while (ARC 3-4).
   **Two cheap seams reserved in ARC 1** so they don't become expensive
   retrofits: (a) the theme-host lifecycle is written as a *general* "platform can
   preempt + restore the theme" pattern (not an F12-special-case), so attract
   slots in later for free; (b) the manifest declares **named surfaces** a theme
   provides layouts for, ARC 1 supporting exactly one (`main`), so multi-monitor
   surfaces are additive later instead of a rewrite. CRT/shaders need nothing now.

### 13.2 Forward-looking scope calls (decided this conversation)

Each judged by "how expensive to retrofit *after* themes bind to the surface?"
Audience = "both — me now, creators later" → contracts built creator-grade now.

| # | Item | Call |
| --- | --- | --- |
| 1 | **Design-token contract** — documented full set a theme may override (palette / spacing / radii / fonts / motion); engine territory consumes a **separate, non-overridable** token set so a theme can't wreck Settings (D2 guarantee). | **Build now** — this IS pillar 1 done right. |
| 2 | **Versioned theme manifest from theme #1** (`schema_version` + `oa_version` range + `capabilities[]` + `surfaces[]` + metadata). | **Build now (light).** |
| 3 | **A11y + motion baseline as tokens** — `prefers-reduced-motion` gate, focus-visible ring token consumed by the nav primitives, contrast-checked default palette. | **Build now** (folds into #1; not a full WCAG audit). |
| 4 | **Controller-glyph abstraction** — HintBar renders glyphs via a `glyphSet` indirection (verb → glyph), one default set shipped. | **Seam now** — defer auto-detect + Xbox/PS/Switch picker. |
| 5 | **Theme vs per-system precedence** — per D19, theme composes over per-system. | **Decide now** (decided; resolver already has the cascade shape). |
| 6 | **Audio as a resolver category** — `ui-sound` category + a verb→sound hook in the new primitives (engine defaults). | **Seam now.** |
| 7 | **Theme contract validator + CI test** — manifest parses, required tokens present, declared nav primitive + surfaces exist. The `bare` theme becomes its fixture. | **Build now (light)** — the load-time validator + one fixture. |
| 8 | **DSL-friendly primitive APIs** — primitives take declarative config objects (orientation/density/focus/easing/data-source), minimal imperative escape hatches, so a future serializable DSL (ARC 2/4) can target them. | **Seam now** (pure discipline, zero extra code). |
| 9 | **Per-theme settings namespace** — reserve a namespaced slice of the settings store for theme-owned prefs (collision-free). One toggle in `bare` proves it. | **Seam now.** |
| 10 | **Loose-folder hot-reload for dev** | **Defer** — Vite HMR already hot-reloads the in-bundle toy theme; the real watcher is Phase 5. |

Net: "build now" collapses to **two coherent workstreams** — the token layer
(#1+#3, with engine-scoping) and the manifest+validator pair (#2+#7) — plus a
handful of cheap seams (#4/#6/#8/#9) and one decision (#5). It does **not**
balloon Phase 3; it makes pillar 1 thorough and stamps three contracts while
we're in the right files. New consolidated artifact: **`THEME_CONTRACT.md`**
(token set + verb vocabulary + manifest schema + resolver categories + surfaces)
— the theme-facing peer of SURFACES.md, and what the validator checks against.

### 13.3 Skeleton-first resequence ("pull the vertical slice forward")

The plan (§6) saves the proof for Phase 6 (Retroverse-as-theme + Wheel). The
operator chose to **pull a vertical slice forward**: stand up *two switchable
whole-shells* as early as possible — even rough, even on a partial substrate —
then deepen each substrate layer *underneath* a thing that already visibly works
(a walking skeleton / tracer bullet), instead of a 22-26-week plumbing march
before the first swap. ARC boundaries are unchanged (the operator did **not**
fold ARC 3-4 magic into ARC 1); only the *order within ARC 1* changes.

The early milestone borrows thin slices from three plan phases at once:
- **Phase 3** — verb-native nav layer + `list`/`grid` primitives + token
  injection (Slice 1, as planned).
- **Phase 5** — just enough active-theme machinery to flip between two themes at
  restart (build-time bundled; no `.oatheme` zips — D6 holds).
- **Phase 6** — Retroverse wrapped as the default theme + **Wheel pulled
  forward** as the second shell (iconic BigBox coverflow — proves a *different
  IA*, not a reskin, the moment it boots).

**Honest caveat:** ARC-1 Wheel is layout + palette + distinct typography/feel —
genuinely a different shell, but the *cinematic* layer (attract, CRT ceremony,
shaders) is still ARC 3-4. The vertical slice proves **swappability + distinct
identity early**; wow-polish lands later.

**Revised ARC-1 slice order** (supersedes §6's phase-sequential order for
execution; the §6 phase *content* is unchanged, just reordered + interleaved):

- **S1 — Nav foundation.** Lock the verb vocabulary (start: `Confirm`, `Back`,
  `Up`/`Down`/`Left`/`Right`, `NextSection`/`PrevSection`, `OpenQuickSettings`,
  `Menu`; reserved `Search`/`Favorite`/`Page`). Relocate `src/nav/` →
  `platform/nav/`. Build the physical-input→verb indirection (`navBindings`,
  OA-wide) + `platform/api/` wrapper. Ship `list` + `grid` primitives **verb-
  native + declarative-props (#8)**. Defaults = the operator-locked controller-nav
  spec.
- **S2 — Walking skeleton (the vertical slice).** Minimal active-theme switch
  (restart-based) + Retroverse wrapped as default theme + a rough **Wheel** second
  shell. Acceptance: switch Retroverse ⇄ Wheel from Settings → Appearance, both
  browse + launch. **This is where the dream first becomes visible.**
- **S3 — Token layer.** The design-token contract (#1) + a11y/motion baseline
  (#3) + engine-territory token isolation. Wheel + Retroverse re-skin through
  tokens. Write `THEME_CONTRACT.md`.
- **S4 — Manifest + validator.** Versioned `theme.toml` (#2) with `capabilities[]`
  + `surfaces[]` (single `main` surface honored) + the load-time validator + CI
  fixture (#7); `bare` theme as the fixture.
- **S5 — Substrate depth.** Palette substrate, generalized asset resolver (theme
  cascade, **+ `ui-sound` category #6**), glyph-set seam in HintBar (#4), per-theme
  settings namespace (#9), remaining `wheel`/`carousel`/`custom` primitives.
  **Sliced 2026-06-11 into 5 per-sub-area micro-slices** (operator choice; design
  forks signed off via AskUserQuestion; order = contracts first), with three
  refinements to the literal scope above:
  - **S5.1 — resolver theme tier ✅ merged** (`783da2e`, DECISIONS D25). Theme tier
    on `resolve_background_asset` + `resolve_ui_sound`; theme overrides home under
    `<exe_dir>/assets/themes/<id>/system-ui/…` (operator-droppable now, no Phase-5
    loader). The **background resolver tier is ready-but-unconsumed** (its consumer
    `SystemBackground` is unmounted since 2026-05-31) → **reviving a theme-owned
    background surface folds into S5.5**.
  - **S5.2 — palette substrate.** Refinement (operator): the per-system palette
    extracts to a **typed `SYSTEM_PALETTES` single-source map** with the baseline
    `[data-system]` CSS **derived at boot** (retiring hand-authored `systems.css`),
    NOT `config/systems/<id>/palette.json` + a build step — per-system palette is
    frontend-only data with no Rust reader, so a typed map is the right home (avoids a
    generated-file drift + a cross-language generator). Plus the per-theme
    `perSystemTokens` scoped override seam (D19's optional sub-cascade) + validator
    extension.
  - **S5.3 — glyph-set:** manifest `glyph_set` field + a 2nd (PlayStation) built-in
    set + `activeGlyphSet()` indirection; auto-detect/picker deferred.
  - **S5.4 — per-theme settings namespace** (#9): `localStorage` `themeId→{}` map +
    host-bound `useThemeSettings()` (collision rule = auto-bound active id) + one
    proof toggle in `bare`.
  - **S5.5 — primitives:** `carousel` (dogfooded into CoverFlow) + `custom` (escape
    hatch) + the #6 verb→sound hook; `wheel` contract typed + reserved (no ARC-1
    consumer); **+ revive a theme-owned background surface** (S5.1 fold-in).
- **Follow-on (after S2's swap gate):** the nav-remap Settings UI (gamepad +
  keyboard rebinding to verbs, conflict validation, always-reachable escape
  hatch, **"Reset to defaults"** = baseline nav map) per D18.

S1 is the immediate next code. S2 is the morale/de-risk milestone. S3-S5 harden
beneath a working swap.
