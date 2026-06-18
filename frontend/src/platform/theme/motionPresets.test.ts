// Unit tests for the named motion-preset registry (ARC 3 Thrust M, audit §3).
// Each preset must build a well-formed spec with the right kind + the right
// channels, and overrides must flow through.

import { describe, it, expect } from "vitest";
import { compileMotionSpec } from "./motionSpec";
import { MOTION_PRESETS, MOTION_PRESET_NAMES, buildPreset, BACK_EASE } from "./motionPresets";

describe("MOTION_PRESETS catalog", () => {
  it("registers the 11 spec presets across the three kinds", () => {
    expect([...MOTION_PRESET_NAMES].sort()).toEqual(
      [
        "art-grow-in",
        "breathe",
        "fade",
        "flip",
        "float",
        "glow-pulse",
        "ken-burns",
        "lift",
        "scale",
        "slide",
        "title-rise",
      ].sort(),
    );
  });

  it("every preset builds a spec that compiles to a non-null WAAPI payload", () => {
    for (const name of MOTION_PRESET_NAMES) {
      const spec = MOTION_PRESETS[name].build();
      expect(compileMotionSpec(spec), `${name} should compile`).not.toBeNull();
    }
  });

  it("transition/selection are one-shot; ambient loops alternate-infinitely", () => {
    for (const name of MOTION_PRESET_NAMES) {
      const p = MOTION_PRESETS[name];
      const spec = p.build();
      if (p.kind === "ambient") {
        expect(spec.repeat, name).toBe("infinite");
        expect(spec.direction, name).toBe("alternate");
      } else {
        expect(spec.repeat ?? 1, name).toBe(1);
      }
    }
  });
});

describe("preset shapes", () => {
  it("flip uses the 3D rotateY channel and compiles a perspective transform", () => {
    expect(MOTION_PRESETS.flip.build().channels.rotateY).toEqual([-90, 0]);
    const c = compileMotionSpec(MOTION_PRESETS.flip.build())!;
    expect(c.keyframes[0].transform).toContain("perspective(800px)");
    expect(c.keyframes[0].transform).toContain("rotateY(-90deg)");
  });

  it("glow-pulse animates box-shadow with the given colour", () => {
    const spec = MOTION_PRESETS["glow-pulse"].build({ color: "#ff0066" });
    expect(spec.channels.boxShadow?.[0]).toContain("#ff0066");
    const c = compileMotionSpec(spec)!;
    expect(c.keyframes[0].boxShadow).toContain("#ff0066");
    expect(c.keyframes[1].boxShadow).toContain("#ff0066");
  });

  it("selection presets use the back-ease overshoot by default", () => {
    expect(MOTION_PRESETS.lift.build().easing).toBe(BACK_EASE);
    expect(MOTION_PRESETS["art-grow-in"].build().easing).toBe(BACK_EASE);
  });

  it("overrides flow through (distance / peak / duration / easing)", () => {
    expect(MOTION_PRESETS.slide.build({ distance: 200 }).channels.y).toEqual([200, 0]);
    expect(MOTION_PRESETS.lift.build({ peak: 1.2 }).channels.scale).toEqual([1, 1.2]);
    expect(MOTION_PRESETS.breathe.build({ duration: 4000 }).duration).toBe(4000);
    expect(MOTION_PRESETS.fade.build({ easing: "linear" }).easing).toBe("linear");
  });
});

describe("buildPreset", () => {
  it("builds a known preset and returns null for an unknown name", () => {
    expect(buildPreset("fade")).not.toBeNull();
    expect(buildPreset("nope")).toBeNull();
  });
});
