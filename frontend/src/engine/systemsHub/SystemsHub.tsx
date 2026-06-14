// Level-1 view: the grid of per-system cards + the library-first / Show-all
// toggle. Picking a card drills into that system's domain hub.

import { For, Show, createSignal, type Component } from "solid-js";
import type { SystemId } from "@oa/platform/themes/registry";
import { HubGrid } from "./HubGrid";
import { SystemCard } from "./SystemCard";
import { useSystemsStats } from "./systemsHubStats";

export const SystemsHub: Component<{ onPick: (id: SystemId) => void }> = (props) => {
  const stats = useSystemsStats();
  const [showAll, setShowAll] = createSignal(false);
  const systems = (): SystemId[] => (showAll() ? stats.allSystems() : stats.librarySystems());

  return (
    <div class="flex flex-col gap-4">
      <div class="flex items-center justify-between gap-3">
        <p class="text-[0.7rem] text-(--color-oa-ink-dim)">
          {stats.librarySystems().length} system
          {stats.librarySystems().length === 1 ? "" : "s"} in your library
          <Show when={stats.incompleteCount() > 0}>
            {" · "}
            <span class="text-amber-300">{stats.incompleteCount()} incomplete</span>
          </Show>
        </p>
        <button
          type="button"
          onClick={(e) => {
            e.currentTarget.blur();
            setShowAll((v) => !v);
          }}
          class="rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-[0.65rem] font-semibold uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
        >
          {showAll() ? "Library only" : "Show all systems"}
        </button>
      </div>
      <HubGrid>
        <For each={systems()}>
          {(id) => (
            <SystemCard systemId={id} stats={stats.statsFor(id)} onPick={() => props.onPick(id)} />
          )}
        </For>
      </HubGrid>
    </div>
  );
};
