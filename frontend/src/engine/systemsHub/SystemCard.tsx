// Level-1 card: one per-system entry in the Systems grid. Shows the three
// status rows (identified / covers / metadata) like the Game-media card; clicks
// drill into that system's domain hub.

import { For, type Component } from "solid-js";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import { HubCard } from "./HubCard";
import { rowState, STATE_CLASS, STATE_GLYPH, type MediaCardStats } from "./systemsHubStats";

export const SystemCard: Component<{
  systemId: SystemId;
  stats: MediaCardStats;
  onPick: () => void;
}> = (props) => {
  const theme = () => systemThemes[props.systemId];
  const rows = (): Array<{ label: string; n: number }> => [
    { label: "identified", n: props.stats.identified },
    { label: "covers", n: props.stats.covered },
    { label: "metadata", n: props.stats.metadataed },
  ];
  return (
    <HubCard
      system={props.systemId}
      title={theme()?.displayName ?? props.systemId}
      subtitle={`${props.stats.total} game${props.stats.total === 1 ? "" : "s"}`}
      onActivate={props.onPick}
    >
      <ul class="flex flex-col gap-1 text-[0.7rem]">
        <For each={rows()}>
          {(r) => {
            const st = () => rowState(r.n, props.stats.total);
            return (
              <li class="flex items-center justify-between gap-2">
                <span class="flex items-center gap-1.5 text-(--color-oa-ink-dim)">
                  <span class={`${STATE_CLASS[st()]} w-3 text-center`}>{STATE_GLYPH[st()]}</span>
                  {r.label}
                </span>
                <span class="tabular-nums text-(--color-oa-ink-dim)">
                  {r.n}/{props.stats.total}
                </span>
              </li>
            );
          }}
        </For>
      </ul>
    </HubCard>
  );
};
