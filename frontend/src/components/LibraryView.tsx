import { createMemo, Show, type Component } from "solid-js";
import type { LibraryStore } from "../library/store";
import type { RomEntry } from "../library/types";
import type { LayoutStore } from "../layout/state";
import type { SidebarView } from "../layout/LeftSidebar";
import { collapseVariantGroups, filterEntries, groupEntries, sortEntries } from "../library/filter";
import { useMedia } from "../library/media";
import { systemThemes, type SystemId } from "../themes/registry";
import type { ViewsStore } from "../views/store";
import { findNode, resolveNodeSystemIds } from "../views/resolver";
import { parsePlatformNodeId } from "../views/defaults";
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
  /// (currentView is `all`, library-manager, or cores). For view-node:
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
    filterEntries(props.library.state.entries, props.currentView, props.searchQuery, viewSystemIds()),
  );
  const sorted = createMemo(() =>
    sortEntries(filtered(), props.layout.sortKey(), getYear),
  );
  // Collapse same-variant-group entries down to their default variant
  // BEFORE bucketing so each multi-region game renders as one tile.
  // The store keeps `groupsByVariantId` empty for single-file games, so
  // unaffected libraries stay byte-identical to the pre-grouping
  // behaviour.
  const collapsed = createMemo(() => {
    const groups = props.library.groupsByVariantId();
    if (groups.size === 0) return sorted();
    const entryById = new Map(props.library.state.entries.map((e) => [e.id, e]));
    return collapseVariantGroups(sorted(), groups, entryById);
  });
  const grouped = createMemo(() =>
    groupEntries(collapsed(), props.layout.groupBy(), systemDisplayName),
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
      // LibraryView never mounts in settings / cores mode (App.tsx Switch
      // routes them to dedicated pages), but TypeScript wants the
      // discriminant exhaustive.
      case "library-manager": return "Library Manager";
      case "cores": return "Cores";
    }
  };

  // Count = number of tiles rendered = collapsed list length (groups + singletons),
  // not raw file count. A library with 3 Castlevania variants + 1 Bonk = 2 tiles.
  const count = () => collapsed().length;
  const hasAny = () => count() > 0;

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
            />
          }
        >
          <Show
            when={props.layout.viewMode() === "list"}
            fallback={
              <VirtualLibraryGrid
                groups={grouped()}
                tileWidth={props.layout.libraryTileSize()}
                onLaunch={props.onLaunch}
                onShowSaves={props.onShowSaves}
                onPickContext={props.onPickContext}
                onFocus={props.onFocus}
                onShowInfo={props.onShowInfo}
                selectedId={props.selectedId}
                variantCountFor={(id) =>
                  props.library.groupsByVariantId().get(id)?.variants.length
                }
              />
            }
          >
            <DetailListView
              groups={grouped()}
              onLaunch={props.onLaunch}
              onShowSaves={props.onShowSaves}
              onPickContext={props.onPickContext}
              onFocus={props.onFocus}
              selectedId={props.selectedId}
              variantCountFor={(id) =>
                props.library.groupsByVariantId().get(id)?.variants.length
              }
            />
          </Show>
        </Show>
      </div>
    </div>
  );
};

const EmptyState: Component<{
  hasQuery: boolean;
  hasSeed: boolean;
  onPickFolder: () => void;
}> = (props) => {
  return (
    <div class="grid h-full place-items-center px-8 py-16">
      <div class="w-full max-w-md text-center">
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
          <p class="text-4xl text-(--color-system-accent)">◐</p>
          <p class="mt-4 text-xs uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
            {props.hasSeed ? "Library is full of placeholders" : "Library is empty"}
          </p>
          <p class="mt-2 text-sm text-(--color-oa-ink)">
            Import a folder of ROMs to get started, or drop one onto this window.
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
        </Show>
      </div>
    </div>
  );
};

export default LibraryView;
