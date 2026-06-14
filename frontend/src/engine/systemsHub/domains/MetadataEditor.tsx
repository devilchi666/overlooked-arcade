// Metadata domain editor — per-system facts (manufacturer / specs / hero copy /
// peripherals) via the shared SystemMetaForm, with the live-preview aside.
// In-pane; optimistic autosave (no save button). Per-GAME metadata stays in its
// own surface (MetadataGamePane) — not here. Persistence unchanged.

import { type Accessor, type Component } from "solid-js";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import { SystemMetaForm } from "../../SystemMetaForm";
import { PanelScaffold } from "../PanelScaffold";

export const MetadataEditor: Component<{ systemId: Accessor<SystemId> }> = (props) => (
  <PanelScaffold
    system={props.systemId()}
    title={systemThemes[props.systemId()]?.displayName ?? props.systemId()}
    subtitle="Metadata · system facts · edits save automatically"
  >
    <SystemMetaForm systemId={props.systemId} showPreview={() => true} />
  </PanelScaffold>
);
