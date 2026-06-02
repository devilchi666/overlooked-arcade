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
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

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
  | { type: "failed"; jobId: number; error: string };

const [activeJobsSig, setActiveJobsSig] = createSignal<JobSnapshot[]>([]);
/// Reactive list of currently-active jobs (pending / running / paused).
/// Ordered most-recently-started first so the bar's stack-visible
/// layout puts the freshly-clicked operation at the top.
export const activeJobs: Accessor<JobSnapshot[]> = activeJobsSig;

let hydrated = false;
const pendingEvents: JobEvent[] = [];

function applyEvent(evt: JobEvent): void {
  switch (evt.type) {
    case "created": {
      const incoming = evt.snapshot;
      setActiveJobsSig((s) => {
        // Hydrate + Created race: list_active_jobs may have already
        // surfaced this row. Idempotent — replace if present, prepend
        // otherwise.
        const idx = s.findIndex((j) => j.id === incoming.id);
        if (idx >= 0) {
          const next = s.slice();
          next[idx] = incoming;
          return next;
        }
        return [incoming, ...s];
      });
      break;
    }
    case "progressed":
      setActiveJobsSig((s) =>
        s.map((j) =>
          j.id === evt.jobId
            ? { ...j, done: evt.done, total: evt.total ?? j.total }
            : j,
        ),
      );
      break;
    case "state_changed":
      // Finished states drop the row from the active list outright —
      // the dedicated Completed / Failed events that follow then
      // become no-ops here (Phase 5's recent-activity panel will
      // consume those separately).
      if (isFinished(evt.state)) {
        setActiveJobsSig((s) => s.filter((j) => j.id !== evt.jobId));
      } else {
        setActiveJobsSig((s) =>
          s.map((j) => (j.id === evt.jobId ? { ...j, state: evt.state } : j)),
        );
      }
      break;
    case "completed":
    case "failed":
      // No-op for the active list; state_changed already filtered.
      // Phase 5 will hook these for the recent-activity panel +
      // completion chime.
      break;
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

void invoke<JobSnapshot[]>("list_active_jobs")
  .then((rows) => {
    setActiveJobsSig(rows);
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
    await invoke<boolean>("pause_job", { jobId });
  } catch (e) {
    console.warn(`[oa-jobs] pause_job(${jobId}) failed:`, e);
  }
}

export async function resumeJob(jobId: number): Promise<void> {
  try {
    await invoke<boolean>("resume_job", { jobId });
  } catch (e) {
    console.warn(`[oa-jobs] resume_job(${jobId}) failed:`, e);
  }
}

export async function cancelJob(jobId: number): Promise<void> {
  try {
    await invoke<boolean>("cancel_job", { jobId });
  } catch (e) {
    console.warn(`[oa-jobs] cancel_job(${jobId}) failed:`, e);
  }
}

export async function pauseAllJobs(paused: boolean): Promise<void> {
  try {
    await invoke<number>("pause_all_jobs", { paused });
  } catch (e) {
    console.warn(`[oa-jobs] pause_all_jobs(${paused}) failed:`, e);
  }
}

export async function cancelAllJobs(): Promise<void> {
  try {
    await invoke<number>("cancel_all_jobs");
  } catch (e) {
    console.warn("[oa-jobs] cancel_all_jobs failed:", e);
  }
}
