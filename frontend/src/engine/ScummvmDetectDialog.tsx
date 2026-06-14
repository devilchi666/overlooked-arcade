import {
  createEffect,
  createSignal,
  For,
  Show,
  type Component,
} from "solid-js";
import {
  findScummvmCli,
  detectScummvmDirectories,
  runScummvmCliDetect,
  writeScummvmDescriptors,
} from "@oa/platform/api/shellApi";
import { open as pickDirectory } from "@tauri-apps/plugin-dialog";
import { Dialog } from "@oa/platform/components/Dialog";

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

/// One row from the ScummVM CLI's `--detect` output. Mirrors the Rust
/// `scummvm_cli::CliDetectionRow` struct (camelCase via serde).
type CliDetectionRow = {
  gameId: string;
  description: string;
  directory: string;
};

/// Detection-source toggle in the dialog. "table" uses the curated
/// built-in sentinel-filename table; "cli" shells out to a standalone
/// ScummVM install for comprehensive detection across the engine's
/// full ~400-game catalog.
type DetectionMode = "table" | "cli";

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

  // Detection-mode toggle. Defaults to the built-in table (zero deps,
  // covers ~18 well-known games). Power users with a standalone
  // ScummVM install can flip to CLI mode for full-catalog detection.
  const [mode, setMode] = createSignal<DetectionMode>("table");
  // ScummVM CLI executable path — auto-discovered on dialog open via
  // `find_scummvm_cli`, operator can override.
  const [cliPath, setCliPath] = createSignal<string | null>(null);
  // `null` = haven't attempted discovery yet; `false` = discovered
  // nothing (CLI mode shows the "ScummVM not found" hint with a
  // manual path-picker button). Distinct from `cliPath` being null
  // because we want to render the hint only after the lookup ran.
  const [cliLookupDone, setCliLookupDone] = createSignal(false);

  // Auto-scan when the dialog opens with an initial folder. Operators
  // who opened it from the Wizard already picked a folder; they
  // shouldn't have to re-pick.
  createEffect(() => {
    if (!props.open) {
      setResults([]);
      setRowState(new Map());
      setError(null);
      setLastWriteSummary(null);
      setMode("table");
      // Don't reset cliPath / cliLookupDone — caches across dialog
      // opens within the same session.
      return;
    }
    // First-time-this-session: try to locate the standalone ScummVM
    // CLI so the mode toggle can show "Found at <path>" immediately
    // (and operators in CLI mode don't have to discover-then-pick).
    if (!cliLookupDone()) {
      void (async () => {
        try {
          const path = await findScummvmCli();
          if (path) setCliPath(path);
        } catch (e) {
          console.warn("[oa-scummvm-detect] find_scummvm_cli failed:", e);
        } finally {
          setCliLookupDone(true);
        }
      })();
    }
    const init = props.initialFolder ?? null;
    if (init && folder() !== init) {
      setFolder(init);
      void runScan(init);
    }
  });

  async function pickCliPath() {
    try {
      const picked = await pickDirectory({
        directory: false,
        multiple: false,
        title: "Locate scummvm executable",
      });
      if (typeof picked === "string") {
        setCliPath(picked);
      }
    } catch (e) {
      console.warn("[oa-scummvm-detect] pick CLI path failed:", e);
    }
  }

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
      // Step 1 — always walk the parent for subdirs via the built-in
      // table detector. This gives us the canonical "what subdirs
      // exist" list + their existing-descriptor flags. In table mode
      // it's the ONLY pass; in CLI mode we layer CLI matches on top.
      let rows = await detectScummvmDirectories<DetectionResult>(parentDir);

      // Step 2 — in CLI mode, layer in matches from a standalone
      // ScummVM install. CLI's gameid-based detection covers the
      // long tail the curated table doesn't know about. For each
      // subdir the CLI matched, replace the table's `matched`
      // (which may have been None) with the CLI's gameid + label.
      if (mode() === "cli") {
        const exe = cliPath();
        if (!exe) {
          setError(
            "ScummVM CLI not configured — pick the scummvm executable above, or switch to the built-in table.",
          );
          setScanning(false);
          return;
        }
        try {
          const cliRows = await runScummvmCliDetect<CliDetectionRow>(exe, parentDir);
          rows = mergeCliRows(rows, cliRows);
        } catch (e) {
          setError(`ScummVM CLI failed: ${e}`);
          setScanning(false);
          return;
        }
      }

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

  /// Merge ScummVM CLI matches into the table walker's results.
  /// CLI rows are keyed by absolute directory; for each subdir the
  /// CLI matched, we set `matched` to the CLI's gameid + label
  /// (overrides any prior curated-table match). CLI gameid alone is
  /// the descriptor body — ScummVM core's internal detection maps
  /// it to the engine.
  ///
  /// Path comparison is case-insensitive + normalizes trailing
  /// separators to handle the Windows / Unix split between OS path
  /// shapes the CLI vs the OA walker emit.
  function mergeCliRows(
    tableRows: DetectionResult[],
    cliRows: CliDetectionRow[],
  ): DetectionResult[] {
    const cliByDir = new Map<string, CliDetectionRow>();
    for (const cli of cliRows) {
      cliByDir.set(normalizeDirKey(cli.directory), cli);
    }
    return tableRows.map((row) => {
      const cli = cliByDir.get(normalizeDirKey(row.directory));
      if (!cli) return row;
      return {
        ...row,
        matched: {
          descriptor: cli.gameId,
          label: cli.description,
          notes: "Detected via standalone ScummVM CLI",
        },
      };
    });
  }

  function normalizeDirKey(p: string): string {
    return p.replace(/[\\/]+$/, "").toLowerCase();
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

  /// Validate a ScummVM descriptor — accepts either bare gameid
  /// (`monkey`) or explicit gameid:engine (`monkey:scumm`).
  /// ScummVM's libretro core's internal detection maps bare gameids
  /// to engines; the explicit form is the override variant.
  /// CLI-detected rows write the bare form (the CLI doesn't report
  /// engine separately); curated-table rows write the explicit form
  /// since we know both. Both validate.
  function descriptorIsValid(descriptor: string): boolean {
    const trimmed = descriptor.trim();
    if (!trimmed) return false;
    return /^[a-z0-9_-]+(:[a-z0-9_-]+)?$/i.test(trimmed);
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
      const written = await writeScummvmDescriptors(writes);
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
      navigate
    >
      <div class="flex flex-col gap-4">
        {/* Detection mode toggle — built-in curated table (default,
            no deps) vs standalone ScummVM CLI (power user, full
            ~400-game catalog). */}
        <fieldset class="rounded-md border border-white/10 bg-white/[0.03] p-3">
          <legend class="px-2 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            Detection source
          </legend>
          <div class="flex flex-col gap-2">
            <label
              data-setting-row
              tabindex="-1"
              class="flex cursor-pointer items-start gap-2 text-xs text-(--color-oa-ink)"
            >
              <input
                type="radio"
                name="scummvm-detect-mode"
                checked={mode() === "table"}
                onChange={() => {
                  setMode("table");
                  if (folder()) void runScan(folder()!);
                }}
                class="mt-0.5 accent-(--color-system-accent)"
              />
              <span class="flex-1">
                <span class="font-medium">Built-in table</span>
                <span class="ml-2 text-(--color-oa-ink-dim)">
                  ~18 well-known SCUMM titles + freewares · no external dep · fastest
                </span>
              </span>
            </label>
            <label
              data-setting-row
              tabindex="-1"
              class="flex cursor-pointer items-start gap-2 text-xs text-(--color-oa-ink)"
            >
              <input
                type="radio"
                name="scummvm-detect-mode"
                checked={mode() === "cli"}
                onChange={() => {
                  setMode("cli");
                  if (folder() && cliPath()) void runScan(folder()!);
                }}
                class="mt-0.5 accent-(--color-system-accent)"
              />
              <span class="flex-1">
                <span class="font-medium">Standalone ScummVM CLI</span>
                <span class="ml-2 text-(--color-oa-ink-dim)">
                  power user · full ~400-game catalog via the standalone install
                </span>
              </span>
            </label>
            {/* CLI path picker row — shown only in CLI mode. Pre-fills
                from `find_scummvm_cli` if a standard install was
                located; operator can override with a custom path. */}
            <Show when={mode() === "cli"}>
              <div class="mt-1 ml-6 flex items-center gap-2 rounded-md border border-white/5 bg-black/20 px-2 py-1.5">
                <div class="min-w-0 flex-1">
                  <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                    ScummVM executable
                  </p>
                  <p class="truncate text-xs font-mono text-(--color-oa-ink)">
                    <Show
                      when={cliPath()}
                      fallback={
                        <span class="text-amber-200">
                          {cliLookupDone()
                            ? "Not found in standard install paths — pick the scummvm executable to use CLI mode."
                            : "Searching…"}
                        </span>
                      }
                    >
                      {(p) => <>{p()}</>}
                    </Show>
                  </p>
                </div>
                <button
                  type="button"
                  data-setting-action
                  onClick={() => void pickCliPath()}
                  class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-xs text-(--color-oa-ink) hover:bg-white/[0.08]"
                >
                  {cliPath() ? "Change" : "Pick"}
                </button>
              </div>
            </Show>
          </div>
        </fieldset>

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
            data-setting-action
            onClick={() => void pickFolder()}
            disabled={scanning() || writing()}
            class="rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-xs uppercase tracking-wider text-(--color-oa-ink) hover:bg-white/[0.08] disabled:opacity-50"
          >
            {folder() ? "Pick another" : "Pick folder"}
          </button>
          <Show when={folder()}>
            <button
              type="button"
              data-setting-action
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
                      <div
                        data-setting-row
                        tabindex="-1"
                        class="flex items-center gap-2"
                      >
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
                          <label
                            data-setting-row
                            tabindex="-1"
                            class="flex items-center gap-1 whitespace-nowrap text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)"
                          >
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
              data-setting-action
              onClick={() => props.onClose()}
              disabled={writing()}
              class="rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-xs uppercase tracking-wider text-(--color-oa-ink-dim) hover:text-(--color-oa-ink)"
            >
              Close
            </button>
            <button
              type="button"
              data-setting-action
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
