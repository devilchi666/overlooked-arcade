// Unit tests for the declarative-motion spring converter (ARC 3 M-mod.1). The
// load-bearing assertion is the F10 BACK-SOLVE: the bench's operator-tuned
// physics pair (stiffness 190 / damping 24) must map to the shipped author
// default `BENCH_SELECTION_SPRING`, so the declarative default reproduces the
// approved feel rather than a fresh guess.

import { describe, it, expect } from "vitest";
import {
  toPhysicalSpring,
  physicalToSpring,
  dampingRatioFromBounce,
  bounceFromDampingRatio,
  BENCH_SELECTION_SPRING,
  type PhysicalSpring,
  type SpringSpec,
} from "./spring";

describe("damping ratio ↔ bounce", () => {
  it("bounce 0 = critically damped (ζ = 1)", () => {
    expect(dampingRatioFromBounce(0)).toBeCloseTo(1, 10);
  });
  it("positive bounce underdamps (ζ < 1)", () => {
    expect(dampingRatioFromBounce(0.5)).toBeCloseTo(0.5, 10);
  });
  it("negative bounce overdamps (ζ > 1)", () => {
    expect(dampingRatioFromBounce(-0.5)).toBeCloseTo(2, 10); // 1/(1-0.5)
  });
  it("clamps so 1 + bounce can't hit zero", () => {
    expect(Number.isFinite(dampingRatioFromBounce(-1))).toBe(true);
    expect(Number.isFinite(dampingRatioFromBounce(-5))).toBe(true);
  });
  it("is its own inverse across both branches", () => {
    for (const z of [0.4, 0.87, 1, 1.5, 3]) {
      expect(dampingRatioFromBounce(bounceFromDampingRatio(z))).toBeCloseTo(z, 8);
    }
  });
});

describe("toPhysicalSpring (Apple/SwiftUI closed form, mass 1)", () => {
  it("bounce 0 yields exact critical damping: c = 2√k", () => {
    const p = toPhysicalSpring({ bounce: 0, duration: 500 });
    expect(p.damping).toBeCloseTo(2 * Math.sqrt(p.stiffness), 8);
  });
  it("stiffness scales as the inverse square of duration", () => {
    const a = toPhysicalSpring({ bounce: 0.2, duration: 250 });
    const b = toPhysicalSpring({ bounce: 0.2, duration: 500 });
    // doubling duration → quarter the stiffness
    expect(a.stiffness / b.stiffness).toBeCloseTo(4, 6);
  });
  it("round-trips through physicalToSpring", () => {
    const specs: SpringSpec[] = [
      { bounce: 0, duration: 300 },
      { bounce: 0.3, duration: 450 },
      { bounce: 0.6, duration: 600 },
      { bounce: -0.4, duration: 350 }, // overdamped branch
    ];
    for (const s of specs) {
      const back = physicalToSpring(toPhysicalSpring(s));
      expect(back.bounce).toBeCloseTo(s.bounce, 6);
      expect(back.duration).toBeCloseTo(s.duration, 4);
    }
  });
});

describe("F10 bench back-solve (the feel guarantee)", () => {
  // SelectionChoreography.tsx createSpring(stiffness: 190, damping: 24), mass 1.
  const BENCH_PHYSICS: PhysicalSpring = { stiffness: 190, damping: 24, mass: 1 };

  it("the bench physics expresses as ≈ {bounce 0.13, duration 456ms}", () => {
    const spec = physicalToSpring(BENCH_PHYSICS);
    expect(spec.bounce).toBeCloseTo(0.129, 2);
    expect(spec.duration).toBeCloseTo(456, 0);
  });

  it("the shipped default reproduces the bench physics within tolerance", () => {
    const p = toPhysicalSpring(BENCH_SELECTION_SPRING);
    expect(p.stiffness).toBeCloseTo(190, 0); // ±0.5
    expect(p.damping).toBeCloseTo(24, 1); // ±0.05
    expect(p.mass).toBe(1);
  });
});
