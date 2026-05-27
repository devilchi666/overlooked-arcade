// Per-game settings dialogs launched from the Game ▾ menu and
// TileContextMenu. Phase D of docs/UI_POLISH_PLAN.md — replaces the seven
// tab-strip surfaces of the old PerGameSettingsDrawer with focused
// single-purpose dialogs. Overview + Core stay in GamePropertiesDialog;
// Region tab is gone (it had no runtime effect).
//
// Each dialog hydrates its own per-game overrides + per-system settings on
// open via `useGameOverrides()`. Persistence flows through the existing
// get_game_overrides / set_game_overrides Tauri commands so the launch path
// keeps working unchanged.

import {
  createEffect,
  createSignal,
  For,
  onCleanup,
  onMount,
  Show,
  type Accessor,
  type Component,
} from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Dialog, DialogSection } from "../layout/Dialog";
import SettingRow, { selectClass } from "./SettingRow";
import {
  BezelPicker,
  OverscanEditor,
  overscanIsZero,
  overscanLabel,
  pathBasename,
  RewindLiveStats,
  type OverscanCropPrefs,
} from "./SystemDialogs";
import CoreOptionsPanel from "./CoreOptionsPanel";
import AnalogBindingsSection from "./AnalogBindingsSection";
import KeypadReference from "./KeypadReference";
import {
  SCALING_MODE_LABELS,
  SCALING_OPTIONS,
  WINDOW_MODE_LABELS,
  WINDOW_OPTIONS,
  type MonitorInfo,
  type ScalingMode,
  type SettingsStore,
  type WindowMode,
} from "../settings/store";
import { shaderPresets, shaderPresetLabel } from "../settings/shader_presets";
import { systemThemes } from "../themes/registry";
import type { RomEntry } from "../library/types";

// --- Per-game overrides shape -------------------------------------------

export type GameOverrides = {
  scalingOverride?: string | null;
  windowModeOverride?: string | null;
  monitorIndexOverride?: number | null;
  regionOverride?: string | null;
  shaderPreset?: string | null;
  bloomAmount?: number | null;
  patchPath?: string | null;
  rewindEnabled?: boolean | null;
  rewindCaptureIntervalFrames?: number | null;
  rewindBufferMegabytes?: number | null;
  displayAspectOverride?: number | null;
  overscanCropOverride?: OverscanCropPrefs | null;
  bezelImagePath?: string | null;
  keypadLayoutNote?: string | null;
  libretroDevice?: number | null;
  libretroDevicePort1?: number | null;
  libretroDevicePort2?: number | null;
  libretroDevicePort3?: number | null;
  libretroDevicePort4?: number | null;
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

const DISPLAY_ASPECT_PRESETS: readonly { value: string; label: string }[] = [
  { value: "1.333", label: "4:3 (CRT TV)" },
  { value: "1.778", label: "16:9 (Widescreen)" },
  { value: "1.0",   label: "1:1 (Square pixels)" },
  { value: "1.143", label: "8:7 (NES authentic)" },
  { value: "1.185", label: "32:27 (PCE 256 authentic)" },
  { value: "1.306", label: "64:49 (PCE 352 authentic)" },
];

const REWIND_INTERVAL_OPTIONS: readonly number[] = [1, 2, 3, 6, 10, 15, 30];
const REWIND_BUFFER_OPTIONS: readonly number[] = [8, 16, 32, 64, 128, 256, 512];

function aspectLabel(value: number): string {
  const preset = DISPLAY_ASPECT_PRESETS.find((p) => Math.abs(Number(p.value) - value) < 0.01);
  if (preset) return preset.label;
  return value.toFixed(3);
}

// --- Shared composable: hydrates overrides + per-system settings -------
//
// Each dialog has its own copy of these signals because they're modal +
// short-lived. Hydrates on open; clears on close to avoid stale data when
// the user opens a different game's dialog afterwards.

type GameOverridesContext = {
  overrides: Accessor<GameOverrides>;
  systemSettings: Accessor<SystemSettings>;
  patch: (p: Partial<GameOverrides>) => Promise<void>;
};

function useGameOverrides(
  entry: Accessor<RomEntry | null>,
  open: Accessor<boolean>,
): GameOverridesContext {
  const [overrides, setOverrides] = createSignal<GameOverrides>({});
  const [systemSettings, setSystemSettings] = createSignal<SystemSettings>({});

  createEffect(() => {
    const e = entry();
    if (!open() || !e) return;
    void (async () => {
      try {
        const got = await invoke<GameOverrides>("get_game_overrides", { id: e.id });
        setOverrides(got ?? {});
      } catch (err) {
        console.warn("[oa-game-dialog] get_game_overrides failed:", err);
        setOverrides({});
      }
      try {
        const sys = await invoke<SystemSettings>("get_system_settings", {
          systemId: e.systemId,
        });
        setSystemSettings(sys ?? {});
      } catch (err) {
        console.warn("[oa-game-dialog] get_system_settings failed:", err);
        setSystemSettings({});
      }
    })();
  });

  async function patch(p: Partial<GameOverrides>) {
    const e = entry();
    if (!e) return;
    const next: GameOverrides = { ...overrides(), ...p };
    const cleaned: GameOverrides = {};
    if (next.scalingOverride != null) cleaned.scalingOverride = next.scalingOverride;
    if (next.windowModeOverride != null) cleaned.windowModeOverride = next.windowModeOverride;
    if (next.monitorIndexOverride != null) cleaned.monitorIndexOverride = next.monitorIndexOverride;
    if (next.regionOverride != null) cleaned.regionOverride = next.regionOverride;
    if (next.shaderPreset != null) cleaned.shaderPreset = next.shaderPreset;
    if (next.bloomAmount != null) cleaned.bloomAmount = next.bloomAmount;
    if (next.patchPath != null && next.patchPath !== "") cleaned.patchPath = next.patchPath;
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
    if (next.keypadLayoutNote != null && next.keypadLayoutNote.trim() !== "") {
      cleaned.keypadLayoutNote = next.keypadLayoutNote;
    }
    if (next.libretroDevice != null) cleaned.libretroDevice = next.libretroDevice;
    if (next.libretroDevicePort1 != null) cleaned.libretroDevicePort1 = next.libretroDevicePort1;
    if (next.libretroDevicePort2 != null) cleaned.libretroDevicePort2 = next.libretroDevicePort2;
    if (next.libretroDevicePort3 != null) cleaned.libretroDevicePort3 = next.libretroDevicePort3;
    if (next.libretroDevicePort4 != null) cleaned.libretroDevicePort4 = next.libretroDevicePort4;
    setOverrides(cleaned);
    try {
      await invoke("set_game_overrides", { id: e.id, overrides: cleaned });
    } catch (err) {
      console.warn("[oa-game-dialog] set_game_overrides failed:", err);
    }
  }

  return { overrides, systemSettings, patch };
}

// --- Dialog title helper -----------------------------------------------

function dialogTitle(entry: RomEntry | null, kind: string): string {
  if (!entry) return kind;
  return `${kind} — ${entry.title}`;
}

function dialogSubtitle(entry: RomEntry | null): string | undefined {
  if (!entry) return undefined;
  const theme = systemThemes[entry.systemId];
  return theme?.displayName ?? entry.systemId;
}

// --- GameDisplayDialog --------------------------------------------------

export const GameDisplayDialog: Component<{
  open: boolean;
  entry: RomEntry | null;
  onClose: () => void;
  settings: SettingsStore;
}> = (props) => {
  const ctx = useGameOverrides(() => props.entry, () => props.open);
  const { overrides, systemSettings, patch } = ctx;
  const [monitors, setMonitors] = createSignal<MonitorInfo[]>([]);

  createEffect(() => {
    if (!props.open) return;
    void invoke<MonitorInfo[]>("list_monitors")
      .then((m) => setMonitors(m ?? []))
      .catch(() => setMonitors([]));
  });

  const inheritedScaling = () => {
    const sys = systemSettings().scalingOverride;
    if (sys) return { label: SCALING_MODE_LABELS[sys as ScalingMode] ?? sys, from: "Per-system" };
    return {
      label: SCALING_MODE_LABELS[props.settings.scalingMode()] ?? props.settings.scalingMode(),
      from: "OA default",
    };
  };
  const inheritedWindow = () => {
    const sys = systemSettings().windowModeOverride;
    if (sys) return { label: WINDOW_MODE_LABELS[sys as WindowMode] ?? sys, from: "Per-system" };
    return {
      label: WINDOW_MODE_LABELS[props.settings.windowMode()] ?? props.settings.windowMode(),
      from: "OA default",
    };
  };
  const inheritedMonitor = () => {
    const fromIdx = (idx: number | null | undefined): string => {
      if (idx === null || idx === undefined) return "Current monitor";
      const m = monitors().find((mm) => mm.index === idx);
      if (!m) return `Monitor ${idx + 1}`;
      return m.name?.trim() || `Monitor ${idx + 1}`;
    };
    const sys = systemSettings().monitorIndexOverride;
    if (sys !== null && sys !== undefined) return { label: fromIdx(sys), from: "Per-system" };
    return { label: fromIdx(props.settings.monitorIndex()), from: "OA default" };
  };

  return (
    <Dialog
      open={props.open}
      onClose={props.onClose}
      title={dialogTitle(props.entry, "Display overrides")}
      subtitle={dialogSubtitle(props.entry)}
      system={props.entry?.systemId}
      size="xl"
    >
      <Show when={props.entry}>
        {(_) => (
          <div class="flex flex-col gap-5">
            <DialogSection title="Scaling + window">
              <SettingRow
                label="Scaling mode"
                inheritedValue={inheritedScaling().label}
                inheritedFrom={inheritedScaling().from}
                overridden={overrides().scalingOverride != null}
              >
                <select
                  class={selectClass("system")}
                  value={overrides().scalingOverride ?? ""}
                  onChange={(e) => void patch({
                    scalingOverride: e.currentTarget.value === "" ? null : e.currentTarget.value,
                  })}
                >
                  <option value="">— Use inherited —</option>
                  <For each={SCALING_OPTIONS}>
                    {(m) => <option value={m}>{SCALING_MODE_LABELS[m as ScalingMode]}</option>}
                  </For>
                </select>
              </SettingRow>
              <SettingRow
                label="Window mode"
                inheritedValue={inheritedWindow().label}
                inheritedFrom={inheritedWindow().from}
                overridden={overrides().windowModeOverride != null}
              >
                <select
                  class={selectClass("system")}
                  value={overrides().windowModeOverride ?? ""}
                  onChange={(e) => void patch({
                    windowModeOverride: e.currentTarget.value === "" ? null : e.currentTarget.value,
                  })}
                >
                  <option value="">— Use inherited —</option>
                  <For each={WINDOW_OPTIONS}>
                    {(m) => <option value={m}>{WINDOW_MODE_LABELS[m as WindowMode]}</option>}
                  </For>
                </select>
              </SettingRow>
              <SettingRow
                label="Monitor"
                inheritedValue={inheritedMonitor().label}
                inheritedFrom={inheritedMonitor().from}
                overridden={overrides().monitorIndexOverride != null}
              >
                <select
                  class={selectClass("system")}
                  value={overrides().monitorIndexOverride?.toString() ?? ""}
                  onChange={(e) => {
                    const v = e.currentTarget.value;
                    void patch({ monitorIndexOverride: v === "" ? null : Number(v) });
                  }}
                >
                  <option value="">— Use inherited —</option>
                  <For each={monitors()}>
                    {(m) => (
                      <option value={m.index.toString()}>
                        {(m.name?.trim() || `Monitor ${m.index + 1}`) +
                          ` (${m.width}×${m.height})`}
                      </option>
                    )}
                  </For>
                </select>
              </SettingRow>
            </DialogSection>

            <DialogSection title="Aspect + crop">
              <SettingRow
                label="Display aspect"
                hint="Pixel-aspect at the renderer; affects Aspect-correct + Pixel-perfect modes"
                inheritedValue={
                  systemSettings().displayAspectOverride != null
                    ? `${aspectLabel(systemSettings().displayAspectOverride!)} (Per-system)`
                    : "Core-reported"
                }
                inheritedFrom={
                  systemSettings().displayAspectOverride != null ? "Per-system" : "OA default"
                }
                overridden={overrides().displayAspectOverride != null}
              >
                <select
                  class={selectClass("system")}
                  value={overrides().displayAspectOverride?.toString() ?? ""}
                  onChange={(e) => {
                    const v = e.currentTarget.value;
                    void patch({ displayAspectOverride: v === "" ? null : Number(v) });
                  }}
                >
                  <option value="">— Use inherited —</option>
                  <For each={DISPLAY_ASPECT_PRESETS}>
                    {(p) => <option value={p.value}>{p.label}</option>}
                  </For>
                </select>
              </SettingRow>
              <SettingRow
                label="Overscan crop"
                hint="Hide source pixels at each edge (T/B/L/R)"
                inheritedValue={
                  overscanIsZero(systemSettings().overscanCropOverride)
                    ? "No crop"
                    : `${overscanLabel(systemSettings().overscanCropOverride)} (Per-system)`
                }
                inheritedFrom={
                  overscanIsZero(systemSettings().overscanCropOverride) ? "OA default" : "Per-system"
                }
                overridden={!overscanIsZero(overrides().overscanCropOverride)}
              >
                <OverscanEditor
                  value={
                    overrides().overscanCropOverride
                    ?? systemSettings().overscanCropOverride
                    ?? { top: 0, bottom: 0, left: 0, right: 0 }
                  }
                  onChange={(next) => void patch({
                    overscanCropOverride: overscanIsZero(next) ? null : next,
                  })}
                />
              </SettingRow>
              <SettingRow
                label="Bezel image"
                hint="PNG / JPEG / WebP overlaid on top of the game pixels"
                inheritedValue={
                  systemSettings().bezelImagePath
                    ? `${pathBasename(systemSettings().bezelImagePath)} (Per-system)`
                    : "Use shader preset default"
                }
                inheritedFrom={
                  systemSettings().bezelImagePath ? "Per-system" : "OA default"
                }
                overridden={overrides().bezelImagePath != null}
              >
                <BezelPicker
                  value={overrides().bezelImagePath ?? null}
                  onChange={(path) => void patch({ bezelImagePath: path })}
                />
              </SettingRow>
            </DialogSection>
          </div>
        )}
      </Show>
    </Dialog>
  );
};

// --- GameRewindDialog ---------------------------------------------------

export const GameRewindDialog: Component<{
  open: boolean;
  entry: RomEntry | null;
  onClose: () => void;
  settings: SettingsStore;
}> = (props) => {
  const { overrides, systemSettings, patch } = useGameOverrides(() => props.entry, () => props.open);

  const inheritedEnabled = () => {
    const sys = systemSettings().rewindEnabled;
    if (sys != null) return { label: sys ? "On" : "Off", from: "Per-system" };
    return { label: props.settings.rewindEnabled() ? "On" : "Off", from: "OA default" };
  };
  const inheritedInterval = () => {
    const sys = systemSettings().rewindCaptureIntervalFrames;
    if (sys != null) return { label: `Every ${sys} frames`, from: "Per-system" };
    return { label: `Every ${props.settings.rewindCaptureIntervalFrames()} frames`, from: "OA default" };
  };
  const inheritedBuffer = () => {
    const sys = systemSettings().rewindBufferMegabytes;
    if (sys != null) return { label: `${sys} MB`, from: "Per-system" };
    return { label: `${props.settings.rewindBufferMegabytes()} MB`, from: "OA default" };
  };

  return (
    <Dialog
      open={props.open}
      onClose={props.onClose}
      title={dialogTitle(props.entry, "Rewind overrides")}
      subtitle={dialogSubtitle(props.entry)}
      system={props.entry?.systemId}
      size="lg"
    >
      <Show when={props.entry}>
        <div class="flex flex-col gap-4">
          <RewindLiveStats open={props.open} />
          <SettingRow
            label="Enable rewind"
            hint="Hold Backspace to step backwards"
            inheritedValue={inheritedEnabled().label}
            inheritedFrom={inheritedEnabled().from}
            overridden={overrides().rewindEnabled != null}
          >
            <select
              class={selectClass("system")}
              value={
                overrides().rewindEnabled == null
                  ? ""
                  : (overrides().rewindEnabled ? "on" : "off")
              }
              onChange={(e) => {
                const v = e.currentTarget.value;
                void patch({ rewindEnabled: v === "" ? null : v === "on" });
              }}
            >
              <option value="">— Use inherited —</option>
              <option value="on">On</option>
              <option value="off">Off</option>
            </select>
          </SettingRow>
          <SettingRow
            label="Capture interval"
            hint="Frames between snapshots"
            inheritedValue={inheritedInterval().label}
            inheritedFrom={inheritedInterval().from}
            overridden={overrides().rewindCaptureIntervalFrames != null}
          >
            <select
              class={selectClass("system")}
              value={overrides().rewindCaptureIntervalFrames?.toString() ?? ""}
              onChange={(e) => {
                const v = e.currentTarget.value;
                void patch({ rewindCaptureIntervalFrames: v === "" ? null : Number(v) });
              }}
            >
              <option value="">— Use inherited —</option>
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
            inheritedValue={inheritedBuffer().label}
            inheritedFrom={inheritedBuffer().from}
            overridden={overrides().rewindBufferMegabytes != null}
          >
            <select
              class={selectClass("system")}
              value={overrides().rewindBufferMegabytes?.toString() ?? ""}
              onChange={(e) => {
                const v = e.currentTarget.value;
                void patch({ rewindBufferMegabytes: v === "" ? null : Number(v) });
              }}
            >
              <option value="">— Use inherited —</option>
              <For each={REWIND_BUFFER_OPTIONS}>
                {(mb) => <option value={String(mb)}>{mb} MB</option>}
              </For>
            </select>
          </SettingRow>
        </div>
      </Show>
    </Dialog>
  );
};

// --- GameShadersDialog --------------------------------------------------

export const GameShadersDialog: Component<{
  open: boolean;
  entry: RomEntry | null;
  onClose: () => void;
  settings: SettingsStore;
}> = (props) => {
  const { overrides, systemSettings, patch } = useGameOverrides(() => props.entry, () => props.open);

  const inheritedShader = () => {
    const sys = systemSettings().shaderPreset;
    if (sys) return { label: shaderPresetLabel(sys), from: "Per-system" };
    return { label: shaderPresetLabel(props.settings.shaderPreset()), from: "OA default" };
  };

  return (
    <Dialog
      open={props.open}
      onClose={props.onClose}
      title={dialogTitle(props.entry, "Shaders")}
      subtitle={dialogSubtitle(props.entry)}
      system={props.entry?.systemId}
      size="lg"
    >
      <Show when={props.entry}>
        <div class="flex flex-col gap-4">
          <SettingRow
            label="Shader preset"
            hint="Applies at next launch"
            inheritedValue={inheritedShader().label}
            inheritedFrom={inheritedShader().from}
            overridden={overrides().shaderPreset != null}
          >
            <select
              class={selectClass("system")}
              value={overrides().shaderPreset ?? ""}
              onChange={(e) => {
                const v = e.currentTarget.value;
                void patch({ shaderPreset: v === "" ? null : v });
              }}
            >
              <option value="">— Use inherited —</option>
              <For each={shaderPresets()}>
                {(p) => <option value={p.name}>{p.displayName}</option>}
              </For>
            </select>
          </SettingRow>
          <SettingRow
            label="Bloom amount (Phosphor only)"
            hint="Overrides the active Phosphor preset's bloom weight. 0 = pure source, 1 = pure blur. Ignored when the active preset isn't Phosphor-based."
            inheritedValue={
              systemSettings().bloomAmount != null
                ? `${systemSettings().bloomAmount!.toFixed(2)} (Per-system)`
                : "Preset default"
            }
            overridden={overrides().bloomAmount != null}
            slider={{
              min: 0,
              max: 1,
              step: 0.05,
              value: overrides().bloomAmount ?? systemSettings().bloomAmount ?? 0.6,
              onInput: (v) => {
                void patch({ bloomAmount: v });
                void invoke("set_bloom_amount", { amount: v }).catch(() => {});
              },
            }}
            onReset={() => void patch({ bloomAmount: null })}
          />
          <p class="text-xs leading-relaxed text-(--color-oa-ink-dim)">
            Presets come from <code>shaders/presets/*.preset.toml</code>; built-ins ship
            in-binary and user files in <code>&lt;exe_dir&gt;/shaders/presets/</code>
            overlay by name. Per-game bloom_amount layers on top of the per-system
            override, which layers on top of the preset's TOML default.
          </p>
        </div>
      </Show>
    </Dialog>
  );
};

// --- GameCoreOptionsDialog ---------------------------------------------

export const GameCoreOptionsDialog: Component<{
  open: boolean;
  entry: RomEntry | null;
  onClose: () => void;
}> = (props) => (
  <Dialog
    open={props.open}
    onClose={props.onClose}
    title={dialogTitle(props.entry, "Core options")}
    subtitle={dialogSubtitle(props.entry)}
    system={props.entry?.systemId}
    size="xl"
  >
    <Show when={props.entry}>
      {(e) => (
        <CoreOptionsPanel systemId={e().systemId} gameId={e().id} />
      )}
    </Show>
  </Dialog>
);

// --- GameInputDialog ---------------------------------------------------

/// Per-system label override for the libretro device-type select. Most
/// cores accept the same device numbers but the system-canonical name
/// differs — Beetle Saturn calls device 5 the "3D Pad" rather than
/// the generic "Analog Pad", FCEUmm calls device 4 the "Zapper", etc.
/// Operators reading the dropdown for a Saturn game shouldn't have to
/// translate "Analog Pad / Paddle" to "3D Pad" in their head. Returns
/// null when the system has no system-specific label for the device id,
/// letting the caller fall back to the generic.
function systemSpecificDeviceLabel(systemId: string, deviceId: number): string | null {
  switch (systemId) {
    case "saturn":
    case "stv":
      if (deviceId === 5) return "3D Pad / Analog";
      if (deviceId === 4) return "Virtua Gun";
      return null;
    case "nes":
      if (deviceId === 4) return "Zapper";
      return null;
    case "snes":
      if (deviceId === 2) return "SNES Mouse";
      if (deviceId === 4) return "Super Scope";
      return null;
    case "psx":
      if (deviceId === 5) return "DualShock / Analog";
      if (deviceId === 4) return "GunCon / Justifier";
      return null;
    case "dreamcast":
      if (deviceId === 4) return "Light Gun (House of the Dead)";
      return null;
    case "atari7800":
      if (deviceId === 4) return "XEGS Light Gun";
      if (deviceId === 5) return "Analog (Trak-Ball / Paddle)";
      return null;
    case "sms":
      if (deviceId === 4) return "Light Phaser";
      return null;
    case "n64":
      if (deviceId === 5) return "Analog (N64 stick)";
      return null;
    case "gamecube":
      // For GC dumps the base RetroPad id maps to "GameCube
      // Controller"; for Wii dumps it maps to a bare "Wii Remote".
      // We can't easily tell GC vs Wii apart from the systemId
      // alone (single Dolphin core covers both via runtime
      // auto-detect) so the label calls out both modes.
      if (deviceId === 1) return "GameCube Controller / Wii Remote";
      // The Wii subclasses below are Dolphin's hand-encoded
      // `((N << 8) | base)` values from
      // Source/Core/DolphinLibretro/Input.cpp:48-54. Picking one
      // forces the Dolphin core into that peripheral mode at
      // retro_set_controller_port_device time.
      if (deviceId === 513)  return "Wii Remote (sideways)";
      if (deviceId === 769)  return "Wii Remote + Nunchuk";
      if (deviceId === 1025) return "Wii Remote + Classic Controller";
      if (deviceId === 1281) return "Wii Remote + Classic Controller Pro";
      if (deviceId === 1537) return "GameCube Controller (Wii mode)";
      return null;
    default:
      return null;
  }
}

const DEVICE_ID_OPTIONS_BASE: readonly { id: number; generic: string }[] = [
  { id: 1, generic: "Standard Pad (RetroPad)" },
  { id: 5, generic: "Analog Pad / Paddle" },
  { id: 2, generic: "Mouse" },
  { id: 4, generic: "Light Gun" },
  { id: 6, generic: "Pointer / Stylus" },
  { id: 3, generic: "Keyboard" },
  { id: 0, generic: "Disconnected (no controller)" },
];

/// GameCube / Wii peripheral subclasses registered by the Dolphin
/// libretro core (Source/Core/DolphinLibretro/Input.cpp lines 48-54 +
/// 922-967). Dolphin hand-encodes the values as `((N << 8) | base)`
/// without the canonical libretro `RETRO_DEVICE_SUBCLASS` macro's
/// `+1` convention — the u32 wire values here are what
/// `retro_set_controller_port_device` actually receives. Operators
/// pick these in the per-game Input dialog when a Wii title needs a
/// specific peripheral (Skyward Sword → Wii Remote + Nunchuk,
/// SSBB → Classic Controller, Mario Kart Wii multiplayer → GC
/// Controller (Wii mode) for Wii U adapter slots, etc.). Real
/// WiiMote / Bluetooth passthrough (1536) intentionally skipped —
/// needs host-side Bluetooth pairing OA doesn't wire today.
const DEVICE_ID_OPTIONS_GAMECUBE: readonly { id: number; generic: string }[] = [
  { id: 513,  generic: "Wii Remote (sideways)" },
  { id: 769,  generic: "Wii Remote + Nunchuk" },
  { id: 1025, generic: "Wii Remote + Classic Controller" },
  { id: 1281, generic: "Wii Remote + Classic Controller Pro" },
  { id: 1537, generic: "GameCube Controller (Wii mode)" },
];

/// Per-system option-list resolution. Most systems just use the base
/// seven libretro device types; GameCube/Wii layers Dolphin's
/// Wii-peripheral subclasses on top so they appear in the dropdown
/// only for GC system games. Future systems with custom subclasses
/// (Saturn 3D Pad sits on the generic ANALOG id 5 today, so no
/// extras needed; if Beetle Saturn ever adds Twin-Stick Pro as a
/// subclass, this is the table to extend).
function deviceOptionsForSystem(
  systemId: string,
): readonly { id: number; generic: string }[] {
  if (systemId === "gamecube") {
    return [...DEVICE_ID_OPTIONS_BASE, ...DEVICE_ID_OPTIONS_GAMECUBE];
  }
  return DEVICE_ID_OPTIONS_BASE;
}

/// Render the device-type option label for a (systemId, deviceId) pair,
/// preferring the system-specific name when one exists.
function deviceOptionLabel(systemId: string, deviceId: number, generic: string): string {
  return systemSpecificDeviceLabel(systemId, deviceId) ?? generic;
}

export const GameInputDialog: Component<{
  open: boolean;
  entry: RomEntry | null;
  onClose: () => void;
}> = (props) => {
  const { overrides, patch } = useGameOverrides(() => props.entry, () => props.open);
  const [showExtraPorts, setShowExtraPorts] = createSignal(false);

  createEffect(() => {
    const o = overrides();
    if (
      o.libretroDevicePort1 != null
      || o.libretroDevicePort2 != null
      || o.libretroDevicePort3 != null
      || o.libretroDevicePort4 != null
    ) {
      setShowExtraPorts(true);
    }
  });

  return (
    <Dialog
      open={props.open}
      onClose={props.onClose}
      title={dialogTitle(props.entry, "Input")}
      subtitle={dialogSubtitle(props.entry)}
      system={props.entry?.systemId}
      size="xl"
    >
      <Show when={props.entry}>
        {(e) => (
          <div class="flex flex-col gap-4">
            <div class="rounded border border-(--color-oa-bg) bg-(--color-oa-bg)/40 p-3">
              <div class="mb-1 text-sm font-medium text-(--color-oa-ink)">
                libretro device type (port 0)
              </div>
              <div class="mb-2 text-xs leading-relaxed text-(--color-oa-ink-dim)">
                What kind of controller this game thinks is plugged in. The core
                has to support the chosen device for it to do anything — most
                JOYPAD cores ignore Mouse / Light Gun; Stella supports Paddle
                (Analog) for Breakout; FCEUmm + Beetle PSX support Light Gun
                for Zapper / Time Crisis.
              </div>
              <select
                class={selectClass("system")}
                value={overrides().libretroDevice == null ? "" : String(overrides().libretroDevice)}
                onChange={(ev) => {
                  const v = ev.currentTarget.value;
                  void patch({ libretroDevice: v === "" ? null : Number(v) });
                }}
              >
                <option value="">— Inherit (Standard Pad) —</option>
                <For each={deviceOptionsForSystem(e().systemId)}>
                  {(opt) => (
                    <option value={String(opt.id)}>
                      {deviceOptionLabel(e().systemId, opt.id, opt.generic)}
                    </option>
                  )}
                </For>
              </select>
              <Show
                when={showExtraPorts()}
                fallback={
                  <button
                    type="button"
                    class="mt-2 rounded border border-(--color-oa-bg-deep) bg-(--color-oa-bg-deep)/40 px-2 py-1 text-xs uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-(--color-oa-bg-deep)/80 hover:text-(--color-oa-ink)"
                    onClick={() => setShowExtraPorts(true)}
                  >
                    + Additional ports (1–4)
                  </button>
                }
              >
                <div class="mt-3 flex flex-col gap-2 border-t border-(--color-oa-bg-deep) pt-3">
                  <Show when={e().systemId === "atari7800"}>
                    <p class="rounded border border-(--color-system-accent)/30 bg-(--color-system-accent)/5 px-2 py-1.5 text-[0.65rem] leading-relaxed text-(--color-oa-ink-dim)">
                      <span class="text-(--color-oa-ink)">Robotron 2084 + twin-stick games:</span> set
                      Port 0 = Standard Pad AND Port 1 = Standard Pad. The
                      second pad becomes the shooting joystick (right stick
                      on a single physical controller, or a second physical
                      controller for arcade-faithful play).
                    </p>
                  </Show>
                  <Show when={e().systemId === "saturn" || e().systemId === "stv"}>
                    <p class="rounded border border-(--color-system-accent)/30 bg-(--color-system-accent)/5 px-2 py-1.5 text-[0.65rem] leading-relaxed text-(--color-oa-ink-dim)">
                      <span class="text-(--color-oa-ink)">Saturn 3D Pad:</span> set Port 0 = "3D Pad /
                      Analog" above. Beetle Saturn interprets device 5 as
                      3D Pad mode (analog stick + analog triggers). Picks
                      up the Saturn-side `analog_routing` per-system
                      defaults automatically.
                    </p>
                  </Show>
                  <Show when={e().systemId === "gamecube"}>
                    <p class="rounded border border-(--color-system-accent)/30 bg-(--color-system-accent)/5 px-2 py-1.5 text-[0.65rem] leading-relaxed text-(--color-oa-ink-dim)">
                      <span class="text-(--color-oa-ink)">Wii peripherals:</span> Dolphin auto-detects
                      GC vs Wii from the disc, but Wii titles often
                      need a specific controller mode. Skyward Sword
                      + Twilight Princess + Galaxy 1/2 + RE4 Wii →
                      Wii Remote + Nunchuk. Super Smash Bros Brawl +
                      Monster Hunter Tri + Xenoblade Chronicles →
                      Wii Remote + Classic Controller (Pro variant
                      for newer titles). NSMB Wii + Excite Truck →
                      Wii Remote (sideways). Mario Kart Wii
                      multiplayer with the Wii U GC adapter →
                      GameCube Controller (Wii mode). Pick on Port 0
                      above; use additional ports below for 2-4
                      player setups.
                    </p>
                  </Show>
                  <For each={[1, 2, 3, 4] as const}>
                    {(portIdx) => {
                      const key =
                        (`libretroDevicePort${portIdx}` as
                          | "libretroDevicePort1"
                          | "libretroDevicePort2"
                          | "libretroDevicePort3"
                          | "libretroDevicePort4");
                      return (
                        <div class="flex items-center gap-2 text-xs">
                          <span class="w-14 shrink-0 text-(--color-oa-ink-dim)">
                            Port {portIdx}
                          </span>
                          <select
                            class={selectClass("system") + " flex-1"}
                            value={
                              overrides()[key] == null ? "" : String(overrides()[key])
                            }
                            onChange={(ev) => {
                              const v = ev.currentTarget.value;
                              void patch({ [key]: v === "" ? null : Number(v) });
                            }}
                          >
                            <option value="">— Inherit (Standard Pad) —</option>
                            <For each={deviceOptionsForSystem(e().systemId)}>
                              {(opt) => (
                                <option value={String(opt.id)}>
                                  {deviceOptionLabel(e().systemId, opt.id, opt.generic)}
                                </option>
                              )}
                            </For>
                          </select>
                        </div>
                      );
                    }}
                  </For>
                </div>
              </Show>
            </div>
            <p class="text-xs leading-relaxed text-(--color-oa-ink-dim)">
              Per-game analog routing overrides. When set, these values layer
              on top of the per-system Analog input settings — at launch, each
              port resolves per-game → per-system → identity. Use this to
              tweak deadzone / sensitivity / keyboard fallback for a specific
              game without affecting the system's default.
            </p>
            <AnalogBindingsSection
              systemId={e().systemId}
              mode="game"
              gameId={e().id}
            />
            <Show when={["coleco", "intv", "o2"].includes(e().systemId)}>
              <div class="rounded border border-(--color-oa-bg) bg-(--color-oa-bg)/40 p-3">
                <div class="mb-1 text-sm font-medium text-(--color-oa-ink)">
                  Keypad layout note
                </div>
                <div class="mb-2 text-xs leading-relaxed text-(--color-oa-ink-dim)">
                  This system shipped paper overlays so the keypad meant
                  something different in each game. Record what KP1–KP9 do in
                  this title; the per-system Bindings page still owns which
                  keyboard key triggers which KP.
                </div>
                <textarea
                  class="w-full resize-y rounded border border-(--color-oa-bg-deep) bg-(--color-oa-bg-deep) px-2 py-1.5 text-xs text-(--color-oa-ink) placeholder:text-(--color-oa-ink-dim)/60 focus:border-(--color-system-accent) focus:outline-none"
                  rows="3"
                  placeholder="KP1=climb-up, KP2=climb-down, KP3=jump, KP4=duck…"
                  value={overrides().keypadLayoutNote ?? ""}
                  onChange={(ev) => {
                    const v = ev.currentTarget.value;
                    void patch({ keypadLayoutNote: v.trim() === "" ? null : v });
                  }}
                />
              </div>
            </Show>
            <Show when={e().systemId === "coleco"}>
              <KeypadReference systemId={e().systemId} />
            </Show>
          </div>
        )}
      </Show>
    </Dialog>
  );
};

// --- Milestones --------------------------------------------------------

type Milestone = {
  id?: number;
  gameId: string;
  name: string;
  description: string;
  region: "save_ram" | "rtc" | "system_ram" | "video_ram";
  offset: number;
  width: 1 | 2 | 4;
  op: "eq" | "neq" | "gt" | "lt" | "geq" | "leq";
  target: number;
  edgeOnly: boolean;
  triggeredAtUnixMs?: number;
};

const MILESTONE_REGION_OPTIONS: readonly Milestone["region"][] = [
  "system_ram",
  "save_ram",
  "rtc",
  "video_ram",
];
const MILESTONE_OP_OPTIONS: readonly Milestone["op"][] = [
  "eq",
  "neq",
  "gt",
  "lt",
  "geq",
  "leq",
];
const MILESTONE_WIDTH_OPTIONS: readonly Milestone["width"][] = [1, 2, 4];

const MILESTONE_OP_LABELS: Record<Milestone["op"], string> = {
  eq: "==",
  neq: "!=",
  gt: ">",
  lt: "<",
  geq: ">=",
  leq: "<=",
};
const MILESTONE_REGION_LABELS: Record<Milestone["region"], string> = {
  save_ram: "Save RAM",
  rtc: "RTC",
  system_ram: "System RAM",
  video_ram: "Video RAM",
};

function parseHexOrDec(raw: string): number | null {
  const t = raw.trim();
  if (!t) return 0;
  if (t.toLowerCase().startsWith("0x")) {
    const n = parseInt(t.slice(2), 16);
    return Number.isFinite(n) ? n : null;
  }
  const n = parseInt(t, t.match(/[a-fA-F]/) ? 16 : 10);
  return Number.isFinite(n) ? n : null;
}

const MilestoneEditor: Component<{
  draft: Milestone;
  onChange: (m: Milestone) => void;
  onSave: () => void;
  onCancel: () => void;
}> = (props) => {
  const set = <K extends keyof Milestone>(k: K, v: Milestone[K]) => {
    props.onChange({ ...props.draft, [k]: v });
  };
  const inputClass =
    "w-full rounded-md border border-white/10 bg-white/[0.04] px-2.5 py-1.5 text-sm text-(--color-oa-ink) focus-visible:border-(--color-system-accent) focus-visible:outline-none";
  return (
    <div class="flex flex-col gap-2 rounded-md border border-(--color-system-accent)/40 bg-white/[0.03] p-3">
      <label class="block space-y-0.5">
        <span class="text-xs uppercase tracking-widest text-(--color-oa-ink-dim)">Name</span>
        <input
          type="text"
          value={props.draft.name}
          onInput={(e) => set("name", e.currentTarget.value)}
          placeholder="Boss 1 defeated"
          class={inputClass}
        />
      </label>
      <label class="block space-y-0.5">
        <span class="text-xs uppercase tracking-widest text-(--color-oa-ink-dim)">
          Description (optional)
        </span>
        <input
          type="text"
          value={props.draft.description}
          onInput={(e) => set("description", e.currentTarget.value)}
          class={inputClass}
        />
      </label>
      <div class="flex gap-2">
        <label class="flex-1 space-y-0.5">
          <span class="text-xs uppercase tracking-widest text-(--color-oa-ink-dim)">Region</span>
          <select
            value={props.draft.region}
            onChange={(e) => set("region", e.currentTarget.value as Milestone["region"])}
            class={inputClass}
          >
            <For each={MILESTONE_REGION_OPTIONS}>
              {(r) => <option value={r}>{MILESTONE_REGION_LABELS[r]}</option>}
            </For>
          </select>
        </label>
        <label class="w-28 space-y-0.5">
          <span class="text-xs uppercase tracking-widest text-(--color-oa-ink-dim)">Width</span>
          <select
            value={String(props.draft.width)}
            onChange={(e) => set("width", Number(e.currentTarget.value) as Milestone["width"])}
            class={inputClass}
          >
            <For each={MILESTONE_WIDTH_OPTIONS}>
              {(w) => <option value={String(w)}>{w === 1 ? "u8" : w === 2 ? "u16" : "u32"}</option>}
            </For>
          </select>
        </label>
      </div>
      <div class="flex gap-2">
        <label class="flex-1 space-y-0.5">
          <span class="text-xs uppercase tracking-widest text-(--color-oa-ink-dim)">
            Offset (hex or dec)
          </span>
          <input
            type="text"
            value={`0x${props.draft.offset.toString(16).toUpperCase()}`}
            onChange={(e) => {
              const n = parseHexOrDec(e.currentTarget.value);
              if (n !== null && n >= 0) set("offset", n);
            }}
            class={inputClass + " font-mono"}
          />
        </label>
        <label class="w-24 space-y-0.5">
          <span class="text-xs uppercase tracking-widest text-(--color-oa-ink-dim)">Op</span>
          <select
            value={props.draft.op}
            onChange={(e) => set("op", e.currentTarget.value as Milestone["op"])}
            class={inputClass}
          >
            <For each={MILESTONE_OP_OPTIONS}>
              {(o) => <option value={o}>{MILESTONE_OP_LABELS[o]}</option>}
            </For>
          </select>
        </label>
        <label class="flex-1 space-y-0.5">
          <span class="text-xs uppercase tracking-widest text-(--color-oa-ink-dim)">Target</span>
          <input
            type="text"
            value={props.draft.target.toString()}
            onChange={(e) => {
              const n = parseHexOrDec(e.currentTarget.value);
              if (n !== null) set("target", n);
            }}
            class={inputClass + " font-mono"}
          />
        </label>
      </div>
      <label class="flex items-center gap-2 text-xs text-(--color-oa-ink-dim)">
        <input
          type="checkbox"
          checked={props.draft.edgeOnly}
          onChange={(e) => set("edgeOnly", e.currentTarget.checked)}
          class="size-3.5 accent-(--color-system-accent)"
        />
        <span>
          Edge-trigger (fire once on transition) — uncheck for level-trigger ("currently in
          state")
        </span>
      </label>
      <div class="flex gap-2 pt-1">
        <button
          type="button"
          onClick={props.onCancel}
          class="flex-1 rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-sm text-(--color-oa-ink) hover:bg-white/[0.08]"
        >
          Cancel
        </button>
        <button
          type="button"
          onClick={props.onSave}
          disabled={!props.draft.name.trim()}
          class="flex-1 rounded-md border border-(--color-system-accent)/40 bg-(--color-system-accent)/15 px-3 py-1.5 text-sm text-(--color-oa-ink) hover:bg-(--color-system-accent)/25 disabled:cursor-not-allowed disabled:opacity-40"
        >
          {props.draft.id == null ? "Add" : "Save"}
        </button>
      </div>
    </div>
  );
};

export const MilestonesDialog: Component<{
  open: boolean;
  entry: RomEntry | null;
  onClose: () => void;
}> = (props) => {
  const [milestones, setMilestones] = createSignal<Milestone[]>([]);
  const [draft, setDraft] = createSignal<Milestone | null>(null);

  function blank(): Milestone {
    return {
      gameId: props.entry?.id ?? "",
      name: "",
      description: "",
      region: "system_ram",
      offset: 0,
      width: 1,
      op: "eq",
      target: 0,
      edgeOnly: true,
    };
  }

  async function refresh() {
    const e = props.entry;
    if (!e) return;
    try {
      const ms = await invoke<Milestone[]>("list_milestones", { gameId: e.id });
      setMilestones(ms ?? []);
    } catch (err) {
      console.warn("[oa-milestones-dialog] list failed:", err);
    }
  }

  createEffect(() => {
    if (props.open && props.entry) {
      void refresh();
      setDraft(null);
    }
  });

  // Live refresh on milestone trigger.
  onMount(() => {
    let unlisten: (() => void) | undefined;
    void listen("oa://milestone-triggered", () => {
      if (props.open && props.entry) void refresh();
    }).then((un) => (unlisten = un));
    onCleanup(() => unlisten?.());
  });

  async function save(m: Milestone) {
    const e = props.entry;
    if (!e) return;
    try {
      if (m.id == null) {
        await invoke<number>("add_milestone", { milestone: { ...m, gameId: e.id } });
      } else {
        await invoke("update_milestone", { milestone: m });
      }
      setDraft(null);
      await refresh();
    } catch (err) {
      console.warn("[oa-milestones-dialog] save failed:", err);
    }
  }

  async function remove(id: number) {
    try {
      await invoke("delete_milestone", { id });
      await refresh();
    } catch (err) {
      console.warn("[oa-milestones-dialog] delete failed:", err);
    }
  }

  async function resetProgress(id: number) {
    try {
      await invoke("reset_milestone_progress", { id });
      await refresh();
    } catch (err) {
      console.warn("[oa-milestones-dialog] reset_progress failed:", err);
    }
  }

  const triggeredCount = () =>
    milestones().filter((m) => m.triggeredAtUnixMs != null).length;

  return (
    <Dialog
      open={props.open}
      onClose={props.onClose}
      title={dialogTitle(props.entry, "Milestones")}
      subtitle={dialogSubtitle(props.entry)}
      system={props.entry?.systemId}
      size="xl"
    >
      <Show when={props.entry}>
        <div class="flex flex-col gap-3">
          <div class="rounded-md border border-white/10 bg-white/[0.03] px-3 py-2 text-xs text-(--color-oa-ink-dim)">
            <p>
              <span class="font-semibold text-(--color-oa-ink)">
                {milestones().length === 0
                  ? "No milestones configured."
                  : `${triggeredCount()} / ${milestones().length} triggered`}
              </span>
            </p>
            <p class="mt-1 leading-relaxed">
              A milestone fires once when a memory value matches the predicate
              (e.g. "byte at 0x1234 == 0x80"). Use the memory inspector
              (Esc → Memory inspector) to find addresses that change with
              in-game progress.
            </p>
          </div>

          <Show when={milestones().length > 0}>
            <ul class="flex flex-col gap-1.5">
              <For each={milestones()}>
                {(m) => (
                  <li class="rounded-md border border-white/10 bg-white/[0.03] p-2.5">
                    <div class="flex items-baseline justify-between gap-2">
                      <span class="truncate text-sm font-medium text-(--color-oa-ink)">
                        {m.name || "(unnamed)"}
                      </span>
                      <Show when={m.triggeredAtUnixMs != null}>
                        <span class="rounded border border-(--color-system-accent)/40 bg-(--color-system-accent)/15 px-1.5 py-0.5 text-[0.55rem] uppercase tracking-widest text-(--color-system-accent)">
                          Triggered
                        </span>
                      </Show>
                    </div>
                    <p class="mt-0.5 font-mono text-[0.65rem] text-(--color-oa-ink-dim)">
                      {MILESTONE_REGION_LABELS[m.region]} @ 0x
                      {m.offset.toString(16).toUpperCase()} (
                      {m.width === 1 ? "u8" : m.width === 2 ? "u16" : "u32"}){" "}
                      {MILESTONE_OP_LABELS[m.op]} {m.target}
                      {m.edgeOnly ? " · edge" : " · level"}
                    </p>
                    <Show when={m.description}>
                      <p class="mt-1 text-[0.65rem] text-(--color-oa-ink-dim)">
                        {m.description}
                      </p>
                    </Show>
                    <div class="mt-2 flex gap-1.5">
                      <button
                        type="button"
                        onClick={() => setDraft({ ...m })}
                        class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim) hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
                      >
                        Edit
                      </button>
                      <Show when={m.triggeredAtUnixMs != null}>
                        <button
                          type="button"
                          onClick={() => m.id != null && void resetProgress(m.id)}
                          class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim) hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
                        >
                          Reset
                        </button>
                      </Show>
                      <button
                        type="button"
                        onClick={() => m.id != null && void remove(m.id)}
                        class="rounded border border-red-500/30 px-2 py-0.5 text-[0.65rem] uppercase tracking-widest text-red-300 hover:bg-red-500/10"
                      >
                        Delete
                      </button>
                    </div>
                  </li>
                )}
              </For>
            </ul>
          </Show>

          <Show
            when={draft() !== null}
            fallback={
              <button
                type="button"
                onClick={() => setDraft(blank())}
                class="w-full rounded-md border border-(--color-system-accent)/40 bg-(--color-system-accent)/10 px-3 py-2 text-sm text-(--color-oa-ink) hover:bg-(--color-system-accent)/20"
              >
                + Add milestone
              </button>
            }
          >
            <MilestoneEditor
              draft={draft()!}
              onChange={(m) => setDraft(m)}
              onSave={() => void save(draft()!)}
              onCancel={() => setDraft(null)}
            />
          </Show>
        </div>
      </Show>
    </Dialog>
  );
};

// --- Cheats ------------------------------------------------------------

type Cheat = {
  id?: number;
  gameId: string;
  name: string;
  description: string;
  region: string;
  offset: number;
  width: number;
  value: number;
  enabled: boolean;
  kind: string;
  code?: string | null;
};

/// Per-system cheat format declaration — returned by the backend's
/// `list_cheat_formats` Tauri command. Each system gets at minimum
/// `memory_poke` + `libretro_code`; systems with documented named
/// formats (Game Genie / GameShark / Action Replay / CodeBreaker /
/// Pro Action Replay) include those between the two.
type CheatFormat = {
  id: string;
  label: string;
  hint: string;
  validationRegex: string;
  isMemoryPoke: boolean;
};

const CHEAT_REGION_OPTIONS = ["system_ram", "save_ram", "video_ram", "rtc"] as const;

type CheatSearchSummary = {
  region: string;
  width: number;
  candidateCount: number;
  top: Array<{ offset: number; currentValue: number; previousValue: number }>;
};
type CheatSearchFilter =
  | { kind: "changed" }
  | { kind: "unchanged" }
  | { kind: "increased" }
  | { kind: "decreased" }
  | { kind: "equal_to_value"; value: number };

export const CheatsDialog: Component<{
  open: boolean;
  entry: RomEntry | null;
  onClose: () => void;
}> = (props) => {
  const [cheats, setCheats] = createSignal<Cheat[]>([]);
  const [draft, setDraft] = createSignal<Cheat | null>(null);
  // Per-system cheat-format declarations, fetched on dialog open.
  // Drives the Type picker + per-format input validation.
  const [cheatFormats, setCheatFormats] = createSignal<CheatFormat[]>([]);
  // Cheat search state
  const [searchActive, setSearchActive] = createSignal(false);
  const [searchRegion, setSearchRegion] = createSignal("system_ram");
  const [searchSummary, setSearchSummary] = createSignal<CheatSearchSummary | null>(null);
  const [searchEqualValue, setSearchEqualValue] = createSignal(0);
  const [searchPending, setSearchPending] = createSignal(false);

  const gameId = () => props.entry?.id ?? "";

  function blank(): Cheat {
    return {
      gameId: gameId(),
      name: "",
      description: "",
      region: "system_ram",
      offset: 0,
      width: 1,
      value: 0,
      enabled: true,
      kind: "memory_poke",
      code: null,
    };
  }

  async function refresh() {
    if (!gameId()) {
      setCheats([]);
      return;
    }
    try {
      const list = await invoke<Cheat[]>("list_cheats", { gameId: gameId() });
      setCheats(list);
    } catch (e) {
      console.warn("[oa-cheats-dialog] list failed:", e);
    }
  }

  /// Pull per-system cheat-format declarations once per dialog open.
  /// Static per system_id — no need to refresh on cheat add/edit.
  async function refreshFormats() {
    const sysId = props.entry?.systemId;
    if (!sysId) {
      setCheatFormats([]);
      return;
    }
    try {
      const formats = await invoke<CheatFormat[]>("list_cheat_formats", {
        systemId: sysId,
      });
      setCheatFormats(formats);
    } catch (e) {
      console.warn("[oa-cheats-dialog] list_cheat_formats failed:", e);
      setCheatFormats([]);
    }
  }

  /// Validate a code string against the chosen format's regex. Returns
  /// `null` (valid) or a short error message. Empty validation_regex
  /// means "no validation" (memory_poke + libretro_code generic).
  function validateCode(kind: string, code: string | null | undefined): string | null {
    const format = cheatFormats().find((f) => f.id === kind);
    if (!format || !format.validationRegex) return null;
    const trimmed = (code ?? "").trim();
    if (!trimmed) return "Code required";
    try {
      const re = new RegExp(format.validationRegex, "i");
      if (!re.test(trimmed)) {
        return `Expected ${format.hint}`;
      }
    } catch (e) {
      console.warn("[oa-cheats-dialog] regex parse failed:", e);
      return null; // fail-open if the pattern itself is broken
    }
    return null;
  }

  /// Label for a saved cheat's format — shown in the cheat list row.
  /// Falls back to the raw `kind` string if the format isn't declared
  /// for the active system (defensive — operators editing a cheat
  /// after switching cores might see a stale kind).
  function labelForKind(kind: string): string {
    return cheatFormats().find((f) => f.id === kind)?.label ?? kind;
  }

  createEffect(() => {
    if (props.open && gameId()) {
      void refresh();
      void refreshFormats();
    }
  });

  async function rearm() {
    try {
      await invoke<number>("arm_cheats", { gameId: gameId() });
    } catch (e) {
      console.warn("[oa-cheats-dialog] arm failed:", e);
    }
  }

  async function save(c: Cheat) {
    try {
      if (c.id) await invoke("update_cheat", { cheat: c });
      else await invoke<number>("add_cheat", { cheat: c });
      setDraft(null);
      await refresh();
      await rearm();
    } catch (e) {
      console.warn("[oa-cheats-dialog] save failed:", e);
    }
  }

  async function toggle(c: Cheat) {
    const next = { ...c, enabled: !c.enabled };
    try {
      await invoke("update_cheat", { cheat: next });
      await refresh();
      await rearm();
    } catch (e) {
      console.warn("[oa-cheats-dialog] toggle failed:", e);
    }
  }

  async function remove(id: number) {
    if (!window.confirm("Delete this cheat?")) return;
    try {
      await invoke("delete_cheat", { id });
      await refresh();
      await rearm();
    } catch (e) {
      console.warn("[oa-cheats-dialog] delete failed:", e);
    }
  }

  // --- Cheat search ---

  async function startSearch() {
    setSearchPending(true);
    try {
      const summary = await invoke<CheatSearchSummary>("start_cheat_search", {
        region: searchRegion(),
      });
      setSearchSummary(summary);
      setSearchActive(true);
    } catch (e) {
      console.warn("[oa-cheat-search] start failed:", e);
      window.alert(
        `Couldn't start search: ${typeof e === "string" ? e : (e as Error)?.message}`,
      );
    } finally {
      setSearchPending(false);
    }
  }
  async function runFilter(filter: CheatSearchFilter) {
    setSearchPending(true);
    try {
      const summary = await invoke<CheatSearchSummary>("filter_cheat_search", { filter });
      setSearchSummary(summary);
    } catch (e) {
      console.warn("[oa-cheat-search] filter failed:", e);
    } finally {
      setSearchPending(false);
    }
  }
  async function peek() {
    try {
      const summary = await invoke<CheatSearchSummary>("peek_cheat_search");
      setSearchSummary(summary);
    } catch (e) {
      console.warn("[oa-cheat-search] peek failed:", e);
    }
  }
  async function endSearch() {
    try {
      await invoke("end_cheat_search");
    } catch (e) {
      console.warn("[oa-cheat-search] end failed:", e);
    }
    setSearchActive(false);
    setSearchSummary(null);
  }
  function makeCheatFrom(offset: number, currentValue: number) {
    setDraft({
      ...blank(),
      name: `Search hit @ 0x${offset.toString(16).toUpperCase()}`,
      region: searchRegion(),
      offset,
      width: 1,
      value: currentValue,
    });
  }

  // End an active search if the dialog closes mid-search — avoids orphaned
  // search state on the Rust side when the operator dismisses the dialog
  // before clicking End.
  createEffect(() => {
    if (!props.open && searchActive()) {
      void endSearch();
    }
  });

  return (
    <Dialog
      open={props.open}
      onClose={props.onClose}
      title={dialogTitle(props.entry, "Cheats")}
      subtitle={dialogSubtitle(props.entry)}
      system={props.entry?.systemId}
      size="xl"
    >
      <Show when={props.entry}>
        <div class="flex flex-col gap-3">
          <p class="text-xs leading-relaxed text-(--color-oa-ink-dim)">
            Memory-poke cheats. Each enabled row writes <code>value</code>{" "}
            (<code>width</code> bytes, little-endian) to memory at{" "}
            <code>(region, offset)</code> every frame. Game Genie / Action
            Replay codes need to be translated to raw address+value via online
            tables for now; per-system code decoders are a follow-up.
          </p>

          {/* Cheat search */}
          <div class="rounded-md border border-white/10 bg-white/[0.02] p-3">
            <div class="flex items-center justify-between gap-3">
              <span class="text-xs uppercase tracking-widest text-(--color-oa-ink-dim)">
                Cheat search
                {searchActive() && searchSummary()
                  ? ` · ${searchSummary()!.candidateCount} candidates`
                  : ""}
              </span>
              <Show
                when={!searchActive()}
                fallback={
                  <button
                    type="button"
                    onClick={() => void endSearch()}
                    class="text-xs uppercase tracking-wider text-(--color-oa-ink-dim) hover:text-(--color-oa-ink)"
                  >
                    End
                  </button>
                }
              >
                <div class="flex items-center gap-2">
                  <select
                    value={searchRegion()}
                    onChange={(e) => setSearchRegion(e.currentTarget.value)}
                    class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-xs text-(--color-oa-ink)"
                  >
                    <For each={CHEAT_REGION_OPTIONS}>
                      {(r) => <option value={r}>{r}</option>}
                    </For>
                  </select>
                  <button
                    type="button"
                    onClick={() => void startSearch()}
                    disabled={searchPending() || !gameId()}
                    class="rounded-md border border-(--color-system-accent)/40 bg-(--color-system-accent)/15 px-2 py-1 text-xs uppercase tracking-wider text-(--color-system-accent-soft) disabled:opacity-50"
                  >
                    Start search
                  </button>
                </div>
              </Show>
            </div>

            <Show when={searchActive()}>
              <div class="mt-3 flex flex-wrap items-center gap-2">
                <button
                  type="button"
                  disabled={searchPending()}
                  onClick={() => void runFilter({ kind: "changed" })}
                  class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-xs text-(--color-oa-ink) hover:bg-white/[0.08] disabled:opacity-50"
                >
                  ≠ Changed
                </button>
                <button
                  type="button"
                  disabled={searchPending()}
                  onClick={() => void runFilter({ kind: "unchanged" })}
                  class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-xs text-(--color-oa-ink) hover:bg-white/[0.08] disabled:opacity-50"
                >
                  = Unchanged
                </button>
                <button
                  type="button"
                  disabled={searchPending()}
                  onClick={() => void runFilter({ kind: "increased" })}
                  class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-xs text-(--color-oa-ink) hover:bg-white/[0.08] disabled:opacity-50"
                >
                  ▲ Increased
                </button>
                <button
                  type="button"
                  disabled={searchPending()}
                  onClick={() => void runFilter({ kind: "decreased" })}
                  class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-xs text-(--color-oa-ink) hover:bg-white/[0.08] disabled:opacity-50"
                >
                  ▼ Decreased
                </button>
                <span class="text-xs text-(--color-oa-ink-dim)">or</span>
                <input
                  type="number"
                  min="0"
                  max="255"
                  value={searchEqualValue()}
                  onInput={(e) => {
                    const n = Number(e.currentTarget.value);
                    if (Number.isFinite(n)) setSearchEqualValue(n);
                  }}
                  class="w-16 rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-xs font-mono text-(--color-oa-ink)"
                />
                <button
                  type="button"
                  disabled={searchPending()}
                  onClick={() =>
                    void runFilter({ kind: "equal_to_value", value: searchEqualValue() })
                  }
                  class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-xs text-(--color-oa-ink) hover:bg-white/[0.08] disabled:opacity-50"
                >
                  = value
                </button>
                <button
                  type="button"
                  onClick={() => void peek()}
                  class="ml-auto rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-xs uppercase tracking-wider text-(--color-oa-ink-dim) hover:text-(--color-oa-ink)"
                >
                  ↻ Refresh
                </button>
              </div>

              <Show
                when={searchSummary() && searchSummary()!.top.length > 0}
                fallback={
                  <p class="mt-2 text-xs text-(--color-oa-ink-dim)">
                    Do something in-game that changes the value you're looking
                    for, then pick a filter above.
                  </p>
                }
              >
                <div class="mt-3 flex max-h-64 flex-col gap-1 overflow-y-auto">
                  <For each={searchSummary()!.top}>
                    {(hit) => (
                      <div class="flex items-center gap-2 rounded border border-white/5 bg-white/[0.02] px-2 py-1.5 text-xs font-mono">
                        <span class="text-(--color-oa-ink-dim)">
                          0x
                          {hit.offset.toString(16).toUpperCase().padStart(4, "0")}
                        </span>
                        <span class="text-(--color-oa-ink)">= {hit.currentValue}</span>
                        <span class="text-(--color-oa-ink-dim)">
                          (was {hit.previousValue})
                        </span>
                        <button
                          type="button"
                          onClick={() => makeCheatFrom(hit.offset, hit.currentValue)}
                          class="ml-auto rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-xs uppercase tracking-wider text-(--color-oa-ink-dim) hover:text-(--color-oa-ink)"
                        >
                          Make cheat
                        </button>
                      </div>
                    )}
                  </For>
                  <Show
                    when={searchSummary()!.candidateCount > searchSummary()!.top.length}
                  >
                    <p class="text-xs text-(--color-oa-ink-dim)">
                      + {searchSummary()!.candidateCount - searchSummary()!.top.length} more
                      — apply another filter to narrow.
                    </p>
                  </Show>
                </div>
              </Show>
            </Show>
          </div>

          <Show when={cheats().length === 0 && draft() === null}>
            <div class="rounded-md border border-white/5 bg-white/[0.02] p-3 text-xs text-(--color-oa-ink-dim)">
              No cheats configured for this game.
            </div>
          </Show>

          <For each={cheats()}>
            {(c) => (
              <div class="flex items-center gap-2 rounded-md border border-white/10 bg-white/[0.03] p-2.5">
                <input
                  type="checkbox"
                  checked={c.enabled}
                  onChange={() => void toggle(c)}
                  title={c.enabled ? "Enabled" : "Disabled"}
                />
                <div class="min-w-0 flex-1">
                  <p class="truncate text-sm text-(--color-oa-ink)">{c.name || "(unnamed)"}</p>
                  {/* Subtitle layout differs per cheat kind: memory_poke
                      shows the raw (region · offset · width · value)
                      tuple; code formats show the format label + a
                      truncated preview of the code string. */}
                  <Show
                    when={c.kind === "memory_poke"}
                    fallback={
                      <p class="truncate text-xs uppercase tracking-widest text-(--color-oa-ink-dim)">
                        {labelForKind(c.kind)} · {c.code ?? ""}
                      </p>
                    }
                  >
                    <p class="text-xs uppercase tracking-widest text-(--color-oa-ink-dim)">
                      {c.region} · 0x
                      {c.offset.toString(16).toUpperCase().padStart(4, "0")} · {c.width}B · ={" "}
                      {c.value}
                    </p>
                  </Show>
                </div>
                <button
                  type="button"
                  onClick={() => setDraft({ ...c })}
                  class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-xs text-(--color-oa-ink) hover:bg-white/[0.08]"
                >
                  Edit
                </button>
                <button
                  type="button"
                  onClick={() => c.id && void remove(c.id)}
                  class="rounded border border-red-500/30 px-2 py-1 text-xs text-red-300 hover:bg-red-500/10"
                >
                  ✕
                </button>
              </div>
            )}
          </For>

          <Show
            when={draft() !== null}
            fallback={
              <button
                type="button"
                onClick={() => setDraft(blank())}
                class="self-start rounded-md border border-(--color-system-accent)/40 bg-(--color-system-accent)/15 px-3 py-1.5 text-xs uppercase tracking-wider text-(--color-system-accent-soft) hover:bg-(--color-system-accent)/25"
              >
                + Add cheat
              </button>
            }
          >
            {(_) => {
              const d = draft()!;
              return (
                <div class="rounded-md border border-(--color-system-accent)/30 bg-white/[0.02] p-3">
                  <div class="grid grid-cols-2 gap-2">
                    <label class="col-span-2 flex flex-col gap-1 text-xs">
                      <span class="uppercase tracking-widest text-(--color-oa-ink-dim)">Type</span>
                      <select
                        value={d.kind}
                        onChange={(e) => setDraft({ ...d, kind: e.currentTarget.value })}
                        class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-sm text-(--color-oa-ink)"
                      >
                        {/* Per-system formats from list_cheat_formats —
                            Memory poke first, named formats (Game Genie /
                            GameShark / Action Replay / etc.) in the middle,
                            generic libretro_code catch-all last. Systems
                            without named formats still get the 2-entry
                            minimum so the picker is never empty. */}
                        <For each={cheatFormats()}>
                          {(f) => <option value={f.id}>{f.label}</option>}
                        </For>
                      </select>
                    </label>
                    <label class="col-span-2 flex flex-col gap-1 text-xs">
                      <span class="uppercase tracking-widest text-(--color-oa-ink-dim)">Name</span>
                      <input
                        type="text"
                        value={d.name}
                        onInput={(e) => setDraft({ ...d, name: e.currentTarget.value })}
                        class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-sm text-(--color-oa-ink)"
                        placeholder="Infinite lives"
                      />
                    </label>

                    <Show
                      when={d.kind !== "memory_poke"}
                      fallback={
                        <>
                          <label class="flex flex-col gap-1 text-xs">
                            <span class="uppercase tracking-widest text-(--color-oa-ink-dim)">
                              Region
                            </span>
                            <select
                              value={d.region}
                              onChange={(e) => setDraft({ ...d, region: e.currentTarget.value })}
                              class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-sm text-(--color-oa-ink)"
                            >
                              <For each={CHEAT_REGION_OPTIONS}>
                                {(r) => <option value={r}>{r}</option>}
                              </For>
                            </select>
                          </label>
                          <label class="flex flex-col gap-1 text-xs">
                            <span class="uppercase tracking-widest text-(--color-oa-ink-dim)">
                              Width
                            </span>
                            <select
                              value={String(d.width)}
                              onChange={(e) =>
                                setDraft({ ...d, width: Number(e.currentTarget.value) })
                              }
                              class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-sm text-(--color-oa-ink)"
                            >
                              <option value="1">1 byte</option>
                              <option value="2">2 bytes</option>
                              <option value="4">4 bytes</option>
                            </select>
                          </label>
                          <label class="flex flex-col gap-1 text-xs">
                            <span class="uppercase tracking-widest text-(--color-oa-ink-dim)">
                              Offset (hex)
                            </span>
                            <input
                              type="text"
                              value={"0x" + d.offset.toString(16).toUpperCase()}
                              onChange={(e) => {
                                const raw = e.currentTarget.value.trim().replace(/^0x/i, "");
                                const n = parseInt(raw, 16);
                                if (Number.isFinite(n)) setDraft({ ...d, offset: n });
                              }}
                              class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-sm font-mono text-(--color-oa-ink)"
                            />
                          </label>
                          <label class="flex flex-col gap-1 text-xs">
                            <span class="uppercase tracking-widest text-(--color-oa-ink-dim)">
                              Value (decimal)
                            </span>
                            <input
                              type="number"
                              value={d.value}
                              onInput={(e) => {
                                const n = Number(e.currentTarget.value);
                                if (Number.isFinite(n)) setDraft({ ...d, value: n });
                              }}
                              class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-sm font-mono text-(--color-oa-ink)"
                            />
                          </label>
                        </>
                      }
                    >
                      <label class="col-span-2 flex flex-col gap-1 text-xs">
                        <span class="uppercase tracking-widest text-(--color-oa-ink-dim)">
                          {labelForKind(d.kind)} code
                        </span>
                        <input
                          type="text"
                          value={d.code ?? ""}
                          onInput={(e) => setDraft({ ...d, code: e.currentTarget.value })}
                          class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-sm font-mono text-(--color-oa-ink)"
                          placeholder={
                            cheatFormats().find((f) => f.id === d.kind)?.hint ??
                            "Paste the cheat code"
                          }
                        />
                        {/* Per-format validation feedback — inline error on
                            malformed input, generic hint when valid. Empty
                            validation_regex (memory_poke + libretro_code
                            generic) shows the hint with no error path. */}
                        <Show
                          when={validateCode(d.kind, d.code)}
                          fallback={
                            <span class="text-xs text-(--color-oa-ink-dim)">
                              {cheatFormats().find((f) => f.id === d.kind)?.hint ??
                                "Format is decided by the core for this system. Multiple codes joined with +."}
                            </span>
                          }
                        >
                          {(err) => (
                            <span class="text-xs text-(--color-oa-warn, theme(colors.amber.300))">
                              {err()}
                            </span>
                          )}
                        </Show>
                      </label>
                    </Show>
                  </div>
                  <div class="mt-3 flex gap-2">
                    <button
                      type="button"
                      onClick={() => void save(d)}
                      disabled={
                        !d.name.trim() ||
                        (d.kind !== "memory_poke" && validateCode(d.kind, d.code) !== null)
                      }
                      class="rounded-md border border-(--color-system-accent)/40 bg-(--color-system-accent)/15 px-3 py-1 text-xs uppercase tracking-wider text-(--color-system-accent-soft) disabled:opacity-50"
                    >
                      {d.id ? "Save" : "Add"}
                    </button>
                    <button
                      type="button"
                      onClick={() => setDraft(null)}
                      class="rounded-md border border-white/10 bg-white/[0.04] px-3 py-1 text-xs uppercase tracking-wider text-(--color-oa-ink-dim) hover:text-(--color-oa-ink)"
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              );
            }}
          </Show>
        </div>
      </Show>
    </Dialog>
  );
};

// --- Discriminated state union (consumed by App.tsx) -------------------

export type GameDialogKind =
  | "core-options"
  | "display"
  | "input"
  | "rewind"
  | "shaders"
  | "milestones"
  | "cheats";

export type GameDialogState = { kind: GameDialogKind; target: RomEntry } | null;
