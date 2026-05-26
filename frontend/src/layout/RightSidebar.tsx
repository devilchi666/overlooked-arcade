import { For, Show, createMemo, createSignal, type Accessor, type Component } from "solid-js";
import type { RomEntry } from "../library/types";
import { WIDGET_REGISTRY } from "./widgets";
import type { LayoutStore } from "./state";
import { activateFocusGroup, useFocusGroup } from "../nav/focus";

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

  // Controller-nav: the action row at the bottom is the primary
  // interactive surface. Three items in display order — Play / Saves /
  // Game info. Read-only widgets above don't take focus in v1. L1 jumps
  // back to the library-grid; B does the same so the operator can back
  // out of the sidebar without grabbing a mouse.
  type ActionKey = "play" | "saves" | "info";
  const actions: ActionKey[] = ["play", "saves", "info"];
  const actionEls = new Map<number, HTMLElement>();
  const [focusedActionIndex, setFocusedActionIndex] = createSignal(0);
  const focusGroup = useFocusGroup({
    id: "right-sidebar",
    orientation: "vertical",
    itemCount: () => (activeEntry() ? actions.length : 0),
    focusedIndex: focusedActionIndex,
    setFocusedIndex: setFocusedActionIndex,
    onActivate: (i) => {
      const entry = activeEntry();
      if (!entry) return;
      const key = actions[i];
      if (key === "play") props.onLaunch(entry);
      else if (key === "saves") props.onShowSaves(entry);
      else if (key === "info") props.onShowInfo(entry);
    },
    onCancel: () => activateFocusGroup("library-grid"),
    neighbours: { left: "library-grid" },
  });
  function bindAction(key: ActionKey, el: HTMLElement | null): void {
    const i = actions.indexOf(key);
    if (i < 0) return;
    if (el) actionEls.set(i, el);
    else actionEls.delete(i);
    focusGroup.bind(i, el);
  }

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
      <div class="flex-1 overflow-y-auto overscroll-contain py-4">
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
                {(widget) => {
                  const W = widget.component;
                  return <W entry={entry()} />;
                }}
              </For>

              {/* Action row at the bottom */}
              <section class="px-3 pt-3">
                <div class="flex flex-col gap-1.5">
                  <button
                    type="button"
                    ref={(el) => bindAction("play", el)}
                    data-oa-focus={focusedActionIndex() === 0 ? "true" : undefined}
                    data-oa-focus-active={focusGroup.isActive() ? "true" : undefined}
                    onMouseEnter={() => setFocusedActionIndex(0)}
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
                      ref={(el) => bindAction("saves", el)}
                      data-oa-focus={focusedActionIndex() === 1 ? "true" : undefined}
                      data-oa-focus-active={focusGroup.isActive() ? "true" : undefined}
                      onMouseEnter={() => setFocusedActionIndex(1)}
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
                      ref={(el) => bindAction("info", el)}
                      data-oa-focus={focusedActionIndex() === 2 ? "true" : undefined}
                      data-oa-focus-active={focusGroup.isActive() ? "true" : undefined}
                      onMouseEnter={() => setFocusedActionIndex(2)}
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
