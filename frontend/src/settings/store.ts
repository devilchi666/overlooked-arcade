import { createEffect, createSignal } from "solid-js";
import { invoke } from "@tauri-apps/api/core";

export type ScalingMode =
  | "pixel-perfect"
  | "aspect-correct"
  | "stretched"
  | "original"
  | "integer-2x"
  | "integer-3x"
  | "integer-4x";

export type WindowMode = "windowed" | "borderless" | "fullscreen";
export type ShellMode = "two-window" | "single-window";
/// Shader preset name. Slice A added the first four built-ins; slice C
/// replaced the hardcoded enum with a TOML registry (built-in .toml files
/// + user files in `<exe_dir>/shaders/presets/`), so the type is `string`
/// — the live list comes from `list_shader_presets` (see
/// `shader_presets.ts`). Unknown names fall back to `"plain"` server-side.
export type ShaderPreset = string;

export type MonitorInfo = {
  index: number;
  name?: string;
  width: number;
  height: number;
  positionX: number;
  positionY: number;
  scaleFactor: number;
};

export type AudioDeviceInfo = {
  name: string;
  isDefault: boolean;
  isActive: boolean;
};

export type CoreEntry = {
  fileName: string;
  libraryName: string;
  libraryVersion: string;
  validExtensions: string;
  /// Bytes on disk; 0 when stat failed.
  sizeBytes: number;
  /// Last-modified epoch ms; 0 when stat failed.
  modifiedUnixMs: number;
  /// True for cores like Mednafen Saturn that need a real file path
  /// (multi-file CD images).
  needFullpath: boolean;
  /// True for cores that handle their own archive extraction internally.
  blockExtract: boolean;
  /// Set when probe failed — the library_* fields will be empty. Row still
  /// renders so the user can see + remove the broken file.
  error?: string;
};

const STORAGE_KEY = "oa.settings.v1";

const DEFAULT_SCALING: ScalingMode = "aspect-correct";
const DEFAULT_WINDOW: WindowMode = "windowed";

const SCALING_OPTIONS: readonly ScalingMode[] = [
  "pixel-perfect",
  "aspect-correct",
  "stretched",
  "original",
  "integer-2x",
  "integer-3x",
  "integer-4x",
];

const WINDOW_OPTIONS: readonly WindowMode[] = ["windowed", "borderless", "fullscreen"];

const SHELL_OPTIONS: readonly ShellMode[] = ["two-window", "single-window"];

// `"system-default"` is a sentinel the App's launch path expands via
// `resolveShaderPreset(value, systemId)` (themes/registry.ts) to the
// active system's `defaultShaderPreset`. Fresh installs start here; users
// can pick any concrete preset in the per-system / per-game UI which
// overrides this value through the normal inheritance chain.
const DEFAULT_SHADER_PRESET: ShaderPreset = "system-default";

// Phase 3 slice C polish — default Phosphor bloom_amount at the OA-wide
// root of the inheritance chain. Matches the bloom_amount the renderer
// applies before any preset / system / game value pushes through, and
// what the four built-in TOML presets ship as.
const DEFAULT_BLOOM_AMOUNT = 0.6;

// Run-ahead frames (RetroArch parity slice 7). 0 = off, 1-5 = peek that
// many frames ahead each render frame to reduce perceived input latency.
// Each extra frame costs 1 save_state + 1 run_frame + 1 load_state per
// real frame; cheap on PCE/NES, eats budget on heavier cores.
const DEFAULT_RUN_AHEAD_FRAMES = 0;

// Auto-remove ROMs from the library when the file is deleted from disk.
// Default off — the watcher's "removed" event keeps the entry (matches
// the historical "soft policy" where the user might be moving / renaming
// the file). Users who keep a tidy library can flip this on so the
// library auto-cleans when ROMs are deleted.
const DEFAULT_AUTO_REMOVE_ON_DELETE = false;

// Phase 4 slice A — rewind defaults. Off by default; 6-frame capture
// interval = ~100 ms at 60 fps (RetroArch's default); 64 MB buffer holds
// ~10 s of any current core (PCE ~50 KB/snap, SNES ~300 KB/snap).
const DEFAULT_REWIND_ENABLED = false;
const DEFAULT_REWIND_INTERVAL = 6;
const DEFAULT_REWIND_BUFFER_MB = 64;

// Controller-nav defaults (Phase 0 of the guided-setup pipeline). On by
// default — couch gamers are the primary audience. Animation budget
// matches Steam Big Picture's snappy-but-visible 120ms; 0 disables the
// transition for operators on very low-end hardware.
const DEFAULT_CONTROLLER_NAV_ENABLED = true;
const DEFAULT_CONTROLLER_NAV_SOURCE: ControllerNavSource = "both";
const DEFAULT_CONTROLLER_NAV_SWAP_AB = false;
const DEFAULT_CONTROLLER_NAV_ANIMATION_MS = 120;

// Per-System Custom UI master toggle (Stage 1 of the per-system-ui
// pipelined arc). Default ON — the plan positions per-system experiences
// as the DEFAULT OA experience, not opt-in. Operators who want a
// uniform plain library flip this off in Settings → Display. Slice 1
// ships the toggle alone; no consumers yet — those land in slices 2-9
// (per-system SFX, backgrounds, boot animations, tile flourishes,
// pilot full builds).
const DEFAULT_PER_SYSTEM_UI_ENABLED = true;

// Boot animations sub-toggle (Per-System UI Stage 1 Slice 4). Default
// ON; only visible in Settings → Display when the master toggle is
// also ON. Disabling drops to a 200 ms fade for system-entry events
// (matches the reduced-motion path). Always-on at the dispatch layer
// when prefers-reduced-motion is true.
const DEFAULT_BOOT_ANIMATIONS_ENABLED = true;

export type ControllerNavSource = "dpad" | "stick-left" | "both";

const CONTROLLER_NAV_SOURCE_OPTIONS: readonly ControllerNavSource[] = [
  "dpad",
  "stick-left",
  "both",
];

function isControllerNavSource(v: unknown): v is ControllerNavSource {
  return typeof v === "string" && (CONTROLLER_NAV_SOURCE_OPTIONS as readonly string[]).includes(v);
}

type Persisted = {
  scalingMode: ScalingMode;
  windowMode: WindowMode;
  monitorIndex: number | null;
  shaderPreset: ShaderPreset;
  bloomAmount: number;
  runAheadFrames: number;
  autoRemoveOnDelete: boolean;
  rewindEnabled: boolean;
  rewindCaptureIntervalFrames: number;
  rewindBufferMegabytes: number;
  controllerNavEnabled: boolean;
  controllerNavSource: ControllerNavSource;
  controllerNavSwapAB: boolean;
  controllerNavAnimationMs: number;
  perSystemUiEnabled: boolean;
  bootAnimationsEnabled: boolean;
};

/// Library folder row as returned by the Rust `list_folders` Tauri command.
/// Mirrors `library_db::Folder` (camelCase). Only the fields the settings
/// store actually uses are typed here — rules etc. live in ImportWizard's
/// local copy of the type.
export type LibraryFolderRow = {
  id: string;
  path: string;
  scanSubfolders: boolean;
  subfoldersAreSystems: boolean;
  watchEnabled: boolean;
  lastScannedAt?: number | null;
};

function isScalingMode(v: unknown): v is ScalingMode {
  return typeof v === "string" && (SCALING_OPTIONS as readonly string[]).includes(v);
}

function isWindowMode(v: unknown): v is WindowMode {
  return typeof v === "string" && (WINDOW_OPTIONS as readonly string[]).includes(v);
}

function isShaderPreset(v: unknown): v is ShaderPreset {
  // Slice C — the preset registry is dynamic (TOML files + user overrides),
  // so we accept any non-empty string. The renderer falls back to `plain`
  // server-side for names not in the registry, which is the right behavior
  // when a user removes a .preset.toml file that some persisted setting
  // still references.
  return typeof v === "string" && v.length > 0;
}

function isStringArray(v: unknown): v is string[] {
  return Array.isArray(v) && v.every((s) => typeof s === "string");
}

/// Pulls the legacy `libraryFolders` array out of `oa.settings.v1` (if it
/// was written before the 2026-05-21 SQLite-folders unification). The
/// settings store calls this once on init, hands the paths to the Rust
/// `migrate_folders_from_local_storage` Tauri command, then rewrites the
/// persisted settings WITHOUT the field. Returns an empty list when the
/// payload is missing, malformed, or already migrated.
function legacyLibraryFoldersFromLocalStorage(): string[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    return isStringArray(parsed.libraryFolders) ? parsed.libraryFolders : [];
  } catch {
    return [];
  }
}

function load(): Persisted {
  const fallback: Persisted = {
    scalingMode: DEFAULT_SCALING,
    windowMode: DEFAULT_WINDOW,
    monitorIndex: null,
    shaderPreset: DEFAULT_SHADER_PRESET,
    bloomAmount: DEFAULT_BLOOM_AMOUNT,
    runAheadFrames: DEFAULT_RUN_AHEAD_FRAMES,
    autoRemoveOnDelete: DEFAULT_AUTO_REMOVE_ON_DELETE,
    rewindEnabled: DEFAULT_REWIND_ENABLED,
    rewindCaptureIntervalFrames: DEFAULT_REWIND_INTERVAL,
    rewindBufferMegabytes: DEFAULT_REWIND_BUFFER_MB,
    controllerNavEnabled: DEFAULT_CONTROLLER_NAV_ENABLED,
    controllerNavSource: DEFAULT_CONTROLLER_NAV_SOURCE,
    controllerNavSwapAB: DEFAULT_CONTROLLER_NAV_SWAP_AB,
    controllerNavAnimationMs: DEFAULT_CONTROLLER_NAV_ANIMATION_MS,
    perSystemUiEnabled: DEFAULT_PER_SYSTEM_UI_ENABLED,
    bootAnimationsEnabled: DEFAULT_BOOT_ANIMATIONS_ENABLED,
  };
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return fallback;
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    return {
      scalingMode: isScalingMode(parsed.scalingMode) ? parsed.scalingMode : DEFAULT_SCALING,
      windowMode: isWindowMode(parsed.windowMode) ? parsed.windowMode : DEFAULT_WINDOW,
      monitorIndex:
        typeof parsed.monitorIndex === "number" && Number.isInteger(parsed.monitorIndex)
          ? parsed.monitorIndex
          : null,
      shaderPreset: isShaderPreset(parsed.shaderPreset) ? parsed.shaderPreset : DEFAULT_SHADER_PRESET,
      bloomAmount:
        typeof parsed.bloomAmount === "number" && Number.isFinite(parsed.bloomAmount)
          ? Math.min(1, Math.max(0, parsed.bloomAmount))
          : DEFAULT_BLOOM_AMOUNT,
      autoRemoveOnDelete:
        typeof parsed.autoRemoveOnDelete === "boolean"
          ? parsed.autoRemoveOnDelete
          : DEFAULT_AUTO_REMOVE_ON_DELETE,
      runAheadFrames:
        typeof parsed.runAheadFrames === "number"
          && Number.isInteger(parsed.runAheadFrames)
          && parsed.runAheadFrames >= 0
          ? Math.min(5, parsed.runAheadFrames)
          : DEFAULT_RUN_AHEAD_FRAMES,
      rewindEnabled:
        typeof parsed.rewindEnabled === "boolean" ? parsed.rewindEnabled : DEFAULT_REWIND_ENABLED,
      rewindCaptureIntervalFrames:
        typeof parsed.rewindCaptureIntervalFrames === "number"
          && Number.isInteger(parsed.rewindCaptureIntervalFrames)
          && parsed.rewindCaptureIntervalFrames >= 1
          ? parsed.rewindCaptureIntervalFrames
          : DEFAULT_REWIND_INTERVAL,
      rewindBufferMegabytes:
        typeof parsed.rewindBufferMegabytes === "number"
          && Number.isInteger(parsed.rewindBufferMegabytes)
          && parsed.rewindBufferMegabytes >= 1
          ? parsed.rewindBufferMegabytes
          : DEFAULT_REWIND_BUFFER_MB,
      controllerNavEnabled:
        typeof parsed.controllerNavEnabled === "boolean"
          ? parsed.controllerNavEnabled
          : DEFAULT_CONTROLLER_NAV_ENABLED,
      controllerNavSource: isControllerNavSource(parsed.controllerNavSource)
        ? parsed.controllerNavSource
        : DEFAULT_CONTROLLER_NAV_SOURCE,
      controllerNavSwapAB:
        typeof parsed.controllerNavSwapAB === "boolean"
          ? parsed.controllerNavSwapAB
          : DEFAULT_CONTROLLER_NAV_SWAP_AB,
      controllerNavAnimationMs:
        typeof parsed.controllerNavAnimationMs === "number"
          && Number.isFinite(parsed.controllerNavAnimationMs)
          && parsed.controllerNavAnimationMs >= 0
          && parsed.controllerNavAnimationMs <= 1000
          ? parsed.controllerNavAnimationMs
          : DEFAULT_CONTROLLER_NAV_ANIMATION_MS,
      perSystemUiEnabled:
        typeof parsed.perSystemUiEnabled === "boolean"
          ? parsed.perSystemUiEnabled
          : DEFAULT_PER_SYSTEM_UI_ENABLED,
      bootAnimationsEnabled:
        typeof parsed.bootAnimationsEnabled === "boolean"
          ? parsed.bootAnimationsEnabled
          : DEFAULT_BOOT_ANIMATIONS_ENABLED,
    };
  } catch {
    return fallback;
  }
}

function save(state: Persisted): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch {
    // Quota or storage disabled — accept the in-memory-only fallback.
  }
}

export function createSettingsStore() {
  const initial = load();
  const [scalingMode, setScalingMode] = createSignal<ScalingMode>(initial.scalingMode);
  const [windowMode, setWindowMode] = createSignal<WindowMode>(initial.windowMode);
  const [monitorIndex, setMonitorIndex] = createSignal<number | null>(initial.monitorIndex);
  // Library folders moved out of localStorage into SQLite as of 2026-05-21
  // — see DECISIONS.md "Library folders: SQLite is the single source of
  // truth". The signal below is hydrated from `list_folders` on init +
  // refreshed after every mutation. Empty array until the first fetch
  // resolves; the watcher effect handles the transient empty state fine
  // (it re-fires when the real list lands).
  const [libraryFolderRows, setLibraryFolderRows] = createSignal<LibraryFolderRow[]>([]);
  const libraryFolders = (): string[] => libraryFolderRows().map((r) => r.path);

  async function refreshLibraryFolders(): Promise<void> {
    try {
      const rows = await invoke<LibraryFolderRow[]>("list_folders", { includeRules: false });
      setLibraryFolderRows(rows);
    } catch (e) {
      console.warn("[oa-settings] list_folders failed:", e);
    }
  }

  async function addLibraryFolderPath(path: string): Promise<void> {
    try {
      await invoke<LibraryFolderRow>("add_folder", {
        path,
        scanSubfolders: true,
        subfoldersAreSystems: false,
        watchEnabled: true,
      });
    } catch (e) {
      // Most likely the path is already tracked — log and refresh anyway
      // so the caller sees the existing row appear.
      console.warn("[oa-settings] add_folder failed (already tracked?):", e);
    }
    await refreshLibraryFolders();
  }

  async function removeLibraryFolderById(id: string): Promise<void> {
    try {
      await invoke("remove_folder", { id });
    } catch (e) {
      console.warn("[oa-settings] remove_folder failed:", e);
    }
    await refreshLibraryFolders();
  }

  async function reorderLibraryFolderIds(orderedIds: string[]): Promise<void> {
    try {
      await invoke("reorder_folders", { orderedIds });
    } catch (e) {
      console.warn("[oa-settings] reorder_folders failed:", e);
    }
    await refreshLibraryFolders();
  }

  // One-shot migration: if the legacy localStorage payload had a
  // `libraryFolders` array, hand it to the Rust migration command then
  // re-save the settings WITHOUT that field so we never run again. The
  // command is idempotent at the SQLite level (paths already present are
  // skipped), so even if the strip-and-save races a crash we're safe.
  const legacy = legacyLibraryFoldersFromLocalStorage();
  void (async () => {
    if (legacy.length > 0) {
      try {
        const inserted = await invoke<number>("migrate_folders_from_local_storage", {
          paths: legacy,
        });
        if (inserted > 0) {
          console.info(`[oa-settings] migrated ${inserted} library folder(s) to SQLite`);
        }
      } catch (e) {
        console.warn("[oa-settings] folder migration failed:", e);
      }
    }
    await refreshLibraryFolders();
  })();

  const [shaderPreset, setShaderPreset] = createSignal<ShaderPreset>(initial.shaderPreset);
  const [bloomAmount, setBloomAmount] = createSignal<number>(initial.bloomAmount);
  const [runAheadFrames, setRunAheadFrames] = createSignal<number>(initial.runAheadFrames);
  const [autoRemoveOnDelete, setAutoRemoveOnDelete] = createSignal<boolean>(initial.autoRemoveOnDelete);
  const [rewindEnabled, setRewindEnabled] = createSignal<boolean>(initial.rewindEnabled);
  const [rewindCaptureIntervalFrames, setRewindCaptureIntervalFrames] =
    createSignal<number>(initial.rewindCaptureIntervalFrames);
  const [rewindBufferMegabytes, setRewindBufferMegabytes] =
    createSignal<number>(initial.rewindBufferMegabytes);
  const [controllerNavEnabled, setControllerNavEnabled] =
    createSignal<boolean>(initial.controllerNavEnabled);
  const [controllerNavSource, setControllerNavSource] =
    createSignal<ControllerNavSource>(initial.controllerNavSource);
  const [controllerNavSwapAB, setControllerNavSwapAB] =
    createSignal<boolean>(initial.controllerNavSwapAB);
  const [controllerNavAnimationMs, setControllerNavAnimationMs] =
    createSignal<number>(initial.controllerNavAnimationMs);
  const [perSystemUiEnabled, setPerSystemUiEnabled] =
    createSignal<boolean>(initial.perSystemUiEnabled);
  const [bootAnimationsEnabled, setBootAnimationsEnabled] =
    createSignal<boolean>(initial.bootAnimationsEnabled);
  // Shell mode preference is file-backed on the Rust side (appDataDir/shell.json),
  // not localStorage — Rust reads it at startup before the WebView exists.
  // We hydrate from the Tauri command on init; setter writes through immediately.
  const [shellModePref, setShellModePrefLocal] = createSignal<ShellMode>("two-window");
  // The *active* shell mode for this session (env-var override may differ
  // from the saved pref). Hydrated alongside the pref so the UI can warn
  // when they diverge.
  const [activeShellMode, setActiveShellMode] = createSignal<ShellMode>("two-window");
  // Audio device preference is also file-backed (appDataDir/audio.json) since
  // the cpal stream gets built before the WebView is alive. `null` = system
  // default. Setter writes through to Rust which both persists AND hot-swaps
  // the running stream.
  const [audioDevice, setAudioDeviceLocal] = createSignal<string | null>(null);

  createEffect(() => {
    save({
      scalingMode: scalingMode(),
      windowMode: windowMode(),
      monitorIndex: monitorIndex(),
      shaderPreset: shaderPreset(),
      bloomAmount: bloomAmount(),
      runAheadFrames: runAheadFrames(),
      autoRemoveOnDelete: autoRemoveOnDelete(),
      rewindEnabled: rewindEnabled(),
      rewindCaptureIntervalFrames: rewindCaptureIntervalFrames(),
      rewindBufferMegabytes: rewindBufferMegabytes(),
      controllerNavEnabled: controllerNavEnabled(),
      controllerNavSource: controllerNavSource(),
      controllerNavSwapAB: controllerNavSwapAB(),
      controllerNavAnimationMs: controllerNavAnimationMs(),
      perSystemUiEnabled: perSystemUiEnabled(),
      bootAnimationsEnabled: bootAnimationsEnabled(),
    });
  });

  // Push the focus-ring animation budget to the CSS root variable
  // immediately on change. The [data-oa-focus="true"] outline transition
  // consumes --oa-focus-anim-ms; setting it to 0ms disables the animation.
  createEffect(() => {
    const ms = controllerNavAnimationMs();
    document.documentElement.style.setProperty("--oa-focus-anim-ms", `${ms}ms`);
  });

  // Push the OA-wide bloom amount to the renderer immediately on change.
  // Per-system / per-game overrides re-push on launch via App.handleLaunch.
  createEffect(() => {
    const amount = bloomAmount();
    invoke("set_bloom_amount", { amount }).catch((e) =>
      console.warn("set_bloom_amount failed:", e),
    );
  });
  // Push run-ahead frame count to the emu thread on change. The emu
  // thread clamps to 0..=5; UI honors the same range.
  createEffect(() => {
    const frames = runAheadFrames();
    invoke("set_run_ahead", { frames }).catch((e) =>
      console.warn("set_run_ahead failed:", e),
    );
  });

  createEffect(() => {
    const mode = scalingMode();
    invoke("set_scaling_mode", { mode }).catch((e) =>
      console.warn("set_scaling_mode failed:", e),
    );
  });
  // Push the OA-wide shader preset to the renderer immediately on change.
  // Per-game / per-system overrides re-push on launch via App.handleLaunch.
  createEffect(() => {
    const preset = shaderPreset();
    invoke("set_shader_preset", { preset }).catch((e) =>
      console.warn("set_shader_preset failed:", e),
    );
  });
  createEffect(() => {
    const mode = windowMode();
    const idx = monitorIndex();
    invoke("set_window_mode", { mode, monitorIndex: idx }).catch((e) =>
      console.warn("set_window_mode failed:", e),
    );
  });
  // Push the OA-wide rewind config to the renderer immediately on change.
  // Per-game / per-system overrides re-push on launch via App.handleLaunch.
  createEffect(() => {
    const enabled = rewindEnabled();
    const captureIntervalFrames = rewindCaptureIntervalFrames();
    const maxMegabytes = rewindBufferMegabytes();
    invoke("set_rewind_config", { enabled, captureIntervalFrames, maxMegabytes }).catch((e) =>
      console.warn("set_rewind_config failed:", e),
    );
  });

  // Hydrate shellModePref from the file on init.
  invoke<string>("get_shell_mode_pref")
    .then((v) => {
      if (v === "single-window" || v === "two-window") setShellModePrefLocal(v);
    })
    .catch((e) => console.warn("get_shell_mode_pref failed:", e));
  // Hydrate activeShellMode from the Rust-side runtime state.
  invoke<string>("get_shell_mode")
    .then((v) => {
      if (v === "single-window" || v === "two-window") setActiveShellMode(v);
    })
    .catch((e) => console.warn("get_shell_mode failed:", e));
  // Hydrate the audio device pref so the picker reflects the persisted choice.
  invoke<string | null>("get_audio_device_pref")
    .then((v) => setAudioDeviceLocal(v ?? null))
    .catch((e) => console.warn("get_audio_device_pref failed:", e));

  function setShellModePref(mode: ShellMode): void {
    setShellModePrefLocal(mode);
    invoke("set_shell_mode_pref", { mode }).catch((e) =>
      console.warn("set_shell_mode_pref failed:", e),
    );
  }

  function setAudioDevice(name: string | null): void {
    setAudioDeviceLocal(name);
    invoke("set_audio_device_pref", { name }).catch((e) =>
      console.warn("set_audio_device_pref failed:", e),
    );
  }

  return {
    scalingMode, setScalingMode,
    windowMode, setWindowMode,
    monitorIndex, setMonitorIndex,
    libraryFolders,
    libraryFolderRows,
    addLibraryFolderPath,
    removeLibraryFolderById,
    reorderLibraryFolderIds,
    refreshLibraryFolders,
    shellModePref, setShellModePref,
    activeShellMode,
    audioDevice, setAudioDevice,
    shaderPreset, setShaderPreset,
    bloomAmount, setBloomAmount,
    runAheadFrames, setRunAheadFrames,
    autoRemoveOnDelete, setAutoRemoveOnDelete,
    rewindEnabled, setRewindEnabled,
    rewindCaptureIntervalFrames, setRewindCaptureIntervalFrames,
    rewindBufferMegabytes, setRewindBufferMegabytes,
    controllerNavEnabled, setControllerNavEnabled,
    controllerNavSource, setControllerNavSource,
    controllerNavSwapAB, setControllerNavSwapAB,
    controllerNavAnimationMs, setControllerNavAnimationMs,
    perSystemUiEnabled, setPerSystemUiEnabled,
    bootAnimationsEnabled, setBootAnimationsEnabled,
  };
}

export type SettingsStore = ReturnType<typeof createSettingsStore>;

export const SCALING_MODE_LABELS: Record<ScalingMode, string> = {
  "pixel-perfect": "Pixel perfect",
  "aspect-correct": "Aspect fit",
  "stretched": "Stretched",
  "original": "1:1 native",
  "integer-2x": "2× integer",
  "integer-3x": "3× integer",
  "integer-4x": "4× integer",
};

export const WINDOW_MODE_LABELS: Record<WindowMode, string> = {
  "windowed": "Windowed",
  "borderless": "Borderless fullscreen",
  "fullscreen": "Exclusive fullscreen",
};

export const SHELL_MODE_LABELS: Record<ShellMode, string> = {
  "two-window": "Two windows (library + game)",
  "single-window": "Single window (overlay UI)",
};

// Slice C — `SHADER_PRESET_LABELS` + `SHADER_PRESET_OPTIONS` were replaced
// by the dynamic registry in `./shader_presets.ts`. Components import
// `shaderPresets` (signal) and `loadShaderPresets` (one-shot fetch) from
// there.

export { SCALING_OPTIONS, WINDOW_OPTIONS, SHELL_OPTIONS };
