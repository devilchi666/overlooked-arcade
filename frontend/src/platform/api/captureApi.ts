// Typed Tauri bridge — capture domain (screenshots + video clips).
//
// Theming Phase 4 Slice 5 (PR B, with cheatsApi + milestonesApi). The capture
// surface: the per-ROM screenshot gallery (list / delete / reveal-in-folder)
// and the video-clip capture pipeline (list / record / stop / delete / convert
// to WebM / reveal-in-folder). Same convention as the other platform/api
// modules (see docs/PLANS/theming-platform-api-bridge.md): one typed named
// export per command, thin pass-through, no error handling here, command-name
// string lives ONLY in this file.
//
// Per D16 the contract types live here (the platform↛components boundary
// forbids importing them from ScreenshotGalleryDialog / QuickSettings); the
// sole consumers keep their structurally-identical local copies.

import { invoke } from "@tauri-apps/api/core";

// --- Backend-contract types this domain owns ----------------------------

/// One screenshot on disk for a ROM (`list_screenshots`).
export type ScreenshotEntry = {
  path: string;
  fileName: string;
  sizeBytes: number;
  modifiedUnixMs: number;
};

/// One recorded video clip directory (`list_video_clips`).
export type VideoClipEntry = {
  clipDir: string;
  displayName: string;
  recordedAtUnixMs: number;
  frameCount: number;
  droppedFrameCount: number;
  fps: number;
  width: number;
  height: number;
  durationSeconds: number;
};

// --- Screenshots --------------------------------------------------------

/// The screenshots captured for a ROM.
export function listScreenshots(romPath: string): Promise<ScreenshotEntry[]> {
  return invoke<ScreenshotEntry[]>("list_screenshots", { romPath });
}

/// Delete a screenshot by path.
export function deleteScreenshot(path: string): Promise<void> {
  return invoke("delete_screenshot", { path });
}

/// Reveal a ROM's screenshot folder in the OS file manager.
export function openScreenshotFolder(romPath: string): Promise<void> {
  return invoke("open_screenshot_folder", { romPath });
}

// --- Video clips --------------------------------------------------------

/// The recorded video clips for a ROM.
export function listVideoClips(romPath: string): Promise<VideoClipEntry[]> {
  return invoke<VideoClipEntry[]>("list_video_clips", { romPath });
}

/// Delete a recorded video clip by its directory.
export function deleteVideoClip(clipDir: string): Promise<void> {
  return invoke("delete_video_clip", { clipDir });
}

/// Start recording a video clip from the current frame.
export function startVideoCapture(displayName: string): Promise<void> {
  return invoke("start_video_capture", { displayName });
}

/// Stop the active video capture, optionally discarding it.
export function stopVideoCapture(discard: boolean): Promise<void> {
  return invoke("stop_video_capture", { discard });
}

/// Transcode a captured clip to WebM; returns the output path.
export function convertVideoClipToWebm(clipDir: string): Promise<string> {
  return invoke<string>("convert_video_clip_to_webm", { clipDir });
}

/// Reveal a video clip's folder in the OS file manager.
export function openVideoClipFolder(clipDir: string): Promise<void> {
  return invoke("open_video_clip_folder", { clipDir });
}
