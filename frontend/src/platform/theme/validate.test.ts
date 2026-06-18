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

describe("validateTheme — views (ARC 2 L2 / D32)", () => {
  type Views = ThemeManifest["views"];

  it("a valid views map (default layout + per_system override) passes clean", () => {
    const v = validateTheme(
      pkg({
        views: {
          "game-browse": { layout: "grid", per_system: { tg16: "wheel", lynx: "grid" } },
          "system-browse": { layout: "list" },
        },
      }),
    );
    expect(v.ok).toBe(true);
    expect(v.errors).toEqual([]);
  });

  it("omitting views is fine (optional)", () => {
    expect(validateTheme(pkg()).ok).toBe(true);
  });

  it("an unknown view type → UNKNOWN_VIEW_TYPE", () => {
    const v = validateTheme(pkg({ views: { "box-art-wall": { layout: "grid" } } as unknown as Views }));
    expect(v.ok).toBe(false);
    expect(codes(v.errors)).toContain("UNKNOWN_VIEW_TYPE");
  });

  it("an unknown layout primitive → INVALID_VIEW_LAYOUT", () => {
    const v = validateTheme(pkg({ views: { "game-browse": { layout: "mosaic" } } as unknown as Views }));
    expect(codes(v.errors)).toContain("INVALID_VIEW_LAYOUT");
  });

  it("a per_system-only entry (no view-wide layout) is valid (L3b — layout optional)", () => {
    const v = validateTheme(pkg({ views: { "game-browse": { per_system: { nes: "list" } } } }));
    expect(v.ok).toBe(true);
    expect(codes(v.errors)).not.toContain("INVALID_VIEW_LAYOUT");
  });

  it("a per_system override on an unknown system id → UNKNOWN_SYSTEM_ID", () => {
    const v = validateTheme(
      pkg({ views: { "game-browse": { layout: "grid", per_system: { dreamcast64: "wheel" } } } as unknown as Views }),
    );
    expect(codes(v.errors)).toContain("UNKNOWN_SYSTEM_ID");
  });

  it("a per_system override with a bad layout → INVALID_VIEW_LAYOUT", () => {
    const v = validateTheme(
      pkg({ views: { "game-browse": { layout: "grid", per_system: { tg16: "spiral" } } } as unknown as Views }),
    );
    expect(codes(v.errors)).toContain("INVALID_VIEW_LAYOUT");
  });

  it("views as an array → INVALID_VIEWS", () => {
    const v = validateTheme(pkg({ views: [] as unknown as Views }));
    expect(codes(v.errors)).toContain("INVALID_VIEWS");
  });
});

describe("validateTheme — motion (ARC 3 M1 / D52)", () => {
  type Motion = ThemeManifest["motion"];

  it("a valid view_transition (preset + timing) passes clean", () => {
    const v = validateTheme(
      pkg({ motion: { view_transition: { preset: "slide", duration: "300ms", easing: "ease-out" } } }),
    );
    expect(v.ok).toBe(true);
    expect(v.errors).toEqual([]);
  });

  it("a preset-only view_transition passes (timing optional)", () => {
    expect(validateTheme(pkg({ motion: { view_transition: { preset: "fade" } } })).ok).toBe(true);
  });

  it("omitting motion is fine (optional)", () => {
    expect(validateTheme(pkg()).ok).toBe(true);
  });

  it("an unknown preset → UNKNOWN_TRANSITION_PRESET", () => {
    const v = validateTheme(
      pkg({ motion: { view_transition: { preset: "zoom" } } as unknown as Motion }),
    );
    expect(v.ok).toBe(false);
    expect(codes(v.errors)).toContain("UNKNOWN_TRANSITION_PRESET");
  });

  it("a blank timing string → INVALID_MOTION", () => {
    const v = validateTheme(pkg({ motion: { view_transition: { preset: "fade", duration: "  " } } }));
    expect(codes(v.errors)).toContain("INVALID_MOTION");
  });

  it("motion as an array → INVALID_MOTION", () => {
    const v = validateTheme(pkg({ motion: [] as unknown as Motion }));
    expect(codes(v.errors)).toContain("INVALID_MOTION");
  });

  it("a valid motionTokens override passes", () => {
    const v = validateTheme(pkg({}, { motionTokens: { fast: "120ms", easeOut: "ease-in" } }));
    expect(v.ok).toBe(true);
    expect(v.errors).toEqual([]);
  });

  it("an unknown motion token key → UNKNOWN_MOTION_TOKEN_KEY", () => {
    const v = validateTheme(
      pkg({}, { motionTokens: { turbo: "1s" } as unknown as ThemePackage["motionTokens"] }),
    );
    expect(codes(v.errors)).toContain("UNKNOWN_MOTION_TOKEN_KEY");
  });

  it("an empty motion token value → EMPTY_MOTION_TOKEN_VALUE", () => {
    const v = validateTheme(pkg({}, { motionTokens: { fast: "  " } }));
    expect(codes(v.errors)).toContain("EMPTY_MOTION_TOKEN_VALUE");
  });
});

describe("validateTheme — perSystemUiConfigs (ARC 2 L2b / D34)", () => {
  it("a well-formed per-system override map passes clean", () => {
    const v = validateTheme(
      pkg({}, { perSystemUiConfigs: { gb: { tileShape: "portrait-3:4" }, nes: { audioProfile: "console" } } }),
    );
    expect(v.ok).toBe(true);
    expect(v.errors).toEqual([]);
  });

  it("an unknown system id key → UNKNOWN_SYSTEM_ID", () => {
    const v = validateTheme(
      pkg({}, { perSystemUiConfigs: { dreamcast64: { tileShape: "square" } } as unknown as Record<string, never> }),
    );
    expect(v.ok).toBe(false);
    expect(codes(v.errors)).toContain("UNKNOWN_SYSTEM_ID");
  });

  it("omitting perSystemUiConfigs is fine (optional)", () => {
    expect(validateTheme(pkg()).ok).toBe(true);
  });
});

describe("validateTheme — motion.view_transition_spec (M-mod.1 §2 basis)", () => {
  it("a well-formed spec passes", () => {
    const v = validateTheme(
      pkg({ motion: { view_transition_spec: { duration: 560, easing: "ease", channels: { opacity: [0, 1], y: [140, 0] } } } }),
    );
    expect(v.ok).toBe(true);
  });

  it("a non-positive / non-number duration is an error", () => {
    expect(
      codes(validateTheme(pkg({ motion: { view_transition_spec: { duration: 0, channels: { opacity: [0, 1] } } } })).errors),
    ).toContain("INVALID_MOTION");
    expect(
      codes(
        validateTheme(
          pkg({ motion: { view_transition_spec: { duration: "x" as unknown as number, channels: { opacity: [0, 1] } } } }),
        ).errors,
      ),
    ).toContain("INVALID_MOTION");
  });

  it("a channel that isn't a [from,to] number pair is an error", () => {
    const v = validateTheme(
      pkg({ motion: { view_transition_spec: { duration: 300, channels: { y: [140] as unknown as [number, number] } } } }),
    );
    expect(v.ok).toBe(false);
    expect(codes(v.errors)).toContain("INVALID_MOTION");
  });

  it("missing channels is an error", () => {
    const motion = { view_transition_spec: { duration: 300 } } as unknown as ThemeManifest["motion"];
    const v = validateTheme(pkg({ motion }));
    expect(v.ok).toBe(false);
    expect(codes(v.errors)).toContain("INVALID_MOTION");
  });

  it("a blank easing string is an error", () => {
    const v = validateTheme(
      pkg({ motion: { view_transition_spec: { duration: 300, easing: "  ", channels: { opacity: [0, 1] } } } }),
    );
    expect(codes(v.errors)).toContain("INVALID_MOTION");
  });
});

describe("validateTheme — motion ref slots (transition/selection/ambient)", () => {
  it("known presets of the right kind pass", () => {
    expect(validateTheme(pkg({ motion: { transition: "slide", selection: "lift", ambient: "breathe" } })).ok).toBe(true);
  });

  it("an unknown preset name is an error", () => {
    const v = validateTheme(pkg({ motion: { transition: "wobblefade" } }));
    expect(v.ok).toBe(false);
    expect(codes(v.errors)).toContain("INVALID_MOTION");
  });

  it("a preset of the wrong KIND for the slot is an error", () => {
    // `breathe` is an ambient preset; it can't fill the transition slot.
    const v = validateTheme(pkg({ motion: { transition: "breathe" } }));
    expect(v.ok).toBe(false);
    expect(codes(v.errors)).toContain("INVALID_MOTION");
  });

  it("an inline spec in a slot is validated as a spec", () => {
    const ok = validateTheme(pkg({ motion: { selection: { duration: 300, channels: { scale: [1, 1.1] } } } }));
    expect(ok.ok).toBe(true);
    const bad = validateTheme(pkg({ motion: { selection: { duration: 0, channels: { scale: [1, 1.1] } } } }));
    expect(codes(bad.errors)).toContain("INVALID_MOTION");
  });
});

describe("validateTheme — detail composition (the declarative element model)", () => {
  it("a composition of known kinds + motion is valid", () => {
    const v = validateTheme(
      pkg({
        detail: {
          elements: [
            { kind: "title", motion: "title-rise" },
            { kind: "system" },
            { kind: "year" },
            { kind: "logo", motion: "art-grow-in" },
          ],
        },
      }),
    );
    expect(v.ok, JSON.stringify(v.errors)).toBe(true);
  });

  it("reserved canvas fields (position/size/ambient) are accepted + inert", () => {
    const v = validateTheme(
      pkg({
        detail: {
          elements: [
            { kind: "title", position: { anchor: "top-left", x: 40, y: 24 }, size: { w: 600 }, ambient: "float" },
          ],
        },
      }),
    );
    expect(v.ok, JSON.stringify(v.errors)).toBe(true);
  });

  it("an unknown element kind is a non-fatal WARNING (renders nothing)", () => {
    const v = validateTheme(
      pkg({ detail: { elements: [{ kind: "marquee" as unknown as "title" }] } }),
    );
    expect(v.ok).toBe(true);
    expect(codes(v.warnings)).toContain("UNKNOWN_ELEMENT_KIND");
  });

  it("an unknown element motion preset is a non-fatal WARNING", () => {
    const v = validateTheme(pkg({ detail: { elements: [{ kind: "title", motion: "wobble" }] } }));
    expect(v.ok).toBe(true);
    expect(codes(v.warnings)).toContain("UNKNOWN_ELEMENT_MOTION");
  });

  it("a structurally malformed composition is an ERROR", () => {
    const notObj = validateTheme(pkg({ detail: "nope" as unknown as ThemeManifest["detail"] }));
    expect(codes(notObj.errors)).toContain("INVALID_DETAIL");

    const noArray = validateTheme(pkg({ detail: { elements: "x" } as unknown as ThemeManifest["detail"] }));
    expect(codes(noArray.errors)).toContain("INVALID_DETAIL");

    const noKind = validateTheme(pkg({ detail: { elements: [{} as unknown as { kind: "title" }] } }));
    expect(codes(noKind.errors)).toContain("INVALID_DETAIL");
  });
});
