// Pointer-tilt hook (Theming ARC 3 Thrust M, M-mod.4; audit §3 category D —
// box-art treatments). PRODUCT code. Lifted from the F10 BoxArtFX bench: a 3D
// rotate that follows the cursor over an element and eases back on leave, for the
// tactile "layered poster" feel (tvOS/PS5). Pure interaction state — the consumer
// binds `transform` to an element's style and wires the two handlers.
//
// Interaction-driven (not auto-motion), so it composes by nesting with the
// breathe (AmbientMotion) and focus spring: each effect owns its own element, the
// transforms multiply. `perspective()` lives on the tilted element so the rotation
// reads as depth.

import { createSignal, type Accessor } from "solid-js";

export type TiltHandle = {
  /// Bind to the element's `transform` (includes the `perspective()`).
  transform: Accessor<string>;
  onPointerMove: (e: PointerEvent & { currentTarget: HTMLElement }) => void;
  onPointerLeave: () => void;
};

const REST = "perspective(900px)";

/// `maxDeg` = peak tilt at the element's edges. Returns the resting perspective
/// transform until the pointer moves over the bound element.
export function useTilt(maxDeg = 14): TiltHandle {
  const [transform, setTransform] = createSignal(REST);
  return {
    transform,
    onPointerMove(e) {
      const r = e.currentTarget.getBoundingClientRect();
      if (r.width === 0 || r.height === 0) return;
      const px = (e.clientX - r.left) / r.width - 0.5; // -0.5..0.5
      const py = (e.clientY - r.top) / r.height - 0.5;
      setTransform(`perspective(900px) rotateY(${px * maxDeg}deg) rotateX(${-py * maxDeg}deg)`);
    },
    onPointerLeave() {
      setTransform(REST);
    },
  };
}
