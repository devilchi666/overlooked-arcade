// The canonical controller model — the single normalization target for the
// Controller Identity arc. Defined once here (frontend) and mirrored in
// crates/oa-input/src/canonical.rs (Rust); the two MUST stay in sync.
//
// Per decision D4 this IS the SDL / Xbox standard layout. Phase 2 maps each
// physical pad's raw buttons/axes onto these names (via controllers.json);
// once normalized, the existing nav verb map (menus) and the per-system
// gameplay maps (default-maps.json) operate on canonical names and "just
// work" for any pad — including non-standard ones like the wired Switch Pro.
//
// This file is the CONTRACT only. Phase 1 introduces no normalization logic;
// it just pins the vocabulary both layers and all later phases build against.

/// Canonical digital buttons. Face buttons use SDL position names (south/east/
/// west/north) rather than letters to stay unambiguous across Nintendo's
/// physically-swapped A/B/X/Y — the letter→position mapping is a per-profile
/// concern (controllers.json), not part of the canonical vocabulary.
export type CanonicalButton =
  | "south" // bottom face (Xbox A)
  | "east" // right face (Xbox B)
  | "west" // left face (Xbox X)
  | "north" // top face (Xbox Y)
  | "up"
  | "down"
  | "left"
  | "right"
  | "l1" // left bumper
  | "r1" // right bumper
  | "l2" // left trigger (also a CanonicalAxis when analog)
  | "r2" // right trigger (also a CanonicalAxis when analog)
  | "l3" // left stick click
  | "r3" // right stick click
  | "start"
  | "select" // a.k.a. back / share
  | "guide"; // home / system button

/// Canonical analog axes. Sticks are bipolar (-1..1); triggers are unipolar
/// (0..1) but reuse the l2/r2 names so a digital-only pad can map them to the
/// CanonicalButton of the same name.
export type CanonicalAxis =
  | "left_x"
  | "left_y"
  | "right_x"
  | "right_y"
  | "l2"
  | "r2";
