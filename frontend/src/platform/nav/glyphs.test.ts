// Unit tests for the S5.3 glyph-set seam — the built-in sets, the registry, the
// active-set indirection, and verb→button→glyph resolution per set.

import { describe, it, expect } from "vitest";
import {
  DEFAULT_GLYPH_SET,
  PLAYSTATION_GLYPH_SET,
  GLYPH_SETS,
  DEFAULT_GLYPH_SET_ID,
  activeGlyphSet,
  setActiveGlyphSetId,
  verbGlyph,
  type GlyphSet,
} from "./glyphs";
import { DEFAULT_BINDINGS } from "./navBindings";
import type { NavButton } from "./types";

const BUTTONS: NavButton[] = [
  "a", "b", "x", "y", "l1", "r1", "l2", "r2", "start", "select", "l3", "r3", "home",
];

function isComplete(set: GlyphSet) {
  for (const b of BUTTONS) expect(set[b], b).toMatch(/\S/);
  expect(set.dpad).toMatch(/\S/);
  expect(set.stick).toMatch(/\S/);
}

describe("built-in glyph sets", () => {
  it("xbox + playstation each cover every button + dpad/stick", () => {
    isComplete(DEFAULT_GLYPH_SET);
    isComplete(PLAYSTATION_GLYPH_SET);
  });

  it("playstation face buttons use the PS symbols", () => {
    expect(PLAYSTATION_GLYPH_SET.a).toBe("✕");
    expect(PLAYSTATION_GLYPH_SET.b).toBe("◯");
    expect(PLAYSTATION_GLYPH_SET.x).toBe("□");
    expect(PLAYSTATION_GLYPH_SET.y).toBe("△");
    // ...distinct from the xbox letters.
    expect(PLAYSTATION_GLYPH_SET.a).not.toBe(DEFAULT_GLYPH_SET.a);
  });

  it("GLYPH_SETS registry maps ids to the sets; default id is xbox", () => {
    expect(GLYPH_SETS.xbox).toBe(DEFAULT_GLYPH_SET);
    expect(GLYPH_SETS.playstation).toBe(PLAYSTATION_GLYPH_SET);
    expect(DEFAULT_GLYPH_SET_ID).toBe("xbox");
  });
});

describe("activeGlyphSet indirection", () => {
  it("defaults to the xbox set", () => {
    expect(activeGlyphSet()).toBe(DEFAULT_GLYPH_SET);
  });

  it("setActiveGlyphSetId switches by id and falls back for unknown/undefined", () => {
    setActiveGlyphSetId("playstation");
    expect(activeGlyphSet()).toBe(PLAYSTATION_GLYPH_SET);

    setActiveGlyphSetId("ps5-does-not-exist");
    expect(activeGlyphSet()).toBe(DEFAULT_GLYPH_SET);

    setActiveGlyphSetId("playstation");
    setActiveGlyphSetId(undefined);
    expect(activeGlyphSet()).toBe(DEFAULT_GLYPH_SET);
  });
});

describe("verbGlyph resolves verb → bound button → set glyph", () => {
  it("Confirm (bound to A by default) paints the set's A glyph", () => {
    // Xbox: A; PlayStation: ✕ — the verb is the same, the glyph tracks the set.
    expect(verbGlyph("Confirm", DEFAULT_BINDINGS, false, DEFAULT_GLYPH_SET)).toBe("A");
    expect(verbGlyph("Confirm", DEFAULT_BINDINGS, false, PLAYSTATION_GLYPH_SET)).toBe("✕");
  });
});
