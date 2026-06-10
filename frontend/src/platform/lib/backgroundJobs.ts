// Background-jobs store. Phase 2 of the background-jobs arc.
//
// Module-level signals are the single source of truth for "what is
// OA doing right now?" Hydrates once at module import via
// `list_active_jobs` and stays in sync via the `oa://job-event`
// broadcast the Rust JobRegistry already emits.
//
// Race-safe hydration: events that arrive BEFORE the initial
// list_active_jobs response queue into `pendingEvents` and are
// applied after the snapshot lands. Otherwise a fast-firing Created
// event between listener setup and hydration could be missed.
//
// All mutation helpers are best-effort silent-on-error — the bar
// degrades to whatever state the next `oa://job-event` carries
// rather than throwing into the UI.

import { createSignal, type Accessor } from "solid-js";
import { createStore, produce } from "solid-js/store";
import * as jobsApi from "@oa/platform/api/jobsApi";
import { downloadCore } from "@oa/platform/api/coresApi";
import { resolveCompletionChime } from "@oa/platform/api/shellApi";
import { listen } from "@tauri-apps/api/event";
import { playAudio } from "./audio";

/// Mirrors `apps/oa-shell/src/job_registry.rs::JobState` serialized
/// as snake_case. Keep in lockstep when new states are added.
export type JobState =
  | "pending"
  | "running"
  | "paused"
  | "completed"
  | "failed"
  | "cancelled"
  | "interrupted";

const FINISHED_STATES: ReadonlyArray<JobState> = [
  "completed",
  "failed",
  "cancelled",
];

function isFinished(state: JobState): boolean {
  return FINISHED_STATES.includes(state);
}

/// Mirrors `apps/oa-shell/src/job_registry.rs::JobSnapshot`
/// (serde rename_all = "camelCase"). The shape is identical to what
/// `list_active_jobs` and the embedded snapshot on the
/// `oa://job-event` Created variant return.
export type JobSnapshot = {
  id: number;
  kind: string;
  label: string;
  systemId: string | null;
  targetId: string | null;
  parentJobId: number | null;
  isPrereq: boolean;
  state: JobState;
  done: number;
  total: number | null;
  unit: string;
  lastEventAt: number;
  startedAt: number;
  finishedAt: number | null;
  canResume: boolean;
  errorMessage: string | null;
  retryCount: number;
};

/// Mirrors `apps/oa-shell/src/job_registry.rs::JobEvent`
/// (serde tag = "type" + rename_all = "snake_case"). Field renames
/// (`jobId`) are applied on the Rust side; the TypeScript shape sees
/// the wire-format directly.
export type JobEvent =
  | { type: "created"; snapshot: JobSnapshot }
  | {
      type: "progressed";
      jobId: number;
      done: number;
      total: number | null;
    }
  | { type: "state_changed"; jobId: number; state: JobState }
  | { type: "completed"; jobId: number }
  | { type: "failed"; jobId: number; error: string }
  | { type: "resume_prompt"; snapshot: JobSnapshot };

/// Mirrors `apps/oa-shell/src/job_prefs.rs::JobPrefs`. Read once at
/// module load + refreshed when the Settings UI mutates a value via
/// `refreshJobPrefs()`. The signal exposed below lets the bar +
/// dialogs react to changes in-flight.
export type JobPrefs = {
  promptBeforeResumeOnLaunch: Record<string, boolean>;
  soundOnCompletion: boolean;
  alwaysShowBar: boolean;
};

const [jobPrefsSig, setJobPrefsSig] = createSignal<JobPrefs>({
  promptBeforeResumeOnLaunch: {},
  soundOnCompletion: true,
  alwaysShowBar: false,
});

export const jobPrefs: Accessor<JobPrefs> = jobPrefsSig;

/// Refresh the JobPrefs signal from the backend. Called once at
/// module load + after the Settings panel toggles update a value
/// (the toggle handlers receive the updated prefs back and can call
/// this for free, but the explicit refresh keeps the API simple).
export async function refreshJobPrefs(): Promise<void> {
  try {
    const p = await jobsApi.getJobPrefs<JobPrefs>();
    setJobPrefsSig(p);
  } catch (e) {
    console.warn("[oa-jobs] refresh job prefs failed:", e);
  }
}
void refreshJobPrefs();

/// Pending ResumePrompt snapshots (kinds with opt-out + interrupted
/// rows from the previous crashed run). Drained by ResumePromptDialog.
const [resumePromptQueueSig, setResumePromptQueueSig] = createSignal<JobSnapshot[]>([]);
export const resumePromptQueue: Accessor<JobSnapshot[]> = resumePromptQueueSig;

export function dismissResumePrompt(jobId: number): void {
  setResumePromptQueueSig((q) => q.filter((s) => s.id !== jobId));
}

/// Reactive list of currently-active jobs (pending / running / paused).
/// Ordered most-recently-started first so the bar's stack-visible
/// layout puts the freshly-clicked operation at the top.
///
/// Backed by a Solid store so that Progressed events mutate the
/// matching row's fields in PLACE rather than swapping in a new
/// object reference. `<For>` keyed by identity then preserves the
/// row's DOM across progress ticks — without this, the bar's
/// pause/cancel buttons get destroyed and recreated 10×/sec and
/// clicks never land (mousedown lands on one DOM node, mouseup on
/// another, so the browser drops the click).
type JobsStore = { items: JobSnapshot[] };
const [store, setStore] = createStore<JobsStore>({ items: [] });
export const activeJobs: Accessor<JobSnapshot[]> = () => store.items;

/// Monotonic counter bumped on every Progressed event. The bar's
/// handle-pulse animation keys off this so the dot re-animates on
/// each tick. Lives separate from the store because store-field
/// mutations don't surface through the array-identity subscription
/// `activeJobs()` carries; without this signal the pulse would only
/// fire when jobs are created or finished, not while they progress.
const [progressTickSig, setProgressTickSig] = createSignal(0);
export const progressTick: Accessor<number> = progressTickSig;

let hydrated = false;
const pendingEvents: JobEvent[] = [];

function applyEvent(evt: JobEvent): void {
  switch (evt.type) {
    case "created": {
      const incoming = evt.snapshot;
      setStore(
        "items",
        produce((arr) => {
          // Hydrate + Created race: list_active_jobs may have already
          // surfaced this row. Idempotent — replace if present,
          // prepend otherwise.
          const idx = arr.findIndex((j) => j.id === incoming.id);
          if (idx >= 0) {
            arr[idx] = incoming;
          } else {
            arr.unshift(incoming);
          }
        }),
      );
      break;
    }
    case "progressed":
      setStore(
        "items",
        produce((arr) => {
          const j = arr.find((j) => j.id === evt.jobId);
          if (!j) return;
          j.done = evt.done;
          if (evt.total !== null) j.total = evt.total;
        }),
      );
      setProgressTickSig((t) => t + 1);
      break;
    case "state_changed":
      // Finished states drop the row from the active list outright —
      // the dedicated Completed / Failed events that follow then
      // become no-ops here (Phase 5's recent-activity panel will
      // consume those separately).
      if (isFinished(evt.state)) {
        setStore(
          "items",
          produce((arr) => {
            const idx = arr.findIndex((j) => j.id === evt.jobId);
            if (idx >= 0) arr.splice(idx, 1);
          }),
        );
      } else {
        setStore(
          "items",
          produce((arr) => {
            const j = arr.find((j) => j.id === evt.jobId);
            if (j) j.state = evt.state;
          }),
        );
      }
      break;
    case "completed":
      // No-op for the active list; state_changed already filtered.
      // Polish-B: dispatch the completion chime if enabled. Resolver
      // returns null when no asset is bundled (silent fallback).
      void maybePlayCompletionChime();
      break;
    case "failed":
      // No-op for the active list; state_changed already filtered.
      // No chime on failure (per plan §"Notification on completion"
      // — chime signals "something finished, check if you care."
      // Failures already surface via the bar's red state pill +
      // recent-activity Failed tab).
      break;
    case "resume_prompt":
      // Polish-D: enqueue the snapshot for ResumePromptDialog.
      // Each kind+target pair only enqueues once even if the
      // backend re-emits (idempotent prepend like Created).
      setResumePromptQueueSig((q) => {
        if (q.some((s) => s.id === evt.snapshot.id)) return q;
        return [evt.snapshot, ...q];
      });
      break;
  }
}

async function maybePlayCompletionChime(): Promise<void> {
  if (!jobPrefsSig().soundOnCompletion) return;
  try {
    const path = await resolveCompletionChime();
    if (path) {
      await playAudio("ui-sounds", path, false);
    }
    // path === null → no chime bundled → silent fallback per spec.
  } catch (e) {
    // Resolver / play failure shouldn't surface to the operator;
    // background-jobs UX is about visibility, not soundtrack.
    console.warn("[oa-jobs] completion chime dispatch failed:", e);
  }
}

// Set up the listener BEFORE invoking hydrate so a Created that
// fires between listen + invoke is captured by `pendingEvents`.
void listen<JobEvent>("oa://job-event", (event) => {
  if (!hydrated) {
    pendingEvents.push(event.payload);
    return;
  }
  applyEvent(event.payload);
}).catch((e) => {
  console.warn("[oa-jobs] listen oa://job-event failed:", e);
});

void jobsApi.listActiveJobs()
  .then((rows) => {
    setStore("items", rows);
    hydrated = true;
    while (pendingEvents.length > 0) {
      const evt = pendingEvents.shift();
      if (evt) applyEvent(evt);
    }
  })
  .catch((e) => {
    // Soft-fail: bar stays empty until next event lands. Common
    // cause is the Rust registry having soft-failed at startup;
    // logged but not user-facing.
    console.warn("[oa-jobs] hydrate list_active_jobs failed:", e);
    hydrated = true; // unblock listener so future events still apply
  });

// ---------------------------------------------------------------------------
// Mutation helpers
// ---------------------------------------------------------------------------

export async function pauseJob(jobId: number): Promise<void> {
  try {
    await jobsApi.pauseJob(jobId);
  } catch (e) {
    console.warn(`[oa-jobs] pause_job(${jobId}) failed:`, e);
  }
}

export async function resumeJob(jobId: number): Promise<void> {
  try {
    await jobsApi.resumeJob(jobId);
  } catch (e) {
    console.warn(`[oa-jobs] resume_job(${jobId}) failed:`, e);
  }
}

export async function cancelJob(jobId: number): Promise<void> {
  try {
    await jobsApi.cancelJob(jobId);
  } catch (e) {
    console.warn(`[oa-jobs] cancel_job(${jobId}) failed:`, e);
  }
}

export async function pauseAllJobs(paused: boolean): Promise<void> {
  try {
    await jobsApi.pauseAllJobs(paused);
  } catch (e) {
    console.warn(`[oa-jobs] pause_all_jobs(${paused}) failed:`, e);
  }
}

export async function cancelAllJobs(): Promise<void> {
  try {
    await jobsApi.cancelAllJobs();
  } catch (e) {
    console.warn("[oa-jobs] cancel_all_jobs failed:", e);
  }
}

/// Phase 3b duplicate-trigger helper. Wraps `coresApi.downloadCore`
/// with a check_duplicate_job pre-flight: when an existing
/// core_download for the same `base` is active, prompts the operator
/// to either restart it (cancel + retry) or wait for the current one
/// to finish. Returns null when the operator chose to wait (so the
/// caller knows the current invocation is a no-op) and the final
/// path when the download proceeded.
///
/// Plan §"Duplicate same-job triggering" specifies three options
/// (Wait / Restart / Cancel) but Wait + Cancel are operationally
/// identical at the call-site level — both end up as "don't do
/// anything new." Phase 3b collapses to a 2-option window.confirm;
/// Phase 5 may upgrade to a richer 3-option dialog component.
export async function downloadCoreWithDuplicateCheck(
  base: string,
  parentJobId?: number,
): Promise<string | null> {
  let existing: JobSnapshot | null = null;
  try {
    existing = await jobsApi.checkDuplicateJob("core_download", base);
  } catch (e) {
    console.warn("[oa-jobs] check_duplicate_job failed:", e);
    // Fall through to the invoke — the duplicate check is best-effort.
  }
  if (existing) {
    const confirmed = window.confirm(
      `${existing.label} is already in progress. Restart it? (Cancel keeps the current download running.)`,
    );
    if (!confirmed) return null;
    await cancelJob(existing.id);
    // Give the worker a moment to observe the cancel flag + finalize
    // the row before we try to create a duplicate row for the new
    // attempt. 250 ms covers the chunk-loop's 100 ms pause poll plus
    // the cancel-cleanup contract.
    await new Promise((r) => setTimeout(r, 250));
  }
  return downloadCore(base, parentJobId);
}
