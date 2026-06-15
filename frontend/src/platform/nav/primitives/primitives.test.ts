// Pure unit coverage for the S5.5/L4b primitive additions. The list/grid/
// carousel/wheel/custom primitives are component-behavioral (focus + DOM) and
// are verified by the operator playtest + typecheck/build, matching the existing
// pure-test approach (no Solid render harness is set up). What IS pure-testable
// lives here: the WheelNav radial geometry (L4b) + the nav-sound dispatcher's
// selector path.

import { describe, it, expect } from "vitest";
import { wheelDisplacement, wheelStepDeg } from "./wheelGeometry";
import { navSoundDispatcher } from "@oa/platform/themes/systemUiSound";

describe("wheelGeometry (WheelNav radial projection, L4b)", () => {
  // Shape-A defaults: right-side vertical wheel.
  const g = { radius: 400, anchorAngle: 270, arcDegrees: 140, window: 6 };

  it("spreads the visible window across arcDegrees", () => {
    // ±window items span the full arc → step = arc / (2·window).
    expect(wheelStepDeg(g)).toBeCloseTo(140 / 12, 6);
  });

  it("pins the focused item at the anchor (zero displacement)", () => {
    const d = wheelDisplacement(0, g);
    expect(d.dx).toBeCloseTo(0, 6);
    expect(d.dy).toBeCloseTo(0, 6);
  });

  it("puts the next item (positive offset) below focus, the previous above", () => {
    expect(wheelDisplacement(1, g).dy).toBeGreaterThan(0); // next → down
    expect(wheelDisplacement(-1, g).dy).toBeLessThan(0); // prev → up
  });

  it("curves neighbours away from the screen (rightward) on both sides — the wheel bulge", () => {
    // anchorAngle 270 = leftmost point of the circle, so any item off-focus is
    // further right (dx > 0); the arc is symmetric in dx for ±offset.
    expect(wheelDisplacement(3, g).dx).toBeGreaterThan(0);
    expect(wheelDisplacement(-3, g).dx).toBeCloseTo(wheelDisplacement(3, g).dx, 6);
  });

  it("grows the vertical displacement monotonically toward the window edge", () => {
    const ys = [1, 2, 3, 4, 5, 6].map((o) => wheelDisplacement(o, g).dy);
    for (let i = 1; i < ys.length; i++) expect(ys[i]).toBeGreaterThan(ys[i - 1]);
    // The window-edge item stays inside the ring's radius vertically.
    expect(Math.abs(ys[ys.length - 1])).toBeLessThanOrEqual(g.radius);
  });
});

describe("navSoundDispatcher", () => {
  it("returns a handler that no-ops (no throw) when the item has no system id", () => {
    const dispatch = navSoundDispatcher<{ systemId?: string }>(
      (it) => it?.systemId as never,
    );
    expect(typeof dispatch).toBe("function");
    expect(() => dispatch("move", undefined)).not.toThrow();
    expect(() => dispatch("confirm", {})).not.toThrow();
  });
});
