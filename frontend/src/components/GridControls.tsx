import { Show, type Component } from "solid-js";

type Props = {
  /** Title for the current view (e.g. "All games", "TurboGrafx-16"). */
  title: string;
  /** Total number of entries currently rendered. */
  count: number;
};

/**
 * Sticky bar above the library grid. Shows the current view title + entry
 * count. View mode / sort / group / per-system settings controls all moved
 * to the top-bar `View` and `System ▾` menus respectively.
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
    </div>
  );
};

export default GridControls;
