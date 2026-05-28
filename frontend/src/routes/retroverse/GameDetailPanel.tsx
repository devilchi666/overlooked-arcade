// Focused-game detail panel rendered in the right column of the
// Retroverse LIBRARY / COLLECTIONS / PLAY NOW tabs and HOME(when a
// game is focused).
//
// Layout matches the operator-supplied library-default-mockup.png:
// cover hero at top, system label + title, conditional chip strip,
// description, screenshots row, your-progress block, PLAY GAME +
// MORE actions pinned at the bottom.
//
// Missing-data rule: chip rows render only when the source has data
// (genre / publisher / release / etc. all hide when not synced) —
// avoids the panel feeling "empty" because a metadata field hasn't
// been enriched yet.

import { For, Show, type Component } from "solid-js";
import { convertFileSrc } from "@tauri-apps/api/core";
import { getDataDir } from "../../lib/dataDir";
import { createResource } from "solid-js";
import { useMedia } from "../../library/media";
import type { RomEntry } from "../../library/types";
import { systemThemes } from "../../themes/registry";

type Props = {
  entry: RomEntry;
  /// Fired when the operator hits the PLAY GAME button. Caller is
  /// responsible for routing through App.handleLaunch so per-game
  /// override resolution / arming / postLaunchUiUpdate all fire.
  onLaunch: (entry: RomEntry) => void;
  /// Fired when the operator hits MORE — opens the existing modal
  /// GameInfoModal for the full surface (variants / region picker /
  /// pick-core / etc.).
  onShowInfo?: (entry: RomEntry) => void;
  /// Fired when the operator hits the favorite heart in the header.
  onToggleFavorite?: (entry: RomEntry, value: boolean) => void;
};

function formatHours(secs?: number): string {
  if (!secs || secs <= 0) return "";
  const hours = Math.floor(secs / 3600);
  const mins = Math.floor((secs % 3600) / 60);
  if (hours > 0) return `${hours}h ${mins}m`;
  return `${mins}m`;
}

function formatDate(unixSecs?: number): string {
  if (!unixSecs) return "";
  try {
    return new Date(unixSecs * 1000).toLocaleDateString(undefined, {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  } catch {
    return "";
  }
}

const GameDetailPanel: Component<Props> = (props) => {
  const media = useMedia();
  const [appDataPath] = createResource(async () => {
    try {
      return await getDataDir();
    } catch {
      return "";
    }
  });

  const metadata = () => media.media(props.entry.id)?.metadata;
  const themeName = () =>
    systemThemes[props.entry.systemId]?.displayName ?? props.entry.systemId;
  const coverSrc = () => media.coverUrl(props.entry.systemId, props.entry.id);

  // Screenshots — gameplay variants, capped at 3. Falls back to empty
  // when not synced.
  const screenshots = () => {
    const g = media.media(props.entry.id)?.screenshotGameplay;
    if (!g || g.length === 0) return [];
    const dir = appDataPath() ?? "";
    if (!dir) return [];
    const sep = dir.endsWith("/") || dir.endsWith("\\") ? "" : "/";
    return g
      .slice(0, 3)
      .map((v) => convertFileSrc(`${dir}${sep}${v.thumbPath ?? v.path}`));
  };

  // Players chip — entry.players first (Phase C3 column), fall through
  // to metadata.players (LaunchBox sync), else "—".
  const playersLabel = () => {
    const p = props.entry.players ?? metadata()?.players;
    if (!p) return null;
    return p > 1 ? `${p} players` : "1 player";
  };

  return (
    <div
      class="flex h-full flex-col overflow-y-auto bg-(--color-oa-bg-deep)"
      data-system={props.entry.systemId}
    >
      {/* Cover hero — full-bleed at the top, aspect-locked. */}
      <div class="relative aspect-[3/4] max-h-[260px] w-full overflow-hidden bg-black/30">
        <Show
          when={coverSrc()}
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
              alt={props.entry.title}
              class="absolute inset-0 h-full w-full object-cover"
              draggable={false}
            />
          )}
        </Show>
        <div class="absolute inset-x-0 bottom-0 h-16 bg-gradient-to-t from-(--color-oa-bg-deep) to-transparent" />

        {/* Favorite heart in top-right corner — keeps tile + detail in
            sync without forcing the operator to use the tile button. */}
        <Show when={props.onToggleFavorite}>
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              props.onToggleFavorite?.(props.entry, !props.entry.favorite);
            }}
            class="absolute right-3 top-3 grid h-8 w-8 place-items-center rounded bg-black/60 text-lg leading-none backdrop-blur transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
            classList={{
              "text-(--color-system-accent)": Boolean(props.entry.favorite),
              "text-(--color-oa-ink-dim) hover:text-(--color-system-accent-soft)":
                !props.entry.favorite,
            }}
            aria-label={props.entry.favorite ? "Remove from favorites" : "Add to favorites"}
            aria-pressed={Boolean(props.entry.favorite)}
            title={props.entry.favorite ? "Remove from favorites" : "Add to favorites"}
          >
            {props.entry.favorite ? "♥" : "♡"}
          </button>
        </Show>
      </div>

      {/* Title + subtitle + system label. */}
      <div class="flex flex-col gap-1 px-5 pt-4">
        <p class="text-[0.6rem] uppercase tracking-[0.4em] text-(--color-system-accent)">
          {themeName()}
        </p>
        <h2 class="text-xl font-semibold leading-tight text-(--color-oa-ink)" title={props.entry.title}>
          {props.entry.title}
        </h2>
      </div>

      {/* Chip strip — only render chips for fields that have data. */}
      <div class="flex flex-wrap gap-1.5 px-5 pt-3">
        <Show when={metadata()?.genre}>
          <span class="rounded border border-(--color-system-accent)/40 bg-(--color-system-accent)/10 px-2 py-0.5 text-[0.6rem] font-medium uppercase tracking-widest text-(--color-system-accent-soft)">
            {metadata()!.genre}
          </span>
        </Show>
        <Show when={metadata()?.year}>
          <span class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            {metadata()!.year}
          </span>
        </Show>
        <Show when={metadata()?.developer}>
          <span class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            {metadata()!.developer}
          </span>
        </Show>
        <Show when={metadata()?.publisher && metadata()?.publisher !== metadata()?.developer}>
          <span class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            {metadata()!.publisher}
          </span>
        </Show>
        <Show when={playersLabel()}>
          <span class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            {playersLabel()}
          </span>
        </Show>
        <Show when={(props.entry.playTimeSecs ?? 0) > 0}>
          <span class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            {formatHours(props.entry.playTimeSecs)} played
          </span>
        </Show>
        <Show when={props.entry.completed}>
          <span class="rounded border border-emerald-400/40 bg-emerald-500/10 px-2 py-0.5 text-[0.6rem] uppercase tracking-widest text-emerald-300">
            ✓ Completed
          </span>
        </Show>
      </div>

      {/* Description — graceful fallback when metadata isn't synced. */}
      <Show when={metadata()?.description}>
        <p class="px-5 pt-4 text-[0.8rem] leading-relaxed text-(--color-oa-ink-dim)">
          {metadata()!.description}
        </p>
      </Show>

      {/* Screenshots row — 3 thumbs, hidden when not synced. */}
      <Show when={screenshots().length > 0}>
        <div class="flex flex-col gap-2 px-5 pt-4">
          <p class="text-[0.55rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
            Screenshots
          </p>
          <div class="flex gap-2 overflow-x-auto pb-1">
            <For each={screenshots()}>
              {(src) => (
                <img
                  src={src}
                  alt=""
                  class="h-16 w-24 shrink-0 rounded border border-white/10 object-cover"
                  draggable={false}
                />
              )}
            </For>
          </div>
        </div>
      </Show>

      {/* Your progress — last played + (placeholder) achievements. */}
      <div class="flex flex-col gap-1.5 px-5 pt-4">
        <p class="text-[0.55rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
          Your progress
        </p>
        <Show
          when={props.entry.lastPlayedAt}
          fallback={
            <p class="text-[0.7rem] text-(--color-oa-ink-dim)/70">
              Never played
            </p>
          }
        >
          <p class="text-[0.75rem] text-(--color-oa-ink-dim)">
            Last played {formatDate(props.entry.lastPlayedAt)}
          </p>
        </Show>
        {/* Achievement bar placeholder — wired in a future
            RetroAchievements slice. */}
        <div class="mt-1 flex items-center gap-2">
          <span class="text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)/60">
            Achievements
          </span>
          <span class="text-[0.6rem] text-(--color-oa-ink-dim)/60">—</span>
        </div>
      </div>

      {/* Action buttons — pinned at the bottom via mt-auto + padding. */}
      <div class="mt-auto flex gap-2 px-5 py-5">
        <button
          type="button"
          onClick={(e) => {
            e.currentTarget.blur();
            props.onLaunch(props.entry);
          }}
          class="flex-1 rounded-md border border-(--color-system-accent) bg-(--color-system-accent)/15 px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-(--color-oa-ink) transition hover:bg-(--color-system-accent)/25 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
        >
          ▶ Play game
        </button>
        <Show when={props.onShowInfo}>
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              props.onShowInfo?.(props.entry);
            }}
            class="rounded-md border border-white/10 bg-white/[0.04] px-4 py-2.5 text-xs uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
            title="Open the full info modal (variants, region picker, core override)"
          >
            ⋯ More
          </button>
        </Show>
      </div>
    </div>
  );
};

export default GameDetailPanel;
