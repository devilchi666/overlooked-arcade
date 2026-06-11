// Shared declarative-config contract for the nav primitives (scope-call #8,
// "seam now"): primitives take declarative config objects (orientation /
// density / focus prominence / easing / data source) with MINIMAL, separable
// imperative escape hatches (the verb-named callbacks), so a future
// serializable layout DSL (ARC 2/3) can target them without a rewrite. The
// callbacks are named by VERB (onConfirm / onSecondary / …), never by button.

import type { Accessor, JSX } from "solid-js";
import type { Hints } from "../HintBar";

/// Spacing scale a theme picks per surface. Maps to layout tokens in S3 (the
/// token layer); in S1 it's surfaced as a `data-oa-density` attribute the
/// primitive sets, so the seam exists before the tokens do.
export type NavDensity = "compact" | "comfortable" | "spacious";

/// How the focused item is visually emphasised. Token-backed in S3; surfaced as
/// `data-oa-focus-prominence` in S1.
export type FocusProminence = "ring" | "scale" | "glow" | "none";

/// Motion curve for focus transitions. A motion token in S3, gated by
/// `prefers-reduced-motion` (the a11y baseline, scope-call #3). Surfaced as
/// `data-oa-easing` in S1.
export type NavEasing = "linear" | "standard" | "emphasized";

/// Context handed to the item renderer.
export type NavItemContext = {
  index: number;
  /** Reactive — true when this item is the focused one AND the group is active. */
  focused: Accessor<boolean>;
};

/// Semantic nav-sound events a primitive emits via `onNavSound` (scope-call #6,
/// "seam now"). Coarser than the full verb set on purpose — these are the cues a
/// browse surface makes noise for; the theme maps them to its `UiSoundEvent`s.
export type NavSoundEvent = "move" | "confirm" | "back" | "secondary";

/// Config common to every nav primitive. `T` is the item type; the primitive is
/// data-source-driven (`items`) + render-prop-driven (`children`), so it owns no
/// item-specific markup.
export type NavPrimitiveBaseProps<T> = {
  /** Stable focus-group id (see platform/nav/focus). */
  id: string;
  /** Declarative data source — array or reactive accessor. */
  items: T[] | Accessor<T[]>;
  /** Item renderer. */
  children: (item: T, ctx: NavItemContext) => JSX.Element;

  // --- declarative styling tokens (seams; deepen in S3) ---
  density?: NavDensity;
  focusProminence?: FocusProminence;
  easing?: NavEasing;
  class?: string;

  // --- focus / region wiring ---
  /** Neighbour group ids per direction — D-pad transfer at edges. */
  neighbours?: { up?: string; down?: string; left?: string; right?: string };
  /** Register without auto-claiming the active slot on mount. Default true. */
  autoActivate?: boolean;
  /** Optional controlled focus. Uncontrolled (internal signal) by default. */
  focusedIndex?: Accessor<number>;
  setFocusedIndex?: (n: number) => void;

  // --- hints ---
  /** Verb-keyed hints published while this primitive is mounted. */
  hints?: Hints;

  // --- verb-native escape hatches (minimal, separable; never raw buttons) ---
  onConfirm?: (index: number, item: T) => void;
  onBack?: () => void;
  onSecondary?: (index: number, item: T) => void;
  onTertiary?: (index: number, item: T) => void;

  /** Nav-sound hook (scope-call #6, engine-default-able): the primitive emits a
   * coarse `NavSoundEvent` on focus move / confirm / back / secondary with the
   * relevant item, and the theme maps it to a `UiSoundEvent` (e.g. via
   * `navSoundDispatcher` in platform/themes/systemUiSound). Omit for a silent
   * primitive. Kept a callback (not a built-in dispatch) so the nav layer stays
   * decoupled from the per-system audio machinery. */
  onNavSound?: (event: NavSoundEvent, item: T | undefined) => void;
};
