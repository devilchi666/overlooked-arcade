// `wheel` nav primitive — RESERVED CONTRACT (S5.5). A true radial / arc layout
// (the iconic BigBox wheel: items on a circle, the focused one at a fixed anchor,
// the wheel rotating under it). The S2 "Wheel" theme was renamed CoverFlow once
// it became clear it shipped a coverflow IA, not a radial wheel; the real radial
// primitive is parked here.
//
// WHY TYPED-BUT-UNIMPLEMENTED: the prop CONTRACT is the expensive-to-retrofit
// part (a future serializable layout DSL + Theme Studio target it; scope-call
// #8). The radial render itself has NO ARC-1 consumer — shipping a half-built
// radial layout would be dead code with subtle geometry bugs no theme exercises.
// So S5.5 reserves the shape and defers the implementation to when a wheel-using
// theme actually lands (ARC 2-3 cinematic shells are the natural home). Using it
// today renders nothing and warns once.

import { type Component, type JSX } from "solid-js";
import type { CarouselItemContext } from "./CarouselNav";
import type { NavPrimitiveBaseProps } from "./types";

/// The reserved radial-wheel contract. Mirrors CarouselNav where it can (a
/// windowed, focus-anchored ring) and adds the radial geometry knobs.
export type WheelNavProps<T> = Omit<NavPrimitiveBaseProps<T>, "children"> & {
  /** Ring radius in px. */
  radius: number;
  /** Total arc the visible items span, in degrees (360 = full ring). */
  arcDegrees?: number;
  /** Items kept in the DOM each side of the anchored focus. Default 6. */
  window?: number;
  /** Angle (deg, 0 = top) the focused item is anchored at. Default 0. */
  anchorAngle?: number;
  /** Rotation transition in ms. Default 300. */
  transitionMs?: number;
  /** Item renderer — same context shape as the carousel (incl. `offset`). */
  children: (item: T, ctx: CarouselItemContext) => JSX.Element;
};

let warned = false;

/// Reserved stub — renders nothing and warns once in DEV. Kept as a real export
/// so the contract is importable + a future implementation is a drop-in (same
/// name, same props) rather than a new primitive.
export function WheelNav<T>(_props: WheelNavProps<T>): ReturnType<Component> {
  if (import.meta.env.DEV && !warned) {
    warned = true;
    console.warn(
      "[oa-nav] WheelNav is a reserved S5.5 contract — the radial layout is not yet " +
        "implemented (deferred until a wheel-using theme lands). Use CarouselNav for a " +
        "horizontal coverflow today.",
    );
  }
  return null;
}
