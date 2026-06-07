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

import { createSignal, Match, onCleanup, onMount, Show, Switch, type Component } from "solid-js";
import {
  currentRoute,
  setCurrentRoute,
  cycleRouteForward,
  cycleRouteBackward,
  RETROVERSE_ROUTES,
  type RetroverseRoute,
} from "../../routing/currentRoute";
import { onNavEvent } from "../../nav/gamepad";
import { useTheme } from "../../routes/retroverse/context";
import CollectionsPage from "../../routes/retroverse/CollectionsPage";
import DiscoverPage from "../../routes/retroverse/DiscoverPage";
import HomePage from "../../routes/retroverse/HomePage";
import LibraryPage from "../../routes/retroverse/LibraryPage";
import PlayNowPage from "../../routes/retroverse/PlayNowPage";
import EngineSummonIcon from "../../engine/EngineSummonIcon";
import { engineSurfaceOpen, openEngineSurface } from "../../platform/engineSurface";

const ROUTE_LABELS: Record<RetroverseRoute, string> = {
  home: "HOME",
  library: "LIBRARY",
  collections: "COLLECTIONS",
  "play-now": "PLAY NOW",
  discover: "DISCOVER",
};

// Per-tab design doc references used to live here for the StubPage
// footer. All 6 tabs are real now — STUB_DESIGN_DOCS removed alongside
// the StubPage import. The per-tab plans still live at
// `docs/PLANS/*-tab-retroverse.md`.

const RetroverseShell: Component = () => {
  const ctx = useTheme();
  const isActive = (r: RetroverseRoute) => currentRoute() === r;

  // Phase B Slice 7 — shell-level L1/R1 = cycle between tabs. Only
  // mounted when the Retroverse flag is ON (App.tsx Show gate), so no
  // explicit flag check needed here — flag flip OFF unmounts this
  // component and cleans the listener up.
  //
  // Theming Substrate ARC 1 Phase 1: gate on engineSurfaceOpen() —
  // when the engine surface is open, the engine owns gamepad input;
  // RetroverseShell's tab-cycler should be inert so the operator's
  // L1/R1 inside Settings doesn't accidentally walk Retroverse tabs
  // underneath the takeover.
  onMount(() => {
    const dispose = onNavEvent((event) => {
      if (event.kind !== "button" || event.phase !== "down") return;
      if (engineSurfaceOpen()) return;
      if (event.button === "l1") cycleRouteBackward();
      else if (event.button === "r1") cycleRouteForward();
    });
    onCleanup(dispose);
  });

  // Clock + date display — tick every 30s so the minute readout stays
  // honest without burning CPU.
  const [now, setNow] = createSignal(new Date());
  onMount(() => {
    const id = window.setInterval(() => setNow(new Date()), 30_000);
    onCleanup(() => window.clearInterval(id));
  });
  const timeStr = () =>
    now().toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
  const dateStr = () =>
    now().toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });

  // Search input — wire to ThemeContext's searchQuery + jump to
  // LIBRARY on Enter so the operator can search from any tab.
  const onSearchKey = (e: KeyboardEvent) => {
    if (e.key === "Enter") {
      (e.currentTarget as HTMLInputElement).blur();
      setCurrentRoute("library");
    }
  };

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
            value={ctx.searchQuery()}
            onInput={(e) => ctx.setSearchQuery(e.currentTarget.value)}
            onKeyDown={onSearchKey}
            class="w-64 rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-xs text-(--color-oa-ink) placeholder:text-(--color-oa-ink-dim)/60 focus:border-(--color-system-accent) focus:outline-none"
            title="Type to filter — Enter jumps to LIBRARY with the active filter."
          />
          {/* Clock + date — ticks every 30s. */}
          <div class="flex flex-col items-end leading-tight">
            <span class="text-[0.8rem] font-semibold tabular-nums text-(--color-oa-ink)">
              {timeStr()}
            </span>
            <span class="text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              {dateStr()}
            </span>
          </div>
          {/* Game-focus indicator — visible only when keyboard
              passthrough is ON (Ctrl+G). Mirrors the legacy
              toolbarRight pill at App.tsx:1457-1464 so operators
              running a game with the core grabbing hotkeys see
              the same "Game focus" cue in Retroverse. */}
          <Show when={ctx.gameFocus()}>
            <span
              title="Game focus is ON — OA hotkeys pass through to the core. Press Ctrl+G to disable."
              class="shrink-0 rounded-md border border-(--color-system-accent)/40 bg-(--color-system-accent)/15 px-2 py-1 text-[0.6rem] font-semibold uppercase tracking-wider text-(--color-system-accent)"
            >
              Game focus
            </span>
          </Show>
          {/* Quit button — fires invoke("quit_app"). Same affordance
              as the legacy toolbarRight Quit at App.tsx:1505-1515.
              Ctrl+Q keyboard shortcut works in both modes via
              App.tsx's keydown handler. */}
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              ctx.onQuit();
            }}
            class="grid h-9 w-9 shrink-0 place-items-center rounded-md border border-white/10 bg-white/[0.04] text-sm text-(--color-oa-ink-dim) transition hover:border-(--color-oa-ink-dim)/50 hover:bg-white/[0.08] hover:text-(--color-oa-ink) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
            title="Quit (Ctrl+Q)"
            aria-label="Quit"
          >
            ✕
          </button>
          {/* Engine summon icon — opens the engine surface (fullscreen
              takeover hosting Settings + Library Manager + Import
              Wizard + BIOS + Core installer + System Health + Background
              Jobs). Theming Substrate D3 requires themes to reserve a
              top-right slot for this affordance; see
              docs/features/theming-substrate/SURFACES.md. F12 and the
              Select+Start chord (wired in App.tsx) also summon. */}
          <EngineSummonIcon />
          {/* Profile chip — avatar from Settings → Profile. Click
              opens the engine surface so the operator reaches the
              Profile category there (it used to route to the in-
              Retroverse SETTINGS tab, retired in Phase 1). */}
          <button
            type="button"
            class="grid h-9 w-9 shrink-0 place-items-center rounded-full border border-(--color-system-accent)/40 bg-(--color-system-accent)/15 text-base text-(--color-oa-ink) transition hover:border-(--color-system-accent) hover:bg-(--color-system-accent)/25 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
            title={ctx.settings.profileDisplayName() || "Profile"}
            aria-label="Profile"
            onClick={(e) => {
              e.currentTarget.blur();
              openEngineSurface();
            }}
          >
            {ctx.settings.profileAvatar() || "👤"}
          </button>
        </div>
      </header>

      {/* Active-tab body. Each page renders its own internal layout. */}
      <main style={{ "grid-area": "body" }} class="min-h-0 min-w-0 overflow-hidden">
        <Switch fallback={<LibraryPage />}>
          <Match when={currentRoute() === "home"}>
            <HomePage />
          </Match>
          <Match when={currentRoute() === "library"}>
            <LibraryPage />
          </Match>
          <Match when={currentRoute() === "collections"}>
            <CollectionsPage />
          </Match>
          <Match when={currentRoute() === "play-now"}>
            <PlayNowPage />
          </Match>
          <Match when={currentRoute() === "discover"}>
            <DiscoverPage />
          </Match>
        </Switch>
      </main>
    </div>
  );
};

export default RetroverseShell;
