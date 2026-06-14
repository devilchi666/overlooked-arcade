import {
  createEffect,
  createMemo,
  createResource,
  createSignal,
  For,
  onCleanup,
  Show,
  type Accessor,
  type Component,
} from "solid-js";
import * as coresApi from "@oa/platform/api/coresApi";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { listenTo, OA_EVENTS } from "@oa/platform/api/eventsApi";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import { SpatialModalScope } from "@oa/platform/nav";

// Phase 1B Slice 4 — Bulk-install missing cores modal.
//
// Opens from two paths in the SystemReadinessChecklist:
//   1. Per-row "Install core…" button (scoped to that single system)
//   2. Top-of-checklist "Install N missing cores…" banner (all
//      systems that show ⚠ Core)
//
// Calls the existing core_installer::download_core(base) Tauri command
// per row; listens to `oa://core-download-progress` events for per-row
// live status. No new download Rust code — Slice 4 is UI on top of the
// Tauri commands core_installer.rs already exposes.

type AvailableCore = {
  base: string;
  fileName: string;
  displayName: string;
  blurb: string;
  systems: string[];
  installed: boolean;
  installedVersion?: string;
  supportedOnHost: boolean;
  buildbotUrl?: string;
  recommended: boolean;
  biosRequired?: string;
};

type CoreDownloadProgress = {
  fileName: string;
  downloadedBytes: number;
  totalBytes?: number;
  phase: "downloading" | "extracting" | "done" | "error";
  message?: string;
};

type RowStatus = "idle" | "downloading" | "done" | "error";

type ModalRow = {
  systemId: SystemId;
  /// Recommended cores for this system. Always non-empty when the row
  /// renders (empty rows are filtered out before construction).
  candidates: AvailableCore[];
  /// Index into candidates. Default 0 (first recommended).
  selectedIdx: number;
  /// Operator-toggleable inclusion. Defaults true.
  enabled: boolean;
  status: RowStatus;
  progress?: CoreDownloadProgress;
  errorMessage?: string;
};

export type MissingCoreBulkPromptProps = {
  open: boolean;
  onClose: () => void;
  systems: Accessor<SystemId[]>;
  /// Operator confirmed at least one install finished — checklist
  /// should refetch its installed-cores snapshot. Fired once per
  /// successful download.
  onCoreInstalled?: () => void;
};

const BTN =
  "rounded-md border border-white/10 bg-white/[0.04] px-3 py-1.5 text-xs font-medium uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink) disabled:cursor-not-allowed disabled:opacity-50";
const BTN_PRIMARY =
  "rounded-md bg-(--color-system-accent) px-3 py-1.5 text-xs font-semibold uppercase tracking-wider text-black/90 transition hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-50";

function formatBytes(n?: number): string {
  if (n === undefined || n === null) return "—";
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}

const MissingCoreBulkPrompt: Component<MissingCoreBulkPromptProps> = (props) => {
  // Fetch the catalog once on first open. createResource auto-suspends
  // until the first call resolves; the modal renders a loading state
  // while the fetch is in flight.
  const [catalog] = createResource(async () => {
    try {
      return await coresApi.availableCores<AvailableCore>();
    } catch (e) {
      console.warn("[oa-missing-core] available_cores failed:", e);
      return [] as AvailableCore[];
    }
  });

  // Build ModalRow[] from props.systems() + the catalog. Recomputes
  // when either changes. Filters: candidate must be recommended,
  // not already installed, and supported on this host.
  const [rows, setRows] = createSignal<ModalRow[]>([]);

  createEffect(() => {
    if (!props.open) return;
    const all = catalog();
    if (!all) return;
    const next: ModalRow[] = [];
    for (const systemId of props.systems()) {
      // Permissive filter: include any installable, supported candidate
      // (recommended OR not). Recommended-first sort ensures the default
      // selection still points at the OA-tested core, but operators
      // never get a silently-dropped row when a system has only
      // non-recommended candidates (the readiness checklist's banner
      // counts based on catalog presence, not recommended-presence —
      // they must agree to avoid the 2026-06-01 "1 missing core" /
      // "no missing cores" UX divergence).
      const candidates = all
        .filter(
          (c) =>
            c.systems.includes(systemId) &&
            !c.installed &&
            c.supportedOnHost,
        )
        .sort((a, b) => Number(b.recommended) - Number(a.recommended));
      if (candidates.length === 0) continue;
      next.push({
        systemId,
        candidates,
        selectedIdx: 0,
        enabled: true,
        status: "idle",
      });
    }
    setRows(next);
  });

  // Reset rows + listeners when the modal closes.
  createEffect(() => {
    if (props.open) return;
    setRows([]);
  });

  // Per-event listener — fired throughout each download. Match by
  // fileName to know which row to update.
  let unlistenProgress: UnlistenFn | undefined;
  createEffect(() => {
    if (!props.open) {
      if (unlistenProgress) {
        unlistenProgress();
        unlistenProgress = undefined;
      }
      return;
    }
    void (async () => {
      try {
        unlistenProgress = await listenTo<CoreDownloadProgress>(
          OA_EVENTS.coreDownloadProgress,
          (event) => {
            const payload = event.payload;
            setRows((prev) =>
              prev.map((r) => {
                const cur = r.candidates[r.selectedIdx];
                if (!cur || cur.fileName !== payload.fileName) return r;
                const status: RowStatus =
                  payload.phase === "done"
                    ? "done"
                    : payload.phase === "error"
                    ? "error"
                    : "downloading";
                const errorMessage =
                  payload.phase === "error" ? payload.message ?? "Download failed" : undefined;
                return { ...r, status, progress: payload, errorMessage };
              }),
            );
            if (payload.phase === "done") {
              props.onCoreInstalled?.();
            }
          },
        );
      } catch (e) {
        console.warn("[oa-missing-core] listen failed:", e);
      }
    })();
  });

  onCleanup(() => {
    if (unlistenProgress) unlistenProgress();
  });

  const anyDownloading = createMemo(() => rows().some((r) => r.status === "downloading"));
  const enabledCount = createMemo(() => rows().filter((r) => r.enabled).length);
  const totalRows = createMemo(() => rows().length);

  function setRow(systemId: SystemId, patch: Partial<ModalRow>) {
    setRows((prev) =>
      prev.map((r) => (r.systemId === systemId ? { ...r, ...patch } : r)),
    );
  }

  async function downloadAll() {
    const targets = rows().filter((r) => r.enabled && r.status !== "done");
    // Mark every target downloading up front so the operator sees the
    // batch kick off — individual events flip statuses one-by-one as
    // the per-row download progresses.
    for (const t of targets) {
      setRow(t.systemId, { status: "downloading", progress: undefined, errorMessage: undefined });
    }
    // Phase 4b bulk_core_install — create the parent job up front so
    // the BackgroundJobsBar shows ONE row "Installing N cores" with
    // an N-children aggregate progress, in addition to each child
    // core_download row. -1 sentinel means the backend's JobRegistry
    // isn't managed; the downloads still proceed but without a parent.
    let parentJobId: number | undefined;
    try {
      const id = await coresApi.startBulkCoreInstall(targets.length);
      if (id > 0) parentJobId = id;
    } catch (e) {
      // Soft-fail — downloads still proceed individually without a
      // parent row in the bar.
      console.warn("[oa-jobs] start_bulk_core_install failed:", e);
    }
    // Fire downloads in parallel — core_installer extracts via a
    // .partial swap so concurrent writes to /cores/ don't trample.
    await Promise.all(
      targets.map(async (t) => {
        const cur = t.candidates[t.selectedIdx];
        if (!cur) return;
        try {
          await coresApi.downloadCore(cur.base, parentJobId);
          // Final `phase: "done"` event already updates the row.
        } catch (e) {
          setRow(t.systemId, {
            status: "error",
            errorMessage: String(e),
          });
        }
      }),
    );
  }

  return (
    <Show when={props.open}>
      <SpatialModalScope
        id="missing-core-bulk"
        onCancel={() => {
          if (!anyDownloading()) props.onClose();
        }}
      >
      <div
        // z-[72]: above the Import Wizard (z-[70]) it opens from + the
        // engine takeover (z-[60]) so it's visible + navigable on top.
        class="fixed inset-0 z-[72] grid place-items-center bg-black/60 backdrop-blur-sm"
        onClick={(e) => {
          if (e.currentTarget === e.target && !anyDownloading()) props.onClose();
        }}
      >
        <div
          class="flex w-full max-w-2xl flex-col overflow-hidden rounded-lg border border-white/10 bg-(--color-oa-bg-deep) shadow-2xl shadow-black/60"
          style={{ "max-height": "85vh" }}
          role="dialog"
          aria-modal="true"
          aria-labelledby="missing-core-bulk-title"
        >
          <header class="flex items-center justify-between border-b border-white/5 px-6 py-4">
            <div>
              <h2
                id="missing-core-bulk-title"
                class="text-sm font-semibold uppercase tracking-[0.3em] text-(--color-oa-ink)"
              >
                Install missing cores
              </h2>
              <p class="mt-0.5 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                {totalRows() === 0
                  ? "Nothing to install"
                  : `${enabledCount()} of ${totalRows()} selected · downloads from libretro buildbot`}
              </p>
            </div>
            <button
              type="button"
              class={BTN}
              disabled={anyDownloading()}
              onClick={(e) => {
                e.currentTarget.blur();
                props.onClose();
              }}
            >
              Close
            </button>
          </header>

          <div class="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto px-6 py-5">
            <Show when={catalog.loading}>
              <p class="rounded-md border border-white/5 bg-black/20 px-3 py-2 text-xs text-(--color-oa-ink-dim)">
                Loading catalog…
              </p>
            </Show>
            <Show when={!catalog.loading && totalRows() === 0}>
              <p class="rounded-md border border-white/5 bg-black/20 px-3 py-3 text-center text-xs text-(--color-oa-ink-dim)">
                No missing cores. 👍
              </p>
            </Show>
            <For each={rows()}>
              {(row) => {
                const theme = () => systemThemes[row.systemId];
                const cur = () => row.candidates[row.selectedIdx];
                return (
                  <div
                    data-system={row.systemId}
                    class="flex flex-col gap-1.5 rounded-md border border-white/5 bg-white/[0.02] px-3 py-2"
                    classList={{
                      "opacity-50": !row.enabled,
                      "border-emerald-400/40 bg-emerald-500/5": row.status === "done",
                      "border-red-400/40 bg-red-500/5": row.status === "error",
                    }}
                  >
                    <div class="flex items-center gap-2">
                      <input
                        type="checkbox"
                        class="accent-(--color-system-accent)"
                        checked={row.enabled}
                        disabled={row.status === "downloading" || row.status === "done"}
                        onChange={(e) => setRow(row.systemId, { enabled: e.currentTarget.checked })}
                      />
                      <span class="text-xs font-semibold text-(--color-system-accent)">
                        {theme()?.shortName ?? row.systemId}
                      </span>
                      <span class="text-xs text-(--color-oa-ink-dim)">
                        {theme()?.displayName ?? row.systemId}
                      </span>
                      <div class="flex-1" />
                      <Show when={row.status === "idle" || row.status === "error"}>
                        <Show
                          when={row.candidates.length > 1}
                          fallback={
                            <span class="text-[0.7rem] font-mono text-(--color-oa-ink)">
                              {cur()?.displayName ?? "—"}
                            </span>
                          }
                        >
                          <select
                            class="rounded border border-white/10 bg-(--color-oa-bg-deep) px-2 py-1 text-[0.7rem] text-(--color-oa-ink)"
                            value={row.selectedIdx}
                            disabled={row.status === "downloading" || row.status === "done"}
                            onChange={(e) =>
                              setRow(row.systemId, { selectedIdx: Number(e.currentTarget.value) })
                            }
                          >
                            <For each={row.candidates}>
                              {(c, i) => <option value={i()}>{c.displayName}</option>}
                            </For>
                          </select>
                        </Show>
                      </Show>
                      <Show when={row.status === "downloading"}>
                        <span class="text-[0.65rem] uppercase tracking-widest text-amber-300">
                          {row.progress?.phase ?? "downloading"}
                        </span>
                      </Show>
                      <Show when={row.status === "done"}>
                        <span class="text-[0.65rem] uppercase tracking-widest text-emerald-300">
                          ✓ Installed
                        </span>
                      </Show>
                      <Show when={row.status === "error"}>
                        <span
                          class="text-[0.65rem] uppercase tracking-widest text-red-300"
                          title={row.errorMessage}
                        >
                          ⚠ Error
                        </span>
                      </Show>
                    </div>
                    <Show when={cur()?.blurb}>
                      <p class="text-[0.65rem] text-(--color-oa-ink-dim)">
                        {cur()!.blurb}
                      </p>
                    </Show>
                    <Show when={cur()?.biosRequired}>
                      <p class="text-[0.65rem] text-amber-300">
                        Requires BIOS: <code class="font-mono">{cur()!.biosRequired}</code>
                      </p>
                    </Show>
                    <Show when={row.status === "downloading" && row.progress}>
                      <div class="flex items-center gap-2">
                        <div class="h-1 flex-1 overflow-hidden rounded-full bg-white/5">
                          <div
                            class="h-full bg-(--color-system-accent) transition-all"
                            style={{
                              width:
                                row.progress!.totalBytes && row.progress!.totalBytes > 0
                                  ? `${Math.min(100, (row.progress!.downloadedBytes / row.progress!.totalBytes) * 100)}%`
                                  : "30%",
                            }}
                          />
                        </div>
                        <span class="text-[0.6rem] font-mono text-(--color-oa-ink-dim)">
                          {formatBytes(row.progress!.downloadedBytes)} /{" "}
                          {formatBytes(row.progress!.totalBytes)}
                        </span>
                      </div>
                    </Show>
                    <Show when={row.status === "error" && row.errorMessage}>
                      <p class="text-[0.65rem] text-red-300">{row.errorMessage}</p>
                    </Show>
                  </div>
                );
              }}
            </For>
          </div>

          <footer class="flex items-center justify-between border-t border-white/5 px-6 py-3">
            <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Cores download from buildbot.libretro.com and land in <code class="font-mono">/cores/</code>.
            </p>
            <div class="flex gap-2">
              <Show when={enabledCount() > 0}>
                <button
                  type="button"
                  class={BTN_PRIMARY}
                  disabled={anyDownloading()}
                  onClick={() => void downloadAll()}
                >
                  {anyDownloading() ? "Downloading…" : `Download ${enabledCount()} cores`}
                </button>
              </Show>
            </div>
          </footer>
        </div>
      </div>
      </SpatialModalScope>
    </Show>
  );
};

export default MissingCoreBulkPrompt;
