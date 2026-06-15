// Unit tests for the per-system UI config resolver + factual touch lookup
// (Theming ARC 2 L2b / D34). uiConfigFor merges the active theme's per-system
// override over BASELINE_UI; systemSupportsTouch is a platform factual fact.

import { describe, it, expect, beforeEach } from "vitest";
import {
  BASELINE_UI,
  uiConfigFor,
  setThemeSystemUiConfigs,
  systemSupportsTouch,
} from "./systemUIConfigs";

describe("uiConfigFor — theme override merged over BASELINE_UI (D34)", () => {
  beforeEach(() => {
    // Reset the active-theme bridge between tests (module-level signal).
    setThemeSystemUiConfigs(undefined);
  });

  it("returns BASELINE_UI when no theme config is bridged", () => {
    expect(uiConfigFor("gb")).toEqual(BASELINE_UI);
    expect(uiConfigFor("nes")).toEqual(BASELINE_UI);
  });

  it("merges a theme's per-system override over the baseline (other fields stay baseline)", () => {
    setThemeSystemUiConfigs({
      gb: { tileShape: "portrait-3:4", interactionStyle: "delayed" },
    });
    const gb = uiConfigFor("gb");
    expect(gb.tileShape).toBe("portrait-3:4");
    expect(gb.interactionStyle).toBe("delayed");
    expect(gb.audioProfile).toBe(BASELINE_UI.audioProfile); // untouched
    expect(gb.layout).toBe(BASELINE_UI.layout);
  });

  it("a system with no override reads BASELINE_UI even when other systems are overridden", () => {
    setThemeSystemUiConfigs({ gb: { tileShape: "square" } });
    expect(uiConfigFor("nes")).toEqual(BASELINE_UI);
  });

  it("clearing the bridge (undefined) returns every system to BASELINE_UI", () => {
    setThemeSystemUiConfigs({ vectrex: { background: "shader" } });
    expect(uiConfigFor("vectrex").background).toBe("shader");
    setThemeSystemUiConfigs(undefined);
    expect(uiConfigFor("vectrex")).toEqual(BASELINE_UI);
  });
});

describe("systemSupportsTouch — factual, theme-independent (D34)", () => {
  it("NDS has touch input", () => {
    expect(systemSupportsTouch("nds")).toBe(true);
  });

  it("non-touch systems do not", () => {
    expect(systemSupportsTouch("gb")).toBe(false);
    expect(systemSupportsTouch("psx")).toBe(false);
  });
});
