import { For, Show, createMemo, type Component } from "solid-js";
import { createSortable, SortableProvider, transformStyle } from "@thisbeyond/solid-dnd";

import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import type { RomEntry } from "@oa/platform/library/types";
import type { ContainerNode, PlatformNode } from "@oa/platform/views/types";
import { countGamesUnder, isGameSetNode } from "@oa/platform/views/resolver";
import { asSmartListKind, SMART_LISTS_BY_KIND } from "@oa/platform/library/smartLists";

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
  /// Unified Navigation Tree Slice 1 — membership count for a game-set node
  /// (collection / smart-list filter). Returns the game count, or null /
  /// undefined when the host doesn't wire counts. System containers + leaves
  /// keep using the entries-based `countGamesUnder`; this is the
  /// membership-based count host (LeftSidebar / ViewEditorPane) supplies for
  /// nodes whose contents are games, not child leaves.
  gameSetCount?: (node: ContainerNode) => number;
  /// Controller-nav binding for a given leaf id. Returns a small handle
  /// the leaf row uses to (a) register its DOM element with the focus
  /// manager and (b) reactively expose its focus state via data attrs.
  /// Containers don't bind in v1 — DPad nav skips them and lands on
  /// the next visible leaf.
  focusBindingFor?: (nodeId: string) => LeafFocusBinding | null;
  /// Unified Navigation Tree Slice 2 — opt-in deep-drag. When true, nested
  /// containers (and game-set nodes nested inside them) render as recursive
  /// *sortable* rows and each container's drag-scope covers all its children,
  /// so a node can be dragged into / out of folders at any depth. The View
  /// Editor opts in; the live sidebar leaves it off (default) and keeps its
  /// leaf-only reorder behaviour byte-for-byte.
  nestedSortable?: boolean;
};

export type LeafFocusBinding = {
  bind: (el: HTMLElement | null) => void;
  focused: () => boolean;
  active: () => boolean;
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
  // Slice 2: with deep-drag on, the sortable scope spans every child
  // (containers + game-sets + leaves) so nested nodes are draggable; off,
  // it stays leaf-only (the live sidebar's unchanged behaviour).
  const nestedSortable = createMemo(() => props.ctx.nestedSortable ?? false);
  const sortableChildIds = createMemo(() =>
    nestedSortable() ? props.container.children.map((c) => c.id) : leafIds(),
  );
  const focusBinding = createMemo(() => props.ctx.focusBindingFor?.(props.container.id) ?? null);
  const isGameSet = createMemo(() => isGameSetNode(props.container));

  // Game-set nodes (collection / smart-list filter) render leaf-style — no
  // twisty, no child recursion — but stay sortable + focusable like any other
  // top-level row. Their "contents" are games resolved via membership, not
  // child leaves. (Unified Navigation Tree Slice 1.)
  return (
    <Show
      when={!isGameSet()}
      fallback={
        <li
          ref={(el) => {
            sortable.ref(el);
            focusBinding()?.bind(el);
          }}
          style={transformStyle(sortable.transform)}
          data-oa-focus={focusBinding()?.focused() ? "true" : undefined}
          data-oa-focus-active={focusBinding()?.active() ? "true" : undefined}
          class="relative"
          classList={{ "z-10": sortable.isActiveDraggable }}
        >
          <GameSetRow
            container={props.container}
            depth={props.depth}
            active={props.ctx.isActiveNode(props.container.id)}
            count={props.ctx.gameSetCount?.(props.container)}
            onNavigate={() => props.ctx.onNavigateToNode(props.container.id)}
            onContextMenu={(pos) => props.ctx.onContainerContextMenu?.(props.container, pos)}
            dragActivators={sortable.dragActivators}
            isDragging={sortable.isActiveDraggable}
          />
        </li>
      }
    >
    <li
      ref={(el) => {
        sortable.ref(el);
        focusBinding()?.bind(el);
      }}
      style={transformStyle(sortable.transform)}
      data-oa-focus={focusBinding()?.focused() ? "true" : undefined}
      data-oa-focus-active={focusBinding()?.active() ? "true" : undefined}
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
        <SortableProvider ids={sortableChildIds()}>
          <ul class="space-y-0.5">
            <For each={props.container.children}>
              {(child) => (
                <Show
                  when={child.kind === "platform"}
                  fallback={
                    // Nested containers. With deep-drag on (View Editor),
                    // recurse into a fully sortable node so it can be
                    // dragged/reordered/nested at any depth. With it off
                    // (live sidebar), keep the non-sortable static
                    // recursion — the sidebar reorders leaves only.
                    nestedSortable() ? (
                      <SortableContainerNode
                        container={child as ContainerNode}
                        depth={props.depth + 1}
                        ctx={props.ctx}
                      />
                    ) : (
                      <StaticContainerNode
                        container={child as ContainerNode}
                        depth={props.depth + 1}
                        ctx={props.ctx}
                      />
                    )
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
    </Show>
  );
};

// ── Sortable leaf ────────────────────────────────────────────────────

export const SortableLeafNode: Component<{
  leaf: PlatformNode;
  depth: number;
  ctx: SidebarTreeContext;
}> = (props) => {
  const sortable = createSortable(props.leaf.id);
  const focusBinding = createMemo(() => props.ctx.focusBindingFor?.(props.leaf.id) ?? null);
  return (
    <li
      ref={(el) => {
        sortable.ref(el);
        focusBinding()?.bind(el);
      }}
      style={transformStyle(sortable.transform)}
      data-system={props.leaf.systemId}
      data-oa-focus={focusBinding()?.focused() ? "true" : undefined}
      data-oa-focus-active={focusBinding()?.active() ? "true" : undefined}
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
  const focusBinding = createMemo(() => props.ctx.focusBindingFor?.(props.container.id) ?? null);
  // Game-set nodes nested inside a container render leaf-style (no twisty /
  // children). Nested reorder is Slice 2, so these aren't sortable here.
  if (isGameSetNode(props.container)) {
    return (
      <li
        ref={(el) => focusBinding()?.bind(el)}
        data-oa-focus={focusBinding()?.focused() ? "true" : undefined}
        data-oa-focus-active={focusBinding()?.active() ? "true" : undefined}
        class="relative"
      >
        <GameSetRow
          container={props.container}
          depth={props.depth}
          active={props.ctx.isActiveNode(props.container.id)}
          count={props.ctx.gameSetCount?.(props.container)}
          onNavigate={() => props.ctx.onNavigateToNode(props.container.id)}
          onContextMenu={(pos) => props.ctx.onContainerContextMenu?.(props.container, pos)}
        />
      </li>
    );
  }
  return (
    <li
      ref={(el) => focusBinding()?.bind(el)}
      data-oa-focus={focusBinding()?.focused() ? "true" : undefined}
      data-oa-focus-active={focusBinding()?.active() ? "true" : undefined}
      class="relative"
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
        data-oa-sidebar-row
        data-oa-tree-node-id={props.container.id}
        data-oa-tree-node-kind="container"
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
        data-oa-sidebar-row
        data-oa-tree-node-id={props.leaf.id}
        data-oa-tree-node-kind="leaf"
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

// ── Game-set row (collection / smart-list filter) ────────────────────

/// Glyph for a game-set node. Smart-list filters reuse the registry glyph
/// (❤ favorites, 🕘 recently-played, …); collections use a generic star.
/// A reserved/unknown filter spec falls back to a neutral list glyph.
function gameSetGlyph(container: ContainerNode): string {
  const rule = container.rule;
  if (rule?.kind === "collection") return "★";
  if (rule?.kind === "filter") {
    const kind = asSmartListKind(rule.spec);
    return kind ? SMART_LISTS_BY_KIND[kind].glyph : "▦";
  }
  return "▦";
}

/// Leaf-style row for a game-set node. Mirrors LeafRow's chrome (accent
/// active-bar, count badge, optional drag handle) but is driven by a
/// ContainerNode + a host-supplied membership count rather than a single
/// system. No twisty — its contents are games resolved via membership, not
/// child leaves. (Unified Navigation Tree Slice 1.)
const GameSetRow: Component<{
  container: ContainerNode;
  depth: number;
  active: boolean;
  count?: number;
  onNavigate: () => void;
  onContextMenu?: (position: { x: number; y: number }) => void;
  dragActivators?: DragActivators;
  isDragging?: boolean;
}> = (props) => {
  const indentRem = () => 0.5 + props.depth * 0.75;
  const rowStyle = () => {
    const base: Record<string, string> = { "padding-left": `${indentRem()}rem` };
    if (props.container.accent) base["--color-system-accent"] = props.container.accent;
    return base;
  };
  return (
    <div
      class="group relative flex w-full items-center gap-2 rounded-md py-2 pr-1 text-left text-xs font-medium transition"
      classList={{
        "bg-(--color-system-accent)/15 text-(--color-oa-ink)": props.active,
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
        data-oa-sidebar-row
        data-oa-tree-node-id={props.container.id}
        data-oa-tree-node-kind="game-set"
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
        title={props.container.label}
        class="flex flex-1 items-center gap-2 text-left"
      >
        <span class="text-sm leading-none" aria-hidden="true">
          {gameSetGlyph(props.container)}
        </span>
        <span class="flex-1 truncate">{props.container.label}</span>
        <Show when={(props.count ?? 0) > 0}>
          <span class="text-[0.6rem] tabular-nums uppercase tracking-widest text-(--color-oa-ink-dim)">
            {props.count}
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
