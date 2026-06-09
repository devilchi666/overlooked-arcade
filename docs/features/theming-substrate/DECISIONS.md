# Theming Substrate — Decisions

Append-only log of implementation decisions made during the build.
Strategic decisions made in the planning conversation live in
[docs/PLANS/theming-substrate.md](../../PLANS/theming-substrate.md)
§3.

---

## 2026-06-06 — Planning decisions (locked)

Captured from the planning conversation that produced
[docs/PLANS/theming-substrate.md](../../PLANS/theming-substrate.md).

### D1 — One unified premium frontend (no binary split)

OA stays one binary, one window. **No LaunchBox/BigBox-style split
into separate Studio + Launcher apps.** Considered explicitly;
rejected.

**Why:** Couch gamers primary (per [VISION.md](../../VISION.md))
— a split makes every settings change a window-switching ceremony.
Tauri's mental model is one app, one webview, one backend —
splitting would add cross-process SQLite locking + IPC + two
updaters + two installers + doubled CI matrix forever. The 15
shipped SETTINGS categories + per-system drill-in are real work
that would have to be re-ported. LaunchBox/BigBox split for
historical/business reasons that don't apply to OA (LaunchBox
shipped first in 2010; BigBox is a paid $50 add-on).

**How to apply:** When a future contributor proposes splitting
OA into Studio + Launcher, point them here. The architectural
unlock that splitting provides (theme creator scope reduction) is
achieved instead via D2 (engine vs theme territory inside one
window).

### D2 — Two surfaces inside one window (engine vs theme territory)

OA's UI splits into engine territory (always engine-rendered,
visually neutral, summoned via fixed affordance) and theme
territory (where the active `.oatheme` package draws).

**Engine territory:** Settings, Library Manager, Import Wizard,
BIOS pre-checks, Core installer, System Health, Background Jobs.

**Theme territory:** library browsing, game launch ceremony,
now-playing, quick-settings overlay, discovery surfaces.

**Why:** Same scope-reduction benefit BigBox themers get (themes
don't redesign Settings) without splitting the binary. Theme
creators get a tight, achievable scope. The "boring necessary"
parts of OA don't degrade visually under poorly-designed themes.

**How to apply:** Phase 1 of ARC 1 implements this. SURFACES.md
will be the canonical surface-by-surface assignment.

### D3 — Engine summon: fullscreen takeover, top-right corner

When the operator summons the engine surface (Settings et al.),
it fullscreen-takes-over the OA window — not a slide-in drawer,
not a modal overlay, not a separate window. The "summon" icon
themes must reserve lives in the top-right corner. Default
hotkey `F12`; default controller chord `Select+Start`. All three
affordances reach the same engine surface.

**Why:** Fullscreen takeover is the most controller-friendly
presentation (full focus, no overlay-vs-background ambiguity).
Top-right keeps the icon out of typical browsing focus paths.
`F12` matches established convention (RetroArch uses F1).

**How to apply:** Phase 1 implements all three affordances.
Theme manifest's `reserves_corner` field must be `"top-right"`
in ARC 1. Future relaxation (themes pick a corner) deferred
until justified.

### D4 — Theme manifest format: TOML

`theme.toml`, not `theme.json` or `theme.yaml`.

**Why:** Matches `config/systems/<id>/system.yaml` peer format
philosophy (declarative + comment-friendly) without YAML's quirks
(significant whitespace + tag mode confusion). TOML supports
inline comments which JSON doesn't.

**How to apply:** Phase 2 writes the manifest schema + parser.

### D5 — Theme swap requires app restart (ARC 1)

Switching the active theme via Manager → Appearance reloads OA.
**Hot-swap deferred to ARC 3** alongside Theme Studio.

**Why:** Tearing down the active theme's context + remounting
cleanly is non-trivial — gamepad listeners, audio routing, focus
state, all need orchestrated unmount. Shipping that complexity in
ARC 1 risks delaying the substrate launch. Restart is acceptable
UX for a setting an operator changes maybe once a month.

**How to apply:** Phase 5 wires `set_active_theme(id)` to trigger
a Tauri app restart, not a live re-mount.

### D6 — Build-time bundling only (ARC 1)

Themes shipping inside the OA binary or as loose folders in
`<exe_dir>/themes/<id>/`. **Runtime loading from extracted
`.oatheme` zips deferred to ARC 2.**

**Why:** Tauri's `tauri://localhost` origin breaks out-of-bundle
dynamic `import()` without explicit CSP allowlist work. That
CSP work is real and shouldn't gate the substrate launch. Loose
folders are sufficient for dev / dogfood / first wave of
operator-curated themes.

**How to apply:** Phase 5 loader only walks loose folders +
extracts zips for static-bundle inclusion. Dynamic runtime
loading lands when ARC 2 adds scripting (and the CSP work
becomes load-bearing for Rhai sandboxing anyway).

### D7 — Kiosk plan's substrate spec absorbed

The 4-layer model + `.oatheme` zip + federated GitHub Index +
in-engine Theme Studio designed in
[docs/features/kiosk-shell/KIOSK_PLAN.md](../kiosk-shell/KIOSK_PLAN.md)
§2.2-2.5 becomes the substrate for ALL of OA. Kiosk-as-such
(attract mode, multi-monitor, 5-bus mixer) becomes capabilities
the substrate exposes; themes opt in via manifest's
`required_engine_capabilities` field.

**Why:** The Kiosk plan's spec is good. It just isn't actually
kiosk-specific — it's a theming substrate that the Kiosk
implementation happened to design first because that's where the
need was most acute. Building two parallel substrates (one for
desktop, one for kiosk) would diverge fast and create maintenance
debt forever.

**How to apply:** ARCs 2-3 of this plan correspond to KIOSK_PLAN
§2.2 (Rhai + WGSL) and §2.3 (Theme Studio). The kiosk-shell
feature folder stays — it'll eventually hold the Kiosk-mode
specifics (attract mode, multi-monitor, 5-bus mixer) once those
are implemented as substrate capabilities. The 4 reference themes
KIOSK_PLAN §2.5 specs become substrate-level reference themes
rather than kiosk-exclusive.

---

## 2026-06-09 — Boundary enforcement decisions

Made while starting the "clear, **enforced** platform/theme separation"
work the operator asked for ("so we can add to the platform or theme
without accidentally wiring them back together with new features or
fixes"). Branch `feat/theming-boundary-enforcement`.

### D8 — Enforce the boundary with a build-checked lint, not convention

The engine/theme split is worthless if it's only documented — it rots the
moment a well-meaning change crosses a layer. Proof: the 2026-06-09 bootless
feature made `platform/components/SystemHeader` import `useTheme` from
`routes/retroverse/` (platform depending on theme) and nothing caught it.

**Decision:** add an ESLint flat config (`frontend/eslint.config.mjs`) that
is a **boundary linter ONLY** — no style/quality rules (`tsc` + review own
those). Its single job is `import/no-restricted-paths` zones encoding the
layer contract in [SURFACES.md](SURFACES.md) §"Layer boundary contract". A
cross-layer import becomes a `npm run lint` failure.

**Why boundary-only:** turning on a full ruleset would flood the codebase
with hundreds of pre-existing style nits, burying the one signal that
matters and making `lint` permanently red (so nobody runs it). A tiny,
always-green boundary linter is one that actually gets run + trusted.

**How to apply:** new layer invariants are added as `no-restricted-paths`
zones as each becomes enforceable. Run `npm run lint` in CI / pre-merge.

### D9 — Enforce-now-then-classify (invert the plan's Phase 3-first order)

The plan (§6) sequences ARC 1 as Phase 3 (substrate) → 4 (Tauri bridge) →
5 (packaging) → 6 (rebuild). That order optimizes for *shipping a theme
ecosystem*. The operator's near-term goal is narrower + more urgent: a
**clean enforced separation** so ongoing platform/theme work doesn't
re-couple.

**Decision:** front-load enforcement. Slice 1 = lint + fix the live
platform↛theme leak (green today). Slice 2+ = drain the 48-file
`components/` grab-bag into the right layers, tightening the lint as it
shrinks. Phase 4 (typed Tauri bridge) drains raw `invoke()`. The plan's
Phase 3/5/6 (substrate / packaging / rebuild) — which are about *enabling
other themes*, not decoupling Retroverse — come after, unchanged.

**Why:** you can't draw an enforceable line while 48 files sit on neither
side of it, and that grab-bag is exactly where features re-couple. Locking
the already-clean edges first stops the bleeding immediately; the grab-bag
relocation then proceeds behind a ratchet that can only tighten.

### D10 — Platform components are prop-driven; the theme supplies handlers

Fixing the SystemHeader leak set the pattern for every future
platform-needs-something-from-the-theme case.

**Decision:** a `platform/` component that needs an app/theme action takes
it as an **optional prop** and the theme (a `routes/` file, which may use
`useTheme()`) supplies it. Platform components never call `useTheme()`.
SystemHeader's "Boot without game" now takes `onBootWithoutGame?` threaded
LibraryPage → LibraryView → SystemHeader; the button renders only when the
handler is wired.

**Why:** keeps platform a pure, theme-agnostic foundation — a different
theme reusing SystemHeader isn't forced to provide a Retroverse-shaped
context, just the handlers it cares about (or none).

### D11 — Store-context split: `usePlatform()` for stores, `useTheme()` for theme

The `ThemeContext` (`routes/retroverse/context`) bundled the platform STORES
(library / customCollections / layout / views / settings) + shared selection
state (searchQuery / focusedEntry / currentView) together with theme/gameplay
handlers. That forced **any** component needing a store — including
engine-surface components like `SettingsPanel` — to import the theme context,
inverting the layer boundary (engine/platform ↛ theme). It's the structural
blocker behind the `engine↛routes` edge and the bulk of the `components/`
grab-bag drain (most grab-bag files read a store via `useTheme()`).

**Decision (2026-06-09, Slice 2 batch 2):** introduce a platform-level
`PlatformProvider` + `usePlatform()` (`platform/platformContext.tsx`)
exposing the stores + shared state — theme-agnostic. App.tsx provides BOTH
`PlatformProvider` and `ThemeProvider` from the **same store instances**, so:
- Theme code (`routes/`) keeps using `useTheme().settings` etc. — untouched.
- Engine + platform code uses `usePlatform()` — never the theme context.

Migration is incremental + low-risk (both contexts live off one set of
instances, no behavior change). First migrated: `engine/SettingsPanel`
(was the one engine file using `useTheme()` for `ctx.settings`).

**How to apply:** when relocating a grab-bag/theme component into
engine/platform, switch its `useTheme()` store reads to `usePlatform()`.
Theme-only gameplay handlers stay on `useTheme()`. Eventually `ThemeContext`
sheds its store fields (reads them from `usePlatform()`) once no theme code
reads stores through it — but that cleanup isn't required for the boundary.

**Why a second provider, not just props:** the stores are read by dozens of
components at many tree depths; prop-drilling five stores everywhere is the
churn this split exists to avoid. Handlers (few, app-specific) stay
prop/`useTheme`-driven per [D10]; data (pervasive, stable) gets a context.

### D12 — A leaf shared by two layers belongs to the LOWER layer

Made during the grab-bag drain (2026-06-09, `feat/theming-grabbag-drain`)
when classifying components imported by BOTH an engine surface and a platform
(per-game) surface: `CoreOptionsPanel`, `AnalogBindingsSection`, and the
reference cards (Keypad / LightGun / GenesisPad) are imported by
`SystemBindingsEditor` / `SystemDialogs` (→ engine) AND by `GameDialogs`
(→ platform). The grab-bag plan's literal list put a couple under engine.

**Decision:** when a leaf is consumed by more than one layer, it lands in the
**lowest** consuming layer (here: `platform/components/`), regardless of which
layer "feels" like its conceptual owner. Putting a dual-consumed leaf in
engine would force the platform consumer to import engine — exactly the
inversion the boundary forbids. The litmus the plan gives ("does a THEME need
to render this?") is the tie-breaker only for single-consumer files; a
two-layer consumer is decided by this rule.

**Corollary — sever, don't relocate, when the edge is a re-export:**
`GameDialogs` (platform) appeared to depend on `SystemDialogs` (engine), but
only pulled a block (`BezelPicker` / `OverscanEditor` / …) that `SystemDialogs`
itself merely **re-exported** from `@oa/platform/components/perSystemSections`.
The fix was to import from the true source, deleting the cross-layer edge
rather than dragging `SystemDialogs` down a layer. Check for re-export
indirection before classifying by apparent import.

**How to apply:** future drains / new components — grep every importer first;
if importers span layers, file under the lowest; if the only cross-layer
import is a re-export, repoint it at the original.

### D13 — App-scoped action singletons use a platform registry, not prop-drilling

`SettingsSections` (now `engine/`) read 5 app-action handlers off
`useTheme()`. Once engine-resident it can't touch the theme context. Three
(`onOpenImportWizard` / `onOpenDebugLog` / `onOpenKeyboardShortcuts`) are thin
wrappers over `@oa/platform/dialogs` setters and were inlined as direct
`setWizardOpen` / `setHelpDialog` calls. The other two — `onAddLibraryFolder`
/ `onRescanLibraryFolders` — are App-scoped behaviours coupled to App-local UI
state (status / busy toasts, scan-progress reporter, post-ingest auto-sync).

**Decision:** expose the two via a platform module-singleton registry
(`platform/libraryAdmin.ts`): App calls `registerLibraryAdmin({...})` on mount;
engine code calls `addLibraryFolder()` / `rescanLibraryFolders()`. This
mirrors `platform/engineSurface.ts` and `platform/dialogs.ts` (platform owns a
stable surface; App wires behaviour into it).

**Why not prop-threading per D10:** D10's prop rule targets *theme-supplied*
handlers a platform component optionally renders (e.g. SystemHeader's
"Boot without game"). These two are different: App-scoped singletons (one
library per app) reached through three intermediary layers
(EngineManagerSurface → SettingsPanel → SettingsSections) that have no other
reason to know about them. Threading two props through three layers expresses
nothing the registry doesn't, with more churn. The registry states the real
ownership: App owns the behaviour, engine merely triggers it. D10 still
governs genuine theme→platform handlers; D13 carves out App-scoped action
singletons that cross into engine.
