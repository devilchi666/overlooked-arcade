// Retroverse-UI Phase B Slice 6 — shared context for Retroverse pages.
//
// Centralizes the state + handlers each Retroverse tab needs to read
// or invoke (library / layout / views / settings stores; searchQuery
// + focusedEntry signals; launch / saves / info / context-menu /
// pick-folder callbacks; the legacy currentView signal so the LIBRARY
// tab can filter by system the same way today's LibraryView does).
//
// App.tsx provides the value via <RetroverseProvider value={...}>;
// individual tabs consume it via `useRetroverse()`. Avoids prop-
// drilling through RetroverseShell into every tab + future-proofs
// Phase C tabs (HOME / COLLECTIONS / etc.) — each just reaches into
// the context for whichever slice it needs.

import { createContext, useContext, type ParentComponent } from "solid-js";
import type { Accessor } from "solid-js";
import type { LibraryStore } from "../../library/store";
import type { CustomCollectionsStore } from "../../library/customCollections";
import type { LayoutStore } from "../../layout/state";
import type { ViewsStore } from "../../views/store";
import type { SettingsStore } from "../../settings/store";
import type { SidebarView } from "../../layout/LeftSidebar";
import type { RomEntry } from "../../library/types";

export type RetroverseContextValue = {
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
};

const RetroverseContext = createContext<RetroverseContextValue>();

export const RetroverseProvider: ParentComponent<{ value: RetroverseContextValue }> = (props) => {
  return (
    <RetroverseContext.Provider value={props.value}>
      {props.children}
    </RetroverseContext.Provider>
  );
};

/// Read the Retroverse context. Throws if called outside the provider —
/// the throw is intentional: rendering a Retroverse page outside the
/// provider is a wiring bug, not a runtime concern.
export function useRetroverse(): RetroverseContextValue {
  const ctx = useContext(RetroverseContext);
  if (!ctx) {
    throw new Error("useRetroverse: must be used inside <RetroverseProvider>");
  }
  return ctx;
}
