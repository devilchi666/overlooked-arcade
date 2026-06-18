# Theming Substrate — Session Log

Each session that touches this feature appends a 3-line entry:
**Shipped / Almost / Next.** Older entries roll to
`SESSION_LOG_ARCHIVE.md` when this file passes ~150 lines
(rolled 2026-06-10 after S3 merge — all of Phase 4 + Phase 4.5 + the grab-bag
drain + the Phase 3 design conversation are in the archive; live file keeps the
Phase 3 build arc: S1 nav foundation / S2 walking skeleton / S3 token layer).

---

## 2026-06-18 — ARC 3 Thrust M: wire the motion model into the consumers — 🔄 on `feat/motion-wire-consumers` (NOT merged; awaiting operator playtest)

> The "wire it up right" tail from the preset-registry entry below. The model +
> registry + contract were merged; almost nothing real consumed them (only the lab
> + dev bench). This branch makes the declarative path consume the model and adds
> the diagnostics toggle + disk-theme authoring + global retime. 5 commits, in order.

- **Shipped on the branch (5 commits):**
  1. **Motion-diagnostics dev-tools toggle** (D58.9) — `platform/theme/motionDebug.ts`
     (localStorage signal, default OFF, persisted across the restart-based swap);
     the per-play `[oa-theme-motion]` logs in `SpecTransition`/`ViewTransition` are
     gated on it, and `AmbientMotion` gained a parallel gated line (surfaces
     reduced-motion suppression). Toggle at Settings → About → Developer tools
     (with the other logging on/off buttons — operator correction; kept persisted
     because theme swaps restart the app). Code NOT stripped — toggled.
  2. **Lab refactor** — inline gloss/reflection/fanart → `<Gloss>`/`<Reflection>`/
     `<FanartCrossfade>`; manifest motion authored via the M-mod slots
     (`transition` = inline-spec escape hatch, `ambient: "breathe"` = named preset)
     resolved via `resolveThemeMotionSpec`/`resolveMotionRef`; lab-private keyframes
     shrink to just the title/meta rise. Physics hero spring kept.
  3. **`declarativeShell` migrated** off `resolveViewTransition`+`ViewTransition`
     onto `resolveThemeMotionSpec`+`SpecTransition`. D54 LANDMINE handled: added an
     opt-in `skipInitial` to SpecTransition (suppresses the masked mount play) so
     the `windowShown`-keyed trigger fires the entrance exactly when the OS presents
     the window — no M1 "entrance before window shown" regression. (`delayMs` lived
     only inside `ViewTransition`; declarativeShell always gated via the windowShown
     trigger, so the migration preserved D54 by keeping that trigger + skipInitial.)
     `bareDeclarative`'s legacy `view_transition` fade still resolves + animates (now
     WAAPI). **`ViewTransition` + `resolveViewTransition` now have NO runtime
     consumer** — kept as tested product code per GRAPHICS_LAB_TESTBED.md.
  4. **Rust `theme_loader` widened** — `ThemeMotion` gains `transition`/`selection`/
     `ambient: Option<MotionRef>`; new `MotionRef` = untagged `Preset(String)` |
     `Spec(toml::Value)` (loose pass-through; frontend stays the allow-list/shape
     authority; additive, no `deny_unknown_fields`). Disk themes can now author the
     slots. `diskTheme.ts` carries them for free (DiskThemeManifest = Omit<…>).
  5. **Global `--motion-scale`** reaches the WAAPI players — new `--motion-scale`
     token (index.css, default 1); pure `scaleMotionTiming` (motionSpec) + DOM
     `readGlobalMotionScale` (motion.ts) fold it into the duration/delay both
     players hand to `element.animate` (WAAPI can't read `var()`).
  - **CI:** tsc + eslint clean; 182 vitest green (12 new across motionDebug/
    scaleMotionTiming); `cargo test -p oa-shell theme_loader` green (10).
- **Almost / NOT done:** a declarative **selection/ambient hook** so data themes get
  selection choreography without custom render code — deliberately DEFERRED (the
  no-code renderer is a flat browse; per-item entrance choreography deserves its own
  design, not a rushed add). The motion FEEL of the migrated declarativeShell is
  eye-unvalidated (operator hasn't built since this branch).
- **Next:** operator playtest (cargo tauri build → switch to a declarative/disk
  theme + Graphics Lab; flip the diagnostics toggle on to trace) → merge. Then the
  selection/ambient declarative hook, and the deferred catalog (`push-hero`,
  attract/Thrust V, `path-move`/keyframe-timeline).

---

## 2026-06-18 — ARC 3 Thrust M: the named preset registry + contract wiring — 🔄 on `feat/motion-presets-and-wiring` (NOT merged)

> Finished the audit §3 seed catalog as a NAMED registry themes pick from, and
> wired it into the theme contract (motion as data). Follows the M-mod.1–.4 model,
> which is now MERGED to main (`6b89f50`/`ce6bff6` incl. the showcase).

- **Shipped on the branch:**
  - **Spec-preset registry** (`motionPresets.ts`): 11 named presets across 3 kinds —
    transition (fade/slide/scale/**flip**), selection (lift/art-grow-in/title-rise,
    back-ease overshoot), ambient (breathe/float/**glow-pulse**/ken-burns). Each =
    defaults over the §2 basis + author overrides; `buildPreset`/`MOTION_PRESET_NAMES`.
  - **Basis extensions** (`motionSpec.ts`): `rotateX/rotateY` (3D, perspective auto-
    prepended → flip) + string channels `filter`/`boxShadow` (cinematic → glow-pulse).
  - **Treatments + composites** (`treatments.tsx` + `index.css` keyframes):
    `<Gloss>` `<Shimmer>` `<Reflection>` `<FanartCrossfade>` (+ `useTilt` = parallax-
    tilt) + `staggerMs()` (lift-stagger). Extracted from the lab's inline code →
    reusable by any theme.
  - **Contract wiring** (`manifest.ts`/`motion.ts`/`validate.ts`): `ThemeMotion`
    gains `transition`/`selection`/`ambient` slots, each a `MotionRef` = a preset
    NAME or an inline spec. `resolveMotionRef` (name→buildPreset / spec→passthrough);
    `resolveThemeMotionSpec` prefers `transition`. Validator: ref must be a known
    preset of the right KIND, or a valid spec (`validateSpec` now covers the new
    channels). 162 vitest green; typecheck + lint green.
  - **Catalog status: 17/21 packaged.** Deferred: `push-hero` (shared-element morph),
    `attract-scroll`/`screensaver` (Thrust V).
- **Almost / NOT done (the remaining "wire it up right"):**
  - **`declarativeShell` (the no-code renderer) still on the OLD M1 path** — biggest
    gap. ⚠️ It relies on `ViewTransition`'s `delayMs`/`oa://window-shown` boot-entrance
    timing (D54); migrating it to `SpecTransition` MUST port that or it reintroduces
    the M1 "entrance plays before the window is shown" bug.
  - **Lab still uses inline gloss/reflection/fanart** — refactor to consume the new
    `treatments.tsx` components + author `ambient: "breathe"` by name (proof + de-dup).
  - **Rust `theme_loader` not widened** for `[motion.transition|selection|ambient]`
    (disk themes can't author them yet; built-ins can. No `deny_unknown_fields`, safe).
- **Next:** lab refactor (low-risk) → declarativeShell migration (carefully, port the
  window-shown delay) → Rust widening → fill `glow-pulse`/treatments into the showcase.

---

## 2026-06-18 — ARC 3 Thrust M, M-mod.1–.4: the declarative motion MODEL — ✅ MERGED to main (`6b89f50`); showcase merged (`ce6bff6`)

> The M0 bench keepers are now a real, theme-authorable motion model, dogfooded on
> a new **strip-on-ship Graphics Lab** testbed theme (Settings → Experimental →
> Graphics Lab; hidden from the Appearance picker via an `experimental` flag; one
> folder + 4 `// [GRAPHICS-LAB]` touch-points to strip — see
> `GRAPHICS_LAB_TESTBED.md`). Engine code stays on ship; only the lab demo strips.

- **Shipped (engine first, then the lab dogfood):**
  - **Spring** — `spring.ts` (pure D57 `{bounce,duration}` ↔ `{stiffness,damping,mass}`,
    Apple closed form; default `BENCH_SELECTION_SPRING` is the F10 k=190/damping=24
    back-solve so the default feel == what was signed off at the bench) +
    `springValue.ts` (the bench's rAF integrator, generalized).
  - **§2 basis** — `motionSpec.ts`: separate channels (opacity/x/y/scale/rotate)
    over timing primitives (duration/delay/easing/repeat/direction);
    `compileMotionSpec` → WAAPI; `presetToSpec` (M1 fade/slide/scale = named defaults
    over the basis).
  - **Players** — `SpecTransition` (trigger-driven, interruptible, WAAPI),
    `AmbientMotion` (loops; no-op under reduced motion), `useTilt` (pointer-tilt).
  - **Contract** — `ThemeMotion.view_transition_spec` (spec wins over the preset);
    `resolveThemeMotionSpec` unifies preset+spec; validator covers the spec shape.
    Lab authors its transition in `LAB_MANIFEST.motion` (manifest-as-data path).
  - **Lab dogfood** — all four audit categories on one surface: Home↔Library
    view-transition (140px/560ms slide), selection choreography (spring cover
    grow-in + staggered title/meta on every focus move), breathe ambient,
    pointer-tilt — composed via nested transforms.
  - **Fixes** — route-switch delay (GridNav keep-mounted + capped 60), transient
    2nd scrollbar during the slide (clip ancestor; **MOTION.md rule #2 promoted to
    CONFIRMED**), slide readability (authorable distance/duration).
  - 156 vitest green; typecheck + lint green.
- **Almost:** the FEEL is eye-unvalidated — operator has not built since the first
  slide; choreography/breathe/tilt timings are best-guess.
- **Next:** operator build + feel tune; graduate the spring/selection + ambient
  config into the manifest contract (only `view_transition_spec` graduated so far);
  Rust `theme_loader` widening for `[motion.view_transition_spec]` (disk themes —
  deferred, no `deny_unknown_fields`); audit §3 leftovers (ken-burns / glow-pulse /
  gloss / reflection); migrate `declarativeShell` presets onto `SpecTransition`.

---

## 2026-06-17 — ARC 3 Thrust M, M0: motion bench BUILT + foundation VALIDATED (the "true yes") — ✅ on branch `theme-arc3-motion-slice-1`

> **Headline: there is NO compositing ceiling on OA's transparent surface, and
> BigBox-tier motion is reachable.** The probe answered the program-halting
> question (can the surface even carry rich motion?) with an unqualified yes, and
> the choreography/showcase/box-art benches proved we can make it FEEL premium on
> real cover art. Operator playtested all four benches and called it.

- **Built (dev-only, gated behind `import.meta.env.DEV`; F10 in `cargo tauri dev`):**
  a 4-tab **motion bench** at `frontend/src/dev/` (App.tsx mounts it; never ships):
  - **`MotionPlayground.tsx` — compositing probe.** 12 cells, same motion via
    different techniques on the real surface. **Every cell PAINTS**, incl. WAAPI,
    `will-change`/`translate3d` GPU promotion, rAF transforms, filter,
    backdrop-filter, scroll-container. Live fps meter.
  - **`SelectionChoreography.tsx`.** Selection-driven choreography (art scales in,
    title rises w/ overshoot, metadata staggers), fanart crossfade, **rAF-spring**
    list momentum, interruptible. Live tuning sliders (entrance/stagger/overshoot/
    spring k/damping).
  - **`MotionShowcase.tsx`.** Big moments (WAAPI: art grows 5× to viewport center
    + return; title orbit/swirl), in/out transition pairs (fade/slide/scale/flip/
    blur, 720 ms), ambient-loop grid (float/glow/breathe/tilt/hue/shimmer/…).
  - **`BoxArtFX.tsx`.** REAL covers via `useMedia().coverUrl` (cycle with Next art):
    reflection + grounded shadow, glass/gloss finish (backdrop-filter frost +
    specular sweep), pointer-tilt.
- **Findings → `MOTION.md`** (results table + interpretation). **WAAPI finding
  REVERSED:** M1's "WAAPI invisible" was a misdiagnosis — the real bug was
  window-present *timing* (one-shot played before the window was shown), already
  fixed by the `oa://window-shown` handshake (D54). The probe's WAAPI loops
  forever → visible. Corrected: `MOTION.md`, the `ViewTransition.tsx` header, and
  the WAAPI memory.
- **High-refresh: confirmed.** Probe showed 60 fps only because the operator's
  panel was set to 60; at 144 Hz the program runs **144 fps** — no rAF cap.
- **Loop validated (D53):** the whole session iterated via `cargo tauri dev` + HMR;
  benches hot-loaded with no rebuild. The day-long M1 tax is gone.
- **CI:** `tsc` + `lint` + `vitest` (179) green; frontend-only.
- **Almost:** the M1 declarative entrance still carries `[oa-theme-motion]`
  diagnostics + isn't feel-tuned (revisit when the real motion model lands). The
  bench's keeper effects need promoting from dev demos into declarative
  `theme.toml` presets (the actual Thrust M work, now unblocked + de-risked).
- **Next:** with the foundation validated, build the **declarative motion model**
  (D52) — turn the bench keepers into named, theme-authorable presets + the
  keyframe vocabulary, dogfooded on a navigable surface (D55). Decide the entrance
  diagnostics cleanup at that point.

---

## 2026-06-16 — ARC 3 Thrust M: motion-foundation PLANNING session (no code; decisions D53–D55, M0 slice queued)

> The planning session the M1 entry below called for. **Outcome: all 6 open
> problems in [PLANS/theming-arc-3-cinematic.md](../../PLANS/theming-arc-3-cinematic.md)
> §"Motion foundation" resolved**; a new **M0 foundation slice** is queued before
> M1 acceptance / M2. No code this session — docs only.

- **The reframe that unlocked it:** dev and build are NOT far apart in *behavior* —
  the M1 day-long tax was *iteration speed* (full `cargo tauri build` per tweak),
  not a dev/build compositing gap. The Rust window-builder is identical in both
  (`main.rs setup_single_window`: `transparent(true)` + `.visible(false)` + DWM);
  only the WebView content source differs (devUrl+HMR vs bundled). So WAAPI is
  invisible in dev too, CSS keyframes paint in both — and `cargo tauri dev`
  reproduces the real surface with ~1 s HMR + live devtools.
- **Decisions (operator signed off each fork in prose, then 2 structured):**
  - **D53** — `cargo tauri dev` (single-window) is the motion-dev loop; build is for
    playtest + final acceptance. Operator agreed to run dev for motion work.
  - **D54** — `oa://window-shown` (the M1 handshake) is THE canonical "shell
    presented" signal; entrance/boot/attract all ride it.
  - **D55** — insert M0; M2 dogfoods on a navigable surface (Retroverse/playground),
    NOT `DeclarativeShell` (it has no runtime view changes — wrong archetype for M2);
    verification lightweight (playground smoke + `animationend` assert, no
    screenshot-diff); scroll-safe rule = never animate the scroll container.
- **M0 (queued, next motion work):** (1) confirm entrance paints under dev;
  (2) hash-mounted motion-playground route; (3) `MOTION.md` compositing catalogue +
  scroll-safe rule + windowShown pattern; (4) `animationend` dev assertion;
  (5) bless `oa://window-shown`. Branch `theme-arc3-motion-slice-1` is the base.
- **Almost:** nothing built — planning only. **Next:** M0 (run dev confirm first).

---

## 2026-06-16 — ARC 3 Thrust M, M1: declarative motion — 🚧 IN PROGRESS / PAUSED for a motion-foundation planning session (branch `theme-arc3-motion-slice-1`, NOT merged)

> **Status, honestly:** the declarative contract (tokens + manifest field +
> validator + Rust carry-through + resolver + dogfood) is solid and green. But
> getting ONE entrance transition to actually render took an entire day of
> operator-in-the-loop round-trips against the real transparent-WebView build,
> and the result still isn't satisfying. The operator's call (correct): **stop,
> wrap, and run a planning session on the motion FOUNDATION** before building the
> harder cinematic thrusts (S shaders, V video) on top of it. If the simplest
> animation is this hard, the substrate + our iteration loop need design. The
> "Motion foundation — open problems" section in
> [docs/PLANS/theming-arc-3-cinematic.md](../../PLANS/theming-arc-3-cinematic.md)
> is the agenda. Do NOT treat M1 as shipped.

- **Built (contract is sound, keep):** the declarative motion data path —
  surface-split (D51 UI/DOM layer), DATA-only (D50), flowing to disk themes (D52).
  - **Motion token group activated** (`tokens.ts`): `ThemeMotionTokens` +
    `MOTION_TOKEN_VAR` (the 7 reserved `--motion-*`/`--ease-*` vars) + a
    `themeMotionTokensCss(scope, tokens)` helper. Kept a SEPARATE group (not
    folded into `ThemeTokens`) because its injection must re-assert the
    `prefers-reduced-motion` floor inside the mount — a scoped inline/class
    duration override would otherwise out-specify the global `:root { --motion-*:
    0ms }` a11y reset for theme-internal motion. App.tsx injects it as a scoped
    `<style>` (next to the perSystemTokens style) that re-zeroes the duration
    vars under reduced-motion. `ThemePackage.motionTokens` + `diskTheme.ts`
    carry-through.
  - **Manifest motion field** (`manifest.ts`): `manifest.motion.view_transition`
    = `{ preset: none|fade|slide|scale, duration?, easing? }` +
    `VIEW_TRANSITION_PRESETS`. `validateTheme` rules: unknown preset →
    `UNKNOWN_TRANSITION_PRESET` (error), blank timing/shape → `INVALID_MOTION`,
    plus motionTokens key/value checks. Carried through the **Rust disk loader**
    (`theme_loader.rs`: `ThemeMotion` / `ViewTransitionConfig` structs) so a
    `theme.toml` `[motion.view_transition]` table reaches the frontend (D52).
  - **Resolver + primitive:** `motion.ts` — `resolveViewTransition` + `parseDurationMs`
    + `usePrefersReducedMotion`. `ViewTransition.tsx` — **CSS-animation** primitive
    (NOT WAAPI — see findings): sets the inline `animation` shorthand naming a
    keyframe from `index.css` (`oa-vt-fade|slide|scale`), restarted via
    clear-reflow-reapply, optional `delayMs`.
  - **Window-ready handshake** (the eventual fix for "nothing renders"): Rust
    creates the shell window `.visible(false)`; `oa_shell_ready` command +
    `present_shell_window` show it on frontend first-paint and emit
    `oa://window-shown` (5 s timeout fallback so it can't stay black). Frontend
    `windowShown.ts` signal; `DeclarativeShell` keys the entrance on it → the
    transition plays the instant the window is actually on screen.
  - **Dogfood:** `neon-list` (disk) declares `slide` (600 ms); `bare-declarative`
    (built-in) declares `fade` (450 ms) + motion-token overrides.
  - **⚠️ HARD-WON FINDINGS (read before M2) — debugged live against the operator's
    transparent single-window build:**
    1. **The Web Animations API does NOT visually composite in single-window
       mode.** That shell is a transparent WebView2 composited over wgpu by DWM;
       `element.animate()` fires (we confirmed `-> ANIMATE` in the log) but never
       recomposites the transparent surface → invisible. **Use CSS animations**
       (the `oa-vt-*` keyframes), which go through the normal paint pipeline DWM
       recomposites — same as the working boot-fade / focus cards. M2/M3 motion
       must stay CSS, not WAAPI.
    2. **A mount-time play is unseen** — the OS window is presented AFTER the
       WebView's first paint, so anything that plays at mount finishes before the
       operator sees the shell. Hence the deferred `entered` flip. Multi-trigger
       replays (mount + timer + window-focus + visibilitychange) stacked into a
       **strobe** — one deferred play is the fix. (Bare `window` focus events also
       proved unreliable in this WebView; `visibilitychange` worked but was
       dropped with the rest.)
    3. **The reduced-motion short-fade in the resolver is mostly moot** — the
       global `* { animation-duration: 0.01ms !important }` reduced-motion reset
       (index.css) wins over our inline `animation`, so view transitions go ~instant
       under reduced motion regardless. Acceptable a11y floor; the resolver branch
       is kept for correctness but the CSS reset is the actual gate.
- **CI state:** `tsc` + `lint` + `vitest` (90 theme / 179 total) green;
  `cargo clean -p oa-shell && cargo check` green. The entrance IS now visible on
  the operator's build (after WAAPI→CSS + the window-ready handshake) but the
  *feel/reliability* isn't there yet and the path to get here was far too costly.
  **Diagnostics (`[oa-theme-motion]`, the `DeclarativeShell MOUNTED`/active-theme
  markers) are intentionally LEFT IN — do not strip until the foundation is settled.**
- **Almost / unresolved:** it renders, but: the timing/feel still needs tuning;
  the scroll-container interaction the operator flagged isn't fully understood;
  and every iteration cost a full `cargo tauri build`. These are foundation
  issues, not polish.
- **Next: a PLANNING session, not M2.** Agenda = the "Motion foundation — open
  problems" section added to [PLANS/theming-arc-3-cinematic.md](../../PLANS/theming-arc-3-cinematic.md).
  Headline problems: (1) what animation techniques actually composite on the
  transparent WebView2/DWM surface (WAAPI doesn't; catalogue what does); (2) a
  fast iteration loop for motion (dev-mode / a motion playground) so we're not
  doing full release builds per tweak; (3) automated/visual verification so we
  can't ship invisible motion again; (4) whether the single-surface
  `DeclarativeShell` is even the right place to prove "view transitions" (it has
  no runtime view changes — only the entrance). Settle these, THEN resume M1
  acceptance and M2.

---

## 2026-06-16 — ARC 2 "P" P.1 S3: disk-theme registry merge + Appearance picker + `themes` pack type — ✅ shipped (branch; **P.1 complete**, pre-merge)

> Branch `theme-oatheme-loader-slice-1`. Closes P.1: on-disk `.oatheme` themes
> are now discovered, validated, selectable, and channel-distributable end to
> end. Operator playtested S1+S2 (Bare vs Bare-declarative render identically).

- **Shipped:**
  - `platform/theme/diskTheme.ts::mergeDiskThemes(builtins, descriptors)` — maps
    each descriptor via `diskThemeToPackage` and appends after the built-ins;
    built-ins WIN any id collision (the bundled default stays a guaranteed,
    un-overridable fallback floor, D44) — shadowed disk ids are logged + skipped.
    Pure (takes builtins as a param → stays in platform/, no themes/ import).
  - `platform/api/themesApi.ts::listDiskThemes()` — typed bridge to the S1
    `oa_themes_list_disk` command.
  - `App.tsx` — approach (b): the synchronous `registerThemes(BUILTIN_THEMES)`
    stays (flash-free first paint); a new `onMount` async pass discovers disk
    themes, RE-registers the merged set, THEN runs `initActiveTheme` so a
    persisted disk-theme id resolves against the full valid set. Discovery
    failure is non-fatal (built-ins stand). The Appearance picker already renders
    `availableThemes()`, so disk themes appear with no picker change; `validateTheme`
    runs on them at registration, excluding invalid ones exactly like a builtin.
  - `crates/oa-packs` — `themes` added to `default_pack_type_specs`
    (`has_bundled_baseline: false`, PD4). Pack `type` is an open string (CP3), so
    a `themes` pack already installs to `<exe_dir>/themes/community/<id>/` via the
    existing pipeline → the S1 loader discovers it → it's selectable. No install-
    path change needed; the spec seed is the declarative home + is test-covered.
  - **Sample theme** (`themes/community/neon-list/` — `theme.toml` +
    `tokens.toml` + `per-system.toml`, distinct id so it isn't shadowed). A live
    on-disk theme auto-discovered in dev; guarded by `shipped_sample_theme_parses`.
  - **S3 follow-up fix (after operator playtest):** the loader's resolvers
    shipped in S1 with NO source-tree fallback, so a repo-placed theme was never
    found when running the workspace `target/release` exe (the operator's "not
    showing up" report — log confirmed "no themes/community/ directory").
    Diagnosed via the runtime log (`theme_loader: no themes/community/ directory`
    + `system_registry: loaded … from …\config\systems` proving a source-tree
    run). Fix: `resolve_themes_subdir` now walks `<exe_dir>/themes/<leaf>` →
    `<repo>/themes/<leaf>`, matching every other resource loader; sample relocated
    from `docs/` to the live `themes/community/`; `load_default` now logs the
    scanned path + theme count. DECISIONS **D46 CORRECTION**.
  - Tests: `diskTheme.test.ts` +3 (mergeDiskThemes: append/dedup/empty);
    `theme_loader.rs` +1 (sample parses).
- **Verified:** `tsc` + `eslint` clean; `npm run test` = **160 passed**;
  `cargo test -p oa-packs` = **14**; `cargo test -p oa-shell theme_loader` = **10**
  (clean rebuilds first). Operator smoke for S3 pending (hand-place `neon-list` →
  restart → select).
- **Almost:** the dogfood's source is still the inline builtin descriptor; the
  plan's "swap bare-declarative's source from inline to disk" is optional polish.
- **Next:** operator playtests the `neon-list` hand-place path; on confirmation,
  **merge `theme-oatheme-loader-slice-1` → main** (P.1 complete = the playtestable
  milestone). Beyond P.1: P.2 (runtime custom-JS themes) stays DEFERRED; system-ui
  asset cascade (`system-ui/` backgrounds/sounds) for disk themes is a future
  accretion.

## 2026-06-16 — ARC 2 "P" P.1 S2: `DeclarativeShell` + `diskThemeToPackage` + `bare` dogfood — ✅ shipped (branch, pre-merge)

> Branch `theme-oatheme-loader-slice-1` (P.1 continues on the one phase branch).
> The frontend half of the declarative loader: a built-in shell that renders any
> declarative theme from data, the descriptor→package mapper, and a zero-code
> dogfood proving `bare` works with no theme component code.

- **Shipped:**
  - `platform/theme/declarativeShell.tsx` — the one built-in `ThemeEntry` (D47)
    that renders EVERY declarative theme: reads the active manifest, resolves the
    `game-browse` layout via `useResolvedLayout` (ARC 2 L3), and mounts the
    matching nav primitive (List/Grid/Carousel/Wheel — all reused, none rebuilt).
    Tokens/perSystemTokens/glyph_set need no code here (App's `.oa-theme-mount` +
    glyph bridge already handle them); cards carry `data-system`; `ThemeBackground`
    follows the focused game's system. List rows mirror `bare` exactly (accent dot
    + title + system short + compact density).
  - `platform/theme/diskTheme.ts` — `DiskThemeDescriptor` TS type (mirrors the
    Rust struct 1:1) + `diskThemeToPackage(desc)`: injects `DeclarativeShell` as
    the entry + synthetic non-empty `entry`/`entry_export` (satisfy the shared
    `ThemeManifest` contract + `validateTheme`; never dereferenced — custom-JS is
    deferred P.2).
  - `themes/declarative-bare/` — the **dogfood**: `bare` re-expressed as a pure
    `DiskThemeDescriptor` → `diskThemeToPackage` → registered in `BUILTIN_THEMES`
    as "Bare (declarative)", beside hand-coded `bare` for A/B. Declares
    `views["game-browse"].layout = "list"` so it renders a list, not the grid
    default. Zero theme component code.
  - **Recognized-settings seed vocabulary**: the shell interprets `compactRows`
    (→ list density) from `settings_schema`; other declared controls still render
    in Appearance + persist but are inert in the generic shell (accrete additively
    — the plan's open-question, settled minimally).
  - Tests: `platform/theme/diskTheme.test.ts` (6 — mapper carries fields,
    validates like a builtin, rejects a bad token key, manifest→primitive
    resolution) + `themes/declarative-bare/index.test.ts` (1 — dogfood is valid +
    list-rendering + DeclarativeShell-backed).
- **Verified:** `tsc` clean, `eslint` clean (incl. the platform↛theme boundary —
  the dogfood test lives under `themes/`), `npm run test` = **157 passed** (18
  files). The `oa-audio transformCallback` stderr is the harmless Tauri-absent
  warning any nav-importing test prints.
- **Almost:** disk themes don't auto-register yet — the dogfood is a builtin; the
  `oa_themes_list_disk` → `diskThemeToPackage` → registry merge is S3.
- **Next:** **P.1 S3** — App merges `oa_themes_list_disk` results into the
  registry (validated like builtins), Appearance picker lists disk themes, and
  `themes` joins `default_pack_type_specs` so a `themes` pack installs → restart →
  selectable. Then swap the dogfood's source from inline to disk.

## 2026-06-16 — ARC 2 "P" P.1 S1: `.oatheme` on-disk loader + discovery command — ✅ shipped (branch, pre-merge)

> Branch `theme-oatheme-loader-slice-1`. First slice of the declarative-first
> `.oatheme` runtime loader arc ([docs/PLANS/theming-oatheme-loader.md](../../PLANS/theming-oatheme-loader.md)).
> The Rust half only — no frontend consumer, no `DeclarativeShell`, no rendering
> (those are S2/S3). PD1–PD4 formalized as **D45–D48** (D44 was already taken;
> see DECISIONS note).

- **Shipped:**
  - `apps/oa-shell/src/theme_loader.rs` — serde structs mirroring the TS
    declarative contract: `DiskThemeManifest` (mirrors `ThemeManifest` **minus
    `entry`/`entry_export`**, snake_case keys incl. `views` + the
    `toggle/slider/select` `settings_schema` union), `DiskThemeTokens` (mirrors
    `ThemeTokens`, camelCase keys), `DiskPerSystem`/`SystemPalettePartial`
    (mirrors `perSystemTokens`). `DiskThemeDescriptor` = manifest + optional
    tokens + optional per-system palette + absolute `base_path`.
  - Discovery: `resolve_themes_community_dir()` (`<exe_dir>/themes/community/`,
    the CP2 `<type>/community/<id>` pack layout) + a **reserved**
    `resolve_themes_dev_dir()` (`<exe_dir>/themes/dev/`, scanned at startup; no
    hot-reload). `load_from_parent_dir()` walks one subdir per theme;
    skip-on-malformed + logged, never fatal — mirrors `emulator_profiles`/`packs`.
    A malformed *optional* sidecar drops just that layer; an all-empty
    `tokens`/`per-system` collapses to `None`.
  - Tauri command `oa_themes_list_disk`, registered in `main.rs`
    (mod + invoke_handler).
  - 9 unit tests: full manifest (views + per_system + all 3 control kinds),
    optional sidecars, camelCase token JSON, descriptor casing
    (basePath camel / manifest snake), skip-malformed-keep-siblings,
    bad-sidecar-not-fatal, empty→None, missing-dir→empty, base-path resolution.
- **Verified:** `cargo test -p oa-shell` = **872 passed** (863 prior + 9 new),
  clean `cargo clean -p oa-shell` build first (stale-fingerprint quirk).
- **Almost:** the on-disk format is fully parsed but nothing renders it yet.
- **Next:** **P.1 S2 — `DeclarativeShell`**: a built-in Solid `ThemeEntry` that
  renders one browse surface from a manifest (resolve per-view/per-system layout
  → mount the matching primitive, paint `ThemeBackground`, honor glyph set +
  `settings_schema`), plus a `diskThemeToPackage(desc)` mapper, dogfooded by
  re-expressing `bare` as pure `theme.toml` + tokens.

## 2026-06-15 — ARC 2 L5: end-user per-system layout override UI ("Layout" Hub domain card) — ✅ shipped + MERGED to main (operator playtested)

> Branch `feat/theming-arc2-l5-layout-picker`. The D32 user-agency headline: a
> runtime "pick your view per system" surface writing the already-built L3 override
> store — pure UI, no new machinery. Three forks signed off (AskUserQuestion):
> home = engine Hub Layout card (not theme territory); scope = ALL FOUR ViewTypes
> exposed (operator overruled game-browse-only) with reserved views labeled;
> primitives = list/grid/carousel/wheel (no `custom`). DECISIONS **D43**.

- **Shipped:**
  - **`engine/systemsHub/domains/LayoutEditor.tsx`** — per-system layout picker
    mirroring `DisplayVideoEditor`'s `PanelScaffold`/`HubSection` shape. One
    `SettingRow` per view (game-browse first, then the reserved system/manufacturer/
    details). Each row: a select with a leading `Theme default — <X>` sentinel
    (→ `clearLayoutOverride`) + the four user primitives (→ `setLayoutOverride(
    activeThemeId, system, view, choice)`); an inheritance chip showing the
    no-override fallback (`resolveLayout({override: undefined, …})`) + its tier
    ("this theme · per-system" / "this theme" / "engine default"); a Reset chip when
    overridden (D30 discipline). Subtitle states overrides apply to the active theme
    (the store is theme-keyed). `HONORED_VIEWS` labels game-browse "Shown in the
    library now" vs the others "Reserved — no renderer yet" (honest, not broken).
  - **`domains.ts`** — new `DomainId "layout"` + `DOMAINS` entry (glyph `▤`, blurb
    "How this system's games are browsed", enabled).
  - **`SystemsHubRoot.tsx`** — import + a `domain() === "layout"` Switch arm.
- **Verified:** typecheck + lint green; `npm run test` = **149 passed** (unchanged —
  the editor is playtest-verified like the other ARC-2 primitive renders; the cascade
  logic is already covered by `layoutResolver.test.ts`); build green. Frontend-only.
- **Almost:** the three reserved views (system/manufacturer/details) persist a pick
  but change nothing visibly until they gain renderers — labeled as such.
- **Playtest (2026-06-15) — passed.** Operator confirmed merge. Also surfaced a wanted
  cross-theme convenience follow-on: **"Copy from theme…" + "Set for all themes"**
  buttons on the layout editor (overrides are theme-scoped by design, D39, so a user
  re-picks per theme) — parked as wanted (PARKING_LOT 2026-06-15; the buttons write the
  existing store, no cascade change).
- **Next:** **L6** — re-home Per-System UI Stage 2/3 as Retroverse content/consumption
  (built into the substrate capability, not engine-global; D33/D34). Then **P**
  (`.oatheme` runtime loader).

## 2026-06-15 — ARC 2 L4b: radial WheelNav primitive + render `wheel` in game-browse — ✅ shipped + MERGED to main (operator playtested: TG-16 wheel; gentle-feel + fast-scroll-deform fixes confirmed)

> Branch `feat/theming-arc2-l4b-wheelnav`. Builds the reserved S5.5 `WheelNav`
> contract (was a warn-once stub) — the BigBox/HyperSpin radial wheel. Shape fork
> signed off: **shape A (right-side vertical wheel) first**; operator wants B/C "and
> other ways to display" later as variations, so the geometry is built general.
> Demo system signed off: **tg16** (AskUserQuestion). DECISIONS **D42**.

- **Shipped:**
  - **`wheelGeometry.ts`** — pure `wheelDisplacement(offset, geom)` + `wheelStepDeg`
    angle→x/y projection, split out so the bug-prone radial math is unit-testable
    (mirrors `spatialGeometry.ts`). Items on a circle of `radius`; focus at
    `anchorAngle`; each item's on-screen pixel delta derived from its signed offset
    (next → below). No circle centre, no track transform — a focus change re-projects
    every item and CSS transitions animate the slide along the arc.
  - **`WheelNav` body** — vertical `useFocusGroup`, windowing (±`window` in DOM,
    scales to 1700+-game systems, D29.1), `useLateClaim`, `onNavSound`, wheel-scroll,
    click-side-to-focus / click-focus-to-confirm. **Shape A is the DEFAULTS**
    (`anchorAngle` 0→270; new optional `anchor` on-screen-pin prop default right-of-
    centre) so the bare primitive renders the iconic wheel; B/C are future prop
    presets over the same engine. Covers stay upright (no counter-rotation). Reserved
    props (radius/arcDegrees/window/anchorAngle/transitionMs) unchanged → drop-in.
  - **`LibraryView`** renders `wheel` via a 4th `<Switch>` arm (grid/list/carousel/
    wheel); carousel + wheel share one controlled browse focus index; ring `radius`
    is `ResizeObserver`-measured pane-height × 0.52 (min 240) so it fills the column
    at any size. `custom` still grid-falls-back.
  - **Retroverse demo:** `views.game-browse.per_system` adds `tg16: "wheel"` (the
    plan's canonical wheel example), alongside `nes: "list"` + `snes: "carousel"`.
- **Verified:** typecheck + lint green; `npm run test` = **149 passed** (+5
  `wheelGeometry` cases replacing the stub `toBeNull` assertion; builtin-themes still
  validates with `tg16: wheel`); build green. Frontend-only.
- **Playtest round 1 (2026-06-15) — gentler wheel feel:** operator found the wheel
  "shrinking / pulling toward the middle" while scrolling. First fix (sync pane-height
  measure in `onMount` to kill an initial radius-settle animation) addressed a real but
  different jump. The actual complaint was the radial feel being too aggressive: tuned
  shape-A defaults gentler — `arcDegrees` 140→80 (near-even vertical spacing instead of
  edge-bunching + a milder horizontal curve), `sideScale` 0.62→0.85 (focus still
  biggest, neighbours don't dwindle), `opacityFalloff`/`minOpacity` softened; bumped
  the `LibraryView` radius multiplier 0.52→0.76 so the shallower arc still fills the
  column. Operator confirmed gentle-but-still-a-wheel is the target; the **near-flat
  vertical strip** + the **continual/looping wheel** are noted future *views* (presets
  over the same geometry, not rewrites).
- **Playtest round 2 (2026-06-15) — fast-scroll deform fix:** slow stepping looked
  right but fast scrolling "deformed the whole wheel and pulled away from the bottom/
  top depending on direction." Cause: every focus step restarts a 300ms CSS transition
  with its velocity reset, so steps arriving faster than the transition stack up and the
  items fall progressively behind the true focus (leading edge gaps). Fix: WheelNav
  detects rapid moves (<140ms apart) and collapses the transition to a ~70ms near-snap
  while fast-scrolling so items track the focus, restoring the full 300ms gentle ease
  ~160ms after the last move (the final resting step still animates nicely). Slow
  stepping unchanged.
- **Almost:** wheel polish (preload buffer for fast scroll, reflection/depth blur,
  per-shape tuning) deferred; B/C + strip + looping shapes are future presets (D42 §2).
- **Next:** **operator visual playtest** — select TG-16 → navigable right-side radial
  wheel (browse via Up/Down + scroll, Confirm launches, Secondary info); NES → list,
  SNES → carousel, others → grid/viewMode. Then merge. **After: L5** (user-facing
  per-system layout picker) → L6 → P.

## 2026-06-15 — ARC 2 L4a: render `carousel` in game-browse (reuse CarouselNav) — ✅ shipped + MERGED to main (operator playtested: SNES coverflow)

> Branch `feat/theming-arc2-l4-wheelnav`. L4 split L4a (carousel render, reuse) +
> L4b (the radial WheelNav geometry, next session) — AskUserQuestion sign-off.
> DECISIONS **D41**.

- **Shipped:** `LibraryView` renders a per-system `carousel` via the `CarouselNav`
  primitive (the path CoverFlow uses): coverflow over the flat `sorted()` list,
  controlled focus (right-pane detail + `onFocus` follow the centred card), covers
  via `useMedia`, `onConfirm`→launch / `onSecondary`→info, cards carry `data-system`
  for Retroverse's per-system accent. The render switch is now a 3-way `<Switch>`
  (grid fallback / list / carousel). `wheel`/`custom` still fall back to grid
  (wheel = L4b; custom = theme-drawn). Retroverse demo: `views.game-browse.per_system
  = { nes: "list", snes: "carousel" }`.
- **Verified:** typecheck + lint green; `npm run test` = **145 passed** (carousel
  render is playtest-verified like the other primitives — no Solid render harness;
  builtin-themes still validates with `snes: carousel`); build green. Frontend-only.
- **Almost:** carousel polish (preload buffer / selectedId ring) deferred; `wheel`
  rendering is L4b.
- **Next:** **operator visual playtest** — select SNES → coverflow; NES → list;
  others → grid/viewMode. Then merge. **After: L4b** (build the radial WheelNav
  primitive + render `wheel`) → L5 override UI → L6 → P.

## 2026-06-15 — ARC 2 L3b: per-system layout wired into game-browse (coexist with viewMode) — ✅ shipped + MERGED to main (operator playtested: NES lists, slider hides on list)

> Branch `feat/theming-arc2-l3b-layout-consumer`. The first LIVE consumer of the
> view/layout contract — per-system layout becomes visible. UX fork signed off
> (AskUserQuestion — **coexist**: the global capsule/list `viewMode` toggle stays
> the default; per-system overrides only where declared). DECISIONS **D40**.

- **Shipped:** `ViewLayoutConfig.layout` made **optional** (a theme can declare
  `per_system` without a view-wide default — else it'd override every system's
  global toggle); validator relaxed (validate layout only when present).
  `resolveDeclaredLayout` / `useDeclaredLayout` — the cascade MINUS the engine
  default (`undefined` = "keep your own default"). `LibraryView` consumes it keyed
  on the existing `selectedSystemId()`: declared `list` → `DetailListView`, `grid`
  → `VirtualLibraryGrid`; carousel/wheel/custom fall back to grid (not yet rendered
  in the shared browse view — follow-on / L4); `undefined` → today's `viewMode`
  switch (behavior-preserving). **Retroverse demo:** `views.game-browse.per_system
  = { nes: "list" }` (no view-wide layout) — NES browses as a list, others keep
  viewMode. Real curation lands in L6.
- **Verified:** typecheck + lint green; `npm run test` = **145 passed** (3 new
  `resolveDeclaredLayout` cases; the L2a missing-layout test repurposed to
  per_system-only-is-valid); build green. Frontend-only.
- **Almost:** carousel/wheel/custom game-browse RENDERING (grid fallback for now) +
  the L5 user-facing per-system picker.
- **Next:** **operator visual playtest** — select NES → game list; other systems /
  All Games → grid (or global viewMode); the global toggle still drives undeclared
  systems. Then merge. **After: L4** — build the reserved WheelNav radial primitive
  (now it has a consumer), then L5 override UI → L6 → P.

## 2026-06-15 — ARC 2 L3a: layout resolver + persisted per-system override store (plumbing, no consumer) — ✅ shipped + MERGED to main, CI-green

> Branch `feat/theming-arc2-l3-layout-resolver`. L3 split L3a (plumbing) + L3b
> (LibraryView consumer + the resolved-layout-vs-viewMode UX call) — AskUserQuestion
> sign-off, same contracts-first split as L2. DECISIONS **D39**.

- **Shipped:** `platform/theme/layoutResolver.ts` — the D32 cascade (user override →
  theme `views[view].per_system[system]` → theme `views[view].layout` → engine
  default) as a **pure `resolveLayout`** + a reactive **`useResolvedLayout(view,
  systemId)`** hook + `ENGINE_DEFAULT_LAYOUTS`. `platform/theme/layoutOverrides.ts`
  — the persisted `(theme_id, system_id, view) → layout` store (one localStorage
  key, theme-id-keyed, `createStore`-reactive; get/set/clear), the D28
  `themeSettings` pattern. **No consumer** — nothing reads the hook yet (L3b).
- **Verified:** typecheck + lint green; `npm run test` = **142 passed** (6 pure
  cascade + 5 override-store round-trip/isolation cases); build green. Frontend-only,
  no visual change → CI-gated (optional boot smoke-test).
- **Almost:** nothing in L3a scope.
- **Next:** **L3b** — wire `useResolvedLayout("game-browse", selectedSystemId())`
  into `LibraryView`; settle how the per-system resolved layout relates to the
  global capsule/list `viewMode` toggle (the UX fork); visual playtest (layouts
  change per system). `wheel` stays the L4 stub. Then L4 WheelNav → L5 override UI
  → L6 → P.

## 2026-06-15 — ARC 2 L2b: D34 systemUIConfigs experiential→Retroverse migration — ✅ shipped + MERGED to main (with L2a; operator playtested visual-identical)

> Branch `feat/theming-arc2-l2b-systemuiconfigs-migration` (off L2a — both merge
> together). The D34 migration: move the experiential per-system config out of the
> platform-global map into theme content; keep `touchInputSupported` factual.
> Home signed off (AskUserQuestion — `ThemePackage.perSystemUiConfigs`). DECISIONS **D38**.

- **Shipped:** `ThemePackage.perSystemUiConfigs?` (peer of `perSystemTokens`);
  platform `systemUIConfigs.ts` keeps the contract (`SystemUIConfig` type + `UI*`
  enums + `BASELINE_UI`) + a new **factual** `systemSupportsTouch()` lookup, drops
  the global per-system map + `touchInputSupported` from the experiential type, and
  `uiConfigFor` now merges the **active theme's** override (bridged from App) over
  `BASELINE_UI`. The gb/nes/vectrex pilot values moved to
  `themes/retroverse/systemUiConfigs.ts`; Retroverse declares `perSystemUiConfigs`.
  App.tsx bridges it (`setThemeSystemUiConfigs`). The 3 touch consumers
  (QuickSettings / StylusOverlay / TouchHotspotOverlay) read `systemSupportsTouch`
  instead of the removed map. Validator checks `perSystemUiConfigs` keys (system-ids).
- **Verified:** typecheck + lint green; `npm run test` = **131 passed** (new
  `systemUIConfigs.test.ts` merge/touch tests + 3 validator cases; builtin-themes
  still clean); build green. Frontend-only.
- **Almost:** nothing in L2b scope. (Deep per-field validation of
  `perSystemUiConfigs` values deferred to the on-disk-theme phase.)
- **Next:** **operator visual-identical playtest** — Retroverse per-system tiles +
  nav SFX unchanged (gb portrait/delayed, nes console-audio, vectrex
  physical/square), CoverFlow/bare uniform, NDS stylus/touch overlays still gate.
  Merge L2a+L2b together. **After: L3** — the resolver + persisted user override
  that finally *consumes* the L2a `views` contract.

## 2026-06-15 — ARC 2 L2a: view/layout manifest contract (schema only) — ✅ shipped + MERGED to main (with L2b), CI-green (no consumer, no visual change)

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

