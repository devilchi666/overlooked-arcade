// Theme host context — re-export shim.
//
// The context MOVED to platform/theme/host.tsx in Theming Substrate ARC 1
// Phase 3 S2 (the walking skeleton). It's a platform-layer SDK contract now
// — every theme (Retroverse + the new Wheel) consumes the same host services
// — so it can't live under routes/retroverse/ (a theme can't import another
// theme's private modules). This file stays as a back-compat re-export so
// Retroverse's ~11 existing `useTheme()` import sites keep working unchanged
// (D15-style move + re-export). New theme code should import directly from
// `@oa/platform/theme/host`.

export { ThemeProvider, useTheme, themePreempted } from "@oa/platform/theme/host";
export type { ThemeContextValue } from "@oa/platform/theme/host";
