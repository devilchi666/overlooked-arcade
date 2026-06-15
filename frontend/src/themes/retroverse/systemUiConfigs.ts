// Retroverse's per-system experiential UI overrides (Theming ARC 2 L2b / D34).
//
// This is THEME content — the per-system *values* that used to live in the
// platform-global `systemUIConfigs` map. The platform owns the *capability*
// (the `SystemUIConfig` type + `BASELINE_UI` + the resolver); Retroverse, the
// flagship, owns the curated per-system character. Exposed on Retroverse's
// `ThemePackage.perSystemUiConfigs`; App.tsx bridges it into `uiConfigFor`,
// which merges each entry over `BASELINE_UI`.
//
// Only systems that DIFFER from `BASELINE_UI` need an entry — every other
// system inherits the baseline. Today that's the three Stage 1 pilots
// (per docs/PLANS/per-system-ui.md §8); Stage 2+ showcase tuning (L6) adds more.
// Note these are only consumed because Retroverse opts into per-system UI (L1
// `per_system_ui`); a theme that doesn't opt in renders uniform regardless.

import type { PerSystemUiConfigs } from "@oa/platform/themes/systemUIConfigs";

export const RETROVERSE_SYSTEM_UI: PerSystemUiConfigs = {
  // Pilot 1: Game Boy — soft / minimal / personal
  gb: {
    interactionStyle: "delayed", // LCD-persistence feel
    tileShape: "portrait-3:4", // handheld convention
    buttonLabels: "nintendo-handheld",
  },

  // Pilot 2: NES — classic / bright / instant
  nes: {
    audioProfile: "console", // toy-piano boops on nav
    background: "animated", // scrolling NES-palette pattern
    buttonLabels: "nintendo-console",
  },

  // Pilot 3: Vectrex — vector phosphor signature
  vectrex: {
    audioProfile: "console", // synthesized vector blips
    background: "shader", // phosphor-screen WGSL shader
    interactionStyle: "physical", // vector-pulse + glow bloom
    tileShape: "square",
    transitionTiming: "slow", // cinematic feel
    buttonLabels: "vectrex-custom",
  },
};
