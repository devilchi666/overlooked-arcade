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

import { For, Show, createSignal, type Component } from "solid-js";
import { convertFileSrc } from "@tauri-apps/api/core";
import { getDataDir } from "@oa/platform/lib/dataDir";
import { createResource } from "solid-js";
import { useMedia } from "@oa/platform/library/media";
import type { RomEntry } from "@oa/platform/library/types";
import { systemThemes } from "@oa/platform/themes/registry";
import {
  getGameInfo,
  getGameInfoOverride,
  setGameInfoOverride,
  type BugSeverity,
} from "@oa/platform/library/gameInfo";
import { useGameInfoBadges } from "@oa/platform/library/gameInfoBadges";
import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";

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

/// Glyph + tint for a single bug severity. Blockers get a red alert
/// triangle; major + minor share an amber warning glyph at different
/// emphasis; cosmetic shows as a neutral dot. Used by the KNOWN ISSUES
/// section.
function severityGlyph(s: BugSeverity): string {
  switch (s) {
    case "blocker": return "⚠";
    case "major":   return "⚠";
    case "minor":   return "•";
    case "cosmetic": return "·";
  }
}
function severityTintClass(s: BugSeverity): string {
  switch (s) {
    case "blocker":  return "text-red-300";
    case "major":    return "text-amber-300";
    case "minor":    return "text-(--color-oa-ink-dim)";
    case "cosmetic": return "text-(--color-oa-ink-dim)/60";
  }
}
function severityRank(s: BugSeverity): number {
  switch (s) {
    case "blocker":  return 3;
    case "major":    return 2;
    case "minor":    return 1;
    case "cosmetic": return 0;
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

  // Bump signal — the Apply best emulator action increments this so
  // the merged resource re-fetches with fresh appliedBestEmulator
  // provenance without forcing a full library refresh.
  const [mergedRefreshKey, setMergedRefreshKey] = createSignal(0);

  // Fetch the merged Game Info record (file layer + operator overrides)
  // for the focused entry. Re-fetches when the operator focuses a
  // different tile — the createResource source closure depends on the
  // identifying fields. Returns null when neither the file layer nor
  // operator overrides have any content, in which case the new
  // sections silently hide.
  const [merged] = createResource(
    () => ({
      systemId: props.entry.systemId,
      romId: props.entry.id,
      romHash: props.entry.sha1,
      romTitle: props.entry.title,
      _: mergedRefreshKey(),
    }),
    async (args) => {
      try {
        return await getGameInfo({
          systemId: args.systemId,
          romId: args.romId,
          romHash: args.romHash,
          romTitle: args.romTitle,
        });
      } catch (e) {
        console.warn("[GameDetailPanel] get_game_info failed:", e);
        return null;
      }
    },
  );

  // Bugs sorted by severity (blocker first). The tile-badge layer
  // also picks the max severity, but here we want operator-visible
  // ordering inside the panel.
  const sortedBugs = () => {
    const m = merged();
    if (!m) return [];
    return [...m.bugs].sort((a, b) => severityRank(b.severity) - severityRank(a.severity));
  };

  // ---- Apply best emulator action (Phase 8) -------------------------
  //
  // Wires the panel's [Apply] button next to the Recommended core
  // block. Writes `GameOverrides.libretro_core` for this game via the
  // existing per-game core-override Tauri path and marks the operator
  // override row's `appliedBestEmulator` provenance flag so:
  //  (a) the button switches to "Applied" without further input;
  //  (b) the tile gains the `✎` local-edits indicator;
  //  (c) the "Reset to default" affordance in the modal editor knows
  //      to unwind this override when used.
  const badges = useGameInfoBadges();

  async function handleApplyBestEmulator(): Promise<void> {
    const entry = props.entry;
    const m = merged();
    const recommended = m?.bestEmulator?.recommended;
    if (!entry || !recommended) return;
    try {
      // 1. Per-game core override write — operator's next launch uses
      // the recommended core. This is the same path the per-game
      // settings drawer uses.
      await invoke("update_game_core_override", {
        id: entry.id,
        value: recommended,
      });
      // 2. Read current override, set the provenance flag, write back.
      // Preserves any other operator edits — we mutate only the flag.
      const ov = await getGameInfoOverride({
        systemId: entry.systemId,
        romId: entry.id,
      });
      await setGameInfoOverride({
        systemId: entry.systemId,
        romId: entry.id,
        overrideRecord: { ...ov, appliedBestEmulator: true },
      });
      // 3. Refresh side effects: tile-badge cache (✎ shows up if it
      // wasn't already) + the merged record this panel reads from
      // (so the Apply button switches to Applied without an entry
      // change).
      await badges.refresh();
      setMergedRefreshKey((k: number) => k + 1);
      // 4. Inline confirmation toast — surfaces via the existing
      // oa://toast channel so the per-system theming carries through.
      await emit("oa://toast", {
        level: "success",
        message: `Best emulator applied — ${recommended} will be used on next launch.`,
        system: entry.systemId,
      });
    } catch (err) {
      console.warn("[GameDetailPanel] Apply best emulator failed:", err);
      await emit("oa://toast", {
        level: "error",
        message: "Apply best emulator failed — see debug log for details.",
        system: entry.systemId,
      });
    }
  }

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
              src={src()}
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

      {/* Description — graceful fallback when metadata isn't synced.
          Operator's Game Info Panel shortSummary (when present) takes
          the place of the libretro-database description. Local-source
          summaries surface with an "(operator note)" mini-label so the
          reader can tell they're operator-authored vs scraped content. */}
      <Show
        when={merged()?.shortSummary}
        fallback={
          <Show when={metadata()?.description}>
            <p class="px-5 pt-4 text-[0.8rem] leading-relaxed text-(--color-oa-ink-dim)">
              {metadata()!.description}
            </p>
          </Show>
        }
      >
        <div class="flex flex-col gap-1 px-5 pt-4">
          <Show when={merged()?.shortSummaryIsLocal}>
            <p class="text-[0.55rem] uppercase tracking-[0.4em] text-(--color-system-accent-soft)">
              Operator note
            </p>
          </Show>
          <p class="text-[0.8rem] leading-relaxed text-(--color-oa-ink-dim)">
            {merged()!.shortSummary}
          </p>
        </div>
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

      {/* Controls supported — chip strip. Hidden when the merged record
          has no entries (file layer empty + no operator override). Each
          chip uses the neutral white/10 border so they don't compete
          with the accent-tinted chips in the header chip strip. */}
      <Show when={(merged()?.controlsSupported ?? []).length > 0}>
        <div class="flex flex-col gap-2 px-5 pt-4">
          <p class="text-[0.55rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
            Controls
          </p>
          <div class="flex flex-wrap gap-1.5">
            <For each={merged()!.controlsSupported}>
              {(c) => (
                <span class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                  {c}
                </span>
              )}
            </For>
          </div>
        </div>
      </Show>

      {/* Recommended core — the file/operator-curated best emulator
          for this game. The Apply button is wired in Phase 8 (writes
          GameOverrides.libretro_core). For Phase 5 it surfaces as a
          disabled placeholder so the section reads correctly while
          the action plumbing lands. When the operator has already
          applied (provenance flag set), the button reflects the
          "Applied" state. */}
      <Show when={merged()?.bestEmulator}>
        <div class="flex flex-col gap-2 px-5 pt-4">
          <p class="text-[0.55rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
            Recommended core
          </p>
          <div class="flex items-start justify-between gap-2">
            <div class="flex flex-1 flex-col gap-1">
              <p class="font-mono text-[0.75rem] text-(--color-oa-ink)">
                {merged()!.bestEmulator!.recommended}
              </p>
              <Show when={merged()!.bestEmulator!.reason}>
                <p class="text-[0.7rem] leading-relaxed text-(--color-oa-ink-dim)">
                  {merged()!.bestEmulator!.reason}
                </p>
              </Show>
            </div>
            <button
              type="button"
              onClick={(e) => {
                e.currentTarget.blur();
                void handleApplyBestEmulator();
              }}
              disabled={merged()?.appliedBestEmulator ?? false}
              title={
                merged()?.appliedBestEmulator
                  ? "Recommended core already applied for this game — use the Game Info tab to reset"
                  : `Set ${merged()!.bestEmulator!.recommended} as the per-game core override`
              }
              class="shrink-0 rounded-md border px-3 py-1 text-[0.6rem] uppercase tracking-widest transition"
              classList={{
                "border-(--color-system-accent)/40 bg-(--color-system-accent)/10 text-(--color-system-accent-soft) cursor-default":
                  merged()?.appliedBestEmulator ?? false,
                "border-white/10 bg-white/[0.04] text-(--color-oa-ink-dim) hover:bg-white/[0.08] hover:text-(--color-oa-ink)":
                  !(merged()?.appliedBestEmulator ?? false),
              }}
            >
              {merged()?.appliedBestEmulator ? "Applied" : "Apply"}
            </button>
          </div>
        </div>
      </Show>

      {/* Known issues — operator-visible bug list. Sorted by severity
          descending (blockers first). Each entry shows a severity glyph
          tinted by severity, the description, and an optional workaround
          line indented underneath. */}
      <Show when={sortedBugs().length > 0}>
        <div class="flex flex-col gap-2 px-5 pt-4">
          <div class="flex items-center justify-between">
            <p class="text-[0.55rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
              Known issues
            </p>
            <span class="text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)/60">
              {sortedBugs().length}
            </span>
          </div>
          <div class="flex flex-col gap-2">
            <For each={sortedBugs()}>
              {(bug) => (
                <div class="flex items-start gap-2">
                  <span
                    class={`mt-px shrink-0 text-sm leading-none ${severityTintClass(bug.severity)}`}
                    aria-label={bug.severity}
                  >
                    {severityGlyph(bug.severity)}
                  </span>
                  <div class="flex flex-1 flex-col gap-0.5">
                    <p class="text-[0.7rem] leading-relaxed text-(--color-oa-ink-dim)">
                      <span
                        class={`mr-1 text-[0.55rem] uppercase tracking-widest ${severityTintClass(bug.severity)}`}
                      >
                        {bug.severity}
                      </span>
                      {bug.description}
                    </p>
                    <Show when={bug.workaround}>
                      <p class="text-[0.65rem] leading-relaxed text-(--color-oa-ink-dim)/70">
                        <span class="text-(--color-oa-ink-dim)/60">Workaround: </span>
                        {bug.workaround}
                      </p>
                    </Show>
                  </div>
                </div>
              )}
            </For>
          </div>
        </div>
      </Show>

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
