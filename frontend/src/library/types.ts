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
};

export type LibraryState = {
  entries: RomEntry[];
};
