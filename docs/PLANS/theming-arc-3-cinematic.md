# Theming ARC 3 — Cinematic & Scripting (declarative-first, surface-split)

**Status:** Planned 2026-06-16. No code yet. The three load-bearing forks were
settled with the operator 2026-06-16 (formalize as **D50–D52** at execution).
Successor to **ARC 2 — Per-System Layout Substrate** (complete:
[theming-arc-2-per-system-layout.md](theming-arc-2-per-system-layout.md) +
[theming-oatheme-loader.md](theming-oatheme-loader.md), P.1 merged). Decisions
land in [features/theming-substrate/DECISIONS.md](../features/theming-substrate/DECISIONS.md).

**Owner-of-decisions:** the operator.

---

## Goal in one line

Make OA **cinematic** — motion, shaders, video, attract — while keeping the
declarative-first spine, so the look flows into the **disk (`.oatheme`) themes**
ARC 2 shipped, with author scripting (Rhai) as a fenced, deferred escape hatch.

## The arc table (D35 renumber)

ARC 1 = Minimum Viable Substrate (done). ARC 2 = Per-System Layout Substrate +
the `.oatheme` loader (done). **ARC 3 = Cinematic & Scripting (this plan).**
ARC 4 = Theme Studio (visual editor; deferred).

---

## Settled forks (D50–D52 at execution)

- **D50 — Rhai scripting is a DEFERRED escape hatch, not an up-front thread.**
  The cinematic *declarative* layer (motion, game/bezel shaders, video, attract
  tiers 1–2) ships first; **none of it needs scripting.** Rhai becomes the final
  thrust (Thrust R) — possibly its own ARC 3.5 — gated behind a `scripting`
  engine capability, compiled/power-user tier only, until the sandbox is proven.
  Rhai is the security-heavy piece (untrusted code execution) and is coupled to
  the deferred **P.2** CSP/trust work (D44). Most of the "wow" lands without it.
- **D51 — Surface split: WGSL shader chrome enriches the game/bezel/background
  (the wgpu surface); UI cinematics are CSS/declarative (the DOM layer).** OA's
  single-window shell is a **transparent WebView2 DOM UI composited by Windows'
  DWM *over* the wgpu game surface** (main.rs:3625–3640) — the UI is **not**
  rendered through wgpu. So there is no "one compositor unifying game + UI"; the
  BigBox-research framing is corrected here. Shaders therefore target the wgpu
  surface (game feed + bezel + background — machinery already exists:
  `ApplyShaderPreset`/Phosphor/bezel); the UI's cinematic feel is the declarative
  **motion** layer. "Shaders over the UI" is explicitly rejected — it would mean
  compositing the WebView through wgpu, a rearchitecture that fights the Tauri
  model for marginal gain.
- **D52 — ARC 3 cinematics flow into declarative disk themes as DATA.** Motion
  presets, per-view/per-system shader-preset *selection*, and video-background
  slots are expressed as fields in `theme.toml`, validated by `validateTheme`,
  and honored by the built-in `DeclarativeShell` — so a community disk theme gets
  cinematic with **zero code**, preserving the low-floor/high-ceiling spine ARC 2
  shipped. Each thrust below extends the declarative manifest contract first, then
  the consumer. Rhai (Thrust R), when it lands, stays gated/compiled-tier — it is
  the one cinematic capability that is NOT declarative.

---

## The cinematic axis, split by surface (the organizing principle, D51)

| Surface | Tech | ARC 3 capability | Flows to disk themes? |
| --- | --- | --- | --- |
| **Game / bezel / background** | wgpu + WGSL | CRT / glow / phosphor / bezel / **blend-mode compositor**; theme-selectable shader presets per view/system; background art/ambient | ✅ via preset *selection* as data |
| **UI** | DOM (WebView) + CSS | motion, transitions, parallax-by-depth, ambient reactivity | ✅ via motion tokens/presets as data |
| **Video / attract** | DOM `<video>` (tiers 1–2) + wgpu live (tier 3) | video-background slots; attract rotation → recorded → live-emulator | ✅ slots as data (tier 3 engine-side) |
| **Behaviors** | Rhai (sandboxed) | event-driven scripted behaviors | ❌ gated/compiled-tier (Thrust R, deferred) |

---

## Thrusts & slices (declarative/GPU first; Rhai last)

Sequencing: **M → S → V → (R deferred).** M is highest perceived value, lowest
risk, pure declarative, and flows to disk themes immediately. Each thrust extends
the manifest contract (D52) before wiring its consumer.

### Thrust M — Declarative Motion & Transitions (UI layer, CSS/DOM)

Activate the **reserved `motion` token group** (`tokens.ts` already reserves it,
S3) as an authorable contract; build interruptible transitions + a keyframe/preset
model. The HyperTheme combo (BigBox research §6): a **preset gallery on top of a
real keyframe model**, not either alone. Avoid BigBox's blocking-storyboard bug —
**all transitions interruptible** (settle-then-transition). `prefers-reduced-motion`
is the a11y floor (downgrade to a short fade, per DECISIONS 2026 boot-anim entry).

- **M0 `[SLICE 0 — FOUNDATION, queued 2026-06-16; resolved in the planning session]`** —
  Build the iteration + verification substrate the M1 attempt proved we lacked,
  BEFORE resuming M1 acceptance or starting M2. Settled this session (D53–D55):
  1. **Bless `cargo tauri dev` (single-window) as the motion-dev loop.** Compositing
     is identical to a build (same Rust window-builder — `transparent(true)` +
     `.visible(false)` + DWM); only the WebView content source differs (Vite HMR vs
     bundled). First step: one confirming run that the entrance paints under dev. Dev
     = iterate; `cargo tauri build` = playtest + final motion acceptance (D53).
  2. **A motion-playground route** (hash-mounted, skips full theme boot) exercising
     the technique matrix on the real transparent surface.
  3. **`MOTION.md`** — the living compositing catalogue (what paints on DWM: opacity /
     transform / filter / `will-change` / CSS transitions vs `@keyframes` / WAAPI=no)
     + the **scroll-safe rule** (never animate the scroll container; animate an inner
     non-scrolling wrapper — fixes open problem #6) + the `windowShown` pattern.
  4. **Lightweight verification** (D55): the playground is the manual smoke surface +
     a cheap dev assertion that warns if a declared transition never reaches
     `animationend`. No screenshot-diff harness — the eye on the real surface is the
     final guard against "fired but DWM didn't paint."
  5. **Bless `oa://window-shown`** as THE canonical "shell presented" signal that
     entrance / boot / attract all key on (D54).
- **M1 `[SLICE 1 — 🚧 ATTEMPTED 2026-06-16, PAUSED for foundation planning]`** —
  Motion token contract + reduced-motion + one declarative view transition. The
  declarative *contract* landed and is green (`ThemeMotionTokens`/`MOTION_TOKEN_VAR`
  + scoped `<style>` injection; `manifest.motion.view_transition` through the Rust
  disk loader + `diskTheme.ts`; `motion.ts` resolver; **CSS-animation**
  `ViewTransition` primitive — NOT WAAPI; dogfood neon-list/bare). But rendering
  it on the real transparent-WebView build was a day-long slog and the result
  isn't satisfying → **paused; see "Motion foundation — open problems" below.**
  Branch `theme-arc3-motion-slice-1`, not merged.
- **M2** — View/route transition presets (fade/slide/scale), interruptible;
  per-view/per-system selection via the ARC 2 resolver pattern. **Dogfooded on a
  NAVIGABLE surface** (Retroverse routes/tabs, or synthetic toggles in the M0
  playground) — NOT `DeclarativeShell`, which has no runtime view changes (D55,
  resolving open problem #5). The M1 entrance stays good-enough on DeclarativeShell.
- **M3** — Parallax-by-depth primitive + the keyframe model + preset gallery.

### Thrust S — Game-Surface Shader Chrome (wgpu)

Theme-driven *selection* of game/bezel shader presets (the render-pipeline
extension recipe already exists — see the `render_pipeline_extension_pattern`
memory / `2ddec27`). Expand the preset library; build the **blend-mode compositor**
(game feed + bezel + background unified in wgpu) — the differentiator *no
incumbent offers in the theme layer* (BigBox research §4).

- **S1** — Theme→shader-preset binding: declarative per-view/per-system preset
  selection in `theme.toml` (validated), driving the existing `ApplyShaderPreset`
  path. No new shaders — wire selection first.
- **S2** — Preset library expansion (CRT variants, glow, scanline) + bezel /
  background-art slots on the wgpu layer.
- **S3** — Blend-mode compositor + art-reactive ambient background ("wow"
  differentiator); WGSL fallback-cap checks for the GL backend.

### Thrust V — Video & Attract

`<video>` background slots (DOM, UI layer) using BigBox's `ImageTypes` checklist
as the media-slot vocabulary (MediaDb already holds per-game media — see the
`metadata_lives_in_mediadb` memory). Attract mode in tiers (KIOSK §5). The D20a
preempt/restore seam is already reserved (`themePreempted()`).

- **V1** — Video-background slot + media-slot vocabulary expansion; DOM `<video>`
  with async load + cache (BigBox's biggest perf lesson: disk I/O, not GPU).
- **V2** — Attract tiers 1 (static rotation) + 2 (pre-recorded video), interruptible.
- **V3 `[DEFERRED within the thrust]`** — Tier 3 live-emulator attract
  (savestate-restore → run N s → fade → next; wgpu surface). Deep; its own slice.

### Thrust R — Rhai Scripting `[DEFERRED — D50]`

Sandboxed, event-driven behaviors (`on_game_focus`, `on_platform_change`, …),
never per-frame, never blocking the render path (KIOSK §2.2 / §11). Gated behind
a `scripting` engine capability; compiled/power-user tier only until the sandbox
is proven; couples to **P.2** (the CSP allowlist becomes load-bearing for Rhai,
D44/D6). Out of scope for the first ARC 3 pass — documented as the escape hatch.

---

## What's explicitly NOT in ARC 3

- **Shaders over the UI** (D51) — rejected; would require WebView-in-wgpu compositing.
- **Theme Studio** (visual editor) — ARC 4.
- **Multi-monitor surfaces beyond `main`** (D20b) — contract reserved, no engine support.
- **Attract tier 3 (live emulator)** within the first pass — deferred to V3.
- **Rhai** in the first pass — Thrust R / ARC 3.5 (D50).
- **5-bus audio mixer / multi-monitor / kiosk-mode** — D20 platform capabilities,
  separate from theming (KIOSK_PLAN, deferred).

## Reuse audit (don't rebuild)

- **`motion` token group** — already reserved in `tokens.ts` (S3); M activates it.
- **Game shader pipeline** — `ApplyShaderPreset` / Phosphor composite / bezel
  overlay already shipped; S *selects* + expands, doesn't rebuild. Extension
  recipe: the `render_pipeline_extension_pattern` memory.
- **Declarative manifest + `validateTheme` + `DeclarativeShell` + disk loader**
  (ARC 2 + P.1) — each thrust extends the manifest + the shell, same pattern as P.1.
- **ARC 2 layout resolver** (`useResolvedLayout`) — the per-view/per-system
  selection pattern S1/M2 reuse for shader/transition selection.
- **MediaDb + media-slot infra** — V's video slots ride it.
- **`themePreempted()` / D20a** — attract's preempt/restore seam.
- **The transparent-WebView-over-wgpu composition** (main.rs) — the surface-split
  reality D51 is built on.

## Verification approach

- Declarative slices (M/S/V): frontend `tsc` + eslint + vitest green (manifest
  contract + resolver/selection tests) + operator smoke (a disk theme declaring
  motion/shader/video renders cinematically; reduced-motion downgrades).
- Shader slices: the GL-backend fallback-cap check; operator smoke on the game
  surface.
- One branch per thrust (or per slice batch) per the branch workflow; merge at
  playtestable milestones.

## Motion foundation — RESOLVED `[planning session 2026-06-16; was the agenda below]`

The planning session ran 2026-06-16. **All six open problems resolved → folded
into the new M0 foundation slice (Thrust M above) + decisions D53–D55.** Summary
of resolutions (the original agenda is preserved verbatim afterward for context):

1. **What composites?** → Don't catalogue from first principles; the **M0
   playground IS the catalogue** — exercise each technique on the real surface,
   record results in `MOTION.md`. Known so far: CSS `@keyframes` paint, WAAPI
   doesn't.
2. **Fast iteration loop** → **`cargo tauri dev` is the loop** (D53). The day-long
   tax was the full-build-per-tweak cost, NOT a dev/build behavior gap — compositing
   is identical (same Rust window-builder); only the WebView source differs (HMR vs
   bundled). Plus the M0 playground route. Operator agreed to run dev for motion work.
3. **Verification** → **Lightweight** (D55): playground as manual smoke + an
   `animationend` dev assertion. No screenshot-diff (the eye is the final guard).
4. **Window-present timing** → **Blessed** (D54): `oa://window-shown` is THE
   canonical "shell presented" signal; entrance/boot/attract all ride it.
5. **Is `DeclarativeShell` the right dogfood?** → **No for M2** (D55): it has no
   runtime view changes. Entrance stays there; **M2 view-transitions move to a
   navigable surface** (Retroverse/playground).
6. **Scroll-container interaction** → **Never animate the scroll container**; animate
   an inner non-scrolling wrapper (or opacity-only for scroll regions). Codified in
   `MOTION.md`; `ViewTransition` usage restructured at M0.

---

### Original agenda (preserved)

M1 (Slice 1) was attempted 2026-06-16 and revealed that **the motion foundation
isn't solid enough to keep building on**. Getting a single declarative entrance
transition to actually render took a full day of operator-in-the-loop round-trips
against the real transparent-WebView build, and the result still isn't satisfying.
The declarative *contract* (tokens, manifest field, validator, Rust carry-through,
resolver, dogfood) is sound and green and worth keeping — but the *rendering +
iteration substrate* needs deliberate design before S/V build on it. Branch
`theme-arc3-motion-slice-1` holds the work (NOT merged); diagnostics left in.

**Resolve these before resuming M1 acceptance / starting M2:**

1. **What animation techniques actually composite on OA's surface?** The
   single-window shell is a **transparent WebView2 composited over wgpu by DWM**.
   Confirmed the hard way: **the Web Animations API (`element.animate`) fires but
   never recomposites the transparent surface → invisible**; **CSS animations**
   (keyframes / the boot-fade pattern) DO render. Need a definitive catalogue —
   CSS transitions? transforms vs opacity vs filter? `will-change`/compositor
   layers? — so we never again ship motion that silently doesn't paint. This
   gates ALL of Thrust S/V too.
2. **A fast iteration loop for motion.** Every tweak this session cost a full
   `cargo tauri build`. Options to evaluate: `cargo tauri dev` (HMR + live
   devtools console) as the standard motion-dev loop; a dedicated **motion
   playground** route/page that exercises the primitives without a full boot;
   storybook-ish harness. Pick one — the current loop is untenable for cinematic
   work.
3. **Verification so we can't ship invisible motion.** WAAPI/jsdom can't test the
   real composite; our green vitest suite passed while nothing rendered. Need
   some visual/integration check (screenshot diff? a manual smoke checklist? a
   dev assertion that the animation actually painted?).
4. **Window-present timing is foundational, now partly solved.** Entrance/boot
   motion races the OS window reveal. M1 landed a real fix — Rust creates the
   window `.visible(false)` and shows it on a frontend first-paint handshake
   (`oa_shell_ready` → `oa://window-shown`, 5 s fallback). Decide if this is the
   blessed pattern (it also kills the launch white-flash) and whether boot
   animation / attract should ride the same signal.
5. **Is `DeclarativeShell` the right place to prove "view transitions"?** It's a
   SINGLE-surface browse shell with **no runtime view changes** — the only
   trigger is the entrance, which made M1 an awkward first proof. M2's premise
   (per-view/per-system transitions firing on layout change) needs an actual
   navigable surface. Reconsider the slice ordering / where motion is dogfooded.
6. **Scroll-container interaction.** Operator flagged the entrance interacting
   oddly with the list scrollbar (transforming a container that holds an
   `overflow-y-auto` child). Understand + decide the safe pattern (animate a
   non-scrolling wrapper? opacity-only for scroll regions?).

## Open questions deferred to execution

- **Motion keyframe model shape** — the minimal authorable keyframe vocabulary
  vs. a preset-only start; settle at M1/M3 against a dogfood (accrete additively).
- **Shader-preset selection granularity** — per-view vs per-system vs per-theme;
  settle at S1 reusing the ARC 2 resolver cascade.
- **Media-slot vocabulary depth** — how much of BigBox's ~47 `ImageTypes` to
  adopt; scope at V1, accrete.
- **Rhai sandbox design** — defer wholesale to Thrust R / ARC 3.5 planning.
