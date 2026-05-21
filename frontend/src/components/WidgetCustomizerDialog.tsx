// View → Customize widgets… dialog. Drives the right-sidebar widget list
// via layout.widgetOrder + layout.widgetHidden. The fields were on the
// store from the start; this is the first UI to mutate them. New widgets
// added to WIDGET_REGISTRY automatically show up here.

import { createMemo, For, type Component } from "solid-js";
import {
  closestCenter,
  createSortable,
  DragDropProvider,
  DragDropSensors,
  SortableProvider,
  transformStyle,
  type DragEventHandler,
} from "@thisbeyond/solid-dnd";
import { Dialog } from "../layout/Dialog";
import type { LayoutStore } from "../layout/state";
import { WIDGET_REGISTRY } from "../layout/widgets";

type Props = {
  open: boolean;
  onClose: () => void;
  layout: LayoutStore;
};

/// One row in the widget customizer sortable list. Visibility checkbox
/// + drag handle separate so the checkbox doesn't trigger drag.
const SortableWidgetRow: Component<{
  id: string;
  label: string;
  hidden: boolean;
  onToggleHidden: () => void;
}> = (props) => {
  const sortable = createSortable(props.id);
  return (
    <li
      ref={sortable.ref}
      style={transformStyle(sortable.transform)}
      class="flex items-center gap-2 rounded-md border border-white/10 bg-white/[0.03] px-2 py-1.5 transition"
      classList={{
        "hover:border-white/20": !sortable.isActiveDraggable,
        "border-(--color-system-accent) bg-(--color-system-accent)/10 z-10 shadow-lg":
          sortable.isActiveDraggable,
      }}
    >
      <span
        class="select-none px-1 text-(--color-oa-ink-dim) hover:text-(--color-oa-ink)"
        classList={{
          "cursor-grab": !sortable.isActiveDraggable,
          "cursor-grabbing": sortable.isActiveDraggable,
        }}
        role="button"
        tabindex="-1"
        aria-label={`Drag handle for ${props.label}`}
        {...sortable.dragActivators}
      >
        ⋮⋮
      </span>
      <input
        type="checkbox"
        checked={!props.hidden}
        onChange={props.onToggleHidden}
        class="size-4 cursor-pointer accent-(--color-system-accent)"
        aria-label={`Show ${props.label}`}
      />
      <span
        class="flex-1 truncate text-sm"
        classList={{
          "text-(--color-oa-ink)": !props.hidden,
          "text-(--color-oa-ink-dim) line-through": props.hidden,
        }}
      >
        {props.label}
      </span>
    </li>
  );
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

  /// solid-dnd onDragEnd — splice + reinsert against the resolved
  /// order. Widget ids are guaranteed unique by the registry.
  const handleDragEnd: DragEventHandler = ({ draggable, droppable }) => {
    if (!draggable || !droppable) return;
    const order = orderedWidgets();
    const fromIdx = order.indexOf(draggable.id as string);
    const toIdx = order.indexOf(droppable.id as string);
    if (fromIdx === -1 || toIdx === -1 || fromIdx === toIdx) return;
    const next = [...order];
    const [moved] = next.splice(fromIdx, 1);
    next.splice(toIdx, 0, moved);
    props.layout.setWidgetOrder(next);
  };

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
        Drag a row by its grip to reorder. Uncheck to hide a widget; it
        stays in the order so re-enabling it restores its position.
      </p>
      <DragDropProvider onDragEnd={handleDragEnd} collisionDetector={closestCenter}>
        <DragDropSensors />
        <SortableProvider ids={orderedWidgets()}>
          <ul class="space-y-1">
            <For each={orderedWidgets()}>
              {(id) => (
                <SortableWidgetRow
                  id={id}
                  label={WIDGET_REGISTRY[id]?.label ?? id}
                  hidden={isHidden(id)}
                  onToggleHidden={() => toggleHidden(id)}
                />
              )}
            </For>
          </ul>
        </SortableProvider>
      </DragDropProvider>
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
