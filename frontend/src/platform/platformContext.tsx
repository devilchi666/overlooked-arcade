// Platform context — the store + shared-state layer the WHOLE app reads,
// independent of any theme.
//
// Why this exists (theming-substrate boundary, DECISIONS D11, 2026-06-09):
// the `ThemeContext` (routes/retroverse/context) historically bundled the
// platform STORES (library / settings / layout / views / collections) +
// shared selection state together with theme/gameplay handlers. That forced
// any component needing a store — including ENGINE-surface components like
// SettingsPanel — to import the theme context, inverting the layer boundary
// (engine/platform must not depend on theme).
//
// `usePlatform()` is the theme-agnostic accessor for that store + state
// layer. Engine and platform components read stores from HERE; only theme
// code (routes/) uses `useTheme()`. App.tsx provides BOTH from the same
// store instances during the transition, so existing `useTheme().settings`
// reads in theme code keep working unchanged while engine/platform code
// migrates to `usePlatform()`.

import { createContext, useContext, type ParentComponent, type Accessor } from "solid-js";
import type { LibraryStore } from "@oa/platform/library/store";
import type { CustomCollectionsStore } from "@oa/platform/library/customCollections";
import type { LayoutStore } from "@oa/platform/layout/state";
import type { ViewsStore } from "@oa/platform/views/store";
import type { SettingsStore } from "@oa/platform/settings/store";
import type { SidebarView } from "@oa/platform/layout/types";
import type { RomEntry } from "@oa/platform/library/types";

/// The platform store + shared-state surface. Pure data/state — no
/// theme/gameplay handlers (those stay on `ThemeContextValue`). Mirrors the
/// store fields `ThemeContextValue` exposes; both are populated from the
/// same instances in App.tsx.
export type PlatformContextValue = {
  library: LibraryStore;
  customCollections: CustomCollectionsStore;
  layout: LayoutStore;
  views: ViewsStore;
  settings: SettingsStore;
  /// Library search box text (shared across the shell).
  searchQuery: Accessor<string>;
  setSearchQuery: (s: string) => void;
  /// The tile/entry the operator has focused (drives the detail pane).
  focusedEntry: Accessor<RomEntry | null>;
  setFocusedEntry: (e: RomEntry | null) => void;
  /// The active sidebar view-node selection.
  currentView: Accessor<SidebarView>;
  setCurrentView: (v: SidebarView) => void;
};

const PlatformContext = createContext<PlatformContextValue>();

export const PlatformProvider: ParentComponent<{ value: PlatformContextValue }> = (
  props,
) => {
  return (
    <PlatformContext.Provider value={props.value}>
      {props.children}
    </PlatformContext.Provider>
  );
};

/// Read the platform store + shared-state layer. Throws if called outside
/// the provider — that's a wiring bug (App.tsx must wrap the tree). Engine
/// and platform components use this; theme code may use `useTheme()`.
export function usePlatform(): PlatformContextValue {
  const ctx = useContext(PlatformContext);
  if (!ctx) {
    throw new Error("usePlatform() called outside <PlatformProvider>");
  }
  return ctx;
}
