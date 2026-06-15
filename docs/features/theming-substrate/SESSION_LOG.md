# Theming Substrate — Session Log

Each session that touches this feature appends a 3-line entry:
**Shipped / Almost / Next.** Older entries roll to
`SESSION_LOG_ARCHIVE.md` when this file passes ~150 lines
(rolled 2026-06-10 after S3 merge — all of Phase 4 + Phase 4.5 + the grab-bag
drain + the Phase 3 design conversation are in the archive; live file keeps the
Phase 3 build arc: S1 nav foundation / S2 walking skeleton / S3 token layer).

---

## 2026-06-15 — ARC 2 L2a: view/layout manifest contract (schema only) — ✅ shipped on branch, CI-green (no consumer, no visual change)

> Branch `feat/theming-arc2-l2a-view-layout-contract`. L2 split into **L2a (contract,
> additive)** + **L2b (the D34 migration)** — operator sign-off via AskUserQuestion.
> This is L2a: stamp the view/layout contract + validator, exactly like S4 stamped
> the manifest before S5 consumed it. DECISIONS **D37**.

- **Shipped:** `manifest.ts` gains `ViewType` (manufacturer-browse / system-browse /
  game-browse / game-details) + `LayoutPrimitive` (list / grid / carousel / wheel /
  custom) + `ViewLayoutConfig` + `ThemeViews` + the `VIEW_TYPES` / `LAYOUT_PRIMITIVES`
  allow-lists + a `views?: ThemeViews` manifest field (per-view default layout +
  optional `per_system` overrides, D32). `validateTheme` validates it (known view
  types / primitives / system-ids; malformed = ERROR like settings_schema —
  `INVALID_VIEWS` / `UNKNOWN_VIEW_TYPE` / `INVALID_VIEW_LAYOUT`, reusing
  `UNKNOWN_SYSTEM_ID`). THEME_CONTRACT §1 row added. **No consumer** — the L3
  resolver is the first reader; built-ins omit `views` (still validate clean).
- **Verified:** typecheck + lint green; `npm run test` = **122 passed** (8 new views
  validator cases); build green. Frontend-only. Contract-only → no visual change, so
  no playtest beyond CI (optional smoke-test: app still boots, themes still switch).
- **Almost:** nothing in L2a scope.
- **Next:** **L2b — the D34 migration:** move the experiential `systemUIConfigs` map
  (layout/audioProfile/interactionStyle/tileShape/…) into `themes/retroverse/`,
  bridge it into the tile/SFX consumers (the L1 opt-in pattern), keep
  `touchInputSupported` factual in platform. Behavior-preserving → visual-identical
  playtest gate.

## 2026-06-15 — ARC 2 L1: per-system-UI consumption opt-in (the D33 fix) — ✅ shipped + MERGED to main (operator playtested)

> Branch `feat/theming-arc2-l1-per-system-opt-in`. The keystone ARC-2 slice (pulled
> forward): convert the forced-global per-system tile/SFX path on the shared grid
> into a per-theme opt-in. Shape signed off (AskUserQuestion — `{tiles,sfx}` struct)
> before code. DECISIONS **D36**.

- **Shipped:** manifest gains `per_system_ui?: { tiles?, sfx? }`; `systemUiSound.ts`
  adds the per-theme consumption layer (`setThemePerSystemUi` + `consumesPerSystemTiles`/
  `consumesPerSystemSfx` = userMaster AND theme-opts-in); App.tsx bridges
  `activeTheme()?.manifest.per_system_ui` (mirrors the glyph-set bridge); `LibraryTile`
  + `VirtualLibraryGrid` + `playSystemUiSound` consume the new gates instead of the raw
  master toggle. **Retroverse declares `{tiles:true, sfx:true}`**; CoverFlow + bare
  declare nothing → uniform grid. User master toggle kept as the global off-switch.
  Validator: `INVALID_PER_SYSTEM_UI` warning (fallback OFF). THEME_CONTRACT §1 row added.
- **Verified:** `npm run typecheck` + `npm run lint` green; `npm run test` = **114 passed**
  (incl. new `systemUiSound.test.ts` 5 gate tests + 2 validator cases); `npm run build`
  green. Frontend-only — Rust resolvers untouched.
- **Almost:** nothing in L1 scope. (LibraryTile's old "perSystemUiEnabled OFF" code
  comment left as harmless historical wording.)
- **Next:** **operator playtest** the acceptance gate — Retroverse per-system as before;
  CoverFlow + bare uniform; user master-off forces uniform under Retroverse. Then merge.
  **After: L2** — view/layout manifest contract + the `systemUIConfigs` experiential→theme
  split (D34).

## 2026-06-15 — ARC 2 planned: Per-System Layout Substrate (D32/D33 → plan + D34/D35) — no code

Planning session. Designed ARC 2 with D32 (per-system layout becomes a
substrate contract) + D33 (per-system UI is a platform capability themes opt
into) as fixed inputs; wrote the plan + two new decisions.

- **Shipped (docs):**
  [PLANS/theming-arc-2-per-system-layout.md](../../PLANS/theming-arc-2-per-system-layout.md)
  — the ARC-2 plan: capability/content ownership line (§2), reconciliation of
  the stale per-system-ui plan (§3), the view-type + layout-primitive model +
  resolution cascade (§4), and the **L1→P slice order** (L1 D33 opt-in pulled
  forward as the keystone · L2 view/layout contract + systemUIConfigs split ·
  L3 resolver + persisted user override · L4 WheelNav body · L5 end-user
  override UI · L6 re-home Per-System UI Stage 2/3 as Retroverse content · P
  `.oatheme` runtime loader). DECISIONS **D34** (factual data + machinery =
  platform; experiential design + content = theme — the shipped global
  `systemUIConfigs.ts` experiential config migrates to Retroverse) + **D35**
  (arc separation/renumber: ARC 2 = layout, ARC 3 = cinematic/scripting [old
  ARC-2 Rhai+WGSL+motion], ARC 4 = Theme Studio).
- **Key operator calls this session:** separate the layout arc from the
  cinematic/scripting arc (D35); pull the D33 consumption-opt-in fix forward as
  L1; the per-system pilot *content* (GB/NES/Vectrex) belongs to the Retroverse
  theme, not to every theme (drove D34).
- **Almost:** n/a (planning only; no build).
- **Next:** **L1 — D33 consumption opt-in** (queued NEXT.md HIGH). ARC 1's only
  residual thread (the `.oatheme` loader) is absorbed as ARC 2's tail slice P.

## 2026-06-11 — BigBox theming research + D32 (per-system layout becomes a substrate contract) — no code

Research-and-decisions session. A 4-agent BigBox theme-system study →
`BIGBOX_RESEARCH_2026-06-11.md`, then walked the §8 open questions with the
operator.

- **Shipped:** `BIGBOX_RESEARCH_2026-06-11.md` (architecture / per-platform
  model / community loves-hates-wants / visual-editor landscape / cinematic
  axis — fully cited). DECISIONS **D32**: per-system *layout* variation + a
  view-type library + per-view layout primitives (mix-and-match per
  manufacturer/system/game) + **end-user runtime override (persisted)** become
  a first-class substrate capability **in ARC 2** — expands/supersedes D19.
  Theme Studio (ARC 3) stays after ARC 2. Research §8 marked resolved.
- **Almost:** n/a (decisions only; no build).
- **Next:** an ARC-2 plan for the D32 capability (merges with the paused
  Per-System UI Stage 2/3). ARC 1 still finishes on its Phase-5 `.oatheme`
  loader line first.

## 2026-06-11 — Phase 6: Retroverse rebuilt as a real theme (the ARC-1 acceptance gate) — ✅ shipped + merged (merge `711f337`; operator playtested — indistinguishable)

> Branch `feat/theming-retroverse-as-theme`. The ARC-1 closer / dogfood: move Retroverse from the
> S2 thin wrapper (D22.8) into a REAL theme physically living under `themes/retroverse/`, consuming
> ONLY platform, and remove the last two boundary exceptions. DECISIONS **D31**. Design-first: the
> reverse-import audit + move plan were signed off (two AskUserQuestion forks) before any code.

- **The reverse-import audit (the one real snag) found ZERO files needing to hoist to platform.**
  Every Retroverse file consumed by a non-Retroverse surface was *already* hoisted by the S2 /
  Phase-4 / grab-bag work (host context → `platform/theme/host`; LeftSidebar / LibraryView /
  VirtualLibraryGrid / EngineSummonIcon → `platform/components`; stores + api → platform). So Phase 6
  collapsed to a **pure physical relocation** + one shim deletion — no platform hoist, no new module.
  The flagship needing *no* new sharing is itself proof the boundary was drawn right in past arcs.
- **Shipped** (three green sub-commits on one branch):
  - **C1 — sever the context shim** (`2ff021c`): repoint every importer of `routes/retroverse/context`
    (App.tsx `ThemeProvider`, RetroverseShell `useTheme`/`themePreempted`, the 6 route pages `useTheme`)
    directly to `@oa/platform/theme/host`; delete the shim (its content moved to platform in S2).
  - **C2 — relocate** (`b5c6508`): `git mv` (history preserved) RetroverseShell + the five route pages
    + GameDetailPanel + SystemInfoPanel + `currentRoute.ts` (theme-private tab routing, §10) into
    `themes/retroverse/`; repoint RetroverseShell's 5 page imports + the two `currentRoute` importers to
    local `./siblings`; `index.tsx` → `./RetroverseShell` + header rewritten (no longer a thin wrapper);
    empty `layout/retroverse/`, `routes/retroverse/`, `routing/` dirs gone. **Dead code removed:**
    `StubPage.tsx` (zero importers since the real pages shipped) + App.tsx's `__retroverse_debug` DevTools
    block (obsolete — predates Retroverse's real tab strip; a future dev-console seam belongs in platform,
    queued in PARKING_LOT). App.tsx now reaches Retroverse only via the sanctioned `registerThemes()` edge.
  - **C3 — drop the exceptions** (`7381ddd`): remove `except: ['./retroverse']` from the `themes↛routes`
    and `themes↛layout` ESLint zones + update the header comment. **Probe-verified:** a throwaway
    `routes/` + `layout/` import from `themes/retroverse/` fires both `import/no-restricted-paths` errors
    (the old `except` would have allowed exactly that), then reverted. **Every theme — Retroverse included
    — is now platform-only with zero exceptions.**
- **Verified:** `npm run typecheck` + `npm run lint` + `npm run test` (**58 passed**) + `npm run build`
  green at each sub-commit. Frontend-only — no Rust (cargo unaffected). Two forks signed off before code
  (AskUserQuestion): the obsolete `__retroverse_debug` block → **delete** + queue a platform dev-console
  seam; `StubPage` → **delete** (dead).
- **Almost:** nothing in Phase-6 scope left.
- **Merged to main `711f337`** after operator playtest (indistinguishable — boot / browse / launch /
  F12 Settings / per-system theming / CoverFlow swap-and-back all confirmed identical). **This closes
  the ARC-1 acceptance gate** — the SDK is proven to host the flagship with zero boundary escapes.
- **Next:** the only remaining ARC-1 work is the original §6 **Phase 5** (`.oatheme` on-disk
  distribution/loader). Operator picks when to start it.

## 2026-06-11 — Phase 3 follow-on (D18): nav-remap Settings UI (gamepad) — ✅ shipped + merged (operator playtested)

> Branch `feat/theming-nav-remap-settings`. The D18 follow-on after the S2 swap gate — the
> Settings surface that edits the OA-wide shell-nav `navBindings` map (built in S1). DECISIONS
> **D30**. This is the **menu/UI** nav remap, NOT the per-system gameplay bindings.

- **Shipped:**
  - **`NavRemapCard`** in `engine/SettingsSections.tsx` — a "Button mapping" card rendered inside
    the existing **Controller navigation** Settings category (Controls), below the A/B-swap
    toggle. One `SettingRow` per action/structural verb (Confirm / Back / Secondary / Tertiary /
    Previous-section / Next-section / Menu / Quick-settings) with a `select` of the remappable
    physical buttons (A/B/X/Y/LB/RB/LT/RT/Start) + "— Unbound —". Edits the **live `navBindings`
    signal**, so dispatch + every on-screen glyph update **instantly (no restart)** and persist to
    `nav_bindings.json`. Per-row **Reset** (to that verb's default button) + a card-level **Reset
    to defaults**.
  - **Pure remap helpers** in `navBindings.ts`: `rebindGamepadVerb(bindings, verb, button)` —
    one-button-per-verb with **conflict resolution** (assigning a button steals it from whatever
    verb had it → that verb's row re-renders as Unbound) + `rawGamepadButtonForVerb` (no-swap
    lookup the UI shows/edits, vs the swap-aware `buttonForVerb` the HintBar paints).
  - **Escape-hatch + validation (D18):** the keyboard arrows + native Enter/Esc are NOT editable
    here, so a user can never strand themselves with no way to confirm/back. A soft amber warning
    appears when Confirm or Back has no gamepad button (still reachable via keyboard).
- **Verified:** `npm run typecheck` + `npm run lint` green; **`npm run test` = 58 passed**
  (51 + 7 new `navBindings.test.ts`: raw lookup, fresh-bind, move-clears-old, conflict-steal,
  unbind, no-mutation); `npm run build` green. Frontend-only.
- **Scope = GAMEPAD only (deliberate).** The operator's follow-up question surfaced that the
  **keyboard** channel binds `KeyboardEvent.key`→verb directly (no A/B/X/Y step) and the dispatch
  is **already wired** (`focus.ts:214`) — only the editing UI is missing. Gamepad is fully
  covered (the browser standard-layout makes "A/B/X/Y" the real buttons). The **keyboard nav-remap
  UI + a real default keyboard map + the future per-controller-ID gameplay-binding auto-config**
  are documented as a TODO in **PARKING_LOT.md (2026-06-11 entry)** — it lives in platform Settings
  so it can land anytime without blocking. Operator chose to merge the working gamepad card now.
- **Shipped + merged** after operator playtest. **This closes the last Phase-3 thread** (remaining
  ARC-1 work: the original §6 Phase 5 `.oatheme` distribution + Phase 6 full Retroverse-as-theme
  move). The keyboard remap is a queued follow-on (PARKING_LOT), not a Phase-3 blocker.

## 2026-06-11 — Phase 3 S5.5: primitives (carousel/custom + reserved wheel + nav-sound + background revival) — ✅ shipped + merged (merge `105fad8`) — **CLOSES S5 + the Phase-3 substrate-depth arc**

> Branch `feat/theming-s5-5-primitives`. The LAST S5 micro-slice — closes the Phase-3
> substrate-depth arc. DECISIONS **D29**. Five parts; the CoverFlow refactor is the
> playtest-risky one (it rewires a shipping theme onto the new primitive).

- **Shipped** (plan §13.3 S5 primitives + #6 + the S5.1 background fold-in):
  - **`CarouselNav`** (`platform/nav/primitives/CarouselNav.tsx`) — generalizes CoverFlow's
    hand-rolled windowed coverflow: windowing (±`window`), centring track shift, per-card
    position/scale/opacity/z-index, horizontal `useFocusGroup`, wheel-browse, click-to-centre,
    late-claim (moved IN from CoverFlow's manual force-claim). Card content is the theme's
    render-prop; ctx adds signed `offset`. **CoverFlow dogfooded onto it** — its bespoke track /
    windowing / `<style>` / wheel handler deleted; it now supplies only cover content + the
    preload buffer + the footer + the shared-selection mirror.
  - **`CustomNav`** — the high-ceiling escape hatch: hands the theme a focus API
    (`{ items, focusedIndex, setFocusedIndex, isActive, activate, bind }`) via a render-prop so
    an arbitrary layout still gets verb-nav + hints. Supersedes the long-deleted
    `customComponent` field.
  - **`WheelNav`** — RESERVED: the typed radial-wheel prop contract + a stub that renders
    nothing + warns once (no ARC-1 consumer → no half-built radial dead code; the contract is
    the expensive part, deferred impl).
  - **`onNavSound` hook (#6)** — added to `NavPrimitiveBaseProps` (coarse `NavSoundEvent`:
    move/confirm/back/secondary); wired into ListNav/GridNav/CarouselNav (fires on the verb
    callbacks + a focus-move effect). Engine default `navSoundDispatcher((item) => item?.systemId)`
    lives in `platform/themes/systemUiSound` (maps event→`UiSoundEvent`, routes through the
    existing gated per-system dispatcher). nav stays decoupled (callback, not a built-in dispatch).
    **CoverFlow wires it** as the live consumer.
  - **Background-surface revival (S5.1 fold-in)** — the dead `SystemBackground` (unmounted since
    2026-05-31, zero importers) **deleted + replaced** by `ThemeBackground`
    (`platform/components/ThemeBackground.tsx`): a generic theme-opt-in backdrop consuming the
    **S5.1 background resolver tier** (theme→platform cascade, ambient theme id), no
    `perSystemUiEnabled` gate, no accent gradient (the backdrop is the theme's own image) + a
    legibility scrim. **CoverFlow mounts it** → S5.1's background tier finally has a live consumer
    (drop `assets/themes/coverflow/system-ui/_baseline/backgrounds/default.png` → it paints).
  - THEME_CONTRACT.md §3 expanded (the 5 primitives + `onNavSound` + `ThemeBackground`).
- **Verified:** `npm run typecheck` + `npm run lint` green; **`npm run test` = 51 passed**
  (49 + WheelNav-stub + navSoundDispatcher; the focus/DOM behavior of the primitives is
  playtest-verified — no Solid render harness is set up, matching the existing pure-test
  approach); `npm run build` green. Frontend-only.
- **Almost:** nothing in S5.5 scope. Deep component tests for the primitives would need a Solid
  render harness (not set up) — noted as a possible future infra add.
- **Playtest round 1 (2026-06-11) — two bugs found + fixed on-branch:**
  - *CoverFlow backdrop painted ABOVE the box art.* CoverFlow passed `class="absolute inset-0"`
    to CarouselNav whose root is already `relative` (conflicting position utilities) and the
    `z-0` backdrop wasn't cleanly behind. Fixed: CarouselNav gets `z-10 h-full w-full` (no
    position conflict; explicitly above the `z-0` ThemeBackground). The middle row is
    `relative overflow-hidden`.
  - *`bare` list couldn't move (arrows did nothing) + clicking a game launched it — so it could
    never become the focus.* Root cause: the **late-claim was only in CarouselNav**, not
    ListNav/GridNav. A late-mounting whole-shell list (bare mounts after the async theme seed)
    never claimed the active focus slot, so only a mouse-click (→ launch) worked. (Latent since
    S4 — bare's earlier playtests used the mouse.) Fixed: extracted **`useLateClaim`**
    (`primitives/lateClaim.ts`) and applied it to **ListNav / GridNav / CarouselNav / CustomNav**
    — every list-like primitive now claims once items appear. typecheck/lint/vitest(51)/build
    green after the fix.
- **Playtest round 2 (2026-06-11) — CoverFlow confirmed good; one more bare bug fixed:**
  *the bare list moved the selection but didn't scroll, so the focused row walked off-screen.*
  The rows are `tabindex=-1` + framework-focused (not native DOM focus) → the browser does no
  scroll-into-view. Added a per-row effect that `scrollIntoView({ block: "nearest" })` when a row
  becomes focused (ListNav + GridNav; CarouselNav already centres via its track transform).
  typecheck/lint/vitest(51)/build green.
- **Next:** **operator re-playtest** — bare: arrows/D-pad move the list AND keep the selection
  on-screen, Confirm launches. Then merge. **That closes S5 + the Phase-3 substrate-depth arc.**

## 2026-06-11 — Phase 3 S5.4: per-theme settings namespace — ✅ shipped + merged (merge `895f8c0`; combined S5.3+S5.4 playtest passed)

> Branch `feat/theming-s5-4-theme-settings` (off main, on top of merged S5.3 — operator
> chose to build S5.4 on S5.3 + playtest both together, one merge). Fourth S5 micro-slice.
> DECISIONS **D28**.

- **Shipped** (plan §13.3 S5 item #9):
  - **`platform/theme/themeSettings.ts`** — a collision-free per-theme prefs namespace.
    `getThemePref`/`setThemePref` over a Solid `createStore` backed by **one localStorage key**
    (`oa.themeSettings` → `{ [themeId]: { … } }`; frontend-owned, survives the restart-based
    swap). **`useThemeSettings()`** returns `{ get, set }` **auto-bound to the active theme's
    id** — a theme never names an id, so it can only touch its own slice (the binding IS the
    collision rule). `get` is reactive (createStore read).
  - **Live consumer = `bare`** (the test bed): a header **"Compact" toggle** writes
    `themeSettings.bare.compactRows` and the list density reacts live; persisted, so it
    survives switching away + back. bare now demos all three S5 seams (per-system dots /
    PS glyphs / theme pref).
  - THEME_CONTRACT.md **§7** added (the fourth settings namespace alongside
    OA-wide / per-system / per-game).
- **Verified:** `npm run typecheck` + `npm run lint` green; **`npm run test` = 49 passed**
  (45 + 4 new `themeSettings.test.ts`: fallback, set→get round-trip, slice isolation,
  localStorage persistence — the runner ships a partial `localStorage` stub so the persistence
  test installs a working in-memory one); `npm run build` green. Frontend-only.
- **Almost:** nothing in S5.4 scope.
- **Next:** **operator playtest S5.3 + S5.4 together** — switch to `bare`: hint bar shows
  ✕/◯/□/△ (Launch = ✕; S5.3), and the header **Compact** toggle changes row density +
  survives a switch-away-and-back (S5.4). Switch to Retroverse → hints read **A**. Then merge
  both. **After: S5.5 — primitives (carousel/custom + reserved wheel + background-surface
  revival)** — the last S5 slice.

## 2026-06-11 — Phase 3 S5.3: glyph-set seam (manifest field + PS set) — ✅ shipped + merged (merge `af13cb7`; combined playtest rides with S5.4)

> Branch `feat/theming-s5-3-glyph-set`. Third S5 micro-slice. The verb→glyph
> indirection already existed (S1 `glyphs.ts`); S5.3 makes it CHOOSABLE with a real
> consumer. DECISIONS **D27**. Scope-call #4 = seam + one alternate set; picker deferred.

- **Shipped** (plan §13.3 S5 item glyph-set #4):
  - **`glyphs.ts`**: `PLAYSTATION_GLYPH_SET` (✕/◯/□/△ + Options ≡ / Create ⊟ / PS),
    `GlyphSetId` (`"xbox"|"playstation"`), `GLYPH_SETS` registry, `DEFAULT_GLYPH_SET_ID`,
    and the **`activeGlyphSet()` signal** + `setActiveGlyphSetId(id)` (unknown/undefined →
    default). HintBar now paints via `activeGlyphSet()` (reactive — switching set repaints
    every hint, same free-update as a remap).
  - **Manifest `glyph_set?: string`** (loose, like `routes` — keeps the manifest type
    decoupled from the nav layer). **App.tsx bridge** `createEffect(() =>
    setActiveGlyphSetId(activeTheme()?.manifest.glyph_set))` — mirrors the S1 settings→nav
    bridges (`setSwapAB`, `setPerSystemUiEnabled`). nav stays a generic leaf; App injects.
  - **Validator**: `UNKNOWN_GLYPH_SET` — a **WARNING**, not an error (a cosmetic glyph
    mismatch must not disqualify a whole theme; hints fall back to xbox). THEME_CONTRACT.md
    §1/§3/§6 updated.
  - **Live consumer = `bare`** (the test bed): its manifest declares
    `glyph_set: "playstation"`, so bare's HintBar Launch hint reads **✕** while Retroverse
    keeps the default **A**. bare now demos BOTH S5.2 (per-system dots) + S5.3 (PS glyphs).
- **Verified:** `npm run typecheck` + `npm run lint` green (lint confirms `platform/theme →
  platform/nav` is allowed); **`npm run test` = 45 passed** (37 + 2 validator glyph cases +
  6 new `glyphs.test.ts`: set completeness, PS symbols, registry, `activeGlyphSet`
  default/switch/fallback, verb→button→glyph per set); `npm run build` green. Frontend-only.
- **Almost:** nothing in S5.3 scope. The user-facing **glyph picker + controller
  auto-detect** stay deferred (scope-call #4) — the seam + the bridge make them a drop-in.
- **Next:** **operator playtest** — switch to `bare`, confirm the hint bar shows ✕/◯/□/△
  (Launch = ✕) and Retroverse still shows A. Then merge. **After: S5.4 — per-theme settings
  namespace.**

## 2026-06-11 — Phase 3 S5.2: palette substrate (typed map + override seam) — ✅ shipped + merged (merge `f5b9b61`)

> Branch `feat/theming-s5-2-palette-substrate`. Second S5 micro-slice. Extracts the
> 46 per-system palettes from hand-authored CSS to a typed single-source map +
> derives the baseline at boot + adds the per-theme override seam. DECISIONS **D26**.
> Palette data-home: **typed TS map**, NOT `config/*.json` + a build step (operator
> AskUserQuestion call — per-system palette is frontend-only data with no Rust reader).

- **Shipped** (plan §13.3 S5 item 1, refined):
  - **`platform/themes/systemPalettes.ts`** — typed `SystemPalette` (`accent`/`soft`/`glow`)
    + `SYSTEM_PALETTES: Record<SystemId, SystemPalette>` (all 46 systems; `glow` derived as
    accent@0.35α — the invariant; one-line identity notes, full hue rationale in git
    history) + `PALETTE_VAR` (key→CSS var, peer of `TOKEN_VAR`). **The single source of
    truth** — `Record<SystemId,…>` forces a palette per system (the parity guarantee the old
    `systems.css` comment asked for by hand).
  - **`systems.css` RETIRED** (deleted; `@import` removed from `index.css`). The global
    `[data-system]` baseline CSS is now **derived from the map at boot** —
    `ensureSystemPaletteBaseline()` injects a `<style id="oa-system-palettes-baseline">`
    into `document.head` in `index.tsx` **before first render** (no flash; runtime
    equivalent of the static import). Idempotent + DOM-guarded (no-op in tests).
  - **Per-theme override seam** (`ThemePackage.perSystemTokens?: PerSystemTokens` — D19's
    optional sub-cascade): App.tsx renders a `<style>` of `perSystemOverrideCss(".oa-theme-mount", …)`
    INSIDE the theme mount (now classed `oa-theme-mount`). Rule shape
    `.oa-theme-mount [data-system="<id>"]{…}` — higher specificity than the global baseline →
    wins inside the theme; engine territory (a SIBLING of the mount) keeps the baseline (the
    structural D2 guarantee, same as the S3 token scope). A system-agnostic theme ships none.
  - **Validator extended** (D24): `perSystemTokens` system ids ∈ `SystemId`, sub-keys ∈
    `SystemPalette`, values non-empty (`UNKNOWN_SYSTEM_ID` / `UNKNOWN_PALETTE_KEY` /
    `EMPTY_PALETTE_VALUE`). THEME_CONTRACT.md §4 + §6 updated (the `systems.css` reference
    in §4 was stale — now points at `systemPalettes.ts`).
- **Verified:** `npm run typecheck` + `npm run lint` green; **`npm run test` = 37 passed**
  (25 + 4 new perSystemTokens validator cases + 8 new `systemPalettes.test.ts` —
  baseline-CSS shape, scoped-override specificity/partial/empty, systemThemes parity, glow
  invariant); `npm run build` green (CSS bundle −7 kB; the baseline left the static bundle).
  Frontend-only — Rust untouched (830 oa-shell tests hold).
- **Live override demo — `bare` reframed as the substrate TEST BED (operator call):** rather
  than distort Retroverse (the default) or re-add per-system colour to the system-agnostic
  CoverFlow, the operator chose `bare` as the consumer — "eventually we'll do a list-view theme
  properly; bare is the test bed." So `bare` now renders a **per-system accent dot** per row
  (`data-system` → `--color-system-accent`) AND ships a scoped `perSystemTokens` override
  recolouring **NES → cyan** + **PSX → magenta**. In bare those rows read the demo colours; in
  engine territory (Settings → Per-system) NES/PSX keep their baseline red/teal — the D19
  sub-cascade + D2 sibling-scope, **visible**. `validateTheme(bare)` still passes (valid
  perSystemTokens; `bare.tokens` stays undefined so the "no design-token overrides" fixture
  assertion holds). All gates re-run green after the bare change.
- **Next:** **operator playtest S5.2** — (1) **baseline parity:** per-system colours
  (Retroverse tiles, Settings per-system drill-in, system-edged toasts) are **identical** to
  before; (2) **override demo:** switch to `bare`, see NES rows' dots cyan + PSX dots magenta
  while Settings keeps NES red / PSX teal. Then merge. **After: S5.3 — glyph-set seam.**

## 2026-06-10 — Phase 3 S5.1: resolver theme tier — ✅ shipped + merged (merge `783da2e`)

> Branch `feat/theming-s5-1-resolver-theme-tier`. First of **five S5 micro-slices**
> (operator chose per-sub-area slicing; order = contracts first). Adds the **theme
> tier** to the two existing per-system resolvers — generalize/connect the shipped
> Per-System-UI machinery into the theme cascade, NOT rebuild it. DECISIONS **D25**.
> **Merged on the test basis** (830 tests + no regression) without a live visual
> playtest, because the **background consumer `SystemBackground` is currently
> unmounted** (dropped 2026-05-31 over a Retroverse visual conflict; `<SystemBackground`
> appears in zero JSX). The resolver tier is correct + tested either way; **reviving a
> theme-owned background surface folds into S5.5** (operator call). The **ui-sound** half
> has a live consumer (grid-nav sounds) and is exercisable now.

- **Shipped** (plan §13.3 S5 item 2 + scope-call #6 first half):
  - **Rust — theme tier on both resolvers.** `resolve_background_asset` +
    `resolve_ui_sound` gain a leading `themeId: Option<String>` param and now walk a
    **theme→platform cascade** via a new shared helper
    `system_ui_assets::candidate_asset_bases(assets_dir, theme_id, system_id, category)`
    (both are `oa-shell` modules, so the cascade isn't duplicated). Order:
    *(ui-sound only)* operator override → **theme/<system>** → **theme/_baseline** →
    system/<system> → system/_baseline → null. Background uses the same minus the
    operator-override tier. Theme overrides home at
    **`<exe_dir>/assets/themes/<themeId>/system-ui/<systemId>/<category>/`** (operator-
    droppable TODAY — no Phase-5 loader needed; mirrors the existing `assets/system-ui/`
    layout). The theme **`_baseline`** tier lets a system-agnostic theme (D19) ship one
    theme-wide backdrop/cue instead of 45 per-system copies. `asset_slug_is_safe` shared
    + applied to both theme id (skips just the theme tier) and system id (refuses the
    resolve).
  - **Frontend — ambient threading, zero consumer churn.** The api wrappers
    (`mediaApi.resolveBackgroundAsset`, `shellApi.resolveUiSound`) take `themeId` first
    (pure typed pass-through, D14). The **dispatchers** resolve it ambiently —
    `lib/audio.ts::dispatchUiSound` and `SystemBackground`'s `resolveBackgroundUrl`
    read `activeThemeId()` and pass it down — so grid nav / boot animation / the
    background component (every consumer) are **unchanged**.
- **Verified:** `cargo test -p oa-shell` = **830 passed** (822 baseline + 8 new
  theme-tier cascade tests, incl. theme-overrides-per-system, theme-`_baseline`-wide,
  fall-through, and unsafe-theme-id-skips-only-the-theme-tier, for both resolvers).
  `npm run typecheck` + `npm run lint` green; `npm run test` = 25 passed;
  `npm run build` green.
- **Almost:** nothing in S5.1 scope left. (The #6 verb→sound **hook in the primitives**
  rides S5.5, where the primitives are built.)
- **Folded into S5.5 (operator call 2026-06-11):** revive a **theme-owned background
  surface** (the dead `SystemBackground` has no mount) so the S5.1 background resolver
  tier gains a live consumer — a theme opts into a backdrop layer (fits the `custom`
  primitive / a theme-opt-in background). Until then the background tier is
  ready-but-unconsumed; the sound tier is live.
- **Next:** **S5.2 — palette substrate** (typed `SYSTEM_PALETTES` single-source map,
  retire `systems.css`, per-theme `perSystemTokens` scoped override seam + validator
  extension). Then S5.3 glyph-set · S5.4 settings namespace · S5.5 primitives (+ the
  background-surface revival).

## 2026-06-10 — Phase 3 S4: versioned manifest + load-time validator (`bare` fixture) — ✅ shipped + merged (operator playtested)

> Branch `feat/theming-manifest-validator`. Turns THEME_CONTRACT.md §6 from a
> documented contract into a machine-checked one — the strict foundation a forgiving
> theme-creation tool (ARC 3) needs. Four S4 design forks signed off (AskUserQuestion,
> all the recommended path) before code. DECISIONS **D24**.

- **Shipped** (plan §13.3 S4 + scope-calls #2/#7):
  - **`validateTheme(pkg)`** (`platform/theme/validate.ts`) — pure, never-throws; returns
    `{themeId, ok, errors, warnings}`. Checks the **declarative surface** (manifest +
    typed `tokens`): required fields, `schema_version` ∈ `SUPPORTED_SCHEMA_VERSIONS`
    (`{1}`; "newer schema — update OA" vs "unsupported" messages), `surfaces` non-empty ⊆
    `HONORED_SURFACES` (`["main"]`), `required_engine_capabilities` ⊆ `ENGINE_CAPABILITIES`
    (**empty in ARC 1** → only `[]` validates), `tokens` keys ∈ `TOKEN_VAR` + values
    non-empty. The token-key check is the data half of the §4 no-override rule. Warnings:
    non-dir-safe `id`, `default_route` ∉ `routes`.
  - **`SUPPORTED_SCHEMA_VERSIONS` + `MAX_SCHEMA_VERSION`** added to `manifest.ts`.
  - **Registry gate** (`registry.ts`): `registerThemes` validates each theme, **excludes
    invalid ones** from the valid set (picker + `activeTheme()` resolve over valid only);
    errors logged always, warnings DEV-only. `setActiveTheme` guards on the valid set. New
    **fallback toast** in `initActiveTheme`: a persisted id that's no longer a valid choice
    (e.g. wheel→coverflow) falls back to the default AND raises a `warn` toast naming it.
  - **`bare` theme** (`themes/bare/index.tsx`, added to `BUILTIN_THEMES`) — the minimal
    valid whole-shell: a plain `ListNav` of games + launch-on-Confirm + `EngineSummonIcon`,
    **no tokens**, system-agnostic, ~110 LOC. Operator-selectable (the "low floor" made
    switchable) AND the validator's canonical fixture (one artifact, both jobs).
  - **Vitest — the frontend's first test runner** (there was none; the gate had to be TS
    since manifests are TS objects with no Rust visibility, D6). `vitest` + `jsdom` +
    `vitest.config.ts` (reuses `vite-plugin-solid` + the `@oa/platform` alias) +
    `npm run test` wired into CI between lint + build. An `overrides:{vite}` pin dedupes
    vitest's nested vite (Solid-plugin type clash). Two suites: `platform/theme/
    validate.test.ts` (15 pure unit tests — every error/warning code via crafted
    fixtures) + `themes/builtin-themes.test.ts` (10 — every bundled theme validates clean,
    `bare` is the minimal fixture, ids unique; lives in `themes/` because validating real
    themes means importing them and `platform ↛ themes` is forbidden).
  - **THEME_CONTRACT.md §6 rewritten** — enforced-now (data) vs backed-structurally
    (sibling-scope + boundary lint) vs deferred (the `<style>:root`/`document.head`/global-
    CSS bypass a package-object validator can't see; Phase-5/untrusted-author concern).
- **Verified:** `npm run typecheck` + `npm run lint` green; **`npm run test` = 25 passed**
  (2 files); `npm run build` green (bare bundled). Frontend-only — no Rust; 822 oa-shell
  tests unaffected. **Merged to main `6fb0653` after operator playtest passed.**
- **Decisions (D24):** validator = declarative-surface gate, NOT a runtime `:root`
  boundary (structural sibling-scope is); Vitest CI is the hard drift-stopper; `bare`
  ships in the picker as fixture+reference; fallback = toast+console (Phase-5 persistent
  banner deferred); schema = supported-set `{1}`.
- **Almost:** nothing in S4 scope left.
- **Next:** **S5 — substrate depth** (palette substrate, asset/`ui-sound` resolver, glyph-set seam,
  per-theme settings namespace, remaining `wheel`/`carousel`/`custom` primitives). The
  nav-remap Settings UI stays the separate D18 follow-on.

## 2026-06-10 — Phase 3 S2: walking skeleton (Retroverse ⇄ CoverFlow swap gate) — ✅ shipped + merged

> **The morale/de-risk milestone — the dream first becomes visible.** Branch
> `feat/theming-walking-skeleton`. Four S2 design decisions signed off
> (AskUserQuestion, all the recommended path) before any code.

- **Shipped** (all 5 S2 scope items + the two D20 seams + the boundary ratchet):
  - **Theme SDK contract** (`platform/theme/types.ts`): a theme = `{ manifest, entry }`;
    the entry is `Component<{ surface: "main" }>` (surface-aware, D20b) consuming ONLY
    platform (usePlatform stores + the host context + `@oa/platform/nav` + `@oa/platform/api`).
  - **Host context → platform** (`platform/theme/host.tsx`): `ThemeContextValue` /
    `ThemeProvider` / `useTheme` moved out of `routes/retroverse/context.tsx` (now a
    re-export shim — D15-style, ~11 importers unchanged) so EVERY theme consumes the
    same launch/saves/info/favorite host services. Adds `themePreempted()` — the
    general D20a preempt/restore seam (= `engineSurfaceOpen()` today; attract reuses it).
  - **Active-theme registry** (`platform/theme/registry.ts`): platform owns the
    `activeThemeId` signal + boot seed + picker list + `setActiveTheme` (persist→restart);
    App injects the concrete `BUILTIN_THEMES` via `registerThemes()` (platform ↛ themes,
    so App is the injection point — D13 pattern). Persisted on
    `LibraryPrefs.active_theme_id` (boot-loaded). App.tsx renders the active theme via
    `<Dynamic component={activeTheme().entry} surface="main"/>` (was hardcoded
    `<RetroverseShell/>`), gated on `activeThemeResolved()` (no default flash).
  - **Restart**: new Rust `restart_app` command via Tauri 2 `AppHandle::restart()`
    (no new plugin; mirrors `quit_app` cleanup) + `shellApi.restartApp()`.
  - **Retroverse = thin wrapper** (`themes/retroverse/index.tsx` → existing
    `RetroverseShell`; layout/routes stay put, full move is Phase 6). Default theme.
  - **Wheel = rough 2nd shell** (`themes/wheel/index.tsx`): full-bleed horizontal
    **coverflow** — centred scaled focused cover, neighbours fanning + dimming, metadata
    strip + Launch button below, Left/Right browse, Confirm launch, Game-info on
    Secondary. System-AGNOSTIC by choice (D19). Built on the S1 `ListNav` primitive
    (horizontal, controlled index) + `usePlatform` + `useMedia` covers. Honest caveat
    baked into the code + picker: layout/feel only — attract/CRT/ceremony is ARC 2-3.
  - **`EngineSummonIcon` re-homed** `engine/` → `platform/components/` (D12 — a leaf
    themes must mount belongs to the lowest consuming layer); both themes mount it, the
    operator's always-available path back to Settings → Themes. RetroverseShell's L1/R1
    gate switched to `themePreempted()`.
  - **Appearance picker**: filled in the existing OA-wide **Themes** Settings category
    (`ThemesSettings` in `engine/SettingsSections.tsx`) — lists registered themes,
    Switch button → in-app confirm → persist + restart. Stale Legacy-Shell card removed.
  - **`surfaces` field** added to `ThemeManifest` (D20b); **6 new lint zones**
    (platform↛themes, engine↛themes, themes↛{engine,routes(except retroverse),
    layout(except retroverse),components}); `themes↛engine` probe-verified to fire.
- **Verified:** `npm run typecheck` + `npm run lint` green; `cargo test -p oa-shell`
  = **822 passed** (incl. the `library_prefs` round-trip now carrying `active_theme_id`).
- **Decision D22** recorded (the 9-point implementation shape + the two
  most-easily-undone constraints: platform-owns-machinery/App-injects, and the
  retroverse lint `except`).
- **Almost:** nothing in S2 scope left. The nav-remap Settings UI is still the
  separate follow-on (after this gate, per D18).
- **Next (operator):** **playtest the swap gate** — boot (lands on Retroverse),
  browse + launch; F12 → Settings → Themes → Switch to Wheel → confirm → app restarts
  into the Wheel coverflow → browse + launch a game → switch back to Retroverse →
  indistinguishable from before. Then merge. **After merge: S3 — token layer**
  (`THEME_CONTRACT.md` + design tokens + a11y/motion baseline + engine-territory token
  isolation), per plan §13.3.

## 2026-06-10 — Phase 3 S3: token layer (design-token contract) — ✅ shipped + merged

> Branch `feat/theming-token-layer`. Preceded by a **BigBox research round** (operator
> asked us to get on the same page on what BigBox themes actually do before S3):
> confirmed the cinematic/motion axis (animation engine, transitions, video snaps,
> attract, Theme Creator) is the heart of **ARC 2-3**, not the token layer. Operator
> chose to keep **S3 strictly static**. Three S3 design forks signed off (all
> recommended) before code.

- **Shipped** (the static token contract per plan §13.3 S3 + scope-calls #1/#3):
  - **Token contract** (`platform/theme/tokens.ts`): typed `ThemeTokens`
    (palette / typography / geometry) + `TOKEN_VAR` map (key → CSS var) +
    `themeTokensToCssVars()`. **Formalizes the EXISTING** `index.css` CSS-variable
    system — does not reinvent it. Motion (`--motion-*`) is deliberately **reserved**
    (documented, not a theme axis yet — ARC 2).
  - **Override mechanism**: `ThemePackage` gains `tokens?: Partial<ThemeTokens>`;
    App.tsx injects them as CSS custom properties **scoped to the S2 theme-mount
    wrapper** (the `isolate` div). The engine surface is a *sibling* of that wrapper →
    scoped tokens can't reach it → **engine territory always reads `:root` (the D2
    guarantee, structural)**. Same token NAMES, different SCOPE — no namespace split.
  - **A11y/motion baseline** (NOT theme-overridable): a global
    `prefers-reduced-motion` reset collapsing `--motion-*` + neutralizing
    transitions/animations app-wide; `focusRing` formalized as `--oa-focus-ring`
    (default = accent, per-system-aware, theme-overridable) and consumed by the
    `[data-oa-focus]` ring.
  - **CoverFlow re-skinned through tokens** (minimal-but-distinct): a cool
    steel-blue/cyan token set vs Retroverse's warm default — same component, visibly
    different shell, ZERO code change. Retroverse ships no tokens (pure `:root`).
  - **`THEME_CONTRACT.md`** written — the theme-facing peer of SURFACES.md (token set +
    engine-reserved guarantee + verb vocab + manifest schema + surfaces + reserved-motion
    note + what the S4 validator checks).
- **Verified:** `npm run typecheck` + `npm run lint` green. Frontend-only (no Rust) —
  822 oa-shell tests unaffected. **Decision D23** recorded.
- **Playtest round (operator confirmed, merged `340c3fe`):** two refinements rode along
  before merge — (1) **CoverFlow cohesion:** art-less games were rendering loud
  per-system-coloured placeholder boxes (the cards carried `data-system`, pulling the
  per-system accent). Dropped `data-system` from CoverFlow cards + footer so every
  accent resolves to the THEME's own token (cyan) — a cohesive uniform identity, and
  the token reskin now actually reads. Made missing-art placeholders subtle (faint glow
  + title text) so they recede. This is the **D19 distinction made concrete**: CoverFlow
  = one uniform identity, Retroverse = per-system worlds; a theme opts out of per-system
  colour simply by not emitting `data-system` (no substrate change). (2) **Image-preload
  buffer:** render window stays ±8 cards, but covers are warmed ±24 into the browser
  cache via off-DOM `Image()` loads (deduped) → smooth fast-scroll, no placeholder flash.
- **Almost:** nothing in S3 scope left.
- **Next:** **S4 — versioned manifest + validator** (the `bare` theme fixture; the
  load-time validator that checks a theme against THEME_CONTRACT.md — turns the contract
  from documented into machine-enforced, the strict foundation a forgiving theme-creation
  tool needs). Design-first proposal before code.

### S2 playtest round 1 (2026-06-10) — fixes + rename

- **Operator playtested; swap gate WORKS** (Retroverse ⇄ CoverFlow, browse + launch
  both). Three bugs found + fixed on the same branch, then re-confirmed working:
  - *Covers painted over the Settings surface* (z-index). The theme mount in App.tsx
    is now wrapped in an `isolation: isolate` stacking context — a theme's internal
    z-indexes can never escape above engine territory / platform modals. Substrate
    guarantee, applies to every theme.
  - *Controls did nothing.* The theme mounts late (async pref seed, behind a Show), so
    its `ListNav` focus group never claimed the active slot. Rebuilt the coverflow on
    `useFocusGroup` **directly** with an explicit `group.activate()` once games load.
  - *Perf:* `ListNav` rendered all 8541 game nodes (no virtualization). Windowed to ±8
    cards on a sliding CSS track, reconciled by stable RomEntry refs. Also added mouse
    click-to-centre + wheel-scroll so it's usable without controller nav.
- **Renamed the second theme `Wheel` → `CoverFlow`** (id `wheel` → `coverflow`, dir
  `themes/wheel/` → `themes/coverflow/`) per operator: what S2 ships is a coverflow
  IA; a true radial/arc **Wheel** is the separate `wheel` nav primitive, parked for S5.
  *Migration note:* a pref persisted as `activeThemeId:"wheel"` is now unknown → the
  registry falls back to the default (Retroverse) on next boot; re-pick CoverFlow once.
- typecheck + lint green throughout; 822 oa-shell tests unaffected (frontend-only).

## 2026-06-10 — Phase 3 S1: nav foundation (verb-native nav layer) — ✅ shipped + merged

> **Merged to main 2026-06-10** — operator playtested ("working as expected").

- **Shipped** on `feat/theming-nav-foundation` (all four S1 scope items + the
  two recommendations the operator approved — persistence-real + HintBar verb
  re-key + arrow-key keyboard):
  - **Relocated `src/nav/` → `platform/nav/`** (git mv: types/gamepad/back/
    focus/HintBar) + new modules; all **24 importers repointed to
    `@oa/platform/nav`** (one barrel `index.ts`). This **closes the Phase-2
    residual wrong-direction edges** (`platform/components/* → ../../nav/*`):
    those imports are now intra-platform. New ratchet lint zone
    **`platform/nav ↛ platform/components`** keeps the nav layer a generic leaf.
  - **Verb vocabulary** (`verbs.ts`): `Confirm`/`Back`/`Secondary`/`Tertiary` +
    directional `Up`/`Down`/`Left`/`Right` + `PrevSection`/`NextSection`/`Menu` +
    reserved-unbound `OpenQuickSettings`/`Search`/`Favorite`/`Page`. (Operator
    sign-off added `Secondary`/`Tertiary` to the plan's headline set — they're
    the X/Y focused-item roles the focus framework already dispatches.)
  - **Input→verb indirection** (`navBindings.ts`): OA-wide `NavBindings`
    (gamepad + keyboard channels) + `DEFAULT_BINDINGS` = the operator-locked
    controller-nav spec verbatim. Persisted in appData (`nav_bindings.json`) via
    new `platform/api/navBindingsApi.ts` + Rust `get/set_nav_bindings`
    (opaque-JSON blob, mirrors `audio.json`). `resolveButtonVerb` /
    `resolveKeyVerb` / `buttonForVerb` resolvers.
  - **A/B swap collapsed into a binding** — the old `swapAB` special-case is gone
    from `focus.ts`/`HintBar`; it's now a resolve-time overlay in `navBindings`
    (`setSwapAB`/`isSwapAB` moved there). `focus.ts` `routeEvent` resolves
    button→verb then dispatches by verb (`dispatchVerb`); focus-group callback
    names (onActivate/onCancel/…) kept stable so the ~15 consumers don't churn.
  - **HintBar is verb-native**: `Hints` re-keyed from physical buttons to verbs
    (`{ Confirm, Back, Secondary, … }` + `dpad`/`stick` descriptors) across **17
    call sites**; glyphs resolve **verb → currently-bound button → glyph** via
    the glyph-set seam (`glyphs.ts`, scope-call #4). Remap / swap re-paints every
    hint for free.
  - **`list` + `grid` primitives** (`primitives/`) — verb-native, declarative
    props (`density`/`focusProminence`/`easing`/data-source/neighbours, surfaced
    as `data-oa-*` seams for the S3 token layer; scope-call #8). Additive — they
    do **not** replace the bespoke VirtualLibraryGrid/LeftSidebar focus usage;
    they're the surface S2's Wheel/Retroverse skeletons consume.
  - **Keyboard**: arrow keys → directional nav at the focus layer (gated:
    nav-enabled, non-editable target, no Ctrl/Meta/Alt). Confirm already works
    natively on focusable buttons; Enter/Back/Esc keyboard verbs deferred to the
    remap follow-on (need a native-control coexistence audit). Schema carries
    both channels now.
- **Verified:** `npm run typecheck` + `npm run lint` green; `cargo test -p
  oa-shell` = **822 passed**. Operator playtested + merged to main 2026-06-10.
- **Decision D21** recorded (focus-group callback names kept; gamepad bus stays
  raw-event so the engine-summon chord + boot-skip are untouched; keyboard arrows
  emit source "dpad").
- **Behavior to watch in playtest:** arrow-key nav is newly live in the shell —
  arrows now move the active focus group (instead of scrolling) when focus isn't
  in an editable field. Everything else should feel identical (defaults = the
  locked spec).
- **Almost:** the remap Settings UI (the verb-rebinding surface) — deliberately
  the follow-on **after** the S2 swap gate, per D18.
- **Next:** **S2 — walking skeleton:** minimal active-theme switch (restart) +
  Retroverse wrapped as default theme + a rough **Wheel** second shell;
  switchable from Settings → Appearance, both browse + launch. The morale/
  de-risk milestone.

