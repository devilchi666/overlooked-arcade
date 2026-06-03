// ResumePromptDialog — Phase 3b's opt-out completion dialog,
// landed as a polish item.
//
// When a kind has `promptBeforeResumeOnLaunch: true` in JobPrefs, the
// resume_interrupted_jobs dispatcher SKIPS the kind's registered
// resumer and instead emits an oa://job-event ResumePrompt variant.
// The frontend store enqueues the snapshot into `resumePromptQueue`;
// this component surfaces a modal for the first row in the queue with
// Resume / Discard options.
//
// Plan §"Resume on app launch" — the dialog model. The Settings panel
// surface for flipping the per-kind toggles is in
// SettingsSections.tsx::BackgroundJobsSettings.

import {
  createMemo,
  createSignal,
  onCleanup,
  Show,
  type Component,
} from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import {
  cancelJob,
  dismissResumePrompt,
  resumePromptQueue,
} from "../../lib/backgroundJobs";

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

export const ResumePromptDialog: Component = () => {
  // Show the dialog for the FIRST snapshot in the queue. Subsequent
  // prompts surface one at a time as the operator dismisses each.
  const current = createMemo(() => resumePromptQueue()[0] ?? null);
  const [busy, setBusy] = createSignal(false);

  // Escape closes the dialog (treats as Discard — dismiss without
  // resuming the work).
  const onKeyDown = (e: KeyboardEvent) => {
    if (current() === null || busy()) return;
    if (e.key === "Escape") {
      // Just close without action — the row stays interrupted, the
      // operator can come back to it.
      const snap = current();
      if (snap) dismissResumePrompt(snap.id);
    }
  };
  window.addEventListener("keydown", onKeyDown);
  onCleanup(() => window.removeEventListener("keydown", onKeyDown));

  const handleResume = async () => {
    const snap = current();
    if (!snap || busy()) return;
    setBusy(true);
    try {
      await invoke<boolean>("resume_one_interrupted_job", { jobId: snap.id });
    } catch (e) {
      console.warn("[oa-jobs] resume_one_interrupted_job failed:", e);
    } finally {
      dismissResumePrompt(snap.id);
      setBusy(false);
    }
  };

  const handleDiscard = async () => {
    const snap = current();
    if (!snap || busy()) return;
    setBusy(true);
    try {
      await cancelJob(snap.id);
    } catch (e) {
      console.warn("[oa-jobs] cancel_job (discard) failed:", e);
    } finally {
      dismissResumePrompt(snap.id);
      setBusy(false);
    }
  };

  const handleClose = () => {
    const snap = current();
    if (snap && !busy()) dismissResumePrompt(snap.id);
  };

  return (
    <Show when={current() !== null}>
      <div
        class="fixed inset-0 z-[70] grid place-items-center bg-black/70 backdrop-blur-sm p-6"
        role="dialog"
        aria-modal="true"
        aria-labelledby="oa-resume-prompt-title"
        onClick={(e) => {
          if (e.currentTarget === e.target) handleClose();
        }}
      >
        <div class="flex w-full max-w-md flex-col overflow-hidden rounded-xl border border-amber-500/30 bg-(--color-oa-bg-deep) shadow-2xl">
          <header class="flex items-start gap-3 border-b border-white/5 px-5 py-4">
            <span
              class="mt-0.5 inline-block h-7 w-7 shrink-0 text-center text-xl leading-7 text-amber-300"
              aria-hidden="true"
            >
              {kindGlyph(current()?.kind ?? "")}
            </span>
            <div class="min-w-0 flex-1">
              <h2
                id="oa-resume-prompt-title"
                class="text-sm font-semibold uppercase tracking-[0.25em] text-(--color-oa-ink)"
              >
                Resume interrupted job?
              </h2>
              <p class="mt-1.5 truncate text-[0.85rem] text-(--color-oa-ink)">
                {current()?.label}
              </p>
              <p class="mt-1 text-[0.7rem] text-(--color-oa-ink-dim)">
                This kind has "Prompt before resuming" enabled in
                Settings → Background Jobs. The work was interrupted
                when OA last exited unexpectedly. Resume to pick up
                where it left off (or where the operation's internal
                idempotency picks it up); Discard to drop the row
                without resuming.
              </p>
            </div>
          </header>
          <div class="flex items-center justify-end gap-2 bg-white/[0.02] px-5 py-3">
            <button
              type="button"
              class="rounded-md border border-rose-500/30 bg-rose-500/10 px-3 py-1.5 text-xs font-semibold uppercase tracking-wider text-rose-200 transition hover:border-rose-400/60 hover:bg-rose-500/20 disabled:opacity-50"
              disabled={busy()}
              onClick={(e) => {
                e.currentTarget.blur();
                void handleDiscard();
              }}
            >
              Discard
            </button>
            <button
              type="button"
              class="rounded-md border border-(--color-system-accent)/50 bg-(--color-system-accent)/15 px-4 py-1.5 text-xs font-semibold uppercase tracking-wider text-(--color-oa-ink) transition hover:border-(--color-system-accent) hover:bg-(--color-system-accent)/25 disabled:opacity-50"
              disabled={busy()}
              onClick={(e) => {
                e.currentTarget.blur();
                void handleResume();
              }}
            >
              {busy() ? "…" : "Resume"}
            </button>
          </div>
        </div>
      </div>
    </Show>
  );
};

export default ResumePromptDialog;
