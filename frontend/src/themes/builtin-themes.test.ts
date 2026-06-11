// CI gate (scope-call #7): every bundled theme must satisfy THEME_CONTRACT.md
// §6, and the `bare` fixture must stay the canonical minimal valid theme. If a
// built-in theme's manifest/tokens drift from the contract, this fails the
// build — the whole point of S4 (turn the documented contract into a machine-
// checked one).
//
// This test lives in themes/ (not platform/) deliberately: validating the real
// themes means importing them, and the ESLint boundary forbids platform ↛
// themes. themes/ is the one layer allowed to see both the concrete themes and
// the platform validator, so it's the correct home for the cross-layer gate.

import { describe, it, expect } from "vitest";
import { validateTheme, formatThemeValidation } from "@oa/platform/theme/validate";
import { BUILTIN_THEMES } from "./index";
import { bare } from "./bare";

describe("BUILTIN_THEMES — contract conformance", () => {
  for (const theme of BUILTIN_THEMES) {
    it(`"${theme.manifest.id}" validates clean against THEME_CONTRACT §6`, () => {
      const v = validateTheme(theme);
      // Surface the formatted issues in the failure message so a drift points
      // straight at the offending field.
      expect(v.ok, formatThemeValidation(v) || "unexpected validation failure").toBe(true);
      expect(v.errors).toEqual([]);
    });
  }

  it("the first theme is the default (Retroverse) and is valid", () => {
    const first = BUILTIN_THEMES[0];
    expect(first?.manifest.id).toBe("retroverse");
    expect(validateTheme(first!).ok).toBe(true);
  });

  it("every theme id is unique (the picker keys on id)", () => {
    const ids = BUILTIN_THEMES.map((t) => t.manifest.id);
    expect(new Set(ids).size).toBe(ids.length);
  });
});

describe("bare — the canonical minimal valid fixture", () => {
  it("is registered in BUILTIN_THEMES", () => {
    expect(BUILTIN_THEMES.some((t) => t.manifest.id === "bare")).toBe(true);
  });

  it("validates clean", () => {
    expect(validateTheme(bare).ok).toBe(true);
  });

  it("ships no token overrides (the purest floor)", () => {
    expect(bare.tokens).toBeUndefined();
  });

  it("declares exactly the single honored surface", () => {
    expect(bare.manifest.surfaces).toEqual(["main"]);
  });

  it("requires no engine capabilities (runs anywhere)", () => {
    expect(bare.manifest.required_engine_capabilities).toEqual([]);
  });
});
