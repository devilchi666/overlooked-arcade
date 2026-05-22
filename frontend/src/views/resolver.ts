// Runtime evaluation of view tree nodes — find a node by id, flatten a
// tree's leaves, resolve a node to the matching SystemIds, count games
// under a node, synthesize a virtual leaf when a deep-link points outside
// the active view. Pure functions; no Solid reactivity here — callers
// wrap the bits they need in `createMemo` for cheap re-evaluation.
//
// Per SIDEBAR_TIER_PLAN.md §2.3 + §3.3.

import { systemThemes, type SystemId } from "../themes/registry";
import type { ContainerNode, ContainerRule, PlatformNode, View, ViewNode } from "./types";
import { parsePlatformNodeId, platformNodeIdFor } from "./defaults";

/// DFS for a node by id. Returns null if not found. Search starts from
/// the view's root container (root itself is matchable — `nodeId = "root"`
/// resolves there). Called frequently by the sidebar render path; the
/// caller should memoize per (view, nodeId) pair.
export function findNode(view: View, nodeId: string): ViewNode | ContainerNode | null {
  if (view.root.id === nodeId) return view.root;
  return findInChildren(view.root.children, nodeId);
}

function findInChildren(children: ViewNode[], nodeId: string): ViewNode | null {
  for (const child of children) {
    if (child.id === nodeId) return child;
    if (child.kind === "container") {
      const inner = findInChildren(child.children, nodeId);
      if (inner) return inner;
    }
  }
  return null;
}

/// DFS-collect every PlatformNode under a starting node. Used by the
/// PR-β sidebar render (still flat — flatten the active view's leaves
/// into a list) and by per-container count badges in PR-γ.
export function flattenLeaves(node: ViewNode | ContainerNode): PlatformNode[] {
  if ("kind" in node && node.kind === "platform") {
    return [node];
  }
  const container = "kind" in node && node.kind === "container" ? node : (node as ContainerNode);
  const out: PlatformNode[] = [];
  for (const child of container.children) {
    out.push(...flattenLeaves(child));
  }
  return out;
}

/// Resolve a node (or a synthesized leaf id) to the SystemIds it filters
/// to. Containers evaluate their rule against the registry. Platform
/// leaves return their own systemId. Root container (no rule) returns
/// every registered system (equivalent to "All Games"). Synthesized
/// leaves — when `nodeId` looks like `platform:<systemId>` but no node
/// with that id exists in the view — return just that SystemId so
/// deep-links to systems outside the active view still resolve.
export function resolveNodeSystemIds(view: View, nodeId: string): SystemId[] {
  const found = findNode(view, nodeId);
  if (found) {
    if ("kind" in found && found.kind === "platform") return [found.systemId];
    const container = found as ContainerNode;
    if (container.rule === null) {
      // Root or any null-rule container — match everything.
      return Object.keys(systemThemes) as SystemId[];
    }
    return systemIdsMatchingRule(container.rule);
  }
  // Synthesized-leaf fallback per SIDEBAR_TIER_PLAN.md §0 / §2.6.
  const synth = parsePlatformNodeId(nodeId);
  return synth && synth in systemThemes ? [synth] : [];
}

function systemIdsMatchingRule(rule: ContainerRule): SystemId[] {
  const out: SystemId[] = [];
  for (const theme of Object.values(systemThemes)) {
    switch (rule.kind) {
      case "formFactor":
        if (theme.formFactor === rule.value) out.push(theme.id);
        break;
      case "manufacturer":
        if (theme.manufacturer === rule.value) out.push(theme.id);
        break;
      case "systemIds":
        if (rule.values.includes(theme.id)) out.push(theme.id);
        break;
    }
  }
  return out;
}

/// Count library entries under a node (recursive for containers; direct
/// for platforms). The PR-γ tree render uses this for per-node count
/// badges — wrapped in `createMemo` so reactive updates only re-evaluate
/// when the entries list or the node identity changes.
export function countGamesUnder(
  node: ViewNode | ContainerNode,
  entries: ReadonlyArray<{ systemId: SystemId; seed?: boolean }>,
): number {
  if ("kind" in node && node.kind === "platform") {
    return entries.filter((e) => e.systemId === node.systemId && !e.seed).length;
  }
  const container = ("kind" in node && node.kind === "container" ? node : (node as ContainerNode));
  return container.children.reduce((sum, c) => sum + countGamesUnder(c, entries), 0);
}

/// Construct a virtual PlatformNode for a SystemId not present in the
/// active view's tree. Used by the sidebar's deep-link routing path —
/// when a search-jump or "Show library" lands on a system the active
/// view's tree excludes, we synthesize a leaf at the root level rather
/// than refusing to navigate. The synthesized id matches the canonical
/// `platform:<systemId>` shape so `findNode` falls through to
/// `parsePlatformNodeId` correctly.
export function synthesizeLeafForSystem(systemId: SystemId): PlatformNode {
  return {
    id: platformNodeIdFor(systemId),
    systemId,
    hidden: false,
  };
}
