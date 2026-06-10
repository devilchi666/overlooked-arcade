// Semantic navigation verbs — the abstract vocabulary the nav primitives,
// the focus framework, and the HintBar all consume INSTEAD of raw buttons.
// See docs/features/theming-substrate/DECISIONS.md D18 (nav is verb-based +
// user-remappable; button meanings are a per-user contract, not per-theme)
// and PLANS/theming-substrate.md §13.3 S1.
//
// The indirection is: raw physical input (NavButton / NavDirection from
// `./types`) → a `navBindings` map (`./navBindings`) → these verbs. Remapping
// a button changes the map; the primitives and every on-screen hint update
// for free because they speak verbs, never buttons.

/// The fixed verb set. Directional verbs (Up/Down/Left/Right) carry the same
/// movement meaning a D-pad/stick direction does; the focus framework still
/// distinguishes the physical *source* (D-pad transfers between regions, stick
/// walks within) via the raw NavDirectionEvent — see `./focus`.
///
/// `Confirm` / `Back` / `Secondary` / `Tertiary` are the four focused-item
/// actions (A / B / X / Y by default). `PrevSection` / `NextSection` are the
/// L1 / R1 role (cycle whatever top-level structure the active theme exposes —
/// tabs in Retroverse). `Menu` is the contextual-options role (Start by
/// default). `OpenQuickSettings` is reserved on the gamepad in S1 (the in-game
/// QS overlay is summoned via the engine/Esc path while the shell nav bus is
/// disabled mid-game) — see the S1 design note. `Search` / `Favorite` / `Page`
/// are reserved-but-unbound: part of the contract so themes/hints can reference
/// them, with no default binding yet.
export type NavVerb =
  // focused-item actions
  | "Confirm"
  | "Back"
  | "Secondary"
  | "Tertiary"
  // directional
  | "Up"
  | "Down"
  | "Left"
  | "Right"
  // structural / global
  | "PrevSection"
  | "NextSection"
  | "Menu"
  | "OpenQuickSettings"
  // reserved-but-unbound (no default binding in S1)
  | "Search"
  | "Favorite"
  | "Page";

/// The four directional verbs, as a runtime-checkable set. A direction verb is
/// dispatched as movement (carrying its physical source) rather than as a
/// focused-item action.
export const DIRECTION_VERBS = ["Up", "Down", "Left", "Right"] as const;
export type DirectionVerb = (typeof DIRECTION_VERBS)[number];

export function isDirectionVerb(verb: NavVerb): verb is DirectionVerb {
  return (DIRECTION_VERBS as readonly string[]).includes(verb);
}

/// Verbs with no default binding in S1. Bindable later via the remap UI
/// (the Phase-3 follow-on) but inert today — listed so the contract is
/// explicit and a theme/hint can reference them without a magic string.
export const RESERVED_VERBS: readonly NavVerb[] = [
  "OpenQuickSettings",
  "Search",
  "Favorite",
  "Page",
];
