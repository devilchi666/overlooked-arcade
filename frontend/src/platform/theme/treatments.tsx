// Box-art & background TREATMENTS (Theming ARC 3 Thrust M; audit §3 categories D
// + the fanart background). PRODUCT code. These are the catalog entries that are
// NOT single `MotionSpec`s — they're reusable CSS/DOM components a theme drops
// around its cover art (or behind its surface). Extracted from the Graphics Lab's
// inline implementations so any theme gets the same premium box-art for free.
//
//   • <Gloss>           — frosted edge + a moving specular sweep (glass finish)
//   • <Reflection>      — a grounded, masked, flipped copy of the art
//   • <Shimmer>         — a standalone specular sweep across a surface
//   • <FanartCrossfade> — full-bleed backdrop that crossfades to a new image,
//                         with an optional Ken-Burns drift (the cinematic look)
//   • parallax-tilt     — see `useTilt` in tilt.ts (an interaction hook, not a
//                         component); re-exported here for one import site.
//
// Keyframes live in index.css (oa-sweep / oa-fade-in / oa-ken-burns) — engine
// territory, since treatments are product code. All honor reduced motion (the
// continuous sweep / Ken-Burns simply doesn't run).

import { createEffect, createSignal, For, onCleanup, Show, type JSX } from "solid-js";

export { useTilt } from "./tilt"; // parallax-tilt treatment

/// Glass finish: a static frosted edge highlight + a specular sweep that crosses
/// the surface on a loop. Place as an absolutely-positioned overlay INSIDE a
/// `position: relative` cover frame (it fills it).
export function Gloss(props: { sweepMs?: number; reducedMotion?: () => boolean; rounded?: string }): JSX.Element {
  const radius = (): string => props.rounded ?? "0.75rem";
  return (
    <>
      <div
        class="pointer-events-none absolute inset-0"
        style={{
          "border-radius": radius(),
          "background-image":
            "linear-gradient(135deg, rgba(255,255,255,.28), rgba(255,255,255,0) 38%, rgba(255,255,255,.04) 60%, rgba(255,255,255,.18))",
          "box-shadow": "inset 0 1px 0 rgba(255,255,255,.4), inset 0 0 0 1px rgba(255,255,255,.12)",
        }}
      />
      <div
        class="pointer-events-none absolute inset-0"
        style={{
          "border-radius": radius(),
          "background-image": "linear-gradient(110deg, transparent 38%, rgba(255,255,255,.45) 50%, transparent 62%)",
          "background-size": "250% 100%",
          animation: props.reducedMotion?.() ? "none" : `oa-sweep ${props.sweepMs ?? 4500}ms linear infinite`,
        }}
      />
    </>
  );
}

/// A standalone specular sweep (no frost) — for shimmering any surface (text,
/// chrome). Same overlay pattern as Gloss's second layer.
export function Shimmer(props: { sweepMs?: number; reducedMotion?: () => boolean; class?: string }): JSX.Element {
  return (
    <div
      class={`pointer-events-none absolute inset-0 ${props.class ?? ""}`}
      style={{
        "background-image": "linear-gradient(110deg, transparent 40%, rgba(255,255,255,.5) 50%, transparent 60%)",
        "background-size": "250% 100%",
        animation: props.reducedMotion?.() ? "none" : `oa-sweep ${props.sweepMs ?? 3200}ms linear infinite`,
      }}
    />
  );
}

/// A grounded reflection: a flipped, mask-faded copy of the art beneath it.
/// Render directly below the cover (it positions itself at `top: 100%`). `width`
/// must match the cover's width (CSS length).
export function Reflection(props: { src: () => string | null; width: string; height?: string }): JSX.Element {
  return (
    <div
      class="pointer-events-none absolute left-0 top-full overflow-hidden"
      style={{ width: props.width, height: props.height ?? "6rem" }}
      aria-hidden="true"
    >
      <Show when={props.src()}>
        {(u) => (
          <img
            src={u()}
            alt=""
            class="-scale-y-100 object-cover"
            style={{
              width: props.width,
              opacity: "0.28",
              "mask-image": "linear-gradient(to bottom, rgba(0,0,0,.7), transparent 70%)",
              "-webkit-mask-image": "linear-gradient(to bottom, rgba(0,0,0,.7), transparent 70%)",
            }}
          />
        )}
      </Show>
    </div>
  );
}

/// Full-bleed backdrop that CROSSFADES to a new image as `url` changes (audit
/// `fanart-crossfade`): the new layer fades in OVER the old, which is removed
/// after the fade — no flash. `kenBurns` adds a slow pan+zoom (idle life).
/// `filter` lets the caller blur/darken (the common "fanart from cover" look).
export function FanartCrossfade(props: {
  url: () => string | null;
  reducedMotion?: () => boolean;
  kenBurns?: boolean;
  filter?: string;
  fadeMs?: number;
  class?: string;
}): JSX.Element {
  // Layer stack: on each url change push a keyed layer + schedule the old one's
  // removal after the fade, so the new image fades in OVER the old (true
  // crossfade, no flash).
  const [layers, setLayers] = createSignal<{ key: number; url: string }[]>([]);
  let key = 0;
  const fade = (): number => props.fadeMs ?? 900;
  createEffect<string | null>((prev) => {
    const u = props.url();
    if (!u || u === prev) return u ?? prev ?? null;
    key += 1;
    const k = key;
    setLayers((ls) => [...ls.slice(-1), { key: k, url: u }]);
    const id = window.setTimeout(() => setLayers((ls) => ls.filter((l) => l.key === k)), fade());
    onCleanup(() => clearTimeout(id));
    return u;
  });
  return (
    <div class={`pointer-events-none absolute inset-0 ${props.class ?? ""}`}>
      <For each={layers()}>
        {(layer) => (
          <div
            class="absolute inset-0 bg-cover bg-center"
            style={{
              "background-image": `url("${layer.url}")`,
              filter: props.filter ?? "none",
              animation: `oa-fade-in ${fade()}ms ease forwards${
                props.kenBurns && !props.reducedMotion?.() ? ", oa-ken-burns 26s ease-in-out infinite alternate" : ""
              }`,
            }}
          />
        )}
      </For>
    </div>
  );
}
