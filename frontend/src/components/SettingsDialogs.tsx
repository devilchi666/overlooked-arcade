// OA-wide settings dialogs launched from the Settings menu.
//
// Each dialog is a small focused surface that mutates one slice of the
// settings store. The store handles persistence + emu-thread sync via
// createEffect; the dialogs just bind inputs to signals. Resource queries
// (monitors, audio devices) fire on dialog mount — keeps the App from
// having to lift those lists into shared state.

import {
  createMemo,
  createResource,
  type Component,
} from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { Dialog, DialogSection } from "../layout/Dialog";
import SettingRow from "./SettingRow";
import {
  SCALING_MODE_LABELS,
  SCALING_OPTIONS,
  SHELL_MODE_LABELS,
  SHELL_OPTIONS,
  WINDOW_MODE_LABELS,
  WINDOW_OPTIONS,
  type AudioDeviceInfo,
  type ControllerNavSource,
  type MonitorInfo,
  type ScalingMode,
  type SettingsStore,
  type ShellMode,
  type WindowMode,
} from "../settings/store";
import { shaderPresets, shaderPresetLabel } from "../settings/shader_presets";

const REWIND_INTERVAL_OPTIONS: readonly number[] = [1, 2, 3, 6, 10, 15, 30];
const REWIND_BUFFER_OPTIONS: readonly number[] = [8, 16, 32, 64, 128, 256, 512];

function monitorLabel(m: MonitorInfo): string {
  const name = m.name?.trim() || `Monitor ${m.index + 1}`;
  return `${name} (${m.width}×${m.height})`;
}

// --- Display ------------------------------------------------------------

export const DisplayDialog: Component<{
  open: boolean;
  onClose: () => void;
  settings: SettingsStore;
}> = (props) => {
  const [monitors] = createResource(async (): Promise<MonitorInfo[]> => {
    try {
      return await invoke<MonitorInfo[]>("list_monitors");
    } catch {
      return [];
    }
  });

  const scalingOptions = SCALING_OPTIONS.map((m) => ({
    value: m,
    label: SCALING_MODE_LABELS[m],
  }));
  const windowOptions = WINDOW_OPTIONS.map((m) => ({
    value: m,
    label: WINDOW_MODE_LABELS[m],
  }));
  const monitorOptions = createMemo(() => [
    { value: "current", label: "Current monitor" },
    ...(monitors() ?? []).map((m) => ({
      value: String(m.index),
      label: monitorLabel(m),
    })),
  ]);

  return (
    <Dialog open={props.open} onClose={props.onClose} title="Display" subtitle="OA-wide" size="xl">
      <div class="flex flex-col gap-5">
        <DialogSection title="Scaling">
          <SettingRow
            label="Scaling mode"
            inherited={null}
            overridden={false}
            select={{
              value: props.settings.scalingMode(),
              options: scalingOptions,
              onChange: (v) => props.settings.setScalingMode(v as ScalingMode),
            }}
          />
        </DialogSection>

        <DialogSection title="Window">
          <SettingRow
            label="Window mode"
            inherited={null}
            overridden={false}
            select={{
              value: props.settings.windowMode(),
              options: windowOptions,
              onChange: (v) => props.settings.setWindowMode(v as WindowMode),
            }}
          />
          <SettingRow
            label="Monitor"
            hint="For borderless"
            inherited={null}
            overridden={false}
            select={{
              value:
                props.settings.monitorIndex() === null
                  ? "current"
                  : String(props.settings.monitorIndex()),
              options: monitorOptions(),
              onChange: (v) =>
                props.settings.setMonitorIndex(v === "current" ? null : Number(v)),
            }}
          />
        </DialogSection>

        <DialogSection title="Run-ahead">
          <SettingRow
            label="Run-ahead frames"
            inherited={null}
            overridden={false}
            slider={{
              min: 0,
              max: 5,
              step: 1,
              value: props.settings.runAheadFrames(),
              format: (v) => (v === 0 ? "off" : `+${v}f`),
              onInput: (v) => {
                if (Number.isInteger(v)) props.settings.setRunAheadFrames(v);
              },
            }}
            description="Reduces perceived input latency by N frames. Each costs one save_state + one run_frame + one load_state per frame. Skipped during scrub / TAS / pause."
          />
        </DialogSection>

        <DialogSection
          title="Per-system experiences"
          description="Each system in your library can feel like its own mini-experience — per-system audio, boot animations, tile flourishes, backgrounds. Disable for a uniform plain library across every system. Stage 1 of the per-system-ui pipelined arc; later stages add per-system layout + behavior."
        >
          <SettingRow
            label="Enabled"
            inherited={null}
            overridden={false}
            toggle={{
              checked: props.settings.perSystemUiEnabled(),
              onChange: (v) => props.settings.setPerSystemUiEnabled(v),
            }}
            description="When off, every system shares one neutral library look — no per-system audio, no boot animations, no per-system flourishes. Cover art and tiles still show; only the per-system character disappears."
          />
        </DialogSection>

        <DialogSection
          title="Controller navigation"
          description="Drive the library and menus with a gamepad. DPad or left stick moves focus; A activates, B cancels, X opens context menus, Y opens details, shoulder bumpers switch between panels."
        >
          <SettingRow
            label="Enabled"
            inherited={null}
            overridden={false}
            toggle={{
              checked: props.settings.controllerNavEnabled(),
              onChange: (v) => props.settings.setControllerNavEnabled(v),
            }}
          />
          <SettingRow
            label="Navigation source"
            inherited={null}
            overridden={false}
            select={{
              value: props.settings.controllerNavSource(),
              options: [
                { value: "both", label: "DPad + left stick" },
                { value: "dpad", label: "DPad only" },
                { value: "stick-left", label: "Left stick only" },
              ],
              onChange: (v) => props.settings.setControllerNavSource(v as ControllerNavSource),
            }}
          />
          <SettingRow
            label="Swap A and B"
            hint="Nintendo layout"
            inherited={null}
            overridden={false}
            toggle={{
              checked: props.settings.controllerNavSwapAB(),
              onChange: (v) => props.settings.setControllerNavSwapAB(v),
            }}
            description="When on, B confirms and A cancels — matches SNES/N64/DS-era Nintendo conventions."
          />
          <SettingRow
            label="Animation budget"
            inherited={null}
            overridden={false}
            select={{
              value: String(props.settings.controllerNavAnimationMs()),
              options: [
                { value: "0", label: "Snappy (no animation)" },
                { value: "120", label: "Subtle (120 ms)" },
                { value: "250", label: "Animated (250 ms)" },
              ],
              onChange: (v) => {
                const ms = Number(v);
                if (Number.isFinite(ms)) props.settings.setControllerNavAnimationMs(ms);
              },
            }}
            description="How long the focus ring transitions when moving between elements."
          />
        </DialogSection>
      </div>
    </Dialog>
  );
};

// --- Audio --------------------------------------------------------------

export const AudioDialog: Component<{
  open: boolean;
  onClose: () => void;
  settings: SettingsStore;
}> = (props) => {
  const [audioDevices] = createResource(async (): Promise<AudioDeviceInfo[]> => {
    try {
      return await invoke<AudioDeviceInfo[]>("list_audio_devices");
    } catch {
      return [];
    }
  });

  const deviceOptions = createMemo(() => [
    { value: "__default__", label: "System default" },
    ...(audioDevices() ?? []).map((d) => ({
      value: d.name,
      label: `${d.name}${d.isDefault ? " (default)" : ""}`,
    })),
  ]);

  return (
    <Dialog open={props.open} onClose={props.onClose} title="Audio" subtitle="OA-wide" size="sm">
      <SettingRow
        label="Output device"
        inherited={null}
        overridden={false}
        select={{
          value: props.settings.audioDevice() ?? "__default__",
          options: deviceOptions(),
          onChange: (v) => props.settings.setAudioDevice(v === "__default__" ? null : v),
        }}
        description="Takes effect immediately — the running stream swaps in place."
      />
    </Dialog>
  );
};

// --- Gameplay (rewind) --------------------------------------------------

export const GameplayDialog: Component<{
  open: boolean;
  onClose: () => void;
  settings: SettingsStore;
}> = (props) => {
  const intervalOptions = REWIND_INTERVAL_OPTIONS.map((n) => ({
    value: String(n),
    label: `${n === 1 ? "Every frame" : `Every ${n} frames`} (~${Math.round((n / 60) * 1000)} ms at 60 fps)`,
  }));
  const bufferOptions = REWIND_BUFFER_OPTIONS.map((mb) => ({
    value: String(mb),
    label: `${mb} MB`,
  }));

  return (
    <Dialog open={props.open} onClose={props.onClose} title="Gameplay" subtitle="OA-wide rewind defaults" size="md">
      <DialogSection
        title="Rewind"
        description="Hold Backspace during gameplay to walk emulation backwards. Snapshots captured every N frames into a memory-bounded ring; older snapshots evict when the cap is reached."
      >
        <SettingRow
          label="Enable rewind"
          inherited={null}
          overridden={false}
          toggle={{
            checked: props.settings.rewindEnabled(),
            onChange: (v) => props.settings.setRewindEnabled(v),
          }}
          description="Captures save-state snapshots while playing."
        />
        <SettingRow
          label="Capture interval"
          inherited={null}
          overridden={false}
          disabled={!props.settings.rewindEnabled()}
          select={{
            value: String(props.settings.rewindCaptureIntervalFrames()),
            options: intervalOptions,
            onChange: (v) => props.settings.setRewindCaptureIntervalFrames(Number(v)),
          }}
          description="Lower = smoother rewind, more CPU + RAM. Higher = coarser rewind, cheaper."
        />
        <SettingRow
          label="Buffer cap"
          inherited={null}
          overridden={false}
          disabled={!props.settings.rewindEnabled()}
          select={{
            value: String(props.settings.rewindBufferMegabytes()),
            options: bufferOptions,
            onChange: (v) => props.settings.setRewindBufferMegabytes(Number(v)),
          }}
          description="Hard cap on RAM. Cores with large save states (SNES ~300 KB / snap) get fewer seconds of history per MB."
        />
      </DialogSection>
    </Dialog>
  );
};

// --- Shaders ------------------------------------------------------------

export const ShadersDialog: Component<{
  open: boolean;
  onClose: () => void;
  settings: SettingsStore;
}> = (props) => {
  const presetOptions = createMemo(() => [
    { value: "system-default", label: shaderPresetLabel("system-default") },
    ...shaderPresets().map((p) => ({ value: p.name, label: p.displayName })),
  ]);

  return (
    <Dialog open={props.open} onClose={props.onClose} title="Shaders" subtitle="OA-wide preset + bloom" size="md">
      <div class="flex flex-col gap-4">
        <SettingRow
          label="Shader preset"
          inherited={null}
          overridden={false}
          select={{
            value: props.settings.shaderPreset(),
            options: presetOptions(),
            onChange: (v) => props.settings.setShaderPreset(v),
          }}
          description="Applies during the final blit. Per-system + per-game overrides win at launch."
        />
        <SettingRow
          label="Phosphor bloom amount"
          inherited={null}
          overridden={false}
          slider={{
            min: 0,
            max: 1,
            step: 0.05,
            value: props.settings.bloomAmount(),
            onInput: (v) => props.settings.setBloomAmount(v),
          }}
          description="OA-wide source/blur mix. Only meaningful when the active preset's base is Phosphor. Per-system + per-game overrides take precedence."
        />
      </div>
    </Dialog>
  );
};

// --- Shell mode label helper (Settings menu uses MenuRadio directly) ----

export { SHELL_MODE_LABELS, SHELL_OPTIONS };
export type { ShellMode };
