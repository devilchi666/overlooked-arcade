// Typed Tauri bridge — media domain (art / metadata / game-info / mame / hashes).
//
// Theming Phase 4 Slice 3. The art + metadata cluster: the per-ROM media
// catalog (covers / screenshots / kinds-to-fetch / disk stats), the
// per-system platform-media slots, libretro-thumbnails + metadata sync, the
// ROM-hash identify pass, art-pack import, the Game Info v1 override layer,
// and the MAME title/metadata catalog. Same convention as settingsApi /
// libraryApi (see docs/PLANS/theming-platform-api-bridge.md): one typed named
// export per command, thin pass-through, no error handling here (call sites
// keep their own try/catch), and the command-name string lives ONLY in this
// file (grep-verifiable).
//
// Shape-divergent getters follow D14 — generic with a canonical default
// (`getPlatformMediaIndex<T = PlatformMediaIndex>`) where multiple call sites
// hold their own partial view. Single-shape getters return their concrete
// contract type (defined here, the proper home for backend-contract shapes).

import { invoke } from "@tauri-apps/api/core";
import type { MediaIndex } from "@oa/platform/library/media";
import type {
  MergedGameInfo,
  GameInfoOverride,
  GameInfoBadge,
  LibraryEntryForBadges,
} from "@oa/platform/library/gameInfo";
import type { MameRefreshReport } from "@oa/platform/library/systemInfo";

// --- Backend-contract types this domain owns ----------------------------

/// One library entry as the media/metadata sync commands consume it. The
/// `sha1` field is carried by some call sites (Library Manager) and omitted
/// by others (drag-drop ingest / wizard) — the backend treats it as optional.
export type MediaSyncEntry = {
  id: string;
  title: string;
  filePath: string;
  systemId: string;
  sha1?: string;
};

/// On-disk media footprint (`media_storage_stats`).
export type MediaStorageStats = {
  coversBytes: number;
  thumbsBytes: number;
  cacheBytes: number;
  totalBytes: number;
};

/// One image variant inside a platform-media slot. Mirrors Rust's
/// `MediaVariant`. The canonical default for `getPlatformMediaIndex`.
export type PlatformMediaVariant = {
  source: { kind: string };
  region?: string;
  path: string;
  thumbPath?: string;
  width?: number;
  height?: number;
  sha1?: string;
  bytes?: number;
};

/// Per-system hardware-art bundle (9 Option-shaped slots). Mirrors Rust's
/// `PlatformMedia`.
export type PlatformMedia = {
  banner?: PlatformMediaVariant;
  clearLogo?: PlatformMediaVariant;
  console?: PlatformMediaVariant;
  controller?: PlatformMediaVariant;
  fanart?: PlatformMediaVariant;
  marquee?: PlatformMediaVariant;
  photo?: PlatformMediaVariant;
  wheel?: PlatformMediaVariant;
  background?: PlatformMediaVariant;
};

/// `get_platform_media_index` return: BTreeMap<systemId, PlatformMedia>.
export type PlatformMediaIndex = Record<string, PlatformMedia>;

/// Per-kind import tally inside one platform's art-pack report.
export type ArtPackKindReport = {
  kind: string;
  sourceFiles: number;
  imported: number;
  skippedNoMatch: number;
};

/// One platform's slice of an art-pack import (`import_art_pack`).
export type ArtPackPlatformReport = {
  platformDir: string;
  systemId: string | null;
  launchboxName: string | null;
  libraryEntries: number;
  byKind: Record<string, ArtPackKindReport>;
  totalImported: number;
  totalSkippedNoMatch: number;
  error: string | null;
};

/// Result of an art-pack import (dry-run or live).
export type ArtPackImportReport = {
  layout: "single-platform" | "multi-platform" | "unknown";
  platforms: ArtPackPlatformReport[];
  totalImported: number;
  totalSkippedNoMatch: number;
};

/// v2 listxml-backed MAME lookup row (`lookup_mame_game`). Merged L1+L3
/// record; the frontend never sees raw L3 overrides on this path.
export type MergedMameGame = {
  name: string;
  description: string;
  year?: string | null;
  manufacturer?: string | null;
  cloneof?: string | null;
  hasLocalEdits: boolean;
};

/// Legacy (v11) MAME.dat-backed lookup row (`lookup_mame_title`). 2nd-tier
/// fallback when `lookup_mame_game` returns null.
export type LegacyMameTitleLookup = {
  romSet: string;
  title: string;
  year?: string | null;
  developer?: string | null;
};

// --- Media catalog (covers / screenshots / kinds / stats) ---------------

/// Hydrate the full per-ROM media catalog (the MediaProvider's source).
export function getMediaIndex(): Promise<MediaIndex> {
  return invoke<MediaIndex>("get_media_index");
}

/// Set a manual cover (or other slot) from a source image on disk.
/// `kind` defaults to "box-front" on the Rust side when omitted.
export function setManualCover(
  romId: string,
  systemId: string,
  sourcePath: string,
  kind?: string,
): Promise<void> {
  return invoke("set_manual_cover", { romId, systemId, sourcePath, kind });
}

/// Clear all media for one ROM.
export function clearMedia(romId: string): Promise<void> {
  return invoke("clear_media", { romId });
}

/// Read which media kinds the libretro-thumbnails sync fetches per ROM.
export function getMediaKindsToFetch(): Promise<string[]> {
  return invoke<string[]>("get_media_kinds_to_fetch");
}

/// Persist which media kinds the sync fetches per ROM.
export function setMediaKindsToFetch(kinds: string[]): Promise<void> {
  return invoke("set_media_kinds_to_fetch", { kinds });
}

/// On-disk media footprint (covers / thumbs / cache / total bytes).
export function mediaStorageStats(): Promise<MediaStorageStats> {
  return invoke<MediaStorageStats>("media_storage_stats");
}

/// Reveal the media-storage root in the OS file manager.
export function openMediaFolder(): Promise<void> {
  return invoke("open_media_folder");
}

/// Result of clearing per-system metadata (`clear_metadata_for_system`).
export type MetadataClearResult = {
  systemId: string;
  scanned: number;
  cleared: number;
};

/// Wipe enriched metadata for every game in a system (Library Manager).
export function clearMetadataForSystem(systemId: string): Promise<MetadataClearResult> {
  return invoke<MetadataClearResult>("clear_metadata_for_system", { systemId });
}

/// Resolve a per-system background asset of `kind` ("default" | "animated")
/// to an absolute path, or `null` when no file exists. `themeId` is the
/// active theme's id (or `null`) — S5.1's theme tier checks
/// `assets/themes/<themeId>/system-ui/…` before the platform per-system +
/// `_baseline` assets. Callers resolve the active id ambiently (see
/// `SystemBackground`), so consumers don't thread it.
export function resolveBackgroundAsset(
  themeId: string | null,
  systemId: string,
  kind: string,
): Promise<string | null> {
  return invoke<string | null>("resolve_background_asset", { themeId, systemId, kind });
}

// --- libretro-thumbnails + metadata sync --------------------------------

/// Sync cover/screenshot art for one system's entries from libretro-thumbnails.
export function syncMediaForSystem(systemId: string, entries: MediaSyncEntry[]): Promise<void> {
  return invoke("sync_media_for_system", { systemId, entries });
}

/// Sync textual metadata (year / genre / publisher / …) for one system's entries.
export function syncMetadataForSystem(systemId: string, entries: MediaSyncEntry[]): Promise<void> {
  return invoke("sync_metadata_for_system", { systemId, entries });
}

/// Read the "only sync identified ROMs" preference (hash-matched entries only).
export function getOnlySyncIdentified(): Promise<boolean> {
  return invoke<boolean>("get_only_sync_identified");
}

/// Persist the "only sync identified ROMs" preference.
export function setOnlySyncIdentified(enabled: boolean): Promise<void> {
  return invoke("set_only_sync_identified", { enabled });
}

// --- ROM-hash identify --------------------------------------------------

/// Resolve canonical titles + sha1s for one system's untracked ROMs against
/// the bundled `rom_hashes` table (the "Identify ROMs" pass).
export function resolveRomHashesForSystem(systemId: string): Promise<void> {
  return invoke("resolve_rom_hashes_for_system", { systemId });
}

/// Re-sync the `rom_hashes` table for one system from the bundled datfiles.
export function syncRomHashesForSystem(systemId: string): Promise<void> {
  return invoke("sync_rom_hashes_for_system", { systemId });
}

// --- Platform media (per-system hardware art) ---------------------------

/// Hydrate the per-system platform-media catalog. Call sites pass their own
/// `PlatformMedia` view via `T` (each component declares its own local slot
/// shape); the canonical default mirrors the Rust struct.
export function getPlatformMediaIndex<T = PlatformMediaIndex>(): Promise<T> {
  return invoke<T>("get_platform_media_index");
}

/// Set one platform-media slot for `systemId` from a source image on disk.
export function setPlatformMedia(
  systemId: string,
  slot: string,
  sourcePath: string,
): Promise<void> {
  return invoke("set_platform_media", { systemId, slot, sourcePath });
}

/// Clear one platform-media slot for `systemId`.
export function clearPlatformMedia(systemId: string, slot: string): Promise<void> {
  return invoke("clear_platform_media", { systemId, slot });
}

// --- Art-pack import ----------------------------------------------------

/// Import (or dry-run analyze) a LaunchBox / EmuMovies art-pack folder.
/// `systemIdOverride` forces a single-platform classification; `dryRun`
/// runs the analysis without writing.
export function importArtPack(args: {
  sourceDir: string;
  systemIdOverride: string | null;
  dryRun: boolean;
}): Promise<ArtPackImportReport> {
  return invoke<ArtPackImportReport>("import_art_pack", args);
}

// --- Game Info v1 (operator override layer) -----------------------------

/// Read the merged Game Info record for one game (file layer + operator
/// overrides). Returns `null` when neither layer has any content.
export function getGameInfo(args: {
  systemId: string;
  romId: string;
  romHash?: string;
  romTitle?: string;
}): Promise<MergedGameInfo | null> {
  return invoke<MergedGameInfo | null>("get_game_info", args);
}

/// Read just the operator's local override for one game (not the merged
/// record). Returns the empty default when no override row exists.
export function getGameInfoOverride(args: {
  systemId: string;
  romId: string;
}): Promise<GameInfoOverride> {
  return invoke<GameInfoOverride>("get_game_info_override", args);
}

/// UPSERT the operator's local overrides for one game. Passing the empty
/// default deletes the row.
export function setGameInfoOverride(args: {
  systemId: string;
  romId: string;
  overrideRecord: GameInfoOverride;
}): Promise<void> {
  return invoke("set_game_info_override", args);
}

/// Blank the operator's local overrides for one game.
export function deleteGameInfoOverride(args: {
  systemId: string;
  romId: string;
}): Promise<void> {
  return invoke("delete_game_info_override", args);
}

/// `[system_id, rom_id]` tuples for every game with at least one operator
/// override — drives the tile-badge `✎` indicator.
export function listGameInfoOverridden(): Promise<Array<[string, string]>> {
  return invoke("list_game_info_overridden");
}

/// Bulk-compute tile-badge data (bug count + max severity + local-edits flag)
/// for `entries`. Only entries with a bug OR an override appear in the result.
export function listGameInfoBadges(entries: LibraryEntryForBadges[]): Promise<GameInfoBadge[]> {
  return invoke("list_game_info_badges", { entries });
}

// --- MAME catalog (title / metadata / L1 refresh) -----------------------

/// Tier-1 listxml-backed MAME lookup by ROM-set name (`null` on miss).
export function lookupMameGame(romSet: string): Promise<MergedMameGame | null> {
  return invoke<MergedMameGame | null>("lookup_mame_game", { romSet });
}

/// Tier-2 legacy MAME.dat-backed lookup by ROM-set name (`null` on miss).
export function lookupMameTitle(romSet: string): Promise<LegacyMameTitleLookup | null> {
  return invoke<LegacyMameTitleLookup | null>("lookup_mame_title", { romSet });
}

/// Write MAME-derived metadata (year / publisher) into a game's MediaDb
/// GameMetadata. Part of the MAME resolve-and-store ingest flow.
export function setGameMameMetadata(
  romId: string,
  year: number | null,
  publisher: string | null,
): Promise<void> {
  return invoke("set_game_mame_metadata", { romId, year, publisher });
}

/// Re-import the L1 (MAME baseline) system-info layer from the operator's
/// local MAME install. L2 (curated) + L3 (overrides) are never touched.
/// `mamePath` overrides auto-detection.
export function refreshMameSystemInfo(args: { mamePath?: string }): Promise<MameRefreshReport> {
  return invoke<MameRefreshReport>("refresh_mame_system_info", args);
}
