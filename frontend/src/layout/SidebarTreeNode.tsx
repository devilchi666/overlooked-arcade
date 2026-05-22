import { For, Show, createMemo, type Component } from "solid-js";

import { systemThemes, type SystemId } from "../themes/registry";
import type { RomEntry } from "../library/types";
import type { ContainerNode, PlatformNode, ViewNode } from "../views/types";
import { countGamesUnder } from "../views/resolver";

/// Recursive node renderer for the PR-γ tree sidebar. Containers carry
/// a twisty + label + cumulative-descendants count badge; leaves carry
/// a system accent dot + short name + own-system count badge. Indent
/// scales with depth (~12px per level per SIDEBAR_TIER_PLAN.md §3.1).
///
/// Click semantics per plan §0:
///   - Twisty area on a container → toggle expanded state (no navigation).
///   - Label area on a container → navigate to the container's view-node
///     (LibraryView filters to the union of descendants per the
///     container's rule).
///   - Leaf row → navigate to the leaf's view-node (single-system view).
///
/// Drag-reorder + container right-click context menu land in γ.2 and
/// γ.3 respectively; this component is render + click + expand only.

export type SidebarTreeContext = {
  entries: RomEntry[];
  isExpanded: (nodeId: string) => boolean;
  isActiveNode: (nodeId: string) => boolean;
  onToggleExpanded: (nodeId: string) => void;
  onNavigateToNode: (nodeId: string) => void;
  onLeafContextMenu?: (systemId: SystemId, position: { x: number; y: number }) => void;
};

type Props = {
  node: ViewNode;
  depth: number;
  ctx: SidebarTreeContext;
};

const SidebarTreeNode: Component<Props> = (props) => {
  return (
    <Show
      when={props.node.kind === "container"}
      fallback={<LeafRow leaf={props.node as PlatformNode} depth={props.depth} ctx={props.ctx} />}
    >
      <ContainerRow
        container={props.node as ContainerNode}
        depth={props.depth}
        ctx={props.ctx}
      />
    </Show>
  );
};

const ContainerRow: Component<{ container: ContainerNode; depth: number; ctx: SidebarTreeContext }> = (props) => {
  const expanded = createMemo(() => props.ctx.isExpanded(props.container.id));
  const active = createMemo(() => props.ctx.isActiveNode(props.container.id));
  const count = createMemo(() => countGamesUnder(props.container, props.ctx.entries));
  const indentRem = () => 0.5 + props.depth * 0.75;

  return (
    <li class="relative">
      <div
        class="group relative flex w-full items-center gap-1 rounded-md pr-2 text-left text-xs font-medium transition"
        classList={{
          "bg-white/[0.07] text-(--color-oa-ink)": active(),
          "text-(--color-oa-ink-dim) hover:bg-white/[0.04] hover:text-(--color-oa-ink)": !active(),
        }}
        style={{ "padding-left": `${indentRem()}rem` }}
      >
        <Show when={active()}>
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
            props.ctx.onToggleExpanded(props.container.id);
          }}
          aria-label={expanded() ? "Collapse" : "Expand"}
          aria-expanded={expanded()}
          class="grid h-5 w-5 shrink-0 place-items-center rounded text-[0.6rem] text-(--color-oa-ink-dim) transition hover:bg-white/[0.06] hover:text-(--color-oa-ink)"
        >
          {expanded() ? "▾" : "▸"}
        </button>
        <button
          type="button"
          onClick={(e) => {
            e.currentTarget.blur();
            props.ctx.onNavigateToNode(props.container.id);
          }}
          aria-pressed={active()}
          class="flex flex-1 items-center gap-2 py-1.5 text-left"
        >
          <span class="flex-1 truncate">{props.container.label}</span>
          <Show when={count() > 0}>
            <span class="text-[0.6rem] tabular-nums uppercase tracking-widest text-(--color-oa-ink-dim)">
              {count()}
            </span>
          </Show>
        </button>
      </div>
      <Show when={expanded() && props.container.children.length > 0}>
        <ul class="space-y-0.5">
          <For each={props.container.children}>
            {(child) => <SidebarTreeNode node={child} depth={props.depth + 1} ctx={props.ctx} />}
          </For>
        </ul>
      </Show>
    </li>
  );
};

const LeafRow: Component<{ leaf: PlatformNode; depth: number; ctx: SidebarTreeContext }> = (props) => {
  const theme = () => systemThemes[props.leaf.systemId];
  const active = createMemo(() => props.ctx.isActiveNode(props.leaf.id));
  const count = createMemo(
    () => props.ctx.entries.filter((e) => e.systemId === props.leaf.systemId && !e.seed).length,
  );
  const indentRem = () => 0.5 + props.depth * 0.75;

  return (
    <li data-system={props.leaf.systemId} class="relative">
      <button
        type="button"
        onClick={(e) => {
          e.currentTarget.blur();
          props.ctx.onNavigateToNode(props.leaf.id);
        }}
        onContextMenu={(e) => {
          if (!props.ctx.onLeafContextMenu) return;
          e.preventDefault();
          props.ctx.onLeafContextMenu(props.leaf.systemId, { x: e.clientX, y: e.clientY });
        }}
        aria-pressed={active()}
        class="group relative flex w-full items-center gap-2 rounded-md py-2 pr-2 text-left text-xs font-medium transition"
        classList={{
          "bg-(--color-system-accent)/15 text-(--color-oa-ink)": active(),
          "text-(--color-oa-ink-dim) hover:bg-white/[0.04] hover:text-(--color-oa-ink)": !active(),
        }}
        style={{ "padding-left": `${indentRem()}rem` }}
        title={theme()?.displayName}
      >
        <Show when={active()}>
          <span
            class="pointer-events-none absolute left-0 top-1/2 h-5 w-0.5 -translate-y-1/2 rounded-r bg-(--color-system-accent)"
            aria-hidden="true"
          />
        </Show>
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
    </li>
  );
};

export default SidebarTreeNode;
