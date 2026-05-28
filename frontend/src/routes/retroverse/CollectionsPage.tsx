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
//   - Right:  <RightDetailPanel> showing focusedEntry — same shape as
//             LIBRARY's right pane.
//
// Smart-list predicates run over the existing LibraryStore — no separate
// fetch. Slice 11a wired the favorite + completed + last_played_at columns
// into RomEntry; those + `players` drive the four implemented smart lists.
// Hidden Gems + Last Played are documented placeholders (depend on rating
// data + chronological play-order semantics not yet shipped).

import { createMemo, createSignal, For, Match, Show, Switch, type Component } from "solid-js";
import VirtualLibraryGrid from "../../components/VirtualLibraryGrid";
import GameDetailPanel from "./GameDetailPanel";
import { HintRegion } from "../../nav/HintBar";
import { activateFocusGroup, useDomQueryFocusGroup } from "../../nav/focus";
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
    description: "Highly rated but barely played — needs rating data not yet enriched.",
    predicate: null,
  },
  {
    id: "last-played",
    label: "Last played",
    glyph: "🏁",
    description: "Chronological play-order — needs session-end log data.",
    predicate: null,
  },
];

const CollectionsPage: Component = () => {
  const ctx = useRetroverse();
  const [activeSmartListId, setActiveSmartListId] = createSignal<SmartListId>("favorites");

  // Retroverse-UI controller-nav v2 — per-region focus groups (per
  // operator spec). DPad LEFT/RIGHT transfers sidebar ↔ center ↔
  // right; UP/DOWN stays within. The grid's "library-grid" inner
  // group still auto-activates on click but the page-level center
  // group is the landing surface for DPad.
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
    onDirection: (dir) => {
      if (dir === "right") {
        activateFocusGroup(CENTER_ID);
        return true;
      }
      return false;
    },
  });
  useDomQueryFocusGroup({
    id: CENTER_ID,
    containerRef: () => centerRef,
    orientation: "vertical",
    autoActivate: false,
    onActivate: (_i, el) => el.click(),
    onDirection: (dir) => {
      if (dir === "left") {
        activateFocusGroup(LEFT_ID);
        return true;
      }
      if (dir === "right") {
        activateFocusGroup(RIGHT_ID);
        return true;
      }
      return false;
    },
  });
  useDomQueryFocusGroup({
    id: RIGHT_ID,
    containerRef: () => rightRef,
    orientation: "vertical",
    autoActivate: false,
    onActivate: (_i, el) => el.click(),
    onDirection: (dir) => {
      if (dir === "left") {
        activateFocusGroup(CENTER_ID);
        return true;
      }
      return false;
    },
  });
  const activeSmartList = () =>
    SMART_LISTS.find((l) => l.id === activeSmartListId()) ?? SMART_LISTS[0]!;

  // Filtered entries for the active smart list. Sorting policy: Recently
  // played descends on lastPlayedAt; everything else falls back on
  // title-natural (the order the LibraryStore already returns).
  const filteredEntries = createMemo<RomEntry[]>(() => {
    const list = activeSmartList();
    if (!list.predicate) return [];
    const matches = ctx.library.state.entries.filter(list.predicate);
    if (list.id === "recent") {
      return [...matches].sort((a, b) => (b.lastPlayedAt ?? 0) - (a.lastPlayedAt ?? 0));
    }
    return matches;
  });

  // Wrap the filtered entries in a single EntryGroup so VirtualLibraryGrid
  // can render them. No grouping at this surface — the header card
  // already names the collection; visual grouping would feel redundant.
  const groups = createMemo<EntryGroup[]>(() => {
    const entries = filteredEntries();
    if (entries.length === 0) return [];
    return [{ id: activeSmartList().id, label: "", entries }];
  });

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
        {/* MY COLLECTIONS — empty + disabled new-button until Slice 12. */}
        <section>
          <p class="px-2 text-[0.55rem] font-semibold uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
            My collections
          </p>
          <p class="mt-2 px-2 text-[0.65rem] text-(--color-oa-ink-dim)/70">
            No custom lists yet. Custom collections — manual and smart-query —
            land in a follow-up slice.
          </p>
          <button
            type="button"
            disabled
            class="mt-2 w-full rounded-md border border-dashed border-white/10 px-2 py-1.5 text-left text-xs text-(--color-oa-ink-dim)/60 transition"
            aria-disabled="true"
            title="Custom collection persistence ships in Phase C3 Slice 12."
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
                const isActive = () => activeSmartListId() === list.id;
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
                        setActiveSmartListId(list.id);
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
            <span class="text-2xl text-(--color-system-accent)">{activeSmartList().glyph}</span>
            <h1 class="text-xl font-semibold uppercase tracking-widest text-(--color-oa-ink)">
              {activeSmartList().label}
            </h1>
            <span class="ml-auto rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Built-in · read-only
            </span>
          </div>
          <p class="mt-2 text-sm text-(--color-oa-ink-dim)">
            {activeSmartList().description}
          </p>
          <p class="mt-1 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)/60">
            {filteredEntries().length}{" "}
            {filteredEntries().length === 1 ? "game" : "games"}
          </p>
        </header>

        {/* Grid — empty-state card when 0 matches. */}
        <div class="min-h-0 flex-1 overflow-hidden">
          <Switch>
            <Match when={activeSmartList().predicate === null}>
              <div class="flex h-full items-center justify-center p-12">
                <div class="max-w-md rounded-xl border border-dashed border-white/10 bg-white/[0.02] p-8 text-center">
                  <p class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
                    Coming in a follow-up slice
                  </p>
                  <p class="mt-3 text-sm text-(--color-oa-ink-dim)">
                    {activeSmartList().description}
                  </p>
                </div>
              </div>
            </Match>
            <Match when={filteredEntries().length === 0}>
              <div class="flex h-full items-center justify-center p-12">
                <div class="max-w-md rounded-xl border border-dashed border-white/10 bg-white/[0.02] p-8 text-center">
                  <p class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
                    Empty
                  </p>
                  <p class="mt-3 text-sm text-(--color-oa-ink-dim)">
                    {activeSmartList().id === "favorites"
                      ? "Heart a game from any tile (or the right-click menu) to add it to Favorites."
                      : activeSmartList().id === "completed"
                        ? "Mark a game complete from the tile context menu to add it here."
                        : activeSmartList().id === "recent"
                          ? "Play a game to populate this list. Sessions are tracked from launch to exit."
                          : "No games match this filter yet."}
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
