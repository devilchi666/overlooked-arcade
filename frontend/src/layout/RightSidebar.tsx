import {
  For,
  Show,
  createEffect,
  createMemo,
  type Accessor,
  type Component,
} from "solid-js";
import type { RomEntry } from "../library/types";
import { WIDGET_REGISTRY } from "./widgets";
import type { LayoutStore } from "./state";
import { activateFocusGroup, useDomQueryFocusGroup } from "../nav/focus";

type Props = {
  layout: LayoutStore;
  /** Focused (hovered/selected) entry — drives widget content unless pinned. */
  focused: Accessor<RomEntry | null>;
  /** When pinned, the sidebar locks to this entry regardless of grid focus. */
  pinned: Accessor<RomEntry | null>;
  onLaunch: (entry: RomEntry) => void;
  onShowSaves: (entry: RomEntry) => void;
  onShowInfo: (entry: RomEntry) => void;
};

/**
 * Right sidebar — dashboard of widgets bound to the focused (or pinned) game.
 * Widgets are rendered in `layout.widgetOrder()` order, skipping those in
 * `layout.widgetHidden()`. Each widget is a small Solid component declared in
 * `widgets/index.tsx` keyed by id.
 *
 * Width is driven by the parent Shell's CSS grid using
 * `--layout-right-sidebar-width` plus the user-overridable signal in
 * LayoutStore. Resizer handle on the LEFT edge.
 */
const RightSidebar: Component<Props> = (props) => {
  // The entry whose widgets render. Pinned wins; otherwise focused.
  const activeEntry = createMemo<RomEntry | null>(() => props.pinned() ?? props.focused());

  // Visible widget definitions in user-defined order.
  const visibleWidgets = createMemo(() => {
    const hidden = new Set(props.layout.widgetHidden());
    return props.layout
      .widgetOrder()
      .filter((id) => !hidden.has(id))
      .map((id) => WIDGET_REGISTRY[id])
      .filter((w): w is NonNullable<typeof w> => w !== undefined);
  });

  const isPinned = () => props.pinned() !== null;

  const togglePin = () => {
    const f = props.focused();
    if (isPinned()) {
      props.layout.setRightSidebarPinnedGameId(null);
    } else if (f) {
      props.layout.setRightSidebarPinnedGameId(f.id);
    }
  };

  // Controller-nav v2: one DOM-query focus group spans every focusable
  // row in the sidebar body — read-only widget sections (top) and the
  // action row (bottom) — keyed by `data-oa-sidebar-row`. The primary
  // play path is preserved: while the group is inactive, focus parks
  // on the first action (Play), so the next R1-arrival from the
  // library lands the ring there. Operators who want to glance at the
  // cover or metadata can DPad up through the widget rows; A on a
  // widget row is a no-op (the rows are read-only — `data-oa-action`
  // only appears on the action buttons).
  let bodyRef: HTMLDivElement | undefined;
  const widgetCount = createMemo(() => visibleWidgets().length);
  const focusGroup = useDomQueryFocusGroup({
    id: "right-sidebar",
    containerRef: () => bodyRef,
    selector: "[data-oa-sidebar-row]",
    onActivate: (_i, el) => {
      const entry = activeEntry();
      if (!entry) return;
      const action = el.getAttribute("data-oa-action");
      if (action === "play") props.onLaunch(entry);
      else if (action === "saves") props.onShowSaves(entry);
      else if (action === "info") props.onShowInfo(entry);
    },
    onCancel: () => activateFocusGroup("library-grid"),
    neighbours: { left: "library-grid" },
  });
  // While the group is inactive, snap focus to the first action button
  // so the next R1 from the grid lands on Play. The effect only fires
  // on the inactive→active boundary; active navigation is left alone.
  createEffect(() => {
    if (!focusGroup.isActive()) {
      focusGroup.setFocusedIndex(widgetCount());
    }
  });

  const beginResize = (event: PointerEvent) => {
    event.preventDefault();
    const startX = event.clientX;
    const startWidth = props.layout.rightSidebarWidth();
    const min = 240;
    const max = 440;
    const onMove = (ev: PointerEvent) => {
      // Right sidebar grows leftward — invert delta sign.
      const next = Math.max(min, Math.min(max, startWidth - (ev.clientX - startX)));
      props.layout.setRightSidebarWidth(next);
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
    <aside class="relative flex h-full flex-col border-l border-white/5 bg-black/15">
      {/* Width resizer — left edge */}
      <div
        class="absolute left-0 top-0 z-10 h-full w-1 cursor-ew-resize hover:bg-(--color-system-accent)/40"
        onPointerDown={beginResize}
        role="separator"
        aria-orientation="vertical"
        aria-label="Resize right sidebar"
      />

      {/* Header — pin toggle + hide button */}
      <header class="flex items-center justify-between gap-2 border-b border-white/5 px-3 py-2">
        <span class="text-[0.55rem] font-semibold uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
          Game details
        </span>
        <div class="flex gap-1">
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              togglePin();
            }}
            disabled={!props.focused() && !isPinned()}
            aria-pressed={isPinned()}
            title={isPinned() ? "Unpin (follow grid focus)" : "Pin to this game"}
            class="rounded border border-white/10 bg-white/[0.03] px-2 py-1 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.07] hover:text-(--color-oa-ink) disabled:cursor-not-allowed disabled:opacity-40"
            classList={{ "text-(--color-system-accent)!": isPinned() }}
          >
            {isPinned() ? "📌 Pinned" : "Pin"}
          </button>
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              props.layout.setRightSidebarVisible(false);
            }}
            title="Hide sidebar"
            class="rounded border border-white/10 bg-white/[0.03] px-2 py-1 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.07] hover:text-(--color-oa-ink)"
          >
            ›
          </button>
        </div>
      </header>

      {/* Body — widgets or empty state */}
      <div ref={bodyRef} class="flex-1 overflow-y-auto overscroll-contain py-4">
        <Show
          when={activeEntry()}
          fallback={
            <div class="flex h-full flex-col items-center justify-center gap-2 px-6 text-center">
              <span class="text-2xl text-(--color-oa-ink-dim)">◐</span>
              <p class="text-[0.7rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                Hover a tile to see details
              </p>
            </div>
          }
        >
          {(entry) => (
            <div class="space-y-5">
              <For each={visibleWidgets()}>
                {(widget, i) => {
                  const W = widget.component;
                  return (
                    <div
                      data-oa-sidebar-row
                      tabindex="-1"
                      onMouseEnter={() => focusGroup.setFocusedIndex(i())}
                      class="rounded-md outline-none"
                    >
                      <W entry={entry()} />
                    </div>
                  );
                }}
              </For>

              {/* Action row at the bottom */}
              <section class="px-3 pt-3">
                <div class="flex flex-col gap-1.5">
                  <button
                    type="button"
                    data-oa-sidebar-row
                    data-oa-action="play"
                    onMouseEnter={() => focusGroup.setFocusedIndex(widgetCount())}
                    onClick={(e) => {
                      e.currentTarget.blur();
                      props.onLaunch(entry());
                    }}
                    class="rounded-md bg-(--color-system-accent) px-3 py-2 text-xs font-semibold uppercase tracking-wider text-(--color-oa-bg-deep) transition hover:brightness-110"
                  >
                    ▶ Play
                  </button>
                  <div class="grid grid-cols-2 gap-1.5">
                    <button
                      type="button"
                      data-oa-sidebar-row
                      data-oa-action="saves"
                      onMouseEnter={() => focusGroup.setFocusedIndex(widgetCount() + 1)}
                      onClick={(e) => {
                        e.currentTarget.blur();
                        props.onShowSaves(entry());
                      }}
                      class="rounded-md border border-white/10 bg-white/[0.03] px-2 py-1.5 text-[0.65rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.07] hover:text-(--color-oa-ink)"
                    >
                      Saves
                    </button>
                    <button
                      type="button"
                      data-oa-sidebar-row
                      data-oa-action="info"
                      onMouseEnter={() => focusGroup.setFocusedIndex(widgetCount() + 2)}
                      onClick={(e) => {
                        e.currentTarget.blur();
                        props.onShowInfo(entry());
                      }}
                      class="rounded-md border border-white/10 bg-white/[0.03] px-2 py-1.5 text-[0.65rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.07] hover:text-(--color-oa-ink)"
                    >
                      Game info
                    </button>
                  </div>
                </div>
              </section>
            </div>
          )}
        </Show>
      </div>
    </aside>
  );
};

export default RightSidebar;
