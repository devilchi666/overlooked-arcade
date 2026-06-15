// Unit tests for the per-user per-system layout override store (ARC 2 L3 / D32).
// Each test uses its own theme id to stay isolated from the shared module store.

import { describe, it, expect } from "vitest";
import {
  getLayoutOverride,
  setLayoutOverride,
  clearLayoutOverride,
} from "./layoutOverrides";

describe("layoutOverrides — (theme, system, view) → layout store", () => {
  it("returns undefined for an unset override", () => {
    expect(getLayoutOverride("t-unset", "tg16", "game-browse")).toBeUndefined();
  });

  it("set → get round-trips", () => {
    setLayoutOverride("t-rt", "tg16", "game-browse", "wheel");
    expect(getLayoutOverride("t-rt", "tg16", "game-browse")).toBe("wheel");
  });

  it("clear removes the override (falls back to undefined)", () => {
    setLayoutOverride("t-clr", "lynx", "game-browse", "list");
    clearLayoutOverride("t-clr", "lynx", "game-browse");
    expect(getLayoutOverride("t-clr", "lynx", "game-browse")).toBeUndefined();
  });

  it("isolates by theme id", () => {
    setLayoutOverride("t-a", "nes", "game-browse", "carousel");
    expect(getLayoutOverride("t-b", "nes", "game-browse")).toBeUndefined();
  });

  it("isolates by system and view", () => {
    setLayoutOverride("t-iso", "nes", "game-browse", "list");
    expect(getLayoutOverride("t-iso", "snes", "game-browse")).toBeUndefined();
    expect(getLayoutOverride("t-iso", "nes", "system-browse")).toBeUndefined();
  });
});
