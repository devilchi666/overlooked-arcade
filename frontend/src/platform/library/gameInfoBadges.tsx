// Game Info Panel v1 — tile-badge store.
//
// The LibraryTile component renders a `⚠ N` overlay for games with
// known issues and a `✎` mark for games the operator has locally
// edited (Phase 6 of docs/PLANS/game-info-panel.md). Per-tile
// invocations of get_game_info would be N round-trips, so we bulk-
// compute once per library refresh via the `list_game_info_badges`
// command and cache the result here.
//
// The Provider takes the library's `entries` accessor + watches it via
// createEffect; whenever the entry list changes, we refresh. The
// `badgeFor` accessor is what the tile component calls on every render
// — pure HashMap lookup, no Solid reactivity churn.

import {
  createContext,
  createEffect,
  createSignal,
  type ParentComponent,
  useContext,
} from "solid-js";
import {
  listGameInfoBadges,
  type GameInfoBadge,
  type LibraryEntryForBadges,
} from "./gameInfo";
import type { RomEntry } from "./types";

type BadgesMap = Map<string, GameInfoBadge>;

function badgeKey(systemId: string, romId: string): string {
  return `${systemId}:${romId}`;
}

type BadgesContextValue = {
  /// Synchronous lookup. Returns undefined when the game has no bug
  /// list and no local edits — tile renders no overlay in that case.
  badgeFor: (systemId: string, romId: string) => GameInfoBadge | undefined;
  /// Manual refresh hook for callers that mutate game info overrides
  /// outside the library-entry life cycle (e.g. the Game Info Panel
  /// editor saves, the Apply buttons). Cheap — full library badge
  /// computation is single-digit ms on the Rust side.
  refresh: () => Promise<void>;
};

/// Fallback used by consumers that render outside the provider (tests,
/// direct-launch path). Returns "no badges anywhere"; the tile
/// renders the same as it did pre-Phase-6.
const EMPTY_BADGES_CONTEXT: BadgesContextValue = {
  badgeFor: () => undefined,
  refresh: async () => {},
};

const GameInfoBadgesContext = createContext<BadgesContextValue>(EMPTY_BADGES_CONTEXT);

type ProviderProps = {
  /// Accessor for the library's RomEntry list — typically
  /// `library.entries`. Provider re-fetches badges whenever the
  /// accessor changes its value.
  entries: () => RomEntry[];
};

export const GameInfoBadgesProvider: ParentComponent<ProviderProps> = (props) => {
  const [map, setMap] = createSignal<BadgesMap>(new Map());

  async function refresh(): Promise<void> {
    const entries = props.entries();
    if (entries.length === 0) {
      setMap(new Map());
      return;
    }
    const slim: LibraryEntryForBadges[] = entries.map((e) => ({
      id: e.id,
      systemId: e.systemId,
      title: e.title,
      sha1: e.sha1,
    }));
    try {
      const badges = await listGameInfoBadges(slim);
      const next: BadgesMap = new Map();
      for (const b of badges) {
        next.set(badgeKey(b.systemId, b.romId), b);
      }
      setMap(next);
    } catch (e) {
      console.warn("[GameInfoBadges] list_game_info_badges failed:", e);
    }
  }

  // Re-fetch whenever the entries list changes (new scan, removed
  // game, etc.). createEffect re-runs every time the accessor's
  // tracked value changes; library mutations naturally trigger this
  // via the existing LibraryStore signals.
  createEffect(() => {
    // Track the entries reference — re-fetch when it swaps.
    props.entries();
    void refresh();
  });

  const value: BadgesContextValue = {
    badgeFor: (systemId, romId) => map().get(badgeKey(systemId, romId)),
    refresh,
  };

  return (
    <GameInfoBadgesContext.Provider value={value}>
      {props.children}
    </GameInfoBadgesContext.Provider>
  );
};

/// Read-side hook. Returns the [`EMPTY_BADGES_CONTEXT`] sentinel when
/// no provider wraps the consumer, so library views that render in
/// non-standard mount points (direct-launch shell, tests) don't crash.
export function useGameInfoBadges(): BadgesContextValue {
  return useContext(GameInfoBadgesContext);
}
