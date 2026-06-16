// ViewTransition — the UI/DOM-layer primitive that plays a resolved view
// transition when its trigger changes (Theming ARC 3 Thrust M, M1; D51 — UI
// cinematics are CSS/DOM, never wgpu, never scripted).
//
// CSS-DRIVEN, not the Web Animations API. Single-window mode is a transparent
// WebView2 composited over wgpu by DWM; WAAPI animations fire (confirmed via
// the oa-theme-motion log: `-> ANIMATE`) but don't recomposite the transparent
// surface, so nothing is seen. CSS animations run through the normal
// style/layout/paint pipeline that DWM recomposites — the boot fade + focus
// cards in this app prove CSS works here. We set the `animation` shorthand
// inline (duration/easing resolved from the theme) naming a keyframe defined in
// index.css (`oa-vt-fade|slide|scale`).
//
// INTERRUPTIBLE BY DESIGN (the BigBox blocking-storyboard bug, avoided). The
// children render synchronously and are ALWAYS live — the animation is purely
// visual. On a trigger change mid-animation we clear `animation`, force a
// reflow, and re-apply it ("settle-then-transition"): the standard CSS
// animation-restart technique. View switching is never blocked. preset "none"
// (or a zero/negative duration) → no animation.

import { createEffect, type JSX } from "solid-js";
import type { ResolvedViewTransition } from "./motion";

/// CSS @keyframes name per preset (defined in index.css). `none` is handled by
/// the caller (skipped), so it has no entry.
const KEYFRAME_NAME: Record<Exclude<ResolvedViewTransition["preset"], "none">, string> = {
  fade: "oa-vt-fade",
  slide: "oa-vt-slide",
  scale: "oa-vt-scale",
};

export type ViewTransitionProps = {
  /// Tracked: when this accessor's value changes, the transition (re)plays.
  /// Typically the resolved layout/view key.
  trigger: () => unknown;
  /// The resolved transition to play. Re-read on each trigger change (so a
  /// reduced-motion toggle takes effect on the next view change).
  transition: () => ResolvedViewTransition;
  class?: string;
  children: JSX.Element;
};

export default function ViewTransition(props: ViewTransitionProps): JSX.Element {
  let el: HTMLDivElement | undefined;
  // Skip the initial (mount) run: the children paint at rest, and we only
  // animate on a SUBSEQUENT trigger change (the deferred entrance / a real view
  // change). Without this the mount run + the entrance would both play → strobe.
  let primed = false;

  createEffect(() => {
    props.trigger(); // track — re-run on every view change
    const t = props.transition();
    const node = el;
    if (!primed) {
      // Establish the baseline at rest; don't animate the initial mount.
      primed = true;
      return;
    }
    if (!node || t.preset === "none" || t.durationMs <= 0) return;
    const keyframe = KEYFRAME_NAME[t.preset];
    // Restart the CSS animation: clear it, force a reflow so the browser sees a
    // genuine change, then re-apply. `both` holds the first frame before start
    // and the last after end (so the content rests fully visible).
    node.style.animation = "none";
    void node.offsetWidth; // reflow — required for the restart to take
    node.style.animation = `${keyframe} ${t.durationMs}ms ${t.easing} both`;
  });

  return (
    <div ref={el} class={props.class}>
      {props.children}
    </div>
  );
}
