// Per-system settings dialogs launched from the System ▾ menu and
// SystemContextMenu. Replaces the routed PerSystemSettingsPage with
// focused single-purpose dialogs.
//
// `SystemSettingsDialog` handles four sections (display / rewind /
// shaders / default-core) — each is a small form mutating the per-system
// override JSON. The Bindings + Core options surfaces are full enough
// editors that they get their own dialog wrappers around the existing
// `SystemBindingsEditor` and `CoreOptionsPanel` components.

import {
  createEffect,
  createResource,
  createSignal,
  For,
  Show,
  type Component,
} from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { Dialog } from "../layout/Dialog";
import {
  SCALING_MODE_LABELS,
  SCALING_OPTIONS,
  WINDOW_MODE_LABELS,
  WINDOW_OPTIONS,
  type CoreEntry,
  type MonitorInfo,
  type ScalingMode,
  type SettingsStore,
  type WindowMode,
} from "../settings/store";
import { shaderPresets, shaderPresetLabel } from "../settings/shader_presets";
import { systemThemes, type SystemId } from "../themes/registry";
import SystemBindingsEditor from "./SystemBindingsEditor";
import CoreOptionsPanel from "./CoreOptionsPanel";
import SettingRow from "./SettingRow";

export type SystemDialogSection =
  | "bindings"
  | "display"
  | "rewind"
  | "shaders"
  | "default-core"
  | "core-options";

type SystemSettings = {
  scalingOverride?: string | null;
  windowModeOverride?: string | null;
  monitorIndexOverride?: number | null;
  shaderPreset?: string | null;
  bloomAmount?: number | null;
  rewindEnabled?: boolean | null;
  rewindCaptureIntervalFrames?: number | null;
  rewindBufferMegabytes?: number | null;
};

const SELECT_CLASS =
  "w-full rounded-md border border-white/10 bg-white/[0.04] px-3 py-2 text-sm text-(--color-oa-ink) transition hover:bg-white/[0.08] focus-visible:outline focus-visible:outline-2 focus-visible:outline-(--color-system-accent) disabled:opacity-50";

const REWIND_INTERVAL_OPTIONS: readonly number[] = [1, 2, 3, 6, 10, 15, 30];
const REWIND_BUFFER_OPTIONS: readonly number[] = [8, 16, 32, 64, 128, 256, 512];

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
  // Resource for the per-system overrides — fires whenever the source
  // tuple changes. The `_` token forces refetch when we call refetch()
  // ourselves (currently only on open).
  const sysSource = () => ({
    sysId: props.systemId,
    section: props.section,
    open: props.open,
  });
  const [overrides, { refetch: refetchOverrides }] = createResource(
    sysSource,
    async (src): Promise<SystemSettings> => {
      if (!src.open || !src.sysId) return {};
      try {
        return (await invoke<SystemSettings>("get_system_settings", { systemId: src.sysId })) ?? {};
      } catch (e) {
        console.warn("[oa-system-dialog] get_system_settings failed:", e);
        return {};
      }
    },
  );

  // Default core pref state (separate persistence — appDataDir/cores.json).
  const [corePref, setCorePref] = createSignal<string | null>(null);
  createEffect(() => {
    if (!props.open || props.section !== "default-core") return;
    void invoke<string | null>("get_core_pref", { systemId: props.systemId })
      .then((v) => setCorePref(v ?? null))
      .catch((e) => console.warn("get_core_pref failed:", e));
  });

  // Monitor + cores lists for the Display + Default-core sections. Cheap
  // — fetched only when the matching section opens.
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

  /// Write-through helper. Strips null/undefined entries to keep the
  /// on-disk file minimal — the launch path treats missing keys as
  /// "inherit OA default."
  async function patch(p: Partial<SystemSettings>) {
    const next: SystemSettings = { ...(overrides() ?? {}), ...p };
    const cleaned: SystemSettings = {};
    if (next.scalingOverride != null) cleaned.scalingOverride = next.scalingOverride;
    if (next.windowModeOverride != null) cleaned.windowModeOverride = next.windowModeOverride;
    if (next.monitorIndexOverride != null) cleaned.monitorIndexOverride = next.monitorIndexOverride;
    if (next.shaderPreset != null) cleaned.shaderPreset = next.shaderPreset;
    if (next.bloomAmount != null) cleaned.bloomAmount = next.bloomAmount;
    if (next.rewindEnabled != null) cleaned.rewindEnabled = next.rewindEnabled;
    if (next.rewindCaptureIntervalFrames != null) cleaned.rewindCaptureIntervalFrames = next.rewindCaptureIntervalFrames;
    if (next.rewindBufferMegabytes != null) cleaned.rewindBufferMegabytes = next.rewindBufferMegabytes;
    try {
      await invoke("set_system_settings", { systemId: props.systemId, settings: cleaned });
      void refetchOverrides();
    } catch (e) {
      console.warn("[oa-system-dialog] set_system_settings failed:", e);
    }
  }

  async function patchCorePref(fileName: string | null) {
    setCorePref(fileName);
    try {
      await invoke("set_core_pref", { systemId: props.systemId, fileName });
    } catch (e) {
      console.warn("set_core_pref failed:", e);
    }
  }

  const theme = () => systemThemes[props.systemId];
  const subtitle = (): string => theme()?.displayName ?? props.systemId;

  const inheritedScalingLabel = (): string =>
    SCALING_MODE_LABELS[props.settings.scalingMode()] ?? props.settings.scalingMode();
  const inheritedWindowLabel = (): string =>
    WINDOW_MODE_LABELS[props.settings.windowMode()] ?? props.settings.windowMode();
  const inheritedMonitorLabel = (): string => {
    const idx = props.settings.monitorIndex();
    if (idx === null) return "Current monitor";
    const m = (monitors() ?? []).find((mm) => mm.index === idx);
    if (!m) return `Monitor ${idx + 1}`;
    return m.name?.trim() || `Monitor ${idx + 1}`;
  };
  const inheritedCoreLabel = (): string => {
    const list = cores() ?? [];
    if (list.length === 0) return "Auto-detect";
    return list[0].libraryName || list[0].fileName;
  };

  return (
    <Dialog
      open={props.open}
      onClose={props.onClose}
      title={props.section ? SECTION_TITLES[props.section] : ""}
      subtitle={subtitle()}
      system={props.systemId}
      size="md"
    >
      {/* --- Display ------------------------------------------------ */}
      <Show when={props.section === "display"}>
        <div class="flex flex-col gap-3">
          <SettingRow
            label="Scaling mode"
            hint="How the framebuffer fills the window"
            inheritedValue={inheritedScalingLabel()}
            overridden={overrides()?.scalingOverride != null}
          >
            <select
              class={SELECT_CLASS}
              value={overrides()?.scalingOverride ?? ""}
              onChange={(e) => {
                const v = e.currentTarget.value;
                void patch({ scalingOverride: v === "" ? null : v });
              }}
            >
              <option value="">— Use OA default —</option>
              <For each={SCALING_OPTIONS}>
                {(m) => <option value={m}>{SCALING_MODE_LABELS[m as ScalingMode]}</option>}
              </For>
            </select>
          </SettingRow>

          <SettingRow
            label="Window mode"
            hint="Per-system window state at launch"
            inheritedValue={inheritedWindowLabel()}
            overridden={overrides()?.windowModeOverride != null}
          >
            <select
              class={SELECT_CLASS}
              value={overrides()?.windowModeOverride ?? ""}
              onChange={(e) => {
                const v = e.currentTarget.value;
                void patch({ windowModeOverride: v === "" ? null : v });
              }}
            >
              <option value="">— Use OA default —</option>
              <For each={WINDOW_OPTIONS}>
                {(m) => <option value={m}>{WINDOW_MODE_LABELS[m as WindowMode]}</option>}
              </For>
            </select>
          </SettingRow>

          <SettingRow
            label="Monitor"
            hint="0-indexed; matches the OA Settings → Display picker"
            inheritedValue={inheritedMonitorLabel()}
            overridden={overrides()?.monitorIndexOverride != null}
          >
            <select
              class={SELECT_CLASS}
              value={overrides()?.monitorIndexOverride?.toString() ?? ""}
              onChange={(e) => {
                const v = e.currentTarget.value;
                void patch({ monitorIndexOverride: v === "" ? null : Number(v) });
              }}
            >
              <option value="">— Use OA default —</option>
              <For each={monitors() ?? []}>
                {(m) => (
                  <option value={m.index.toString()}>
                    {(m.name?.trim() || `Monitor ${m.index + 1}`) + ` (${m.width}×${m.height})`}
                  </option>
                )}
              </For>
            </select>
          </SettingRow>
          <p class="text-[0.6rem] uppercase tracking-widest text-amber-300/80">
            Scaffold — persists today but runtime effect lands in Phase 3.
          </p>
        </div>
      </Show>

      {/* --- Rewind ------------------------------------------------- */}
      <Show when={props.section === "rewind"}>
        <div class="flex flex-col gap-3">
          <SettingRow
            label="Enable rewind"
            hint="Hold Backspace during gameplay to step backwards"
            inheritedValue={props.settings.rewindEnabled() ? "On" : "Off"}
            overridden={overrides()?.rewindEnabled != null}
          >
            <select
              class={SELECT_CLASS}
              value={
                overrides()?.rewindEnabled == null
                  ? ""
                  : overrides()!.rewindEnabled
                  ? "on"
                  : "off"
              }
              onChange={(e) => {
                const v = e.currentTarget.value;
                void patch({ rewindEnabled: v === "" ? null : v === "on" });
              }}
            >
              <option value="">— Use OA default —</option>
              <option value="on">On</option>
              <option value="off">Off</option>
            </select>
          </SettingRow>

          <SettingRow
            label="Capture interval"
            hint="Frames between snapshots"
            inheritedValue={`Every ${props.settings.rewindCaptureIntervalFrames()} frames`}
            overridden={overrides()?.rewindCaptureIntervalFrames != null}
          >
            <select
              class={SELECT_CLASS}
              value={overrides()?.rewindCaptureIntervalFrames?.toString() ?? ""}
              onChange={(e) => {
                const v = e.currentTarget.value;
                void patch({ rewindCaptureIntervalFrames: v === "" ? null : Number(v) });
              }}
            >
              <option value="">— Use OA default —</option>
              <For each={REWIND_INTERVAL_OPTIONS}>
                {(n) => (
                  <option value={String(n)}>
                    {n === 1 ? "Every frame" : `Every ${n} frames`}
                  </option>
                )}
              </For>
            </select>
          </SettingRow>

          <SettingRow
            label="Buffer cap"
            hint="Hard memory ceiling for the rewind ring"
            inheritedValue={`${props.settings.rewindBufferMegabytes()} MB`}
            overridden={overrides()?.rewindBufferMegabytes != null}
          >
            <select
              class={SELECT_CLASS}
              value={overrides()?.rewindBufferMegabytes?.toString() ?? ""}
              onChange={(e) => {
                const v = e.currentTarget.value;
                void patch({ rewindBufferMegabytes: v === "" ? null : Number(v) });
              }}
            >
              <option value="">— Use OA default —</option>
              <For each={REWIND_BUFFER_OPTIONS}>
                {(mb) => <option value={String(mb)}>{mb} MB</option>}
              </For>
            </select>
          </SettingRow>
        </div>
      </Show>

      {/* --- Shaders ------------------------------------------------ */}
      <Show when={props.section === "shaders"}>
        <div class="flex flex-col gap-3">
          <SettingRow
            label="Shader preset"
            hint="Applies during the final blit. Takes effect on next launch."
            inheritedValue={shaderPresetLabel(props.settings.shaderPreset())}
            overridden={overrides()?.shaderPreset != null}
          >
            <select
              class={SELECT_CLASS}
              value={overrides()?.shaderPreset ?? ""}
              onChange={(e) => {
                const v = e.currentTarget.value;
                void patch({ shaderPreset: v === "" ? null : v });
              }}
            >
              <option value="">— Use OA default —</option>
              <For each={shaderPresets()}>
                {(p) => <option value={p.name}>{p.displayName}</option>}
              </For>
            </select>
          </SettingRow>

          <SettingRow
            label="Bloom amount (Phosphor only)"
            hint="Overrides the Phosphor preset's bloom weight. 0 = pure source, 1 = pure blur."
            inheritedValue="Preset default"
            overridden={overrides()?.bloomAmount != null}
          >
            <div class="flex items-center gap-3">
              <input
                type="range"
                min="0"
                max="1"
                step="0.05"
                value={overrides()?.bloomAmount ?? 0.6}
                onInput={(e) => {
                  const v = Number(e.currentTarget.value);
                  if (!Number.isFinite(v)) return;
                  void patch({ bloomAmount: v });
                  void invoke("set_bloom_amount", { amount: v }).catch(() => {});
                }}
                class="flex-1"
              />
              <span class="font-mono text-sm w-12 text-right tabular-nums">
                {(overrides()?.bloomAmount ?? 0.6).toFixed(2)}
              </span>
              <Show when={overrides()?.bloomAmount != null}>
                <button
                  type="button"
                  onClick={() => void patch({ bloomAmount: null })}
                  class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-xs text-(--color-oa-ink) hover:bg-white/[0.08]"
                >
                  Reset
                </button>
              </Show>
            </div>
          </SettingRow>
        </div>
      </Show>

      {/* --- Default core ------------------------------------------- */}
      <Show when={props.section === "default-core"}>
        <div class="flex flex-col gap-3">
          <SettingRow
            label="Default core"
            hint="Used when a game has no per-game core override"
            inheritedValue={inheritedCoreLabel()}
            inheritedFrom="Auto-detect"
            overridden={corePref() !== null}
          >
            <select
              class={SELECT_CLASS}
              value={corePref() ?? ""}
              onChange={(e) => {
                const v = e.currentTarget.value;
                void patchCorePref(v === "" ? null : v);
              }}
            >
              <option value="">— Use auto-detect —</option>
              <For each={cores() ?? []}>
                {(c) => (
                  <option value={c.fileName}>
                    {c.libraryName} ({c.libraryVersion})
                  </option>
                )}
              </For>
            </select>
          </SettingRow>
          <p class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            Per-game overrides (right-click a tile → Change core…) take precedence.
          </p>
        </div>
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
