import { describe, expect, it } from "vitest";
import { nextSliderValue } from "./settingsRowNav";

// The slider "adjust mode" (Confirm on a SettingRow slider; up/right +,
// down/left −) steps via nextSliderValue. These pin the clamp + step-grid
// snap so repeated gamepad steps stay on the grid and respect the bounds.
describe("nextSliderValue", () => {
  it("steps up and down by one step within bounds", () => {
    expect(nextSliderValue(2, 1, 0, 5, 1)).toBe(3);
    expect(nextSliderValue(2, 1, 0, 5, -1)).toBe(1);
  });

  it("clamps at max and min", () => {
    expect(nextSliderValue(5, 1, 0, 5, 1)).toBe(5);
    expect(nextSliderValue(0, 1, 0, 5, -1)).toBe(0);
  });

  it("snaps to the step grid for fractional steps (no float drift)", () => {
    // bloom amount: min 0, step 0.05. Walking up from 0.35 must stay on grid.
    const a = nextSliderValue(0.35, 0.05, 0, 1, 1);
    expect(a).toBeCloseTo(0.4, 10);
    // and the value is exactly grid-aligned, not 0.39999999999999997
    expect(Math.round(a / 0.05)).toBe(8);
  });

  it("never overshoots max when the last step would exceed it", () => {
    // 4.5 + 1 = 5.5 → clamp to 5.
    expect(nextSliderValue(4.5, 1, 0, 5, 1)).toBe(5);
  });

  it("handles unbounded inputs (no min/max declared)", () => {
    expect(nextSliderValue(10, 2, -Infinity, Infinity, 1)).toBe(12);
    expect(nextSliderValue(10, 2, -Infinity, Infinity, -1)).toBe(8);
  });
});
