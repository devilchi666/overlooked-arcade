// Unit tests for the per-system-UI consumption gate (Theming ARC 2 L1, D33/D34).
// The effective gate folds the USER master toggle AND the active theme's
// per_system_ui opt-in; default (no theme bridged) is OFF for both surfaces.

import { describe, it, expect, beforeEach } from "vitest";
import {
  setPerSystemUiEnabled,
  setThemePerSystemUi,
  consumesPerSystemTiles,
  consumesPerSystemSfx,
} from "./systemUiSound";

describe("per-system-UI consumption gate (D33/D34)", () => {
  beforeEach(() => {
    // Reset to a known baseline: master ON (default), theme opted OUT.
    setPerSystemUiEnabled(true);
    setThemePerSystemUi(undefined);
  });

  it("defaults OFF when the active theme declares nothing (even with master ON)", () => {
    expect(consumesPerSystemTiles()).toBe(false);
    expect(consumesPerSystemSfx()).toBe(false);
  });

  it("master ON + theme opts into both → both consumed", () => {
    setThemePerSystemUi({ tiles: true, sfx: true });
    expect(consumesPerSystemTiles()).toBe(true);
    expect(consumesPerSystemSfx()).toBe(true);
  });

  it("master OFF overrides a theme that opts in → both off", () => {
    setThemePerSystemUi({ tiles: true, sfx: true });
    setPerSystemUiEnabled(false);
    expect(consumesPerSystemTiles()).toBe(false);
    expect(consumesPerSystemSfx()).toBe(false);
  });

  it("surfaces are independent — a theme can take sfx without tiles", () => {
    setThemePerSystemUi({ sfx: true });
    expect(consumesPerSystemTiles()).toBe(false);
    expect(consumesPerSystemSfx()).toBe(true);
  });

  it("setThemePerSystemUi(undefined) coerces to both OFF", () => {
    setThemePerSystemUi({ tiles: true, sfx: true });
    setThemePerSystemUi(undefined);
    expect(consumesPerSystemTiles()).toBe(false);
    expect(consumesPerSystemSfx()).toBe(false);
  });
});
