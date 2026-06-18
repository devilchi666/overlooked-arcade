// ViewTransition — the UI/DOM-layer primitive that plays a resolved view
// transition when its trigger changes (Theming ARC 3 Thrust M, M1; D51 — UI
// cinematics are CSS/DOM, never wgpu, never scripted).
//
// CSS-DRIVEN. NOTE (corrected 2026-06-17, ARC 3 M0 probe): the earlier claim
// that the Web Animations API "doesn't composite" on OA's transparent surface
// was a MISDIAGNOSIS — WAAPI paints fine here; M1's invisible entrance was a
// window-present *timing* bug (a one-shot played before the window was shown),
// fixed by the `oa://window-shown` handshake. See docs/.../MOTION.md. We keep
// this primitive CSS-based anyway because the transition is declarative,
// theme-authored data (duration/easing resolved from the theme, naming a keyframe
// in index.css: `oa-vt-fade|slide|scale`) — CSS is the right fit for that. WAAPI
// is reserved for computed/imperative motion (the M0 benches use it).
//
// INTERRUPTIBLE BY DESIGN (the BigBox blocking-storyboard bug, avoided). The
// children render synchronously and are ALWAYS live — the animation is purely
// visual. On a trigger change mid-animation we clear `animation`, force a
// reflow, and re-apply it ("settle-then-transition"): the standard CSS
// animation-restart technique. View switching is never blocked. preset "none"
// (or a zero/negative duration) → no animation.

import { createEffect, type JSX } from "solid-js";
import type { ResolvedViewTransition } from "./motion";
import { motionDebugEnabled } from "./motionDebug";

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
  /// Optional CSS `animation-delay` (ms) before the visible part starts. With
  /// `both` fill, the content is held at the FROM frame (hidden) during the
  /// delay. M1 uses this for the entrance: the OS window isn't presented until
  /// ~450 ms after mount, so a play at mount would finish while the window is
  /// still hidden (the "nothing seen" report — confirmed by the log: one
  /// `-> CSS-ANIMATE` at mount, invisible). Delaying the visible portion past
  /// window-present makes the one play land on-screen. Default 0.
  delayMs?: number;
  class?: string;
  children: JSX.Element;
};

export default function ViewTransition(props: ViewTransitionProps): JSX.Element {
  let el: HTMLDivElement | undefined;

  createEffect(() => {
    const key = props.trigger(); // track — re-run on every view change
    const t = props.transition();
    const node = el;
    const delay = props.delayMs ?? 0;
    const skip = !node ? "no-node" : t.preset === "none" ? "preset-none" : t.durationMs <= 0 ? "zero-duration" : "";
    if (motionDebugEnabled())
      console.log(
        `[oa-theme-motion] ViewTransition key=${String(key)} preset=${t.preset} dur=${t.durationMs} delay=${delay} -> ${skip || "CSS-ANIMATE"}`,
      );
    if (!node || skip || t.preset === "none") return;
    const keyframe = KEYFRAME_NAME[t.preset];
    // Restart the CSS animation: clear it, force a reflow so the browser sees a
    // genuine change, then re-apply. `both` holds the FROM frame during the
    // delay (content hidden until the window is presented) and the TO frame
    // after end (content at rest). Shorthand time order: duration, then delay.
    node.style.animation = "none";
    void node.offsetWidth; // reflow — required for the restart to take
    node.style.animation = `${keyframe} ${t.durationMs}ms ${t.easing} ${delay}ms both`;
  });

  return (
    <div ref={el} class={props.class}>
      {props.children}
    </div>
  );
}
