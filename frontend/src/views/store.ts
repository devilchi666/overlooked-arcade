// ViewsStore — Solid composable that hydrates views.json from Rust on
// mount, seeds defaults / migrates from legacy `layout.systemOrder` when
// the file is absent, and writes back via the Tauri `set_views` command
// on every mutation. Mirrors LayoutStore's shape (signals + setters +
// a hydration gate). Per SIDEBAR_TIER_PLAN.md §2.3 + §2.6.

import { batch, createEffect, createMemo, createSignal, onMount } from "solid-js";
import { invoke } from "@tauri-apps/api/core";

import {
  buildDefaultFormFactorView,
  buildDefaultManufacturerView,
  DEFAULT_VIEW_ID,
  LEGACY_VIEW_ID,
  MANUFACTURER_VIEW_ID,
  platformNodeIdFor,
  ROOT_NODE_ID,
} from "./defaults";
import { buildLegacyFlatView } from "./migration";
import {
  CURRENT_SCHEMA_VERSION,
  type ContainerNode,
  type ContainerRule,
  type View,
  type ViewNode,
  type ViewsConfig,
} from "./types";
import { systemThemes, type SystemId } from "../themes/registry";

export type NewViewTemplate =
  | "blank"
  | "copy-formfactor"
  | "copy-manufacturer"
  | "copy-legacy";

type LayoutLite = { systemOrder?: string[] };

export function createViewsStore() {
  const [config, setConfig] = createSignal<ViewsConfig>({
    schemaVersion: CURRENT_SCHEMA_VERSION,
    activeViewId: DEFAULT_VIEW_ID,
    views: [buildDefaultFormFactorView(), buildDefaultManufacturerView()],
    bannerDismissed: false,
  });
  const [hydrated, setHydrated] = createSignal(false);

  const activeView = createMemo<View | null>(() => {
    const c = config();
    return c.views.find((v) => v.id === c.activeViewId) ?? c.views[0] ?? null;
  });

  onMount(async () => {
    let next: ViewsConfig | null = null;
    try {
      next = await invoke<ViewsConfig | null>("get_views");
    } catch (e) {
      console.warn("[oa-views] get_views failed:", e);
    }
    if (next) {
      // Existing file — trust it (Rust-side migrate_inplace has already
      // bumped schemaVersion if needed). Then reconcile shipped defaults
      // so any newly-shipped default view (e.g. Manufacturers added in
      // v2.1) is appended without touching user state.
      setConfig(ensureShippedDefaults(next));
    } else {
      // No file yet — decide migration path based on legacy systemOrder.
      let legacyOrder: string[] = [];
      try {
        const layout = await invoke<LayoutLite>("get_layout");
        if (Array.isArray(layout.systemOrder)) legacyOrder = layout.systemOrder;
      } catch (e) {
        console.warn("[oa-views] get_layout failed during migration:", e);
      }
      const defaultView = buildDefaultFormFactorView();
      const manufacturerView = buildDefaultManufacturerView();
      let seeded: ViewsConfig;
      if (legacyOrder.length > 0) {
        const legacyView = buildLegacyFlatView(legacyOrder);
        seeded = {
          schemaVersion: CURRENT_SCHEMA_VERSION,
          activeViewId: LEGACY_VIEW_ID,
          views: [defaultView, manufacturerView, legacyView],
          bannerDismissed: false,
        };
      } else {
        seeded = {
          schemaVersion: CURRENT_SCHEMA_VERSION,
          activeViewId: DEFAULT_VIEW_ID,
          views: [defaultView, manufacturerView],
          bannerDismissed: false,
        };
      }
      setConfig(seeded);
      try {
        await invoke("set_views", { config: seeded });
      } catch (e) {
        console.warn("[oa-views] initial set_views failed:", e);
      }
    }
    setHydrated(true);
  });

  // Write-through to disk on every mutation after hydration. Gated on
  // hydrated() so the initial default-state setConfig doesn't echo
  // back before the disk read completes.
  createEffect(() => {
    if (!hydrated()) return;
    const snapshot = config();
    invoke("set_views", { config: snapshot }).catch((e) =>
      console.warn("[oa-views] set_views failed:", e),
    );
  });

  // ── Mutations ────────────────────────────────────────────────────

  function setActiveView(viewId: string): void {
    setConfig((prev) => ({ ...prev, activeViewId: viewId }));
  }

  function setBannerDismissed(dismissed: boolean): void {
    setConfig((prev) => ({ ...prev, bannerDismissed: dismissed }));
  }

  function toggleExpanded(nodeId: string): void {
    setConfig((prev) => mapActiveView(prev, (view) => {
      const expanded = new Set(view.expandedNodes);
      if (expanded.has(nodeId)) expanded.delete(nodeId);
      else expanded.add(nodeId);
      return { ...view, expandedNodes: [...expanded] };
    }));
  }

  function setNodeHidden(nodeId: string, hidden: boolean): void {
    setConfig((prev) => mapActiveView(prev, (view) => ({
      ...view,
      root: mapNode(view.root, nodeId, (n) => ({ ...n, hidden })),
    })));
  }

  function reorderChildren(parentId: string, newOrder: string[]): void {
    setConfig((prev) => mapActiveView(prev, (view) => ({
      ...view,
      root: mapNode(view.root, parentId, (n) => {
        if (!("children" in n)) return n;
        const byId = new Map<string, ViewNode>();
        for (const c of n.children) byId.set(c.id, c);
        const reordered: ViewNode[] = [];
        const seen = new Set<string>();
        for (const id of newOrder) {
          const child = byId.get(id);
          if (child && !seen.has(id)) {
            seen.add(id);
            reordered.push(child);
          }
        }
        // Append any children not in newOrder (defensive — UI should
        // pass the full set, but a stale newOrder shouldn't lose nodes).
        for (const c of n.children) {
          if (!seen.has(c.id)) reordered.push(c);
        }
        return { ...n, children: reordered };
      }),
    })));
  }

  /// Convenience for PR-γ's top-level drag-reorder — equivalent to
  /// reorderChildren("root", newOrder) but spelled for readability at
  /// the call site.
  function reorderTopLevel(newOrder: string[]): void {
    reorderChildren("root", newOrder);
  }

  /// Move a node from its current parent into `targetParentId`,
  /// inserting before `insertBeforeId` (or appending to the end when
  /// null). Used by PR-v2.2's cross-container drag + right-click "Move
  /// to category…" submenu.
  ///
  /// Cycle guard: refuses moves that would place the node inside
  /// itself or one of its descendants (e.g. dragging a container into
  /// one of its own children). Same-parent moves work — they're
  /// equivalent to reorderChildren but spelled as a position-relative
  /// shift, which matches the drag-end-handler's perspective when
  /// it doesn't pre-compute a full new order.
  function moveNode(
    nodeId: string,
    targetParentId: string,
    insertBeforeId: string | null,
  ): void {
    setConfig((prev) => mapActiveView(prev, (view) => {
      const located = locateNode(view.root, nodeId);
      if (!located) return view;
      if (nodeId === targetParentId) return view;
      // Cycle guard — refuse moves into the node's own subtree.
      if (isAncestor(view.root, nodeId, targetParentId)) return view;

      // Remove from current parent.
      let newRoot = mapNode(view.root, located.parentId, (parent) => {
        if (!("children" in parent)) return parent;
        return {
          ...parent,
          children: parent.children.filter((c) => c.id !== nodeId),
        };
      });

      // Insert into target parent at insertBeforeId's position (or end).
      newRoot = mapNode(newRoot, targetParentId, (parent) => {
        if (!("children" in parent)) return parent;
        const insertIdx = insertBeforeId === null
          ? parent.children.length
          : parent.children.findIndex((c) => c.id === insertBeforeId);
        const at = insertIdx < 0 ? parent.children.length : insertIdx;
        const newChildren = parent.children.slice();
        newChildren.splice(at, 0, located.node);
        return { ...parent, children: newChildren };
      });

      return { ...view, root: newRoot };
    }));
  }

  /// Replace the entire active view's tree. Used when applying
  /// `reorderForFormFactor` after the operator picks "Try Form Factor
  /// view" — swaps the seeded default for the reordered version.
  function replaceView(viewId: string, replacement: View): void {
    setConfig((prev) => ({
      ...prev,
      views: prev.views.map((v) => (v.id === viewId ? replacement : v)),
    }));
  }

  /// Batched migration commit — apply Option C reorder + switch active
  /// view + dismiss banner in one update so write-through fires once.
  function commitTryFormFactor(reorderedDefault: View): void {
    batch(() => {
      replaceView(DEFAULT_VIEW_ID, reorderedDefault);
      setActiveView(DEFAULT_VIEW_ID);
      setBannerDismissed(true);
    });
  }

  // ── v3.1: view metadata CRUD ───────────────────────────────────

  /// Create a new user-built view from a template. Returns the new
  /// view's id so the caller can immediately switch active to it /
  /// jump to the editor. Per VIEW_EDITOR_PLAN.md §1.2.
  ///
  /// Template semantics:
  ///   blank             — empty root container ("My View"), no children
  ///   copy-formfactor   — deep clone of the shipped FormFactor view
  ///   copy-manufacturer — deep clone of the shipped Manufacturer view
  ///   copy-legacy       — deep clone of the seeded Flat-Legacy view
  ///                       (only valid on upgrade installs that have it)
  function createView(name: string, template: NewViewTemplate): string {
    const trimmed = name.trim();
    if (!trimmed) throw new Error("View name cannot be empty");
    const newId = generateUserViewId(config().views);
    let root: ContainerNode;
    let expandedNodes: string[];
    if (template === "blank") {
      root = {
        id: ROOT_NODE_ID,
        label: trimmed,
        rule: null,
        accent: null,
        art: null,
        hidden: false,
        children: [],
      };
      expandedNodes = [ROOT_NODE_ID];
    } else {
      const sourceId =
        template === "copy-formfactor"
          ? DEFAULT_VIEW_ID
          : template === "copy-manufacturer"
            ? MANUFACTURER_VIEW_ID
            : LEGACY_VIEW_ID;
      const source = config().views.find((v) => v.id === sourceId);
      if (!source) {
        throw new Error(`Cannot clone view ${sourceId} — not present in current config`);
      }
      const clone = cloneViewTree(source.root, newId);
      root = { ...clone, label: trimmed };
      expandedNodes = source.expandedNodes.map((nodeId) =>
        rewriteClonedNodeId(nodeId, source.root.id, root.id, newId),
      );
    }
    // v3.5.2: clone explicitlyRemoved from the source for copy-*
    // templates so the operator's "I removed NES from Platforms"
    // intent carries into a new "Copy of Platforms". Blank views
    // start fresh.
    const sourceForRemoved =
      template === "copy-formfactor"
        ? config().views.find((v) => v.id === DEFAULT_VIEW_ID)
        : template === "copy-manufacturer"
          ? config().views.find((v) => v.id === MANUFACTURER_VIEW_ID)
          : template === "copy-legacy"
            ? config().views.find((v) => v.id === LEGACY_VIEW_ID)
            : null;
    const newView: View = {
      id: newId,
      name: trimmed,
      kind: "user-built",
      expandedNodes,
      root,
      explicitlyRemoved: sourceForRemoved
        ? [...(sourceForRemoved.explicitlyRemoved ?? [])]
        : [],
    };
    setConfig((prev) => ({ ...prev, views: [...prev.views, newView] }));
    return newId;
  }

  function renameView(viewId: string, name: string): void {
    const trimmed = name.trim();
    if (!trimmed) return;
    setConfig((prev) => ({
      ...prev,
      views: prev.views.map((v) => (v.id === viewId ? { ...v, name: trimmed } : v)),
    }));
  }

  /// Delete a user-built view. Refuses on `user-builtin` kind (shipped
  /// defaults stay deletable only via factory reset / views.json edit).
  /// Falls back active view to FormFactor if the deleted view was
  /// active.
  function deleteView(viewId: string): void {
    const target = config().views.find((v) => v.id === viewId);
    if (!target) return;
    if (target.kind === "user-builtin") return;
    setConfig((prev) => {
      const remaining = prev.views.filter((v) => v.id !== viewId);
      const nextActive =
        prev.activeViewId === viewId
          ? (remaining.find((v) => v.id === DEFAULT_VIEW_ID)?.id ?? remaining[0]?.id ?? prev.activeViewId)
          : prev.activeViewId;
      return { ...prev, views: remaining, activeViewId: nextActive };
    });
  }

  // ── v3.2: container + leaf CRUD ────────────────────────────────

  /// Add a new container as the last child of `parentId` in the active
  /// view. Returns the new container's id. Generated as
  /// `${viewId}:container:N` so ids stay unique within and across
  /// views (mirrors the cloneViewTree id-rewriting convention).
  function addContainer(
    parentId: string,
    label: string,
    rule: ContainerRule | null = null,
  ): string | null {
    const view = activeView();
    if (!view) return null;
    const newId = generateContainerId(view);
    const container: ContainerNode = {
      id: newId,
      label: label.trim() || "New container",
      rule,
      accent: null,
      art: null,
      hidden: false,
      children: [],
    };
    setConfig((prev) => mapActiveView(prev, (v) => ({
      ...v,
      root: mapNode(v.root, parentId, (n) => {
        if (!("children" in n)) return n;
        return { ...n, children: [...n.children, { kind: "container", ...container }] };
      }),
    })));
    return newId;
  }

  function setContainerLabel(nodeId: string, label: string): void {
    const trimmed = label.trim();
    if (!trimmed) return;
    setConfig((prev) => mapActiveView(prev, (view) => ({
      ...view,
      root: mapNode(view.root, nodeId, (n) => ({ ...n, label: trimmed })),
    })));
  }

  function setContainerRule(nodeId: string, rule: ContainerRule | null): void {
    setConfig((prev) => mapActiveView(prev, (view) => ({
      ...view,
      root: mapNode(view.root, nodeId, (n) => ({ ...n, rule })),
    })));
  }

  /// Add a platform leaf for `systemId` as the last child of
  /// `parentId`. Returns the new leaf's id, or null if the system is
  /// already represented by a leaf anywhere in the active view (no
  /// duplicate manual leaves per view — operators wanting overlap
  /// across containers use rule-based containers per
  /// VIEW_EDITOR_PLAN.md §0.11).
  ///
  /// v3.5.2: clears the systemId from the view's explicitlyRemoved
  /// list — operator added it back, so the marker that would have
  /// prevented v3.5.3's reconciler from re-introducing it should
  /// also clear.
  function addPlatformLeaf(parentId: string, systemId: SystemId): string | null {
    const view = activeView();
    if (!view) return null;
    if (containsLeafForSystem(view.root, systemId)) return null;
    const leafId = platformNodeIdFor(systemId);
    setConfig((prev) => mapActiveView(prev, (v) => ({
      ...v,
      root: mapNode(v.root, parentId, (n) => {
        if (!("children" in n)) return n;
        return {
          ...n,
          children: [
            ...n.children,
            { kind: "platform", id: leafId, systemId, hidden: false },
          ],
        };
      }),
      explicitlyRemoved: (v.explicitlyRemoved ?? []).filter((s) => s !== systemId),
    })));
    return leafId;
  }

  /// Remove a node (container OR leaf) from its parent's children.
  /// Refuses on the root id — the root is a structural anchor, not a
  /// removable container. Cascades any descendant subtree along with
  /// the removed node.
  ///
  /// v3.5.2: when the removed node is a platform leaf, push its
  /// systemId onto the view's `explicitlyRemoved` list (deduped).
  /// v3.5.3's reconciler consults this list to keep the deletion
  /// from being undone on next launch. Container removals don't
  /// push anything — only individual platforms can be auto-re-added
  /// by the reconciler.
  function removeNode(nodeId: string): void {
    if (nodeId === ROOT_NODE_ID) return;
    setConfig((prev) => mapActiveView(prev, (view) => {
      const located = locateNode(view.root, nodeId);
      const removedLeafSystemId =
        located && "kind" in located.node && located.node.kind === "platform"
          ? located.node.systemId
          : null;
      const existing = view.explicitlyRemoved ?? [];
      const updatedRemoved =
        removedLeafSystemId && !existing.includes(removedLeafSystemId)
          ? [...existing, removedLeafSystemId]
          : existing;
      return {
        ...view,
        root: removeNodeFromTree(view.root, nodeId),
        explicitlyRemoved: updatedRemoved,
      };
    }));
  }

  return {
    config,
    activeView,
    hydrated,
    setActiveView,
    setBannerDismissed,
    toggleExpanded,
    setNodeHidden,
    reorderChildren,
    reorderTopLevel,
    moveNode,
    replaceView,
    commitTryFormFactor,
    createView,
    renameView,
    deleteView,
    addContainer,
    setContainerLabel,
    setContainerRule,
    addPlatformLeaf,
    removeNode,
  };
}

export type ViewsStore = ReturnType<typeof createViewsStore>;

// ── Internal tree-mutation helpers ─────────────────────────────────

/// Reconciler — runs on hydrate to bring older configs forward to the
/// current shipped-default surface. Two passes:
///
/// 1. Append any entirely-missing shipped default view (β.2's original
///    behavior). Covers operators whose views.json was created before
///    a particular shipped view existed.
///
/// 2. Auto-extend existing FormFactor + Manufacturer shipped views
///    with newly-registered systems (v3.5.3). When a future PR adds
///    a 42nd system to systemThemes, this pass slots it into the
///    appropriate bucket of the operator's existing FormFactor +
///    Manufacturer views — unless the operator has explicitly
///    removed it (per the v3.5.2 marker).
///
/// Flat-Legacy stays frozen-after-seed; auto-extending its single
/// container would reshuffle the operator's customized order.
///
/// Idempotent: running on a config that's already up-to-date returns
/// it unchanged (by reference). The write-through effect persists
/// the augmented config on next mutation, so a one-shot launch after
/// new systems land is enough to update the operator's views.json.
function ensureShippedDefaults(config: ViewsConfig): ViewsConfig {
  const have = new Set(config.views.map((v) => v.id));
  let updated = config;
  if (!have.has(DEFAULT_VIEW_ID)) {
    updated = { ...updated, views: [...updated.views, buildDefaultFormFactorView()] };
  }
  if (!have.has(MANUFACTURER_VIEW_ID)) {
    updated = { ...updated, views: [...updated.views, buildDefaultManufacturerView()] };
  }
  // v3.5.3: auto-extend pass for shipped FormFactor + Manufacturer.
  updated = {
    ...updated,
    views: updated.views.map((view) => {
      if (view.id === DEFAULT_VIEW_ID) return autoExtendShippedView(view, "formFactor");
      if (view.id === MANUFACTURER_VIEW_ID) return autoExtendShippedView(view, "manufacturer");
      return view;
    }),
  };
  return updated;
}

/// Walk the registry; for each system not present in `view` AND not
/// in `view.explicitlyRemoved`, find the container whose rule matches
/// the system's tag (formFactor or manufacturer per `ruleKind`) and
/// append a platform leaf there. If no matching container exists
/// (operator deleted "Consoles" entirely), skip the leaf — surfacing
/// it elsewhere would surprise the operator. Never reorders existing
/// leaves; never modifies hidden flags.
function autoExtendShippedView(
  view: View,
  ruleKind: "formFactor" | "manufacturer",
): View {
  const existing = new Set<SystemId>();
  collectLeafSystemIds(view.root, existing);
  const removed = new Set<SystemId>((view.explicitlyRemoved ?? []) as SystemId[]);

  let root = view.root;
  let changed = false;
  for (const systemId of Object.keys(systemThemes) as SystemId[]) {
    if (existing.has(systemId)) continue;
    if (removed.has(systemId)) continue;
    const tagValue =
      ruleKind === "formFactor"
        ? systemThemes[systemId].formFactor
        : systemThemes[systemId].manufacturer;
    const target = findContainerByRule(root, ruleKind, tagValue);
    if (!target) continue;
    root = mapNode(root, target.id, (n) => {
      if (!("children" in n)) return n;
      return {
        ...n,
        children: [
          ...n.children,
          {
            kind: "platform",
            id: platformNodeIdFor(systemId),
            systemId,
            hidden: false,
          },
        ],
      };
    });
    changed = true;
  }
  return changed ? { ...view, root } : view;
}

function collectLeafSystemIds(node: ContainerNode | ViewNode, into: Set<SystemId>): void {
  if ("kind" in node && node.kind === "platform") {
    into.add(node.systemId);
    return;
  }
  if (!("children" in node)) return;
  for (const child of node.children) collectLeafSystemIds(child, into);
}

function findContainerByRule(
  node: ContainerNode | ViewNode,
  ruleKind: "formFactor" | "manufacturer",
  value: string,
): ContainerNode | null {
  if (!("children" in node)) return null;
  const container = node as ContainerNode;
  if (container.rule && container.rule.kind === ruleKind && container.rule.value === value) {
    return container;
  }
  for (const child of container.children) {
    const inner = findContainerByRule(child, ruleKind, value);
    if (inner) return inner;
  }
  return null;
}

function mapActiveView(config: ViewsConfig, fn: (view: View) => View): ViewsConfig {
  return {
    ...config,
    views: config.views.map((v) => (v.id === config.activeViewId ? fn(v) : v)),
  };
}

type NodeLike = ViewNode | View["root"];

/// Recursive tree map — replaces the matching node via `transform` and
/// rebuilds the path back to the root. Non-matching subtrees are
/// preserved by reference (Solid's reactivity sees the new top-level
/// object and re-runs the write-through effect; downstream consumers
/// that memoize per-node identity still skip work for unchanged
/// subtrees).
function mapNode<T extends NodeLike>(node: T, targetId: string, transform: (n: T) => T): T {
  if (node.id === targetId) return transform(node);
  if (!("children" in node)) return node;
  return {
    ...node,
    children: node.children.map((c) => mapNode(c, targetId, transform as never)),
  } as T;
}

/// DFS that returns the node matching `nodeId` along with its parent
/// container's id. Returns null if not found OR if the node is the
/// root (root has no parent within the tree). Used by moveNode to
/// find a node's current location so it can be removed from its
/// current parent's children.
function locateNode(
  root: View["root"],
  nodeId: string,
): { node: ViewNode; parentId: string } | null {
  function walk(container: View["root"]): { node: ViewNode; parentId: string } | null {
    for (const child of container.children) {
      if (child.id === nodeId) return { node: child, parentId: container.id };
      if ("children" in child) {
        const inner = walk(child as View["root"]);
        if (inner) return inner;
      }
    }
    return null;
  }
  return walk(root);
}

// ── v3.2 helpers ──────────────────────────────────────────────────

/// Generate a unique container id within the given view. Scheme:
/// `${viewId}:container:N`. Walks all existing node ids in the view
/// to avoid collisions even when the operator has manually edited
/// views.json.
function generateContainerId(view: View): string {
  const taken = new Set<string>();
  walkIds(view.root, taken);
  let counter = 1;
  for (;;) {
    const candidate = `${view.id}:container:${counter}`;
    if (!taken.has(candidate)) return candidate;
    counter++;
  }
}

function walkIds(node: ViewNode | ContainerNode, into: Set<string>): void {
  into.add(node.id);
  if ("children" in node) {
    for (const child of node.children) walkIds(child, into);
  }
}

/// DFS for any platform leaf with `systemId === target`. Used to
/// refuse duplicate manual leaves in addPlatformLeaf — operators who
/// want a system to appear in multiple containers should use rule-
/// based containers (formFactor / manufacturer / systemIds).
function containsLeafForSystem(node: ViewNode | ContainerNode, target: SystemId): boolean {
  if ("kind" in node && node.kind === "platform") return node.systemId === target;
  if (!("children" in node)) return false;
  for (const child of node.children) {
    if (containsLeafForSystem(child, target)) return true;
  }
  return false;
}

/// Recursive tree map — removes the matching node from its parent's
/// children and rebuilds the path back to the root.
function removeNodeFromTree(node: ContainerNode, targetId: string): ContainerNode {
  const filtered: ViewNode[] = [];
  for (const child of node.children) {
    if (child.id === targetId) continue;
    if (child.kind === "container") {
      filtered.push({ kind: "container", ...removeNodeFromTree(child, targetId) });
    } else {
      filtered.push(child);
    }
  }
  return { ...node, children: filtered };
}

// ── v3.1 helpers ──────────────────────────────────────────────────

/// Generate a unique id for a new user-built view. Counter-based with a
/// collision-check fallback so manual edits to views.json that introduce
/// `user-N`-shaped ids don't accidentally collide.
function generateUserViewId(existing: ReadonlyArray<View>): string {
  const taken = new Set(existing.map((v) => v.id));
  let counter = 1;
  for (;;) {
    const candidate = `user-${counter}`;
    if (!taken.has(candidate)) return candidate;
    counter++;
  }
}

/// Deep-clone a view's root container into a fresh tree with rewritten
/// node ids prefixed by the new view's id. Keeps the structure /
/// rules / accents / hide flags intact; just renames node ids so the
/// clone's nodes don't accidentally collide with the source view's
/// nodes (relevant for the expandedNodes lookup + drag-reorder
/// targeting).
function cloneViewTree(source: ContainerNode, newViewId: string): ContainerNode {
  function clone(node: ContainerNode): ContainerNode {
    return {
      ...node,
      id: rewriteClonedNodeId(node.id, source.id, ROOT_NODE_ID, newViewId),
      children: node.children.map((c): ViewNode => {
        if (c.kind === "container") {
          return { kind: "container", ...clone(c) };
        }
        return {
          ...c,
          id: rewriteClonedNodeId(c.id, source.id, ROOT_NODE_ID, newViewId),
        };
      }),
    };
  }
  return clone(source);
}

/// Rewrite a node id from `source` tree's namespace to the cloned
/// tree's namespace. Root nodes keep "root"; other nodes get prefixed
/// with the new view's id so cloned containers / leaves don't
/// collide with the source view's ids.
function rewriteClonedNodeId(
  nodeId: string,
  sourceRootId: string,
  destRootId: string,
  newViewId: string,
): string {
  if (nodeId === sourceRootId) return destRootId;
  return `${newViewId}:${nodeId}`;
}

/// Returns true when `descendantId` is `ancestorId` itself OR one of
/// its descendants. Used as the cycle guard in moveNode — prevents
/// moving a container into itself or into one of its own children.
function isAncestor(
  root: View["root"],
  ancestorId: string,
  descendantId: string,
): boolean {
  function walk(node: ViewNode | View["root"]): boolean {
    if (node.id === ancestorId) {
      return findInSubtree(node, descendantId);
    }
    if ("children" in node) {
      for (const child of node.children) {
        if (walk(child)) return true;
      }
    }
    return false;
  }
  function findInSubtree(node: ViewNode | View["root"], targetId: string): boolean {
    if (node.id === targetId) return true;
    if ("children" in node) {
      for (const child of node.children) {
        if (findInSubtree(child, targetId)) return true;
      }
    }
    return false;
  }
  return walk(root);
}
