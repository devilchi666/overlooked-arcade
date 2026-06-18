// Dogfood test (P.1 S2): the bare-declarative theme is a valid, list-rendering,
// zero-code package backed by the built-in DeclarativeShell. Lives in themes/
// (not platform/) so it may import the theme package without crossing the
// platform ↛ theme boundary.

import { describe, it, expect } from "vitest";
import { bareDeclarative } from "./index";
import DeclarativeShell from "@oa/platform/theme/declarativeShell";
import { validateTheme } from "@oa/platform/theme/validate";
import { resolveLayout } from "@oa/platform/theme/layoutResolver";
import { resolveMotionRef } from "@oa/platform/theme/motion";
import { MOTION_PRESETS } from "@oa/platform/theme/motionPresets";

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

  it("declares an ARC 3 motion transition + motion tokens (still valid)", () => {
    // The manifest motion field (preset selection) the resolver consumes.
    expect(bareDeclarative.manifest.motion?.view_transition?.preset).toBe("fade");
    // The motion-token override (durations/easings) App.tsx scoped-injects.
    expect(bareDeclarative.motionTokens?.fast).toBeTruthy();
    // Declaring motion must not break validation.
    expect(validateTheme(bareDeclarative).ok).toBe(true);
  });

  it("declares the ARC 3 selection + ambient slots, resolvable + correct-kind", () => {
    const motion = bareDeclarative.manifest.motion;
    // The slots the DeclarativeShell's per-card SelectionMotion consumes. A
    // FULL-WIDTH list row uses non-width-changing motion (a `y` rise + a glow), NOT
    // a centred `scale` preset — those fling left-aligned content (MOTION.md #3).
    expect(motion?.selection).toBe("title-rise");
    expect(motion?.ambient).toBe("glow-pulse");
    // Each must resolve to a concrete spec (preset name → §2 spec).
    expect(resolveMotionRef(motion?.selection)).not.toBeNull();
    expect(resolveMotionRef(motion?.ambient)).not.toBeNull();
    // …and be the kind its slot expects (the validator's name+KIND rule, D58.5).
    expect(MOTION_PRESETS["title-rise"]?.kind).toBe("selection");
    expect(MOTION_PRESETS["glow-pulse"]?.kind).toBe("ambient");
  });
});
