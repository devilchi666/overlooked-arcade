// Pure geometry for the spatial navigation engine (Unified Navigation arc,
// Phase 1). Given the focused element's rectangle, a set of candidate
// rectangles, and a direction, pick the best neighbour to move to.
//
// Model (the "pure spatial" movement model — webOS / Tizen / Steam-TV style):
//   - A candidate is "ahead" in a direction when its CENTER is strictly past
//     the current center on that axis (a 1px epsilon avoids picking same-row
//     siblings as "below" each other).
//   - Score = primary-axis distance + cross-axis MISALIGNMENT × weight. The
//     misalignment term is the non-overlap gap between the two boxes on the
//     perpendicular axis (0 when they overlap), so an aligned neighbour beats a
//     diagonally-offset one even when the offset one is physically closer.
//   - DOM order breaks ties (candidates are passed in document order and the
//     comparison is strict `<`, so the earliest wins a tie).
//
// This handles columns, rows, tab-strips, grids, and mixed forms with zero
// per-surface configuration. Kept pure (no DOM) so it unit-tests headlessly.

import type { NavDirection } from "./types";

export interface NavRect {
  left: number;
  top: number;
  right: number;
  bottom: number;
  cx: number;
  cy: number;
}

/// Build a NavRect (centers precomputed) from any edge-bearing rect, e.g. a
/// DOMRect from `getBoundingClientRect()`.
export function toNavRect(r: {
  left: number;
  top: number;
  right: number;
  bottom: number;
}): NavRect {
  return {
    left: r.left,
    top: r.top,
    right: r.right,
    bottom: r.bottom,
    cx: (r.left + r.right) / 2,
    cy: (r.top + r.bottom) / 2,
  };
}

/// Non-overlap distance between two 1-D spans. 0 when they overlap at all,
/// otherwise the gap between the nearest edges.
function axisGap(aMin: number, aMax: number, bMin: number, bMax: number): number {
  if (aMax < bMin) return bMin - aMax;
  if (bMax < aMin) return aMin - bMax;
  return 0;
}

/// How strongly cross-axis misalignment is penalised relative to primary-axis
/// distance among candidates that still OVERLAP on the cross axis. >1 keeps
/// movement "honest" (a neighbour more directly in line is preferred over a
/// closer-but-slightly-offset one). Tunable; 3 feels right on forms/grids.
const CROSS_WEIGHT = 3;

/// Flat penalty added when a candidate does NOT overlap the current element on
/// the cross axis at all (a "diagonal" target). Larger than any on-screen
/// primary distance, so an aligned same-row/-column neighbour ALWAYS beats a
/// diagonal one — e.g. Right from a form input lands on the same-row Reset
/// button, never a full-width section header sitting just below. Non-overlapping
/// candidates are still reachable when nothing aligned exists; they just rank
/// behind everything aligned.
const MISALIGN_PENALTY = 1_000_000;

/// Score a candidate for a move in `dir` from `cur`. Returns null when the
/// candidate is not ahead in that direction (i.e. not a legal target). Lower
/// score = better.
export function scoreCandidate(
  cur: NavRect,
  cand: NavRect,
  dir: NavDirection,
): number | null {
  const EPS = 1;
  let primary: number;
  let cross: number;
  switch (dir) {
    case "down":
      if (cand.cy <= cur.cy + EPS) return null;
      primary = cand.cy - cur.cy;
      cross = axisGap(cur.left, cur.right, cand.left, cand.right);
      break;
    case "up":
      if (cand.cy >= cur.cy - EPS) return null;
      primary = cur.cy - cand.cy;
      cross = axisGap(cur.left, cur.right, cand.left, cand.right);
      break;
    case "right":
      if (cand.cx <= cur.cx + EPS) return null;
      primary = cand.cx - cur.cx;
      cross = axisGap(cur.top, cur.bottom, cand.top, cand.bottom);
      break;
    case "left":
      if (cand.cx >= cur.cx - EPS) return null;
      primary = cur.cx - cand.cx;
      cross = axisGap(cur.top, cur.bottom, cand.top, cand.bottom);
      break;
  }
  return primary + cross * CROSS_WEIGHT + (cross > 0 ? MISALIGN_PENALTY : 0);
}

/// Pick the best item to move to in `dir` from `cur`. `candidates` should be in
/// document order (so ties resolve to the earliest). Returns null at an edge
/// (no candidate is ahead in that direction).
export function pickInDirection<T>(
  cur: NavRect,
  candidates: Array<{ rect: NavRect; item: T }>,
  dir: NavDirection,
): T | null {
  let best: T | null = null;
  let bestScore = Infinity;
  for (const c of candidates) {
    const s = scoreCandidate(cur, c.rect, dir);
    if (s === null) continue;
    if (s < bestScore) {
      bestScore = s;
      best = c.item;
    }
  }
  return best;
}

/// Euclidean distance between two rect centers. Used to recover focus to the
/// nearest focusable after the previously-focused element unmounts (e.g. a
/// Settings category swap replaces the whole center pane).
export function distanceToCenter(from: NavRect, to: NavRect): number {
  const dx = to.cx - from.cx;
  const dy = to.cy - from.cy;
  return Math.sqrt(dx * dx + dy * dy);
}

/// Pick the candidate whose center is closest to `from`, ignoring direction.
export function nearestTo<T>(
  from: NavRect,
  candidates: Array<{ rect: NavRect; item: T }>,
): T | null {
  let best: T | null = null;
  let bestDist = Infinity;
  for (const c of candidates) {
    const d = distanceToCenter(from, c.rect);
    if (d < bestDist) {
      bestDist = d;
      best = c.item;
    }
  }
  return best;
}
