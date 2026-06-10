// Library Manager → Game media → "Import art pack from folder…"
//
// Operator points at a LaunchBox / EmuMovies art-pack folder; the
// dialog runs a dry-run analysis first (shows per-platform × per-kind
// counts), then on confirm runs the live import via the
// art_pack_importer Rust module. See
// docs/features/media-taxonomy/README.md Phase 3 for the design.

import {
  createSignal,
  For,
  Show,
  type Component,
} from "solid-js";
import { importArtPack } from "@oa/platform/api/mediaApi";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { Dialog } from "@oa/platform/components/Dialog";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";

type Props = {
  open: boolean;
  onClose: () => void;
  /// Caller's callback for when an import completed successfully —
  /// Library Manager triggers a media re-hydrate via the MediaProvider.
  onImported?: () => void;
};

type KindReport = {
  kind: string;
  sourceFiles: number;
  imported: number;
  skippedNoMatch: number;
};

type PlatformReport = {
  platformDir: string;
  systemId: string | null;
  launchboxName: string | null;
  libraryEntries: number;
  byKind: Record<string, KindReport>;
  totalImported: number;
  totalSkippedNoMatch: number;
  error: string | null;
};

type ImportReport = {
  layout: "single-platform" | "multi-platform" | "unknown";
  platforms: PlatformReport[];
  totalImported: number;
  totalSkippedNoMatch: number;
};

const ALL_SYSTEM_IDS = Object.keys(systemThemes) as SystemId[];

export const ImportArtPackDialog: Component<Props> = (props) => {
  const [sourceDir, setSourceDir] = createSignal<string>("");
  // Operator-supplied system_id, used when layout = single-platform.
  // null means "not picked yet" (dialog forces a pick before analyze).
  const [systemOverride, setSystemOverride] = createSignal<SystemId | "">("");
  const [report, setReport] = createSignal<ImportReport | null>(null);
  const [busy, setBusy] = createSignal<"analyzing" | "importing" | null>(null);
  const [errMsg, setErrMsg] = createSignal<string>("");

  function reset() {
    setSourceDir("");
    setSystemOverride("");
    setReport(null);
    setBusy(null);
    setErrMsg("");
  }

  async function pickFolder() {
    try {
      const chosen = await openDialog({
        directory: true,
        multiple: false,
        title: "Pick LaunchBox / EmuMovies art-pack folder",
      });
      if (typeof chosen === "string" && chosen.length > 0) {
        setSourceDir(chosen);
        setReport(null);
        setErrMsg("");
      }
    } catch (e) {
      console.warn("[oa-art-import] pick folder failed:", e);
    }
  }

  async function runDryRun() {
    const dir = sourceDir();
    if (!dir) {
      setErrMsg("Pick a source folder first.");
      return;
    }
    setBusy("analyzing");
    setErrMsg("");
    try {
      const r = await importArtPack({
        sourceDir: dir,
        systemIdOverride: systemOverride() || null,
        dryRun: true,
      });
      setReport(r);
    } catch (e) {
      console.warn("[oa-art-import] dry-run failed:", e);
      setErrMsg(String(e));
    } finally {
      setBusy(null);
    }
  }

  async function runImport() {
    const dir = sourceDir();
    if (!dir) return;
    setBusy("importing");
    setErrMsg("");
    try {
      const r = await importArtPack({
        sourceDir: dir,
        systemIdOverride: systemOverride() || null,
        dryRun: false,
      });
      setReport(r);
      props.onImported?.();
    } catch (e) {
      console.warn("[oa-art-import] live import failed:", e);
      setErrMsg(String(e));
    } finally {
      setBusy(null);
    }
  }

  function closeAndReset() {
    reset();
    props.onClose();
  }

  // ---- render ----

  return (
    <Dialog
      open={props.open}
      onClose={closeAndReset}
      title="Import art pack"
      subtitle="LaunchBox / EmuMovies folder → media/<system>/<kind>/<rom>.png"
      size="xl"
    >
      <div class="space-y-4 p-4">
        {/* Source folder picker */}
        <div class="space-y-1">
          <label class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
            Source folder
          </label>
          <div class="flex items-center gap-2">
            <input
              type="text"
              value={sourceDir()}
              readOnly
              placeholder="(none selected)"
              class="flex-1 min-w-0 rounded border border-white/10 bg-white/[0.04] px-3 py-1.5 text-xs text-(--color-oa-ink) placeholder:text-(--color-oa-ink-dim)/50"
            />
            <button
              type="button"
              onClick={pickFolder}
              disabled={busy() !== null}
              class="rounded border border-white/10 bg-white/[0.04] px-3 py-1.5 text-xs uppercase tracking-wider text-(--color-oa-ink) transition hover:bg-white/[0.08] disabled:cursor-not-allowed disabled:opacity-40"
            >
              Choose…
            </button>
          </div>
        </div>

        {/* Optional system_id override (used when source folder is a
            single-platform layout — kind dirs at root). The dry-run
            response will tell us if this is required; for now we
            offer the picker upfront. */}
        <Show when={sourceDir()}>
          <div class="space-y-1">
            <label class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
              System override (for single-platform folders)
            </label>
            <select
              value={systemOverride()}
              onChange={(e) => {
                setSystemOverride(e.currentTarget.value as SystemId | "");
                setReport(null);
              }}
              disabled={busy() !== null}
              class="w-full rounded border border-white/10 bg-white/[0.04] px-3 py-1.5 text-xs text-(--color-oa-ink) disabled:opacity-40"
            >
              <option value="">(auto-detect — leave empty for multi-platform packs)</option>
              <For each={ALL_SYSTEM_IDS}>
                {(sid) => (
                  <option value={sid}>
                    {systemThemes[sid].displayName} ({sid})
                  </option>
                )}
              </For>
            </select>
          </div>
        </Show>

        {/* Action row: Analyze (dry-run) + Import (live, only after a
            successful dry-run) */}
        <div class="flex items-center gap-2">
          <button
            type="button"
            onClick={() => void runDryRun()}
            disabled={!sourceDir() || busy() !== null}
            class="rounded border border-white/10 bg-white/[0.04] px-3 py-1.5 text-xs uppercase tracking-wider text-(--color-oa-ink) transition hover:bg-white/[0.08] disabled:cursor-not-allowed disabled:opacity-40"
          >
            {busy() === "analyzing" ? "Analyzing…" : "Analyze (dry run)"}
          </button>
          <button
            type="button"
            onClick={() => void runImport()}
            disabled={!report() || (report()?.totalImported ?? 0) === 0 || busy() !== null}
            class="rounded border border-white/10 bg-(--color-system-accent)/15 px-3 py-1.5 text-xs uppercase tracking-wider text-(--color-oa-ink) transition hover:bg-(--color-system-accent)/25 disabled:cursor-not-allowed disabled:opacity-40"
          >
            {busy() === "importing"
              ? "Importing…"
              : `Import ${report()?.totalImported ?? 0} files`}
          </button>
        </div>

        {/* Error message */}
        <Show when={errMsg()}>
          <p class="rounded border border-red-500/30 bg-red-500/10 px-3 py-2 text-xs text-red-200">
            {errMsg()}
          </p>
        </Show>

        {/* Report */}
        <Show when={report()}>
          {(r) => (
            <div class="space-y-3 rounded border border-white/10 bg-white/[0.02] p-3">
              <div class="flex items-center justify-between">
                <p class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
                  Layout: {r().layout}
                </p>
                <p class="text-xs text-(--color-oa-ink-dim)">
                  Total: {r().totalImported} imported · {r().totalSkippedNoMatch} no-match
                </p>
              </div>
              <For each={r().platforms}>
                {(p) => (
                  <div class="rounded border border-white/5 bg-white/[0.02] p-2">
                    <div class="flex items-center justify-between">
                      <p class="text-xs text-(--color-oa-ink)">
                        <Show when={p.launchboxName} fallback={p.platformDir}>
                          <span>{p.launchboxName}</span>
                          <span class="text-(--color-oa-ink-dim)"> → {p.systemId}</span>
                        </Show>
                      </p>
                      <p class="text-[0.65rem] text-(--color-oa-ink-dim)">
                        {p.libraryEntries} library entries
                      </p>
                    </div>
                    <Show when={p.error}>
                      <p class="mt-1 text-xs text-amber-300">⚠ {p.error}</p>
                    </Show>
                    <Show when={Object.keys(p.byKind).length > 0}>
                      <div class="mt-2 grid grid-cols-2 gap-x-3 gap-y-1 text-[0.7rem] text-(--color-oa-ink-dim) lg:grid-cols-3">
                        <For each={Object.values(p.byKind)}>
                          {(k) => (
                            <div class="flex justify-between gap-2">
                              <span>{k.kind}</span>
                              <span>
                                <span class="text-(--color-oa-ink)">{k.imported}</span>
                                {" / "}
                                <span>{k.sourceFiles}</span>
                              </span>
                            </div>
                          )}
                        </For>
                      </div>
                    </Show>
                  </div>
                )}
              </For>
            </div>
          )}
        </Show>

        {/* Close */}
        <div class="flex justify-end gap-2 pt-2">
          <button
            type="button"
            onClick={closeAndReset}
            class="rounded border border-white/10 bg-white/[0.04] px-3 py-1.5 text-xs uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
          >
            Close
          </button>
        </div>
      </div>
    </Dialog>
  );
};
