// Unit tests for the disk-theme bridge (P.1 S2) — the descriptor → ThemePackage
// mapper + the manifest → layout-primitive resolution the DeclarativeShell relies
// on. Covers the S2 acceptance "vitest covers the manifest→primitive mapping".

import { describe, it, expect } from "vitest";
import { diskThemeToPackage, type DiskThemeDescriptor } from "./diskTheme";
import DeclarativeShell from "./declarativeShell";
import { validateTheme } from "./validate";
import { resolveLayout } from "./layoutResolver";

/// A minimal valid on-disk descriptor (no entry/entry_export — those are
/// implicit for a declarative theme, D45).
function descriptor(overrides: Partial<DiskThemeDescriptor> = {}): DiskThemeDescriptor {
  return {
    manifest: {
      id: "demo",
      name: "Demo",
      version: "1.0.0",
      schema_version: 1,
      oa_version: "^0.x",
      default_route: "library",
      routes: ["library"],
      context_slots: ["library"],
      required_engine_capabilities: [],
      reserves_corner: "top-right",
      surfaces: ["main"],
      ...overrides.manifest,
    },
    tokens: overrides.tokens,
    perSystemTokens: overrides.perSystemTokens,
    basePath: overrides.basePath ?? "",
  };
}

describe("diskThemeToPackage", () => {
  it("injects the DeclarativeShell entry + synthetic entry/entry_export", () => {
    const pkg = diskThemeToPackage(descriptor());
    expect(pkg.entry).toBe(DeclarativeShell);
    // Synthetic strings exist only to satisfy the shared contract; non-empty.
    expect(pkg.manifest.entry).toBeTruthy();
    expect(pkg.manifest.entry_export).toBeTruthy();
  });

  it("carries the manifest, tokens, and perSystemTokens through unchanged", () => {
    const pkg = diskThemeToPackage(
      descriptor({
        tokens: { bg: "oklch(0.1 0 0)", tileRadius: "12px" },
        perSystemTokens: { nes: { accent: "oklch(0.72 0.16 195)" } },
      }),
    );
    expect(pkg.manifest.id).toBe("demo");
    expect(pkg.manifest.surfaces).toEqual(["main"]);
    expect(pkg.tokens).toEqual({ bg: "oklch(0.1 0 0)", tileRadius: "12px" });
    expect(pkg.perSystemTokens).toEqual({ nes: { accent: "oklch(0.72 0.16 195)" } });
  });

  it("produces a package that passes validateTheme (disk themes validate like builtins)", () => {
    const v = validateTheme(diskThemeToPackage(descriptor()));
    expect(v.ok).toBe(true);
    expect(v.errors).toEqual([]);
  });

  it("a malformed descriptor is REJECTED by validateTheme, same as a builtin", () => {
    // Unknown token key is a disqualifying ERROR — proves the mapper doesn't
    // bypass validation (S3 will exclude such a disk theme).
    const v = validateTheme(
      diskThemeToPackage(descriptor({ tokens: { notAToken: "x" } as never })),
    );
    expect(v.ok).toBe(false);
    expect(v.errors.some((e) => e.code === "UNKNOWN_TOKEN_KEY")).toBe(true);
  });
});

describe("manifest → layout primitive (what DeclarativeShell resolves)", () => {
  it("honors a declared view-level layout", () => {
    const pkg = diskThemeToPackage(
      descriptor({ manifest: { views: { "game-browse": { layout: "list" } } } as never }),
    );
    expect(
      resolveLayout({ themeViews: pkg.manifest.views, view: "game-browse" }),
    ).toBe("list");
  });

  it("falls back to the engine default (grid) when no view layout is declared", () => {
    const pkg = diskThemeToPackage(descriptor());
    expect(
      resolveLayout({ themeViews: pkg.manifest.views, view: "game-browse" }),
    ).toBe("grid");
  });
});
