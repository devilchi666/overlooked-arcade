// Reactive spring integrator (Theming ARC 3 Thrust M, M-mod.2). PRODUCT code —
// the runtime half of the spring model; `spring.ts` is the pure {bounce,duration}
// ↔ physics converter this consumes. Kept separate from spring.ts so that file
// stays DOM/Solid-free and trivially testable.
//
// This is the F10 bench's `createSpring` (SelectionChoreography.tsx) lifted to
// product code and parameterized by the D57 author surface: you pass a
// `{ bounce, duration }` SpringSpec, it converts to {stiffness,damping,mass} and
// integrates. Frame-rate independent (semi-implicit Euler with a clamped dt) so
// it feels identical at 60 and 144 Hz — the bench validated exactly this.
//
// MUST be called under a component owner (registers an onCleanup for its rAF).

import { createSignal, onCleanup, type Accessor } from "solid-js";
import { toPhysicalSpring, type SpringSpec } from "./spring";

export type SpringHandle = {
  /// The live, reactive spring value.
  value: Accessor<number>;
  /// Retarget — the value animates toward `target` with the spring's overshoot.
  set: (target: number) => void;
  /// Jump instantly to `value` (no animation), zeroing velocity. Use to reset
  /// before a replayed entrance, e.g. `snap(0.92); set(1)` for a grow-in.
  snap: (value: number) => void;
};

export type SpringOptions = {
  /// Settle thresholds — the loop stops once |velocity| and |value-target| are
  /// both under these. Defaults suit unit-scale springs (scale/opacity); bump for
  /// large pixel ranges where settling within 0.005px is needlessly precise.
  restDelta?: number;
  restVelocity?: number;
};

export function createSpringValue(
  initial: number,
  spec: SpringSpec,
  opts: SpringOptions = {},
): SpringHandle {
  const { stiffness, damping, mass } = toPhysicalSpring(spec);
  const restDelta = opts.restDelta ?? 0.005;
  const restVelocity = opts.restVelocity ?? 0.01;

  const [value, setValue] = createSignal(initial);
  let current = initial;
  let velocity = 0;
  let target = initial;
  let raf = 0;
  let last = 0;

  const tick = (t: number): void => {
    // Clamp dt to 40ms so a long frame (tab stall, GC) can't fling the spring.
    const dt = Math.min((last ? t - last : 16) / 1000, 0.04);
    last = t;
    const accel = (-stiffness * (current - target) - damping * velocity) / mass;
    velocity += accel * dt;
    current += velocity * dt;
    setValue(current);
    if (Math.abs(velocity) > restVelocity || Math.abs(current - target) > restDelta) {
      raf = requestAnimationFrame(tick);
    } else {
      current = target;
      velocity = 0;
      setValue(target);
      raf = 0;
      last = 0;
    }
  };

  const start = (): void => {
    if (!raf) {
      last = 0;
      raf = requestAnimationFrame(tick);
    }
  };

  onCleanup(() => {
    if (raf) cancelAnimationFrame(raf);
  });

  return {
    value,
    set(t: number) {
      target = t;
      start();
    },
    snap(v: number) {
      current = v;
      velocity = 0;
      target = v;
      setValue(v);
      if (raf) {
        cancelAnimationFrame(raf);
        raf = 0;
        last = 0;
      }
    },
  };
}
