import {
  createEffect,
  createResource,
  createSignal,
  For,
  onCleanup,
  onMount,
  Show,
  type Component,
} from "solid-js";
import { invoke } from "@tauri-apps/api/core";
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
import SettingRow from "./SettingRow";
import SystemBindingsEditor from "./SystemBindingsEditor";

// Phase 2.8 slice C — Per-system settings page.
//
// Reached via the system page header `⚙` button. Same left-rail tab layout
// as the OA-wide Settings page (slice A) but scoped to one system. Built on
// top of two new commands:
//
//   get_system_settings(systemId) → SystemSettings
//   set_system_settings(systemId, prefs) → ()
//
// + the existing get_core_pref / set_core_pref pair for the Cores tab. The
// per-system core override stays in `cores.json` (its existing store) so we
// don't have to migrate it; the UI surfaces it via SettingRow alongside the
// new overrides without anyone noticing the split.
//
// Tabs that are SCAFFOLD-only (the value persists but doesn't yet take
// effect at runtime — Phase 3+ work) are flagged with a "Coming soon" hint.
// Display + Cores tabs ARE live in this slice.

type SystemSettings = {
  scalingOverride?: string | null;
  windowModeOverride?: string | null;
  monitorIndexOverride?: number | null;
  shaderPreset?: string | null;
  /// Phase 3 slice C polish — overrides the Phosphor composite weight at
  /// launch time, layered on top of the active preset's TOML default.
  bloomAmount?: number | null;
  rewindEnabled?: boolean | null;
  rewindCaptureIntervalFrames?: number | null;
  rewindBufferMegabytes?: number | null;
};

type Props = {
  systemId: SystemId;
  onBack: () => void;
  /// Source for the inherited (OA-wide) defaults shown alongside each
  /// per-system override input.
  settings: SettingsStore;
  /// Optional deep-link target — when set, seeds the active tab to this
  /// value instead of the localStorage-persisted last choice. Used by the
  /// SystemHeader's quick-action buttons + SystemContextMenu's "Edit
  /// bindings…" item to land directly on the user-requested tab.
  initialTab?: "display" | "audio" | "input" | "rewind" | "cores" | "shaders" | "theme";
};

const TABS = ["display", "audio", "input", "rewind", "cores", "shaders", "theme"] as const;
type TabId = typeof TABS[number];
const TAB_LABELS: Record<TabId, string> = {
  display: "Display",
  audio:   "Audio",
  input:   "Input",
  rewind:  "Rewind",
  cores:   "Cores",
  shaders: "Shaders",
  theme:   "Theme",
};
const TAB_HINTS: Record<TabId, string> = {
  display: "Scaling + window-mode + monitor override",
  audio:   "Per-system audio output (Phase 3+)",
  input:   "Button bindings (managed from the system page)",
  rewind:  "Hold-Backspace rewind enable / buffer size",
  cores:   "Default libretro core for this system",
  shaders: "Default visual preset (Phase 3+)",
  theme:   "Per-system accent + marquee override (Phase 4+)",
};

// Mirror the same picker sets as the OA-wide Gameplay tab so the inheritance
// chain has consistent options at every level.
const REWIND_INTERVAL_OPTIONS: readonly number[] = [1, 2, 3, 6, 10, 15, 30];
const REWIND_BUFFER_OPTIONS: readonly number[] = [8, 16, 32, 64, 128, 256, 512];

const SELECT_CLASS =
  "w-full rounded-md border border-white/10 bg-white/[0.04] px-3 py-2 text-sm text-(--color-oa-ink) transition hover:bg-white/[0.08] focus-visible:outline focus-visible:outline-2 focus-visible:outline-(--color-system-accent) disabled:opacity-50";

const PerSystemSettingsPage: Component<Props> = (props) => {
  const TAB_STORAGE = "oa.per-system-settings.activeTab";
  // Deep-linked initialTab from props wins over the persisted choice — the
  // navigation site (system header button, context-menu item) is more
  // informative about user intent than the last-visited tab from a prior
  // session.
  const initialTab: TabId = (() => {
    if (props.initialTab && TABS.includes(props.initialTab as TabId)) {
      return props.initialTab as TabId;
    }
    const saved = localStorage.getItem(TAB_STORAGE) as TabId | null;
    return saved && TABS.includes(saved) ? saved : "display";
  })();
  const [activeTab, setActiveTab] = createSignal<TabId>(initialTab);
  createEffect(() => {
    localStorage.setItem(TAB_STORAGE, activeTab());
  });

  // Apply the system's data-attribute on the page wrapper for theme cascade.
  // Mirrors what SystemPage does so the per-system accent paints correctly.
  onMount(() => {
    document.documentElement.dataset.system = props.systemId;
  });
  onCleanup(() => {
    delete document.documentElement.dataset.system;
  });

  // Esc returns to the previous view. Page-mode equivalent of the OA-wide
  // SettingsPage's Esc handler.
  onMount(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      const tag = (document.activeElement as HTMLElement | null)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;
      e.stopPropagation();
      props.onBack();
    };
    window.addEventListener("keydown", onKey, { capture: true });
    onCleanup(() => window.removeEventListener("keydown", onKey, { capture: true }));
  });

  // --- Per-system overrides (the new commands) ----------------------------

  const [overrides, setOverrides] = createSignal<SystemSettings>({});
  const [hydrated, setHydrated] = createSignal(false);
  createResource(async () => {
    try {
      const got = await invoke<SystemSettings>("get_system_settings", { systemId: props.systemId });
      setOverrides(got ?? {});
    } catch (e) {
      console.warn("[oa-per-system] get_system_settings failed:", e);
    } finally {
      setHydrated(true);
    }
    return null;
  });

  // Write-through helper. Each setting input calls patch({ field: value }).
  // Null clears the override (back to "inherit OA-wide default").
  async function patch(p: Partial<SystemSettings>) {
    const next: SystemSettings = { ...overrides(), ...p };
    // Drop null/undefined entries so the on-disk file stays minimal.
    const cleaned: SystemSettings = {};
    if (next.scalingOverride != null) cleaned.scalingOverride = next.scalingOverride;
    if (next.windowModeOverride != null) cleaned.windowModeOverride = next.windowModeOverride;
    if (next.monitorIndexOverride != null) cleaned.monitorIndexOverride = next.monitorIndexOverride;
    if (next.shaderPreset != null) cleaned.shaderPreset = next.shaderPreset;
    if (next.rewindEnabled != null) cleaned.rewindEnabled = next.rewindEnabled;
    if (next.rewindCaptureIntervalFrames != null) cleaned.rewindCaptureIntervalFrames = next.rewindCaptureIntervalFrames;
    if (next.rewindBufferMegabytes != null) cleaned.rewindBufferMegabytes = next.rewindBufferMegabytes;
    setOverrides(cleaned);
    try {
      await invoke("set_system_settings", { systemId: props.systemId, settings: cleaned });
    } catch (e) {
      console.warn("[oa-per-system] set_system_settings failed:", e);
    }
  }

  // --- Cores tab — bridges existing per-system core pref ------------------

  const [cores] = createResource(async (): Promise<CoreEntry[]> => {
    try {
      return await invoke<CoreEntry[]>("list_cores");
    } catch (e) {
      console.warn("list_cores failed:", e);
      return [];
    }
  });
  const [corePref, setCorePref] = createSignal<string | null>(null);
  createResource(async () => {
    try {
      const v = await invoke<string | null>("get_core_pref", { systemId: props.systemId });
      setCorePref(v ?? null);
    } catch (e) {
      console.warn(`get_core_pref(${props.systemId}) failed:`, e);
    }
    return null;
  });
  async function patchCorePref(fileName: string | null) {
    setCorePref(fileName);
    try {
      await invoke("set_core_pref", { systemId: props.systemId, fileName });
    } catch (e) {
      console.warn("set_core_pref failed:", e);
    }
  }

  // --- Monitor list for the Display tab inherited chip --------------------

  const [monitors] = createResource(async (): Promise<MonitorInfo[]> => {
    try {
      return await invoke<MonitorInfo[]>("list_monitors");
    } catch (e) {
      console.warn("list_monitors failed:", e);
      return [];
    }
  });

  // --- Helpers for inherited-value display --------------------------------

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
    // First detected core wins as the shell's fallback (see resolve_cores
    // in main.rs). Same logic the launch path uses when no per-system pref
    // is set, so this is faithful to runtime behavior.
    return list[0].libraryName || list[0].fileName;
  };

  const theme = () => systemThemes[props.systemId];

  return (
    <div
      class="flex h-full w-full flex-col bg-(--color-oa-bg)"
      data-system={props.systemId}
      role="region"
      aria-labelledby="per-system-title"
    >
      <header class="flex items-center justify-between border-b border-white/5 bg-(--color-oa-bg-deep)/60 px-6 py-4">
        <div class="flex items-center gap-3">
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              props.onBack();
            }}
            class="rounded-md border border-white/10 bg-white/[0.04] px-2.5 py-1 text-xs uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
            title="Back (Esc)"
            aria-label="Back"
          >
            ‹ Back
          </button>
          <div>
            <p class="text-[0.6rem] uppercase tracking-[0.3em] text-(--color-system-accent)">
              {theme().shortName} · per-system settings
            </p>
            <h2
              id="per-system-title"
              class="mt-0.5 text-base font-semibold text-(--color-oa-ink)"
            >
              {theme().displayName}
            </h2>
          </div>
        </div>
        <p class="hidden text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim) lg:block">
          {TAB_LABELS[activeTab()]} · {TAB_HINTS[activeTab()]}
        </p>
      </header>

      <div class="flex min-h-0 flex-1">
        <nav class="flex w-44 shrink-0 flex-col gap-1 border-r border-white/5 bg-black/20 px-2 py-4">
          <For each={TABS}>
            {(tab) => (
              <button
                type="button"
                onClick={(e) => {
                  e.currentTarget.blur();
                  setActiveTab(tab);
                }}
                aria-pressed={activeTab() === tab}
                class="relative rounded-md px-3 py-2 text-left text-xs font-medium uppercase tracking-wider transition"
                classList={{
                  "bg-white/[0.06] text-(--color-oa-ink)": activeTab() === tab,
                  "text-(--color-oa-ink-dim) hover:bg-white/[0.04] hover:text-(--color-oa-ink)": activeTab() !== tab,
                }}
              >
                <Show when={activeTab() === tab}>
                  <span
                    class="pointer-events-none absolute left-0 top-1/2 h-5 w-0.5 -translate-y-1/2 rounded-r bg-(--color-system-accent)"
                    aria-hidden="true"
                  />
                </Show>
                {TAB_LABELS[tab]}
              </button>
            )}
          </For>
        </nav>

        <section class="min-w-0 flex-1 overflow-y-auto px-6 py-5">
          <Show when={!hydrated()}>
            <p class="text-xs uppercase tracking-widest text-(--color-oa-ink-dim)">
              Loading per-system overrides…
            </p>
          </Show>

          {/* --- Display ---------------------------------------------------- */}
          <Show when={activeTab() === "display"}>
            <div class="flex flex-col gap-3">
              <PendingRuntimeBanner>
                Per-system display overrides persist now but don't yet take effect at
                launch — the renderer still reads the OA-wide value. Wiring lands
                alongside per-game shader work in Phase 3.
              </PendingRuntimeBanner>

              <SettingRow
                label="Scaling mode"
                hint="How the framebuffer fills the window"
                inheritedValue={inheritedScalingLabel()}
                overridden={overrides().scalingOverride != null}
              >
                <select
                  class={SELECT_CLASS}
                  value={overrides().scalingOverride ?? ""}
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
                overridden={overrides().windowModeOverride != null}
              >
                <select
                  class={SELECT_CLASS}
                  value={overrides().windowModeOverride ?? ""}
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
                overridden={overrides().monitorIndexOverride != null}
              >
                <select
                  class={SELECT_CLASS}
                  value={overrides().monitorIndexOverride?.toString() ?? ""}
                  onChange={(e) => {
                    const v = e.currentTarget.value;
                    void patch({ monitorIndexOverride: v === "" ? null : Number(v) });
                  }}
                >
                  <option value="">— Use OA default —</option>
                  <For each={monitors() ?? []}>
                    {(m) => (
                      <option value={m.index.toString()}>
                        {(m.name?.trim() || `Monitor ${m.index + 1}`) +
                          ` (${m.width}×${m.height})`}
                      </option>
                    )}
                  </For>
                </select>
              </SettingRow>
            </div>
          </Show>

          {/* --- Audio ------------------------------------------------------ */}
          <Show when={activeTab() === "audio"}>
            <div class="flex flex-col gap-3">
              <PendingRuntimeBanner>
                Per-system audio overrides are deferred. Most systems use the OA-wide
                output device + sample rate; system-specific mono / stereo / latency
                shapes land alongside the Phase 3 audio work.
              </PendingRuntimeBanner>
              <p class="text-xs text-(--color-oa-ink-dim)">
                No per-system audio settings yet. The OA-wide audio device + latency
                profile applies to every system.
              </p>
            </div>
          </Show>

          {/* --- Input ------------------------------------------------------ */}
          <Show when={activeTab() === "input"}>
            <SystemBindingsEditor systemId={props.systemId} />
          </Show>

          {/* --- Rewind ----------------------------------------------------- */}
          <Show when={activeTab() === "rewind"}>
            <div class="flex flex-col gap-3">
              <SettingRow
                label="Enable rewind"
                hint="Hold Backspace during gameplay to step backwards"
                inheritedValue={props.settings.rewindEnabled() ? "On" : "Off"}
                overridden={overrides().rewindEnabled != null}
              >
                <select
                  class={SELECT_CLASS}
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
                  <option value="">— Use OA default —</option>
                  <option value="on">On</option>
                  <option value="off">Off</option>
                </select>
              </SettingRow>

              <SettingRow
                label="Capture interval"
                hint="Frames between snapshots"
                inheritedValue={`Every ${props.settings.rewindCaptureIntervalFrames()} frames`}
                overridden={overrides().rewindCaptureIntervalFrames != null}
              >
                <select
                  class={SELECT_CLASS}
                  value={overrides().rewindCaptureIntervalFrames?.toString() ?? ""}
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
                overridden={overrides().rewindBufferMegabytes != null}
              >
                <select
                  class={SELECT_CLASS}
                  value={overrides().rewindBufferMegabytes?.toString() ?? ""}
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
              <p class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                Takes effect on next launch of a {theme().displayName} game.
              </p>
            </div>
          </Show>

          {/* --- Cores ------------------------------------------------------ */}
          <Show when={activeTab() === "cores"}>
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
                Change takes effect on next ROM launch. Per-game overrides (right-click
                a tile → Run with core…) take precedence over this.
              </p>
            </div>
          </Show>

          {/* --- Shaders ---------------------------------------------------- */}
          <Show when={activeTab() === "shaders"}>
            <div class="flex flex-col gap-3">
              <SettingRow
                label="Shader preset"
                hint="Applies during the final blit. Takes effect on next launch."
                inheritedValue={shaderPresetLabel(props.settings.shaderPreset())}
                overridden={overrides().shaderPreset != null}
              >
                <select
                  class={SELECT_CLASS}
                  value={overrides().shaderPreset ?? ""}
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
                hint="Overrides the active Phosphor preset's bloom weight. 0 = pure source, 1 = pure blur. Ignored when the active preset isn't Phosphor-based."
                inheritedValue="Preset default"
                overridden={overrides().bloomAmount != null}
              >
                <div class="flex items-center gap-3">
                  <input
                    type="range"
                    min="0"
                    max="1"
                    step="0.05"
                    value={overrides().bloomAmount ?? 0.6}
                    onInput={(e) => {
                      const v = Number(e.currentTarget.value);
                      if (!Number.isFinite(v)) return;
                      void patch({ bloomAmount: v });
                      // Live preview: push the value straight to the renderer
                      // so the user sees the change while dragging, not just
                      // on next launch. The launch path also resolves the
                      // chain — this is purely an interactive hot-path.
                      void invoke("set_bloom_amount", { amount: v }).catch(() => {});
                    }}
                    class="flex-1"
                  />
                  <span class="font-mono text-sm w-12 text-right tabular-nums">
                    {(overrides().bloomAmount ?? 0.6).toFixed(2)}
                  </span>
                  <Show when={overrides().bloomAmount != null}>
                    <button
                      type="button"
                      onClick={() => void patch({ bloomAmount: null })}
                      class="text-xs px-2 py-1 rounded bg-(--color-oa-surface-2) hover:bg-(--color-oa-surface-3)"
                    >
                      Reset
                    </button>
                  </Show>
                </div>
              </SettingRow>
              <p class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                Presets ship as TOML files in <code>shaders/presets/</code> (slice C).
                Drop your own <code>&lt;name&gt;.preset.toml</code> in
                <code> &lt;exe_dir&gt;/shaders/presets/</code> to override a built-in or
                add a new entry; edits hot-reload while the game is running (slice D).
              </p>
            </div>
          </Show>

          {/* --- Theme ------------------------------------------------------ */}
          <Show when={activeTab() === "theme"}>
            <div class="flex flex-col gap-3">
              <PendingRuntimeBanner>
                Per-system theme override (accent / marquee / font) is Phase 4+ alongside
                the cabinet mode work. The current theme cascade pulls from
                `themes/registry.ts` + `themes/systems.css` — adding an override surface
                means writing through to a per-system theme JSON the registry reads.
              </PendingRuntimeBanner>
              <p class="text-xs text-(--color-oa-ink-dim)">
                Today {theme().displayName} uses the registry's accent and shortName.
                Override UI lands when at least two systems are shipped (to give the
                "compare vs. default" picker a real fallback).
              </p>
            </div>
          </Show>
        </section>
      </div>
    </div>
  );
};

const PendingRuntimeBanner: Component<{ children: any }> = (props) => (
  <div class="rounded-md border border-amber-500/30 bg-amber-500/5 px-3 py-2 text-[0.7rem] text-(--color-oa-ink)">
    <span class="font-semibold uppercase tracking-widest text-amber-300">Scaffold</span>{" "}
    — {props.children}
  </div>
);

export default PerSystemSettingsPage;
