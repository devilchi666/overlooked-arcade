// Retroverse-UI Phase C4 Slice 14 — PLAY NOW tab.
//
// Three-pane internal layout matching docs/PLANS/play-now-tab-retroverse.md:
//   - Left:   mood sidebar (For you / Continue / With a friend / Nostalgia
//             wired; Quick / Marathon / Challenge as placeholders; DAILY
//             group with Surprise me + Daily roulette placeholder).
//   - Center: massive hero card (cover hero + WHY line + Play Now + Reroll)
//             + Netflix-style rails (Continue where you left off /
//             Multi-player tonight / Favorites you haven't touched).
//   - Right:  GameDetailPanel when an entry is focused — same shape as
//             LIBRARY's right pane.
//
// Simple-heuristic recommender v1: active mood narrows the candidate pool,
// then the hero pick scans the pool against a short priority list. WHY
// lines are template-based ("Picked up where you left off · last played
// N days ago"). Reroll picks a different entry from the same pool.
//
// Surprise me = launch a random entry immediately. Other moods marked
// placeholder until session-length / difficulty data exists.

import { createMemo, createSignal, For, Match, Show, Switch, type Component } from "solid-js";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useMedia } from "../../library/media";
import type { RomEntry } from "../../library/types";
import GameDetailPanel from "./GameDetailPanel";
import { HintRegion } from "../../nav/HintBar";
import { activateFocusGroup, useDomQueryFocusGroup } from "../../nav/focus";
import { systemThemes } from "../../themes/registry";
import { useRetroverse } from "./context";

type MoodId =
  | "for-you"
  | "continue"
  | "with-a-friend"
  | "nostalgia"
  | "quick"
  | "marathon"
  | "challenge"
  | "surprise-me"
  | "daily-roulette";

type MoodDef = {
  id: MoodId;
  label: string;
  glyph: string;
  /// Candidate-pool filter. `null` means placeholder (Quick / Marathon /
  /// Challenge / Daily roulette — needs data that doesn't exist yet).
  pool: ((entry: RomEntry) => boolean) | null;
  /// True for moods that LAUNCH immediately on selection instead of
  /// re-rendering (Surprise me).
  instantLaunch?: boolean;
};

const MOODS: readonly MoodDef[] = [
  {
    id: "for-you",
    label: "For you",
    glyph: "✨",
    pool: () => true,
  },
  {
    id: "continue",
    label: "Continue",
    glyph: "⏵",
    pool: (entry) => Boolean(entry.lastPlayedAt),
  },
  {
    id: "with-a-friend",
    label: "With a friend",
    glyph: "👥",
    pool: (entry) => (entry.players ?? 1) >= 2,
  },
  {
    id: "nostalgia",
    label: "Nostalgia",
    glyph: "★",
    pool: (entry) => Boolean(entry.favorite),
  },
  {
    id: "quick",
    label: "Quick",
    glyph: "⚡",
    pool: null,
  },
  {
    id: "marathon",
    label: "Marathon",
    glyph: "⌛",
    pool: null,
  },
  {
    id: "challenge",
    label: "Challenge",
    glyph: "⚔",
    pool: null,
  },
  {
    id: "surprise-me",
    label: "Surprise me",
    glyph: "⤴",
    pool: () => true,
    instantLaunch: true,
  },
  {
    id: "daily-roulette",
    label: "Daily roulette",
    glyph: "🎲",
    pool: null,
  },
];

type WhyLine = {
  text: string;
  /// True when this entry should be the hero pick. Higher-priority lines
  /// short-circuit the search.
  priority: number;
};

/// Pick the best WHY-line for an entry. Returns null when no template
/// matches (i.e. nothing notable to say); caller falls back to random.
function whyLineFor(entry: RomEntry): WhyLine | null {
  const nowSecs = Math.floor(Date.now() / 1000);
  const day = 24 * 60 * 60;

  // 1. Continue where you left off — within 14 days, with play time.
  if (entry.lastPlayedAt && nowSecs - entry.lastPlayedAt < 14 * day) {
    const ago = Math.max(0, Math.floor((nowSecs - entry.lastPlayedAt) / day));
    const hours = Math.floor((entry.playTimeSecs ?? 0) / 3600);
    const mins = Math.floor(((entry.playTimeSecs ?? 0) % 3600) / 60);
    const timePart =
      hours > 0
        ? `${hours}h ${mins}m in`
        : entry.playTimeSecs && entry.playTimeSecs > 0
          ? `${mins}m in`
          : "fresh save";
    const dayPart =
      ago === 0 ? "earlier today" : ago === 1 ? "yesterday" : `${ago} days ago`;
    return {
      text: `Picked up where you left off · ${timePart} · last played ${dayPart}`,
      priority: 100,
    };
  }

  // 2. Favorited but untouched — high "you'll love this again" signal.
  if (entry.favorite) {
    if (!entry.lastPlayedAt) {
      return {
        text: "From your favorites — you've never played this yet",
        priority: 80,
      };
    }
    const ago = Math.floor((nowSecs - entry.lastPlayedAt) / day);
    if (ago >= 30) {
      return {
        text: `From your favorites — you haven't touched this in ${ago} days`,
        priority: 70,
      };
    }
    return {
      text: "From your favorites",
      priority: 40,
    };
  }

  // 3. Multi-player + late evening — couch co-op hour.
  if ((entry.players ?? 1) >= 2) {
    return { text: `Multi-player · couch co-op tonight?`, priority: 30 };
  }

  // 4. Generic random.
  return null;
}

function pickRandom<T>(pool: readonly T[]): T | null {
  if (pool.length === 0) return null;
  return pool[Math.floor(Math.random() * pool.length)] ?? null;
}

const PlayNowPage: Component = () => {
  const ctx = useRetroverse();
  const media = useMedia();
  const [activeMoodId, setActiveMoodId] = createSignal<MoodId>("for-you");

  // Retroverse-UI controller-nav v2 — per-region focus groups so
  // DPad LEFT/RIGHT transfers mood sidebar ↔ hero+rails ↔ right pane.
  let leftRef: HTMLElement | undefined;
  let centerRef: HTMLElement | undefined;
  let rightRef: HTMLElement | undefined;
  const LEFT_ID = "retroverse-play-now-left";
  const CENTER_ID = "retroverse-play-now-center";
  const RIGHT_ID = "retroverse-play-now-right";
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
  // Reroll seed — bumping it forces the hero memo to re-pick a fresh entry.
  const [rerollSeed, setRerollSeed] = createSignal(0);

  const activeMood = () => MOODS.find((m) => m.id === activeMoodId()) ?? MOODS[0]!;

  const launchableEntries = createMemo<RomEntry[]>(() =>
    ctx.library.state.entries.filter((e) => !e.seed),
  );

  const candidatePool = createMemo<RomEntry[]>(() => {
    const mood = activeMood();
    if (!mood.pool) return [];
    return launchableEntries().filter(mood.pool);
  });

  /// Hero = highest-priority WHY-line winner in the pool, with a fallback
  /// to a random pick. rerollSeed is a dependency so "Show another" can
  /// shuffle the result.
  const heroEntry = createMemo<RomEntry | null>(() => {
    // Touch rerollSeed for reactivity.
    rerollSeed();
    const pool = candidatePool();
    if (pool.length === 0) return null;

    let bestEntry: RomEntry | null = null;
    let bestPriority = -1;
    for (const entry of pool) {
      const why = whyLineFor(entry);
      const p = why?.priority ?? 0;
      if (p > bestPriority) {
        bestEntry = entry;
        bestPriority = p;
      }
    }

    // If everything tied at zero priority, pick at random instead so
    // reroll actually changes the result.
    if (bestPriority <= 0) return pickRandom(pool);
    return bestEntry;
  });

  const heroWhy = createMemo(() => {
    const e = heroEntry();
    if (!e) return null;
    return whyLineFor(e)?.text ?? "Daily random pick";
  });

  const heroCoverSrc = () => {
    const e = heroEntry();
    if (!e) return undefined;
    return media.coverUrl(e.systemId, e.id);
  };

  const heroSystemName = () => {
    const e = heroEntry();
    if (!e) return "";
    return systemThemes[e.systemId]?.displayName ?? e.systemId;
  };

  // Rails — independent of the hero pick. Each rail uses its own filter
  // and shows up to N entries.
  const continueRail = createMemo<RomEntry[]>(() =>
    launchableEntries()
      .filter((e) => Boolean(e.lastPlayedAt))
      .sort((a, b) => (b.lastPlayedAt ?? 0) - (a.lastPlayedAt ?? 0))
      .slice(0, 8),
  );

  const multiplayerRail = createMemo<RomEntry[]>(() =>
    launchableEntries()
      .filter((e) => (e.players ?? 1) >= 2)
      .slice(0, 8),
  );

  const favoritesUntouchedRail = createMemo<RomEntry[]>(() => {
    const nowSecs = Math.floor(Date.now() / 1000);
    const day = 24 * 60 * 60;
    return launchableEntries()
      .filter((e) => {
        if (!e.favorite) return false;
        if (!e.lastPlayedAt) return true;
        return nowSecs - e.lastPlayedAt > 30 * day;
      })
      .slice(0, 8);
  });

  const onMoodClick = (mood: MoodDef) => {
    if (mood.instantLaunch) {
      const pick = pickRandom(launchableEntries());
      if (pick) void ctx.onLaunch(pick);
      return;
    }
    setActiveMoodId(mood.id);
    setRerollSeed((n) => n + 1);
  };

  const launchHero = () => {
    const e = heroEntry();
    if (e) void ctx.onLaunch(e);
  };

  const reroll = () => setRerollSeed((n) => n + 1);

  // Compact rail card — smaller than LibraryTile, with reason chip slot.
  // Doesn't reuse LibraryTile to avoid pulling in the full grid focus
  // group + sound dispatch for a static carousel. Kept inline to keep
  // the file self-contained for Slice 14.
  const RailCard: Component<{
    entry: RomEntry;
    reasonChip?: string;
  }> = (cardProps) => {
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

  const Rail: Component<{
    title: string;
    entries: readonly RomEntry[];
    reasonChipFor?: (entry: RomEntry) => string | undefined;
    emptyState?: string;
  }> = (railProps) => {
    return (
      <section class="px-8 py-4">
        <header class="mb-2 flex items-center gap-2">
          <span class="text-[0.6rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
            {railProps.title}
          </span>
          <span class="text-[0.6rem] text-(--color-oa-ink-dim)/60">
            · {railProps.entries.length}
          </span>
        </header>
        <Show
          when={railProps.entries.length > 0}
          fallback={
            <p class="text-[0.7rem] text-(--color-oa-ink-dim)/70">
              {railProps.emptyState ?? "Empty"}
            </p>
          }
        >
          <div class="flex gap-3 overflow-x-auto pb-2">
            <For each={railProps.entries as RomEntry[]}>
              {(entry) => (
                <RailCard
                  entry={entry}
                  reasonChip={railProps.reasonChipFor?.(entry)}
                />
              )}
            </For>
          </div>
        </Show>
      </section>
    );
  };

  return (
    <div
      class="grid h-full w-full"
      style={{
        "grid-template-columns": "260px minmax(0,1fr) 360px",
      }}
    >
      {/* Phase C4 hints — A Play / Y Reroll hero / X Surprise me. */}
      <HintRegion
        hints={{
          a: "Play",
          b: "Back",
          x: "Surprise me",
          y: "Reroll hero",
          l1: "Prev tab",
          r1: "Next tab",
        }}
      />

      {/* Left: mood sidebar. */}
      <aside
        ref={(el) => (leftRef = el)}
        class="min-w-0 overflow-y-auto border-r border-white/5 px-3 py-4"
      >
        <p class="px-2 text-[0.55rem] font-semibold uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
          Moods
        </p>
        <ul class="mt-2 flex flex-col gap-0.5">
          <For each={MOODS.filter((m) => m.id !== "surprise-me" && m.id !== "daily-roulette")}>
            {(mood) => {
              const isActive = () => activeMoodId() === mood.id;
              const isPlaceholder = () => mood.pool === null;
              return (
                <li>
                  <button
                    type="button"
                    onClick={(e) => {
                      e.currentTarget.blur();
                      onMoodClick(mood);
                    }}
                    class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-xs transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
                    classList={{
                      "bg-(--color-system-accent)/15 text-(--color-oa-ink)":
                        isActive() && !mood.instantLaunch,
                      "text-(--color-oa-ink-dim) hover:bg-white/[0.04] hover:text-(--color-oa-ink)":
                        !isActive() || mood.instantLaunch === true,
                      "opacity-60": isPlaceholder(),
                    }}
                    disabled={isPlaceholder()}
                    aria-current={isActive() ? "page" : undefined}
                  >
                    <span class="w-4 text-center text-sm">{mood.glyph}</span>
                    <span class="truncate">{mood.label}</span>
                  </button>
                </li>
              );
            }}
          </For>
        </ul>
        <section class="mt-6 border-t border-white/5 pt-4">
          <p class="px-2 text-[0.55rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)/70">
            Daily
          </p>
          <ul class="mt-1.5 flex flex-col gap-0.5">
            <For each={MOODS.filter((m) => m.id === "surprise-me" || m.id === "daily-roulette")}>
              {(mood) => {
                const isPlaceholder = () => mood.pool === null;
                return (
                  <li>
                    <button
                      type="button"
                      onClick={(e) => {
                        e.currentTarget.blur();
                        onMoodClick(mood);
                      }}
                      class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-xs transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
                      classList={{
                        "text-(--color-oa-ink-dim) hover:bg-white/[0.04] hover:text-(--color-oa-ink)":
                          !isPlaceholder(),
                        "opacity-60": isPlaceholder(),
                      }}
                      disabled={isPlaceholder()}
                    >
                      <span class="w-4 text-center text-sm">{mood.glyph}</span>
                      <span class="truncate">{mood.label}</span>
                    </button>
                  </li>
                );
              }}
            </For>
          </ul>
        </section>
        <p class="mt-6 px-2 text-[0.6rem] leading-relaxed text-(--color-oa-ink-dim)/60">
          Recommendations adapt to time of day + your recent play. Quick /
          Marathon / Challenge ship once session-length data is tracked.
        </p>
      </aside>

      {/* Center: hero card + rails. */}
      <section
        ref={(el) => (centerRef = el)}
        class="flex min-h-0 min-w-0 flex-col overflow-y-auto"
      >
        <Show
          when={heroEntry()}
          fallback={
            <div class="flex h-full items-center justify-center p-12">
              <div class="max-w-md rounded-xl border border-dashed border-white/10 bg-white/[0.02] p-8 text-center">
                <p class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
                  Empty
                </p>
                <p class="mt-3 text-sm text-(--color-oa-ink-dim)">
                  <Switch fallback="No games match this mood yet.">
                    <Match when={activeMood().pool === null}>
                      This mood ships in a follow-up — needs data that
                      doesn't exist yet (session-length / difficulty).
                    </Match>
                    <Match when={launchableEntries().length === 0}>
                      Your library is empty. Import some games first.
                    </Match>
                  </Switch>
                </p>
              </div>
            </div>
          }
        >
          {(entry) => (
            <article class="relative px-8 py-6">
              <div class="overflow-hidden rounded-xl border border-white/10 bg-(--color-oa-bg-deep)/80">
                <div class="grid grid-cols-[minmax(0,2fr)_minmax(0,3fr)] gap-0">
                  <div class="relative aspect-[3/4] min-h-[280px] max-h-[420px] bg-(--color-oa-bg-deep)">
                    <Show
                      when={heroCoverSrc()}
                      fallback={
                        <div
                          class="absolute inset-0"
                          style={{
                            background:
                              "radial-gradient(circle at 30% 25%, var(--color-system-glow), transparent 60%), linear-gradient(135deg, var(--color-system-accent) 0%, var(--color-oa-bg-deep) 100%)",
                          }}
                        />
                      }
                    >
                      {(src) => (
                        <img
                          src={convertFileSrc(src())}
                          alt={entry().title}
                          class="absolute inset-0 h-full w-full object-cover"
                          draggable={false}
                        />
                      )}
                    </Show>
                  </div>
                  <div
                    class="flex flex-col justify-between gap-6 p-6"
                    data-system={entry().systemId}
                  >
                    <div>
                      <p class="text-[0.6rem] uppercase tracking-[0.4em] text-(--color-system-accent)">
                        Feature pick · {heroSystemName()}
                      </p>
                      <h1 class="mt-2 text-3xl font-semibold leading-tight text-(--color-oa-ink)">
                        {entry().title}
                      </h1>
                      <p class="mt-4 text-sm leading-relaxed text-(--color-oa-ink-dim)">
                        {heroWhy()}
                      </p>
                    </div>
                    <div class="flex flex-wrap gap-2">
                      <button
                        type="button"
                        onClick={(e) => {
                          e.currentTarget.blur();
                          launchHero();
                        }}
                        class="rounded-md border border-(--color-system-accent) bg-(--color-system-accent)/15 px-5 py-2 text-xs font-semibold uppercase tracking-wider text-(--color-oa-ink) transition hover:bg-(--color-system-accent)/25 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
                      >
                        ▶ Play now
                      </button>
                      <button
                        type="button"
                        onClick={(e) => {
                          e.currentTarget.blur();
                          reroll();
                        }}
                        class="rounded-md border border-white/10 bg-white/[0.04] px-4 py-2 text-xs uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
                      >
                        ↻ Show another
                      </button>
                      <button
                        type="button"
                        onClick={(e) => {
                          e.currentTarget.blur();
                          ctx.setFocusedEntry(entry());
                        }}
                        class="rounded-md border border-white/10 bg-white/[0.04] px-4 py-2 text-xs uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
                      >
                        Show details
                      </button>
                    </div>
                  </div>
                </div>
              </div>
            </article>
          )}
        </Show>

        {/* Rails — independent of mood pool; surface different angles. */}
        <Rail
          title="Continue where you left off"
          entries={continueRail()}
          reasonChipFor={(e) => {
            if (!e.lastPlayedAt) return undefined;
            const days = Math.floor(
              (Math.floor(Date.now() / 1000) - e.lastPlayedAt) / (24 * 60 * 60),
            );
            return days === 0 ? "Today" : days === 1 ? "1d ago" : `${days}d ago`;
          }}
          emptyState="Play a game to populate this rail."
        />
        <Rail
          title="Multi-player tonight"
          entries={multiplayerRail()}
          reasonChipFor={(e) =>
            e.players ? `${e.players}P` : undefined
          }
          emptyState="No multi-player games in your library yet."
        />
        <Rail
          title="From your favorites you haven't touched"
          entries={favoritesUntouchedRail()}
          reasonChipFor={() => "★"}
          emptyState="Heart a few games — the ones you haven't played in a while will surface here."
        />
      </section>

      {/* Right: focused-game detail. */}
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
                  Pick a card from the hero or a rail to see its detail
                  here.
                </p>
              </div>
            </div>
          }
        >
          <GameDetailPanel
            entry={ctx.focusedEntry()!}
            onLaunch={(e) => void ctx.onLaunch(e)}
            onShowInfo={ctx.onShowInfo}
            onToggleFavorite={ctx.onToggleFavorite}
          />
        </Show>
      </aside>
    </div>
  );
};

export default PlayNowPage;
