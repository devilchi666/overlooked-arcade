// Smart-list predicates — the single source of truth for the built-in
// "filtered game-set" definitions (Favorites / Recently Played / … ).
//
// Unified Navigation Tree Slice 2 lifted these out of the COLLECTIONS-tab
// page (themes/retroverse/CollectionsPage.tsx) so BOTH consumers share one
// definition rather than drifting:
//   - the COLLECTIONS tab renders them as a left-pane list, and
//   - the navigation-tree resolver evaluates them as `filter`-rule nodes
//     (a `filter:<kind>` node resolves to the rom-id set its predicate
//     matches — see platform/views/resolver.ts).
//
// The predicates are PURE: anything time-dependent (the recency window)
// takes `nowSecs` from the eval context rather than calling Date.now()
// internally, so the resolver stays deterministic + unit-testable. The
// caller supplies `Math.floor(Date.now() / 1000)` at render.

import type { RomEntry } from "./types";

/// Stable identifier for a built-in smart list. Persisted inside a
/// `filter`-rule node's opaque `spec` as `{ kind: SmartListKind }` (the
/// Rust mirror keeps `spec` as free-form JSON, so adding a kind needs no
/// schema bump). camelCase to match the views serde convention.
export type SmartListKind =
  | "favorites"
  | "recentlyPlayed"
  | "completed"
  | "multiPlayer"
  | "hiddenGems"
  | "lastPlayed";

/// Inputs a predicate may need beyond the entry itself. Kept minimal +
/// explicit so predicates are pure functions (no ambient Date.now()).
export type SmartListEvalContext = {
  /// Current time in Unix seconds. Drives the "recently played" window.
  nowSecs: number;
};

export type SmartListDef = {
  kind: SmartListKind;
  label: string;
  /// Single-glyph icon for the sidebar row / header card.
  glyph: string;
  /// One-line description shown on the COLLECTIONS header card.
  description: string;
  /// Returns true to include `entry` in this list. Pure — all
  /// time-dependence flows through `ctx`.
  predicate: (entry: RomEntry, ctx: SmartListEvalContext) => boolean;
};

// 30-day recency window — matches the original design doc's "rolling
// window."
const RECENT_WINDOW_SECONDS = 30 * 24 * 60 * 60;

// Hidden Gems "barely touched" threshold. Under 30 minutes of total play
// time across all sessions is the "you sampled it but never really got
// into it" zone — the operator's existing rating / favoriting then becomes
// the "this deserves another look" signal.
const HIDDEN_GEM_PLAY_THRESHOLD_SECS = 30 * 60;
// Hidden Gems rating cutoff. 4.0 / 5.0 covers "great" and above when the
// metadata enrichment populates the rating field; libraries without rating
// data fall back to the favorite-flagged half of the OR (so the list still
// surfaces something useful).
const HIDDEN_GEM_RATING_CUTOFF = 4.0;

/// The built-in smart lists, in display order. Every entry ships a real
/// predicate (the historical Hidden Gems / Last Played placeholders are
/// long since wired). Dump-quality is deliberately NOT here yet — as a
/// navigation node it needs a product meaning + an any-vs-default-variant
/// semantic call, and its data lives variant-side, not on RomEntry; it's
/// reserved for a follow-up (Slice 2 design note).
export const SMART_LISTS: readonly SmartListDef[] = [
  {
    kind: "favorites",
    label: "Favorites",
    glyph: "❤",
    description: "Games you've ♥-tagged from any tile or context menu.",
    predicate: (entry) => Boolean(entry.favorite),
  },
  {
    kind: "recentlyPlayed",
    label: "Recently played",
    glyph: "🕘",
    description: "Played within the last 30 days. Sorted by most recent.",
    predicate: (entry, ctx) => {
      if (!entry.lastPlayedAt) return false;
      return ctx.nowSecs - entry.lastPlayedAt < RECENT_WINDOW_SECONDS;
    },
  },
  {
    kind: "completed",
    label: "Completed",
    glyph: "✓",
    description: "Marked complete via the tile context menu.",
    predicate: (entry) => Boolean(entry.completed),
  },
  {
    kind: "multiPlayer",
    label: "Multi-player",
    glyph: "👥",
    description: "Two-player or more (metadata-driven).",
    predicate: (entry) => (entry.players ?? 1) >= 2,
  },
  {
    kind: "hiddenGems",
    label: "Hidden gems",
    glyph: "💎",
    description:
      "Highly rated or favorited, but you've barely touched them (< 30 minutes total).",
    predicate: (entry) => {
      const playTime = entry.playTimeSecs ?? 0;
      if (playTime >= HIDDEN_GEM_PLAY_THRESHOLD_SECS) return false;
      const highlyRated =
        typeof entry.rating === "number" && entry.rating >= HIDDEN_GEM_RATING_CUTOFF;
      return highlyRated || Boolean(entry.favorite);
    },
  },
  {
    kind: "lastPlayed",
    label: "Last played",
    glyph: "🏁",
    description:
      "Every game you've ever launched, most recent first. The full play history — Recently played is the 30-day rolling subset.",
    predicate: (entry) => Boolean(entry.lastPlayedAt),
  },
];

/// O(1) lookup by kind. Built once at module load.
export const SMART_LISTS_BY_KIND: Readonly<Record<SmartListKind, SmartListDef>> =
  Object.fromEntries(SMART_LISTS.map((l) => [l.kind, l])) as Record<
    SmartListKind,
    SmartListDef
  >;

/// Narrow an opaque `filter`-rule spec to a known built-in kind, or null
/// when it isn't one. Takes the structural `Record<string, unknown>` shape
/// (NOT the views `FilterRuleSpec` type) so this library module stays free
/// of a back-edge import on platform/views. The resolver passes
/// `rule.spec` straight in.
export function asSmartListKind(
  spec: Record<string, unknown> | null | undefined,
): SmartListKind | null {
  const k = spec && typeof spec === "object" ? spec["kind"] : undefined;
  return typeof k === "string" && k in SMART_LISTS_BY_KIND ? (k as SmartListKind) : null;
}

/// Evaluate a built-in smart list over a flat entry list, returning the
/// matching rom-id set. The resolver wraps this as a `{ kind: "games" }`
/// membership; the COLLECTIONS tab keeps its own filter+sort because it
/// also sorts the results, which is a presentation concern.
export function evaluateSmartList(
  kind: SmartListKind,
  entries: ReadonlyArray<RomEntry>,
  ctx: SmartListEvalContext,
): Set<string> {
  const def = SMART_LISTS_BY_KIND[kind];
  const out = new Set<string>();
  for (const e of entries) {
    if (def.predicate(e, ctx)) out.add(e.id);
  }
  return out;
}
