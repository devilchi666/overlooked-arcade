// Unit tests for the D18 remap helpers — the pure conflict-resolving rebind +
// the raw (no-swap) verb→button lookup the Settings UI edits through.

import { describe, it, expect } from "vitest";
import {
  DEFAULT_BINDINGS,
  rawGamepadButtonForVerb,
  rebindGamepadVerb,
  type NavBindings,
} from "./navBindings";

describe("rawGamepadButtonForVerb", () => {
  it("finds the button bound to a verb in the default map", () => {
    expect(rawGamepadButtonForVerb(DEFAULT_BINDINGS, "Confirm")).toBe("a");
    expect(rawGamepadButtonForVerb(DEFAULT_BINDINGS, "Back")).toBe("b");
    expect(rawGamepadButtonForVerb(DEFAULT_BINDINGS, "NextSection")).toBe("r1");
  });

  it("returns null for an unbound verb", () => {
    expect(rawGamepadButtonForVerb(DEFAULT_BINDINGS, "OpenQuickSettings")).toBeNull();
  });
});

describe("rebindGamepadVerb", () => {
  it("assigns a verb to a fresh button", () => {
    const next = rebindGamepadVerb(DEFAULT_BINDINGS, "OpenQuickSettings", "l2");
    expect(rawGamepadButtonForVerb(next, "OpenQuickSettings")).toBe("l2");
  });

  it("moves a verb to a new button, clearing its old one", () => {
    // Confirm: a → y. The old `a` entry is removed.
    const next = rebindGamepadVerb(DEFAULT_BINDINGS, "Confirm", "y");
    expect(rawGamepadButtonForVerb(next, "Confirm")).toBe("y");
    expect(next.gamepad.a).toBeUndefined();
  });

  it("resolves a conflict: assigning a button steals it from the verb that had it", () => {
    // Bind Back to A (A was Confirm). A becomes Back; Confirm loses its only
    // button → Unbound.
    const next = rebindGamepadVerb(DEFAULT_BINDINGS, "Back", "a");
    expect(next.gamepad.a).toBe("Back");
    expect(rawGamepadButtonForVerb(next, "Confirm")).toBeNull();
    // Back's old button (b) is cleared — Back has exactly one button.
    expect(next.gamepad.b).toBeUndefined();
  });

  it("unbinds a verb when button is null", () => {
    const next = rebindGamepadVerb(DEFAULT_BINDINGS, "Confirm", null);
    expect(rawGamepadButtonForVerb(next, "Confirm")).toBeNull();
  });

  it("never mutates the input bindings", () => {
    const before: NavBindings = {
      gamepad: { ...DEFAULT_BINDINGS.gamepad },
      keyboard: { ...DEFAULT_BINDINGS.keyboard },
    };
    rebindGamepadVerb(DEFAULT_BINDINGS, "Confirm", "x");
    expect(DEFAULT_BINDINGS.gamepad).toEqual(before.gamepad);
  });
});
