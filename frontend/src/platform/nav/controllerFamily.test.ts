// Tests for controller label-family resolution + label lookup.

import { describe, it, expect } from "vitest";
import { resolveFamily, familyLabel } from "./controllerFamily";

describe("resolveFamily", () => {
  it("classifies the operator's Faceoff pad as Nintendo via the SDL table", () => {
    // 0e6f:0184 = PDP Faceoff, SDL k_eControllerType_SwitchInputOnlyController.
    expect(resolveFamily("vidpid:0e6f:0184")).toBe("nintendo");
  });

  it("classifies known Xbox + PlayStation pads from the SDL table", () => {
    // 045e:028e = Xbox 360 wired; 054c:05c4 = DualShock 4 — both canonical SDL.
    expect(resolveFamily("vidpid:045e:028e")).toBe("xbox");
    expect(resolveFamily("vidpid:054c:05c4")).toBe("playstation");
  });

  it("falls back to a name-keyword heuristic for pads not in the table", () => {
    expect(resolveFamily("vidpid:dead:beef", "Some Switch Pro Knockoff")).toBe("nintendo");
    expect(resolveFamily("vidpid:dead:beef", "Generic DualSense Clone")).toBe("playstation");
    expect(resolveFamily("vidpid:dead:beef", "No-name X-Box pad")).toBe("xbox");
  });

  it("returns generic when nothing matches", () => {
    expect(resolveFamily("vidpid:dead:beef")).toBe("generic");
    expect(resolveFamily("vidpid:dead:beef", "Mystery Gamepad")).toBe("generic");
    expect(resolveFamily("name:weird-pad")).toBe("generic");
  });
});

describe("familyLabel", () => {
  it("shows Nintendo labels in their physical positions (the Faceoff fix)", () => {
    // south = Confirm = NavButton "a"; on a Nintendo pad that button reads "B".
    expect(familyLabel("nintendo", "a")).toBe("B"); // south
    expect(familyLabel("nintendo", "b")).toBe("A"); // east
    expect(familyLabel("nintendo", "x")).toBe("Y"); // west
    expect(familyLabel("nintendo", "y")).toBe("X"); // north
  });

  it("shows Xbox + PlayStation label sets", () => {
    expect(familyLabel("xbox", "a")).toBe("A");
    expect(familyLabel("playstation", "a")).toBe("✕");
    expect(familyLabel("playstation", "y")).toBe("△");
    expect(familyLabel("nintendo", "l2")).toBe("ZL");
  });

  it("uppercases non-face buttons it has no family label for", () => {
    expect(familyLabel("nintendo", "start")).toBe("START");
    expect(familyLabel("xbox", "home")).toBe("HOME");
  });
});
