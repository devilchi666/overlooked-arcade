// Unit tests for the S5.2 palette substrate — the typed single-source map, the
// derived global baseline CSS, and the scoped per-theme override CSS.

import { describe, it, expect } from "vitest";
import {
  SYSTEM_PALETTES,
  PALETTE_VAR,
  systemPaletteBaselineCss,
  perSystemOverrideCss,
  type SystemPalette,
} from "./systemPalettes";
import { systemThemes, type SystemId } from "./registry";

describe("SYSTEM_PALETTES (typed single source)", () => {
  it("has a palette for every registered system (parity with systemThemes)", () => {
    const paletteIds = Object.keys(SYSTEM_PALETTES).sort();
    const systemIds = Object.keys(systemThemes).sort();
    expect(paletteIds).toEqual(systemIds);
  });

  it("every palette has non-empty accent / soft / glow", () => {
    for (const [id, pal] of Object.entries(SYSTEM_PALETTES)) {
      for (const key of Object.keys(PALETTE_VAR) as (keyof SystemPalette)[]) {
        expect(pal[key], `${id}.${key}`).toMatch(/\S/);
      }
    }
  });

  it("glow is the accent at 0.35 alpha (the baseline invariant)", () => {
    const tg16 = SYSTEM_PALETTES.tg16;
    expect(tg16.accent).toBe("oklch(0.74 0.18 55)");
    expect(tg16.glow).toBe("oklch(0.74 0.18 55 / 0.35)");
    // The derivation holds for every baseline entry.
    for (const pal of Object.values(SYSTEM_PALETTES)) {
      expect(pal.glow).toBe(pal.accent.replace(/\)\s*$/, " / 0.35)"));
    }
  });
});

describe("systemPaletteBaselineCss (global baseline)", () => {
  const css = systemPaletteBaselineCss();

  it("emits one global [data-system] rule per system with all three vars", () => {
    for (const id of Object.keys(SYSTEM_PALETTES) as SystemId[]) {
      const pal = SYSTEM_PALETTES[id];
      expect(css).toContain(`[data-system="${id}"]{`);
      expect(css).toContain(`--color-system-accent:${pal.accent};`);
      expect(css).toContain(`--color-system-accent-soft:${pal.soft};`);
      expect(css).toContain(`--color-system-glow:${pal.glow};`);
    }
  });

  it("the baseline is unscoped (no theme-mount prefix) so engine + theme both read it", () => {
    expect(css).not.toContain(".oa-theme-mount");
  });
});

describe("perSystemOverrideCss (scoped theme override)", () => {
  it("scopes each rule under the mount selector so it beats the baseline only inside the theme", () => {
    const css = perSystemOverrideCss(".oa-theme-mount", {
      nes: { accent: "#f00", glow: "#f003" },
    });
    expect(css).toBe(
      `.oa-theme-mount [data-system="nes"]{--color-system-accent:#f00;--color-system-glow:#f003;}`,
    );
  });

  it("emits only the overridden keys (partial overrides inherit the baseline)", () => {
    const css = perSystemOverrideCss(".oa-theme-mount", { snes: { soft: "#abc" } });
    expect(css).toContain("--color-system-accent-soft:#abc;");
    expect(css).not.toContain("--color-system-accent:");
    expect(css).not.toContain("--color-system-glow:");
  });

  it("returns empty string for no overrides (a system-agnostic theme ships none)", () => {
    expect(perSystemOverrideCss(".oa-theme-mount", {})).toBe("");
    // A system present but with no concrete keys also contributes nothing.
    expect(perSystemOverrideCss(".oa-theme-mount", { nes: {} })).toBe("");
  });
});
