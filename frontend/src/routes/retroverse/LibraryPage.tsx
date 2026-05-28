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
// Slice 7 adds the footer hint bar. Future polish: dedicated header
// card (ALL GAMES count + Sort/View/Filters controls per the mockup),
// system-label header on every tile, custom mini systems sidebar
// styled per the mockup.

import { Show, type Component } from "solid-js";
import LeftSidebar from "../../layout/LeftSidebar";
import LibraryView from "../../components/LibraryView";
import RightDetailPanel from "../../components/RightDetailPanel";
import { HintRegion } from "../../nav/HintBar";
import { useRetroverse } from "./context";

const LibraryPage: Component = () => {
  const ctx = useRetroverse();

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
      <aside class="min-w-0 overflow-hidden border-r border-white/5">
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
      <section class="min-h-0 min-w-0 overflow-hidden">
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
      <aside class="min-w-0 overflow-hidden border-l border-white/5">
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
