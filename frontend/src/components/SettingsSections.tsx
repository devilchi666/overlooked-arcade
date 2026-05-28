// Reusable settings section bodies for Retroverse-UI Phase C1.
//
// Each section component renders one logical settings surface without
// the Dialog wrapper. SettingsPage embeds these directly into its
// center pane (one per category); the legacy modal SettingsDialogs.tsx
// can later switch to importing from here too (deferred — the parallel-
// file approach keeps Phase C1 risk low).
//
// Section content + behaviour is bit-for-bit identical to the
// corresponding DialogSection blocks in SettingsDialogs.tsx — same
// SettingRow inputs, same store bindings, same descriptions, same
// createResource queries. Only the outer wrapper differs (Card vs
// DialogSection — both visually similar).
//
// See docs/PLANS/settings-tab-retroverse.md for the design + the
// rollout plan at docs/PLANS/retroverse-ui-rollout.md.

import {
  createMemo,
  createResource,
  Show,
  type Component,
  type JSX,
} from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import SettingRow from "./SettingRow";
import {
  SCALING_MODE_LABELS,
  SCALING_OPTIONS,
  WINDOW_MODE_LABELS,
  WINDOW_OPTIONS,
  type AudioDeviceInfo,
  type ControllerNavSource,
  type MonitorInfo,
  type ScalingMode,
  type SettingsStore,
  type WindowMode,
} from "../settings/store";
import { shaderPresets, shaderPresetLabel } from "../settings/shader_presets";

const REWIND_INTERVAL_OPTIONS: readonly number[] = [1, 2, 3, 6, 10, 15, 30];
const REWIND_BUFFER_OPTIONS: readonly number[] = [8, 16, 32, 64, 128, 256, 512];

function monitorLabel(m: MonitorInfo): string {
  const name = m.name?.trim() || `Monitor ${m.index + 1}`;
  return `${name} (${m.width}×${m.height})`;
}

// Light card wrapper used to group rows inside the SettingsPage center
// pane. Cheap visual parity with DialogSection without depending on
// the Dialog primitive. Title is optional — a card with no title is
// just a rounded container.
const SettingsCard: Component<{
  title?: string;
  description?: string;
  children: JSX.Element;
}> = (props) => {
  return (
    <section class="rounded-xl border border-white/10 bg-white/[0.02] p-5">
      <Show when={props.title}>
        <header class="mb-3">
          <h3 class="text-[0.7rem] font-semibold uppercase tracking-[0.3em] text-(--color-oa-ink)">
            {props.title}
          </h3>
          <Show when={props.description}>
            <p class="mt-1 text-[0.75rem] leading-relaxed text-(--color-oa-ink-dim)">
              {props.description}
            </p>
          </Show>
        </header>
      </Show>
      <div class="flex flex-col gap-3">{props.children}</div>
    </section>
  );
};

// --- Display (scaling / window / run-ahead) ----------------------------

export const DisplayBaseSettings: Component<{ settings: SettingsStore }> = (props) => {
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
    <div class="flex flex-col gap-4">
      <SettingsCard title="Scaling">
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
      </SettingsCard>

      <SettingsCard title="Window">
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
      </SettingsCard>

      <SettingsCard title="Run-ahead">
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
      </SettingsCard>
    </div>
  );
};

// --- Per-system UI -----------------------------------------------------

export const PerSystemUiSettings: Component<{ settings: SettingsStore }> = (props) => {
  return (
    <div class="flex flex-col gap-4">
      <SettingsCard
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
        <Show when={props.settings.perSystemUiEnabled()}>
          <SettingRow
            label="Boot animations"
            inherited={null}
            overridden={false}
            toggle={{
              checked: props.settings.bootAnimationsEnabled(),
              onChange: (v) => props.settings.setBootAnimationsEnabled(v),
            }}
            description="Brief overlay that plays when you enter a system from the sidebar — tints the library with the system's accent color for about a second. Disable for instant transitions with no overlay; re-enable to restore the full animation. The OS reduce-motion preference shortens the overlay to a 200 ms fade independently when active."
          />
        </Show>
      </SettingsCard>
    </div>
  );
};

// --- Controller navigation --------------------------------------------

export const ControllerNavSettings: Component<{ settings: SettingsStore }> = (props) => {
  return (
    <div class="flex flex-col gap-4">
      <SettingsCard
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
      </SettingsCard>
    </div>
  );
};

// --- Experimental (hosts the Retroverse master toggle) ----------------

export const ExperimentalSettings: Component<{ settings: SettingsStore }> = (props) => {
  return (
    <div class="flex flex-col gap-4">
      <SettingsCard
        title="Experimental"
        description="Preview-quality features still under active development. Safe to toggle; defaults preserve today's behavior. See docs/PLANS/retroverse-ui-rollout.md for the rollout plan."
      >
        <SettingRow
          label="Retroverse UI"
          inherited={null}
          overridden={false}
          toggle={{
            checked: props.settings.experimentalRetroverseUi(),
            onChange: (v) => props.settings.setExperimentalRetroverseUi(v),
          }}
          description="Top-toolbar tab IA (HOME / LIBRARY / COLLECTIONS / PLAY NOW / DISCOVER / SETTINGS) replacing today's sidebar-driven layout. Flipping this OFF returns to the legacy Shell layout immediately — no restart required."
        />
      </SettingsCard>
    </div>
  );
};

// --- Audio --------------------------------------------------------------

export const AudioSettings: Component<{ settings: SettingsStore }> = (props) => {
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
    <div class="flex flex-col gap-4">
      <SettingsCard title="Output device">
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
      </SettingsCard>
    </div>
  );
};

// --- Gameplay (rewind) -------------------------------------------------

export const GameplaySettings: Component<{ settings: SettingsStore }> = (props) => {
  const intervalOptions = REWIND_INTERVAL_OPTIONS.map((n) => ({
    value: String(n),
    label: `${n === 1 ? "Every frame" : `Every ${n} frames`} (~${Math.round((n / 60) * 1000)} ms at 60 fps)`,
  }));
  const bufferOptions = REWIND_BUFFER_OPTIONS.map((mb) => ({
    value: String(mb),
    label: `${mb} MB`,
  }));

  return (
    <div class="flex flex-col gap-4">
      <SettingsCard
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
      </SettingsCard>
    </div>
  );
};

// --- Shaders -----------------------------------------------------------

export const ShadersSettings: Component<{ settings: SettingsStore }> = (props) => {
  const presetOptions = createMemo(() => [
    { value: "system-default", label: shaderPresetLabel("system-default") },
    ...shaderPresets().map((p) => ({ value: p.name, label: p.displayName })),
  ]);

  return (
    <div class="flex flex-col gap-4">
      <SettingsCard title="Preset + bloom">
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
      </SettingsCard>
    </div>
  );
};
