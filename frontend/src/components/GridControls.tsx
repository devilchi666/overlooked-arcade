import { Show, type Component } from "solid-js";
import {
  LIBRARY_TILE_SIZE_MAX,
  LIBRARY_TILE_SIZE_MIN,
  LIBRARY_TILE_SIZE_STEP,
} from "../layout/state";

type Props = {
  /** Title for the current view (e.g. "All games", "TurboGrafx-16"). */
  title: string;
  /** Total number of entries currently rendered. */
  count: number;
  /** Current tile-size target in px (slider value). Omit to hide the slider
   *  (e.g. when the active view is the detail list). */
  tileSize?: number;
  onTileSizeChange?: (px: number) => void;
};

/**
 * Sticky bar above the library grid. Shows the current view title + entry
 * count, and (when the capsule grid is active) a tile-size slider that
 * scrubs the rendered tile width live.
 */
const GridControls: Component<Props> = (props) => {
  return (
    <div class="sticky top-0 z-10 flex items-center justify-between gap-3 border-b border-white/5 bg-(--color-oa-bg-deep)/95 px-(--layout-content-padding-x) py-2 backdrop-blur">
      <div class="flex min-w-0 items-baseline gap-3">
        <h2 class="truncate text-sm font-semibold text-(--color-oa-ink)">{props.title}</h2>
        <Show when={props.count > 0}>
          <p class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            {props.count} {props.count === 1 ? "game" : "games"}
          </p>
        </Show>
      </div>
      <Show when={props.tileSize !== undefined && props.onTileSizeChange}>
        <label
          class="flex shrink-0 items-center gap-2 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)"
          title={`Tile size: ${props.tileSize}px`}
        >
          <span aria-hidden="true">Tile size</span>
          <input
            type="range"
            min={LIBRARY_TILE_SIZE_MIN}
            max={LIBRARY_TILE_SIZE_MAX}
            step={LIBRARY_TILE_SIZE_STEP}
            value={props.tileSize}
            onInput={(e) => props.onTileSizeChange?.(+e.currentTarget.value)}
            aria-label="Library tile size"
            class="h-1 w-28 cursor-pointer accent-(--color-system-accent)"
          />
          <span class="w-10 text-right tabular-nums normal-case tracking-normal">
            {props.tileSize}
          </span>
        </label>
      </Show>
    </div>
  );
};

export default GridControls;
