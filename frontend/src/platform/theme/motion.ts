// Declarative motion resolver + reduced-motion plumbing (Theming ARC 3 Thrust
// M, M1). The pure half of the motion layer: it turns a theme's declared
// `manifest.motion` (DATA, D50) into a concrete `ResolvedViewTransition` the
// UI/DOM-layer `ViewTransition` component plays (D51). No DOM, no scripting.
//
// Two callers:
//   1. `DeclarativeShell` — resolves the active theme's transition + the live
//      `prefers-reduced-motion` state into the transition it hands `ViewTransition`.
//   2. Vitest (`motion.test.ts`) — the manifest→transition resolution gate.
//
// REDUCED MOTION (the a11y floor, per DECISIONS' boot-anim entry — "downgrade
// to a short fade", not a hard cut). When the user prefers reduced motion, ANY
// declared transition collapses to a short fade — never instant (a gentle
// crossfade keeps the view change legible without vestibular motion). The
// resolver does this itself rather than reading `var(--motion-*)`: the global
// reduced-motion CSS zeroes those duration tokens, but the boot-anim contract
// wants a short fade, not 0ms — so the floor is encoded here as a fixed value.

import { createSignal, onCleanup, type Accessor } from "solid-js";
import type { MotionRef, ThemeMotion, ViewTransitionPreset } from "./manifest";
import { presetToSpec, REDUCED_MOTION_SPEC, type MotionSpec } from "./motionSpec";
import { buildPreset, type PresetOpts } from "./motionPresets";

/// The standard transition duration when a theme declares a preset but no
/// `duration`. Matches `--motion-medium` (index.css) numerically — but is a
/// fixed number, not a token read, so it's immune to the reduced-motion reset.
export const DEFAULT_TRANSITION_MS = 250;

/// The short-fade duration the reduced-motion floor downgrades to. Gentle, not
/// instant (the DECISIONS boot-anim guidance) — enough to keep a view change
/// legible without vestibular motion.
export const REDUCED_MOTION_FADE_MS = 120;

/// The default easing when a theme declares a preset but no `easing`. The
/// concrete value of `--ease-out` (a decelerate curve) — concrete, not a
/// `var()`, because it drives the Web Animations API, which can't resolve
/// custom properties.
export const DEFAULT_TRANSITION_EASING = "cubic-bezier(0.16, 1, 0.3, 1)";

/// A fully resolved, ready-to-play view transition. `preset: "none"` +
/// `durationMs: 0` means "render with no animation" (the consumer skips
/// animating entirely).
export type ResolvedViewTransition = {
  preset: ViewTransitionPreset;
  durationMs: number;
  easing: string;
};

/// The no-animation result — a theme that declares nothing, or `preset: "none"`.
const NO_TRANSITION: ResolvedViewTransition = {
  preset: "none",
  durationMs: 0,
  easing: DEFAULT_TRANSITION_EASING,
};

/// Parse a CSS time string (`"250ms"`, `"0.4s"`, `"400"`) to milliseconds.
/// Returns `null` for an absent / unparseable / non-positive value so the
/// caller falls back to the default duration. Bare numbers are treated as ms
/// (lenient — matches how an author might write `duration = 250`).
export function parseDurationMs(value: string | undefined): number | null {
  if (value == null) return null;
  const trimmed = value.trim();
  const match = /^(-?\d*\.?\d+)\s*(ms|s)?$/i.exec(trimmed);
  if (!match) return null;
  const n = Number(match[1]);
  if (!Number.isFinite(n) || n <= 0) return null;
  const ms = match[2]?.toLowerCase() === "s" ? n * 1000 : n;
  return ms;
}

/// Resolve a theme's declared motion + the live reduced-motion state into the
/// concrete transition to play. Pure.
///
///   • no `motion` / no `view_transition` / `preset: "none"` → NO_TRANSITION.
///   • `prefers-reduced-motion` → a short fade (the a11y floor), regardless of
///     the declared preset/timing.
///   • otherwise → the declared preset, with `duration`/`easing` falling back
///     to the motion defaults when omitted.
export function resolveViewTransition(
  motion: ThemeMotion | undefined,
  reducedMotion: boolean,
): ResolvedViewTransition {
  const vt = motion?.view_transition;
  if (!vt || vt.preset === "none") return NO_TRANSITION;
  if (reducedMotion) {
    return { preset: "fade", durationMs: REDUCED_MOTION_FADE_MS, easing: "ease-out" };
  }
  const duration = parseDurationMs(vt.duration);
  const easing = vt.easing != null && vt.easing.trim().length > 0 ? vt.easing : DEFAULT_TRANSITION_EASING;
  return {
    preset: vt.preset,
    durationMs: duration ?? DEFAULT_TRANSITION_MS,
    easing,
  };
}

/// Resolve a theme's declared motion to a §2 `MotionSpec` for the `SpecTransition`
/// player (M-mod.1). The unified manifest→spec path: a theme may author the full
/// basis (`view_transition_spec`, which WINS) or the shortcut preset
/// (`view_transition`, mapped to the same basis via `presetToSpec`). Pure.
///
///   • `prefers-reduced-motion` → REDUCED_MOTION_SPEC (a short fade), regardless.
///   • `view_transition_spec` present → that spec.
///   • else `view_transition` preset → `presetToSpec` with its duration/easing.
///   • else (no motion / `preset: "none"`) → null (no animation).
export function resolveThemeMotionSpec(
  motion: ThemeMotion | undefined,
  reducedMotion: boolean,
): MotionSpec | null {
  if (reducedMotion) return REDUCED_MOTION_SPEC;
  // M-mod unified slot wins; falls back to the legacy view_transition* fields.
  if (motion?.transition != null) return resolveMotionRef(motion.transition);
  if (motion?.view_transition_spec) return motion.view_transition_spec;
  const vt = motion?.view_transition;
  if (!vt || vt.preset === "none") return null;
  const duration = parseDurationMs(vt.duration);
  const easing = vt.easing != null && vt.easing.trim().length > 0 ? vt.easing : undefined;
  return presetToSpec(vt.preset, {
    duration: duration ?? undefined,
    easing,
  });
}

/// Resolve a `MotionRef` (a named preset string OR an inline spec) to a concrete
/// `MotionSpec` (M-mod). The unified path for the `transition`/`selection`/
/// `ambient` slots. A string is looked up in `MOTION_PRESETS` (with optional
/// author overrides); a spec passes through; anything unresolved → null. Does NOT
/// apply the reduced-motion floor — the players (SpecTransition / AmbientMotion)
/// do, so an ambient can drop to nothing while a transition drops to a fade.
export function resolveMotionRef(
  ref: MotionRef | undefined,
  opts?: PresetOpts,
): MotionSpec | null {
  if (ref == null) return null;
  if (typeof ref === "string") return buildPreset(ref, opts);
  return ref; // already a MotionSpec
}

/// Read the OS "reduce motion" preference once (non-reactive). Guards for
/// non-DOM contexts (vitest/SSR) → `false`.
function prefersReducedMotion(): boolean {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") return false;
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/// A reactive accessor that tracks `prefers-reduced-motion`, updating when the
/// OS preference changes mid-session. Safe in non-DOM contexts (returns a
/// constant `false` signal). Registers its listener under the caller's owner so
/// it's cleaned up with the component.
export function usePrefersReducedMotion(): Accessor<boolean> {
  const [reduced, setReduced] = createSignal(prefersReducedMotion());
  if (typeof window !== "undefined" && typeof window.matchMedia === "function") {
    const mq = window.matchMedia("(prefers-reduced-motion: reduce)");
    const onChange = (): void => {
      setReduced(mq.matches);
    };
    mq.addEventListener?.("change", onChange);
    onCleanup(() => mq.removeEventListener?.("change", onChange));
  }
  return reduced;
}
