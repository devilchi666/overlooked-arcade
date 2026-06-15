// `wheel` nav primitive — a verb-native, declaratively-configured RADIAL wheel
// (the iconic BigBox / HyperSpin shape: items on a circle, the focused one
// pinned at a fixed on-screen anchor, the wheel rotating under it as you
// browse). L4b body — S5.5 shipped only the typed contract + a warn-once stub;
// this is the real radial geometry.
//
// GEOMETRY IS GENERAL ON PURPOSE. The visible "shape" is just a choice of where
// the focus sits ON the circle (`anchorAngle`) and WHERE that point lands on
// screen (`anchor`). Each item's position is a pure `angle → x/y` projection of
// its signed offset from focus, so future display modes are PRESETS over the
// same engine, not rewrites:
//   • Shape A — vertical right-side wheel (DEFAULTS here): anchorAngle 270°
//     (focus = leftmost point of the circle, centre off-screen right), anchored
//     right-of-centre so neighbours fan up/down and curve away rightward, the
//     iconic wheel. Leaves the left of the pane free for art.
//   • Shape B — centred radial fan: anchor {x:"50%",y:"50%"} + a wider arc.
//   • Shape C — bottom convex arc: anchorAngle ~0°, anchor {x:"50%",y:"82%"}.
// Shape A is the only one wired in L4b (Retroverse tg16 demo); B/C land later as
// new anchor/anchorAngle/arc presets (operator: "we'll add B and C later").
//
// Like CarouselNav it's a thin wrapper over `useFocusGroup` (vertical), windowed
// (only ±`window` items in the DOM, so it scales to 1700+-game systems, D29.1),
// verb-native (DECISIONS D18), with declarative styling seams as `data-oa-*`
// attributes. The track does NOT translate; each item derives its own on-screen
// position from its offset, so a focus change re-projects every item and CSS
// transitions animate the slide along the arc (the "rotation").

import {
  createEffect,
  createMemo,
  createSignal,
  For,
  onCleanup,
  untrack,
  type Accessor,
  type Component,
  type JSX,
} from "solid-js";
import { useFocusGroup } from "../focus";
import { HintRegion } from "../HintBar";
import type { CarouselItemContext } from "./CarouselNav";
import { useLateClaim } from "./lateClaim";
import type { NavPrimitiveBaseProps } from "./types";
import { wheelDisplacement } from "./wheelGeometry";

/// The radial-wheel contract. Mirrors CarouselNav where it can (a windowed,
/// focus-anchored ring sharing the `CarouselItemContext` render shape) and adds
/// the radial geometry knobs.
export type WheelNavProps<T> = Omit<NavPrimitiveBaseProps<T>, "children"> & {
  /** Ring radius in px. The vertical extent of the visible arc ≈ `radius`, so
   * size it ~half the pane height for a wheel that fills the column. */
  radius: number;
  /** Total arc the visible window (±`window` items) spans, in degrees. Default 80
   * — a gentle curve with near-even vertical spacing (a larger arc compresses
   * the spacing toward the edges, which reads as items "bunching"). A future
   * near-flat "strip" view drops this further (~20). */
  arcDegrees?: number;
  /** Items kept in the DOM each side of the anchored focus. Default 6. */
  window?: number;
  /** Angle (deg, 0 = top, clockwise) the focused item sits at ON the circle.
   * Default 270 (leftmost point → the shape-A right-side wheel). */
  anchorAngle?: number;
  /** Where the focused item is pinned ON SCREEN (CSS lengths/percentages,
   * resolved against the container). Default right-of-centre, vertically
   * centred — the shape-A layout. Change this (+ `anchorAngle`/`arcDegrees`)
   * for the centred-fan / bottom-arc shapes. */
  anchor?: { x?: string; y?: string };
  /** Scale of the focused item / the farthest visible item; intermediate items
   * lerp between them by |offset|/window. Default 1 / 0.85 (a gentle size
   * falloff — the focus is biggest, but the others don't dwindle). */
  focusedScale?: number;
  sideScale?: number;
  /** Per-step opacity falloff for items away from focus, clamped at `minOpacity`. */
  opacityFalloff?: number;
  minOpacity?: number;
  /** Rotation / item transition in ms. Default 300. */
  transitionMs?: number;
  /** Item renderer — same context shape as the carousel (incl. signed `offset`). */
  children: (item: T, ctx: CarouselItemContext) => JSX.Element;
};

export function WheelNav<T>(props: WheelNavProps<T>): ReturnType<Component> {
  const itemsArr = (): T[] =>
    typeof props.items === "function" ? (props.items as Accessor<T[]>)() : props.items;

  const [internalIdx, setInternalIdx] = createSignal(0);
  const focusedIndex = (): number => (props.focusedIndex ? props.focusedIndex() : internalIdx());

  // Fast-scroll detection. A CSS transition restarts (with its velocity reset)
  // every time the focus moves; when steps arrive faster than the transition
  // duration the items fall progressively behind the true focus and the wheel
  // visibly deforms / the leading edge gaps. So when moves come in rapid
  // succession we collapse the transition to a near-snap (items keep up), and
  // restore the full gentle ease once scrolling settles.
  const [fastScrolling, setFastScrolling] = createSignal(false);
  let lastMoveAt = 0;
  let settleTimer: ReturnType<typeof setTimeout> | undefined;
  onCleanup(() => settleTimer && clearTimeout(settleTimer));

  const setFocusedIndex = (n: number): void => {
    const len = itemsArr().length;
    if (len === 0) return;
    const clamped = Math.max(0, Math.min(len - 1, n));
    const now = typeof performance !== "undefined" ? performance.now() : Date.now();
    if (now - lastMoveAt < 140) {
      setFastScrolling(true);
      if (settleTimer) clearTimeout(settleTimer);
      // Drop back to the smooth ease shortly after the last rapid move, so the
      // final resting step animates nicely.
      settleTimer = setTimeout(() => setFastScrolling(false), 160);
    }
    lastMoveAt = now;
    if (props.setFocusedIndex) props.setFocusedIndex(clamped);
    else setInternalIdx(clamped);
  };

  const win = (): number => props.window ?? 6;
  const arcDegrees = (): number => props.arcDegrees ?? 80;
  const anchorAngle = (): number => props.anchorAngle ?? 270;
  const anchorX = (): string => props.anchor?.x ?? "62%";
  const anchorY = (): string => props.anchor?.y ?? "50%";
  const focusedScale = (): number => props.focusedScale ?? 1;
  const sideScale = (): number => props.sideScale ?? 0.85;
  const falloff = (): number => props.opacityFalloff ?? 0.07;
  const minOpacity = (): number => props.minOpacity ?? 0.35;
  const transitionMs = (): number => props.transitionMs ?? 300;
  // ~snap while fast-scrolling so items track the focus; full ease when settled.
  const effectiveTransitionMs = (): number => (fastScrolling() ? 70 : transitionMs());

  const group = useFocusGroup({
    id: props.id,
    orientation: "vertical",
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

  // Late-claim once items appear (a wheel is often a late-mounting whole-shell
  // surface — see useLateClaim, the S5.5 fix).
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

  // Windowed slice — only ±win items around focus are in the DOM.
  const lo = createMemo(() => Math.max(0, focusedIndex() - win()));
  const visible = createMemo(() => {
    const list = itemsArr();
    const hi = Math.min(list.length - 1, focusedIndex() + win());
    return list.slice(lo(), hi + 1);
  });

  // Pure projection (see wheelGeometry): an item `offset` steps from focus → its
  // pixel displacement from the on-screen anchor point. Positive offset (next
  // item, higher index) rotates DOWN so the wheel reads like a list.
  const displacement = (offset: number): { dx: number; dy: number } =>
    wheelDisplacement(offset, {
      radius: props.radius,
      anchorAngle: anchorAngle(),
      arcDegrees: arcDegrees(),
      window: win(),
    });

  const scaleFor = (offset: number): number => {
    const t = Math.min(1, Math.abs(offset) / win());
    return focusedScale() - (focusedScale() - sideScale()) * t;
  };

  const onWheel = (e: WheelEvent): void => {
    e.preventDefault();
    setFocusedIndex(focusedIndex() + (e.deltaY > 0 ? 1 : -1));
    group.activate();
  };

  return (
    <div
      class={`oa-wheel-nav relative overflow-hidden ${props.class ?? ""}`}
      data-oa-orientation="vertical"
      data-oa-density={props.density ?? "comfortable"}
      data-oa-focus-prominence={props.focusProminence ?? "scale"}
      data-oa-easing={props.easing ?? "standard"}
      onWheel={onWheel}
    >
      {props.hints ? <HintRegion hints={props.hints} /> : null}
      <For each={visible()}>
        {(item, i) => {
          const absIndex = (): number => lo() + i();
          const offset = (): number => absIndex() - focusedIndex();
          const focused: Accessor<boolean> = () => group.isActive() && offset() === 0;
          const d = (): { dx: number; dy: number } => displacement(offset());
          return (
            <div
              class="oa-wheel-item absolute"
              ref={(el) => group.bind(absIndex(), el)}
              tabindex={-1}
              data-oa-focus={offset() === 0 ? "true" : undefined}
              style={{
                left: anchorX(),
                top: anchorY(),
                transform: `translate(calc(-50% + ${d().dx}px), calc(-50% + ${d().dy}px)) scale(${scaleFor(offset())})`,
                opacity: String(Math.max(minOpacity(), 1 - Math.abs(offset()) * falloff())),
                "z-index": String(100 - Math.abs(offset())),
                transition: `transform ${effectiveTransitionMs()}ms cubic-bezier(.22,.61,.36,1), opacity ${effectiveTransitionMs()}ms ease`,
                "will-change": "transform, opacity",
              }}
              onClick={() => {
                // Click an item off the anchor → bring it to focus; click the
                // focused item → confirm (launch).
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
  );
}
