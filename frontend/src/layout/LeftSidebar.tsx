import { For, Show, createMemo, createSignal, type Component } from "solid-js";
import type { LibraryStore } from "../library/store";
import { systemThemes, type SystemId } from "../themes/registry";
import type { LayoutStore } from "./state";

/// Optional tab to land on when navigating to a per-system settings page.
/// Lets the system page header's quick-action buttons + SystemContextMenu
/// deep-link past the persisted last-tab so e.g. "Edit bindings…" actually
/// drops on Input regardless of what was open last session.
export type SidebarView =
  | { kind: "all" }
  | { kind: "system"; id: SystemId }
  | { kind: "settings" }
  | { kind: "cores" };

type Props = {
  layout: LayoutStore;
  library: LibraryStore;
  currentView: SidebarView;
  onNavigate: (view: SidebarView) => void;
  /// Right-click on a system entry opens the SystemContextMenu anchored
  /// at the click coords. The menu surfaces per-system settings (the
  /// primary value-add — without it, the per-system page is only
  /// reachable via the ⚙ button in GridControls after left-clicking a
  /// system to filter the library).
  onSystemContext?: (id: SystemId, position: { x: number; y: number }) => void;
};

const DRAG_MIME = "application/x-oa-system";

/// Reconcile layout.systemOrder() against the live registry. Returns systems
/// in user-defined order, with any newly-registered systems appended in
/// their registry order (so adding a system later doesn't disappear it).
function orderedSystemIds(
  registry: SystemId[],
  userOrder: string[],
): SystemId[] {
  const registrySet = new Set(registry);
  const result: SystemId[] = [];
  const seen = new Set<string>();
  for (const id of userOrder) {
    if (registrySet.has(id as SystemId) && !seen.has(id)) {
      result.push(id as SystemId);
      seen.add(id);
    }
  }
  for (const id of registry) {
    if (!seen.has(id)) {
      result.push(id);
      seen.add(id);
    }
  }
  return result;
}

/**
 * Left sidebar — primary navigation surface. Sections (top → bottom):
 *
 *   Quick destinations — Home / All Games / Favorites / Recent / Continue.
 *     Pinned to the top, not reorderable. For Phase 2.5 only "All" navigates;
 *     others are placeholders until the feature lands.
 *
 *   Systems — one entry per registered SystemId. Counts pulled from the
 *     library store. Drag-to-reorder is deferred to a follow-up slice; for
 *     now display order matches `Object.keys(systemThemes)`.
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
  const registryIds = createMemo(() => Object.keys(systemThemes) as SystemId[]);
  const countForSystem = (id: SystemId): number =>
    props.library.state.entries.filter((e) => e.systemId === id && !e.seed).length;
  // Filter the registry by (a) explicitly user-hidden and (b) auto-hide-empty
  // when that pref is on. Order applies AFTER filtering. The active view's
  // system is always kept visible even if it would otherwise be hidden, so
  // navigating into a system never makes the sidebar entry vanish under you.
  const systemIds = createMemo(() => {
    const hidden = new Set(props.layout.hiddenSystems());
    const autoHide = props.layout.autoHideEmptySystems();
    const activeId =
      props.currentView.kind === "system" ? props.currentView.id : null;
    const filtered = registryIds().filter((id) => {
      if (id === activeId) return true;
      if (hidden.has(id)) return false;
      if (autoHide && countForSystem(id) === 0) return false;
      return true;
    });
    return orderedSystemIds(filtered, props.layout.systemOrder());
  });
  const totalCount = createMemo(() => props.library.state.entries.filter((e) => !e.seed).length);
  // Drag state — tracks which system index the user is dragging plus the
  // current drop-target index for the visual indicator.
  const [dragSourceIndex, setDragSourceIndex] = createSignal<number | null>(null);
  const [dragOverIndex, setDragOverIndex] = createSignal<number | null>(null);

  function commitReorder(from: number, to: number) {
    if (from === to) return;
    const current = systemIds();
    const next = current.slice();
    const [moved] = next.splice(from, 1);
    // Splice removes the source first; if dropping after the original
    // position, the target index has shifted by one.
    const insertAt = to > from ? to - 1 : to;
    next.splice(insertAt, 0, moved);
    props.layout.setSystemOrder(next);
  }

  const isActive = (view: SidebarView): boolean => {
    const cv = props.currentView;
    if (cv.kind !== view.kind) return false;
    if (cv.kind === "system" && view.kind === "system") return cv.id === view.id;
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
        {/* Quick destinations */}
        <ul class="space-y-0.5">
          <QuickItem icon="▦" label="All Games" badge={totalCount() > 0 ? String(totalCount()) : undefined} active={isActive({ kind: "all" })} collapsed={isCollapsed()} onClick={() => props.onNavigate({ kind: "all" })} />
        </ul>

        {/* Systems */}
        <SectionHeader label="Systems" collapsed={isCollapsed()} />
        <ul class="space-y-0.5">
          <For each={systemIds()}>
            {(id, index) => (
              <SystemItem
                id={id}
                count={countForSystem(id)}
                active={isActive({ kind: "system", id })}
                collapsed={isCollapsed()}
                draggable={!isCollapsed()}
                isDraggingThis={dragSourceIndex() === index()}
                dropIndicatorAbove={dragOverIndex() === index() && dragSourceIndex() !== null && dragSourceIndex() !== index()}
                onClick={() => props.onNavigate({ kind: "system", id })}
                onContextMenu={(pos) => props.onSystemContext?.(id, pos)}
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
                "bg-(--color-system-accent)": dragOverIndex() === systemIds().length,
              }}
              onDragOver={(ev) => {
                ev.preventDefault();
                setDragOverIndex(systemIds().length);
              }}
              onDrop={(ev) => {
                ev.preventDefault();
                const from = dragSourceIndex();
                setDragSourceIndex(null);
                setDragOverIndex(null);
                if (from === null) return;
                commitReorder(from, systemIds().length);
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

      {/* Shell-level actions pinned to bottom — Cores + Settings + collapse
          toggle. Sits below the navigation regions because these are app-wide
          actions (not library destinations) and the BigBox / Steam family
          convention puts settings at the bottom of the primary nav surface. */}
      <div class="space-y-1 border-t border-white/5 p-2">
        <button
          type="button"
          onClick={(e) => {
            e.currentTarget.blur();
            props.onNavigate({ kind: "cores" });
          }}
          class="flex w-full items-center gap-2 rounded-md border border-white/10 bg-white/[0.03] px-2 py-1.5 text-[0.65rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.07] hover:text-(--color-oa-ink)"
          classList={{
            "bg-white/[0.07] text-(--color-oa-ink)": isActive({ kind: "cores" }),
          }}
          aria-pressed={isActive({ kind: "cores" })}
          title="Cores"
        >
          <span aria-hidden="true">▥</span>
          <Show when={!isCollapsed()}>
            <span>Cores</span>
          </Show>
        </button>
        <button
          type="button"
          onClick={(e) => {
            e.currentTarget.blur();
            props.onNavigate({ kind: "settings" });
          }}
          class="flex w-full items-center gap-2 rounded-md border border-white/10 bg-white/[0.03] px-2 py-1.5 text-[0.65rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.07] hover:text-(--color-oa-ink)"
          classList={{
            "bg-white/[0.07] text-(--color-oa-ink)": isActive({ kind: "settings" }),
          }}
          aria-pressed={isActive({ kind: "settings" })}
          title="Settings"
        >
          <span aria-hidden="true">⚙</span>
          <Show when={!isCollapsed()}>
            <span>Settings</span>
          </Show>
        </button>
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
