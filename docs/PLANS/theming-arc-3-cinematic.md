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

- **M1 `[SLICE 1 — ✅ SHIPPED 2026-06-16]`** — Motion token contract +
  reduced-motion plumbing + one interruptible view transition, declared in
  `theme.toml` (motion fields → `ThemeManifest` + `validateTheme`) and honored by
  `DeclarativeShell`. Smallest valuable, disk-theme-flowing start; establishes the
  contract M2/M3 build on. Shipped: `ThemeMotionTokens`/`MOTION_TOKEN_VAR` +
  scoped `<style>` injection (re-asserts the reduced-motion floor);
  `manifest.motion.view_transition` (preset + timing) carried through the Rust
  disk loader + `diskTheme.ts`; `motion.ts` resolver
  (`resolveViewTransition`/`usePrefersReducedMotion`) + the WAAPI `ViewTransition`
  primitive (interruptible). Reduced-motion → short fade. Dogfood: `neon-list`
  (slide) + `bare-declarative` (fade + motion tokens).
- **M2** — View/route transition presets (fade/slide/scale), interruptible;
  per-view/per-system selection via the ARC 2 resolver pattern.
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

## Open questions deferred to execution

- **Motion keyframe model shape** — the minimal authorable keyframe vocabulary
  vs. a preset-only start; settle at M1/M3 against a dogfood (accrete additively).
- **Shader-preset selection granularity** — per-view vs per-system vs per-theme;
  settle at S1 reusing the ARC 2 resolver cascade.
- **Media-slot vocabulary depth** — how much of BigBox's ~47 `ImageTypes` to
  adopt; scope at V1, accrete.
- **Rhai sandbox design** — defer wholesale to Thrust R / ARC 3.5 planning.
