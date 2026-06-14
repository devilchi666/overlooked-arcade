// Level-2 view: the grid of domain cards for one system. Picking a domain card
// opens that domain's editor (level 3).

import { For, type Component } from "solid-js";
import type { SystemId } from "@oa/platform/themes/registry";
import { HubGrid } from "./HubGrid";
import { DomainCard } from "./DomainCard";
import { DOMAINS, type DomainId } from "./domains";

export const SystemHubDetail: Component<{
  systemId: SystemId;
  onPickDomain: (d: DomainId) => void;
}> = (props) => (
  <HubGrid>
    <For each={DOMAINS}>
      {(d) => (
        <DomainCard domain={d} system={props.systemId} onPick={() => props.onPickDomain(d.id)} />
      )}
    </For>
  </HubGrid>
);
