// One row in the buildbot catalog UI. Shared between the global Cores
// Manager (`CoresPage`) and the per-system strip on the system landing
// header (`SystemCoresStrip`). Self-contained: caller passes the core
// entry + current progress + busy state + install handler; the card
// renders the title / recommended chip / installed chip / BIOS warning /
// install button / progress strip / error banner.

import { Show, type Component } from "solid-js";

export type AvailableCore = {
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

export type DownloadProgress = {
  fileName: string;
  downloadedBytes: number;
  totalBytes: number | null;
  phase: "downloading" | "extracting" | "done" | "error";
  message?: string;
};

export function humanBytes(b: number): string {
  if (b < 1024) return `${b} B`;
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
  return `${(b / 1024 / 1024).toFixed(1)} MB`;
}

export function progressPercent(p: DownloadProgress | undefined): number | null {
  if (!p || !p.totalBytes || p.totalBytes <= 0) return null;
  return Math.min(100, Math.round((p.downloadedBytes / p.totalBytes) * 100));
}

type Props = {
  core: AvailableCore;
  progress: DownloadProgress | undefined;
  /// The currently-busy install key, if any. Cards compare against
  /// their own `install-<base>` to know if they should render the
  /// "Downloading…" state.
  busyKey: string | null;
  onInstall: () => void;
  /// Compact variant for the per-system strip — narrower padding and
  /// no filename row to keep the strip's vertical footprint small.
  compact?: boolean;
};

export const CatalogCoreCard: Component<Props> = (props) => {
  const c = () => props.core;
  const p = () => props.progress;
  const phase = () => p()?.phase;
  const pct = () => progressPercent(p());
  const busyKey = `install-${c().base}`;
  const installable = () => c().supportedOnHost;
  const compact = () => props.compact === true;
  return (
    <article
      class="rounded-lg border border-white/10 bg-black/20"
      classList={{
        "p-3": !compact(),
        "p-2": compact(),
      }}
    >
      <header class="flex items-start justify-between gap-3">
        <div class="min-w-0">
          <h4 class="truncate text-sm font-semibold text-(--color-oa-ink)">
            {c().displayName}
            <Show when={c().recommended}>
              <span class="ml-2 rounded bg-(--color-system-accent)/20 px-1.5 py-0.5 text-[0.55rem] uppercase tracking-widest text-(--color-system-accent)">
                recommended
              </span>
            </Show>
            <Show when={c().installed}>
              <span class="ml-2 rounded bg-white/[0.08] px-1.5 py-0.5 text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink)">
                installed
                <Show when={c().installedVersion}>
                  <span> · v{c().installedVersion}</span>
                </Show>
              </span>
            </Show>
          </h4>
          <p class="mt-0.5 text-[0.7rem] text-(--color-oa-ink-dim)">{c().blurb}</p>
          <Show when={!compact()}>
            <p class="mt-0.5 truncate text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              <code class="lowercase">{c().fileName}</code>
            </p>
          </Show>
          <Show when={c().biosRequired}>
            <p class="mt-1 rounded border border-amber-500/20 bg-amber-500/5 px-2 py-1 text-[0.6rem] uppercase tracking-widest text-amber-200">
              ⚠ needs <code class="font-mono normal-case text-amber-100">{c().biosRequired}</code> in <code class="font-mono normal-case text-amber-100">&lt;exe_dir&gt;/system/</code>
            </p>
          </Show>
        </div>
        <button
          type="button"
          onClick={props.onInstall}
          disabled={!installable() || props.busyKey === busyKey}
          class="shrink-0 rounded-md border px-2.5 py-1 text-[0.6rem] uppercase tracking-wider transition disabled:opacity-50"
          classList={{
            "border-(--color-system-accent)/30 bg-(--color-system-accent)/15 text-(--color-oa-ink) hover:bg-(--color-system-accent)/25":
              installable() && !c().installed,
            "border-white/10 bg-white/[0.03] text-(--color-oa-ink-dim) hover:bg-white/[0.08] hover:text-(--color-oa-ink)":
              installable() && c().installed,
            "border-white/5 bg-black/30 text-(--color-oa-ink-dim) cursor-not-allowed":
              !installable(),
          }}
          title={
            !installable()
              ? "No buildbot build for this OS/architecture"
              : c().installed
                ? "Re-download and replace"
                : "Download to <exe_dir>/cores/"
          }
        >
          {props.busyKey === busyKey
            ? phase() === "extracting" ? "Extracting…" : "Downloading…"
            : c().installed ? "Update" : "Install"}
        </button>
      </header>
      <Show when={props.busyKey === busyKey || phase() === "extracting" || phase() === "downloading"}>
        <div class="mt-2">
          <div class="h-1 overflow-hidden rounded bg-white/[0.06]">
            <div
              class="h-full bg-(--color-system-accent) transition-all"
              style={{
                width:
                  pct() !== null
                    ? `${pct()}%`
                    : phase() === "extracting"
                      ? "100%"
                      : "30%",
              }}
            />
          </div>
          <p class="mt-1 text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            {phase() === "extracting"
              ? "Extracting zip"
              : pct() !== null
                ? `${pct()}% · ${humanBytes(p()?.downloadedBytes ?? 0)} of ${humanBytes(p()?.totalBytes ?? 0)}`
                : `${humanBytes(p()?.downloadedBytes ?? 0)} downloaded`}
          </p>
        </div>
      </Show>
      <Show when={phase() === "error"}>
        <p class="mt-2 rounded bg-red-950/30 px-2 py-1 text-[0.65rem] text-red-300">
          {p()?.message ?? "Install failed."}
        </p>
      </Show>
    </article>
  );
};
