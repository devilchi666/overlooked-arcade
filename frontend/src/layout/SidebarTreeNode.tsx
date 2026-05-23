import { For, Show, createMemo, type Component } from "solid-js";
import { createSortable, SortableProvider, transformStyle } from "@thisbeyond/solid-dnd";

import { systemThemes, type SystemId } from "../themes/registry";
import type { RomEntry } from "../library/types";
import type { ContainerNode, PlatformNode } from "../views/types";
import { countGamesUnder } from "../views/resolver";

/// Recursive node renderer for the PR-γ sidebar tree, with solid-dnd
/// drag-reorder wiring. Per SIDEBAR_TIER_PLAN.md §3.1 + §3.5.
///
/// v1 tree shape is two-level (root → form-factor / legacy container
/// → platform leaves). Sortable scopes mirror that: the top-level
/// SortableProvider lives in LeftSidebar around the root.children
/// list; each container's SortableProvider lives inline around its
/// leaf children. Cross-container drag is rejected at the LeftSidebar
/// onDragEnd handler (different parent → no-op).
///
/// Click semantics per plan §0:
///   - Twisty area on a container → toggle expanded state (no
///     navigation).
///   - Label area on a container → navigate to the container's
///     view-node (LibraryView filters to the union of descendants per
///     the container's rule).
///   - Leaf row → navigate to the leaf's view-node (single-system
///     view).
///
/// Drag activators are bound to a small ⋮⋮ handle on each sortable
/// row so clicks on the row's navigation button aren't intercepted
/// by the drag sensor — same pattern LibraryManagerPage uses for its
/// folder-row list.

export type SidebarTreeContext = {
  entries: RomEntry[];
  isExpanded: (nodeId: string) => boolean;
  isActiveNode: (nodeId: string) => boolean;
  onToggleExpanded: (nodeId: string) => void;
  onNavigateToNode: (nodeId: string) => void;
  onLeafContextMenu?: (systemId: SystemId, position: { x: number; y: number }) => void;
  onContainerContextMenu?: (container: ContainerNode, position: { x: number; y: number }) => void;
};

// ── Sortable top-level container ─────────────────────────────────────

export const SortableContainerNode: Component<{
  container: ContainerNode;
  depth: number;
  ctx: SidebarTreeContext;
}> = (props) => {
  const sortable = createSortable(props.container.id);
  const expanded = createMemo(() => props.ctx.isExpanded(props.container.id));
  const leafChildren = createMemo(() =>
    props.container.children.filter((c): c is PlatformNode & { kind: "platform" } => c.kind === "platform"),
  );
  const leafIds = createMemo(() => leafChildren().map((c) => c.id));

  return (
    <li
      ref={sortable.ref}
      style={transformStyle(sortable.transform)}
      class="relative"
      classList={{
        "z-10": sortable.isActiveDraggable,
      }}
    >
      <ContainerRow
        container={props.container}
        depth={props.depth}
        expanded={expanded()}
        active={props.ctx.isActiveNode(props.container.id)}
        entries={props.ctx.entries}
        onToggleExpanded={() => props.ctx.onToggleExpanded(props.container.id)}
        onNavigate={() => props.ctx.onNavigateToNode(props.container.id)}
        onContextMenu={(pos) => props.ctx.onContainerContextMenu?.(props.container, pos)}
        dragActivators={sortable.dragActivators}
        isDragging={sortable.isActiveDraggable}
      />
      <Show when={expanded() && props.container.children.length > 0}>
        <SortableProvider ids={leafIds()}>
          <ul class="space-y-0.5">
            <For each={props.container.children}>
              {(child) => (
                <Show
                  when={child.kind === "platform"}
                  fallback={
                    // Deeper containers (v3+ shape) — non-sortable static
                    // recursion. v1 default + legacy views never reach
                    // this branch; included so v3+ trees still render
                    // even though their nested reorder lands in a
                    // later PR.
                    <StaticContainerNode
                      container={child as ContainerNode}
                      depth={props.depth + 1}
                      ctx={props.ctx}
                    />
                  }
                >
                  <SortableLeafNode
                    leaf={child as PlatformNode}
                    depth={props.depth + 1}
                    ctx={props.ctx}
                  />
                </Show>
              )}
            </For>
          </ul>
        </SortableProvider>
      </Show>
    </li>
  );
};

// ── Sortable leaf ────────────────────────────────────────────────────

export const SortableLeafNode: Component<{
  leaf: PlatformNode;
  depth: number;
  ctx: SidebarTreeContext;
}> = (props) => {
  const sortable = createSortable(props.leaf.id);
  return (
    <li
      ref={sortable.ref}
      style={transformStyle(sortable.transform)}
      data-system={props.leaf.systemId}
      class="relative"
      classList={{ "z-10": sortable.isActiveDraggable }}
    >
      <LeafRow
        leaf={props.leaf}
        depth={props.depth}
        active={props.ctx.isActiveNode(props.leaf.id)}
        entries={props.ctx.entries}
        onNavigate={() => props.ctx.onNavigateToNode(props.leaf.id)}
        onContextMenu={(pos) => props.ctx.onLeafContextMenu?.(props.leaf.systemId, pos)}
        dragActivators={sortable.dragActivators}
        isDragging={sortable.isActiveDraggable}
      />
    </li>
  );
};

// ── Static fallback for v3+ deep trees ───────────────────────────────

const StaticContainerNode: Component<{
  container: ContainerNode;
  depth: number;
  ctx: SidebarTreeContext;
}> = (props) => {
  const expanded = createMemo(() => props.ctx.isExpanded(props.container.id));
  return (
    <li class="relative">
      <ContainerRow
        container={props.container}
        depth={props.depth}
        expanded={expanded()}
        active={props.ctx.isActiveNode(props.container.id)}
        entries={props.ctx.entries}
        onToggleExpanded={() => props.ctx.onToggleExpanded(props.container.id)}
        onNavigate={() => props.ctx.onNavigateToNode(props.container.id)}
        onContextMenu={(pos) => props.ctx.onContainerContextMenu?.(props.container, pos)}
      />
      <Show when={expanded() && props.container.children.length > 0}>
        <ul class="space-y-0.5">
          <For each={props.container.children}>
            {(child) =>
              child.kind === "container" ? (
                <StaticContainerNode
                  container={child as ContainerNode}
                  depth={props.depth + 1}
                  ctx={props.ctx}
                />
              ) : (
                <li class="relative">
                  <LeafRow
                    leaf={child as PlatformNode}
                    depth={props.depth + 1}
                    active={props.ctx.isActiveNode(child.id)}
                    entries={props.ctx.entries}
                    onNavigate={() => props.ctx.onNavigateToNode(child.id)}
                    onContextMenu={(pos) =>
                      props.ctx.onLeafContextMenu?.((child as PlatformNode).systemId, pos)
                    }
                  />
                </li>
              )
            }
          </For>
        </ul>
      </Show>
    </li>
  );
};

// ── Shared row primitives ────────────────────────────────────────────

type DragActivators = ReturnType<typeof createSortable>["dragActivators"];

const ContainerRow: Component<{
  container: ContainerNode;
  depth: number;
  expanded: boolean;
  active: boolean;
  entries: RomEntry[];
  onToggleExpanded: () => void;
  onNavigate: () => void;
  onContextMenu?: (position: { x: number; y: number }) => void;
  dragActivators?: DragActivators;
  isDragging?: boolean;
}> = (props) => {
  const count = createMemo(() => countGamesUnder(props.container, props.entries));
  const indentRem = () => 0.5 + props.depth * 0.75;
  /// v3.3: per-container accent override applied as an inline
  /// `--color-system-accent` CSS variable. Cascades into the
  /// container's own active-row tint + twisty + count badge without
  /// leaking into descendant leaf rows (those carry their own
  /// `data-system` rule that resets the variable for their subtree).
  const rowStyle = () => {
    const base: Record<string, string> = { "padding-left": `${indentRem()}rem` };
    if (props.container.accent) {
      base["--color-system-accent"] = props.container.accent;
    }
    return base;
  };
  return (
    <div
      class="group relative flex w-full items-center gap-1 rounded-md pr-1 text-left text-xs font-medium transition"
      classList={{
        "bg-white/[0.07] text-(--color-oa-ink)": props.active,
        "text-(--color-oa-ink-dim) hover:bg-white/[0.04] hover:text-(--color-oa-ink)":
          !props.active && !props.isDragging,
        "bg-(--color-system-accent)/10 shadow-lg": props.isDragging === true,
      }}
      style={rowStyle()}
    >
      <Show when={props.active}>
        <span
          class="pointer-events-none absolute left-0 top-1/2 h-5 w-0.5 -translate-y-1/2 rounded-r bg-(--color-system-accent)"
          aria-hidden="true"
        />
      </Show>
      <button
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          e.currentTarget.blur();
          props.onToggleExpanded();
        }}
        aria-label={props.expanded ? "Collapse" : "Expand"}
        aria-expanded={props.expanded}
        class="grid h-5 w-5 shrink-0 place-items-center rounded text-[0.6rem] text-(--color-oa-ink-dim) transition hover:bg-white/[0.06] hover:text-(--color-oa-ink)"
      >
        {props.expanded ? "▾" : "▸"}
      </button>
      <button
        type="button"
        onClick={(e) => {
          e.currentTarget.blur();
          props.onNavigate();
        }}
        onContextMenu={(e) => {
          if (!props.onContextMenu) return;
          e.preventDefault();
          props.onContextMenu({ x: e.clientX, y: e.clientY });
        }}
        aria-pressed={props.active}
        class="flex flex-1 items-center gap-2 py-1.5 text-left"
      >
        <span class="flex-1 truncate">{props.container.label}</span>
        <Show when={count() > 0}>
          <span class="text-[0.6rem] tabular-nums uppercase tracking-widest text-(--color-oa-ink-dim)">
            {count()}
          </span>
        </Show>
      </button>
      <Show when={props.dragActivators}>
        <span
          class="select-none px-1 text-[0.7rem] text-(--color-oa-ink-dim) opacity-0 transition group-hover:opacity-100"
          classList={{
            "cursor-grab": !props.isDragging,
            "cursor-grabbing opacity-100": props.isDragging === true,
          }}
          role="button"
          tabindex="-1"
          aria-label={`Drag handle for ${props.container.label}`}
          {...props.dragActivators!}
        >
          ⋮⋮
        </span>
      </Show>
    </div>
  );
};

const LeafRow: Component<{
  leaf: PlatformNode;
  depth: number;
  active: boolean;
  entries: RomEntry[];
  onNavigate: () => void;
  onContextMenu?: (position: { x: number; y: number }) => void;
  dragActivators?: DragActivators;
  isDragging?: boolean;
}> = (props) => {
  const theme = () => systemThemes[props.leaf.systemId];
  const count = createMemo(
    () => props.entries.filter((e) => e.systemId === props.leaf.systemId && !e.seed).length,
  );
  const indentRem = () => 0.5 + props.depth * 0.75;

  return (
    <div
      class="group relative flex w-full items-center gap-2 rounded-md py-2 pr-1 text-left text-xs font-medium transition"
      classList={{
        "bg-(--color-system-accent)/15 text-(--color-oa-ink)": props.active,
        "text-(--color-oa-ink-dim) hover:bg-white/[0.04] hover:text-(--color-oa-ink)":
          !props.active && !props.isDragging,
        "bg-(--color-system-accent)/10 shadow-lg": props.isDragging === true,
      }}
      style={{ "padding-left": `${indentRem()}rem` }}
    >
      <Show when={props.active}>
        <span
          class="pointer-events-none absolute left-0 top-1/2 h-5 w-0.5 -translate-y-1/2 rounded-r bg-(--color-system-accent)"
          aria-hidden="true"
        />
      </Show>
      <button
        type="button"
        onClick={(e) => {
          e.currentTarget.blur();
          props.onNavigate();
        }}
        onContextMenu={(e) => {
          if (!props.onContextMenu) return;
          e.preventDefault();
          props.onContextMenu({ x: e.clientX, y: e.clientY });
        }}
        aria-pressed={props.active}
        title={theme()?.displayName}
        class="flex flex-1 items-center gap-2 text-left"
      >
        <span
          class="inline-block h-2 w-2 shrink-0 rounded-full bg-(--color-system-accent)"
          aria-hidden="true"
        />
        <span class="flex-1 truncate">{theme()?.shortName ?? props.leaf.systemId}</span>
        <Show when={count() > 0}>
          <span class="text-[0.6rem] tabular-nums uppercase tracking-widest text-(--color-oa-ink-dim)">
            {count()}
          </span>
        </Show>
      </button>
      <Show when={props.dragActivators}>
        <span
          class="select-none px-1 text-[0.7rem] text-(--color-oa-ink-dim) opacity-0 transition group-hover:opacity-100"
          classList={{
            "cursor-grab": !props.isDragging,
            "cursor-grabbing opacity-100": props.isDragging === true,
          }}
          role="button"
          tabindex="-1"
          aria-label={`Drag handle for ${theme()?.displayName ?? props.leaf.systemId}`}
          {...props.dragActivators!}
        >
          ⋮⋮
        </span>
      </Show>
    </div>
  );
};
