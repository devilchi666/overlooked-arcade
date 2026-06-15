import { createEffect, createMemo, createSignal, Match, onCleanup, onMount, Show, Switch, type Component } from "solid-js";
import type { LibraryStore } from "@oa/platform/library/store";
import type { RomEntry } from "@oa/platform/library/types";
import type { SortKey, GroupBy, ViewMode } from "@oa/platform/layout/state";
import type { SidebarView } from "@oa/platform/layout/types";
import {
  collapseDiscSets,
  collapseVariantGroups,
  filterEntries,
  groupEntries,
  sortEntries,
} from "@oa/platform/library/filter";
import DiscPickerDialog from "./DiscPickerDialog";
import { useMedia } from "@oa/platform/library/media";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import { useDeclaredLayout } from "@oa/platform/theme/layoutResolver";
import { CarouselNav, WheelNav } from "@oa/platform/nav";
import type { ViewsStore } from "@oa/platform/views/store";
import { findNode, resolveNodeSystemIds } from "@oa/platform/views/resolver";
import { parsePlatformNodeId } from "@oa/platform/views/defaults";
import DetailListView from "./DetailListView";
import GridControls from "./GridControls";
import SystemHeader from "./SystemHeader";
import VirtualLibraryGrid from "./VirtualLibraryGrid";

/// The browse-appearance the LIBRARY view reads. Settings IA Slice 3 moved
/// these from the global `layout` store to PER-THEME storage: the theme owns
/// its tile size / sort / grouping / view mode and passes them in, so the
/// shared platform grid stays config-agnostic (it never reaches into
/// active-theme settings itself — that would invert the layer boundary).
export type LibraryAppearance = {
  sortKey: () => SortKey;
  groupBy: () => GroupBy;
  viewMode: () => ViewMode;
  tileSize: () => number;
  setTileSize: (px: number) => void;
};

type Props = {
  library: LibraryStore;
  appearance: LibraryAppearance;
  views: ViewsStore;
  currentView: SidebarView;
  searchQuery: string;
  onLaunch: (entry: RomEntry) => void;
  onShowSaves: (entry: RomEntry) => void;
  onPickContext: (entry: RomEntry, position: { x: number; y: number }) => void;
  onFocus: (entry: RomEntry) => void;
  /// Controller-nav: Y button on a focused tile opens the game info modal.
  /// Mouse path is unchanged (tile context menu still has its own entry).
  onShowInfo?: (entry: RomEntry) => void;
  /// Accessor for the currently-selected entry id (or null). Passed through
  /// to the grid / list views so tiles can render an accent ring on the
  /// active pick.
  selectedId?: () => string | null;
  onPickFolder: () => void;
  /// Phase 1B Slice 6 — primary CTA on the first-launch hero opens
  /// the guided-setup wizard. Required so any caller wiring a
  /// LibraryView gets a compile error if they forget to pass it.
  /// The legacy `onPickFolder` stays as the secondary muted-link
  /// affordance for operators who prefer the quick path.
  onImportWizard: () => void;
  /// Retroverse-UI Phase C3 — pass-through favorite toggle. Forwarded
  /// to VirtualLibraryGrid → LibraryTile. When omitted the heart
  /// overlay hides (gracefully degrades for surfaces that don't wire
  /// favorites).
  onToggleFavorite?: (entry: RomEntry, value: boolean) => void;
  /// Retroverse-UI LIBRARY polish — pass-through for the per-tile
  /// system-label header strip. See LibraryTile's prop docs.
  showSystemHeader?: boolean;
  /// Retroverse-UI unified focus — passes through to VirtualLibraryGrid's
  /// `focusGroupNeighbours` so the grid's DPad edge-spillover lands on
  /// the Retroverse page's per-region groups instead of the legacy
  /// left-sidebar / right-sidebar ids.
  gridFocusNeighbours?: { left?: string; right?: string };
  /// Boot a no-game-capable core's built-in browser (DOSBox / ScummVM).
  /// Forwarded to SystemHeader's "Boot without game" button. Supplied by
  /// the theme (LibraryPage) so platform components stay prop-driven and
  /// don't reach into the theme context (layer boundary).
  onBootWithoutGame?: (systemId: SystemId) => void;
};

/**
 * Library view — renders GridControls + the active view (Capsule grid or
 * Detail list). Owns the filter / sort / group pipeline so App.tsx stays
 * focused on shell-level wiring. Lives inside MediaProvider so widgets can
 * call useMedia() for cover URLs + metadata.
 */
const LibraryView: Component<Props> = (props) => {
  const media = useMedia();
  const getYear = (romId: string): number | undefined =>
    media.media(romId)?.metadata?.year;
  const systemDisplayName = (id: string): string =>
    systemThemes[id as SystemId]?.displayName ?? id;

  /// Resolve the active view-node selection to the SystemId set the
  /// library should restrict to. `null` means "no system filter"
  /// (currentView is `all`). For view-node:
  /// look up the node in the active view, resolve via the views
  /// resolver (handles container rules + synthesized-leaf fallback for
  /// deep-links outside the active view's tree).
  const viewSystemIds = createMemo<SystemId[] | null>(() => {
    const cv = props.currentView;
    if (cv.kind !== "view-node") return null;
    const active = props.views.activeView();
    if (active && active.id === cv.viewId) {
      return resolveNodeSystemIds(active, cv.nodeId);
    }
    // Active-view mismatch (rare — could happen mid-view-switch with a
    // stale deep-link). Fall back to synthesized-leaf interpretation.
    const synth = parsePlatformNodeId(cv.nodeId);
    return synth ? [synth] : [];
  });

  /// When the view-node selection resolves to exactly one system, that
  /// system drives the page chrome (header, data-system accent cascade,
  /// title). Container selections (PR-γ surface; PR-β doesn't render
  /// clickable containers) leave this null and fall back to the
  /// generic title.
  const selectedSystemId = createMemo<SystemId | null>(() => {
    const cv = props.currentView;
    if (cv.kind !== "view-node") return null;
    const active = props.views.activeView();
    if (active && active.id === cv.viewId) {
      const node = findNode(active, cv.nodeId);
      if (node && "kind" in node && node.kind === "platform") return node.systemId;
    }
    return parsePlatformNodeId(cv.nodeId);
  });

  // ARC 2 L3b (D32/D40): per-system layout for the game-browse view. The
  // active theme's `views["game-browse"]` (+ the user's persisted override, L5)
  // can pick a layout primitive for the CURRENT system; `useDeclaredLayout`
  // returns `undefined` when nothing is declared → we keep the existing global
  // capsule/list `viewMode` toggle as the default (the "coexist" model, D40).
  // grid → VirtualLibraryGrid, list → DetailListView, carousel → CarouselNav
  // (L4a), wheel → WheelNav radial (L4b). `custom` is theme-drawn (not rendered
  // in the shared browse view) so it falls back to grid for now.
  const declaredLayout = useDeclaredLayout("game-browse", selectedSystemId);
  const effectiveLayoutMode = (): "grid" | "list" | "carousel" | "wheel" => {
    const d = declaredLayout();
    if (d === "list") return "list";
    if (d === "carousel") return "carousel"; // L4a
    if (d === "wheel") return "wheel"; // L4b
    if (d) return "grid"; // custom not rendered here (L5/L6) → grid
    return props.appearance.viewMode() === "list" ? "list" : "grid";
  };

  const filtered = createMemo(() =>
    filterEntries(
      props.library.state.entries,
      props.currentView,
      props.searchQuery,
      viewSystemIds(),
      // VL Phase E Sub-phase 3 — search also matches the identity's
      // canonical title (e.g. "castlevania" hits a lone "Akumajou
      // Dracula (Japan)" dump once identified).
      props.library.groupsByVariantId(),
    ),
  );
  // Collapse same-variant-group entries down to their default variant
  // BEFORE sorting and bucketing so each multi-region game renders as
  // one tile. VL Phase E Sub-phase 3 — the collapse rewrites the tile
  // title to the identity's canonical title, and the sort runs AFTER
  // the collapse so tiles order by what they actually display (a
  // "Akumajou Dracula" dump whose canonical name is "Castlevania"
  // sorts under C, where its tile sits).
  //
  // Phase A1 Sub-phase 4 — disc-set collapse runs AFTER variant
  // collapse. Result: a multi-disc game like "Final Fantasy IX" with
  // four region variants × four discs renders as a single tile
  // (variant collapse picks one region's disc set; disc-set collapse
  // picks one disc of that set as the representative tile).
  const collapsed = createMemo(() => {
    const groups = props.library.groupsByVariantId();
    const variantCollapsed =
      groups.size === 0
        ? filtered()
        : collapseVariantGroups(
            filtered(),
            groups,
            new Map(props.library.state.entries.map((e) => [e.id, e])),
          );
    return collapseDiscSets(variantCollapsed);
  });
  const sorted = createMemo(() =>
    sortEntries(collapsed(), props.appearance.sortKey(), getYear),
  );
  const grouped = createMemo(() =>
    groupEntries(sorted(), props.appearance.groupBy(), systemDisplayName),
  );

  // ARC 2 L4a/L4b — flat-list browse primitives (`carousel` coverflow + `wheel`
  // radial) over the flat `sorted()` list. Shared controlled focus so the
  // right-pane detail + `onFocus` follow the focused card; cover art via
  // `useMedia` (identity key → per-file fallback).
  const [browseIdx, setBrowseIdx] = createSignal(0);
  createEffect(() => {
    const n = sorted().length;
    if (n > 0 && browseIdx() > n - 1) setBrowseIdx(n - 1);
  });
  createEffect(() => {
    const g = sorted()[browseIdx()];
    if (g) props.onFocus(g);
  });

  // ARC 2 L4b — the radial wheel sizes its ring to the browse pane (vertical
  // extent ≈ radius), so it fills the column at any window size.
  let paneRef: HTMLDivElement | undefined;
  const [paneHeight, setPaneHeight] = createSignal(720);
  onMount(() => {
    if (!paneRef) return;
    // Measure SYNCHRONOUSLY up front so the wheel paints at the correct radius
    // on its first frame. `onMount` runs before the browser paints, and
    // getBoundingClientRect forces layout now — so the radius is right before
    // anything is shown. (Relying only on the async ResizeObserver below made
    // the wheel paint at the default radius and then ANIMATE down to the real
    // one a beat later — the "wheel gets smaller / top pulls down" jump on first
    // scroll. The intermediate radius is now never painted, so no animation.)
    const initial = paneRef.getBoundingClientRect().height;
    if (initial > 0) setPaneHeight(initial);
    if (typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver((entries) => {
      const h = entries[0]?.contentRect.height;
      // Only react to real resizes (window/layout), not sub-pixel reflow churn.
      if (h && h > 0 && Math.abs(h - paneHeight()) > 2) setPaneHeight(h);
    });
    ro.observe(paneRef);
    onCleanup(() => ro.disconnect());
  });
  const wheelRadius = (): number => Math.max(240, paneHeight() * 0.52);
  const coverFor = (entry: RomEntry): string | null =>
    (entry.identityId ? media.coverUrl(entry.systemId, entry.identityId) : null) ??
    media.coverUrl(entry.systemId, entry.id);

  const title = (): string => {
    const sysId = selectedSystemId();
    if (sysId) return systemThemes[sysId]?.displayName ?? sysId;
    const cv = props.currentView;
    switch (cv.kind) {
      case "all": return "All Games";
      case "view-node": {
        // Container selection (PR-γ surface) — fall back to the node's
        // label by looking it up in the active view. If lookup fails,
        // surface a generic title rather than the raw node id.
        const active = props.views.activeView();
        if (active && active.id === cv.viewId) {
          const node = findNode(active, cv.nodeId);
          if (node && "label" in node) return node.label;
        }
        return "Library";
      }
    }
  };

  // Count = number of tiles rendered = collapsed list length (groups + singletons),
  // not raw file count. A library with 3 Castlevania variants + 1 Bonk = 2 tiles.
  const count = () => collapsed().length;
  const hasAny = () => count() > 0;

  // Phase A1 Sub-phase 4 — disc-set tile click intercept. The collapsed
  // representative entry carries discSetId; tiles with it open the
  // DiscPickerDialog instead of launching directly. The dialog fetches
  // members via list_disc_set_members and forwards the operator's pick
  // to the real `props.onLaunch`.
  const [discPickerEntry, setDiscPickerEntry] = createSignal<RomEntry | null>(null);
  const wrappedOnLaunch = (entry: RomEntry) => {
    if (entry.discSetId !== undefined) {
      setDiscPickerEntry(entry);
    } else {
      props.onLaunch(entry);
    }
  };

  return (
    <div class="flex h-full flex-col" data-system={selectedSystemId() ?? undefined}>
      <Show when={selectedSystemId()}>
        <SystemHeader
          systemId={selectedSystemId()!}
          gameCount={count()}
          onBootWithoutGame={props.onBootWithoutGame}
        />
      </Show>
      <GridControls
        title={title()}
        count={count()}
        tileSize={effectiveLayoutMode() === "grid" ? props.appearance.tileSize() : undefined}
        onTileSizeChange={props.appearance.setTileSize}
      />
      <div class="min-h-0 flex-1" ref={paneRef}>
        <Show
          when={hasAny()}
          fallback={
            <EmptyState
              hasQuery={props.searchQuery.length > 0}
              hasSeed={props.library.state.entries.some((e) => e.seed)}
              onPickFolder={props.onPickFolder}
              onImportWizard={props.onImportWizard}
            />
          }
        >
          <Switch
            fallback={
              <VirtualLibraryGrid
                groups={grouped()}
                tileWidth={props.appearance.tileSize()}
                onLaunch={wrappedOnLaunch}
                onShowSaves={props.onShowSaves}
                onPickContext={props.onPickContext}
                onFocus={props.onFocus}
                onShowInfo={props.onShowInfo}
                selectedId={props.selectedId}
                variantCountFor={(id) =>
                  props.library.groupsByVariantId().get(id)?.variants.length
                }
                onToggleFavorite={props.onToggleFavorite}
                showSystemHeader={props.showSystemHeader}
                focusGroupNeighbours={props.gridFocusNeighbours}
              />
            }
          >
            <Match when={effectiveLayoutMode() === "list"}>
              <DetailListView
                groups={grouped()}
                onLaunch={wrappedOnLaunch}
                onShowSaves={props.onShowSaves}
                onPickContext={props.onPickContext}
                onFocus={props.onFocus}
                onShowInfo={props.onShowInfo}
                selectedId={props.selectedId}
                variantCountFor={(id) =>
                  props.library.groupsByVariantId().get(id)?.variants.length
                }
                focusGroupNeighbours={props.gridFocusNeighbours}
              />
            </Match>
            <Match when={effectiveLayoutMode() === "carousel"}>
              {/* L4a — per-system `carousel` browse (CarouselNav coverflow over
                  the flat sorted list). Cards carry data-system so Retroverse's
                  per-system accent drives the focus ring. */}
              <CarouselNav
                id="library-carousel"
                class="h-full w-full"
                items={sorted}
                focusedIndex={browseIdx}
                setFocusedIndex={setBrowseIdx}
                cardWidth={210}
                pitch={168}
                neighbours={props.gridFocusNeighbours}
                hints={{ dpad: "Browse", stick: "Browse", Confirm: "Launch", Secondary: "Game info" }}
                onConfirm={(_i, g) => wrappedOnLaunch(g)}
                onSecondary={(_i, g) => props.onShowInfo?.(g)}
              >
                {(entry, ctx) => (
                  <div
                    class="relative aspect-[3/4] w-full overflow-hidden rounded-xl border bg-(--color-oa-bg-deep) shadow-2xl shadow-black/60"
                    data-system={entry.systemId}
                    classList={{
                      "border-(--color-system-accent) ring-2 ring-(--color-system-accent)/60": ctx.focused(),
                      "border-white/10": !ctx.focused(),
                    }}
                  >
                    <Show
                      when={coverFor(entry)}
                      fallback={
                        <div
                          class="absolute inset-0 flex items-end p-3"
                          style={{
                            background:
                              "radial-gradient(ellipse 130% 70% at 50% -10%, var(--color-system-glow), transparent 70%), var(--color-oa-bg-deep)",
                          }}
                        >
                          <span class="line-clamp-3 text-[0.7rem] font-medium leading-tight text-(--color-oa-ink-dim)">
                            {entry.title}
                          </span>
                        </div>
                      }
                    >
                      {(src) => (
                        <img
                          src={src()}
                          alt={entry.title}
                          class="absolute inset-0 h-full w-full object-contain"
                          loading="lazy"
                          decoding="async"
                        />
                      )}
                    </Show>
                  </div>
                )}
              </CarouselNav>
            </Match>
            <Match when={effectiveLayoutMode() === "wheel"}>
              {/* L4b — per-system `wheel` browse (WheelNav radial, shape A:
                  right-side vertical wheel). The focused cover juts left toward
                  the pane centre; neighbours fan up/down + curve away right,
                  leaving the left of the column free. Cards carry data-system so
                  Retroverse's per-system accent drives the focus ring. */}
              <WheelNav
                id="library-wheel"
                class="h-full w-full"
                items={sorted}
                focusedIndex={browseIdx}
                setFocusedIndex={setBrowseIdx}
                radius={wheelRadius()}
                neighbours={props.gridFocusNeighbours}
                hints={{ dpad: "Browse", stick: "Browse", Confirm: "Launch", Secondary: "Game info" }}
                onConfirm={(_i, g) => wrappedOnLaunch(g)}
                onSecondary={(_i, g) => props.onShowInfo?.(g)}
              >
                {(entry, ctx) => (
                  <div
                    class="relative aspect-[3/4] w-44 overflow-hidden rounded-xl border bg-(--color-oa-bg-deep) shadow-2xl shadow-black/60"
                    data-system={entry.systemId}
                    classList={{
                      "border-(--color-system-accent) ring-2 ring-(--color-system-accent)/60": ctx.focused(),
                      "border-white/10": !ctx.focused(),
                    }}
                  >
                    <Show
                      when={coverFor(entry)}
                      fallback={
                        <div
                          class="absolute inset-0 flex items-end p-3"
                          style={{
                            background:
                              "radial-gradient(ellipse 130% 70% at 50% -10%, var(--color-system-glow), transparent 70%), var(--color-oa-bg-deep)",
                          }}
                        >
                          <span class="line-clamp-3 text-[0.7rem] font-medium leading-tight text-(--color-oa-ink-dim)">
                            {entry.title}
                          </span>
                        </div>
                      }
                    >
                      {(src) => (
                        <img
                          src={src()}
                          alt={entry.title}
                          class="absolute inset-0 h-full w-full object-contain"
                          loading="lazy"
                          decoding="async"
                        />
                      )}
                    </Show>
                  </div>
                )}
              </WheelNav>
            </Match>
          </Switch>
        </Show>
      </div>
      <Show when={discPickerEntry()}>
        {(entry) => (
          <DiscPickerDialog
            entry={entry()}
            onLaunch={props.onLaunch}
            onClose={() => setDiscPickerEntry(null)}
          />
        )}
      </Show>
    </div>
  );
};

const EmptyState: Component<{
  hasQuery: boolean;
  hasSeed: boolean;
  onPickFolder: () => void;
  /// Phase 1B Slice 6 — primary path on first-launch hero. Opens the
  /// guided-setup wizard. Required so any future caller wiring a
  /// LibraryView gets a compile error if they forget to pass it
  /// (legacy ad-hoc folder-picker stays as the muted secondary).
  onImportWizard: () => void;
}> = (props) => {
  return (
    <div class="grid h-full place-items-center px-8 py-16">
      <div class="w-full max-w-lg text-center">
        <Show
          when={!props.hasQuery}
          fallback={
            <>
              <p class="text-2xl">🔍</p>
              <p class="mt-3 text-xs uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
                No matches
              </p>
              <p class="mt-2 text-sm text-(--color-oa-ink-dim)">
                Try a different search.
              </p>
            </>
          }
        >
          {/* Phase 1B Slice 6: first-launch hero. Single primary CTA
              routes to the guided-setup wizard; the legacy single-shot
              folder-picker is preserved as a muted secondary link so
              power users who want the quick path don't lose it. The
              hasSeed branch keeps the compact treatment for operators
              past first launch who happen to have placeholder-only
              libraries (rare; mostly post-seed cleanup state). */}
          <Show
            when={!props.hasSeed}
            fallback={
              <>
                <p class="text-4xl text-(--color-system-accent)">◐</p>
                <p class="mt-4 text-xs uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
                  Library is full of placeholders
                </p>
                <p class="mt-2 text-sm text-(--color-oa-ink)">
                  Clear the placeholder rows to start fresh, or import a folder of ROMs.
                </p>
                <button
                  type="button"
                  onClick={(e) => {
                    e.currentTarget.blur();
                    props.onPickFolder();
                  }}
                  class="mt-5 rounded-md bg-(--color-system-accent) px-4 py-2 text-xs font-semibold uppercase tracking-wider text-(--color-oa-bg-deep) transition hover:brightness-110"
                >
                  Import folder
                </button>
              </>
            }
          >
            <p class="text-4xl text-(--color-system-accent)">◐</p>
            <h2 class="mt-5 text-3xl font-semibold leading-tight tracking-tight text-(--color-oa-ink)">
              Welcome to Overlooked Arcade
            </h2>
            <p class="mt-3 text-sm leading-relaxed text-(--color-oa-ink-dim)">
              Drop in your ROMs and OA will get them ready. We'll detect systems, pick cores that match your hardware, and walk you through anything that needs your input.
            </p>
            <button
              type="button"
              onClick={(e) => {
                e.currentTarget.blur();
                props.onImportWizard();
              }}
              class="mt-6 rounded-md bg-(--color-system-accent) px-5 py-2.5 text-sm font-semibold uppercase tracking-wider text-(--color-oa-bg-deep) transition hover:brightness-110"
            >
              Set up your library
            </button>
            <p class="mt-4 text-[0.7rem] text-(--color-oa-ink-dim)">
              Or{" "}
              <button
                type="button"
                onClick={(e) => {
                  e.currentTarget.blur();
                  props.onPickFolder();
                }}
                class="underline decoration-dotted underline-offset-2 hover:text-(--color-oa-ink)"
              >
                pick a folder the quick way
              </button>
            </p>
          </Show>
        </Show>
      </div>
    </div>
  );
};

export default LibraryView;
