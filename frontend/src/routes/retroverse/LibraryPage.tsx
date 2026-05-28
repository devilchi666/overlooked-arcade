// Retroverse-UI Phase B Slice 6 — real LIBRARY tab.
//
// Three-pane internal layout matching the operator-supplied
// library-default-mockup.png:
//   - Left:  system filter sidebar (existing LeftSidebar, reused as-is)
//   - Center: filtered grid via existing LibraryView (filter/sort/group
//            pipeline + GridControls + VirtualLibraryGrid)
//   - Right: <RightDetailPanel> showing focusedEntry() — always-visible,
//            no more modal in this code path. Slice 3's variant="panel"
//            is what makes this work.
//
// Reads everything from RetroverseContext (provided by App.tsx) so the
// existing library state signals + handlers are reused 1:1 — no
// duplication of filter pipeline, no separate launch flow.
//
// Controller-nav v2 (operator spec): 3 page-level Retroverse focus groups
// (-left / -center / -right) with DPad LEFT/RIGHT region transfer + UP/
// DOWN within-region. This mirrors HomePage / CollectionsPage /
// PlayNowPage / SettingsPage. The embedded LeftSidebar +
// VirtualLibraryGrid still register their own focus groups
// ("left-sidebar" / "library-grid") — those stay dormant in Retroverse
// mode unless the operator mouse-clicks into a tile (then
// VirtualLibraryGrid's onFocus auto-activates the grid group, restoring
// 2D nav). The tradeoff vs the legacy UI: gamepad DPad walks the grid
// linearly (DOM order) in Retroverse mode rather than 2D; consistent
// with the other Retroverse tabs.

import { Show, type Component } from "solid-js";
import LeftSidebar from "../../layout/LeftSidebar";
import LibraryView from "../../components/LibraryView";
import RightDetailPanel from "../../components/RightDetailPanel";
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
        "grid-template-columns": "240px minmax(0,1fr) 360px",
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

      {/* Right: persistent focused-game detail. Variant "panel" drops
          the backdrop / Close button / modal HintRegion so the panel
          sits flush against the tab content. Empty state when nothing
          is focused — RightDetailPanel renders nothing on a null
          entry, so we frame it with a placeholder card. */}
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
          <RightDetailPanel
            entry={ctx.focusedEntry()}
            onClose={() => ctx.setFocusedEntry(null)}
            onLaunched={(entry, slot) => ctx.onPostLaunch(entry, slot)}
          />
        </Show>
      </aside>
    </div>
  );
};

export default LibraryPage;
