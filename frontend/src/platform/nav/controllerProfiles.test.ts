// Unit tests for Phase-2 controller-profile resolution (the non-standard-pad
// normalization). Covers the standard-mapping bypass, device-key matching,
// canonical→NavButton remap, DPad/HAT, and forward-compat skipping.

import { describe, it, expect } from "vitest";
import { resolveLayout, buildLayout } from "./controllerProfiles";

describe("resolveLayout", () => {
  it("returns null for a standard-mapping pad (browser already canonical)", () => {
    // Even a pad WITH a profile is left to the standard path when the browser
    // reports a standard mapping — the profile is a non-standard-pad fix.
    expect(resolveLayout("vidpid:0e6f:0184", "standard")).toBeNull();
  });

  it("returns null when no profile matches the device-key", () => {
    expect(resolveLayout("vidpid:dead:beef", "")).toBeNull();
    expect(resolveLayout("name:some-random-pad", "")).toBeNull();
  });

  it("resolves the bundled Faceoff (PDP 0e6f:0184) profile for a non-standard mapping", () => {
    const layout = resolveLayout("vidpid:0e6f:0184", "");
    expect(layout).not.toBeNull();
    // DPad reports as a HAT on axis 9 (idle 3.286) — must survive resolution.
    expect(layout?.hatAxis).toBe(9);
    // SDL-derived mapping: the bottom (south) button is raw index 1 → "a"
    // (Confirm). This is the crux of the bug fix — index 1 was previously
    // mis-read as "b"/Back by the blind standard-layout table.
    expect(layout?.buttons[1]).toBe("a");
    expect(layout?.buttons[2]).toBe("b"); // east
    expect(layout?.buttons[0]).toBe("x"); // west
    expect(layout?.buttons[3]).toBe("y"); // north
  });
});

describe("buildLayout", () => {
  it("inverts canonical→index into index→NavButton by position", () => {
    const layout = buildLayout({
      deviceKey: "test",
      buttons: { south: 0, east: 1, west: 2, north: 3, start: 9, guide: 12 },
    });
    expect(layout.buttons).toEqual({
      0: "a", // south = bottom = Confirm-default
      1: "b", // east
      2: "x", // west
      3: "y", // north
      9: "start",
      12: "home", // guide → home
    });
  });

  it("maps dpad directions to indices and passes hatAxis through", () => {
    const layout = buildLayout({
      deviceKey: "test",
      dpad: { up: 12, down: 13, left: 14, right: 15 },
      hatAxis: 9,
    });
    expect(layout.dpad).toEqual({ 12: "up", 13: "down", 14: "left", 15: "right" });
    expect(layout.hatAxis).toBe(9);
  });

  it("defaults hatAxis to null and skips unknown canonical names", () => {
    const layout = buildLayout({
      deviceKey: "test",
      // "paddle1" is not a known canonical name → silently skipped at runtime
      // (forward-compat: a future DB entry won't break an older build).
      buttons: { south: 0, paddle1: 7 },
    });
    expect(layout.buttons).toEqual({ 0: "a" }); // paddle1 silently skipped
    expect(layout.hatAxis).toBeNull();
  });
});
