// Help → Debug log…
//
// Live view of the Rust log ring. Polls `get_recent_logs` at 1 Hz while
// open; the ring is shared between Rust + frontend (the latter via
// `logbridge.ts`) so this is the single timeline of everything the app
// emits.
//
// Designed for me (Claude) to use when investigating problems — the
// "Copy log path" button gives the user a path they can paste back
// into a chat so I can `Read` the on-disk session log directly.

import { createEffect, createMemo, createSignal, For, on, onCleanup, Show, type Component } from "solid-js";
import { getRecentLogs, getLogFilePath, revealLogsFolder } from "@oa/platform/api/shellApi";
import { Dialog } from "@oa/platform/components/Dialog";

type LogLevel = "error" | "warn" | "info" | "debug" | "trace";

type LogEntry = {
  timestampMs: number;
  level: LogLevel;
  target: string;
  message: string;
};

type Props = {
  open: boolean;
  onClose: () => void;
};

const POLL_INTERVAL_MS = 1000;
const ALL_LEVELS: readonly LogLevel[] = ["error", "warn", "info", "debug", "trace"];

function formatTimestamp(ms: number): string {
  if (!Number.isFinite(ms) || ms <= 0) return "—";
  try {
    const d = new Date(ms);
    const hh = String(d.getHours()).padStart(2, "0");
    const mm = String(d.getMinutes()).padStart(2, "0");
    const ss = String(d.getSeconds()).padStart(2, "0");
    const mmm = String(d.getMilliseconds()).padStart(3, "0");
    return `${hh}:${mm}:${ss}.${mmm}`;
  } catch {
    return "—";
  }
}

const LEVEL_TEXT: Record<LogLevel, string> = {
  error: "text-red-300",
  warn: "text-amber-300",
  info: "text-(--color-oa-ink)",
  debug: "text-(--color-oa-ink-dim)",
  trace: "text-(--color-oa-ink-dim)/70",
};

export const DebugLogDialog: Component<Props> = (props) => {
  const [entries, setEntries] = createSignal<LogEntry[]>([]);
  const [filter, setFilter] = createSignal<LogLevel | "all">("all");
  const [search, setSearch] = createSignal("");
  const [autoScroll, setAutoScroll] = createSignal(true);
  const [logFilePath, setLogFilePath] = createSignal<string | null>(null);
  const [copied, setCopied] = createSignal(false);

  let pollId: number | undefined;
  let scrollAnchor: HTMLDivElement | undefined;

  function startPoll() {
    if (pollId !== undefined) return;
    const fetch = () => {
      void getRecentLogs<LogEntry>(2000)
        .then((rows) => setEntries(rows ?? []))
        .catch(() => {});
    };
    fetch();
    pollId = window.setInterval(fetch, POLL_INTERVAL_MS);
  }
  function stopPoll() {
    if (pollId !== undefined) {
      window.clearInterval(pollId);
      pollId = undefined;
    }
  }

  // Start/stop polling and grab the path on open.
  createEffect(() => {
    if (props.open) {
      startPoll();
      void getLogFilePath()
        .then((p) => setLogFilePath(p ?? null))
        .catch(() => setLogFilePath(null));
    } else {
      stopPoll();
      setCopied(false);
    }
  });

  onCleanup(() => stopPoll());

  // Scroll to bottom when entries change AND auto-scroll is on. Using
  // `on(...)` so we only react to actual entry changes, not other
  // dependencies the createEffect might pick up.
  createEffect(
    on(entries, () => {
      if (autoScroll() && scrollAnchor) {
        scrollAnchor.scrollIntoView({ behavior: "auto", block: "end" });
      }
    }),
  );

  // Filtered + searched entries. `search` matches against target +
  // message, case-insensitive. `filter` keeps only the selected level
  // (or all when "all").
  const filtered = createMemo(() => {
    const f = filter();
    const q = search().trim().toLowerCase();
    return entries().filter((e) => {
      if (f !== "all" && e.level !== f) return false;
      if (q.length > 0) {
        const hay = `${e.target} ${e.message}`.toLowerCase();
        if (!hay.includes(q)) return false;
      }
      return true;
    });
  });

  async function copyPath() {
    const p = logFilePath();
    if (!p) return;
    try {
      await navigator.clipboard.writeText(p);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1500);
    } catch {
      /* Clipboard not granted; the path is still visible in the field. */
    }
  }

  async function openFolder() {
    try {
      await revealLogsFolder();
    } catch (e) {
      console.warn("[oa-debug-log] reveal failed:", e);
    }
  }

  return (
    <Dialog open={props.open} onClose={props.onClose} title="Debug log" size="lg" navigate>
      {/* Header controls — filter chips + search + auto-scroll toggle */}
      <div class="mb-3 flex flex-wrap items-center gap-2">
        <div class="flex items-center gap-1">
          <button
            type="button"
            data-setting-action
            onClick={() => setFilter("all")}
            class="rounded px-2 py-0.5 text-[0.65rem] uppercase tracking-widest transition"
            classList={{
              "bg-white/[0.08] text-(--color-oa-ink)": filter() === "all",
              "text-(--color-oa-ink-dim) hover:text-(--color-oa-ink)": filter() !== "all",
            }}
          >
            All
          </button>
          <For each={ALL_LEVELS}>
            {(lvl) => (
              <button
                type="button"
                data-setting-action
                onClick={() => setFilter(lvl)}
                class="rounded px-2 py-0.5 text-[0.65rem] uppercase tracking-widest transition"
                classList={{
                  "bg-white/[0.08] text-(--color-oa-ink)": filter() === lvl,
                  "text-(--color-oa-ink-dim) hover:text-(--color-oa-ink)": filter() !== lvl,
                }}
              >
                {lvl}
              </button>
            )}
          </For>
        </div>
        <input
          type="search"
          placeholder="Filter…"
          value={search()}
          onInput={(e) => setSearch(e.currentTarget.value)}
          class="flex-1 min-w-[10rem] rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-xs text-(--color-oa-ink) focus-visible:border-(--color-system-accent) focus-visible:outline-none"
        />
        <label
          data-setting-row
          tabindex="-1"
          class="flex cursor-pointer items-center gap-1.5 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)"
        >
          <input
            type="checkbox"
            checked={autoScroll()}
            onChange={(e) => setAutoScroll(e.currentTarget.checked)}
            class="size-3 accent-(--color-system-accent)"
          />
          Auto-scroll
        </label>
      </div>

      {/* Log body — monospace, level-colored */}
      <div class="overflow-auto rounded-md border border-white/10 bg-black/40 font-mono text-[0.7rem] leading-[1.5]"
           style={{ "max-height": "55vh" }}>
        <Show
          when={filtered().length > 0}
          fallback={
            <div class="px-3 py-6 text-center text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              {entries().length === 0
                ? "No log entries yet — bridge ready, waiting for events."
                : "No entries match the current filter."}
            </div>
          }
        >
          <For each={filtered()}>
            {(e) => (
              <div class="flex gap-2 px-3 py-0.5 hover:bg-white/[0.03]">
                <span class="shrink-0 text-(--color-oa-ink-dim)">
                  {formatTimestamp(e.timestampMs)}
                </span>
                <span
                  class="shrink-0 w-12 uppercase"
                  classList={{ [LEVEL_TEXT[e.level]]: true }}
                >
                  {e.level}
                </span>
                <span class="shrink-0 text-(--color-oa-ink-dim) max-w-[14rem] truncate">
                  {e.target || "—"}
                </span>
                <span
                  class="min-w-0 whitespace-pre-wrap break-words"
                  classList={{ [LEVEL_TEXT[e.level]]: true }}
                >
                  {e.message}
                </span>
              </div>
            )}
          </For>
          <div ref={scrollAnchor} aria-hidden="true" />
        </Show>
      </div>

      {/* Footer — log file path + actions */}
      <div class="mt-3 flex flex-wrap items-center justify-between gap-2">
        <p class="min-w-0 flex-1 truncate text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
          <Show when={logFilePath()} fallback="No log file yet">
            File: <code class="font-mono normal-case text-(--color-oa-ink)">{logFilePath()}</code>
          </Show>
        </p>
        <div class="flex shrink-0 items-center gap-2">
          <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            {filtered().length} / {entries().length} entries
          </span>
          <button
            type="button"
            data-setting-action
            onClick={copyPath}
            disabled={!logFilePath()}
            class="rounded-md border border-white/10 bg-white/[0.04] px-3 py-1 text-xs uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink) disabled:cursor-not-allowed disabled:opacity-40"
          >
            {copied() ? "Copied!" : "Copy path"}
          </button>
          <button
            type="button"
            data-setting-action
            onClick={openFolder}
            class="rounded-md border border-white/10 bg-white/[0.04] px-3 py-1 text-xs uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
          >
            Open folder
          </button>
        </div>
      </div>
    </Dialog>
  );
};
