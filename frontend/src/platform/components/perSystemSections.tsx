// Shared per-system override section components.
//
// Lifted out of SystemDialogs.tsx so both the legacy SystemSettingsDialog
// (System ▾ menu, SystemContextMenu launch path) and the Retroverse
// SETTINGS → Per-system surface render the same fields the same way.
// Each section is one declarative form chunk — no internal state — and
// takes the per-system overrides API plus the resources it needs
// (monitors / cores) as props so the parent owns the lazy-fetch
// decisions.

import { createEffect, createResource, createSignal, For, Show, type Accessor, type Component } from "solid-js";
import * as coresApi from "@oa/platform/api/coresApi";
import { getRewindState } from "@oa/platform/api/rewindTasApi";
import { getSystemSettings, setSystemSettings, setBloomAmount } from "@oa/platform/api/settingsApi";
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
} from "@oa/platform/settings/store";
import { shaderPresets, shaderPresetLabel } from "@oa/platform/settings/shader_presets";
import type { SystemId } from "@oa/platform/themes/registry";
import SettingRow, { selectClass } from "./SettingRow";

export type OverscanCropPrefs = {
  top: number;
  bottom: number;
  left: number;
  right: number;
};

export type PerSystemOverrides = {
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

/// True when every edge is zero — treated equivalent to "no crop set".
export function overscanIsZero(c: OverscanCropPrefs | null | undefined): boolean {
  if (!c) return true;
  return c.top === 0 && c.bottom === 0 && c.left === 0 && c.right === 0;
}

export function overscanLabel(c: OverscanCropPrefs | null | undefined): string {
  if (overscanIsZero(c)) return "No crop";
  return `T${c!.top} · B${c!.bottom} · L${c!.left} · R${c!.right}`;
}

export function pathBasename(p: string | null | undefined): string | null {
  if (!p) return null;
  const idx = Math.max(p.lastIndexOf("/"), p.lastIndexOf("\\"));
  return idx >= 0 ? p.slice(idx + 1) : p;
}

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

// --- Hook: per-system overrides resource + patch helper ---------------

export type PerSystemOverridesApi = {
  overrides: Accessor<PerSystemOverrides>;
  patch: (p: Partial<PerSystemOverrides>) => Promise<void>;
  refetch: () => void;
};

/// Fetches + writes the per-system override JSON. `active` gates the
/// fetch — pass a `() => props.open` style accessor so the resource
/// stays cold while the surface is closed. The hook re-fetches when
/// `systemId` changes so the new Retroverse picker can swap systems
/// without remounting the consumer components.
export function usePerSystemOverrides(args: {
  systemId: Accessor<SystemId | null>;
  active: Accessor<boolean>;
}): PerSystemOverridesApi {
  const source = () => ({
    sysId: args.systemId(),
    active: args.active(),
  });
  const [resource, { refetch }] = createResource(
    source,
    async (src): Promise<PerSystemOverrides> => {
      if (!src.active || !src.sysId) return {};
      try {
        return (await getSystemSettings<PerSystemOverrides>(src.sysId)) ?? {};
      } catch (e) {
        console.warn("[oa-per-system] get_system_settings failed:", e);
        return {};
      }
    },
  );
  const overrides = (): PerSystemOverrides => resource() ?? {};
  async function patch(p: Partial<PerSystemOverrides>) {
    const sysId = args.systemId();
    if (!sysId) return;
    const next: PerSystemOverrides = { ...overrides(), ...p };
    // Carry every existing field through generically; only strip
    // null/undefined + zero-overscan + empty strings. The PerSystemOverrides
    // TS type only enumerates ~11 fields but the Rust SystemSettings
    // struct has more (keyboard_passthrough / region_priority_override /
    // revision_priority_override / platform_music_path /
    // ui_sound_{click,navigate,back,launch,error,scroll_tick}). Hand-
    // listing dropped any field the type didn't know about — dormant
    // bug today (no frontend surface writes those fields) but the
    // moment one lands, every save through this surface would wipe
    // them. Same shape as the 2026-06-04 light-gun regression in
    // library_db.rs::set_game_overrides.
    const cleaned: Record<string, unknown> = { ...next };
    for (const k of Object.keys(cleaned)) {
      const v = cleaned[k];
      if (v == null) {
        delete cleaned[k];
      } else if (typeof v === "string" && v.trim() === "") {
        delete cleaned[k];
      }
    }
    if (
      cleaned.overscanCropOverride
      && overscanIsZero(cleaned.overscanCropOverride as OverscanCropPrefs)
    ) {
      delete cleaned.overscanCropOverride;
    }
    try {
      await setSystemSettings(sysId, cleaned);
      void refetch();
    } catch (e) {
      console.warn("[oa-per-system] set_system_settings failed:", e);
    }
  }
  return { overrides, patch, refetch };
}

// --- Section components ----------------------------------------------

type DisplayProps = {
  api: PerSystemOverridesApi;
  settings: SettingsStore;
  monitors: Accessor<MonitorInfo[]>;
};

export const PerSystemDisplaySection: Component<DisplayProps> = (props) => {
  const o = () => props.api.overrides();
  const inheritedScalingLabel = (): string =>
    SCALING_MODE_LABELS[props.settings.scalingMode()] ?? props.settings.scalingMode();
  const inheritedWindowLabel = (): string =>
    WINDOW_MODE_LABELS[props.settings.windowMode()] ?? props.settings.windowMode();
  const inheritedMonitorLabel = (): string => {
    const idx = props.settings.monitorIndex();
    if (idx === null) return "Current monitor";
    const m = (props.monitors() ?? []).find((mm) => mm.index === idx);
    if (!m) return `Monitor ${idx + 1}`;
    return m.name?.trim() || `Monitor ${idx + 1}`;
  };
  return (
    <div class="flex flex-col gap-3">
      <SettingRow
        label="Scaling mode"
        hint="How the framebuffer fills the window"
        inheritedValue={inheritedScalingLabel()}
        overridden={o().scalingOverride != null}
      >
        <select
          class={selectClass("system")}
          value={o().scalingOverride ?? ""}
          onChange={(e) => {
            const v = e.currentTarget.value;
            void props.api.patch({ scalingOverride: v === "" ? null : v });
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
        overridden={o().windowModeOverride != null}
      >
        <select
          class={selectClass("system")}
          value={o().windowModeOverride ?? ""}
          onChange={(e) => {
            const v = e.currentTarget.value;
            void props.api.patch({ windowModeOverride: v === "" ? null : v });
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
        overridden={o().monitorIndexOverride != null}
      >
        <select
          class={selectClass("system")}
          value={o().monitorIndexOverride?.toString() ?? ""}
          onChange={(e) => {
            const v = e.currentTarget.value;
            void props.api.patch({ monitorIndexOverride: v === "" ? null : Number(v) });
          }}
        >
          <option value="">— Use OA default —</option>
          <For each={props.monitors() ?? []}>
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
        overridden={o().displayAspectOverride != null}
      >
        <select
          class={selectClass("system")}
          value={o().displayAspectOverride?.toString() ?? ""}
          onChange={(e) => {
            const v = e.currentTarget.value;
            void props.api.patch({
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
        overridden={!overscanIsZero(o().overscanCropOverride)}
      >
        <OverscanEditor
          value={o().overscanCropOverride ?? { top: 0, bottom: 0, left: 0, right: 0 }}
          onChange={(next) => void props.api.patch({
            overscanCropOverride: overscanIsZero(next) ? null : next,
          })}
        />
      </SettingRow>

      <SettingRow
        label="Bezel image"
        hint="PNG / JPEG / WebP overlaid on top of the game pixels"
        inheritedValue="Use shader preset default"
        overridden={o().bezelImagePath != null}
      >
        <BezelPicker
          value={o().bezelImagePath ?? null}
          onChange={(path) => void props.api.patch({ bezelImagePath: path })}
        />
      </SettingRow>
    </div>
  );
};

type RewindProps = {
  api: PerSystemOverridesApi;
  settings: SettingsStore;
  /// Polling gate for the live stats display. Pass `true` while the
  /// containing surface is visible so RewindLiveStats's 2 Hz timer
  /// doesn't hammer when the panel is hidden.
  liveStatsActive: Accessor<boolean>;
};

export const PerSystemRewindSection: Component<RewindProps> = (props) => {
  const o = () => props.api.overrides();
  return (
    <div class="flex flex-col gap-3">
      <RewindLiveStats open={props.liveStatsActive()} />
      <SettingRow
        label="Enable rewind"
        hint="Hold Backspace during gameplay to step backwards"
        inheritedValue={props.settings.rewindEnabled() ? "On" : "Off"}
        overridden={o().rewindEnabled != null}
      >
        <select
          class={selectClass("system")}
          value={
            o().rewindEnabled == null ? "" : o().rewindEnabled ? "on" : "off"
          }
          onChange={(e) => {
            const v = e.currentTarget.value;
            void props.api.patch({ rewindEnabled: v === "" ? null : v === "on" });
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
        overridden={o().rewindCaptureIntervalFrames != null}
      >
        <select
          class={selectClass("system")}
          value={o().rewindCaptureIntervalFrames?.toString() ?? ""}
          onChange={(e) => {
            const v = e.currentTarget.value;
            void props.api.patch({ rewindCaptureIntervalFrames: v === "" ? null : Number(v) });
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
        overridden={o().rewindBufferMegabytes != null}
      >
        <select
          class={selectClass("system")}
          value={o().rewindBufferMegabytes?.toString() ?? ""}
          onChange={(e) => {
            const v = e.currentTarget.value;
            void props.api.patch({ rewindBufferMegabytes: v === "" ? null : Number(v) });
          }}
        >
          <option value="">— Use OA default —</option>
          <For each={REWIND_BUFFER_OPTIONS}>
            {(mb) => <option value={String(mb)}>{mb} MB</option>}
          </For>
        </select>
      </SettingRow>
    </div>
  );
};

type ShadersProps = {
  api: PerSystemOverridesApi;
  settings: SettingsStore;
};

export const PerSystemShadersSection: Component<ShadersProps> = (props) => {
  const o = () => props.api.overrides();
  return (
    <div class="flex flex-col gap-3">
      <SettingRow
        label="Shader preset"
        hint="Applies during the final blit. Takes effect on next launch."
        inheritedValue={shaderPresetLabel(props.settings.shaderPreset())}
        overridden={o().shaderPreset != null}
      >
        <select
          class={selectClass("system")}
          value={o().shaderPreset ?? ""}
          onChange={(e) => {
            const v = e.currentTarget.value;
            void props.api.patch({ shaderPreset: v === "" ? null : v });
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
        overridden={o().bloomAmount != null}
        slider={{
          min: 0,
          max: 1,
          step: 0.05,
          value: o().bloomAmount ?? 0.6,
          onInput: (v) => {
            void props.api.patch({ bloomAmount: v });
            void setBloomAmount(v).catch(() => {});
          },
        }}
        onReset={() => void props.api.patch({ bloomAmount: null })}
      />
    </div>
  );
};

type DefaultCoreProps = {
  systemId: Accessor<SystemId | null>;
  /// Active gate so callers can defer the get_core_pref fetch when
  /// the surface isn't shown.
  active: Accessor<boolean>;
  cores: Accessor<CoreEntry[]>;
};

export const PerSystemDefaultCoreSection: Component<DefaultCoreProps> = (props) => {
  const [corePref, setCorePref] = createSignal<string | null>(null);
  createEffect(() => {
    const sysId = props.systemId();
    if (!props.active() || !sysId) return;
    void coresApi.getCorePref(sysId)
      .then((v) => setCorePref(v ?? null))
      .catch((e) => console.warn("get_core_pref failed:", e));
  });
  async function patchCorePref(fileName: string | null) {
    const sysId = props.systemId();
    if (!sysId) return;
    setCorePref(fileName);
    try {
      await coresApi.setCorePref(sysId, fileName);
    } catch (e) {
      console.warn("set_core_pref failed:", e);
    }
  }
  const inheritedCoreLabel = (): string => {
    const list = props.cores() ?? [];
    if (list.length === 0) return "Auto-detect";
    return list[0].libraryName || list[0].fileName;
  };
  return (
    <div class="flex flex-col gap-3">
      <SettingRow
        label="Default core"
        hint="Used when a game has no per-game core override"
        inheritedValue={inheritedCoreLabel()}
        inheritedFrom="Auto-detect"
        overridden={corePref() !== null}
      >
        <select
          class={selectClass("system")}
          value={corePref() ?? ""}
          onChange={(e) => {
            const v = e.currentTarget.value;
            void patchCorePref(v === "" ? null : v);
          }}
        >
          <option value="">— Use auto-detect —</option>
          <For each={props.cores() ?? []}>
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
  );
};

// --- Editors -----------------------------------------------------------

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
      const s = await getRewindState<RewindState>();
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
