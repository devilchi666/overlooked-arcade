// Per-system cores strip — rendered inside SystemHeader, below the
// identity band, when a user has navigated into a system view. Shows
// only cores from the buildbot catalog that target this system, with
// install + update buttons.
//
// Default state: collapsed, with a count chip ("Cores · 2 of 4 installed
// ▶"). Click to expand. Independent of the global Cores Manager — they
// share the catalog data + install handler logic but each instance
// fetches + subscribes locally.

import {
  createMemo,
  createResource,
  createSignal,
  For,
  Show,
  type Component,
} from "solid-js";
import * as coresApi from "@oa/platform/api/coresApi";
import { listenScoped } from "@oa/platform/lib/eventListener";
import type { SystemId } from "@oa/platform/themes/registry";
import {
  CatalogCoreCard,
  type AvailableCore,
  type DownloadProgress,
} from "./CatalogCoreCard";

type Props = {
  systemId: SystemId;
  /// Default-expanded state. Defaults to collapsed (false) so the
  /// system landing band stays focused on the library grid; users who
  /// want to install a core open the strip with one click.
  defaultExpanded?: boolean;
};

const SystemCoresStrip: Component<Props> = (props) => {
  const [tick, setTick] = createSignal(0);
  const [catalog] = createResource(tick, async (): Promise<AvailableCore[]> => {
    try {
      return await coresApi.availableCores<AvailableCore>();
    } catch (e) {
      console.warn("[oa-cores-strip] available_cores failed:", e);
      return [];
    }
  });
  const refresh = () => setTick((n) => n + 1);

  // Per-base progress map (downloads in flight). Same shape as the
  // global Cores Manager; we keep a separate copy because this strip
  // mounts/unmounts as the user navigates between systems.
  const [progress, setProgress] = createSignal<Record<string, DownloadProgress>>({});
  const [busy, setBusy] = createSignal<string | null>(null);

  listenScoped<DownloadProgress>("oa://core-download-progress", (e) => {
    setProgress((m) => ({ ...m, [e.payload.fileName]: e.payload }));
    if (e.payload.phase === "done" || e.payload.phase === "error") {
      refresh();
    }
  });

  // Filter the catalog to cores that target this system. Both wired
  // and queued slugs hit here — the strip is system-scoped, not
  // wired-status-scoped.
  const cores = createMemo(() =>
    (catalog() ?? []).filter((c) => c.systems.includes(props.systemId)),
  );

  const installedCount = () => cores().filter((c) => c.installed).length;
  const totalCount = () => cores().length;

  const [expanded, setExpanded] = createSignal(props.defaultExpanded === true);

  async function handleInstall(c: AvailableCore) {
    if (!c.supportedOnHost) return;
    const key = `install-${c.base}`;
    if (busy() === key) return;
    setBusy(key);
    try {
      await coresApi.downloadCore(c.base);
      refresh();
    } catch (e) {
      console.warn("[oa-cores-strip] download_core failed:", e);
    } finally {
      setBusy(null);
    }
  }

  return (
    <Show when={totalCount() > 0}>
      <div class="mt-3 rounded-md border border-white/5 bg-black/15">
        <button
          type="button"
          onClick={() => setExpanded((v) => !v)}
          class="flex w-full items-center justify-between gap-3 px-3 py-1.5 text-left transition hover:bg-white/[0.03]"
          aria-expanded={expanded()}
        >
          <div class="flex items-baseline gap-2">
            <span
              class="text-[0.6rem] uppercase tracking-widest"
              classList={{
                "text-(--color-system-accent)": expanded(),
                "text-(--color-oa-ink-dim)": !expanded(),
              }}
            >
              {expanded() ? "▼" : "▶"}
            </span>
            <span class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Cores
            </span>
            <span class="text-xs text-(--color-oa-ink)">
              {installedCount()} of {totalCount()} installed
            </span>
          </div>
          <span class="text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
            Browse · install · update
          </span>
        </button>
        <Show when={expanded()}>
          <div class="grid grid-cols-1 gap-2 border-t border-white/5 p-2 sm:grid-cols-2">
            <For each={cores()}>
              {(c) => (
                <CatalogCoreCard
                  core={c}
                  progress={progress()[c.fileName]}
                  busyKey={busy()}
                  onInstall={() => void handleInstall(c)}
                  compact
                />
              )}
            </For>
          </div>
        </Show>
      </div>
    </Show>
  );
};

export default SystemCoresStrip;
