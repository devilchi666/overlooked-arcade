// The declarative motion BASIS (Theming ARC 3 Thrust M, M-mod.1; audit §2).
// PRODUCT code — the engine of the motion model; the Graphics Lab only exercises
// it. Pure (no DOM, no Solid) so it's trivially unit-testable; the DOM player is
// SpecTransition.tsx.
//
// THE IDEA (audit §2): don't parameterize per effect. Every named effect is a
// PRESET = curated defaults over ONE shared basis — a small set of timing
// primitives applied to a small set of separate, compositor-cheap channels. This
// module is that basis + the compiler that turns a spec into Web Animations API
// keyframes. (MOTION.md proved WAAPI composites on OA's transparent surface — the
// M1 "WAAPI invisible" finding was a window-timing misdiagnosis, since reversed.)
//
// SCOPE NOW: the one-shot entrance/transition subset — from→to channels + timing.
// The §2 primitives not yet modeled (repeat/direction/stagger/keyframes[]/
// composite, and the spring channel for selection choreography) widen this
// additively in M-mod.2+.

/// A single animatable channel as a `[from, to]` pair. Channels are kept SEPARATE
/// (Motion-style), never folded into one `transform` string by the author — the
/// compiler composes the transform.
export type MotionChannel = readonly [from: number, to: number];

/// The compositor-cheap quartet (audit §2 — validated as the BigBox-tier set in
/// MOTION.md): `opacity`, `translate {x,y}` (px), `scale`, `rotate` (deg). The
/// depth/3D and filter tiers widen this later, gated as "high ceiling".
export type MotionChannels = {
  opacity?: MotionChannel;
  x?: MotionChannel;
  y?: MotionChannel;
  scale?: MotionChannel;
  rotate?: MotionChannel;
};

/// A resolved, ready-to-play motion spec: timing primitives over a channel set.
/// `duration`/`delay` in ms; `easing` is a concrete CSS easing (keyword or
/// `cubic-bezier(...)`) — WAAPI can't resolve `var()`.
export type MotionSpec = {
  duration: number;
  delay?: number;
  easing?: string;
  channels: MotionChannels;
};

/// The default easing for a spec that omits one — an even decelerate
/// (easeOutCubic). Gentler than the shell's `--ease-out` expo curve, which
/// front-loads ~90% of the travel into the first third (reads as a pop, not a
/// glide — the 2026-06-17 lab finding).
export const DEFAULT_SPEC_EASING = "cubic-bezier(0.33, 1, 0.68, 1)";

/// The reduced-motion floor (audit/a11y): any spec collapses to a short, gentle
/// opacity fade — never instant (keeps a view change legible without vestibular
/// motion), matching `resolveViewTransition`'s floor.
export const REDUCED_MOTION_SPEC: MotionSpec = {
  duration: 120,
  easing: "ease-out",
  channels: { opacity: [0, 1] },
};

/// The transform string for one end of the animation (`which` = 0 from / 1 to).
/// Only channels present in the spec contribute; absent → `undefined` (no
/// transform key emitted, so unrelated transforms aren't clobbered).
function transformAt(ch: MotionChannels, which: 0 | 1): string | undefined {
  const parts: string[] = [];
  if (ch.x) parts.push(`translateX(${ch.x[which]}px)`);
  if (ch.y) parts.push(`translateY(${ch.y[which]}px)`);
  if (ch.scale) parts.push(`scale(${ch.scale[which]})`);
  if (ch.rotate) parts.push(`rotate(${ch.rotate[which]}deg)`);
  return parts.length ? parts.join(" ") : undefined;
}

/// The compiled WAAPI payload: the two-frame keyframe array + the timing options
/// `element.animate(keyframes, options)` takes.
export type CompiledMotion = {
  keyframes: Keyframe[];
  options: KeyframeAnimationOptions;
};

/// Compile a spec to a WAAPI keyframe set + options. Returns `null` for a no-op
/// (no channels, or non-positive duration) so the player can skip cleanly.
/// `fill: "both"` holds the FROM frame before and the TO frame after, so the
/// content rests at its end state.
export function compileMotionSpec(spec: MotionSpec): CompiledMotion | null {
  if (spec.duration <= 0) return null;
  const fromT = transformAt(spec.channels, 0);
  const toT = transformAt(spec.channels, 1);
  const hasOpacity = spec.channels.opacity != null;
  if (!fromT && !toT && !hasOpacity) return null;

  const from: Keyframe = {};
  const to: Keyframe = {};
  if (hasOpacity) {
    from.opacity = spec.channels.opacity![0];
    to.opacity = spec.channels.opacity![1];
  }
  if (fromT) from.transform = fromT;
  if (toT) to.transform = toT;

  return {
    keyframes: [from, to],
    options: {
      duration: spec.duration,
      delay: spec.delay ?? 0,
      easing: spec.easing ?? DEFAULT_SPEC_EASING,
      fill: "both",
    },
  };
}

/// The M1 view-transition presets (`fade`/`slide`/`scale`) expressed as the
/// SAME basis — i.e. a preset is just named defaults over §2 (audit §2's whole
/// premise). `none` → null (no animation). `distance` overrides the slide travel
/// (px); `duration`/`easing` override the timing. This is what lets presets and
/// fully-authored specs share one player.
export type PresetSpecOptions = {
  duration?: number;
  easing?: string;
  distance?: number;
};

export function presetToSpec(
  preset: "none" | "fade" | "slide" | "scale",
  opts: PresetSpecOptions = {},
): MotionSpec | null {
  if (preset === "none") return null;
  const duration = opts.duration ?? 320;
  const easing = opts.easing ?? DEFAULT_SPEC_EASING;
  const distance = opts.distance ?? 96;
  const channels: MotionChannels =
    preset === "fade"
      ? { opacity: [0, 1] }
      : preset === "slide"
        ? { opacity: [0, 1], y: [distance, 0] }
        : { opacity: [0, 1], scale: [0.88, 1] };
  return { duration, easing, channels };
}
