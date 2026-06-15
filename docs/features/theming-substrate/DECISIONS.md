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

### D14 — `platform/api/` wrapper convention (Phase 4): generic getters for shape-divergent commands; api layer owns the backend-contract type

Phase 4 Slice 1 (2026-06-09, `feat/theming-platform-api-settings`) created the
first typed Tauri-bridge module, `platform/api/settingsApi.ts`. Two patterns
locked here apply to every later slice.

**Generic getters where call sites disagree on the return shape.** Several
commands (`get_game_overrides`, `get_system_settings`) are read by multiple
files that each declare only the fields they touch — `App.tsx`'s `GameOver`,
`GameDialogs`'s 19-field `GameOverrides`, `perSystemSections`'s
`PerSystemOverrides`, `AnalogBindingsSection`'s `{ analogRouting? }`. These are
deliberate **partial views** of one backend struct, not duplicates to unify.
Forcing one canonical return type would over-constrain the fuller callers and
under-type the narrow ones.

**Decision:** shape-divergent getters are generic with a canonical default —
`getGameOverrides<T = GameOverrides>(id)` / `getSystemSettings<T = SystemSettings>(systemId)`.
Every existing call site already passed an explicit `invoke<LocalType>(...)`,
so migration is mechanical (`invoke<T>("cmd", args)` → `wrapper<T>(args)`) with
zero type churn and no `any`. The canonical default types (`GameOverrides`,
`SystemSettings`, `OverscanCropPrefs`, `VideoState`) are **defined + exported
in the api module** — the api layer is the proper home for backend-contract
shapes, and new code gets a real default. Setters that take a cleaned payload
accept `Record<string, unknown>` (matching the call sites' generic carry-through
that lets unknown Rust-struct fields round-trip).

**Why not import the component types instead:** `settingsApi` is under
`platform/`, so the `platform ↛ components` lint zone forbids it importing
`GameOverrides` from `platform/components/GameDialogs`. The contract type must
live in the api (or a lower) layer regardless.

**Import style:** named imports by default (tree-shakeable, greppable); a
`import * as settingsApi` namespace alias only where a file's local signal
setters / exports shadow the wrapper names (`setScalingMode`, `setWindowMode`,
`playAudio`, `setPresentationMode`, …). The namespace form is still
named-export-based and statically analyzable.

**Scope discipline (also a precedent):** assign a command to its module by
**concern, not by caller**, and don't migrate a command from another slice's
module just because it sits in a file you're touching — `set_rewind_config`,
`apply_game_core_options`, `arm_*` and `get/set_layout` were left as raw
`invoke()` in mid-migration files because they belong to `rewindTasApi` /
`coresApi` / `inputApi` / `viewsApi`. A module is "done" only when its command
strings grep to only its api file; a file can legitimately straddle slices
until then (the lint rule is off until Slice 6).

**How to apply:** later slices follow the same shape — generic getter +
canonical default for any command with divergent call-site views; concrete
typed params otherwise; contract types exported from the api module; migrate
strictly the slice's own commands.

### D15 — Existing typed-binding modules MOVE to `platform/api/` + re-export for compat

Phase 4 Slice 3 (2026-06-09, `feat/theming-platform-api-media`) hit a case
Slices 1-2 didn't: two files — `platform/library/gameInfo.ts` and
`platform/library/systemInfo.ts` — were **already** thin typed `invoke()`
wrapper modules (one exported function per command + the domain types),
predating the `platform/api/` concept. Slices 1-2 only ever migrated *inline*
`invoke()` call sites inside store/component files; here the call site IS a
typed wrapper whose name (`getGameInfo`, `refreshMameSystemInfo`) collides 1:1
with the wrapper mediaApi would expose.

**Decision:** the wrapper functions **move** into the api module (their proper
long-term home — the command string lands there, satisfying the one-place
rule), and the original domain module **re-exports** them
(`export { getGameInfo, … } from "@oa/platform/api/mediaApi"`). The shared
domain TYPES (`MergedGameInfo`, `GameInfoOverride`, `MameRefreshReport`,
`LibraryEntryForBadges`, …) stay in the domain module; mediaApi pulls them in
via `import type` (erased at runtime, so the value re-export one way + the
type import the other way is **not** a runtime cycle).

**Why move-and-re-export, not delegate or repoint:**
- *Delegating* (keep `gameInfo.getGameInfo` defined, have it call
  `mediaApi.getGameInfo`) double-wraps an identical signature for zero benefit —
  two definitions of the same thin pass-through.
- *Repointing every consumer* (import the function from mediaApi, the types
  from the domain module) splits each consumer's single import into two and
  churns 3+ unrelated files per migrated module.
- Re-export gives exactly one definition (in the api module, the discoverable
  surface), keeps consumers' import paths working unchanged, and keeps the
  types co-located with their domain. New code can import the function from
  either path; the lint rule (Slice 6) bans raw `invoke`, not non-api import
  sources, so the re-export is permanently fine.

**Distinction from D14's store-method case:** Slice 1-2 store files
(`library/store.ts`, `customCollections.ts`) kept their functions and swapped
`invoke(...)` → `api.xxx(...)` inside because those functions carry real logic
(signal updates, dedup, post-hydrate) — they're not pure pass-throughs. D15
covers the *pure typed-binding* file whose function adds nothing over the
wrapper; that one moves wholesale.

**How to apply:** when a later slice's command lives in a pre-existing thin
typed-binding module (vs. an inline call site), move the wrapper to the api
module and re-export from the domain module; leave commands that belong to a
*different* slice defined-in-place (e.g. systemInfo's six `get/set_system_info*`
stay raw until Slice 6, so systemInfo.ts keeps its `invoke` import and just
re-exports the one mame command).

---

### D16 — Component-local backend-contract types re-home INTO `platform/api/`, never the reverse (Phase 4 Slice 4)

**Decision:** when a Phase-4 wrapper needs a backend-contract type that today
lives component-local (e.g. `AvailableCore` in `CatalogCoreCard`,
`CoreOptionsSnapshot` in `CoreOptionsPanel`, `InstallResult` /
`ControllerDeviceDescriptor` / `AnalogSticksInfo` in their one consumer), the
canonical type is **defined in the api module** and the consumer imports it
back — the type never gets imported by the api module from a component file.

**Why:** the enforced six-zone boundary lint forbids `platform→components`
imports, and `platform/api/` is under `platform/`. So an api module physically
cannot `import type { AvailableCore } from "../components/CatalogCoreCard"` —
that's a lint error. The dependency must point the other way
(`components→platform/api`, which is allowed and already how components reach
the rest of platform). This is the boundary expressing itself at the type
level: backend-contract shapes belong to the api layer, not to whichever
component happened to declare them first. D14 already said "the api layer owns
the backend-contract type" — D16 is the corollary that makes it mechanically
forced rather than stylistic.

**How to apply:** (1) single-consumer shape → delete the local def, define it in
the api module, `import type` it back into the consumer. (2) Multi-consumer
shape-divergent → generic wrapper with a canonical default defined in the api
module (D14); each call site keeps its local view via the type arg, so the
local defs stay (they're genuinely different views, not duplicates). (3) A type
family too heavy to relocate AND only needed as an opaque forwarded blob (the
analog `routing` cluster) → make the wrapper **generic on that param** (`routing:
R`) so the caller's type flows through without the api module ever naming it.
Never reach for `any` to dodge the boundary.

---

### D17 — Tauri events are a second backend-contract surface; corral them like invoke (Phase 4.5)

**Decision:** raw `listen` / `emit` / `once` from `@tauri-apps/api/event` are
banned outside `platform/api/`, exactly like raw `invoke`. The one allowed
module is `platform/api/eventsApi.ts`, which owns: an `OA_EVENTS` const registry
(every `oa://…` channel string, keyed camelCase — the single source of truth for
event names), the moved `listenScoped` (auto-cleanup), `listenTo` (manual
lifecycle, returns the UnlistenFn), and `emitEvent`. `platform/lib/eventListener`
re-exports `listenScoped` for back-compat so existing import paths don't churn.

**Why:** the invoke ban closed command-name coupling but left the symmetric
hole — a theme could still hard-wire to an event-name string (and one did:
`routes/retroverse/GameDetailPanel` emitted `"oa://toast"` directly). Event names
are a backend contract just like command names; a theme binding to one is the
same coupling. Closing it makes "platform and theme can't be re-coupled" true on
both channels, not just invoke.

**How to apply:** subscribe via `listenScoped(OA_EVENTS.x, handler)` (scoped) or
`listenTo(OA_EVENTS.x, handler)` (manual unlisten); publish via
`emitEvent(OA_EVENTS.x, payload)`. Payloads stay generic on `<T>` (each call site
declares the shape it reads — same convention as the invoke wrappers' generic
getters; no per-event payload types forced). **Type-only** imports from
`@tauri-apps/api/event` (`type UnlistenFn`, `type EventCallback`) stay allowed —
only the three value imports are restricted. The `src/platform/api/**` override
that exempts the invoke ban covers this rule too (same `no-restricted-imports`).

---

### D18 — Nav is verb-based + user-remappable; button meanings are a per-user contract, not per-theme (Phase 3 principle)

**Decision:** frontend navigation is modeled as a fixed vocabulary of semantic
verbs (`Confirm`, `Back`, `Up`/`Down`/`Left`/`Right`, `NextSection`/`PrevSection`,
`OpenQuickSettings`, `Menu`, room for `Search`/`Favorite`/`Page`). The 5 Phase-3
nav primitives AND the HintBar consume **verbs**, never raw buttons. A
physical-input→verb indirection layer (a `navBindings` config, OA-wide tier) is
built in Phase 3 so input is **user-remappable in Settings** (gamepad + keyboard).
The remap Settings UI is a follow-on slice after the toy-theme gate, and MUST
ship a "Reset to defaults" button. Defaults = the existing operator-locked
controller-nav spec.

**Why:**
- *Two separate binding layers, don't conflate them.* Gameplay bindings
  (physical pad → emulated console inputs) are already user-remappable
  (`SystemBindingsEditor`, per-system, shipped). This decision is about the
  SHELL-nav layer, which was hardcoded.
- *Verbs, not motions.* Some movement is structural ("cycle tab" is meaningless
  in a tab-less theme; "move left" only means something inside a layout). Remap
  the stable verbs; let layout-specific motion stay abstract.
- *Per-USER, not per-theme — that's the key.* Earlier in the arc we worried that
  letting THEMES remap button meanings would fragment muscle memory across
  themes. Per-user remapping is the opposite: one config applied identically to
  every theme → full user control AND cross-theme consistency. So the two goals
  stop fighting. Themes restyle hints + choose layouts; they never redefine
  `Back`. (Refines the 2026-05 operator-locked nav spec: that spec becomes the
  *default* map, and the system gains user remapping on top — not a contradiction.)
- *Accessibility is the headline win* — rebind for reachability/preference;
  keyboard parity required, not gamepad-only.
- *The HintBar falls out for free* — once it renders glyphs from the current
  input→verb map, remapping updates every on-screen hint automatically (this also
  settles the "how buttons look" theming question: hints are data-driven).

**How to apply:** build the primitives verb-native from day one (retrofitting the
indirection after they're written is far more expensive). Persist `navBindings`
OA-wide via a `platform/api/` wrapper. The remap UI lives in the OA-wide
Input/Controls settings surface; design for conflict validation (no Confirm==Back
deadlock), directional (D-pad/stick) remap, keyboard parity, a guaranteed
always-reachable escape hatch, and the "Reset to defaults" affordance.

---

### D19 — Per-system theming is a Retroverse feature, NOT a substrate contract (vision correction)

**Decision (2026-06-10, design conversation):** the "each console is its own
place" per-system identity — TG-16 orange/cream, Vectrex phosphor green, VB
red-on-black, per-system SFX/backgrounds/boot (the shipped Per-System UI
Stage 1) — is a feature **of the Retroverse theme specifically**. It is **not**
a cross-cutting axis every theme must honor, and the substrate must **not**
elevate it to a platform-mandatory contract. The substrate's whole purpose is
**swappable whole-shells, BigBox-style** (Retroverse / Wheel / Cabinet / …),
each free to treat per-system identity however it likes — heavily, lightly, or
not at all.

**Why:** the operator corrected an assumption the plan had quietly baked in
(it treated per-system theming as substrate-level — see plan §6 Phase 3's
palette pillar framing). The original idea was always swappable shells like
BigBox; per-system "worlds" were only ever Retroverse's take. Conflating the
two over-constrains every other theme and misdirects ARC-1 investment.

**How to apply:**
- **Per-system DATA stays platform-provided** (`palette.json`, accent, era art
  are *factual* — TG-16 simply *is* orange/cream). **Consuming** it is a theme's
  choice. The asset resolver + palette injection remain useful platform plumbing
  any theme MAY opt into.
- **The token contract ([D-tokens, Phase 3]) is theme-first.** The whole-shell's
  look is the primary token surface; per-system tokens are an **optional
  sub-cascade** a theme opts into, not the center of gravity.
- **Cascade precedence ([plan §6 Phase 3 #5]) stops being a conflict.** It's not
  "theme vs per-system fighting" — it's "a theme optionally consumes per-system
  data, and may override it." The outer-layer cascade (active-theme/<system> →
  active-theme/_shared → per-system-UI registry → engine `_baseline`) still
  holds; it's just low-stakes plumbing now.
- Don't write any platform code that *requires* a theme to render per-system
  identity. A system-agnostic theme (one visual language across all systems) is
  a first-class valid theme.

---

### D20 — Kiosk/cabinet capabilities are PLATFORM features (engine-owned, theme-opt-in), deferred — but two seams reserved in ARC 1

**Decision (2026-06-10, design conversation):** attract mode, CRT/shader UI
chrome, and multi-monitor surfaces (marquee / manuals / second-controls) are
**platform capabilities a shell toggles on/off**, engine-owned and theme-opt-in
via the manifest's `required_engine_capabilities` field — **not** features each
theme implements. They are **out of scope for a good while** (ARC 2-3). Of that
list, only the seams that would be expensive to retrofit are reserved in ARC 1;
everything else waits.

**Why:** the operator framed these correctly as platform settings/features that
ride above any shell. The manifest already anticipated this (the
`required_engine_capabilities` field exists in the plan). Deferring the features
is cheap; deferring the *seams* for multi-surface would not be.

**How to apply — what gets plumbed now vs later:**
- **CRT / shaders → nothing now.** Shaders are isolated in the render layer
  (the CrtLite/preset pipeline). The "UI-chrome shader any shell opts into" is
  ARC 2. Only discipline required: themes never own the render pipeline directly.
- **Attract mode → no new code, just framing.** Attract = "platform preempts the
  theme's surface when idle, then resumes it" — the *same* lifecycle as the
  engine takeover (F12 fullscreen) we already build in Phase 1. **Seam reserved:**
  write the theme-host lifecycle as a *general* "platform can preempt + restore
  the theme" pattern, not an F12-special-case. Attract then slots in for free.
- **Multi-monitor (marquee / manuals / second-controls) → seam reserved.** The
  only genuinely invasive retrofit. **Seam:** the manifest declares **named
  surfaces** a theme provides layouts for; ARC 1 supports exactly one — `main`.
  The theme entry-component contract is written surface-aware ("render surface
  X") rather than single-surface-hardcoded. When marquee/manual/control-panel
  surfaces land, existing themes just declare more. Manifest field + one line in
  the SDK contract — near-free now, expensive later.
- Everything in this list stays a **platform setting** (engine renders the
  capability; theme opts in via manifest), consistent with the three-tier
  settings split.

---

### D21 — Nav verb layer: implementation shape (Phase 3 S1)

**Decision (2026-06-10, `feat/theming-nav-foundation`):** the build choices made
turning D18 into code, locked with operator sign-off before writing.

1. **The raw `NavEvent` bus (`gamepad.ts`) stays raw — verb resolution happens in
   the consumer, not the producer.** `gamepad.ts` keeps emitting physical
   button/direction events on `onNavEvent`, because **non-focus consumers depend
   on raw buttons**: the engine-summon chord (`engineSurface.ts`, Select+Start),
   the boot-animation skip (`SystemBootAnimation`, any-input), and RetroverseShell's
   global L1/R1 tab cycling. `focus.ts` resolves button→verb at dispatch time via
   `navBindings`; the keyboard path resolves key→verb in its own listener. Moving
   resolution into the producer would have broken those raw consumers.
   *Constraint for future work:* don't make `gamepad.ts` emit verbs — add a verb
   layer above it instead.

2. **Focus-group callback names are kept (`onActivate`/`onCancel`/`onSecondary`/
   `onTertiary`/`onStart`/`onShoulderL`/`onShoulderR`), mapped from verbs in
   `dispatchVerb`.** Renaming them to verb names (`onConfirm`/…) would churn ~15
   consumer files for zero behavior gain; the verb-nativeness lives in the routing
   layer + the HintBar + the NEW primitives (which DO expose verb-named props).
   The callback names are an internal mapping detail documented at the dispatch site.

3. **A/B swap is a resolve-time overlay, not a stored binding.** `setSwapAB` flips
   a signal; `resolveButtonVerb`/`buttonForVerb` swap A↔B at lookup. This keeps the
   swap orthogonal to (future) remapped bindings and avoids a dual-write between the
   existing `controllerNavSwapAB` setting and `nav_bindings.json` — the setting
   stays the single persisted source and is re-applied via the App.tsx effect.

4. **`nav_bindings.json` is an opaque blob to the backend.** The Rust commands
   round-trip a `serde_json::Value` verbatim; the binding shape + validation live
   in TS (`navBindings.ts`'s `normalize`). Mirrors the `audio.json` pattern; keeps
   the backend dumb and the contract frontend-owned.

5. **Keyboard scope = arrow-key directional nav only in S1** (source `"dpad"`, so
   per-source `onDirection` handlers treat it like the pad). Confirm works natively
   on focusable buttons; Enter/Back/Esc keyboard verbs wait for the remap follow-on,
   which must audit native-control + existing-global-handler (F12/Ctrl-B/in-game
   Esc) coexistence. The `keyboard` channel exists in the schema now so the
   follow-on is additive.

**Why record this:** the producer-stays-raw constraint (1) and the swap-as-overlay
choice (3) are the two things a later contributor could most easily undo by
"cleaning up," reintroducing the exact coupling/dual-write this avoided.

---

### D22 — Walking-skeleton implementation shape (Phase 3 S2)

**Decision (2026-06-10, `feat/theming-walking-skeleton`):** the build choices made
turning the S2 design into code, locked with operator sign-off (four AskUserQuestion
answers, all the recommended path) before writing.

1. **The theme host context is a platform SDK contract — relocated, move+re-export.**
   `ThemeContextValue` / `ThemeProvider` / `useTheme` moved
   `routes/retroverse/context.tsx` → `platform/theme/host.tsx`; the old path
   re-exports for back-compat (D15-style, ~11 Retroverse importers unchanged). A
   theme can't import another theme (`routes/retroverse/` is Retroverse's private
   tree), so the *host services every theme needs* (launch / saves / info /
   favorites…) must live in platform. App.tsx stays the provider that wires real
   behaviour in. Stores still come via `usePlatform()` (D11); the host context
   carries the app-specific **handlers**.

2. **Platform owns the active-theme MACHINERY; App INJECTS the concrete themes.**
   `platform/theme/registry.ts` owns the `activeThemeId` signal, the boot seed, the
   picker's `{id,name}` list, and `setActiveTheme` (persist→restart). It does NOT
   import any concrete theme — `platform ↛ themes` is forbidden. The concrete list
   lives in `themes/index.ts` (`BUILTIN_THEMES`, first = default = Retroverse); App
   calls `registerThemes(BUILTIN_THEMES)` at boot. Same inversion-avoiding pattern as
   `platform/libraryAdmin.ts` + `platform/engineSurface.ts` (D13). *Constraint:* never
   make platform import a theme — inject from App.

3. **`activeThemeId` persists on `LibraryPrefs.active_theme_id`** (serde-default
   `Option<String>`), the OA-wide install-level prefs bag read at boot before any
   theme mounts. Switch = read-merge-write that one field (preserving the struct's
   non-defaulted fields) then restart. App gates the theme mount on
   `activeThemeResolved()` so first paint shows the persisted theme, no default flash.

4. **Restart = a new `restart_app` Rust command via Tauri 2 `AppHandle::restart()`** —
   no new plugin/dependency; mirrors `quit_app`'s temp-sweep + session-persist cleanup
   before relaunch. D5 (swap requires restart) realized here.

5. **`EngineSummonIcon` re-homed `engine/` → `platform/components/` (applies D12).**
   It's a leaf THEMES must mount (plan D3) but a theme can't import `engine/`; it
   depends only on `platform/engineSurface`, so the lowest consuming layer (platform)
   is its home. Both themes mount it; neither crosses the boundary.

6. **`themePreempted()` is the general preempt/restore seam (D20a).** Lives in
   `platform/theme/host.tsx` = `engineSurfaceOpen()` today; RetroverseShell's L1/R1
   tab-cycle gate switched from `engineSurfaceOpen()` to `themePreempted()`. Attract
   mode (ARC 2-3) reuses the same predicate with no theme-side change. *Constraint:*
   new preemptors (attract) OR the signal feeding it; don't re-hardcode
   `engineSurfaceOpen()` in theme code.

7. **`surfaces` field added to `ThemeManifest` + `ThemeEntryProps.surface` (D20b
   seam).** The entry is surface-aware from theme #1; ARC 1 honors exactly `"main"`.
   Multi-monitor surfaces widen the union later — additive, not a rewrite.

8. **Retroverse is a THIN wrapper (S2 sign-off).** `themes/retroverse/index.tsx`
   renders the existing `<RetroverseShell />`; `layout/retroverse/` +
   `routes/retroverse/` stay put (full move is Phase 6). The eslint `themes ↛ routes`
   and `themes ↛ layout` zones carry an `except: ['./retroverse']` to allow exactly
   that pointer; every OTHER theme (Wheel) is platform-only.

9. **The Wheel is system-AGNOSTIC by choice (D19).** A flat coverflow over the whole
   library, one cover per identity, no per-system grouping — proving per-system
   identity is Retroverse's take, not a substrate requirement. Built on the `ListNav`
   primitive (horizontal, controlled index) + `usePlatform` stores + `useMedia`
   covers; the cinematic layer (attract/CRT/ceremony) is honestly deferred to ARC 2-3.

**New lint zones (all green, `themes↛engine` probe-verified):** `platform↛themes`,
`engine↛themes`, `themes↛engine`, `themes↛routes` (except retroverse),
`themes↛layout` (except retroverse), `themes↛components`.

**Why record this:** items (2) the platform-owns-machinery/App-injects inversion and
(8) the retroverse lint `except` are the two a later contributor could most easily
"simplify" wrong — by importing a theme into platform, or by dropping the except and
forcing the Retroverse file-move early (that's Phase 6, deliberately not S2).

---

### D23 — Token layer: scope-not-namespace + motion stays ARC 2 (Phase 3 S3)

**Decision (2026-06-10, `feat/theming-token-layer`):** the build choices for the S3
design-token contract, locked with operator sign-off (three AskUserQuestion answers,
all recommended) + a prior research round on BigBox theming.

1. **Formalize, don't reinvent.** OA already had a CSS-variable token system
   (`index.css` `@theme` + `:root`: palette / `--font-display` / `--layout-*` /
   `--motion-*`). S3 adds a typed `ThemeTokens` + a `TOKEN_VAR` map (key → CSS var) +
   `themeTokensToCssVars()` (`platform/theme/tokens.ts`). The values stay CSS vars.

2. **Override = scoped inline-var injection; D2 is STRUCTURAL.** A theme declares
   `tokens?: Partial<ThemeTokens>` on its `ThemePackage`; App.tsx injects them as CSS
   custom properties on the S2 **theme-mount wrapper** (the `isolate` div). The engine
   surface is a *sibling* of that wrapper, not a descendant, so scoped theme tokens
   cannot reach it — engine territory always reads the `:root` defaults. That sibling
   scope IS the D2 "a theme can't wreck Settings" guarantee; no defensive engine
   re-baseline needed (operator picked the minimal option). The contract rule that
   seals it (THEME_CONTRACT.md §4): a theme styles only via its `tokens` + its own
   scoped classes, and **never** writes a global `:root`/`<style>` token override. The
   S4 validator enforces it.

3. **Same token NAMES, different SCOPE — no namespace split.** Engine and theme both
   read e.g. `--color-oa-bg`; the theme just overrides it locally. Splitting into
   `--oa-engine-*` vs `--oa-theme-*` was considered and rejected — the sibling scope
   already isolates them, and one vocabulary keeps the system legible.

4. **`focusRing` formalized** as `--oa-focus-ring` (default `var(--color-system-accent)`,
   so it stays per-system-aware via the `[data-system]` cascade; theme-overridable to a
   fixed color). The `[data-oa-focus]` outline now consumes it.

5. **A11y/motion baseline is NOT theme-overridable.** A global
   `prefers-reduced-motion` reset (collapse `--motion-*` + the standard
   transition/animation neutralizer) sits OUTSIDE the token contract — every theme
   inherits it; a theme can't opt its users out of reduced motion.

6. **Motion stays ARC 2 (operator decision after BigBox research).** Researched what
   BigBox themes actually do: WPF/XAML with a full animation engine (storyboards /
   transforms / easing / triggers), view transitions (`TransitionPresenter`), video
   snaps as backgrounds, attract mode, startup/pause/exit themes, rich data binding to
   all metadata + media, multiple view types, a visual Theme Creator. **Conclusion: the
   cinematic/motion axis is the heart of ARC 2-3, NOT the token layer.** A noted insight
   for ARC-2 planning: on our *web* stack much of that motion (slide/fade/scale/easing,
   `<video>` backgrounds, view crossfades, a basic attract loop) is achievable
   *declaratively* via CSS / Web Animations — it does NOT all require the ARC-2 Rhai
   scripting engine; Rhai is for theme-*authored* custom logic, WGSL for shader chrome.
   So ARC 2 can likely ship a declarative motion/transition layer before scripting. The
   operator chose to keep S3 strictly static regardless; THEME_CONTRACT.md §4 documents
   motion as a **reserved** token category so it slots in without a contract break.

7. **CoverFlow re-skinned minimally** (operator picked "prove the mechanism"): a cool
   steel-blue/cyan token set (bg/accent) vs Retroverse's warm default — same component,
   different tokens, visibly different shell. Retroverse ships **no** tokens (pure
   `:root`).

**New artifact:** [THEME_CONTRACT.md](THEME_CONTRACT.md) — the theme-facing peer of
SURFACES.md (token set + verb vocab + manifest schema + surfaces + the reserved-motion
note + what the S4 validator checks).

**Why record this:** (2) the structural-D2 / no-global-override rule and (6) the
motion-is-ARC-2 boundary are the two a later contributor could most easily erode — by
"helpfully" letting a theme ship global CSS, or by sneaking animation into the token
layer because the web stack makes it easy.

---

### D24 — Manifest validator: declarative-surface gate + Vitest CI + bare-as-fixture (Phase 3 S4)

**Decision (2026-06-10, `feat/theming-manifest-validator`):** the build choices for the
S4 versioned-manifest + load-time validator, locked with operator sign-off (four
AskUserQuestion answers, all the recommended path) before writing.

1. **The validator checks the DECLARATIVE surface only; it's a pure function.**
   `validateTheme(pkg)` (`platform/theme/validate.ts`) reads a ThemePackage's `manifest`
   + typed `tokens` and returns `{themeId, ok, errors, warnings}`; it never throws, never
   touches the DOM. It covers THEME_CONTRACT.md §6's manifest / `schema_version` /
   `surfaces` / `required_engine_capabilities` / token-key / token-value checks. The
   token-key check (keys ∈ `TOKEN_VAR`) IS the data half of the "no engine-var override"
   rule: a theme can only set keys mapping to known sibling-scoped CSS vars, so even a
   hostile token VALUE can't escape the mount.

2. **The "no global `:root` override" rule is NOT runtime-enforced — by choice.** A
   theme's entry is an opaque Solid component; a `<style>:root{}`/`document.head`/global-
   CSS-import bypass is invisible to a package-object validator. The real protection is
   STRUCTURAL (the S3 sibling-scope mount — scoped tokens physically can't reach engine
   territory) + the ESLint layer boundary (theme ↛ engine). Static source inspection of
   the bypass is a deferred **Phase-5 / untrusted-author** concern; built-ins are
   reviewed. Documented as a known gap in THEME_CONTRACT.md §6. *Constraint:* don't
   present the validator as a security boundary against malicious global CSS — it isn't;
   the sibling-scope is.

3. **Two run sites: registration-time (dev-loud) + a Vitest CI gate (the hard one).**
   `registerThemes()` validates each package, **excludes invalid ones from the valid
   set** (so they can't be picked or activated), logs errors always (an operator may
   wonder why a theme vanished; the log is captured) and warnings in DEV only. The
   authoritative drift-stopper is the **Vitest** suite — which required standing up the
   frontend's **first test runner** (there was none; CI was lint + build + `cargo test`,
   and the manifests are TS objects with no Rust visibility per D6, so the gate HAD to be
   TS). Added `vitest` + `jsdom` + `vitest.config.ts` (reuses `vite-plugin-solid` + the
   `@oa/platform` alias) + an `npm run test` CI step. An `overrides: { vite }` pin
   dedupes vitest's nested vite so the Solid-plugin types don't clash. *Constraint:* keep
   one vite — drop the override and the dual-vite type error returns.

4. **The validator test lives in `themes/`, not `platform/`.** Validating the REAL
   bundled themes means importing them, and the ESLint boundary forbids `platform ↛
   themes`. `themes/` is the one layer allowed to see both the concrete themes and the
   platform validator, so `themes/builtin-themes.test.ts` is the correct home for the
   cross-layer gate; the pure unit tests (crafted-invalid fixtures, no theme import) stay
   in `platform/theme/validate.test.ts`.

5. **`bare` is a REAL theme in the picker, doubling as the fixture.** `themes/bare/`
   (added to `BUILTIN_THEMES`) is the minimal valid whole-shell — a plain ListNav of
   games + launch-on-Confirm + the `EngineSummonIcon`, **no tokens**, ~110 LOC, system-
   agnostic. It's operator-selectable (the north-star "low floor" made switchable +
   dogfooded end-to-end: browse / launch / restart) AND the canonical fixture the CI gate
   validates. One artifact, both jobs — so the lowest-floor reference can never silently
   drift from what the validator accepts.

6. **`schema_version` is a supported-SET (`{1}`), not a min/max range.** `MISSING_FIELD`
   if absent/non-number; `UNSUPPORTED_SCHEMA_VERSION` otherwise, with the message
   distinguishing "targets a newer schema than this build (up to N) — update OA" (declared
   > max known) from "unsupported schema". A range + migrations waits for the first
   breaking schema bump.

7. **Fallback loudness = toast + console (ARC 1).** When the persisted `active_theme_id`
   isn't a valid choice (renamed/removed like wheel→coverflow, or now-invalid),
   `activeTheme()` already falls back to the default (first valid); `initActiveTheme()`
   now ALSO raises a `warn` toast naming the fallback + pointing at Settings → Themes. The
   Phase-5 §6 **persistent banner** is deferred to when real on-disk themes can fail in
   the field — in ARC 1 a built-in can only fail via a CI-caught dev bug, so a sticky
   banner would be premature. The default theme is a **CI-guaranteed-valid invariant**
   (the Vitest gate asserts Retroverse validates), so there is always ≥1 valid theme.

**Why record this:** (2) the "validator is not the `:root` boundary" framing and (3) the
one-vite override are the two a later contributor could most easily get wrong — by adding
a runtime CSS-scan and calling the gap closed, or by "cleaning up" the override and
reintroducing the dual-vite type clash.

---

### D25 — Resolver theme tier: `assets/themes/<id>/` home + 4-tier cascade + ambient themeId (Phase 3 S5.1)

**Decision (2026-06-10, `feat/theming-s5-1-resolver-theme-tier`):** build choices for
adding the theme tier to the per-system **asset** + **ui-sound** resolvers — the first of
the five S5 micro-slices (operator chose per-sub-area slicing; this is S5.1, the resolver
cascade). The S5 design forks were signed off via AskUserQuestion before code.

1. **Theme override assets home under `<exe_dir>/assets/themes/<themeId>/system-ui/
   <systemId>/<category>/` — NOT the Phase-5 `<exe_dir>/themes/<id>/` loader path.**
   ARC-1 themes are JS-bundled (D6 — no on-disk theme folder yet), so homing theme
   *asset* overrides under the existing `assets/` tree makes the theme tier
   **operator-droppable today** without waiting on the Phase-5 loose-folder loader, and
   mirrors the `assets/system-ui/<systemId>/<category>/` layout operators already know.
   *Constraint:* when Phase 5 adds the loader, don't "move" these into the themes/ folder —
   this stays the operator-override home; a bundled theme's own assets can be discovered
   additionally.

2. **A single 4-tier cascade with a theme `_baseline`, shared across both resolvers.**
   Order: *(ui-sound only)* operator override → `theme/<system>` → `theme/_baseline` →
   `system/<system>` → `system/_baseline` → null (background omits the operator tier).
   The theme **`_baseline`** (theme-wide, not per-system) lets a system-agnostic theme
   (D19) ship ONE backdrop/cue for the whole library instead of 45 per-system copies —
   symmetric with the platform tier's per-system+`_baseline` shape. The ordered bases come
   from one shared `candidate_asset_bases()` in `system_ui_assets.rs` (both resolvers are
   `oa-shell` modules, so the cascade logic isn't duplicated across files); each resolver
   supplies its own `category` + extension list.

3. **Operator override stays ABOVE the theme tier (ui-sound).** An explicit per-system
   file a user wired via the per-system audio UI is the most specific intent and beats a
   theme's sound; the theme sits between operator-override and the platform bundles.

4. **`themeId` is resolved AMBIENTLY in the dispatchers, not threaded through consumers.**
   The api wrappers (`resolveBackgroundAsset` / `resolveUiSound`) take `themeId` explicitly
   (pure typed pass-through, D14 convention). The dispatcher internals
   (`lib/audio.ts::dispatchUiSound`, `SystemBackground`'s `resolveBackgroundUrl`) read
   `activeThemeId()` and pass it down — so every *consumer* call site (grid nav, boot
   animation, the background component) is **unchanged**. The active theme is ambient app
   state; consumers shouldn't have to know about it. (`platform/lib` + `platform/components`
   reading `platform/theme/registry` is intra-platform/allowed; runtime read, no init cycle.)

**Why record this:** (1) the `assets/themes/` home vs the Phase-5 loader path and (4)
ambient-vs-threaded are the two a later contributor could most easily undo — by relocating
theme assets into the not-yet-existent themes/ loader folder, or by threading `themeId`
through every `playSystemUiSound` / background call instead of resolving it once.

---

### D26 — Palette substrate: typed single-source map + runtime-derived baseline + scoped `perSystemTokens` (Phase 3 S5.2)

**Decision (2026-06-11, `feat/theming-s5-2-palette-substrate`):** build choices for the
palette substrate, with operator sign-off on the data-home fork (AskUserQuestion) before code.

1. **Per-system palette lives as a typed TS map, NOT `config/systems/<id>/palette.json` +
   a build step.** `platform/themes/systemPalettes.ts` is the single source of truth
   (`SYSTEM_PALETTES: Record<SystemId, SystemPalette>`); the plan's literal §6 wording
   (`palette.json` + a `systems.generated.css` generator) was **rejected** because per-system
   palette is **frontend-only data with no Rust reader** — a `config/*.json` + cross-language
   generator would add a generated file that can drift, for zero benefit. The typed map is
   greppable, validator-readable, Theme-Studio-round-trippable, and `Record<SystemId,…>`
   enforces a palette per system at compile time (the parity the old `systems.css` comment
   asked for by hand). *Constraint:* don't "promote" this to `config/*.json` unless Rust
   actually needs to read per-system palette.

2. **`systems.css` retired; the baseline `[data-system]` CSS is DERIVED from the map at boot.**
   `ensureSystemPaletteBaseline()` injects a `<style id="oa-system-palettes-baseline">` into
   `document.head` from `index.tsx` **before first render** (no flash) — the runtime
   equivalent of the deleted `@import "./themes/systems.css"`. Idempotent + DOM-guarded so it's
   a no-op in non-DOM (test) contexts; it is **never** called at module-load (only the explicit
   entry call), so importing the module has no side effect. The baseline is injected
   **globally** (engine + theme both read `[data-system]`), exactly as `systems.css` was.
   `glow` is **derived** (accent at 0.35 alpha — the invariant every baseline followed); only
   `accent` + `soft` are authored per system.

3. **The per-theme override (`perSystemTokens`) is a SCOPED `<style>`, not inline vars.** A
   per-system override must target *descendant* `[data-system]` elements, which an inline
   `style=` on the mount cannot do (inline vars apply to the element, not its descendants'
   attribute selectors). So App.tsx renders `<style>` of
   `.oa-theme-mount [data-system="<id>"]{…}` inside the theme mount. Specificity (class +
   attribute = 0,2,0) beats the global baseline (0,1,0) **inside** the mount; engine territory
   is a **sibling** of the mount, never matched — the same structural D2 guarantee the S3 token
   scope uses. This is a *scoped* `<style>`, NOT the D24-forbidden *global* `:root`/`<style>`
   override — the prefix is what makes it legal. *Constraint:* keep the `.oa-theme-mount`
   prefix; an unprefixed `[data-system]` rule from a theme WOULD leak to engine territory.

4. **`bare` is the override seam's live consumer — reframed as the substrate TEST BED
   (operator call).** The only shipping theme that renders `data-system` is Retroverse (the
   default, which we don't recolor); CoverFlow is system-agnostic by design (D19). Rather than
   leave the seam unconsumed (the S5.1-background shape) OR distort a shipping theme, the
   operator reframed `bare` from "purest minimal floor" to **test bed** — "eventually a proper
   list-view theme grows from here; bare is where new substrate capabilities get their first
   real consumer." So `bare` now renders a per-system accent dot (`data-system`) and ships a
   scoped `perSystemTokens` demo (NES→cyan, PSX→magenta), making the D19 sub-cascade + D2
   sibling-scope visible (bare shows the demo colours; engine territory keeps the baseline).
   `bare.tokens` stays undefined, so its "no design-token overrides" fixture role is intact;
   the demo just exercises the *separate* `perSystemTokens` field. The **baseline extraction**
   remains S5.2's primary live deliverable (accents byte-identical after retiring `systems.css`).

**Why record this:** (1) the typed-map-not-config-json home and (3) the scoped-`<style>`
mechanism (incl. *why* inline vars can't do it, and the `.oa-theme-mount` prefix being
load-bearing for D2) are the two a later contributor could most easily get wrong — by
"consolidating" palette into `config/*.json`, or by dropping the scope prefix and leaking a
theme's per-system override into engine territory.

---

### D27 — Glyph-set seam: loose manifest field + App-bridged active set + unknown-is-a-warning (Phase 3 S5.3)

**Decision (2026-06-11, `feat/theming-s5-3-glyph-set`):** build choices for making the
existing verb→glyph indirection (S1) theme-choosable. Scope-call #4 = "seam + one alternate
set; defer the picker + auto-detect."

1. **`manifest.glyph_set` is a loose `string`, not a `GlyphSetId` union.** Like `routes` and
   `required_engine_capabilities`, the manifest stays a plain-data shape that doesn't import
   the nav layer; the validator checks the value against the runtime `GLYPH_SETS` registry.
   This keeps `platform/theme/manifest.ts` decoupled from `platform/nav` (the union lives in
   `glyphs.ts`, where glyph sets belong).

2. **The active glyph set is App-bridged into nav, not read by nav from the theme.** `glyphs.ts`
   owns an `activeGlyphSet()` signal + `setActiveGlyphSetId()`; App.tsx bridges
   `activeTheme()?.manifest.glyph_set` into it via a `createEffect` — the SAME pattern as the
   S1 settings→nav bridges (`setSwapAB`, `setPerSystemUiEnabled`). nav stays a generic leaf
   (it does not import the theme registry); App is the one place that knows the active theme.
   *Constraint:* don't make `glyphs.ts`/HintBar reach into `platform/theme/registry` — bridge
   from App. (A future user picker / controller auto-detect calls the same setter; the seam is
   "set the active glyph set," source-agnostic.)

3. **Unknown `glyph_set` is a WARNING, not an error.** An error would EXCLUDE the theme from
   the picker (unselectable). Disqualifying a whole theme over a cosmetic hint-glyph typo is
   disproportionate — `setActiveGlyphSetId` already falls back to xbox, so a bad value renders
   fine (just default glyphs). Contrast `surfaces` / `required_engine_capabilities` (errors):
   those gate whether the theme can *function*; glyph choice never does. *Constraint:* keep it
   a warning — don't "tighten" it to an error.

4. **`bare` is the live consumer** (the test bed, per D26): its manifest declares
   `glyph_set: "playstation"`, so bare visibly paints ✕/◯/□/△ while Retroverse keeps the
   default A. One real theme exercising the seam end-to-end, no shipping-theme distortion.

**Why record this:** (2) the App-bridge-not-nav-reads-theme direction and (3) warning-not-error
are the two a later contributor could most easily undo — by coupling nav to the theme registry
for "convenience," or by making a cosmetic mismatch disqualify a theme.

---

### D28 — Per-theme settings: localStorage namespace + active-id-bound accessor (Phase 3 S5.4)

**Decision (2026-06-11, `feat/theming-s5-4-theme-settings`):** the per-theme settings
namespace (scope-call #9).

1. **One `localStorage` key, keyed by theme id — not Rust `LibraryPrefs`.** `oa.themeSettings`
   → `{ [themeId]: { … } }`. Per-theme prefs are per-install/per-user, exactly like the OA
   settings store (which is also `localStorage`), so they belong in the same frontend tier — no
   Rust round-trip. Survives the restart-based swap (Tauri WebView localStorage persists to
   disk). Backed by a Solid `createStore` so reads are reactive.

2. **`useThemeSettings()` auto-binds the ACTIVE theme's id — that binding IS the collision
   rule.** A theme calls `get(key, fallback)` / `set(key, value)` and **never names a theme
   id**; the hook injects `activeTheme()?.manifest.id`. So a theme physically cannot read or
   clobber another theme's slice (or OA settings, which live under a different key). This is a
   **fourth** namespace alongside the locked OA-wide / per-system / per-game three-tier split —
   distinct keyspace, collision-free by construction, no merging of tiers.

3. **Persistence is best-effort (guarded), values are opaque JSON.** `load`/`persist` are
   try/caught + `typeof localStorage` guarded, so a missing/broken store never breaks the app
   (it degrades to in-memory). Values are generic on the caller's type arg (each theme declares
   what it stored) — same opaque-blob convention as `nav_bindings.json`. *Test note:* the
   Vitest runner ships only a partial `localStorage` stub, so the persistence test installs a
   working in-memory one via `vi.stubGlobal` rather than relying on the env.

4. **Live consumer = `bare`** (the test bed, per D26/D27): a header "Compact" toggle writes
   `themeSettings.bare.compactRows` and the list density reacts + persists across a swap.

**Why record this:** (1) localStorage-not-Rust and (2) the id-bound accessor are the two a
later contributor could undo — by "promoting" theme prefs into `LibraryPrefs` (needless Rust
coupling), or by letting `get`/`set` take an explicit themeId (which would break the
collision-free guarantee — a theme could then read another's slice).

---

### D29 — Primitives slice: carousel-dogfood, reserved-not-stubbed wheel, nav-sound as a callback, background revival (Phase 3 S5.5)

**Decision (2026-06-11, `feat/theming-s5-5-primitives`):** build choices for the arc-closing
primitives slice.

1. **CarouselNav GENERALIZES CoverFlow, then CoverFlow is dogfooded onto it.** The primitive
   owns the windowing / track shift / per-card layout / focus / wheel / click-to-centre — and
   the **late-claim** (claim once items first appear) moved IN from CoverFlow's hand-rolled
   force-claim, since "whole-shell surface mounts after the async theme seed" is a general
   primitive concern, not a CoverFlow quirk. CoverFlow keeps only theme-specific parts (cover
   content render-prop, preload buffer, footer, shared-selection mirror). Dogfooding (not just
   shipping the primitive beside the bespoke code) is what proves the primitive is actually
   sufficient — and deletes the duplicate windowing. *Constraint:* keep CoverFlow ON the
   primitive; if it needs something the primitive lacks, extend the primitive, don't fork back.

2. **WheelNav is RESERVED (typed contract + stub), NOT implemented.** The radial layout has no
   ARC-1 consumer; shipping a half-built radial render would be dead code with geometry bugs no
   theme exercises (the *code-exists-isn't-live* trap). The expensive-to-retrofit part is the
   prop CONTRACT (a future DSL / Theme Studio target it), so S5.5 ships that + a stub that
   renders nothing and warns once. The impl lands when a wheel-using theme does. Same call shape
   as S5.1's background tier and S5.2's override seam: stamp the contract, defer the body.

3. **`onNavSound` is a CALLBACK on the primitives; the dispatcher lives in `platform/themes`.**
   The primitives emit a coarse `NavSoundEvent` and the theme maps it — the nav layer never
   imports the per-system audio machinery (keeps nav a generic leaf, like D27's glyph bridge).
   The engine default `navSoundDispatcher` is in `platform/themes/systemUiSound` (where the
   gating + per-system resolver already live) and is generic on the item type via a `systemIdOf`
   selector, so nav never needs to know an item carries a system. Focus-move sounds fire from a
   `createEffect` tracking ONLY `focusedIndex` (items read `untrack`ed) so a data change doesn't
   ring a spurious move.

4. **The dead `SystemBackground` is REPLACED by a generic `ThemeBackground`, not revived in
   place.** `SystemBackground` was Retroverse-era (accent gradient + `perSystemUiEnabled` gate +
   per-system focus) and was dropped 2026-05-31 for competing with Retroverse's chrome. Reviving
   it as-is would reintroduce that. Instead S5.5 deletes it (zero importers) and ships
   `ThemeBackground` — a theme-opt-in surface (a theme that mounts it WANTS it → no master-toggle
   gate; the backdrop is the theme's own image → no accent gradient) consuming the **S5.1
   background resolver tier**. This gives S5.1's background half its first live consumer
   (CoverFlow mounts it). *Constraint:* a theme that doesn't want a backdrop simply doesn't mount
   it — don't re-add a global mount.

**Why record this:** (1) keep-CoverFlow-on-the-primitive and (2) wheel-reserved-not-half-built
are the two a later contributor could erode — by forking CoverFlow's layout back off the
primitive "to tweak it," or by fleshing out WheelNav before a consumer exists (dead radial code).

---

### D30 — Nav-remap Settings UI: dropdown-per-verb, gamepad-scoped, conflict-by-steal, keyboard escape hatch (Phase 3 D18 follow-on)

**Decision (2026-06-11, `feat/theming-nav-remap-settings`):** the design of the shell-nav remap
Settings surface (the D18 follow-on after the S2 swap gate). This edits the OA-wide `navBindings`
map (built in S1) — the **menu/UI** nav layer, distinct from the per-system **gameplay** bindings
(`SystemBindingsEditor`).

1. **Dropdown-per-verb, NOT "press-to-bind."** Each action verb gets a `<select>` of physical
   buttons. A press-to-bind capture mode was rejected: the gamepad is actively *driving the menu*
   (the nav bus), so capturing a raw press would fight the very input that's navigating the
   Settings screen. Dropdowns are robust across mouse / keyboard / controller-as-cursor and need
   no capture-vs-dispatch arbitration. (A press-to-bind affordance could be added later as an
   enhancement, but the dropdown is the correct accessible baseline.)

2. **Scope = gamepad ACTION/structural verbs only.** Directional movement stays on the D-pad /
   left stick (not per-button remappable in S1). The reserved no-consumer verbs (Search /
   Favorite / Page) are omitted until they do something. The **keyboard** channel (arrows + native
   Enter/Esc) is deliberately **not editable here** — it's the **always-reachable escape hatch**
   D18 requires: a user can never remap themselves into a corner with no way to confirm/back.

3. **Conflict resolution = button-steal, surfaced by re-render.** The map is button-keyed
   (one button → one verb), so `rebindGamepadVerb` (pure, in `navBindings.ts`) clears the verb's
   old button and, if the target button belonged to another verb, transfers it — that verb's row
   re-renders as Unbound. The operator *sees* the conflict resolve. A soft warning flags Confirm/
   Back being unbound (keyboard still covers it). No hard block — the keyboard escape hatch makes
   a deadlock impossible.

4. **Edits the LIVE signal — instant, no restart.** `setNavBindings` updates the reactive
   `navBindings` signal (focus dispatch + HintBar glyphs repaint immediately) and persists to
   `nav_bindings.json`. Unlike the theme swap (D5, restart), a binding change is live. Lives in
   the existing **Controller navigation** card group (Controls), below the A/B-swap toggle (which
   stays a separate resolve-time overlay, D21 — noted in the card so the two don't confuse).

**Why record this:** (1) dropdown-not-press-to-bind (someone may "improve" it into a capture mode
that fights the nav bus) and (2) the keyboard-as-fixed-escape-hatch (someone may make it editable
and reintroduce the deadlock risk) are the two a later contributor could get wrong.

---

### D31 — Retroverse-as-theme: the move was pure relocation; nothing hoisted to platform (Phase 6)

**Decision (2026-06-11, `feat/theming-retroverse-as-theme`):** the build choices for the ARC-1
acceptance gate — moving Retroverse from the S2 thin wrapper (D22.8) into a real theme physically
living under `themes/retroverse/`, consuming only platform, and removing the last two boundary
exceptions.

1. **The reverse-import audit found ZERO files needing to hoist to platform.** The premise of the
   move was that any Retroverse file consumed by a *non-Retroverse* surface is shared and must go
   to `platform/` (D12 leaf-to-lowest-layer), not into the theme. The audit (every importer of all
   11 files, from outside the retroverse trees) found that the S2 / Phase-4 / grab-bag groundwork
   had **already hoisted everything shared** — the host context (→ `platform/theme/host`),
   LeftSidebar / LibraryView / VirtualLibraryGrid / EngineSummonIcon (→ `platform/components`), all
   stores + the typed api. So Phase 6 collapsed to a **pure physical relocation** of
   Retroverse-private files (RetroverseShell + 8 route files + `currentRoute.ts`) plus deleting one
   shim — no platform hoist, no new platform module. That the dogfood needed *no* new sharing is
   itself the proof the platform/theme boundary was drawn correctly in ARCs past.

2. **`currentRoute.ts` is theme-private (moved INTO the theme), per plan §10.** Its only external
   consumer was App.tsx's `__retroverse_debug` DevTools block — itself Retroverse-specific glue, not
   a genuine platform need. So `currentRoute` moved into `themes/retroverse/` and the debug block was
   **deleted** (obsolete: it predated Retroverse's real tab strip, which now reads the signal). App.tsx
   ends coupled to Retroverse only through the sanctioned `registerThemes(BUILTIN_THEMES)` injection
   edge (D22.2). A future dev-console seam belongs in **platform** (theme-agnostic, every shell gets
   it) — queued in PARKING_LOT, not rebuilt against one theme's route model.

3. **`context.tsx` was DELETED, not moved.** It was a pure S2 re-export shim of
   `@oa/platform/theme/host` (the content already lived in platform); Phase 6 repointed its importers
   (App.tsx + RetroverseShell + the 6 pages) directly at the platform host and removed the shim. The
   D15 move+re-export happened in S2; Phase 6 retires the back-compat layer.

4. **Both `except: ['./retroverse']` exceptions removed + probe-verified.** With the files relocated
   and importing only platform + siblings, the `themes↛routes` and `themes↛layout` ESLint zones no
   longer need the carve-out. A throwaway `routes/` + `layout/` import from `themes/retroverse/` was
   confirmed to fire both `import/no-restricted-paths` errors (the old `except` would have allowed
   exactly that), then reverted. **Every theme — Retroverse included — is now platform-only with zero
   exceptions.** This is the ARC-1 acceptance gate: the SDK hosts the flagship with no escapes.

5. **Pure refactor, "indistinguishable" bar.** Zero intended user-visible change; `git mv` preserved
   history on all 9 files. Landed as three green sub-commits (C1 sever shim · C2 relocate + delete
   dead code · C3 drop exceptions + probe), each passing typecheck + lint + vitest(58) + build.

**Why record this:** (1) the no-hoist outcome is the load-bearing finding — a later contributor
extending Retroverse should NOT assume theme files can be shared back into platform ad hoc (the audit
discipline is D12); and (2) the `currentRoute` theme-private home + the deleted dev-console (someone
may "helpfully" re-add a theme-coupled DevTools global in App.tsx — it belongs in platform).

---

### D32 — Per-system LAYOUT variation + view-type library + end-user override become a substrate contract in ARC 2 (expands/supersedes D19)

**Decision (operator, 2026-06-11 — out of the BigBox competitive research,
`features/theming-substrate/BIGBOX_RESEARCH_2026-06-11.md` §3/§8).** The
"each system gets a polished, dedicated home" pillar is promoted from
*colors + assets* to *layout*. The theming substrate gains, **in ARC 2**, a
first-class view/layout capability shaped like BigBox's:

1. **A library of VIEW TYPES** — the distinct screens of the library journey
   (manufacturer-browse, system-browse, game-browse, game-details, …). A
   theme composes these and styles each.
2. **A library of LAYOUT PRIMITIVES per view** — wheel, carousel/coverflow,
   grid, list, custom (the S5.5 primitive set is the seed; `WheelNav` is the
   reserved contract).
3. **Theme-declared per-system layout.** A theme manifest declares, per view,
   which layout to use, and **may vary it per manufacturer/system** (theme's
   curated design: TG-16 → wheel, Lynx → grid, Vectrex → vector-glow list).
   This is the part D19 said the substrate would NOT support — **D19 is
   expanded:** per-system theming becomes a substrate contract, not a
   Retroverse-only feature. (D19's *reasoning* — don't over-build per-system
   machinery before it's needed — held correctly through ARC 1; ARC 2 is when
   it's needed.)
4. **End-user runtime override.** The user can override the active layout per
   system / per view at runtime (BigBox's "pick your view"), and the choice
   **persists**. This is the T2 "mix and match" answer + the explicit
   operator call that overrides are user-facing, not theme-author-only.

**Relationship to existing decisions:**
- **Expands/supersedes D19** (per-system theming = Retroverse-only). Record
  the supersession here; D19 stays in the log as the ARC-1-correct stance.
- Builds on the shipped cascades: **S5.1** (per-system *assets*) + **S5.2**
  (per-system *palette* via `perSystemTokens`) already resolve-by-active-
  system; D32 extends that same "resolve by active system" plumbing up to
  *layout/primitive* choice. Incremental seam, not a rebuild.
- Sits under **D18** (nav verbs are a per-USER contract): a user's layout
  override is the same philosophy applied to layout — user agency over the
  theme's defaults, persisted per-user.
- **T3 unchanged:** Theme Studio (ARC 3) stays sequenced after ARC 2 — you
  need the layout/motion/shader capabilities to exist before a visual editor
  for them is meaningful.

**Why:** this is the operator's headline takeaway from the BigBox research —
the single feature that makes "each system its own home" literally true at
the layout level (not just recolor + reskin), and the mix-and-match richness
that keeps people on BigBox. The competing model (theme branches on
`SystemId` in code, no engine support) reproduces BigBox's XAML-author
burden, which OA's declarative-first north star exists to avoid.

**How to apply / open for next pass (the "how" is deliberately deferred —
this records the "what"):**
- Manifest schema: how a theme declares `views[].layout` + `per_system`
  overrides (extends the `theme.toml` / `ThemeTokens` surface; validator
  gate per S4 pattern).
- Persistence: a per-user `(theme_id, system_id, view) → layout` override
  store (new namespace; parallels D28 per-theme settings + the nav-bindings
  persistence). Survives the restart swap.
- Resolution order: theme per-system default → engine default, with the
  user override on top (cascade shape mirrors S5.1/S5.2).
- Reconcile with **D20** (kiosk/cabinet capabilities) and the **per-system-ui**
  arc — per-system layout is the natural merge point for Per-System UI
  Stage 2/3, which paused for content.
- Scope guard: this is ARC 2 work; ARC 1 finishes on its current Phase-5
  (`.oatheme` loader) line first.

### D33 — Per-system UI is a shared substrate capability (D32 reaffirmed over D19), with consumption made strictly theme-opt-in

**Date:** 2026-06-15. **Decision:** operator reaffirmed the **D32 model** over the
older D19 ("per-system theming is Retroverse-only") after an audit of where the
shipped Per-System UI machinery actually lives. Per-system UI (backgrounds, boot
animations, tile flourishes, SFX, palettes, and the ARC-2 layout/view capability)
stays a **platform-owned capability that any theme opts into** — NOT pulled back
into `themes/retroverse/`. The substrate owns the per-system *contract*; each
theme *chooses* whether to consume it; the user can override at runtime (D32).

**Audit findings that informed this (2026-06-15):**
- All per-system UI machinery already lives in `platform/` (`platform/themes/
  systemUIConfigs.ts`, `systemBootAnimation.ts`, `systemUiSound.ts`,
  `systemPalettes.ts`; `platform/components/{ThemeBackground,SystemBootAnimation,
  LibraryTile}.tsx`; Rust `system_ui_assets.rs`) — correct home for a capability.
- **Backgrounds are already correctly theme-opt-in:** `<ThemeBackground>` is
  mounted only by CoverFlow; App.tsx unmounted the global background 2026-05-31
  (Retroverse visual conflict); Retroverse consumes none of it.
- **Boot animations are dormant** — `<SystemBootAnimation>` is mounted nowhere
  (was App-global, now orphaned).
- **Residual mis-scope (the fix this decision mandates):** tile flourishes +
  per-system SFX are still forced cross-theme through the shared `platform` grid
  (`LibraryTile` reads `tileShape`/`interactionStyle`; grid-nav dispatches
  `playSystemUiSound`), gated only by a single **global** `perSystemUiEnabled`
  toggle. Any theme using the shared `LibraryView`/grid (incl. Retroverse)
  inherits Retroverse-flavored per-system tiles/SFX whether it wants them or not.

**How to apply (ARC 2):**
- Make per-system UI consumption **uniformly theme-opt-in**, matching how
  backgrounds already behave: a theme declares whether it consumes per-system UI
  (Retroverse: yes; CoverFlow: backgrounds only; bare: no). Convert the global
  `perSystemUiEnabled`-gated tile/SFX path in the shared grid into a per-theme
  opt-in (capability stays in platform; *consumption* becomes a theme choice).
- **Re-home the paused Per-System UI Stage 2/3 as ARC-2 work** that builds *into*
  the substrate capability (consumed by Retroverse), NOT as engine-global
  behavior. This is the per-system-ui ↔ theming merge point D32 named.
- Capability stays in `platform/`; only the *forced-global consumption* is the
  defect to correct. Do not duplicate the machinery into themes.

**Why:** D32 is the newer, deliberately-reasoned position (from the BigBox
research) and the per-system capability is ARC 2's headline user-facing value;
D19's "Retroverse-private" would forfeit "any theme can have per-system
experiences" and force per-theme duplication. The operator's instinct that
per-system UI "should be theme-only" is satisfied not by privatizing the
machinery but by making *consumption* a per-theme opt-in. Supersedes the
"Retroverse-only" half of **D19**; **D32** stands.
