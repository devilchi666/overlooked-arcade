import { createEffect, createSignal, Show, type Component } from "solid-js";
import type { RomEntry } from "../library/types";
import { useMedia } from "../library/media";
import { DEFAULT_TILE_ASPECT, systemThemes } from "../themes/registry";

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
  const coverSrc = () => media.coverUrl(props.entry.systemId, props.entry.id);

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
      onClick={() => props.onFocus?.(props.entry)}
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
    >
      <div
        class="relative w-full overflow-hidden bg-(--color-oa-bg-deep)"
        style={{ "aspect-ratio": theme().tileAspect ?? DEFAULT_TILE_ASPECT }}
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
        <Show when={!props.entry.seed && props.entry.coreOverride}>
          <span
            class="absolute left-2 top-2 rounded bg-black/65 px-1.5 py-0.5 text-[0.55rem] font-medium uppercase tracking-widest text-(--color-system-accent-soft) backdrop-blur"
            title={`Custom core: ${props.entry.coreOverride}`}
          >
            Core ◆
          </span>
        </Show>
        <Show when={!props.entry.seed && (props.variantCount ?? 0) > 1}>
          <span
            class="absolute bottom-2 left-2 rounded bg-black/70 px-1.5 py-0.5 text-[0.6rem] font-semibold uppercase tracking-widest text-(--color-system-accent-soft) backdrop-blur"
            title={`${props.variantCount} versions / regions — right-click to pick`}
          >
            ▼ {props.variantCount}
          </span>
        </Show>
        <Show when={!props.entry.seed && props.onShowSaves}>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              e.currentTarget.blur();
              props.onShowSaves?.(props.entry);
            }}
            class="absolute right-2 top-2 rounded bg-black/70 px-2 py-1 text-[0.6rem] font-medium uppercase tracking-widest text-(--color-system-accent-soft) backdrop-blur opacity-0 transition-opacity duration-200 group-hover:opacity-100 focus-visible:opacity-100"
            aria-label="Open save states"
          >
            Saves
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
