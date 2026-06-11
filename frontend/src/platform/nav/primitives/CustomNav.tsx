// `custom` nav primitive — the high-ceiling escape hatch (S5.5). The list / grid
// / carousel primitives cover the common IAs; `custom` is for a theme that wants
// to draw an ARBITRARY layout but still get verb-native focus + hints + the
// nav-sound hook "for free". (It supersedes the old `customComponent` manifest
// field, which was an unrealized indirection — deleted in Phase 2.)
//
// Instead of an item render-prop, `custom` hands the THEME a focus API via its
// children render-prop: the items, the focused index, an `isActive`/`activate`
// pair, and a `bind(index)` ref factory to attach to whatever focusable nodes it
// draws. The theme owns 100% of the markup; the primitive owns the focus group +
// verb routing (DECISIONS D18). Minimal + declarative-config-friendly (#8).

import { createSignal, type Accessor, type Component, type JSX } from "solid-js";
import { useFocusGroup, type FocusOrientation } from "../focus";
import { HintRegion } from "../HintBar";
import type { NavPrimitiveBaseProps } from "./types";

/// The focus API handed to a `custom` primitive's render-prop.
export type CustomNavApi<T> = {
  /** The resolved items (reactive). */
  items: Accessor<T[]>;
  /** Focused index (reactive). */
  focusedIndex: Accessor<number>;
  /** Move focus (clamped by the focus group). */
  setFocusedIndex: (n: number) => void;
  /** True when this group owns the active input slot. */
  isActive: Accessor<boolean>;
  /** Claim the active slot (e.g. on click into the surface). */
  activate: () => void;
  /** Ref factory — `ref={bind(i)}` registers element `i` as focusable. */
  bind: (index: number) => (el: HTMLElement) => void;
};

export type CustomNavProps<T> = Omit<NavPrimitiveBaseProps<T>, "children"> & {
  /** Movement model for verb routing. Default "vertical". */
  orientation?: FocusOrientation;
  /** Column count when `orientation` is "grid" (D-pad wrap math). */
  columns?: number;
  /** Render-prop: draw the layout, wiring focus via the handed API. */
  children: (api: CustomNavApi<T>) => JSX.Element;
};

export function CustomNav<T>(props: CustomNavProps<T>): ReturnType<Component> {
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
    columns: props.columns != null ? () => props.columns ?? 1 : undefined,
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

  const api: CustomNavApi<T> = {
    items: itemsArr,
    focusedIndex,
    setFocusedIndex,
    isActive: group.isActive,
    activate: group.activate,
    bind: (index) => (el) => group.bind(index, el),
  };

  return (
    <div
      class={`oa-custom-nav ${props.class ?? ""}`}
      data-oa-density={props.density ?? "comfortable"}
      data-oa-focus-prominence={props.focusProminence ?? "ring"}
      data-oa-easing={props.easing ?? "standard"}
    >
      {props.hints ? <HintRegion hints={props.hints} /> : null}
      {props.children(api)}
    </div>
  );
}
