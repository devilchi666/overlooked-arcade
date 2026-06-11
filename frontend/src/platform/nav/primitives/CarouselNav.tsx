// `carousel` nav primitive — a verb-native, declaratively-configured horizontal
// coverflow / filmstrip (S5.5). Generalizes the windowed-track pattern CoverFlow
// hand-rolled in S2: a centred focused card, neighbours fanning out + dimming on
// a sliding track, with only a ±`window` slice of cards in the DOM (so it scales
// to a multi-thousand-item library). The card *content* is the theme's render
// prop; the primitive owns layout (position / scale / opacity / z-index), the
// track transform, focus wiring, wheel-browse, and click-to-centre.
//
// Like ListNav/GridNav it's a thin wrapper over `useFocusGroup` (horizontal),
// verb-native (DECISIONS D18), with declarative styling seams surfaced as
// `data-oa-*` attributes for the token layer. Imperative escape hatches are the
// verb-named callbacks + the optional `onNavSound` hook (scope-call #6).

import { createEffect, createMemo, createSignal, For, untrack, type Accessor, type Component, type JSX } from "solid-js";
import { useFocusGroup } from "../focus";
import { HintRegion } from "../HintBar";
import { useLateClaim } from "./lateClaim";
import type { NavItemContext, NavPrimitiveBaseProps } from "./types";

/// Item context for the carousel — adds the signed `offset` from the focused
/// card (0 = focused, ±n = n cards away) so the renderer can react to depth.
export type CarouselItemContext = NavItemContext & {
  offset: Accessor<number>;
};

export type CarouselNavProps<T> = Omit<NavPrimitiveBaseProps<T>, "children"> & {
  /** Card width in px. */
  cardWidth: number;
  /** Distance between adjacent card centres in px (< cardWidth → overlap). */
  pitch: number;
  /** Cards kept in the DOM each side of focus (render window). Default 8. */
  window?: number;
  /** Scale of the focused / side cards. Default 1 / 0.78. */
  focusedScale?: number;
  sideScale?: number;
  /** Per-step opacity falloff for side cards, clamped at `minOpacity`. */
  opacityFalloff?: number;
  minOpacity?: number;
  /** Card/track transition in ms. Default 260. */
  transitionMs?: number;
  /** Card renderer — receives the carousel item context (incl. `offset`). */
  children: (item: T, ctx: CarouselItemContext) => JSX.Element;
};

export function CarouselNav<T>(props: CarouselNavProps<T>): ReturnType<Component> {
  const itemsArr = (): T[] =>
    typeof props.items === "function" ? (props.items as Accessor<T[]>)() : props.items;

  const [internalIdx, setInternalIdx] = createSignal(0);
  const focusedIndex = (): number => (props.focusedIndex ? props.focusedIndex() : internalIdx());
  const setFocusedIndex = (n: number): void => {
    const len = itemsArr().length;
    if (len === 0) return;
    const clamped = Math.max(0, Math.min(len - 1, n));
    if (props.setFocusedIndex) props.setFocusedIndex(clamped);
    else setInternalIdx(clamped);
  };

  const win = (): number => props.window ?? 8;
  const focusedScale = (): number => props.focusedScale ?? 1;
  const sideScale = (): number => props.sideScale ?? 0.78;
  const falloff = (): number => props.opacityFalloff ?? 0.16;
  const minOpacity = (): number => props.minOpacity ?? 0.25;
  const transitionMs = (): number => props.transitionMs ?? 260;

  const group = useFocusGroup({
    id: props.id,
    orientation: "horizontal",
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

  // Late-claim once items appear (a carousel is often a late-mounting
  // whole-shell surface — see useLateClaim).
  useLateClaim(group, () => itemsArr().length, props.autoActivate);

  // Nav-sound on focus move — track ONLY focusedIndex (read items untracked so a
  // data change doesn't fire a spurious move). Skip the initial settle.
  let firstSettle = true;
  createEffect(() => {
    const idx = focusedIndex();
    if (firstSettle) {
      firstSettle = false;
      return;
    }
    props.onNavSound?.("move", untrack(() => itemsArr()[idx]));
  });

  // Windowed slice — only ±win cards around focus are in the DOM.
  const lo = createMemo(() => Math.max(0, focusedIndex() - win()));
  const visible = createMemo(() => {
    const list = itemsArr();
    const hi = Math.min(list.length - 1, focusedIndex() + win());
    return list.slice(lo(), hi + 1);
  });

  // Track shift centres the focused card at the container's 50%.
  const trackTransform = (): string =>
    `translateX(calc(50% - ${focusedIndex() * props.pitch + props.cardWidth / 2}px))`;

  const onWheel = (e: WheelEvent): void => {
    e.preventDefault();
    setFocusedIndex(focusedIndex() + (e.deltaY > 0 ? 1 : -1));
    group.activate();
  };

  return (
    <div
      class={`oa-carousel-nav relative overflow-hidden ${props.class ?? ""}`}
      data-oa-orientation="horizontal"
      data-oa-density={props.density ?? "comfortable"}
      data-oa-focus-prominence={props.focusProminence ?? "scale"}
      data-oa-easing={props.easing ?? "standard"}
      onWheel={onWheel}
    >
      {props.hints ? <HintRegion hints={props.hints} /> : null}
      <div
        class="oa-carousel-track absolute inset-0"
        style={{
          transform: trackTransform(),
          transition: `transform ${transitionMs()}ms cubic-bezier(.22,.61,.36,1)`,
          "will-change": "transform",
        }}
      >
        <For each={visible()}>
          {(item, i) => {
            const absIndex = (): number => lo() + i();
            const offset = (): number => absIndex() - focusedIndex();
            const focused: Accessor<boolean> = () => group.isActive() && offset() === 0;
            return (
              <div
                class="oa-carousel-card absolute top-1/2"
                ref={(el) => group.bind(absIndex(), el)}
                tabindex={-1}
                data-oa-focus={offset() === 0 ? "true" : undefined}
                style={{
                  left: `${absIndex() * props.pitch}px`,
                  width: `${props.cardWidth}px`,
                  transform: `translateY(-50%) scale(${offset() === 0 ? focusedScale() : sideScale()})`,
                  opacity: String(Math.max(minOpacity(), 1 - Math.abs(offset()) * falloff())),
                  "z-index": String(100 - Math.abs(offset())),
                  transition: `transform ${transitionMs()}ms cubic-bezier(.22,.61,.36,1), opacity ${transitionMs()}ms ease`,
                  "will-change": "transform, opacity",
                }}
                onClick={() => {
                  // Click a side card → centre it; click the centred card → confirm.
                  if (offset() === 0) {
                    props.onConfirm?.(absIndex(), item);
                    props.onNavSound?.("confirm", item);
                  } else {
                    setFocusedIndex(absIndex());
                    group.activate();
                  }
                }}
              >
                {props.children(item, { index: absIndex(), focused, offset })}
              </div>
            );
          }}
        </For>
      </div>
    </div>
  );
}
