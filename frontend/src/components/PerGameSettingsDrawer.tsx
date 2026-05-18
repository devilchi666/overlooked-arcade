import {
  createEffect,
  createSignal,
  For,
  onCleanup,
  onMount,
  Show,
  type Component,
} from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
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
import { systemThemes } from "../themes/registry";
import type { LibraryStore } from "../library/store";
import type { RomEntry } from "../library/types";
import SettingRow from "./SettingRow";

// Phase 2.8 slice D — per-game settings drawer.
//
// Slides in from the right when triggered (typically from the tile context
// menu's "Properties…" item). Tabs along the top — different from the
// per-system page's left-rail because the drawer is narrower (480px) and
// horizontal tabs scale better at that width.
//
// Three-tier inheritance: per-game (this drawer) → per-system (slice C
// PerSystemSettingsPage) → OA-wide (slice A SettingsPage). Each SettingRow
// computes its inherited chip from `perSystem.field ?? oaWide.field` and
// passes `inheritedFrom` ("Per-system" or "OA default") to label the chip
// correctly.
//
// Persistence:
//   - Display + Region fields → `games.overrides_json` via the new
//     get_game_overrides / set_game_overrides Tauri commands.
//   - Core override → existing `games.core_override` column via
//     update_game_core_override. Kept as its own column because the launch
//     path already reads it; bridging here is just a thin wrapper.

type GameOverrides = {
  scalingOverride?: string | null;
  windowModeOverride?: string | null;
  monitorIndexOverride?: number | null;
  regionOverride?: string | null;
  shaderPreset?: string | null;
  /// Phase 3 slice C polish — per-game Phosphor composite weight override.
  /// Wins over per-system value at launch.
  bloomAmount?: number | null;
  rewindEnabled?: boolean | null;
  rewindCaptureIntervalFrames?: number | null;
  rewindBufferMegabytes?: number | null;
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
};

const TABS = ["overview", "core", "display", "audio", "input", "rewind", "shaders", "region", "milestones"] as const;
type TabId = typeof TABS[number];
const TAB_LABELS: Record<TabId, string> = {
  overview:   "Overview",
  core:       "Core",
  display:    "Display",
  audio:      "Audio",
  input:      "Input",
  rewind:     "Rewind",
  shaders:    "Shaders",
  region:     "Region",
  milestones: "Milestones",
};

// Phase 4 slice F — milestone shape mirrors Rust `library_db::Milestone`.
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

const MILESTONE_REGION_OPTIONS: readonly Milestone["region"][] = ["system_ram", "save_ram", "rtc", "video_ram"];
const MILESTONE_OP_OPTIONS: readonly Milestone["op"][] = ["eq", "neq", "gt", "lt", "geq", "leq"];
const MILESTONE_WIDTH_OPTIONS: readonly Milestone["width"][] = [1, 2, 4];

const REWIND_INTERVAL_OPTIONS: readonly number[] = [1, 2, 3, 6, 10, 15, 30];
const REWIND_BUFFER_OPTIONS: readonly number[] = [8, 16, 32, 64, 128, 256, 512];

type Props = {
  open: boolean;
  entry: RomEntry | null;
  onClose: () => void;
  /// Source for the OA-wide inherited defaults.
  settings: SettingsStore;
  library: LibraryStore;
};

const SELECT_CLASS =
  "w-full rounded-md border border-white/10 bg-white/[0.04] px-3 py-2 text-sm text-(--color-oa-ink) transition hover:bg-white/[0.08] focus-visible:outline focus-visible:outline-2 focus-visible:outline-(--color-system-accent) disabled:opacity-50";

const PerGameSettingsDrawer: Component<Props> = (props) => {
  const [activeTab, setActiveTab] = createSignal<TabId>("overview");
  const [overrides, setOverrides] = createSignal<GameOverrides>({});
  const [systemSettings, setSystemSettings] = createSignal<SystemSettings>({});
  const [cores, setCores] = createSignal<CoreEntry[]>([]);
  const [monitors, setMonitors] = createSignal<MonitorInfo[]>([]);
  const [systemCorePref, setSystemCorePref] = createSignal<string | null>(null);
  const [milestones, setMilestones] = createSignal<Milestone[]>([]);
  const [draftMilestone, setDraftMilestone] = createSignal<Milestone | null>(null);

  // Hydrate on each open. Per-game overrides + per-system settings + cores +
  // monitors all refresh together so the inheritance display is consistent.
  createEffect(() => {
    const e = props.entry;
    if (!props.open || !e) return;
    setActiveTab("overview");
    void (async () => {
      try {
        const got = await invoke<GameOverrides>("get_game_overrides", { id: e.id });
        setOverrides(got ?? {});
      } catch (err) {
        console.warn("[oa-drawer] get_game_overrides failed:", err);
        setOverrides({});
      }
      try {
        const sys = await invoke<SystemSettings>("get_system_settings", {
          systemId: e.systemId,
        });
        setSystemSettings(sys ?? {});
      } catch (err) {
        console.warn("[oa-drawer] get_system_settings failed:", err);
        setSystemSettings({});
      }
      try {
        const list = await invoke<CoreEntry[]>("list_cores");
        setCores(list ?? []);
      } catch {
        setCores([]);
      }
      try {
        const mons = await invoke<MonitorInfo[]>("list_monitors");
        setMonitors(mons ?? []);
      } catch {
        setMonitors([]);
      }
      try {
        const v = await invoke<string | null>("get_core_pref", { systemId: e.systemId });
        setSystemCorePref(v ?? null);
      } catch {
        setSystemCorePref(null);
      }
      try {
        const ms = await invoke<Milestone[]>("list_milestones", { gameId: e.id });
        setMilestones(ms ?? []);
      } catch {
        setMilestones([]);
      }
      setDraftMilestone(null);
    })();
  });

  // Milestone-trigger event refreshes the list so the "triggered at"
  // chip updates live during gameplay if the drawer is open.
  onMount(() => {
    let unlisten: (() => void) | undefined;
    void listen("oa://milestone-triggered", () => {
      const e = props.entry;
      if (!props.open || !e) return;
      void invoke<Milestone[]>("list_milestones", { gameId: e.id })
        .then((ms) => setMilestones(ms ?? []))
        .catch(() => {});
    }).then((un) => (unlisten = un));
    onCleanup(() => unlisten?.());
  });

  // --- Milestone helpers ------------------------------------------------

  function blankMilestone(): Milestone {
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

  async function refreshMilestones() {
    const e = props.entry;
    if (!e) return;
    try {
      const ms = await invoke<Milestone[]>("list_milestones", { gameId: e.id });
      setMilestones(ms ?? []);
    } catch (err) {
      console.warn("[oa-drawer] list_milestones failed:", err);
    }
  }

  async function saveMilestone(m: Milestone) {
    const e = props.entry;
    if (!e) return;
    try {
      if (m.id == null) {
        const id = await invoke<number>("add_milestone", { milestone: { ...m, gameId: e.id } });
        console.log("[oa-drawer] add_milestone -> id", id);
      } else {
        await invoke("update_milestone", { milestone: m });
      }
      setDraftMilestone(null);
      await refreshMilestones();
    } catch (err) {
      console.warn("[oa-drawer] save_milestone failed:", err);
    }
  }

  async function deleteMilestone(id: number) {
    try {
      await invoke("delete_milestone", { id });
      await refreshMilestones();
    } catch (err) {
      console.warn("[oa-drawer] delete_milestone failed:", err);
    }
  }

  async function resetMilestone(id: number) {
    try {
      await invoke("reset_milestone_progress", { id });
      await refreshMilestones();
    } catch (err) {
      console.warn("[oa-drawer] reset_milestone_progress failed:", err);
    }
  }

  // Esc closes. Capture-phase so we win against any underlying handlers.
  onMount(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!props.open) return;
      if (e.key === "Escape") {
        const tag = (document.activeElement as HTMLElement | null)?.tagName;
        if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;
        e.stopPropagation();
        props.onClose();
      }
    };
    window.addEventListener("keydown", onKey, { capture: true });
    onCleanup(() => window.removeEventListener("keydown", onKey, { capture: true }));
  });

  async function patch(p: Partial<GameOverrides>) {
    const e = props.entry;
    if (!e) return;
    const next: GameOverrides = { ...overrides(), ...p };
    // Drop null/undefined entries so the on-disk JSON stays minimal.
    const cleaned: GameOverrides = {};
    if (next.scalingOverride != null) cleaned.scalingOverride = next.scalingOverride;
    if (next.windowModeOverride != null) cleaned.windowModeOverride = next.windowModeOverride;
    if (next.monitorIndexOverride != null) cleaned.monitorIndexOverride = next.monitorIndexOverride;
    if (next.regionOverride != null) cleaned.regionOverride = next.regionOverride;
    if (next.shaderPreset != null) cleaned.shaderPreset = next.shaderPreset;
    if (next.rewindEnabled != null) cleaned.rewindEnabled = next.rewindEnabled;
    if (next.rewindCaptureIntervalFrames != null) cleaned.rewindCaptureIntervalFrames = next.rewindCaptureIntervalFrames;
    if (next.rewindBufferMegabytes != null) cleaned.rewindBufferMegabytes = next.rewindBufferMegabytes;
    setOverrides(cleaned);
    try {
      await invoke("set_game_overrides", { id: e.id, overrides: cleaned });
    } catch (err) {
      console.warn("[oa-drawer] set_game_overrides failed:", err);
    }
  }

  async function patchCoreOverride(fileName: string | null) {
    const e = props.entry;
    if (!e) return;
    try {
      await props.library.setCoreOverride(e.id, fileName);
    } catch (err) {
      console.warn("[oa-drawer] setCoreOverride failed:", err);
    }
  }

  // --- Inherited value resolvers (per-system → OA-wide chain) -------------

  function inheritedScaling(): { label: string; from: string } {
    const sys = systemSettings().scalingOverride;
    if (sys) return { label: SCALING_MODE_LABELS[sys as ScalingMode] ?? sys, from: "Per-system" };
    return {
      label: SCALING_MODE_LABELS[props.settings.scalingMode()] ?? props.settings.scalingMode(),
      from: "OA default",
    };
  }
  function inheritedWindow(): { label: string; from: string } {
    const sys = systemSettings().windowModeOverride;
    if (sys) return { label: WINDOW_MODE_LABELS[sys as WindowMode] ?? sys, from: "Per-system" };
    return {
      label: WINDOW_MODE_LABELS[props.settings.windowMode()] ?? props.settings.windowMode(),
      from: "OA default",
    };
  }
  function inheritedMonitor(): { label: string; from: string } {
    const fromIdx = (idx: number | null | undefined): string => {
      if (idx === null || idx === undefined) return "Current monitor";
      const m = monitors().find((mm) => mm.index === idx);
      if (!m) return `Monitor ${idx + 1}`;
      return m.name?.trim() || `Monitor ${idx + 1}`;
    };
    const sys = systemSettings().monitorIndexOverride;
    if (sys !== null && sys !== undefined) {
      return { label: fromIdx(sys), from: "Per-system" };
    }
    return { label: fromIdx(props.settings.monitorIndex()), from: "OA default" };
  }
  function inheritedShader(): { label: string; from: string } {
    const sys = systemSettings().shaderPreset;
    if (sys) {
      return {
        label: shaderPresetLabel(sys),
        from: "Per-system",
      };
    }
    return {
      label: shaderPresetLabel(props.settings.shaderPreset()),
      from: "OA default",
    };
  }

  function inheritedRewindEnabled(): { label: string; from: string } {
    const sys = systemSettings().rewindEnabled;
    if (sys != null) return { label: sys ? "On" : "Off", from: "Per-system" };
    return { label: props.settings.rewindEnabled() ? "On" : "Off", from: "OA default" };
  }
  function inheritedRewindInterval(): { label: string; from: string } {
    const sys = systemSettings().rewindCaptureIntervalFrames;
    if (sys != null) return { label: `Every ${sys} frames`, from: "Per-system" };
    return {
      label: `Every ${props.settings.rewindCaptureIntervalFrames()} frames`,
      from: "OA default",
    };
  }
  function inheritedRewindBuffer(): { label: string; from: string } {
    const sys = systemSettings().rewindBufferMegabytes;
    if (sys != null) return { label: `${sys} MB`, from: "Per-system" };
    return { label: `${props.settings.rewindBufferMegabytes()} MB`, from: "OA default" };
  }

  function inheritedCore(): { label: string; from: string } {
    const sysCore = systemCorePref();
    if (sysCore) {
      const found = cores().find((c) => c.fileName === sysCore);
      return {
        label: found ? `${found.libraryName} (${found.libraryVersion})` : sysCore,
        from: "Per-system",
      };
    }
    // OA fallback = first detected core (matches main.rs launch path)
    const first = cores()[0];
    return {
      label: first ? first.libraryName || first.fileName : "Auto-detect",
      from: "Auto-detect",
    };
  }

  const theme = () => {
    const e = props.entry;
    if (!e) return null;
    return systemThemes[e.systemId];
  };

  return (
    <Show when={props.open && props.entry !== null}>
      <div
        class="fixed inset-0 z-50 flex justify-end"
        onClick={(e) => {
          if (e.currentTarget === e.target) props.onClose();
        }}
        role="dialog"
        aria-modal="true"
        aria-labelledby="per-game-drawer-title"
      >
        {/* Backdrop. Doesn't cover the whole window since the drawer hosts
            its own click-outside-to-close — the wrapper above catches
            backdrop clicks. Render a translucent layer for visual focus. */}
        <div
          class="absolute inset-0 bg-black/45 backdrop-blur-sm"
          onClick={() => props.onClose()}
          aria-hidden="true"
        />
        <aside
          class="relative flex h-full w-full max-w-[30rem] flex-col overflow-hidden border-l border-white/10 bg-(--color-oa-bg-deep) shadow-2xl shadow-black/60"
          data-system={props.entry!.systemId}
          onClick={(e) => e.stopPropagation()}
        >
          <header class="flex items-start justify-between gap-3 border-b border-white/5 bg-(--color-system-accent)/8 px-5 py-4">
            <div class="min-w-0 flex-1">
              <p class="text-[0.6rem] uppercase tracking-[0.3em] text-(--color-system-accent)">
                {theme()?.shortName ?? props.entry!.systemId} · game settings
              </p>
              <h2
                id="per-game-drawer-title"
                class="mt-0.5 truncate text-base font-semibold text-(--color-oa-ink)"
                title={props.entry!.title}
              >
                {props.entry!.title}
              </h2>
            </div>
            <button
              type="button"
              onClick={(e) => {
                e.currentTarget.blur();
                props.onClose();
              }}
              class="rounded-md border border-white/10 bg-white/[0.04] px-2.5 py-1 text-xs uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
              title="Close (Esc)"
            >
              ✕
            </button>
          </header>

          <nav class="flex shrink-0 gap-0.5 overflow-x-auto border-b border-white/5 bg-black/20 px-3 py-2">
            <For each={TABS}>
              {(tab) => (
                <button
                  type="button"
                  onClick={(e) => {
                    e.currentTarget.blur();
                    setActiveTab(tab);
                  }}
                  aria-pressed={activeTab() === tab}
                  class="shrink-0 rounded-md px-2.5 py-1 text-[0.65rem] font-medium uppercase tracking-wider transition"
                  classList={{
                    "bg-(--color-system-accent)/15 text-(--color-system-accent)": activeTab() === tab,
                    "text-(--color-oa-ink-dim) hover:bg-white/[0.04] hover:text-(--color-oa-ink)": activeTab() !== tab,
                  }}
                >
                  {TAB_LABELS[tab]}
                </button>
              )}
            </For>
          </nav>

          <section class="min-h-0 flex-1 overflow-y-auto px-5 py-4">
            {/* --- Overview ------------------------------------------------ */}
            <Show when={activeTab() === "overview"}>
              <div class="flex flex-col gap-3">
                <dl class="grid grid-cols-[7rem_1fr] gap-x-3 gap-y-1.5 text-xs">
                  <dt class="text-(--color-oa-ink-dim)">Title</dt>
                  <dd class="text-(--color-oa-ink)">{props.entry!.title}</dd>
                  <dt class="text-(--color-oa-ink-dim)">System</dt>
                  <dd class="text-(--color-oa-ink)">{theme()?.displayName ?? props.entry!.systemId}</dd>
                  <dt class="text-(--color-oa-ink-dim)">ROM path</dt>
                  <dd class="truncate font-mono text-[0.65rem] text-(--color-oa-ink)" title={props.entry!.filePath}>
                    {props.entry!.filePath}
                  </dd>
                  <Show when={props.entry!.archiveInnerPath}>
                    <dt class="text-(--color-oa-ink-dim)">In archive</dt>
                    <dd class="truncate font-mono text-[0.65rem] text-(--color-oa-ink)">
                      {props.entry!.archiveInnerPath}
                    </dd>
                  </Show>
                  <dt class="text-(--color-oa-ink-dim)">Added</dt>
                  <dd class="text-(--color-oa-ink)">{new Date(props.entry!.addedAt).toLocaleDateString()}</dd>
                </dl>
                <p class="rounded-md border border-white/5 bg-black/20 px-3 py-2 text-[0.7rem] text-(--color-oa-ink-dim)">
                  Custom user fields (tags, personal notes) and rich metadata editing land
                  alongside the per-game activity log in Phase 4.
                </p>
              </div>
            </Show>

            {/* --- Core --------------------------------------------------- */}
            <Show when={activeTab() === "core"}>
              <div class="flex flex-col gap-3">
                <SettingRow
                  label="Core override"
                  hint="Used instead of the per-system default for this game only"
                  inheritedValue={inheritedCore().label}
                  inheritedFrom={inheritedCore().from}
                  overridden={!!props.entry!.coreOverride}
                >
                  <select
                    class={SELECT_CLASS}
                    value={props.entry!.coreOverride ?? ""}
                    onChange={(e) => {
                      const v = e.currentTarget.value;
                      void patchCoreOverride(v === "" ? null : v);
                    }}
                  >
                    <option value="">— Use per-system / auto —</option>
                    <For each={cores()}>
                      {(c) => (
                        <option value={c.fileName}>
                          {c.libraryName} ({c.libraryVersion})
                        </option>
                      )}
                    </For>
                  </select>
                </SettingRow>
                <p class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                  Takes effect on next launch. Right-click the tile → "Change core…" reaches the same setting.
                </p>
              </div>
            </Show>

            {/* --- Display ------------------------------------------------ */}
            <Show when={activeTab() === "display"}>
              <div class="flex flex-col gap-3">
                <ScaffoldBanner>
                  Per-game display overrides persist now but don't yet take effect at launch —
                  the renderer still reads the OA-wide value. Wiring lands alongside per-game
                  shader work in Phase 3.
                </ScaffoldBanner>
                <SettingRow
                  label="Scaling mode"
                  inheritedValue={inheritedScaling().label}
                  inheritedFrom={inheritedScaling().from}
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
                    class={SELECT_CLASS}
                    value={overrides().windowModeOverride ?? ""}
                    onChange={(e) => {
                      const v = e.currentTarget.value;
                      void patch({ windowModeOverride: v === "" ? null : v });
                    }}
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
                    class={SELECT_CLASS}
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
              </div>
            </Show>

            {/* --- Audio -------------------------------------------------- */}
            <Show when={activeTab() === "audio"}>
              <ScaffoldBanner>
                Per-game audio profile (volume / latency / mono-fold) — placeholder. OA-wide
                audio output applies to every game today.
              </ScaffoldBanner>
            </Show>

            {/* --- Input -------------------------------------------------- */}
            <Show when={activeTab() === "input"}>
              <ScaffoldBanner>
                Per-game button remap is deferred. System bindings on the system page apply
                to every game; per-game overrides need a richer Input surface that knows how
                to overlay just a few buttons (e.g. "this game uses A for jump instead of B").
              </ScaffoldBanner>
            </Show>

            {/* --- Rewind -------------------------------------------------- */}
            <Show when={activeTab() === "rewind"}>
              <div class="flex flex-col gap-3">
                <SettingRow
                  label="Enable rewind"
                  hint="Hold Backspace to step backwards"
                  inheritedValue={inheritedRewindEnabled().label}
                  inheritedFrom={inheritedRewindEnabled().from}
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
                    <option value="">— Use inherited —</option>
                    <option value="on">On</option>
                    <option value="off">Off</option>
                  </select>
                </SettingRow>

                <SettingRow
                  label="Capture interval"
                  hint="Frames between snapshots"
                  inheritedValue={inheritedRewindInterval().label}
                  inheritedFrom={inheritedRewindInterval().from}
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
                  inheritedValue={inheritedRewindBuffer().label}
                  inheritedFrom={inheritedRewindBuffer().from}
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
                    <option value="">— Use inherited —</option>
                    <For each={REWIND_BUFFER_OPTIONS}>
                      {(mb) => <option value={String(mb)}>{mb} MB</option>}
                    </For>
                  </select>
                </SettingRow>
              </div>
            </Show>

            {/* --- Shaders ------------------------------------------------ */}
            <Show when={activeTab() === "shaders"}>
              <div class="flex flex-col gap-3">
                <SettingRow
                  label="Shader preset"
                  hint="Applies at next launch"
                  inheritedValue={inheritedShader().label}
                  inheritedFrom={inheritedShader().from}
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
                >
                  <div class="flex items-center gap-3">
                    <input
                      type="range"
                      min="0"
                      max="1"
                      step="0.05"
                      value={overrides().bloomAmount ?? systemSettings().bloomAmount ?? 0.6}
                      onInput={(e) => {
                        const v = Number(e.currentTarget.value);
                        if (!Number.isFinite(v)) return;
                        void patch({ bloomAmount: v });
                        // Live preview during drag — see slice-C polish note
                        // in PerSystemSettingsPage. Launch-path resolution
                        // still runs, this is the interactive overlay.
                        void invoke("set_bloom_amount", { amount: v }).catch(() => {});
                      }}
                      class="flex-1"
                    />
                    <span class="font-mono text-sm w-12 text-right tabular-nums">
                      {(overrides().bloomAmount ?? systemSettings().bloomAmount ?? 0.6).toFixed(2)}
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
                  Presets come from <code>shaders/presets/*.preset.toml</code> (slice C —
                  built-ins shipped in-binary; user files in <code>&lt;exe_dir&gt;/shaders/presets/</code>
                  overlay by name; edits hot-reload via slice D). Per-game bloom_amount layers
                  on top of the per-system override, which layers on top of the preset's TOML default.
                </p>
              </div>
            </Show>

            {/* --- Milestones -------------------------------------------- */}
            <Show when={activeTab() === "milestones"}>
              <MilestonesTab
                milestones={milestones()}
                draft={draftMilestone()}
                onStartNew={() => setDraftMilestone(blankMilestone())}
                onChangeDraft={(m) => setDraftMilestone(m)}
                onCancelDraft={() => setDraftMilestone(null)}
                onSave={(m) => void saveMilestone(m)}
                onEdit={(m) => setDraftMilestone(m)}
                onDelete={(id) => void deleteMilestone(id)}
                onReset={(id) => void resetMilestone(id)}
              />
            </Show>

            {/* --- Region ------------------------------------------------- */}
            <Show when={activeTab() === "region"}>
              <div class="flex flex-col gap-3">
                <ScaffoldBanner>
                  Emulator region override (USA / Japan / Europe / …) — distinct from the per-game
                  cover-art region surface in the Region picker. Affects BIOS region detection +
                  some game compatibility. Persists today; runtime effect lands per-core.
                </ScaffoldBanner>
                <SettingRow
                  label="Emulator region"
                  inheritedValue="Auto-detect"
                  inheritedFrom="Per-system"
                  overridden={overrides().regionOverride != null}
                >
                  <select
                    class={SELECT_CLASS}
                    value={overrides().regionOverride ?? ""}
                    onChange={(e) => {
                      const v = e.currentTarget.value;
                      void patch({ regionOverride: v === "" ? null : v });
                    }}
                  >
                    <option value="">— Use inherited —</option>
                    <option value="usa">USA</option>
                    <option value="japan">Japan</option>
                    <option value="europe">Europe</option>
                    <option value="world">World / Auto</option>
                  </select>
                </SettingRow>
              </div>
            </Show>
          </section>
        </aside>
      </div>
    </Show>
  );
};

const ScaffoldBanner: Component<{ children: any }> = (props) => (
  <div class="rounded-md border border-amber-500/30 bg-amber-500/5 px-3 py-2 text-[0.7rem] text-(--color-oa-ink)">
    <span class="font-semibold uppercase tracking-widest text-amber-300">Scaffold</span>{" "}
    — {props.children}
  </div>
);

// --- Milestones tab body ---------------------------------------------

type MilestonesTabProps = {
  milestones: Milestone[];
  draft: Milestone | null;
  onStartNew: () => void;
  onChangeDraft: (m: Milestone) => void;
  onCancelDraft: () => void;
  onSave: (m: Milestone) => void;
  onEdit: (m: Milestone) => void;
  onDelete: (id: number) => void;
  onReset: (id: number) => void;
};

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

const MilestonesTab: Component<MilestonesTabProps> = (props) => {
  const triggeredCount = (): number => props.milestones.filter((m) => m.triggeredAtUnixMs != null).length;
  return (
    <div class="flex flex-col gap-3">
      <div class="rounded-md border border-white/10 bg-white/[0.03] px-3 py-2 text-xs text-(--color-oa-ink-dim)">
        <p>
          <span class="font-semibold text-(--color-oa-ink)">
            {props.milestones.length === 0
              ? "No milestones configured."
              : `${triggeredCount()} / ${props.milestones.length} triggered`}
          </span>
        </p>
        <p class="mt-1">
          A milestone fires once when a memory value matches the predicate (e.g. "byte at
          0x1234 == 0x80"). Use the memory inspector (Esc → Memory inspector) to find
          addresses that change with in-game progress.
        </p>
      </div>

      {/* Existing milestones list */}
      <Show when={props.milestones.length > 0}>
        <ul class="flex flex-col gap-1.5">
          <For each={props.milestones}>
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
                  {MILESTONE_REGION_LABELS[m.region]} @ 0x{m.offset.toString(16).toUpperCase()}{" "}
                  ({m.width === 1 ? "u8" : m.width === 2 ? "u16" : "u32"}) {MILESTONE_OP_LABELS[m.op]}{" "}
                  {m.target}
                  {m.edgeOnly ? " · edge" : " · level"}
                </p>
                <Show when={m.description}>
                  <p class="mt-1 text-[0.65rem] text-(--color-oa-ink-dim)">{m.description}</p>
                </Show>
                <div class="mt-2 flex gap-1.5">
                  <button
                    type="button"
                    onClick={() => props.onEdit(m)}
                    class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim) hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
                  >
                    Edit
                  </button>
                  <Show when={m.triggeredAtUnixMs != null}>
                    <button
                      type="button"
                      onClick={() => m.id != null && props.onReset(m.id)}
                      class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim) hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
                    >
                      Reset
                    </button>
                  </Show>
                  <button
                    type="button"
                    onClick={() => m.id != null && props.onDelete(m.id)}
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

      {/* Draft editor (new or edit) */}
      <Show when={props.draft !== null} fallback={
        <button
          type="button"
          onClick={props.onStartNew}
          class="w-full rounded-md border border-(--color-system-accent)/40 bg-(--color-system-accent)/10 px-3 py-2 text-sm text-(--color-oa-ink) hover:bg-(--color-system-accent)/20"
        >
          + Add milestone
        </button>
      }>
        <MilestoneEditor
          draft={props.draft!}
          onChange={props.onChangeDraft}
          onSave={() => props.onSave(props.draft!)}
          onCancel={props.onCancelDraft}
        />
      </Show>
    </div>
  );
};

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
        <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">Name</span>
        <input
          type="text"
          value={props.draft.name}
          onInput={(e) => set("name", e.currentTarget.value)}
          placeholder="Boss 1 defeated"
          class={inputClass}
        />
      </label>
      <label class="block space-y-0.5">
        <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">Description (optional)</span>
        <input
          type="text"
          value={props.draft.description}
          onInput={(e) => set("description", e.currentTarget.value)}
          class={inputClass}
        />
      </label>
      <div class="flex gap-2">
        <label class="flex-1 space-y-0.5">
          <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">Region</span>
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
        <label class="w-24 space-y-0.5">
          <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">Width</span>
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
          <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">Offset (hex or dec)</span>
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
          <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">Op</span>
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
          <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">Target</span>
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
          Edge-trigger (fire once on transition) — uncheck for level-trigger ("currently in state")
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

export default PerGameSettingsDrawer;
