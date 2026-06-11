// `list` nav primitive — a verb-native, declaratively-configured focusable
// list (vertical or horizontal). Themes pick + configure it; it consumes verbs
// (via the focus framework's verb routing), never raw buttons (DECISIONS D18).
//
// It's a thin wrapper over `useFocusGroup`: data-source + render-prop driven,
// with declarative styling seams (density / focusProminence / easing) surfaced
// as data attributes for the S3 token layer to style. Imperative escape hatches
// are the verb-named callbacks only.
//
// Additive: existing bespoke focus surfaces (VirtualLibraryGrid, LeftSidebar,
// …) keep their own useFocusGroup usage. This is the theme-facing primitive the
// S2 walking-skeleton themes (Retroverse / Wheel) consume.

import { createEffect, createSignal, For, untrack, type Accessor, type Component } from "solid-js";
import { useFocusGroup } from "../focus";
import { HintRegion } from "../HintBar";
import type { NavPrimitiveBaseProps } from "./types";

export type ListNavProps<T> = NavPrimitiveBaseProps<T> & {
  /** Movement axis. Default "vertical". */
  orientation?: "vertical" | "horizontal";
};

export function ListNav<T>(props: ListNavProps<T>): ReturnType<Component> {
  const itemsArr = (): T[] =>
    typeof props.items === "function" ? (props.items as Accessor<T[]>)() : props.items;

  const [internalIdx, setInternalIdx] = createSignal(0);
  const focusedIndex = (): number => (props.focusedIndex ? props.focusedIndex() : internalIdx());
  const setFocusedIndex = (n: number): void => {
    if (props.setFocusedIndex) props.setFocusedIndex(n);
    else setInternalIdx(n);
  };

  const group = useFocusGroup({
    id: props.id,
    orientation: props.orientation ?? "vertical",
    itemCount: () => itemsArr().length,
    focusedIndex,
    setFocusedIndex,
    neighbours: props.neighbours,
    autoClaim: props.autoActivate,
    onActivate: (i) => {
      props.onConfirm?.(i, itemsArr()[i]);
      props.onNavSound?.("confirm", itemsArr()[i]);
    },
    onCancel: () => {
      props.onBack?.();
      props.onNavSound?.("back", undefined);
    },
    onSecondary: (i) => {
      props.onSecondary?.(i, itemsArr()[i]);
      props.onNavSound?.("secondary", itemsArr()[i]);
    },
    onTertiary: (i) => props.onTertiary?.(i, itemsArr()[i]),
  });

  // Nav-sound on focus move (scope-call #6) — tracks ONLY focusedIndex (items
  // read untracked so a data change doesn't fire a spurious move); skips the
  // initial settle. No-op unless the theme wired `onNavSound`.
  let firstSettle = true;
  createEffect(() => {
    const idx = focusedIndex();
    if (firstSettle) {
      firstSettle = false;
      return;
    }
    props.onNavSound?.("move", untrack(() => itemsArr()[idx]));
  });

  return (
    <div
      class={`oa-list-nav ${props.class ?? ""}`}
      data-oa-orientation={props.orientation ?? "vertical"}
      data-oa-density={props.density ?? "comfortable"}
      data-oa-focus-prominence={props.focusProminence ?? "ring"}
      data-oa-easing={props.easing ?? "standard"}
    >
      {props.hints ? <HintRegion hints={props.hints} /> : null}
      <For each={itemsArr()}>
        {(item, i) => {
          const focused: Accessor<boolean> = () => group.isActive() && focusedIndex() === i();
          return (
            <div
              class="oa-list-nav-item"
              ref={(el) => group.bind(i(), el)}
              tabindex={-1}
              data-oa-focus={focused() ? "true" : undefined}
              data-oa-focus-active={focused() ? "true" : undefined}
              onClick={() => {
                group.activate();
                setFocusedIndex(i());
                props.onConfirm?.(i(), item);
                props.onNavSound?.("confirm", item);
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
