// Theme host context — the engine-provided service contract EVERY theme
// consumes (not just Retroverse).
//
// Born as the Retroverse-UI shared context (RetroverseContext), renamed
// ThemeContext in Theming Substrate ARC 1 Phase 2 Slice C because the shape
// was already theme-generic, and RELOCATED here (routes/retroverse/context.tsx
// → platform/theme/host.tsx) in Phase 3 S2 (the walking skeleton). The move
// is what makes it a real SDK contract: a second theme (Wheel) needs the same
// launch / saves / info / favorite host services Retroverse does, and a theme
// can't import from routes/retroverse/ (theme ↛ theme). So the host services
// belong in platform, where any theme may reach them. App.tsx stays the
// provider — it IS the host that wires real behaviour into this surface.
//
// routes/retroverse/context.tsx now re-exports from here (D15-style move +
// re-export) so Retroverse's ~11 existing useTheme() importers don't churn.
//
// Stores vs handlers: per DECISIONS D11, pervasive STORES are read via
// usePlatform(); this context carries the few APP-SPECIFIC HANDLERS (launch,
// dialogs, favorites…) a theme invokes. The store fields are still present on
// the value for back-compat with Retroverse's existing reads; new theme code
// should prefer usePlatform() for stores and useTheme() for handlers.

import { createContext, useContext, type ParentComponent } from "solid-js";
import type { Accessor } from "solid-js";
import type { LibraryStore } from "@oa/platform/library/store";
import type { CustomCollectionsStore } from "@oa/platform/library/customCollections";
import type { LayoutStore } from "@oa/platform/layout/state";
import type { ViewsStore } from "@oa/platform/views/store";
import type { SettingsStore } from "@oa/platform/settings/store";
import type { SidebarView } from "@oa/platform/layout/types";
import type { RomEntry } from "@oa/platform/library/types";
import { engineSurfaceOpen } from "@oa/platform/engineSurface";

export type ThemeContextValue = {
  library: LibraryStore;
  /// Retroverse-UI Phase C3 Slice 12 — operator-built collections.
  /// Companion to `library`; lives alongside so the TileContextMenu
  /// + CollectionsPage can read membership state without prop-drilling
  /// through every consumer.
  customCollections: CustomCollectionsStore;
  layout: LayoutStore;
  views: ViewsStore;
  settings: SettingsStore;
  searchQuery: Accessor<string>;
  setSearchQuery: (s: string) => void;
  focusedEntry: Accessor<RomEntry | null>;
  setFocusedEntry: (e: RomEntry | null) => void;
  currentView: Accessor<SidebarView>;
  setCurrentView: (v: SidebarView) => void;
  onLaunch: (entry: RomEntry, slot?: number, stateFile?: string) => Promise<unknown>;
  /// Boot a system's libretro core with no content so it opens its own
  /// built-in browser/menu (DOSBox-Pure, ScummVM). Surfaced by
  /// SystemHeader's "Boot without game" button, shown only when
  /// `systemSupportsBootless(systemId)` returns true. Sourced from
  /// App.handleBootWithoutGame.
  onBootWithoutGame: (systemId: string) => void;
  onShowSaves: (entry: RomEntry) => void;
  onShowInfo: (entry: RomEntry) => void;
  onPickContext: (entry: RomEntry, position: { x: number; y: number }) => void;
  onPickFolder: () => Promise<unknown> | void;
  /// Post-launch UI bridge — sets gameRunning / runningEntry /
  /// currentRomTitle, hides the library overlay in single-window mode.
  /// Mirrors what App.tsx passes to GameInfoModal as `onLaunched`.
  /// Tile-click launches go through `onLaunch` (App.handleLaunch)
  /// which already does the same work inline; this exists for surfaces
  /// where the launch sidesteps handleLaunch (e.g. the GameInfoModal's
  /// own Launch / Resume buttons).
  onPostLaunch: (entry: RomEntry, slot?: number) => void;
  /// Retroverse-UI Phase C3 — flip favorite state for a tile / detail-
  /// panel game. Sourced from `library.setFavorite` in App.tsx; passed
  /// through to LibraryView in LibraryPage and to CollectionsPage's
  /// inline tile grid in the COLLECTIONS tab.
  onToggleFavorite: (entry: RomEntry, value: boolean) => void;
  /// Retroverse-UI Phase C3 — flip completed state. Same shape as
  /// onToggleFavorite. Used by COLLECTIONS context menu + future
  /// completion celebrations.
  onToggleCompleted: (entry: RomEntry, value: boolean) => void;
  /// SETTINGS → Library wrap — invoked when the operator clicks the
  /// "Add" button in the Library folders header. Opens the OS folder
  /// picker; on selection adds + scans the folder. Sourced from
  /// App.handleAddLibraryFolder so the embedded panel reuses the
  /// same picker UX the legacy LibraryManagerPage entry uses.
  onAddLibraryFolder: () => void;
  /// SETTINGS → Library wrap — invoked when the operator clicks the
  /// "Rescan all" button. Sourced from App.handleRescanLibraryFolders;
  /// disabled by the page when no folders are tracked.
  onRescanLibraryFolders: () => void;
  /// Phase C3 Slice 12 — open the NewCollectionDialog. `seedRomId` is
  /// non-null when the open call came from a tile-context-menu
  /// "+ New collection…" tail entry, so the dialog seeds the new
  /// collection with that rom on create. Used by CollectionsPage's
  /// "+ New collection" sidebar button (passes null).
  onOpenNewCollection: (seedRomId: string | null) => void;
  /// Phase C3 Slice 12 — open the same dialog in rename mode for an
  /// existing collection. Used by the right-click context menu on a
  /// custom-collection sidebar row.
  onOpenRenameCollection: (collectionId: string, currentName: string) => void;
  /// Game-focus state — true when the operator has flipped keyboard
  /// passthrough ON (Ctrl+G) so OA hotkeys go to the running core
  /// instead of the shell. RetroverseShell renders a small status pill
  /// in the header when this is true; mirrors the legacy toolbarRight
  /// indicator at App.tsx:1457-1464.
  gameFocus: Accessor<boolean>;
  /// Quit handler — fires `invoke("quit_app")`. Bound to the ✕ button
  /// in the RetroverseShell header; mirrors the legacy toolbarRight
  /// Quit button at App.tsx:1505-1515. Ctrl+Q keyboard shortcut works
  /// in both modes via App.tsx's keydown handler.
  onQuit: () => void;
  /// Open the live debug-log dialog (Rust + frontend log ring). In
  /// the legacy Shell this was reachable from Help → Debug log…;
  /// Retroverse exposes the same dialog via a button in SETTINGS →
  /// About → Report a bug card so operator bug-report flow doesn't
  /// regress.
  onOpenDebugLog: () => void;
  /// Open the Keyboard shortcuts cheatsheet. Same migration story as
  /// onOpenDebugLog — legacy Shell's Help menu had it; Retroverse
  /// surfaces it from SETTINGS → About.
  onOpenKeyboardShortcuts: () => void;
  /// Phase 1B Slice 1 (Guided Setup arc) — re-establish a path to the
  /// Import Wizard. The 4-step modal lives in `ImportWizard.tsx`; the
  /// legacy toolbar that opened it was deleted with the 2026-05-31
  /// legacy-Shell removal, leaving the wizard orphaned. Settings →
  /// Library now exposes a "Re-scan with smart detection" card whose
  /// primary button calls this handler.
  onOpenImportWizard: () => void;
};

const ThemeContext = createContext<ThemeContextValue>();

export const ThemeProvider: ParentComponent<{ value: ThemeContextValue }> = (props) => {
  return (
    <ThemeContext.Provider value={props.value}>
      {props.children}
    </ThemeContext.Provider>
  );
};

/// Read the theme host context. Throws if called outside the provider —
/// the throw is intentional: rendering a theme surface outside the
/// provider is a wiring bug, not a runtime concern.
export function useTheme(): ThemeContextValue {
  const ctx = useContext(ThemeContext);
  if (!ctx) {
    throw new Error("useTheme: must be used inside <ThemeProvider>");
  }
  return ctx;
}

/// DECISIONS D20(a) seam — the GENERAL "platform has preempted the active
/// theme's surface" predicate. A theme makes its global input handlers
/// (e.g. Wheel's coverflow nav, Retroverse's L1/R1 tab cycling) inert while
/// this is true, instead of each shell checking `engineSurfaceOpen()`
/// directly. Today the engine takeover (F12 / Settings / Select+Start) is
/// the one preemptor; attract mode (ARC 2-3) reuses this exact signal with
/// no theme-side change. Written general now so attract slots in for free
/// rather than special-casing F12.
export const themePreempted = (): boolean => engineSurfaceOpen();
