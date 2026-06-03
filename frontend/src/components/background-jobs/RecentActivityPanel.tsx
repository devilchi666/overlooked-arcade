// Recent activity panel — Phase 5 of the background-jobs arc.
//
// Full-viewport overlay surfacing the last 100 finished jobs in a
// tabbed view (Active / Completed / Failed / Cancelled). Triggered
// from the BackgroundJobsBar's header (the "Recent activity" link
// when the bar is expanded). Plan §"Recent activity panel."
//
// The Active tab reads from the live activeJobs() store; the three
// finished tabs invoke list_recent_jobs once on mount + when the
// operator clicks the tab again. No live updates on finished tabs
// — they're a snapshot of the rolling buffer.

import {
  createMemo,
  createSignal,
  For,
  onCleanup,
  Show,
  type Component,
} from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { activeJobs, type JobSnapshot, type JobState } from "../../lib/backgroundJobs";

type Tab = "active" | "completed" | "failed" | "cancelled";

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
  test_job: "●",
};

function kindGlyph(kind: string): string {
  return KIND_GLYPH[kind] ?? "•";
}

function stateGlyph(state: JobState): string {
  switch (state) {
    case "completed":
      return "✓";
    case "failed":
      return "✗";
    case "cancelled":
      return "—";
    case "interrupted":
      return "⏸";
    case "paused":
      return "⏸";
    case "running":
      return "●";
    case "pending":
      return "○";
  }
}

function stateColorClass(state: JobState): string {
  switch (state) {
    case "completed":
      return "text-emerald-300";
    case "failed":
      return "text-rose-300";
    case "cancelled":
      return "text-(--color-oa-ink-dim)";
    case "interrupted":
      return "text-amber-300";
    default:
      return "text-(--color-oa-ink-dim)";
  }
}

function formatDuration(startedAt: number, finishedAt: number | null): string {
  if (finishedAt === null) return "—";
  const ms = finishedAt - startedAt;
  if (ms < 1000) return `${ms} ms`;
  const s = Math.round(ms / 100) / 10;
  if (s < 60) return `${s} s`;
  const m = Math.floor(s / 60);
  const rs = Math.round(s % 60);
  return `${m}m ${rs}s`;
}

function formatTimestamp(ms: number): string {
  const d = new Date(ms);
  return d.toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });
}

export const RecentActivityPanel: Component<{
  open: boolean;
  onClose: () => void;
}> = (props) => {
  const [tab, setTab] = createSignal<Tab>("active");
  const [completed, setCompleted] = createSignal<JobSnapshot[]>([]);
  const [failed, setFailed] = createSignal<JobSnapshot[]>([]);
  const [cancelled, setCancelled] = createSignal<JobSnapshot[]>([]);
  const [loading, setLoading] = createSignal(false);

  const refresh = async () => {
    setLoading(true);
    try {
      const all = await invoke<JobSnapshot[]>("list_recent_jobs", {
        limit: 100,
      });
      setCompleted(all.filter((j) => j.state === "completed"));
      setFailed(all.filter((j) => j.state === "failed"));
      setCancelled(all.filter((j) => j.state === "cancelled"));
    } catch (e) {
      console.warn("[bg-jobs-recent] list_recent_jobs failed:", e);
    } finally {
      setLoading(false);
    }
  };

  // Refresh on every open transition.
  let prevOpen = false;
  const refreshIfNewlyOpen = () => {
    const isOpen = props.open;
    if (isOpen && !prevOpen) void refresh();
    prevOpen = isOpen;
  };
  refreshIfNewlyOpen();

  // Escape closes.
  const onKeyDown = (e: KeyboardEvent) => {
    if (props.open && e.key === "Escape") props.onClose();
  };
  window.addEventListener("keydown", onKeyDown);
  onCleanup(() => window.removeEventListener("keydown", onKeyDown));

  // Track open transitions via a memo (refresh on each open).
  createMemo(refreshIfNewlyOpen);

  const currentRows = createMemo<JobSnapshot[]>(() => {
    switch (tab()) {
      case "active":
        return activeJobs();
      case "completed":
        return completed();
      case "failed":
        return failed();
      case "cancelled":
        return cancelled();
    }
  });

  return (
    <Show when={props.open}>
      <div
        class="fixed inset-0 z-[65] grid place-items-center bg-black/70 backdrop-blur-sm p-6"
        onClick={(e) => {
          if (e.currentTarget === e.target) props.onClose();
        }}
        role="dialog"
        aria-modal="true"
        aria-label="Recent background-jobs activity"
      >
        <div class="flex h-full max-h-[80vh] w-full max-w-3xl flex-col overflow-hidden rounded-xl border border-white/10 bg-(--color-oa-bg-deep) shadow-2xl">
          {/* Header */}
          <header class="flex items-center justify-between border-b border-white/5 px-6 py-4">
            <div>
              <h2 class="text-sm font-semibold uppercase tracking-[0.3em] text-(--color-oa-ink)">
                Recent activity
              </h2>
              <p class="mt-1 text-[0.7rem] text-(--color-oa-ink-dim)">
                Last 100 finished background jobs. Plus everything
                currently running.
              </p>
            </div>
            <button
              type="button"
              class="rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-xs text-(--color-oa-ink-dim) transition hover:border-white/20 hover:text-(--color-oa-ink)"
              onClick={(e) => {
                e.currentTarget.blur();
                props.onClose();
              }}
              aria-label="Close recent activity panel"
            >
              ✕
            </button>
          </header>

          {/* Tab strip */}
          <div class="flex gap-1 border-b border-white/5 bg-white/[0.02] px-4 py-2">
            <TabButton
              label="Running"
              count={activeJobs().length}
              active={tab() === "active"}
              onClick={() => setTab("active")}
            />
            <TabButton
              label="Completed"
              count={completed().length}
              active={tab() === "completed"}
              onClick={() => setTab("completed")}
            />
            <TabButton
              label="Failed"
              count={failed().length}
              active={tab() === "failed"}
              onClick={() => setTab("failed")}
            />
            <TabButton
              label="Cancelled"
              count={cancelled().length}
              active={tab() === "cancelled"}
              onClick={() => setTab("cancelled")}
            />
            <div class="flex-1" />
            <button
              type="button"
              class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim) transition hover:text-(--color-oa-ink)"
              onClick={(e) => {
                e.currentTarget.blur();
                void refresh();
              }}
              disabled={loading()}
            >
              {loading() ? "…" : "Refresh"}
            </button>
          </div>

          {/* Row list */}
          <div class="min-h-0 flex-1 overflow-y-auto px-4 py-3">
            <Show
              when={currentRows().length > 0}
              fallback={
                <p class="py-8 text-center text-[0.75rem] text-(--color-oa-ink-dim)">
                  No {tab() === "active" ? "active" : tab()} jobs.
                </p>
              }
            >
              <ul class="space-y-1.5">
                <For each={currentRows()}>
                  {(job) => <RowEntry job={job} />}
                </For>
              </ul>
            </Show>
          </div>
        </div>
      </div>
    </Show>
  );
};

const TabButton: Component<{
  label: string;
  count: number;
  active: boolean;
  onClick: () => void;
}> = (props) => {
  return (
    <button
      type="button"
      class="rounded px-3 py-1 text-[0.7rem] font-semibold uppercase tracking-widest transition"
      classList={{
        "bg-(--color-system-accent)/15 text-(--color-oa-ink)": props.active,
        "text-(--color-oa-ink-dim) hover:bg-white/[0.04] hover:text-(--color-oa-ink)":
          !props.active,
      }}
      onClick={(e) => {
        e.currentTarget.blur();
        props.onClick();
      }}
    >
      {props.label}
      <span class="ml-1.5 tabular-nums text-(--color-oa-ink-dim)/70">
        {props.count}
      </span>
    </button>
  );
};

const RowEntry: Component<{ job: JobSnapshot }> = (props) => {
  const j = () => props.job;
  return (
    <li class="grid grid-cols-[auto_1fr_auto_auto_auto] items-center gap-3 rounded-lg border border-white/10 bg-white/[0.02] px-3 py-2 text-[0.75rem]">
      <span
        class="inline-block w-5 text-center text-base leading-5 text-(--color-system-accent)"
        aria-hidden="true"
      >
        {kindGlyph(j().kind)}
      </span>
      <div class="min-w-0">
        <p class="truncate font-medium text-(--color-oa-ink)" title={j().label}>
          {j().label}
        </p>
        <Show when={j().errorMessage}>
          <p
            class="mt-0.5 truncate text-[0.65rem] text-rose-300/80"
            title={j().errorMessage ?? ""}
          >
            {j().errorMessage}
          </p>
        </Show>
      </div>
      <span class="shrink-0 text-[0.65rem] tabular-nums text-(--color-oa-ink-dim)">
        {formatDuration(j().startedAt, j().finishedAt)}
      </span>
      <span class="shrink-0 text-[0.65rem] text-(--color-oa-ink-dim)">
        {formatTimestamp(j().finishedAt ?? j().startedAt)}
      </span>
      <span
        class={`shrink-0 text-base ${stateColorClass(j().state)}`}
        aria-label={`State: ${j().state}`}
      >
        {stateGlyph(j().state)}
      </span>
    </li>
  );
};

export default RecentActivityPanel;
