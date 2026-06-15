// Unit tests for the pure layout-resolution cascade (Theming ARC 2 L3 / D32):
// user override → theme per_system → theme view default → engine default.

import { describe, it, expect } from "vitest";
import { resolveLayout, ENGINE_DEFAULT_LAYOUTS } from "./layoutResolver";
import type { ThemeViews } from "./manifest";

describe("resolveLayout — the D32 cascade", () => {
  it("falls to the engine default when nothing is declared", () => {
    expect(resolveLayout({ view: "game-browse" })).toBe(
      ENGINE_DEFAULT_LAYOUTS["game-browse"],
    );
    expect(resolveLayout({ view: "game-details" })).toBe("custom");
  });

  it("uses the theme's view default when declared (no per_system, no override)", () => {
    const themeViews: ThemeViews = { "game-browse": { layout: "list" } };
    expect(resolveLayout({ themeViews, view: "game-browse" })).toBe("list");
  });

  it("a theme's per_system override beats its view default (when that system is in context)", () => {
    const themeViews: ThemeViews = {
      "game-browse": { layout: "grid", per_system: { tg16: "wheel" } },
    };
    expect(resolveLayout({ themeViews, view: "game-browse", systemId: "tg16" })).toBe("wheel");
  });

  it("per_system only applies to the matching system — others get the view default", () => {
    const themeViews: ThemeViews = {
      "game-browse": { layout: "grid", per_system: { tg16: "wheel" } },
    };
    expect(resolveLayout({ themeViews, view: "game-browse", systemId: "nes" })).toBe("grid");
  });

  it("per_system is ignored when no system is in context (view-level resolution)", () => {
    const themeViews: ThemeViews = {
      "game-browse": { layout: "list", per_system: { tg16: "wheel" } },
    };
    expect(resolveLayout({ themeViews, view: "game-browse", systemId: null })).toBe("list");
  });

  it("a user override beats everything (theme per_system + view default)", () => {
    const themeViews: ThemeViews = {
      "game-browse": { layout: "grid", per_system: { tg16: "wheel" } },
    };
    expect(
      resolveLayout({ override: "carousel", themeViews, view: "game-browse", systemId: "tg16" }),
    ).toBe("carousel");
  });
});
