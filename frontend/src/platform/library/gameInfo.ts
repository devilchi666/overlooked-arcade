// Frontend bindings for the Game Info Panel v1 Tauri commands.
//
// Rust side: apps/oa-shell/src/game_info.rs (data model + merge layer)
// and apps/oa-shell/src/main.rs (the Tauri command handlers).
//
// Theming Phase 4 Slice 3: the typed command wrappers now live in
// platform/api/mediaApi.ts (the game-info cluster of the media domain) and
// are re-exported here so existing consumers keep their import path. This
// module retains the shared types + the empty-override default.

export {
  getGameInfo,
  getGameInfoOverride,
  setGameInfoOverride,
  deleteGameInfoOverride,
  listGameInfoOverridden,
  listGameInfoBadges,
} from "@oa/platform/api/mediaApi";

/// Severity scale matching the Rust BugSeverity enum, serialized as
/// lowercase strings.
export type BugSeverity = "blocker" | "major" | "minor" | "cosmetic";

export type GameBug = {
  description: string;
  severity: BugSeverity;
  workaround?: string;
};

export type BestEmulator = {
  recommended: string;
  reason?: string;
};

/// One labelled tappable region for the TouchHotspotOverlay.
/// Coordinates in NDS bottom-screen native space (0..256 × 0..192).
/// Matches the Rust `TouchHotspot` struct in
/// `apps/oa-shell/src/game_info.rs`.
export type TouchHotspot = {
  label: string;
  x: number;
  y: number;
  w: number;
  h: number;
};

/// Final per-game record after the field-typed precedence merge of
/// the file layer (docs/cores/<id>/games-info.md) + operator local
/// overrides (SQLite game_info_overrides). The frontend doesn't
/// reapply precedence — the Rust merge does it once.
export type MergedGameInfo = {
  systemId: string;
  date?: number;
  publisher?: string;
  region?: string;
  version?: string;
  playerCount?: number;
  genre?: string;
  shortSummary?: string;
  /// True when shortSummary came from the operator's local override.
  /// Drives the "(operator note)" mini-label.
  shortSummaryIsLocal: boolean;
  controlsSupported: string[];
  bestEmulator?: BestEmulator;
  bugs: GameBug[];
  /// Game-specific tappable regions (NDS stylus titles, etc.).
  /// Empty array when the game record has none.
  touchHotspots: TouchHotspot[];
  /// True when the operator has at least one local override on this
  /// game. Drives the ✎ tile-badge indicator.
  hasLocalEdits: boolean;
  /// Provenance: "Apply best emulator" panel button was used.
  appliedBestEmulator: boolean;
  /// Provenance: "Apply controls" panel button was used.
  appliedControls: boolean;
};

/// Operator's local overrides — Layer 3 of the data model. All fields
/// optional; arrays use `undefined` (no override) vs `[]` (operator
/// cleared the list).
export type GameInfoOverride = {
  shortSummary?: string;
  controlsSupported?: string[];
  bestEmulator?: string;
  bestEmulatorReason?: string;
  bugs?: GameBug[];
  appliedBestEmulator: boolean;
  appliedControls: boolean;
};

/// Empty default — matches Rust's `GameInfoOverride::default()`.
export const EMPTY_GAME_INFO_OVERRIDE: GameInfoOverride = {
  appliedBestEmulator: false,
  appliedControls: false,
};

/// Reduced shape the tile-badge UI consumes — bug count + max severity
/// for the `⚠ N` overlay, plus the local-edits flag for the `✎` mark.
export type GameInfoBadge = {
  systemId: string;
  romId: string;
  bugCount: number;
  maxSeverity?: BugSeverity;
  hasLocalEdits: boolean;
};

/// Minimal library-entry view the bulk-badge command needs. Same
/// shape as a slice of `RomEntry`; the caller transforms.
export type LibraryEntryForBadges = {
  id: string;
  systemId: string;
  title: string;
  sha1?: string;
};
