import type { SystemId } from "../themes/registry";

export type RomId = string;

export type RomEntry = {
  id: RomId;
  title: string;
  systemId: SystemId;
  /// On-disk file the user sees. For raw ROMs, this is the ROM itself. For
  /// games living inside archives, this is the .zip/.7z path PLUS a `#inner`
  /// suffix encoding the unique inner ROM (so the UNIQUE constraint on
  /// games.file_path lets multiple entries share one archive on disk).
  filePath: string;
  addedAt: number;
  coverPath?: string;
  seed?: boolean;
  /// Per-game core override (filename of a .dll/.so/.dylib in <exe_dir>/cores/).
  /// Set via the tile context menu (right-click → "Run with core…"). Empty /
  /// undefined falls back to the per-system pref → hardcoded default.
  coreOverride?: string;
  /// Posix-style inner path inside the archive at `filePath.split("#")[0]`.
  /// When set, the launch path routes through archive::extract_for_launch
  /// instead of `std::fs::read`. Cart-format inners run from extracted bytes;
  /// CD-set inners (cue/m3u/toc) extract to appData/temp/<rom_id>/.
  archiveInnerPath?: string;
  /// SHA-1 of the ROM bytes, stamped by the Identify ROMs flow. Sent
  /// to sync_media_for_system so the server can resolve the canonical
  /// libretro-database name and do exact-filename matching against
  /// libretro-thumbnails (much fewer false positives than the old fuzzy
  /// filename comparison).
  sha1?: string;
  /// Region / catalog serial pulled from libretro-database on a hash
  /// match. Diagnostic for now.
  serial?: string;
  /// Retroverse-UI Phase C3 — operator-tagged favorite. Drives the
  /// Favorites smart-list in COLLECTIONS + the heart overlay on
  /// LibraryTile. Optional in the type because pre-Phase-C3 lookups
  /// (find_game_by_id / find_game_by_sha1 / search) don't SELECT
  /// the column; the canonical source is `list_games` via the
  /// LibraryStore hydration.
  favorite?: boolean;
  /// Retroverse-UI Phase C3 — operator-marked completed. Drives the
  /// Completed smart-list in COLLECTIONS.
  completed?: boolean;
  /// Unix-seconds timestamp of the last session. Written by the Phase A
  /// Slice 2 close_active_session helper.
  lastPlayedAt?: number;
  /// Total seconds played across all sessions. Phase A Slice 2.
  playTimeSecs?: number;
  /// Maximum simultaneous players supported. Populated by metadata
  /// enrichment; drives the COLLECTIONS Multi-Player smart-list.
  players?: number;
  /// Editorial rating 0.0–5.0 from metadata enrichment. Will drive
  /// Hidden Gems smart-list in a follow-up once populated.
  rating?: number;
  /// Phase A1 Sub-phase 4 — FK into the disc_sets table for one disc
  /// of a multi-disc game. Undefined for single-disc and cart games.
  /// Drives library tile grouping (one tile per set rather than one
  /// per disc); see DiscPickerOverlay for the disc selection UX.
  discSetId?: number;
  /// 1-based disc index within the parent set ("Disc 1" / "Disc 2"
  /// / …). Undefined for standalone games. Orders the disc-picker.
  discNumber?: number;
};

export type LibraryState = {
  entries: RomEntry[];
};

/// Variant metadata returned by Rust's `list_game_groups`. One per file —
/// the right-click "Run version" submenu enumerates this list.
export type VariantInfo = {
  id: RomId;
  title: string;
  filePath: string;
  archiveInnerPath?: string;
  coverPath?: string;
  region?: string;
  regions: string[];
  revision: number;
  flags: string[];
  isPrerelease: boolean;
  /// True for the group's chosen default variant (frontend renders ✓
  /// next to this entry in the context-menu submenus).
  isDefault: boolean;
};

/// A multi-variant group (e.g. all dumps of "Castlevania" on TG-16).
/// `displayBaseTitle` is what the tile shows; `variants` carries every
/// dump in priority order.
export type GameGroupInfo = {
  systemId: SystemId;
  baseTitleKey: string;
  displayBaseTitle: string;
  defaultVariantId: RomId;
  variants: VariantInfo[];
};
