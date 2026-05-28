// Retroverse-UI Phase C2 Slice 16 — HOME tab (code-first skeleton).
//
// Three-pane internal layout matching the operator-supplied
// default-theme-mockup.png:
//
//   - Left:   SYSTEMS sidebar — every installed system with game count.
//             Click to swap the active system the hero panel + popular
//             games rail display.
//   - Center: scrollable column with:
//             * Hero panel (system art + name + stats + progress).
//             * Popular games rail (per active system, sorted by play time).
//             * Quick Launch strip (Random / Favorites / Most Played /
//               Last Played / Multiplayer / Achievements).
//             * Recently Played rail (across all systems).
//             * System Status footer (CPU / RAM / Storage gauges).
//   - Right:  <RightDetailPanel> showing focusedEntry — kept for shell
//             consistency with LIBRARY / COLLECTIONS / PLAY NOW.
//
// Content gaps (per docs/PLANS/retroverse-ui-rollout.md HOME analysis):
// hero art comes from PlatformMedia.console + .fanart slots (procedural
// gradient fallback when missing); blurb / year / architecture / media
// chips are skipped for v1 since no per-system metadata schema exists
// yet. Operator can drop image files in <data_dir>/media/platform/<id>/
// or import an art pack today — HOME picks them up immediately.

import {
  createMemo,
  createResource,
  createSignal,
  For,
  onCleanup,
  onMount,
  Show,
  type Component,
} from "solid-js";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { useMedia } from "../../library/media";
import { usePlatformMedia } from "../../library/platformMedia";
import { systemThemes, type SystemId } from "../../themes/registry";
import type { RomEntry } from "../../library/types";
import GameDetailPanel from "./GameDetailPanel";
import SystemInfoPanel from "./SystemInfoPanel";
import { HintRegion } from "../../nav/HintBar";
import { activateFocusGroup, useDomQueryFocusGroup } from "../../nav/focus";
import { setCurrentRoute } from "../../routing/currentRoute";
import { useRetroverse } from "./context";

type SystemStatus = {
  cpuPercent: number;
  ramUsedBytes: number;
  ramTotalBytes: number;
  dataDirFreeBytes: number | null;
  dataDirTotalBytes: number | null;
};

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const kb = bytes / 1024;
  if (kb < 1024) return `${kb.toFixed(0)} KB`;
  const mb = kb / 1024;
  if (mb < 1024) return `${mb.toFixed(1)} MB`;
  const gb = mb / 1024;
  if (gb < 1024) return `${gb.toFixed(1)} GB`;
  return `${(gb / 1024).toFixed(2)} TB`;
}

function formatHours(secs: number): string {
  const hours = Math.floor(secs / 3600);
  const mins = Math.floor((secs % 3600) / 60);
  if (hours > 0) return `${hours}h ${mins}m`;
  return `${mins}m`;
}

const HomePage: Component = () => {
  const ctx = useRetroverse();
  const media = useMedia();
  const platformMedia = usePlatformMedia();

  // Retroverse-UI controller-nav v2 — per-region focus groups so
  // DPad / left-stick LEFT/RIGHT transfers between sidebar ↔ center ↔
  // right pane (operator spec). UP/DOWN stays within a region. L1/R1
  // cycles Retroverse tabs at the shell level — these groups don't
  // wire neighbours so shoulder bumpers don't double-fire here.
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

  // Per-system entry index — derived once from the LibraryStore. Used
  // by the sidebar count badges + the popular games rail.
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

  // Default active system: pick the system with the most games installed.
  // Falls back to the first SystemId in registry order when the library
  // is empty (fresh install).
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

  // Sidebar — list of every system with games. Sorted by count desc so
  // the operator's main system lands at the top.
  const systemsWithGames = createMemo(() => {
    const list = [...entriesBySystem().entries()].map(([sys, entries]) => ({
      systemId: sys,
      count: entries.length,
    }));
    return list.sort((a, b) => b.count - a.count);
  });

  // Active-system stats — count, total play time, completed count,
  // progress percent (completed / count).
  const activeSystemStats = createMemo(() => {
    const entries = entriesBySystem().get(activeSystemId()) ?? [];
    const totalPlaySecs = entries.reduce((acc, e) => acc + (e.playTimeSecs ?? 0), 0);
    const completed = entries.filter((e) => e.completed).length;
    const percent =
      entries.length > 0 ? Math.round((completed / entries.length) * 100) : 0;
    return {
      count: entries.length,
      completed,
      percent,
      totalPlaySecs,
    };
  });

  // Popular games for the active system: sort by play time desc; tie-
  // break favorites first; cap at 10.
  const popularGames = createMemo<RomEntry[]>(() => {
    const entries = entriesBySystem().get(activeSystemId()) ?? [];
    return [...entries]
      .sort((a, b) => {
        const pa = a.playTimeSecs ?? 0;
        const pb = b.playTimeSecs ?? 0;
        if (pa !== pb) return pb - pa;
        return Number(Boolean(b.favorite)) - Number(Boolean(a.favorite));
      })
      .slice(0, 10);
  });

  // Recently played — across all systems, sorted by lastPlayedAt desc.
  const recentlyPlayed = createMemo<RomEntry[]>(() =>
    ctx.library.state.entries
      .filter((e) => !e.seed && Boolean(e.lastPlayedAt))
      .sort((a, b) => (b.lastPlayedAt ?? 0) - (a.lastPlayedAt ?? 0))
      .slice(0, 10),
  );

  // System Status — polled every 3s. createResource gives us a refresh()
  // handle the interval timer calls.
  const [statusReadingTick, setStatusReadingTick] = createSignal(0);
  const [systemStatus] = createResource(statusReadingTick, async () => {
    try {
      return await invoke<SystemStatus>("get_system_status");
    } catch (e) {
      console.warn("[oa-home] get_system_status failed:", e);
      return null;
    }
  });

  onMount(() => {
    const handle = window.setInterval(() => {
      setStatusReadingTick((n) => n + 1);
    }, 3000);
    onCleanup(() => window.clearInterval(handle));
  });

  const heroArtSrc = () => {
    const sys = activeSystemId();
    return platformMedia.slotUrl(sys, "console") ?? platformMedia.slotUrl(sys, "wheel");
  };
  const heroBgSrc = () => {
    const sys = activeSystemId();
    return platformMedia.slotUrl(sys, "fanart") ?? platformMedia.slotUrl(sys, "background");
  };

  const launchRandomGame = () => {
    const all = ctx.library.state.entries.filter((e) => !e.seed);
    if (all.length === 0) return;
    const pick = all[Math.floor(Math.random() * all.length)];
    if (pick) void ctx.onLaunch(pick);
  };

  const QuickLaunchButton: Component<{
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
      class="flex min-w-[120px] flex-col items-center gap-1.5 rounded-lg border border-white/10 bg-white/[0.03] px-3 py-3 text-center transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
      classList={{
        "hover:border-(--color-system-accent) hover:bg-white/[0.06]": !qProps.disabled,
        "opacity-50 cursor-not-allowed": qProps.disabled === true,
      }}
    >
      <span class="text-2xl">{qProps.glyph}</span>
      <span class="text-[0.65rem] font-semibold uppercase tracking-widest text-(--color-oa-ink)">
        {qProps.label}
      </span>
    </button>
  );

  // Colored horizontal gauge — green/amber/red bar with label + percent
  // header + optional sublabel underneath. CPU + RAM use the high-is-bad
  // colorway; Storage flips it via `invert` so low free = red.
  const Gauge: Component<{
    label: string;
    percent: number;
    sublabel?: string;
    invert?: boolean;
  }> = (gProps) => {
    const bounded = () =>
      Math.min(100, Math.max(0, Number.isFinite(gProps.percent) ? gProps.percent : 0));
    const colorClass = () => {
      const p = bounded();
      const high = gProps.invert ? p < 15 : p > 85;
      const mid = gProps.invert ? p < 40 : p > 60;
      if (high) return "bg-red-500";
      if (mid) return "bg-amber-500";
      return "bg-emerald-500";
    };
    return (
      <div class="flex flex-col gap-1">
        <div class="flex items-baseline justify-between">
          <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            {gProps.label}
          </span>
          <span class="text-[0.7rem] font-semibold text-(--color-oa-ink)">
            {Math.round(bounded())}%
          </span>
        </div>
        <div class="h-1.5 w-full overflow-hidden rounded-full bg-white/5">
          <div
            class={`h-full rounded-full transition-[width] duration-500 ${colorClass()}`}
            style={{ width: `${bounded()}%` }}
          />
        </div>
        <Show when={gProps.sublabel}>
          <p class="text-[0.55rem] text-(--color-oa-ink-dim)/70">{gProps.sublabel}</p>
        </Show>
      </div>
    );
  };

  const RailCard: Component<{ entry: RomEntry; reasonChip?: string }> = (cardProps) => {
    const coverSrc = () => media.coverUrl(cardProps.entry.systemId, cardProps.entry.id);
    const systemLabel = () =>
      systemThemes[cardProps.entry.systemId]?.shortName ?? cardProps.entry.systemId;
    return (
      <button
        type="button"
        onClick={() => {
          ctx.setFocusedEntry(cardProps.entry);
          void ctx.onLaunch(cardProps.entry);
        }}
        class="group flex w-32 shrink-0 flex-col gap-1.5 text-left focus-visible:outline-none"
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
        <p class="truncate text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
          {systemLabel()}
        </p>
        <Show when={cardProps.reasonChip}>
          <p class="truncate text-[0.6rem] text-(--color-system-accent-soft)">
            {cardProps.reasonChip}
          </p>
        </Show>
      </button>
    );
  };

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
          a: "Play",
          b: "Back",
          x: "Search",
          y: "Favorite",
          l1: "Prev tab",
          r1: "Next tab",
        }}
      />

      {/* Left: SYSTEMS sidebar. */}
      <aside
        ref={(el) => (leftRef = el)}
        class="min-w-0 overflow-y-auto border-r border-white/5 px-3 py-4"
      >
        <p class="px-2 text-[0.55rem] font-semibold uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
          Systems
        </p>
        <p class="mt-1 px-2 text-[0.6rem] text-(--color-oa-ink-dim)/70">
          {ctx.library.state.entries.filter((e) => !e.seed).length} games ·{" "}
          {systemsWithGames().length} systems
        </p>
        <ul class="mt-3 flex flex-col gap-0.5">
          <For each={systemsWithGames()}>
            {({ systemId, count }) => {
              const isActive = () => activeSystemId() === systemId;
              const theme = systemThemes[systemId];
              return (
                <li>
                  <button
                    type="button"
                    onClick={(e) => {
                      e.currentTarget.blur();
                      setActiveSystemIdSig(systemId);
                    }}
                    class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-xs transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
                    classList={{
                      "bg-(--color-system-accent)/15 text-(--color-oa-ink)": isActive(),
                      "text-(--color-oa-ink-dim) hover:bg-white/[0.04] hover:text-(--color-oa-ink)":
                        !isActive(),
                    }}
                    aria-current={isActive() ? "page" : undefined}
                  >
                    <span class="truncate">{theme?.shortName ?? systemId}</span>
                    <span class="ml-auto text-[0.6rem] text-(--color-oa-ink-dim)">{count}</span>
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
      </aside>

      {/* Center: hero + popular + quick launch + recent + status. */}
      <section
        ref={(el) => (centerRef = el)}
        class="min-h-0 min-w-0 overflow-y-auto"
      >
        {/* Hero panel. */}
        <article class="relative overflow-hidden border-b border-white/5">
          <Show
            when={heroBgSrc()}
            fallback={
              <div
                class="absolute inset-0"
                style={{
                  background:
                    "radial-gradient(circle at 70% 30%, var(--color-system-glow), transparent 60%), linear-gradient(135deg, var(--color-system-accent) 0%, var(--color-oa-bg-deep) 100%)",
                }}
              />
            }
          >
            {(src) => (
              <img
                src={convertFileSrc(src())}
                alt=""
                class="absolute inset-0 h-full w-full object-cover opacity-50"
                draggable={false}
              />
            )}
          </Show>
          <div class="absolute inset-0 bg-gradient-to-t from-(--color-oa-bg-deep) via-transparent to-transparent" />

          <div class="relative grid min-h-[280px] grid-cols-[minmax(0,2fr)_minmax(0,3fr)] gap-6 px-8 py-8">
            <div class="flex items-center justify-center">
              <Show
                when={heroArtSrc()}
                fallback={
                  <div class="grid aspect-square w-full max-w-[240px] place-items-center rounded-xl border border-white/10 bg-black/40 text-5xl font-semibold uppercase tracking-widest text-(--color-system-accent-soft) backdrop-blur">
                    {activeTheme()?.shortName ?? activeSystemId()}
                  </div>
                }
              >
                {(src) => (
                  <img
                    src={convertFileSrc(src())}
                    alt={activeTheme()?.displayName ?? activeSystemId()}
                    class="max-h-[260px] w-auto object-contain drop-shadow-2xl"
                    draggable={false}
                  />
                )}
              </Show>
            </div>
            <div class="flex flex-col justify-center gap-3">
              <p class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-system-accent)">
                System spotlight
              </p>
              <h1 class="text-4xl font-semibold leading-tight text-(--color-oa-ink)">
                {activeTheme()?.displayName ?? activeSystemId()}
              </h1>
              <p class="max-w-xl text-sm leading-relaxed text-(--color-oa-ink-dim)">
                {/* Per-system blurb deferred — see HOME content-gap analysis */}
                Drop a custom blurb in a follow-up content slice. For now,
                this card highlights your library's footprint for the
                system + your progress against it.
              </p>

              <div class="mt-2 flex flex-wrap items-center gap-2 text-[0.7rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                <span class="rounded border border-white/10 bg-black/30 px-2 py-0.5 backdrop-blur">
                  {activeSystemStats().count}{" "}
                  {activeSystemStats().count === 1 ? "game" : "games"}
                </span>
                <Show when={activeSystemStats().totalPlaySecs > 0}>
                  <span class="rounded border border-white/10 bg-black/30 px-2 py-0.5 backdrop-blur">
                    {formatHours(activeSystemStats().totalPlaySecs)} played
                  </span>
                </Show>
                <Show when={activeSystemStats().completed > 0}>
                  <span class="rounded border border-white/10 bg-black/30 px-2 py-0.5 backdrop-blur">
                    {activeSystemStats().completed} completed
                  </span>
                </Show>
              </div>

              <Show when={activeSystemStats().count > 0}>
                <div class="mt-3 max-w-md">
                  <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                    Your progress · {activeSystemStats().percent}%
                  </p>
                  <div class="mt-1 h-1.5 w-full overflow-hidden rounded-full bg-white/5">
                    <div
                      class="h-full rounded-full bg-(--color-system-accent)"
                      style={{ width: `${activeSystemStats().percent}%` }}
                    />
                  </div>
                </div>
              </Show>
            </div>
          </div>
        </article>

        {/* Popular games rail (per active system). */}
        <section class="px-8 py-4">
          <header class="mb-2 flex items-center gap-2">
            <span class="text-[0.6rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
              Popular on {activeTheme()?.shortName ?? activeSystemId()}
            </span>
            <span class="text-[0.6rem] text-(--color-oa-ink-dim)/60">
              · {popularGames().length}
            </span>
          </header>
          <Show
            when={popularGames().length > 0}
            fallback={
              <p class="text-[0.7rem] text-(--color-oa-ink-dim)/70">
                No games installed for this system yet.
              </p>
            }
          >
            <div class="flex gap-3 overflow-x-auto pb-2">
              <For each={popularGames()}>
                {(entry) => (
                  <RailCard
                    entry={entry}
                    reasonChip={
                      entry.playTimeSecs && entry.playTimeSecs > 0
                        ? formatHours(entry.playTimeSecs)
                        : entry.favorite
                          ? "★"
                          : undefined
                    }
                  />
                )}
              </For>
            </div>
          </Show>
        </section>

        {/* Quick Launch strip. */}
        <section class="px-8 py-4">
          <p class="mb-2 text-[0.6rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
            Quick launch
          </p>
          <div class="flex flex-wrap gap-3">
            <QuickLaunchButton
              glyph="🎲"
              label="Random"
              onClick={launchRandomGame}
              disabled={ctx.library.state.entries.length === 0}
              disabledHint="Import games first."
            />
            <QuickLaunchButton
              glyph="❤"
              label="Favorites"
              onClick={() => setCurrentRoute("collections")}
            />
            <QuickLaunchButton
              glyph="📈"
              label="Most played"
              onClick={() => setCurrentRoute("collections")}
            />
            <QuickLaunchButton
              glyph="🕘"
              label="Last played"
              onClick={() => setCurrentRoute("collections")}
            />
            <QuickLaunchButton
              glyph="👥"
              label="Multi-player"
              onClick={() => setCurrentRoute("collections")}
            />
            <QuickLaunchButton
              glyph="🏆"
              label="Achievements"
              onClick={() => {
                /* RetroAchievements integration not in any phase yet. */
              }}
              disabled
              disabledHint="RetroAchievements integration ships separately."
            />
          </div>
        </section>

        {/* Recently played rail (across all systems). */}
        <section class="px-8 py-4">
          <header class="mb-2 flex items-center gap-2">
            <span class="text-[0.6rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
              Recently played
            </span>
            <span class="text-[0.6rem] text-(--color-oa-ink-dim)/60">
              · {recentlyPlayed().length}
            </span>
          </header>
          <Show
            when={recentlyPlayed().length > 0}
            fallback={
              <p class="text-[0.7rem] text-(--color-oa-ink-dim)/70">
                Play a game to populate this rail. Sessions are tracked from
                launch to exit.
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
                  const chip =
                    days === null
                      ? undefined
                      : days === 0
                        ? "Today"
                        : days === 1
                          ? "1d ago"
                          : `${days}d ago`;
                  return <RailCard entry={entry} reasonChip={chip} />;
                }}
              </For>
            </div>
          </Show>
        </section>

        {/* System Status moved to the bottom of the right pane below
            (operator spec, post-Phase-C2-validation). Center pane ends
            with Recently played. */}
      </section>

      {/* Right: focused-card detail on top, System Status gauges
          pinned to the bottom (operator spec). */}
      <aside
        ref={(el) => (rightRef = el)}
        class="flex h-full min-w-0 flex-col overflow-hidden border-l border-white/5"
      >
        <div class="min-h-0 flex-1 overflow-hidden">
          <Show
            when={ctx.focusedEntry()}
            fallback={
              <SystemInfoPanel
                systemId={activeSystemId()}
                entries={entriesBySystem().get(activeSystemId()) ?? []}
                onPickGame={(game) => {
                  ctx.setFocusedEntry(game);
                }}
              />
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
        </div>

        {/* System Status gauges — always visible at the bottom of the
            right pane. Colored bars: CPU/RAM green→amber→red, Storage
            free inverted (low free = red). */}
        <div class="shrink-0 border-t border-white/5 bg-(--color-oa-bg-deep)/60 px-5 py-4">
          <p class="mb-3 text-[0.55rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
            System status
          </p>
          <Show
            when={systemStatus()}
            fallback={
              <p class="text-[0.6rem] text-(--color-oa-ink-dim)/70">
                Sampling host load…
              </p>
            }
          >
            {(status) => {
              const ramPercent = () =>
                status().ramTotalBytes > 0
                  ? (status().ramUsedBytes / status().ramTotalBytes) * 100
                  : 0;
              const freePercent = () => {
                const free = status().dataDirFreeBytes;
                const total = status().dataDirTotalBytes;
                if (free === null || total === null || !total) return null;
                return (free / total) * 100;
              };
              return (
                <div class="flex flex-col gap-3">
                  <Gauge label="CPU" percent={status().cpuPercent} />
                  <Gauge
                    label="RAM"
                    percent={ramPercent()}
                    sublabel={`${formatBytes(status().ramUsedBytes)} / ${formatBytes(status().ramTotalBytes)}`}
                  />
                  <Show
                    when={freePercent() !== null}
                    fallback={
                      <Gauge label="Storage" percent={0} sublabel="—" invert />
                    }
                  >
                    <Gauge
                      label="Storage free"
                      percent={freePercent() ?? 0}
                      sublabel={`${formatBytes(status().dataDirFreeBytes ?? 0)} of ${formatBytes(status().dataDirTotalBytes ?? 0)}`}
                      invert
                    />
                  </Show>
                </div>
              );
            }}
          </Show>
        </div>
      </aside>
    </div>
  );
};

export default HomePage;
