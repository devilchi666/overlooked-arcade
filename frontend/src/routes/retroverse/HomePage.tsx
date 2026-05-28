// HOME v2 — per the operator-supplied HOME mockup.
//
// Three-pane layout:
//
//   - Left:   SYSTEMS list (icon + name + era subline + count) with a
//             "VIEW ALL SYSTEMS" link separator + a QUICK LAUNCH panel
//             pinned to the bottom (6 icon-tiles: Random / Recent /
//             Top Rated / Hidden Gems / Favorites / Play Now).
//   - Center: System spotlight hero (title + tagline + flag/release +
//             blurb + console image bleeding right + 6-card stats
//             grid: Games / Favorites / Played / Time Played / Last
//             Played / Completed). Below the hero: popular-cover
//             carousel. Below that: RECENTLY PLAYED panel with its
//             own carousel.
//   - Right:  <SystemInfoPanel> with 4 stacked cards (SYSTEM
//             INFORMATION / TECHNICAL DETAILS / SUPPORTED PERIPHERALS
//             / ACHIEVEMENTS). Stays on system info permanently —
//             clicking a popular or recently-played cover opens the
//             existing GameInfoModal rather than swapping the right
//             pane.
//
// Stub data for the dense system specs lives in systemMetadataStubs.ts;
// rows missing from a system's stub render as "—" (graceful
// degradation).

import {
  createMemo,
  createSignal,
  For,
  Show,
  type Component,
} from "solid-js";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useMedia } from "../../library/media";
import { usePlatformMedia } from "../../library/platformMedia";
import { systemThemes, type SystemId } from "../../themes/registry";
import type { RomEntry } from "../../library/types";
import { HintRegion } from "../../nav/HintBar";
import { activateFocusGroup, useDomQueryFocusGroup } from "../../nav/focus";
import { setCurrentRoute } from "../../routing/currentRoute";
import SystemInfoPanel from "./SystemInfoPanel";
import { getSystemSpecs } from "./systemMetadataStubs";
import { useRetroverse } from "./context";

function formatHours(secs: number): string {
  if (secs <= 0) return "—";
  const days = Math.floor(secs / 86400);
  const hours = Math.floor((secs % 86400) / 3600);
  const mins = Math.floor((secs % 3600) / 60);
  if (days > 0) return `${days}d ${hours}h ${mins}m`;
  if (hours > 0) return `${hours}h ${mins}m`;
  return `${mins}m`;
}

function formatDate(unixSecs?: number): string {
  if (!unixSecs) return "—";
  try {
    return new Date(unixSecs * 1000).toLocaleDateString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  } catch {
    return "—";
  }
}

const HomePage: Component = () => {
  const ctx = useRetroverse();
  const media = useMedia();
  const platformMedia = usePlatformMedia();

  // Per-region focus groups (operator-spec controller-nav). Sidebar
  // includes both Systems + Quick Launch since they're one column;
  // center walks hero / popular / recent in DOM order; right walks
  // the SystemInfoPanel cards.
  let leftRef: HTMLElement | undefined;
  let centerRef: HTMLElement | undefined;
  let rightRef: HTMLElement | undefined;
  const LEFT_ID = "retroverse-home-left";
  const CENTER_ID = "retroverse-home-center";
  const RIGHT_ID = "retroverse-home-right";
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

  const entriesBySystem = createMemo(() => {
    const grouped = new Map<SystemId, RomEntry[]>();
    for (const e of ctx.library.state.entries) {
      if (e.seed) continue;
      const sys = e.systemId as SystemId;
      const list = grouped.get(sys);
      if (list) list.push(e);
      else grouped.set(sys, [e]);
    }
    return grouped;
  });

  const defaultSystemId = createMemo<SystemId>(() => {
    let best: SystemId | undefined;
    let bestCount = -1;
    for (const [sys, entries] of entriesBySystem().entries()) {
      if (entries.length > bestCount) {
        best = sys;
        bestCount = entries.length;
      }
    }
    return best ?? (Object.keys(systemThemes)[0] as SystemId);
  });

  const [activeSystemIdSig, setActiveSystemIdSig] = createSignal<SystemId | null>(null);
  const activeSystemId = () => activeSystemIdSig() ?? defaultSystemId();
  const activeTheme = () => systemThemes[activeSystemId()];
  const activeSpecs = () => getSystemSpecs(activeSystemId());

  const systemsWithGames = createMemo(() => {
    const list = [...entriesBySystem().entries()].map(([sys, entries]) => ({
      systemId: sys,
      count: entries.length,
    }));
    return list.sort((a, b) => b.count - a.count);
  });

  const activeSystemStats = createMemo(() => {
    const entries = entriesBySystem().get(activeSystemId()) ?? [];
    const totalPlaySecs = entries.reduce((acc, e) => acc + (e.playTimeSecs ?? 0), 0);
    const completed = entries.filter((e) => e.completed).length;
    const favorites = entries.filter((e) => e.favorite).length;
    const played = entries.filter((e) => (e.playTimeSecs ?? 0) > 0).length;
    const lastPlayedAt = entries.reduce<number | undefined>((acc, e) => {
      const t = e.lastPlayedAt;
      if (!t) return acc;
      if (!acc || t > acc) return t;
      return acc;
    }, undefined);
    return {
      count: entries.length,
      favorites,
      played,
      totalPlaySecs,
      lastPlayedAt,
      completed,
    };
  });

  const popularGames = createMemo<RomEntry[]>(() => {
    const entries = entriesBySystem().get(activeSystemId()) ?? [];
    return [...entries]
      .sort((a, b) => {
        const pa = a.playTimeSecs ?? 0;
        const pb = b.playTimeSecs ?? 0;
        if (pa !== pb) return pb - pa;
        return Number(Boolean(b.favorite)) - Number(Boolean(a.favorite));
      })
      .slice(0, 14);
  });

  const recentlyPlayed = createMemo<RomEntry[]>(() =>
    ctx.library.state.entries
      .filter((e) => !e.seed && Boolean(e.lastPlayedAt))
      .sort((a, b) => (b.lastPlayedAt ?? 0) - (a.lastPlayedAt ?? 0))
      .slice(0, 12),
  );

  const heroConsoleSrc = () => {
    const sys = activeSystemId();
    return platformMedia.slotUrl(sys, "console") ?? platformMedia.slotUrl(sys, "wheel");
  };
  const heroFanartSrc = () => {
    const sys = activeSystemId();
    return platformMedia.slotUrl(sys, "fanart") ?? platformMedia.slotUrl(sys, "background");
  };

  const launchRandomGame = () => {
    const all = ctx.library.state.entries.filter((e) => !e.seed);
    if (all.length === 0) return;
    const pick = all[Math.floor(Math.random() * all.length)];
    if (pick) void ctx.onLaunch(pick);
  };

  // Center-pane cover card — reused for both the Popular carousel and
  // Recently Played carousel. Operator click opens GameInfoModal
  // (the existing legacy surface) instead of launching, per the
  // operator's "right pane stays on system info" spec.
  const CoverCard: Component<{
    entry: RomEntry;
    sublabel?: string;
  }> = (cardProps) => {
    const coverSrc = () => media.coverUrl(cardProps.entry.systemId, cardProps.entry.id);
    return (
      <button
        type="button"
        onClick={(e) => {
          e.currentTarget.blur();
          ctx.onShowInfo(cardProps.entry);
        }}
        class="group flex w-32 shrink-0 flex-col gap-1 text-left focus-visible:outline-none"
        title={cardProps.entry.title}
      >
        <div class="relative aspect-[3/4] overflow-hidden rounded border border-white/10 bg-(--color-oa-bg-deep) transition group-hover:-translate-y-0.5 group-hover:border-(--color-system-accent)">
          <Show
            when={coverSrc()}
            fallback={
              <div
                class="absolute inset-0"
                style={{
                  background:
                    "linear-gradient(135deg, var(--color-system-accent) 0%, var(--color-oa-bg-deep) 100%)",
                }}
              />
            }
          >
            {(src) => (
              <img
                src={convertFileSrc(src())}
                alt={cardProps.entry.title}
                class="absolute inset-0 h-full w-full object-cover"
                draggable={false}
              />
            )}
          </Show>
        </div>
        <p class="truncate text-[0.7rem] font-medium text-(--color-oa-ink)">
          {cardProps.entry.title}
        </p>
        <Show when={cardProps.sublabel}>
          <p class="truncate text-[0.55rem] text-(--color-oa-ink-dim)">
            {cardProps.sublabel}
          </p>
        </Show>
      </button>
    );
  };

  // Six stat cards in the hero grid.
  const StatCard: Component<{
    glyph: string;
    label: string;
    value: string;
  }> = (statProps) => (
    <div class="flex items-center gap-3 rounded-lg border border-white/10 bg-white/[0.03] px-4 py-3">
      <span class="text-xl text-(--color-system-accent)">{statProps.glyph}</span>
      <div class="min-w-0 flex-1">
        <p class="text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
          {statProps.label}
        </p>
        <p class="truncate text-lg font-semibold text-(--color-oa-ink)">
          {statProps.value}
        </p>
      </div>
    </div>
  );

  const QuickLaunchTile: Component<{
    glyph: string;
    label: string;
    onClick: () => void;
    disabled?: boolean;
    disabledHint?: string;
  }> = (qProps) => (
    <button
      type="button"
      onClick={(e) => {
        e.currentTarget.blur();
        if (!qProps.disabled) qProps.onClick();
      }}
      disabled={qProps.disabled}
      title={qProps.disabled ? qProps.disabledHint : undefined}
      class="flex flex-col items-center justify-center gap-1.5 rounded-lg border border-white/10 bg-white/[0.03] px-2 py-3 text-center transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
      classList={{
        "hover:border-(--color-system-accent) hover:bg-white/[0.06]": !qProps.disabled,
        "opacity-50 cursor-not-allowed": qProps.disabled === true,
      }}
    >
      <span class="text-xl">{qProps.glyph}</span>
      <span class="text-[0.55rem] font-semibold uppercase tracking-widest text-(--color-oa-ink)">
        {qProps.label}
      </span>
    </button>
  );

  return (
    <div
      class="grid h-full w-full"
      style={{
        "grid-template-columns": "260px minmax(0,1fr) 360px",
      }}
      data-system={activeSystemId()}
    >
      <HintRegion
        hints={{
          a: "Select",
          b: "Back",
          x: "Search",
          y: "Filters",
          start: "Options",
          l1: "Prev tab",
          r1: "Next tab",
        }}
      />

      {/* ===== LEFT: Systems list + Quick Launch panel ===== */}
      <aside
        ref={(el) => (leftRef = el)}
        class="flex min-w-0 flex-col overflow-hidden border-r border-white/5"
      >
        {/* Top: SYSTEMS list (scrollable). */}
        <div class="flex min-h-0 flex-1 flex-col overflow-y-auto px-3 py-4">
          <div class="flex items-center justify-between px-2">
            <p class="text-[0.55rem] font-semibold uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
              Systems
            </p>
            <span class="text-(--color-oa-ink-dim)">⌧</span>
          </div>
          <ul class="mt-3 flex flex-col gap-1">
            <For each={systemsWithGames()}>
              {({ systemId, count }) => {
                const isActive = () => activeSystemId() === systemId;
                const theme = systemThemes[systemId];
                const subline = () => getSystemSpecs(systemId).sidebarSubline;
                return (
                  <li>
                    <button
                      type="button"
                      onClick={(e) => {
                        e.currentTarget.blur();
                        setActiveSystemIdSig(systemId);
                      }}
                      class="flex w-full items-center gap-2 rounded-md border px-2 py-1.5 text-left transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
                      classList={{
                        "border-(--color-system-accent) bg-(--color-system-accent)/15": isActive(),
                        "border-transparent hover:border-white/10 hover:bg-white/[0.04]":
                          !isActive(),
                      }}
                      aria-current={isActive() ? "page" : undefined}
                    >
                      <span class="grid h-7 w-7 shrink-0 place-items-center rounded bg-white/5 text-[0.6rem] font-semibold uppercase tracking-widest text-(--color-system-accent-soft)">
                        {theme?.shortName?.slice(0, 4) ?? systemId.slice(0, 3)}
                      </span>
                      <div class="min-w-0 flex-1">
                        <p class="truncate text-[0.7rem] font-semibold text-(--color-oa-ink)">
                          {theme?.shortName ?? systemId}
                        </p>
                        <Show when={subline()}>
                          <p class="truncate text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                            {subline()}
                          </p>
                        </Show>
                      </div>
                      <span class="text-[0.6rem] text-(--color-oa-ink-dim)">
                        {count}
                      </span>
                    </button>
                  </li>
                );
              }}
            </For>
          </ul>
          <Show when={systemsWithGames().length === 0}>
            <p class="mt-4 px-2 text-[0.65rem] text-(--color-oa-ink-dim)/70">
              No games installed yet. Import some ROMs to populate HOME.
            </p>
          </Show>
        </div>

        {/* Divider + VIEW ALL SYSTEMS link. */}
        <button
          type="button"
          onClick={(e) => {
            e.currentTarget.blur();
            setCurrentRoute("library");
          }}
          class="flex items-center justify-between border-y border-white/5 px-5 py-3 text-[0.55rem] font-semibold uppercase tracking-[0.3em] text-(--color-oa-ink-dim) transition hover:bg-white/[0.04] hover:text-(--color-oa-ink)"
        >
          <span>View all systems</span>
          <span class="text-(--color-oa-ink-dim)">
            {systemsWithGames().length} systems
          </span>
        </button>

        {/* Bottom: Quick Launch panel. */}
        <section class="shrink-0 px-3 pb-4 pt-3">
          <p class="mb-2 px-2 text-[0.55rem] font-semibold uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
            Quick launch
          </p>
          <div class="grid grid-cols-3 gap-2">
            <QuickLaunchTile
              glyph="🎲"
              label="Random"
              onClick={launchRandomGame}
              disabled={ctx.library.state.entries.length === 0}
              disabledHint="Import games first."
            />
            <QuickLaunchTile
              glyph="🕘"
              label="Recent"
              onClick={() => setCurrentRoute("collections")}
            />
            <QuickLaunchTile
              glyph="⭐"
              label="Top rated"
              onClick={() => setCurrentRoute("collections")}
            />
            <QuickLaunchTile
              glyph="💎"
              label="Hidden gems"
              onClick={() => setCurrentRoute("collections")}
            />
            <QuickLaunchTile
              glyph="❤"
              label="Favorites"
              onClick={() => setCurrentRoute("collections")}
            />
            <QuickLaunchTile
              glyph="▶"
              label="Play now"
              onClick={() => setCurrentRoute("play-now")}
            />
          </div>
        </section>
      </aside>

      {/* ===== CENTER: System spotlight hero + carousels ===== */}
      <section
        ref={(el) => (centerRef = el)}
        class="min-h-0 min-w-0 overflow-y-auto"
      >
        {/* Hero — title, blurb, console image, stats grid, popular carousel. */}
        <article class="relative overflow-hidden border-b border-white/5">
          <Show when={heroFanartSrc()}>
            {(src) => (
              <img
                src={convertFileSrc(src())}
                alt=""
                class="absolute inset-0 h-full w-full object-cover opacity-20"
                draggable={false}
              />
            )}
          </Show>
          <div class="absolute inset-0 bg-gradient-to-t from-(--color-oa-bg-deep) via-(--color-oa-bg-deep)/60 to-transparent" />

          <div class="relative grid grid-cols-[minmax(0,3fr)_minmax(0,2fr)] gap-6 px-8 pt-8">
            {/* Hero top-left: title block. */}
            <div class="flex flex-col gap-3">
              <h1 class="text-4xl font-semibold leading-none tracking-tight text-(--color-oa-ink)">
                {(activeTheme()?.displayName ?? activeSystemId()).toUpperCase()}
              </h1>
              <Show when={activeSpecs().tagline}>
                <p class="text-[0.65rem] uppercase tracking-[0.3em] text-(--color-system-accent)">
                  {activeSpecs().tagline}
                </p>
              </Show>
              <Show when={activeSpecs().releaseDate}>
                <p class="text-xs text-(--color-oa-ink-dim)">
                  <span class="mr-1">{activeSpecs().releaseFlag ?? ""}</span>
                  Released: {activeSpecs().releaseDate}
                </p>
              </Show>
              <Show when={activeSpecs().blurb}>
                <p class="max-w-md text-[0.8rem] leading-relaxed text-(--color-oa-ink-dim)">
                  {activeSpecs().blurb}
                </p>
              </Show>
            </div>

            {/* Hero top-right: console image bleeding right. */}
            <div class="flex items-center justify-center">
              <Show
                when={heroConsoleSrc()}
                fallback={
                  <div class="grid aspect-square w-full max-w-[200px] place-items-center rounded-xl border border-white/10 bg-black/40 text-4xl font-semibold uppercase tracking-widest text-(--color-system-accent-soft) backdrop-blur">
                    {activeTheme()?.shortName ?? activeSystemId()}
                  </div>
                }
              >
                {(src) => (
                  <img
                    src={convertFileSrc(src())}
                    alt={activeTheme()?.displayName ?? activeSystemId()}
                    class="max-h-[220px] w-auto object-contain drop-shadow-2xl"
                    draggable={false}
                  />
                )}
              </Show>
            </div>
          </div>

          {/* Stats grid — 6 cards in a 3-col grid. */}
          <div class="relative grid grid-cols-3 gap-3 px-8 pt-6 pb-6">
            <StatCard glyph="🎮" label="Games" value={String(activeSystemStats().count)} />
            <StatCard glyph="❤" label="Favorites" value={String(activeSystemStats().favorites)} />
            <StatCard glyph="🎯" label="Played" value={String(activeSystemStats().played)} />
            <StatCard
              glyph="⏱"
              label="Time played"
              value={formatHours(activeSystemStats().totalPlaySecs)}
            />
            <StatCard
              glyph="🗓"
              label="Last played"
              value={formatDate(activeSystemStats().lastPlayedAt)}
            />
            <StatCard
              glyph="🏆"
              label="Completed"
              value={String(activeSystemStats().completed)}
            />
          </div>

          {/* Popular cover carousel — horizontal scroll. */}
          <Show when={popularGames().length > 0}>
            <div class="relative px-8 pb-6">
              <div class="flex gap-3 overflow-x-auto pb-2">
                <For each={popularGames()}>
                  {(entry) => {
                    const sub = () => {
                      const t = entry.playTimeSecs ?? 0;
                      if (t > 0) return formatHours(t);
                      return entry.favorite ? "★" : "";
                    };
                    return <CoverCard entry={entry} sublabel={sub()} />;
                  }}
                </For>
              </div>
            </div>
          </Show>
        </article>

        {/* Recently played panel — separate panel below the hero. */}
        <section class="px-8 py-6">
          <header class="mb-3 flex items-center justify-between">
            <p class="text-[0.6rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
              Recently played
            </p>
            <span class="text-[0.6rem] text-(--color-oa-ink-dim)/60">
              · {recentlyPlayed().length}
            </span>
          </header>
          <Show
            when={recentlyPlayed().length > 0}
            fallback={
              <p class="text-[0.7rem] text-(--color-oa-ink-dim)/70">
                Play a game to populate this rail. Sessions are tracked
                from launch to exit.
              </p>
            }
          >
            <div class="flex gap-3 overflow-x-auto pb-2">
              <For each={recentlyPlayed()}>
                {(entry) => {
                  const days = entry.lastPlayedAt
                    ? Math.floor(
                        (Math.floor(Date.now() / 1000) - entry.lastPlayedAt) /
                          (24 * 60 * 60),
                      )
                    : null;
                  const sub =
                    days === null
                      ? undefined
                      : days === 0
                        ? "Today"
                        : days === 1
                          ? "1d ago"
                          : `${days}d ago`;
                  return <CoverCard entry={entry} sublabel={sub} />;
                }}
              </For>
            </div>
          </Show>
        </section>
      </section>

      {/* ===== RIGHT: dense system info panel ===== */}
      <aside
        ref={(el) => (rightRef = el)}
        class="min-w-0 overflow-hidden border-l border-white/5"
      >
        <SystemInfoPanel systemId={activeSystemId()} />
      </aside>
    </div>
  );
};

export default HomePage;
