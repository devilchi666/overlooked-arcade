// Unit tests for the declarative-motion resolver (ARC 3 M1). Pure — the DOM
// `ViewTransition` component is operator-smoke-verified (WAAPI isn't faithfully
// implemented in jsdom); these cover the manifest→transition resolution + the
// CSS-time parser + the reduced-motion downgrade.

import { describe, it, expect } from "vitest";
import {
  DEFAULT_TRANSITION_EASING,
  DEFAULT_TRANSITION_MS,
  REDUCED_MOTION_FADE_MS,
  parseDurationMs,
  resolveViewTransition,
  resolveThemeMotionSpec,
} from "./motion";
import { REDUCED_MOTION_SPEC } from "./motionSpec";
import type { ThemeMotion } from "./manifest";

describe("parseDurationMs", () => {
  it("parses ms, s, and bare numbers", () => {
    expect(parseDurationMs("250ms")).toBe(250);
    expect(parseDurationMs("0.4s")).toBe(400);
    expect(parseDurationMs("2s")).toBe(2000);
    expect(parseDurationMs("180")).toBe(180);
    expect(parseDurationMs("  300ms ")).toBe(300);
  });

  it("returns null for absent / unparseable / non-positive values", () => {
    expect(parseDurationMs(undefined)).toBeNull();
    expect(parseDurationMs("")).toBeNull();
    expect(parseDurationMs("fast")).toBeNull();
    expect(parseDurationMs("0ms")).toBeNull();
    expect(parseDurationMs("-100ms")).toBeNull();
  });
});

describe("resolveViewTransition", () => {
  it("no motion → no transition", () => {
    const r = resolveViewTransition(undefined, false);
    expect(r.preset).toBe("none");
    expect(r.durationMs).toBe(0);
  });

  it("an explicit `none` preset → no transition", () => {
    const m: ThemeMotion = { view_transition: { preset: "none" } };
    expect(resolveViewTransition(m, false).preset).toBe("none");
  });

  it("a declared preset resolves with the declared timing", () => {
    const m: ThemeMotion = { view_transition: { preset: "slide", duration: "300ms", easing: "ease-in" } };
    const r = resolveViewTransition(m, false);
    expect(r.preset).toBe("slide");
    expect(r.durationMs).toBe(300);
    expect(r.easing).toBe("ease-in");
  });

  it("a declared preset with no timing falls back to the motion defaults", () => {
    const m: ThemeMotion = { view_transition: { preset: "scale" } };
    const r = resolveViewTransition(m, false);
    expect(r.preset).toBe("scale");
    expect(r.durationMs).toBe(DEFAULT_TRANSITION_MS);
    expect(r.easing).toBe(DEFAULT_TRANSITION_EASING);
  });

  it("an unparseable duration falls back to the default duration", () => {
    const m: ThemeMotion = { view_transition: { preset: "fade", duration: "quick" } };
    expect(resolveViewTransition(m, false).durationMs).toBe(DEFAULT_TRANSITION_MS);
  });

  it("reduced motion downgrades ANY declared transition to a short fade (not instant)", () => {
    const m: ThemeMotion = { view_transition: { preset: "slide", duration: "600ms" } };
    const r = resolveViewTransition(m, true);
    expect(r.preset).toBe("fade");
    expect(r.durationMs).toBe(REDUCED_MOTION_FADE_MS);
    expect(r.durationMs).toBeGreaterThan(0); // a gentle fade, never a hard cut
  });

  it("reduced motion still yields no transition when none was declared", () => {
    expect(resolveViewTransition(undefined, true).preset).toBe("none");
    expect(resolveViewTransition({ view_transition: { preset: "none" } }, true).preset).toBe("none");
  });
});

describe("resolveThemeMotionSpec — manifest → §2 spec (M-mod.1)", () => {
  it("no motion → null", () => {
    expect(resolveThemeMotionSpec(undefined, false)).toBeNull();
    expect(resolveThemeMotionSpec({ view_transition: { preset: "none" } }, false)).toBeNull();
  });

  it("an authored view_transition_spec wins verbatim", () => {
    const spec = { duration: 560, easing: "ease", channels: { opacity: [0, 1] as const, y: [140, 0] as const } };
    const m: ThemeMotion = { view_transition_spec: spec };
    expect(resolveThemeMotionSpec(m, false)).toBe(spec);
  });

  it("the spec wins even when a preset is also declared", () => {
    const spec = { duration: 400, channels: { opacity: [0, 1] as const } };
    const m: ThemeMotion = { view_transition: { preset: "slide" }, view_transition_spec: spec };
    expect(resolveThemeMotionSpec(m, false)).toBe(spec);
  });

  it("falls back to mapping a preset (with its timing) to the basis", () => {
    const m: ThemeMotion = { view_transition: { preset: "slide", duration: "600ms", easing: "linear" } };
    const r = resolveThemeMotionSpec(m, false)!;
    expect(r.duration).toBe(600);
    expect(r.easing).toBe("linear");
    expect(r.channels.y).toEqual([96, 0]); // slide default travel
    expect(r.channels.opacity).toEqual([0, 1]);
  });

  it("reduced motion → the short-fade spec regardless of what was declared", () => {
    const m: ThemeMotion = { view_transition_spec: { duration: 560, channels: { y: [140, 0] } } };
    expect(resolveThemeMotionSpec(m, true)).toBe(REDUCED_MOTION_SPEC);
  });
});
