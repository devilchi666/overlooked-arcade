// View → Customize widgets… dialog. Drives the right-sidebar widget list
// via layout.widgetOrder + layout.widgetHidden. The fields were on the
// store from the start; this is the first UI to mutate them. New widgets
// added to WIDGET_REGISTRY automatically show up here.

import { createMemo, For, type Component } from "solid-js";
import { Dialog } from "../layout/Dialog";
import type { LayoutStore } from "../layout/state";
import { WIDGET_REGISTRY } from "../layout/widgets";

type Props = {
  open: boolean;
  onClose: () => void;
  layout: LayoutStore;
};

export const WidgetCustomizerDialog: Component<Props> = (props) => {
  // Resolve the live widget order against the registry. Unknown ids in
  // the persisted order are dropped silently (they refer to widgets that
  // existed in an older build); newly-registered widgets get appended at
  // the end so they don't disappear.
  const orderedWidgets = createMemo(() => {
    const registryIds = Object.keys(WIDGET_REGISTRY);
    const userOrder = props.layout.widgetOrder();
    const seen = new Set<string>();
    const out: string[] = [];
    for (const id of userOrder) {
      if (registryIds.includes(id) && !seen.has(id)) {
        out.push(id);
        seen.add(id);
      }
    }
    for (const id of registryIds) {
      if (!seen.has(id)) {
        out.push(id);
        seen.add(id);
      }
    }
    return out;
  });

  const isHidden = (id: string): boolean =>
    props.layout.widgetHidden().includes(id);

  function toggleHidden(id: string) {
    const cur = props.layout.widgetHidden();
    if (cur.includes(id)) {
      props.layout.setWidgetHidden(cur.filter((x) => x !== id));
    } else {
      props.layout.setWidgetHidden([...cur, id]);
    }
  }

  function moveUp(index: number) {
    if (index <= 0) return;
    const order = orderedWidgets().slice();
    [order[index - 1], order[index]] = [order[index], order[index - 1]];
    props.layout.setWidgetOrder(order);
  }

  function moveDown(index: number) {
    const order = orderedWidgets().slice();
    if (index >= order.length - 1) return;
    [order[index], order[index + 1]] = [order[index + 1], order[index]];
    props.layout.setWidgetOrder(order);
  }

  function resetDefaults() {
    props.layout.setWidgetOrder(Object.keys(WIDGET_REGISTRY));
    props.layout.setWidgetHidden([]);
  }

  return (
    <Dialog
      open={props.open}
      onClose={props.onClose}
      title="Customize widgets"
      subtitle="Right-sidebar layout"
      size="sm"
    >
      <p class="mb-3 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
        Drag-free reorder — use the arrows. Uncheck to hide a widget; it
        stays in the order so re-enabling it restores its position.
      </p>
      <ul class="space-y-1">
        <For each={orderedWidgets()}>
          {(id, i) => (
            <li class="flex items-center gap-2 rounded-md border border-white/10 bg-white/[0.03] px-2 py-1.5">
              <input
                type="checkbox"
                checked={!isHidden(id)}
                onChange={() => toggleHidden(id)}
                class="size-4 cursor-pointer accent-(--color-system-accent)"
                aria-label={`Show ${WIDGET_REGISTRY[id]?.label ?? id}`}
              />
              <span
                class="flex-1 truncate text-sm"
                classList={{
                  "text-(--color-oa-ink)": !isHidden(id),
                  "text-(--color-oa-ink-dim) line-through": isHidden(id),
                }}
              >
                {WIDGET_REGISTRY[id]?.label ?? id}
              </span>
              <button
                type="button"
                onClick={() => moveUp(i())}
                disabled={i() === 0}
                class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-xs text-(--color-oa-ink) transition hover:bg-white/[0.08] disabled:cursor-not-allowed disabled:opacity-40"
                title="Move up"
                aria-label="Move up"
              >
                ↑
              </button>
              <button
                type="button"
                onClick={() => moveDown(i())}
                disabled={i() === orderedWidgets().length - 1}
                class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-xs text-(--color-oa-ink) transition hover:bg-white/[0.08] disabled:cursor-not-allowed disabled:opacity-40"
                title="Move down"
                aria-label="Move down"
              >
                ↓
              </button>
            </li>
          )}
        </For>
      </ul>
      <div class="mt-3 flex justify-end">
        <button
          type="button"
          onClick={resetDefaults}
          class="rounded-md border border-white/10 bg-white/[0.04] px-3 py-1 text-xs uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
        >
          Reset to defaults
        </button>
      </div>
    </Dialog>
  );
};
