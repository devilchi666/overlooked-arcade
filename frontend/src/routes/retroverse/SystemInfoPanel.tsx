// Retroverse-UI right-pane redesign — HOME default state.
//
// Shows the active-system overview when no game is focused. Replaces
// the "No selection · Click a card on the popular or recently-played
// rails to see its detail here" placeholder with something the
// operator actually wants to read on HOME — system identity + library
// stats + top-3 popular games.
//
// HomePage renders this when `focusedEntry()` is null; flips to
// GameDetailPanel as soon as the operator picks a card.
//
// Data sources:
//   - systemThemes — displayName / shortName.
//   - usePlatformMedia — console / fanart / wheel slot URLs.
//   - LibraryStore — entries filtered by systemId for the stats.
//
// Blurb is intentionally a placeholder until the per-system blurb
// schema lands (HOME content-gap follow-up).

import { For, Show, type Component } from "solid-js";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useMedia } from "../../library/media";
import { usePlatformMedia } from "../../library/platformMedia";
import type { RomEntry } from "../../library/types";
import { systemThemes, type SystemId } from "../../themes/registry";

type Props = {
  systemId: SystemId;
  /// Pre-filtered entries for the active system. Caller does the
  /// filter so we don't repeat the entriesBySystem memo here.
  entries: readonly RomEntry[];
  /// Click handler when the operator picks a popular-games tile.
  onPickGame: (entry: RomEntry) => void;
};

function formatHours(secs: number): string {
  const hours = Math.floor(secs / 3600);
  const mins = Math.floor((secs % 3600) / 60);
  if (hours > 0) return `${hours}h ${mins}m`;
  return `${mins}m`;
}

const SystemInfoPanel: Component<Props> = (props) => {
  const platformMedia = usePlatformMedia();
  const media = useMedia();

  const theme = () => systemThemes[props.systemId];
  const consoleSrc = () =>
    platformMedia.slotUrl(props.systemId, "console") ??
    platformMedia.slotUrl(props.systemId, "wheel");
  const fanartSrc = () =>
    platformMedia.slotUrl(props.systemId, "fanart") ??
    platformMedia.slotUrl(props.systemId, "background");

  const stats = () => {
    const e = props.entries;
    const totalPlaySecs = e.reduce((acc, x) => acc + (x.playTimeSecs ?? 0), 0);
    const completed = e.filter((x) => x.completed).length;
    const favorites = e.filter((x) => x.favorite).length;
    const percent = e.length > 0 ? Math.round((completed / e.length) * 100) : 0;
    return {
      count: e.length,
      completed,
      favorites,
      percent,
      totalPlaySecs,
    };
  };

  // Top 3 most-played games for this system. Tie-break favorites first.
  const topGames = () =>
    [...props.entries]
      .sort((a, b) => {
        const pa = a.playTimeSecs ?? 0;
        const pb = b.playTimeSecs ?? 0;
        if (pa !== pb) return pb - pa;
        return Number(Boolean(b.favorite)) - Number(Boolean(a.favorite));
      })
      .slice(0, 3);

  return (
    <div
      class="flex h-full flex-col overflow-y-auto bg-(--color-oa-bg-deep)"
      data-system={props.systemId}
    >
      {/* Hero header — fanart background, console image overlay,
          gradient masking. */}
      <div class="relative aspect-[3/4] max-h-[260px] w-full overflow-hidden bg-black/30">
        <Show
          when={fanartSrc()}
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
              class="absolute inset-0 h-full w-full object-cover opacity-60"
              draggable={false}
            />
          )}
        </Show>
        <div class="absolute inset-0 bg-gradient-to-t from-(--color-oa-bg-deep) via-transparent to-transparent" />

        {/* Console centerpiece — overlays the fanart. */}
        <div class="absolute inset-0 grid place-items-center px-6">
          <Show
            when={consoleSrc()}
            fallback={
              <div class="grid aspect-square w-32 place-items-center rounded-xl border border-white/10 bg-black/40 text-2xl font-semibold uppercase tracking-widest text-(--color-system-accent-soft) backdrop-blur">
                {theme()?.shortName ?? props.systemId}
              </div>
            }
          >
            {(src) => (
              <img
                src={convertFileSrc(src())}
                alt={theme()?.displayName ?? props.systemId}
                class="max-h-[200px] w-auto object-contain drop-shadow-2xl"
                draggable={false}
              />
            )}
          </Show>
        </div>
      </div>

      {/* Name + blurb. */}
      <div class="flex flex-col gap-2 px-5 pt-4">
        <p class="text-[0.6rem] uppercase tracking-[0.4em] text-(--color-system-accent)">
          System spotlight
        </p>
        <h2 class="text-xl font-semibold leading-tight text-(--color-oa-ink)">
          {theme()?.displayName ?? props.systemId}
        </h2>
        <p class="text-[0.75rem] leading-relaxed text-(--color-oa-ink-dim)">
          {/* Per-system blurb deferred — HOME content-gap follow-up. */}
          Drop a custom blurb in a follow-up content slice. This panel
          shows your library footprint for the system + a quick way
          back into your most-played titles.
        </p>
      </div>

      {/* Stats grid — count / play time / completed / favorites. */}
      <div class="grid grid-cols-2 gap-2 px-5 pt-4">
        <div class="rounded-lg border border-white/10 bg-white/[0.03] px-3 py-2">
          <p class="text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            Games
          </p>
          <p class="mt-0.5 text-lg font-semibold text-(--color-oa-ink)">
            {stats().count}
          </p>
        </div>
        <div class="rounded-lg border border-white/10 bg-white/[0.03] px-3 py-2">
          <p class="text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            Played
          </p>
          <p class="mt-0.5 text-lg font-semibold text-(--color-oa-ink)">
            <Show when={stats().totalPlaySecs > 0} fallback="—">
              {formatHours(stats().totalPlaySecs)}
            </Show>
          </p>
        </div>
        <div class="rounded-lg border border-white/10 bg-white/[0.03] px-3 py-2">
          <p class="text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            Completed
          </p>
          <p class="mt-0.5 text-lg font-semibold text-(--color-oa-ink)">
            {stats().completed}
          </p>
        </div>
        <div class="rounded-lg border border-white/10 bg-white/[0.03] px-3 py-2">
          <p class="text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            Favorites
          </p>
          <p class="mt-0.5 text-lg font-semibold text-(--color-oa-ink)">
            {stats().favorites}
          </p>
        </div>
      </div>

      {/* Progress bar — completion %. */}
      <Show when={stats().count > 0}>
        <div class="flex flex-col gap-1 px-5 pt-4">
          <div class="flex items-baseline justify-between">
            <span class="text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Your progress
            </span>
            <span class="text-[0.7rem] font-semibold text-(--color-oa-ink)">
              {stats().percent}%
            </span>
          </div>
          <div class="h-1.5 w-full overflow-hidden rounded-full bg-white/5">
            <div
              class="h-full rounded-full bg-(--color-system-accent) transition-[width] duration-500"
              style={{ width: `${stats().percent}%` }}
            />
          </div>
        </div>
      </Show>

      {/* Top 3 popular games — quick re-entry. */}
      <Show when={topGames().length > 0}>
        <div class="flex flex-col gap-2 px-5 pt-4 pb-5">
          <p class="text-[0.55rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
            Most played here
          </p>
          <ul class="flex flex-col gap-1">
            <For each={topGames()}>
              {(game) => {
                const coverSrc = () => media.coverUrl(game.systemId, game.id);
                return (
                  <li>
                    <button
                      type="button"
                      onClick={(e) => {
                        e.currentTarget.blur();
                        props.onPickGame(game);
                      }}
                      class="flex w-full items-center gap-2 rounded-md border border-white/5 bg-white/[0.02] px-2 py-1.5 text-left transition hover:border-(--color-system-accent)/40 hover:bg-white/[0.04] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
                    >
                      <div class="h-10 w-8 shrink-0 overflow-hidden rounded border border-white/10">
                        <Show
                          when={coverSrc()}
                          fallback={
                            <div
                              class="h-full w-full"
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
                              alt={game.title}
                              class="h-full w-full object-cover"
                              draggable={false}
                            />
                          )}
                        </Show>
                      </div>
                      <div class="min-w-0 flex-1">
                        <p class="truncate text-[0.75rem] font-medium text-(--color-oa-ink)">
                          {game.title}
                        </p>
                        <p class="truncate text-[0.6rem] text-(--color-oa-ink-dim)">
                          <Show
                            when={(game.playTimeSecs ?? 0) > 0}
                            fallback={game.favorite ? "★ Favorite" : "Never played"}
                          >
                            {formatHours(game.playTimeSecs ?? 0)} played
                          </Show>
                        </p>
                      </div>
                    </button>
                  </li>
                );
              }}
            </For>
          </ul>
        </div>
      </Show>
    </div>
  );
};

export default SystemInfoPanel;
