// Spring parameter conversion — the pure core of the declarative motion model
// (Theming ARC 3 Thrust M, M-mod.1). PRODUCT code (stays on ship); the Graphics
// Lab theme only *exercises* it.
//
// DECISIONS D57 locked the AUTHOR-facing spring surface as `{ bounce, duration }`
// (the iOS `UISpringTimingParameters` / SwiftUI `Spring(duration:bounce:)` /
// Motion convention) — orthogonal and predictable: `duration` = perceived speed,
// `bounce` = overshoot. The physics triple `{ stiffness, damping, mass }` is the
// documented escape hatch, converted internally — never the primary surface,
// because you can't predict settle time from stiffness/damping and "bouncier but
// same speed" means recomputing two coupled values.
//
// This module is the converter both directions: author → physics (what the rAF
// integrator in M-mod.2 will consume) and physics → author (so a value tuned in
// the F10 bench's stiffness/damping terms round-trips back to an authorable
// `{ bounce, duration }` pair). No DOM, no Solid — trivially unit-testable.

/// Author-facing spring (D57). `bounce` ∈ [-1, 1]: 0 = critically damped (no
/// overshoot), → 1 = increasingly bouncy (underdamped), < 0 = overdamped/sluggish.
/// `duration` is the perceptual settling time in **milliseconds** (matches the
/// `duration` primitive in the rest of the §2 basis).
export type SpringSpec = {
  bounce: number;
  duration: number;
};

/// The physics triple the integrator actually runs (semi-implicit Euler:
/// `a = -(stiffness/mass)·x - (damping/mass)·v`). `mass` defaults to 1 — the
/// bench's `createSpring` is implicitly mass-1, and Apple's closed form assumes
/// it — so author springs never need to touch it.
export type PhysicalSpring = {
  stiffness: number;
  damping: number;
  mass: number;
};

/// `bounce` → damping ratio ζ, the Apple/SwiftUI mapping:
///   bounce ≥ 0 (underdamped/critical): ζ = 1 − bounce  → ζ ∈ (0, 1]
///   bounce < 0  (overdamped):          ζ = 1 / (1 + bounce) → ζ > 1
/// Clamped to [-0.99, 1] so `1 + bounce` can't hit zero (infinite damping).
export function dampingRatioFromBounce(bounce: number): number {
  const b = Math.max(-0.99, Math.min(1, bounce));
  return b >= 0 ? 1 - b : 1 / (1 + b);
}

/// The inverse: damping ratio ζ → `bounce`. ζ ≤ 1 → underdamped branch
/// (bounce = 1 − ζ); ζ > 1 → overdamped branch (bounce = 1/ζ − 1). Used by
/// `physicalToSpring` and the back-solve test.
export function bounceFromDampingRatio(zeta: number): number {
  return zeta <= 1 ? 1 - zeta : 1 / zeta - 1;
}

/// Author spring → physics, via Apple's `Spring(duration:bounce:)` closed form
/// (mass = 1):
///   ωₙ = 2π / durationₛ           (undamped natural frequency)
///   stiffness = ωₙ² · mass
///   damping   = 2 · ζ · √(stiffness · mass)   where ζ = dampingRatioFromBounce(bounce)
/// `duration` is taken in ms and converted to seconds so stiffness/damping land
/// in the same unit system the bench's integrator used (per-second).
export function toPhysicalSpring(spec: SpringSpec, mass = 1): PhysicalSpring {
  const durationS = Math.max(spec.duration, 1) / 1000;
  const omega = (2 * Math.PI) / durationS;
  const stiffness = omega * omega * mass;
  const zeta = dampingRatioFromBounce(spec.bounce);
  const damping = 2 * zeta * Math.sqrt(stiffness * mass);
  return { stiffness, damping, mass };
}

/// Physics → author spring, the inverse of `toPhysicalSpring`:
///   ωₙ = √(stiffness / mass);  durationₘₛ = 2π / ωₙ · 1000
///   ζ  = damping / (2 · √(stiffness · mass));  bounce = bounceFromDampingRatio(ζ)
/// This is what makes a stiffness/damping value tuned at the F10 bench
/// expressible as an authorable `{ bounce, duration }` (see `BENCH_SELECTION_SPRING`).
export function physicalToSpring(phys: PhysicalSpring): SpringSpec {
  const omega = Math.sqrt(phys.stiffness / phys.mass);
  const duration = ((2 * Math.PI) / omega) * 1000;
  const zeta = phys.damping / (2 * Math.sqrt(phys.stiffness * phys.mass));
  return { bounce: bounceFromDampingRatio(zeta), duration };
}

/// The selection-momentum spring default, derived from the F10 motion bench's
/// operator-tuned `createSpring(stiffness: 190, damping: 24)` (mass 1) —
/// `SelectionChoreography.tsx`. Back-solving that physics pair through
/// `physicalToSpring` gives ≈ `{ bounce: 0.13, duration: 456ms }`, so shipping
/// THIS as the default means the declarative selection spring feels identical to
/// what the operator already signed off on at F10 — not a fresh round-number
/// guess. The back-solve relationship is asserted in `spring.test.ts`.
export const BENCH_SELECTION_SPRING: SpringSpec = { bounce: 0.13, duration: 456 };
