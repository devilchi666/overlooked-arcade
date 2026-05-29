// LIBRARY tab — three-pane layout matching the operator-supplied
// library-default-mockup.png:
//   - Left:   LeftSidebar (system filters, reused from legacy).
//             Wrapped by a page-level LEFT_ID `useDomQueryFocusGroup`
//             — the SAME pattern HOME / COLLECTIONS / PLAY NOW /
//             SETTINGS use for their sidebars. Stick walks systems;
//             DPad-RIGHT transfers to CENTER. LeftSidebar's internal
//             `"left-sidebar"` group stays registered for legacy-mode
//             use but doesn't compete in Retroverse because the
//             page-level LEFT_ID claims active first on mount.
//   - Center: LibraryView (filter / sort / group pipeline + grid /
//             detail list). Wrapped by a page-level CENTER group; a
//             delegating effect hands off to `"library-grid"` for
//             2D nav whenever it's registered (the ONE LIBRARY-
//             specific behaviour beyond the unified region model).
//   - Right:  GameDetailPanel when an entry is focused, "No selection"
//             placeholder otherwise.
//
// All three page-level groups use the same `useDomQueryFocusGroup`
// pattern + neighbours wiring as the other Retroverse pages. The
// only LIBRARY-specific behaviour is the delegating effect for grid
// 2D nav.

import { createEffect, createMemo, Show, type Component } from "solid-js";
import LeftSidebar from "../../layout/LeftSidebar";
import LibraryView from "../../components/LibraryView";
import GameDetailPanel from "./GameDetailPanel";
import { HintRegion } from "../../nav/HintBar";
import {
  activateFocusGroup,
  activeFocusGroupId,
  groupsVersion,
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

  // Page-level region focus groups — same pattern as HOME /
  // COLLECTIONS / PLAY NOW / SETTINGS. LEFT_ID wraps the
  // `<aside>` containing LeftSidebar and auto-claims on mount.
  // CENTER_ID and RIGHT_ID register with `autoActivate: false` so
  // they're DPad-transfer targets but don't compete for the initial
  // active slot.
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
    // Match only the LeftSidebar's primary navigation rows — twisty
    // toggle buttons and the collapse/expand button at the bottom
    // intentionally stay out of controller walk (the twisty is replaced
    // by DPad LEFT/RIGHT on container rows via onDirection below; the
    // collapse toggle is mouse-only utility chrome).
    selector: "[data-oa-sidebar-row]",
    orientation: "vertical",
    onActivate: (_i, el) => el.click(),
    onDirection: (direction, currentIndex, source) => {
      // Stick LEFT/RIGHT on a container row expands/collapses the
      // tree — matches the operator's mental model of "stick walks
      // and explores within the region." DPad LEFT/RIGHT is reserved
      // for region transfer and always falls through to the default
      // (DPad-RIGHT activates `neighbours.right` = CENTER; DPad-LEFT
      // has no leftward neighbour so it no-ops). Stick UP/DOWN walks
      // normally; leaf rows + the All Games button never consume.
      if (source !== "stick-left") return false;
      if (direction !== "left" && direction !== "right") return false;
      const root = leftRef;
      if (!root) return false;
      const items = Array.from(
        root.querySelectorAll<HTMLElement>("[data-oa-sidebar-row]"),
      ).filter((el) => !(el as Partial<HTMLButtonElement>).disabled);
      const focused = items[currentIndex];
      if (!focused) return false;
      if (focused.getAttribute("data-oa-tree-node-kind") !== "container") {
        return false;
      }
      const nodeId = focused.getAttribute("data-oa-tree-node-id");
      if (!nodeId) return false;
      const expanded = new Set(ctx.views.activeView()?.expandedNodes ?? []);
      const isExpanded = expanded.has(nodeId);
      if (direction === "left") {
        if (isExpanded) {
          ctx.views.toggleExpanded(nodeId);
          return true;
        }
        return false;
      }
      if (!isExpanded) {
        ctx.views.toggleExpanded(nodeId);
        return true;
      }
      return false;
    },
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
  // CENTER group becomes active AND the grid is registered, hand off
  // to "library-grid" so DPad UP/DOWN walks rows + LEFT/RIGHT walks
  // columns. The dependency on `groupsVersion` makes this re-fire
  // when the grid mounts LATER (empty library → imported games OR
  // list-view → capsule-view switch), so the operator gets 2D nav
  // automatically without leaving + re-entering the tab.
  //
  // When the grid isn't registered (detail-list view, empty library)
  // CENTER_ID stays active — operators can still walk header buttons
  // and GridControls via the page-level DOM-query group.
  createEffect(() => {
    // Read groupsVersion so this effect re-runs on every register /
    // unregister; the actual condition is the activeFocusGroupId match
    // + grid being in the manager's map (encapsulated by activate's
    // own `groups.has` gate).
    groupsVersion();
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
          dpad: "Switch region",
          stick: "Navigate",
          a: "Play",
          b: "Back",
          x: "Search",
          y: "Filters",
          l1: "Prev tab",
          r1: "Next tab",
        }}
      />
      {/* Left: system filter sidebar — wrapped in the page-level
          LEFT_ID DOM-query group, same as HOME / COLLECTIONS / PLAY
          NOW / SETTINGS sidebars. LeftSidebar's internal
          "left-sidebar" group stays for legacy-shell compatibility
          but doesn't claim active in Retroverse because the page-
          level LEFT_ID is registered first and claims on mount. */}
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
