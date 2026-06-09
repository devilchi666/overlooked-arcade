// Typed Tauri bridge — settings / display / audio domain.
//
// Theming Phase 4 (the platform/api/ bridge): every backend command in the
// video-display / audio / system-settings / per-game-overrides / shell-mode
// cluster lives behind exactly ONE named, typed wrapper here. Themes and
// platform code call these instead of `invoke("set_window_mode", …)` so the
// backend contract is pinned in a single place and a theme author has a
// discoverable, typed surface to build against.
//
// Convention (see docs/PLANS/theming-platform-api-bridge.md):
//   - One named export per command, camelCase of the command name.
//   - Thin typed pass-through. No behavior change, no error handling here —
//     call sites keep their own try/catch + reportInvokeError(...).
//   - Command-name string lives ONLY in this file (grep-verifiable).
//
// Slice 1 of 6. The `no raw invoke() outside platform/api/` lint rule lands
// in Slice 6 once the count hits zero.

import { invoke } from "@tauri-apps/api/core";
import type {
  ShaderPreset,
  ShellMode,
  MonitorInfo,
  AudioDeviceInfo,
} from "@oa/platform/settings/store";
import type { ShaderPresetEntry } from "@oa/platform/settings/shader_presets";

// --- Backend-contract types this domain owns ----------------------------
//
// These mirror Rust structs and are the canonical home for the shapes the
// settings commands move. Existing call sites still carry their own partial
// views (each declares only the fields it touches) and pass them via the
// generic type parameter on the getters below; new code should prefer these.

/// Overscan crop, in source pixels per edge. All-zero == "no crop".
export type OverscanCropPrefs = {
  top: number;
  bottom: number;
  left: number;
  right: number;
};

/// Live video-capture state (`get_video_state`).
export type VideoState = {
  capturing: boolean;
  frameCount: number;
  droppedFrameCount: number;
  displayName: string;
  clipDir: string;
};

/// Per-game override blob (`get/set_game_overrides`). Superset of the
/// per-file frontend views; the Rust struct carries a few more fields
/// (core_options / analog_routing / platform_music_path) that round-trip
/// generically through `set_game_overrides`.
export type GameOverrides = {
  scalingOverride?: string | null;
  windowModeOverride?: string | null;
  monitorIndexOverride?: number | null;
  regionOverride?: string | null;
  shaderPreset?: string | null;
  bloomAmount?: number | null;
  patchPath?: string | null;
  rewindEnabled?: boolean | null;
  rewindCaptureIntervalFrames?: number | null;
  rewindBufferMegabytes?: number | null;
  displayAspectOverride?: number | null;
  overscanCropOverride?: OverscanCropPrefs | null;
  bezelImagePath?: string | null;
  keypadLayoutNote?: string | null;
  libretroDevice?: number | null;
  libretroDevicePort1?: number | null;
  libretroDevicePort2?: number | null;
  libretroDevicePort3?: number | null;
  libretroDevicePort4?: number | null;
};

/// Per-system settings blob (`get/set_system_settings`). Same display
/// fields as GameOverrides minus the per-game-only ones.
export type SystemSettings = {
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

// --- Video / display -----------------------------------------------------

/// Live video-capture state for the running session.
export function getVideoState(): Promise<VideoState> {
  return invoke<VideoState>("get_video_state");
}

/// Set the shell window mode + target monitor. `mode` arrives override-
/// resolved (per-game → per-system → OA-wide) so it is a plain string;
/// `WindowMode` enumerates the canonical set.
export function setWindowMode(mode: string, monitorIndex: number | null): Promise<void> {
  return invoke("set_window_mode", { mode, monitorIndex });
}

/// Set the renderer scaling mode (`ScalingMode` is the canonical set;
/// override resolution widens it to string).
export function setScalingMode(mode: string): Promise<void> {
  return invoke("set_scaling_mode", { mode });
}

/// Set the active shader preset by name.
export function setShaderPreset(preset: ShaderPreset): Promise<void> {
  return invoke("set_shader_preset", { preset });
}

/// Set the Phosphor bloom amount (0..1).
export function setBloomAmount(amount: number): Promise<void> {
  return invoke("set_bloom_amount", { amount });
}

/// Set the run-ahead frame count (0 = off, clamped 0..=5 on the emu thread).
export function setRunAhead(frames: number): Promise<void> {
  return invoke("set_run_ahead", { frames });
}

/// Override the display aspect ratio; `null` trusts the core's reported AV.
export function setDisplayAspectOverride(aspect: number | null): Promise<void> {
  return invoke("set_display_aspect_override", { aspect });
}

/// Set the per-edge overscan crop (source pixels).
export function setOverscanCrop(crop: OverscanCropPrefs): Promise<void> {
  return invoke("set_overscan_crop", crop);
}

/// Override the bezel image with an absolute path to a PNG/JPEG/WebP.
export function setBezelImageOverride(path: string): Promise<void> {
  return invoke("set_bezel_image_override", { path });
}

/// Clear any bezel override; the shader preset's TOML default stays active.
export function clearBezelImageOverride(): Promise<void> {
  return invoke("clear_bezel_image_override");
}

/// List the connected monitors (index / geometry / scale).
export function listMonitors(): Promise<MonitorInfo[]> {
  return invoke<MonitorInfo[]>("list_monitors");
}

/// List the available shader presets (built-in TOML + user files).
export function listShaderPresets(): Promise<ShaderPresetEntry[]> {
  return invoke<ShaderPresetEntry[]>("list_shader_presets");
}

/// Read the persisted presentation mode (windowed-desktop / cabinet / …).
export function getPresentationMode(): Promise<string> {
  return invoke<string>("get_presentation_mode");
}

/// Persist + apply the presentation mode.
export function setPresentationMode(mode: string): Promise<void> {
  return invoke("set_presentation_mode", { mode });
}

// --- Audio ---------------------------------------------------------------

/// List the available output audio devices.
export function listAudioDevices(): Promise<AudioDeviceInfo[]> {
  return invoke<AudioDeviceInfo[]>("list_audio_devices");
}

/// Read the persisted audio-device preference (`null` = system default).
export function getAudioDevicePref(): Promise<string | null> {
  return invoke<string | null>("get_audio_device_pref");
}

/// Persist + hot-swap the output audio device (`null` = system default).
export function setAudioDevicePref(name: string | null): Promise<void> {
  return invoke("set_audio_device_pref", { name });
}

/// Set the gain (0..1) on a named audio bus.
export function setAudioVolume(bus: string, gain: number): Promise<void> {
  return invoke("set_audio_volume", { bus, gain });
}

/// Play `path` on the named bus; `looped` keeps it running (BGM-shaped buses).
export function playAudio(bus: string, path: string, looped: boolean): Promise<void> {
  return invoke("play_audio", { bus, path, looped });
}

/// Stop whatever is playing on the named bus.
export function stopAudio(bus: string): Promise<void> {
  return invoke("stop_audio", { bus });
}

// --- System settings (per-system) ---------------------------------------

/// Read a system's persisted settings. Call sites pass their own partial
/// view via `T` (each declares only the fields it reads).
export function getSystemSettings<T = SystemSettings>(systemId: string): Promise<T> {
  return invoke<T>("get_system_settings", { systemId });
}

/// Write a system's settings blob (carried generically — unknown fields
/// round-trip unchanged).
export function setSystemSettings(systemId: string, settings: Record<string, unknown>): Promise<void> {
  return invoke("set_system_settings", { systemId, settings });
}

// --- Per-game overrides --------------------------------------------------

/// Read a game's override blob. Call sites pass their own partial view via `T`.
export function getGameOverrides<T = GameOverrides>(gameId: string): Promise<T> {
  return invoke<T>("get_game_overrides", { id: gameId });
}

/// Write a game's override blob (carried generically — unknown fields
/// round-trip unchanged).
export function setGameOverrides(gameId: string, overrides: Record<string, unknown>): Promise<void> {
  return invoke("set_game_overrides", { id: gameId, overrides });
}

// --- Shell mode / kiosk --------------------------------------------------

/// Read the *active* shell mode for this session (env override may differ
/// from the saved pref). Returns the raw string; callers narrow to ShellMode.
export function getShellMode(): Promise<string> {
  return invoke<string>("get_shell_mode");
}

/// Read the persisted shell-mode preference (file-backed, not localStorage).
export function getShellModePref(): Promise<string> {
  return invoke<string>("get_shell_mode_pref");
}

/// Persist the shell-mode preference.
export function setShellModePref(mode: ShellMode): Promise<void> {
  return invoke("set_shell_mode_pref", { mode });
}

/// True when the session was launched with the `--kiosk` override.
export function getKioskMode(): Promise<boolean> {
  return invoke<boolean>("get_kiosk_mode");
}
