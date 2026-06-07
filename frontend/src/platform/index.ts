// Platform — barrel export.
//
// Themes import only from `@oa/platform` (Vite alias to this directory).
// Engine code can import from either side. The alias + this barrel
// were added in Theming Substrate ARC 1 Phase 2 Slice A; future Phase 2
// slices migrate stores (settings / library / layout / views /
// customCollections), shared components (LibraryTile / LibraryView /
// LeftSidebar / perSystemSections), the per-system registry, and the
// shared lib helpers into this directory.
//
// Phase 1 + 2 Slice A surface (today):
// - engineSurface — fullscreen takeover visibility + summon helpers
// - dialogs — open/close state for the engine-anchored + per-game
//   dialogs (5 from Phase 1, ~10 more from Phase 2 Slice A)

export * as engineSurface from "./engineSurface";
export * as dialogs from "./dialogs";
