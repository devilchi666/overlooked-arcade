// Pure unit coverage for the spatial-navigation geometry (Unified Navigation
// arc, Phase 1). The engine itself is DOM/behavioral (verified by typecheck +
// build + operator playtest, matching the existing pure-test approach); the
// directional scoring is pure and lives here.

import { describe, it, expect } from "vitest";
import {
  toNavRect,
  scoreCandidate,
  pickInDirection,
  nearestTo,
  type NavRect,
} from "./spatialGeometry";

/// Build a NavRect from x/y/w/h for terse fixtures.
function box(x: number, y: number, w = 100, h = 30): NavRect {
  return toNavRect({ left: x, top: y, right: x + w, bottom: y + h });
}

describe("scoreCandidate — direction gating", () => {
  const cur = box(0, 100);

  it("rejects candidates that are not ahead in the direction", () => {
    expect(scoreCandidate(cur, box(0, 100), "down")).toBeNull(); // same row
    expect(scoreCandidate(cur, box(0, 50), "down")).toBeNull(); // above
    expect(scoreCandidate(cur, box(0, 150), "up")).toBeNull(); // below
    expect(scoreCandidate(cur, box(-200, 100), "right")).toBeNull(); // left of
    expect(scoreCandidate(cur, box(200, 100), "left")).toBeNull(); // right of
  });

  it("accepts candidates ahead in the direction", () => {
    expect(scoreCandidate(cur, box(0, 150), "down")).not.toBeNull();
    expect(scoreCandidate(cur, box(0, 50), "up")).not.toBeNull();
    expect(scoreCandidate(cur, box(200, 100), "right")).not.toBeNull();
    expect(scoreCandidate(cur, box(-200, 100), "left")).not.toBeNull();
  });
});

describe("scoreCandidate — alignment preference", () => {
  it("prefers a vertically-aligned neighbour over a diagonal one even if the diagonal is closer", () => {
    const cur = box(0, 0);
    // aligned: directly below, 100px down
    const aligned = box(0, 100);
    // diagonal: only 60px down but shifted 300px to the right (no x-overlap)
    const diagonal = box(300, 60);
    const sa = scoreCandidate(cur, aligned, "down")!;
    const sd = scoreCandidate(cur, diagonal, "down")!;
    expect(sa).toBeLessThan(sd);
  });

  it("treats overlapping cross-axis spans as zero misalignment", () => {
    const cur = box(0, 0, 200, 30);
    const overlapping = box(50, 100, 50, 30); // x-span sits inside cur's
    const score = scoreCandidate(cur, overlapping, "down")!;
    // Pure primary distance (centers 100 apart) with no cross penalty.
    expect(score).toBeCloseTo(100);
  });
});

describe("pickInDirection", () => {
  const cur = box(0, 100);

  it("returns the nearest aligned neighbour", () => {
    const candidates = [
      { rect: box(0, 200), item: "far-below" },
      { rect: box(0, 130), item: "near-below" },
      { rect: box(0, 60), item: "above" },
    ];
    expect(pickInDirection(cur, candidates, "down")).toBe("near-below");
  });

  it("returns null at an edge (nothing ahead)", () => {
    const candidates = [
      { rect: box(0, 60), item: "above-1" },
      { rect: box(0, 20), item: "above-2" },
    ];
    expect(pickInDirection(cur, candidates, "down")).toBeNull();
  });

  it("breaks ties by document order (first wins)", () => {
    const candidates = [
      { rect: box(0, 160), item: "first" },
      { rect: box(0, 160), item: "second" }, // identical position
    ];
    expect(pickInDirection(cur, candidates, "down")).toBe("first");
  });

  it("moves sideways across a tab strip", () => {
    const a = box(0, 0, 80, 24);
    const b = box(90, 0, 80, 24);
    const c = box(180, 0, 80, 24);
    const candidates = [
      { rect: a, item: "a" },
      { rect: b, item: "b" },
      { rect: c, item: "c" },
    ];
    // from a → right lands on b (nearest), not c
    expect(pickInDirection(a, candidates.filter((x) => x.item !== "a"), "right")).toBe("b");
    // from c → left lands on b
    expect(pickInDirection(c, candidates.filter((x) => x.item !== "c"), "left")).toBe("b");
  });
});

describe("nearestTo (focus recovery)", () => {
  it("picks the closest center regardless of direction", () => {
    const from = box(0, 100);
    const candidates = [
      { rect: box(0, 400), item: "far" },
      { rect: box(20, 110), item: "close" },
      { rect: box(0, 0), item: "mid" },
    ];
    expect(nearestTo(from, candidates)).toBe("close");
  });

  it("returns null with no candidates", () => {
    expect(nearestTo(box(0, 0), [])).toBeNull();
  });
});
