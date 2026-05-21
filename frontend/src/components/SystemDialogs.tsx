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
  onCleanup,
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

export type OverscanCropPrefs = {
  top: number;
  bottom: number;
  left: number;
  right: number;
};

type SystemSettings = {
  scalingOverride?: string | null;
  windowModeOverride?: string | null;
  monitorIndexOverride?: number | null;
  shaderPreset?: string | null;
  bloomAmount?: number | null;
  rewindEnabled?: boolean | null;
  rewindCaptureIntervalFrames?: number | null;
  rewindBufferMegabytes?: number | null;
  displayAspectOverride?: number | null;
  overscanCropOverride?: OverscanCropPrefs | null;
  bezelImagePath?: string | null;
};

/// Truncate an absolute path to its leaf basename for compact chip
/// display in the SettingRow inheritance UI. `null` round-trips so the
/// caller can decide what "no value" means.
export function pathBasename(p: string | null | undefined): string | null {
  if (!p) return null;
  const idx = Math.max(p.lastIndexOf("/"), p.lastIndexOf("\\"));
  return idx >= 0 ? p.slice(idx + 1) : p;
}

/// True when every edge is zero — treated equivalent to "no crop set".
export function overscanIsZero(c: OverscanCropPrefs | null | undefined): boolean {
  if (!c) return true;
  return c.top === 0 && c.bottom === 0 && c.left === 0 && c.right === 0;
}

export function overscanLabel(c: OverscanCropPrefs | null | undefined): string {
  if (overscanIsZero(c)) return "No crop";
  return `T${c!.top} · B${c!.bottom} · L${c!.left} · R${c!.right}`;
}

/// Common display-aspect presets surfaced in the dropdown. Each value
/// is the ratio (width/height) the renderer applies; the label is what
/// the user sees. Pick "" to clear the override → fall back to the
/// core-reported aspect.
const DISPLAY_ASPECT_PRESETS: readonly { value: string; label: string }[] = [
  { value: "1.333", label: "4:3 (CRT TV)" },
  { value: "1.778", label: "16:9 (Widescreen)" },
  { value: "1.0",   label: "1:1 (Square pixels)" },
  { value: "1.143", label: "8:7 (NES authentic)" },
  { value: "1.185", label: "32:27 (PCE 256 authentic)" },
  { value: "1.306", label: "64:49 (PCE 352 authentic)" },
];

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
    if (next.displayAspectOverride != null) cleaned.displayAspectOverride = next.displayAspectOverride;
    if (next.overscanCropOverride != null && !overscanIsZero(next.overscanCropOverride)) {
      cleaned.overscanCropOverride = next.overscanCropOverride;
    }
    if (next.bezelImagePath != null && next.bezelImagePath !== "") {
      cleaned.bezelImagePath = next.bezelImagePath;
    }
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

          <SettingRow
            label="Display aspect"
            hint="Pixel-aspect at the renderer; affects Aspect-correct + Pixel-perfect modes"
            inheritedValue="Core-reported"
            overridden={overrides()?.displayAspectOverride != null}
          >
            <select
              class={SELECT_CLASS}
              value={overrides()?.displayAspectOverride?.toString() ?? ""}
              onChange={(e) => {
                const v = e.currentTarget.value;
                void patch({
                  displayAspectOverride: v === "" ? null : Number(v),
                });
              }}
            >
              <option value="">— Use core-reported —</option>
              <For each={DISPLAY_ASPECT_PRESETS}>
                {(p) => <option value={p.value}>{p.label}</option>}
              </For>
            </select>
          </SettingRow>

          <SettingRow
            label="Overscan crop"
            hint="Hide source pixels at each edge (top/bottom/left/right); the cropped region stretches to fill"
            inheritedValue="No crop"
            overridden={!overscanIsZero(overrides()?.overscanCropOverride)}
          >
            <OverscanEditor
              value={overrides()?.overscanCropOverride ?? { top: 0, bottom: 0, left: 0, right: 0 }}
              onChange={(next) => void patch({
                overscanCropOverride: overscanIsZero(next) ? null : next,
              })}
            />
          </SettingRow>

          <SettingRow
            label="Bezel image"
            hint="PNG / JPEG / WebP overlaid on top of the game pixels"
            inheritedValue="Use shader preset default"
            overridden={overrides()?.bezelImagePath != null}
          >
            <BezelPicker
              value={overrides()?.bezelImagePath ?? null}
              onChange={(path) => void patch({ bezelImagePath: path })}
            />
          </SettingRow>
        </div>
      </Show>

      {/* --- Rewind ------------------------------------------------- */}
      <Show when={props.section === "rewind"}>
        <div class="flex flex-col gap-3">
          <RewindLiveStats open={props.open && props.section === "rewind"} />
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

// --- Overscan crop editor ---------------------------------------------
//
// Four pixel-count inputs (T / B / L / R) in one row, sharing a small
// "reset to zero" button. Re-used by per-system + per-game Display tabs
// — exported so PerGameSettingsDrawer can import without duplicating
// the layout. Caller-controlled (value + onChange) so each context
// owns its own persistence.

/// Bezel-image file picker. Exported so the per-game drawer reuses
/// it. Wraps `@tauri-apps/plugin-dialog`'s native file picker. Empty
/// path = no override (inherit per-system / preset default).
export const BezelPicker: Component<{
  value: string | null;
  onChange: (next: string | null) => void;
}> = (props) => {
  async function pick() {
    try {
      const mod = await import("@tauri-apps/plugin-dialog");
      const picked = await mod.open({
        multiple: false,
        directory: false,
        filters: [{ name: "Image", extensions: ["png", "jpg", "jpeg", "webp"] }],
      });
      if (typeof picked === "string" && picked.length > 0) {
        props.onChange(picked);
      }
    } catch (e) {
      console.warn("[BezelPicker] pick failed:", e);
    }
  }
  return (
    <div class="flex flex-1 flex-wrap items-center gap-2 text-xs">
      <Show
        when={props.value}
        fallback={<span class="text-(--color-oa-ink-dim)">No override (inherit)</span>}
      >
        <span class="flex-1 truncate font-mono text-(--color-oa-ink)" title={props.value!}>
          {pathBasename(props.value)}
        </span>
      </Show>
      <button
        type="button"
        onClick={(e) => {
          e.currentTarget.blur();
          void pick();
        }}
        class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
      >
        Pick…
      </button>
      <Show when={props.value}>
        <button
          type="button"
          onClick={(e) => {
            e.currentTarget.blur();
            props.onChange(null);
          }}
          class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
        >
          Clear
        </button>
      </Show>
    </div>
  );
};

export const OverscanEditor: Component<{
  value: OverscanCropPrefs;
  onChange: (next: OverscanCropPrefs) => void;
}> = (props) => {
  const inputClass =
    "w-12 rounded border border-white/10 bg-white/[0.04] px-1.5 py-0.5 text-center text-xs text-(--color-oa-ink) focus-visible:outline focus-visible:outline-1 focus-visible:outline-(--color-system-accent)";

  function setEdge(key: keyof OverscanCropPrefs, raw: string) {
    const n = Math.max(0, Math.min(99, Number(raw) || 0));
    const next = { ...props.value, [key]: n };
    props.onChange(next);
  }

  return (
    <div class="flex flex-wrap items-center gap-2 text-xs">
      <label class="flex items-center gap-1">
        <span class="text-(--color-oa-ink-dim)">T</span>
        <input
          type="number"
          min={0}
          max={99}
          class={inputClass}
          value={props.value.top}
          onChange={(e) => setEdge("top", e.currentTarget.value)}
        />
      </label>
      <label class="flex items-center gap-1">
        <span class="text-(--color-oa-ink-dim)">B</span>
        <input
          type="number"
          min={0}
          max={99}
          class={inputClass}
          value={props.value.bottom}
          onChange={(e) => setEdge("bottom", e.currentTarget.value)}
        />
      </label>
      <label class="flex items-center gap-1">
        <span class="text-(--color-oa-ink-dim)">L</span>
        <input
          type="number"
          min={0}
          max={99}
          class={inputClass}
          value={props.value.left}
          onChange={(e) => setEdge("left", e.currentTarget.value)}
        />
      </label>
      <label class="flex items-center gap-1">
        <span class="text-(--color-oa-ink-dim)">R</span>
        <input
          type="number"
          min={0}
          max={99}
          class={inputClass}
          value={props.value.right}
          onChange={(e) => setEdge("right", e.currentTarget.value)}
        />
      </label>
      <Show when={!overscanIsZero(props.value)}>
        <button
          type="button"
          class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim) hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
          onClick={() => props.onChange({ top: 0, bottom: 0, left: 0, right: 0 })}
        >
          Reset
        </button>
      </Show>
    </div>
  );
};

// --- Rewind live stats display ----------------------------------------
//
// Reads `get_rewind_state` to show the operator how many snapshots /
// seconds / MB the ring is currently holding. Most useful while a game
// is running; degrades gracefully to a "no rewind activity" banner when
// the ring is empty (typical case — opening the rewind tab from the
// system header while not in a game). Polls at 2 Hz so the bytes
// counter ticks visibly when actively recording.

type RewindState = {
  enabled: boolean;
  snapshotCount: number;
  byteSize: number;
  captureIntervalFrames: number;
  fps: number;
  scrubbing: boolean;
  scrubPosition: number;
};

function formatBytes(b: number): string {
  if (b < 1024) return `${b} B`;
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
  return `${(b / (1024 * 1024)).toFixed(1)} MB`;
}

export const RewindLiveStats: Component<{ open: boolean }> = (props) => {
  const [state, setState] = createSignal<RewindState | null>(null);
  let timer: number | null = null;

  async function poll() {
    try {
      const s = await invoke<RewindState>("get_rewind_state");
      setState(s);
    } catch (e) {
      console.warn("[oa-rewind-stats] get_rewind_state failed:", e);
    }
  }

  createEffect(() => {
    if (props.open) {
      void poll();
      timer = window.setInterval(() => void poll(), 500);
    } else if (timer !== null) {
      clearInterval(timer);
      timer = null;
    }
  });
  onCleanup(() => {
    if (timer !== null) clearInterval(timer);
  });

  const secondsHeld = (): number | null => {
    const s = state();
    if (!s || s.snapshotCount === 0 || s.fps <= 0 || s.captureIntervalFrames === 0) return null;
    return (s.snapshotCount * s.captureIntervalFrames) / s.fps;
  };

  return (
    <Show when={state() !== null && state()!.snapshotCount > 0}
      fallback={
        <div class="rounded-md border border-white/5 bg-white/[0.02] px-4 py-2 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
          {state()?.enabled === false
            ? "Rewind disabled — no live data"
            : "No rewind activity (launch a game with rewind enabled to see live ring stats)"}
        </div>
      }
    >
      <div class="rounded-md border border-(--color-system-accent)/30 bg-(--color-system-accent)/[0.06] px-4 py-2 text-xs text-(--color-oa-ink)">
        <span class="font-medium text-(--color-system-accent)">Live</span>
        <span class="mx-2 text-(--color-oa-ink-dim)">·</span>
        <Show when={secondsHeld() !== null}>
          <span>{secondsHeld()!.toFixed(1)}s held</span>
          <span class="mx-2 text-(--color-oa-ink-dim)">·</span>
        </Show>
        <span>{state()!.snapshotCount} snap{state()!.snapshotCount === 1 ? "" : "s"}</span>
        <span class="mx-2 text-(--color-oa-ink-dim)">·</span>
        <span>{formatBytes(state()!.byteSize)}</span>
        <Show when={state()!.scrubbing}>
          <span class="mx-2 text-(--color-oa-ink-dim)">·</span>
          <span class="text-amber-300">scrubbing @ {state()!.scrubPosition}</span>
        </Show>
      </div>
    </Show>
  );
};
