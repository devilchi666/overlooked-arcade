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
  createSignal,
  For,
  onMount,
  Show,
  type Component,
  type JSX,
} from "solid-js";
import { getBiosStatus } from "@oa/platform/api/coresApi";
import { listMonitors, listAudioDevices } from "@oa/platform/api/settingsApi";
import * as libraryApi from "@oa/platform/api/libraryApi";
import {
  getGameFocusToggle,
  setGameFocusToggle,
  setUiIntercepting,
} from "@oa/platform/api/shellApi";
import * as jobsApi from "@oa/platform/api/jobsApi";
import { detectCpuTier, getSystemStatus } from "@oa/platform/api/systemApi";
import { confirm } from "@oa/platform/lib/confirm";
import { emitEvent, OA_EVENTS } from "@oa/platform/api/eventsApi";
import {
  availableThemes,
  activeThemeId,
  setActiveTheme,
} from "@oa/platform/theme/registry";
import { open as pickDirectory } from "@tauri-apps/plugin-dialog";
import {
  refreshMameSystemInfo,
  type MameRefreshReport,
} from "@oa/platform/library/systemInfo";
import CoresPage from "./CoresPage";
import LibraryManagerPage from "./LibraryManagerPage";
import { refreshJobPrefs } from "@oa/platform/lib/backgroundJobs";
import DevToolsPanel from "./DevToolsPanel";
import AppearancePanel from "./AppearancePanel";
import { usePlatform } from "@oa/platform/platformContext";
import { setHelpDialog } from "@oa/platform/dialogs";
import { addLibraryFolder, rescanLibraryFolders } from "@oa/platform/libraryAdmin";
import SettingRow from "@oa/platform/components/SettingRow";
import {
  navBindings,
  setNavBindings,
  rawGamepadButtonForVerb,
  rebindGamepadVerb,
  DEFAULT_BINDINGS,
  type NavButton,
  type NavVerb,
} from "@oa/platform/nav";
import {
  SCALING_MODE_LABELS,
  SCALING_OPTIONS,
  WINDOW_MODE_LABELS,
  WINDOW_OPTIONS,
  type AudioDeviceInfo,
  type MonitorInfo,
  type ScalingMode,
  type SettingsStore,
  type WindowMode,
} from "@oa/platform/settings/store";
import { shaderPresets, shaderPresetLabel } from "@oa/platform/settings/shader_presets";
import { eventCodeToRustKey, formatKey } from "../systems/keymap";

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
      return await listMonitors();
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

// --- Game-focus toggle hotkey -----------------------------------------

const FOCUS_MODIFIER_KEYS = new Set([
  "LShift", "RShift", "LControl", "RControl",
  "LAlt", "RAlt", "LMeta", "RMeta",
]);

/// Capture control for the operator-configurable Game-focus toggle chord.
/// Reads the current binding via `get_game_focus_toggle`; "Change" listens
/// for a key combo (modifiers tracked left/right via `eventCodeToRustKey`)
/// and completes on the first non-modifier key, persisting via
/// `set_game_focus_toggle`. "Clear" unbinds the hotkey (toggle still works
/// from the running emulator via the chord — empty just disables it).
const GameFocusToggleCard: Component = () => {
  const [chord, setChord] = createSignal<string>("");
  const [capturing, setCapturing] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  onMount(() => {
    void getGameFocusToggle().then(setChord).catch(() => {});
  });

  const pretty = (c: string) =>
    c ? c.split("+").map((k) => formatKey(k)).join(" + ") : "Unbound";

  let held = new Set<string>();
  let keyDownHandler: ((e: KeyboardEvent) => void) | null = null;
  let keyUpHandler: ((e: KeyboardEvent) => void) | null = null;

  const stopCapture = () => {
    if (keyDownHandler) window.removeEventListener("keydown", keyDownHandler, true);
    if (keyUpHandler) window.removeEventListener("keyup", keyUpHandler, true);
    keyDownHandler = null;
    keyUpHandler = null;
    held = new Set();
    // Release the input gate we took while capturing.
    void setUiIntercepting(false).catch(() => {});
    setCapturing(false);
  };

  const save = (c: string) => {
    void setGameFocusToggle(c)
      .then((normalized) => {
        setChord(normalized);
        setError(null);
      })
      .catch((e) => setError(String(e)));
  };

  const beginCapture = () => {
    setError(null);
    held = new Set();
    setCapturing(true);
    // Gate gameplay/hotkey input while we listen so stray keys don't fire
    // hotkeys or leak to the core during capture.
    void setUiIntercepting(true).catch(() => {});
    keyDownHandler = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") {
        stopCapture();
        return;
      }
      const name = eventCodeToRustKey(e.code);
      if (!name) return;
      held.add(name);
      // Complete the chord on the first non-modifier key, capturing whatever
      // modifiers are still held (insertion order → "LControl+G").
      if (!FOCUS_MODIFIER_KEYS.has(name)) {
        const combo = Array.from(held).join("+");
        stopCapture();
        save(combo);
      }
    };
    keyUpHandler = (e: KeyboardEvent) => {
      const name = eventCodeToRustKey(e.code);
      if (name) held.delete(name);
    };
    window.addEventListener("keydown", keyDownHandler, true);
    window.addEventListener("keyup", keyUpHandler, true);
  };

  return (
    <SettingsCard
      title="Game focus (keyboard routing)"
      description="When Game focus is ON, the keyboard goes to the emulated machine — type into MSX BASIC / DOS / arcade test menus — and OA's hotkeys (save/load/slots) are suppressed. When OFF, the keyboard drives the shell. This is the hotkey that toggles between the two; it always works, even from inside a running game."
    >
      <div class="flex items-center justify-between gap-4 py-2">
        <div class="flex flex-col gap-0.5">
          <span class="text-sm text-(--color-oa-ink)">Toggle hotkey</span>
          <span class="text-xs text-white/50">
            {capturing()
              ? "Press a key or combo… (Esc to cancel)"
              : pretty(chord())}
          </span>
          <Show when={error()}>
            <span class="text-xs text-red-400">{error()}</span>
          </Show>
        </div>
        <div class="flex gap-2">
          <button
            type="button"
            data-setting-action
            class="rounded-md border border-white/15 bg-white/5 px-3 py-1.5 text-xs text-(--color-oa-ink) hover:bg-white/10"
            onClick={() => (capturing() ? stopCapture() : beginCapture())}
          >
            {capturing() ? "Cancel" : "Change"}
          </button>
          <button
            type="button"
            data-setting-action
            class="rounded-md border border-white/15 bg-white/5 px-3 py-1.5 text-xs text-(--color-oa-ink) hover:bg-white/10 disabled:opacity-40"
            disabled={capturing() || !chord()}
            onClick={() => save("")}
          >
            Clear
          </button>
        </div>
      </div>
    </SettingsCard>
  );
};

// --- Controller navigation --------------------------------------------

// --- Nav button mapping (D18 remap UI) --------------------------------------
//
// Rebinds the SHELL-NAVIGATION verbs to physical gamepad buttons (the OA-wide
// `navBindings` map). This is the menu/UI nav layer — NOT the per-system
// gameplay bindings (SystemBindingsEditor maps a pad to the emulated console).
// Edits the LIVE `navBindings` signal, so dispatch + every on-screen glyph
// update instantly (no restart) and persist to `nav_bindings.json`.
//
// Scope: the gamepad ACTION/structural verbs. Directional movement stays on the
// D-pad / left stick (not per-button remappable in S1). The keyboard arrows +
// native Enter/Esc are the always-reachable escape hatch (D18) — they aren't
// editable here, so a user can never strand themselves with no way to confirm.

/// Action/structural verbs the gamepad map binds (directions excluded; the
/// reserved no-consumer verbs Search/Favorite/Page omitted until they do
/// something).
const REMAP_VERBS: ReadonlyArray<{ verb: NavVerb; label: string; hint?: string }> = [
  { verb: "Confirm", label: "Confirm", hint: "activate / launch" },
  { verb: "Back", label: "Back", hint: "cancel" },
  { verb: "Secondary", label: "Secondary", hint: "details / info" },
  { verb: "Tertiary", label: "Tertiary" },
  { verb: "PrevSection", label: "Previous section", hint: "cycle tabs ◄" },
  { verb: "NextSection", label: "Next section", hint: "cycle tabs ►" },
  { verb: "Menu", label: "Menu", hint: "context options" },
  { verb: "OpenQuickSettings", label: "Quick settings" },
];

/// Physical buttons offered. Excludes Select (reserved for the Select+Start
/// engine-summon chord) + stick clicks / Home (special).
const REMAP_BUTTONS: ReadonlyArray<NavButton> = [
  "a",
  "b",
  "x",
  "y",
  "l1",
  "r1",
  "l2",
  "r2",
  "start",
];

const REMAP_BUTTON_LABEL: Record<string, string> = {
  a: "A",
  b: "B",
  x: "X",
  y: "Y",
  l1: "LB / L1",
  r1: "RB / R1",
  l2: "LT / L2",
  r2: "RT / R2",
  start: "Start",
};

const NavRemapCard: Component = () => {
  // Set `verb`'s gamepad button (or unbind), with conflict resolution in the
  // pure helper — assigning a button steals it from any verb that had it (that
  // verb's row re-renders as Unbound). Persists + applies instantly.
  const rebind = (verb: NavVerb, button: NavButton | null): void => {
    void setNavBindings(rebindGamepadVerb(navBindings(), verb, button));
  };

  const currentButton = (verb: NavVerb): NavButton | null =>
    rawGamepadButtonForVerb(navBindings(), verb);
  const defaultButton = (verb: NavVerb): NavButton | null =>
    rawGamepadButtonForVerb(DEFAULT_BINDINGS, verb);

  const confirmUnbound = (): boolean => currentButton("Confirm") === null;
  const backUnbound = (): boolean => currentButton("Back") === null;

  return (
    <SettingsCard
      title="Button mapping"
      description="Rebind how the gamepad drives the menus (not in-game controls — those are per-system, in Library Manager). Changes apply instantly. D-pad / left stick always move focus; keyboard arrows + Enter/Esc always work as a fallback."
    >
      <Show when={confirmUnbound() || backUnbound()}>
        <div class="rounded-md border border-amber-400/30 bg-amber-400/10 px-4 py-2.5 text-xs leading-relaxed text-amber-200/90">
          {confirmUnbound() && backUnbound()
            ? "Confirm and Back"
            : confirmUnbound()
              ? "Confirm"
              : "Back"}{" "}
          {confirmUnbound() && backUnbound() ? "have" : "has"} no gamepad button. The keyboard
          (Enter / Esc) still works, but you may want to assign a button below.
        </div>
      </Show>

      <For each={REMAP_VERBS}>
        {(row) => {
          const cur = (): NavButton | null => currentButton(row.verb);
          const def = (): NavButton | null => defaultButton(row.verb);
          return (
            <SettingRow
              label={row.label}
              hint={row.hint}
              inherited={null}
              overridden={cur() !== def()}
              onReset={() => rebind(row.verb, def())}
              select={{
                value: cur() ?? "",
                options: [
                  { value: "", label: "— Unbound —" },
                  ...REMAP_BUTTONS.map((b) => ({ value: b, label: REMAP_BUTTON_LABEL[b] })),
                ],
                onChange: (v) => rebind(row.verb, v === "" ? null : (v as NavButton)),
              }}
            />
          );
        }}
      </For>

      <div class="flex items-center justify-between gap-4 pt-1">
        <p class="text-xs text-(--color-oa-ink-dim)">
          The “Swap A and B” toggle above applies on top of this mapping.
        </p>
        <button
          type="button"
          data-setting-action
          onClick={() => void setNavBindings(DEFAULT_BINDINGS)}
          class="shrink-0 rounded-md border border-white/10 bg-white/[0.03] px-3 py-1.5 text-xs font-medium text-(--color-oa-ink-dim) transition hover:bg-white/[0.07] hover:text-(--color-oa-ink)"
        >
          Reset to defaults
        </button>
      </div>
    </SettingsCard>
  );
};

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
      <NavRemapCard />
      <GameFocusToggleCard />
    </div>
  );
};

// --- Experimental (preview-quality feature toggles) -------------------

/// Mirror of Rust `LibraryPrefs` (camelCase via #[serde(rename_all)]).
/// Only the fields ExperimentalSettings actually reads/writes are
/// typed — others survive round-trip via JS spread.
type LibraryPrefsLike = Record<string, unknown> & {
  discTrackExperimentalEnabled?: boolean;
};

export const ExperimentalSettings: Component = () => {
  const [devToolsOpen, setDevToolsOpen] = createSignal(false);

  // Mirror of LibraryPrefs.disc_track_experimental_enabled from the
  // backend (prefs.json). Fetched once on mount; writes round-trip
  // through set_library_prefs with the rest of the prefs object
  // spread-preserved so we don't reset region/revision priority.
  // Default false matches the Rust default after the 2026-06-03 pivot.
  const [libraryPrefs, setLibraryPrefs] = createSignal<LibraryPrefsLike | null>(null);
  onMount(() => {
    libraryApi.getLibraryPrefs<LibraryPrefsLike>()
      .then((p) => setLibraryPrefs(p))
      .catch((e) => console.warn("get_library_prefs failed:", e));
  });
  async function setDiscTrackExperimental(v: boolean) {
    const current = libraryPrefs();
    if (!current) return;
    const next = { ...current, discTrackExperimentalEnabled: v };
    setLibraryPrefs(next);
    try {
      await libraryApi.setLibraryPrefs(next);
    } catch (e) {
      console.warn("set_library_prefs failed:", e);
    }
  }

  return (
    <div class="flex flex-col gap-4">
      <SettingsCard
        title="Experimental"
        description="Preview-quality features still under active development. Safe to toggle; defaults preserve today's behavior."
      >
        <SettingRow
          label="Per-track SHA-1 disc identification"
          inherited={null}
          overridden={false}
          toggle={{
            checked: !!libraryPrefs()?.discTrackExperimentalEnabled,
            onChange: (v) => {
              void setDiscTrackExperimental(v);
            },
          }}
          description="When ON, Identify ROMs on disc-shape systems (PSX / Saturn / Sega CD / Dreamcast / PCE-CD / etc.) computes per-track SHA-1 hashes against the redump catalog before falling back to filename match. Costs 5–60 seconds per disc on the first run; later runs hit the per-game cache. Works only on raw, unarchived .cue+.bin or .gdi extracted directly from DiscImageCreator — CHD-stored discs and archived ZIPs won't match the redump-published bytes regardless. Default OFF; filename-based fuzzy matching is the primary path."
        />
      </SettingsCard>

      {/* Dev tools — collapsed-by-default disclosure for affordances only
          developers ever need. Lives behind a click so it doesn't add
          noise for normal operators. The Background Jobs test spawner
          moved here from Settings → Library on 2026-06-03. */}
      <SettingsCard title="Dev tools">
        <button
          type="button"
          data-setting-action
          aria-expanded={devToolsOpen()}
          onClick={(e) => {
            e.currentTarget.blur();
            setDevToolsOpen((v) => !v);
          }}
          class="flex w-full items-center justify-between gap-2 rounded-md border border-white/10 bg-white/[0.02] px-3 py-2 text-left text-[0.75rem] text-(--color-oa-ink) transition hover:border-white/20 hover:bg-white/[0.04]"
        >
          <span>
            <span class="font-semibold">Background Jobs — test spawner</span>
            <span class="ml-2 text-[0.65rem] text-(--color-oa-ink-dim)">
              synthetic jobs to exercise the BackgroundJobsBar
            </span>
          </span>
          <span class="shrink-0 text-(--color-oa-ink-dim)" aria-hidden="true">
            {devToolsOpen() ? "▾" : "▸"}
          </span>
        </button>

        <Show when={devToolsOpen()}>
          <div class="mt-3 space-y-2 rounded-md border border-white/5 bg-white/[0.02] p-3">
            <p class="text-[0.65rem] leading-relaxed text-(--color-oa-ink-dim)">
              Spawns a synthetic background job that ticks progress at 10 Hz.
              Use it to exercise the bar's pause / resume / cancel /
              auto-collapse without burning a real core download. Click
              multiple times to test multi-job thresholds.
            </p>
            <div class="flex flex-wrap gap-2">
              <button
                type="button"
                class="rounded-md border border-(--color-system-accent)/40 bg-(--color-system-accent)/15 px-3 py-1.5 text-xs font-semibold uppercase tracking-wider text-(--color-oa-ink) transition hover:border-(--color-system-accent) hover:bg-(--color-system-accent)/25"
                onClick={(e) => {
                  e.currentTarget.blur();
                  void jobsApi.spawnTestJob(30).catch((err) => {
                    console.warn("[oa-jobs] spawn_test_job failed:", err);
                  });
                }}
              >
                Spawn 30 s test job
              </button>
              <button
                type="button"
                class="rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-xs font-semibold uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:border-white/20 hover:text-(--color-oa-ink)"
                onClick={(e) => {
                  e.currentTarget.blur();
                  void jobsApi.spawnTestJob(10).catch((err) => {
                    console.warn("[oa-jobs] spawn_test_job failed:", err);
                  });
                }}
              >
                Spawn 10 s test job
              </button>
            </div>
          </div>
        </Show>
      </SettingsCard>
    </div>
  );
};

// --- Audio --------------------------------------------------------------

export const AudioSettings: Component<{ settings: SettingsStore }> = (props) => {
  const [audioDevices] = createResource(async (): Promise<AudioDeviceInfo[]> => {
    try {
      return await listAudioDevices();
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

// --- Performance — Guided Setup Phase 2 ------------------------------

/// Mirror of Rust `cpu_tier::CpuTier` (camelCase via serde).
type CpuTierVariant = "high" | "mid" | "low";

/// Mirror of Rust `cpu_tier::TierSource`.
type TierSource = "detected" | "cached" | "override";

/// Mirror of Rust `cpu_tier::CpuTierSnapshot`. Returned by the
/// `detect_cpu_tier` Tauri command.
type CpuTierSnapshot = {
  tier: CpuTierVariant;
  source: TierSource;
  info: {
    brand: string;
    coresPhysical: number;
    baseClockGhz: number;
  };
};

/// Mirror of the relevant LibraryPrefs slice for the override path.
/// Other fields round-trip via spread.
type PerfPrefsLike = Record<string, unknown> & {
  cpuTierOverride?: CpuTierVariant | null;
};

const TIER_LABELS: Record<CpuTierVariant, string> = {
  high: "High",
  mid: "Mid",
  low: "Low",
};

const SOURCE_HINTS: Record<TierSource, string> = {
  detected: "Auto-detected from your CPU this session.",
  cached: "Cached from first launch. Will re-detect if you swap hardware.",
  override: "Manually set in this dialog. Auto-detection is bypassed.",
};

export const PerformanceSettings: Component<{ settings: SettingsStore }> = (_props) => {
  // Detected snapshot — refetched after each override change so the
  // tier chip reflects what `detect_or_load` would return on next launch.
  const [snapshot, setSnapshot] = createSignal<CpuTierSnapshot | null>(null);
  const [prefs, setPrefs] = createSignal<PerfPrefsLike | null>(null);

  async function refresh() {
    try {
      const [s, p] = await Promise.all([
        detectCpuTier<CpuTierSnapshot>(),
        libraryApi.getLibraryPrefs<PerfPrefsLike>(),
      ]);
      setSnapshot(s);
      setPrefs(p);
    } catch (e) {
      console.warn("PerformanceSettings refresh failed:", e);
    }
  }

  onMount(() => void refresh());

  async function setOverride(value: "auto" | CpuTierVariant) {
    const current = prefs();
    if (!current) return;
    const next: PerfPrefsLike = {
      ...current,
      cpuTierOverride: value === "auto" ? null : value,
    };
    setPrefs(next);
    try {
      await libraryApi.setLibraryPrefs(next);
      await refresh();
    } catch (e) {
      console.warn("set_library_prefs failed:", e);
    }
  }

  const currentOverride = (): "auto" | CpuTierVariant => {
    const o = prefs()?.cpuTierOverride;
    if (o === "high" || o === "mid" || o === "low") return o;
    return "auto";
  };

  return (
    <div class="flex flex-col gap-4">
      <SettingsCard
        title="CPU tier"
        description="Drives core picks in the Import Wizard. Detected from your CPU brand + physical core count + base clock. Override below if the auto-detection misclassifies your hardware (Steam Deck under-rates, old workstations over-rate, throttled laptops over-rate)."
      >
        {/* Read-only detected-hardware summary. Always visible so the
            operator can see what OA thinks the host is, even when an
            override is in play. */}
        <Show
          when={snapshot()}
          fallback={
            <p class="px-1 text-[0.75rem] text-(--color-oa-ink-dim)">
              Detecting CPU…
            </p>
          }
        >
          {(snap) => (
            <div class="flex flex-col gap-2 rounded-md border border-white/5 bg-white/[0.02] p-3">
              <div class="flex items-baseline justify-between gap-3">
                <span class="text-[0.7rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                  Detected
                </span>
                <span class="inline-flex items-center gap-2 rounded border border-(--color-system-accent)/40 bg-(--color-system-accent)/15 px-2 py-0.5 text-[0.7rem] font-mono uppercase tracking-wide text-(--color-system-accent-soft)">
                  {TIER_LABELS[snap().tier]}-tier
                </span>
              </div>
              <p class="font-mono text-xs text-(--color-oa-ink)">
                {snap().info.brand}
              </p>
              <p class="text-[0.7rem] text-(--color-oa-ink-dim)">
                {snap().info.coresPhysical} physical core{snap().info.coresPhysical === 1 ? "" : "s"}
                {" · "}
                {snap().info.baseClockGhz.toFixed(2)} GHz base
              </p>
              <p class="text-[0.65rem] italic text-(--color-oa-ink-dim)">
                {SOURCE_HINTS[snap().source]}
              </p>
            </div>
          )}
        </Show>

        <SettingRow
          label="Override tier"
          inherited={null}
          overridden={currentOverride() !== "auto"}
          select={{
            value: currentOverride(),
            options: [
              { value: "auto", label: "Auto (detected)" },
              { value: "high", label: "High" },
              { value: "mid", label: "Mid" },
              { value: "low", label: "Low" },
            ],
            onChange: (v) => {
              void setOverride(v as "auto" | CpuTierVariant);
            },
          }}
          description="Auto uses the detected tier. Pick a specific tier to override — the Import Wizard will recommend cores for that tier instead. Useful for Steam Deck (set High; iGPU is stronger than CPU rating suggests), or for thermal-throttled laptops (set Mid or Low if sustained performance falls short of boost-clock readings)."
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

// --- Profile -----------------------------------------------------------

/// A small set of presets so the operator doesn't have to remember
/// which emoji renders well at chip size. Custom emoji still works via
/// the freeform input.
const AVATAR_PRESETS = ["👤", "🎮", "👾", "🕹", "🤖", "🦊", "🐺", "⚡", "🌙", "🍄", "🦄", "🎯"];

export const ProfileSettings: Component<{ settings: SettingsStore }> = (props) => {
  return (
    <div class="flex flex-col gap-4">
      <SettingsCard title="Identity">
        <div class="flex items-center gap-4">
          <div class="grid h-16 w-16 shrink-0 place-items-center rounded-full border border-(--color-system-accent)/40 bg-(--color-system-accent)/15 text-3xl">
            {props.settings.profileAvatar() || "👤"}
          </div>
          <div class="min-w-0 flex-1">
            <label class="block text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Display name
            </label>
            <input
              type="text"
              value={props.settings.profileDisplayName()}
              onInput={(e) => props.settings.setProfileDisplayName(e.currentTarget.value)}
              placeholder="Your name"
              maxLength={32}
              class="mt-1 w-full rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-sm text-(--color-oa-ink) placeholder:text-(--color-oa-ink-dim)/60 focus:border-(--color-system-accent) focus:outline-none"
            />
          </div>
        </div>
      </SettingsCard>

      <SettingsCard title="Avatar">
        <div class="flex flex-col gap-3">
          <div class="flex flex-wrap gap-2">
            <For each={AVATAR_PRESETS}>
              {(preset) => {
                const isActive = () => props.settings.profileAvatar() === preset;
                return (
                  <button
                    type="button"
                    onClick={(e) => {
                      e.currentTarget.blur();
                      props.settings.setProfileAvatar(preset);
                    }}
                    class="grid h-10 w-10 place-items-center rounded-full border text-xl transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
                    classList={{
                      "border-(--color-system-accent) bg-(--color-system-accent)/15":
                        isActive(),
                      "border-white/10 bg-white/[0.04] hover:border-white/20": !isActive(),
                    }}
                    aria-pressed={isActive()}
                  >
                    {preset}
                  </button>
                );
              }}
            </For>
          </div>
          <label class="flex flex-col gap-1">
            <span class="text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Or paste a custom emoji
            </span>
            <input
              type="text"
              value={props.settings.profileAvatar()}
              onInput={(e) => {
                // Limit to a single grapheme — emoji can be multi-codepoint
                // but most chip-friendly avatars are single visible glyphs.
                const raw = e.currentTarget.value;
                const trimmed = [...raw][0] ?? "";
                props.settings.setProfileAvatar(trimmed);
              }}
              maxLength={8}
              class="w-full rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-sm text-(--color-oa-ink) focus:border-(--color-system-accent) focus:outline-none"
            />
          </label>
          <p class="text-[0.65rem] text-(--color-oa-ink-dim)/70">
            The avatar drives the top-right profile chip on the
            Retroverse shell. Custom image uploads land in a future
            slice once the content-packs avatar pipeline ships.
          </p>
        </div>
      </SettingsCard>
    </div>
  );
};

// --- Cores -------------------------------------------------------------

export const CoresCategorySettings: Component = () => {
  // CoresPage embeds full-page chrome (header, etc.) — in the SETTINGS
  // tab context we let it own the whole center pane. The onBack
  // callback is a no-op because the SETTINGS sidebar handles
  // navigation away from the category. Wrapping in a thin div keeps
  // the page's height calc happy.
  return (
    <div class="h-full -mx-8 -my-6">
      <CoresPage onBack={() => { /* no-op: SETTINGS sidebar owns nav */ }} />
    </div>
  );
};

// --- Library / Media / BIOS — informational cards ---------------------

export const LibrarySettings: Component = () => {
  // Settings IA redesign (Slice 1): Library is now the directory CUSTODIAN —
  // tracked folders + scan options, region/version priority, cleanup, and Game
  // media. Onboarding (the "Set up your library" wizard CTA) moved to the new
  // **Import & Setup** category; sidebar-systems visibility + Views +
  // Collections moved to the new **Organize My Collection** category. So this
  // category just embeds LibraryManagerPage's (now 2-tab) management surface.
  const platform = usePlatform();
  return (
    <div class="-mx-8 -mb-6">
      <LibraryManagerPage
        settings={platform.settings}
        library={platform.library}
        onAddLibraryFolder={addLibraryFolder}
        onRescanLibraryFolders={rescanLibraryFolders}
      />
    </div>
  );
};

// (Removed: the legacy `MediaSettings` category body — per-system art now lives
// in Settings → Systems → a system → Media; the global media prefs stay on the
// Library category's Game-media tab. Per-System Settings Hub arc S5.)

type BiosEntryStatus = "ok" | "unknownHash" | "missing" | "error";

type BiosStatusEntry = {
  slug: string;
  label: string;
  required: string;
  status: BiosEntryStatus;
  detail: string;
};

type BiosStatusResponse = {
  systemDir: string;
  entries: BiosStatusEntry[];
};

const BIOS_PILL_STYLES: Record<BiosEntryStatus, { ring: string; bg: string; text: string; label: string }> = {
  ok: {
    ring: "border-emerald-400/40",
    bg: "bg-emerald-500/10",
    text: "text-emerald-300",
    label: "Ready",
  },
  unknownHash: {
    ring: "border-amber-400/40",
    bg: "bg-amber-500/10",
    text: "text-amber-300",
    label: "Present · unknown hash",
  },
  missing: {
    ring: "border-red-400/40",
    bg: "bg-red-500/10",
    text: "text-red-300",
    label: "Missing",
  },
  error: {
    ring: "border-amber-400/40",
    bg: "bg-amber-500/10",
    text: "text-amber-300",
    label: "Read error",
  },
};

export const BiosSettings: Component = () => {
  const [status, { refetch }] = createResource(async () => {
    return await getBiosStatus<BiosStatusResponse>();
  });

  const counts = createMemo(() => {
    const list = status()?.entries ?? [];
    return {
      total: list.length,
      ok: list.filter((e) => e.status === "ok").length,
      unknownHash: list.filter((e) => e.status === "unknownHash").length,
      missing: list.filter((e) => e.status === "missing").length,
      error: list.filter((e) => e.status === "error").length,
    };
  });

  /// Two-group split: Issues (missing / unknown hash / read error) on top,
  /// Ready (ok) below. Each group sorted alphabetically by label so
  /// position is predictable. Operator spec 2026-06-03.
  const groupedEntries = createMemo(() => {
    const entries = status()?.entries ?? [];
    const byLabel = (a: BiosStatusEntry, b: BiosStatusEntry) =>
      a.label.localeCompare(b.label);
    return {
      issues: entries.filter((e) => e.status !== "ok").sort(byLabel),
      ready: entries.filter((e) => e.status === "ok").sort(byLabel),
    };
  });

  const renderBiosRow = (entry: BiosStatusEntry): JSX.Element => {
    const style = BIOS_PILL_STYLES[entry.status];
    return (
      <li class="grid grid-cols-[1fr_auto] gap-x-3 gap-y-1 rounded-lg border border-white/5 bg-white/[0.02] px-3 py-2">
        <div class="min-w-0">
          <p class="text-[0.8rem] font-semibold text-(--color-oa-ink)">
            {entry.label}
          </p>
          <p class="mt-0.5 truncate text-[0.65rem] text-(--color-oa-ink-dim)" title={entry.required}>
            {entry.required}
          </p>
        </div>
        <span
          class={`self-start rounded border ${style.ring} ${style.bg} px-2 py-0.5 text-[0.55rem] uppercase tracking-widest ${style.text}`}
        >
          {style.label}
        </span>
        <Show when={entry.detail}>
          <p class="col-span-2 break-all border-t border-white/5 pt-1.5 font-mono text-[0.6rem] text-(--color-oa-ink-dim)">
            {entry.detail}
          </p>
        </Show>
      </li>
    );
  };

  return (
    <div class="flex flex-col gap-4">
      <SettingsCard title="System directory">
        <p class="text-[0.7rem] text-(--color-oa-ink-dim)">
          BIOS files live in:
        </p>
        <code class="mt-2 block break-all rounded border border-white/10 bg-black/40 px-2 py-1.5 font-mono text-[0.65rem] text-(--color-oa-ink)">
          {status()?.systemDir ?? "Loading…"}
        </code>
        <p class="mt-3 text-[0.7rem] text-(--color-oa-ink-dim)">
          Drop the required BIOS files into this folder. OA verifies
          each at launch time and refuses to start a system with a
          missing / wrong BIOS (per-system sha-1 table). The pre-launch
          check surfaces the exact required filenames in the error
          toast if a BIOS is missing.
        </p>
      </SettingsCard>

      <SettingsCard title="BIOS status">
        <div class="flex items-center justify-between gap-3">
          <p class="text-[0.7rem] text-(--color-oa-ink-dim)">
            {status.loading
              ? "Scanning…"
              : `${counts().ok} ready · ${counts().unknownHash + counts().error} present-but-unrecognized · ${counts().missing} missing (of ${counts().total} systems).`}
          </p>
          <button
            type="button"
            onClick={() => refetch()}
            disabled={status.loading}
            class="rounded-md border border-white/10 bg-white/[0.04] px-3 py-1 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink) transition hover:bg-white/[0.08] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent) disabled:opacity-50"
          >
            Refresh
          </button>
        </div>

        <Show when={status()?.entries}>
          <div class="mt-3 flex flex-col gap-4">
            {/* Issues — anything not "ok". Hidden when empty so the
                Ready section can sit flush against the header on a
                fully-staged install. */}
            <Show when={groupedEntries().issues.length > 0}>
              <section>
                <h4 class="mb-1.5 flex items-center gap-2 text-[0.55rem] uppercase tracking-[0.4em] text-amber-300/90">
                  <span aria-hidden="true">⚠</span>
                  Missing &amp; issues
                  <span class="text-(--color-oa-ink-dim)/60">
                    · {groupedEntries().issues.length}
                  </span>
                </h4>
                <ul class="flex flex-col gap-1.5">
                  <For each={groupedEntries().issues}>{(entry) => renderBiosRow(entry)}</For>
                </ul>
              </section>
            </Show>

            {/* Ready — all "ok" entries. Hidden when empty (a fresh
                install with everything missing wouldn't have any). */}
            <Show when={groupedEntries().ready.length > 0}>
              <section>
                <h4 class="mb-1.5 flex items-center gap-2 text-[0.55rem] uppercase tracking-[0.4em] text-emerald-300/90">
                  <span aria-hidden="true">✓</span>
                  Ready
                  <span class="text-(--color-oa-ink-dim)/60">
                    · {groupedEntries().ready.length}
                  </span>
                </h4>
                <ul class="flex flex-col gap-1.5">
                  <For each={groupedEntries().ready}>{(entry) => renderBiosRow(entry)}</For>
                </ul>
              </section>
            </Show>
          </div>
        </Show>
        <p class="mt-3 text-[0.65rem] text-(--color-oa-ink-dim)/70">
          Green = filename + content hash both match a known canonical
          dump. Amber = file is present under a known name but its
          content hash isn't in OA's table (the core usually still
          boots; the launch path warns rather than refuses). Red = no
          accepted filename found in <code class="font-mono">system/</code>.
        </p>
      </SettingsCard>
    </div>
  );
};

// --- About -------------------------------------------------------------

export const AboutSettings: Component = () => {
  return (
    <div class="flex flex-col gap-4">
      <SettingsCard title="Overlooked Arcade">
        <div class="space-y-3 text-sm">
          <p class="text-[0.65rem] uppercase tracking-[0.3em] text-(--color-system-accent)">
            Premium emulator frontend
          </p>
          <p class="text-(--color-oa-ink-dim)">
            A dedicated home for the consoles modern emulators forgot —
            TurboGrafx-16, Lynx, Atari 7800, SMS / Game Gear, MSX,
            ColecoVision, Vectrex, Virtual Boy, WonderSwan, and friends.
          </p>
          <p class="text-(--color-oa-ink-dim)">
            Non-commercial. A gift to the retro community. Built on
            forked C cores (Beetle PCE Fast, Mednafen, MAME modules)
            loaded as libretro .dlls. Shell is GPL-2.0; cores keep
            their upstream licenses.
          </p>
        </div>
      </SettingsCard>

      <SettingsCard title="Credits">
        <div class="space-y-2 text-[0.75rem] text-(--color-oa-ink-dim)">
          <p>
            <span class="text-(--color-oa-ink)">Cores:</span> Beetle PCE
            Fast / Mednafen / Stella / Mesen / nestopia / fceumm /
            snes9x / Genesis Plus GX / PicoDrive / Beetle Saturn /
            mupen64plus / Dolphin / Flycast / PCSX2 / DOSBox-pure /
            ScummVM and many more.
          </p>
          <p>
            <span class="text-(--color-oa-ink)">Shell:</span> Rust +
            Tauri 2 + wgpu (WGSL) + Solid + Tailwind. Libretro
            integration via libloading.
          </p>
          <p>
            <span class="text-(--color-oa-ink)">Art &amp; metadata:</span>{" "}
            libretro-thumbnails + LaunchBox + EmuMovies community packs.
          </p>
          <p>
            <span class="text-(--color-oa-ink)">System metadata:</span>{" "}
            derived in part from MAME (BSD-3-Clause) —{" "}
            <code class="text-(--color-oa-ink-dim)/80">-listxml</code>{" "}
            machine data; per-system descriptions from arcade-history.com's{" "}
            <code class="text-(--color-oa-ink-dim)/80">history.xml</code>{" "}
            (the Gaming-History project).
          </p>
        </div>
      </SettingsCard>

      <SettingsCard title="Report a bug">
        <p class="text-[0.75rem] text-(--color-oa-ink-dim)">
          Found a crash or a bug? Open an issue on the project's
          GitHub. Open the debug log below if the bug surfaced at
          runtime — frontend logs land in the same stream as Rust
          logs, so the dump is a self-contained reproduction
          artifact.
        </p>
        <div class="mt-3 flex flex-wrap gap-2">
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              setHelpDialog("debug-log");
            }}
            class="rounded-md border border-(--color-system-accent)/40 bg-(--color-system-accent)/10 px-3 py-1.5 text-[0.7rem] font-semibold uppercase tracking-wider text-(--color-system-accent) transition hover:bg-(--color-system-accent)/20 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
          >
            Open debug log…
          </button>
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              setHelpDialog("shortcuts");
            }}
            class="rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-[0.7rem] font-semibold uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:border-(--color-oa-ink-dim)/50 hover:bg-white/[0.08] hover:text-(--color-oa-ink) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
          >
            Keyboard shortcuts…
          </button>
        </div>
      </SettingsCard>

      {/* Developer tools. Rendered unconditionally for now (the whole app is
          pre-release); the panel is clearly labelled "dev build". Re-gate behind
          import.meta.env.DEV before any public release — tracked in PARKING_LOT. */}
      <DevToolsPanel />
    </div>
  );
};

// --- Storage -----------------------------------------------------------

type StorageSystemStatus = {
  cpuPercent: number;
  ramUsedBytes: number;
  ramTotalBytes: number;
  dataDirFreeBytes: number | null;
  dataDirTotalBytes: number | null;
};

function formatStorageBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const kb = bytes / 1024;
  if (kb < 1024) return `${kb.toFixed(0)} KB`;
  const mb = kb / 1024;
  if (mb < 1024) return `${mb.toFixed(1)} MB`;
  const gb = mb / 1024;
  if (gb < 1024) return `${gb.toFixed(1)} GB`;
  return `${(gb / 1024).toFixed(2)} TB`;
}

export const StorageSettings: Component = () => {
  const [dataDir] = createResource(async () => {
    try {
      const mod = await import("@oa/platform/lib/dataDir");
      return await mod.getDataDir();
    } catch {
      return "";
    }
  });
  const [storageInfo] = createResource(async () => {
    try {
      return await getSystemStatus<StorageSystemStatus>();
    } catch {
      return null;
    }
  });

  const isPortable = () => {
    const d = dataDir();
    if (!d) return false;
    // Portable mode places the data dir under <exe_dir>/settings/.
    // AppData mode places it under AppData/Roaming. A path-suffix
    // heuristic is good enough for the indicator.
    return /[\\\/]settings([\\\/]|$)/i.test(d) && !d.toLowerCase().includes("appdata");
  };

  const freePercent = () => {
    const info = storageInfo();
    if (!info || !info.dataDirFreeBytes || !info.dataDirTotalBytes) return null;
    return Math.round((info.dataDirFreeBytes / info.dataDirTotalBytes) * 100);
  };

  return (
    <div class="flex flex-col gap-4">
      <SettingsCard title="Data directory">
        <div class="space-y-2">
          <div class="flex items-baseline justify-between gap-3 text-[0.75rem]">
            <span class="shrink-0 text-(--color-oa-ink-dim)">Mode</span>
            <span class="text-right text-(--color-oa-ink)">
              {isPortable() ? "Portable" : "AppData"}
            </span>
          </div>
          <div class="flex flex-col gap-1">
            <span class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Path
            </span>
            <code class="break-all rounded border border-white/10 bg-black/40 px-2 py-1.5 font-mono text-[0.65rem] text-(--color-oa-ink)">
              {dataDir() ?? "Loading…"}
            </code>
          </div>
          <p class="text-[0.65rem] text-(--color-oa-ink-dim)/70">
            Portable mode is opted-in by dropping a `portable.txt`
            marker next to oa-shell.exe. Switching modes requires
            restarting OA.
          </p>
        </div>
      </SettingsCard>

      <SettingsCard title="Free space">
        <Show
          when={storageInfo() && storageInfo()!.dataDirFreeBytes !== null}
          fallback={
            <p class="text-[0.7rem] text-(--color-oa-ink-dim)/70">
              Couldn't match the data-dir drive against any sysinfo
              disk entry. Rare; happens on exotic mount setups.
            </p>
          }
        >
          <div class="space-y-2">
            <div class="flex items-baseline justify-between">
              <span class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                On data drive
              </span>
              <span class="text-2xl font-semibold text-(--color-oa-ink)">
                {formatStorageBytes(storageInfo()!.dataDirFreeBytes ?? 0)}
              </span>
            </div>
            <p class="text-[0.65rem] text-(--color-oa-ink-dim)">
              of {formatStorageBytes(storageInfo()!.dataDirTotalBytes ?? 0)}{" "}
              total ({freePercent()}% free)
            </p>
            <div class="h-1.5 w-full overflow-hidden rounded-full bg-white/5">
              <div
                class="h-full rounded-full bg-emerald-500"
                style={{ width: `${freePercent() ?? 0}%` }}
              />
            </div>
          </div>
        </Show>
      </SettingsCard>

      <SettingsCard title="Subdirectories">
        <p class="text-[0.7rem] text-(--color-oa-ink-dim)">
          Save states / saves / logs / per-game overrides / scanned
          library / sync caches all live under the data directory.
        </p>
      </SettingsCard>

      <MameRefreshCard />
    </div>
  );
};

/// "Refresh MAME system info" card — Phase 5 of the System Info Panel
/// v1. Operator clicks once, OA shells out to their local MAME's
/// `-listxml` + reads `history.xml` from the canonical install layout,
/// overwrites the SQLite system_info_mame (L1) table without touching
/// L2 (curated YAML) or L3 (operator overrides).
///
/// MAME auto-detect probes <exe_dir>/Emulators/MAME/ first; if not
/// found, the operator can use the folder picker to point at their
/// own MAME install (binary OR parent folder). The re-import is
/// session-scoped per plan §10 — next OA release rebakes from the
/// bundled slim files, overwriting whatever the operator imported.
const MameRefreshCard: Component = () => {
  const [busy, setBusy] = createSignal(false);
  const [lastReport, setLastReport] = createSignal<MameRefreshReport | null>(null);
  const [lastError, setLastError] = createSignal<string | null>(null);

  async function refresh(customMamePath?: string) {
    setBusy(true);
    setLastError(null);
    try {
      const report = await refreshMameSystemInfo({ mamePath: customMamePath });
      setLastReport(report);
      const missing = report.missingSystems.length;
      const histNote = report.historyPresent ? "" : " (no history.xml found — descriptions not refreshed)";
      const missNote = missing > 0 ? ` ${missing} OA system${missing === 1 ? "" : "s"} not covered by this MAME release.` : "";
      await emitEvent(OA_EVENTS.toast, {
        level: "success",
        message: `Refreshed ${report.systemsRefreshed} systems from MAME ${report.mameVersion}.${missNote}${histNote}`,
      });
    } catch (e) {
      const msg = String(e);
      console.warn("[StorageSettings] refresh_mame_system_info failed:", e);
      setLastError(msg);
      // If MAME wasn't found, prompt for a folder rather than just
      // erroring. Operators don't always have it on PATH.
      const looksLikeNotFound = /not found|No such|not.*detected/i.test(msg);
      if (looksLikeNotFound && !customMamePath) {
        await emitEvent(OA_EVENTS.toast, {
          level: "warning",
          message:
            "MAME not found at the canonical path. Pick your MAME folder.",
        });
        const picked = await pickDirectory({
          directory: true,
          multiple: false,
          title: "Locate your MAME folder",
        }).catch(() => null);
        if (picked && !Array.isArray(picked)) {
          // Retry with the operator's pick.
          await refresh(picked);
        }
      } else {
        await emitEvent(OA_EVENTS.toast, {
          level: "error",
          message: `Refresh failed: ${msg}`,
        });
      }
    } finally {
      setBusy(false);
    }
  }

  return (
    <SettingsCard title="MAME system info">
      <div class="flex flex-col gap-3">
        <p class="text-[0.7rem] text-(--color-oa-ink-dim)">
          Re-imports per-system metadata (CPU / sound / resolution /
          refresh rate / peripherals) from your local MAME install,
          overwriting the bundled L1 baseline. Curated values (
          <code class="text-(--color-oa-ink-dim)/80">docs/cores/&lt;id&gt;/system-info.yaml</code>
          ) and your local edits stay untouched.
        </p>
        <p class="text-[0.65rem] text-(--color-oa-ink-dim)/70">
          Looks for MAME at{" "}
          <code class="text-(--color-oa-ink-dim)/80">
            &lt;exe_dir&gt;/Emulators/MAME/mame.exe
          </code>{" "}
          first, then PATH + typical system install paths. A folder
          picker opens if nothing's found.
        </p>
        <p class="text-[0.6rem] text-amber-300/80">
          Session-scoped: the next OA update rebakes from the bundled
          slim files and overwrites this refresh.
        </p>

        <div class="flex items-center gap-3 pt-1">
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              refresh();
            }}
            disabled={busy()}
            class="rounded-md border border-(--color-system-accent)/40 bg-(--color-system-accent)/20 px-4 py-2 text-[0.7rem] uppercase tracking-widest text-(--color-system-accent-soft) transition hover:bg-(--color-system-accent)/30 disabled:opacity-40"
          >
            {busy() ? "Refreshing…" : "Refresh MAME system info"}
          </button>
          <Show when={lastReport()}>
            <span class="text-[0.65rem] text-(--color-oa-ink-dim)">
              Last refresh: MAME {lastReport()!.mameVersion} ·{" "}
              {lastReport()!.systemsRefreshed} systems
            </span>
          </Show>
          <Show when={lastError() && !lastReport()}>
            <span class="text-[0.65rem] text-red-300">{lastError()}</span>
          </Show>
        </div>
      </div>
    </SettingsCard>
  );
};

// --- Themes ------------------------------------------------------------

/// Short blurb per built-in theme for the Appearance picker. Keyed by
/// manifest id; unknown ids fall back to a generic line. (S2 — the manifest
/// doesn't carry a description field yet; the versioned manifest in S4 may
/// add one, at which point this map retires.)
const THEME_BLURBS: Record<string, string> = {
  retroverse:
    "The default shell — a top-toolbar IA (HOME / LIBRARY / COLLECTIONS / PLAY NOW / DISCOVER) with per-system theming.",
  coverflow:
    "A full-bleed horizontal coverflow. Rough pilot (ARC 1): layout + feel only — the cinematic layer (attract / CRT / launch ceremony) lands in a later arc.",
};

/// Settings → Themes — the active-theme picker (Theming Substrate ARC 1
/// Phase 3 S2, the walking-skeleton swap gate). Lists the build-time themes
/// the platform registry knows about; switching persists the choice into
/// LibraryPrefs and RESTARTS the app (DECISIONS D5 — no hot-swap in ARC 1).
export const ThemesSettings: Component = () => {
  // Effective active id: the stored preference, or the first registered
  // theme (the default) when nothing is stored yet.
  const effectiveActiveId = (): string | undefined =>
    activeThemeId() ?? availableThemes()[0]?.id;

  const onSwitch = async (id: string, name: string): Promise<void> => {
    const ok = await confirm(
      `Switch the OA shell to "${name}"? OA will restart to apply the change.`,
      { confirmLabel: "Switch + restart", cancelLabel: "Cancel" },
    );
    if (!ok) return;
    // setActiveTheme persists then relaunches; it does not resolve past the
    // restart, so there's nothing to await after.
    await setActiveTheme(id);
  };

  return (
    <div class="flex flex-col gap-4">
      <SettingsCard title="Shell theme">
        <div class="space-y-3">
          <For each={availableThemes()}>
            {(theme) => {
              const isActive = (): boolean => effectiveActiveId() === theme.id;
              return (
                <div
                  class="flex items-center justify-between rounded-lg border px-4 py-3"
                  classList={{
                    "border-(--color-system-accent)/40 bg-(--color-system-accent)/[0.08]":
                      isActive(),
                    "border-white/10 bg-white/[0.02]": !isActive(),
                  }}
                >
                  <div class="min-w-0 flex-1">
                    <p class="text-sm font-semibold text-(--color-oa-ink)">
                      {theme.name}
                    </p>
                    <p class="text-[0.65rem] text-(--color-oa-ink-dim)">
                      {THEME_BLURBS[theme.id] ?? "A whole-shell OA theme."}
                    </p>
                  </div>
                  <Show
                    when={!isActive()}
                    fallback={
                      <span class="shrink-0 rounded border border-emerald-400/40 bg-emerald-500/10 px-2 py-0.5 text-[0.55rem] uppercase tracking-widest text-emerald-300">
                        Active
                      </span>
                    }
                  >
                    <button
                      type="button"
                      onClick={() => void onSwitch(theme.id, theme.name)}
                      class="shrink-0 rounded-md border border-(--color-system-accent)/50 bg-(--color-system-accent)/15 px-3 py-1.5 text-[0.65rem] font-semibold uppercase tracking-widest text-(--color-oa-ink) transition hover:bg-(--color-system-accent)/25 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--color-system-accent)"
                    >
                      Switch
                    </button>
                  </Show>
                </div>
              );
            }}
          </For>
        </div>
      </SettingsCard>

      {/* Settings IA Slice 3 — the active theme's declarative appearance
          options, rendered generically from its manifest `settings_schema`. */}
      <SettingsCard
        title="Appearance"
        description="Options the active theme exposes. Each theme decides what's configurable here — and remembers its own choices."
      >
        <AppearancePanel />
      </SettingsCard>

      <SettingsCard title="About theme switching">
        <p class="text-[0.7rem] text-(--color-oa-ink-dim)">
          Switching the shell theme restarts OA to apply the change cleanly
          (hot-swap is a later arc). Themes are bundled with this build;
          loadable theme packages land in a future arc. The engine surface
          you're in right now (Settings / Library Manager / Import / System
          Health) looks the same under every theme — only the browsing shell
          changes.
        </p>
      </SettingsCard>
    </div>
  );
};

// ---------------------------------------------------------------------------
// Background Jobs — Phase 5 of the background-jobs arc
// ---------------------------------------------------------------------------

type JobPrefsShape = {
  promptBeforeResumeOnLaunch: Record<string, boolean>;
  soundOnCompletion: boolean;
  alwaysShowBar: boolean;
};

/// Plan §"Kind taxonomy" — display order + label for the per-kind
/// auto-resume toggles. Mirrors the JobKind discriminator strings the
/// Rust side stores; keep in lockstep when JobKind variants are added.
const JOB_KINDS: readonly { id: string; label: string; description: string }[] = [
  {
    id: "core_download",
    label: "Core downloads",
    description:
      "libretro core .dll downloads from buildbot. Phase 3b shipped byte-level Range resume — auto-resume picks up where the crash left off without re-downloading.",
  },
  {
    id: "bulk_core_install",
    label: "Bulk core installs",
    description:
      "Guided Setup's parallel install of N missing cores. Children are individual core downloads; auto-resume re-creates the parent and resumes each child.",
  },
  {
    id: "hash_resolve",
    label: "Identify ROMs",
    description:
      "Per-system ROM hash + canonical-title lookup. Re-trigger via Settings → Library; existing internal idempotency skips already-stamped rows.",
  },
  {
    id: "dat_sync",
    label: "ROM database sync",
    description:
      "libretro-database .dat fetch + parse for a system's hash table. Auto-triggered as a prereq when Identify ROMs runs against an empty table.",
  },
  {
    id: "artwork_sync",
    label: "Artwork sync",
    description:
      "Per-system artwork download from libretro-thumbnails. Re-trigger via Settings → Library → Sync media; internal idempotency skips already-downloaded files.",
  },
  {
    id: "folder_scan",
    label: "Folder scans",
    description:
      "Import wizard folder walks. Re-trigger via Import Wizard or Settings → Library → Re-scan; the walk re-runs from the folder root.",
  },
  {
    id: "mame_listxml_import",
    label: "MAME catalog refresh",
    description:
      "Refresh of the local MAME machine catalog from a MAME install's listxml output. Re-trigger via Settings → MAME → Refresh MAME system info.",
  },
];

export const BackgroundJobsSettings: Component = () => {
  const [prefs, setPrefs] = createSignal<JobPrefsShape>({
    promptBeforeResumeOnLaunch: {},
    soundOnCompletion: true,
    alwaysShowBar: false,
  });
  const [historyCount, setHistoryCount] = createSignal<number>(0);
  const [clearStatus, setClearStatus] = createSignal<string>("");

  const refreshPrefs = async () => {
    try {
      const p = await jobsApi.getJobPrefs<JobPrefsShape>();
      setPrefs(p);
    } catch (e) {
      console.warn("[bg-jobs-settings] get_job_prefs failed:", e);
    }
  };
  const refreshHistory = async () => {
    try {
      const rows = await jobsApi.listRecentJobs<unknown>(100);
      setHistoryCount(rows.length);
    } catch (e) {
      console.warn("[bg-jobs-settings] list_recent_jobs failed:", e);
    }
  };

  // Hydrate on mount.
  void refreshPrefs();
  void refreshHistory();

  const togglePromptForKind = async (kindId: string, prompt: boolean) => {
    try {
      const updated = await jobsApi.setJobResumePrompt<JobPrefsShape>(kindId, prompt);
      setPrefs(updated);
    } catch (e) {
      console.warn(`[bg-jobs-settings] set_job_resume_prompt(${kindId}) failed:`, e);
    }
  };

  return (
    <div class="flex flex-col gap-4">
      <SettingsCard
        title="Auto-resume on launch"
        description={
          'When OA crashes mid-operation, the next launch detects the orphan via the `oa.lock` marker and either auto-resumes the interrupted work OR shows a prompt. Toggle ON for a kind = prompt before resuming. Default OFF = auto-resume silently.'
        }
      >
        <div class="space-y-2">
          {JOB_KINDS.map((kind) => (
            <label class="flex items-start gap-3 rounded-lg border border-white/10 bg-white/[0.02] px-3 py-2">
              <input
                type="checkbox"
                class="mt-1"
                checked={prefs().promptBeforeResumeOnLaunch[kind.id] === true}
                onChange={(e) => {
                  void togglePromptForKind(kind.id, e.currentTarget.checked);
                }}
              />
              <div class="min-w-0 flex-1">
                <p class="text-[0.8rem] font-semibold text-(--color-oa-ink)">
                  Prompt before resuming {kind.label.toLowerCase()}
                </p>
                <p class="mt-0.5 text-[0.65rem] leading-relaxed text-(--color-oa-ink-dim)">
                  {kind.description}
                </p>
              </div>
            </label>
          ))}
        </div>
      </SettingsCard>

      <SettingsCard
        title="Bar behavior"
        description="How the persistent BackgroundJobsBar (bottom of the viewport) behaves while operations are running."
      >
        <div class="space-y-2">
          <label class="flex items-center gap-3 rounded-lg border border-white/10 bg-white/[0.02] px-3 py-2">
            <input
              type="checkbox"
              class="mt-0.5"
              checked={prefs().alwaysShowBar}
              onChange={async (e) => {
                try {
                  const updated = await jobsApi.setJobAlwaysShowBar<JobPrefsShape>(
                    e.currentTarget.checked,
                  );
                  setPrefs(updated);
                  // Sync the module-level live signal so the bar
                  // reacts immediately.
                  await refreshJobPrefs();
                } catch (err) {
                  console.warn("[bg-jobs-settings] set_job_always_show_bar:", err);
                }
              }}
            />
            <div class="min-w-0 flex-1">
              <p class="text-[0.8rem] font-semibold text-(--color-oa-ink)">
                Always show the bar
              </p>
              <p class="mt-0.5 text-[0.65rem] leading-relaxed text-(--color-oa-ink-dim)">
                Default OFF — the bar's handle only appears while at
                least one job is active. ON keeps the handle visible
                at all times (labels "No active jobs" when idle).
              </p>
            </div>
          </label>
          <label class="flex items-center gap-3 rounded-lg border border-white/10 bg-white/[0.02] px-3 py-2">
            <input
              type="checkbox"
              class="mt-0.5"
              checked={prefs().soundOnCompletion}
              onChange={async (e) => {
                try {
                  const updated = await jobsApi.setJobSoundOnCompletion<JobPrefsShape>(
                    e.currentTarget.checked,
                  );
                  setPrefs(updated);
                  await refreshJobPrefs();
                } catch (err) {
                  console.warn("[bg-jobs-settings] set_job_sound_on_completion:", err);
                }
              }}
            />
            <div class="min-w-0 flex-1">
              <p class="text-[0.8rem] font-semibold text-(--color-oa-ink)">
                Sound on completion
              </p>
              <p class="mt-0.5 text-[0.65rem] leading-relaxed text-(--color-oa-ink-dim)">
                A subtle chime when a job completes (plan §"Notification
                on completion"). Silent fallback when the chime asset
                isn't bundled — see{" "}
                <span class="font-mono text-(--color-system-accent)/80">
                  docs/features/background-jobs/ASSETS.md
                </span>{" "}
                for where to drop the file.
              </p>
            </div>
          </label>
        </div>
      </SettingsCard>

      <SettingsCard
        title="Failure handling"
        description="How OA recovers from transient network failures during a download."
      >
        <div class="rounded-lg border border-white/10 bg-white/[0.02] px-3 py-3">
          <p class="text-[0.8rem] font-semibold text-(--color-oa-ink)">
            Auto-retry transient network errors
          </p>
          <p class="mt-1 text-[0.65rem] leading-relaxed text-(--color-oa-ink-dim)">
            5xx server errors + dropped connections retry up to 3
            attempts with 1 s / 5 s / 30 s exponential backoff before
            mark_failed. Retries are invisible to the operator
            (logged at warn-level in oa-current.log). Permanent 4xx
            errors fail immediately. Operator-triggered cancel
            during a backoff sleep takes effect within 100 ms.
          </p>
          <p class="mt-2 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)/70">
            Always on · 3 attempts · 1 / 5 / 30 s
          </p>
        </div>
      </SettingsCard>

      <SettingsCard
        title="History"
        description="The recent-activity panel keeps the last 100 finished jobs (completed / failed / cancelled) in a rolling buffer."
      >
        <div class="flex items-center justify-between rounded-lg border border-white/10 bg-white/[0.02] px-3 py-3">
          <p class="text-[0.75rem] text-(--color-oa-ink)">
            <span class="tabular-nums font-semibold">
              {historyCount()}
            </span>{" "}
            <span class="text-(--color-oa-ink-dim)">of 100 history rows used</span>
          </p>
          <button
            type="button"
            class="rounded-md border border-rose-500/30 bg-rose-500/10 px-3 py-1.5 text-[0.7rem] font-semibold uppercase tracking-wider text-rose-200 transition hover:border-rose-400/60 hover:bg-rose-500/20"
            onClick={async (e) => {
              e.currentTarget.blur();
              const ok = await confirm(
                "Clear all background-job history rows? Active rows (pending/running/paused) are preserved.",
                { title: "Clear job history", confirmLabel: "Clear history", danger: true },
              );
              if (!ok) return;
              try {
                await jobsApi.clearJobHistory();
                await refreshHistory();
                setClearStatus("History cleared.");
                window.setTimeout(() => setClearStatus(""), 3000);
              } catch (err) {
                setClearStatus(`Clear failed: ${String(err)}`);
              }
            }}
          >
            Clear recent activity
          </button>
        </div>
        <Show when={clearStatus() !== ""}>
          <p class="mt-2 text-[0.7rem] text-(--color-oa-ink-dim)">
            {clearStatus()}
          </p>
        </Show>
      </SettingsCard>
    </div>
  );
};
