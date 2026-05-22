// Migration helpers for the views model.
//
// `buildLegacyFlatView` converts a pre-views-model operator's customized
// `layout.systemOrder` into a single-container "Flat (Legacy)" view —
// preserves their drag-reorder work as a selectable view rather than
// silently discarding it. PR-β seeds this on first launch alongside the
// default form-factor view; PR-γ's migration banner offers the operator a
// choice between them.
//
// `reorderForFormFactor` implements "Option C" from the design contract:
// when the operator clicks "Try Form Factor view," their relative ordering
// of platforms within each form-factor bucket is preserved (NES still sits
// above SNES if that's how they arranged the legacy list). Per
// SIDEBAR_TIER_PLAN.md §0 + §2.3.

import { systemThemes, type SystemId } from "../themes/registry";
import {
  DEFAULT_VIEW_ID,
  LEGACY_VIEW_ID,
  ROOT_NODE_ID,
  platformNodeIdFor,
} from "./defaults";
import type { ContainerNode, View, ViewNode } from "./types";

const FLAT_LEGACY_CONTAINER_ID = "container:flat-legacy";

/// Build a single-container view holding every registered system in the
/// operator's old `systemOrder`, with any registry-only systems (added
/// after their last reorder) appended in registry order. Result is the
/// same shape today's flat sidebar shows — switching to it from PR-γ's
/// banner produces zero visible change.
export function buildLegacyFlatView(systemOrder: ReadonlyArray<string>): View {
  const registry = Object.keys(systemThemes) as SystemId[];
  const registrySet = new Set<string>(registry);
  const seen = new Set<string>();
  const leaves: ViewNode[] = [];
  for (const id of systemOrder) {
    if (registrySet.has(id) && !seen.has(id)) {
      seen.add(id);
      leaves.push({
        kind: "platform",
        id: platformNodeIdFor(id as SystemId),
        systemId: id as SystemId,
        hidden: false,
      });
    }
  }
  for (const id of registry) {
    if (!seen.has(id)) {
      leaves.push({
        kind: "platform",
        id: platformNodeIdFor(id),
        systemId: id,
        hidden: false,
      });
    }
  }

  const container: ContainerNode = {
    id: FLAT_LEGACY_CONTAINER_ID,
    label: "All Systems",
    /// systemIds rule lets the runtime treat this container as a
    /// "match this exact set" filter — identical-feeling to today's
    /// `kind: "all"` for the legacy operator.
    rule: { kind: "systemIds", values: leaves.map((n) => (n as { systemId: SystemId }).systemId) },
    accent: null,
    art: null,
    hidden: false,
    children: leaves,
  };

  const root: ContainerNode = {
    id: ROOT_NODE_ID,
    label: "All Systems",
    rule: null,
    accent: null,
    art: null,
    hidden: false,
    children: [{ kind: "container", ...container }],
  };

  return {
    id: LEGACY_VIEW_ID,
    name: "Flat (Legacy)",
    kind: "user-builtin",
    expandedNodes: [ROOT_NODE_ID, FLAT_LEGACY_CONTAINER_ID],
    root,
  };
}

/// Apply Option C — within each form-factor bucket of `defaultView`,
/// reorder the platform leaves so they match the operator's relative
/// ordering from `systemOrder`. Systems not present in `systemOrder`
/// are appended after the ordered ones in their original bucket
/// position. PR-γ's "Try Form Factor view" button calls this before
/// switching the active view so the operator's customization doesn't
/// vanish.
export function reorderForFormFactor(
  defaultView: View,
  systemOrder: ReadonlyArray<string>,
): View {
  // Rank lookup: position in operator's customized order.
  const rank = new Map<string, number>();
  systemOrder.forEach((id, idx) => rank.set(id, idx));

  function reorderContainer(container: ContainerNode): ContainerNode {
    const reorderedChildren = container.children.map((child) => {
      if (child.kind === "container") return { kind: "container" as const, ...reorderContainer(child) };
      return child;
    });
    // Sort platform leaves by operator-rank (unranked sink to the end
    // in their original relative order). Container children are
    // pinned in place.
    const stable = reorderedChildren.map((c, idx) => ({ c, idx }));
    stable.sort((a, b) => {
      if (a.c.kind !== "platform" || b.c.kind !== "platform") return a.idx - b.idx;
      const ra = rank.get(a.c.systemId);
      const rb = rank.get(b.c.systemId);
      if (ra === undefined && rb === undefined) return a.idx - b.idx;
      if (ra === undefined) return 1;
      if (rb === undefined) return -1;
      return ra - rb;
    });
    return { ...container, children: stable.map((s) => s.c) };
  }

  return {
    ...defaultView,
    id: DEFAULT_VIEW_ID,
    root: reorderContainer(defaultView.root),
  };
}
