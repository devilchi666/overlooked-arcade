// Unit tests for the declarative motion BASIS compiler (ARC 3 M-mod.1, audit §2).
// The pure half: channel→WAAPI keyframe compilation + the preset-as-defaults
// mapping. The DOM player (SpecTransition) is exercised in the lab, not here.

import { describe, it, expect } from "vitest";
import {
  compileMotionSpec,
  presetToSpec,
  DEFAULT_SPEC_EASING,
  type MotionSpec,
} from "./motionSpec";

describe("compileMotionSpec — channels → WAAPI keyframes", () => {
  it("composes separate transform channels into one transform string per frame", () => {
    const spec: MotionSpec = {
      duration: 400,
      channels: { y: [140, 0], scale: [0.9, 1] },
    };
    const c = compileMotionSpec(spec)!;
    expect(c.keyframes).toHaveLength(2);
    expect(c.keyframes[0].transform).toBe("translateY(140px) scale(0.9)");
    expect(c.keyframes[1].transform).toBe("translateY(0px) scale(1)");
  });

  it("emits opacity as its own property, not in the transform", () => {
    const c = compileMotionSpec({ duration: 300, channels: { opacity: [0, 1], x: [20, 0] } })!;
    expect(c.keyframes[0].opacity).toBe(0);
    expect(c.keyframes[1].opacity).toBe(1);
    expect(c.keyframes[0].transform).toBe("translateX(20px)");
  });

  it("omits the transform key entirely when no transform channels are set", () => {
    const c = compileMotionSpec({ duration: 300, channels: { opacity: [0, 1] } })!;
    expect("transform" in c.keyframes[0]).toBe(false);
    expect(c.keyframes[0].opacity).toBe(0);
  });

  it("carries timing into options with fill:both and the default easing", () => {
    const c = compileMotionSpec({ duration: 560, delay: 40, channels: { opacity: [0, 1] } })!;
    expect(c.options.duration).toBe(560);
    expect(c.options.delay).toBe(40);
    expect(c.options.fill).toBe("both");
    expect(c.options.easing).toBe(DEFAULT_SPEC_EASING);
  });

  it("honors an explicit easing", () => {
    const c = compileMotionSpec({ duration: 300, easing: "linear", channels: { opacity: [0, 1] } })!;
    expect(c.options.easing).toBe("linear");
  });

  it("defaults to a single normal-direction iteration", () => {
    const c = compileMotionSpec({ duration: 300, channels: { opacity: [0, 1] } })!;
    expect(c.options.iterations).toBe(1);
    expect(c.options.direction).toBe("normal");
  });

  it("compiles an ambient loop: repeat infinite + alternate (a breathe/float)", () => {
    const c = compileMotionSpec({
      duration: 2600,
      repeat: "infinite",
      direction: "alternate",
      channels: { scale: [1, 1.04] },
    })!;
    expect(c.options.iterations).toBe(Infinity);
    expect(c.options.direction).toBe("alternate");
    expect(c.keyframes[0].transform).toBe("scale(1)");
    expect(c.keyframes[1].transform).toBe("scale(1.04)");
  });

  it("honors a finite repeat count", () => {
    expect(compileMotionSpec({ duration: 300, repeat: 3, channels: { opacity: [0, 1] } })!.options.iterations).toBe(3);
  });

  it("returns null for a no-op (no channels, or non-positive duration)", () => {
    expect(compileMotionSpec({ duration: 300, channels: {} })).toBeNull();
    expect(compileMotionSpec({ duration: 0, channels: { opacity: [0, 1] } })).toBeNull();
  });
});

describe("presetToSpec — a preset is named defaults over the basis", () => {
  it("none → null", () => {
    expect(presetToSpec("none")).toBeNull();
  });
  it("fade → opacity only", () => {
    expect(presetToSpec("fade")!.channels).toEqual({ opacity: [0, 1] });
  });
  it("slide → opacity + y travel (default 96px)", () => {
    expect(presetToSpec("slide")!.channels).toEqual({ opacity: [0, 1], y: [96, 0] });
  });
  it("scale → opacity + scale", () => {
    expect(presetToSpec("scale")!.channels).toEqual({ opacity: [0, 1], scale: [0.88, 1] });
  });
  it("distance / duration / easing overrides flow through", () => {
    const s = presetToSpec("slide", { distance: 200, duration: 600, easing: "linear" })!;
    expect(s.channels.y).toEqual([200, 0]);
    expect(s.duration).toBe(600);
    expect(s.easing).toBe("linear");
  });
  it("compiles end-to-end (slide override → a real keyframe set)", () => {
    const c = compileMotionSpec(presetToSpec("slide", { distance: 140, duration: 560 })!)!;
    expect(c.keyframes[0].transform).toBe("translateY(140px)");
    expect(c.options.duration).toBe(560);
  });
});
