// Game Metadata domain editor — per-game facts (year / developer / genre / …)
// for the games belonging to THIS system, via MetadataGamePane locked to the
// system (no all-systems picker). Distinct from Platform Metadata (console
// facts). In-pane; optimistic autosave. Persistence unchanged
// (game_metadata_overrides). This replaces the old standalone "Game metadata"
// Settings category — per-game facts now live under their system.

import { createResource, createSignal, type Accessor, type Component } from "solid-js";
import { listGameGroups } from "@oa/platform/api/libraryApi";
import type { GameGroupInfo } from "@oa/platform/library/types";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import MetadataGamePane from "../../MetadataGamePane";
import { PanelScaffold } from "../PanelScaffold";

export const GameMetadataEditor: Component<{ systemId: Accessor<SystemId> }> = (props) => {
  const [groups] = createResource(async (): Promise<GameGroupInfo[]> => {
    try {
      return await listGameGroups();
    } catch (e) {
      console.warn("[per-system-hub] list_game_groups failed:", e);
      return [];
    }
  });
  const [previewOpen] = createSignal(true);
  const systems = () => [
    {
      id: props.systemId(),
      displayName: systemThemes[props.systemId()]?.displayName ?? props.systemId(),
    },
  ];

  return (
    <PanelScaffold
      system={props.systemId()}
      title={systemThemes[props.systemId()]?.displayName ?? props.systemId()}
      subtitle="Game metadata · per-game facts for this system"
      fill
    >
      <MetadataGamePane
        previewOpen={previewOpen}
        groups={groups() ?? []}
        systems={systems()}
        lockedSystemId={props.systemId()}
      />
    </PanelScaffold>
  );
};
