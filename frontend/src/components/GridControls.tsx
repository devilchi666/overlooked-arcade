import { For, Show, type Component } from "solid-js";
import {
  GROUP_BY_LABELS,
  GROUP_BY_OPTIONS,
  SORT_KEY_LABELS,
  SORT_KEY_OPTIONS,
  VIEW_MODE_LABELS,
  VIEW_MODE_OPTIONS,
  type GroupBy,
  type LayoutStore,
  type SortKey,
  type ViewMode,
} from "../layout/state";

type Props = {
  layout: LayoutStore;
  /** Title for the current view (e.g. "All games", "TurboGrafx-16"). */
  title: string;
  /** Total number of entries currently rendered. */
  count: number;
  /** Show the per-system settings ⚙ button when set. Triggered from
   *  LibraryView only when the active SidebarView is `kind: "system"`. */
  onOpenSystemSettings?: () => void;
};

const SELECT_CLASS =
  "rounded-md border border-white/10 bg-white/[0.03] px-2 py-1 text-xs text-(--color-oa-ink) transition hover:bg-white/[0.07] focus-visible:outline focus-visible:outline-1 focus-visible:outline-(--color-system-accent)";

const VIEW_BTN =
  "px-2 py-1 text-[0.65rem] uppercase tracking-wider transition";

/**
 * Sticky bar above the library grid. Combines:
 *   - Current view title + entry count (left)
 *   - View mode picker (segmented: Capsule / List)
 *   - Sort dropdown
 *   - Group-by dropdown
 *
 * State is fully owned by the LayoutStore so the values persist across
 * restarts via layout.json.
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
      <div class="flex shrink-0 items-center gap-2">
        {/* Per-system settings ⚙ — only rendered when the parent passes
            `onOpenSystemSettings` (i.e. when the view is system-filtered).
            Sits at the front of the actions cluster as the most prominent
            system-scoped affordance. */}
        <Show when={props.onOpenSystemSettings}>
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              props.onOpenSystemSettings?.();
            }}
            class="flex items-center gap-1.5 rounded-md border border-white/10 bg-white/[0.03] px-2 py-1 text-[0.65rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.07] hover:text-(--color-oa-ink) hover:border-(--color-system-accent)/50"
            title="Per-system settings"
          >
            <span aria-hidden="true">⚙</span>
            <span>Settings</span>
          </button>
        </Show>
        {/* View mode — segmented control */}
        <div class="flex overflow-hidden rounded-md border border-white/10 bg-white/[0.03]">
          <For each={VIEW_MODE_OPTIONS}>
            {(mode) => (
              <button
                type="button"
                onClick={(e) => {
                  e.currentTarget.blur();
                  props.layout.setViewMode(mode as ViewMode);
                }}
                aria-pressed={props.layout.viewMode() === mode}
                class={VIEW_BTN}
                classList={{
                  "bg-(--color-system-accent)/15 text-(--color-oa-ink)":
                    props.layout.viewMode() === mode,
                  "text-(--color-oa-ink-dim) hover:bg-white/[0.05] hover:text-(--color-oa-ink)":
                    props.layout.viewMode() !== mode,
                }}
                title={VIEW_MODE_LABELS[mode]}
              >
                {mode === "capsule" ? "▦" : "≡"}
              </button>
            )}
          </For>
        </div>
        {/* Sort */}
        <label class="flex items-center gap-1.5">
          <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">Sort</span>
          <select
            value={props.layout.sortKey()}
            onChange={(e) => props.layout.setSortKey(e.currentTarget.value as SortKey)}
            class={SELECT_CLASS}
          >
            <For each={SORT_KEY_OPTIONS}>
              {(k) => <option value={k}>{SORT_KEY_LABELS[k]}</option>}
            </For>
          </select>
        </label>
        {/* Group */}
        <label class="flex items-center gap-1.5">
          <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">Group</span>
          <select
            value={props.layout.groupBy()}
            onChange={(e) => props.layout.setGroupBy(e.currentTarget.value as GroupBy)}
            class={SELECT_CLASS}
          >
            <For each={GROUP_BY_OPTIONS}>
              {(g) => <option value={g}>{GROUP_BY_LABELS[g]}</option>}
            </For>
          </select>
        </label>
      </div>
    </div>
  );
};

export default GridControls;
