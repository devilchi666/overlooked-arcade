// Unified Navigation Tree Slice 1 — node→membership resolution.
//
// Covers the keystone generalization: a node resolves to a set of SYSTEMS
// (system / group / root nodes — the pre-existing behaviour, untouched) or
// to an explicit set of GAMES (collection nodes, and the reserved-inert
// filter nodes). See features/unified-nav-tree/VIEW_MODEL.md.

import { describe, expect, it } from "vitest";
import { resolveNodeMembership, resolveNodeSystemIds } from "./resolver";
import { collectionNodeIdFor } from "./defaults";
import type { SystemId } from "@oa/platform/themes/registry";
import type { View } from "./types";

function makeView(): View {
  return {
    id: "v1",
    name: "Test",
    kind: "user-built",
    expandedNodes: [],
    root: {
      id: "root",
      label: "Root",
      rule: null,
      accent: null,
      art: null,
      hidden: false,
      children: [
        {
          kind: "platform",
          id: "platform:nes",
          systemId: "nes" as SystemId,
          hidden: false,
        },
        {
          kind: "container",
          id: "container:fav",
          label: "Favorites",
          rule: { kind: "collection", collectionId: "fav" },
          accent: null,
          art: null,
          hidden: false,
          children: [],
        },
        {
          kind: "container",
          id: "container:recent",
          label: "Recently Played",
          rule: { kind: "filter", spec: {} },
          accent: null,
          art: null,
          hidden: false,
          children: [],
        },
      ],
    },
  };
}

const members: ReadonlyMap<string, ReadonlySet<string>> = new Map([
  ["fav", new Set(["g1", "g2"])],
]);

describe("resolveNodeMembership", () => {
  it("resolves a platform leaf to its single system", () => {
    const m = resolveNodeMembership(makeView(), "platform:nes", members);
    expect(m).toEqual({ kind: "systems", systemIds: ["nes"] });
  });

  it("resolves the root (null rule) to all registered systems", () => {
    const m = resolveNodeMembership(makeView(), "root", members);
    expect(m.kind).toBe("systems");
    if (m.kind === "systems") expect(m.systemIds).toContain("nes");
  });

  it("resolves a collection-rule container to its member rom ids", () => {
    const m = resolveNodeMembership(makeView(), "container:fav", members);
    expect(m.kind).toBe("games");
    if (m.kind === "games") expect([...m.romIds]).toEqual(["g1", "g2"]);
  });

  it("resolves a synthesized collection:<id> node not in the tree", () => {
    // The sidebar's read-only Collections section navigates to a synthesized
    // id; findNode misses, so the collection id is recovered from the id.
    const m = resolveNodeMembership(makeView(), collectionNodeIdFor("fav"), members);
    expect(m.kind).toBe("games");
    if (m.kind === "games") expect([...m.romIds]).toEqual(["g1", "g2"]);
  });

  it("resolves an unknown collection id to an empty game set", () => {
    const m = resolveNodeMembership(makeView(), collectionNodeIdFor("nope"), members);
    expect(m).toEqual({ kind: "games", romIds: new Set() });
  });

  it("resolves a reserved filter-rule node to an empty game set (D59, inert)", () => {
    const m = resolveNodeMembership(makeView(), "container:recent", members);
    expect(m).toEqual({ kind: "games", romIds: new Set() });
  });
});

describe("resolveNodeSystemIds (unchanged sibling)", () => {
  it("still resolves a platform leaf to its system", () => {
    expect(resolveNodeSystemIds(makeView(), "platform:nes")).toEqual(["nes"]);
  });

  it("returns no systems for a collection rule (game-set node, not system-keyed)", () => {
    expect(resolveNodeSystemIds(makeView(), "container:fav")).toEqual([]);
  });
});
