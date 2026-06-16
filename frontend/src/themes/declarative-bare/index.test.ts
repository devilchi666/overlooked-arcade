// Dogfood test (P.1 S2): the bare-declarative theme is a valid, list-rendering,
// zero-code package backed by the built-in DeclarativeShell. Lives in themes/
// (not platform/) so it may import the theme package without crossing the
// platform ↛ theme boundary.

import { describe, it, expect } from "vitest";
import { bareDeclarative } from "./index";
import DeclarativeShell from "@oa/platform/theme/declarativeShell";
import { validateTheme } from "@oa/platform/theme/validate";
import { resolveLayout } from "@oa/platform/theme/layoutResolver";

describe("bare-declarative dogfood", () => {
  it("is a valid, list-rendering, DeclarativeShell-backed package", () => {
    expect(bareDeclarative.entry).toBe(DeclarativeShell);
    expect(bareDeclarative.manifest.id).toBe("bare-declarative");
    // The zero-code proof must itself validate (it ships in BUILTIN_THEMES).
    expect(validateTheme(bareDeclarative).ok).toBe(true);
    // It declares a list (so it matches hand-coded `bare`, not the grid default).
    expect(
      resolveLayout({ themeViews: bareDeclarative.manifest.views, view: "game-browse" }),
    ).toBe("list");
    // It re-states bare's scoped per-system palette demo.
    expect(bareDeclarative.perSystemTokens?.nes?.accent).toBeTruthy();
  });
});
