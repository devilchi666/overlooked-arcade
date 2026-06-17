// MotionPlayground — DEV-ONLY compositing probe harness (Theming ARC 3 Thrust M,
// M0 foundation slice). Toggle with F10 while running `cargo tauri dev` in
// single-window mode.
//
// WHY THIS EXISTS: the M1 attempt proved the Web Animations API fires but never
// recomposites OA's transparent-WebView/DWM surface (it's invisible), while CSS
// @keyframes paint. That's a foundation-validity question, not a polish one —
// BigBox-tier motion leans on GPU-promoted transforms, rAF physics, and
// animation libraries (many of which use WAAPI under the hood). If those don't
// composite here, the "beat BigBox" goal is in trouble and we need to know NOW.
//
// HOW TO READ IT: every cell runs the SAME visible motion (a card sweeping
// left↔right, or a pulse) via a DIFFERENT technique, on the real surface. A cell
// that is FROZEN = that technique does not composite here. The rAF cells print a
// live ticking value, so a frozen-but-ticking cell proves the JS ran and only the
// PAINT failed (vs. JS never firing). The CRITICAL pair is `css-transform` vs.
// `css-transform + will-change`: if the plain one moves and the promoted one
// freezes, GPU-layer promotion is the killer (and explains WAAPI).
//
// Results get written up in docs/features/theming-substrate/MOTION.md. This file
// is gated behind import.meta.env.DEV at the App mount, so it never ships.

import { createSignal, onCleanup, onMount, Show, type JSX } from "solid-js";
import SelectionChoreography from "./SelectionChoreography";
import MotionShowcase from "./MotionShowcase";
import BoxArtFX from "./BoxArtFX";
import DragDropProbe from "./DragDropProbe";

const AMP = 180; // px of horizontal travel
const DUR = 1600; // ms per one-way sweep

// Keyframes injected scoped to this harness (not index.css — self-contained).
const KEYFRAMES = `
@keyframes oa-probe-tx     { 0% { transform: translateX(0); }            100% { transform: translateX(${AMP}px); } }
@keyframes oa-probe-tx3d   { 0% { transform: translate3d(0,0,0); }       100% { transform: translate3d(${AMP}px,0,0); } }
@keyframes oa-probe-op     { 0% { opacity: .12; }                        100% { opacity: 1; } }
@keyframes oa-probe-filter { 0% { filter: hue-rotate(0deg) saturate(1);} 100% { filter: hue-rotate(320deg) saturate(2.6); } }
@keyframes oa-probe-back   { 0% { backdrop-filter: blur(0px); }          100% { backdrop-filter: blur(10px); } }
`;

const cssAnim = (name: string): JSX.CSSProperties => ({
  animation: `${name} ${DUR}ms ease-in-out infinite alternate`,
});

/// One probe. `tone:"critical"` highlights the cells whose result decides the
/// toolkit (will-change promotion, rAF, WAAPI).
function Cell(props: {
  tag: string;
  label: string;
  verdict: string;
  tone?: "base" | "critical";
  children: JSX.Element;
}): JSX.Element {
  return (
    <div
      class="rounded-lg border p-3"
      classList={{
        "border-white/10 bg-black/40": props.tone !== "critical",
        "border-amber-400/50 bg-amber-400/[0.05]": props.tone === "critical",
      }}
    >
      <div class="mb-2 flex items-baseline justify-between gap-2">
        <span class="text-xs font-semibold text-white">{props.label}</span>
        <span class="font-mono text-[0.55rem] uppercase tracking-wider text-white/40">{props.tag}</span>
      </div>
      <div class="relative h-14 overflow-hidden rounded bg-white/[0.04]">{props.children}</div>
      <p class="mt-2 text-[0.62rem] leading-snug text-white/45">{props.verdict}</p>
    </div>
  );
}

/// The moving card — identical visual across cells so the eye compares like for
/// like. `extra` carries the per-cell style (the animation under test).
const card = (extra: JSX.CSSProperties, body?: JSX.Element): JSX.Element => (
  <div
    class="absolute left-2 top-2 grid h-10 w-16 place-items-center rounded-md text-[0.5rem] font-bold text-black"
    style={{
      "background-image": "linear-gradient(135deg, #f0abfc, #67e8f9)",
      ...extra,
    }}
  >
    {body}
  </div>
);

export default function MotionPlayground(props: { onClose: () => void }): JSX.Element {
  // Refs for the JS-driven cells.
  let rafEl: HTMLDivElement | undefined;
  let rafWcEl: HTMLDivElement | undefined;
  let waapiEl: HTMLDivElement | undefined;

  const [rafX, setRafX] = createSignal(0);
  const [ticks, setTicks] = createSignal(0);
  const [fps, setFps] = createSignal(0);
  const [fpsMin, setFpsMin] = createSignal(999);
  const [tab, setTab] = createSignal<"probe" | "choreo" | "showcase" | "boxart" | "dragdrop">("probe");

  onMount(() => {
    let raf = 0;
    let start = 0;
    let last = 0;
    let acc = 0;
    let frames = 0;
    let minSeen = 999;
    let waapi: Animation | undefined;

    if (waapiEl) {
      waapi = waapiEl.animate(
        [{ transform: "translateX(0)" }, { transform: `translateX(${AMP}px)` }],
        { duration: DUR, iterations: Infinity, direction: "alternate", easing: "ease-in-out" },
      );
    }

    const loop = (t: number): void => {
      if (!start) start = t;
      // Smooth there-and-back, matching the CSS `alternate` cells.
      const phase = ((t - start) % (DUR * 2)) / (DUR * 2);
      const x = AMP * (0.5 - 0.5 * Math.cos(phase * 2 * Math.PI));
      if (rafEl) rafEl.style.transform = `translateX(${x}px)`;
      if (rafWcEl) rafWcEl.style.transform = `translateX(${x}px)`;
      setRafX(Math.round(x));
      setTicks((n) => n + 1);

      // FPS (refresh-rate sanity — the operator runs 120Hz+).
      if (last) {
        const dt = t - last;
        acc += dt;
        frames += 1;
        if (acc >= 400) {
          const f = Math.round((frames * 1000) / acc);
          setFps(f);
          if (f < minSeen) {
            minSeen = f;
            setFpsMin(f);
          }
          acc = 0;
          frames = 0;
        }
      }
      last = t;
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);

    onCleanup(() => {
      cancelAnimationFrame(raf);
      waapi?.cancel();
    });
  });

  return (
    <div class="fixed inset-0 z-[200] bg-[#0b0d12] text-white">
      <style>{KEYFRAMES}</style>

      <div class="mx-auto flex h-full max-w-6xl flex-col px-6 py-6">
        <header class="mb-5 flex items-start justify-between gap-6">
          <div>
            <h1 class="text-lg font-bold tracking-tight">Motion compositing probe</h1>
            <p class="mt-1 max-w-3xl text-xs leading-relaxed text-white/55">
              Each cell runs the same motion via a different technique on the real
              transparent-WebView surface. A <b class="text-white">frozen</b> cell =
              that technique does not composite here. rAF cells print a live value —
              frozen-but-ticking means the JS ran and only the paint failed. Compare
              the amber cells: if <span class="font-mono">css-transform</span> moves
              but <span class="font-mono">+will-change</span> freezes, GPU promotion
              is the killer. Run under <span class="font-mono">cargo tauri dev</span>,
              single-window. F10 to close.
            </p>
          </div>
          <div class="shrink-0 text-right">
            <div class="font-mono text-2xl font-bold tabular-nums text-cyan-300">{fps()} fps</div>
            <div class="font-mono text-[0.6rem] text-white/40">min {fpsMin() === 999 ? "—" : fpsMin()} · rAF {ticks()}</div>
            <button
              class="mt-2 rounded border border-white/20 px-3 py-1 text-xs hover:bg-white/10"
              onClick={() => props.onClose()}
            >
              Close (F10)
            </button>
          </div>
        </header>

        <div class="mb-4 flex gap-1 border-b border-white/10">
          <button
            class="-mb-px border-b-2 px-3 py-1.5 text-xs font-semibold"
            classList={{ "border-cyan-400 text-white": tab() === "probe", "border-transparent text-white/45 hover:text-white/70": tab() !== "probe" }}
            onClick={() => setTab("probe")}
          >
            Compositing probe
          </button>
          <button
            class="-mb-px border-b-2 px-3 py-1.5 text-xs font-semibold"
            classList={{ "border-cyan-400 text-white": tab() === "choreo", "border-transparent text-white/45 hover:text-white/70": tab() !== "choreo" }}
            onClick={() => setTab("choreo")}
          >
            Selection choreography
          </button>
          <button
            class="-mb-px border-b-2 px-3 py-1.5 text-xs font-semibold"
            classList={{ "border-cyan-400 text-white": tab() === "showcase", "border-transparent text-white/45 hover:text-white/70": tab() !== "showcase" }}
            onClick={() => setTab("showcase")}
          >
            Motion showcase
          </button>
          <button
            class="-mb-px border-b-2 px-3 py-1.5 text-xs font-semibold"
            classList={{ "border-cyan-400 text-white": tab() === "boxart", "border-transparent text-white/45 hover:text-white/70": tab() !== "boxart" }}
            onClick={() => setTab("boxart")}
          >
            Box art FX
          </button>
          <button
            class="-mb-px border-b-2 px-3 py-1.5 text-xs font-semibold"
            classList={{ "border-cyan-400 text-white": tab() === "dragdrop", "border-transparent text-white/45 hover:text-white/70": tab() !== "dragdrop" }}
            onClick={() => setTab("dragdrop")}
          >
            Drag &amp; drop probe
          </button>
        </div>

        <Show when={tab() === "dragdrop"}>
          <DragDropProbe />
        </Show>

        <Show when={tab() === "choreo"}>
          <div class="min-h-0 flex-1">
            <SelectionChoreography />
          </div>
        </Show>

        <Show when={tab() === "showcase"}>
          <div class="min-h-0 flex-1">
            <MotionShowcase />
          </div>
        </Show>

        <Show when={tab() === "boxart"}>
          <div class="min-h-0 flex-1">
            <BoxArtFX />
          </div>
        </Show>

        <Show when={tab() === "probe"}>
          <div class="min-h-0 flex-1 overflow-y-auto pr-1">
        <div class="grid grid-cols-2 gap-3 md:grid-cols-3">
          <Cell
            tag="css/transform"
            label="CSS @keyframes — transform"
            verdict="Baseline. Expected to MOVE (boot-fade/focus-cards prove CSS paints here)."
          >
            {card(cssAnim("oa-probe-tx"))}
          </Cell>

          <Cell
            tag="css/transform+wc"
            label="CSS transform + will-change"
            tone="critical"
            verdict="CRITICAL. will-change promotes to a compositor layer. If this FREEZES while the plain one moves, GPU promotion is what kills compositing (and explains WAAPI)."
          >
            {card({ ...cssAnim("oa-probe-tx"), "will-change": "transform" })}
          </Cell>

          <Cell
            tag="css/translate3d"
            label="CSS @keyframes — translate3d"
            tone="critical"
            verdict="translate3d forces a GPU layer too. Second promotion vector — should agree with the will-change cell."
          >
            {card(cssAnim("oa-probe-tx3d"))}
          </Cell>

          <Cell
            tag="raf/transform"
            label="rAF → style.transform"
            tone="critical"
            verdict="CRITICAL. The physics/momentum path (springs, wheel inertia). If this MOVES, we can do JS-driven motion without WAAPI."
          >
            <div
              ref={rafEl}
              class="absolute left-2 top-2 grid h-10 w-16 place-items-center rounded-md text-[0.5rem] font-bold text-black"
              style={{ "background-image": "linear-gradient(135deg, #f0abfc, #67e8f9)" }}
            >
              {rafX()}
            </div>
          </Cell>

          <Cell
            tag="raf/transform+wc"
            label="rAF → transform + will-change"
            tone="critical"
            verdict="rAF physics path, but on a promoted layer. Tells us if physics survives GPU promotion."
          >
            <div
              ref={rafWcEl}
              class="absolute left-2 top-2 grid h-10 w-16 place-items-center rounded-md text-[0.5rem] font-bold text-black"
              style={{ "background-image": "linear-gradient(135deg, #f0abfc, #67e8f9)", "will-change": "transform" }}
            >
              {rafX()}
            </div>
          </Cell>

          <Cell
            tag="waapi/transform"
            label="WAAPI element.animate"
            tone="critical"
            verdict="Known-bad from M1 (expected FROZEN). Confirms it — and that any WAAPI-based lib (Motion One, etc.) is unusable on this surface."
          >
            <div
              ref={waapiEl}
              class="absolute left-2 top-2 grid h-10 w-16 place-items-center rounded-md text-[0.5rem] font-bold text-black"
              style={{ "background-image": "linear-gradient(135deg, #f0abfc, #67e8f9)" }}
            />
          </Cell>

          <Cell
            tag="css/transition"
            label="CSS transition (state toggle)"
            verdict="transition + a class flip every sweep. Discrete state-change motion (selection crossfades, enter/exit)."
          >
            <div
              class="absolute left-2 top-2 h-10 w-16 rounded-md"
              style={{
                "background-image": "linear-gradient(135deg, #f0abfc, #67e8f9)",
                transition: `transform ${DUR}ms ease-in-out`,
                animation: `oa-probe-tx ${DUR * 2}ms steps(1) infinite`,
              }}
            />
          </Cell>

          <Cell
            tag="css/opacity"
            label="CSS @keyframes — opacity"
            verdict="Opacity pulse (non-transform). Crossfades. Expected to work — opacity is main-thread-cheap."
          >
            {card(cssAnim("oa-probe-op"))}
          </Cell>

          <Cell
            tag="css/filter"
            label="CSS @keyframes — filter"
            verdict="Animated hue-rotate/saturate. Glows, color shifts. Does filter composite here?"
          >
            {card({ ...cssAnim("oa-probe-filter"), "background-image": "linear-gradient(135deg, #f0abfc, #f59e0b)" })}
          </Cell>

          <Cell
            tag="css/backdrop"
            label="backdrop-filter (animated blur)"
            verdict="Frosted-glass depth. backdrop-filter is expensive + flaky in WebView2 — a real risk to flag."
          >
            <div class="absolute inset-0 grid grid-cols-6 opacity-60">
              {Array.from({ length: 24 }, (_, i) => (
                <div classList={{ "bg-fuchsia-500/40": i % 2 === 0, "bg-cyan-400/40": i % 2 === 1 }} />
              ))}
            </div>
            <div class="absolute left-2 top-2 h-10 w-16 rounded-md border border-white/30" style={cssAnim("oa-probe-back")} />
          </Cell>

          <Cell
            tag="scroll/transform-parent"
            label="Animate a scroll CONTAINER"
            tone="critical"
            verdict="Open problem #6: the animated element WRAPS an overflow-y-auto child. Scroll it while it animates — watch the scrollbar glitch/clip. Confirms the 'never animate the scroll container' rule."
          >
            <div class="absolute inset-1" style={cssAnim("oa-probe-tx")}>
              <div class="h-full w-24 overflow-y-auto rounded bg-white/10 p-1 text-[0.5rem]">
                {Array.from({ length: 20 }, (_, i) => (
                  <div class="py-0.5">row {i}</div>
                ))}
              </div>
            </div>
          </Cell>

          <Cell
            tag="scroll/transform-inner"
            label="Animate INSIDE a stable scroll box"
            verdict="The safe pattern: scroll viewport stays put, an inner wrapper transforms. Scrollbar should behave."
          >
            <div class="absolute inset-1 overflow-y-auto rounded bg-white/10 p-1 text-[0.5rem]">
              <div style={cssAnim("oa-probe-tx")} class="mb-1 h-6 w-16 rounded bg-cyan-300/80" />
              {Array.from({ length: 20 }, (_, i) => (
                <div class="py-0.5">row {i}</div>
              ))}
            </div>
          </Cell>
        </div>

        <p class="mt-5 text-[0.6rem] text-white/35">
          Record which cells move vs. freeze in docs/features/theming-substrate/MOTION.md.
          The eye is the verdict — a unit test can't see the DWM composite.
        </p>
          </div>
        </Show>
      </div>
    </div>
  );
}
