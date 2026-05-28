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
import type { LayoutStore } from "../../layout/state";
import type { ViewsStore } from "../../views/store";
import type { SettingsStore } from "../../settings/store";
import type { SidebarView } from "../../layout/LeftSidebar";
import type { RomEntry } from "../../library/types";

export type RetroverseContextValue = {
  library: LibraryStore;
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
  /// Post-launch UI bridge — what App.tsx does after a successful launch
  /// (sets gameRunning / runningEntry / currentRomTitle, hides the
  /// library overlay in single-window mode, etc.). Mirrors what
  /// App.tsx already passes to GameInfoModal as `onLaunched`. LIBRARY
  /// tab passes this to RightDetailPanel's `onLaunched` so launching
  /// from the persistent detail pane keeps the App-level shell state
  /// in sync. Tile-click launches don't need this — they go through
  /// `onLaunch` (App.handleLaunch) which already does the same work.
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
