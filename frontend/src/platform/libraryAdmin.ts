// Platform — library-admin action registry.
//
// The "add a library folder" and "rescan all folders" actions are
// App-scoped: they drive App-local UI state (status / busy toasts, the
// scan-progress reporter, post-ingest auto-sync) and there is exactly one
// library per running app. The engine Library surface
// (engine/SettingsSections → engine/LibraryManagerPage) needs to invoke them
// without importing App.tsx or the theme context — doing so would invert the
// layer boundary (engine must not import theme/App).
//
// App.tsx registers the concrete implementations on mount via
// registerLibraryAdmin(); engine code calls addLibraryFolder() /
// rescanLibraryFolders(). This mirrors the module-singleton pattern already
// used by platform/engineSurface.ts and platform/dialogs.ts: platform owns a
// stable surface, App wires the behaviour into it.
//
// Per DECISIONS D10 the alternative — prop-threading the two handlers from
// App → EngineManagerSurface → SettingsPanel → SettingsSections — was
// rejected: three layers of prop drilling for two App-scoped singletons,
// when a registered service expresses the actual ownership (App owns the
// behaviour; engine just triggers it) with no churn.

type LibraryAdminActions = {
  /// Open an OS folder picker, add the chosen folder to the library, and
  /// scan it. No-op until App registers the implementation.
  addLibraryFolder: () => void | Promise<void>;
  /// Re-scan every already-registered library folder. No-op until App
  /// registers the implementation.
  rescanLibraryFolders: () => void | Promise<void>;
};

const noop = () => {};
let registered: LibraryAdminActions = {
  addLibraryFolder: noop,
  rescanLibraryFolders: noop,
};

/// App.tsx calls this once on mount to supply the real behaviour.
export function registerLibraryAdmin(actions: LibraryAdminActions): void {
  registered = actions;
}

/// Trigger the "add a library folder" flow. Safe to call before
/// registration (no-op).
export function addLibraryFolder(): void | Promise<void> {
  return registered.addLibraryFolder();
}

/// Trigger the "rescan all folders" flow. Safe to call before registration
/// (no-op).
export function rescanLibraryFolders(): void | Promise<void> {
  return registered.rescanLibraryFolders();
}
