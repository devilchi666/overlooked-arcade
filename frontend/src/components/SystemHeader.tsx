import { type Component } from "solid-js";
import { systemThemes, type SystemId } from "../themes/registry";
import SystemCoresStrip from "./SystemCoresStrip";

type Props = {
  systemId: SystemId;
  gameCount: number;
};

/// System landing band — shown above the GridControls bar when the user
/// has navigated into a system-filtered view (left-clicked a system in
/// the left sidebar). Surfaces the system's identity + a collapsible
/// per-system cores strip (browse + install). All other per-system
/// surfaces (bindings, shaders, default core, etc.) are reachable from
/// the top-bar `System ▾` menu or the sidebar right-click context menu.
const SystemHeader: Component<Props> = (props) => {
  const theme = () => systemThemes[props.systemId];
  return (
    <header
      class="border-b border-white/5 bg-(--color-oa-bg-deep)/95 px-(--layout-content-padding-x) py-4 backdrop-blur"
      data-system={props.systemId}
    >
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
      <SystemCoresStrip systemId={props.systemId} />
    </header>
  );
};

export default SystemHeader;
