// Retroverse-UI Phase C5 — DISCOVER tab.
//
// Three-pane "learn and wander" surface per docs/PLANS/discover-tab-
// retroverse.md. The full design has 9 axes; v1 ships with four
// powered by data we already collect (year / genre / region /
// developer columns in the games table) plus stub empty-states for
// the five that depend on Phase C6 content-packs.
//
// Working axes derive their facets in-memory from the LibraryStore
// — at OA's library scale (<10K rows) a single .filter pass per
// render is well under a millisecond. Future C6 work adds editorial
// articles / spotlights / anniversaries / cult-classic / lost-game
// picks that feed the remaining axes.

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
import { useMedia, type GameMetadata } from "../../library/media";
import type { RomEntry } from "../../library/types";
import { useRetroverse } from "./context";

type AxisId =
  | "featured"
  | "on-this-day"
  | "by-era"
  | "by-genre"
  | "by-publisher"
  | "by-developer"
  | "system-dive"
  | "cult-classics"
  | "lost-games";

type AxisDef = {
  id: AxisId;
  label: string;
  glyph: string;
  /// Short description shown as the active-axis header subtitle.
  description: string;
  /// True for axes that drive a facet picker + grid (genre / region /
  /// developer / era). False for stub axes that show an empty-state
  /// card pointing at Phase C6 content-packs.
  wired: boolean;
};

const AXES: readonly AxisDef[] = [
  {
    id: "featured",
    label: "Featured",
    glyph: "▣",
    description:
      "Rotating editorial picks — articles + system spotlights + anniversaries.",
    wired: false,
  },
  {
    id: "on-this-day",
    label: "On this day",
    glyph: "📅",
    description:
      "Anniversaries — release dates landing today across years.",
    wired: false,
  },
  {
    id: "by-era",
    label: "By era",
    glyph: "⏳",
    description:
      "Browse the library by hardware era. Each row pulls in games released within that decade.",
    wired: true,
  },
  {
    id: "by-genre",
    label: "By genre",
    glyph: "🎮",
    description: "Browse the library grouped by genre.",
    wired: true,
  },
  {
    id: "by-publisher",
    label: "By publisher",
    glyph: "🏷",
    description: "Browse the library grouped by publisher.",
    wired: true,
  },
  {
    id: "by-developer",
    label: "By developer",
    glyph: "🏢",
    description: "Browse the library grouped by developer / studio.",
    wired: true,
  },
  {
    id: "system-dive",
    label: "System dive",
    glyph: "🎯",
    description:
      "Deep dive into one system — history, key games, technical detail. Use the HOME tab in the meantime; SYSTEM DIVE adds editorial overlays in a content-pack release.",
    wired: false,
  },
  {
    id: "cult-classics",
    label: "Cult classics",
    glyph: "💎",
    description: "Under-celebrated games with editorial framing.",
    wired: false,
  },
  {
    id: "lost-games",
    label: "Lost games",
    glyph: "👻",
    description:
      "Cancelled / unreleased / prototype entries — preservation angle.",
    wired: false,
  },
];

/// Era buckets keyed off the `year` column. Tuned to the systems OA
/// covers — pre-Atari arcade through 2000s handhelds. Games with
/// unknown year fall into "Unknown / not enriched."
type EraBucket = {
  id: string;
  label: string;
  /// Inclusive year range; null means "any year" (Unknown bucket).
  range: [number, number] | null;
};

const ERA_BUCKETS: readonly EraBucket[] = [
  { id: "70s-arcade", label: "70s arcade", range: [1970, 1979] },
  { id: "8-bit", label: "8-bit era (1980–1985)", range: [1980, 1985] },
  { id: "16-bit", label: "16-bit war (1986–1992)", range: [1986, 1992] },
  { id: "32-64-bit", label: "32 / 64-bit (1993–1999)", range: [1993, 1999] },
  { id: "handheld-modern", label: "Modern handheld (2000–2010)", range: [2000, 2010] },
  { id: "post-2010", label: "Post-2010 retro releases", range: [2011, 9999] },
  { id: "unknown", label: "Unknown / not enriched", range: null },
];

const DiscoverPage: Component = () => {
  const ctx = useRetroverse();
  const media = useMedia();
  const [activeAxisId, setActiveAxisId] = createSignal<AxisId>("by-genre");
  /// Picked facet within the active axis — collects "current genre"
  /// for By genre, "current region" for By region, etc. Reset when
  /// the axis changes so a previous axis's pick doesn't leak.
  const [activeFacet, setActiveFacet] = createSignal<string | null>(null);

  function pickAxis(id: AxisId) {
    setActiveAxisId(id);
    setActiveFacet(null);
  }

  // Page-level focus regions — same shape as the other Retroverse
  // pages. LEFT = axis list, CENTER = facet picker + grid, RIGHT =
  // GameDetailPanel.
  let leftRef: HTMLElement | undefined;
  let centerRef: HTMLElement | undefined;
  let rightRef: HTMLElement | undefined;
  const LEFT_ID = "retroverse-discover-left";
  const CENTER_ID = "retroverse-discover-center";
  const RIGHT_ID = "retroverse-discover-right";
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
  // Delegating effect — when CENTER becomes active and the grid is
  // mounted, hand off to "library-grid" for 2D nav (same pattern as
  // LIBRARY / COLLECTIONS).
  createEffect(() => {
    groupsVersion();
    if (activeFocusGroupId() === CENTER_ID) {
      activateFocusGroup("library-grid");
    }
  });

  const activeAxis = () =>
    AXES.find((a) => a.id === activeAxisId()) ?? AXES[0]!;

  /// Facet list for the active axis. Empty for stub axes (which render
  /// their empty-state card instead). Sorted by count descending so
  /// the most populous facet sits at the top. Reads from MediaContext
  /// metadata (year / genre / developer / publisher) — that's where
  /// the libretro-database sync writes enriched fields, not the
  /// games-table columns. `media.media(romId)?.metadata` returns the
  /// canonical GameMetadata for the row.
  const facetList = createMemo<{ value: string; count: number }[]>(() => {
    const axis = activeAxis();
    if (!axis.wired) return [];
    const counts = new Map<string, number>();
    const entries = ctx.library.state.entries;
    for (const e of entries) {
      if (e.seed) continue;
      const meta = media.media(e.id)?.metadata;
      const value = facetValueFor(axis.id, meta);
      if (value === null) continue;
      counts.set(value, (counts.get(value) ?? 0) + 1);
    }
    return Array.from(counts.entries())
      .map(([value, count]) => ({ value, count }))
      .sort((a, b) => b.count - a.count || a.value.localeCompare(b.value));
  });

  /// Filtered entries for the picked facet. Empty until the operator
  /// selects a facet from the picker.
  const filteredEntries = createMemo<RomEntry[]>(() => {
    const axis = activeAxis();
    if (!axis.wired) return [];
    const facet = activeFacet();
    if (!facet) return [];
    return ctx.library.state.entries.filter((e) => {
      if (e.seed) return false;
      const meta = media.media(e.id)?.metadata;
      return facetValueFor(axis.id, meta) === facet;
    });
  });

  const groups = createMemo<EntryGroup[]>(() => {
    const entries = filteredEntries();
    if (entries.length === 0) return [];
    return [{ id: activeAxisId(), label: "", entries }];
  });

  return (
    <div
      class="grid h-full w-full"
      style={{
        "grid-template-columns": "260px minmax(0,1fr) 360px",
      }}
    >
      <HintRegion
        hints={{
          dpad: "Switch region",
          stick: "Navigate",
          a: "Open",
          b: "Back",
          x: "Search",
          y: "Save for later",
          l1: "Prev tab",
          r1: "Next tab",
        }}
      />

      {/* Left — EXPLORE axes. Wired axes render normally; stub ones
          dim to 60% so the operator can see at a glance what's
          available in v1 vs what's coming with content-packs. */}
      <aside
        ref={(el) => (leftRef = el)}
        class="min-w-0 overflow-y-auto border-r border-white/5 px-3 py-4"
      >
        <p class="px-2 text-[0.55rem] font-semibold uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
          Explore
        </p>
        <ul class="mt-1.5 flex flex-col gap-0.5">
          <For each={AXES}>
            {(axis) => {
              const isActive = () => activeAxisId() === axis.id;
              return (
                <li>
                  <button
                    type="button"
                    onClick={(e) => {
                      e.currentTarget.blur();
                      pickAxis(axis.id);
                    }}
                    class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-xs transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
                    classList={{
                      "bg-(--color-system-accent)/15 text-(--color-oa-ink)": isActive(),
                      "text-(--color-oa-ink-dim) hover:bg-white/[0.04] hover:text-(--color-oa-ink)":
                        !isActive(),
                      "opacity-60": !axis.wired,
                    }}
                    aria-current={isActive() ? "page" : undefined}
                  >
                    <span class="w-4 text-center text-sm">{axis.glyph}</span>
                    <span class="truncate">{axis.label}</span>
                    <Show when={!axis.wired}>
                      <span class="ml-auto text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)/70">
                        soon
                      </span>
                    </Show>
                  </button>
                </li>
              );
            }}
          </For>
        </ul>

        <div class="mt-6 border-t border-white/5 pt-4">
          <p class="px-2 text-[0.6rem] leading-relaxed text-(--color-oa-ink-dim)/70">
            ⓘ Editorial axes (featured / anniversaries / cult / lost) ship via
            community + OA-curated content packs. Phase C6 wires the pack
            mechanism; until then those axes show empty states.
          </p>
        </div>
      </aside>

      {/* Center — axis-specific body. */}
      <section
        ref={(el) => (centerRef = el)}
        class="flex min-h-0 min-w-0 flex-col overflow-hidden"
      >
        <header class="border-b border-white/5 px-8 py-5">
          <div class="flex items-center gap-3">
            <span class="text-2xl text-(--color-system-accent)">{activeAxis().glyph}</span>
            <h1 class="text-xl font-semibold uppercase tracking-widest text-(--color-oa-ink)">
              {activeAxis().label}
            </h1>
            <Show when={!activeAxis().wired}>
              <span class="ml-auto rounded border border-amber-400/30 bg-amber-500/10 px-2 py-0.5 text-[0.6rem] uppercase tracking-widest text-amber-300">
                Ships with content packs
              </span>
            </Show>
          </div>
          <p class="mt-2 max-w-2xl text-sm text-(--color-oa-ink-dim)">
            {activeAxis().description}
          </p>
        </header>

        <div class="min-h-0 flex-1 overflow-y-auto px-8 py-6">
          <Switch>
            {/* Wired branch — facet picker + grid */}
            <Match when={activeAxis().wired}>
              <Show
                when={facetList().length > 0}
                fallback={
                  <EmptyCard
                    headline="No data yet"
                    body="Library entries don't carry the metadata this axis needs. Run Settings → Game media → Sync metadata to enrich, then come back."
                  />
                }
              >
                <FacetPicker
                  axis={activeAxisId()}
                  facets={facetList()}
                  active={activeFacet()}
                  onPick={setActiveFacet}
                />
                <Show
                  when={activeFacet() !== null}
                  fallback={
                    <EmptyCard
                      headline="Pick a facet above"
                      body={`Browse ${facetList().length} ${facetCountWord(activeAxisId())} — click one to fill the grid.`}
                    />
                  }
                >
                  <Show
                    when={filteredEntries().length > 0}
                    fallback={
                      <EmptyCard
                        headline="No games in this facet"
                        body="The facet was tagged but no library entries match. (May indicate a metadata inconsistency.)"
                      />
                    }
                  >
                    <div class="mt-6 min-h-[400px]">
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
                    </div>
                  </Show>
                </Show>
              </Show>
            </Match>

            {/* Stub branch — empty state pointing at content packs */}
            <Match when={!activeAxis().wired}>
              <EmptyCard
                headline="Ships with content packs"
                body="Editorial content (articles, spotlights, anniversaries, cult / lost-game picks) is distributed via Phase C6 content packs. See docs/PLANS/content-packs.md for the design. Until that arc lands, the four facet axes (era / genre / region / developer) are the working part of DISCOVER."
              />
            </Match>
          </Switch>
        </div>
      </section>

      {/* Right — focused-game detail. Same pattern as LIBRARY /
          COLLECTIONS. */}
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
                  Focus a tile to see its detail. Wandering through facets is
                  what DISCOVER's for — pick a region you haven't tried, a
                  developer you don't recognise.
                </p>
              </div>
            </div>
          }
        >
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

// --- Helpers ----------------------------------------------------------

/// Pull the facet value off MediaContext metadata for a given axis.
/// Returns null when the field is missing — the entry is skipped in
/// counts / lists. By-era special-cases the Unknown bucket so rows
/// without a year aren't dropped entirely.
function facetValueFor(axis: AxisId, meta: GameMetadata | undefined): string | null {
  switch (axis) {
    case "by-genre":
      return meta?.genre?.trim() || null;
    case "by-publisher":
      return meta?.publisher?.trim() || null;
    case "by-developer":
      return meta?.developer?.trim() || null;
    case "by-era": {
      const y = meta?.year;
      if (y === undefined || y === null) {
        return ERA_BUCKETS[ERA_BUCKETS.length - 1]?.id ?? null;
      }
      for (const b of ERA_BUCKETS) {
        if (b.range && y >= b.range[0] && y <= b.range[1]) return b.id;
      }
      return ERA_BUCKETS[ERA_BUCKETS.length - 1]?.id ?? null;
    }
    default:
      return null;
  }
}

function facetCountWord(axis: AxisId): string {
  switch (axis) {
    case "by-genre":
      return "genres";
    case "by-publisher":
      return "publishers";
    case "by-developer":
      return "developers";
    case "by-era":
      return "eras";
    default:
      return "facets";
  }
}

function facetDisplayLabel(axis: AxisId, value: string): string {
  if (axis === "by-era") {
    return ERA_BUCKETS.find((b) => b.id === value)?.label ?? value;
  }
  return value;
}

const FacetPicker: Component<{
  axis: AxisId;
  facets: { value: string; count: number }[];
  active: string | null;
  onPick: (v: string) => void;
}> = (props) => (
  <div class="flex flex-wrap gap-2">
    <For each={props.facets}>
      {(facet) => {
        const isActive = () => props.active === facet.value;
        return (
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              props.onPick(facet.value);
            }}
            class="flex items-center gap-2 rounded-full border px-3 py-1 text-xs transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
            classList={{
              "border-(--color-system-accent) bg-(--color-system-accent)/15 text-(--color-oa-ink)": isActive(),
              "border-white/10 bg-white/[0.04] text-(--color-oa-ink-dim) hover:bg-white/[0.08] hover:text-(--color-oa-ink)":
                !isActive(),
            }}
          >
            <span>{facetDisplayLabel(props.axis, facet.value)}</span>
            <span class="text-[0.6rem] text-(--color-oa-ink-dim)">{facet.count}</span>
          </button>
        );
      }}
    </For>
  </div>
);

const EmptyCard: Component<{ headline: string; body: string }> = (props) => (
  <div class="rounded-xl border border-dashed border-white/10 bg-white/[0.02] p-8 text-center">
    <p class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
      {props.headline}
    </p>
    <p class="mx-auto mt-3 max-w-md text-sm text-(--color-oa-ink-dim)">
      {props.body}
    </p>
  </div>
);

export default DiscoverPage;
