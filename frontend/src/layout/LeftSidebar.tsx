import { For, Show, createMemo, type Component } from "solid-js";
import {
  closestCenter,
  DragDropProvider,
  DragDropSensors,
  SortableProvider,
  type DragEventHandler,
} from "@thisbeyond/solid-dnd";

import type { LibraryStore } from "../library/store";
import { systemThemes, type SystemId } from "../themes/registry";
import type { LayoutStore } from "./state";
import type { ContainerNode, PlatformNode, ViewNode } from "../views/types";
import { parsePlatformNodeId, platformNodeIdFor } from "../views/defaults";
import { flattenLeaves } from "../views/resolver";
import type { ViewsStore } from "../views/store";
import {
  SortableContainerNode,
  SortableLeafNode,
  type LeafFocusBinding,
  type SidebarTreeContext,
} from "./SidebarTreeNode";
import { useFocusGroup } from "../nav/focus";
import { createSignal } from "solid-js";
import SidebarMigrationBanner from "../components/SidebarMigrationBanner";

/// Which top-level surface the main pane is showing. `all` and `view-node`
/// are library views (filtered or not); `library-manager` and `cores` are
/// the two routed full pages. Per-system settings live in dialogs now,
/// not a page — there is no longer a deep-link tab discriminant.
///
/// `view-node` is the PR-β fold of the old `system` variant — navigation
/// is a pointer into the active view's tree (`viewId` + `nodeId`), not
/// a bare SystemId. The runtime resolves filterable SystemIds via
/// `resolveNodeSystemIds` (views/resolver.ts).
export type SidebarView =
  | { kind: "all" }
  | { kind: "view-node"; viewId: string; nodeId: string }
  | { kind: "library-manager" }
  | { kind: "cores" };

type Props = {
  layout: LayoutStore;
  library: LibraryStore;
  views: ViewsStore;
  currentView: SidebarView;
  onNavigate: (view: SidebarView) => void;
  /// Right-click on a leaf platform row opens the SystemContextMenu
  /// anchored at the click coords. Container right-click goes to
  /// `onContainerContext` instead — only "Hide from sidebar" lives
  /// there in v1.
  onSystemContext?: (id: SystemId, position: { x: number; y: number }) => void;
  onContainerContext?: (container: ContainerNode, position: { x: number; y: number }) => void;
  /// Override the "left-sidebar" focus group's DPad edge-spillover
  /// neighbours. Defaults to `right: "library-grid"` for the legacy
  /// Shell; Retroverse LIBRARY page overrides to point at its center
  /// region group id so DPad-RIGHT at the sidebar's right edge spills
  /// to the Retroverse center pane (which then delegates to the grid
  /// via LibraryPage's effect).
  focusGroupNeighbours?: { left?: string; right?: string };
};

/**
 * Left sidebar — primary navigation surface. Sections (top → bottom):
 *
 *   Quick destinations — Home / All Games / Favorites / Recent / Continue.
 *     Pinned to the top, not reorderable. For Phase 2.5 only "All" navigates;
 *     others are placeholders until the feature lands.
 *
 *   Platforms — the tiered tree (PR-γ). Containers carry twisties + recursive
 *     count badges; leaves carry the per-system accent + own count. Cascade
 *     auto-hide-empty applies bottom-up. Collapsed-mode sidebar falls back
 *     to a flat icon-only render of every visible leaf (no container chrome
 *     fits in the icon column).
 *
 *   Playlists — section header + create-button. No items yet.
 *   Smart Views — section header + create-button. No items yet.
 *
 * Width is driven by the parent Shell's CSS grid via
 * `--layout-left-sidebar-width` (or the collapsed token). The resizer
 * handle on the right edge writes back to `layout.setLeftSidebarWidth()`.
 *
 * Drag-reorder is intentionally absent in γ.1 (the tree render) and lands
 * via solid-dnd in γ.2. Migration banner is γ.4; per-node hide UX is γ.3.
 */
const LeftSidebar: Component<Props> = (props) => {
  const isCollapsed = () => props.layout.leftSidebarCollapsed();

  /// SystemId driving the active-row highlight + always-visible-while-
  /// active filter rule. Mirrors viewToSystemId in App.tsx — node lookup
  /// in the active view with a synthesized-leaf fallback for deep-links
  /// pointing outside the active view's tree.
  const activeSystemId = createMemo<SystemId | null>(() => {
    const cv = props.currentView;
    if (cv.kind !== "view-node") return null;
    const view = props.views.activeView();
    if (!view || view.id !== cv.viewId) {
      return parsePlatformNodeId(cv.nodeId);
    }
    // Either matches a leaf in the tree or falls through to the synth
    // shape — same semantics as App.tsx::viewToSystemId.
    for (const leaf of flattenLeaves(view.root)) {
      if (leaf.id === cv.nodeId) return leaf.systemId;
    }
    return parsePlatformNodeId(cv.nodeId);
  });

  /// Filtered copy of the active view's root container — applies the
  /// legacy `layout.hiddenSystems` + `autoHideEmptySystems` filters
  /// bottom-up across the tree. Per-node `hidden` flags also honored
  /// (γ.3 wires up the right-click that sets them; the storage already
  /// landed in PR-β). The active leaf is preserved through hide /
  /// auto-hide gates so navigating to a hidden system doesn't make
  /// its sidebar row vanish; its ancestor containers stay visible
  /// because the recursion sees a non-null filtered child.
  const filteredRoot = createMemo<ContainerNode | null>(() => {
    const view = props.views.activeView();
    if (!view) return null;
    const hiddenSet = new Set(props.layout.hiddenSystems());
    const autoHide = props.layout.autoHideEmptySystems();
    const active = activeSystemId();
    const entries = props.library.state.entries;
    return filterTree(view.root, { hiddenSet, autoHide, active, entries }) as ContainerNode | null;
  });

  /// Collapsed-mode fallback — DFS of the filtered tree's leaves. The
  /// icon-only sidebar doesn't have room for container chrome, so we
  /// degrade to today's β-style flat render of just the visible
  /// platform leaves.
  const collapsedLeaves = createMemo<PlatformNode[]>(() => {
    const root = filteredRoot();
    if (!root) return [];
    return flattenLeaves(root);
  });

  const expandedSet = createMemo(() => {
    const view = props.views.activeView();
    return new Set(view?.expandedNodes ?? []);
  });

  const activeViewId = createMemo(() => props.views.activeView()?.id ?? "");

  /// Lookup of node id → parent container id used by the drag-end
  /// handler to gate cross-scope drops. Built from the FILTERED tree
  /// (matches what's visually present in the sortable scopes). Top-
  /// level children are mapped to the sentinel `"__root__"` so we can
  /// distinguish "top-level" from "inside a particular container."
  const parentOfId = createMemo(() => {
    const map = new Map<string, string>();
    const root = filteredRoot();
    if (!root) return map;
    for (const child of root.children) {
      map.set(child.id, "__root__");
      if (child.kind === "container") {
        for (const grand of child.children) {
          map.set(grand.id, child.id);
        }
      }
    }
    return map;
  });

  /// solid-dnd onDragEnd. Two paths:
  ///   - Same-parent drag → reorderSiblingsInPlace (γ.2 behavior,
  ///     unchanged). Writes against the UNFILTERED view tree so
  ///     hidden / auto-hide-empty siblings keep their positions.
  ///   - Cross-parent leaf drag (v2.2) → ViewsStore.moveNode. Source
  ///     leaf removed from its current container, inserted into the
  ///     target container at the drop position (before the drop leaf,
  ///     or appended if the drop target IS a container header).
  ///
  /// Cross-parent CONTAINER drag (e.g. dragging "Consoles" onto a
  /// leaf in "Handhelds") stays a silent no-op — containers don't
  /// nest in v2 default views, and dragging top-level containers
  /// across each other is meaningless at root level. v3's View
  /// Editor introduces the surface for nested containers.
  const handleSidebarDragEnd: DragEventHandler = ({ draggable, droppable }) => {
    if (!droppable || draggable.id === droppable.id) return;
    const dragId = String(draggable.id);
    const dropId = String(droppable.id);
    const dragParent = parentOfId().get(dragId);
    const dropParent = parentOfId().get(dropId);
    if (!dragParent) return;

    const view = props.views.activeView();
    if (!view) return;

    if (dragParent === dropParent) {
      // Same-parent reorder — γ.2 path.
      if (dragParent === "__root__") {
        reorderSiblingsInPlace(view.root.id, view.root.children, dragId, dropId, props.views);
      } else {
        const parent = findContainerById(view.root, dragParent);
        if (!parent) return;
        reorderSiblingsInPlace(parent.id, parent.children, dragId, dropId, props.views);
      }
      return;
    }

    // Cross-parent drag. Only leaf drags supported in v2.2 — container
    // drags across scopes silently no-op (no container-nesting in v2).
    const dragNode = findNode(view.root, dragId);
    const isLeafDrag = dragNode && "kind" in dragNode && dragNode.kind === "platform";
    if (!isLeafDrag) return;

    // Determine target container + insertion anchor based on what was
    // dropped on.
    const dropNode = findNode(view.root, dropId);
    let targetParentId: string;
    let insertBeforeId: string | null;
    if (dropNode && "children" in dropNode) {
      // Dropped on a container header → append to that container.
      targetParentId = dropId;
      insertBeforeId = null;
    } else if (dropParent && dropParent !== "__root__") {
      // Dropped on a leaf in a different container → insert at that
      // leaf's position (drop leaf shifts down).
      targetParentId = dropParent;
      insertBeforeId = dropId;
    } else {
      // Drop target's parent is root (unexpected for a leaf drop);
      // bail rather than guess.
      return;
    }

    props.views.moveNode(dragId, targetParentId, insertBeforeId);
  };

  const topLevelIds = createMemo(() => filteredRoot()?.children.map((c) => c.id) ?? []);

  // ── Controller-nav focus group ─────────────────────────────────────
  //
  // Flat nav list = "All Games" quick destination + every visible
  // top-level item (containers + leaves). Containers walk depth-first:
  // a container appears in the list followed by its children IF the
  // container is expanded; collapsed containers contribute only their
  // own row. The R1 bumper transfers focus to the library grid.
  type SidebarNavItem =
    | { kind: "all" }
    | { kind: "container"; nodeId: string; container: ContainerNode }
    | { kind: "leaf"; nodeId: string; systemId: SystemId };

  /// DFS the filtered tree, emitting containers + leaves in render
  /// order, skipping the subtree of any collapsed container. The
  /// `isExpanded` check matches the same source-of-truth the tree
  /// renderer uses (treeContext.isExpanded, but inlined here so the
  /// memo's deps are explicit + the lookup happens against the live
  /// views store).
  const navItems = createMemo<SidebarNavItem[]>(() => {
    const items: SidebarNavItem[] = [{ kind: "all" }];
    const root = filteredRoot();
    if (!root) return items;
    const expanded = expandedSet();
    const activeLeafSysId = activeSystemId();
    const isContainerExpanded = (n: ContainerNode): boolean => {
      if (expanded.has(n.id)) return true;
      // Auto-expand containers holding the active leaf (matches the
      // tree-render auto-expand behaviour so focus + render stay in
      // sync).
      return activeLeafSysId !== null && containerContainsSystem(n, activeLeafSysId);
    };
    const walk = (node: ContainerNode | ViewNode): void => {
      if ("kind" in node && node.kind === "platform") {
        items.push({ kind: "leaf", nodeId: node.id, systemId: node.systemId });
        return;
      }
      const c = node as ContainerNode;
      items.push({ kind: "container", nodeId: c.id, container: c });
      if (isContainerExpanded(c)) {
        for (const child of c.children) walk(child);
      }
    };
    for (const child of root.children) walk(child);
    return items;
  });

  const [focusedSidebarIndex, setFocusedSidebarIndex] = createSignal(0);

  // Pre-bind a registry: index by leaf nodeId for the tree's bind
  // callback, plus a separate ref slot for the All Games button. The
  // focus.ts manager calls `.focus()` + `.scrollIntoView()` on bound
  // elements when index changes.
  const navElements = new Map<number, HTMLElement>();

  const sidebarFocus = useFocusGroup({
    id: "left-sidebar",
    orientation: "vertical",
    itemCount: () => navItems().length,
    focusedIndex: focusedSidebarIndex,
    setFocusedIndex: setFocusedSidebarIndex,
    onActivate: (i) => {
      const item = navItems()[i];
      if (!item) return;
      if (item.kind === "all") {
        props.onNavigate({ kind: "all" });
      } else if (item.kind === "leaf") {
        props.onNavigate({
          kind: "view-node",
          viewId: activeViewId(),
          nodeId: item.nodeId,
        });
      } else {
        // Container: navigate to its view-node (matches mouse click on
        // the label area). Twisty-style expand/collapse is the L/R
        // direction handler's job.
        props.onNavigate({
          kind: "view-node",
          viewId: activeViewId(),
          nodeId: item.nodeId,
        });
      }
    },
    onSecondary: (i) => {
      const item = navItems()[i];
      if (!item) return;
      const el = navElements.get(i);
      if (!el) return;
      const r = el.getBoundingClientRect();
      const pos = { x: r.right - 16, y: r.top + r.height / 2 };
      if (item.kind === "leaf") {
        props.onSystemContext?.(item.systemId, pos);
      } else if (item.kind === "container") {
        props.onContainerContext?.(item.container, pos);
      }
    },
    onDirection: (direction, currentIndex) => {
      const item = navItems()[currentIndex];
      if (!item || item.kind !== "container") return false;
      if (direction === "left") {
        // Collapse if expanded; no-op if already collapsed (the tree
        // has no parent-container concept in v1).
        if (expandedSet().has(item.nodeId)) {
          props.views.toggleExpanded(item.nodeId);
        }
        return true;
      }
      if (direction === "right") {
        // Expand if collapsed; if already expanded, consume the event
        // so the new edge-spillover doesn't yank focus out of the
        // sidebar (regression flagged in the controller-pipeline
        // audit). Operator can press DPad-RIGHT once on a collapsed
        // container to expand, then again on a child to walk in;
        // pressing RIGHT on an already-expanded container is now a
        // no-op rather than a spill.
        if (!expandedSet().has(item.nodeId)) {
          props.views.toggleExpanded(item.nodeId);
        }
        return true;
      }
      return false;
    },
    neighbours: {
      left: props.focusGroupNeighbours?.left,
      right: props.focusGroupNeighbours?.right ?? "library-grid",
    },
  });

  // Reverse lookup: nodeId → index in navItems ("all" is 0; containers
  // and leaves get their flat index in DFS order). Used by both the
  // tree's leaf bind callback and the container row bind callback.
  const nodeIndexById = createMemo(() => {
    const m = new Map<string, number>();
    navItems().forEach((it, i) => {
      if (it.kind === "leaf" || it.kind === "container") m.set(it.nodeId, i);
    });
    return m;
  });

  const focusBindingFor = (nodeId: string): LeafFocusBinding | null => {
    const i = nodeIndexById().get(nodeId);
    if (i === undefined) return null;
    return {
      bind: (el) => {
        if (el) navElements.set(i, el);
        else navElements.delete(i);
        sidebarFocus.bind(i, el);
      },
      focused: () => focusedSidebarIndex() === i,
      active: () => sidebarFocus.isActive(),
    };
  };

  const treeContext: SidebarTreeContext = {
    get entries() {
      return props.library.state.entries;
    },
    isExpanded: (nodeId: string) => {
      if (expandedSet().has(nodeId)) return true;
      // Auto-expand any container that contains the active leaf — keeps
      // the highlighted row visible without mutating the persisted
      // expandedNodes set (the operator's explicit collapse choice
      // survives a navigation).
      const sysId = activeSystemId();
      if (!sysId) return false;
      const view = props.views.activeView();
      if (!view) return false;
      const node = findNode(view.root, nodeId);
      if (!node || !("children" in node)) return false;
      return containerContainsSystem(node, sysId);
    },
    isActiveNode: (nodeId: string) => {
      const cv = props.currentView;
      return (
        cv.kind === "view-node" &&
        cv.viewId === activeViewId() &&
        cv.nodeId === nodeId
      );
    },
    onToggleExpanded: (nodeId: string) => props.views.toggleExpanded(nodeId),
    onNavigateToNode: (nodeId: string) =>
      props.onNavigate({ kind: "view-node", viewId: activeViewId(), nodeId }),
    onLeafContextMenu: (systemId, position) => props.onSystemContext?.(systemId, position),
    onContainerContextMenu: (container, position) => props.onContainerContext?.(container, position),
    focusBindingFor,
  };

  const totalCount = createMemo(() =>
    props.library.state.entries.filter((e) => !e.seed).length,
  );

  const isActive = (view: SidebarView): boolean => {
    const cv = props.currentView;
    if (cv.kind !== view.kind) return false;
    if (cv.kind === "view-node" && view.kind === "view-node") {
      return cv.viewId === view.viewId && cv.nodeId === view.nodeId;
    }
    return true;
  };

  const beginResize = (event: PointerEvent) => {
    event.preventDefault();
    const startX = event.clientX;
    const startWidth = props.layout.leftSidebarWidth();
    const min = 200;
    const max = 360;
    const onMove = (ev: PointerEvent) => {
      const next = Math.max(min, Math.min(max, startWidth + (ev.clientX - startX)));
      props.layout.setLeftSidebarWidth(next);
    };
    const onUp = () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      document.body.style.cursor = "";
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp, { once: true });
    document.body.style.cursor = "ew-resize";
  };

  return (
    <aside class="relative flex h-full flex-col border-r border-white/5 bg-black/20">
      <nav class="flex-1 overflow-y-auto overscroll-contain px-2 py-3">
        {/* Upgrade-install migration banner (only renders when active
            view is Flat-Legacy + not yet dismissed; hidden in collapsed
            sidebar mode). */}
        <SidebarMigrationBanner
          views={props.views}
          layout={props.layout}
          collapsed={isCollapsed()}
        />

        {/* Quick destinations */}
        <ul class="space-y-0.5">
          <QuickItem
            icon="▦"
            label="All Games"
            badge={totalCount() > 0 ? String(totalCount()) : undefined}
            active={isActive({ kind: "all" })}
            collapsed={isCollapsed()}
            onClick={() => props.onNavigate({ kind: "all" })}
            controllerFocus={{
              bind: (el) => {
                if (el) navElements.set(0, el);
                else navElements.delete(0);
                sidebarFocus.bind(0, el);
              },
              focused: () => focusedSidebarIndex() === 0,
              active: () => sidebarFocus.isActive(),
            }}
          />
        </ul>

        {/* Platforms — section header is a view picker dropdown when
            the operator has more than one view available (FormFactor +
            Manufacturers ship as defaults from v2.1; user-built views
            land in v3). Collapses to a thin divider in icon-only mode. */}
        <ViewPickerHeader views={props.views} collapsed={isCollapsed()} />
        <Show
          when={!isCollapsed()}
          fallback={
            <ul class="space-y-0.5">
              <For each={collapsedLeaves()}>
                {(leaf) => (
                  <CollapsedLeaf
                    leaf={leaf}
                    active={
                      props.currentView.kind === "view-node" &&
                      props.currentView.viewId === activeViewId() &&
                      props.currentView.nodeId === leaf.id
                    }
                    onClick={() =>
                      props.onNavigate({
                        kind: "view-node",
                        viewId: activeViewId(),
                        nodeId: leaf.id,
                      })
                    }
                    onContextMenu={(pos) => props.onSystemContext?.(leaf.systemId, pos)}
                    controllerFocus={focusBindingFor(leaf.id) ?? undefined}
                  />
                )}
              </For>
            </ul>
          }
        >
          <Show when={filteredRoot()}>
            {(root) => (
              <DragDropProvider
                onDragEnd={handleSidebarDragEnd}
                collisionDetector={closestCenter}
              >
                <DragDropSensors />
                <SortableProvider ids={topLevelIds()}>
                  <ul class="space-y-0.5">
                    <For each={root().children}>
                      {(child) =>
                        child.kind === "container" ? (
                          <SortableContainerNode
                            container={child as ContainerNode}
                            depth={0}
                            ctx={treeContext}
                          />
                        ) : (
                          <SortableLeafNode
                            leaf={child as PlatformNode}
                            depth={0}
                            ctx={treeContext}
                          />
                        )
                      }
                    </For>
                  </ul>
                </SortableProvider>
              </DragDropProvider>
            )}
          </Show>
        </Show>

        {/* Playlists (placeholder — feature lands later) */}
        <SectionHeader label="Playlists" collapsed={isCollapsed()} />
        <Show when={!isCollapsed()}>
          <p class="px-3 py-2 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            No playlists yet
          </p>
        </Show>

        {/* Smart Views (placeholder) */}
        <SectionHeader label="Smart Views" collapsed={isCollapsed()} />
        <Show when={!isCollapsed()}>
          <p class="px-3 py-2 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            No smart views yet
          </p>
        </Show>
      </nav>

      {/* Collapse / expand toggle pinned to bottom. Cores + Settings buttons
          previously lived here too; both routes now live elsewhere — Cores
          via Retroverse SETTINGS → Cores (or the legacy Shell's Library ▾
          → Cores Manager…), Settings via Retroverse SETTINGS or the legacy
          Shell's Settings menu. */}
      <div class="space-y-1 border-t border-white/5 p-2">
        <button
          type="button"
          onClick={(e) => {
            e.currentTarget.blur();
            props.layout.setLeftSidebarCollapsed(!isCollapsed());
          }}
          class="w-full rounded-md border border-white/10 bg-white/[0.03] px-2 py-1.5 text-[0.65rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.07] hover:text-(--color-oa-ink)"
          title={isCollapsed() ? "Expand sidebar" : "Collapse to icons"}
        >
          {isCollapsed() ? "›" : "‹ Collapse"}
        </button>
      </div>

      {/* Width resizer — right edge drag handle. Disabled when collapsed. */}
      <Show when={!isCollapsed()}>
        <div
          class="absolute right-0 top-0 z-10 h-full w-1 cursor-ew-resize hover:bg-(--color-system-accent)/40"
          onPointerDown={beginResize}
          role="separator"
          aria-orientation="vertical"
          aria-label="Resize left sidebar"
        />
      </Show>
    </aside>
  );
};

// ── Pure tree filter ─────────────────────────────────────────────────

type FilterCtx = {
  hiddenSet: Set<string>;
  autoHide: boolean;
  active: SystemId | null;
  entries: { systemId: SystemId; seed?: boolean }[];
};

/// Bottom-up filter. Returns null when the node should not render, or
/// a filtered copy when it should. Active leaf bypasses hide / auto-
/// hide gates so it stays visible after navigation; its ancestor
/// containers stay visible naturally because the recursion sees a
/// non-null filtered child.
function filterTree(node: ContainerNode | ViewNode, ctx: FilterCtx): ContainerNode | ViewNode | null {
  if ("kind" in node && node.kind === "platform") {
    const leaf = node;
    if (leaf.systemId === ctx.active) return leaf;
    if (leaf.hidden) return null;
    if (ctx.hiddenSet.has(leaf.systemId)) return null;
    if (ctx.autoHide && countEntriesForSystem(ctx.entries, leaf.systemId) === 0) return null;
    return leaf;
  }
  const container = (node as ContainerNode);
  if (container.hidden) {
    // Hidden container — but if it contains the active leaf, surface
    // the active leaf at this position so navigation doesn't strand.
    if (ctx.active && containerContainsSystem(container, ctx.active)) {
      // Recurse to find just the active branch.
      const filteredChildren = container.children
        .map((c) => filterTree(c, ctx))
        .filter((c): c is ViewNode => c !== null);
      if (filteredChildren.length > 0) {
        return { ...container, children: filteredChildren as ViewNode[] };
      }
    }
    return null;
  }
  const filteredChildren = container.children
    .map((c) => filterTree(c, ctx))
    .filter((c): c is ViewNode => c !== null);
  // Cascade auto-hide-empty: container with zero visible descendants
  // disappears when the toggle is on. Empty containers stay visible
  // when auto-hide is off so the tree shape remains visible while the
  // operator scans the library.
  if (filteredChildren.length === 0 && ctx.autoHide) return null;
  return { ...container, children: filteredChildren as ViewNode[] };
}

function countEntriesForSystem(
  entries: { systemId: SystemId; seed?: boolean }[],
  id: SystemId,
): number {
  let n = 0;
  for (const e of entries) {
    if (e.systemId === id && !e.seed) n++;
  }
  return n;
}

function containerContainsSystem(node: ContainerNode | ViewNode, id: SystemId): boolean {
  if ("kind" in node && node.kind === "platform") return node.systemId === id;
  const container = node as ContainerNode;
  for (const child of container.children) {
    if (containerContainsSystem(child, id)) return true;
  }
  return false;
}

function findNode(node: ContainerNode | ViewNode, nodeId: string): ContainerNode | ViewNode | null {
  if (node.id === nodeId) return node;
  if ("kind" in node && node.kind === "platform") return null;
  const container = node as ContainerNode;
  for (const child of container.children) {
    const inner = findNode(child, nodeId);
    if (inner) return inner;
  }
  return null;
}

/// Variant of findNode that only returns the node if it's a container
/// — used by the drag-end handler when looking up a parent by id.
function findContainerById(root: ContainerNode, id: string): ContainerNode | null {
  const found = findNode(root, id);
  if (!found || !("children" in found)) return null;
  return found as ContainerNode;
}

/// Apply a drag-end reorder against a parent's UNFILTERED children
/// list — splice the dragged node out of its source position and
/// insert it at the drop target's position. Hidden / auto-hide-empty
/// siblings naturally keep their positions because we operate on the
/// full child set.
function reorderSiblingsInPlace(
  parentId: string,
  fullChildren: ViewNode[],
  dragId: string,
  dropId: string,
  views: ViewsStore,
): void {
  const fromIdx = fullChildren.findIndex((c) => c.id === dragId);
  const toIdx = fullChildren.findIndex((c) => c.id === dropId);
  if (fromIdx < 0 || toIdx < 0) return;
  const ids = fullChildren.map((c) => c.id);
  const [moved] = ids.splice(fromIdx, 1);
  ids.splice(toIdx, 0, moved);
  views.reorderChildren(parentId, ids);
}

// Re-export so App.tsx + other callers can build view-node SidebarViews
// without reaching into views/defaults directly.
export { platformNodeIdFor };

// ── Section + Quick + CollapsedLeaf rows ────────────────────────────

const SectionHeader: Component<{ label: string; collapsed: boolean }> = (props) => (
  <Show
    when={!props.collapsed}
    fallback={<div class="my-2 h-px bg-white/5" />}
  >
    <h3 class="mt-4 mb-1 px-3 text-[0.6rem] font-semibold uppercase tracking-[0.3em] text-(--color-oa-ink-dim)">
      {props.label}
    </h3>
  </Show>
);

/// Platforms section header with an embedded view picker. When only
/// one view is registered the picker degrades to a plain header (the
/// dropdown's single option is uninteresting). With multiple views
/// (v2.1 ships two defaults: Platforms + Manufacturers; v3 adds
/// user-built views) the active view's name becomes a native select
/// whose options switch `activeViewId`. Per SIDEBAR_TIER_PLAN.md §8
/// v2 ("operator picks active via a small <select> in the sidebar
/// header").
const ViewPickerHeader: Component<{
  views: ViewsStore;
  collapsed: boolean;
}> = (props) => {
  const cfg = () => props.views.config();
  const viewList = () => cfg().views;
  const activeId = () => cfg().activeViewId;
  const activeName = () => {
    const list = viewList();
    return list.find((v) => v.id === activeId())?.name ?? list[0]?.name ?? "Platforms";
  };

  return (
    <Show
      when={!props.collapsed}
      fallback={<div class="my-2 h-px bg-white/5" />}
    >
      <Show
        when={viewList().length > 1}
        fallback={
          <h3 class="mt-4 mb-1 px-3 text-[0.6rem] font-semibold uppercase tracking-[0.3em] text-(--color-oa-ink-dim)">
            {activeName()}
          </h3>
        }
      >
        <div class="mt-4 mb-1 px-2">
          <label class="relative block">
            <select
              value={activeId()}
              onChange={(e) => {
                props.views.setActiveView(e.currentTarget.value);
                e.currentTarget.blur();
              }}
              aria-label="Active view"
              class="w-full cursor-pointer appearance-none rounded bg-transparent px-2 py-1 pr-6 text-[0.6rem] font-semibold uppercase tracking-[0.3em] text-(--color-oa-ink-dim) transition hover:bg-white/[0.04] hover:text-(--color-oa-ink) focus:outline-none focus:bg-white/[0.04] focus:text-(--color-oa-ink)"
            >
              <For each={viewList()}>
                {(view) => <option value={view.id} class="bg-(--color-oa-bg-deep) text-(--color-oa-ink) normal-case tracking-normal">{view.name}</option>}
              </For>
            </select>
            <span
              aria-hidden="true"
              class="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-[0.55rem] text-(--color-oa-ink-dim)"
            >
              ▾
            </span>
          </label>
        </div>
      </Show>
    </Show>
  );
};

const QuickItem: Component<{
  icon: string;
  label: string;
  badge?: string;
  active: boolean;
  disabled?: boolean;
  collapsed: boolean;
  onClick: () => void;
  controllerFocus?: LeafFocusBinding;
}> = (props) => (
  <li
    ref={(el) => props.controllerFocus?.bind(el)}
    data-oa-focus={props.controllerFocus?.focused() ? "true" : undefined}
    data-oa-focus-active={props.controllerFocus?.active() ? "true" : undefined}
  >
    <button
      type="button"
      data-oa-sidebar-row
      onClick={(e) => {
        e.currentTarget.blur();
        if (!props.disabled) props.onClick();
      }}
      disabled={props.disabled}
      aria-pressed={props.active}
      class="group relative flex w-full items-center gap-3 rounded-md px-3 py-2 text-left text-xs font-medium transition"
      classList={{
        "bg-white/[0.07] text-(--color-oa-ink)": props.active,
        "text-(--color-oa-ink-dim) hover:bg-white/[0.04] hover:text-(--color-oa-ink)": !props.active && !props.disabled,
        "cursor-not-allowed opacity-50": props.disabled === true,
        "justify-center px-2": props.collapsed,
      }}
      title={props.collapsed ? props.label : undefined}
    >
      <Show when={props.active}>
        <span
          class="pointer-events-none absolute left-0 top-1/2 h-5 w-0.5 -translate-y-1/2 rounded-r bg-(--color-system-accent)"
          aria-hidden="true"
        />
      </Show>
      <span class="text-sm leading-none">{props.icon}</span>
      <Show when={!props.collapsed}>
        <span class="flex-1">{props.label}</span>
        <Show when={props.badge}>
          <span class="text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            {props.badge}
          </span>
        </Show>
      </Show>
    </button>
  </li>
);

/// Icon-only fallback used when the sidebar is collapsed. No tree chrome
/// (twisties / labels don't fit in the icon column); just the per-system
/// accent dot + tooltip so the operator can navigate by hover.
const CollapsedLeaf: Component<{
  leaf: PlatformNode;
  active: boolean;
  onClick: () => void;
  onContextMenu?: (position: { x: number; y: number }) => void;
  controllerFocus?: LeafFocusBinding;
}> = (props) => {
  const theme = () => systemThemes[props.leaf.systemId];
  return (
    <li
      ref={(el) => props.controllerFocus?.bind(el)}
      data-system={props.leaf.systemId}
      data-oa-focus={props.controllerFocus?.focused() ? "true" : undefined}
      data-oa-focus-active={props.controllerFocus?.active() ? "true" : undefined}
    >
      <button
        type="button"
        data-oa-sidebar-row
        onClick={(e) => {
          e.currentTarget.blur();
          props.onClick();
        }}
        onContextMenu={(e) => {
          if (!props.onContextMenu) return;
          e.preventDefault();
          props.onContextMenu({ x: e.clientX, y: e.clientY });
        }}
        aria-pressed={props.active}
        title={theme()?.displayName}
        class="group relative flex w-full items-center justify-center rounded-md px-2 py-2 transition"
        classList={{
          "bg-(--color-system-accent)/15": props.active,
          "hover:bg-white/[0.04]": !props.active,
        }}
      >
        <Show when={props.active}>
          <span
            class="pointer-events-none absolute left-0 top-1/2 h-5 w-0.5 -translate-y-1/2 rounded-r bg-(--color-system-accent)"
            aria-hidden="true"
          />
        </Show>
        <span
          class="inline-block h-2.5 w-2.5 shrink-0 rounded-full bg-(--color-system-accent)"
          aria-hidden="true"
        />
      </button>
    </li>
  );
};

export default LeftSidebar;
