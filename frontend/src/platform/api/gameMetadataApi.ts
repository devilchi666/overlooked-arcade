// Typed Tauri bridge — game-factual metadata override domain (Metadata
// Curation arc S1 backend, S3 editor).
//
// One typed named export per command, thin pass-through, no error
// handling here, command-name string lives ONLY in this file (the
// platform/api invoke-ban). Types live in platform/library/gameMetadata
// (their domain home); pulled here via `import type` (erased).

import { invoke } from "@tauri-apps/api/core";
import type {
  GameIdentityRow,
  GameMetadataOverride,
} from "@oa/platform/library/gameMetadata";

/// Read the operator's raw factual-metadata override for one identity
/// (default-constructed when no row exists — the editor distinguishes
/// "operator set this" from "fall through to the enriched value").
export function getGameMetadataOverride(args: {
  identityId: string;
}): Promise<GameMetadataOverride> {
  return invoke<GameMetadataOverride>("get_game_metadata_override", args);
}

/// UPSERT (or DELETE if every field is empty) the override for one identity.
export function setGameMetadata(args: {
  identityId: string;
  overrideRecord: GameMetadataOverride;
}): Promise<void> {
  return invoke("set_game_metadata", args);
}

/// Clear ALL of one identity's factual-metadata overrides (drop the row).
export function deleteGameMetadataOverride(args: { identityId: string }): Promise<void> {
  return invoke("delete_game_metadata_override", args);
}

/// Clear ONE field; the row sparse-deletes if it was the last edit.
/// `field` is the camelCase override key (e.g. "year", "sortTitle").
export function resetGameMetadataField(args: {
  identityId: string;
  field: string;
}): Promise<void> {
  return invoke("reset_game_metadata_field", args);
}

/// Read one identity's pristine (pre-override) canonical metadata — the
/// "Default" provenance baseline beneath the override layer. Null when
/// the id is unknown.
export function getIdentity(args: { identityId: string }): Promise<GameIdentityRow | null> {
  return invoke<GameIdentityRow | null>("get_identity", args);
}

/// `identity_id`s with at least one factual-metadata override — one
/// query that drives the game-picker "edited" dot + the "overridden
/// only" filter.
export function listGameMetadataOverridden(): Promise<string[]> {
  return invoke<string[]>("list_game_metadata_overridden");
}
