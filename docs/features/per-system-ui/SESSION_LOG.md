# Per-System Custom UI — Session Log

## 2026-05-27 — Slice 4: Boot animation framework

Branch `feat/per-system-ui-stage-1-slice-4` cut from main after the
Slice 3 merge (`15a632a`) + status flip (`16ddfb0`).

- **Shipped:**
  - **boot-intro SFX plumbing** (`b0735f5`). `audio_player::
    resolve_ui_sound` event match grows a `"boot-intro"` arm that
    skips the SystemSettings override tier (no UI surface for that
    event in v1) and falls straight to the bundled-asset lookup at
    `<systemId>/sounds/boot-intro.<ext>`. Frontend `UiSoundEvent`
    type in `lib/audio.ts` gains the same variant so
    `dispatchUiSound(systemId, "boot-intro")` typechecks for Slice
    4b. No behaviour change on its own — pilot slices (6-8) drop
    the actual .ogg files.
  - **Settings + dispatcher bridge** (`9431051`).
    `Settings.bootAnimationsEnabled` boolean persisted alongside
    `perSystemUiEnabled`, default ON. Settings → Display → "Per-
    system experiences" gains a "Boot animations" sub-toggle gated
    on the master toggle via `<Show>`. `themes/systemBootAnimation.ts`
    exports `setBootAnimationsEnabled` (bridge) +
    `isBootAnimationsEnabled` (accessor); App.tsx `createEffect`
    mirrors the store signal into the bridge.
  - **SystemBootAnimation component** (`9431051`). Triggered by
    `on(() => props.activeSystemId(), …)` so the trigger key is
    explicitly activeSystemId — flipping the toggles or reduced-
    motion doesn't re-fire the animation. Null and same-system
    transitions no-op. Full path runs the `oa-boot-fade` keyframe
    over 1000 ms with a radial gradient tinted by
    `--color-system-accent`; compressed path (sub-toggle off OR
    `prefers-reduced-motion`) collapses to 200 ms. The component
    sets `--oa-boot-duration` per fire so the keyframe duration
    matches. Master toggle off suppresses entirely. Dispatches the
    per-system `"boot-intro"` SFX on the full path; compressed
    paths skip the SFX so audio doesn't outlast the visual.
    Skippable on mouse click / keypress / gamepad nav event;
    `mousedown` (not `mousemove`) to avoid accidental skip when
    the cursor moves into the OA window.
  - **CSS** (`9431051`). New `.oa-boot-animation` class +
    `@keyframes oa-boot-fade` in `index.css`. Radial gradient
    fades in / holds at ~55% opacity / fades out over
    `--oa-boot-duration`. Pilot slices (6-8) override per-system
    via dropped `keyframes.css` files at
    `<systemId>/boot-animation/` later — Slice 4 ships the
    framework only.
  - **App.tsx mount** (`9431051`). SystemBootAnimation mounted in
    `<main>` alongside SystemBackground, fed by the existing
    `activeSystemId` memo (= `viewToSystemId(currentView())`). So
    sidebar nav drives the trigger; hover and tile clicks don't.
- **Almost:** Operator playtest. Click any system in the left
  sidebar from "All Games" — screen briefly washes with that
  system's accent color (~1 s default). Flip Settings → Display →
  Boot animations off — same trigger now compresses to 200 ms.
  Flip Per-system experiences off — boot suppresses entirely. If
  your OS has reduce-motion on, full path is unreachable
  regardless of the sub-toggle. Per-pilot keyframe overrides
  arrive in Slices 6-8.
- **Next:** Slice 5 — tile flourish system. `interactionStyle`
  (`instant` / `delayed` / `physical`) drives the focus animation;
  `tileShape` overrides the existing `tileAspect`. Stays in-config
  — no asset drops needed for Slice 5.

## 2026-05-26 — Slice 3: Per-system background renderer

Branch `feat/per-system-ui-stage-1-slice-3` cut from main after the
Slice 2 merge (`8e26e79`) and the ASSETS.md catalog landing
(`eee0b16`).

- **Shipped:**
  - **Rust resolver** (`apps/oa-shell/src/system_ui_assets.rs`,
    `41d1c5b`). New module separate from `audio_player.rs` — audio
    cascades stay with the rodio mixer state, this one owns
    disk-only resolvers for non-audio per-system assets (backgrounds
    in Slice 3; boot animations land here in Slice 4). Tauri command
    `resolve_background_asset(systemId, kind)` cascades operator-
    untouched per-system bundle →
    `<exe_dir>/assets/system-ui/<systemId>/backgrounds/<basename>.<ext>`
    → universal `_baseline/backgrounds/<basename>.<ext>` → None.
    `kind = "default"` walks STATIC_EXTS [png, jpg, jpeg, webp];
    `kind = "animated"` walks ANIMATED_EXTS [webm, mp4]. 8 new unit
    tests cover the same shape as Slice 2's audio_player tests
    (per-system precedence, baseline fallback, empty disk → None,
    extension priority, extension walk, path-traversal guard, no
    cross-contamination between static/animated cascades). Workspace
    cargo test: 497 green (489 + 8).
  - **SystemBackground component**
    (`frontend/src/components/SystemBackground.tsx`, `1e8c6f2`).
    Three rendering paths driven by `SystemUIConfig.background`:
    `static` → CSS radial gradient base + optional image overlay
    when `default.{png,jpg,jpeg,webp}` exists; `animated` → looping
    `<video autoplay muted playsinline>` when
    `animated.{webm,mp4}` exists; `shader` → falls back to `static`
    until Slice 8 (Vectrex pilot) lands the shader-driven render
    path. `createResource` keyed on `(systemId, kind)` so the
    resolver fires once per active-system change. Honors
    `isPerSystemUiEnabled()` master toggle. Pointer-events: none +
    aria-hidden so it's purely decorative.
  - **App.tsx mount**. SystemBackground mounted as the first child
    of `<main>`; existing library / page content wrapped in
    `<div class="relative z-10 h-full">` to stack above the
    absolutely-positioned background. systemId source mirrors
    RightSidebar's activeEntry pattern: `pinnedEntry?.systemId ??
    focusedEntry?.systemId ?? null`.
- **Playtest fixes (same day, before merge):**
  - `2ee043f`: widen Tauri asset-protocol scope to
    `<exe_dir>/assets/**` at runtime so `convertFileSrc` URLs under
    the assets dir don't 403. The tauri.conf.json scope only covers
    `$APPDATA/**` and operator dropped a release-build asset under
    the exe dir; mirrors the existing portable-mode scope-widening
    pattern next to it in main.rs.
  - `f5551d6`: SystemBackground falls back from `animated` to
    `default` when no animated asset exists. NES is configured
    `background: "animated"` per plan §8, so a dropped `default.png`
    would otherwise go unmatched and drop to gradient-only — the
    fallback makes any single asset drop "just work" for testing.
    Pilot slices 6-8 still ship the configured kind properly.
  - `247353e`: SystemBackground systemId source switched from
    pinned-first to focused-first. The earlier pinned-first pattern
    mirrored RightSidebar's activeEntry, but a stale
    `rightSidebarPinnedGameId` from a prior session was locking the
    background to one system regardless of where the cursor was.
    Pinned stays as a fallback only when nothing else applies.
  - `c58493d`: document-level mouseover listener walks
    `closest('[data-system]')` and feeds a new `hoveredSystemId`
    signal, plus `activeSystemId` (sidebar-filtered system view)
    enters the source chain. LibraryTile deliberately doesn't change
    selection on hover (per LibraryTile.tsx:90-94 comment), but the
    background is decorative — hover-following doesn't break the
    "pick game, open settings" flow that decision protected. Final
    source chain: hovered → focused → activeView → pinned → null.
- **Shipped (merge close-out):** Branch merged `--no-ff` to main as
  `feat/per-system-ui-stage-1-slice-3` (commit `15a632a`).
  Static-path operator-validated. Animated-path code-complete
  pending content; Slice 7 (NES pilot) ships the actual
  scrolling-palette WebM.
- **Next:** Slice 4 — boot animation framework + Settings sub-toggle
  "Boot animations" (visible only when "Per-system experiences" is
  ON). Honors `prefers-reduced-motion`; skippable on any input.

## 2026-05-26 — Slice 2: Per-system SFX wiring

Branch `feat/per-system-ui-stage-1-slice-2` cut from main after the
Slice 1 merge (`fa9d487`).

- **Shipped:**
  - **Rust resolver cascade** (`apps/oa-shell/src/audio_player.rs`).
    `resolve_ui_sound` now cascades: operator override in
    `SystemSettings.ui_sound_<event>` → per-system bundle at
    `<exe_dir>/assets/system-ui/<systemId>/sounds/<event>.<ext>` →
    universal baseline at
    `<exe_dir>/assets/system-ui/_baseline/sounds/<event>.<ext>` →
    None. New helpers `resolve_assets_dir()` (mirrors
    `resolve_cores_dir`) and `find_bundled_ui_sound_in_dir(assets_dir,
    system_id, event)` (pure + unit-testable). Walks extension priority
    `[ogg, opus, wav, mp3, flac, m4a]` matching rodio's
    `symphonia-all` feature. Defensive path-traversal guard rejects
    system slugs containing `/`, `\\`, or `..`. 6 new unit tests
    (in addition to the 7 pre-existing audio_player tests) — all 13
    green; full workspace test suite green at 476 + 6 = 482.
  - **Frontend dispatcher** (`frontend/src/themes/systemUiSound.ts`).
    `playSystemUiSound(systemId, event)` gates on the master toggle
    + per-system `audioProfile === "none"` opt-out; fire-and-forget
    on top of `dispatchUiSound`. Exports `setPerSystemUiEnabled(on)`
    bridge + `isPerSystemUiEnabled` accessor for downstream slices
    (e.g. Slice 4's boot-animation framework will read it).
  - **Library-grid call sites**
    (`frontend/src/components/VirtualLibraryGrid.tsx`). The focus
    group's `setFocusedIndex` callback fires `"navigate"` for the
    newly-focused entry; `onActivate` fires `"launch"` before calling
    the launch handler. Mouse hover + click stay silent in v1
    (gamepad-centric per couch-gamer audience).
  - **App.tsx bridge**. New `createEffect(() =>
    setPerSystemUiEnabled(settings.perSystemUiEnabled()))` mirrors
    the Settings master toggle into the dispatcher. Flipping the
    Settings toggle off suppresses every per-system SFX dispatch
    immediately.
- **Almost:** Operator playtest with a CC0 click sound dropped at
  `<exe_dir>/assets/system-ui/_baseline/sounds/navigate.ogg` —
  walking the library grid with DPad should produce that click.
  Operator content prereq: pick + drop one CC0 click file before
  testing. (In dev, `<exe_dir>` is `target/debug/`; in installed
  builds it's the install root.) Slice ships the plumbing; the
  bundled assets land with Slice 6 (GB pilot) onward.
- **Next:** Slice 3 — per-system background renderer. Static
  gradient default driven by `systemThemes[id].accent`; pilots get
  custom assets later.

## 2026-05-26 — Stream opened + Slice 1 (foundation)

- **Shipped:**
  - Feature folder created — README + ROADMAP (Stage 1 sliced 1-9) +
    this SESSION_LOG + DECISIONS. Plan stays at
    `docs/PLANS/per-system-ui.md` as source of truth; this folder
    tracks what's implemented vs. what's still on paper.
  - Branch `feat/per-system-ui-stage-1-slice-1` cut from main after
    the controller-nav v2 polish merge (`716dc78`).
  - `frontend/src/themes/systemUIConfigs.ts` — `SystemUIConfig`
    interface with the Stage 1 surface (layout / navigation /
    emphasis / background / audioProfile / interactionStyle /
    tileShape / transitionTiming / buttonLabels + optional
    backgroundAsset / soundEffects / customComponent). `BASELINE_UI`
    default + `systemUIConfigs: Record<SystemId, SystemUIConfig>`
    with full pilot configs for GB / NES / Vectrex (per plan §8) and
    baseline for the other ~37 systems. `uiConfigFor(id)` helper for
    consumers.
  - `frontend/src/lib/reducedMotion.ts` — module-level
    `prefersReducedMotion` accessor backed by a shared
    `matchMedia("(prefers-reduced-motion: reduce)")` listener.
    Reactive: consumers in later slices (boot animation framework,
    tile flourish system, transition timing) read it to short-
    circuit long-form animations.
  - `frontend/src/settings/store.ts` — new `perSystemUiEnabled`
    field (default ON per plan §10), persisted in `oa.settings.v1`
    alongside the existing controller-nav fields. Setter exported
    through the store object.
  - `frontend/src/components/SettingsDialogs.tsx` — new
    "Per-system experiences" `DialogSection` in `DisplayDialog`,
    above the existing "Controller navigation" section. Single
    `Enabled` toggle with a description that explains what flipping
    it off does. No sub-toggles yet (boot animations sub-toggle
    arrives with Slice 4).
- **Almost:** Operator playtest of the toggle. No visible behaviour
  change yet — the toggle persists and exposes a reactive accessor,
  but no consumers wire to it until Slice 2. Worth a smoke test that
  the toggle persists across restart + that the new DialogSection
  renders correctly in the Display dialog.
- **Next:** Slice 2 — per-system SFX wiring. Universal CC0 click on
  nav / select / back / launch routed through the existing 4-bus
  mixer's `ui-sounds` bus. First operator content prereq: a single
  CC0 click sound dropped at
  `<exe_dir>/assets/system-ui/_baseline/sounds/click.ogg` (filename +
  path TBD with the slice; locking in Slice 2 planning).

## Storage decision (locked 2026-05-26, Slice 1 planning)

Plan §14.2 asked whether `SystemUIConfig` should live as a sibling
file to `themes/registry.ts` or merge into the existing `SystemTheme`
shape. Operator confirmed: **sibling file**. The visual-theme
registry stays untouched while the new behavioral layer rolls in;
merging can happen later once Stage 1+2+3 settle the final shape. See
[DECISIONS.md](DECISIONS.md) for full rationale.
