// Per-system settings dialogs launched from the System ▾ menu and
// SystemContextMenu. Wraps the shared per-system section components
// (lifted into perSystemSections.tsx so the Retroverse SETTINGS →
// Per-system surface can render the same fields).
//
// `SystemSettingsDialog` selects one section at a time via the
// `section` prop — small forms in a focused modal. The Retroverse
// per-system page shows all sections at once on a long scroll surface
// (no section prop). Both consume the same section components.

import { createResource, Show, type Component } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { Dialog } from "../layout/Dialog";
import type {
  CoreEntry,
  MonitorInfo,
  SettingsStore,
} from "../settings/store";
import { systemThemes, type SystemId } from "../themes/registry";
import SystemBindingsEditor from "./SystemBindingsEditor";
import CoreOptionsPanel from "./CoreOptionsPanel";
import {
  PerSystemDefaultCoreSection,
  PerSystemDisplaySection,
  PerSystemRewindSection,
  PerSystemShadersSection,
  usePerSystemOverrides,
} from "./perSystemSections";

export type SystemDialogSection =
  | "bindings"
  | "display"
  | "rewind"
  | "shaders"
  | "default-core"
  | "core-options";

// Re-exports for callers that still imported these helpers from this
// module before the perSystemSections lift. New code should import
// directly from perSystemSections.tsx.
export {
  BezelPicker,
  OverscanEditor,
  RewindLiveStats,
  overscanIsZero,
  overscanLabel,
  pathBasename,
  type OverscanCropPrefs,
} from "./perSystemSections";

// --- Combined dialog for display / rewind / shaders / default-core ----

type SystemSettingsDialogProps = {
  open: boolean;
  section: SystemDialogSection | null;
  systemId: SystemId;
  onClose: () => void;
  /// OA-wide settings store — used to display inherited values for chip
  /// rendering and as the fallback when an override is cleared.
  settings: SettingsStore;
};

const SECTION_TITLES: Record<SystemDialogSection, string> = {
  bindings: "Bindings",
  display: "Display overrides",
  rewind: "Rewind overrides",
  shaders: "Shaders",
  "default-core": "Default core",
  "core-options": "Core options",
};

export const SystemSettingsDialog: Component<SystemSettingsDialogProps> = (props) => {
  const api = usePerSystemOverrides({
    systemId: () => props.systemId,
    active: () => props.open,
  });

  // Monitor + cores lists for the Display + Default-core sections.
  // Lazy-fetched — only when the matching section opens.
  const [monitors] = createResource(
    () => props.open && props.section === "display",
    async (cond): Promise<MonitorInfo[]> => {
      if (!cond) return [];
      try {
        return await invoke<MonitorInfo[]>("list_monitors");
      } catch {
        return [];
      }
    },
  );
  const [cores] = createResource(
    () => props.open && props.section === "default-core",
    async (cond): Promise<CoreEntry[]> => {
      if (!cond) return [];
      try {
        return await invoke<CoreEntry[]>("list_cores");
      } catch {
        return [];
      }
    },
  );

  const theme = () => systemThemes[props.systemId];
  const subtitle = (): string => theme()?.displayName ?? props.systemId;

  return (
    <Dialog
      open={props.open}
      onClose={props.onClose}
      title={props.section ? SECTION_TITLES[props.section] : ""}
      subtitle={subtitle()}
      system={props.systemId}
      size="md"
    >
      <Show when={props.section === "display"}>
        <PerSystemDisplaySection
          api={api}
          settings={props.settings}
          monitors={() => monitors() ?? []}
        />
      </Show>

      <Show when={props.section === "rewind"}>
        <PerSystemRewindSection
          api={api}
          settings={props.settings}
          liveStatsActive={() => props.open && props.section === "rewind"}
        />
      </Show>

      <Show when={props.section === "shaders"}>
        <PerSystemShadersSection
          api={api}
          settings={props.settings}
        />
      </Show>

      <Show when={props.section === "default-core"}>
        <PerSystemDefaultCoreSection
          systemId={() => props.systemId}
          active={() => props.open && props.section === "default-core"}
          cores={() => cores() ?? []}
        />
      </Show>
    </Dialog>
  );
};

// --- Bindings dialog (wraps SystemBindingsEditor) ---------------------

export const SystemBindingsDialog: Component<{
  open: boolean;
  systemId: SystemId;
  onClose: () => void;
}> = (props) => (
  <Dialog
    open={props.open}
    onClose={props.onClose}
    title="Bindings"
    subtitle={systemThemes[props.systemId]?.displayName ?? props.systemId}
    system={props.systemId}
    size="lg"
  >
    <SystemBindingsEditor systemId={props.systemId} />
  </Dialog>
);

// --- Core options dialog (wraps CoreOptionsPanel) ---------------------

export const SystemCoreOptionsDialog: Component<{
  open: boolean;
  systemId: SystemId;
  onClose: () => void;
}> = (props) => (
  <Dialog
    open={props.open}
    onClose={props.onClose}
    title="Core options"
    subtitle={systemThemes[props.systemId]?.displayName ?? props.systemId}
    system={props.systemId}
    size="lg"
  >
    <CoreOptionsPanel systemId={props.systemId} gameId={null} />
  </Dialog>
);
