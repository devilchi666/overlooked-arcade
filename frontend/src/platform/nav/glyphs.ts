// Controller-glyph abstraction (scope-call #4, "seam now").
//
// A GlyphSet maps physical buttons (+ the two directional descriptors) to the
// short label the HintBar paints. One default set ships (Xbox-style). The
// HintBar renders a VERB's glyph by resolving verb → currently-bound button
// (via navBindings, swap-aware) → glyph — so remapping updates every hint for
// free, and a future Xbox/PS/Switch glyph-set picker is a drop-in (swap the
// GlyphSet; resolution is unchanged). Auto-detect + the picker are deferred.

import type { NavButton } from "./types";
import type { NavVerb } from "./verbs";
import { buttonForVerb, type NavBindings } from "./navBindings";

/// Glyphs for every physical button plus the D-pad / stick directional
/// descriptors (which are not remappable per-button in S1, so they carry fixed
/// glyphs rather than resolving through a verb).
export type GlyphSet = Record<NavButton, string> & {
  dpad: string;
  stick: string;
};

/// Xbox-style default: Y top, A bottom, X left, B right; LB/RB shoulders;
/// ≡ start, ▢ select. Matches the pre-verb HINT_GLYPH map verbatim.
export const DEFAULT_GLYPH_SET: GlyphSet = {
  a: "A",
  b: "B",
  x: "X",
  y: "Y",
  l1: "LB",
  r1: "RB",
  l2: "LT",
  r2: "RT",
  start: "≡",
  select: "▢",
  l3: "L3",
  r3: "R3",
  home: "⌂",
  dpad: "✥",
  stick: "○",
};

/// Glyph for a verb under the current bindings (swap-aware), or null when no
/// physical button maps to it (e.g. reserved verbs) — the HintBar omits those.
export function verbGlyph(
  verb: NavVerb,
  bindings: NavBindings,
  swap: boolean,
  glyphSet: GlyphSet = DEFAULT_GLYPH_SET,
): string | null {
  const button = buttonForVerb(verb, bindings, swap);
  return button ? glyphSet[button] : null;
}
