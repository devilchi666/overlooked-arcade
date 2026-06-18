// The named motion-preset registry (Theming ARC 3 Thrust M; audit §3 seed
// catalog). PRODUCT code. A preset = curated defaults over the §2 basis
// (`motionSpec.ts`) — the audit's whole premise: themes reference effects BY NAME
// (low floor) and may override the exposed params (high ceiling). This module is
// the catalog the manifest motion-section + resolver consume.
//
// SCOPE: the spec-expressible presets (one `MotionSpec` each) across three kinds —
//   • transition  — fired on a route/view change (SpecTransition)
//   • selection   — fired on focused-item change (SpecTransition, one-shot)
//   • ambient     — continuous idle loop (AmbientMotion; repeat:"infinite")
// Box-art treatments (gloss/reflection/parallax-tilt/shimmer), composites
// (lift-stagger/fanart-crossfade), and push-hero/attract are NOT spec presets —
// they live as components/hooks/treatments, registered separately.
//
// SPRINGS: the selection presets use a BACK-EASE overshoot curve (the bench's
// actual entrance choreography easing) rather than physics — a true physics
// spring is the higher-fidelity escape hatch via `createSpringValue`.

import { DEFAULT_SPEC_EASING, type MotionSpec } from "./motionSpec";

/// Back-ease with overshoot — the F10 bench's entrance curve (`art`/`title` rise).
export const BACK_EASE = "cubic-bezier(0.34, 1.7, 0.64, 1)";

export type MotionPresetKind = "transition" | "selection" | "ambient";

/// Author-exposable overrides. A preset reads only the ones it exposes; the rest
/// are ignored. (Mirrors the audit's "each preset lists its exposed params".)
export type PresetOpts = {
  duration?: number;
  easing?: string;
  /// Travel distance in px (slide / title-rise / float).
  distance?: number;
  /// Peak scale (lift / breathe).
  peak?: number;
  /// Glow colour (glow-pulse) — pass the system accent for themed glows.
  color?: string;
};

export type MotionPreset = {
  kind: MotionPresetKind;
  /// Build the concrete spec, applying any author overrides over the defaults.
  build: (opts?: PresetOpts) => MotionSpec;
};

const ambient = (channels: MotionSpec["channels"], duration: number): MotionSpec => ({
  duration,
  easing: "ease-in-out",
  repeat: "infinite",
  direction: "alternate",
  channels,
});

/// The catalog. Keyed by preset name (the string a theme authors).
export const MOTION_PRESETS: Record<string, MotionPreset> = {
  // --- transitions (route/view change) ---
  fade: {
    kind: "transition",
    build: (o = {}) => ({ duration: o.duration ?? 320, easing: o.easing ?? DEFAULT_SPEC_EASING, channels: { opacity: [0, 1] } }),
  },
  slide: {
    kind: "transition",
    build: (o = {}) => ({
      duration: o.duration ?? 320,
      easing: o.easing ?? DEFAULT_SPEC_EASING,
      channels: { opacity: [0, 1], y: [o.distance ?? 96, 0] },
    }),
  },
  scale: {
    kind: "transition",
    build: (o = {}) => ({
      duration: o.duration ?? 320,
      easing: o.easing ?? DEFAULT_SPEC_EASING,
      channels: { opacity: [0, 1], scale: [0.92, 1] },
    }),
  },
  flip: {
    kind: "transition",
    build: (o = {}) => ({
      duration: o.duration ?? 460,
      easing: o.easing ?? DEFAULT_SPEC_EASING,
      channels: { opacity: [0, 1], rotateY: [-90, 0] },
    }),
  },

  // --- selection choreography (focused-item change, one-shot) ---
  lift: {
    kind: "selection",
    build: (o = {}) => ({ duration: o.duration ?? 260, easing: o.easing ?? BACK_EASE, channels: { scale: [1, o.peak ?? 1.08] } }),
  },
  "art-grow-in": {
    kind: "selection",
    build: (o = {}) => ({
      duration: o.duration ?? 440,
      easing: o.easing ?? BACK_EASE,
      channels: { opacity: [0, 1], scale: [0.9, 1] },
    }),
  },
  "title-rise": {
    kind: "selection",
    build: (o = {}) => ({
      duration: o.duration ?? 440,
      easing: o.easing ?? BACK_EASE,
      channels: { opacity: [0, 1], y: [o.distance ?? 22, 0] },
    }),
  },

  // --- ambient (idle loops) ---
  breathe: {
    kind: "ambient",
    build: (o = {}) => ambient({ scale: [1, o.peak ?? 1.03] }, o.duration ?? 2600),
  },
  float: {
    kind: "ambient",
    build: (o = {}) => ambient({ y: [0, -(o.distance ?? 10)] }, o.duration ?? 2400),
  },
  "glow-pulse": {
    kind: "ambient",
    build: (o = {}) => {
      const color = o.color ?? "rgba(140, 180, 255, 0.7)";
      return ambient({ boxShadow: [`0 0 8px 0 ${color}`, `0 0 30px 8px ${color}`] }, o.duration ?? 1800);
    },
  },
  "ken-burns": {
    kind: "ambient",
    build: (o = {}) => ambient({ scale: [1.12, 1.22], x: [0, -8], y: [0, -6] }, o.duration ?? 20000),
  },
};

/// The preset names, for the validator's allow-list and any pickers.
export const MOTION_PRESET_NAMES: readonly string[] = Object.keys(MOTION_PRESETS);

/// Build a named preset to a concrete spec, or `null` for an unknown name.
export function buildPreset(name: string, opts?: PresetOpts): MotionSpec | null {
  return MOTION_PRESETS[name]?.build(opts) ?? null;
}

/// `lift-stagger` / staggered entrance (audit §3): the per-child delay (ms) for
/// cascading a selection preset across a list's children — apply `title-rise`
/// (or `lift`) to each child with this delay and the row settles in sequence.
/// `base` is the lead-in before the first child; `step` the gap between children.
export function staggerMs(index: number, step = 70, base = 0): number {
  return base + index * step;
}
