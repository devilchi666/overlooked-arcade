import { type Component } from "solid-js";
import { systemThemes, type SystemId } from "../themes/registry";
import type { SystemSettingsTab } from "../layout/LeftSidebar";

type Props = {
  systemId: SystemId;
  gameCount: number;
  /// Navigate to the per-system settings page, optionally landing on a
  /// specific tab. The header's quick-action buttons each pass a tab.
  onOpenSettings: (tab?: SystemSettingsTab) => void;
};

const QUICK_ACTION =
  "flex items-center gap-1.5 rounded-md border border-white/10 bg-white/[0.03] px-3 py-1.5 text-xs uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-(--color-system-accent)/15 hover:text-(--color-oa-ink) hover:border-(--color-system-accent)/50";

/// System landing page header — shown above the GridControls bar when the
/// user has navigated into a system-filtered view (left-clicked a system
/// in the left sidebar). Surfaces the system's identity (display name,
/// short tag chip, accent color via the data-system cascade) and the
/// most common per-system actions (Bindings / Cores / Shaders / Settings)
/// as deep-links into the per-system settings page's tabs.
///
/// Bindings is the most-requested quick action — without this header the
/// only way to reach the per-system bindings editor was: click ⚙ in the
/// grid bar → click "Input" in the tab strip → bindings. Now it's one
/// click from the system page.
const SystemHeader: Component<Props> = (props) => {
  const theme = () => systemThemes[props.systemId];
  return (
    <header
      class="border-b border-white/5 bg-(--color-oa-bg-deep)/95 px-(--layout-content-padding-x) py-4 backdrop-blur"
      data-system={props.systemId}
    >
      <div class="flex flex-wrap items-end justify-between gap-3">
        <div class="min-w-0 flex-1">
          <div class="flex items-center gap-2">
            <span class="inline-flex items-center rounded-md border border-(--color-system-accent)/40 bg-(--color-system-accent)/15 px-2 py-0.5 font-mono text-[0.65rem] uppercase tracking-wider text-(--color-system-accent-soft)">
              {theme()?.shortName ?? props.systemId.toUpperCase()}
            </span>
            <h1 class="truncate text-lg font-semibold text-(--color-oa-ink)">
              {theme()?.displayName ?? props.systemId}
            </h1>
          </div>
          <p class="mt-1 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            {props.gameCount} {props.gameCount === 1 ? "game" : "games"} in library
          </p>
        </div>

        <div class="flex flex-wrap items-center gap-2">
          <button
            type="button"
            class={QUICK_ACTION}
            title="Edit button bindings for this system"
            onClick={(e) => {
              e.currentTarget.blur();
              props.onOpenSettings("input");
            }}
          >
            <span aria-hidden="true">⌨</span>
            <span>Bindings</span>
          </button>
          <button
            type="button"
            class={QUICK_ACTION}
            title="Pick the default libretro core for this system"
            onClick={(e) => {
              e.currentTarget.blur();
              props.onOpenSettings("cores");
            }}
          >
            <span aria-hidden="true">⚡</span>
            <span>Cores</span>
          </button>
          <button
            type="button"
            class={QUICK_ACTION}
            title="Per-system shader preset + bloom amount"
            onClick={(e) => {
              e.currentTarget.blur();
              props.onOpenSettings("shaders");
            }}
          >
            <span aria-hidden="true">🎞</span>
            <span>Shaders</span>
          </button>
          <button
            type="button"
            class={
              QUICK_ACTION +
              " border-(--color-system-accent)/40 bg-(--color-system-accent)/10 text-(--color-system-accent-soft)"
            }
            title="All per-system settings"
            onClick={(e) => {
              e.currentTarget.blur();
              props.onOpenSettings();
            }}
          >
            <span aria-hidden="true">⚙</span>
            <span>Settings</span>
          </button>
        </div>
      </div>
    </header>
  );
};

export default SystemHeader;
