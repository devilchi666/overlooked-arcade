# Theming ARC 2 — Per-System Layout Substrate

**Status:** Planning (design locked 2026-06-15). No code this session.
**Predecessor:** ARC 1 (Minimum Viable Substrate) — complete bar the
`.oatheme` loader, which this arc absorbs. See
[PLANS/theming-substrate.md](theming-substrate.md) for ARCs 1–4 and the
fixed-input decisions D1–D33 in
[features/theming-substrate/DECISIONS.md](../features/theming-substrate/DECISIONS.md).
**This arc:** ARC 2 of 4 (renumbered this session — see D35).

---

## 0. TL;DR

ARC 2 makes **"each system gets a polished, dedicated home"** literally true
at the *layout* level, not just colour + assets. A theme can declare a
different **layout per system per view** (TG-16 → wheel, Lynx → grid,
Vectrex → vector-glow list), the user can **override** that per system at
runtime (and it persists), and per-system UI stops being a forced-global
behaviour and becomes a **per-theme opt-in capability**. The signature
per-system *content* (SFX, backgrounds, boot animations, the Vectrex custom
view) is **Retroverse's content**, not a platform default every theme
inherits.

This is **Thrust A** — the declarative half of the old ARC-2. The cinematic /
scripting half (declarative motion → Rhai → WGSL) is split out to **ARC 3**
(D35). Theme Studio is **ARC 4**.

Everything here is declarative — **no scripting, no shaders.** It extends the
"resolve-by-active-system" plumbing S5.1 (assets) and S5.2 (palette) already
established up to *layout/primitive choice*. Incremental seam, not a rebuild.

**Fixed inputs (do not relitigate):** D32 (per-system layout becomes a
substrate contract; end-user override is persisted), D33 (per-system UI is a
platform capability themes opt INTO; the residual defect is the forced-global
tile/SFX path). D34 + D35 are added by this plan (below).

---

## 1. Why this arc exists — the D32/D33 story

The BigBox competitive research
([features/theming-substrate/BIGBOX_RESEARCH_2026-06-11.md](../features/theming-substrate/BIGBOX_RESEARCH_2026-06-11.md))
found that BigBox's most-valued, hardest-to-match feature is **per-platform
differentiation** — "the Nintendo home looks nothing like the Sega home." It
achieves this three ways: per-platform *theme* assignment, per-platform *view*
selection, and filename-keyed *asset* resolution.

OA already matches the asset trick (S5.1 cascade) and the recolour
(S5.2 `perSystemTokens`). **The missing piece is per-system *layout/view*
choice as a first-class substrate contract.** ARC 1's D19 deliberately scoped
per-system theming as Retroverse-only ("don't over-build the machinery before
it's needed") — and that was correct *through ARC 1*. ARC 2 is when it's
needed: **D32 expands/supersedes D19** — per-system layout becomes a substrate
contract, theme-declared and user-overridable.

D33 then corrects how per-system UI is *consumed*: today tile flourishes +
per-system SFX are forced cross-theme through the shared `platform` grid,
gated only by a single global `perSystemUiEnabled` toggle. Any theme using the
shared grid inherits Retroverse-flavoured per-system tiles/SFX whether it
wants them or not. The fix: **consumption becomes a per-theme opt-in** (the
capability stays in platform; backgrounds already work this way — `<ThemeBackground>`
is mounted only by CoverFlow).

---

## 2. The capability / content ownership line (D34)

This is the load-bearing architecture of the arc. Draw the line once, here:

| Concern | Owner | Where it lives |
| --- | --- | --- |
| Layout resolver + cascade | **Platform** | `platform/theme/` (new) |
| Layout primitives (list/grid/carousel/**wheel**/custom) | **Platform** | `platform/nav/primitives/` (S5.5 set + WheelNav body) |
| `views` / `per_system` manifest contract + validator | **Platform** | `platform/theme/{manifest,validate}.ts` |
| User-override store | **Platform** | new persistence namespace |
| Factual per-system data (palette/accent) | **Platform** | `platform/themes/systemPalettes.ts` (S5.2 — already split) |
| Asset cascade tiers + SFX dispatcher + boot framework | **Platform** | `platform/themes/*`, `system_ui_assets.rs` |
| Thin `_baseline` per-system fallback | **Platform** | the cascade's `_baseline` tier (already exists) |
| **Per-system layout *choices*** (TG-16→wheel…) | **Theme** | the theme's manifest `views[].per_system` |
| **Per-system SFX banks / backgrounds / boot animations** | **Theme** | `assets/themes/<id>/system-ui/<system>/…` (S5.1 theme tier) |
| **The Vectrex custom view, tile flourishes, signature feel** | **Theme** | the theme's own code/content |

**Consequence (the migration this arc does):** the shipped platform-global
`systemUIConfigs.ts` currently holds *experiential* per-system choices
(`layout` / `audioProfile` / `interactionStyle` / `tileShape`) for all 40
systems as a platform default — exactly the "forced cross-theme" defect D33
names. ARC 2 **migrates the experiential config out of platform-global into
Retroverse-owned declaration.** Retroverse, the comprehensive flagship,
naturally carries the full 40-system set; CoverFlow carries none (backgrounds
only); `bare` carries none. Platform keeps only the thin `_baseline` fallback
so a theme that opts in without authoring 40 configs still renders *something*.

**Rule of thumb:** *factual data + machinery = platform; experiential design +
content = theme.* Recorded as **D34**.

This is also why the paused **Per-System UI Stage 1 pilots (slices 6–9)** and
**Stages 2–3** re-home here as **Retroverse content**, not platform behaviour
(L6). The "signature per-system experiences" are Retroverse's expression of
the platform capability — every other theme gets the capability and ships its
own, or nothing.

---

## 3. Reconciling the old Per-System UI plan

[PLANS/per-system-ui.md](per-system-ui.md) predates the theming substrate and
its **foundational stance is superseded** by D32/D33/D34:

| Old per-system-ui framing | ARC-2 replacement |
| --- | --- |
| "Per-system UI is the **DEFAULT** OA experience" | It's a **per-theme opt-in capability** (D33). Retroverse opts in fully; the default theme being Retroverse means the *out-of-box* experience is unchanged, but the *mechanism* is opt-in, not global. |
| "Mode 1 Themed / Mode 2 No-theme, via a **global master toggle**" | The "mode" is **which theme you run** + that theme's opt-in level. The global `perSystemUiEnabled` toggle becomes a per-theme capability **+** a user **master-off** (the accessibility / reduced-motion / low-end escape — kept). |
| "All per-system character is **hardcoded in-tree**, no theme format" | Per-system character is **theme content** in the theme's tree (D34). The substrate *is* the theme format. |
| Stale paths: `frontend/src/themes/systemUIConfigs.ts`, `<SystemBackground>`, `VirtualLibraryGrid` SFX | Current: `platform/themes/systemUIConfigs.ts`, `<ThemeBackground>` (S5.5 replaced `SystemBackground`), grid SFX via the gated dispatcher. |

The old plan's *content specs* (pilot SFX character, asset sourcing strategy,
boot-animation policy, the GB/NES/Vectrex deliverable lists) stay useful — they
become L6's content brief, scoped as Retroverse content. The old plan's
*architecture* (modes, global toggle, hardcoded-no-format) is retired. L6 of
this plan supersedes per-system-ui Stages 2–3; the per-system-ui feature
folder is annotated to point here.

---

## 4. The view-type + layout model

**View types** — the distinct screens of the library journey. A theme composes
+ styles these. ARC-2 enum (extensible, creator-grade from day one):

- `manufacturer-browse` — pick a manufacturer (Nintendo / Sega / …)
- `system-browse` — pick a system within a manufacturer / flat
- `game-browse` — the library grid/wheel/list of games (the primary view)
- `game-details` — a single game's detail surface

(Reserved for later honoring, declared now so the contract is stable:
`home`, `collections-browse`, `now-playing`. The validator accepts the full
enum; the engine *honors* a growing subset — same "reserve the contract, defer
the body" pattern as S5.5's WheelNav and D20b's `surfaces`.)

**Layout primitives** — the S5.5 set is the seed: `list` (ListNav), `grid`
(GridNav), `carousel` (CarouselNav), `wheel` (WheelNav — reserved stub, built
in L4), `custom` (CustomNav escape hatch). A theme picks a primitive per view,
and **may vary it per system**.

**Manifest shape (the L2 contract):**

```ts
views?: {
  [view in ViewType]?: {
    layout: LayoutPrimitive;                 // theme's default for this view
    per_system?: Partial<Record<SystemId, LayoutPrimitive>>;  // overrides
  };
};
```

Authored as a typed object in ARC 2 (read from `theme.toml` once the loader
lands, P). Validated per the S4 pattern: known view types, known primitives,
known system ids; a malformed `views` block is a **disqualifying error** (a
broken layout map is worse than none) — mirrors `settings_schema` validation
(THEME_CONTRACT.md §8).

**Resolution cascade (L3)** for "which primitive renders `view` for
`systemId`?":

```
user override (theme_id, system_id, view)   ← persisted, highest
  → theme views[view].per_system[system_id]
  → theme views[view].layout
  → engine default for view                  ← lowest
```

Mirrors the S5.1/S5.2 "resolve by active system" cascades exactly, with the
user-override tier on top — the same shape as D18's nav-bindings (user agency
over a theme's defaults, persisted per-user).

---

## 5. Slice plan (contracts-first; D33 fix pulled forward)

Each slice ends in an operator-playtestable milestone, validator/tests green,
one real consumer. Branch-per-arc per the feature-branch workflow (one branch,
commit/push freely, merge at playtestable milestones).

### L1 — D33 consumption opt-in *(the keystone, pulled forward)* — ✅ SHIPPED + MERGED to main 2026-06-15, operator playtested (DECISIONS D36)

> Built as designed: manifest `per_system_ui?: { tiles?, sfx? }`, App-bridged into
> `systemUiSound.ts` gates (`consumesPerSystemTiles`/`Sfx` = userMaster AND theme),
> Retroverse opts in, CoverFlow/bare uniform, user master kept as off-switch.
> Validator `INVALID_PER_SYSTEM_UI` warning. typecheck/lint/vitest(114)/build green.

Convert the global `perSystemUiEnabled`-gated tile/SFX path in the shared grid
into a **per-theme opt-in capability**, matching how backgrounds already
behave. A theme declares how much per-system UI it consumes (Retroverse: full;
CoverFlow: backgrounds only; `bare`: none). Keep a **user master-off** as the
accessibility / reduced-motion / low-end escape (the one legitimate survivor of
the old global toggle).

- Manifest gains a `per_system_ui` capability declaration (shape TBD at slice
  planning — likely a small struct: `{ tiles?: bool, sfx?: bool,
  backgrounds?: bool, boot?: bool }` or a coarse enum; resolve in slice
  planning). Validated.
- `LibraryTile` (tileShape/interactionStyle) + grid-nav SFX dispatch read the
  **active theme's** opt-in, not a global flag. App bridges the active theme's
  capability into the gated paths (same App-bridge pattern as S5.3's glyph set
  / S5.1's ambient themeId).
- The user master-off remains, gating *above* the per-theme opt-in.

*Why first:* it's the concrete user-visible defect, it doesn't depend on the
new view contract, and it establishes the **platform-capability /
theme-consumption split** every later slice builds on.

**Gate:** the same library reads per-system (tiles + nav SFX) under Retroverse
and **uniform** under CoverFlow / `bare`; the user master-off still forces
uniform under Retroverse.

### L2 — split into L2a (contract) + L2b (D34 migration) — DECISIONS D37

The plan's original L2 bundled the additive contract with the consumer-touching
migration; split for independent playtestability (the S4→S5 pattern).

#### L2a — View/layout contract (schema only) — ✅ SHIPPED on branch 2026-06-15, CI-green

- Added the `ViewType` + `LayoutPrimitive` enums + `VIEW_TYPES`/`LAYOUT_PRIMITIVES`
  allow-lists + the manifest `views?: { [view]: { layout, per_system? } }` block
  (§4) to `manifest.ts`. Extended `validateTheme` (malformed = ERROR) + 8 Vitest
  cases. **No consumer** — the L3 resolver is the first reader; built-ins omit
  `views`. Pure additive, zero visual change. typecheck/lint/vitest(122)/build green.

**Gate:** validator unit + builtin-themes tests green (built-ins validate with no
`views`); CI-only (no visual change → no playtest beyond an optional boot smoke-test).

#### L2b — D34 `systemUIConfigs` migration — ✅ SHIPPED + MERGED to main 2026-06-15 (with L2a; operator playtested visual-identical) (DECISIONS D38)

> Built as designed: experiential per-system config → `themes/retroverse/systemUiConfigs.ts`
> via `ThemePackage.perSystemUiConfigs` (peer of `perSystemTokens`); App bridges it;
> `uiConfigFor` merges over `BASELINE_UI`; `touchInputSupported` split to platform-factual
> `systemSupportsTouch` (3 touch consumers repointed). typecheck/lint/vitest(131)/build green.

- **Migrate the experiential per-system config** out of platform-global
  `systemUIConfigs.ts`: `touchInputSupported` is the *only* factual field (hardware:
  has-touchscreen; gates stylus/touch overlays regardless of theme) → **stays
  platform**; everything else (layout / audioProfile / interactionStyle / tileShape /
  …) is experiential → moves to `themes/retroverse/`, **bridged into the tile/SFX
  consumers via the L1 opt-in pattern** (only read when the theme opts in). Platform
  keeps a thin `_baseline` fallback.
- Behavior-preserving — Retroverse renders identically to today.

**Gate:** validator + builtin-themes green; **operator visual-identical playtest** —
Retroverse per-system tiles/SFX unchanged; CoverFlow/bare unaffected.

### L3 — split into L3a (resolver + store) + L3b (consumer + UX) — DECISIONS D39

Split for the same contracts-first reason as L2: the plumbing is clean/testable;
the consumer raises a real UX question (resolved per-system layout vs the
existing global capsule/list `viewMode` toggle).

#### L3a — Resolver + persisted override store — ✅ SHIPPED on branch 2026-06-15, CI-green

- Pure `resolveLayout` over the §4 cascade (**user override → theme `per_system`
  → theme view default → engine default**) + a reactive `useResolvedLayout(view,
  systemId)` hook + `ENGINE_DEFAULT_LAYOUTS` (`layoutResolver.ts`).
- The **user-override store** (`layoutOverrides.ts`): the `(theme_id, system_id,
  view) → layout` namespace — **localStorage**, theme-id-keyed, `createStore`-
  reactive (the D28 per-theme-settings pattern; survives the restart swap).
- **No consumer** — CI-gated, no visual change. typecheck/lint/vitest(142)/build green.

#### L3b — first live consumer (game-browse) + the viewMode UX call *(NEXT)*

- Wire `useResolvedLayout("game-browse", selectedSystemId())` into `LibraryView`
  (which already computes `selectedSystemId()` + already switches grid↔list on
  `viewMode`). Retroverse declares a couple of per-system layout variations to
  prove the cascade end-to-end.
- **Settle the UX fork:** how the per-system resolved layout relates to the
  existing global capsule/list `viewMode` toggle (supersede vs coexist). `wheel`
  stays the L4 stub.

**Gate (L3b):** switching the active system on the browse view changes the
resolved primitive per Retroverse's declared map; with no override set, the theme
default wins; **operator visual playtest.**

### L4 — WheelNav implementation

Build the reserved radial **WheelNav** primitive (S5.5 shipped only the typed
contract + a warn-once stub — "reserve the contract, defer the body"). It now
has a consumer: a per-system layout can select `wheel`. The BigBox-signature
parity piece (HyperSpin-style wheel is the aesthetic reference per the
research).

- Verb-native + declarative-props like the other primitives; `onNavSound`
  hook; windowing for large libraries (reuse CarouselNav's windowing
  discipline, D29.1).

**Gate:** a system whose resolved layout is `wheel` renders a navigable radial
wheel (browse + launch); falls back cleanly where unsupported.

### L5 — End-user override UI

The runtime **"pick your view per system"** surface (the D32 user-agency
headline), writing the L3 override store. Recommend it lives in the **engine
Per-System Settings Hub** (the shipped card-based Systems hub is the natural
home — a "Layout" domain card per system) with a per-row Reset, mirroring the
D30 nav-remap card's reset discipline. (Alternative: a theme-territory quick
action. Settle at slice planning; the hub is the recommendation — it's
engine-owned, theme-agnostic, and already controller-navigable.)

**Gate:** user overrides a system's `game-browse` layout → it persists across a
restart; Reset restores the theme default.

### L6 — Re-home Per-System UI Stage 2/3 as Retroverse content/consumption

Rebuild the paused per-system-ui **behaviour layer (Stage 2)** + **experience
layer (Stage 3)** as ARC-2 work **consumed by Retroverse**, built *into* the
substrate capability — NOT as engine-global behaviour (D33/D34). Includes:

- Per-system interaction feel / tile emphasis / focus-ring style as Retroverse
  content + opt-in.
- The pending **Stage 1 pilots (GB / NES / Vectrex, old slices 6–9)** as
  **Retroverse content** — SFX banks, backgrounds, boot animations, the Vectrex
  custom view (a Retroverse-private `CustomNav` consumer), per-core README
  "Per-system UI" notes. *Content production, not new architecture* — can ride
  this slice or stay parked as a content task (operator call at the time).
- Stage 3 in-game overlay theming / library↔game transitions / per-system
  metadata priorities — scope-gate at slice planning; some may defer to ARC 3
  (transitions overlap the cinematic axis).

**Gate:** Retroverse shows ≥1 fully-realised per-system experience (a pilot)
end-to-end; a non-opting theme is unaffected.

### P — Runtime `.oatheme` loader (closes the last ARC-1 thread)

The deferred §6 Phase 5 work: on-disk discovery + extract + **runtime dynamic
`import()`** of a theme entry from an extracted `.oatheme` zip, plus the
**CSP allowlist** that ARC 1 deferred (D6 — `tauri://localhost` breaks
out-of-bundle dynamic imports without it). The build-time-bundled half already
shipped (S2 active-theme machinery + Phase 6); P adds the loose-folder /
zip path.

- Rust `theme_loader.rs`: discover `<exe_dir>/themes/<id>/` folders + `.oatheme`
  zips, validate manifest (schema + oa_version + capabilities), extract to
  cache, expose discovery to the frontend registry.
- Frontend: dynamic-import the active on-disk theme's entry; the S4 validator
  runs on it (now the "untrusted author" deferred-gap in THEME_CONTRACT.md §6
  starts to matter — note it, don't fully close it; full source-scan hardening
  is an ARC-3/4 concern).
- Conflict/failure policies per §6 Phase 5 (duplicate ids, active-theme load
  failure → fallback + the persistent banner S4 deferred, capability gap →
  refuse + message).

*Placed last in ARC 2* because it's the natural bridge to ARC 3: D6 notes the
CSP work "becomes load-bearing for Rhai sandboxing anyway," so P tees up ARC 3.

**Gate:** drop a built theme folder into `<exe_dir>/themes/`, restart, see it
in Settings → Themes, switch to it, see the UI change.

---

## 6. What's explicitly NOT in ARC 2

Pushed to **ARC 3 — Cinematic & Scripting** (D35):

- Declarative motion / transitions (the reserved `motion` token category) —
  even though much of it is CSS-achievable (D23.6), it's grouped with the
  cinematic axis so ARC 2 stays a clean declarative-layout arc.
- `<video>` backgrounds / attract mode (D20a seam is reserved; the feature is
  ARC 3).
- Rhai sandboxed behaviours.
- WGSL shader chrome (D20 CRT/shaders).
- Full media-slot vocabulary expansion (BigBox `ImageTypes` checklist,
  research §D) — rides ARC 3's media binding.

Pushed to **ARC 4 — Theme Studio**: the visual editor (Model A round-tripping,
research §6/§F). Unchanged.

Out of scope entirely this arc: multi-monitor `surfaces` beyond `main` (D20b —
contract reserved, no engine support); per-section user-selectable view
*variants within one view* (research §2 — parked; ARC 2 does per-system layout,
not N-variants-per-section).

---

## 7. Decisions added this session

- **D34** — Per-system *capability* is platform; per-system *content +
  experiential choices* are theme. The shipped platform-global
  `systemUIConfigs.ts` experiential config migrates to Retroverse-owned
  declaration; platform keeps the factual data + a thin `_baseline`. (§2.)
- **D35** — Arc separation + renumber: ARC 2 = Per-System Layout Substrate
  (this plan), ARC 3 = Cinematic & Scripting (old ARC-2 behaviours+shaders +
  declarative motion), ARC 4 = Theme Studio (old ARC 3). The `.oatheme` loader
  moves into ARC 2's tail.

Both recorded in
[features/theming-substrate/DECISIONS.md](../features/theming-substrate/DECISIONS.md).

---

## 8. Verification posture

Per-slice: `npm run typecheck` + `npm run lint` (the boundary zones must stay
green — D34's migration must not create a `platform → themes` edge; per-system
*content* moving to Retroverse is correct, the *capability* staying in platform
is correct) + `npm run test` (Vitest — validator + any new resolver units) +
`npm run build`; `cargo test -p oa-shell` for the Rust-touching slices (L1 SFX
gating, P loader). Operator playtest at each gate.

The validator (S4) is the drift-stopper for the new contract: `views` /
`per_system` validation lands in L2 and the builtin-themes Vitest gate keeps
Retroverse/CoverFlow/`bare` honest.

---

## 9. Critical files (anticipated)

- `platform/theme/manifest.ts` — `ViewType` / `LayoutPrimitive` enums + `views`
  block + `per_system_ui` capability (L1/L2).
- `platform/theme/validate.ts` + `validate.test.ts` — contract gate (L1/L2).
- `platform/theme/layoutResolver.ts` (new) + `useResolvedLayout` (L3).
- `platform/theme/layoutOverrides.ts` (new) — user-override store (L3).
- `platform/nav/primitives/WheelNav.tsx` — body (L4).
- `platform/themes/systemUIConfigs.ts` — experiential split (L2/D34).
- `themes/retroverse/` — `views` map + migrated per-system config + pilot
  content (L2/L3/L6).
- `engine/` Per-System Settings Hub — layout override card (L5).
- `apps/oa-shell/src/theme_loader.rs` (new) + frontend loader + CSP (P).
- `frontend/src/platform/components/LibraryTile.tsx` + grid SFX dispatch —
  per-theme opt-in (L1).
