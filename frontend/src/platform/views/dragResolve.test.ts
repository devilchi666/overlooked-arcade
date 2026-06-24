// Unified Navigation Tree — Arc Slice 2 drag resolution.
//
// resolveDragOutcome is the one pure place the nesting rules live. These
// tests pin the generalized (any-kind, any-depth) behaviour:
//   - same-parent reorder
//   - cross-parent nest INTO a folder
//   - cross-parent insert BESIDE a leaf / game-set
//   - root immovable, cycle rejected
// See features/unified-nav-tree/DECISIONS NT4 + the Slice 2 plan.

import { describe, expect, it } from "vitest";
import { resolveDragOutcome } from "./dragResolve";
import type { SystemId } from "@oa/platform/themes/registry";
import type { View } from "./types";

/// Fixture tree:
///   root
///     ├─ platform:nes            (leaf, parent = root)
///     ├─ container:group "Nintendo"   (folder)
///     │     └─ platform:snes     (leaf, parent = group)
///     ├─ container:list "2-Player"    (smart-list game-set, parent = root)
///     └─ container:empty "Empty"      (folder, no children)
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
        { kind: "platform", id: "platform:nes", systemId: "nes" as SystemId, hidden: false },
        {
          kind: "container",
          id: "container:group",
          label: "Nintendo",
          rule: { kind: "manufacturer", value: "nintendo" },
          accent: null,
          art: null,
          hidden: false,
          children: [
            { kind: "platform", id: "platform:snes", systemId: "snes" as SystemId, hidden: false },
          ],
        },
        {
          kind: "container",
          id: "container:list",
          label: "2-Player",
          rule: { kind: "filter", spec: { kind: "multiPlayer" } },
          accent: null,
          art: null,
          hidden: false,
          children: [],
        },
        {
          kind: "container",
          id: "container:empty",
          label: "Empty",
          rule: null,
          accent: null,
          art: null,
          hidden: false,
          children: [],
        },
      ],
    },
  };
}

describe("resolveDragOutcome", () => {
  it("returns null for a drop on self", () => {
    expect(resolveDragOutcome(makeView(), "platform:nes", "platform:nes")).toBeNull();
  });

  it("refuses to move the root", () => {
    expect(resolveDragOutcome(makeView(), "root", "platform:nes")).toBeNull();
  });

  it("returns null for unknown ids", () => {
    expect(resolveDragOutcome(makeView(), "nope", "platform:nes")).toBeNull();
    expect(resolveDragOutcome(makeView(), "platform:nes", "nope")).toBeNull();
  });

  it("reorders within the same parent (top level)", () => {
    // nes (idx0) dropped onto group (idx1) → nes moves to group's slot.
    const out = resolveDragOutcome(makeView(), "platform:nes", "container:group");
    expect(out).toEqual({
      kind: "reorder",
      parentId: "root",
      order: ["container:group", "platform:nes", "container:list", "container:empty"],
    });
  });

  it("nests INTO a folder on a cross-parent drop onto the folder", () => {
    // snes (inside group) dropped onto the empty folder → append into empty.
    const out = resolveDragOutcome(makeView(), "platform:snes", "container:empty");
    expect(out).toEqual({
      kind: "move",
      nodeId: "platform:snes",
      targetParentId: "container:empty",
      insertBeforeId: null,
    });
  });

  it("nests a top-level list into a group by dropping onto the group's child", () => {
    // The headline path: 2-Player (top level) dropped onto snes (inside the
    // Nintendo group) → inserted before snes, i.e. joins the group.
    const out = resolveDragOutcome(makeView(), "container:list", "platform:snes");
    expect(out).toEqual({
      kind: "move",
      nodeId: "container:list",
      targetParentId: "container:group",
      insertBeforeId: "platform:snes",
    });
  });

  it("inserts BESIDE a game-set node (does not nest into it)", () => {
    // snes (inside group) dropped onto the 2-Player smart-list → game-sets
    // ignore children, so snes lands beside it in root, not inside it.
    const out = resolveDragOutcome(makeView(), "platform:snes", "container:list");
    expect(out).toEqual({
      kind: "move",
      nodeId: "platform:snes",
      targetParentId: "root",
      insertBeforeId: "container:list",
    });
  });

  it("rejects a cycle (folder onto its own descendant)", () => {
    expect(resolveDragOutcome(makeView(), "container:group", "platform:snes")).toBeNull();
  });
});
