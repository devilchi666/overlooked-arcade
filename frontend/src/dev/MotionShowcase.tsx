// MotionShowcase — DEV-ONLY motion exploration bench (Theming ARC 3 Thrust M, M0).
// The third tab of the F10 bench. A "what's possible" sampler now that the probe
// proved EVERYTHING composites on OA's transparent surface. Three sections:
//
//   1. Big moments — JS/WAAPI-driven computed moves (art grows ~5x to true screen
//      center and back; title orbits the window and lands home). WAAPI on purpose:
//      a live proof the M1 "WAAPI invisible" finding was a misdiagnosis.
//   2. In / out transitions — explicit leave+enter pairs (fade, slide, scale,
//      flip, blur), cycled so you can compare exit choreography.
//   3. Ambient loops — a grid of continuous CSS effects (float, glow, breathe,
//      tilt, shimmer, drift, hue, blur, orbit, wobble) for picking a vocabulary.
//
// Never ships (gated behind import.meta.env.DEV at the App mount).

import { createSignal, For, onCleanup, type JSX } from "solid-js";

const KEYFRAMES = `
/* ambient loops */
@keyframes oa-sc-float   { 0%,100%{transform:translateY(0)}      50%{transform:translateY(-14px)} }
@keyframes oa-sc-breathe { 0%,100%{transform:scale(1)}           50%{transform:scale(1.14)} }
@keyframes oa-sc-glow    { 0%,100%{box-shadow:0 0 6px 0 rgba(56,189,248,.45)} 50%{box-shadow:0 0 30px 8px rgba(56,189,248,.95)} }
@keyframes oa-sc-tilt    { 0%,100%{transform:rotateX(0) rotateY(0)} 50%{transform:rotateX(20deg) rotateY(22deg)} }
@keyframes oa-sc-hue     { from{filter:hue-rotate(0deg)}          to{filter:hue-rotate(360deg)} }
@keyframes oa-sc-blur    { 0%,100%{filter:blur(0)}               50%{filter:blur(5px)} }
@keyframes oa-sc-wobble  { 0%,100%{transform:rotate(-7deg)}      50%{transform:rotate(7deg)} }
@keyframes oa-sc-drift   { from{background-position:0% 50%}       to{background-position:200% 50%} }
@keyframes oa-sc-orbit   { from{transform:rotate(0) translateX(20px) rotate(0)} to{transform:rotate(360deg) translateX(20px) rotate(-360deg)} }
@keyframes oa-sc-shimmer { from{background-position:-160% 0}      to{background-position:260% 0} }
/* in / out */
@keyframes oa-io-fade-out  { to{opacity:0} }
@keyframes oa-io-fade-in   { from{opacity:0} }
@keyframes oa-io-slide-out { to{opacity:0; transform:translateX(-70px)} }
@keyframes oa-io-slide-in  { from{opacity:0; transform:translateX(70px)} }
@keyframes oa-io-scale-out { to{opacity:0; transform:scale(.65)} }
@keyframes oa-io-scale-in  { from{opacity:0; transform:scale(1.35)} }
@keyframes oa-io-flip-out  { to{opacity:0; transform:perspective(700px) rotateY(90deg)} }
@keyframes oa-io-flip-in   { from{opacity:0; transform:perspective(700px) rotateY(-90deg)} }
@keyframes oa-io-blur-out  { to{opacity:0; filter:blur(12px)} }
@keyframes oa-io-blur-in   { from{opacity:0; filter:blur(12px)} }
`;

const AMBIENT: { label: string; anim: string; extra?: JSX.CSSProperties }[] = [
  { label: "float", anim: "oa-sc-float 2.4s ease-in-out infinite" },
  { label: "breathe", anim: "oa-sc-breathe 2.6s ease-in-out infinite" },
  { label: "glow", anim: "oa-sc-glow 1.8s ease-in-out infinite" },
  { label: "3D tilt", anim: "oa-sc-tilt 3s ease-in-out infinite" },
  { label: "hue cycle", anim: "oa-sc-hue 4s linear infinite" },
  { label: "blur breathe", anim: "oa-sc-blur 2.4s ease-in-out infinite" },
  { label: "wobble", anim: "oa-sc-wobble 1.6s ease-in-out infinite" },
  {
    label: "gradient drift",
    anim: "oa-sc-drift 4s linear infinite",
    extra: { "background-image": "linear-gradient(90deg,#7c3aed,#22d3ee,#7c3aed)", "background-size": "200% 100%" },
  },
  { label: "orbit", anim: "oa-sc-orbit 2.2s linear infinite" },
  {
    label: "shimmer sweep",
    anim: "oa-sc-shimmer 2.2s linear infinite",
    extra: { "background-image": "linear-gradient(110deg,transparent 30%,rgba(255,255,255,.55) 50%,transparent 70%)", "background-size": "200% 100%", "background-color": "#334155" },
  },
];

const IO_PRESETS: { name: string; out: string; in: string }[] = [
  { name: "Fade", out: "oa-io-fade-out", in: "oa-io-fade-in" },
  { name: "Slide", out: "oa-io-slide-out", in: "oa-io-slide-in" },
  { name: "Scale", out: "oa-io-scale-out", in: "oa-io-scale-in" },
  { name: "Flip 3D", out: "oa-io-flip-out", in: "oa-io-flip-in" },
  { name: "Blur", out: "oa-io-blur-out", in: "oa-io-blur-in" },
];
const IO_DUR = 720;
const IO_ITEMS = [
  { label: "Neon Drift", bg: "linear-gradient(135deg,#7c3aed,#22d3ee)" },
  { label: "Astro Hauler", bg: "linear-gradient(135deg,#0284c7,#7dd3fc)" },
  { label: "Pixel Samurai", bg: "linear-gradient(135deg,#dc2626,#fecaca)" },
  { label: "Voltage", bg: "linear-gradient(135deg,#ca8a04,#fef08a)" },
];

export default function MotionShowcase(): JSX.Element {
  let artEl: HTMLDivElement | undefined;
  let titleEl: HTMLHeadingElement | undefined;

  // Timer lifecycle — onCleanup only registers at component scope (not inside
  // click handlers), so track timers here and clear them on unmount.
  let loopIv: number | undefined;
  const pending = new Set<number>();
  const after = (fn: () => void, ms: number): void => {
    const id = window.setTimeout(() => {
      pending.delete(id);
      fn();
    }, ms);
    pending.add(id);
  };
  onCleanup(() => {
    if (loopIv) clearInterval(loopIv);
    pending.forEach((id) => clearTimeout(id));
  });

  // --- Big moments (WAAPI, computed) ----------------------------------------
  const growToCenter = (): void => {
    if (!artEl) return;
    const r = artEl.getBoundingClientRect();
    const dx = window.innerWidth / 2 - (r.left + r.width / 2);
    const dy = window.innerHeight / 2 - (r.top + r.height / 2);
    artEl.animate(
      [
        { transform: "translate(0,0) scale(1)" },
        { transform: `translate(${dx}px,${dy}px) scale(5)`, offset: 0.42 },
        { transform: `translate(${dx}px,${dy}px) scale(5)`, offset: 0.62 },
        { transform: "translate(0,0) scale(1)" },
      ],
      { duration: 3200, easing: "ease-in-out" },
    );
  };

  const swirl = (): void => {
    if (!titleEl) return;
    const R = 230;
    const N = 36;
    const frames: Keyframe[] = [];
    for (let i = 0; i <= N; i += 1) {
      const t = i / N;
      const a = t * 2 * Math.PI;
      const x = Math.sin(a) * R;
      const y = -(1 - Math.cos(a)) * R; // 0 at ends, arcs up and around
      frames.push({ transform: `translate(${x}px,${y}px) rotate(${t * 360}deg)` });
    }
    titleEl.animate(frames, { duration: 2600, easing: "ease-in-out" });
  };

  const [looping, setLooping] = createSignal(false);
  const toggleLoop = (): void => {
    if (looping()) {
      setLooping(false);
      if (loopIv) {
        clearInterval(loopIv);
        loopIv = undefined;
      }
      return;
    }
    setLooping(true);
    const run = (): void => {
      growToCenter();
      after(swirl, 3400);
    };
    run();
    loopIv = window.setInterval(run, 6400);
  };

  // --- In / out swap (two-layer leave+enter) --------------------------------
  const [preset, setPreset] = createSignal(0);
  let cur = 0;
  let stackKey = 0;
  const [stack, setStack] = createSignal<{ key: number; idx: number; anim: string }[]>([
    { key: 0, idx: 0, anim: "" },
  ]);
  const swap = (): void => {
    const p = IO_PRESETS[preset()];
    const next = (cur + 1) % IO_ITEMS.length;
    setStack((s) => s.map((l) => ({ ...l, anim: `${p.out} ${IO_DUR}ms ease both` })));
    stackKey += 1;
    const k = stackKey;
    setStack((s) => [...s.slice(-1), { key: k, idx: next, anim: `${p.in} ${IO_DUR}ms ease both` }]);
    cur = next;
    after(() => setStack((s) => s.filter((l) => l.key === k)), IO_DUR + 40);
    setPreset((x) => (x + 1) % IO_PRESETS.length); // next swap shows a new style
  };

  return (
    <div class="flex h-full min-h-0 flex-col gap-4 overflow-y-auto pr-1">
      <style>{KEYFRAMES}</style>

      {/* 1. Big moments */}
      <section class="rounded-xl border border-white/10 bg-black/30 p-4">
        <div class="mb-3 flex flex-wrap items-center gap-2">
          <h3 class="mr-2 text-sm font-bold">Big moments</h3>
          <button class="rounded border border-white/20 px-2.5 py-1 text-xs hover:bg-white/10" onClick={growToCenter}>Grow to center (5×)</button>
          <button class="rounded border border-white/20 px-2.5 py-1 text-xs hover:bg-white/10" onClick={swirl}>Title swirl</button>
          <button
            class="rounded border px-2.5 py-1 text-xs"
            classList={{ "border-cyan-400/60 bg-cyan-400/10 text-cyan-200": looping(), "border-white/20 hover:bg-white/10": !looping() }}
            onClick={toggleLoop}
          >
            {looping() ? "■ Stop loop" : "▶ Loop both"}
          </button>
          <span class="text-[0.6rem] text-white/40">WAAPI-driven, computed to the real viewport center</span>
        </div>
        <div class="relative flex h-44 items-end justify-between px-6 pb-4">
          <h2 ref={titleEl} class="text-3xl font-black tracking-tight text-white drop-shadow">
            Neon Drift
          </h2>
          <div
            ref={artEl}
            class="grid h-24 w-16 place-items-center rounded-lg border border-white/25 text-[0.55rem] font-bold text-black shadow-2xl"
            style={{ "background-image": "linear-gradient(135deg,#fff,#a78bfa)", "transform-origin": "center" }}
          >
            BOX ART
          </div>
        </div>
      </section>

      {/* 2. In / out transitions */}
      <section class="rounded-xl border border-white/10 bg-black/30 p-4">
        <div class="mb-3 flex flex-wrap items-center gap-2">
          <h3 class="mr-2 text-sm font-bold">In / out transitions</h3>
          <button class="rounded border border-white/20 px-2.5 py-1 text-xs hover:bg-white/10" onClick={swap}>Swap →</button>
          <span class="text-[0.6rem] text-white/50">next: <b class="text-white/80">{IO_PRESETS[preset()].name}</b> (leave + enter pair; cycles each swap)</span>
        </div>
        <div class="relative h-28 overflow-hidden rounded-lg bg-[#0b0d12]">
          <For each={stack()}>
            {(layer) => (
              <div
                class="absolute inset-0 grid place-items-center text-lg font-bold text-black"
                style={{ "background-image": IO_ITEMS[layer.idx].bg, animation: layer.anim }}
              >
                {IO_ITEMS[layer.idx].label}
              </div>
            )}
          </For>
        </div>
      </section>

      {/* 3. Ambient loops */}
      <section class="rounded-xl border border-white/10 bg-black/30 p-4">
        <h3 class="mb-3 text-sm font-bold">Ambient loops <span class="text-[0.6rem] font-normal text-white/40">(continuous — for background life)</span></h3>
        <div class="grid grid-cols-3 gap-3 sm:grid-cols-5">
          <For each={AMBIENT}>
            {(a) => (
              <div class="flex flex-col items-center gap-2">
                <div class="grid h-16 w-16 place-items-center rounded-lg" style={{ "background-color": "#1e293b", "transform-style": "preserve-3d", animation: a.anim, ...a.extra }}>
                  <span class="text-[0.5rem] font-bold text-white/70">{a.label}</span>
                </div>
              </div>
            )}
          </For>
        </div>
      </section>

      <p class="text-[0.6rem] text-white/35">
        Sampling what the surface can do. Tell me which of these read as "premium"
        vs. "gimmick" — the keepers become presets in the declarative motion model.
      </p>
    </div>
  );
}
