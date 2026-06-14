// Core / Launcher domain editor — reuses PerSystemDefaultCoreSection + the
// per-system launcher dropdown (lifted from PerSystemSettingsBody). In-pane.
// Persistence unchanged: setCorePref / get+setLauncherPref.

import {
  createResource,
  createSignal,
  For,
  Show,
  type Accessor,
  type Component,
} from "solid-js";
import * as coresApi from "@oa/platform/api/coresApi";
import * as emulatorApi from "@oa/platform/api/emulatorApi";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import { PerSystemDefaultCoreSection } from "@oa/platform/components/perSystemSections";
import type { CoreEntry } from "@oa/platform/settings/store";
import { HubSection, PanelScaffold } from "../PanelScaffold";

// Subset of Rust EmulatorProfileInfo the launcher dropdown needs.
type EmulatorProfileInfo = {
  id: string;
  displayName: string;
  binaryName: string;
  supportedSystems: string[];
  binaryPath: string | null;
};

export const CoreLauncherEditor: Component<{ systemId: Accessor<SystemId> }> = (props) => {
  const active = () => true;
  const [cores] = createResource(async (): Promise<CoreEntry[]> => {
    try {
      return await coresApi.listCores();
    } catch {
      return [];
    }
  });
  const [launcherProfiles] = createResource(async (): Promise<EmulatorProfileInfo[]> => {
    try {
      return await emulatorApi.listEmulatorProfiles<EmulatorProfileInfo>();
    } catch {
      return [];
    }
  });
  const [launcherTick, setLauncherTick] = createSignal(0);
  const [launcherPref] = createResource(
    () => [props.systemId(), launcherTick()] as const,
    async ([sysId]): Promise<string | null> => {
      try {
        return (await emulatorApi.getLauncherPref(sysId)) ?? null;
      } catch {
        return null;
      }
    },
  );
  const supporting = (): EmulatorProfileInfo[] =>
    (launcherProfiles() ?? []).filter((p) => p.supportedSystems.includes(props.systemId()));
  async function setLauncher(profileId: string | null): Promise<void> {
    try {
      await emulatorApi.setLauncherPref(props.systemId(), profileId);
      setLauncherTick((n) => n + 1);
    } catch (e) {
      console.warn("[per-system-hub] set_launcher_pref failed:", e);
    }
  }

  return (
    <PanelScaffold
      system={props.systemId()}
      title={systemThemes[props.systemId()]?.displayName ?? props.systemId()}
      subtitle="Core / Launcher"
    >
      <HubSection title="Default core">
        <PerSystemDefaultCoreSection systemId={props.systemId} active={active} cores={() => cores() ?? []} />
      </HubSection>

      <Show when={supporting().length > 0}>
        <HubSection title="Launcher">
          <div class="flex flex-col gap-2">
            <label data-setting-row tabindex="-1" class="flex flex-wrap items-center gap-3">
              <span class="text-sm text-(--color-oa-ink)">Default launcher</span>
              <select
                class="rounded border border-white/10 bg-black/40 px-2 py-1 text-sm text-(--color-oa-ink)"
                value={launcherPref() ?? ""}
                onChange={(e) => void setLauncher(e.currentTarget.value || null)}
              >
                <option value="">Libretro core (default)</option>
                <For each={supporting()}>
                  {(p) => <option value={p.id}>{p.displayName} (standalone)</option>}
                </For>
              </select>
            </label>
            <For each={supporting()}>
              {(p) => (
                <Show when={launcherPref() === p.id && !p.binaryPath}>
                  <span class="text-[0.7rem] text-amber-300">
                    {p.displayName}'s binary path isn't set — launches will fail until you point OA
                    at your install in Settings → Cores → External emulators.
                  </span>
                </Show>
              )}
            </For>
            <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Standalone emulators manage their own video / saves / input — the core card applies
              only to the libretro core.
            </p>
          </div>
        </HubSection>
      </Show>
    </PanelScaffold>
  );
};
