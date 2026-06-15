// Unit tests for the theme validator (S4). Pure — no theme imports; each case
// constructs a minimal ThemePackage and asserts the §6 rule it should trip.
// The companion themes/builtin-themes.test.ts checks the REAL bundled themes.

import { describe, it, expect } from "vitest";
import { validateTheme, type ThemeIssueCode } from "./validate";
import type { ThemePackage } from "./types";
import type { ThemeManifest } from "./manifest";

// A throwaway entry — the validator never touches it (it checks declarative
// data only), so a no-op stand-in is enough.
const noopEntry = (() => null) as unknown as ThemePackage["entry"];

function manifest(over: Partial<ThemeManifest> = {}): ThemeManifest {
  return {
    id: "demo",
    name: "Demo",
    version: "1.0.0",
    schema_version: 1,
    oa_version: "^0.x",
    entry: "./index.tsx",
    entry_export: "demo",
    default_route: "library",
    routes: ["library"],
    context_slots: ["library"],
    required_engine_capabilities: [],
    reserves_corner: "top-right",
    surfaces: ["main"],
    ...over,
  };
}

function pkg(mOver: Partial<ThemeManifest> = {}, over: Partial<ThemePackage> = {}): ThemePackage {
  return { manifest: manifest(mOver), entry: noopEntry, ...over };
}

const codes = (issues: { code: ThemeIssueCode }[]): ThemeIssueCode[] => issues.map((i) => i.code);

describe("validateTheme — valid", () => {
  it("a complete minimal manifest with no tokens passes clean", () => {
    const v = validateTheme(pkg());
    expect(v.ok).toBe(true);
    expect(v.errors).toEqual([]);
    expect(v.warnings).toEqual([]);
    expect(v.themeId).toBe("demo");
  });

  it("a present token key with an undefined value inherits (no error)", () => {
    // themeTokensToCssVars drops null/undefined → falls through to :root.
    const v = validateTheme(pkg({}, { tokens: { bg: undefined } }));
    expect(v.ok).toBe(true);
  });

  it("a valid token override passes", () => {
    const v = validateTheme(pkg({}, { tokens: { bg: "oklch(20% 0 0)", accent: "#0ff" } }));
    expect(v.ok).toBe(true);
  });

  it("a valid perSystemTokens override passes", () => {
    const v = validateTheme(
      pkg({}, { perSystemTokens: { nes: { accent: "#f00" }, snes: { soft: "#abc", glow: "#0001" } } }),
    );
    expect(v.ok).toBe(true);
    expect(v.errors).toEqual([]);
  });

  it("a known glyph_set passes with no issues", () => {
    const v = validateTheme(pkg({ glyph_set: "playstation" }));
    expect(v.ok).toBe(true);
    expect(v.warnings).toEqual([]);
  });

  it("a well-formed per_system_ui passes clean", () => {
    const v = validateTheme(pkg({ per_system_ui: { tiles: true, sfx: false } }));
    expect(v.ok).toBe(true);
    expect(v.warnings).toEqual([]);
  });
});

describe("validateTheme — errors", () => {
  it("missing id → MISSING_FIELD + themeId falls back to <unknown>", () => {
    const v = validateTheme(pkg({ id: "" }));
    expect(v.ok).toBe(false);
    expect(codes(v.errors)).toContain("MISSING_FIELD");
    expect(v.themeId).toBe("<unknown>");
  });

  it("blank name → MISSING_FIELD", () => {
    expect(codes(validateTheme(pkg({ name: "   " })).errors)).toContain("MISSING_FIELD");
  });

  it("non-number schema_version → MISSING_FIELD", () => {
    const v = validateTheme(pkg({ schema_version: undefined as unknown as number }));
    expect(codes(v.errors)).toContain("MISSING_FIELD");
  });

  it("unsupported (too-new) schema_version → UNSUPPORTED_SCHEMA_VERSION with 'newer' message", () => {
    const v = validateTheme(pkg({ schema_version: 2 }));
    expect(codes(v.errors)).toContain("UNSUPPORTED_SCHEMA_VERSION");
    const issue = v.errors.find((e) => e.code === "UNSUPPORTED_SCHEMA_VERSION");
    expect(issue?.message).toMatch(/newer/i);
  });

  it("unknown (too-old/0) schema_version → UNSUPPORTED_SCHEMA_VERSION without 'newer'", () => {
    const v = validateTheme(pkg({ schema_version: 0 }));
    const issue = v.errors.find((e) => e.code === "UNSUPPORTED_SCHEMA_VERSION");
    expect(issue).toBeDefined();
    expect(issue?.message).not.toMatch(/newer/i);
  });

  it("empty surfaces → EMPTY_SURFACES", () => {
    expect(codes(validateTheme(pkg({ surfaces: [] })).errors)).toContain("EMPTY_SURFACES");
  });

  it("unhonored surface → UNKNOWN_SURFACE", () => {
    const v = validateTheme(pkg({ surfaces: ["marquee"] as unknown as ThemeManifest["surfaces"] }));
    expect(codes(v.errors)).toContain("UNKNOWN_SURFACE");
  });

  it("unadvertised required capability → UNADVERTISED_CAPABILITY", () => {
    const v = validateTheme(pkg({ required_engine_capabilities: ["attract-mode"] }));
    expect(codes(v.errors)).toContain("UNADVERTISED_CAPABILITY");
  });

  it("unknown token key → UNKNOWN_TOKEN_KEY", () => {
    const v = validateTheme(
      pkg({}, { tokens: { bogus: "x" } as unknown as ThemePackage["tokens"] }),
    );
    expect(codes(v.errors)).toContain("UNKNOWN_TOKEN_KEY");
  });

  it("empty token value → EMPTY_TOKEN_VALUE", () => {
    const v = validateTheme(pkg({}, { tokens: { bg: "  " } }));
    expect(codes(v.errors)).toContain("EMPTY_TOKEN_VALUE");
  });

  it("unknown system id in perSystemTokens → UNKNOWN_SYSTEM_ID", () => {
    const v = validateTheme(
      pkg({}, { perSystemTokens: { atari9000: { accent: "#f00" } } as unknown as ThemePackage["perSystemTokens"] }),
    );
    expect(codes(v.errors)).toContain("UNKNOWN_SYSTEM_ID");
  });

  it("unknown palette key in perSystemTokens → UNKNOWN_PALETTE_KEY", () => {
    const v = validateTheme(
      pkg({}, { perSystemTokens: { nes: { bogus: "#f00" } } as unknown as ThemePackage["perSystemTokens"] }),
    );
    expect(codes(v.errors)).toContain("UNKNOWN_PALETTE_KEY");
  });

  it("empty palette value in perSystemTokens → EMPTY_PALETTE_VALUE", () => {
    const v = validateTheme(pkg({}, { perSystemTokens: { nes: { accent: "  " } } }));
    expect(codes(v.errors)).toContain("EMPTY_PALETTE_VALUE");
  });
});

describe("validateTheme — warnings (non-fatal)", () => {
  it("non-directory-safe id → INVALID_ID warning, still ok", () => {
    const v = validateTheme(pkg({ id: "Bad Id" }));
    expect(v.ok).toBe(true);
    expect(codes(v.warnings)).toContain("INVALID_ID");
  });

  it("default_route not in routes → DEFAULT_ROUTE_NOT_IN_ROUTES warning, still ok", () => {
    const v = validateTheme(pkg({ default_route: "nope", routes: ["library"] }));
    expect(v.ok).toBe(true);
    expect(codes(v.warnings)).toContain("DEFAULT_ROUTE_NOT_IN_ROUTES");
  });

  it("unknown glyph_set → UNKNOWN_GLYPH_SET warning, still ok (hints fall back)", () => {
    const v = validateTheme(pkg({ glyph_set: "ps5" }));
    expect(v.ok).toBe(true);
    expect(codes(v.warnings)).toContain("UNKNOWN_GLYPH_SET");
  });

  it("malformed per_system_ui → INVALID_PER_SYSTEM_UI warning, still ok (falls back OFF)", () => {
    const v = validateTheme(
      pkg({ per_system_ui: { tiles: "yes" } as unknown as { tiles?: boolean } }),
    );
    expect(v.ok).toBe(true);
    expect(codes(v.warnings)).toContain("INVALID_PER_SYSTEM_UI");
  });
});

describe("validateTheme — settings_schema (Settings IA Slice 3)", () => {
  type Schema = ThemeManifest["settings_schema"];

  it("a valid toggle + slider + select schema passes clean", () => {
    const v = validateTheme(
      pkg({
        settings_schema: [
          { key: "compact", type: "toggle", label: "Compact", default: false },
          { key: "tile", type: "slider", label: "Tile", default: 220, min: 120, max: 400, step: 20, unit: "px" },
          {
            key: "sort",
            type: "select",
            label: "Sort",
            default: "title",
            options: [
              { value: "title", label: "Name" },
              { value: "year", label: "Year" },
            ],
          },
        ],
      }),
    );
    expect(v.ok).toBe(true);
    expect(v.errors).toEqual([]);
  });

  it("omitting settings_schema is fine (optional)", () => {
    expect(validateTheme(pkg()).ok).toBe(true);
  });

  it("duplicate keys → SETTING_KEY_INVALID", () => {
    const v = validateTheme(
      pkg({
        settings_schema: [
          { key: "x", type: "toggle", label: "A", default: false },
          { key: "x", type: "toggle", label: "B", default: true },
        ],
      }),
    );
    expect(codes(v.errors)).toContain("SETTING_KEY_INVALID");
  });

  it("missing key → SETTING_KEY_INVALID", () => {
    const v = validateTheme(
      pkg({ settings_schema: [{ type: "toggle", label: "A", default: false }] as unknown as Schema }),
    );
    expect(codes(v.errors)).toContain("SETTING_KEY_INVALID");
  });

  it("missing label → SETTING_CONTROL_INVALID", () => {
    const v = validateTheme(
      pkg({ settings_schema: [{ key: "x", type: "toggle", default: false }] as unknown as Schema }),
    );
    expect(codes(v.errors)).toContain("SETTING_CONTROL_INVALID");
  });

  it("slider default out of [min,max] → SETTING_CONTROL_INVALID", () => {
    const v = validateTheme(
      pkg({ settings_schema: [{ key: "t", type: "slider", label: "T", default: 999, min: 0, max: 100, step: 5 }] }),
    );
    expect(codes(v.errors)).toContain("SETTING_CONTROL_INVALID");
  });

  it("slider min >= max → SETTING_CONTROL_INVALID", () => {
    const v = validateTheme(
      pkg({ settings_schema: [{ key: "t", type: "slider", label: "T", default: 5, min: 10, max: 10, step: 1 }] }),
    );
    expect(codes(v.errors)).toContain("SETTING_CONTROL_INVALID");
  });

  it("select default not among options → SETTING_CONTROL_INVALID", () => {
    const v = validateTheme(
      pkg({
        settings_schema: [
          { key: "s", type: "select", label: "S", default: "ghost", options: [{ value: "a", label: "A" }] },
        ],
      }),
    );
    expect(codes(v.errors)).toContain("SETTING_CONTROL_INVALID");
  });

  it("unknown control type → SETTING_CONTROL_INVALID", () => {
    const v = validateTheme(
      pkg({ settings_schema: [{ key: "x", type: "radio", label: "X", default: 1 }] as unknown as Schema }),
    );
    expect(codes(v.errors)).toContain("SETTING_CONTROL_INVALID");
  });
});
