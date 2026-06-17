# Motion — what composites on OA's surface (Theming ARC 3 Thrust M)

> **Sibling doc:** [MOTION_VOCABULARY_AUDIT_2026-06-17.md](MOTION_VOCABULARY_AUDIT_2026-06-17.md)
> — the competitive audit (BigBox / HyperSpin / ES-DE / Pegasus / Playnite /
> tvOS / PS5 / Switch / Netflix / Plex) deriving **which named presets to build**
> (a ~21-preset seed catalog) and **the shared parameter basis** every preset is
> defaults over. Read it before locking the Thrust-M preset registry. This doc =
> what _paints_ here; that doc = what to _author_.

OA's single-window shell is a **transparent WebView2 composited over wgpu by
Windows DWM** (`main.rs setup_single_window`). That composition has a quirk: some
animation techniques fire but never recomposite the transparent surface, so they
are **invisible** — the WAAPI failure that cost M1 a day. This doc is the
**empirical catalogue** of what actually paints here, plus the rules that fall out
of it. It is the gate for all of Thrust M (motion), and for any DOM motion in
Thrust V (video/attract).

> **The eye is the verdict.** A unit test (jsdom/WAAPI) cannot see the DWM
> composite — our green vitest suite passed in M1 while nothing rendered. The only
> reliable check is a human watching the real surface. That's what the probe is.

## How to run the probe

1. `cargo tauri dev` (single-window mode). Dev reproduces the real surface — the
   Rust window-builder is identical to a release build; only the WebView content
   source differs (D53).
2. Press **F10** (dev-only) to toggle the **motion compositing probe** overlay
   (`frontend/src/dev/MotionPlayground.tsx`).
3. Each cell runs the SAME motion via a different technique. **Frozen = does not
   composite here.** rAF cells print a live ticking value — frozen-but-ticking
   proves the JS ran and only the *paint* failed (vs. JS never firing).
4. Record results in the table below.

## Results — first probe run, 2026-06-17 (single-window confirmed)

Run on the confirmed real surface (log: `shell_mode = single-window`, `single
transparent WebviewWindow built (single-window)`, window `presented
(frontend-ready)`). **Every cell PAINTS.** This is the best-case outcome and it
**overturns M1's "WAAPI is invisible" finding** (see correction below).

| Cell (tag) | Technique | Verdict | Notes |
| --- | --- | --- | --- |
| `css/transform` | CSS `@keyframes` translateX | ✅ PAINTS | baseline |
| `css/transform+wc` | + `will-change: transform` | ✅ PAINTS | **GPU promotion is fine** |
| `css/translate3d` | `@keyframes` translate3d | ✅ PAINTS | GPU layer fine too |
| `raf/transform` | rAF → `style.transform` | ✅ PAINTS | physics path OPEN (numbers ticking) |
| `raf/transform+wc` | rAF + `will-change` | ✅ PAINTS | physics on promoted layer fine |
| `waapi/transform` | `element.animate` | ✅ **PAINTS** | **contradicts M1** — see below |
| `css/transition` | `transition` + class flip | ✅ (subtle) | works; the probe's flip is hard to read |
| `css/opacity` | `@keyframes` opacity | ✅ PAINTS | crossfades fine |
| `css/filter` | `@keyframes` hue/saturate | ✅ PAINTS | glows/color fine |
| `css/backdrop` | animated `backdrop-filter` | ✅ PAINTS | blurs fine |
| `scroll/transform-parent` | animate a scroll CONTAINER | ✅ moves + scrolls | **did NOT obviously break** — see caveat |
| `scroll/transform-inner` | animate INSIDE a stable scroll box | ✅ moves + scrolls | safe pattern works |

**fps: 60.** TBD whether that's a 60 Hz panel or a WebView2/DWM rAF cap. Matters
only for the 120 Hz+ high-refresh goal (secondary); investigate later.

## Conclusion: NO compositing ceiling — the toolkit is fully open

Everything composites on OA's transparent single-window surface: CSS `@keyframes`,
CSS transitions, **rAF-driven transforms**, **WAAPI**, **GPU-layer promotion**
(`will-change` / `translate3d`), `filter`, and `backdrop-filter`. Consequences:

- **rAF physics is available** → momentum, springs, parallax-follow, wheel inertia
  — the engine for BigBox-tier selection choreography.
- **WAAPI works** → WAAPI-based libraries (Motion One, spring engines) are viable.
- **GPU promotion is safe** → no need to avoid `will-change`/`translateZ`.
- **"Beat BigBox" is reachable on this stack.** The program-halting risk is not
  present.

**Validated end-to-end (2026-06-17):** beyond the probe, the M0 bench
(`frontend/src/dev/`, F10 in dev) carries three more tabs — **selection
choreography** (rAF-spring momentum + staggered entrance + fanart crossfade),
**motion showcase** (5× grow-to-center, title swirl, in/out pairs, ambient loops),
and **box-art FX** (real covers with reflection+shadow, glass finish, pointer-tilt).
Operator playtested all four and confirmed premium feel on real art — the "true
yes." Keeper effects become declarative `theme.toml` presets (D52).

### Correction: M1's "WAAPI doesn't composite" was a MISDIAGNOSIS

M1 concluded WAAPI was invisible and switched everything to CSS. The probe proves
WAAPI paints fine. The reconciliation: M1's WAAPI was a **one-shot entrance played
at mount**, and the OS presents the window ~hundreds of ms *after* the WebView's
first paint — so the one-shot ran and finished while the window was still hidden.
It was a **timing** failure, not a compositing one. The probe's WAAPI loops
forever (`iterations: Infinity`), so it's still running when observed → visible.
**The fix that actually worked in M1 was the `oa://window-shown` handshake (D54),
not the WAAPI→CSS switch.** (The `ViewTransition.tsx` header + the M1 SESSION_LOG
entry still carry the old WAAPI claim — correct them when next editing.)

## Rules (accrete as the probe + real choreography teach us)

1. **Don't play motion before the window is shown.** Key entrance/boot/attract
   motion on `oa://window-shown` (D54). This — not a WAAPI ban — was the real M1
   lesson. (WAAPI/CSS/rAF all composite; see results above.)
2. **Scroll container — REVISIT, not a confirmed rule.** The probe animated a
   scroll container's parent *and* scrolled it with no obvious break, so M1's
   "transforming a scroll-container parent disturbs the scrollbar" claim did not
   reproduce in isolation. The real `ViewTransition` in `declarativeShell.tsx`
   wraps the `overflow-y-auto` nav with a `flex-1 min-h-0` transform — if a glitch
   shows up there, prefer animating an inner non-scrolling wrapper. Confirm on the
   real surface before treating as a rule.
3. _(more to come as real choreography teaches us)_

## The motion archetypes we're actually chasing (so the bench tests the right thing)

The satisfying "movement" in LaunchBox/BigBox is **selection-driven, per-element,
continuous, and choreographed** — not page-level transitions:

- **Selection choreography** — on focus change, the new item's logo/art/metadata
  animate in (scale + fade + slide, often staggered) while the old animates out.
- **Wheel/list physics** — momentum, easing, snap.
- **Ambient** — drifting/parallax backgrounds, pulsing glows, looping marquees.
- **Video snaps** — play on selection dwell (Thrust V).
- **Easing with character** — cubic/back/elastic, authored as data.

M1's entrance fade is the shallowest slice; the soul is the keyframe/physics model
(M3-tier) and it likely needs pulling forward. The probe exists to confirm the
techniques those archetypes require are available BEFORE we design the model.
