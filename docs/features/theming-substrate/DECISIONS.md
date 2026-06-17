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

---

### D34 — Per-system *capability* is platform; per-system *content + experiential choices* are theme (ARC-2 ownership line)

**Date:** 2026-06-15 (ARC-2 planning;
[PLANS/theming-arc-2-per-system-layout.md](../../PLANS/theming-arc-2-per-system-layout.md)
§2). **Decision:** the operator drew the precise line D33 left implicit, when
clarifying that the per-system-ui Stage-1 pilots' content (GB/NES/Vectrex SFX,
backgrounds, boot animations, the Vectrex view) "is for the Retroverse theme,
not for every theme."

**The line:** *factual data + machinery = platform; experiential design +
content = theme.*

- **Platform owns the capability** — the layout resolver + cascade, the layout
  primitives (list/grid/carousel/**wheel**/custom), the `views`/`per_system`
  manifest contract + validator, the user-override store, the *factual*
  per-system data (palette/accent — already split to `SYSTEM_PALETTES` in
  S5.2), the asset-cascade tiers + SFX dispatcher + boot framework, and a thin
  `_baseline` per-system fallback.
- **Each theme owns its content + experiential choices** — the actual SFX
  banks / backgrounds / boot animations (homed in the theme's asset tree, the
  S5.1 theme tier `assets/themes/<id>/system-ui/<system>/…`), the per-system
  *layout choices* (declared in the theme's manifest `views[].per_system`), the
  Vectrex custom view, tile flourishes, signature feel.

**Consequence (the migration ARC 2 does):** the shipped platform-global
`systemUIConfigs.ts` holds *experiential* per-system choices (`layout` /
`audioProfile` / `interactionStyle` / `tileShape`) for all 40 systems as a
platform default every theme would inherit — exactly the "forced cross-theme"
defect D33 names. ARC 2 migrates the experiential config out of platform-global
into **Retroverse-owned** declaration (Retroverse, the flagship, carries the
full 40-system set; CoverFlow none; `bare` none). Platform keeps only the thin
`_baseline` fallback so a theme that opts in without authoring 40 configs still
renders something.

**Therefore** the paused Per-System UI **Stage 1 pilots (slices 6–9)** +
**Stages 2–3** re-home into ARC 2 as **Retroverse content**, NOT platform
behaviour — the per-system-ui ↔ theming merge point D32/D33 named.

**Why:** without this line, "make consumption theme-opt-in" (D33) is
ambiguous about *where the consumed content lives*. Baking rich per-system
content into the platform `_baseline` would re-create the forced-global defect
one layer down (every theme inherits Retroverse's taste). Homing rich content
in the theme + a thin platform fallback gives genuine per-theme expression with
a graceful floor.

**How to apply:** new per-system *experiential* work lands as theme content +
manifest declaration; new per-system *machinery* lands in platform. Never
duplicate the machinery into a theme; never bake a theme's signature content
into the platform baseline. The boundary lint already enforces the import
direction (`platform ↛ themes`); D34 is the content/data analogue.

---

### D35 — Arc separation + renumber: ARC 2 = layout, ARC 3 = cinematic/scripting, ARC 4 = Theme Studio

**Date:** 2026-06-15 (ARC-2 planning). **Decision:** the old ARC-2 ("Behaviors
+ Shaders — Rhai + WGSL") is **split**. The declarative per-system *layout*
capability (D32/D33/D34) is a self-contained, scripting-free arc and the
operator's headline; the cinematic/scripting axis is a different beast with a
different risk profile. Fusing them made ARC 2 a 15+ week monster spanning two
unrelated thrusts.

**New arc structure:**
- **ARC 2 — Per-System Layout Substrate** — D32 layout/view capability + D33
  consumption opt-in + the per-system-ui Stage 2/3 re-home + the `.oatheme`
  runtime loader (the deferred §6 Phase 5). Fully declarative.
  [PLANS/theming-arc-2-per-system-layout.md](../../PLANS/theming-arc-2-per-system-layout.md).
- **ARC 3 — Cinematic & Scripting** — the old ARC-2 content: declarative
  motion/transitions (the reserved `motion` token category), `<video>`
  backgrounds / attract, sandboxed Rhai behaviours, WGSL shader chrome, media
  binding. Natural internal order: declarative motion → Rhai → WGSL.
- **ARC 4 — Theme Studio** — the old ARC 3 (visual editor, Model A
  round-tripping), bumped one slot. Unchanged in content; D32's "Theme Studio
  stays after the layout/motion/shader capabilities exist" sequencing holds.

**The `.oatheme` loader sits at ARC 2's tail** (not ARC 3) because it closes
the last open ARC-1 thread *and* D6 notes its CSP work "becomes load-bearing
for Rhai sandboxing anyway" — so it tees up ARC 3 cleanly.

**Why:** ARC 2 ships the BigBox-parity headline ("each system is literally its
own home at the layout level") without waiting on a scripting engine, and the
cinematic axis gets its own focused arc instead of a cramped tail slice. The
declarative-motion insight (D23.6 — much of "motion" is CSS-achievable without
Rhai) lives in ARC 3 with the rest of the cinematic work rather than leaking
into the layout arc.

**How to apply:** "ARC 2" now unambiguously means the layout substrate.
References to "ARC 2 = Rhai + WGSL" and "Theme Studio (ARC 3)" in older docs
(plan §5 table, D32, the BigBox research, THEME_CONTRACT) are updated to the
2/3/4 numbering as those docs are next touched; the plan §5 arc table is
updated this session.

---

### D36 — ARC-2 L1: per-system-UI consumption is a per-theme manifest opt-in, App-bridged, gated under the user master toggle (the D33 fix)

**Date:** 2026-06-15 (`feat/theming-arc2-l1-per-system-opt-in`). **Decision:** the
build shape for L1 — converting the forced-global per-system tile/SFX path into a
per-theme opt-in. Shape signed off via AskUserQuestion (the `{ tiles, sfx }` struct)
before code.

1. **Manifest field `per_system_ui?: { tiles?: boolean; sfx?: boolean }`** — a small
   *separable* struct, not a coarse boolean and not a richer one. The two genuinely
   *forced-global* surfaces (the only ones the shared grid imposes cross-theme) are
   tile flourishes (`LibraryTile` `tileShape`/`interactionStyle`) and nav SFX
   (`playSystemUiSound`); the code already gates them separately, so the contract
   mirrors that. Backgrounds + boot are already opt-in by **component mount**
   (`ThemeBackground` / `SystemBootAnimation`) → no flag. Per-system **layout** (D32)
   is the *separate* `views` field (L2) — deliberately NOT folded in here.

2. **App-bridged into the gate, exactly like S5.3's `glyph_set`.** `systemUiSound.ts`
   holds a `themeUiSig` (default `{tiles:false, sfx:false}`) + `setThemePerSystemUi`;
   App.tsx `createEffect(() => setThemePerSystemUi(activeTheme()?.manifest.per_system_ui))`.
   nav/themes stays a generic leaf; App is the one place that knows the active theme.
   *Constraint:* don't make `systemUiSound`/consumers reach into `theme/registry` —
   bridge from App.

3. **Effective gate = `userMaster AND activeThemeOptsIn(surface)`.** New accessors
   `consumesPerSystemTiles()` / `consumesPerSystemSfx()` fold the existing user master
   toggle (`isPerSystemUiEnabled`, kept) with the per-theme flag. The user toggle
   survives as a **global off-switch** (accessibility / low-end escape, default ON) —
   master-off forces uniform regardless of theme. `LibraryTile` + the grid's
   column-fit estimate consume `consumesPerSystemTiles`; `playSystemUiSound` consumes
   `consumesPerSystemSfx`.

4. **Default OFF (the D33 rule).** A theme that declares nothing gets a uniform grid.
   **Retroverse** (the default/flagship) must now *explicitly* declare
   `per_system_ui: { tiles: true, sfx: true }` to keep its per-system feel; CoverFlow
   + bare declare nothing → uniform. The flagship needing an explicit opt-in is the
   point: per-system UI is a platform capability themes *choose*, not a forced default.

5. **Malformed value = validator WARNING + fall back OFF**, not an error (mirrors
   `glyph_set`). A consumption flag shouldn't disqualify a whole theme; the safe
   fallback (uniform) is graceful. `INVALID_PER_SYSTEM_UI`.

**Verification:** typecheck + lint + vitest (114, incl. new `systemUiSound.test.ts`
gate tests + 2 validator cases) + build green; frontend-only (Rust resolvers
unchanged). **Acceptance gate (operator playtest):** Retroverse reads per-system
(tiles + nav SFX) as before; CoverFlow + bare read **uniform**; the user master-off
forces uniform even under Retroverse.

**Why record this:** (2) the App-bridge-not-consumer-reads-theme direction and (4)
the default-OFF / flagship-must-opt-in are the two a later contributor could undo —
by coupling the gate to the theme registry, or by "restoring" per-system UI as a
global default and re-creating the D33 forced-cross-theme defect.

---

### D37 — ARC-2 L2 split: stamp the view/layout contract (L2a) before the D34 migration (L2b)

**Date:** 2026-06-15 (`feat/theming-arc2-l2a-view-layout-contract`). **Decision:**
the plan's L2 bundled two different-risk pieces — the *additive* view/layout
contract and the *consumer-touching* D34 `systemUIConfigs` migration. Split via
AskUserQuestion sign-off into **L2a (contract, this slice)** + **L2b (migration,
next)**, so each is independently playtestable and the contract is stamped before
anything consumes it (the S4→S5 pattern).

**L2a build shape (shipped):**
1. **`ViewType`** = `manufacturer-browse | system-browse | game-browse |
   game-details` — the library journey. The validator allow-lists the full set; the
   engine *honors* a growing subset (L3 wires `game-browse` first). The union
   extends (home / collections / now-playing) when honored — "reserve the contract,
   defer the body" (D20b pattern).
2. **`LayoutPrimitive`** = `list | grid | carousel | wheel | custom` — the S5.5
   nav-primitive set. Named as plain string-literals in `manifest.ts` so the
   contract stays **decoupled from the nav layer** (like `glyph_set`); the L3
   resolver owns the `LayoutPrimitive → nav component` mapping. `wheel` is still the
   reserved S5.5 stub until L4.
3. **Manifest `views?: { [view]: { layout, per_system? } }`** — per-view default
   layout + optional per-system override (D32). `per_system` keys kept loose
   `string` in the type (no registry coupling); the validator checks them against
   `SYSTEM_PALETTES`/SystemId.
4. **Validator: malformed = ERROR** (not warning) — a broken layout map is worse
   than none, same rationale as `settings_schema`. Codes `INVALID_VIEWS` /
   `UNKNOWN_VIEW_TYPE` / `INVALID_VIEW_LAYOUT`; reuses `UNKNOWN_SYSTEM_ID` for
   per-system keys. (Contrast `per_system_ui`/`glyph_set` = warnings: those degrade
   gracefully to a safe default; a malformed *layout map* can't.)
5. **No consumer in L2a.** Built-ins declare no `views` (still validate clean); the
   L3 resolver is the first reader. Pure additive, zero visual change.

**L2b (next):** the D34 migration — `touchInputSupported` is the *only* factual
`SystemUIConfig` field (hardware: has-touchscreen; gates stylus/touch overlays
regardless of theme) and stays platform; everything else (layout / audioProfile /
interactionStyle / tileShape / …) is experiential → moves to `themes/retroverse/`,
bridged into the tile/SFX consumers via the L1 opt-in pattern (only read when the
theme opts in). Behavior-preserving (visual-identical gate).

**Why record this:** the split rationale (contract before migration) and the
factual-vs-experiential line (`touchInputSupported` is the lone factual field) are
what a later contributor needs to not (a) fold the risky migration back into the
contract slice, or (b) drag `touchInputSupported` into the theme and break
touch-overlay gating for non-Retroverse themes.

---

### D38 — ARC-2 L2b: per-system experiential config is theme content (`ThemePackage.perSystemUiConfigs`), bridged + merged over `BASELINE_UI`; `touchInputSupported` stays platform-factual

**Date:** 2026-06-15 (`feat/theming-arc2-l2b-systemuiconfigs-migration`). **Decision:**
the D34 migration build shape — moving the experiential per-system config out of
the platform-global `systemUIConfigs` map into theme content. Home signed off via
AskUserQuestion (`ThemePackage.perSystemUiConfigs`, the peer of `perSystemTokens`).

1. **`ThemePackage.perSystemUiConfigs?: Partial<Record<SystemId, Partial<SystemUIConfig>>>`**
   — theme content, the exact peer of `perSystemTokens`. App.tsx bridges
   `activeTheme()?.perSystemUiConfigs` into a platform signal
   (`setThemeSystemUiConfigs`, the L1/glyph-set pattern); `uiConfigFor(systemId)`
   returns `{ ...BASELINE_UI, ...override[systemId] }`. Rejected: a manifest field
   (it's content, not metadata — same reason `perSystemTokens` isn't on the
   manifest) and a separate registry injection (more indirection than the package
   field needs).

2. **Platform keeps the contract; the theme owns the values.** `SystemUIConfig`
   type + the `UI*` enums + `BASELINE_UI` stay in `platform/themes/systemUIConfigs.ts`
   (the capability); the per-system *values* moved to
   `themes/retroverse/systemUiConfigs.ts`. Only the 3 systems that differ from
   baseline (gb/nes/vectrex pilots) have entries — every other system inherits
   `BASELINE_UI`, so the migrated content is tiny.

3. **`touchInputSupported` is FACTUAL → stays platform, split OUT of `SystemUIConfig`.**
   It's a hardware fact (NDS has a touchscreen under *any* theme) and gates the
   stylus/touch overlays theme-independently — so it cannot be theme content. New
   `systemSupportsTouch(systemId)` lookup (a `Set<SystemId>`); the 3 factual
   consumers (QuickSettings / StylusOverlay / TouchHotspotOverlay) read it instead
   of the removed map. *Constraint:* don't move touch-support into the theme — it
   would break touch overlays for non-Retroverse themes.

4. **Behavior-preserving.** Retroverse declares gb/nes/vectrex exactly as the old
   global map → identical render. CoverFlow/bare ship none → `BASELINE_UI` (and
   don't consume tiles/SFX anyway, per L1's opt-in gate). Composes with L1: the
   experiential config is only *read* on the tile/SFX paths a theme opted into.

5. **Validator:** `perSystemUiConfigs` keys checked against SystemId (reuse
   `UNKNOWN_SYSTEM_ID`), mirroring `perSystemTokens`; field VALUES are enum-typed
   `Partial<SystemUIConfig>` so deep value validation waits for on-disk themes.

**Verification:** typecheck + lint + vitest (131; new `systemUIConfigs.test.ts`
merge/touch tests + validator cases) + build green. Frontend-only.
**Acceptance gate (operator playtest, visual-identical):** Retroverse per-system
tiles + nav SFX unchanged (gb portrait/delayed, nes console-audio, vectrex
physical/square); CoverFlow/bare uniform; NDS stylus/touch overlays still gate.

**Why record this:** (1) the package-field-not-manifest home and (3) the
`touchInputSupported`-stays-factual split are the two a later contributor could
get wrong — by promoting the config to the manifest, or by folding touch-support
back into the theme content and breaking non-Retroverse touch overlays.

---

### D39 — ARC-2 L3 split: ship the layout resolver + persisted override store (L3a) before the LibraryView consumer (L3b)

**Date:** 2026-06-15 (`feat/theming-arc2-l3-layout-resolver`). **Decision:** L3
(per-system layout becomes real) splits — **L3a** the plumbing (resolver cascade +
persisted user-override store + hook, no consumer), **L3b** the LibraryView
consumer + the resolved-layout-vs-`viewMode` UX call + visual playtest. Same
contracts-first split as L2; signed off via AskUserQuestion. L3a build shape:

1. **Resolution cascade** (`layoutResolver.ts`) — `user override → theme
   views[view].per_system[system] → theme views[view].layout → engine default`,
   the same "resolve by active system" shape as S5.1 (assets) / S5.2 (palette)
   with D18's per-user override tier on top. Split into a **pure `resolveLayout`**
   (inputs → primitive; directly tested) + a reactive **`useResolvedLayout(view,
   systemId)`** hook that wires the active theme's `views` (registry) + the
   override store + the engine defaults. `ENGINE_DEFAULT_LAYOUTS` is the bottom
   tier (browse views → `grid`, `game-details` → `custom`).

2. **Persisted override store** (`layoutOverrides.ts`) — the `(theme_id,
   system_id, view) → layout` namespace D32 names. One localStorage key
   (`oa.layoutOverrides`), `createStore`-backed (reactive), keyed by **theme id**
   so overrides don't leak across themes; mirrors the D28 `themeSettings`
   localStorage pattern (frontend-owned, survives the restart swap). `get` /
   `set` / `clear` (clear prunes empty branches).

3. **No consumer in L3a** (the contracts-first point). Nothing reads
   `useResolvedLayout` yet — L3b wires it into the game-browse view keyed on
   `LibraryView`'s existing `selectedSystemId()` and decides how the per-system
   resolved layout relates to the existing global capsule/list `viewMode` toggle
   (the real UX fork, deferred here on purpose). So L3a is CI-gated, no visual
   change.

**Verification:** typecheck + lint + vitest (142; pure-cascade tests +
override-store round-trip/isolation) + build green. Frontend-only.

**Why record this:** the pure-core / reactive-hook split (1) and the
theme-id-keyed override store (2) are what a later contributor should preserve —
don't make the pure resolver reach into the DOM/registry, and don't drop the
theme-id key (it keeps one theme's "pick your view" from leaking into another).

---

### D40 — ARC-2 L3b: per-system layout COEXISTS with the global viewMode toggle (overrides only where declared); `layout` becomes optional

**Date:** 2026-06-15 (`feat/theming-arc2-l3b-layout-consumer`). **Decision:** how
the per-system resolved layout relates to the existing global capsule/list
`viewMode` toggle, signed off via AskUserQuestion — **coexist**: the global toggle
stays the default for every system; the theme's per-system `views` (and, later, the
user's L5 override) override it ONLY for systems where declared. Behavior-preserving.

1. **`ViewLayoutConfig.layout` is now OPTIONAL.** A theme must be able to declare
   `per_system` overrides WITHOUT a view-wide `layout` — a view default would sit
   above `viewMode` in the cascade and override every (undeclared) system's global
   toggle, breaking the coexist promise. So `layout?` is optional; the validator
   validates it only when present (the L2a "missing layout = error" rule is
   relaxed). (Refines the L2a/D37 contract.)

2. **A "declared-only" resolver tier.** `resolveDeclaredLayout` / `useDeclaredLayout`
   run the cascade MINUS the engine-default fallback → return `undefined` when
   neither the user nor the theme declares a layout. That `undefined` is the signal
   the consumer uses to keep its OWN default. (`resolveLayout` with the engine
   default stays for consumers with no fallback of their own.)

3. **`LibraryView` consumes it keyed on the existing `selectedSystemId()`.** When a
   single system is in context and a layout is declared → render that primitive;
   else → today's exact `viewMode()` capsule/list switch. Mapping: `grid` →
   `VirtualLibraryGrid`, `list` → `DetailListView`; `carousel`/`wheel`/`custom` are
   NOT yet rendered in the shared browse view → fall back to `grid` for now
   (carousel/wheel game-browse rendering is a follow-on; `wheel` also needs the L4
   primitive). So L3b proves the cascade end-to-end with grid↔list.

4. **Retroverse demo: `views: { "game-browse": { per_system: { nes: "list" } } }`** —
   NO view-wide `layout` (per the coexist rule), only a per-system override. NES
   browses as a text list; every other system keeps the operator's global viewMode.
   Clearly labelled a demo — real per-system layout curation lands in L6.

**Verification:** typecheck + lint + vitest (145) + build green. Frontend-only.
**Acceptance gate (operator visual playtest):** select NES → game list renders as a
DetailListView; select another system / All Games → grid (or the global viewMode);
flipping the global capsule/list toggle still affects all undeclared systems.

**Why record this:** (1) `layout`-optional + (3) the declared-only-vs-engine-default
distinction are the two a later contributor could undo — by re-requiring `layout`
(which would silently override everyone's global toggle), or by having LibraryView
call `resolveLayout` (engine default) instead of `useDeclaredLayout`, forcing grid
on users who set viewMode=list globally.

---

### D41 — ARC-2 L4 split: render `carousel` in game-browse (L4a, reuse) before building the `WheelNav` radial primitive (L4b, new geometry)

**Date:** 2026-06-15 (`feat/theming-arc2-l4-wheelnav`). **Decision:** L4 (render
carousel/wheel + build WheelNav) splits — **L4a** wire the EXISTING `CarouselNav`
into game-browse (reuse, lower risk), **L4b** build the reserved radial `WheelNav`
geometry + render `wheel` (new, playtest-sensitive). Signed off via AskUserQuestion;
L4b deferred to a fresh focused session (radial math + windowing for big libraries
shouldn't be rushed at the tail of a long session). L4a build shape:

1. **`LibraryView` renders `carousel` via the `CarouselNav` primitive** — the same
   path CoverFlow uses: a coverflow over the flat `sorted()` list, controlled focus
   (so the right-pane detail + `onFocus` follow the centred card), cover art via
   `useMedia` (identity key → per-file fallback), `onConfirm`→launch /
   `onSecondary`→info. Cards carry `data-system` so Retroverse's per-system accent
   drives the focus ring (vs CoverFlow's deliberately system-agnostic cards, D19).
   The render switch became a 3-way `<Switch>` (grid fallback / list / carousel).

2. **`wheel` + `custom` still fall back to grid** in game-browse — `wheel` needs the
   L4b primitive; `custom` is theme-drawn markup not meaningful for the shared view.
   So a per-system `carousel` is now live; `wheel`/`custom` are honored-as-grid
   until their slices.

3. **Retroverse demo extended:** `views.game-browse.per_system = { nes: "list",
   snes: "carousel" }` — SNES browses as a coverflow (good box art), NES as a list,
   others keep the global viewMode. Demos; real curation is L6.

**Verification:** typecheck + lint + vitest (145) + build green. Frontend-only.
**Acceptance gate (operator visual playtest):** select SNES → coverflow of SNES
games (centred scaled cover, neighbours fanning, Confirm launches); NES → list;
others → grid / global viewMode.

**Why record this:** the reuse-before-new-geometry split (and that `wheel`/`custom`
honor-as-grid until built) is what keeps the shared browse view from sprouting a
half-built radial layout — L4b builds the real `WheelNav` when it gets a focused
session.

### D42 — ARC-2 L4b: build the radial `WheelNav` as a general angle→x/y engine (shape A = defaults), render `wheel` in game-browse

**Date:** 2026-06-15 (`feat/theming-arc2-l4b-wheelnav`). **Decision:** implement the
reserved S5.5 `WheelNav` contract (was a warn-once stub) as the iconic BigBox /
HyperSpin radial wheel, but build the geometry **general** so future display shapes
are presets, not rewrites. The operator picked **shape A first** (right-side vertical
wheel) and explicitly asked that B/C "and other ways to display" land later as
variations. Build shape:

1. **Geometry is a pure angle→x/y projection** (`platform/nav/primitives/
   wheelGeometry.ts`, split out so the bug-prone radial math is unit-testable —
   mirrors the spatial engine's `spatialGeometry.ts`). Items sit on a circle of
   `radius`; the focused item is at `anchorAngle` (deg, 0 = top); each item's
   on-screen position is the pixel DELTA from wherever the focus is pinned, derived
   from its signed `offset`. Positive offset (next item) rotates DOWN so the wheel
   reads like a vertical list. No circle centre, no track transform — a focus change
   re-projects every visible item and CSS transitions animate the slide along the arc.

2. **Shape A is the DEFAULTS, not a special case.** `anchorAngle` default moved
   **0 → 270** (focus = leftmost point of the circle → centre off-screen right) and a
   new optional `anchor` prop (on-screen pin point, default right-of-centre `62%/50%`)
   was added. So `<WheelNav radius={…}>{…}</WheelNav>` renders the iconic wheel with
   zero config (low-floor). **Shape B** (centred fan) = `anchor:{x:"50%",y:"50%"}` +
   wider arc; **shape C** (bottom arc) = `anchorAngle ~0` + `anchor:{y:"82%"}`. These
   are new prop presets over the same body — recorded so the next session adds them
   without touching the engine. Optional visual knobs (`focusedScale`/`sideScale`/
   `opacityFalloff`/`minOpacity`/`arcDegrees`) mirror CarouselNav for parity; the
   originally-reserved props (`radius`/`arcDegrees`/`window`/`anchorAngle`/
   `transitionMs`) are unchanged so it stays a drop-in.

3. **Vertical orientation + windowing + parity wiring.** `useFocusGroup({orientation:
   "vertical"})` (Up/Down browses, matching the region-bias hybrid); only ±`window`
   items in the DOM (scales to NES's 1708 games, D29.1); `useLateClaim`, `onNavSound`,
   wheel-scroll, click-side-to-focus / click-focus-to-confirm — all lifted from
   CarouselNav. **Covers stay upright** (no per-item counter-rotation — the
   slide-along-the-arc motion is the wheel feel; rotating art hurts legibility).

4. **`LibraryView` renders `wheel`** as a 4th `<Switch>` arm (grid/list/carousel/
   wheel); the carousel + wheel share one controlled browse focus index; the ring
   `radius` is derived from a `ResizeObserver`-measured pane height (`0.52 ×` height,
   min 240) so the wheel fills the column at any window size. `custom` still
   grid-falls-back (theme-drawn; L5/L6).

5. **Retroverse demo:** `views.game-browse.per_system` adds `tg16: "wheel"` — the
   plan's own canonical wheel example and the project's flagship system — joining
   `nes: "list"`, `snes: "carousel"`. Demo; real curation is L6.

**Verification:** typecheck + lint + vitest (**149**, +5 `wheelGeometry` cases
replacing the stub assertion) + build green. Frontend-only.
**Acceptance gate (operator visual playtest):** select TG-16 → a navigable right-side
radial wheel of TG-16 games (focused cover pinned right-of-centre, neighbours fanning
up/down + curving away, the left of the pane free); Up/Down + scroll rotate it;
Confirm launches; Secondary opens info. NES → list, SNES → carousel, others →
grid/viewMode unchanged.

**Why record this:** the "general engine, shape A as defaults" call is the load-bearing
one — it's what lets B/C/other display modes be additive presets instead of a WheelNav
rewrite (the operator's stated intent), and it's the seam the future serializable layout
DSL / Theme Studio target.

### D43 — ARC-2 L5: end-user per-system layout override UI lives as a "Layout" domain card in the engine Per-System Settings Hub

**Date:** 2026-06-15 (`feat/theming-arc2-l5-layout-picker`). **Decision:** ship the
D32 "pick your view per system" user-agency surface as the editing UI on top of the
already-built L3 resolver + override store — pure UI, no new machinery. Three forks
settled with the operator before code (AskUserQuestion):

1. **Home = a new "Layout" domain card in the per-system Settings Hub**
   (`engine/systemsHub/`), NOT a theme-territory quick action. Engine-owned,
   theme-agnostic, already controller-navigable, consistent with the other
   per-system editors — and it keeps the *machinery* on the engine side per D34
   (experiential content is the theme's; the picker is machinery). New
   `DomainId "layout"` + `DOMAINS` entry + `domains/LayoutEditor.tsx` wired into the
   `SystemsHubRoot` Switch.

2. **Scope = ALL FOUR `ViewType`s get a picker now** (operator overruled my
   game-browse-only recommendation). Only `game-browse` has a live renderer
   (`LibraryView`); `system-browse` / `manufacturer-browse` / `game-details` are
   honored by the resolver but nothing mounts them yet. The editor exposes all four
   but **labels each row** "Shown in the library now" vs "Reserved — no renderer yet"
   (a `HONORED_VIEWS` set, grown as views gain consumers) so a no-op pick reads as
   honest, not broken — per the project's "code-exists isn't proof a feature is live"
   discipline. `game-browse` is ordered first.

3. **Picker primitives = list / grid / carousel / wheel** — curated from
   `LAYOUT_PRIMITIVES` by excluding `custom` (it means "the theme draws this view"
   and isn't authorable from a dropdown). `custom` still *appears* in the inheritance
   chip when a view's default resolves to it (e.g. `game-details` → engine default
   `custom`).

**Reset discipline** mirrors the D30 NavRemapCard via the shared `SettingRow`
primitive: each row's select carries a leading `Theme default — <X>` sentinel
(value `""` → `clearLayoutOverride`); the inheritance chip shows the no-override
fallback value (`resolveLayout({override: undefined, …})`) and which tier supplies it
("this theme · per-system" / "this theme" / "engine default"); the Reset chip appears
only when overridden. Overrides are **keyed by the active theme id**, so the subtitle
states they apply to the active theme — the override store guarantees they don't leak
across themes (a Retroverse pick is invisible under CoverFlow). Because the override is
the TOP cascade tier, a pick takes effect immediately (LibraryView's `useResolvedLayout`)
and persists across the restart-based theme swap.

**Verification:** typecheck + lint + vitest (**149**, unchanged — the editor is
playtest-verified like the other ARC-2 primitive renders; the cascade logic it leans
on is already covered by `layoutResolver.test.ts`) + build green. Frontend-only.
**Acceptance gate (operator playtest):** Settings → Systems → <system> → Layout →
set Game browse → list/carousel/wheel → LibraryView reflects it immediately for that
system; restart → still applied; Reset → back to the theme default. A reserved view's
pick persists in the store but visibly changes nothing yet (labeled as such).

**Why record this:** L5 is the D32 user-agency headline made real — it's the first
surface where an end user, not a theme author, sets per-system layout. The "expose all
four views but label the reserved ones" call is the load-bearing one: it honors the
operator's choice to surface the full contract while refusing to imply a renderer
exists where it doesn't.

### D44 — P (`.oatheme` loader): the default theme stays bundled; externalization hardens the platform API into a versioned contract + makes untrusted-author security real; engine growth (ARC 3 motion/scripting) stays safe via additive + version-gated changes

**Date:** 2026-06-15 (design conversation, no code — forward guidance for when P is
scheduled). **Context:** operator asked, ahead of building P, (1) whether finishing the
`.oatheme` runtime loader means Retroverse + the other built-in themes can be moved into
`.oatheme` files and removed from the binary, and (2) whether dynamically-loaded themes
will cause problems once the engine grows motion + scripting (ARC 3). Recorded so it
isn't re-litigated.

**Decisions / guidance:**

1. **The default theme STAYS bundled in the binary — do not evict it.** Three reasons:
   (a) it's the **guaranteed fallback floor** — the S4/D24 registry failure model is
   "active on-disk theme fails to validate/load → fall back to a known-good theme," and
   that floor must always be present + load-proof; an external default could be
   missing/corrupt and leave no UI; (b) **first-run / offline** boots into a real UI
   with zero external files; (c) it's the **in-tree dogfood that catches platform-API
   breaks at build time** (see #3). Likely end-state: default (Retroverse or `bare`)
   bundled; community themes — and optionally non-default built-ins like CoverFlow —
   external. The driver for P is letting *others* drop themes in, NOT moving ours out.

2. **Externalizing a theme is NOT just "move the folder."** Built-in themes today
   consume `@oa/platform` via a **build-time Vite alias** (compiled into the one
   bundle). A standalone `.oatheme` that's `import()`-ed at runtime must be built as a
   **separate bundle that binds platform as a shared RUNTIME module** — so P must expose
   platform as a stable runtime API surface (a shared global the dynamic import links
   against), not a compile-time alias. That packaging change is core P work, alongside
   the Rust discovery/extract/validate path + the CSP allowlist.

3. **Externalization imposes two genuine new burdens** (the true price, worth weighing
   before evicting anything): (a) **the platform API hardens into a versioned contract**
   — in-tree themes get refactored in lockstep with platform (typechecked together); a
   *prebuilt* external theme cannot, so breaking changes then require migrations +
   `schema_version` discipline. Keeping the in-tree dogfood bundled preserves build-time
   break detection even after externals exist. (b) **Untrusted-author security becomes
   real** — the S4 validator can't catch a `<style>:root` global-CSS bypass today, nor
   arbitrary Rhai/WGSL tomorrow (THEME_CONTRACT §6 deferred gap). P opens that door;
   **ARC 3's Rhai sandbox is where it's hardened.**

4. **Engine growth (ARC 3 motion + scripting) does NOT break external themes IF growth
   stays additive + version-gated** — which is the existing discipline, via seams
   already in place: `schema_version` (too-new vs unsupported messaging + migration
   path), `required_engine_capabilities ⊆ ENGINE_CAPABILITIES` (empty in ARC 1;
   ARC 3 populates `motion`/`scripting`/`shaders` → a theme needing a capability the
   build lacks gets a clean refusal, not a crash), and the reserved `motion` token
   category (S3). Adding motion fills a reserved slot; scripting is a new gated
   capability.

5. **P is the on-ramp to ARC 3, not an obstacle.** Per D6, the CSP allowlist P must add
   to permit out-of-bundle dynamic imports "becomes load-bearing for Rhai sandboxing
   anyway" — same boundary. That's why P is sequenced as the bridge from ARC 2 into
   ARC 3.

**Why record this:** the "evict our themes from the binary?" question is easy to
re-ask, and the answer (keep the default bundled; externalize for *community* themes;
accept that doing so freezes the platform API into a contract + makes author-trust a
real concern) is load-bearing for how P and ARC 3 are scoped.

---

## 2026-06-16 — ARC 2 "P" P.1 S1: `.oatheme` runtime loader (declarative-first)

> Branch `theme-oatheme-loader-slice-1`. The four planning decisions locked
> with the operator 2026-06-16 (PD1–PD4 in
> [docs/PLANS/theming-oatheme-loader.md](../../PLANS/theming-oatheme-loader.md))
> formalized here at execution. **Numbering note:** the plan reserved
> "D44–D47" for these, but D44 was already taken by the 2026-06-15 P
> forward-guidance entry above, so PD1–PD4 land as **D45–D48**.

### D45 (= PD1) — Runtime `.oatheme` themes are declarative-only in ARC 2 (P.1); custom-JS loading is deferred (P.2)

Disk-loaded themes are **data, never code** — no author-supplied JavaScript at
runtime. A built-in generic `DeclarativeShell` (P.1 S2) renders every disk
theme by interpreting its manifest + tokens + per-system palette. Custom-code
shells (Retroverse-class) stay **build-time built-ins**; the scripted escape
hatch is ARC 3 (Rhai).

**Why:** sidesteps the three hard, deferred problems of loading author JS in the
Tauri WebView — shared Solid/`@oa/platform` singletons (two Solid instances =
broken reactivity), the CSP/import-map origin rules on dynamic `import()` off
the asset protocol, and arbitrary-code-execution trust on a downloaded pack
(sha256 only proves the bytes match the registry). It also matches ARC 2's own
"fully declarative, no scripting" scope and the project's declarative-first
philosophy. **Honest ceiling:** a declarative theme is a single-surface browse
shell (layout primitive per view/system + palette + background + sounds + glyph
set + settings); it cannot express Retroverse's multi-tab/detail-panel
structure — that high ceiling stays compiled-in / ARC 3.

**How applied (S1):** `theme.toml` mirrors `ThemeManifest` **minus
`entry`/`entry_export`** — those are implicit (the loader supplies
`DeclarativeShell`). See `apps/oa-shell/src/theme_loader.rs::DiskThemeManifest`.

### D46 (= PD2) — On-disk manifest is `theme.toml`; themes live at `<exe_dir>/themes/community/<id>/`

The on-disk theme definition is **TOML** (`theme.toml` + optional
`tokens.toml` / `per-system.toml`), discovered under
`<exe_dir>/themes/community/<id>/` — the same `<type>/community/<id>` shape the
pack channel installs to (CP2), with `themes` as the pack `type` (CP3). A loose
`<exe_dir>/themes/dev/<id>/` path is **reserved** for hand-dropped dev themes
(scanned at startup; hot-reload deliberately NOT wired — swap-by-restart is the
shipped model, so hot-reload is pure dev ergonomics, deferred until it earns its
keep).

**Why TOML:** `serde` + `toml` is already a workspace dep; theme manifests were
sketched as TOML in the ARC 1 §6 Phase 5 plan; the format is an internal detail
behind the loader. Manifest keys stay snake_case (matching `ThemeManifest`'s TS
field names verbatim); token keys are camelCase (matching `ThemeTokens`) — the
same casing split the TS contract already uses, so the parsed document maps 1:1
onto the frontend types with no transform.

**How applied (S1):** `resolve_themes_community_dir()` /
`resolve_themes_dev_dir()` resolve the paths; `load_from_parent_dir()` walks one
subdirectory per theme.

**CORRECTION (S3, 2026-06-16):** S1 shipped these resolvers with *no* source-tree
fallback (on the theory that disk themes are install-time artifacts like recipe
packs). That broke the operator's actual playtest workflow: `cargo tauri build`
runs `target/release/oa-shell.exe`, whose `<exe_dir>` has no resources beside it,
so a repo-placed theme was never found (the loader logged "no themes/community/
directory"). Every OTHER resource loader (`system_registry`,
`emulator_profiles`' baseline `config/`) already carries a source-tree fallback
for exactly this. Fixed in S3: `resolve_themes_subdir` now walks
`<exe_dir>/themes/<leaf>` → `<repo>/themes/<leaf>` (the latter via baked
`CARGO_MANIFEST_DIR`, harmless in production where that path doesn't exist). The
canonical sample theme accordingly lives at `<repo>/themes/community/neon-list/`
so it's auto-discovered in dev.

### D47 (= PD3) — One built-in `DeclarativeShell` renders every declarative theme

A single compiled-in shell interprets the manifest (`views` → layout primitive
via the ARC 2 `resolveLayout`/`useResolvedLayout` machinery) + tokens +
`ThemeBackground` + glyph set + `settings_schema`. Declarative themes ship
**zero code**.

**Why:** reuses every ARC 2 layout primitive + resolver + the S3/S5 token and
per-system substrate + the S4 validator + the swap-by-restart registry — the
disk theme is just new *data* feeding machinery that already exists.

**Status:** the shell itself is **P.1 S2** (not in S1). S1 only parses + exposes
the descriptor; there is no frontend consumer or rendering yet.

### D48 (= PD4) — Themes distribute as the `themes` pack type; no bundled baseline

Disk themes travel the oa-packs channel as a new `themes` pack `type`
(`has_bundled_baseline: false` — built-ins are compiled in; community disk
themes are purely additive, like editorial/CP4). A pack's `manifest.yml`
(oa-packs identity layer) and the theme's `theme.toml` (theme-definition layer)
coexist in the pack zip: the pack installer reads the former, the theme loader
the latter.

**Why:** install / verify / update / rollback / network-gate / Privacy-log all
already work on the channel — `themes` rides them for free. No baseline to ship
because the fallback floor is the *bundled* default theme (D44), not a disk one.

**Status:** wiring `themes` into `default_pack_type_specs` + the Appearance
picker is **P.1 S3**. S1's loader already discovers anything dropped at
`<exe_dir>/themes/community/<id>/`, so a hand-placed folder works today; a
channel-installed `themes` pack lands at the same path once S3 registers the
type.

### D49 — P.1 S2 `DeclarativeShell` implementation calls (recognized-settings vocabulary; dogfood as builtin; flat-browse layout)

**Date:** 2026-06-16 (branch `theme-oatheme-loader-slice-1`). Execution-level
choices made building the generic shell + mapper + dogfood. Subordinate to
D45–D48; recorded so they aren't re-litigated.

1. **Synthetic `entry`/`entry_export` on mapped disk themes.** A declarative
   theme's on-disk manifest omits `entry`/`entry_export` (D45), but the shared
   `ThemeManifest` contract + `validateTheme()` require non-empty strings.
   `diskThemeToPackage` injects placeholders (`"<declarative-shell>"` /
   `"DeclarativeShell"`) that are **never dereferenced** — the entry component is
   always `DeclarativeShell`. *Why:* keeps one `ThemeManifest` type + one
   validator for built-in and disk themes (no parallel "disk manifest validator");
   the placeholders cost nothing and document that the real entry is implicit.

2. **The `DeclarativeShell` interprets a small RECOGNIZED settings vocabulary**,
   not arbitrary `settings_schema` keys. S2 recognizes exactly one — `compactRows`
   (→ list row density), matching hand-coded `bare`. Other declared controls still
   render in the Appearance panel + persist per-theme, but are **inert in the
   generic shell** until the vocabulary accretes. *Why:* a generic shell can't
   divine what an arbitrary key means; settling the minimum against the `bare`
   dogfood and accreting additively (CP3-style) is the plan's stated approach
   (open-question §"How much vocabulary"). The alternative — a rich layout DSL in
   `settings_schema` — is over-build for S2 and trends toward scripting (ARC 3).

3. **The S2 dogfood ships as a BUILT-IN (`themes/declarative-bare/`), not an
   on-disk file.** It builds an inline `DiskThemeDescriptor` → `diskThemeToPackage`
   → `BUILTIN_THEMES`. *Why:* exercises the descriptor→package→render path NOW,
   decoupled from the disk-loading + registry-merge wiring (P.1 S3). S3 swaps only
   the *source* of the descriptor (inline → `oa_themes_list_disk`); the render
   path is identical. It sits beside hand-coded `bare` for A/B parity testing.

4. **Flat all-systems browse resolves layout at the VIEW level**
   (`useResolvedLayout("game-browse", () => null)`), not per focused-item system.
   *Why:* the shell renders one flat list/grid of every game, so there is no
   single system in context; per-system LAYOUT variation (D32) belongs to a
   future system-scoped browse view, not a flat list whose layout would otherwise
   thrash as focus crosses systems. Per-system *palette* (perSystemTokens) still
   applies per-card via `data-system` — that's orthogonal and works today.

---

## 2026-06-16 — ARC 3 (Cinematic & Scripting) planning: the three load-bearing forks

> Plan: [docs/PLANS/theming-arc-3-cinematic.md](../../PLANS/theming-arc-3-cinematic.md).
> Settled with the operator 2026-06-16 in a planning session (prose discussion →
> AskUserQuestion). No code yet; Slice 1 (M1) queued in NEXT.md HIGH band.

### D50 — Rhai scripting is a DEFERRED escape hatch in ARC 3, not an up-front thread

The cinematic *declarative* layer (motion, game/bezel shaders, video, attract
tiers 1–2) ships first; none of it needs scripting. Rhai becomes the final thrust
(Thrust R, possibly its own ARC 3.5), gated behind a `scripting` engine
capability, compiled/power-user tier only, until the sandbox is proven.

**Why:** most of the "wow" lands without scripting; Rhai is the security-heavy
piece (untrusted code execution) and is coupled to the deferred **P.2** CSP/trust
work (D44/D6 — the CSP allowlist becomes load-bearing for the Rhai sandbox).
Front-loading it would mean carrying the riskiest, most architecture-heavy axis
before the high-value, low-risk declarative axes — backwards.

### D51 — Surface split: WGSL chrome targets the wgpu surface (game/bezel/background); UI cinematics are CSS/declarative

OA's single-window shell is a **transparent WebView2 DOM UI composited by Windows'
DWM *over* the wgpu game surface** (main.rs:3625–3640) — the UI is NOT rendered
through wgpu. So shaders target the wgpu surface (game feed + bezel + background —
machinery already shipped: `ApplyShaderPreset`/Phosphor/bezel), and the UI's
cinematic feel is the declarative **motion** layer. "Shaders over the UI" is
rejected.

**Why:** the BigBox-research "one wgpu compositor unifying game + bezels + UI / no
airspace problem" framing is only half-true for OA — it holds for the game
surface, not the DOM UI. Pursuing shader effects across the UI would require
compositing the entire WebView through wgpu, a rearchitecture that fights the
Tauri model for marginal gain. The game-surface **blend-mode compositor** (game +
bezel + background) is still a real differentiator no incumbent offers in the
theme layer (BigBox research §4).

### D52 — ARC 3 cinematics flow into declarative disk themes as DATA

Motion presets, per-view/per-system shader-preset *selection*, and
video-background slots are expressed as fields in `theme.toml`, validated by
`validateTheme`, and honored by the built-in `DeclarativeShell` — so a community
disk (`.oatheme`) theme gets cinematic with zero code. Each ARC 3 thrust extends
the declarative manifest contract first, then its consumer. Rhai (Thrust R), when
it lands, stays gated/compiled-tier — the one cinematic capability that is NOT
declarative.

**Why:** preserves the low-floor/high-ceiling spine ARC 2 + P.1 shipped (disk
themes, `DeclarativeShell`). If ARC 3 cinematics were compiled-tier only, the disk
themes we just shipped would stay visually plain forever and the "let others drop
in rich themes" goal of the `.oatheme` loader would be hollow. The declarative
selection-as-data pattern reuses the ARC 2 resolver cascade (`useResolvedLayout`).

---

## 2026-06-16 — Motion foundation planning session (ARC 3 Thrust M)

The M1 attempt (Slice 1) shipped a sound declarative contract but cost a day to
get one entrance to render on the transparent-WebView build. The operator paused
for a foundation planning session; these resolve its agenda (see
[PLANS/theming-arc-3-cinematic.md](../../PLANS/theming-arc-3-cinematic.md)
§"Motion foundation"). They precede a new **M0 foundation slice** before M1
acceptance / M2.

### D53 — `cargo tauri dev` is the motion-dev loop; `cargo tauri build` is for playtest + final acceptance

Motion/UI-visual iteration runs under `cargo tauri dev` (single-window): Vite HMR
+ live devtools, ~1 s per tweak. Real playtests, anything touching cores or the
real install layout, and final sign-off on any motion still happen in a
`cargo tauri build`. Discipline: **iterate in dev, accept in build.**

**Why:** the day-long M1 tax was the full-build-per-tweak cost, NOT a behavioral
gap between dev and build. The Rust window-builder is identical in both
(`transparent(true)` + `.visible(false)` + DWM compositing — `main.rs`
`setup_single_window`); only the WebView content source differs (devUrl+HMR vs
bundled assets). So *how* an animation composites onto the transparent surface is
byte-identical — WAAPI is invisible in dev too, CSS keyframes paint in both. The
operator's standing build-only habit (see the `operator_uses_cargo_tauri_build`
memory) is correct for gameplay playtests; motion is the one area the build loop
actively fights, so dev is added as the inner loop there. A theoretical divergence
is guarded by the "accept in build" step.

**How to apply:** M0 step 1 is one confirming dev run that the entrance paints.
Use dev for all M/V visual iteration. Caveats that DON'T affect compositing:
custom URI schemes are blocked in the dev WebView (`tauri_custom_uri_schemes_blocked_in_dev`
memory); disk themes resolve under `target/debug` in dev.

### D54 — `oa://window-shown` is the canonical "shell presented" signal; entrance/boot/attract all ride it

The M1 window-ready handshake (Rust creates the window `.visible(false)`; frontend
`oa_shell_ready` → `present_shell_window` shows it + emits `oa://window-shown`; 5 s
timeout fallback) is blessed as the standard pattern. Any entrance / boot / attract
motion keys its first play on `oa://window-shown` rather than a guessed delay.

**Why:** it solves a real ordering bug (the OS presents the window AFTER the
WebView's first paint, so mount-time plays finish unseen) AND kills the launch
white-flash for free. One signal beats per-feature timing guesses across the
remaining cinematic thrusts.

### D55 — Insert an M0 foundation slice; M2 dogfoods on a navigable surface; verification stays lightweight

Before resuming M1 acceptance or starting M2, ship **M0**: the dev loop (D53), a
hash-mounted motion-playground route, `MOTION.md` (compositing catalogue +
scroll-safe rule + windowShown pattern), and an `animationend` dev assertion. Then:
the M1 entrance stays good-enough on `DeclarativeShell`, but **M2 view-transitions
are dogfooded on a navigable surface** (Retroverse routes/tabs, or playground
toggles). Verification stays **lightweight** (playground smoke + the dev assertion);
no screenshot-diff harness.

**Why:** `DeclarativeShell` is single-surface with no runtime view changes — its
only transition trigger is the entrance, so it can't exercise M2's premise
(per-view/per-system transitions on view change). Proving M2 there would be the
wrong archetype on the wrong surface. The scroll-container glitch (open problem #6)
traces to `ViewTransition` wrapping the `overflow-y-auto` nav directly — animating a
scroll container's parent creates a new containing block/stacking context; the rule
is to animate an inner non-scrolling wrapper. Screenshot-diff can't catch "fired but
DWM didn't composite" anyway (only the eye can), so the human-in-the-loop playground
is the pragmatic guard for a one-person craft loop.

### D56 — Motion foundation VALIDATED: no compositing ceiling; the M1 WAAPI finding is reversed (2026-06-17)

The M0 bench (compositing probe + choreography + showcase + box-art FX,
`frontend/src/dev/`) ran on the confirmed real surface and proved **every animation
technique composites** on OA's transparent single-window WebView: CSS `@keyframes`,
CSS transitions, rAF-driven transforms, **WAAPI**, GPU-layer promotion
(`will-change`/`translate3d`), `filter`, and `backdrop-filter`. High-refresh
confirmed (144 fps, no rAF cap). Operator playtested all four benches incl. real
cover art with reflection/shadow + glass finish and called it the "true yes."

**The M1 "WAAPI doesn't composite" finding (the basis for going CSS-only) is
formally REVERSED.** It was a misdiagnosis: M1's WAAPI was a one-shot entrance
played at mount, and the OS presents the window ~hundreds of ms after first paint,
so it finished unseen. The fix that worked was the `oa://window-shown` handshake
(D54), not the WAAPI→CSS switch. WAAPI is available; WAAPI-based libraries (Motion
One, spring engines) are viable.

**Consequences:** (a) "beat BigBox" motion is reachable on this stack — the
program-halting risk is not present. (b) The toolkit is open; pick per need — CSS
for declarative theme-authored transitions, rAF/springs for physics, WAAPI for
computed/imperative moves. (c) `ViewTransition` stays CSS because it's *declarative
data*, not because WAAPI is broken (comment corrected). (d) The dev bench is the
ongoing motion-authoring surface; its keeper effects feed the declarative motion
model (D52). Catalogue + interpretation: `MOTION.md`.

**Open caveat:** the M1 declarative entrance still carries `[oa-theme-motion]`
diagnostics and isn't feel-tuned — fold both into the declarative-motion-model work,
not a separate pass.
