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

import { createEffect, createMemo, createSignal, For, Match, onCleanup, onMount, Show, Switch, type Component } from "solid-js";
import VirtualLibraryGrid from "@oa/platform/components/VirtualLibraryGrid";
import { confirm } from "@oa/platform/lib/confirm";
import GameDetailPanel from "./GameDetailPanel";
import { HintRegion } from "@oa/platform/nav";
import {
  activateFocusGroup,
  activeFocusGroupId,
  groupsVersion,
  useDomQueryFocusGroup,
} from "@oa/platform/nav";
import type { EntryGroup } from "@oa/platform/library/filter";
import type { RomEntry } from "@oa/platform/library/types";
import { SMART_LISTS, type SmartListKind } from "@oa/platform/library/smartLists";
import { useTheme } from "@oa/platform/theme/host";

/// Slice 12 — the active sidebar entry now spans two kinds: the
/// existing built-in smart-list set OR a custom collection by id.
/// `kind` discriminates; `id` is the matching identifier in each
/// case (SmartListKind vs the SQLite-side collection row id).
///
/// Unified Navigation Tree Slice 2 — the smart-list predicates +
/// metadata now live in the shared `@oa/platform/library/smartLists`
/// registry (one source for this tab AND the nav-tree `filter` nodes).
type ActiveList =
  | { kind: "smart"; id: SmartListKind }
  | { kind: "custom"; id: string };

const CollectionsPage: Component = () => {
  const ctx = useTheme();
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
    return SMART_LISTS.find((l) => l.kind === cur.id) ?? null;
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
      if (!list) return [];
      const evalCtx = { nowSecs: Math.floor(Date.now() / 1000) };
      const matches = ctx.library.state.entries.filter((e) => list.predicate(e, evalCtx));
      if (list.kind === "recentlyPlayed" || list.kind === "lastPlayed") {
        return [...matches].sort((a, b) => (b.lastPlayedAt ?? 0) - (a.lastPlayedAt ?? 0));
      }
      if (list.kind === "hiddenGems") {
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

  /// Operator clicks "+ New collection" — defers to the App-level
  /// NewCollectionDialog (registered on the ThemeContext) so
  /// every entry point uses the same Dialog primitive surface.
  function handleNewCollection() {
    ctx.onOpenNewCollection(null);
  }

  /// Right-click target for the per-row context menu (Rename /
  /// Delete). `null` = no menu open. Position is the click coords
  /// so the popover anchors to the click site.
  const [rowContextFor, setRowContextFor] = createSignal<{
    collectionId: string;
    currentName: string;
    position: { x: number; y: number };
  } | null>(null);

  function openRowContext(
    collectionId: string,
    currentName: string,
    e: MouseEvent,
  ) {
    e.preventDefault();
    setRowContextFor({
      collectionId,
      currentName,
      position: { x: e.clientX, y: e.clientY },
    });
  }
  function closeRowContext() {
    setRowContextFor(null);
  }

  function handleRename(target: { collectionId: string; currentName: string }) {
    closeRowContext();
    ctx.onOpenRenameCollection(target.collectionId, target.currentName);
  }

  async function handleDelete(target: { collectionId: string; currentName: string }) {
    closeRowContext();
    const ok = await confirm(
      `Delete \"${target.currentName}\"? Games stay in your library; the collection and its membership list are removed.`,
      { title: "Delete collection", confirmLabel: "Delete", danger: true },
    );
    if (!ok) return;
    // If the deleted collection is currently the active list, fall
    // back to Favorites so the center pane keeps something rendered.
    const cur = activeList();
    if (cur.kind === "custom" && cur.id === target.collectionId) {
      setActiveList({ kind: "smart", id: "favorites" });
    }
    await ctx.customCollections.deleteCollection(target.collectionId);
  }

  // After a new collection lands in the store (createCollection
  // resolved server-side and refresh() updated `collections`), jump
  // the sidebar selection to it so the operator immediately sees the
  // empty-state copy nudging them toward Add to collection ▸. We
  // detect "new collection just appeared" by tracking the count
  // delta — simpler than threading a promise back through the
  // dialog.
  let prevCount = ctx.customCollections.state.collections.length;
  createEffect(() => {
    const cur = ctx.customCollections.state.collections;
    if (cur.length > prevCount) {
      const latest = cur[cur.length - 1];
      if (latest) setActiveList({ kind: "custom", id: latest.id });
    }
    prevCount = cur.length;
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
          dpad: "Switch region",
          stick: "Navigate",
          Confirm: "Play",
          Back: "Back",
          Secondary: "Search",
          Tertiary: "Favorite",
          PrevSection: "Prev tab",
          NextSection: "Next tab",
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
                        onContextMenu={(e) => openRowContext(col.id, col.name, e)}
                        class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-xs transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
                        classList={{
                          "bg-(--color-system-accent)/15 text-(--color-oa-ink)": isActive(),
                          "text-(--color-oa-ink-dim) hover:bg-white/[0.04] hover:text-(--color-oa-ink)":
                            !isActive(),
                        }}
                        aria-current={isActive() ? "page" : undefined}
                        title={`${col.name}\n(Right-click for rename / delete)`}
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
                  return cur.kind === "smart" && cur.id === list.kind;
                };
                const count = createMemo(() => {
                  const evalCtx = { nowSecs: Math.floor(Date.now() / 1000) };
                  return ctx.library.state.entries.filter((e) =>
                    list.predicate(e, evalCtx),
                  ).length;
                });
                return (
                  <li>
                    <button
                      type="button"
                      onClick={(e) => {
                        e.currentTarget.blur();
                        setActiveList({ kind: "smart", id: list.kind });
                      }}
                      class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-xs transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
                      classList={{
                        "bg-(--color-system-accent)/15 text-(--color-oa-ink)": isActive(),
                        "text-(--color-oa-ink-dim) hover:bg-white/[0.04] hover:text-(--color-oa-ink)":
                          !isActive(),
                      }}
                      aria-current={isActive() ? "page" : undefined}
                    >
                      <span class="w-4 text-center text-sm">{list.glyph}</span>
                      <span class="truncate">{list.label}</span>
                      <span class="ml-auto text-[0.6rem] text-(--color-oa-ink-dim)">
                        {count()}
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
                          : id === "recentlyPlayed"
                            ? "Play a game to populate this list. Sessions are tracked from launch to exit."
                            : id === "lastPlayed"
                              ? "Play any game to start tracking your history. Once a session ends the game shows up here."
                              : id === "hiddenGems"
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

      {/* Slice 12E — right-click context popover for a custom-
          collection sidebar row. Rename opens the Dialog primitive
          in rename mode; Delete runs a confirm before firing. Closes
          on outside-click + Escape via the global handlers below. */}
      <Show when={rowContextFor()}>
        {(target) => <CollectionRowContextMenu target={target()} onClose={closeRowContext} onRename={handleRename} onDelete={(t) => void handleDelete(t)} />}
      </Show>
    </div>
  );
};

/// Tiny popover with Rename / Delete actions for one custom-
/// collection row. Closes on Esc + outside click; keyboard A on the
/// focused row activates it (same pattern TileContextMenu uses).
const CollectionRowContextMenu: Component<{
  target: { collectionId: string; currentName: string; position: { x: number; y: number } };
  onClose: () => void;
  onRename: (target: { collectionId: string; currentName: string }) => void;
  onDelete: (target: { collectionId: string; currentName: string }) => void;
}> = (props) => {
  const t = () => ({
    collectionId: props.target.collectionId,
    currentName: props.target.currentName,
  });
  function onWindowKey(e: KeyboardEvent) {
    if (e.key === "Escape") props.onClose();
  }
  function onWindowClick(e: MouseEvent) {
    const target = e.target as HTMLElement | null;
    if (!target || !target.closest("[data-collection-row-ctx]")) {
      props.onClose();
    }
  }
  onMount(() => {
    window.addEventListener("keydown", onWindowKey, true);
    window.addEventListener("mousedown", onWindowClick, true);
  });
  onCleanup(() => {
    window.removeEventListener("keydown", onWindowKey, true);
    window.removeEventListener("mousedown", onWindowClick, true);
  });
  return (
    <div
      data-collection-row-ctx
      class="fixed z-50 min-w-[12rem] overflow-hidden rounded-md border border-white/10 bg-(--color-oa-bg-deep)/95 text-sm shadow-2xl shadow-black/60 backdrop-blur"
      style={{
        left: `${props.target.position.x}px`,
        top: `${props.target.position.y}px`,
      }}
      onClick={(e) => e.stopPropagation()}
    >
      <div class="border-b border-white/5 px-3 py-2">
        <p class="truncate text-xs font-medium text-(--color-oa-ink)">
          {props.target.currentName}
        </p>
        <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
          Custom collection
        </p>
      </div>
      <ul class="py-1">
        <li>
          <button
            type="button"
            onClick={() => props.onRename(t())}
            class="flex w-full items-center px-3 py-1.5 text-left text-sm text-(--color-oa-ink) hover:bg-white/[0.06]"
          >
            Rename…
          </button>
        </li>
        <li>
          <button
            type="button"
            onClick={() => props.onDelete(t())}
            class="flex w-full items-center px-3 py-1.5 text-left text-sm text-(--color-oa-ink-dim) hover:bg-red-500/10 hover:text-(--color-oa-ink)"
          >
            Delete…
          </button>
        </li>
      </ul>
    </div>
  );
};

export default CollectionsPage;
