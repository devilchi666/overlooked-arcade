// Per-system layout resolver (Theming ARC 2 L3 / D32).
//
// Answers "which layout primitive renders <view> for <system>?" via the D32
// cascade — the same "resolve by active system" shape as the S5.1 asset and
// S5.2 palette resolvers, with the user-override tier on top (D18's per-user
// agency over a theme's defaults):
//
//   user override → theme views[view].per_system[system]
//                 → theme views[view].layout → engine default
//
// L3a ships the resolver + the persisted override store (layoutOverrides.ts) +
// the reactive hook — NO consumer yet. L3b wires `useResolvedLayout` into the
// game-browse view (keyed on the current system) and settles how it relates to
// the existing global capsule/list viewMode toggle; L4 builds the `wheel`
// primitive; L5 builds the user-facing override UI.

import { createMemo, type Accessor } from "solid-js";
import type { LayoutPrimitive, ThemeViews, ViewType } from "./manifest";
import type { SystemId } from "@oa/platform/themes/registry";
import { activeTheme, activeThemeId } from "./registry";
import { getLayoutOverride } from "./layoutOverrides";

/// The engine's fallback layout per view — the bottom of the cascade, used when
/// neither the user nor the active theme declares one. `game-details` defaults to
/// `custom` (a detail surface is theme-drawn, not a list/grid); the browse views
/// default to `grid`. The L3 resolver iterates this set; honored view types grow
/// over ARC 2 (L2a stamped the contract for the full set).
export const ENGINE_DEFAULT_LAYOUTS: Record<ViewType, LayoutPrimitive> = {
  "manufacturer-browse": "grid",
  "system-browse": "grid",
  "game-browse": "grid",
  "game-details": "custom",
};

/// PURE cascade — no reactivity, no registry/store reads. Given the resolved
/// inputs (the user override if any, the active theme's `views` map, the view,
/// and the system), return the effective layout primitive. Tested directly;
/// `useResolvedLayout` wires the reactive sources into it.
export function resolveLayout(args: {
  override?: LayoutPrimitive;
  themeViews?: ThemeViews;
  view: ViewType;
  systemId?: SystemId | null;
}): LayoutPrimitive {
  const { override, themeViews, view, systemId } = args;
  // 1. user override (highest)
  if (override) return override;
  // 2/3. the active theme's per-view declaration
  const v = themeViews?.[view];
  if (v) {
    // 2. per-system override within the theme (only when a system is in context)
    if (systemId) {
      const perSystem = v.per_system?.[systemId];
      if (perSystem) return perSystem;
    }
    // 3. the theme's default layout for this view
    if (v.layout) return v.layout;
  }
  // 4. engine default (lowest)
  return ENGINE_DEFAULT_LAYOUTS[view];
}

/// Reactive resolver for the ACTIVE theme + a (reactive) system context. Reads
/// the user override (keyed by the active theme id), the active theme's `views`
/// map, and the engine defaults. `systemId` is an accessor so the result
/// re-resolves as the browse view's current system changes; pass `() => null`
/// for view-level resolution with no system in context (the theme/engine view
/// default).
export function useResolvedLayout(
  view: ViewType,
  systemId: Accessor<SystemId | null | undefined>,
): Accessor<LayoutPrimitive> {
  return createMemo(() => {
    const themeId = activeThemeId();
    const sys = systemId() ?? null;
    const override =
      themeId && sys ? getLayoutOverride(themeId, sys, view) : undefined;
    return resolveLayout({
      override,
      themeViews: activeTheme()?.manifest.views,
      view,
      systemId: sys,
    });
  });
}
