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
};

export type LibraryState = {
  entries: RomEntry[];
};
