// Theme SDK contract — the typed shape of a whole-shell theme package.
//
// Theming Substrate ARC 1 Phase 3 S2 (the walking skeleton / swap gate,
// docs/PLANS/theming-substrate.md §13.3). A theme is NOT a framework: it's
// a small package object pairing a manifest with an entry component. The
// entry renders a NAMED SURFACE and consumes only the platform layer
// (usePlatform stores + the host context + @oa/platform/nav +
// @oa/platform/api) — never engine/, never another theme.
//
// Two D20 seams are baked into this contract so the expensive retrofits
// (attract mode, multi-monitor) become additive later instead of rewrites:
//   - ThemeSurface (D20b): the entry is surface-aware from day one. ARC 1
//     honors exactly "main"; marquee/manuals/second-controls widen the union
//     later and existing themes just declare more surfaces in their manifest.
//   - The general preempt/restore lifecycle (D20a) lives in host.tsx's
//     `themePreempted()`, not in this type — the entry stays declarative.

import type { Component } from "solid-js";
import type { ThemeManifest, ThemeSurface } from "./manifest";

// ThemeSurface is defined in manifest.ts (a manifest concept — the
// `surfaces` field lists them) and re-exported here so theme entry code can
// import surface + entry types from one place.
export type { ThemeSurface };

/// Props every theme entry component receives. Surface-aware by design
/// (D20b) — the entry switches its layout on `surface`. In ARC 1 it's
/// always `"main"`.
export type ThemeEntryProps = { surface: ThemeSurface };

/// A theme's root component. Receives only ThemeEntryProps; reads stores
/// via usePlatform(), host services via useTheme() (platform/theme/host),
/// nav via @oa/platform/nav, backend via @oa/platform/api.
export type ThemeEntry = Component<ThemeEntryProps>;

/// A built-in (ARC 1: build-time bundled, DECISIONS D6) whole-shell theme.
export type ThemePackage = {
  /// Parsed manifest. In S2 these are authored inline as typed objects;
  /// Phase 5 reads them from each theme's `theme.toml`.
  manifest: ThemeManifest;
  /// The theme's root component, mounted by App.tsx for the active theme.
  entry: ThemeEntry;
};
