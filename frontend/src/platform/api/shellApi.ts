// Typed Tauri bridge — shell / host domain (app lifecycle, data dir, input
// intercept, logging, file reveal, ScummVM CLI, per-system sounds).
//
// Theming Phase 4 Slice 6 (the closer, with jobsApi + systemApi). The grab-bag
// of host-process commands that don't belong to a content domain: quit, the
// resolved data dir, the gameplay-input intercept gate, the frontend→Rust log
// bridge + the in-memory log ring + log-file reveal, reveal-in-folder, the
// ScummVM detection CLI, and the per-system sound/music resolvers. Same
// convention as the other platform/api modules (see
// docs/PLANS/theming-platform-api-bridge.md): one typed named export per
// command, thin pass-through, no error handling here, command-name string lives
// ONLY in this file.
//
// The reads with a single local-typed consumer (log entries, ScummVM detection
// rows) stay generic (D14) rather than naming the consumer's type here.

import { invoke } from "@tauri-apps/api/core";

// --- App lifecycle ------------------------------------------------------

/// Quit the application.
export function quitApp(): Promise<void> {
  return invoke("quit_app");
}

/// Relaunch the application. Theming Substrate ARC 1 — switching the active
/// whole-shell theme requires a restart in ARC 1 (DECISIONS D5). The backend
/// runs the same cleanup as quit, then Tauri spawns a fresh process; the
/// returned Promise never resolves (the webview is torn down first).
export function restartApp(): Promise<void> {
  return invoke("restart_app");
}

/// Open the WebView inspector (DevTools). Debug builds only — rejects in
/// release. Wired to the dev-tools panel in Settings → About.
export function openDevtools(): Promise<void> {
  return invoke("open_devtools");
}

/// Enable on-demand verbose backend logging for the given subsystems
/// ("media" / "audio" / "render"); [] restores the base level. DevTools panel.
export function setLogStreams(streams: string[]): Promise<void> {
  return invoke("set_log_streams", { streams });
}

/// The resolved OA data dir (portable `<exe_dir>/settings/` or AppData).
export function getOaDataDir(): Promise<string> {
  return invoke<string>("get_oa_data_dir");
}

/// Gate gameplay input + hotkeys while a UI capture/overlay is active.
export function setUiIntercepting(intercepting: boolean): Promise<void> {
  return invoke("set_ui_intercepting", { intercepting });
}

/// The direct-launch config parsed from CLI args (null = normal launch).
/// Generic (D14): the caller owns the `DirectLaunchConfig` shape.
export function getDirectLaunchConfig<T>(): Promise<T | null> {
  return invoke<T | null>("get_direct_launch_config");
}

// --- Game focus ---------------------------------------------------------

/// Current game-focus state (machine-focus vs shell-focus).
export function getGameFocus(): Promise<boolean> {
  return invoke<boolean>("get_game_focus");
}

/// The configured game-focus toggle chord (e.g. "Ctrl+G").
export function getGameFocusToggle(): Promise<string> {
  return invoke<string>("get_game_focus_toggle");
}

/// Set the game-focus toggle chord; returns the normalized chord string.
export function setGameFocusToggle(chord: string): Promise<string> {
  return invoke<string>("set_game_focus_toggle", { chord });
}

// --- Logging ------------------------------------------------------------

/// Ship one frontend log line into the unified Rust log stream.
export function logFromFrontend(level: string, target: string, message: string): Promise<void> {
  return invoke("log_from_frontend", { level, target, message });
}

/// The last N entries of the in-memory log ring (Debug Log dialog poll).
export function getRecentLogs<T>(limit: number): Promise<T[]> {
  return invoke<T[]>("get_recent_logs", { limit });
}

/// The absolute path of the current session's log file.
export function getLogFilePath(): Promise<string | null> {
  return invoke<string | null>("get_log_file_path");
}

/// Reveal the logs folder in the OS file manager.
export function revealLogsFolder(): Promise<void> {
  return invoke("reveal_logs_folder");
}

/// Reveal a game file in the OS file manager.
export function revealGameFileInFolder(filePath: string): Promise<void> {
  return invoke("reveal_game_file_in_folder", { filePath });
}

// --- ScummVM detection --------------------------------------------------

/// Locate a ScummVM CLI on PATH / common install dirs; null if not found.
export function findScummvmCli(): Promise<string | null> {
  return invoke<string | null>("find_scummvm_cli");
}

/// Detect ScummVM-playable subdirectories under a parent dir (table mode).
export function detectScummvmDirectories<T>(parentDir: string): Promise<T[]> {
  return invoke<T[]>("detect_scummvm_directories", { parentDir });
}

/// Layer in CLI-detected matches from a standalone ScummVM install.
export function runScummvmCliDetect<T>(scummvmPath: string, parentDir: string): Promise<T[]> {
  return invoke<T[]>("run_scummvm_cli_detect", { scummvmPath, parentDir });
}

/// Write `.scummvm` descriptor files for the chosen rows; returns the count.
export function writeScummvmDescriptors<W>(writes: W): Promise<number> {
  return invoke<number>("write_scummvm_descriptors", { writes });
}

// --- Sounds -------------------------------------------------------------

/// Resolve a per-system UI-sound event to a file path (null = no override).
/// `themeId` is the active theme's id (or `null`) — S5.1's theme tier checks
/// `assets/themes/<themeId>/system-ui/…/sounds/` between the operator
/// override and the platform per-system + `_baseline` cues. The dispatcher
/// (`platform/lib/audio.ts`) resolves the active id ambiently.
export function resolveUiSound(
  themeId: string | null,
  systemId: string,
  event: string,
): Promise<string | null> {
  return invoke<string | null>("resolve_ui_sound", { themeId, systemId, event });
}

/// Resolve the platform music for a (system, optional game) pair.
export function resolvePlatformMusic(systemId: string, gameId: string | null): Promise<string | null> {
  return invoke<string | null>("resolve_platform_music", { systemId, gameId });
}

/// Resolve the job-completion chime sound path (null = none configured).
export function resolveCompletionChime(): Promise<string | null> {
  return invoke<string | null>("resolve_completion_chime");
}
