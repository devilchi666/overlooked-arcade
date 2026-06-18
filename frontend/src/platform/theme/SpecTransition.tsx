// SpecTransition — the basis-driven motion player (ARC 3 Thrust M, M-mod.1).
// PRODUCT code. The general successor to the M1 `ViewTransition` (CSS preset
// shortcut): it plays a full `MotionSpec` (audit §2 basis) via the Web Animations
// API on each `trigger` change. WAAPI (not CSS @keyframes) because a spec's
// channels/timing are arbitrary author data — no point minting a named @keyframes
// per spec — and MOTION.md confirmed WAAPI composites on OA's transparent surface
// (the M1 "WAAPI invisible" finding was a window-present timing misdiagnosis).
//
// INTERRUPTIBLE: children render synchronously and are always live (the animation
// is purely visual, never gates the view swap). On a re-trigger mid-flight we
// cancel the in-flight animation and start the new one — the BigBox blocking-
// storyboard bug, avoided.
//
// REDUCED MOTION: when set, any spec collapses to REDUCED_MOTION_SPEC (a short
// fade) — the a11y floor, never instant.
//
// NOTE (M-mod.1 sequencing): the M1 preset path (`ViewTransition` +
// `resolveViewTransition`) stays for existing themes (declarativeShell/bare/
// coverflow); migrating them onto this player — presets resolving to specs via
// `presetToSpec` — is the cleanup once the spec path is validated on the lab.

import { createEffect, onCleanup, type JSX } from "solid-js";
import { compileMotionSpec, scaleMotionTiming, REDUCED_MOTION_SPEC, type MotionSpec } from "./motionSpec";
import { readGlobalMotionScale } from "./motion";
import { motionDebugEnabled } from "./motionDebug";

export type SpecTransitionProps = {
  /// Tracked: when this changes, the spec (re)plays. Typically the route/view key.
  trigger: () => unknown;
  /// The spec to play, re-read on each trigger (null = no animation).
  spec: () => MotionSpec | null;
  /// Live reduced-motion state; when true the spec collapses to a short fade.
  reducedMotion?: () => boolean;
  /// When true, the FIRST (mount) effect run is recorded but NOT played — only
  /// subsequent `trigger` changes play. For a boot entrance the caller keys
  /// `trigger` on `oa://window-shown` (D54): the OS presents the window ~hundreds
  /// of ms after first paint, so a play at mount would run (and finish, given
  /// `fill: both`) while the window is still hidden — the M1 "entrance played
  /// before the window was shown" bug. Skipping the mount run means the only play
  /// is the windowShown flip, i.e. the instant the window is actually on screen.
  /// Default false (the lab + runtime transitions want the mount play).
  skipInitial?: boolean;
  class?: string;
  children: JSX.Element;
};

export default function SpecTransition(props: SpecTransitionProps): JSX.Element {
  let el: HTMLDivElement | undefined;
  let anim: Animation | undefined;
  let isFirstRun = true;

  createEffect(() => {
    const key = props.trigger(); // track — re-run on every view change
    const reduced = props.reducedMotion?.() ?? false;
    const spec: MotionSpec | null = reduced ? REDUCED_MOTION_SPEC : props.spec();
    const node = el;
    const compiled = spec ? compileMotionSpec(spec) : null;
    const wasFirst = isFirstRun;
    isFirstRun = false;
    const skip = props.skipInitial && wasFirst
      ? "skip-initial"
      : !node
        ? "no-node"
        : typeof node.animate !== "function"
          ? "no-waapi"
          : !compiled
            ? "no-op"
            : "";
    if (motionDebugEnabled())
      console.log(
        `[oa-theme-motion] SpecTransition key=${String(key)} reduced=${reduced} dur=${spec?.duration ?? 0} -> ${skip || "WAAPI-PLAY"}`,
      );
    if (!node || skip || !compiled) return;
    anim?.cancel(); // interruptible — drop any in-flight play before the new one
    // Fold the global --motion-scale into the timing (WAAPI can't read var()).
    const scaled = scaleMotionTiming(compiled, readGlobalMotionScale(node));
    anim = node.animate(scaled.keyframes, scaled.options);
  });

  onCleanup(() => anim?.cancel());

  return (
    <div ref={el} class={props.class}>
      {props.children}
    </div>
  );
}
