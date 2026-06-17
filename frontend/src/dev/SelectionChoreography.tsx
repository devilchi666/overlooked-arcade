// SelectionChoreography — DEV-ONLY motion bench (Theming ARC 3 Thrust M, M0).
// The second tab of the F10 motion bench. The compositing probe proved the
// surface CAN move; this proves we can make it FEEL like BigBox.
//
// What it demonstrates, all on the real transparent surface:
//   • selection-driven choreography — on focus change the hero RE-PLAYS an
//     entrance: art scales in, title rises with overshoot, metadata STAGGERS up.
//   • a fanart CROSSFADE (two stacked gradient layers, new fades in over old).
//   • a rAF SPRING driving the list's vertical position (momentum/settle) — the
//     physics path the probe unlocked, in a real context.
//   • interruptible — scroll fast and the in-flight entrance restarts cleanly.
//
// Tune the FEEL live with the sliders (no rebuild). Structural changes ride HMR
// under `cargo tauri dev`. Findings/decisions → docs/.../MOTION.md. Never ships
// (gated behind import.meta.env.DEV at the App mount).

import { createEffect, createMemo, createSignal, For, onCleanup, onMount, Show, type JSX } from "solid-js";

type Game = {
  title: string;
  meta: string[]; // year · genre · players · rating — staggered in
  bg: string; // hero gradient (the "fanart")
  accent: string;
};

const GAMES: Game[] = [
  { title: "Neon Drift", meta: ["1992", "Racing", "1–2 Players", "★★★★☆"], bg: "linear-gradient(135deg,#1e1b4b,#7c3aed 60%,#22d3ee)", accent: "#a78bfa" },
  { title: "Astro Hauler", meta: ["1989", "Shoot 'em up", "1 Player", "★★★★★"], bg: "linear-gradient(135deg,#0c4a6e,#0284c7 55%,#7dd3fc)", accent: "#38bdf8" },
  { title: "Crypt of the Lich", meta: ["1994", "Action RPG", "1 Player", "★★★★☆"], bg: "linear-gradient(135deg,#3b0764,#a21caf 55%,#f0abfc)", accent: "#e879f9" },
  { title: "Turbo Gridiron", meta: ["1991", "Sports", "1–4 Players", "★★★☆☆"], bg: "linear-gradient(135deg,#14532d,#16a34a 55%,#bbf7d0)", accent: "#4ade80" },
  { title: "Pixel Samurai", meta: ["1993", "Fighting", "1–2 Players", "★★★★★"], bg: "linear-gradient(135deg,#7f1d1d,#dc2626 55%,#fecaca)", accent: "#f87171" },
  { title: "Deep Six", meta: ["1990", "Adventure", "1 Player", "★★★★☆"], bg: "linear-gradient(135deg,#083344,#0e7490 55%,#a5f3fc)", accent: "#22d3ee" },
  { title: "Star Carrier", meta: ["1995", "Strategy", "1 Player", "★★★☆☆"], bg: "linear-gradient(135deg,#1e293b,#475569 55%,#cbd5e1)", accent: "#94a3b8" },
  { title: "Mecha Brawl '88", meta: ["1988", "Fighting", "1–2 Players", "★★★★☆"], bg: "linear-gradient(135deg,#451a03,#ea580c 55%,#fed7aa)", accent: "#fb923c" },
  { title: "Glacier Run", meta: ["1992", "Platformer", "1 Player", "★★★★★"], bg: "linear-gradient(135deg,#172554,#2563eb 55%,#bfdbfe)", accent: "#60a5fa" },
  { title: "Voltage", meta: ["1996", "Puzzle", "1–2 Players", "★★★★☆"], bg: "linear-gradient(135deg,#422006,#ca8a04 55%,#fef08a)", accent: "#facc15" },
];

const ROW_H = 46;

/// A rAF spring toward a target, frame-rate independent (matters at 144Hz). This
/// is the physics path the compositing probe unlocked.
function createSpring(initial: number, stiffness: () => number, damping: () => number) {
  const [value, setValue] = createSignal(initial);
  let current = initial;
  let velocity = 0;
  let target = initial;
  let raf = 0;
  let last = 0;
  const tick = (t: number): void => {
    const dt = Math.min((last ? t - last : 16) / 1000, 0.04);
    last = t;
    const a = -stiffness() * (current - target) - damping() * velocity;
    velocity += a * dt;
    current += velocity * dt;
    setValue(current);
    if (Math.abs(velocity) > 0.02 || Math.abs(current - target) > 0.05) {
      raf = requestAnimationFrame(tick);
    } else {
      current = target;
      velocity = 0;
      setValue(target);
      raf = 0;
      last = 0;
    }
  };
  onCleanup(() => raf && cancelAnimationFrame(raf));
  return {
    value,
    set(t: number) {
      target = t;
      if (!raf) {
        last = 0;
        raf = requestAnimationFrame(tick);
      }
    },
  };
}

function Slider(props: { label: string; min: number; max: number; step?: number; value: () => number; set: (n: number) => void; unit?: string }): JSX.Element {
  return (
    <label class="flex items-center gap-2 text-[0.62rem] text-white/60">
      <span class="w-24 shrink-0">{props.label}</span>
      <input
        type="range"
        min={props.min}
        max={props.max}
        step={props.step ?? 1}
        value={props.value()}
        onInput={(e) => props.set(Number(e.currentTarget.value))}
        class="flex-1 accent-cyan-400"
      />
      <span class="w-12 shrink-0 text-right font-mono text-white/80">{Math.round(props.value())}{props.unit ?? ""}</span>
    </label>
  );
}

const KEYFRAMES = `
@keyframes oa-choreo-rise {
  from { opacity: 0; transform: translateY(22px) scale(.94); }
  to   { opacity: 1; transform: translateY(0)    scale(1); }
}
@keyframes oa-choreo-art {
  from { opacity: 0; transform: translateX(40px) scale(.92); }
  to   { opacity: 1; transform: translateX(0)    scale(1); }
}
@keyframes oa-choreo-bg-in { from { opacity: 0; } to { opacity: 1; } }
`;

export default function SelectionChoreography(): JSX.Element {
  const [sel, setSel] = createSignal(0);
  const [auto, setAuto] = createSignal(false);

  // Live-tunable feel.
  const [entranceMs, setEntranceMs] = createSignal(440);
  const [staggerMs, setStaggerMs] = createSignal(70);
  const [overshoot, setOvershoot] = createSignal(1.7); // back-ease overshoot
  const [stiffness, setStiffness] = createSignal(190);
  const [damping, setDamping] = createSignal(24);

  const game = createMemo(() => GAMES[sel()]);
  const ease = createMemo(() => `cubic-bezier(.34,${overshoot().toFixed(2)},.64,1)`);

  // List position spring (momentum/settle).
  const track = createSpring(0, stiffness, damping);
  createEffect(() => track.set(sel() * ROW_H));

  const move = (d: number): void => {
    setSel((s) => Math.max(0, Math.min(GAMES.length - 1, s + d)));
  };

  // Fanart crossfade: keep [leaving, current]; the new layer fades in OVER the
  // old (which is removed after the fade) → no flash, a real crossfade.
  const [bgLayers, setBgLayers] = createSignal<{ key: number; bg: string }[]>([{ key: 0, bg: GAMES[0].bg }]);
  let bgKey = 0;
  createEffect<number | undefined>((prev) => {
    const s = sel();
    if (prev === undefined) return s; // skip first run
    bgKey += 1;
    const k = bgKey;
    setBgLayers((ls) => [...ls.slice(-1), { key: k, bg: GAMES[s].bg }]);
    const id = setTimeout(() => setBgLayers((ls) => ls.filter((l) => l.key === k)), 700);
    onCleanup(() => clearTimeout(id));
    return s;
  });

  // Arrow keys (capture + stopImmediatePropagation so the app's nav doesn't also move).
  onMount(() => {
    const onKey = (e: KeyboardEvent): void => {
      if (e.key === "ArrowDown") { e.preventDefault(); e.stopImmediatePropagation(); move(1); }
      else if (e.key === "ArrowUp") { e.preventDefault(); e.stopImmediatePropagation(); move(-1); }
    };
    window.addEventListener("keydown", onKey, { capture: true });
    onCleanup(() => window.removeEventListener("keydown", onKey, { capture: true }));
  });

  // Autoplay (attract-style) to watch the choreography loop hands-free.
  createEffect(() => {
    if (!auto()) return;
    const id = setInterval(() => setSel((s) => (s + 1) % GAMES.length), 1700);
    onCleanup(() => clearInterval(id));
  });

  const riseStyle = (delay: number): JSX.CSSProperties => ({
    animation: `oa-choreo-rise ${entranceMs()}ms ${ease()} ${delay}ms both`,
  });

  return (
    <div class="flex h-full min-h-0 flex-col">
      <style>{KEYFRAMES}</style>

      {/* tuning + transport */}
      <div class="mb-3 flex flex-wrap items-center gap-x-6 gap-y-2 rounded-lg border border-white/10 bg-black/40 p-3">
        <div class="flex items-center gap-2">
          <button class="rounded border border-white/20 px-2 py-1 text-xs hover:bg-white/10" onClick={() => move(-1)}>↑ Prev</button>
          <button class="rounded border border-white/20 px-2 py-1 text-xs hover:bg-white/10" onClick={() => move(1)}>↓ Next</button>
          <button
            class="rounded border px-2 py-1 text-xs"
            classList={{ "border-cyan-400/60 bg-cyan-400/10 text-cyan-200": auto(), "border-white/20 hover:bg-white/10": !auto() }}
            onClick={() => setAuto((a) => !a)}
          >
            {auto() ? "■ Auto" : "▶ Auto"}
          </button>
          <span class="ml-1 text-[0.6rem] text-white/40">↑/↓ keys work too</span>
        </div>
        <div class="grid flex-1 grid-cols-1 gap-x-8 gap-y-1.5 sm:grid-cols-2 lg:grid-cols-3">
          <Slider label="entrance" min={120} max={900} value={entranceMs} set={setEntranceMs} unit="ms" />
          <Slider label="stagger" min={0} max={180} value={staggerMs} set={setStaggerMs} unit="ms" />
          <Slider label="overshoot" min={0} max={3} step={0.05} value={overshoot} set={setOvershoot} />
          <Slider label="spring k" min={40} max={400} value={stiffness} set={setStiffness} />
          <Slider label="damping" min={6} max={50} value={damping} set={setDamping} />
        </div>
      </div>

      {/* stage */}
      <div class="grid min-h-0 flex-1 grid-cols-[200px_1fr] gap-4">
        {/* the list — spring-driven track, selected row scales + brightens */}
        <div class="relative overflow-hidden rounded-xl border border-white/10 bg-black/30">
          <div class="absolute inset-x-0 top-1/2 -translate-y-1/2" style={{ transform: `translateY(calc(-50% - ${track.value()}px))` }}>
            <For each={GAMES}>
              {(g, i) => {
                const selected = createMemo(() => i() === sel());
                return (
                  <div
                    class="mx-2 flex items-center gap-2 rounded-md px-3 transition-[background-color,transform,opacity] duration-200"
                    style={{
                      height: `${ROW_H - 8}px`,
                      "margin-bottom": "8px",
                      transform: selected() ? "scale(1.04)" : "scale(1)",
                      opacity: selected() ? "1" : "0.5",
                      "background-color": selected() ? "rgba(255,255,255,0.08)" : "transparent",
                    }}
                  >
                    <span class="h-5 w-1 rounded-full transition-all duration-200" style={{ "background-color": g.accent, opacity: selected() ? "1" : "0.35", height: selected() ? "20px" : "12px" }} />
                    <span class="truncate text-xs" style={{ color: selected() ? "#fff" : "rgba(255,255,255,0.7)" }}>{g.title}</span>
                  </div>
                );
              }}
            </For>
          </div>
        </div>

        {/* the hero — crossfading fanart + replayed entrance choreography */}
        <div class="relative overflow-hidden rounded-xl border border-white/10">
          {/* fanart crossfade layers */}
          <div class="absolute inset-0 bg-[#0b0d12]" />
          <For each={bgLayers()}>
            {(layer) => (
              <div class="absolute inset-0" style={{ "background-image": layer.bg, animation: `oa-choreo-bg-in 650ms ease both` }} />
            )}
          </For>
          <div class="absolute inset-0 bg-gradient-to-t from-black/80 via-black/30 to-transparent" />

          {/* content remounts per selection (keyed) → entrance keyframes replay */}
          <Show when={game()} keyed>
            {(g) => (
              <div class="absolute inset-0 flex flex-col justify-end gap-3 p-7">
                <div class="self-end" style={riseStyle(40)}>
                  <div
                    class="grid h-28 w-40 place-items-center rounded-lg border border-white/20 text-xs font-bold text-black shadow-xl"
                    style={{ "background-image": `linear-gradient(135deg,#fff,${g.accent})`, animation: `oa-choreo-art ${entranceMs()}ms ${ease()} 40ms both` }}
                  >
                    BOX ART
                  </div>
                </div>
                <h2 class="text-4xl font-black tracking-tight text-white drop-shadow" style={riseStyle(0)}>
                  {g.title}
                </h2>
                <div class="flex flex-wrap gap-2">
                  <For each={g.meta}>
                    {(m, i) => (
                      <span
                        class="rounded-full border border-white/25 bg-black/40 px-3 py-1 text-[0.65rem] uppercase tracking-wider text-white/85"
                        style={riseStyle(120 + i() * staggerMs())}
                      >
                        {m}
                      </span>
                    )}
                  </For>
                </div>
              </div>
            )}
          </Show>
        </div>
      </div>

      <p class="mt-3 text-[0.6rem] text-white/35">
        Selection-driven choreography on the real surface. Dial the feel with the
        sliders; if it can match a BigBox theme here, the model is rich enough. Note
        what vocabulary you reach for — that becomes the declarative motion contract.
      </p>
    </div>
  );
}
