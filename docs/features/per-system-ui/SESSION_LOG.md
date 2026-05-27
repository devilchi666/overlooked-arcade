# Per-System Custom UI — Session Log

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
- **Almost:** Operator playtest. With no per-system assets dropped
  on disk, the background renders as an accent-colored radial
  gradient that subtly tints when the operator focuses a tile from
  a different system. Drop a sample image at
  `<exe_dir>/assets/system-ui/_baseline/backgrounds/default.png`
  to see the bundled-asset path light up; drop one at
  `<exe_dir>/assets/system-ui/nes/backgrounds/default.png` to see
  per-system precedence over baseline. Drop `animated.webm` in
  `nes/backgrounds/` to see the NES pilot's animated path (NES is
  already configured for `background: "animated"` per plan §8).
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
