// Pure unit coverage for the S5.5 primitive additions. The list/grid/carousel/
// custom primitives are component-behavioral (focus + DOM) and are verified by
// the operator playtest + typecheck/build, matching the existing pure-test
// approach (no Solid render harness is set up). What IS pure-testable lives here:
// the reserved WheelNav stub + the nav-sound dispatcher's selector path.

import { describe, it, expect } from "vitest";
import { WheelNav } from "./WheelNav";
import { navSoundDispatcher } from "@oa/platform/themes/systemUiSound";

describe("WheelNav (reserved contract stub)", () => {
  it("renders nothing — the radial layout is deferred", () => {
    expect(WheelNav<number>({ id: "w", items: [], radius: 120, children: () => null })).toBeNull();
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
