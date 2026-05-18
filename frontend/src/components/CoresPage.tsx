import { createMemo, createResource, createSignal, For, onCleanup, onMount, Show, type Component } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open as pickFile } from "@tauri-apps/plugin-dialog";
import type { CoreEntry } from "../settings/store";
import { systemThemes, type SystemId } from "../themes/registry";

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
};

type DownloadProgress = {
  fileName: string;
  downloadedBytes: number;
  totalBytes: number | null;
  phase: "downloading" | "extracting" | "done" | "error";
  message?: string;
};

type Props = {
  /// Back button target — same shape as SettingsPage / PerSystemSettingsPage.
  onBack: () => void;
};

function humanBytes(b: number): string {
  if (b < 1024) return `${b} B`;
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
  return `${(b / 1024 / 1024).toFixed(1)} MB`;
}

function humanModified(unixMs: number): string {
  if (unixMs <= 0) return "—";
  const d = new Date(unixMs);
  if (Number.isNaN(d.getTime())) return "—";
  return d.toISOString().slice(0, 10);
}

/// Dedicated OA-wide Cores view. Lists every libretro core in
/// `<exe_dir>/cores/` with the metadata `retro_get_system_info` reports
/// (library name + version + supported extensions), plus disk metadata
/// and per-row actions: pick a system to assign as default, remove the
/// .dll after confirm, and an "Add core…" button that picks a .dll via
/// dialog and copies it into the cores folder.
///
/// Feature 2 (buildbot catalog + install + update) layers on top of this
/// surface in a follow-up commit.
const CoresPage: Component<Props> = (props) => {
  const [cores, { refetch }] = createResource(async (): Promise<CoreEntry[]> => {
    try {
      return await invoke<CoreEntry[]>("list_cores");
    } catch (e) {
      console.warn("list_cores failed:", e);
      return [];
    }
  });
  // Per-system core prefs — we need to know which system, if any, currently
  // points to a given .dll so the row's chip can say "default for tg16".
  const systemIds = Object.keys(systemThemes) as SystemId[];
  const [prefsTick, setPrefsTick] = createSignal(0);
  const [prefs] = createResource(prefsTick, async (): Promise<Record<string, string | null>> => {
    const result: Record<string, string | null> = {};
    for (const id of systemIds) {
      try {
        const v = await invoke<string | null>("get_core_pref", { systemId: id });
        result[id] = v ?? null;
      } catch (e) {
        console.warn(`get_core_pref(${id}) failed:`, e);
        result[id] = null;
      }
    }
    return result;
  });

  const [busy, setBusy] = createSignal<string | null>(null);
  const [status, setStatus] = createSignal<string>("");

  // Feature 2 — catalog of buildbot-installable cores. Re-fetched after every
  // install/remove so the "Installed (v…)" chip stays in sync with the
  // installed list.
  const [catalogTick, setCatalogTick] = createSignal(0);
  const [catalog] = createResource(catalogTick, async (): Promise<AvailableCore[]> => {
    try {
      return await invoke<AvailableCore[]>("available_cores");
    } catch (e) {
      console.warn("available_cores failed:", e);
      return [];
    }
  });
  const refreshCatalog = () => setCatalogTick((n) => n + 1);

  // Per-base progress map. Phase + downloaded/total bytes feed the row's
  // little progress strip while a download is in flight.
  const [progress, setProgress] = createSignal<Record<string, DownloadProgress>>({});
  onMount(() => {
    let unlisten: (() => void) | undefined;
    void listen<DownloadProgress>("oa://core-download-progress", (e) => {
      setProgress((m) => ({ ...m, [e.payload.fileName]: e.payload }));
      if (e.payload.phase === "done" || e.payload.phase === "error") {
        // Refresh the installed-cores list once the .dll lands so the
        // row flips to "Installed (v…)".
        refetch();
        refreshCatalog();
      }
    }).then((un) => (unlisten = un));
    onCleanup(() => unlisten?.());
  });

  function progressPercent(p: DownloadProgress | undefined): number | null {
    if (!p || !p.totalBytes || p.totalBytes <= 0) return null;
    return Math.min(100, Math.round((p.downloadedBytes / p.totalBytes) * 100));
  }

  async function handleInstall(c: AvailableCore) {
    if (!c.supportedOnHost) {
      setStatus(`No buildbot build for this OS/ARCH (${navigator.platform}).`);
      return;
    }
    setBusy(`install-${c.base}`);
    setStatus(`Downloading ${c.displayName}…`);
    try {
      await invoke<string>("download_core", { base: c.base });
      setStatus(`Installed ${c.displayName}.`);
      refetch();
      refreshCatalog();
    } catch (e) {
      setStatus(`Install failed: ${String(e)}`);
    } finally {
      setBusy(null);
    }
  }

  const systemsUsingCore = (fileName: string): SystemId[] => {
    const p = prefs() ?? {};
    return systemIds.filter((id) => p[id] === fileName);
  };

  async function handleAddCore() {
    const picked = await pickFile({
      multiple: false,
      filters: [
        {
          name: "libretro core",
          extensions: ["dll", "so", "dylib"],
        },
      ],
    }).catch((e) => {
      console.warn("pickFile failed:", e);
      return null;
    });
    if (!picked || Array.isArray(picked)) return;
    setBusy("install");
    setStatus(`Validating ${picked}…`);
    try {
      await invoke<string>("install_core_from_path", { path: picked });
      setStatus(`Added ${picked.split(/[/\\]/).pop()} to cores folder.`);
      refetch();
    } catch (e) {
      setStatus(`Add failed: ${String(e)}`);
    } finally {
      setBusy(null);
    }
  }

  async function handleRemove(c: CoreEntry) {
    const label = c.libraryName || c.fileName;
    if (!window.confirm(`Remove ${label} from the cores folder?\n\nThe .dll file will be deleted from <exe_dir>/cores/. Any system or game pointing at this core will fall back to auto-detect on next launch.`)) {
      return;
    }
    setBusy(c.fileName);
    setStatus(`Removing ${label}…`);
    try {
      await invoke("remove_installed_core", { fileName: c.fileName });
      // Clear any per-system prefs that still point at this file so the
      // UI doesn't show a stale "default for X" chip after refetch.
      for (const id of systemsUsingCore(c.fileName)) {
        try {
          await invoke("set_core_pref", { systemId: id, fileName: null });
        } catch (e) {
          console.warn(`clearing core_pref(${id}) after remove failed:`, e);
        }
      }
      setStatus(`Removed ${label}.`);
      setPrefsTick((n) => n + 1);
      refetch();
    } catch (e) {
      setStatus(`Remove failed: ${String(e)}`);
    } finally {
      setBusy(null);
    }
  }

  async function handleSetDefault(systemId: string, fileName: string | null) {
    setBusy(`pref-${systemId}`);
    try {
      await invoke("set_core_pref", { systemId, fileName });
      setPrefsTick((n) => n + 1);
    } catch (e) {
      setStatus(`Set default failed: ${String(e)}`);
    } finally {
      setBusy(null);
    }
  }

  const empty = createMemo(() => (cores() ?? []).length === 0);

  return (
    <div
      class="flex h-full w-full flex-col bg-(--color-oa-bg)"
      role="region"
      aria-labelledby="cores-title"
    >
      <header class="flex items-center justify-between border-b border-white/5 bg-(--color-oa-bg-deep)/60 px-6 py-4">
        <div class="flex items-center gap-3">
          <button
            type="button"
            onClick={(e) => {
              e.currentTarget.blur();
              props.onBack();
            }}
            class="rounded-md border border-white/10 bg-white/[0.04] px-2.5 py-1 text-xs uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
          >
            ← Back
          </button>
          <div>
            <h2
              id="cores-title"
              class="text-sm font-semibold uppercase tracking-[0.3em] text-(--color-oa-ink)"
            >
              Cores
            </h2>
            <p class="mt-0.5 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Libretro cores installed in &lt;exe_dir&gt;/cores/
            </p>
          </div>
        </div>
        <div class="flex items-center gap-2">
          <button
            type="button"
            onClick={() => refetch()}
            class="rounded-md border border-white/10 bg-white/[0.04] px-2.5 py-1 text-[0.65rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
            title="Re-scan the cores folder"
          >
            ↻ Refresh
          </button>
          <button
            type="button"
            onClick={handleAddCore}
            disabled={busy() === "install"}
            class="rounded-md border border-(--color-system-accent)/30 bg-(--color-system-accent)/15 px-3 py-1 text-[0.65rem] uppercase tracking-wider text-(--color-oa-ink) transition hover:bg-(--color-system-accent)/25 disabled:opacity-50"
          >
            {busy() === "install" ? "Adding…" : "Add core…"}
          </button>
        </div>
      </header>

      <Show when={status()}>
        <div class="border-b border-white/5 bg-black/20 px-6 py-2 text-[0.7rem] text-(--color-oa-ink-dim)">
          {status()}
        </div>
      </Show>

      <section class="min-h-0 flex-1 overflow-y-auto px-6 py-6 space-y-8">
        <Show when={!empty()} fallback={
          <div class="rounded-lg border border-dashed border-white/10 bg-black/10 px-6 py-12 text-center">
            <p class="text-sm text-(--color-oa-ink-dim)">
              No libretro cores found in <code class="text-(--color-oa-ink)">&lt;exe_dir&gt;/cores/</code>.
            </p>
            <p class="mt-2 text-[0.7rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              Use "Add core…" to install one, or browse the buildbot catalog below.
            </p>
          </div>
        }>
          <div class="flex flex-col gap-3" data-installed-list>
            <h3 class="text-[0.7rem] uppercase tracking-[0.3em] text-(--color-oa-ink-dim)">
              Installed
            </h3>
            <For each={cores() ?? []}>
              {(c) => (
                <article
                  class="rounded-lg border border-white/10 bg-black/20 p-4"
                  classList={{
                    "border-red-500/40 bg-red-950/10": Boolean(c.error),
                  }}
                >
                  <header class="flex items-start justify-between gap-3">
                    <div class="min-w-0">
                      <h3 class="truncate text-sm font-semibold text-(--color-oa-ink)">
                        {c.libraryName || c.fileName}
                      </h3>
                      <p class="mt-0.5 truncate text-[0.7rem] text-(--color-oa-ink-dim)">
                        <code>{c.fileName}</code>
                        <Show when={c.libraryVersion}>
                          <span> · v{c.libraryVersion}</span>
                        </Show>
                        <span> · {humanBytes(c.sizeBytes)}</span>
                        <span> · {humanModified(c.modifiedUnixMs)}</span>
                      </p>
                    </div>
                    <div class="flex shrink-0 items-center gap-2">
                      {/* Per-row Update — only renders when the file maps
                          back to a catalog entry that supports this host. */}
                      {(() => {
                        const cat = (catalog() ?? []).find((x) => x.fileName === c.fileName);
                        if (!cat || !cat.supportedOnHost) return null;
                        const busyKey = `install-${cat.base}`;
                        return (
                          <button
                            type="button"
                            onClick={() => void handleInstall(cat)}
                            disabled={busy() === busyKey}
                            class="rounded-md border border-white/10 bg-white/[0.03] px-2.5 py-1 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink) disabled:opacity-50"
                            title={`Re-fetch from ${cat.buildbotUrl ?? "buildbot"}`}
                          >
                            {busy() === busyKey ? "Updating…" : "Update"}
                          </button>
                        );
                      })()}
                      <button
                        type="button"
                        onClick={() => handleRemove(c)}
                        disabled={busy() === c.fileName}
                        class="rounded-md border border-white/10 bg-white/[0.03] px-2.5 py-1 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:border-red-500/40 hover:bg-red-950/30 hover:text-red-300 disabled:opacity-50"
                      >
                        {busy() === c.fileName ? "Removing…" : "Remove"}
                      </button>
                    </div>
                  </header>

                  <Show when={c.error}>
                    <p class="mt-3 rounded bg-red-950/30 px-3 py-2 text-[0.7rem] text-red-300">
                      {c.error}
                    </p>
                  </Show>

                  <Show when={!c.error && c.validExtensions}>
                    <div class="mt-3 flex flex-wrap gap-1.5">
                      <For each={c.validExtensions.split("|").filter(Boolean)}>
                        {(ext) => (
                          <span class="rounded bg-white/[0.06] px-1.5 py-0.5 text-[0.6rem] font-mono uppercase tracking-wider text-(--color-oa-ink-dim)">
                            .{ext}
                          </span>
                        )}
                      </For>
                      <Show when={c.needFullpath}>
                        <span class="rounded bg-amber-950/40 px-1.5 py-0.5 text-[0.6rem] uppercase tracking-wider text-amber-300" title="Core requires a filesystem path — archived ROMs are extracted before launch.">
                          path-only
                        </span>
                      </Show>
                    </div>
                  </Show>

                  <Show when={!c.error}>
                    <div class="mt-3 flex items-center gap-3">
                      <label class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                        Set as default for
                      </label>
                      <select
                        class="rounded border border-white/10 bg-black/40 px-2 py-1 text-xs text-(--color-oa-ink)"
                        value=""
                        onChange={(e) => {
                          const v = e.currentTarget.value;
                          e.currentTarget.value = "";
                          if (!v) return;
                          void handleSetDefault(v, c.fileName);
                        }}
                      >
                        <option value="">— pick a system —</option>
                        <For each={systemIds}>
                          {(id) => (
                            <option value={id}>
                              {systemThemes[id].shortName} ({systemThemes[id].displayName})
                            </option>
                          )}
                        </For>
                      </select>
                      <Show when={systemsUsingCore(c.fileName).length > 0}>
                        <div class="flex flex-wrap gap-1.5">
                          <For each={systemsUsingCore(c.fileName)}>
                            {(id) => (
                              <span
                                class="rounded bg-(--color-system-accent)/20 px-2 py-0.5 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink)"
                                data-system={id}
                              >
                                default · {systemThemes[id].shortName}
                                <button
                                  type="button"
                                  class="ml-1.5 text-(--color-oa-ink-dim) hover:text-(--color-oa-ink)"
                                  title={`Clear ${systemThemes[id].shortName} default`}
                                  onClick={() => void handleSetDefault(id, null)}
                                >
                                  ✕
                                </button>
                              </span>
                            )}
                          </For>
                        </div>
                      </Show>
                    </div>
                  </Show>
                </article>
              )}
            </For>
          </div>
        </Show>

        {/* --- Browse cores (buildbot catalog) -------------------------- */}
        <div class="flex flex-col gap-3">
          <div class="flex items-baseline justify-between">
            <h3 class="text-[0.7rem] uppercase tracking-[0.3em] text-(--color-oa-ink-dim)">
              Browse cores
            </h3>
            <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              buildbot.libretro.com · nightly
            </span>
          </div>
          <Show when={(catalog() ?? []).some((c) => !c.supportedOnHost)}>
            <p class="rounded-md border border-amber-500/30 bg-amber-950/20 px-3 py-2 text-[0.7rem] text-amber-200">
              No buildbot build for this OS/architecture — Install is disabled. You can still
              drop a .dll/.so/.dylib in <code>&lt;exe_dir&gt;/cores/</code> manually.
            </p>
          </Show>
          <div class="grid grid-cols-1 gap-3 lg:grid-cols-2">
            <For each={catalog() ?? []}>
              {(c) => {
                const p = () => progress()[c.fileName];
                const phase = () => p()?.phase;
                const pct = () => progressPercent(p());
                const busyKey = `install-${c.base}`;
                const installable = c.supportedOnHost;
                return (
                  <article class="rounded-lg border border-white/10 bg-black/20 p-3">
                    <header class="flex items-start justify-between gap-3">
                      <div class="min-w-0">
                        <h4 class="truncate text-sm font-semibold text-(--color-oa-ink)">
                          {c.displayName}
                          <Show when={c.installed}>
                            <span class="ml-2 rounded bg-(--color-system-accent)/20 px-1.5 py-0.5 text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink)">
                              installed
                              <Show when={c.installedVersion}>
                                <span> · v{c.installedVersion}</span>
                              </Show>
                            </span>
                          </Show>
                        </h4>
                        <p class="mt-0.5 text-[0.7rem] text-(--color-oa-ink-dim)">
                          {c.blurb}
                        </p>
                        <p class="mt-0.5 truncate text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                          <code class="lowercase">{c.fileName}</code>
                        </p>
                      </div>
                      <button
                        type="button"
                        onClick={() => void handleInstall(c)}
                        disabled={!installable || busy() === busyKey}
                        class="shrink-0 rounded-md border px-2.5 py-1 text-[0.6rem] uppercase tracking-wider transition disabled:opacity-50"
                        classList={{
                          "border-(--color-system-accent)/30 bg-(--color-system-accent)/15 text-(--color-oa-ink) hover:bg-(--color-system-accent)/25":
                            installable && !c.installed,
                          "border-white/10 bg-white/[0.03] text-(--color-oa-ink-dim) hover:bg-white/[0.08] hover:text-(--color-oa-ink)":
                            installable && c.installed,
                          "border-white/5 bg-black/30 text-(--color-oa-ink-dim) cursor-not-allowed":
                            !installable,
                        }}
                        title={
                          !installable
                            ? "No buildbot build for this OS/architecture"
                            : c.installed
                              ? "Re-download and replace"
                              : "Download to <exe_dir>/cores/"
                        }
                      >
                        {busy() === busyKey
                          ? phase() === "extracting" ? "Extracting…" : "Downloading…"
                          : c.installed ? "Update" : "Install"}
                      </button>
                    </header>
                    <Show when={c.systems.length > 0}>
                      <div class="mt-2 flex flex-wrap gap-1">
                        <For each={c.systems}>
                          {(s) => {
                            const t = systemThemes[s as SystemId];
                            return (
                              <span
                                class="rounded bg-white/[0.06] px-1.5 py-0.5 text-[0.55rem] uppercase tracking-widest text-(--color-oa-ink-dim)"
                                data-system={s}
                              >
                                {t?.shortName ?? s}
                              </span>
                            );
                          }}
                        </For>
                      </div>
                    </Show>
                    <Show when={busy() === busyKey || phase() === "extracting" || phase() === "downloading"}>
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
              }}
            </For>
          </div>
        </div>
      </section>
    </div>
  );
};

export default CoresPage;
