import { createEffect, createResource, createSignal, For, onCleanup, onMount, Show, type Component } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { LibraryStore } from "../library/store";
import { useMedia } from "../library/media";
import {
  PRESENTATION_LABELS,
  PRESENTATION_OPTIONS,
  type LayoutStore,
} from "../layout/state";
import {
  SCALING_MODE_LABELS,
  SCALING_OPTIONS,
  SHELL_MODE_LABELS,
  SHELL_OPTIONS,
  WINDOW_MODE_LABELS,
  WINDOW_OPTIONS,
  type AudioDeviceInfo,
  type CoreEntry,
  type MonitorInfo,
  type ScalingMode,
  type SettingsStore,
  type ShellMode,
  type WindowMode,
} from "../settings/store";
import { systemThemes, type SystemId } from "../themes/registry";

type Props = {
  /// Navigate back to the previous view. Used by the header Back button
  /// and by the Escape key handler. Replaces the modal-era `onClose`.
  onBack: () => void;
  settings: SettingsStore;
  library: LibraryStore;
  layout: LayoutStore;
  onAddLibraryFolder: () => void;
  onRescanLibraryFolders: () => void;
};

type SyncProgressPayload = {
  systemId: string;
  done: number;
  total: number;
  currentRomTitle: string;
  lastAction: string;
};

type SyncSummaryPayload = {
  systemId: string;
  total: number;
  matched: number;
  downloaded: number;
  cached: number;
  unmatched: number;
  errors: number;
};

type MetadataSyncSummaryPayload = {
  systemId: string;
  total: number;
  matched: number;
  updated: number;
  unchanged: number;
  unmatched: number;
  errors: number;
};

type MediaStorageStats = {
  coversBytes: number;
  thumbsBytes: number;
  cacheBytes: number;
  totalBytes: number;
};

function humanBytes(b: number): string {
  if (b < 1024) return `${b} B`;
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
  if (b < 1024 * 1024 * 1024) return `${(b / 1024 / 1024).toFixed(1)} MB`;
  return `${(b / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

function monitorLabel(m: MonitorInfo): string {
  const name = m.name?.trim() || `Monitor ${m.index + 1}`;
  return `${name} (${m.width}×${m.height})`;
}

const SELECT_CLASS =
  "w-full rounded-md border border-white/10 bg-white/[0.04] px-3 py-2 text-sm font-medium text-(--color-oa-ink) transition hover:bg-white/[0.08] focus-visible:outline focus-visible:outline-2 focus-visible:outline-(--color-oa-ink-dim)";

const TABS = ["presentation", "display", "audio", "gameplay", "cores", "library", "media"] as const;
type TabId = typeof TABS[number];
const TAB_LABELS: Record<TabId, string> = {
  presentation: "Presentation",
  display:      "Display",
  audio:        "Audio",
  gameplay:     "Gameplay",
  cores:        "Cores",
  library:      "Library",
  media:        "Game media",
};
const TAB_HINTS: Record<TabId, string> = {
  presentation: "Mode (Desktop / Theater / Cabinet) + sidebar layout",
  display:      "Scaling, window mode, monitor, shell mode",
  audio:        "Output device + sample-rate routing",
  gameplay:     "Rewind, save-state behavior, replay",
  cores:        "Detected libretro cores + per-system defaults",
  library:      "Tracked folders, scanning, ingest",
  media:        "Cover art + snapshots + titles, libretro-thumbnails sync, region priority",
};

// Reasonable picker steps for the rewind buffer + capture interval.
// Capture interval: 1 frame (smoothest, costliest) to 30 frames (~500ms at 60 fps).
const REWIND_INTERVAL_OPTIONS: readonly number[] = [1, 2, 3, 6, 10, 15, 30];
// Buffer cap: 8 MB to 512 MB. Operators on systems with big RAM can push it up;
// the default 64 MB holds ~10 s on every current core comfortably.
const REWIND_BUFFER_OPTIONS: readonly number[] = [8, 16, 32, 64, 128, 256, 512];

const SettingsPage: Component<Props> = (props) => {
  // Tabbed layout — sidebar nav on the left, per-tab content on the right.
  // Persists the last-selected tab in localStorage so reopening the modal
  // lands you where you were.
  const TAB_STORAGE = "oa.settings.activeTab";
  const initialTab: TabId = (() => {
    const saved = localStorage.getItem(TAB_STORAGE) as TabId | null;
    return saved && TABS.includes(saved) ? saved : "display";
  })();
  const [activeTab, setActiveTab] = createSignal<TabId>(initialTab);
  createEffect(() => {
    localStorage.setItem(TAB_STORAGE, activeTab());
  });

  // Esc returns to the previous view. Page-mode equivalent of the modal's
  // close-on-Esc — the parent owns the navigation target (typically
  // `setCurrentView({ kind: "all" })`).
  const escHandler = (e: KeyboardEvent) => {
    if (e.key !== "Escape") return;
    const tag = (document.activeElement as HTMLElement | null)?.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;
    e.stopPropagation();
    props.onBack();
  };
  onMount(() => window.addEventListener("keydown", escHandler, { capture: true }));
  onCleanup(() => window.removeEventListener("keydown", escHandler, { capture: true }));

  // Page-mode resource fetches: fire once on mount; un/remount handles the
  // refresh cadence the modal-era `() => props.open` source used to drive.
  const [monitors] = createResource(async (): Promise<MonitorInfo[]> => {
    try {
      return await invoke<MonitorInfo[]>("list_monitors");
    } catch (e) {
      console.warn("list_monitors failed:", e);
      return [];
    }
  });

  const [audioDevices] = createResource(async (): Promise<AudioDeviceInfo[]> => {
    try {
      return await invoke<AudioDeviceInfo[]>("list_audio_devices");
    } catch (e) {
      console.warn("list_audio_devices failed:", e);
      return [];
    }
  });

  // Cores-folder scan + per-system core preference state. The preference is
  // file-backed on the Rust side (appDataDir/cores.json) and takes effect on
  // next launch — same pattern as shellMode.
  const [cores] = createResource(async (): Promise<CoreEntry[]> => {
    try {
      return await invoke<CoreEntry[]>("list_cores");
    } catch (e) {
      console.warn("list_cores failed:", e);
      return [];
    }
  });

  const systemIds = Object.keys(systemThemes) as SystemId[];
  const [corePrefs, setCorePrefs] = createSignal<Record<SystemId, string | null>>(
    Object.fromEntries(systemIds.map((id) => [id, null])) as Record<SystemId, string | null>,
  );
  // Hydrate per-system core prefs from Rust once on page mount.
  createResource(async () => {
    const next: Record<string, string | null> = {};
    for (const id of systemIds) {
      try {
        const v = await invoke<string | null>("get_core_pref", { systemId: id });
        next[id] = v ?? null;
      } catch (e) {
        console.warn(`get_core_pref(${id}) failed:`, e);
        next[id] = null;
      }
    }
    setCorePrefs(next as Record<SystemId, string | null>);
    return null;
  });
  async function setCorePref(systemId: SystemId, fileName: string | null) {
    setCorePrefs((prev) => ({ ...prev, [systemId]: fileName }));
    try {
      await invoke("set_core_pref", { systemId, fileName });
    } catch (e) {
      console.warn("set_core_pref failed:", e);
    }
  }

  // --- Game media (covers): sync, region priority, storage stats ---

  const media = useMedia();

  // Per-system live sync progress, keyed by systemId. Updated by oa://library-sync events.
  const [syncProgress, setSyncProgress] = createSignal<Record<string, SyncProgressPayload>>({});
  // Per-system "is sync running" flag, used to disable the button.
  const [syncing, setSyncing] = createSignal<Record<string, boolean>>({});
  // Same shape for the per-system metadata sync — independent signals so
  // the two buttons can run independently and don't share progress state.
  const [metaProgress, setMetaProgress] = createSignal<Record<string, SyncProgressPayload>>({});
  const [metaSyncing, setMetaSyncing] = createSignal<Record<string, boolean>>({});
  let unlistenSync: UnlistenFn | undefined;
  let unlistenSyncDone: UnlistenFn | undefined;
  let unlistenMeta: UnlistenFn | undefined;
  let unlistenMetaDone: UnlistenFn | undefined;

  onMount(async () => {
    try {
      unlistenSync = await listen<SyncProgressPayload>("oa://library-sync", (ev) => {
        setSyncProgress((prev) => ({ ...prev, [ev.payload.systemId]: ev.payload }));
      });
      unlistenSyncDone = await listen<SyncSummaryPayload>("oa://library-sync-complete", (ev) => {
        setSyncing((prev) => ({ ...prev, [ev.payload.systemId]: false }));
        // Surface the final tally as the "last progress" line.
        setSyncProgress((prev) => ({
          ...prev,
          [ev.payload.systemId]: {
            systemId: ev.payload.systemId,
            done: ev.payload.total,
            total: ev.payload.total,
            currentRomTitle: "",
            lastAction: `done: ${ev.payload.downloaded} new / ${ev.payload.cached} cached / ${ev.payload.unmatched} unmatched / ${ev.payload.errors} errors`,
          },
        }));
      });
      unlistenMeta = await listen<SyncProgressPayload>("oa://library-metadata-sync", (ev) => {
        setMetaProgress((prev) => ({ ...prev, [ev.payload.systemId]: ev.payload }));
      });
      unlistenMetaDone = await listen<MetadataSyncSummaryPayload>("oa://library-metadata-sync-complete", (ev) => {
        setMetaSyncing((prev) => ({ ...prev, [ev.payload.systemId]: false }));
        setMetaProgress((prev) => ({
          ...prev,
          [ev.payload.systemId]: {
            systemId: ev.payload.systemId,
            done: ev.payload.total,
            total: ev.payload.total,
            currentRomTitle: "",
            lastAction: `done: ${ev.payload.updated} updated / ${ev.payload.unchanged} unchanged / ${ev.payload.unmatched} unmatched / ${ev.payload.errors} errors`,
          },
        }));
      });
    } catch (e) {
      console.warn("SettingsPage: listen('oa://library-(metadata-)sync*') failed:", e);
    }
  });
  onCleanup(() => {
    unlistenSync?.();
    unlistenSyncDone?.();
    unlistenMeta?.();
    unlistenMetaDone?.();
  });

  async function startSync(systemId: SystemId) {
    const entries = props.library.state.entries
      .filter((e) => e.systemId === systemId && !e.seed)
      .map((e) => ({
        id: e.id,
        title: e.title,
        filePath: e.filePath,
        systemId: e.systemId,
        sha1: e.sha1,
      }));
    if (entries.length === 0) {
      setSyncProgress((prev) => ({
        ...prev,
        [systemId]: {
          systemId,
          done: 0,
          total: 0,
          currentRomTitle: "",
          lastAction: "no entries to sync",
        },
      }));
      return;
    }
    setSyncing((prev) => ({ ...prev, [systemId]: true }));
    setSyncProgress((prev) => ({
      ...prev,
      [systemId]: { systemId, done: 0, total: entries.length, currentRomTitle: "", lastAction: "starting…" },
    }));
    try {
      await invoke("sync_media_for_system", { systemId, entries });
      // Belt-and-braces full re-hydrate. The per-ROM oa://media-updated
      // events should have already populated the MediaContext, but a fresh
      // pull guards against missed events (subscription timing / handler
      // not yet attached / etc.). Cheap — one IPC + one Solid store update.
      await media.refreshAll();
    } catch (e) {
      console.warn("sync_media_for_system failed:", e);
      setSyncing((prev) => ({ ...prev, [systemId]: false }));
      setSyncProgress((prev) => ({
        ...prev,
        [systemId]: { systemId, done: 0, total: entries.length, currentRomTitle: "", lastAction: `error: ${e}` },
      }));
    }
  }

  // Feature 3 — hash-based ROM identification.
  // Two stages: first sync the libretro-database dat into our local
  // rom_hashes table; then resolve every game in the library that
  // doesn't yet have a sha1 stamped on it, hashing + looking up + on
  // hit overwriting the title with the canonical name. Both stages
  // are per-system; we surface them on the Game media tab next to the
  // other sync rows.
  type HashSyncSummaryPayload = {
    systemId: string;
    upstreamEntries: number;
    written: number;
    fromCache: boolean;
  };
  type HashResolveProgressPayload = {
    systemId: string;
    done: number;
    total: number;
    currentTitle: string;
    lastAction: string;
  };
  type HashResolveSummaryPayload = {
    systemId: string;
    scanned: number;
    matched: number;
    unmatched: number;
    skippedCd: number;
    errors: number;
    /// Canonical entries available in the local rom_hashes table when
    /// the resolve started. `0` = libretro-database has no dat for this
    /// system → the UI shows a "no hash DB available" message rather
    /// than "X unknown."
    canonicalEntries: number;
  };
  const [hashSyncing, setHashSyncing] = createSignal<Record<string, boolean>>({});
  const [hashSyncSummary, setHashSyncSummary] = createSignal<Record<string, HashSyncSummaryPayload>>({});
  const [hashResolving, setHashResolving] = createSignal<Record<string, boolean>>({});
  const [hashResolveProgress, setHashResolveProgress] =
    createSignal<Record<string, HashResolveProgressPayload>>({});
  const [hashResolveSummary, setHashResolveSummary] =
    createSignal<Record<string, HashResolveSummaryPayload>>({});

  onMount(() => {
    let un1: UnlistenFn | undefined;
    let un2: UnlistenFn | undefined;
    let un3: UnlistenFn | undefined;
    void (async () => {
      try {
        un1 = await listen<HashSyncSummaryPayload>("oa://rom-hashes-synced", (ev) => {
          setHashSyncSummary((p) => ({ ...p, [ev.payload.systemId]: ev.payload }));
          setHashSyncing((p) => ({ ...p, [ev.payload.systemId]: false }));
        });
        un2 = await listen<HashResolveProgressPayload>("oa://rom-hash-resolve-progress", (ev) => {
          setHashResolveProgress((p) => ({ ...p, [ev.payload.systemId]: ev.payload }));
        });
        un3 = await listen<HashResolveSummaryPayload>("oa://rom-hash-resolve-complete", (ev) => {
          setHashResolveSummary((p) => ({ ...p, [ev.payload.systemId]: ev.payload }));
          setHashResolving((p) => ({ ...p, [ev.payload.systemId]: false }));
        });
      } catch (e) {
        console.warn("SettingsPage: hash listen failed:", e);
      }
    })();
    onCleanup(() => {
      un1?.();
      un2?.();
      un3?.();
    });
  });

  async function startHashSync(systemId: SystemId) {
    setHashSyncing((p) => ({ ...p, [systemId]: true }));
    try {
      await invoke("sync_rom_hashes_for_system", { systemId });
    } catch (e) {
      console.warn("sync_rom_hashes_for_system failed:", e);
      setHashSyncing((p) => ({ ...p, [systemId]: false }));
    }
  }

  // "Only sync identified ROMs" pref — file-backed on the Rust side
  // alongside the other media prefs. Hydrated once on mount; setter
  // writes through.
  const [onlySyncIdentified, setOnlySyncIdentifiedLocal] = createSignal<boolean>(true);
  onMount(() => {
    void invoke<boolean>("get_only_sync_identified")
      .then((v) => setOnlySyncIdentifiedLocal(v))
      .catch((e) => console.warn("get_only_sync_identified failed:", e));
  });
  async function setOnlySyncIdentifiedPref(v: boolean) {
    setOnlySyncIdentifiedLocal(v);
    try {
      await invoke("set_only_sync_identified", { enabled: v });
    } catch (e) {
      console.warn("set_only_sync_identified failed:", e);
    }
  }

  async function startHashResolve(systemId: SystemId) {
    setHashResolving((p) => ({ ...p, [systemId]: true }));
    setHashResolveProgress((p) => ({
      ...p,
      [systemId]: { systemId, done: 0, total: 0, currentTitle: "", lastAction: "starting…" },
    }));
    try {
      await invoke("resolve_rom_hashes_for_system", { systemId });
      // The completed library should reflect canonical titles now —
      // pull a fresh entries list so the UI updates.
      await props.library.refresh();
    } catch (e) {
      console.warn("resolve_rom_hashes_for_system failed:", e);
      setHashResolving((p) => ({ ...p, [systemId]: false }));
    }
  }

  async function startMetadataSync(systemId: SystemId) {
    const entries = props.library.state.entries
      .filter((e) => e.systemId === systemId && !e.seed)
      .map((e) => ({
        id: e.id,
        title: e.title,
        filePath: e.filePath,
        systemId: e.systemId,
        sha1: e.sha1,
      }));
    if (entries.length === 0) {
      setMetaProgress((prev) => ({
        ...prev,
        [systemId]: {
          systemId,
          done: 0,
          total: 0,
          currentRomTitle: "",
          lastAction: "no entries to sync",
        },
      }));
      return;
    }
    setMetaSyncing((prev) => ({ ...prev, [systemId]: true }));
    setMetaProgress((prev) => ({
      ...prev,
      [systemId]: { systemId, done: 0, total: entries.length, currentRomTitle: "", lastAction: "starting…" },
    }));
    try {
      await invoke("sync_metadata_for_system", { systemId, entries });
      await media.refreshAll();
    } catch (e) {
      console.warn("sync_metadata_for_system failed:", e);
      setMetaSyncing((prev) => ({ ...prev, [systemId]: false }));
      setMetaProgress((prev) => ({
        ...prev,
        [systemId]: { systemId, done: 0, total: entries.length, currentRomTitle: "", lastAction: `error: ${e}` },
      }));
    }
  }

  // Region priority list — bound to MediaContext (which hydrates from Rust).
  const [regionDraft, setRegionDraft] = createSignal<string[]>([]);
  createEffect(() => {
    // Mirror context value into local mutable draft for reordering UX.
    setRegionDraft(media.regionPriority());
  });
  function moveRegion(idx: number, delta: number) {
    setRegionDraft((prev) => {
      const next = [...prev];
      const target = idx + delta;
      if (target < 0 || target >= next.length) return prev;
      [next[idx], next[target]] = [next[target], next[idx]];
      void media.setRegionPriority(next);
      return next;
    });
  }
  function removeRegion(idx: number) {
    setRegionDraft((prev) => {
      const next = prev.filter((_, i) => i !== idx);
      void media.setRegionPriority(next);
      return next;
    });
  }
  function addRegion(name: string) {
    if (!name) return;
    setRegionDraft((prev) => {
      if (prev.includes(name)) return prev;
      const next = [...prev, name];
      void media.setRegionPriority(next);
      return next;
    });
  }

  // Disk usage — fetched on page mount AND after each sync completes.
  const [storageStats, { refetch: refetchStorage }] = createResource(
    async (): Promise<MediaStorageStats | null> => {
      try {
        return await invoke<MediaStorageStats>("media_storage_stats");
      } catch (e) {
        console.warn("media_storage_stats failed:", e);
        return null;
      }
    },
  );
  // After any sync completes, refresh the disk usage display.
  createEffect(() => {
    const v = Object.values(syncing()).some((b) => b);
    if (!v) {
      void refetchStorage();
    }
  });

  return (
    <div
      class="flex h-full w-full flex-col bg-(--color-oa-bg)"
      role="region"
      aria-labelledby="settings-title"
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
            title="Back (Esc)"
            aria-label="Back"
          >
            ‹ Back
          </button>
          <div>
            <h2
              id="settings-title"
              class="text-sm font-semibold uppercase tracking-[0.3em] text-(--color-oa-ink)"
            >
              Settings
            </h2>
            <p class="mt-0.5 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
              {TAB_LABELS[activeTab()]} · {TAB_HINTS[activeTab()]}
            </p>
          </div>
        </div>
      </header>

      <div class="flex min-h-0 flex-1">
            {/* Sidebar — vertical tab nav. Active tab gets a left-edge accent
                and a brighter background. LaunchBox / Steam / macOS-Settings
                convention. */}
            <nav class="flex w-44 shrink-0 flex-col gap-1 border-r border-white/5 bg-black/20 px-2 py-4">
              <For each={TABS}>
                {(tab) => (
                  <button
                    type="button"
                    onClick={(e) => {
                      e.currentTarget.blur();
                      setActiveTab(tab);
                    }}
                    aria-pressed={activeTab() === tab}
                    class="relative rounded-md px-3 py-2 text-left text-xs font-medium uppercase tracking-wider transition"
                    classList={{
                      "bg-white/[0.06] text-(--color-oa-ink)": activeTab() === tab,
                      "text-(--color-oa-ink-dim) hover:bg-white/[0.04] hover:text-(--color-oa-ink)": activeTab() !== tab,
                    }}
                  >
                    <Show when={activeTab() === tab}>
                      <span
                        class="pointer-events-none absolute left-0 top-1/2 h-5 w-0.5 -translate-y-1/2 rounded-r bg-(--color-system-accent)"
                        aria-hidden="true"
                      />
                    </Show>
                    {TAB_LABELS[tab]}
                  </button>
                )}
              </For>
            </nav>

            {/* Main content — per-tab body. Each tab block is wrapped in
                <Show> so non-active tabs neither render nor mount their
                resources (createResource etc. are tied to props.open, so
                this is purely a visibility toggle). */}
            <section class="min-h-0 flex-1 space-y-6 overflow-y-auto px-6 py-6">
            <Show when={activeTab() === "presentation"}>
            <div class="space-y-4">
              <h3 class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
                Presentation
              </h3>

              <div class="space-y-2">
                <p class="text-xs text-(--color-oa-ink-dim)">Mode</p>
                <div class="grid grid-cols-3 gap-2">
                  <For each={PRESENTATION_OPTIONS}>
                    {(mode) => {
                      const active = () => props.layout.presentationMode() === mode;
                      const description =
                        mode === "desktop" ? "Mouse + keyboard, full chrome."
                        : mode === "theater" ? "Couch + controller, larger fonts, idle-hide chrome."
                        : "Kiosk: chromeless coverflow, attract loop, PIN-locked admin.";
                      return (
                        <button
                          type="button"
                          onClick={(e) => {
                            e.currentTarget.blur();
                            props.layout.setPresentationMode(mode);
                          }}
                          aria-pressed={active()}
                          class="flex flex-col gap-1 rounded-md border px-3 py-3 text-left transition"
                          classList={{
                            "border-(--color-system-accent) bg-(--color-system-accent)/10": active(),
                            "border-white/10 bg-white/[0.02] hover:bg-white/[0.05]": !active(),
                          }}
                        >
                          <span class="text-sm font-semibold uppercase tracking-wider text-(--color-oa-ink)">
                            {PRESENTATION_LABELS[mode]}
                          </span>
                          <span class="text-[0.65rem] leading-relaxed text-(--color-oa-ink-dim)">
                            {description}
                          </span>
                        </button>
                      );
                    }}
                  </For>
                </div>
                <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                  Cabinet mode ships as layout-only in this build — attract loop, marquee output, and PIN-locked admin land in Phase 4.
                </p>
              </div>

              <div class="space-y-2">
                <p class="text-xs text-(--color-oa-ink-dim)">Sidebars</p>
                <div class="flex flex-col gap-1.5">
                  <label class="flex cursor-pointer items-center gap-3 rounded-md border border-white/10 bg-white/[0.02] px-3 py-2 transition hover:bg-white/[0.05]">
                    <input
                      type="checkbox"
                      checked={!props.layout.leftSidebarCollapsed()}
                      onChange={(e) => props.layout.setLeftSidebarCollapsed(!e.currentTarget.checked)}
                      class="h-3.5 w-3.5 accent-(--color-system-accent)"
                    />
                    <div class="flex-1">
                      <p class="text-xs text-(--color-oa-ink)">Left sidebar expanded</p>
                      <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                        Toggle with Ctrl+B
                      </p>
                    </div>
                  </label>
                  <label class="flex cursor-pointer items-center gap-3 rounded-md border border-white/10 bg-white/[0.02] px-3 py-2 transition hover:bg-white/[0.05]">
                    <input
                      type="checkbox"
                      checked={props.layout.rightSidebarVisible()}
                      onChange={(e) => props.layout.setRightSidebarVisible(e.currentTarget.checked)}
                      class="h-3.5 w-3.5 accent-(--color-system-accent)"
                    />
                    <div class="flex-1">
                      <p class="text-xs text-(--color-oa-ink)">Right sidebar visible</p>
                      <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                        Game details + widgets
                      </p>
                    </div>
                  </label>
                </div>
              </div>
            </div>
            </Show>

            <Show when={activeTab() === "display"}>
            <div class="space-y-3">
              <h3 class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
                Display
              </h3>

              <label class="block space-y-1">
                <span class="text-xs text-(--color-oa-ink-dim)">Scaling mode</span>
                <select
                  value={props.settings.scalingMode()}
                  onChange={(e) =>
                    props.settings.setScalingMode(e.currentTarget.value as ScalingMode)
                  }
                  class={SELECT_CLASS}
                >
                  <For each={SCALING_OPTIONS}>
                    {(mode) => <option value={mode}>{SCALING_MODE_LABELS[mode]}</option>}
                  </For>
                </select>
              </label>

              <label class="block space-y-1">
                <span class="text-xs text-(--color-oa-ink-dim)">Window mode</span>
                <select
                  value={props.settings.windowMode()}
                  onChange={(e) =>
                    props.settings.setWindowMode(e.currentTarget.value as WindowMode)
                  }
                  class={SELECT_CLASS}
                >
                  <For each={WINDOW_OPTIONS}>
                    {(mode) => <option value={mode}>{WINDOW_MODE_LABELS[mode]}</option>}
                  </For>
                </select>
              </label>

              <label class="block space-y-1">
                <span class="text-xs text-(--color-oa-ink-dim)">Monitor (for borderless)</span>
                <select
                  value={props.settings.monitorIndex() === null ? "current" : String(props.settings.monitorIndex())}
                  onChange={(e) => {
                    const v = e.currentTarget.value;
                    props.settings.setMonitorIndex(v === "current" ? null : Number(v));
                  }}
                  class={SELECT_CLASS}
                >
                  <option value="current">Current monitor</option>
                  <For each={monitors() ?? []}>
                    {(m) => <option value={String(m.index)}>{monitorLabel(m)}</option>}
                  </For>
                </select>
              </label>

              <label class="block space-y-1">
                <span class="text-xs text-(--color-oa-ink-dim)">Shell mode</span>
                <select
                  value={props.settings.shellModePref()}
                  onChange={(e) =>
                    props.settings.setShellModePref(e.currentTarget.value as ShellMode)
                  }
                  class={SELECT_CLASS}
                >
                  <For each={SHELL_OPTIONS}>
                    {(mode) => <option value={mode}>{SHELL_MODE_LABELS[mode]}</option>}
                  </For>
                </select>
                <Show
                  when={props.settings.activeShellMode() !== props.settings.shellModePref()}
                  fallback={
                    <span class="block text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                      Takes effect on next launch.
                    </span>
                  }
                >
                  <span class="block text-[0.65rem] uppercase tracking-widest text-(--color-system-accent)">
                    Active this session: {SHELL_MODE_LABELS[props.settings.activeShellMode()]}.
                    The <code class="font-mono normal-case">OA_SHELL_MODE</code> env var is
                    overriding the saved preference. Unset it to use the saved value next launch.
                  </span>
                </Show>
              </label>

              <label class="block space-y-1">
                <span class="text-xs text-(--color-oa-ink-dim)">Run-ahead frames</span>
                <div class="flex items-center gap-3">
                  <input
                    type="range"
                    min="0"
                    max="5"
                    step="1"
                    value={props.settings.runAheadFrames()}
                    onInput={(e) => {
                      const v = Number(e.currentTarget.value);
                      if (Number.isInteger(v)) props.settings.setRunAheadFrames(v);
                    }}
                    class="flex-1"
                  />
                  <span class="font-mono text-sm w-12 text-right tabular-nums">
                    {props.settings.runAheadFrames() === 0 ? "off" : `+${props.settings.runAheadFrames()}f`}
                  </span>
                </div>
                <span class="block text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                  Reduces perceived input latency by N frames. Each costs 1 save_state + 1 run_frame + 1
                  load_state per frame — cheap on PCE / NES, may exceed budget on heavier cores. Skipped
                  during scrub / TAS / pause / fast-forward / slow-mo.
                </span>
              </label>

              <label class="block space-y-1">
                <span class="text-xs text-(--color-oa-ink-dim)">Phosphor bloom amount</span>
                <div class="flex items-center gap-3">
                  <input
                    type="range"
                    min="0"
                    max="1"
                    step="0.05"
                    value={props.settings.bloomAmount()}
                    onInput={(e) => {
                      const v = Number(e.currentTarget.value);
                      if (Number.isFinite(v)) props.settings.setBloomAmount(v);
                    }}
                    class="flex-1"
                  />
                  <span class="font-mono text-sm w-12 text-right tabular-nums">
                    {props.settings.bloomAmount().toFixed(2)}
                  </span>
                </div>
                <span class="block text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                  OA-wide default for the Phosphor preset's source/blur mix. Per-system + per-game
                  overrides win at launch. Only meaningful when the active preset's base is Phosphor.
                </span>
              </label>
            </div>
            </Show>

            <Show when={activeTab() === "gameplay"}>
            <div class="space-y-4">
              <div class="space-y-3">
                <h3 class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
                  Rewind
                </h3>
                <p class="text-xs leading-relaxed text-(--color-oa-ink-dim)">
                  Hold <kbd class="rounded border border-white/15 bg-white/[0.04] px-1.5 py-0.5 font-mono text-[0.65rem] text-(--color-oa-ink)">Backspace</kbd>
                  {" "}during gameplay to walk emulation backwards. Snapshots are captured every N frames into
                  a memory-bounded ring; older snapshots are evicted when the cap is reached.
                </p>

                <label class="flex cursor-pointer items-center gap-3 rounded-md border border-white/10 bg-white/[0.03] px-3 py-2.5 text-sm transition hover:bg-white/[0.06]">
                  <input
                    type="checkbox"
                    checked={props.settings.rewindEnabled()}
                    onChange={(e) => props.settings.setRewindEnabled(e.currentTarget.checked)}
                    class="size-4 cursor-pointer accent-(--color-system-accent)"
                  />
                  <span class="flex-1">
                    <span class="block text-(--color-oa-ink)">Enable rewind</span>
                    <span class="block text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                      Captures save-state snapshots while playing
                    </span>
                  </span>
                </label>

                <label class="block space-y-1">
                  <span class="text-xs text-(--color-oa-ink-dim)">Capture interval (frames between snapshots)</span>
                  <select
                    value={String(props.settings.rewindCaptureIntervalFrames())}
                    onChange={(e) =>
                      props.settings.setRewindCaptureIntervalFrames(Number(e.currentTarget.value))
                    }
                    class={SELECT_CLASS}
                    disabled={!props.settings.rewindEnabled()}
                  >
                    <For each={REWIND_INTERVAL_OPTIONS}>
                      {(n) => (
                        <option value={String(n)}>
                          {n === 1 ? "Every frame" : `Every ${n} frames`}
                          {" "}
                          (~{Math.round((n / 60) * 1000)} ms at 60 fps)
                        </option>
                      )}
                    </For>
                  </select>
                  <span class="block text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                    Lower = smoother rewind, more CPU + RAM. Higher = coarser rewind, cheaper.
                  </span>
                </label>

                <label class="block space-y-1">
                  <span class="text-xs text-(--color-oa-ink-dim)">Buffer cap</span>
                  <select
                    value={String(props.settings.rewindBufferMegabytes())}
                    onChange={(e) =>
                      props.settings.setRewindBufferMegabytes(Number(e.currentTarget.value))
                    }
                    class={SELECT_CLASS}
                    disabled={!props.settings.rewindEnabled()}
                  >
                    <For each={REWIND_BUFFER_OPTIONS}>
                      {(mb) => <option value={String(mb)}>{mb} MB</option>}
                    </For>
                  </select>
                  <span class="block text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                    Hard cap on RAM. Cores with large save states (SNES ~300 KB / snap) get fewer
                    seconds of history per MB.
                  </span>
                </label>
              </div>
            </div>
            </Show>

            <Show when={activeTab() === "cores"}>
            <div class="space-y-3">
              <div class="flex items-center justify-between">
                <h3 class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
                  Cores
                </h3>
                <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                  in <code class="font-mono normal-case">&lt;exe_dir&gt;/cores/</code>
                </span>
              </div>
              <Show
                when={(cores() ?? []).length > 0}
                fallback={
                  <p class="text-xs leading-relaxed text-(--color-oa-ink-dim)">
                    No libretro cores detected. Download a core .dll from
                    {" "}
                    <code class="font-mono normal-case">buildbot.libretro.com/nightly/</code>
                    {" "}
                    and drop it in the cores folder next to the .exe.
                  </p>
                }
              >
                <ul class="space-y-1">
                  <For each={cores() ?? []}>
                    {(c) => (
                      <li class="rounded border border-white/5 bg-white/[0.02] px-3 py-2 text-xs">
                        <div class="flex items-baseline justify-between gap-3">
                          <span class="font-medium text-(--color-oa-ink)">{c.libraryName}</span>
                          <span class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">{c.libraryVersion}</span>
                        </div>
                        <div class="mt-0.5 truncate font-mono text-[0.65rem] text-(--color-oa-ink-dim)">{c.fileName}</div>
                        <div class="mt-0.5 text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                          ext: {c.validExtensions || "—"}
                        </div>
                      </li>
                    )}
                  </For>
                </ul>
              </Show>

              <For each={systemIds}>
                {(id) => (
                  <label class="block space-y-1">
                    <span class="text-xs text-(--color-oa-ink-dim)">
                      Default core for {systemThemes[id].shortName}
                    </span>
                    <select
                      value={corePrefs()[id] ?? "__default__"}
                      onChange={(e) => {
                        const v = e.currentTarget.value;
                        void setCorePref(id, v === "__default__" ? null : v);
                      }}
                      class={SELECT_CLASS}
                    >
                      <option value="__default__">Default (auto-detect)</option>
                      <For each={cores() ?? []}>
                        {(c) => <option value={c.fileName}>{c.libraryName} ({c.fileName})</option>}
                      </For>
                    </select>
                    <span class="block text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                      Takes effect on next launch.
                    </span>
                  </label>
                )}
              </For>
            </div>
            </Show>

            <Show when={activeTab() === "media"}>
            <div class="space-y-3">
              <div class="flex items-center justify-between">
                <h3 class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
                  Game media
                </h3>
                <button
                  type="button"
                  onClick={(e) => {
                    e.currentTarget.blur();
                    void invoke("open_media_folder");
                  }}
                  class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
                >
                  Open folder
                </button>
              </div>

              {/* Only-sync-identified gate. Default on — the fuzzy
                  filename matcher produced a lot of wrong-art mismatches
                  in the field, especially for repacked / renamed sets.
                  When this is on, only ROMs whose sha1 matched a
                  libretro-database canonical entry (via Identify ROMs)
                  get their art synced. Turn it off to fall back to
                  fuzzy filename matching against ALL library entries
                  at the strict 0.95 threshold. */}
              <label class="flex cursor-pointer items-start gap-3 rounded border border-white/5 bg-white/[0.02] px-3 py-2 text-xs text-(--color-oa-ink)">
                <input
                  type="checkbox"
                  checked={onlySyncIdentified()}
                  onChange={(e) => void setOnlySyncIdentifiedPref(e.currentTarget.checked)}
                  class="mt-0.5 h-3.5 w-3.5 accent-(--color-system-accent)"
                />
                <span class="flex-1">
                  <span class="block font-medium">
                    Only sync media for identified ROMs (recommended)
                  </span>
                  <span class="block text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                    Skip ROMs that haven't been hash-identified via Identify ROMs. Stops the fuzzy
                    filename matcher from producing wrong-art mismatches on repacked or renamed sets.
                  </span>
                </span>
              </label>

              {/* Kinds to fetch — controls which libretro-thumbnails subdirs
                  the sync pulls per ROM. Defaults to all three; users on
                  metered connections can drop the extras to cut sync
                  bandwidth ~3×. */}
              <div class="space-y-1">
                <p class="text-xs text-(--color-oa-ink-dim)">
                  Kinds to fetch (per-ROM downloads during sync)
                </p>
                <div class="flex flex-wrap gap-2">
                  <For each={["boxart", "snap", "title"] as const}>
                    {(k) => {
                      const checked = () => media.kindsToFetch().includes(k);
                      const label =
                        k === "boxart" ? "Boxart"
                        : k === "snap" ? "Snapshots"
                        : "Title screens";
                      return (
                        <label class="flex cursor-pointer items-center gap-2 rounded border border-white/10 bg-white/[0.04] px-3 py-1.5 text-xs text-(--color-oa-ink) transition hover:bg-white/[0.08]">
                          <input
                            type="checkbox"
                            checked={checked()}
                            onChange={(e) => {
                              const enabled = e.currentTarget.checked;
                              const current = media.kindsToFetch();
                              const next = enabled
                                ? Array.from(new Set([...current, k]))
                                : current.filter((x) => x !== k);
                              void media.setKindsToFetch(next);
                            }}
                            class="h-3.5 w-3.5 accent-(--color-system-accent)"
                          />
                          <span>{label}</span>
                        </label>
                      );
                    }}
                  </For>
                </div>
              </div>

              {/* Per-system sync rows. Each shows a Sync button + the last
                  emitted progress line + an inline bar. The button disables
                  while that system is syncing. */}
              <For each={systemIds}>
                {(id) => {
                  const prog = () => syncProgress()[id];
                  const isSyncing = () => syncing()[id] === true;
                  const metaProg = () => metaProgress()[id];
                  const isMetaSyncing = () => metaSyncing()[id] === true;
                  const pct = () => {
                    const p = prog();
                    if (!p || p.total === 0) return 0;
                    return Math.round((p.done / p.total) * 100);
                  };
                  const metaPct = () => {
                    const p = metaProg();
                    if (!p || p.total === 0) return 0;
                    return Math.round((p.done / p.total) * 100);
                  };
                  return (
                    <div class="space-y-1 rounded border border-white/5 bg-white/[0.02] px-3 py-2" data-system={id}>
                      <div class="flex items-center justify-between gap-3">
                        <span class="text-xs font-medium text-(--color-oa-ink)">
                          {systemThemes[id].displayName}
                        </span>
                        <span class="flex gap-1.5">
                          <button
                            type="button"
                            disabled={isSyncing()}
                            onClick={(e) => {
                              e.currentTarget.blur();
                              void startSync(id);
                            }}
                            class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink) disabled:cursor-not-allowed disabled:opacity-50"
                          >
                            {isSyncing() ? "Syncing…" : "Sync media"}
                          </button>
                          <button
                            type="button"
                            disabled={isMetaSyncing()}
                            onClick={(e) => {
                              e.currentTarget.blur();
                              void startMetadataSync(id);
                            }}
                            class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink) disabled:cursor-not-allowed disabled:opacity-50"
                          >
                            {isMetaSyncing() ? "Syncing…" : "Sync metadata"}
                          </button>
                          <button
                            type="button"
                            disabled={hashSyncing()[id] === true}
                            onClick={(e) => {
                              e.currentTarget.blur();
                              void startHashSync(id);
                            }}
                            class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink) disabled:cursor-not-allowed disabled:opacity-50"
                            title="Fetch libretro-database hash DB for this system"
                          >
                            {hashSyncing()[id] ? "Syncing…" : "Sync hashes"}
                          </button>
                          <button
                            type="button"
                            disabled={hashResolving()[id] === true}
                            onClick={(e) => {
                              e.currentTarget.blur();
                              void startHashResolve(id);
                            }}
                            class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink) disabled:cursor-not-allowed disabled:opacity-50"
                            title="Hash every ROM in this system and rename to the canonical title on a match"
                          >
                            {hashResolving()[id] ? "Identifying…" : "Identify ROMs"}
                          </button>
                        </span>
                      </div>
                      <Show when={prog()}>
                        {(p) => (
                          <>
                            <div class="h-1 w-full overflow-hidden rounded-full bg-white/5">
                              <div
                                class="h-full bg-(--color-system-accent) transition-[width] duration-200"
                                style={{ width: `${pct()}%` }}
                              />
                            </div>
                            <p class="truncate text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                              media · {p().done}/{p().total} · {p().currentRomTitle || p().lastAction}
                            </p>
                          </>
                        )}
                      </Show>
                      <Show when={metaProg()}>
                        {(p) => (
                          <>
                            <div class="h-1 w-full overflow-hidden rounded-full bg-white/5">
                              <div
                                class="h-full bg-(--color-system-accent) transition-[width] duration-200"
                                style={{ width: `${metaPct()}%` }}
                              />
                            </div>
                            <p class="truncate text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                              metadata · {p().done}/{p().total} · {p().currentRomTitle || p().lastAction}
                            </p>
                          </>
                        )}
                      </Show>
                      <Show when={hashSyncSummary()[id]}>
                        {(s) => (
                          <p class="truncate text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                            hashes ·
                            {s().written > 0
                              ? ` ${s().written} canonical entries indexed${s().fromCache ? " (from cache)" : ""}`
                              : " no upstream dat for this system"}
                          </p>
                        )}
                      </Show>
                      <Show when={hashResolveProgress()[id]}>
                        {(p) => {
                          const pct = (): number => {
                            const v = p();
                            return v.total === 0 ? 0 : Math.round((v.done / v.total) * 100);
                          };
                          return (
                            <>
                              <div class="h-1 w-full overflow-hidden rounded-full bg-white/5">
                                <div
                                  class="h-full bg-(--color-system-accent) transition-[width] duration-200"
                                  style={{ width: `${pct()}%` }}
                                />
                              </div>
                              <p class="truncate text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                                identify · {p().done}/{p().total} · {p().currentTitle || p().lastAction}
                              </p>
                            </>
                          );
                        }}
                      </Show>
                      <Show when={hashResolveSummary()[id]}>
                        {(s) => (
                          <p class="truncate text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                            {s().canonicalEntries === 0
                              ? "identify · libretro-database has no hash DB for this system"
                              : `identify done · ${s().matched} matched · ${s().unmatched} unknown · ${s().skippedCd} CD-skipped · ${s().errors} errors (${s().canonicalEntries} canonical entries in DB)`}
                          </p>
                        )}
                      </Show>
                    </div>
                  );
                }}
              </For>

              {/* Region priority — ordered list with up/down/remove + add. */}
              <div class="space-y-1">
                <p class="text-xs text-(--color-oa-ink-dim)">
                  Region priority (first match wins)
                </p>
                <ul class="space-y-1">
                  <For each={regionDraft()}>
                    {(region, i) => (
                      <li class="flex items-center justify-between gap-2 rounded border border-white/5 bg-white/[0.02] px-3 py-1.5 text-xs">
                        <span class="text-(--color-oa-ink)">{region}</span>
                        <span class="flex gap-1">
                          <button
                            type="button"
                            disabled={i() === 0}
                            onClick={(e) => {
                              e.currentTarget.blur();
                              moveRegion(i(), -1);
                            }}
                            class="rounded border border-white/10 bg-white/[0.04] px-1.5 py-0.5 text-[0.6rem] text-(--color-oa-ink-dim) hover:bg-white/[0.08] hover:text-(--color-oa-ink) disabled:cursor-not-allowed disabled:opacity-40"
                            aria-label="Move up"
                          >
                            ↑
                          </button>
                          <button
                            type="button"
                            disabled={i() === regionDraft().length - 1}
                            onClick={(e) => {
                              e.currentTarget.blur();
                              moveRegion(i(), 1);
                            }}
                            class="rounded border border-white/10 bg-white/[0.04] px-1.5 py-0.5 text-[0.6rem] text-(--color-oa-ink-dim) hover:bg-white/[0.08] hover:text-(--color-oa-ink) disabled:cursor-not-allowed disabled:opacity-40"
                            aria-label="Move down"
                          >
                            ↓
                          </button>
                          <button
                            type="button"
                            onClick={(e) => {
                              e.currentTarget.blur();
                              removeRegion(i());
                            }}
                            class="rounded border border-white/10 bg-white/[0.04] px-1.5 py-0.5 text-[0.6rem] text-(--color-oa-ink-dim) hover:bg-red-500/15 hover:text-(--color-oa-ink)"
                            aria-label="Remove"
                          >
                            ×
                          </button>
                        </span>
                      </li>
                    )}
                  </For>
                </ul>
                <select
                  onChange={(e) => {
                    const v = e.currentTarget.value;
                    if (v) addRegion(v);
                    e.currentTarget.value = "";
                  }}
                  class={SELECT_CLASS}
                >
                  <option value="">+ Add region…</option>
                  <Show when={!regionDraft().includes("USA")}><option value="USA">USA</option></Show>
                  <Show when={!regionDraft().includes("Japan")}><option value="Japan">Japan</option></Show>
                  <Show when={!regionDraft().includes("Europe")}><option value="Europe">Europe</option></Show>
                  <Show when={!regionDraft().includes("World")}><option value="World">World</option></Show>
                  <Show when={!regionDraft().includes("Asia")}><option value="Asia">Asia</option></Show>
                  <Show when={!regionDraft().includes("Korea")}><option value="Korea">Korea</option></Show>
                </select>
              </div>

              {/* Disk usage */}
              <Show when={storageStats()}>
                {(s) => (
                  <p class="text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                    Disk: {humanBytes(s().coversBytes)} covers · {humanBytes(s().thumbsBytes)} thumbs · {humanBytes(s().cacheBytes)} cache · {humanBytes(s().totalBytes)} total
                  </p>
                )}
              </Show>
            </div>
            </Show>

            <Show when={activeTab() === "audio"}>
            <div class="space-y-3">
              <h3 class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
                Audio
              </h3>
              <label class="block space-y-1">
                <span class="text-xs text-(--color-oa-ink-dim)">Output device</span>
                <select
                  value={props.settings.audioDevice() ?? "__default__"}
                  onChange={(e) => {
                    const v = e.currentTarget.value;
                    props.settings.setAudioDevice(v === "__default__" ? null : v);
                  }}
                  class={SELECT_CLASS}
                >
                  <option value="__default__">System default</option>
                  <For each={audioDevices() ?? []}>
                    {(d) => (
                      <option value={d.name}>
                        {d.name}
                        {d.isDefault ? " (default)" : ""}
                      </option>
                    )}
                  </For>
                </select>
                <span class="block text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                  Takes effect immediately — the running stream swaps in place.
                </span>
              </label>
            </div>
            </Show>

            <Show when={activeTab() === "library"}>
            <div class="space-y-3">
              <div class="flex items-center justify-between">
                <h3 class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
                  Library folders
                </h3>
                <div class="flex gap-2">
                  <button
                    type="button"
                    onClick={(e) => {
                      e.currentTarget.blur();
                      props.onAddLibraryFolder();
                    }}
                    class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-[0.65rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
                  >
                    Add
                  </button>
                  <button
                    type="button"
                    onClick={(e) => {
                      e.currentTarget.blur();
                      props.onRescanLibraryFolders();
                    }}
                    disabled={props.settings.libraryFolders().length === 0}
                    class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-[0.65rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink) disabled:cursor-not-allowed disabled:opacity-50"
                  >
                    Rescan all
                  </button>
                </div>
              </div>
              <Show
                when={props.settings.libraryFolders().length > 0}
                fallback={
                  <p class="text-xs text-(--color-oa-ink-dim)">
                    No folders tracked. Add one with the button above or use Pick folder in the header.
                  </p>
                }
              >
                <ul class="space-y-1">
                  <For each={props.settings.libraryFolders()}>
                    {(folder) => (
                      <li class="flex items-center justify-between gap-3 rounded border border-white/5 bg-white/[0.02] px-3 py-2 text-xs">
                        <span class="truncate font-mono text-(--color-oa-ink)" title={folder}>
                          {folder}
                        </span>
                        <button
                          type="button"
                          onClick={(e) => {
                            e.currentTarget.blur();
                            props.settings.setLibraryFolders(
                              props.settings.libraryFolders().filter((f) => f !== folder),
                            );
                          }}
                          class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
                        >
                          Remove
                        </button>
                      </li>
                    )}
                  </For>
                </ul>
              </Show>

              {/* --- Sidebar systems (LaunchBox-equivalent) --- */}
              <div class="mt-6 space-y-2">
                <h3 class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
                  Sidebar systems
                </h3>
                <label class="flex items-center gap-2 text-xs text-(--color-oa-ink)">
                  <input
                    type="checkbox"
                    checked={props.layout.autoHideEmptySystems()}
                    onChange={(e) => props.layout.setAutoHideEmptySystems(e.currentTarget.checked)}
                  />
                  <span>Auto-hide systems with no games</span>
                </label>
                <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                  Uncheck a system to hide it from the left sidebar. Hidden systems still live
                  in the registry; per-system files (bindings, settings) are preserved.
                </p>
                <ul class="space-y-1">
                  <For each={Object.keys(systemThemes) as SystemId[]}>
                    {(id) => {
                      const theme = systemThemes[id];
                      const count = () =>
                        props.library.state.entries.filter((e) => e.systemId === id && !e.seed).length;
                      const hidden = () => props.layout.hiddenSystems().includes(id);
                      return (
                        <li class="flex items-center gap-3 rounded border border-white/5 bg-white/[0.02] px-3 py-2 text-xs">
                          <input
                            type="checkbox"
                            checked={!hidden()}
                            onChange={(e) => {
                              const list = props.layout.hiddenSystems();
                              if (e.currentTarget.checked) {
                                props.layout.setHiddenSystems(list.filter((s) => s !== id));
                              } else if (!list.includes(id)) {
                                props.layout.setHiddenSystems([...list, id]);
                              }
                            }}
                          />
                          <span class="flex-1 truncate text-(--color-oa-ink)">{theme.displayName}</span>
                          <span class="text-(--color-oa-ink-dim) tabular-nums">{count()}</span>
                        </li>
                      );
                    }}
                  </For>
                </ul>
              </div>

              {/* --- Library cleanup --- */}
              <div class="mt-6 space-y-2">
                <h3 class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
                  Cleanup
                </h3>
                <label class="flex items-center gap-2 text-xs text-(--color-oa-ink)">
                  <input
                    type="checkbox"
                    checked={props.settings.autoRemoveOnDelete()}
                    onChange={(e) => props.settings.setAutoRemoveOnDelete(e.currentTarget.checked)}
                  />
                  <span>Auto-remove from library when the file is deleted</span>
                </label>
                <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                  Off (default) keeps library entries when files vanish — useful for moves /
                  renames. On removes the matching row when the watcher reports the file gone.
                </p>

                <div class="mt-3 flex flex-wrap items-center gap-2">
                  <span class="text-xs text-(--color-oa-ink-dim)">Clear games for:</span>
                  <select
                    class={SELECT_CLASS}
                    value=""
                    onChange={async (e) => {
                      const id = e.currentTarget.value as SystemId;
                      e.currentTarget.value = "";
                      if (!id) return;
                      const theme = systemThemes[id];
                      if (!window.confirm(
                        `Remove all ${theme.displayName} games from the library? Files on disk are NOT touched.`,
                      )) return;
                      const n = await props.library.clearForSystem(id);
                      window.alert(`Removed ${n} game${n === 1 ? "" : "s"} from ${theme.displayName}.`);
                    }}
                  >
                    <option value="" disabled>(pick a system)</option>
                    <For each={Object.keys(systemThemes) as SystemId[]}>
                      {(id) => <option value={id}>{systemThemes[id].displayName}</option>}
                    </For>
                  </select>
                </div>

                <div class="mt-3 rounded border border-red-500/30 bg-red-500/[0.05] p-3">
                  <p class="text-xs text-red-300 font-medium">Danger zone</p>
                  <p class="mt-1 text-[0.65rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                    Resets the entire game library. Files on disk are NOT touched — the
                    library DB just forgets everything. Re-scan or re-import folders to
                    rebuild.
                  </p>
                  <button
                    type="button"
                    onClick={async (e) => {
                      e.currentTarget.blur();
                      if (!window.confirm(
                        "Reset the entire library? Every game row will be removed from the database. Files on disk are NOT deleted.",
                      )) return;
                      await props.library.clear();
                      window.alert("Library reset. Re-scan a folder to rebuild.");
                    }}
                    class="mt-2 rounded border border-red-500/40 bg-red-500/10 px-3 py-1 text-xs uppercase tracking-wider text-red-300 transition hover:bg-red-500/20"
                  >
                    Reset entire library
                  </button>
                </div>
              </div>
            </div>
            </Show>
            </section>
          </div>
      </div>
  );
};

export default SettingsPage;
