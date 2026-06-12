// Frontend bindings for the game-factual metadata override layer
// (Metadata Curation arc Wave 1 / S1 backend, S3 editor).
//
// Rust side: apps/oa-shell/src/library_db.rs (GameMetadataOverride +
// game_metadata_overrides table, keyed by identity_id) + main.rs (the
// Tauri command handlers). The override is applied at read time over the
// enriched `game_identities` columns in `list_game_groups`; the DB
// columns stay pristine, so a reset restores the synced/baked value.
//
// Same module split as systemInfo: the invoke wrappers live in
// platform/api/gameMetadataApi (the invoke-ban home) and are re-exported
// here; the TYPES live in this domain module and are pulled into the api
// wrapper via `import type` (erased).

export {
  getGameMetadataOverride,
  setGameMetadata,
  deleteGameMetadataOverride,
  resetGameMetadataField,
  getIdentity,
  listGameMetadataOverridden,
} from "@oa/platform/api/gameMetadataApi";

/// Operator's per-field factual-metadata override for one game identity.
/// Every field is optional: `undefined` means "no override, fall through
/// to the enriched identity value"; a value wins at read time. Mirrors
/// the Rust `GameMetadataOverride` (camelCase serde). `genre` is the full
/// multi-value list (the read-path merge flattens it onto the identity's
/// single-TEXT genre column).
export type GameMetadataOverride = {
  title?: string;
  sortTitle?: string;
  year?: number;
  developer?: string;
  publisher?: string;
  genre?: string[];
  players?: number;
  maxPlayers?: number;
  region?: string;
  rating?: number;
  releaseType?: string;
  series?: string;
  description?: string;
};

/// Empty default — matches Rust's `GameMetadataOverride::default()`.
/// Passing this to `setGameMetadata` deletes the row so the table stays
/// sparse.
export const EMPTY_GAME_METADATA_OVERRIDE: GameMetadataOverride = {};

/// One `game_identities` row — the pristine (pre-override) canonical
/// metadata the editor reads as the "Default" provenance baseline.
/// Mirrors the Rust `GameIdentityRow` (camelCase serde).
export type GameIdentityRow = {
  id: string;
  systemId: string;
  canonicalTitle: string;
  normalizedTitle: string;
  year?: number;
  genre?: string;
  developer?: string;
  publisher?: string;
  players?: number;
  rating?: number;
  canonicalCoverPath?: string;
  defaultVariantId?: string;
};
