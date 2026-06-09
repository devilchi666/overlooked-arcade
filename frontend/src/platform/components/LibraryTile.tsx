import { createEffect, createSignal, For, Show, type Component } from "solid-js";
import type { RomEntry } from "@oa/platform/library/types";
import { useMedia } from "@oa/platform/library/media";
import { useGameInfoBadges } from "@oa/platform/library/gameInfoBadges";
import { DEFAULT_TILE_ASPECT, systemThemes } from "@oa/platform/themes/registry";
import { uiConfigFor, type UITileShape } from "@oa/platform/themes/systemUIConfigs";
import { isPerSystemUiEnabled } from "@oa/platform/themes/systemUiSound";

/// Per-System UI Stage 1 Slice 5 — map the `tileShape` enum to a CSS
/// `aspect-ratio` value. `"auto"` returns null so the caller falls back
/// to the existing `systemThemes[id].tileAspect`. `"circle"` shares the
/// 1/1 aspect with `"square"` and additionally applies a border-radius
/// override on the cover container.
function aspectForTileShape(shape: UITileShape): string | null {
  switch (shape) {
    case "square":
    case "circle":
      return "1 / 1";
    case "portrait-3:4":
      return "3 / 4";
    case "landscape-4:3":
      return "4 / 3";
    case "wide-16:9":
      return "16 / 9";
    case "auto":
    default:
      return null;
  }
}

type Props = {
  entry: RomEntry;
  onLaunch: (entry: RomEntry) => void;
  onShowSaves?: (entry: RomEntry) => void;
  /// Right-click handler. Opens the unified tile context menu (cover, saves,
  /// core picker, remove). Position is raw client coords for the popover anchor.
  onPickContext?: (entry: RomEntry, position: { x: number; y: number }) => void;
  /// Focus / select callback — fired on single-click (mouse) and on keyboard
  /// focus (Tab navigation). Hover does NOT trigger this; selection sticks
  /// until the user clicks another tile or Tabs away. Used by the right
  /// sidebar to display widgets for the currently-picked entry and by Tools
  /// menu / per-system / per-game settings to know which game is "active"
  /// without having to launch it.
  onFocus?: (entry: RomEntry) => void;
  /// Visual selected state. The tile renders an accent ring when true. Parent
  /// derives this from the current selection signal so virtualized tiles only
  /// re-render when their own selected-ness flips.
  selected?: boolean;
  /// When this tile represents a multi-variant game group (e.g. multiple
  /// regions / revisions of the same title), the count of variants in
  /// that group. The tile renders a `▼N` badge so the user knows the
  /// right-click menu offers "Run version ▸ ...". Single-file games
  /// pass `undefined` (or 1) — no badge rendered.
  variantCount?: number;
  /// VL Phase B — Preservation-mode variant ribbon: short region /
  /// revision chips (e.g. `["USA", "JP", "EU"]`) summarizing the group's
  /// variants. When present (preservation mode + multi-variant group)
  /// the tile renders the ribbon along the bottom and suppresses the
  /// plain ▼N badge — the ribbon already conveys the multi-variant
  /// state with more detail. `undefined` in casual mode.
  variantRibbon?: string[];
  /// Retroverse-UI Phase C3 — flip favorite state. Fires from the heart
  /// overlay (always-visible top-right) and the context-menu Favorite
  /// item. When omitted the heart hides entirely (caller didn't wire
  /// favorites in this surface). Caller stops propagation as needed.
  onToggleFavorite?: (entry: RomEntry, value: boolean) => void;
  /// Retroverse-UI LIBRARY polish — when true, render a small system-
  /// label header strip at the top of the cover area (system short
  /// name on a backdrop-blur band). Matches the operator-supplied
  /// library-default-mockup.png. Off by default — legacy LibraryView
  /// callers don't pass this so behavior is unchanged.
  showSystemHeader?: boolean;
};

const Placeholder: Component = () => (
  <div
    class="absolute inset-0"
    style={{
      background:
        "radial-gradient(circle at 30% 25%, var(--color-system-glow), transparent 60%), linear-gradient(135deg, var(--color-system-accent) 0%, var(--color-oa-bg-deep) 100%)",
    }}
  />
);

const LibraryTile: Component<Props> = (props) => {
  const theme = () => systemThemes[props.entry.systemId];
  const media = useMedia();
  /// Game Info Panel v1 Phase 6 — pull this tile's badge (bug count +
  /// has-local-edits) from the global store. Returns undefined for the
  /// vast majority of tiles that have no badge data; only games with
  /// known issues OR operator overrides render the overlay.
  const badges = useGameInfoBadges();
  const tileBadge = () => badges.badgeFor(props.entry.systemId, props.entry.id);
  /// Per-System UI Stage 1 Slice 5: pull the per-system tileShape +
  /// interactionStyle. When `perSystemUiEnabled` is OFF (uniform plain
  /// library mode), both fall back to the existing theme aspect + the
  /// default interaction feel.
  const uiConfig = () => uiConfigFor(props.entry.systemId);
  const tileAspectStyle = (): string => {
    if (!isPerSystemUiEnabled()) return theme().tileAspect ?? DEFAULT_TILE_ASPECT;
    const override = aspectForTileShape(uiConfig().tileShape);
    return override ?? theme().tileAspect ?? DEFAULT_TILE_ASPECT;
  };
  const isCircle = () =>
    isPerSystemUiEnabled() && uiConfig().tileShape === "circle";
  /// Surfaced as a `data-oa-interaction` attribute on the tile button
  /// so the CSS rules in index.css (`@layer base`) can swap transition
  /// timing + add a click-bounce keyframe for the `physical` profile.
  /// When the master toggle is off we publish `instant` so every tile
  /// shares the baseline feel regardless of its config entry.
  const interactionAttr = (): string => {
    if (!isPerSystemUiEnabled()) return "instant";
    return uiConfig().interactionStyle;
  };
  /// Subsystem distinction for systems where a single OA system_id covers
  /// multiple hardware variants distinguishable from the ROM file (e.g.
  /// NGP mono vs NGPC color via `.ngp` vs `.ngc` extension). Returns the
  /// hardware-specific short label to render in place of the system's
  /// canonical shortName, or null if the system has no such split.
  const subsystemLabel = (): string | null => {
    const sysId = props.entry.systemId;
    const path = props.entry.filePath ?? "";
    if (sysId === "ngp") {
      // .ngp = mono Neo Geo Pocket (1998); .ngc = NGP Color (1999).
      // Beetle NeoPop covers both via ROM-header auto-detect; the
      // tile label surfaces which is which at a glance.
      const lower = path.toLowerCase();
      if (lower.endsWith(".ngc")) return "NGPC";
      if (lower.endsWith(".ngp")) return "NGP";
    }
    return null;
  };
  // `coverUrl` is reactive via the MediaContext store — changing variants
  // (region pick, manual override, sync) causes this to re-render the <img>.
  // VL Phase E Sub-phase 3 — the identity's media key (canonical
  // "parent" artwork, keyed by `idn-…` in the same MediaDb) wins when
  // present; the per-file key is the everyday fallback. Nothing writes
  // identity-key art yet — the resolution order ships ahead of the
  // canonical-art management UI (Phase B/F) so it lights up when that
  // lands.
  const coverSrc = () =>
    (props.entry.identityId
      ? media.coverUrl(props.entry.systemId, props.entry.identityId)
      : null) ?? media.coverUrl(props.entry.systemId, props.entry.id);

  // Per-tile load/error state so we can show shimmer while the image
  // streams in and gracefully fall back to the gradient on error.
  const [imgLoaded, setImgLoaded] = createSignal(false);
  const [imgErrored, setImgErrored] = createSignal(false);
  // Only reset load state when the URL ACTUALLY changes. Previously this
  // ran as a side effect inside a derived getter that re-fired on every
  // MediaIndex update — including metadata-only updates that didn't touch
  // boxart — wiping imgLoaded back to false. Since the URL was identical,
  // the browser didn't re-fire onLoad, so imgLoaded stayed stuck at false
  // and the shimmer overlay covered the cached image until app restart.
  let prevSrc: string | null = null;
  createEffect(() => {
    const s = coverSrc();
    if (s !== prevSrc) {
      prevSrc = s;
      setImgLoaded(false);
      setImgErrored(false);
    }
  });

  return (
    <button
      type="button"
      // Single click selects; double click launches. Hover no longer changes
      // selection — picking a game and then mousing toward the menu used to
      // re-focus whichever tiles the cursor crossed, which made "pick game,
      // open its settings" effectively impossible.
      //
      // Phase A1 Sub-phase 4 hotfix — disc-set tiles fire onLaunch on
      // SINGLE click instead. The wrapped onLaunch handler in LibraryView
      // opens the DiscPickerDialog rather than actually launching, so the
      // operator's instinct of "click the set, see the discs" works. We
      // still call onFocus first so the right pane / context menu pick
      // up the focus state alongside the picker.
      onClick={() => {
        props.onFocus?.(props.entry);
        if (props.entry.discSetId !== undefined) {
          props.onLaunch(props.entry);
        }
      }}
      onDblClick={() => props.onLaunch(props.entry)}
      onContextMenu={(e) => {
        if (props.entry.seed || !props.onPickContext) return;
        e.preventDefault();
        props.onPickContext(props.entry, { x: e.clientX, y: e.clientY });
      }}
      // Keyboard focus still feeds the selection — Tab-to-tile then Enter
      // (the browser's default Activate-button) launches via the click path,
      // which now selects. To launch from the keyboard, the user can press
      // Enter twice quickly (the double-click handler fires on rapid
      // repeats), use the right sidebar's Launch button, or right-click →
      // Launch from the context menu.
      onFocus={() => props.onFocus?.(props.entry)}
      class="group relative flex w-full flex-col overflow-hidden rounded-lg border bg-white/[0.03] text-left shadow-lg shadow-black/40 transition duration-200 hover:-translate-y-0.5 hover:shadow-xl hover:shadow-black/60 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
      classList={{
        "border-white/5": !props.selected,
        "border-(--color-system-accent) ring-1 ring-(--color-system-accent)/60":
          props.selected === true,
      }}
      aria-pressed={props.selected === true}
      data-system={props.entry.systemId}
      data-oa-interaction={interactionAttr()}
    >
      <div
        class="relative w-full overflow-hidden bg-(--color-oa-bg-deep)"
        classList={{ "rounded-full": isCircle() }}
        style={{ "aspect-ratio": tileAspectStyle() }}
      >
        {/* Cover area: the orange/system gradient placeholder ONLY shows
            when there's no cover (or the load errored) — when a cover IS
            present, any letterbox bars from object-contain expose the
            tile's dark parent bg, not the gradient. Layout is still
            stable across all three states because the slot's aspect-ratio
            is fixed at the parent level. */}
        <Show when={!coverSrc() || imgErrored()}>
          <Placeholder />
        </Show>
        <Show when={imgErrored() ? null : coverSrc()}>
          {(src) => (
            <>
              <Show when={!imgLoaded()}>
                <div class="oa-cover-shimmer absolute inset-0" aria-hidden="true" />
              </Show>
              <img
                src={src()}
                alt={props.entry.title}
                class="absolute inset-0 h-full w-full object-contain transition-transform duration-200 ease-out group-hover:scale-[1.02]"
                classList={{ "opacity-0": !imgLoaded(), "opacity-100": imgLoaded() }}
                style={{ transition: "opacity 200ms ease, transform 200ms ease" }}
                loading="lazy"
                decoding="async"
                ref={(el) => {
                  // Cached-image safety net: if the <img> hits the DOM and
                  // is already complete (browser served it from cache before
                  // we attached onLoad), flip imgLoaded immediately so the
                  // fade-in doesn't get stuck.
                  queueMicrotask(() => {
                    if (el.complete && el.naturalHeight > 0) {
                      setImgLoaded(true);
                    }
                  });
                }}
                onLoad={() => setImgLoaded(true)}
                onError={() => setImgErrored(true)}
              />
            </>
          )}
        </Show>
        <div class="pointer-events-none absolute inset-0 ring-1 ring-white/5 ring-inset" />
        <div
          class="pointer-events-none absolute inset-0 opacity-0 transition-opacity duration-200 group-hover:opacity-100"
          style={{ "box-shadow": "inset 0 0 0 2px var(--color-system-accent)" }}
        />
        <Show when={props.entry.seed}>
          <span class="absolute right-2 top-2 rounded bg-black/60 px-1.5 py-0.5 text-[0.55rem] font-medium uppercase tracking-widest text-(--color-oa-ink-dim) backdrop-blur">
            Preview
          </span>
        </Show>
        {/* Retroverse-UI LIBRARY polish — system-label header strip at
            the top of the cover. Opt-in via `showSystemHeader` prop so
            legacy UI unchanged. Sits behind the favorite heart + Saves
            button (top-right corner overlay region); positioned on the
            top-LEFT so they don't collide. */}
        <Show when={props.showSystemHeader && !props.entry.seed}>
          <span
            class="absolute left-2 top-2 inline-flex items-center gap-1 rounded bg-black/55 px-2 py-0.5 text-[0.55rem] font-semibold uppercase tracking-widest text-(--color-oa-ink) backdrop-blur"
            title={theme().displayName}
          >
            <span class="text-(--color-system-accent)">◆</span>
            {subsystemLabel() ?? theme().shortName}
          </span>
        </Show>
        <Show when={!props.entry.seed && props.entry.coreOverride}>
          <span
            class="absolute left-2 rounded bg-black/65 px-1.5 py-0.5 text-[0.55rem] font-medium uppercase tracking-widest text-(--color-system-accent-soft) backdrop-blur"
            classList={{
              // Stack under the system-header strip when both are shown.
              "top-9": Boolean(props.showSystemHeader),
              "top-2": !props.showSystemHeader,
            }}
            title={`Custom core: ${props.entry.coreOverride}`}
          >
            Core ◆
          </span>
        </Show>
        {/* Casual-mode multi-variant badge. Suppressed when the
            Preservation ribbon is shown (the ribbon below conveys the
            same multi-variant state with region/revision detail). */}
        <Show
          when={
            !props.entry.seed &&
            (props.variantCount ?? 0) > 1 &&
            !(props.variantRibbon && props.variantRibbon.length > 0)
          }
        >
          <span
            class="absolute bottom-2 left-2 rounded bg-black/70 px-1.5 py-0.5 text-[0.6rem] font-semibold uppercase tracking-widest text-(--color-system-accent-soft) backdrop-blur"
            title={`${props.variantCount} versions / regions — right-click to pick`}
          >
            ▼ {props.variantCount}
          </span>
        </Show>
        {/* VL Phase B — Preservation-mode variant ribbon. A row of
            region / revision chips summarizing every dump under this
            identity. Bottom-left, wraps, capped (the helper bakes a
            "+N" overflow chip) so it can't blow out the tile. */}
        <Show when={!props.entry.seed && props.variantRibbon && props.variantRibbon.length > 0}>
          <div
            class="absolute bottom-2 left-2 right-10 flex flex-wrap gap-1"
            title="Versions / regions in your library — open the Variants tab to launch a specific one"
          >
            <For each={props.variantRibbon}>
              {(chip) => (
                <span class="rounded bg-black/70 px-1.5 py-0.5 text-[0.55rem] font-semibold uppercase tracking-wider text-(--color-system-accent-soft) backdrop-blur">
                  {chip}
                </span>
              )}
            </For>
          </div>
        </Show>
        {/* Phase A1 Sub-phase 4 polish — disc-set badge. Renders in
            the top-left corner when the entry is a collapsed multi-
            disc representative. Distinct from the variant ▼N badge
            (bottom-left) so multi-region multi-disc games show BOTH
            at once. Click on the tile opens DiscPickerDialog (wired
            via LibraryView's intercepted onLaunch). */}
        <Show when={!props.entry.seed && (props.entry.discSetMemberCount ?? 0) > 1}>
          <span
            class="absolute top-2 left-2 rounded bg-black/70 px-1.5 py-0.5 text-[0.6rem] font-semibold uppercase tracking-widest text-(--color-system-accent-soft) backdrop-blur"
            title={`${props.entry.discSetMemberCount} disc set — click to pick`}
          >
            💿 {props.entry.discSetMemberCount}
          </span>
        </Show>
        {/* Game Info Panel v1 — tile badges (bottom-right corner). The
            warning indicator surfaces known-issue count, tinted by max
            severity (red for blocker, amber for major, neutral for
            minor/cosmetic). The pencil glyph marks operator-locally-
            edited games — both indicators visible together when both
            conditions hold. */}
        <Show when={!props.entry.seed && tileBadge()}>
          <div class="absolute bottom-2 right-2 flex items-center gap-1">
            <Show when={(tileBadge()!.bugCount) > 0}>
              <span
                class="rounded bg-black/70 px-1.5 py-0.5 text-[0.6rem] font-semibold uppercase tracking-widest backdrop-blur"
                classList={{
                  "text-red-300":   tileBadge()!.maxSeverity === "blocker",
                  "text-amber-300": tileBadge()!.maxSeverity === "major",
                  "text-(--color-oa-ink-dim)":
                    tileBadge()!.maxSeverity === "minor" ||
                    tileBadge()!.maxSeverity === "cosmetic",
                }}
                title={`${tileBadge()!.bugCount} known issue${
                  tileBadge()!.bugCount === 1 ? "" : "s"
                } — ${tileBadge()!.maxSeverity ?? "none"} severity`}
              >
                ⚠ {tileBadge()!.bugCount}
              </span>
            </Show>
            <Show when={tileBadge()!.hasLocalEdits}>
              <span
                class="rounded bg-black/70 px-1.5 py-0.5 text-[0.6rem] text-(--color-system-accent-soft) backdrop-blur"
                title="Operator has edited this game's info locally"
              >
                ✎
              </span>
            </Show>
          </div>
        </Show>
        <Show when={!props.entry.seed && props.onShowSaves}>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              e.currentTarget.blur();
              props.onShowSaves?.(props.entry);
            }}
            class="absolute top-2 rounded bg-black/70 px-2 py-1 text-[0.6rem] font-medium uppercase tracking-widest text-(--color-system-accent-soft) backdrop-blur opacity-0 transition-opacity duration-200 group-hover:opacity-100 focus-visible:opacity-100"
            classList={{
              // Shift left when the heart is present so they don't collide.
              "right-10": Boolean(props.onToggleFavorite) && !props.entry.seed,
              "right-2": !props.onToggleFavorite || props.entry.seed,
            }}
            aria-label="Open save states"
          >
            Saves
          </button>
        </Show>
        {/* Retroverse-UI Phase C3 — favorite heart overlay. Always-visible
            top-right corner (per docs/PLANS/collections-tab-retroverse.md +
            library-default-mockup.png). Filled when favorited, outline when
            not. Hidden entirely for seed tiles and when the parent didn't
            wire `onToggleFavorite`. */}
        <Show when={!props.entry.seed && props.onToggleFavorite}>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              e.currentTarget.blur();
              props.onToggleFavorite?.(props.entry, !props.entry.favorite);
            }}
            class="absolute right-2 top-2 grid h-7 w-7 place-items-center rounded bg-black/60 text-base leading-none backdrop-blur transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
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
      <div class="flex flex-col gap-1 px-3 py-3">
        <h3 class="truncate text-sm font-medium text-(--color-oa-ink)">
          {props.entry.title}
        </h3>
        <p class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
          {subsystemLabel() ?? theme().shortName}
        </p>
      </div>
    </button>
  );
};

export default LibraryTile;
