// Level-2 card: one per-system DOMAIN (Display & Video, Input, Core/Launcher,
// Media, Metadata, BIOS). Clicking opens that domain's editor. Disabled domains
// (not yet wired this slice) render "Coming soon".

import { Show, type Component } from "solid-js";
import type { SystemId } from "@oa/platform/themes/registry";
import { HubCard } from "./HubCard";
import type { DomainDef } from "./domains";

export const DomainCard: Component<{
  domain: DomainDef;
  system: SystemId;
  onPick: () => void;
}> = (props) => (
  <HubCard
    system={props.system}
    title={`${props.domain.glyph}  ${props.domain.label}`}
    subtitle={props.domain.blurb}
    disabled={!props.domain.enabled}
    onActivate={props.onPick}
  >
    <Show when={!props.domain.enabled}>
      <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)/70">
        Coming soon
      </span>
    </Show>
  </HubCard>
);
