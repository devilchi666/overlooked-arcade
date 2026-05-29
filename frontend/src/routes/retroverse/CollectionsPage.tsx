// Retroverse-UI Phase C3 Slice 11b — COLLECTIONS tab.
//
// Three-pane internal layout matching docs/PLANS/collections-tab-retroverse.md:
//   - Left:   sidebar with three groups (MY COLLECTIONS / SMART LISTS /
//             CURATED). MY COLLECTIONS empty + new-button disabled until
//             Slice 12 lands persistence; CURATED empty until content-packs
//             infra ships (Phase C6).
//   - Center: collection header card (count + Built-in badge) + filtered
//             tile grid via VirtualLibraryGrid. Empty-state card when the
//             active smart-list has no matches.
//   - Right:  GameDetailPanel when an entry is focused — same shape as
//             LIBRARY's right pane.
//
// Smart-list predicates run over the existing LibraryStore — no separate
// fetch. Slice 11a wired the favorite + completed + last_played_at columns
// into RomEntry. All six smart lists now ship real predicates: Favorites
// / Recently played / Completed / Multi-player + the follow-up additions
// Hidden Gems (rating ≥ 4 OR favorite, AND playTimeSecs < 30 min) and
// Last Played (every entry with a lastPlayedAt, full chronological
// history — Recently played is its 30-day subset).

import { createEffect, createMemo, createSignal, For, Match, Show, Switch, type Component } from "solid-js";
import VirtualLibraryGrid from "../../components/VirtualLibraryGrid";
import GameDetailPanel from "./GameDetailPanel";
import { HintRegion } from "../../nav/HintBar";
import {
  activateFocusGroup,
  activeFocusGroupId,
  groupsVersion,
  useDomQueryFocusGroup,
} from "../../nav/focus";
import type { EntryGroup } from "../../library/filter";
import type { RomEntry } from "../../library/types";
import { useRetroverse } from "./context";

type SmartListId =
  | "favorites"
  | "recent"
  | "completed"
  | "multi-player"
  | "hidden-gems"
  | "last-played";

/// Slice 12 — the active sidebar entry now spans two kinds: the
/// existing built-in smart-list set OR a custom collection by id.
/// `kind` discriminates; `id` is the matching identifier in each
/// case (SmartListId vs the SQLite-side collection row id).
type ActiveList =
  | { kind: "smart"; id: SmartListId }
  | { kind: "custom"; id: string };

type SmartListDef = {
  id: SmartListId;
  label: string;
  glyph: string;
  /// Description shown on the header card when this list is active.
  description: string;
  /// Predicate over a library entry — returns true to include it. When
  /// `null` the list is a placeholder (Hidden Gems / Last Played); UI
  /// shows a "wired-in-a-follow-up" empty state.
  predicate: ((entry: RomEntry) => boolean) | null;
};

// 30-day recency window — matches the design doc's "rolling window."
const RECENT_WINDOW_SECONDS = 30 * 24 * 60 * 60;

// Hidden Gems "barely touched" threshold. Under 30 minutes of total
// play time across all sessions is the "you sampled it but never
// really got into it" zone — the operator's existing rating /
// favoriting then becomes the "this deserves another look" signal.
const HIDDEN_GEM_PLAY_THRESHOLD_SECS = 30 * 60;
// Hidden Gems rating cutoff. 4.0 / 5.0 covers "great" and above when
// the metadata enrichment populates the rating field; libraries
// without rating data fall back to the favorite-flagged half of the
// OR (so the list still surfaces something useful).
const HIDDEN_GEM_RATING_CUTOFF = 4.0;

const SMART_LISTS: readonly SmartListDef[] = [
  {
    id: "favorites",
    label: "Favorites",
    glyph: "❤",
    description: "Games you've ♥-tagged from any tile or context menu.",
    predicate: (entry) => Boolean(entry.favorite),
  },
  {
    id: "recent",
    label: "Recently played",
    glyph: "🕘",
    description: "Played within the last 30 days. Sorted by most recent.",
    predicate: (entry) => {
      if (!entry.lastPlayedAt) return false;
      const nowSecs = Math.floor(Date.now() / 1000);
      return nowSecs - entry.lastPlayedAt < RECENT_WINDOW_SECONDS;
    },
  },
  {
    id: "completed",
    label: "Completed",
    glyph: "✓",
    description: "Marked complete via the tile context menu.",
    predicate: (entry) => Boolean(entry.completed),
  },
  {
    id: "multi-player",
    label: "Multi-player",
    glyph: "👥",
    description: "Two-player or more (metadata-driven).",
    predicate: (entry) => (entry.players ?? 1) >= 2,
  },
  {
    id: "hidden-gems",
    label: "Hidden gems",
    glyph: "💎",
    description:
      "Highly rated or favorited, but you've barely touched them (< 30 minutes total).",
    predicate: (entry) => {
      const playTime = entry.playTimeSecs ?? 0;
      if (playTime >= HIDDEN_GEM_PLAY_THRESHOLD_SECS) return false;
      const highlyRated =
        typeof entry.rating === "number" &&
        entry.rating >= HIDDEN_GEM_RATING_CUTOFF;
      return highlyRated || Boolean(entry.favorite);
    },
  },
  {
    id: "last-played",
    label: "Last played",
    glyph: "🏁",
    description:
      "Every game you've ever launched, most recent first. The full play history — Recently played is the 30-day rolling subset.",
    predicate: (entry) => Boolean(entry.lastPlayedAt),
  },
];

const CollectionsPage: Component = () => {
  const ctx = useRetroverse();
  const [activeList, setActiveList] = createSignal<ActiveList>({
    kind: "smart",
    id: "favorites",
  });

  // When a custom collection becomes active, hydrate its member id
  // set on first selection. `ensureMembers` is idempotent — repeat
  // selections of the same collection are O(1) cache hits.
  createEffect(() => {
    const list = activeList();
    if (list.kind === "custom") {
      void ctx.customCollections.ensureMembers(list.id);
    }
  });

  // Per-region focus groups (unified-focus model). `neighbours` drives
  // both DPad edge-spillover and L1/R1 shoulder-bumper transfer
  // (RetroverseShell intercepts L1/R1 globally for tab cycling).
  // The grid's "library-grid" inner group still auto-activates on
  // mouse click for the mouse flow; controller flow stays on the
  // page-level CENTER for COLLECTIONS smart-lists.
  let leftRef: HTMLElement | undefined;
  let centerRef: HTMLElement | undefined;
  let rightRef: HTMLElement | undefined;
  const LEFT_ID = "retroverse-collections-left";
  const CENTER_ID = "retroverse-collections-center";
  const RIGHT_ID = "retroverse-collections-right";
  useDomQueryFocusGroup({
    id: LEFT_ID,
    containerRef: () => leftRef,
    orientation: "vertical",
    onActivate: (_i, el) => el.click(),
    neighbours: { right: CENTER_ID },
  });
  useDomQueryFocusGroup({
    id: CENTER_ID,
    containerRef: () => centerRef,
    orientation: "vertical",
    autoActivate: false,
    onActivate: (_i, el) => el.click(),
    neighbours: { left: LEFT_ID, right: RIGHT_ID },
  });
  useDomQueryFocusGroup({
    id: RIGHT_ID,
    containerRef: () => rightRef,
    orientation: "vertical",
    autoActivate: false,
    onActivate: (_i, el) => el.click(),
    neighbours: { left: CENTER_ID },
  });

  // Same delegating pattern as LibraryPage — when CENTER becomes
  // active and the COLLECTIONS grid (which also registers under
  // "library-grid") is mounted, hand off so DPad walks the grid in
  // 2D. groupsVersion re-fires the effect when the grid mounts or
  // unmounts (e.g. switching smart-lists with vs without matches).
  createEffect(() => {
    groupsVersion();
    if (activeFocusGroupId() === CENTER_ID) {
      activateFocusGroup("library-grid");
    }
  });

  const activeSmartList = () => {
    const cur = activeList();
    if (cur.kind !== "smart") return null;
    return SMART_LISTS.find((l) => l.id === cur.id) ?? null;
  };

  const activeCustomCollection = () => {
    const cur = activeList();
    if (cur.kind !== "custom") return null;
    return ctx.customCollections.state.collections.find((c) => c.id === cur.id) ?? null;
  };

  // Filtered entries for the active list. Smart lists run their
  // predicate over `library.state.entries`. Custom collections filter
  // the same entries by membership lookup against the hydrated Set.
  // Sorting policy:
  //   Smart Recently played + Last played: descending on lastPlayedAt.
  //   Smart Hidden gems: descending on rating, then favorite.
  //   Smart everything else: title-natural (library order).
  //   Custom: collection sort_order (Rust-side ORDER BY); local order
  //     follows the Set's insertion order which mirrors add order.
  const filteredEntries = createMemo<RomEntry[]>(() => {
    const cur = activeList();
    if (cur.kind === "smart") {
      const list = activeSmartList();
      if (!list || !list.predicate) return [];
      const matches = ctx.library.state.entries.filter(list.predicate);
      if (list.id === "recent" || list.id === "last-played") {
        return [...matches].sort((a, b) => (b.lastPlayedAt ?? 0) - (a.lastPlayedAt ?? 0));
      }
      if (list.id === "hidden-gems") {
        return [...matches].sort((a, b) => {
          const ra = a.rating ?? 0;
          const rb = b.rating ?? 0;
          if (ra !== rb) return rb - ra;
          const fa = a.favorite ? 1 : 0;
          const fb = b.favorite ? 1 : 0;
          return fb - fa;
        });
      }
      return matches;
    }
    // Custom collection — filter library entries by membership Set.
    const memberIds = ctx.customCollections.state.members[cur.id];
    if (!memberIds || memberIds.size === 0) return [];
    return ctx.library.state.entries.filter((e) => memberIds.has(e.id));
  });

  // Wrap the filtered entries in a single EntryGroup so VirtualLibraryGrid
  // can render them. No grouping at this surface — the header card
  // already names the collection; visual grouping would feel redundant.
  const groups = createMemo<EntryGroup[]>(() => {
    const entries = filteredEntries();
    if (entries.length === 0) return [];
    const cur = activeList();
    return [{ id: cur.id, label: "", entries }];
  });

  // Header display info — name / glyph / description / badge / count.
  // Unified across smart + custom so the JSX below stays compact.
  const headerInfo = createMemo(() => {
    const cur = activeList();
    if (cur.kind === "smart") {
      const list = activeSmartList();
      return {
        name: list?.label ?? "Collection",
        glyph: list?.glyph ?? "★",
        description: list?.description ?? "",
        badge: "Built-in · read-only",
      };
    }
    const col = activeCustomCollection();
    return {
      name: col?.name ?? "Collection",
      glyph: "📁",
      description:
        col === null
          ? ""
          : col.memberCount === 0
            ? "No games yet. Right-click any tile in LIBRARY → Add to collection ▸ to drop entries in."
            : "Operator-built collection. Add or remove games from the tile context menu in any tab.",
      badge: "Custom · editable",
    };
  });

  /// Operator clicks "+ New collection". v1 uses window.prompt for the
  /// name — Slice D upgrades this to the Dialog primitive + supports
  /// rename / delete from a context menu on the sidebar row. The
  /// prompt is a placeholder, not the final UX.
  async function handleNewCollection() {
    const name = window.prompt("Name this collection:", "");
    if (!name || !name.trim()) return;
    const id = await ctx.customCollections.createCollection(name.trim());
    if (id) setActiveList({ kind: "custom", id });
  }

  return (
    <div
      class="grid h-full w-full"
      style={{
        "grid-template-columns": "260px minmax(0,1fr) 360px",
      }}
    >
      {/* Phase C3 hints — same shape as LIBRARY but Y becomes "Favorite"
          since heart toggling is the dominant curatorial action here. */}
      <HintRegion
        hints={{
          dpad: "Switch region",
          stick: "Navigate",
          a: "Play",
          b: "Back",
          x: "Search",
          y: "Favorite",
          l1: "Prev tab",
          r1: "Next tab",
        }}
      />

      {/* Left: collection sidebar with three groups. */}
      <aside
        ref={(el) => (leftRef = el)}
        class="min-w-0 overflow-y-auto border-r border-white/5 px-3 py-4"
      >
        {/* MY COLLECTIONS — operator-built lists from Slice 12. */}
        <section>
          <p class="px-2 text-[0.55rem] font-semibold uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
            My collections
          </p>
          <Show
            when={ctx.customCollections.state.collections.length > 0}
            fallback={
              <p class="mt-2 px-2 text-[0.65rem] text-(--color-oa-ink-dim)/70">
                No custom lists yet. Click + New collection below, then
                right-click any tile to add games.
              </p>
            }
          >
            <ul class="mt-1.5 flex flex-col gap-0.5">
              <For each={ctx.customCollections.state.collections}>
                {(col) => {
                  const isActive = () => {
                    const cur = activeList();
                    return cur.kind === "custom" && cur.id === col.id;
                  };
                  return (
                    <li>
                      <button
                        type="button"
                        onClick={(e) => {
                          e.currentTarget.blur();
                          setActiveList({ kind: "custom", id: col.id });
                        }}
                        class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-xs transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
                        classList={{
                          "bg-(--color-system-accent)/15 text-(--color-oa-ink)": isActive(),
                          "text-(--color-oa-ink-dim) hover:bg-white/[0.04] hover:text-(--color-oa-ink)":
                            !isActive(),
                        }}
                        aria-current={isActive() ? "page" : undefined}
                        title={col.name}
                      >
                        <span class="w-4 text-center text-sm">📁</span>
                        <span class="truncate">{col.name}</span>
                        <span class="ml-auto text-[0.6rem] text-(--color-oa-ink-dim)">
                          {col.memberCount}
                        </span>
                      </button>
                    </li>
                  );
                }}
              </For>
            </ul>
          </Show>
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              void handleNewCollection();
            }}
            class="mt-2 w-full rounded-md border border-dashed border-white/15 px-2 py-1.5 text-left text-xs text-(--color-oa-ink-dim) transition hover:border-(--color-system-accent)/50 hover:bg-white/[0.04] hover:text-(--color-oa-ink) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
          >
            + New collection
          </button>
        </section>

        {/* SMART LISTS — 4 wired + 2 placeholders. */}
        <section class="mt-6">
          <p class="px-2 text-[0.55rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)/70">
            Smart lists
          </p>
          <ul class="mt-1.5 flex flex-col gap-0.5">
            <For each={SMART_LISTS}>
              {(list) => {
                const isActive = () => {
                  const cur = activeList();
                  return cur.kind === "smart" && cur.id === list.id;
                };
                const isPlaceholder = () => list.predicate === null;
                const count = createMemo(() =>
                  list.predicate
                    ? ctx.library.state.entries.filter(list.predicate).length
                    : 0,
                );
                return (
                  <li>
                    <button
                      type="button"
                      onClick={(e) => {
                        e.currentTarget.blur();
                        setActiveList({ kind: "smart", id: list.id });
                      }}
                      class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-xs transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
                      classList={{
                        "bg-(--color-system-accent)/15 text-(--color-oa-ink)": isActive(),
                        "text-(--color-oa-ink-dim) hover:bg-white/[0.04] hover:text-(--color-oa-ink)":
                          !isActive(),
                        "opacity-60": isPlaceholder(),
                      }}
                      aria-current={isActive() ? "page" : undefined}
                    >
                      <span class="w-4 text-center text-sm">{list.glyph}</span>
                      <span class="truncate">{list.label}</span>
                      <span class="ml-auto text-[0.6rem] text-(--color-oa-ink-dim)">
                        {isPlaceholder() ? "—" : count()}
                      </span>
                    </button>
                  </li>
                );
              }}
            </For>
          </ul>
        </section>

        {/* CURATED — empty until content-packs ships. */}
        <section class="mt-6 border-t border-white/5 pt-4">
          <p class="px-2 text-[0.55rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)/60">
            Curated
          </p>
          <p class="mt-2 px-2 text-[0.65rem] text-(--color-oa-ink-dim)/70">
            No curated collection packs installed. Pack browser ships in
            Phase C6 (content-packs.md).
          </p>
        </section>
      </aside>

      {/* Center: header card + tile grid. */}
      <section
        ref={(el) => (centerRef = el)}
        class="flex min-h-0 min-w-0 flex-col overflow-hidden"
      >
        {/* Header card — name + count + flavor badge. */}
        <header class="border-b border-white/5 px-8 py-5">
          <div class="flex items-center gap-3">
            <span class="text-2xl text-(--color-system-accent)">{headerInfo().glyph}</span>
            <h1 class="text-xl font-semibold uppercase tracking-widest text-(--color-oa-ink)">
              {headerInfo().name}
            </h1>
            <span class="ml-auto rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              {headerInfo().badge}
            </span>
          </div>
          <p class="mt-2 text-sm text-(--color-oa-ink-dim)">
            {headerInfo().description}
          </p>
          <p class="mt-1 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)/60">
            {filteredEntries().length}{" "}
            {filteredEntries().length === 1 ? "game" : "games"}
          </p>
        </header>

        {/* Grid — empty-state card when 0 matches. */}
        <div class="min-h-0 flex-1 overflow-hidden">
          <Switch>
            <Match when={filteredEntries().length === 0}>
              <div class="flex h-full items-center justify-center p-12">
                <div class="max-w-md rounded-xl border border-dashed border-white/10 bg-white/[0.02] p-8 text-center">
                  <p class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
                    Empty
                  </p>
                  <p class="mt-3 text-sm text-(--color-oa-ink-dim)">
                    {(() => {
                      const cur = activeList();
                      if (cur.kind === "custom") {
                        return "This collection is empty. Right-click a tile in any tab and pick Add to collection ▸ to drop entries in.";
                      }
                      const id = cur.id;
                      return id === "favorites"
                        ? "Heart a game from any tile (or the right-click menu) to add it to Favorites."
                        : id === "completed"
                          ? "Mark a game complete from the tile context menu to add it here."
                          : id === "recent"
                            ? "Play a game to populate this list. Sessions are tracked from launch to exit."
                            : id === "last-played"
                              ? "Play any game to start tracking your history. Once a session ends the game shows up here."
                              : id === "hidden-gems"
                                ? "Either heart a few games you've barely played, or run Sync metadata to enrich ratings — entries you've spent < 30 minutes with will surface here."
                                : "No games match this filter yet.";
                    })()}
                  </p>
                </div>
              </div>
            </Match>
            <Match when={filteredEntries().length > 0}>
              <VirtualLibraryGrid
                groups={groups()}
                tileWidth={ctx.layout.libraryTileSize()}
                onLaunch={(entry) => void ctx.onLaunch(entry)}
                onShowSaves={ctx.onShowSaves}
                onPickContext={ctx.onPickContext}
                onFocus={ctx.setFocusedEntry}
                onShowInfo={ctx.onShowInfo}
                selectedId={() => ctx.focusedEntry()?.id ?? null}
                variantCountFor={(id) =>
                  ctx.library.groupsByVariantId().get(id)?.variants.length
                }
                onToggleFavorite={ctx.onToggleFavorite}
                focusGroupNeighbours={{ left: LEFT_ID, right: RIGHT_ID }}
              />
            </Match>
          </Switch>
        </div>
      </section>

      {/* Right: focused-game detail. Same shape as LIBRARY's right pane. */}
      <aside
        ref={(el) => (rightRef = el)}
        class="min-w-0 overflow-hidden border-l border-white/5"
      >
        <Show
          when={ctx.focusedEntry()}
          fallback={
            <div class="flex h-full items-center justify-center p-8">
              <div class="max-w-xs text-center">
                <p class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
                  No selection
                </p>
                <p class="mt-3 text-sm text-(--color-oa-ink-dim)">
                  Focus a game in the grid to see its detail here.
                </p>
              </div>
            </div>
          }
        >
          {/* Show's fallback handles null; the Show body unwraps via
              accessor so GameDetailPanel always gets a non-null entry. */}
          <GameDetailPanel
            entry={ctx.focusedEntry()!}
            onLaunch={(e) => void ctx.onLaunch(e)}
            onShowInfo={ctx.onShowInfo}
            onToggleFavorite={ctx.onToggleFavorite}
          />
        </Show>
      </aside>
    </div>
  );
};

export default CollectionsPage;
