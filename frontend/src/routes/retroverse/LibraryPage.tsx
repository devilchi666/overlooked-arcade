// LIBRARY tab — three-pane layout matching the operator-supplied
// library-default-mockup.png:
//   - Left:   LeftSidebar (system filters, reused from legacy).
//   - Center: LibraryView (filter / sort / group pipeline + grid /
//             detail list).
//   - Right:  GameDetailPanel when an entry is focused, "No selection"
//             placeholder otherwise.
//
// Unified-focus model: three page-level region groups declare
// `neighbours` for DPad edge-spillover. The grid's own "library-grid"
// focus group is delegated to from CENTER via a createEffect so that
// when the operator transfers into the center region, they land in
// the grid for 2D navigation (UP/DOWN walks rows, LEFT/RIGHT walks
// columns; at the absolute top-left tile DPad LEFT spills to the
// sidebar, at the absolute bottom-right tile DPad RIGHT spills to
// the right pane via the grid's overridden focusGroupNeighbours).

import { createEffect, createMemo, Show, type Component } from "solid-js";
import LeftSidebar from "../../layout/LeftSidebar";
import LibraryView from "../../components/LibraryView";
import GameDetailPanel from "./GameDetailPanel";
import { HintRegion } from "../../nav/HintBar";
import {
  activateFocusGroup,
  activeFocusGroupId,
  useDomQueryFocusGroup,
} from "../../nav/focus";
import { systemThemes, type SystemId } from "../../themes/registry";
import { findNode } from "../../views/resolver";
import { useRetroverse } from "./context";

const LibraryPage: Component = () => {
  const ctx = useRetroverse();

  // Header card title + count. When the operator filters by a system
  // via the sidebar (view-node selection), title becomes the system's
  // display name; otherwise "All games". Count is the launchable entry
  // total (seed tiles excluded) — the visible grid below shows a
  // search-filtered subset, but the header reads the system-level
  // total for orientation. Matches the operator-supplied
  // library-default-mockup.png header card.
  // Resolve the active view-node selection (if any) to a single
  // SystemId. Container nodes resolve to null — the header shows
  // "All games" then.
  const headerSystemId = createMemo<SystemId | null>(() => {
    const cv = ctx.currentView();
    if (cv.kind !== "view-node") return null;
    const active = ctx.views.activeView();
    if (!active || active.id !== cv.viewId) return null;
    const node = findNode(active, cv.nodeId);
    if (node && "kind" in node && node.kind === "platform") {
      return node.systemId as SystemId;
    }
    return null;
  });

  const headerTitle = createMemo(() => {
    const sys = headerSystemId();
    if (!sys) return "All games";
    return systemThemes[sys]?.displayName ?? sys;
  });

  const headerCount = createMemo(() => {
    const entries = ctx.library.state.entries.filter((e) => !e.seed);
    const sys = headerSystemId();
    if (!sys) return entries.length;
    return entries.filter((e) => e.systemId === sys).length;
  });

  // Per-region focus groups (unified-focus model). `neighbours` drives
  // DPad edge-spillover. CENTER_ID is the page-level fallback for
  // when the grid isn't mounted (empty library / detail-list view);
  // a delegating effect below hands focus off to the grid's own
  // "library-grid" group whenever CENTER becomes active and the grid
  // is registered, restoring full 2D nav.
  let leftRef: HTMLElement | undefined;
  let centerRef: HTMLElement | undefined;
  let rightRef: HTMLElement | undefined;
  const LEFT_ID = "retroverse-library-left";
  const CENTER_ID = "retroverse-library-center";
  const RIGHT_ID = "retroverse-library-right";
  const GRID_ID = "library-grid";
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

  // Delegating effect — restore 2D grid nav. Whenever the page-level
  // CENTER group becomes active, immediately hand off to the grid's
  // own "library-grid" focus group (orientation: "grid") so DPad
  // UP/DOWN walks rows + LEFT/RIGHT walks columns. The grid's
  // focusGroupNeighbours override (passed in below) re-points
  // LEFT/RIGHT spillover at LEFT_ID / RIGHT_ID so at-corner DPad
  // exits land on the Retroverse regions instead of the legacy
  // sidebars. `activateFocusGroup` is a no-op when "library-grid"
  // isn't registered (grid not mounted — empty library or list view),
  // so CENTER_ID stays active in that fallback case.
  createEffect(() => {
    if (activeFocusGroupId() === CENTER_ID) {
      activateFocusGroup(GRID_ID);
    }
  });

  return (
    <div
      class="grid h-full w-full"
      style={{
        "grid-template-columns": "260px minmax(0,1fr) 360px",
      }}
    >
      {/* Phase B Slice 7 — LIBRARY-tab hint bar. Shell-level L1/R1
          cycle between Retroverse tabs (RetroverseShell owns the
          wiring); page-level A/B/X/Y describe the LIBRARY surface
          actions per docs/PLANS/retroverse-ui-rollout.md. */}
      <HintRegion
        hints={{
          a: "Play",
          b: "Back",
          x: "Search",
          y: "Filters",
          l1: "Prev tab",
          r1: "Next tab",
        }}
      />
      {/* Left: system filter sidebar — reuses the existing LeftSidebar
          so tier folders / per-system filters / drag-reorder all work
          unchanged. Visual polish vs the mockup happens in a follow-up. */}
      <aside
        ref={(el) => (leftRef = el)}
        class="min-w-0 overflow-hidden border-r border-white/5"
      >
        <LeftSidebar
          layout={ctx.layout}
          library={ctx.library}
          views={ctx.views}
          currentView={ctx.currentView()}
          onNavigate={ctx.setCurrentView}
        />
      </aside>

      {/* Center: header card + existing LibraryView. The header card
          surfaces title + count + flavor badge per the mockup; the
          embedded LibraryView GridControls keeps sort/view/group
          controls available below. */}
      <section
        ref={(el) => (centerRef = el)}
        class="flex min-h-0 min-w-0 flex-col overflow-hidden"
      >
        <header class="flex shrink-0 items-end justify-between gap-3 border-b border-white/5 px-8 py-4">
          <div>
            <h1 class="text-2xl font-semibold uppercase tracking-wide text-(--color-oa-ink)">
              {headerTitle()}
            </h1>
            <p class="mt-0.5 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              {headerCount()}{" "}
              <span class="text-(--color-system-accent)">
                {headerCount() === 1 ? "game" : "games"}
              </span>
            </p>
          </div>
        </header>
        <div class="min-h-0 flex-1 overflow-hidden">
          <LibraryView
            library={ctx.library}
            layout={ctx.layout}
            views={ctx.views}
            currentView={ctx.currentView()}
            searchQuery={ctx.searchQuery()}
            onLaunch={(entry) => void ctx.onLaunch(entry)}
            onShowSaves={ctx.onShowSaves}
            onPickContext={ctx.onPickContext}
            onFocus={ctx.setFocusedEntry}
            onShowInfo={ctx.onShowInfo}
            selectedId={() => ctx.focusedEntry()?.id ?? null}
            onPickFolder={() => void ctx.onPickFolder()}
            onToggleFavorite={ctx.onToggleFavorite}
            showSystemHeader
            gridFocusNeighbours={{ left: LEFT_ID, right: RIGHT_ID }}
          />
        </div>
      </section>

      {/* Right: focused-game detail via GameDetailPanel. Empty-state
          placeholder card when nothing is focused. */}
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
          {(entry) => (
            <GameDetailPanel
              entry={entry()}
              onLaunch={(e) => void ctx.onLaunch(e)}
              onShowInfo={ctx.onShowInfo}
              onToggleFavorite={ctx.onToggleFavorite}
            />
          )}
        </Show>
      </aside>
    </div>
  );
};

export default LibraryPage;
