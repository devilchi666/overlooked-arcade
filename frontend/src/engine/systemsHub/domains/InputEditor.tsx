// Input domain editor — launches the existing per-system Bindings + Core-options
// editors (kept as dialogs; large enough to warrant their own surface, and they
// already route through the spatial layer when the engine is active). Persistence
// unchanged.

import { createSignal, type Accessor, type Component } from "solid-js";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import { SystemBindingsDialog, SystemCoreOptionsDialog } from "../../SystemDialogs";
import { HubSection, PanelScaffold } from "../PanelScaffold";

const LaunchButton: Component<{ label: string; hint: string; onClick: () => void }> = (props) => (
  <button
    type="button"
    onClick={(e) => {
      e.currentTarget.blur();
      props.onClick();
    }}
    class="flex min-w-[14rem] flex-col items-start gap-1 rounded-md border border-white/10 bg-white/[0.04] px-3 py-2 text-left transition hover:border-(--color-system-accent)/50 hover:bg-white/[0.08] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
  >
    <span class="text-sm font-medium text-(--color-oa-ink)">{props.label}</span>
    <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">{props.hint}</span>
  </button>
);

export const InputEditor: Component<{ systemId: Accessor<SystemId> }> = (props) => {
  const [bindingsOpen, setBindingsOpen] = createSignal(false);
  const [coreOptionsOpen, setCoreOptionsOpen] = createSignal(false);
  return (
    <PanelScaffold
      system={props.systemId()}
      title={systemThemes[props.systemId()]?.displayName ?? props.systemId()}
      subtitle="Input · bindings + core options"
    >
      <HubSection title="Controls">
        <div class="flex flex-wrap gap-2">
          <LaunchButton
            label="Edit bindings…"
            hint="Keyboard + gamepad mappings for this system"
            onClick={() => setBindingsOpen(true)}
          />
          <LaunchButton
            label="Core options…"
            hint="Libretro core-level toggles (region, expansions, …)"
            onClick={() => setCoreOptionsOpen(true)}
          />
        </div>
      </HubSection>

      <SystemBindingsDialog
        open={bindingsOpen()}
        systemId={props.systemId()}
        onClose={() => setBindingsOpen(false)}
      />
      <SystemCoreOptionsDialog
        open={coreOptionsOpen()}
        systemId={props.systemId()}
        onClose={() => setCoreOptionsOpen(false)}
      />
    </PanelScaffold>
  );
};
