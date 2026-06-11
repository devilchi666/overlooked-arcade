// Controller-glyph abstraction (scope-call #4, "seam now").
//
// A GlyphSet maps physical buttons (+ the two directional descriptors) to the
// short label the HintBar paints. One default set ships (Xbox-style). The
// HintBar renders a VERB's glyph by resolving verb → currently-bound button
// (via navBindings, swap-aware) → glyph — so remapping updates every hint for
// free, and a future Xbox/PS/Switch glyph-set picker is a drop-in (swap the
// GlyphSet; resolution is unchanged). Auto-detect + the picker are deferred.

import { createSignal, type Accessor } from "solid-js";
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

/// PlayStation-style set: ✕ bottom (Confirm in the Western PS convention),
/// ◯ right, □ left, △ top; Options (≡) / Create (⊟); PS home. Shoulders keep
/// L1/R1/L2/R2 text labels (legible + unambiguous). S5.3 ships this as the
/// second built-in set so the glyph-set indirection has a real consumer; the
/// user-facing picker + pad auto-detect stay deferred (scope-call #4).
export const PLAYSTATION_GLYPH_SET: GlyphSet = {
  a: "✕",
  b: "◯",
  x: "□",
  y: "△",
  l1: "L1",
  r1: "R1",
  l2: "L2",
  r2: "R2",
  start: "≡",
  select: "⊟",
  l3: "L3",
  r3: "R3",
  home: "PS",
  dpad: "✥",
  stick: "○",
};

/// Built-in glyph-set ids. A theme picks one via its manifest `glyph_set`
/// (default `xbox`); the union widens when more sets ship.
export type GlyphSetId = "xbox" | "playstation";

/// The built-in glyph-set registry — the single map from id → GlyphSet. The
/// validator checks a theme's `glyph_set` against this, and `setActiveGlyphSetId`
/// resolves through it.
export const GLYPH_SETS: Record<GlyphSetId, GlyphSet> = {
  xbox: DEFAULT_GLYPH_SET,
  playstation: PLAYSTATION_GLYPH_SET,
};

/// The fallback glyph set when a theme declares none (or an unknown id).
export const DEFAULT_GLYPH_SET_ID: GlyphSetId = "xbox";

const [activeGlyphSetSig, setActiveGlyphSetSig] = createSignal<GlyphSet>(DEFAULT_GLYPH_SET);

/// The glyph set the HintBar currently paints (reactive). Defaults to the Xbox
/// set; App.tsx bridges the active theme's manifest `glyph_set` here via
/// `setActiveGlyphSetId` (mirrors the S1 settings→nav bridges like `setSwapAB`).
/// A future user picker / controller auto-detect would call the same setter —
/// the seam is "set the active glyph set"; the source is swappable (#4 deferred).
export const activeGlyphSet: Accessor<GlyphSet> = activeGlyphSetSig;

/// Set the active glyph set by id. Unknown / undefined falls back to the default
/// set, so a theme with a typo'd `glyph_set` still renders (Xbox) — matching the
/// validator's WARNING (a cosmetic mismatch must not disqualify a whole theme).
export function setActiveGlyphSetId(id: string | undefined): void {
  setActiveGlyphSetSig(id != null && id in GLYPH_SETS ? GLYPH_SETS[id as GlyphSetId] : DEFAULT_GLYPH_SET);
}

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
