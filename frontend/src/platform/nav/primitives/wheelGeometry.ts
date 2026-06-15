// Pure radial-wheel geometry for WheelNav (L4b). Kept separate from the
// component so the projection math — the bug-prone, playtest-sensitive part of
// the wheel — is unit-testable without a Solid render harness (mirrors the
// spatial engine's pure `spatialGeometry.ts` split).
//
// The model: items sit on a circle of `radius`. The focused item is at
// `anchorAngle` (deg, 0 = top, clockwise); each other item is `stepDeg` further
// along the arc per step away from focus. We never need the circle's absolute
// centre — only each item's pixel DELTA from wherever the focused item is
// pinned on screen. Positive offset (the next / higher-index item) rotates so
// it lands BELOW focus, so the wheel reads like a vertical list.

const DEG = Math.PI / 180;

export type WheelGeometry = {
  /** Ring radius in px. */
  radius: number;
  /** Angle (deg, 0 = top, clockwise) the focused item sits at on the circle. */
  anchorAngle: number;
  /** Total arc the visible window (±`window` items) spans, in degrees. */
  arcDegrees: number;
  /** Items visible each side of focus (the windowing half-width). */
  window: number;
};

/// Degrees between adjacent items so the full ±`window` window spans `arcDegrees`.
export function wheelStepDeg(g: WheelGeometry): number {
  return g.arcDegrees / (2 * g.window);
}

/// Pixel displacement of an item `offset` steps from focus, relative to the
/// on-screen point where the focused item is pinned. `dy > 0` is downward
/// (screen coords), so a positive offset (next item) is below focus.
export function wheelDisplacement(offset: number, g: WheelGeometry): { dx: number; dy: number } {
  const aRad = g.anchorAngle * DEG;
  const theta = (g.anchorAngle - offset * wheelStepDeg(g)) * DEG;
  return {
    dx: g.radius * (Math.sin(theta) - Math.sin(aRad)),
    dy: g.radius * (Math.cos(aRad) - Math.cos(theta)),
  };
}
