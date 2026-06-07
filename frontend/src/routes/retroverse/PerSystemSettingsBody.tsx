// Retroverse SETTINGS → Per-system center pane body.
//
// Renders all four override sections (Display / Rewind / Shaders /
// Default core) as inline cards alongside two launchers for the
// bigger editors (Bindings + Core options) which keep their existing
// dialog UI. The active system id is owned by the parent SettingsPage
// so swapping systems doesn't unmount the whole body — section-level
// resources (overrides, monitors, cores) re-fetch via the hook's
// systemId-tracked source.

import { createResource, createSignal, Show, type Accessor, type Component } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import {
  PerSystemDefaultCoreSection,
  PerSystemDisplaySection,
  PerSystemRewindSection,
  PerSystemShadersSection,
  usePerSystemOverrides,
} from "../../components/perSystemSections";
import {
  SystemBindingsDialog,
  SystemCoreOptionsDialog,
} from "../../components/SystemDialogs";
import PerSystemInfoSection from "./PerSystemInfoSection";
import type { CoreEntry, MonitorInfo, SettingsStore } from "@oa/platform/settings/store";

type Props = {
  /// Currently active system id; null until the operator picks one.
  /// Reactive so swapping systems from the sidebar updates the body
  /// in place rather than remounting.
  systemId: Accessor<SystemId | null>;
  settings: SettingsStore;
};

const PerSystemSettingsBody: Component<Props> = (props) => {
  const theme = () => {
    const id = props.systemId();
    return id ? systemThemes[id] : null;
  };

  const active = () => props.systemId() !== null;
  const api = usePerSystemOverrides({
    systemId: props.systemId,
    active,
  });

  // Both lists are needed across all 4 sections on this page, so we
  // eagerly fetch them once a system is picked. Re-fetch is cheap and
  // the cache lasts the page session.
  const [monitors] = createResource(active, async (on): Promise<MonitorInfo[]> => {
    if (!on) return [];
    try {
      return await invoke<MonitorInfo[]>("list_monitors");
    } catch {
      return [];
    }
  });
  const [cores] = createResource(active, async (on): Promise<CoreEntry[]> => {
    if (!on) return [];
    try {
      return await invoke<CoreEntry[]>("list_cores");
    } catch {
      return [];
    }
  });

  const [bindingsOpen, setBindingsOpen] = createSignal(false);
  const [coreOptionsOpen, setCoreOptionsOpen] = createSignal(false);

  return (
    <Show
      when={props.systemId()}
      fallback={
        <div class="flex h-full items-center justify-center p-12">
          <div class="max-w-md rounded-xl border border-dashed border-white/10 bg-white/[0.02] p-8 text-center">
            <p class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
              No system picked
            </p>
            <p class="mt-3 text-sm text-(--color-oa-ink-dim)">
              Pick a system from the sidebar to view + edit its overrides.
              Per-system values override OA-wide defaults; per-game settings
              override these in turn.
            </p>
          </div>
        </div>
      }
    >
      {(sysId) => (
        <div class="flex flex-col gap-6" data-system={sysId()}>
          <header class="flex items-center gap-3 rounded-xl border border-white/5 bg-white/[0.02] px-4 py-3">
            <span
              class="inline-block h-3 w-3 shrink-0 rounded-full bg-(--color-system-accent)"
              aria-hidden="true"
            />
            <div class="min-w-0 flex-1">
              <p class="truncate text-base font-semibold text-(--color-oa-ink)">
                {theme()?.displayName ?? sysId()}
              </p>
              <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                Per-system overrides · falls back to OA-wide defaults when
                empty
              </p>
            </div>
          </header>

          {/* Display */}
          <Card title="Display">
            <PerSystemDisplaySection
              api={api}
              settings={props.settings}
              monitors={() => monitors() ?? []}
            />
          </Card>

          {/* Rewind */}
          <Card title="Rewind">
            <PerSystemRewindSection
              api={api}
              settings={props.settings}
              liveStatsActive={active}
            />
          </Card>

          {/* Shaders */}
          <Card title="Shaders">
            <PerSystemShadersSection
              api={api}
              settings={props.settings}
            />
          </Card>

          {/* Default core */}
          <Card title="Default core">
            <PerSystemDefaultCoreSection
              systemId={props.systemId}
              active={active}
              cores={() => cores() ?? []}
            />
          </Card>

          {/* System info — Phase 4 edit UI for the HOME tab's right
              pane fields. Each row inherits from L2 (curated YAML) +
              L1 (MAME) by default; typing in a row promotes the value
              to an L3 operator override. "Reset all overrides for
              this system" drops the entire L3 row. */}
          <Card title="System info">
            <PerSystemInfoSection systemId={props.systemId} />
          </Card>

          {/* Bindings + Core options stay as dialog launchers — those
              editors are big enough that embedding them inline would
              overwhelm this surface. */}
          <Card title="More">
            <div class="flex flex-wrap gap-2">
              <LauncherButton
                label="Edit bindings…"
                hint="Keyboard + gamepad mappings for this system"
                onClick={() => setBindingsOpen(true)}
              />
              <LauncherButton
                label="Core options…"
                hint="Libretro core-level toggles (region, expansions, …)"
                onClick={() => setCoreOptionsOpen(true)}
              />
            </div>
          </Card>

          <SystemBindingsDialog
            open={bindingsOpen()}
            systemId={sysId()}
            onClose={() => setBindingsOpen(false)}
          />
          <SystemCoreOptionsDialog
            open={coreOptionsOpen()}
            systemId={sysId()}
            onClose={() => setCoreOptionsOpen(false)}
          />
        </div>
      )}
    </Show>
  );
};

const Card: Component<{ title: string; children: import("solid-js").JSX.Element }> = (props) => (
  <section class="rounded-xl border border-white/5 bg-white/[0.02] p-5">
    <h3 class="mb-4 text-[0.65rem] font-semibold uppercase tracking-[0.3em] text-(--color-system-accent)">
      {props.title}
    </h3>
    {props.children}
  </section>
);

const LauncherButton: Component<{
  label: string;
  hint: string;
  onClick: () => void;
}> = (props) => (
  <button
    type="button"
    onClick={(e) => {
      e.currentTarget.blur();
      props.onClick();
    }}
    class="flex min-w-[14rem] flex-col items-start gap-1 rounded-md border border-white/10 bg-white/[0.04] px-3 py-2 text-left transition hover:border-(--color-system-accent)/50 hover:bg-white/[0.08] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
  >
    <span class="text-sm font-medium text-(--color-oa-ink)">{props.label}</span>
    <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
      {props.hint}
    </span>
  </button>
);

export default PerSystemSettingsBody;
