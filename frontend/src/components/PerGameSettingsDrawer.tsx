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
import CoreOptionsPanel from "./CoreOptionsPanel";
import AnalogBindingsSection from "./AnalogBindingsSection";
import {
  BezelPicker,
  OverscanEditor,
  overscanIsZero,
  overscanLabel,
  pathBasename,
  RewindLiveStats,
  type OverscanCropPrefs,
} from "./SystemDialogs";
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
  /// RetroArch-parity slice — IPS/UPS/BPS patch applied to ROM bytes
  /// before the core sees them. Absolute path. Null = no patching.
  patchPath?: string | null;
  rewindEnabled?: boolean | null;
  rewindCaptureIntervalFrames?: number | null;
  rewindBufferMegabytes?: number | null;
  /// Override the renderer's display_aspect for this game. Wins over
  /// per-system. None = inherit per-system → core-reported.
  displayAspectOverride?: number | null;
  /// Per-game per-edge overscan crop. Wins over per-system. None or
  /// all-zero = inherit per-system → no crop.
  overscanCropOverride?: OverscanCropPrefs | null;
  /// Per-game bezel image override. Wins over per-system and over the
  /// active shader preset's TOML default.
  bezelImagePath?: string | null;
  /// Free-form per-game keypad layout note. Coleco / Intv / O2 shipped
  /// paper overlays that told the player what each number meant in the
  /// active game ("KP1=climb, KP2=duck"). Operators record those
  /// mappings here as a reference panel; bindings still live in the
  /// per-system Bindings page. Null / empty string = no note.
  keypadLayoutNote?: string | null;
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

/// Common display-aspect presets surfaced in the dropdown. Mirrors the
/// per-system dialog's `DISPLAY_ASPECT_PRESETS`; kept duplicated rather
/// than imported to avoid coupling the per-game drawer to the per-
/// system dialog module.
const DISPLAY_ASPECT_PRESETS: readonly { value: string; label: string }[] = [
  { value: "1.333", label: "4:3 (CRT TV)" },
  { value: "1.778", label: "16:9 (Widescreen)" },
  { value: "1.0",   label: "1:1 (Square pixels)" },
  { value: "1.143", label: "8:7 (NES authentic)" },
  { value: "1.185", label: "32:27 (PCE 256 authentic)" },
  { value: "1.306", label: "64:49 (PCE 352 authentic)" },
];

/// Format an aspect ratio as a human-readable label matching the preset
/// list when the value lands on one of them. Falls back to the raw
/// number formatted as `1.234`. Used for the inherited-value chip.
function aspectLabel(value: number): string {
  const preset = DISPLAY_ASPECT_PRESETS.find((p) => Math.abs(Number(p.value) - value) < 0.01);
  if (preset) return preset.label;
  return value.toFixed(3);
}

const TABS = ["overview", "core", "core-options", "display", "input", "rewind", "shaders", "region", "milestones", "cheats"] as const;
export type GameDrawerTab = typeof TABS[number];
type TabId = GameDrawerTab;
const TAB_LABELS: Record<TabId, string> = {
  overview:       "Overview",
  core:           "Core",
  "core-options": "Core options",
  display:        "Display",
  input:          "Input",
  rewind:         "Rewind",
  shaders:        "Shaders",
  region:         "Region",
  milestones:     "Milestones",
  cheats:         "Cheats",
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
  /// Optional deep-link target — landing tab when the drawer opens. The
  /// Game ▾ menu items use this so e.g. "Cheats…" drops the user directly
  /// on the Cheats tab instead of always opening on Overview.
  initialTab?: TabId;
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
    setActiveTab(props.initialTab ?? "overview");
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
    if (next.displayAspectOverride != null) cleaned.displayAspectOverride = next.displayAspectOverride;
    if (next.overscanCropOverride != null && !overscanIsZero(next.overscanCropOverride)) {
      cleaned.overscanCropOverride = next.overscanCropOverride;
    }
    if (next.bezelImagePath != null && next.bezelImagePath !== "") {
      cleaned.bezelImagePath = next.bezelImagePath;
    }
    // Keypad layout note — empty string collapses to "no note" so an
    // operator who blanks the textarea clears the override rather than
    // persisting an empty entry.
    if (next.keypadLayoutNote != null && next.keypadLayoutNote.trim() !== "") {
      cleaned.keypadLayoutNote = next.keypadLayoutNote;
    }
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

                {/* RetroArch parity slice — soft patch */}
                <SettingRow
                  label="ROM patch"
                  hint="IPS / UPS / BPS patch applied to ROM bytes before the core sees them"
                  inheritedValue="No patch"
                  overridden={overrides().patchPath != null}
                >
                  <div class="flex items-center gap-2">
                    <span
                      class="flex-1 truncate text-xs"
                      classList={{
                        "text-(--color-oa-ink)": overrides().patchPath != null,
                        "text-(--color-oa-ink-dim)": overrides().patchPath == null,
                      }}
                      title={overrides().patchPath ?? "No patch"}
                    >
                      {overrides().patchPath
                        ? overrides().patchPath!.split(/[\/\\]/).pop()
                        : "No patch selected"}
                    </span>
                    <button
                      type="button"
                      class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-xs text-(--color-oa-ink) hover:bg-white/[0.08]"
                      onClick={async (e) => {
                        e.currentTarget.blur();
                        try {
                          const picked = await invoke<string | null>("pick_patch_file");
                          if (picked) void patch({ patchPath: picked });
                        } catch (err) {
                          console.warn("[oa-properties] pick_patch_file failed:", err);
                        }
                      }}
                    >
                      Pick…
                    </button>
                    <Show when={overrides().patchPath != null}>
                      <button
                        type="button"
                        class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-xs text-(--color-oa-ink-dim) hover:bg-white/[0.08]"
                        onClick={() => void patch({ patchPath: null })}
                      >
                        Clear
                      </button>
                    </Show>
                  </div>
                </SettingRow>
                <p class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                  Takes effect on next launch. Byte-source ROMs only (HuCards, NES, SNES carts);
                  CD images can't be patched in-place from this side.
                </p>
              </div>
            </Show>

            {/* --- Display ------------------------------------------------ */}
            <Show when={activeTab() === "display"}>
              <div class="flex flex-col gap-3">
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
                <SettingRow
                  label="Display aspect"
                  hint="Pixel-aspect at the renderer; affects Aspect-correct + Pixel-perfect modes"
                  inheritedValue={
                    systemSettings().displayAspectOverride != null
                      ? `${aspectLabel(systemSettings().displayAspectOverride!)} (Per-system)`
                      : "Core-reported"
                  }
                  inheritedFrom={
                    systemSettings().displayAspectOverride != null
                      ? "Per-system"
                      : "OA default"
                  }
                  overridden={overrides().displayAspectOverride != null}
                >
                  <select
                    class={SELECT_CLASS}
                    value={overrides().displayAspectOverride?.toString() ?? ""}
                    onChange={(e) => {
                      const v = e.currentTarget.value;
                      void patch({
                        displayAspectOverride: v === "" ? null : Number(v),
                      });
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
                    overscanIsZero(systemSettings().overscanCropOverride)
                      ? "OA default"
                      : "Per-system"
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
              </div>
            </Show>

            {/* --- Input (analog routing per-game override) ---------------- */}
            <Show when={activeTab() === "input"}>
              <div class="flex flex-col gap-3">
                <p class="text-xs text-(--color-oa-ink-dim)">
                  Per-game analog routing overrides. When set, these values
                  layer on top of the per-system Analog input settings — at
                  launch, each port resolves per-game → per-system → identity.
                  Use this to tweak deadzone / sensitivity / keyboard fallback
                  for a specific game without affecting the system's default.
                </p>
                <AnalogBindingsSection
                  systemId={props.entry!.systemId}
                  mode="game"
                  gameId={props.entry!.id}
                />
                {/* Keypad layout note — surface for systems whose canonical
                    controller had a non-game-specific keypad shipped with
                    paper overlays. Coleco / Intv / O2 are the canonical
                    examples; the note carries through cleanly for any
                    system the operator wants to leave a free-form
                    reference on. */}
                <Show when={["coleco", "intv", "o2"].includes(props.entry!.systemId)}>
                  <div class="rounded border border-(--color-oa-bg) bg-(--color-oa-bg)/40 p-3">
                    <div class="mb-1 text-xs font-medium text-(--color-oa-ink)">
                      Keypad layout note
                    </div>
                    <div class="mb-2 text-[11px] leading-snug text-(--color-oa-ink-dim)">
                      This system shipped paper overlays so the keypad meant
                      something different in each game. Record what KP1–KP9 do
                      in this title; the per-system Bindings page still owns
                      which keyboard key triggers which KP.
                    </div>
                    <textarea
                      class="w-full resize-y rounded border border-(--color-oa-bg-deep) bg-(--color-oa-bg-deep) px-2 py-1.5 text-xs text-(--color-oa-ink) placeholder:text-(--color-oa-ink-dim)/60 focus:border-(--color-system-accent) focus:outline-none"
                      rows="3"
                      placeholder="KP1=climb-up, KP2=climb-down, KP3=jump, KP4=duck…"
                      value={overrides().keypadLayoutNote ?? ""}
                      onChange={(e) => {
                        const v = e.currentTarget.value;
                        void patch({ keypadLayoutNote: v.trim() === "" ? null : v });
                      }}
                    />
                  </div>
                </Show>
              </div>
            </Show>

            {/* --- Rewind -------------------------------------------------- */}
            <Show when={activeTab() === "rewind"}>
              <div class="flex flex-col gap-3">
                <RewindLiveStats open={activeTab() === "rewind"} />
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

            {/* --- Core options ---------------------------------------- */}
            <Show when={activeTab() === "core-options"}>
              <CoreOptionsPanel
                systemId={props.entry?.systemId ?? ""}
                gameId={props.entry?.id ?? null}
              />
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

            {/* --- Cheats ------------------------------------------------ */}
            <Show when={activeTab() === "cheats"}>
              <CheatsTab gameId={props.entry?.id ?? ""} />
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

// --- Cheats tab body --------------------------------------------------

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
  /// "memory_poke" (default) — writes value to (region, offset, width) every
  /// frame via memory_region_mut. "libretro_code" — passes `code` through
  /// retro_cheat_set and lets the core decode Game Genie / GameShark /
  /// Action Replay / etc. per its system's conventions.
  kind: string;
  /// Raw code string for libretro_code cheats. Ignored for memory_poke.
  code?: string | null;
};

type CheatsTabProps = {
  gameId: string;
};

const REGION_OPTIONS = ["system_ram", "save_ram", "video_ram", "rtc"] as const;

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

const CheatsTab: Component<CheatsTabProps> = (props) => {
  const [cheats, setCheats] = createSignal<Cheat[]>([]);
  const [draft, setDraft] = createSignal<Cheat | null>(null);
  // Cheat search state
  const [searchActive, setSearchActive] = createSignal(false);
  const [searchRegion, setSearchRegion] = createSignal("system_ram");
  const [searchSummary, setSearchSummary] = createSignal<CheatSearchSummary | null>(null);
  const [searchEqualValue, setSearchEqualValue] = createSignal(0);
  const [searchPending, setSearchPending] = createSignal(false);

  async function startSearch() {
    setSearchPending(true);
    try {
      const summary = await invoke<CheatSearchSummary>("start_cheat_search", { region: searchRegion() });
      setSearchSummary(summary);
      setSearchActive(true);
    } catch (e) {
      console.warn("[cheat-search] start failed:", e);
      window.alert(`Couldn't start search: ${typeof e === "string" ? e : (e as Error)?.message}`);
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
      console.warn("[cheat-search] filter failed:", e);
    } finally {
      setSearchPending(false);
    }
  }
  async function peek() {
    try {
      const summary = await invoke<CheatSearchSummary>("peek_cheat_search");
      setSearchSummary(summary);
    } catch (e) {
      console.warn("[cheat-search] peek failed:", e);
    }
  }
  async function endSearch() {
    try {
      await invoke("end_cheat_search");
    } catch (e) {
      console.warn("[cheat-search] end failed:", e);
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

  async function refresh() {
    if (!props.gameId) {
      setCheats([]);
      return;
    }
    try {
      const list = await invoke<Cheat[]>("list_cheats", { gameId: props.gameId });
      setCheats(list);
    } catch (e) {
      console.warn("[cheats] list failed:", e);
    }
  }

  createEffect(() => {
    if (props.gameId) void refresh();
  });

  async function rearm() {
    try {
      await invoke<number>("arm_cheats", { gameId: props.gameId });
    } catch (e) {
      console.warn("[cheats] arm failed:", e);
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
      console.warn("[cheats] save failed:", e);
    }
  }

  async function toggle(c: Cheat) {
    const next = { ...c, enabled: !c.enabled };
    try {
      await invoke("update_cheat", { cheat: next });
      await refresh();
      await rearm();
    } catch (e) {
      console.warn("[cheats] toggle failed:", e);
    }
  }

  async function remove(id: number) {
    if (!window.confirm("Delete this cheat?")) return;
    try {
      await invoke("delete_cheat", { id });
      await refresh();
      await rearm();
    } catch (e) {
      console.warn("[cheats] delete failed:", e);
    }
  }

  function blank(): Cheat {
    return {
      gameId: props.gameId,
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

  return (
    <div class="flex flex-col gap-3">
      <p class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
        Memory-poke cheats. Each enabled row writes `value` (`width` bytes, little-endian)
        to memory at `(region, offset)` every frame. Game Genie / Action Replay codes
        need to be translated to raw address+value via online tables for now;
        per-system code decoders are a follow-up.
      </p>

      {/* --- Cheat search --- */}
      <div class="rounded-md border border-white/10 bg-white/[0.02] p-3">
        <div class="flex items-center justify-between gap-3">
          <span class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            Cheat search {searchActive() && searchSummary() ? `· ${searchSummary()!.candidateCount} candidates` : ""}
          </span>
          <Show when={!searchActive()} fallback={
            <button
              type="button"
              onClick={() => void endSearch()}
              class="text-[0.65rem] uppercase tracking-wider text-(--color-oa-ink-dim) hover:text-(--color-oa-ink)"
            >
              End
            </button>
          }>
            <div class="flex items-center gap-2">
              <select
                value={searchRegion()}
                onChange={(e) => setSearchRegion(e.currentTarget.value)}
                class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-xs text-(--color-oa-ink)"
              >
                <For each={REGION_OPTIONS}>{(r) => <option value={r}>{r}</option>}</For>
              </select>
              <button
                type="button"
                onClick={() => void startSearch()}
                disabled={searchPending() || !props.gameId}
                class="rounded-md border border-(--color-system-accent)/40 bg-(--color-system-accent)/15 px-2 py-1 text-xs uppercase tracking-wider text-(--color-system-accent-soft) disabled:opacity-50"
              >
                Start search
              </button>
            </div>
          </Show>
        </div>

        <Show when={searchActive()}>
          <div class="mt-3 flex flex-wrap items-center gap-2">
            <button type="button" disabled={searchPending()} onClick={() => void runFilter({ kind: "changed" })}
              class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-xs text-(--color-oa-ink) hover:bg-white/[0.08] disabled:opacity-50">≠ Changed</button>
            <button type="button" disabled={searchPending()} onClick={() => void runFilter({ kind: "unchanged" })}
              class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-xs text-(--color-oa-ink) hover:bg-white/[0.08] disabled:opacity-50">= Unchanged</button>
            <button type="button" disabled={searchPending()} onClick={() => void runFilter({ kind: "increased" })}
              class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-xs text-(--color-oa-ink) hover:bg-white/[0.08] disabled:opacity-50">▲ Increased</button>
            <button type="button" disabled={searchPending()} onClick={() => void runFilter({ kind: "decreased" })}
              class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-xs text-(--color-oa-ink) hover:bg-white/[0.08] disabled:opacity-50">▼ Decreased</button>
            <span class="text-[0.65rem] text-(--color-oa-ink-dim)">or</span>
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
            <button type="button" disabled={searchPending()} onClick={() => void runFilter({ kind: "equal_to_value", value: searchEqualValue() })}
              class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-xs text-(--color-oa-ink) hover:bg-white/[0.08] disabled:opacity-50">= value</button>
            <button type="button" onClick={() => void peek()}
              class="ml-auto rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-[0.65rem] uppercase tracking-wider text-(--color-oa-ink-dim) hover:text-(--color-oa-ink)">↻ Refresh</button>
          </div>

          <Show when={searchSummary() && searchSummary()!.top.length > 0} fallback={
            <p class="mt-2 text-[0.6rem] text-(--color-oa-ink-dim)">
              Do something in-game that changes the value you're looking for, then pick a filter above.
            </p>
          }>
            <div class="mt-3 flex max-h-48 flex-col gap-1 overflow-y-auto">
              <For each={searchSummary()!.top}>
                {(hit) => (
                  <div class="flex items-center gap-2 rounded border border-white/5 bg-white/[0.02] px-2 py-1.5 text-xs font-mono">
                    <span class="text-(--color-oa-ink-dim)">0x{hit.offset.toString(16).toUpperCase().padStart(4, "0")}</span>
                    <span class="text-(--color-oa-ink)">= {hit.currentValue}</span>
                    <span class="text-(--color-oa-ink-dim)">(was {hit.previousValue})</span>
                    <button
                      type="button"
                      onClick={() => makeCheatFrom(hit.offset, hit.currentValue)}
                      class="ml-auto rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) hover:text-(--color-oa-ink)"
                    >
                      Make cheat
                    </button>
                  </div>
                )}
              </For>
              <Show when={searchSummary()!.candidateCount > searchSummary()!.top.length}>
                <p class="text-[0.6rem] text-(--color-oa-ink-dim)">
                  + {searchSummary()!.candidateCount - searchSummary()!.top.length} more — apply another filter to narrow.
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
              <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                {c.region} · 0x{c.offset.toString(16).toUpperCase().padStart(4, "0")} ·
                {" "}{c.width}B · = {c.value}
              </p>
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
                    <option value="memory_poke">Memory poke (raw address + value)</option>
                    <option value="libretro_code">Code (Game Genie / GameShark / Action Replay / raw)</option>
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

                <Show when={d.kind === "libretro_code"} fallback={
                  <>
                    <label class="flex flex-col gap-1 text-xs">
                      <span class="uppercase tracking-widest text-(--color-oa-ink-dim)">Region</span>
                      <select
                        value={d.region}
                        onChange={(e) => setDraft({ ...d, region: e.currentTarget.value })}
                        class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-sm text-(--color-oa-ink)"
                      >
                        <For each={REGION_OPTIONS}>
                          {(r) => <option value={r}>{r}</option>}
                        </For>
                      </select>
                    </label>
                    <label class="flex flex-col gap-1 text-xs">
                      <span class="uppercase tracking-widest text-(--color-oa-ink-dim)">Width</span>
                      <select
                        value={String(d.width)}
                        onChange={(e) => setDraft({ ...d, width: Number(e.currentTarget.value) })}
                        class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-sm text-(--color-oa-ink)"
                      >
                        <option value="1">1 byte</option>
                        <option value="2">2 bytes</option>
                        <option value="4">4 bytes</option>
                      </select>
                    </label>
                    <label class="flex flex-col gap-1 text-xs">
                      <span class="uppercase tracking-widest text-(--color-oa-ink-dim)">Offset (hex)</span>
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
                      <span class="uppercase tracking-widest text-(--color-oa-ink-dim)">Value (decimal)</span>
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
                }>
                  <label class="col-span-2 flex flex-col gap-1 text-xs">
                    <span class="uppercase tracking-widest text-(--color-oa-ink-dim)">
                      Code (Game Genie / GameShark / Action Replay / raw address:value)
                    </span>
                    <input
                      type="text"
                      value={d.code ?? ""}
                      onInput={(e) => setDraft({ ...d, code: e.currentTarget.value })}
                      class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-sm font-mono text-(--color-oa-ink)"
                      placeholder="e.g. SXIOPO   ·   AENZIAEH+OZNZAAOE   ·   00B0CFA:09"
                    />
                    <span class="text-[0.6rem] text-(--color-oa-ink-dim)">
                      Format is decided by the core for this system. Beetle / Mednafen
                      cores generally accept Game Genie / Pro Action Replay / raw
                      <code> address:value</code> strings; FCEUmm + Mesen accept 6-char
                      Game Genie + ARLY format. Multiple codes joined with <code>+</code>.
                    </span>
                  </label>
                </Show>
              </div>
              <div class="mt-3 flex gap-2">
                <button
                  type="button"
                  onClick={() => void save(d)}
                  disabled={!d.name.trim()}
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
  );
};

export default PerGameSettingsDrawer;
