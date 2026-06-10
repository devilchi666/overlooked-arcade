// Typed Tauri bridge — operator-built custom collections.
//
// Theming Phase 4 Slice 2. Backs `customCollections.ts` (the store) — the
// parent collection rows + per-collection member id sets. Same convention as
// settingsApi: one typed named export per command, thin pass-through, command
// string lives only here.

import { invoke } from "@tauri-apps/api/core";
import type { RomId } from "@oa/platform/library/types";
import type { CustomCollection } from "@oa/platform/library/customCollections";

/// List all custom collections (ordered by sort_order then created_at).
export function listCustomCollections(): Promise<CustomCollection[]> {
  return invoke<CustomCollection[]>("list_custom_collections");
}

/// List the member rom ids of a single collection.
export function listCollectionMembers(collectionId: string): Promise<RomId[]> {
  return invoke<RomId[]>("list_collection_members", { collectionId });
}

/// Create a new (empty) collection; returns the new collection id.
export function createCustomCollection(name: string): Promise<string> {
  return invoke<string>("create_custom_collection", { name });
}

/// Rename a collection.
export function renameCustomCollection(id: string, name: string): Promise<void> {
  return invoke("rename_custom_collection", { id, name });
}

/// Delete a collection (and its membership rows).
export function deleteCustomCollection(id: string): Promise<void> {
  return invoke("delete_custom_collection", { id });
}

/// Add a game to a collection.
export function addToCustomCollection(collectionId: string, romId: RomId): Promise<void> {
  return invoke("add_to_custom_collection", { collectionId, romId });
}

/// Remove a game from a collection.
export function removeFromCustomCollection(collectionId: string, romId: RomId): Promise<void> {
  return invoke("remove_from_custom_collection", { collectionId, romId });
}
