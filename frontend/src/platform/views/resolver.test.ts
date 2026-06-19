// Unified Navigation Tree — node→membership resolution.
//
// Slice 1 covered the keystone generalization (systems vs an explicit game
// set). Slice 2 adds:
//   - `filter`-rule nodes backed by the shared smart-list predicates, and
//   - NT4 ancestry-aware composition: a `filterWithinParent` node narrows
//     its parent (favorite ∧ 2-player), with cross-axis (games ∩ systems)
//     intersection falling out.
// See features/unified-nav-tree/VIEW_MODEL.md + DECISIONS NT4.

import { describe, expect, it } from "vitest";
import {
  countGameSetNode,
  findNodePath,
  isGameSetNode,
  resolveNodeMembership,
  resolveNodeSystemIds,
} from "./resolver";
import { collectionNodeIdFor, filterNodeIdFor } from "./defaults";
import type { SystemId } from "@oa/platform/themes/registry";
import type { RomEntry } from "@oa/platform/library/types";
import type { ContainerNode, View, ViewNode } from "./types";

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

function rom(id: string, over: Partial<RomEntry> = {}): RomEntry {
  return {
    id,
    title: id,
    systemId: "nes" as SystemId,
    filePath: `/lib/${id}`,
    addedAt: 0,
    ...over,
  };
}

// g1: nes, favorite, 2P · g2: nes, favorite, 1P · g3: snes, not-fav, 2P
const entries: RomEntry[] = [
  rom("g1", { systemId: "nes" as SystemId, favorite: true, players: 2 }),
  rom("g2", { systemId: "nes" as SystemId, favorite: true, players: 1 }),
  rom("g3", { systemId: "snes" as SystemId, favorite: false, players: 2 }),
];

describe("resolveNodeMembership", () => {
  it("resolves a platform leaf to its single system", () => {
    const m = resolveNodeMembership(makeView(), "platform:nes", {
      collectionMembers: members,
    });
    expect(m).toEqual({ kind: "systems", systemIds: ["nes"] });
  });

  it("resolves the root (null rule) to all registered systems", () => {
    const m = resolveNodeMembership(makeView(), "root", { collectionMembers: members });
    expect(m.kind).toBe("systems");
    if (m.kind === "systems") expect(m.systemIds).toContain("nes");
  });

  it("resolves a collection-rule container to its member rom ids", () => {
    const m = resolveNodeMembership(makeView(), "container:fav", {
      collectionMembers: members,
    });
    expect(m.kind).toBe("games");
    if (m.kind === "games") expect([...m.romIds]).toEqual(["g1", "g2"]);
  });

  it("resolves a synthesized collection:<id> node not in the tree", () => {
    const m = resolveNodeMembership(makeView(), collectionNodeIdFor("fav"), {
      collectionMembers: members,
    });
    expect(m.kind).toBe("games");
    if (m.kind === "games") expect([...m.romIds]).toEqual(["g1", "g2"]);
  });

  it("resolves an unknown collection id to an empty game set", () => {
    const m = resolveNodeMembership(makeView(), collectionNodeIdFor("nope"), {
      collectionMembers: members,
    });
    expect(m).toEqual({ kind: "games", romIds: new Set() });
  });
});

describe("filter nodes (Slice 2)", () => {
  it("resolves an in-tree filter-rule node to its predicate matches", () => {
    const view = makeView();
    // Point container:recent at the favorites predicate so we can assert
    // a concrete match set.
    (view.root.children[2] as ContainerNode).rule = {
      kind: "filter",
      spec: { kind: "favorites" },
    };
    const m = resolveNodeMembership(view, "container:recent", { entries });
    expect(m.kind).toBe("games");
    if (m.kind === "games") expect([...m.romIds].sort()).toEqual(["g1", "g2"]);
  });

  it("resolves a synthesized filter:<kind> node from the sidebar section", () => {
    const m = resolveNodeMembership(makeView(), filterNodeIdFor("multiPlayer"), {
      entries,
    });
    expect(m.kind).toBe("games");
    if (m.kind === "games") expect([...m.romIds].sort()).toEqual(["g1", "g3"]);
  });

  it("resolves an unknown filter kind to an empty game set (reserved, e.g. dumpQuality)", () => {
    const m = resolveNodeMembership(makeView(), filterNodeIdFor("dumpQuality"), {
      entries,
    });
    expect(m).toEqual({ kind: "games", romIds: new Set() });
  });

  it("resolves a filter node with no entries wired to empty (graceful degrade)", () => {
    const m = resolveNodeMembership(makeView(), filterNodeIdFor("favorites"), {});
    expect(m).toEqual({ kind: "games", romIds: new Set() });
  });
});

describe("NT4 composition (filterWithinParent)", () => {
  /// root → collection "fav" (off) → filter "multiPlayer" (child); the
  /// child's filterWithinParent decides whether it narrows the parent.
  function makeNested(childFwp: boolean): View {
    const child: ViewNode = {
      kind: "container",
      id: "container:mp",
      label: "Multi-player",
      rule: { kind: "filter", spec: { kind: "multiPlayer" } },
      accent: null,
      art: null,
      hidden: false,
      filterWithinParent: childFwp,
      children: [],
    };
    const fav: ViewNode = {
      kind: "container",
      id: "container:fav",
      label: "Favorites",
      rule: { kind: "collection", collectionId: "fav" },
      accent: null,
      art: null,
      hidden: false,
      children: [child],
    };
    return {
      id: "v2",
      name: "Nested",
      kind: "user-built",
      expandedNodes: [],
      root: {
        id: "root",
        label: "Root",
        rule: null,
        accent: null,
        art: null,
        hidden: false,
        children: [fav],
      },
    };
  }

  const ctx = { collectionMembers: members, entries };

  it("ON narrows the parent: favorite ∧ 2-player = {g1}", () => {
    const m = resolveNodeMembership(makeNested(true), "container:mp", ctx);
    expect(m.kind).toBe("games");
    // fav = {g1,g2}; multi-player = {g1,g3}; intersection = {g1}.
    if (m.kind === "games") expect([...m.romIds]).toEqual(["g1"]);
  });

  it("OFF (default) ignores the parent: all multi-player = {g1,g3}", () => {
    const m = resolveNodeMembership(makeNested(false), "container:mp", ctx);
    expect(m.kind).toBe("games");
    if (m.kind === "games") expect([...m.romIds].sort()).toEqual(["g1", "g3"]);
  });

  it("cross-axis: a multi-player filter ON under a system group ∩s by system", () => {
    // root → group(nes) (off) → filter multiPlayer (on). multi-player on
    // nes = {g1} (g3 is snes).
    const child: ViewNode = {
      kind: "container",
      id: "container:mp",
      label: "Multi-player",
      rule: { kind: "filter", spec: { kind: "multiPlayer" } },
      accent: null,
      art: null,
      hidden: false,
      filterWithinParent: true,
      children: [],
    };
    const group: ViewNode = {
      kind: "container",
      id: "container:nintendo",
      label: "Nintendo",
      rule: { kind: "systemIds", values: ["nes" as SystemId] },
      accent: null,
      art: null,
      hidden: false,
      children: [child],
    };
    const view: View = {
      id: "v3",
      name: "Cross",
      kind: "user-built",
      expandedNodes: [],
      root: {
        id: "root",
        label: "Root",
        rule: null,
        accent: null,
        art: null,
        hidden: false,
        children: [group],
      },
    };
    const m = resolveNodeMembership(view, "container:mp", ctx);
    expect(m.kind).toBe("games");
    if (m.kind === "games") expect([...m.romIds]).toEqual(["g1"]);
  });
});

describe("findNodePath", () => {
  it("returns root→node inclusive for a nested node", () => {
    const view = makeView();
    const path = findNodePath(view, "container:fav");
    expect(path?.map((n) => n.id)).toEqual(["root", "container:fav"]);
  });

  it("returns just the root when asked for the root", () => {
    expect(findNodePath(makeView(), "root")?.map((n) => n.id)).toEqual(["root"]);
  });

  it("returns null for a node not in the tree", () => {
    expect(findNodePath(makeView(), "collection:fav")).toBeNull();
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

describe("isGameSetNode (Slice 1)", () => {
  const view = makeView();
  it("is true for collection + filter rule containers", () => {
    expect(isGameSetNode(view.root.children[1])).toBe(true); // container:fav
    expect(isGameSetNode(view.root.children[2])).toBe(true); // container:recent
  });
  it("is false for platform leaves, the root, and system/group containers", () => {
    expect(isGameSetNode(view.root.children[0])).toBe(false); // platform:nes
    expect(isGameSetNode(view.root)).toBe(false); // null-rule root
    const group: ContainerNode = {
      id: "g",
      label: "G",
      rule: { kind: "manufacturer", value: "nintendo" },
      accent: null,
      art: null,
      hidden: false,
      children: [],
    };
    expect(isGameSetNode(group)).toBe(false);
  });
});

describe("countGameSetNode (Slice 1)", () => {
  function collectionNode(collectionId: string): ContainerNode {
    return {
      id: "c",
      label: "C",
      rule: { kind: "collection", collectionId },
      accent: null,
      art: null,
      hidden: false,
      children: [],
    };
  }
  function filterNode(kind: string): ContainerNode {
    return {
      id: "f",
      label: "F",
      rule: { kind: "filter", spec: { kind } },
      accent: null,
      art: null,
      hidden: false,
      children: [],
    };
  }

  it("counts a collection from the eager memberCount map (not the lazy member set)", () => {
    const n = countGameSetNode(collectionNode("fav"), {
      entries,
      collectionMemberCounts: new Map([["fav", 7]]),
    });
    expect(n).toBe(7);
  });

  it("counts an unknown / unloaded collection as 0", () => {
    expect(countGameSetNode(collectionNode("nope"), { entries })).toBe(0);
  });

  it("counts a smart-list filter by evaluating its predicate over the entries", () => {
    // multi-player = {g1, g3} from the shared fixtures.
    expect(countGameSetNode(filterNode("multiPlayer"), { entries })).toBe(2);
    // favorites = {g1, g2}.
    expect(countGameSetNode(filterNode("favorites"), { entries })).toBe(2);
  });

  it("counts a reserved/unknown filter kind as 0 (e.g. dumpQuality)", () => {
    expect(countGameSetNode(filterNode("dumpQuality"), { entries })).toBe(0);
  });

  it("returns null for a non-game-set node so callers fall back to countGamesUnder", () => {
    const group: ContainerNode = {
      id: "g",
      label: "G",
      rule: { kind: "formFactor", value: "console" },
      accent: null,
      art: null,
      hidden: false,
      children: [],
    };
    expect(countGameSetNode(group, { entries })).toBeNull();
    expect(countGameSetNode(makeView().root, { entries })).toBeNull();
  });
});
