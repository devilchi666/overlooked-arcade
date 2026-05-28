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

import { Show, type Component } from "solid-js";
import LeftSidebar from "../../layout/LeftSidebar";
import LibraryView from "../../components/LibraryView";
import GameDetailPanel from "./GameDetailPanel";
import { HintRegion } from "../../nav/HintBar";
import { activateFocusGroup, useDomQueryFocusGroup } from "../../nav/focus";
import { useRetroverse } from "./context";

const LibraryPage: Component = () => {
  const ctx = useRetroverse();

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

      {/* Center: existing LibraryView covers the entire filter + sort +
          group + grid + detail-list pipeline. Reusing it sidesteps
          re-implementing GridControls and the view-node resolver. */}
      <section
        ref={(el) => (centerRef = el)}
        class="min-h-0 min-w-0 overflow-hidden"
      >
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
        />
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
