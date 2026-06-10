// Retroverse-UI Phase C3 Slice 12 — operator-built collections store.
//
// Companion to the existing LibraryStore. Handles the parent
// collection rows + per-collection member id sets. Hydrates on init
// from `list_custom_collections` + a per-collection
// `list_collection_members`. Mutations follow the same optimistic-
// update + revert-on-failure pattern the favorite / completed flags
// use in `LibraryStore` — local state updates synchronously so the
// UI is responsive, the Rust write is awaited in the background, and
// a failed write reverts the optimistic delta.
//
// The membership set is a `Map<collectionId, Set<romId>>` for O(1)
// "is this game in collection X?" lookups (the TileContextMenu's
// "Add to collection ▸" submenu needs that on every render).

import { createStore, produce } from "solid-js/store";
import {
  addToCustomCollection,
  createCustomCollection,
  deleteCustomCollection,
  listCollectionMembers,
  listCustomCollections,
  removeFromCustomCollection,
  renameCustomCollection,
} from "@oa/platform/api/collectionsApi";
import type { RomId } from "./types";

/// Mirrors `library_db::CustomCollectionRow` (camelCase).
export type CustomCollection = {
  id: string;
  name: string;
  sortOrder: number;
  createdAt: number;
  updatedAt: number;
  memberCount: number;
};

type State = {
  /// Collections ordered by sort_order then created_at, exactly as
  /// the Rust query returns them. Render order matches.
  collections: CustomCollection[];
  /// Member rom ids per collection. `undefined` = not yet hydrated
  /// (lazy load on first selection). `Set` rather than `Array` so the
  /// "is X in this collection?" check the TileContextMenu does on
  /// every focused tile stays O(1).
  members: Record<string, Set<RomId>>;
  /// `true` between init() and the first list_custom_collections
  /// response; consumers can show a quick placeholder rather than
  /// the empty-state copy that would otherwise render incorrectly.
  loading: boolean;
};

export type CustomCollectionsStore = ReturnType<typeof createCustomCollectionsStore>;

export function createCustomCollectionsStore() {
  const [state, setState] = createStore<State>({
    collections: [],
    members: {},
    loading: true,
  });

  /// Hydrate collection list. Called once at startup; cheap to call
  /// again to refresh (operator imports a content pack later, etc.).
  async function refresh(): Promise<void> {
    try {
      const rows = await listCustomCollections();
      setState(
        produce((s) => {
          s.collections = rows;
          s.loading = false;
        }),
      );
    } catch (e) {
      console.warn("[oa-collections] list_custom_collections failed:", e);
      setState("loading", false);
    }
  }

  /// Hydrate member ids for a single collection. Idempotent — call
  /// whenever a sidebar entry becomes active. No-op if already loaded
  /// AND not forced; the `force` path is used after a mutation that
  /// might race with another window's writes.
  async function ensureMembers(collectionId: string, force = false): Promise<void> {
    if (!force && state.members[collectionId] !== undefined) return;
    try {
      const ids = await listCollectionMembers(collectionId);
      setState(
        produce((s) => {
          s.members[collectionId] = new Set(ids);
        }),
      );
    } catch (e) {
      console.warn("[oa-collections] list_collection_members failed:", e);
      // On failure, mark as empty so the consumer doesn't loop on
      // ensureMembers retries. The next mutation refresh will recover.
      setState(
        produce((s) => {
          s.members[collectionId] = new Set();
        }),
      );
    }
  }

  async function createCollection(name: string): Promise<string | null> {
    try {
      const id = await createCustomCollection(name);
      // Refresh the list rather than splice locally — sort_order is
      // computed server-side and we want it correct.
      await refresh();
      setState(
        produce((s) => {
          s.members[id] = new Set();
        }),
      );
      return id;
    } catch (e) {
      console.warn("[oa-collections] create_custom_collection failed:", e);
      return null;
    }
  }

  async function renameCollection(id: string, name: string): Promise<void> {
    const prev = state.collections.find((c) => c.id === id)?.name;
    setState(
      "collections",
      (list) => list.map((c) => (c.id === id ? { ...c, name } : c)),
    );
    try {
      await renameCustomCollection(id, name);
    } catch (e) {
      console.warn("[oa-collections] rename_custom_collection failed:", e);
      if (prev !== undefined) {
        setState(
          "collections",
          (list) => list.map((c) => (c.id === id ? { ...c, name: prev } : c)),
        );
      }
    }
  }

  async function deleteCollection(id: string): Promise<void> {
    // Snapshot for revert.
    const snapshot = state.collections;
    const memberSnapshot = state.members[id];
    setState(
      produce((s) => {
        s.collections = s.collections.filter((c) => c.id !== id);
        delete s.members[id];
      }),
    );
    try {
      await deleteCustomCollection(id);
    } catch (e) {
      console.warn("[oa-collections] delete_custom_collection failed:", e);
      setState(
        produce((s) => {
          s.collections = snapshot;
          if (memberSnapshot) s.members[id] = memberSnapshot;
        }),
      );
    }
  }

  async function addToCollection(collectionId: string, romId: RomId): Promise<void> {
    // Optimistic — update the local Set + bump the parent's member_count
    // so the sidebar badge increments without waiting on a refresh.
    const alreadyMember = state.members[collectionId]?.has(romId) === true;
    if (alreadyMember) return;
    setState(
      produce((s) => {
        const existing = s.members[collectionId] ?? new Set<RomId>();
        existing.add(romId);
        s.members[collectionId] = existing;
        s.collections = s.collections.map((c) =>
          c.id === collectionId ? { ...c, memberCount: c.memberCount + 1 } : c,
        );
      }),
    );
    try {
      await addToCustomCollection(collectionId, romId);
    } catch (e) {
      console.warn("[oa-collections] add_to_custom_collection failed:", e);
      setState(
        produce((s) => {
          s.members[collectionId]?.delete(romId);
          s.collections = s.collections.map((c) =>
            c.id === collectionId ? { ...c, memberCount: c.memberCount - 1 } : c,
          );
        }),
      );
    }
  }

  async function removeFromCollection(collectionId: string, romId: RomId): Promise<void> {
    const wasMember = state.members[collectionId]?.has(romId) === true;
    if (!wasMember) return;
    setState(
      produce((s) => {
        s.members[collectionId]?.delete(romId);
        s.collections = s.collections.map((c) =>
          c.id === collectionId ? { ...c, memberCount: Math.max(0, c.memberCount - 1) } : c,
        );
      }),
    );
    try {
      await removeFromCustomCollection(collectionId, romId);
    } catch (e) {
      console.warn("[oa-collections] remove_from_custom_collection failed:", e);
      setState(
        produce((s) => {
          const set = s.members[collectionId] ?? new Set<RomId>();
          set.add(romId);
          s.members[collectionId] = set;
          s.collections = s.collections.map((c) =>
            c.id === collectionId ? { ...c, memberCount: c.memberCount + 1 } : c,
          );
        }),
      );
    }
  }

  /// Is the given rom a member of any collection? Used by the
  /// TileContextMenu to set a chevron / pill on "Add to collection ▸"
  /// without iterating the full member map per render.
  function isInAnyCollection(romId: RomId): boolean {
    for (const set of Object.values(state.members)) {
      if (set?.has(romId)) return true;
    }
    return false;
  }

  // Kick off the initial hydrate. Caller doesn't need to await this —
  // consumers reading `state.collections` will reactively re-render
  // when the list lands.
  void refresh();

  return {
    state,
    refresh,
    ensureMembers,
    createCollection,
    renameCollection,
    deleteCollection,
    addToCollection,
    removeFromCollection,
    isInAnyCollection,
  };
}
