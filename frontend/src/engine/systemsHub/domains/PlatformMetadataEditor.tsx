// Platform Metadata domain editor — facts about the CONSOLE itself (manufacturer
// / specs / hero copy / peripherals) via the shared SystemMetaForm, with the
// live preview. Distinct from Game Metadata (per-game facts), which is its own
// domain card. In-pane; optimistic autosave. Persistence unchanged.
//
// ("Platform" vs "System" terminology under review — see PARKING_LOT 2026-06-14.)

import { type Accessor, type Component } from "solid-js";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import { SystemMetaForm } from "../../SystemMetaForm";
import { PanelScaffold } from "../PanelScaffold";

export const PlatformMetadataEditor: Component<{ systemId: Accessor<SystemId> }> = (props) => (
  <PanelScaffold
    system={props.systemId()}
    title={systemThemes[props.systemId()]?.displayName ?? props.systemId()}
    subtitle="Platform metadata · console facts · edits save automatically"
  >
    <SystemMetaForm systemId={props.systemId} showPreview={() => true} />
  </PanelScaffold>
);
