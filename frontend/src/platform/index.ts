// Platform — barrel export.
//
// Themes import only from `@oa/platform` (Vite alias to this directory).
// Engine code can import from either side. The alias + this barrel were
// added in Theming Substrate ARC 1 Phase 2 Slice A; Slice B moved the
// stores (settings / library / layout / views), lib helpers, and the
// per-system theme registry in; Slice C moved the shared component
// surface (LibraryTile / LibraryView / LeftSidebar / Dialog / etc. —
// see ./components/) and added the Theme SDK manifest type
// (./theme/manifest.ts).
//
// Deep imports (`@oa/platform/library/store`, `@oa/platform/components/
// Dialog`) are the norm for code; this barrel re-exports the small
// named surface that's convenient to grab unqualified.
//
// Namespaced surface:
// - engineSurface — fullscreen takeover visibility + summon helpers
// - dialogs — open/close state for the engine-anchored + per-game
//   dialogs (Platform owns open/close; themes pick anchors)

export * as engineSurface from "./engineSurface";
export * as dialogs from "./dialogs";

// Theme SDK — manifest contract for theme packages (no loader until
// Phase 5; ARC 1 themes are build-time bundled).
export type {
  ThemeManifest,
  ThemeContextSlot,
  ReservedCorner,
} from "./theme/manifest";
