// LIBRARY tab — three-pane layout matching the operator-supplied
// library-default-mockup.png:
//   - Left:   LeftSidebar (system filters, reused from legacy).
//   - Center: LibraryView (filter / sort / group pipeline + grid /
//             detail list).
//   - Right:  GameDetailPanel when an entry is focused, "No selection"
//             placeholder otherwise.
//
// Three page-level focus groups (left / center / right) with DPad
// LEFT/RIGHT region transfer + UP/DOWN within. Embedded LeftSidebar +
// VirtualLibraryGrid focus groups stay dormant in Retroverse mode
// unless the operator mouse-clicks a tile (which auto-activates
// "library-grid" and restores 2D nav for the mouse flow).

import { createMemo, Show, type Component } from "solid-js";
import LeftSidebar from "../../layout/LeftSidebar";
import LibraryView from "../../components/LibraryView";
import GameDetailPanel from "./GameDetailPanel";
import { HintRegion } from "../../nav/HintBar";
import { activateFocusGroup, useDomQueryFocusGroup } from "../../nav/focus";
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

  // Retroverse-UI controller-nav v2 — per-region focus groups so DPad
  // LEFT/RIGHT transfers sidebar ↔ grid ↔ right detail pane. UP/DOWN
  // stays within a region. L1/R1 cycles Retroverse tabs at the shell
  // level — these groups don't wire shoulder neighbours so the shell
  // handler isn't double-fired by an in-page transfer.
  let leftRef: HTMLElement | undefined;
  let centerRef: HTMLElement | undefined;
  let rightRef: HTMLElement | undefined;
  const LEFT_ID = "retroverse-library-left";
  const CENTER_ID = "retroverse-library-center";
  const RIGHT_ID = "retroverse-library-right";
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
