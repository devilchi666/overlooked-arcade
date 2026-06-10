// Typed Tauri bridge — background-jobs domain (the JobRegistry surface).
//
// Theming Phase 4 Slice 6 (the closer, with systemApi + shellApi). The
// background-jobs surface: the active/recent job lists, per-job and bulk
// pause/resume/cancel, history clear, duplicate-job pre-flight, the resume-
// prompt prefs, the one-shot test job, and the library/directory scan kick-off
// + cancel. Same convention as the other platform/api modules (see
// docs/PLANS/theming-platform-api-bridge.md): one typed named export per
// command, thin pass-through, no error handling here (the backgroundJobs.ts
// store keeps its own best-effort try/catch), command-name string lives ONLY
// in this file.
//
// The job shapes (`JobSnapshot` / `JobPrefs`) live in the backgroundJobs store
// (their established home + widely imported); pulled here via `import type`
// (erased — no runtime cycle even though backgroundJobs imports these wrappers).
// `getJobPrefs` / `listRecentJobs` / `setJobResumePrompt` are generic (D14) —
// the Settings panel reads narrower local shapes than the store's canonical.

import { invoke } from "@tauri-apps/api/core";
import type { JobSnapshot, JobPrefs } from "@oa/platform/lib/backgroundJobs";

// --- Backend-contract types this domain owns ----------------------------

/// The `start_background_scan` payload (library import smart-classification).
export type BackgroundScanArgs = {
  folder: string;
  extensions: string[];
  extensionToSystem: Record<string, string>;
};

// --- Job lists ----------------------------------------------------------

/// The currently-active jobs (the store hydrates from this once at load).
export function listActiveJobs(): Promise<JobSnapshot[]> {
  return invoke<JobSnapshot[]>("list_active_jobs");
}

/// Recent finished jobs. Generic (D14): canonical shape is `JobSnapshot`; the
/// Settings history table reads a narrower view.
export function listRecentJobs<T = JobSnapshot>(limit: number): Promise<T[]> {
  return invoke<T[]>("list_recent_jobs", { limit });
}

// --- Mutations ----------------------------------------------------------

/// Pause one job; returns whether the state actually changed.
export function pauseJob(jobId: number): Promise<boolean> {
  return invoke<boolean>("pause_job", { jobId });
}

/// Resume one paused job.
export function resumeJob(jobId: number): Promise<boolean> {
  return invoke<boolean>("resume_job", { jobId });
}

/// Cancel one job.
export function cancelJob(jobId: number): Promise<boolean> {
  return invoke<boolean>("cancel_job", { jobId });
}

/// Pause / unpause every pausable job; returns the count affected.
export function pauseAllJobs(paused: boolean): Promise<number> {
  return invoke<number>("pause_all_jobs", { paused });
}

/// Cancel every cancellable job; returns the count affected.
export function cancelAllJobs(): Promise<number> {
  return invoke<number>("cancel_all_jobs");
}

/// Clear the finished-job history; returns the count removed.
export function clearJobHistory(): Promise<number> {
  return invoke<number>("clear_job_history");
}

/// Pre-flight: is a job of `kind` already active for `targetId`? (Drives the
/// duplicate-download confirm.)
export function checkDuplicateJob(kind: string, targetId: string): Promise<JobSnapshot | null> {
  return invoke<JobSnapshot | null>("check_duplicate_job", { kind, targetId });
}

// --- Resume prefs + test job --------------------------------------------

/// The job-resume prefs. Generic (D14): canonical shape is `JobPrefs`; the
/// Settings panel reads a narrower local view.
export function getJobPrefs<T = JobPrefs>(): Promise<T> {
  return invoke<T>("get_job_prefs");
}

/// Toggle the per-kind resume-prompt opt-out; returns the updated prefs.
export function setJobResumePrompt<T = JobPrefs>(kind: string, prompt: boolean): Promise<T> {
  return invoke<T>("set_job_resume_prompt", { kind, prompt });
}

/// Resume one interrupted job carried over from a crashed prior run.
export function resumeOneInterruptedJob(jobId: number): Promise<boolean> {
  return invoke<boolean>("resume_one_interrupted_job", { jobId });
}

/// Toggle the always-show-bar pref; returns the updated prefs.
export function setJobAlwaysShowBar<T = JobPrefs>(enabled: boolean): Promise<T> {
  return invoke<T>("set_job_always_show_bar", { enabled });
}

/// Toggle the sound-on-completion pref; returns the updated prefs.
export function setJobSoundOnCompletion<T = JobPrefs>(enabled: boolean): Promise<T> {
  return invoke<T>("set_job_sound_on_completion", { enabled });
}

/// Spawn a no-op timed test job (Settings → diagnostics).
export function spawnTestJob(durationSecs: number): Promise<void> {
  return invoke("spawn_test_job", { durationSecs });
}

// --- Library / directory scans ------------------------------------------

/// Kick off a smart-classification library scan; returns the job id.
export function startBackgroundScan(args: BackgroundScanArgs): Promise<number> {
  return invoke<number>("start_background_scan", args);
}

/// Kick off a directory-mode scan (DOSBox / ScummVM subdir systems).
export function startBackgroundDirectoryScan(folder: string, systemId: string): Promise<number> {
  return invoke<number>("start_background_directory_scan", { folder, systemId });
}

/// Cancel a running scan job.
export function cancelBackgroundScan(jobId: number): Promise<void> {
  return invoke("cancel_background_scan", { jobId });
}
