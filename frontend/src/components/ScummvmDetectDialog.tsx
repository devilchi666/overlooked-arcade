import {
  createEffect,
  createSignal,
  For,
  Show,
  type Component,
} from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { open as pickDirectory } from "@tauri-apps/plugin-dialog";
import { Dialog } from "../layout/Dialog";

/// One row in the detection report returned by the
/// `detect_scummvm_directories` Tauri command. Mirrors the Rust
/// `scummvm_detect::DetectionResult` struct (camelCase via serde).
type DetectionResult = {
  directory: string;
  directoryName: string;
  matched: MatchedGame | null;
  descriptorPath: string;
  alreadyExists: boolean;
};

type MatchedGame = {
  descriptor: string;
  label: string;
  notes: string;
};

/// Per-row operator overrides. Keyed by `directory` (absolute path).
/// Captures the descriptor text the operator chose (auto-populated
/// from `matched.descriptor` when detection succeeded, but the
/// operator can edit it) and whether the row is included in the
/// write batch.
type RowState = {
  descriptor: string;
  include: boolean;
  overwrite: boolean;
};

type Props = {
  open: boolean;
  onClose: () => void;
  /// Initial folder to scan. When provided (e.g. from the Import
  /// Wizard), the dialog skips the folder picker and scans on open.
  initialFolder?: string | null;
};

export const ScummvmDetectDialog: Component<Props> = (props) => {
  const [folder, setFolder] = createSignal<string | null>(null);
  const [results, setResults] = createSignal<DetectionResult[]>([]);
  const [rowState, setRowState] = createSignal<Map<string, RowState>>(new Map());
  const [scanning, setScanning] = createSignal(false);
  const [writing, setWriting] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [lastWriteSummary, setLastWriteSummary] = createSignal<string | null>(null);

  // Auto-scan when the dialog opens with an initial folder. Operators
  // who opened it from the Wizard already picked a folder; they
  // shouldn't have to re-pick.
  createEffect(() => {
    if (!props.open) {
      setResults([]);
      setRowState(new Map());
      setError(null);
      setLastWriteSummary(null);
      return;
    }
    const init = props.initialFolder ?? null;
    if (init && folder() !== init) {
      setFolder(init);
      void runScan(init);
    }
  });

  async function pickFolder() {
    try {
      const picked = await pickDirectory({ directory: true, multiple: false });
      if (typeof picked === "string") {
        setFolder(picked);
        await runScan(picked);
      }
    } catch (e) {
      console.warn("[oa-scummvm-detect] pick failed:", e);
    }
  }

  async function runScan(parentDir: string) {
    setScanning(true);
    setError(null);
    setResults([]);
    setRowState(new Map());
    setLastWriteSummary(null);
    try {
      const rows = await invoke<DetectionResult[]>("detect_scummvm_directories", {
        parentDir,
      });
      setResults(rows);
      // Seed per-row state — detected rows pre-fill the descriptor and
      // are included by default; un-detected rows have an empty
      // descriptor and are excluded by default (operator opts in by
      // filling in a descriptor manually). Existing-descriptor rows
      // are excluded by default to avoid clobbering operator-curated
      // files.
      const seed = new Map<string, RowState>();
      for (const r of rows) {
        seed.set(r.directory, {
          descriptor: r.matched?.descriptor ?? "",
          include: r.matched !== null && !r.alreadyExists,
          overwrite: false,
        });
      }
      setRowState(seed);
    } catch (e) {
      setError(String(e));
    } finally {
      setScanning(false);
    }
  }

  function setRow(directory: string, partial: Partial<RowState>) {
    setRowState((prev) => {
      const next = new Map(prev);
      const current = next.get(directory) ?? {
        descriptor: "",
        include: false,
        overwrite: false,
      };
      next.set(directory, { ...current, ...partial });
      return next;
    });
  }

  function includedCount(): number {
    let n = 0;
    for (const [, s] of rowState()) {
      if (s.include && s.descriptor.trim().length > 0) n++;
    }
    return n;
  }

  /// Validate a `gameid:engineid` descriptor — minimal shape check
  /// (non-empty, exactly one `:`, both halves non-empty, ASCII).
  /// ScummVM's own detector will reject mistyped ids at game-load
  /// time; this just stops obvious typos at the dialog level.
  function descriptorIsValid(descriptor: string): boolean {
    const trimmed = descriptor.trim();
    if (!trimmed) return false;
    const m = trimmed.match(/^([a-z0-9_-]+):([a-z0-9_-]+)$/i);
    return m !== null;
  }

  async function writeBatch() {
    setWriting(true);
    setError(null);
    setLastWriteSummary(null);
    try {
      const writes: Array<{ path: string; descriptor: string; overwrite: boolean }> = [];
      for (const r of results()) {
        const s = rowState().get(r.directory);
        if (!s || !s.include) continue;
        const desc = s.descriptor.trim();
        if (!descriptorIsValid(desc)) continue;
        writes.push({
          path: r.descriptorPath,
          descriptor: desc,
          overwrite: s.overwrite,
        });
      }
      if (writes.length === 0) {
        setError("Nothing to write — include at least one row with a valid descriptor.");
        setWriting(false);
        return;
      }
      const written = await invoke<number>("write_scummvm_descriptors", { writes });
      setLastWriteSummary(
        `Wrote ${written} of ${writes.length} descriptor${writes.length === 1 ? "" : "s"}. Re-scan your library to see the new games.`,
      );
      // Refresh the scan to update `alreadyExists` flags + reset
      // include defaults.
      const f = folder();
      if (f) await runScan(f);
    } catch (e) {
      setError(String(e));
    } finally {
      setWriting(false);
    }
  }

  return (
    <Dialog
      open={props.open}
      onClose={props.onClose}
      title="Detect ScummVM games"
      subtitle="Walks a folder for game subdirectories and auto-generates .scummvm descriptor files for known titles."
      size="xl"
      system="scummvm"
    >
      <div class="flex flex-col gap-4">
        {/* Folder picker row — always visible so operator can re-scan
            a different parent without closing the dialog. */}
        <div class="flex items-center gap-2 rounded-md border border-white/10 bg-white/[0.03] p-3">
          <div class="min-w-0 flex-1">
            <p class="text-xs uppercase tracking-widest text-(--color-oa-ink-dim)">
              Parent folder
            </p>
            <p class="truncate text-sm text-(--color-oa-ink)">
              {folder() ?? "Pick a folder containing your ScummVM game subdirectories"}
            </p>
          </div>
          <button
            type="button"
            onClick={() => void pickFolder()}
            disabled={scanning() || writing()}
            class="rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-xs uppercase tracking-wider text-(--color-oa-ink) hover:bg-white/[0.08] disabled:opacity-50"
          >
            {folder() ? "Pick another" : "Pick folder"}
          </button>
          <Show when={folder()}>
            <button
              type="button"
              onClick={() => void runScan(folder()!)}
              disabled={scanning() || writing()}
              class="rounded-md border border-(--color-system-accent)/40 bg-(--color-system-accent)/15 px-3 py-1.5 text-xs uppercase tracking-wider text-(--color-system-accent-soft) hover:bg-(--color-system-accent)/25 disabled:opacity-50"
            >
              {scanning() ? "Scanning…" : "Re-scan"}
            </button>
          </Show>
        </div>

        <Show when={error()}>
          <div class="rounded-md border border-red-500/30 bg-red-500/10 p-3 text-xs text-red-200">
            {error()}
          </div>
        </Show>

        <Show when={lastWriteSummary()}>
          <div class="rounded-md border border-green-500/30 bg-green-500/10 p-3 text-xs text-green-200">
            {lastWriteSummary()}
          </div>
        </Show>

        <Show when={!scanning() && results().length === 0 && folder()}>
          <div class="rounded-md border border-white/5 bg-white/[0.02] p-4 text-xs text-(--color-oa-ink-dim)">
            No subdirectories found in this folder. ScummVM expects each
            game to live in its own top-level subdirectory of the parent
            folder you pick — e.g.{" "}
            <code class="font-mono">
              {"<library>"}/ScummVM/Monkey Island/MONKEY.000…
            </code>
            .
          </div>
        </Show>

        <Show when={results().length > 0}>
          <div class="flex flex-col gap-2">
            <p class="text-xs uppercase tracking-widest text-(--color-oa-ink-dim)">
              {results().length} subdirector{results().length === 1 ? "y" : "ies"} ·{" "}
              {results().filter((r) => r.matched).length} auto-detected ·{" "}
              {results().filter((r) => r.alreadyExists).length} already have a descriptor
            </p>
            <div class="flex max-h-96 flex-col gap-2 overflow-y-auto">
              <For each={results()}>
                {(r) => {
                  const state = () => rowState().get(r.directory);
                  return (
                    <div class="flex flex-col gap-2 rounded-md border border-white/10 bg-white/[0.03] p-3">
                      <div class="flex items-center gap-2">
                        <input
                          type="checkbox"
                          checked={state()?.include ?? false}
                          onChange={(e) =>
                            setRow(r.directory, { include: e.currentTarget.checked })
                          }
                          title={state()?.include ? "Included in write batch" : "Excluded"}
                        />
                        <div class="min-w-0 flex-1">
                          <p class="truncate text-sm text-(--color-oa-ink)">
                            {r.directoryName}
                            <Show when={r.alreadyExists}>
                              <span class="ml-2 rounded bg-amber-500/20 px-1.5 py-0.5 text-[0.55rem] uppercase tracking-widest text-amber-200">
                                .scummvm exists
                              </span>
                            </Show>
                          </p>
                          <Show
                            when={r.matched}
                            fallback={
                              <p class="text-[0.65rem] text-(--color-oa-ink-dim)">
                                Not recognized — fill in{" "}
                                <code class="font-mono">gameid:engineid</code> manually
                                if you know it.
                              </p>
                            }
                          >
                            {(m) => (
                              <p class="text-[0.65rem] text-(--color-oa-ink-dim)">
                                {m().label} · {m().notes}
                              </p>
                            )}
                          </Show>
                        </div>
                      </div>
                      <div class="flex items-center gap-2">
                        <input
                          type="text"
                          value={state()?.descriptor ?? ""}
                          onInput={(e) =>
                            setRow(r.directory, { descriptor: e.currentTarget.value })
                          }
                          placeholder="gameid:engineid (e.g. monkey:scumm)"
                          class="flex-1 rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-sm font-mono text-(--color-oa-ink)"
                        />
                        <Show when={r.alreadyExists}>
                          <label class="flex items-center gap-1 whitespace-nowrap text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                            <input
                              type="checkbox"
                              checked={state()?.overwrite ?? false}
                              onChange={(e) =>
                                setRow(r.directory, { overwrite: e.currentTarget.checked })
                              }
                            />
                            Overwrite
                          </label>
                        </Show>
                      </div>
                      <Show
                        when={
                          (state()?.descriptor ?? "").trim().length > 0 &&
                          !descriptorIsValid(state()?.descriptor ?? "")
                        }
                      >
                        <p class="text-[0.65rem] text-amber-200">
                          Descriptor must be in the form{" "}
                          <code class="font-mono">gameid:engineid</code>.
                        </p>
                      </Show>
                    </div>
                  );
                }}
              </For>
            </div>
          </div>
        </Show>

        <div class="flex items-center justify-between border-t border-white/10 pt-3">
          <p class="text-xs text-(--color-oa-ink-dim)">
            {includedCount()} descriptor{includedCount() === 1 ? "" : "s"} queued
          </p>
          <div class="flex gap-2">
            <button
              type="button"
              onClick={() => props.onClose()}
              disabled={writing()}
              class="rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-xs uppercase tracking-wider text-(--color-oa-ink-dim) hover:text-(--color-oa-ink)"
            >
              Close
            </button>
            <button
              type="button"
              onClick={() => void writeBatch()}
              disabled={writing() || scanning() || includedCount() === 0}
              class="rounded-md border border-(--color-system-accent)/40 bg-(--color-system-accent)/15 px-3 py-1.5 text-xs uppercase tracking-wider text-(--color-system-accent-soft) hover:bg-(--color-system-accent)/25 disabled:opacity-50"
            >
              {writing() ? "Writing…" : `Write ${includedCount()} descriptor${includedCount() === 1 ? "" : "s"}`}
            </button>
          </div>
        </div>
      </div>
    </Dialog>
  );
};

export default ScummvmDetectDialog;
