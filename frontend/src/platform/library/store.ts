// Frontend library store.
//
// Source of truth lives in Rust at appDataDir/library/games.sqlite. This
// store is the in-memory mirror — hydrates once at mount via `list_games`,
// then keeps itself in sync with mutation commands. The Solid store proxy
// is what the WebView reactive system reads from; Rust never pushes via
// events for library mutations (that's MediaContext's pattern).
//
// One-shot migration: on first launch after the SQLite upgrade, the
// previous localStorage[oa.library.v1] entries are sent to Rust via
// `migrate_library_from_local_storage`, then the localStorage key is
// cleared. Idempotent on Rust side (INSERT OR IGNORE) so a partial
// migration that didn't clear localStorage is harmless.

import { createSignal } from "solid-js";
import { createStore } from "solid-js/store";
import {
  addGames,
  clearGameGroupDefault,
  deleteAllGames,
  deleteGame,
  deleteGamesForSystem,
  dropSeedGames,
  listGameGroups,
  listGames,
  migrateLibraryFromLocalStorage,
  setGameGroupDefault,
  updateGameCompleted,
  updateGameCoreOverride,
  updateGameFavorite,
} from "@oa/platform/api/libraryApi";
import type { GameGroupInfo, LibraryState, RomEntry, RomId } from "./types";
import type { SystemId } from "@oa/platform/themes/registry";

const LEGACY_STORAGE_KEY = "oa.library.v1";

// Same six placeholders as before — inserted on truly-fresh installs so the
// empty state has visual interest. Stored Rust-side now (seed=true), cleared
// on first real ingest by add_games dropping seeds.
const SEED_ENTRIES: RomEntry[] = [
  { id: "tg16-bonks-adventure",  title: "Bonk's Adventure",   systemId: "tg16", filePath: "<seed:bonks-adventure>",  addedAt: 0, seed: true },
  { id: "tg16-blazing-lazers",   title: "Blazing Lazers",     systemId: "tg16", filePath: "<seed:blazing-lazers>",   addedAt: 0, seed: true },
  { id: "tg16-r-type",           title: "R-Type",             systemId: "tg16", filePath: "<seed:r-type>",           addedAt: 0, seed: true },
  { id: "tg16-ys-book-i-ii",     title: "Ys Book I & II",     systemId: "tg16", filePath: "<seed:ys-book-i-ii>",     addedAt: 0, seed: true },
  { id: "tg16-splatterhouse",    title: "Splatterhouse",      systemId: "tg16", filePath: "<seed:splatterhouse>",    addedAt: 0, seed: true },
  { id: "tg16-military-madness", title: "Military Madness",   systemId: "tg16", filePath: "<seed:military-madness>", addedAt: 0, seed: true },
];

function readLegacyLocalStorage(): RomEntry[] | null {
  try {
    const raw = localStorage.getItem(LEGACY_STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as unknown;
    if (
      parsed &&
      typeof parsed === "object" &&
      "entries" in parsed &&
      Array.isArray((parsed as LibraryState).entries)
    ) {
      const entries = (parsed as LibraryState).entries.filter(
        (e): e is RomEntry =>
          typeof e === "object" &&
          e !== null &&
          typeof (e as RomEntry).id === "string" &&
          typeof (e as RomEntry).title === "string" &&
          typeof (e as RomEntry).systemId === "string" &&
          typeof (e as RomEntry).filePath === "string",
      );
      return entries;
    }
  } catch {
    // localStorage disabled or malformed JSON — fall through.
  }
  return null;
}

function clearLegacyLocalStorage(): void {
  try {
    localStorage.removeItem(LEGACY_STORAGE_KEY);
  } catch {
    // ignore
  }
}

export type CreateLibraryStoreOptions = {
  /** Resolves to `true` to run the normal bootstrap (default), or `false`
   *  to skip it entirely. Used by direct-launch mode: the frontend boots
   *  straight into a game and never renders the library grid, so the
   *  `list_games` / `list_game_groups` / seed-insertion / migration work
   *  is pure waste. If omitted, bootstrap runs immediately. */
  shouldBootstrap?: Promise<boolean>;
};

export function createLibraryStore(options: CreateLibraryStoreOptions = {}) {
  // Start empty; hydrate inside the async bootstrap below. The store proxy
  // is alive immediately so consumers can subscribe before hydration lands.
  const [state, setState] = createStore<LibraryState>({ entries: [] });
  const [hydrated, setHydrated] = createSignal(false);
  // Variant grouping — keyed by EVERY variant's id (not just the
  // default) so any tile / context-menu callsite can look up "which
  // group am I in?" with one Map.get.
  //
  // VL Phase E Sub-phase 3 — singleton groups are included too: every
  // game belongs to an identity now, and tiles read the identity's
  // canonical title / metadata / artwork through this map. Variant-
  // specific affordances (the ▼ badge, the "Run version ▸" submenu)
  // still gate on `variants.length > 1` at their call sites.
  const [groupsByVariantId, setGroupsByVariantId] =
    createSignal<Map<RomId, GameGroupInfo>>(new Map());

  async function refreshGroups(): Promise<void> {
    try {
      const groups = await listGameGroups();
      const map = new Map<RomId, GameGroupInfo>();
      for (const g of groups) {
        for (const v of g.variants) {
          map.set(v.id, g);
        }
      }
      setGroupsByVariantId(map);
    } catch (e) {
      console.warn("[oa-library] list_game_groups failed:", e);
    }
  }
  // Suppress consumer access to the store until hydrate finishes. Components
  // can check `hydrated()` if they want to defer first render; LibraryView's
  // empty-state fallback handles the brief in-between state.

  void bootstrap();

  async function bootstrap() {
    // Direct-launch short-circuit: caller awaits a Tauri command to decide
    // whether to bootstrap, and resolves the promise with false to skip.
    // Mark hydrated so any consumers gating on `hydrated()` don't wait
    // forever for a bootstrap that isn't coming.
    if (options.shouldBootstrap) {
      const proceed = await options.shouldBootstrap;
      if (!proceed) {
        console.log("[oa-library] bootstrap skipped (direct-launch mode)");
        setHydrated(true);
        return;
      }
    }

    // Step 1: try the one-shot migration if legacy localStorage has data.
    const legacy = readLegacyLocalStorage();
    if (legacy && legacy.length > 0) {
      const realOnly = legacy.filter((e) => !e.seed && !e.filePath.startsWith("<seed"));
      if (realOnly.length > 0) {
        try {
          const added = await migrateLibraryFromLocalStorage(realOnly);
          console.log(`[oa-library] migrated ${added} entries from localStorage`);
        } catch (e) {
          console.warn("[oa-library] migration failed:", e);
        }
      }
      // Clear regardless — re-running migration is safe (idempotent on Rust
      // side) but consuming the localStorage key signals "we've moved past v1."
      clearLegacyLocalStorage();
    }

    // Step 2: hydrate from Rust.
    let games: RomEntry[] = [];
    try {
      games = await listGames();
    } catch (e) {
      console.warn("[oa-library] list_games failed:", e);
    }

    // Step 3: if the DB is truly empty (fresh install, no legacy), seed it
    // so the empty state has visual interest. Seeds get dropped on first
    // real add via add_games → drop_seed_games.
    if (games.length === 0) {
      try {
        await addGames(SEED_ENTRIES);
        games = await listGames();
      } catch (e) {
        console.warn("[oa-library] seed insert failed:", e);
      }
    }

    setState("entries", games);
    await refreshGroups();
    setHydrated(true);
  }

  return {
    state,
    hydrated,
    groupsByVariantId,
    refreshGroups,
    /// Pull the authoritative game list from Rust and replace the local
    /// mirror. Used after server-side mutations the store didn't initiate
    /// (e.g. the rom_hashes resolve flow rewriting canonical titles).
    async refresh(): Promise<void> {
      try {
        const fresh = await listGames();
        setState("entries", fresh);
        await refreshGroups();
      } catch (e) {
        console.warn("[oa-library] refresh failed:", e);
      }
    },
    /// Pin a (system_id, base_title) group to a specific variant.
    /// Re-fetches the groups so the UI updates immediately.
    async setGroupDefault(
      systemId: SystemId,
      baseTitle: string,
      preferredGameId: RomId,
    ): Promise<void> {
      try {
        await setGameGroupDefault(systemId, baseTitle, preferredGameId);
        await refreshGroups();
      } catch (e) {
        console.warn("[oa-library] set_game_group_default failed:", e);
      }
    },
    async clearGroupDefault(systemId: SystemId, baseTitle: string): Promise<void> {
      try {
        await clearGameGroupDefault(systemId, baseTitle);
        await refreshGroups();
      } catch (e) {
        console.warn("[oa-library] clear_game_group_default failed:", e);
      }
    },
    /// Add scanned ROMs. Writes through to Rust, then refreshes local cache
    /// from the source of truth. Returns the number of newly-added entries
    /// (Rust dedups by file_path).
    async addScannedRoms(entries: RomEntry[]): Promise<number> {
      let added = 0;
      try {
        added = await addGames(entries);
      } catch (e) {
        console.warn("[oa-library] add_games failed:", e);
        return 0;
      }
      // If this is the first real ingest, kick out seed rows.
      if (added > 0) {
        const hadSeed = state.entries.some((e) => e.seed);
        if (hadSeed) {
          try {
            await dropSeedGames();
          } catch (e) {
            console.warn("[oa-library] drop_seed_games failed:", e);
          }
        }
      }
      // Refresh from authoritative source.
      try {
        const fresh = await listGames();
        setState("entries", fresh);
        await refreshGroups();
      } catch (e) {
        console.warn("[oa-library] refresh after add failed:", e);
      }
      return added;
    },
    async remove(id: RomId): Promise<void> {
      try {
        await deleteGame(id);
        setState("entries", (prev) => prev.filter((e) => e.id !== id));
      } catch (e) {
        console.warn("[oa-library] delete_game failed:", e);
      }
    },
    /// Set or clear the per-game core override. `value` is the .dll filename
    /// or null/empty to revert.
    async setCoreOverride(id: RomId, value: string | null): Promise<void> {
      const normalized = value && value.length > 0 ? value : null;
      try {
        await updateGameCoreOverride(id, normalized);
        setState("entries", (prev) =>
          prev.map((e) => (e.id === id ? { ...e, coreOverride: normalized ?? undefined } : e)),
        );
      } catch (e) {
        console.warn("[oa-library] update_game_core_override failed:", e);
      }
    },
    /// Retroverse-UI Phase C3 — flip the favorite flag for a game.
    /// Optimistic local update + persistence write; the COLLECTIONS
    /// Favorites smart-list re-derives from `state.entries` so the
    /// tile heart + the smart-list count update together. Idempotent
    /// at the SQL level (writing the same value twice is a no-op).
    async setFavorite(id: RomId, value: boolean): Promise<void> {
      setState("entries", (prev) =>
        prev.map((e) => (e.id === id ? { ...e, favorite: value } : e)),
      );
      try {
        await updateGameFavorite(id, value);
      } catch (e) {
        console.warn("[oa-library] update_game_favorite failed:", e);
        // Revert local state on Rust-side failure so UI matches DB.
        setState("entries", (prev) =>
          prev.map((e) => (e.id === id ? { ...e, favorite: !value } : e)),
        );
      }
    },
    /// Retroverse-UI Phase C3 — flip the completed flag. Same shape as
    /// setFavorite; drives the COLLECTIONS Completed smart-list.
    async setCompleted(id: RomId, value: boolean): Promise<void> {
      setState("entries", (prev) =>
        prev.map((e) => (e.id === id ? { ...e, completed: value } : e)),
      );
      try {
        await updateGameCompleted(id, value);
      } catch (e) {
        console.warn("[oa-library] update_game_completed failed:", e);
        setState("entries", (prev) =>
          prev.map((e) => (e.id === id ? { ...e, completed: !value } : e)),
        );
      }
    },
    async clear(): Promise<void> {
      // Single DELETE statement — much faster than the per-row loop this
      // method previously did, and atomic. The setting page's "Reset
      // entire library" action calls this AFTER its own confirm() dialog.
      try {
        await deleteAllGames();
      } catch (e) {
        console.warn("[oa-library] delete_all_games failed:", e);
      }
      setState("entries", []);
    },
    /// Delete every game tagged with the given system id from the DB
    /// + the local mirror. Returns the number removed. Used by Settings
    /// → Library → "Clear games for this system".
    async clearForSystem(systemId: string): Promise<number> {
      let n = 0;
      try {
        n = await deleteGamesForSystem(systemId);
      } catch (e) {
        console.warn(`[oa-library] delete_games_for_system(${systemId}) failed:`, e);
        return 0;
      }
      setState("entries", (prev) => prev.filter((e) => e.systemId !== systemId));
      return n;
    },
    /// Re-hydrate the local mirror from Rust. Used after operations that
    /// modify the DB outside the store's usual write-through path (e.g.
    /// the auto-remove watcher event).
    async reload(): Promise<void> {
      try {
        const fresh = await listGames();
        setState("entries", fresh);
        await refreshGroups();
      } catch (e) {
        console.warn("[oa-library] reload failed:", e);
      }
    },
  };
}

export type LibraryStore = ReturnType<typeof createLibraryStore>;

// Compatibility shim — some callers (older ingest.ts paths) used the
// synchronous signature. The store still exposes `state` synchronously; any
// mutation path needs to await the returned promise.
export type { SystemId };
