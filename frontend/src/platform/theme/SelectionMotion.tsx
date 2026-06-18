// SelectionMotion — the per-item selection + ambient player for the flat
// (no-hero) browse surface (ARC 3 Thrust M; the declarative selection/ambient
// hook). PRODUCT code, sibling to SpecTransition/AmbientMotion.
//
// THE NO-HERO REFRAMING. In a hero shell (the Graphics Lab) the `selection` slot
// drives ONE hero element whose content swaps as focus changes, so the preset
// reads as an entrance. The DeclarativeShell has NO hero — it's a flat grid/list
// of PERSISTENT cards — so here `selection` is the FOCUSED CARD playing its preset
// in place (a pop/grow on focus-gain) and `ambient` is an idle loop on the FOCUSED
// CARD ONLY. A theme that declares `[motion.selection]` / `[motion.ambient]` thus
// gets per-item choreography with ZERO render code: DeclarativeShell wraps every
// card in this.
//
// Wrap a card:
//   <SelectionMotion focused={ctx.focused} selection={sel} ambient={amb}
//                    reducedMotion={reduced}>{card}</SelectionMotion>
//
// TWO NESTED ELEMENTS on purpose: selection (scale/y) and ambient (breathe scale)
// both animate `transform`; on one element they'd clobber each other. Outer = the
// selection one-shot target; inner = the ambient loop (a reused <AmbientMotion>);
// the card renders inside both.
//
// LANDMINES handled (MOTION.md + DECISIONS D54/D58):
//  • Boot en-masse / "before the window is shown" (the M1 bug): the selection
//    one-shot records but SKIPS its first effect run, so a card that mounts
//    already-focused (index 0) stays quiet at boot. Only a real false→true focus
//    flip plays — and the first focus move is necessarily after the OS presents the
//    window (the user can't navigate before then), so no per-card `windowShown`
//    wiring is needed here.
//  • fill:both rest state: `compileMotionSpec` holds the TO frame (motionSpec.ts),
//    so a sustained-emphasis preset (`lift` → scale 1.08) would stay popped after
//    focus leaves. We CANCEL the selection anim on focus-loss → the element returns
//    to its CSS rest state. (For an entrance preset that ends at neutral this
//    cancel is a visual no-op.) That is exactly why selection is NOT a plain
//    SpecTransition: its null-spec path returns WITHOUT cancelling.
//  • 1000 live animations: ambient is gated to `focused()` (its spec → null when
//    not focused), so at most ONE ambient loop is ever live regardless of list size
//    — ListNav/GridNav render the full, unwindowed list.
//  • Interruptible: re-focusing the same card mid-flight cancels + restarts (WAAPI).
//  • Reduced motion: NOT re-implemented here (D58.6). Selection floors to a short
//    fade (REDUCED_MOTION_SPEC, matching SpecTransition); ambient floors to nothing
//    (AmbientMotion's own floor). Each player owns its floor.

import { createEffect, onCleanup, type JSX } from "solid-js";
import AmbientMotion from "./AmbientMotion";
import { compileMotionSpec, scaleMotionTiming, REDUCED_MOTION_SPEC, type MotionSpec } from "./motionSpec";
import { readGlobalMotionScale } from "./motion";
import { motionDebugEnabled } from "./motionDebug";

export type SelectionMotionProps = {
  /// Reactive focus state of THIS item — pass the nav primitive's `ctx.focused`.
  focused: () => boolean;
  /// The selection spec, played one-shot on focus-gain (null = no selection motion).
  selection?: () => MotionSpec | null;
  /// The ambient loop, run only while focused (null = no ambient).
  ambient?: () => MotionSpec | null;
  /// Live reduced-motion state; selection floors to a short fade, ambient to nothing.
  reducedMotion?: () => boolean;
  /// Layout-pass-through class applied to BOTH transform carriers (so a card that
  /// must fill its parent — `h-full w-full` — fills through both wrappers). Use
  /// sizing/layout classes only; anything box-affecting (padding/border) would
  /// double-apply across the two nested elements.
  class?: string;
  children: JSX.Element;
};

export default function SelectionMotion(props: SelectionMotionProps): JSX.Element {
  let el: HTMLDivElement | undefined;
  let anim: Animation | undefined;
  let isFirstRun = true;

  // Selection one-shot — plays on focus-GAIN, cancels on focus-LOSS (→ CSS rest).
  createEffect(() => {
    const focused = props.focused();
    const reduced = props.reducedMotion?.() ?? false;
    const wasFirst = isFirstRun;
    isFirstRun = false;
    const node = el;

    // Always drop any in-flight selection anim first. This is what returns a
    // sustained-emphasis preset (e.g. `lift`, held at scale 1.08 by fill:both) to
    // its CSS rest state the moment focus leaves.
    anim?.cancel();

    const spec: MotionSpec | null = !focused
      ? null
      : reduced
        ? REDUCED_MOTION_SPEC
        : (props.selection?.() ?? null);
    const compiled = spec ? compileMotionSpec(spec) : null;
    const skip = wasFirst
      ? "skip-initial"
      : !focused
        ? "defocus"
        : !node
          ? "no-node"
          : typeof node.animate !== "function"
            ? "no-waapi"
            : !compiled
              ? "no-op"
              : "";
    if (motionDebugEnabled())
      console.log(
        `[oa-theme-motion] SelectionMotion focused=${focused} reduced=${reduced} dur=${spec?.duration ?? 0} -> ${skip || "WAAPI-PLAY"}`,
      );
    if (skip || !node || !compiled) return;
    // Fold the global --motion-scale into the timing (WAAPI can't read var()).
    const scaled = scaleMotionTiming(compiled, readGlobalMotionScale(node));
    anim = node.animate(scaled.keyframes, scaled.options);
  });

  onCleanup(() => anim?.cancel());

  return (
    <div ref={el} class={props.class}>
      <AmbientMotion
        class={props.class}
        spec={() => (props.focused() ? (props.ambient?.() ?? null) : null)}
        reducedMotion={props.reducedMotion}
      >
        {props.children}
      </AmbientMotion>
    </div>
  );
}
