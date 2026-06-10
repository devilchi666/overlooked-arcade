// `grid` nav primitive — a verb-native, declaratively-configured focusable
// grid. Same contract as ListNav (data-source + render-prop driven, declarative
// styling seams, verb-named escape hatches) but laid out as a column grid; the
// focus framework's grid orientation drives up/down-by-columns + left/right
// linear walk (Steam Big Picture / Xbox dashboard semantics). DECISIONS D18.
//
// Additive — see ListNav's header note. This is the theme-facing primitive S2's
// Wheel/Retroverse skeletons consume for tile grids; VirtualLibraryGrid keeps
// its bespoke virtualized useFocusGroup usage.

import { createSignal, For, type Accessor, type Component } from "solid-js";
import { useFocusGroup } from "../focus";
import { HintRegion } from "../HintBar";
import type { NavPrimitiveBaseProps } from "./types";

export type GridNavProps<T> = NavPrimitiveBaseProps<T> & {
  /** Column count — drives up/down jump distance + row wrapping. */
  columns: number | Accessor<number>;
};

export function GridNav<T>(props: GridNavProps<T>): ReturnType<Component> {
  const itemsArr = (): T[] =>
    typeof props.items === "function" ? (props.items as Accessor<T[]>)() : props.items;
  const cols = (): number =>
    typeof props.columns === "function" ? (props.columns as Accessor<number>)() : props.columns;

  const [internalIdx, setInternalIdx] = createSignal(0);
  const focusedIndex = (): number => (props.focusedIndex ? props.focusedIndex() : internalIdx());
  const setFocusedIndex = (n: number): void => {
    if (props.setFocusedIndex) props.setFocusedIndex(n);
    else setInternalIdx(n);
  };

  const group = useFocusGroup({
    id: props.id,
    orientation: "grid",
    itemCount: () => itemsArr().length,
    columns: cols,
    focusedIndex,
    setFocusedIndex,
    neighbours: props.neighbours,
    autoClaim: props.autoActivate,
    onActivate: (i) => props.onConfirm?.(i, itemsArr()[i]),
    onCancel: () => props.onBack?.(),
    onSecondary: (i) => props.onSecondary?.(i, itemsArr()[i]),
    onTertiary: (i) => props.onTertiary?.(i, itemsArr()[i]),
  });

  return (
    <div
      class={`oa-grid-nav ${props.class ?? ""}`}
      data-oa-density={props.density ?? "comfortable"}
      data-oa-focus-prominence={props.focusProminence ?? "ring"}
      data-oa-easing={props.easing ?? "standard"}
      style={{
        display: "grid",
        "grid-template-columns": `repeat(${cols()}, minmax(0, 1fr))`,
      }}
    >
      {props.hints ? <HintRegion hints={props.hints} /> : null}
      <For each={itemsArr()}>
        {(item, i) => {
          const focused: Accessor<boolean> = () => group.isActive() && focusedIndex() === i();
          return (
            <div
              class="oa-grid-nav-item"
              ref={(el) => group.bind(i(), el)}
              tabindex={-1}
              data-oa-focus={focused() ? "true" : undefined}
              data-oa-focus-active={focused() ? "true" : undefined}
              onClick={() => {
                group.activate();
                setFocusedIndex(i());
                props.onConfirm?.(i(), item);
              }}
            >
              {props.children(item, { index: i(), focused })}
            </div>
          );
        }}
      </For>
    </div>
  );
}
