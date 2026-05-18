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

type Persisted = {
  scalingMode: ScalingMode;
  windowMode: WindowMode;
  monitorIndex: number | null;
  libraryFolders: string[];
  shaderPreset: ShaderPreset;
  bloomAmount: number;
  runAheadFrames: number;
  autoRemoveOnDelete: boolean;
  rewindEnabled: boolean;
  rewindCaptureIntervalFrames: number;
  rewindBufferMegabytes: number;
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

function load(): Persisted {
  const fallback: Persisted = {
    scalingMode: DEFAULT_SCALING,
    windowMode: DEFAULT_WINDOW,
    monitorIndex: null,
    libraryFolders: [],
    shaderPreset: DEFAULT_SHADER_PRESET,
    bloomAmount: DEFAULT_BLOOM_AMOUNT,
    runAheadFrames: DEFAULT_RUN_AHEAD_FRAMES,
    autoRemoveOnDelete: DEFAULT_AUTO_REMOVE_ON_DELETE,
    rewindEnabled: DEFAULT_REWIND_ENABLED,
    rewindCaptureIntervalFrames: DEFAULT_REWIND_INTERVAL,
    rewindBufferMegabytes: DEFAULT_REWIND_BUFFER_MB,
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
      libraryFolders: isStringArray(parsed.libraryFolders) ? parsed.libraryFolders : [],
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
  const [libraryFolders, setLibraryFolders] = createSignal<string[]>(initial.libraryFolders);
  const [shaderPreset, setShaderPreset] = createSignal<ShaderPreset>(initial.shaderPreset);
  const [bloomAmount, setBloomAmount] = createSignal<number>(initial.bloomAmount);
  const [runAheadFrames, setRunAheadFrames] = createSignal<number>(initial.runAheadFrames);
  const [autoRemoveOnDelete, setAutoRemoveOnDelete] = createSignal<boolean>(initial.autoRemoveOnDelete);
  const [rewindEnabled, setRewindEnabled] = createSignal<boolean>(initial.rewindEnabled);
  const [rewindCaptureIntervalFrames, setRewindCaptureIntervalFrames] =
    createSignal<number>(initial.rewindCaptureIntervalFrames);
  const [rewindBufferMegabytes, setRewindBufferMegabytes] =
    createSignal<number>(initial.rewindBufferMegabytes);
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
      libraryFolders: libraryFolders(),
      shaderPreset: shaderPreset(),
      bloomAmount: bloomAmount(),
      runAheadFrames: runAheadFrames(),
      autoRemoveOnDelete: autoRemoveOnDelete(),
      rewindEnabled: rewindEnabled(),
      rewindCaptureIntervalFrames: rewindCaptureIntervalFrames(),
      rewindBufferMegabytes: rewindBufferMegabytes(),
    });
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
    libraryFolders, setLibraryFolders,
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
