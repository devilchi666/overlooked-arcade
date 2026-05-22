import { For, Show, createMemo, createSignal, type Component } from "solid-js";
import type { LibraryStore } from "../library/store";
import { systemThemes, type SystemId } from "../themes/registry";
import type { LayoutStore } from "./state";
import type { PlatformNode, ContainerNode, ViewNode } from "../views/types";
import { parsePlatformNodeId, platformNodeIdFor } from "../views/defaults";
import type { ViewsStore } from "../views/store";

/// Which top-level surface the main pane is showing. `all` and `view-node`
/// are library views (filtered or not); `library-manager` and `cores` are
/// the two routed full pages. Per-system settings live in dialogs now,
/// not a page — there is no longer a deep-link tab discriminant.
///
/// `view-node` replaces the old `system` variant (PR-β fold): navigation
/// is now a pointer into the active view's tree (`viewId` + `nodeId`),
/// not a bare SystemId. The runtime resolves filterable SystemIds via
/// `resolveNodeSystemIds` (views/resolver.ts). The flat-system
/// discriminant is removed, not aliased — TypeScript catches any missed
/// call site.
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
  /// Right-click on a system entry opens the SystemContextMenu anchored
  /// at the click coords. This is the primary route to per-system
  /// settings dialogs from the sidebar (the System ▾ menu bar entry is
  /// the other; GridControls and SystemHeader no longer carry ⚙).
  onSystemContext?: (id: SystemId, position: { x: number; y: number }) => void;
};

const DRAG_MIME = "application/x-oa-system";

/// PR-β flat render — walks the active view's tree and collects every
/// PlatformNode together with the id of the container that holds it.
/// Drag-reorder uses `parentId` to decide whether a drop should be
/// committed (same-parent reorders via ViewsStore.reorderChildren;
/// different-parent drops are silent no-ops — cross-container drag
/// lands in PR-γ).
type FlatLeaf = { leaf: PlatformNode; parentId: string };

function collectFlatLeaves(root: ContainerNode): FlatLeaf[] {
  const out: FlatLeaf[] = [];
  function walk(container: ContainerNode): void {
    for (const child of container.children) {
      if (child.kind === "platform") {
        out.push({ leaf: child, parentId: container.id });
      } else if (child.kind === "container") {
        walk(child);
      }
    }
  }
  walk(root);
  return out;
}

/**
 * Left sidebar — primary navigation surface. Sections (top → bottom):
 *
 *   Quick destinations — Home / All Games / Favorites / Recent / Continue.
 *     Pinned to the top, not reorderable. For Phase 2.5 only "All" navigates;
 *     others are placeholders until the feature lands.
 *
 *   Systems — one entry per platform leaf in the active view's tree.
 *     PR-β renders this flat (DFS-flattened active view leaves); PR-γ
 *     replaces the flat render with the recursive tree per
 *     SIDEBAR_TIER_PLAN.md §2.5 → §3.1.
 *
 *   Playlists — section header + create-button. No items yet.
 *   Smart Views — section header + create-button. No items yet.
 *
 * Width is driven by the parent Shell's CSS grid via `--layout-left-sidebar-width`
 * (or the collapsed token). The resizer handle on the right edge writes back
 * to `layout.setLeftSidebarWidth()`.
 */
const LeftSidebar: Component<Props> = (props) => {
  const isCollapsed = () => props.layout.leftSidebarCollapsed();

  /// The currently-selected platform leaf, if any. Drives the
  /// always-visible-while-active filter rule and the per-row active
  /// highlight. Resolution mirrors viewToSystemId in App.tsx — node
  /// lookup with a synthesized-leaf fallback for deep-links pointing
  /// outside the active view's tree.
  const activeSystemId = createMemo<SystemId | null>(() => {
    const cv = props.currentView;
    if (cv.kind !== "view-node") return null;
    const view = props.views.activeView();
    if (!view || view.id !== cv.viewId) {
      return parsePlatformNodeId(cv.nodeId);
    }
    for (const { leaf } of allLeaves()) {
      if (leaf.id === cv.nodeId) return leaf.systemId;
    }
    return parsePlatformNodeId(cv.nodeId);
  });

  /// Every platform leaf across the active view, with its container
  /// parent id (used by drag-reorder for same-parent gating).
  const allLeaves = createMemo<FlatLeaf[]>(() => {
    const view = props.views.activeView();
    if (!view) return [];
    return collectFlatLeaves(view.root);
  });

  const countForSystem = (id: SystemId): number =>
    props.library.state.entries.filter((e) => e.systemId === id && !e.seed).length;

  /// Apply the legacy layout.hiddenSystems + autoHideEmptySystems
  /// filters on top of the active view's leaves. PR-γ migrates hide
  /// state onto per-node `hidden` flags; for PR-β the legacy
  /// SystemId-keyed lists stay authoritative so behavior matches
  /// today exactly.
  const visibleLeaves = createMemo<FlatLeaf[]>(() => {
    const hidden = new Set(props.layout.hiddenSystems());
    const autoHide = props.layout.autoHideEmptySystems();
    const active = activeSystemId();
    return allLeaves().filter(({ leaf }) => {
      if (leaf.hidden) return false;
      if (leaf.systemId === active) return true;
      if (hidden.has(leaf.systemId)) return false;
      if (autoHide && countForSystem(leaf.systemId) === 0) return false;
      return true;
    });
  });

  const totalCount = createMemo(() => props.library.state.entries.filter((e) => !e.seed).length);

  // Drag state — tracks which row index the user is dragging plus the
  // current drop-target index for the visual indicator.
  const [dragSourceIndex, setDragSourceIndex] = createSignal<number | null>(null);
  const [dragOverIndex, setDragOverIndex] = createSignal<number | null>(null);

  function commitReorder(from: number, to: number) {
    if (from === to) return;
    const leaves = visibleLeaves();
    const source = leaves[from];
    if (!source) return;
    // Adjusted target index (splice removes source first; dropping
    // after the original position shifts the target by one).
    const insertAt = to > from ? to - 1 : to;
    const targetSibling = leaves[insertAt];
    // Cross-container drops are silent no-ops in PR-β. The drop
    // completes visually (drag state clears in onDrop) but the tree
    // is unchanged. Cross-container drag-reorder lands in PR-γ /
    // post-v1 per SIDEBAR_TIER_PLAN.md §0.
    if (targetSibling && targetSibling.parentId !== source.parentId) return;

    // Build the new order within the source's parent container.
    const parentId = source.parentId;
    const parentLeaves = leaves.filter((l) => l.parentId === parentId);
    const localFrom = parentLeaves.findIndex((l) => l.leaf.id === source.leaf.id);
    const localInsertAt = (() => {
      if (!targetSibling) {
        // Dropped past the last visible leaf — append within parent.
        return parentLeaves.length;
      }
      const idx = parentLeaves.findIndex((l) => l.leaf.id === targetSibling.leaf.id);
      return idx >= 0 ? idx : parentLeaves.length;
    })();
    if (localFrom < 0) return;
    const reordered = parentLeaves.slice();
    const [moved] = reordered.splice(localFrom, 1);
    const adjustedInsertAt = localInsertAt > localFrom ? localInsertAt - 1 : localInsertAt;
    reordered.splice(adjustedInsertAt, 0, moved);
    // ViewsStore.reorderChildren operates on the full ordered list of
    // children ids — but our visible-leaves filter may have dropped
    // some siblings (hidden / auto-hide-empty). Fetch the parent's
    // full child set and stitch the reorder into it.
    const view = props.views.activeView();
    if (!view) return;
    const parentChildren = findContainerChildren(view.root, parentId);
    if (!parentChildren) return;
    const reorderedIds = new Set(reordered.map((l) => l.leaf.id));
    const reorderQueue = reordered.map((l) => l.leaf.id);
    const newOrder: string[] = [];
    for (const child of parentChildren) {
      if (child.kind === "platform" && reorderedIds.has(child.id)) {
        const nextId = reorderQueue.shift();
        if (nextId !== undefined) newOrder.push(nextId);
      } else {
        newOrder.push(child.id);
      }
    }
    props.views.reorderChildren(parentId, newOrder);
  }

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

  const activeViewId = createMemo(() => props.views.activeView()?.id ?? "");

  return (
    <aside class="relative flex h-full flex-col border-r border-white/5 bg-black/20">
      <nav class="flex-1 overflow-y-auto overscroll-contain px-2 py-3">
        {/* Quick destinations */}
        <ul class="space-y-0.5">
          <QuickItem icon="▦" label="All Games" badge={totalCount() > 0 ? String(totalCount()) : undefined} active={isActive({ kind: "all" })} collapsed={isCollapsed()} onClick={() => props.onNavigate({ kind: "all" })} />
        </ul>

        {/* Systems */}
        <SectionHeader label="Systems" collapsed={isCollapsed()} />
        <ul class="space-y-0.5">
          <For each={visibleLeaves()}>
            {({ leaf }, index) => (
              <SystemItem
                id={leaf.systemId}
                count={countForSystem(leaf.systemId)}
                active={isActive({ kind: "view-node", viewId: activeViewId(), nodeId: leaf.id })}
                collapsed={isCollapsed()}
                draggable={!isCollapsed()}
                isDraggingThis={dragSourceIndex() === index()}
                dropIndicatorAbove={dragOverIndex() === index() && dragSourceIndex() !== null && dragSourceIndex() !== index()}
                onClick={() => props.onNavigate({ kind: "view-node", viewId: activeViewId(), nodeId: leaf.id })}
                onContextMenu={(pos) => props.onSystemContext?.(leaf.systemId, pos)}
                onDragStart={(ev) => {
                  setDragSourceIndex(index());
                  ev.dataTransfer?.setData(DRAG_MIME, String(index()));
                  if (ev.dataTransfer) ev.dataTransfer.effectAllowed = "move";
                }}
                onDragOver={(ev) => {
                  if (dragSourceIndex() === null) return;
                  ev.preventDefault();
                  if (ev.dataTransfer) ev.dataTransfer.dropEffect = "move";
                  // Decide whether the cursor is in the top or bottom half —
                  // dropping in the top half inserts above; bottom half below.
                  const rect = (ev.currentTarget as HTMLElement).getBoundingClientRect();
                  const above = ev.clientY < rect.top + rect.height / 2;
                  setDragOverIndex(above ? index() : index() + 1);
                }}
                onDragLeave={() => {
                  // Only clear if no other item set us as the target since.
                  if (dragOverIndex() === index() || dragOverIndex() === index() + 1) {
                    // Defer clearing — the next dragOver on a sibling will
                    // overwrite us. Clearing here causes flicker.
                  }
                }}
                onDrop={(ev) => {
                  ev.preventDefault();
                  const from = dragSourceIndex();
                  const to = dragOverIndex();
                  setDragSourceIndex(null);
                  setDragOverIndex(null);
                  if (from === null || to === null) return;
                  commitReorder(from, to);
                }}
                onDragEnd={() => {
                  setDragSourceIndex(null);
                  setDragOverIndex(null);
                }}
              />
            )}
          </For>
          {/* Final drop zone — lets the user drop past the last item to
              append at the end. Only renders while a drag is in progress. */}
          <Show when={dragSourceIndex() !== null}>
            <li
              class="h-1.5 rounded transition"
              classList={{
                "bg-(--color-system-accent)": dragOverIndex() === visibleLeaves().length,
              }}
              onDragOver={(ev) => {
                ev.preventDefault();
                setDragOverIndex(visibleLeaves().length);
              }}
              onDrop={(ev) => {
                ev.preventDefault();
                const from = dragSourceIndex();
                setDragSourceIndex(null);
                setDragOverIndex(null);
                if (from === null) return;
                commitReorder(from, visibleLeaves().length);
              }}
            />
          </Show>
        </ul>

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
          previously lived here too; both routes are now reached from the
          menu bar (Library ▾ → Library Manager… / Cores Manager…). */}
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

/// DFS lookup for a container's direct children. Used by the drag-reorder
/// path to stitch a visible-leaf reorder back into the container's full
/// child set (since hidden / auto-hide-empty filtering can drop siblings
/// from the visible list).
function findContainerChildren(node: ContainerNode, containerId: string): ViewNode[] | null {
  if (node.id === containerId) return node.children;
  for (const child of node.children) {
    if (child.kind === "container") {
      const inner = findContainerChildren(child, containerId);
      if (inner) return inner;
    }
  }
  return null;
}

// `platformNodeIdFor` is imported for completeness — used by the App.tsx
// helpers that pair with this file. Re-exported so callers can build
// view-node SidebarViews without reaching into views/defaults directly.
export { platformNodeIdFor };

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

const QuickItem: Component<{
  icon: string;
  label: string;
  badge?: string;
  active: boolean;
  disabled?: boolean;
  collapsed: boolean;
  onClick: () => void;
}> = (props) => (
  <li>
    <button
      type="button"
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

const SystemItem: Component<{
  id: SystemId;
  count: number;
  active: boolean;
  collapsed: boolean;
  draggable: boolean;
  isDraggingThis: boolean;
  dropIndicatorAbove: boolean;
  onClick: () => void;
  onContextMenu?: (position: { x: number; y: number }) => void;
  onDragStart: (ev: DragEvent) => void;
  onDragOver: (ev: DragEvent) => void;
  onDragLeave: (ev: DragEvent) => void;
  onDrop: (ev: DragEvent) => void;
  onDragEnd: (ev: DragEvent) => void;
}> = (props) => {
  const theme = () => systemThemes[props.id];
  return (
    <li
      data-system={props.id}
      class="relative"
      classList={{ "opacity-40": props.isDraggingThis }}
      draggable={props.draggable}
      onDragStart={props.onDragStart}
      onDragOver={props.onDragOver}
      onDragLeave={props.onDragLeave}
      onDrop={props.onDrop}
      onDragEnd={props.onDragEnd}
    >
      <Show when={props.dropIndicatorAbove}>
        <span
          class="pointer-events-none absolute -top-0.5 left-2 right-2 h-0.5 rounded bg-(--color-system-accent)"
          aria-hidden="true"
        />
      </Show>
      <button
        type="button"
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
        class="group relative flex w-full items-center gap-3 rounded-md px-3 py-2 text-left text-xs font-medium transition"
        classList={{
          "bg-(--color-system-accent)/15 text-(--color-oa-ink)": props.active,
          "text-(--color-oa-ink-dim) hover:bg-white/[0.04] hover:text-(--color-oa-ink)": !props.active,
          "justify-center px-2": props.collapsed,
        }}
        title={props.collapsed ? theme().displayName : undefined}
      >
        <Show when={props.active}>
          <span
            class="pointer-events-none absolute left-0 top-1/2 h-5 w-0.5 -translate-y-1/2 rounded-r bg-(--color-system-accent)"
            aria-hidden="true"
          />
        </Show>
        <span
          class="inline-block h-2 w-2 shrink-0 rounded-full bg-(--color-system-accent)"
          aria-hidden="true"
        />
        <Show when={!props.collapsed}>
          <span class="flex-1 truncate">{theme().displayName}</span>
          <Show when={props.count > 0}>
            <span class="text-[0.6rem] tabular-nums uppercase tracking-widest text-(--color-oa-ink-dim)">
              {props.count}
            </span>
          </Show>
        </Show>
      </button>
    </li>
  );
};

export default LeftSidebar;
