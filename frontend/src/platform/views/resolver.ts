// Runtime evaluation of view tree nodes — find a node by id, flatten a
// tree's leaves, resolve a node to the matching SystemIds, count games
// under a node, synthesize a virtual leaf when a deep-link points outside
// the active view. Pure functions; no Solid reactivity here — callers
// wrap the bits they need in `createMemo` for cheap re-evaluation.
//
// Per SIDEBAR_TIER_PLAN.md §2.3 + §3.3.

import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import type { RomEntry } from "@oa/platform/library/types";
import {
  asSmartListKind,
  evaluateSmartList,
} from "@oa/platform/library/smartLists";
import type { ContainerNode, ContainerRule, PlatformNode, View, ViewNode } from "./types";
import {
  parseCollectionNodeId,
  parseFilterNodeId,
  parsePlatformNodeId,
  platformNodeIdFor,
} from "./defaults";

/// Shared empty member set so the "no members yet / unknown collection"
/// path never allocates. ReadonlySet so callers can't mutate it.
const EMPTY_ROM_SET: ReadonlySet<string> = new Set<string>();

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

/// The chain of nodes from the view root down to (and including) `nodeId`,
/// or null when the node isn't in this view's tree. Unified Navigation Tree
/// NT4 needs the ancestor path — not just the node — because a node with
/// `filterWithinParent` set composes its membership with its parent's
/// effective membership, so resolution can no longer happen by id in
/// isolation. `path[0]` is always the root container; `path[path.length-1]`
/// is the target. Sibling to `findNode` (which only returns the target).
export function findNodePath(
  view: View,
  nodeId: string,
): (ContainerNode | ViewNode)[] | null {
  if (view.root.id === nodeId) return [view.root];
  const path: (ContainerNode | ViewNode)[] = [view.root];
  return walkPath(view.root.children, nodeId, path) ? path : null;
}

function walkPath(
  children: ViewNode[],
  nodeId: string,
  path: (ContainerNode | ViewNode)[],
): boolean {
  for (const child of children) {
    if (child.id === nodeId) {
      path.push(child);
      return true;
    }
    if (child.kind === "container") {
      path.push(child);
      if (walkPath(child.children, nodeId, path)) return true;
      path.pop();
    }
  }
  return false;
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
      case "collection":
      case "filter":
        // Not system-keyed rules — they resolve to an explicit game set
        // via resolveNodeMembership, not to SystemIds. A direct
        // resolveNodeSystemIds call on such a node (e.g. an old caller
        // doing a system count) gets an empty system list, which is the
        // correct "no systems match" answer.
        break;
    }
  }
  return out;
}

/// The membership a node resolves to. Unified Navigation Tree keystone:
/// generalizes resolution from "every node maps to SystemId[]" to "a node
/// maps to a set of systems OR an explicit set of games." System / group /
/// root nodes stay `systems`; collection (and, from Slice 2, filter) nodes
/// resolve to `games`. See features/unified-nav-tree/VIEW_MODEL.md.
export type NodeMembership =
  | { kind: "systems"; systemIds: SystemId[] }
  | { kind: "games"; romIds: ReadonlySet<string> };

/// Inputs the membership resolver may need beyond the view + node id.
/// All optional with graceful degradation — a caller that wires only
/// collections (Slice 1's behaviour) keeps working; filter nodes + the
/// NT4 cross-axis intersection just resolve empty when their inputs are
/// absent. Passed as one object so future inputs don't reshuffle args.
export type MembershipContext = {
  /// The `customCollections` store's member map (`Map<id, Set<romId>>`).
  /// Drives `collection`-rule nodes.
  collectionMembers?: ReadonlyMap<string, ReadonlySet<string>>;
  /// The live library entries — needed to evaluate `filter`-rule
  /// predicates AND to intersect a game-set with a system-set (NT4
  /// cross-axis: which rom ids belong to an allowed system).
  entries?: ReadonlyArray<RomEntry>;
  /// Current time in Unix seconds for time-dependent predicates
  /// (Recently Played). Caller supplies `Math.floor(Date.now()/1000)`.
  nowSecs?: number;
};

/// Resolve a node id to its membership, **ancestry-aware** (NT4). A node
/// with `filterWithinParent` set intersects its own membership with its
/// parent's effective membership, chaining up the path until the nearest
/// "off" ancestor (which is the independent base set); off nodes (the
/// default) resolve on their own membership regardless of position — i.e.
/// today's flat behaviour. Cross-axis intersection (a game-set node under
/// a system group) falls out: membership is the common currency, so
/// `{games} ∩ {systems}` collapses to the games in an allowed system.
///
/// Game-set nodes (collection rules; `filter` rules backed by the shared
/// smart-list predicates; and the synthesized `collection:<id>` /
/// `filter:<kind>` ids from the sidebar's read-only sections) resolve to
/// `{ games }`. System / group / root nodes stay `{ systems }`.
///
/// `resolveNodeSystemIds` is intentionally left intact for callers that
/// still need a pure system list (count badges, active-row highlight);
/// this is an additive sibling, not a replacement.
export function resolveNodeMembership(
  view: View,
  nodeId: string,
  ctx: MembershipContext,
): NodeMembership {
  const path = findNodePath(view, nodeId);
  if (path) {
    // Fold parent ∩ child down the path. Start at the root's own
    // membership; each `filterWithinParent` node narrows the running
    // effective set, each off node resets to its own (the base set).
    let effective = ownMembership(view, path[0], ctx);
    for (let i = 1; i < path.length; i++) {
      const node = path[i];
      const own = ownMembership(view, node, ctx);
      const composes =
        !("kind" in node && node.kind === "platform") &&
        Boolean((node as ContainerNode).filterWithinParent);
      effective = composes ? intersectMembership(effective, own, ctx) : own;
    }
    return effective;
  }

  // Not in the persisted tree — the sidebar's read-only Collections /
  // Smart Lists sections navigate to synthesized ids (Slice 4 adds real
  // nodes; both shapes resolve here). Synthesized nodes are standalone
  // (no parent), so no composition.
  const synthCollectionId = parseCollectionNodeId(nodeId);
  if (synthCollectionId !== null) {
    return {
      kind: "games",
      romIds: ctx.collectionMembers?.get(synthCollectionId) ?? EMPTY_ROM_SET,
    };
  }
  const synthFilterKind = parseFilterNodeId(nodeId);
  if (synthFilterKind !== null) {
    return evalFilterMembership({ kind: synthFilterKind }, ctx);
  }

  // Synthesized `platform:<systemId>` deep-link or unknown id — delegate
  // to the system resolver (its own synthesized-leaf fallback handles it).
  return { kind: "systems", systemIds: resolveNodeSystemIds(view, nodeId) };
}

/// A single node's OWN membership, ignoring ancestry. Platform leaves →
/// their system; collection rules → their member set; filter rules → their
/// predicate matches; everything else (root null-rule, system/group
/// rules) → systems via the unchanged `resolveNodeSystemIds`.
function ownMembership(
  view: View,
  node: ContainerNode | ViewNode,
  ctx: MembershipContext,
): NodeMembership {
  if ("kind" in node && node.kind === "platform") {
    return { kind: "systems", systemIds: [(node as PlatformNode).systemId] };
  }
  const rule = (node as ContainerNode).rule;
  if (rule?.kind === "collection") {
    return {
      kind: "games",
      romIds: ctx.collectionMembers?.get(rule.collectionId) ?? EMPTY_ROM_SET,
    };
  }
  if (rule?.kind === "filter") {
    return evalFilterMembership(rule.spec, ctx);
  }
  return { kind: "systems", systemIds: resolveNodeSystemIds(view, node.id) };
}

/// Evaluate a `filter`-rule spec to a `{ games }` membership via the shared
/// smart-list registry. Unknown kinds (e.g. the reserved-but-unwired
/// `dumpQuality`) and a missing entry list resolve to empty — the same
/// graceful-degrade contract collection nodes use when their members
/// aren't wired.
function evalFilterMembership(
  spec: Record<string, unknown>,
  ctx: MembershipContext,
): NodeMembership {
  const kind = asSmartListKind(spec);
  if (kind === null || !ctx.entries) {
    return { kind: "games", romIds: EMPTY_ROM_SET };
  }
  return {
    kind: "games",
    romIds: evaluateSmartList(kind, ctx.entries, { nowSecs: ctx.nowSecs ?? 0 }),
  };
}

/// Intersect two memberships (NT4 composition). systems ∩ systems stays a
/// system set (no library needed — preserves the pure-system fast path);
/// any pair involving a game-set collapses to a rom-id set, projecting a
/// system-set onto the rom ids of its member systems via `ctx.entries`.
function intersectMembership(
  a: NodeMembership,
  b: NodeMembership,
  ctx: MembershipContext,
): NodeMembership {
  if (a.kind === "systems" && b.kind === "systems") {
    const allow = new Set(b.systemIds);
    return { kind: "systems", systemIds: a.systemIds.filter((s) => allow.has(s)) };
  }
  const aRoms = membershipToRomIds(a, ctx);
  const bRoms = membershipToRomIds(b, ctx);
  const [small, large] = aRoms.size <= bRoms.size ? [aRoms, bRoms] : [bRoms, aRoms];
  const out = new Set<string>();
  for (const id of small) if (large.has(id)) out.add(id);
  return { kind: "games", romIds: out };
}

/// Project a membership onto a concrete rom-id set. Game-set → its ids
/// directly; system-set → the ids of entries in an allowed system (empty
/// when `ctx.entries` is absent — the cross-axis intersection degrades to
/// empty rather than guessing).
function membershipToRomIds(
  m: NodeMembership,
  ctx: MembershipContext,
): ReadonlySet<string> {
  if (m.kind === "games") return m.romIds;
  const allow = new Set<SystemId>(m.systemIds);
  const out = new Set<string>();
  for (const e of ctx.entries ?? []) {
    if (allow.has(e.systemId)) out.add(e.id);
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

/// DFS-collect every container node under `node` that carries
/// `hidden: true`. Used by v2.3's "Hidden containers" recovery UI in
/// LibraryManagerPage — once an operator right-clicks a container to
/// hide it, this is the surface that exposes the un-hide affordance.
/// Walks at any depth so v3+ custom views with nested containers are
/// covered, not just the v2 default views' two-level shape.
export function collectHiddenContainers(node: ContainerNode | ViewNode): ContainerNode[] {
  const out: ContainerNode[] = [];
  function walk(n: ContainerNode | ViewNode): void {
    if ("kind" in n && n.kind === "platform") return;
    const container = n as ContainerNode;
    if (container.hidden) out.push(container);
    for (const child of container.children) walk(child);
  }
  walk(node);
  return out;
}

/// Returns true when `targetId` matches `node`'s id or any descendant's
/// id. Used by container-hide routing to decide whether to redirect
/// the operator off a stranded node when their parent container gets
/// hidden.
export function nodeContainsId(node: ViewNode | ContainerNode, targetId: string): boolean {
  if (node.id === targetId) return true;
  if ("kind" in node && node.kind === "platform") return false;
  const container = node as ContainerNode;
  for (const child of container.children) {
    if (nodeContainsId(child, targetId)) return true;
  }
  return false;
}
