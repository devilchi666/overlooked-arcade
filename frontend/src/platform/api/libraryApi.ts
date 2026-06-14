// Typed Tauri bridge — library domain (games / folders / groups / migration).
//
// Theming Phase 4 Slice 2. The store-heavy core: the games table, the
// variant/identity groups, the tracked library folders + their scan rules,
// and the one-shot localStorage→SQLite migrations. Same convention as
// settingsApi (see docs/PLANS/theming-platform-api-bridge.md): one typed
// named export per command, thin pass-through, command string lives only here.

import { invoke } from "@tauri-apps/api/core";
import type { GameGroupInfo, RomEntry, RomId } from "@oa/platform/library/types";
import type { SystemId } from "@oa/platform/themes/registry";
import type { LibraryFolderRow } from "@oa/platform/settings/store";

// --- Games ---------------------------------------------------------------

/// The authoritative game list (the store's in-memory mirror hydrates from this).
export function listGames(): Promise<RomEntry[]> {
  return invoke<RomEntry[]>("list_games");
}

/// Fetch a single game row by id (`null` if not found).
export function getGame(id: string): Promise<RomEntry | null> {
  return invoke<RomEntry | null>("get_game", { id });
}

/// Insert scanned ROMs (Rust dedups by file_path); returns the count added.
export function addGames(entries: RomEntry[]): Promise<number> {
  return invoke<number>("add_games", { entries });
}

/// Drop the fresh-install seed rows on first real ingest; returns count removed.
export function dropSeedGames(): Promise<number> {
  return invoke<number>("drop_seed_games");
}

/// Delete a single game by id.
export function deleteGame(id: RomId): Promise<void> {
  return invoke("delete_game", { id });
}

/// Delete every game (the "Reset entire library" action); returns count removed.
export function deleteAllGames(): Promise<number> {
  return invoke<number>("delete_all_games");
}

/// Delete every game tagged with `systemId`; returns count removed.
export function deleteGamesForSystem(systemId: string): Promise<number> {
  return invoke<number>("delete_games_for_system", { systemId });
}

/// Resolve a game id from its on-disk path (`null` if untracked).
export function findGameIdByPath(path: string): Promise<string | null> {
  return invoke<string | null>("find_game_id_by_path", { path });
}

/// Set or clear the per-game core override (`null` to revert to the default).
export function updateGameCoreOverride(id: RomId, value: string | null): Promise<void> {
  return invoke("update_game_core_override", { id, value });
}

/// Flip the favorite flag for a game.
export function updateGameFavorite(id: RomId, value: boolean): Promise<void> {
  return invoke("update_game_favorite", { id, value });
}

/// Flip the completed flag for a game.
export function updateGameCompleted(id: RomId, value: boolean): Promise<void> {
  return invoke("update_game_completed", { id, value });
}

// --- Variant / identity groups ------------------------------------------

/// The grouped (identity-backed) view of the library.
export function listGameGroups(): Promise<GameGroupInfo[]> {
  return invoke<GameGroupInfo[]>("list_game_groups");
}

/// Pin a (system, base title) group to a specific variant.
export function setGameGroupDefault(
  systemId: SystemId,
  baseTitle: string,
  preferredGameId: RomId,
): Promise<void> {
  return invoke("set_game_group_default", { systemId, baseTitle, preferredGameId });
}

/// Clear a group's pinned variant.
export function clearGameGroupDefault(systemId: SystemId, baseTitle: string): Promise<void> {
  return invoke("clear_game_group_default", { systemId, baseTitle });
}

// --- Library folders -----------------------------------------------------

/// Args for `add_folder` (mirrors the Rust command's fields).
export type AddFolderArgs = {
  path: string;
  scanSubfolders: boolean;
  subfoldersAreSystems: boolean;
  watchEnabled: boolean;
};

/// Partial field set for `update_folder` (any omitted field is left unchanged).
export type UpdateFolderFields = {
  scanSubfolders?: boolean;
  subfoldersAreSystems?: boolean;
  watchEnabled?: boolean;
  lastScannedAt?: number;
};

/// One scan-rule row for `set_folder_rules`.
export type FolderRuleInput = {
  folderId: string;
  matchPattern: string;
  systemId: string;
};

/// List the tracked library folders. `includeRules` controls whether each
/// row carries its scan rules. Call sites pass their own row view via `T`
/// (`LibraryFolderRow` in the settings store, the richer `Folder` in the
/// import wizard).
export function listFolders<T = LibraryFolderRow>(includeRules: boolean): Promise<T[]> {
  return invoke<T[]>("list_folders", { includeRules });
}

/// Track a new library folder; returns the created row (caller picks `T`).
export function addFolder<T = LibraryFolderRow>(args: AddFolderArgs): Promise<T> {
  return invoke<T>("add_folder", args);
}

/// Stop tracking a folder by id.
export function removeFolder(id: string): Promise<void> {
  return invoke("remove_folder", { id });
}

/// Update a tracked folder's scan fields.
export function updateFolder(id: string, fields: UpdateFolderFields): Promise<void> {
  return invoke("update_folder", { id, fields });
}

/// Reorder the tracked folders.
export function reorderFolders(orderedIds: string[]): Promise<void> {
  return invoke("reorder_folders", { orderedIds });
}

/// Replace a folder's scan rules (empty array clears them).
export function setFolderRules(folderId: string, rules: FolderRuleInput[]): Promise<void> {
  return invoke("set_folder_rules", { folderId, rules });
}

/// Update the filesystem watcher's tracked folders + extensions.
export function setWatchedFolders(folders: string[], extensions: string[]): Promise<void> {
  return invoke("set_watched_folders", { folders, extensions });
}

/// Read-only preview of relinking a moved folder (Settings IA Slice 2): how
/// many tracked ROMs are present at `newPath`. No DB write.
export type RepointPreview = {
  total: number;
  matched: number;
  missing: number;
  sampleMissing: string[];
  oldPath: string;
  newPath: string;
};

/// Result of a committed relink: the updated folder row + rebased game count.
export type RepointResult<T = LibraryFolderRow> = {
  folder: T;
  gamesUpdated: number;
};

/// Preview relinking folder `folderId` to `newPath` (same ROMs, new path).
export function previewRepointFolder(folderId: string, newPath: string): Promise<RepointPreview> {
  return invoke<RepointPreview>("preview_repoint_folder", { folderId, newPath });
}

/// Commit the relink: rebase the folder + its games' paths in place. Game ids
/// stay stable so covers/metadata/favorites/play-time survive.
export function repointFolder<T = LibraryFolderRow>(
  folderId: string,
  newPath: string,
): Promise<RepointResult<T>> {
  return invoke<RepointResult<T>>("repoint_folder", { folderId, newPath });
}

/// True when a directory has no importable content (pre-flight for add).
export function directoryIsEmpty(path: string): Promise<boolean> {
  return invoke<boolean>("directory_is_empty", { path });
}

// --- Library prefs ------------------------------------------------------

/// Read the LibraryPrefs blob (sort / disc-track-experimental / perf tier /
/// view defaults). Generic (D14): each call site reads its own partial view.
export function getLibraryPrefs<T>(): Promise<T> {
  return invoke<T>("get_library_prefs");
}

/// Persist the LibraryPrefs blob. `prefs` is forwarded verbatim (callers build
/// their own next-state object).
export function setLibraryPrefs<P>(prefs: P): Promise<void> {
  return invoke("set_library_prefs", { prefs });
}

/// The unidentified-games rows for a system (Library Manager audit dialog).
/// Generic (D14): the caller owns the `UnidentifiedGameRow` shape.
export function listUnidentifiedGames<T>(systemId: string): Promise<T[]> {
  return invoke<T[]>("list_unidentified_games", { systemId });
}

// --- One-shot localStorage → SQLite migrations --------------------------

/// Migrate legacy localStorage library entries into SQLite; returns count.
export function migrateLibraryFromLocalStorage(entries: RomEntry[]): Promise<number> {
  return invoke<number>("migrate_library_from_local_storage", { entries });
}

/// Migrate legacy localStorage library-folder paths into SQLite; returns count.
export function migrateFoldersFromLocalStorage(paths: string[]): Promise<number> {
  return invoke<number>("migrate_folders_from_local_storage", { paths });
}
