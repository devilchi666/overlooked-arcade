import { createMemo, createSignal, Show, type Component } from "solid-js";
import type { LibraryStore } from "@oa/platform/library/store";
import type { RomEntry } from "@oa/platform/library/types";
import type { LayoutStore } from "@oa/platform/layout/state";
import type { SidebarView } from "@oa/platform/layout/types";
import {
  collapseDiscSets,
  collapseVariantGroups,
  filterEntries,
  groupEntries,
  sortEntries,
  variantRibbonChips,
} from "@oa/platform/library/filter";
import type { LibraryMode } from "@oa/platform/settings/store";
import DiscPickerDialog from "./DiscPickerDialog";
import { useMedia } from "@oa/platform/library/media";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import type { ViewsStore } from "@oa/platform/views/store";
import { findNode, resolveNodeSystemIds } from "@oa/platform/views/resolver";
import { parsePlatformNodeId } from "@oa/platform/views/defaults";
import DetailListView from "./DetailListView";
import GridControls from "./GridControls";
import SystemHeader from "./SystemHeader";
import VirtualLibraryGrid from "./VirtualLibraryGrid";

type Props = {
  library: LibraryStore;
  layout: LayoutStore;
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
  /// VL Phase B — library presentation mode. `preservation` makes
  /// multi-variant tiles render a region/revision ribbon; `casual`
  /// (or omitted) keeps the clean one-tile-per-game look. Tiles still
  /// collapse to one-per-identity in both modes; the ribbon is the
  /// only visible difference (the Variants tab is the launch surface).
  libraryMode?: LibraryMode;
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
    sortEntries(collapsed(), props.layout.sortKey(), getYear),
  );
  const grouped = createMemo(() =>
    groupEntries(sorted(), props.layout.groupBy(), systemDisplayName),
  );

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
  // VL Phase B — Preservation-mode variant ribbon. Returns the chip
  // list for a tile's group only in preservation mode; casual mode (or
  // a single-variant group) returns undefined so the grid renders the
  // plain ▼N badge / nothing.
  const variantRibbonFor = (id: string): string[] | undefined => {
    if (props.libraryMode !== "preservation") return undefined;
    const group = props.library.groupsByVariantId().get(id);
    if (!group) return undefined;
    const chips = variantRibbonChips(group);
    return chips.length > 0 ? chips : undefined;
  };

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
        />
      </Show>
      <GridControls
        title={title()}
        count={count()}
        tileSize={props.layout.viewMode() === "capsule" ? props.layout.libraryTileSize() : undefined}
        onTileSizeChange={props.layout.setLibraryTileSize}
      />
      <div class="min-h-0 flex-1">
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
          <Show
            when={props.layout.viewMode() === "list"}
            fallback={
              <VirtualLibraryGrid
                groups={grouped()}
                tileWidth={props.layout.libraryTileSize()}
                onLaunch={wrappedOnLaunch}
                onShowSaves={props.onShowSaves}
                onPickContext={props.onPickContext}
                onFocus={props.onFocus}
                onShowInfo={props.onShowInfo}
                selectedId={props.selectedId}
                variantCountFor={(id) =>
                  props.library.groupsByVariantId().get(id)?.variants.length
                }
                variantRibbonFor={variantRibbonFor}
                onToggleFavorite={props.onToggleFavorite}
                showSystemHeader={props.showSystemHeader}
                focusGroupNeighbours={props.gridFocusNeighbours}
              />
            }
          >
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
              variantRibbonFor={variantRibbonFor}
              focusGroupNeighbours={props.gridFocusNeighbours}
            />
          </Show>
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
