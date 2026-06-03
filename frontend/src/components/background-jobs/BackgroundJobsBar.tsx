// BackgroundJobsBar — Phase 2 of the background-jobs arc.
//
// Persistent UI surface for "what is OA doing right now?" Reads
// `activeJobs` from `lib/backgroundJobs.ts` (which mirrors the Rust
// JobRegistry via the `oa://job-event` broadcast) and routes the
// per-row controls back through the Slice A Tauri commands.
//
// State machine:
//   - Hidden          — no active jobs.
//   - HandleVisible   — active jobs, bar collapsed to a thin handle
//                       at the bottom of the viewport that pulses on
//                       progress events.
//   - Expanded        — full stack-visible bar; auto-collapses to
//                       HandleVisible after 2 s of bar-input idle.
//
// Plan reference: docs/PLANS/background-jobs-and-progress-bar.md
// §"Bar UI shape" + §"Bar placement in the Retroverse layout"
// + §"Existing per-operation UI — hybrid coexistence".
//
// Phase 1 pilot caveat (preserved through this slice): per-row pause
// flips the JobHandle.pause flag and stops the chunk loop streaming,
// but core_download doesn't bridge the flag back to `mark_paused`, so
// the row's `state` column stays `running` until the operator hits
// resume or cancel. UI still shows the row as `running` for now;
// Phase 3 will wire per-kind pause → mark_paused.

import {
  createEffect,
  createMemo,
  createSignal,
  For,
  onCleanup,
  Show,
  type Component,
} from "solid-js";
import {
  activeJobs,
  cancelAllJobs,
  cancelJob,
  jobPrefs,
  pauseAllJobs,
  pauseJob,
  progressTick,
  resumeJob,
  type JobSnapshot,
  type JobState,
} from "../../lib/backgroundJobs";
import RecentActivityPanel from "./RecentActivityPanel";

const MAX_VISIBLE_ROWS = 3;
const AUTO_COLLAPSE_MS = 2000;

/// Per-kind glyph mapped from the snake_case `kind` discriminator.
/// Phase 1 only has `core_download`; the rest are seeded so the bar
/// looks correct once Phase 4 wires them.
const KIND_GLYPH: Record<string, string> = {
  core_download: "↓",
  bulk_core_install: "⤓",
  dat_sync: "⟳",
  hash_resolve: "🔍",
  disc_track_hash: "💿",
  artwork_sync: "🖼",
  metadata_sync: "ℹ",
  folder_scan: "📂",
  mame_listxml_import: "▦",
};

function kindGlyph(kind: string): string {
  return KIND_GLYPH[kind] ?? "•";
}

/// Format `done / total {unit}` for the per-row detail line. Bytes
/// get human-readable scaling (KB / MB / GB); other units pass
/// through as plain integers.
function formatProgressNumbers(
  done: number,
  total: number | null,
  unit: string,
): string {
  if (unit === "bytes") {
    return total !== null
      ? `${formatBytes(done)} / ${formatBytes(total)}`
      : `${formatBytes(done)}`;
  }
  return total !== null
    ? `${done.toLocaleString()} / ${total.toLocaleString()} ${unit}`
    : `${done.toLocaleString()} ${unit}`;
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  const kb = n / 1024;
  if (kb < 1024) return `${kb.toFixed(1)} KB`;
  const mb = kb / 1024;
  if (mb < 1024) return `${mb.toFixed(1)} MB`;
  const gb = mb / 1024;
  return `${gb.toFixed(2)} GB`;
}

function percentDone(j: JobSnapshot): number | null {
  if (j.total === null || j.total <= 0) return null;
  return Math.min(100, Math.max(0, (j.done / j.total) * 100));
}

function stateLabel(state: JobState): string {
  switch (state) {
    case "pending":
      return "Pending";
    case "running":
      return "";
    case "paused":
      return "Paused";
    case "completed":
      return "Done";
    case "failed":
      return "Failed";
    case "cancelled":
      return "Cancelled";
    case "interrupted":
      return "Interrupted";
  }
}

// ===========================================================================
// Component
// ===========================================================================

const BackgroundJobsBar: Component = () => {
  const [expanded, setExpanded] = createSignal(false);
  const [recentOpen, setRecentOpen] = createSignal(false);

  // Auto-collapse timer. Reset every time the operator interacts
  // with the bar. The bar is FIXED-position at the bottom of the
  // viewport, so a "bar interaction" = pointer event within the
  // bar's DOM tree (handled via onMouseMove / onPointerDown below).
  let collapseTimer: number | null = null;
  const cancelCollapseTimer = () => {
    if (collapseTimer !== null) {
      window.clearTimeout(collapseTimer);
      collapseTimer = null;
    }
  };
  const armCollapseTimer = () => {
    cancelCollapseTimer();
    collapseTimer = window.setTimeout(() => {
      setExpanded(false);
      collapseTimer = null;
    }, AUTO_COLLAPSE_MS);
  };

  // When the bar transitions to Expanded, start the idle timer; when
  // it collapses (manually or auto), stop it.
  createEffect(() => {
    if (expanded()) {
      armCollapseTimer();
    } else {
      cancelCollapseTimer();
    }
  });
  onCleanup(cancelCollapseTimer);

  // Pulse the handle on every Progressed event. `progressTick`
  // increments inside the store's Progressed reducer; the pulse dot
  // below reads it via a data attribute so the CSS animation
  // re-triggers each tick.

  const jobs = () => activeJobs();
  const jobCount = () => jobs().length;
  const visibleJobs = () => jobs().slice(0, MAX_VISIBLE_ROWS);
  const hiddenCount = () => Math.max(0, jobCount() - MAX_VISIBLE_ROWS);

  // Pause-all / cancel-all confirmations. Plan §"Bar header":
  // "Both confirm before applying when 3+ jobs are active."
  const handlePauseAll = async () => {
    if (jobCount() >= 3) {
      const ok = window.confirm(
        `Pause all ${jobCount()} running background jobs?`,
      );
      if (!ok) return;
    }
    await pauseAllJobs(true);
  };
  const handleCancelAll = async () => {
    if (jobCount() >= 3) {
      const ok = window.confirm(
        `Cancel all ${jobCount()} running background jobs? Their progress will be discarded per each operation's cancel-cleanup policy.`,
      );
      if (!ok) return;
    } else if (jobCount() >= 1) {
      // Single + double cancels still confirm because cancel is
      // destructive (drops the .partial for core_download, discards
      // partial scan rows for folder_scan, etc.).
      const ok = window.confirm(
        `Cancel ${jobCount() === 1 ? "the running background job" : `all ${jobCount()} running background jobs`}?`,
      );
      if (!ok) return;
    }
    await cancelAllJobs();
  };

  const handleSingleCancel = async (job: JobSnapshot) => {
    const ok = window.confirm(`Cancel "${job.label}"?`);
    if (!ok) return;
    await cancelJob(job.id);
  };

  // Polish-C: when the always-show toggle is on, the bar's handle
  // stays visible even with no active jobs (labels swap to "No
  // active jobs"). When off (the default), the bar only renders
  // while jobCount() > 0 per the original spec.
  const visible = () => jobCount() > 0 || jobPrefs().alwaysShowBar;

  // Pulse-on-progress: a key off pulseTick re-triggers the
  // bar-handle's pulse animation. The actual CSS keyframe lives
  // inline below.
  return (
    <Show when={visible()}>
      <div
        class="oa-bg-jobs-bar pointer-events-none fixed inset-x-0 bottom-0 z-[55] flex justify-center px-4 pb-[60px]"
        onMouseMove={() => {
          if (expanded()) armCollapseTimer();
        }}
        onPointerDown={() => {
          if (expanded()) armCollapseTimer();
        }}
      >
        {/* HandleVisible state — thin pill at the bottom of the
            viewport showing the active job count. Click to expand.
            Polish-C: when always-show is on AND no jobs active,
            the pill shows "No active jobs" and the pulse dot
            stays static. */}
        <Show when={!expanded()}>
          <button
            type="button"
            class="oa-bg-jobs-handle pointer-events-auto group flex items-center gap-2 rounded-t-md border border-(--color-system-accent)/30 border-b-0 bg-(--color-oa-bg-deep)/95 px-4 py-1 text-[0.7rem] font-semibold uppercase tracking-widest text-(--color-oa-ink-dim) backdrop-blur transition hover:border-(--color-system-accent) hover:bg-(--color-oa-bg-deep) hover:text-(--color-oa-ink)"
            onClick={() => setExpanded(true)}
            title={
              jobCount() === 0
                ? "No active background jobs — click to expand"
                : `${jobCount()} background job${jobCount() === 1 ? "" : "s"} active — click to expand`
            }
            aria-label={
              jobCount() === 0
                ? "No active background jobs; click to expand"
                : `${jobCount()} active background jobs; click to expand`
            }
          >
            {/* Pulse dot keyed off pulseTick so it re-animates on
                every activeJobs update (Progressed events). Stays
                static + dim when no jobs are active. */}
            <span
              data-pulse={progressTick()}
              class="oa-bg-jobs-handle-dot inline-block h-2 w-2 rounded-full"
              classList={{
                "bg-(--color-system-accent)": jobCount() > 0,
                "bg-(--color-oa-ink-dim)/40": jobCount() === 0,
              }}
            />
            <span>
              <Show
                when={jobCount() > 0}
                fallback={<>No active jobs</>}
              >
                {jobCount()} background job{jobCount() === 1 ? "" : "s"}
              </Show>
            </span>
            <span class="text-(--color-oa-ink-dim)/60 transition group-hover:text-(--color-system-accent)">
              ▲
            </span>
          </button>
        </Show>

        {/* Expanded state — full stack-visible bar. */}
        <Show when={expanded()}>
          <div
            class="oa-bg-jobs-panel pointer-events-auto flex w-full max-w-2xl flex-col overflow-hidden rounded-md border border-white/10 bg-(--color-oa-bg-deep)/95 shadow-lg backdrop-blur"
            role="region"
            aria-label="Active background jobs"
          >
            {/* Header — visible always when expanded; the
                Pause-all / Cancel-all buttons only enable for 2+
                jobs (plan §"Bar header"). */}
            <div class="flex items-center justify-between gap-3 border-b border-white/5 bg-white/[0.02] px-3 py-1.5">
              <div class="flex items-center gap-2">
                <Show when={jobCount() >= 2}>
                  <button
                    type="button"
                    class="rounded border border-white/10 bg-white/[0.04] px-2 py-1 text-[0.65rem] font-semibold uppercase tracking-widest text-(--color-oa-ink-dim) transition hover:border-white/20 hover:text-(--color-oa-ink)"
                    onClick={(e) => {
                      e.currentTarget.blur();
                      void handlePauseAll();
                    }}
                    title="Pause every active job (Phase 1: pauses streaming but row state stays running until Phase 3 wires per-kind mark_paused)"
                  >
                    Pause all
                  </button>
                  <button
                    type="button"
                    class="rounded border border-rose-500/30 bg-rose-500/10 px-2 py-1 text-[0.65rem] font-semibold uppercase tracking-widest text-rose-200 transition hover:border-rose-400/60 hover:bg-rose-500/20 hover:text-rose-100"
                    onClick={(e) => {
                      e.currentTarget.blur();
                      void handleCancelAll();
                    }}
                    title="Cancel every active job"
                  >
                    Cancel all
                  </button>
                </Show>
                <span class="text-[0.65rem] font-semibold uppercase tracking-widest text-(--color-oa-ink-dim)">
                  {jobCount()} active
                </span>
              </div>
              <div class="flex items-center gap-2">
                <button
                  type="button"
                  class="text-[0.6rem] font-semibold uppercase tracking-widest text-(--color-oa-ink-dim) transition hover:text-(--color-system-accent)"
                  onClick={(e) => {
                    e.currentTarget.blur();
                    setRecentOpen(true);
                  }}
                  title="Recent activity — last 100 finished jobs"
                >
                  Recent activity →
                </button>
                <button
                  type="button"
                  class="text-(--color-oa-ink-dim) transition hover:text-(--color-oa-ink)"
                  onClick={(e) => {
                    e.currentTarget.blur();
                    setExpanded(false);
                  }}
                  title="Collapse"
                  aria-label="Collapse background jobs bar"
                >
                  ▼
                </button>
              </div>
            </div>

            {/* Per-row stack. */}
            <For each={visibleJobs()}>
              {(job) => (
                <JobRow job={job} onSingleCancel={handleSingleCancel} />
              )}
            </For>

            {/* "+N more" affordance. Phase 2 just shows the count;
                Phase 5's recent-activity panel becomes the
                full-stack drill-in. */}
            <Show when={hiddenCount() > 0}>
              <div class="border-t border-white/5 bg-white/[0.02] px-3 py-1 text-center text-[0.65rem] font-semibold uppercase tracking-widest text-(--color-oa-ink-dim)">
                +{hiddenCount()} more
              </div>
            </Show>
          </div>
        </Show>
      </div>
      {/* CSS-only pulse keyframe. Inline so the bar's host stylesheet
          doesn't need a separate import. The `data-pulse` attribute
          re-triggers it on every activeJobs update. */}
      <style>
        {`
        @keyframes oa-bg-jobs-pulse {
          0% { transform: scale(1); opacity: 0.85; }
          50% { transform: scale(1.4); opacity: 1; }
          100% { transform: scale(1); opacity: 0.85; }
        }
        .oa-bg-jobs-handle-dot {
          animation: oa-bg-jobs-pulse 700ms ease-out;
        }
        `}
      </style>
      <RecentActivityPanel
        open={recentOpen()}
        onClose={() => setRecentOpen(false)}
      />
    </Show>
  );
};

const JobRow: Component<{
  job: JobSnapshot;
  onSingleCancel: (job: JobSnapshot) => void;
}> = (props) => {
  const j = () => props.job;
  const pct = createMemo(() => percentDone(j()));
  const numbers = createMemo(() =>
    formatProgressNumbers(j().done, j().total, j().unit),
  );
  const isPaused = () => j().state === "paused";

  return (
    <div class="border-t border-white/5 px-3 py-2">
      <div class="flex items-center gap-3">
        <span
          class="inline-block h-5 w-5 shrink-0 text-center text-base leading-5 text-(--color-system-accent)"
          aria-hidden="true"
        >
          {kindGlyph(j().kind)}
        </span>
        <div class="min-w-0 flex-1 text-[0.8rem] text-(--color-oa-ink)">
          <div class="flex items-center gap-2">
            <span class="truncate font-medium" title={j().label}>
              {j().label}
            </span>
            <Show when={stateLabel(j().state) !== ""}>
              <span class="rounded bg-white/[0.06] px-1.5 py-0.5 text-[0.6rem] font-semibold uppercase tracking-widest text-(--color-oa-ink-dim)">
                {stateLabel(j().state)}
              </span>
            </Show>
          </div>
        </div>
        <span class="shrink-0 text-[0.7rem] tabular-nums text-(--color-oa-ink-dim)">
          {numbers()}
        </span>
        <button
          type="button"
          class="shrink-0 rounded border border-white/10 bg-white/[0.04] px-1.5 py-0.5 text-[0.7rem] text-(--color-oa-ink-dim) transition hover:border-white/20 hover:text-(--color-oa-ink)"
          onClick={(e) => {
            e.currentTarget.blur();
            if (isPaused()) {
              void resumeJob(j().id);
            } else {
              void pauseJob(j().id);
            }
          }}
          title={
            isPaused()
              ? "Resume this job"
              : "Pause this job (Phase 1: stops streaming but row state stays running until Phase 3)"
          }
          aria-label={isPaused() ? "Resume job" : "Pause job"}
        >
          {isPaused() ? "▶" : "⏸"}
        </button>
        <button
          type="button"
          class="shrink-0 rounded border border-rose-500/20 bg-rose-500/10 px-1.5 py-0.5 text-[0.7rem] text-rose-200 transition hover:border-rose-400/50 hover:bg-rose-500/20"
          onClick={(e) => {
            e.currentTarget.blur();
            props.onSingleCancel(j());
          }}
          title="Cancel this job"
          aria-label="Cancel job"
        >
          ✕
        </button>
      </div>
      {/* Progress bar — determinate when total is known, otherwise
          a thin indeterminate stripe (rare in Phase 1 since
          core_download always has Content-Length). */}
      <div class="mt-1.5 h-1 w-full overflow-hidden rounded-full bg-white/[0.06]">
        <Show
          when={pct() !== null}
          fallback={
            <div class="h-full w-1/3 animate-pulse rounded-full bg-(--color-system-accent)/60" />
          }
        >
          <div
            class="h-full rounded-full bg-(--color-system-accent) transition-[width] duration-150 ease-out"
            style={{ width: `${pct() ?? 0}%` }}
          />
        </Show>
      </div>
    </div>
  );
};

export default BackgroundJobsBar;
