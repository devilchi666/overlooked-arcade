# Motion Vocabulary Audit — what named effects to build, and how to parameterize them (2026-06-17)

**Purpose.** Before we lock the Thrust-M motion-preset registry, derive (not
invent) two things: (1) the **must-cover floor** — the union of motion effects the
competition can express, so we match it as table stakes; and (2) the **shared
parameter basis** — one small set of primitives every named effect is a preset
over, so "how many parameters per effect" stops being an open-ended question.

**Method.** Four parallel research passes — LaunchBox **BigBox** (the primary
competitor), other frontends (**HyperSpin / ES-DE / Pegasus / Playnite**), premium
**console + streaming UIs** (tvOS / PS5 / Switch / Xbox / Netflix / Plex), and the
**web-animation parameter surface** (WAAPI / Motion / GSAP / CSS / spring physics) —
synthesized here and cross-checked against our own
[MOTION.md](MOTION.md) (what actually composites on OA's transparent surface).
Numbers marked _inferred_ come from reference re-implementations, not vendor specs.

> **Companion audit — read alongside this one.**
> [THEME_BUILDER_WISHLIST_2026-06-17.md](THEME_BUILDER_WISHLIST_2026-06-17.md) is the
> sibling pass: where this doc derives the **motion** vocabulary to build, the wishlist
> mines the **broader theme features** authors across BigBox / ES-DE / HyperSpin /
> RetroArch / Playnite / Pegasus repeatedly asked for but never got. Its motion-relevant
> carry-overs we still want added — **motion paths (bezier position curves)** and a
> **keyframe-timeline escape hatch** (HyperSpin's Tier-1 `MotionPath` / `timeline`) — are
> additive to the D57 seed catalog in §3 below. Treat the two docs as one "what to build"
> set: this for motion, the wishlist for everything else.

---

## 1. The competitive landscape in one read

Two camps:

- **Declarative, fixed-vocabulary frontends** — BigBox, HyperSpin, ES-DE. They ship
  an _enumerated_ set of transitions/effects the author selects from. **Low ceiling,
  but that's the bar to clear and exceed.**
- **Raw-toolkit frontends** — Pegasus (QML/Qt Quick), Playnite (WPF). No fixed
  vocabulary; the author hand-writes the entire Qt/WPF animation library (true
  springs, interruptible tweens, arbitrary easing). **Infinite ceiling, zero floor
  — you must be a developer to make anything move.**

**OA's wedge is to be both at once** (the locked north star: very low floor + very
high ceiling): a curated preset gallery _on top of_ a real keyframe/spring model.
BigBox gestures at this (built-in transitions + escape-hatch raw WPF Storyboards)
but the escape hatch is "convoluted… would take a book" (Jason Carr) and the
built-ins **don't expose timing or easing at all** — you pick a transition _type_,
not its feel.

### What BigBox can and can't do (the bar)

| | BigBox |
| --- | --- |
| Built-in transitions | Fade · Flip · Rotate · Explosion · directional Slide, via `TransitionPresenter` + a `TransitionSelector` bound to the global Options→Transitions choice |
| CoverFlow/wheel | `FlowControl` (3D coverflow) + vertical/horizontal/wall/text-list wheels; layout props (`CurveAmount`, `CameraZPosition`, `VisibleCount`, `Spacing`, `ItemZPosition`) but **scroll speed/easing is engine-internal** |
| Video / attract | `VideoControl` box-art/background video; Startup/Pause/Shutdown theme videos; Attract + Screensaver modes (wait time, interval, speed range) |
| Author escape hatch | Full WPF `Storyboard` in theme XAML (confirmed in shipped themes) |
| **Per-transition duration / easing** | ❌ **not exposed to authors** for built-ins — fixed by the engine |
| **Inter-element sync** | ❌ wheel-move and background-transition fire sequentially, no exposed coordination |
| **Spring / physics** | ❌ none — only `ElasticEase`/`BackEase` keyframe overshoot (kinematic, not a solver) |
| **Interruptibility** | ~ tied to navigation, responds to input, but no documented mid-flight retarget control |
| Perf | GPU-config-sensitive; transitions reported "software-rendered"/laggy; video stutter on the WMP backend |

_Sources: launchbox forums (transition selector, flip/cube, background-transition-delay), feedback.launchbox-app.com "themes and where they apply", featurebase attract-mode, MS WPF easing docs._

### What the premium UIs do that the frontends don't

The console/streaming tier is where "premium" actually lives, and it's almost
entirely **spring physics + depth + restraint + choreography**:

- **Focus-scale (lift):** focused item grows ~**1.1×** + drop shadow (tvOS: shadow 0×16, radius 25, 0.3α). PS5 cyan card scale, Switch icon focus.
- **Parallax tilt + layer shift:** tvOS layered posters (2–5 layers) tilt **±10°** and shift **±4pt** following input; specular shine sweeps with the tilt.
- **Spring settle:** tvOS press uses a **critically-damped spring** (damping 0.9, 0.1–0.2s), not a tween — the defining "premium" feel.
- **Backdrop crossfade + Ken Burns:** full-screen art crossfades (~**2s**, no black flash) and drifts (scale **1.15×**, pan **60px**, ~**20s linear**); logo fades in _after_ the backdrop.
- **Staggered reveal:** rows/tiles cascade in at ~**50ms/item**; title/metadata slide-up (translateY 50→0 + fade) _delayed behind_ the focus move.
- **Connected / shared-element transition:** a tile **morphs into the detail hero** across navigation instead of a hard cut (tvOS, Xbox Fluent "connected animations", web View Transitions).
- **Snap + multi-modal:** carousel snap paired with sound + haptic.

_Sources: devsign.co tvOS focus effects (concrete numbers), Apple layered-image guide, nerdy.dev Switch recreation, willbeeching Plex/Netflix screensaver, Fluent motion docs._

**The thesis: BigBox's catalog is the floor; the console/streaming idioms are the
ceiling; the gap between them is spring physics + depth + interruptible
choreography — exactly the things BigBox structurally lacks and OA's surface
already supports** (MOTION.md: rAF springs, WAAPI, GPU promotion, filter, backdrop
all composite here).

---

## 2. The shared parameter basis (answers "how many parameters")

**Don't parameterize per effect.** Every named effect is a preset = curated
defaults over ONE shared basis. The basis (synthesized from WAAPI / Motion / GSAP /
CSS — the minimal set that expresses tween AND spring AND staggered/keyframed
motion):

| Primitive | Type / unit | Default | Notes |
| --- | --- | --- | --- |
| `duration` | ms | per-preset | maps to `--motion-*` tokens for global retiming |
| `delay` | ms | 0 | negative allowed (start mid-motion) |
| `easing` | keyword \| `cubic-bezier()` \| `"spring"` | preset | `"spring"` switches to the spring sub-object |
| `spring` | `{ bounce, duration }` author-facing; `{ stiffness, damping, mass, velocity }` escape hatch | `{ bounce: 0.2, duration }` | **see §2.1** |
| `repeat` | int \| `infinite` | 1 | ambient loops use `infinite` |
| `direction` | `normal`/`reverse`/`alternate` | normal | subsumes "yoyo" |
| `fill` | `none`/`forwards`/`backwards`/`both` | both | hold end-state |
| `stagger` | ms (per child) | 0 | the ~50ms cascade primitive |
| `keyframes` + `offsets[]` | values + [0,1] stops | even | multi-stop escape hatch |
| `composite` | `replace`/`add` | replace | `add` = layered/interrupting motion (WAAPI-only primitive worth keeping) |

…applied to a small set of **animatable channels** (keep them _separate_, Motion-style, not one `transform` string):

- **Compositor-cheap quartet (prioritize):** `opacity`, `translate {x,y}`, `scale`, `rotate` — stay off the main thread; **MOTION.md validated exactly these as the BigBox-tier set on OA's transparent WebView.**
- **Depth/3D tier:** `rotateX/Y` (tilt), per-layer `translate` (parallax shift), `transform-origin`.
- **Cinematic tier (gate as "high ceiling"):** `filter` (blur/brightness/saturate/drop-shadow), `backdrop-filter`. More expensive; opt-in.

So the answer to "how many parameters per effect" is: **~8 shared primitives + a
channel list. A preset names defaults over them and declares which subset it
exposes.** No bespoke per-effect parameter sprawl.

### 2.1 Spring: expose `{ bounce, duration }`, not `{ stiffness, damping, mass }`

Both describe the same second-order spring. The physics triple is maximally
expressive but un-authorable (you can't predict settle time; "bouncier but same
speed" means recomputing two coupled values). The perceptual pair `{ bounce,
duration }` is **orthogonal** — duration = perceived speed, bounce = overshoot —
and is what **iOS `UISpringTimingParameters` / SwiftUI / modern Motion all
converged on**. `bounce`: 0 = critically damped (no overshoot) … 1 = endless
oscillation. **Recommendation: `{ bounce, duration }` is the author surface
(low floor, round-trips cleanly in the future Theme Studio); `{ stiffness, damping,
mass, velocity }` is a documented escape hatch converted internally.** Matches the
project's low-floor/high-ceiling lock exactly.

---

## 3. Seed preset catalog (answers "what named effects")

The union of the floor (competitor parity) + the ceiling (premium idioms),
expressed over §2's basis. Each preset lists only its **non-default / exposed**
params. **Parity** = matches a competitor (table stakes); **Surpass** = a premium
idiom BigBox lacks. Grouped by the UI event that fires it.

### A. View / route transitions — fire on navigating between routes/tabs
| Preset | Effect | Exposed params | Floor/Surpass |
| --- | --- | --- | --- |
| `none` | instant cut | — | parity (BigBox "None") |
| `fade` | opacity 0↔1 | duration, easing | parity (Fade / ES-DE fade) |
| `slide` | translateX/Y by direction | duration, easing, direction, distance | parity (Slide / ES-DE slide) |
| `scale` | scale 0.96→1 + fade | duration, spring | parity+ (Flip-family, but spring'd) |
| `flip` | rotateY/X 3D | duration, easing, axis | parity (FlipTransition) |
| `push-hero` | outgoing recedes, incoming **morphs from the focused tile** | duration, spring | **surpass** (shared-element; BigBox can't) |

### B. Selection / focus choreography — fire on focused item change
| Preset | Effect | Exposed params | Floor/Surpass |
| --- | --- | --- | --- |
| `lift` | focused item scale 1→~1.08 + shadow | scale, spring{bounce,duration} | parity (tvOS/PS5/Switch focus) |
| `lift-stagger` | lift + neighbors settle + title rise + metadata stagger | scale, stagger, spring | **surpass** (choreographed, interruptible) |
| `art-grow-in` | selected art scales in from ~0.9 | from-scale, spring | surpass (M0 bench keeper) |
| `title-rise` | title translateY 50→0 + fade, delayed behind focus | distance, delay, spring | parity (Switch/Netflix metadata) |
| `fanart-crossfade` | backdrop crossfades to focused game's art | crossfade duration, dwell | parity (Netflix/Plex/PS5 ambient) |

### C. Ambient / idle — continuous loops (`repeat: infinite`)
| Preset | Effect | Exposed params | Floor/Surpass |
| --- | --- | --- | --- |
| `breathe` | gentle scale pulse | amplitude, duration | surpass (M0 keeper) |
| `float` | translateY bob | amplitude, duration | surpass |
| `glow-pulse` | filter/box-shadow pulse (focus outline) | color, duration | parity (Switch pulsing outline) |
| `ken-burns` | slow pan + zoom on artwork | scale (~1.15), pan, duration (~20s, linear) | parity (Plex/Netflix/Apple TV) |
| `shimmer` | specular sweep across surface | duration, angle | parity (tvOS specular) |

### D. Box-art / poster treatments — per-tile, focus/pointer-driven
| Preset | Effect | Exposed params | Floor/Surpass |
| --- | --- | --- | --- |
| `parallax-tilt` | 3D tilt ±~10° + layer shift ±~4pt following pointer/stick | max-tilt, layer-depth | parity (tvOS layered posters) |
| `gloss` | glass/specular finish + sweep | opacity, sweep | surpass (M0 box-art keeper) |
| `reflection` | grounded reflection + contact shadow | opacity, height | surpass (M0 box-art keeper) |

### E. Attract / screensaver — idle auto-navigation *(Thrust V territory; listed for completeness)*
| Preset | Effect | Exposed params | Floor/Surpass |
| --- | --- | --- | --- |
| `attract-scroll` | auto-advance selection through library | wait, interval, speed | parity (BigBox/HyperSpin/ES-DE attract) |
| `screensaver` | fullscreen art slideshow: crossfade + ken-burns + delayed logo | interval, crossfade, ken-burns params | parity (Plex/Netflix screensaver) |

**~21 presets across 5 categories.** The compositor-cheap quartet (A, B, C-breathe/
float, partial D) is the buildable Thrust-M core; the 3D/filter tier (parallax-tilt,
gloss, shimmer) is a second pass; **E is Thrust V (video/attract)** and is
cross-referenced, not built in M.

---

## 4. How OA surpasses BigBox (evidence-backed differentiators)

1. **Spring physics as a first-class author param.** BigBox has none (only ease
   overshoot); every premium UI (tvOS/iOS/Switch) is spring-based. We expose
   `{ bounce, duration }` — predictable, premium feel BigBox can't reach.
2. **Author-exposed timing + easing on _every_ preset.** BigBox built-ins hide it
   entirely. Our presets are tunable over the shared basis, with a global
   `--motion-*` token to retime the whole shell.
3. **Interruptible choreography.** BigBox can't coordinate or retarget mid-flight
   (wheel + background fire sequentially, uncoordinated). Our settle-then-transition
   + `composite: add` retargets cleanly on fast navigation — the core "premium" feel.
4. **Per-system / per-view composability.** Motion presets compose with the shipped
   ARC-2 per-system layout (tg16 wheel vs SNES coverflow can carry different motion).
   BigBox motion is global to a theme.
5. **Dual authoring (the wedge).** Preset gallery (low floor) on top of the keyframe/
   spring basis (high ceiling) — BigBox's escape hatch is "convoluted/undocumented";
   the raw-toolkit frontends (Pegasus/Playnite) have no floor at all.
6. **Validated surface.** MOTION.md already proved opacity/translate/scale/rotate +
   filter + backdrop + WAAPI + springs all composite here at 144fps — no rendering
   risk, unlike BigBox's GPU-config-sensitive, stutter-prone WPF path.

---

## 5. Decisions — ✅ LOCKED 2026-06-17 (operator sign-off; DECISIONS **D57**)

1. **Author spring surface = `{ bounce, duration }`** (escape hatch `{ stiffness,
   damping, mass, velocity }`, converted internally). Matches iOS/Motion + north star.
2. **Channels = separate primitives** (`x`/`y`/`scale`/`rotate`/`opacity`), not a
   `transform` string (Motion model; composes + per-channel easing).
3. **Tier the catalog:** Thrust-M ships the compositor-cheap core (A/B/C-core);
   3D-tilt + filter presets are a fenced second pass; attract (E) defers to Thrust V.
4. **`theme.toml` shape:** widen `[motion]` from the single `view_transition` table
   (shipped M1) to `[motion.selection]`, `[motion.ambient]`, `[motion.boxart]` tables,
   each `preset = "..."` + the exposed params. Validated by `validateTheme` (malformed
   = disqualifying error, like `views`).
5. **Dogfood surface = Retroverse routes/tabs** (real view changes + selection moves),
   per D55 — NOT the single-surface `DeclarativeShell`.

---

## 6. Sources

**BigBox:** forums.launchbox-app.com (transition-selector 33324, flip/cube 67019,
xaml tips 28698, startup video 49405, bg-transition-delay 57186, lag 65173);
feedback.launchbox-app.com 9915075; launchbox.featurebase.app attract-mode 2175719;
github G-rila/BigBoxRGS; MS WPF easing-functions docs.
**Frontends:** Attract-Mode hyperspin.nut loader; gameroomsolutions HyperSpin
structure; ES-DE THEMES.md (master + v1.2.1) + USERGUIDE.md; pegasus-frontend.org
docs/api + homage Carousel.qml; api.playnite.link themes docs + RedSchism/ubiquity.
**Console/streaming:** devsign.co tvOS focus effects; Apple AppleTV_PG layered
artwork; HeroTransitions/Hero + peterfriese hero-animation; windowscentral Xbox/
Fluent; fluent2.microsoft.design/motion; nerdy.dev Switch recreation; willbeeching
Plex/Netflix screensaver; createbytes Netflix UX; getdesign.md PlayStation.
**Parameter basis:** MDN WAAPI timing options / KeyframeEffect / Keyframe Formats;
motion.dev react-transitions; maximeheckel spring physics; gsap.com Tween; MDN CSS
transition/animation/timing-function/fill-mode/direction; Apple
UISpringTimingParameters; kvin.me effortless-ui-spring-animations; medium
demystifying-uikit-spring.

_Verification: BigBox built-in-transition params + no-spring confirmed across
multiple forum threads + the developer's own "under-documented" admission. Console
numbers (tvOS 1.1×/±10°/±4pt/damping-0.9, Switch 50ms stagger, Plex 2s/1.15×/20s)
are from reference re-implementations where vendor specs aren't public — flagged
inferred but directionally solid and mutually consistent. Parameter basis
cross-checked against 4 engines; Motion's numeric spring defaults disagree across
builds (verify on the pinned version), but the `{ bounce, duration }` author surface
is stable._
