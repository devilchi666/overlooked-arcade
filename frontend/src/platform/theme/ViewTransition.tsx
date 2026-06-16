// ViewTransition — the UI/DOM-layer primitive that plays a resolved view
// transition when its trigger changes (Theming ARC 3 Thrust M, M1; D51 — UI
// cinematics are CSS/DOM, never wgpu, never scripted).
//
// INTERRUPTIBLE BY DESIGN (the BigBox blocking-storyboard bug, avoided). The
// children render synchronously and are ALWAYS live — the animation is purely
// visual, layered on top via the Web Animations API. When the trigger changes
// mid-animation, the in-flight animation is cancelled and a fresh one starts
// from the preset's "from" state ("settle-then-transition"): view switching is
// never blocked waiting for an animation to finish. preset "none" (or a
// zero/negative duration) → no animation at all.
//
// WAAPI (not CSS transitions) on purpose: `el.animate()` cancellation is the
// clean interruption primitive, and it needs no from→to class-flip dance. The
// trade-off is that `easing` must be a concrete curve (WAAPI can't resolve
// `var()`) — the resolver guarantees that (see motion.ts). In non-DOM/older
// contexts where `element.animate` is absent (vitest/jsdom), the component
// degrades to rendering its children with no animation.

import { createEffect, onCleanup, type JSX } from "solid-js";
import type { ResolvedViewTransition } from "./motion";

/// WAAPI keyframes per preset. `none` is handled by the caller (skipped), so it
/// has no entry. Each animates an "enter": from a displaced/transparent state
/// to the resting identity state.
const KEYFRAMES: Record<Exclude<ResolvedViewTransition["preset"], "none">, Keyframe[]> = {
  fade: [{ opacity: 0 }, { opacity: 1 }],
  slide: [
    { opacity: 0, transform: "translateY(24px)" },
    { opacity: 1, transform: "translateY(0)" },
  ],
  scale: [
    { opacity: 0, transform: "scale(0.96)" },
    { opacity: 1, transform: "scale(1)" },
  ],
};

export type ViewTransitionProps = {
  /// Tracked: when this accessor's value changes, the transition (re)plays.
  /// Typically the resolved layout primitive / view key.
  trigger: () => unknown;
  /// The resolved transition to play. Re-read on each trigger change (so a
  /// reduced-motion toggle takes effect on the next view change).
  transition: () => ResolvedViewTransition;
  class?: string;
  children: JSX.Element;
};

export default function ViewTransition(props: ViewTransitionProps): JSX.Element {
  let el: HTMLDivElement | undefined;
  let anim: Animation | undefined;

  createEffect(() => {
    const key = props.trigger(); // track — re-run on every view change
    const t = props.transition();
    const node = el;
    // [oa-theme-motion] diagnostic — lands in oa-current.log. Tells us, in one
    // line, whether the effect runs and why it does/doesn't animate.
    const reason =
      !node
        ? "no-node"
        : t.preset === "none"
          ? "preset-none"
          : t.durationMs <= 0
            ? "zero-duration"
            : typeof node.animate !== "function"
              ? "no-waapi"
              : "ANIMATE";
    console.log(
      `[oa-theme-motion] ViewTransition key=${String(key)} preset=${t.preset} dur=${t.durationMs} ease=${t.easing} -> ${reason}`,
    );
    if (!node) return;
    if (t.preset === "none" || t.durationMs <= 0) return;
    if (typeof node.animate !== "function") return; // jsdom / unsupported → no-op
    // Interruptible: drop any in-flight animation, start fresh. Never blocks
    // the (already-rendered) children.
    anim?.cancel();
    anim = node.animate(KEYFRAMES[t.preset], {
      duration: t.durationMs,
      easing: t.easing,
      fill: "backwards",
    });
  });

  onCleanup(() => anim?.cancel());

  return (
    <div ref={el} class={props.class}>
      {props.children}
    </div>
  );
}
