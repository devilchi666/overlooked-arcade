// Typed Tauri bridge — rewind / TAS / save-slot domain.
//
// Theming Phase 4 Slice 5 (PR A, with emulatorApi). The in-game state-history
// surface: the rewind buffer (config + live state + scrub control), TAS
// recording / replay + the recordings list, and the per-ROM save-slot
// inventory. Same convention as the other platform/api modules (see
// docs/PLANS/theming-platform-api-bridge.md): one typed named export per
// command, thin pass-through, no error handling here, command-name string lives
// ONLY in this file.
//
// `getRewindState` / `listSaveSlots` are generic (D14) — RewindState is read
// identically in two places and SaveSlot in two; rather than force an import on
// either, the wrappers default to the canonical shape defined here and each
// call site keeps its local view via the type arg.

import { invoke } from "@tauri-apps/api/core";

// --- Backend-contract types this domain owns ----------------------------

/// Live rewind-buffer state (`get_rewind_state`). Mirror of Rust
/// `SharedRewindState`. The canonical default for `getRewindState`.
export type RewindState = {
  enabled: boolean;
  snapshotCount: number;
  byteSize: number;
  captureIntervalFrames: number;
  fps: number;
  scrubbing: boolean;
  scrubPosition: number;
};

/// The `set_rewind_config` payload.
export type RewindConfig = {
  enabled: boolean;
  captureIntervalFrames: number;
  maxMegabytes: number;
};

/// TAS engine mode (`get_tas_state`). Lowercase serde-rename of the Rust enum.
export type TasMode = "idle" | "recording" | "replaying";

/// Live TAS state (`get_tas_state`).
export type TasState = {
  mode: TasMode;
  frame: number;
  totalFrames: number;
  displayName: string;
};

/// One recorded TAS file (`list_tas_recordings`).
export type TasListEntry = {
  filePath: string;
  displayName: string;
  recordedAtUnixMs: number;
  frameCount: number;
  fps: number;
  durationSeconds: number;
};

/// One per-ROM save slot (`list_save_slots`). The canonical default for
/// `listSaveSlots`.
export type SaveSlot = {
  slot: number;
  exists: boolean;
  sizeBytes: number;
  modifiedAtMs?: number;
  thumbnailDataUrl?: string;
};

// --- Rewind -------------------------------------------------------------

/// Live rewind-buffer state. Generic (D14): canonical shape is `RewindState`.
export function getRewindState<T = RewindState>(): Promise<T> {
  return invoke<T>("get_rewind_state");
}

/// Push the rewind config (enable + capture cadence + buffer cap) to the emu.
export function setRewindConfig(config: RewindConfig): Promise<void> {
  return invoke("set_rewind_config", config);
}

/// Enter rewind-scrub mode (pause + arm the scrub slider).
export function startRewindScrub(): Promise<void> {
  return invoke("start_rewind_scrub");
}

/// Leave rewind-scrub mode, committing (jump to the scrubbed frame) or not.
export function endRewindScrub(commit: boolean): Promise<void> {
  return invoke("end_rewind_scrub", { commit });
}

/// Move the scrub cursor to `stepsBack` snapshots before the live frame.
export function setRewindScrubPosition(stepsBack: number): Promise<void> {
  return invoke("set_rewind_scrub_position", { stepsBack });
}

// --- TAS ----------------------------------------------------------------

/// Live TAS engine state.
export function getTasState(): Promise<TasState> {
  return invoke<TasState>("get_tas_state");
}

/// Start recording a TAS from the current frame.
export function startTasRecording(displayName: string): Promise<void> {
  return invoke("start_tas_recording", { displayName });
}

/// Stop the active TAS recording, optionally discarding it.
export function stopTasRecording(discard: boolean): Promise<void> {
  return invoke("stop_tas_recording", { discard });
}

/// Replay a recorded TAS file from the start.
export function startTasReplay(filePath: string): Promise<void> {
  return invoke("start_tas_replay", { filePath });
}

/// Stop the active TAS replay.
export function stopTasReplay(): Promise<void> {
  return invoke("stop_tas_replay");
}

/// The recorded TAS files for a ROM.
export function listTasRecordings(romPath: string): Promise<TasListEntry[]> {
  return invoke<TasListEntry[]>("list_tas_recordings", { romPath });
}

/// Delete a recorded TAS file.
export function deleteTasRecording(filePath: string): Promise<void> {
  return invoke("delete_tas_recording", { filePath });
}

// --- Save slots ---------------------------------------------------------

/// The per-ROM save-slot inventory. Generic (D14): canonical shape is `SaveSlot`.
export function listSaveSlots<T = SaveSlot>(romPath: string): Promise<T[]> {
  return invoke<T[]>("list_save_slots", { romPath });
}

/// Delete a per-ROM save slot.
export function deleteSaveSlot(romPath: string, slot: number): Promise<void> {
  return invoke("delete_save_slot", { romPath, slot });
}
