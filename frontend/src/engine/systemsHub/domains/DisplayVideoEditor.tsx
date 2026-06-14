// Display & Video domain editor — reuses the shared per-system override sections
// (Display / Rewind / Shaders) + the usePerSystemOverrides hook verbatim. In-pane
// (no dialog). Persistence is unchanged: getSystemSettings / setSystemSettings.

import { createResource, type Accessor, type Component } from "solid-js";
import { listMonitors } from "@oa/platform/api/settingsApi";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import {
  PerSystemDisplaySection,
  PerSystemRewindSection,
  PerSystemShadersSection,
  usePerSystemOverrides,
} from "@oa/platform/components/perSystemSections";
import type { MonitorInfo, SettingsStore } from "@oa/platform/settings/store";
import { HubSection, PanelScaffold } from "../PanelScaffold";

export const DisplayVideoEditor: Component<{
  systemId: Accessor<SystemId>;
  settings: SettingsStore;
}> = (props) => {
  const active = () => true;
  const api = usePerSystemOverrides({ systemId: props.systemId, active });
  const [monitors] = createResource(async (): Promise<MonitorInfo[]> => {
    try {
      return await listMonitors();
    } catch {
      return [];
    }
  });

  return (
    <PanelScaffold
      system={props.systemId()}
      title={systemThemes[props.systemId()]?.displayName ?? props.systemId()}
      subtitle="Display & Video · overrides fall back to OA-wide defaults"
    >
      <HubSection title="Display">
        <PerSystemDisplaySection api={api} settings={props.settings} monitors={() => monitors() ?? []} />
      </HubSection>
      <HubSection title="Rewind">
        <PerSystemRewindSection api={api} settings={props.settings} liveStatsActive={active} />
      </HubSection>
      <HubSection title="Shaders">
        <PerSystemShadersSection api={api} settings={props.settings} />
      </HubSection>
    </PanelScaffold>
  );
};
