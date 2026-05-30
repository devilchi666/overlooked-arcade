// Frontend bindings for the Game Info Panel v1 Tauri commands.
//
// Rust side: apps/oa-shell/src/game_info.rs (data model + merge layer)
// and apps/oa-shell/src/main.rs (the Tauri command handlers).

import { invoke } from "@tauri-apps/api/core";

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

/// Read the merged Game Info record for one game. Returns `null`
/// when neither the file layer nor the operator's overrides have
/// any content — UI hides the new panel sections entirely.
export async function getGameInfo(args: {
  systemId: string;
  romId: string;
  romHash?: string;
  romTitle?: string;
}): Promise<MergedGameInfo | null> {
  return invoke<MergedGameInfo | null>("get_game_info", args);
}

/// UPSERT the operator's local overrides for one game. Passing
/// `EMPTY_GAME_INFO_OVERRIDE` deletes the row.
export async function setGameInfoOverride(args: {
  systemId: string;
  romId: string;
  overrideRecord: GameInfoOverride;
}): Promise<void> {
  return invoke("set_game_info_override", args);
}

/// Convenience: blank the operator's local overrides. Equivalent
/// to setGameInfoOverride with EMPTY_GAME_INFO_OVERRIDE but reads
/// more explicitly at call sites.
export async function deleteGameInfoOverride(args: {
  systemId: string;
  romId: string;
}): Promise<void> {
  return invoke("delete_game_info_override", args);
}

/// Returns `[system_id, rom_id]` tuples for every game with at least
/// one operator override. Cached once per library refresh + consumed
/// by the tile-badge `✎` indicator.
export async function listGameInfoOverridden(): Promise<Array<[string, string]>> {
  return invoke("list_game_info_overridden");
}
