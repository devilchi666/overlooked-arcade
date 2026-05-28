// Retroverse-UI Phase B Slice 5 — top-level Retroverse shell.
//
// Active only when the operator flips Settings → Display → Experimental
// → Retroverse UI ON. Replaces the existing Shell wholesale in App.tsx's
// return via a <Show when={isRetroverseUiEnabled()} fallback={<Shell />}>
// — two distinct UIs, no hybrid state.
//
// Owns the top toolbar (logo + 6-tab strip + search input) and dispatches
// to the active-route page below it. Each page renders its own internal
// layout (LIBRARY uses a 3-pane shape; HOME / DISCOVER will use different
// shapes per docs/PLANS/<tab>-retroverse.md).
//
// Phase B Slice 5 ships the shell + tab routing with a real LibraryPage
// placeholder + StubPages for the other five tabs. Slice 6 fills in
// LIBRARY's real grid + sidebar + detail pane. Slice 7 adds the
// footer hint bar.

import { Match, onCleanup, onMount, Switch, type Component } from "solid-js";
import {
  currentRoute,
  setCurrentRoute,
  cycleRouteForward,
  cycleRouteBackward,
  RETROVERSE_ROUTES,
  type RetroverseRoute,
} from "../../routing/currentRoute";
import { onNavEvent } from "../../nav/gamepad";
import CollectionsPage from "../../routes/retroverse/CollectionsPage";
import LibraryPage from "../../routes/retroverse/LibraryPage";
import SettingsPage from "../../routes/retroverse/SettingsPage";
import StubPage from "../../routes/retroverse/StubPage";

const ROUTE_LABELS: Record<RetroverseRoute, string> = {
  home: "HOME",
  library: "LIBRARY",
  collections: "COLLECTIONS",
  "play-now": "PLAY NOW",
  discover: "DISCOVER",
  settings: "SETTINGS",
};

// Per-tab design doc references for the StubPage footer text. LIBRARY
// has no entry here because LibraryPage is the real surface (not a stub).
const STUB_DESIGN_DOCS: Record<Exclude<RetroverseRoute, "library">, string> = {
  home: "docs/features/per-system-ui/assets/default-theme-mockup.png",
  collections: "docs/PLANS/collections-tab-retroverse.md",
  "play-now": "docs/PLANS/play-now-tab-retroverse.md",
  discover: "docs/PLANS/discover-tab-retroverse.md",
  settings: "docs/PLANS/settings-tab-retroverse.md",
};

const RetroverseShell: Component = () => {
  const isActive = (r: RetroverseRoute) => currentRoute() === r;

  // Phase B Slice 7 — shell-level L1/R1 = cycle between tabs. Only
  // mounted when the Retroverse flag is ON (App.tsx Show gate), so no
  // explicit flag check needed here — flag flip OFF unmounts this
  // component and cleans the listener up.
  onMount(() => {
    const dispose = onNavEvent((event) => {
      if (event.kind !== "button" || event.phase !== "down") return;
      if (event.button === "l1") cycleRouteBackward();
      else if (event.button === "r1") cycleRouteForward();
    });
    onCleanup(dispose);
  });

  return (
    <div
      class="grid h-full w-full overflow-hidden bg-(--color-oa-bg-deep) text-(--color-oa-ink)"
      style={{
        "grid-template-rows": "64px minmax(0,1fr)",
        "grid-template-areas": `"top" "body"`,
      }}
    >
      {/* Top toolbar — logo + tab strip + search. Mirrors the
          operator-supplied library-default-mockup.png. */}
      <header
        style={{ "grid-area": "top" }}
        class="flex items-center gap-6 border-b border-white/5 bg-(--color-oa-bg-deep)/95 px-6 backdrop-blur"
      >
        <div class="flex shrink-0 items-center gap-2">
          <span class="text-lg text-(--color-system-accent)">◤</span>
          <div class="leading-tight">
            <p class="text-sm font-semibold uppercase tracking-[0.2em]">
              Overlooked Arcade
            </p>
            <p class="text-[0.55rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
              Retroverse · experimental
            </p>
          </div>
        </div>
        <nav class="flex flex-1 items-center justify-center gap-1" aria-label="Retroverse tabs">
          {RETROVERSE_ROUTES.map((r) => (
            <button
              type="button"
              onClick={(e) => {
                e.currentTarget.blur();
                setCurrentRoute(r);
              }}
              class="rounded-md px-4 py-2 text-xs font-semibold uppercase tracking-widest transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
              classList={{
                "bg-(--color-system-accent)/15 text-(--color-oa-ink)": isActive(r),
                "text-(--color-oa-ink-dim) hover:bg-white/[0.04] hover:text-(--color-oa-ink)": !isActive(r),
              }}
              aria-current={isActive(r) ? "page" : undefined}
            >
              {ROUTE_LABELS[r]}
            </button>
          ))}
        </nav>
        <div class="flex shrink-0 items-center gap-3">
          <input
            type="search"
            placeholder="Search games…"
            class="w-64 rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-xs text-(--color-oa-ink) placeholder:text-(--color-oa-ink-dim)/60 focus:border-(--color-system-accent) focus:outline-none"
            disabled
            title="Slice 6 will wire this to the existing searchQuery signal."
          />
        </div>
      </header>

      {/* Active-tab body. Each page renders its own internal layout. */}
      <main style={{ "grid-area": "body" }} class="min-h-0 min-w-0 overflow-hidden">
        <Switch fallback={<LibraryPage />}>
          <Match when={currentRoute() === "home"}>
            <StubPage title={ROUTE_LABELS.home} designDoc={STUB_DESIGN_DOCS.home} />
          </Match>
          <Match when={currentRoute() === "library"}>
            <LibraryPage />
          </Match>
          <Match when={currentRoute() === "collections"}>
            <CollectionsPage />
          </Match>
          <Match when={currentRoute() === "play-now"}>
            <StubPage title={ROUTE_LABELS["play-now"]} designDoc={STUB_DESIGN_DOCS["play-now"]} />
          </Match>
          <Match when={currentRoute() === "discover"}>
            <StubPage title={ROUTE_LABELS.discover} designDoc={STUB_DESIGN_DOCS.discover} />
          </Match>
          <Match when={currentRoute() === "settings"}>
            <SettingsPage />
          </Match>
        </Switch>
      </main>
    </div>
  );
};

export default RetroverseShell;
