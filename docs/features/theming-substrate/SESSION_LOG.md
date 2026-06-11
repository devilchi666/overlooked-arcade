# Theming Substrate — Session Log

Each session that touches this feature appends a 3-line entry:
**Shipped / Almost / Next.** Older entries roll to
`SESSION_LOG_ARCHIVE.md` when this file passes ~150 lines
(rolled 2026-06-10 after S3 merge — all of Phase 4 + Phase 4.5 + the grab-bag
drain + the Phase 3 design conversation are in the archive; live file keeps the
Phase 3 build arc: S1 nav foundation / S2 walking skeleton / S3 token layer).

---

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

