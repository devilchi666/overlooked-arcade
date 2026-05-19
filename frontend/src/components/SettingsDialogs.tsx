// OA-wide settings dialogs launched from the Settings menu.
//
// Each dialog is a small focused surface that mutates one slice of the
// settings store. The store handles persistence + emu-thread sync via
// createEffect; the dialogs just bind inputs to signals. Resource queries
// (monitors, audio devices) fire on dialog mount — keeps the App from
// having to lift those lists into shared state.

import {
  createResource,
  For,
  type Component,
} from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { Dialog } from "../layout/Dialog";
import {
  SCALING_MODE_LABELS,
  SCALING_OPTIONS,
  SHELL_MODE_LABELS,
  SHELL_OPTIONS,
  WINDOW_MODE_LABELS,
  WINDOW_OPTIONS,
  type AudioDeviceInfo,
  type MonitorInfo,
  type ScalingMode,
  type SettingsStore,
  type ShellMode,
  type WindowMode,
} from "../settings/store";
import { shaderPresets, shaderPresetLabel } from "../settings/shader_presets";

const SELECT_CLASS =
  "w-full rounded-md border border-white/10 bg-white/[0.04] px-3 py-2 text-sm font-medium text-(--color-oa-ink) transition hover:bg-white/[0.08] focus-visible:outline focus-visible:outline-2 focus-visible:outline-(--color-oa-ink-dim)";

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

  return (
    <Dialog open={props.open} onClose={props.onClose} title="Display" subtitle="OA-wide" size="md">
      <div class="flex flex-col gap-3">
        <label class="block space-y-1">
          <span class="text-xs text-(--color-oa-ink-dim)">Scaling mode</span>
          <select
            value={props.settings.scalingMode()}
            onChange={(e) =>
              props.settings.setScalingMode(e.currentTarget.value as ScalingMode)
            }
            class={SELECT_CLASS}
          >
            <For each={SCALING_OPTIONS}>
              {(m) => <option value={m}>{SCALING_MODE_LABELS[m]}</option>}
            </For>
          </select>
        </label>

        <label class="block space-y-1">
          <span class="text-xs text-(--color-oa-ink-dim)">Window mode</span>
          <select
            value={props.settings.windowMode()}
            onChange={(e) =>
              props.settings.setWindowMode(e.currentTarget.value as WindowMode)
            }
            class={SELECT_CLASS}
          >
            <For each={WINDOW_OPTIONS}>
              {(m) => <option value={m}>{WINDOW_MODE_LABELS[m]}</option>}
            </For>
          </select>
        </label>

        <label class="block space-y-1">
          <span class="text-xs text-(--color-oa-ink-dim)">Monitor (for borderless)</span>
          <select
            value={
              props.settings.monitorIndex() === null
                ? "current"
                : String(props.settings.monitorIndex())
            }
            onChange={(e) => {
              const v = e.currentTarget.value;
              props.settings.setMonitorIndex(v === "current" ? null : Number(v));
            }}
            class={SELECT_CLASS}
          >
            <option value="current">Current monitor</option>
            <For each={monitors() ?? []}>
              {(m) => <option value={String(m.index)}>{monitorLabel(m)}</option>}
            </For>
          </select>
        </label>

        <label class="block space-y-1">
          <span class="text-xs text-(--color-oa-ink-dim)">Run-ahead frames</span>
          <div class="flex items-center gap-3">
            <input
              type="range"
              min="0"
              max="5"
              step="1"
              value={props.settings.runAheadFrames()}
              onInput={(e) => {
                const v = Number(e.currentTarget.value);
                if (Number.isInteger(v)) props.settings.setRunAheadFrames(v);
              }}
              class="flex-1"
            />
            <span class="font-mono text-sm w-12 text-right tabular-nums">
              {props.settings.runAheadFrames() === 0
                ? "off"
                : `+${props.settings.runAheadFrames()}f`}
            </span>
          </div>
          <span class="block text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            Reduces perceived input latency by N frames. Each costs 1 save_state + 1 run_frame + 1
            load_state per frame. Skipped during scrub / TAS / pause.
          </span>
        </label>
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

  return (
    <Dialog open={props.open} onClose={props.onClose} title="Audio" subtitle="OA-wide" size="sm">
      <label class="block space-y-1">
        <span class="text-xs text-(--color-oa-ink-dim)">Output device</span>
        <select
          value={props.settings.audioDevice() ?? "__default__"}
          onChange={(e) => {
            const v = e.currentTarget.value;
            props.settings.setAudioDevice(v === "__default__" ? null : v);
          }}
          class={SELECT_CLASS}
        >
          <option value="__default__">System default</option>
          <For each={audioDevices() ?? []}>
            {(d) => (
              <option value={d.name}>
                {d.name}
                {d.isDefault ? " (default)" : ""}
              </option>
            )}
          </For>
        </select>
        <span class="block text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
          Takes effect immediately — the running stream swaps in place.
        </span>
      </label>
    </Dialog>
  );
};

// --- Gameplay (rewind) --------------------------------------------------

export const GameplayDialog: Component<{
  open: boolean;
  onClose: () => void;
  settings: SettingsStore;
}> = (props) => (
  <Dialog open={props.open} onClose={props.onClose} title="Gameplay" subtitle="OA-wide rewind defaults" size="md">
    <div class="space-y-3">
      <p class="text-xs leading-relaxed text-(--color-oa-ink-dim)">
        Hold <kbd class="rounded border border-white/15 bg-white/[0.04] px-1.5 py-0.5 font-mono text-[0.65rem] text-(--color-oa-ink)">Backspace</kbd>{" "}
        during gameplay to walk emulation backwards. Snapshots captured every N frames into a
        memory-bounded ring; older snapshots evict when the cap is reached.
      </p>

      <label class="flex cursor-pointer items-center gap-3 rounded-md border border-white/10 bg-white/[0.03] px-3 py-2.5 text-sm transition hover:bg-white/[0.06]">
        <input
          type="checkbox"
          checked={props.settings.rewindEnabled()}
          onChange={(e) => props.settings.setRewindEnabled(e.currentTarget.checked)}
          class="size-4 cursor-pointer accent-(--color-system-accent)"
        />
        <span class="flex-1">
          <span class="block text-(--color-oa-ink)">Enable rewind</span>
          <span class="block text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            Captures save-state snapshots while playing
          </span>
        </span>
      </label>

      <label class="block space-y-1">
        <span class="text-xs text-(--color-oa-ink-dim)">Capture interval (frames between snapshots)</span>
        <select
          value={String(props.settings.rewindCaptureIntervalFrames())}
          onChange={(e) =>
            props.settings.setRewindCaptureIntervalFrames(Number(e.currentTarget.value))
          }
          class={SELECT_CLASS}
          disabled={!props.settings.rewindEnabled()}
        >
          <For each={REWIND_INTERVAL_OPTIONS}>
            {(n) => (
              <option value={String(n)}>
                {n === 1 ? "Every frame" : `Every ${n} frames`} (~{Math.round((n / 60) * 1000)} ms at 60 fps)
              </option>
            )}
          </For>
        </select>
        <span class="block text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
          Lower = smoother rewind, more CPU + RAM. Higher = coarser rewind, cheaper.
        </span>
      </label>

      <label class="block space-y-1">
        <span class="text-xs text-(--color-oa-ink-dim)">Buffer cap</span>
        <select
          value={String(props.settings.rewindBufferMegabytes())}
          onChange={(e) =>
            props.settings.setRewindBufferMegabytes(Number(e.currentTarget.value))
          }
          class={SELECT_CLASS}
          disabled={!props.settings.rewindEnabled()}
        >
          <For each={REWIND_BUFFER_OPTIONS}>
            {(mb) => <option value={String(mb)}>{mb} MB</option>}
          </For>
        </select>
        <span class="block text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
          Hard cap on RAM. Cores with large save states (SNES ~300 KB / snap) get fewer
          seconds of history per MB.
        </span>
      </label>
    </div>
  </Dialog>
);

// --- Shaders ------------------------------------------------------------

export const ShadersDialog: Component<{
  open: boolean;
  onClose: () => void;
  settings: SettingsStore;
}> = (props) => (
  <Dialog open={props.open} onClose={props.onClose} title="Shaders" subtitle="OA-wide preset + bloom" size="md">
    <div class="flex flex-col gap-3">
      <label class="block space-y-1">
        <span class="text-xs text-(--color-oa-ink-dim)">Shader preset</span>
        <select
          value={props.settings.shaderPreset()}
          onChange={(e) => props.settings.setShaderPreset(e.currentTarget.value)}
          class={SELECT_CLASS}
        >
          <option value="system-default">{shaderPresetLabel("system-default")}</option>
          <For each={shaderPresets()}>
            {(p) => <option value={p.name}>{p.displayName}</option>}
          </For>
        </select>
        <span class="block text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
          Applies during the final blit. Per-system + per-game overrides win at launch.
        </span>
      </label>

      <label class="block space-y-1">
        <span class="text-xs text-(--color-oa-ink-dim)">Phosphor bloom amount</span>
        <div class="flex items-center gap-3">
          <input
            type="range"
            min="0"
            max="1"
            step="0.05"
            value={props.settings.bloomAmount()}
            onInput={(e) => {
              const v = Number(e.currentTarget.value);
              if (Number.isFinite(v)) props.settings.setBloomAmount(v);
            }}
            class="flex-1"
          />
          <span class="font-mono text-sm w-12 text-right tabular-nums">
            {props.settings.bloomAmount().toFixed(2)}
          </span>
        </div>
        <span class="block text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
          OA-wide source/blur mix. Only meaningful when the active preset's base is Phosphor.
          Per-system + per-game overrides take precedence.
        </span>
      </label>
    </div>
  </Dialog>
);

// --- Shell mode label helper (Settings menu uses MenuRadio directly) ----

export { SHELL_MODE_LABELS, SHELL_OPTIONS };
export type { ShellMode };
