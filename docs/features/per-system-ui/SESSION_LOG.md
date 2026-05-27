# Per-System Custom UI — Session Log

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
