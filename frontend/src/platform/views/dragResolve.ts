// Unified Navigation Tree — Arc Slice 2. The single, pure resolver that
// turns a solid-dnd drag (a dragged node id + the node it was dropped on)
// into a concrete tree mutation. Both the View Editor's drag handler and
// (eventually) the live sidebar call this so the nesting rules live in one
// tested place instead of being copy-pasted per surface.
//
// The rule generalizes the pre-Slice-2 leaf-only behaviour to ANY node kind
// at ANY depth:
//
//   - Same parent          → reorder the dragged node to the drop target's
//                            position (preserves top-level folder reorder +
//                            within-folder leaf reorder).
//   - Cross-parent, dropped
//     ONTO a folder         → nest the dragged node INTO that folder
//                            (append). A "folder" is a container that is
//                            not a game-set node — game-set nodes
//                            (collections / smart-lists) ignore their
//                            children, so dropping onto one inserts beside
//                            it instead.
//   - Cross-parent, dropped
//     onto any other row    → insert the dragged node BEFORE that row in the
//                            row's parent.
//
// Cycle-safe: refuses to drop a node into itself or one of its descendants
// (mirrors ViewsStore.moveNode's own guard, but rejected here so the caller
// can no-op cleanly). The root container is never movable.

import { findNodePath, isGameSetNode } from "./resolver";
import type { ContainerNode, View } from "./types";

export type DragOutcome =
  | { kind: "reorder"; parentId: string; order: string[] }
  | {
      kind: "move";
      nodeId: string;
      targetParentId: string;
      insertBeforeId: string | null;
    };

/// Resolve a drag (dragId dropped onto dropId) into a tree mutation, or
/// null when the drag is a no-op / illegal. Pure over the view tree — no
/// store access, fully unit-testable.
export function resolveDragOutcome(
  view: View,
  dragId: string,
  dropId: string,
): DragOutcome | null {
  if (dragId === dropId) return null;

  const dragPath = findNodePath(view, dragId);
  // Not found, or the dragged node is the root (root has no parent → not
  // movable; path length 1 means it's the root itself).
  if (!dragPath || dragPath.length < 2) return null;

  const dropPath = findNodePath(view, dropId);
  if (!dropPath || dropPath.length < 1) return null;

  // Cycle guard — the drop target lives inside the dragged node's subtree.
  if (dropPath.some((n) => n.id === dragId)) return null;

  const dragParent = dragPath[dragPath.length - 2] as ContainerNode;
  const dropNode = dropPath[dropPath.length - 1];
  const dropParent =
    dropPath.length >= 2 ? (dropPath[dropPath.length - 2] as ContainerNode) : null;

  // Same parent → reorder within that parent.
  if (dropParent && dropParent.id === dragParent.id) {
    const order = reorderedIds(dragParent, dragId, dropId);
    if (!order) return null;
    return { kind: "reorder", parentId: dragParent.id, order };
  }

  // Cross-parent, dropped onto a real folder → nest into it (append).
  const dropIsFolder =
    "children" in dropNode && !isGameSetNode(dropNode);
  if (dropIsFolder) {
    return {
      kind: "move",
      nodeId: dragId,
      targetParentId: dropId,
      insertBeforeId: null,
    };
  }

  // Cross-parent, dropped onto a leaf / game-set row → insert before it in
  // its parent.
  if (dropParent) {
    return {
      kind: "move",
      nodeId: dragId,
      targetParentId: dropParent.id,
      insertBeforeId: dropId,
    };
  }

  return null;
}

/// Compute the parent's child-id order with `dragId` moved to `dropId`'s
/// slot. Returns null when either id isn't a direct child (stale drag).
function reorderedIds(
  parent: ContainerNode,
  dragId: string,
  dropId: string,
): string[] | null {
  const ids = parent.children.map((c) => c.id);
  const fromIdx = ids.indexOf(dragId);
  const toIdx = ids.indexOf(dropId);
  if (fromIdx < 0 || toIdx < 0) return null;
  ids.splice(fromIdx, 1);
  ids.splice(toIdx, 0, dragId);
  return ids;
}
