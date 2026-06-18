// AmbientMotion — the continuous-loop player (ARC 3 Thrust M, M-mod.3; audit §3
// category C). PRODUCT code, sibling to SpecTransition. Where SpecTransition plays
// a spec ON a trigger change (entrances/transitions), AmbientMotion plays a
// LOOPING spec once on mount and lets it run (breathe/float/glow — idle life).
//
// The spec carries its own loop shape (`repeat: "infinite"`, `direction:
// "alternate"`); this just starts it and cleans it up. WAAPI, like SpecTransition
// (MOTION.md: composites here).
//
// REDUCED MOTION: ambient = perpetual motion, the worst case for vestibular
// sensitivity — so under reduced motion this plays NOTHING (the element rests at
// its natural state). That's why ambient needs its own player rather than reusing
// SpecTransition (whose reduced path is a one-shot fade, wrong for a loop).

import { createEffect, onCleanup, type JSX } from "solid-js";
import { compileMotionSpec, type MotionSpec } from "./motionSpec";
import { motionDebugEnabled } from "./motionDebug";

export type AmbientMotionProps = {
  /// The looping spec (typically `repeat: "infinite"`). Re-read reactively, so a
  /// theme can swap or stop the ambient by returning a new spec / null.
  spec: () => MotionSpec | null;
  /// When true, no ambient plays (the a11y floor). The element stays at rest.
  reducedMotion?: () => boolean;
  class?: string;
  children: JSX.Element;
};

export default function AmbientMotion(props: AmbientMotionProps): JSX.Element {
  let el: HTMLDivElement | undefined;
  let anim: Animation | undefined;

  createEffect(() => {
    const reduced = props.reducedMotion?.() ?? false;
    const spec = props.spec();
    const node = el;
    anim?.cancel(); // drop any prior loop before (re)starting
    const skip = reduced
      ? "reduced"
      : !node
        ? "no-node"
        : typeof node.animate !== "function"
          ? "no-waapi"
          : !spec
            ? "no-op"
            : "";
    if (motionDebugEnabled())
      console.log(
        `[oa-theme-motion] AmbientMotion dur=${spec?.duration ?? 0} repeat=${spec?.repeat ?? "1"} -> ${skip || "WAAPI-LOOP"}`,
      );
    if (reduced || !node || typeof node.animate !== "function" || !spec) return;
    const compiled = compileMotionSpec(spec);
    if (!compiled) return;
    anim = node.animate(compiled.keyframes, compiled.options);
  });

  onCleanup(() => anim?.cancel());

  return (
    <div ref={el} class={props.class}>
      {props.children}
    </div>
  );
}
