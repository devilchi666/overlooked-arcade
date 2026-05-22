import { createMemo, Show, type Component } from "solid-js";
import type { LibraryStore } from "../library/store";
import type { RomEntry } from "../library/types";
import type { LayoutStore } from "../layout/state";
import type { SidebarView } from "../layout/LeftSidebar";
import { collapseVariantGroups, filterEntries, groupEntries, sortEntries } from "../library/filter";
import { useMedia } from "../library/media";
import { systemThemes, type SystemId } from "../themes/registry";
import DetailListView from "./DetailListView";
import GridControls from "./GridControls";
import SystemHeader from "./SystemHeader";
import VirtualLibraryGrid from "./VirtualLibraryGrid";

type Props = {
  library: LibraryStore;
  layout: LayoutStore;
  currentView: SidebarView;
  searchQuery: string;
  onLaunch: (entry: RomEntry) => void;
  onShowSaves: (entry: RomEntry) => void;
  onPickContext: (entry: RomEntry, position: { x: number; y: number }) => void;
  onFocus: (entry: RomEntry) => void;
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

  const filtered = createMemo(() =>
    filterEntries(props.library.state.entries, props.currentView, props.searchQuery),
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
    const cv = props.currentView;
    if (cv.kind === "system") {
      return systemThemes[cv.id]?.displayName ?? cv.id;
    }
    switch (cv.kind) {
      case "all": return "All Games";
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
    <div class="flex h-full flex-col" data-system={props.currentView.kind === "system" ? props.currentView.id : undefined}>
      <Show when={props.currentView.kind === "system"}>
        <SystemHeader
          systemId={(props.currentView as { kind: "system"; id: SystemId }).id}
          gameCount={count()}
        />
      </Show>
      <GridControls title={title()} count={count()} />
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
                onLaunch={props.onLaunch}
                onShowSaves={props.onShowSaves}
                onPickContext={props.onPickContext}
                onFocus={props.onFocus}
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
