import { createEffect, createResource, createSignal, For, onCleanup, onMount, Show, type Component } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  closestCenter,
  createSortable,
  DragDropProvider,
  DragDropSensors,
  SortableProvider,
  transformStyle,
  type DragEventHandler,
} from "@thisbeyond/solid-dnd";
import type { LibraryStore } from "../library/store";
import { useMedia } from "../library/media";
import type { LayoutStore } from "../layout/state";
import type { SettingsStore } from "../settings/store";
import { systemThemes, type SystemId } from "../themes/registry";
import type { ViewsStore } from "../views/store";
import { collectHiddenContainers, findNode } from "../views/resolver";
import { platformNodeIdFor } from "../views/defaults";
import SettingRow, { selectClass } from "./SettingRow";
import ViewsManagerTab from "./ViewsManagerTab";
import { ImportArtPackDialog } from "./ImportArtPackDialog";

type Props = {
  /// Navigate back to the previous view. Used by the header Back button
  /// and by the Escape key handler. Replaces the modal-era `onClose`.
  onBack: () => void;
  settings: SettingsStore;
  library: LibraryStore;
  layout: LayoutStore;
  views: ViewsStore;
  onAddLibraryFolder: () => void;
  onRescanLibraryFolders: () => void;
  /// Optional deep-link target — Library menu items use this to land on
  /// "library" vs "media" instead of the persisted last-visited tab.
  initialTab?: "library" | "media" | "views";
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

/// One row in the region-priority sortable list. Lives at module scope
/// so each row only re-creates its own `createSortable` registration on
/// mount, not on every parent re-render. The whole row is the drag
/// affordance — no other interactive controls compete for clicks.
const SortableRegionRow: Component<{ region: string; idx: number }> = (props) => {
  const sortable = createSortable(props.region);
  return (
    <li
      ref={sortable.ref}
      style={transformStyle(sortable.transform)}
      class="flex items-center gap-2 rounded border border-white/5 bg-white/[0.02] px-3 py-1.5 text-xs select-none transition"
      classList={{
        "cursor-grab hover:border-white/15 hover:bg-white/[0.04]":
          !sortable.isActiveDraggable,
        "cursor-grabbing border-(--color-system-accent) bg-(--color-system-accent)/10 z-10 shadow-lg":
          sortable.isActiveDraggable,
      }}
      {...sortable.dragActivators}
    >
      <span class="w-4 text-center text-(--color-oa-ink-dim)" aria-hidden="true">⋮⋮</span>
      <span class="w-6 text-right tabular-nums text-(--color-oa-ink-dim)">
        {props.idx + 1}.
      </span>
      <span class="flex-1 text-(--color-oa-ink)">{props.region}</span>
    </li>
  );
};

/// One row in the media region-priority sortable list (Game media
/// tab). Mirrors the library region priority — but the underlying
/// state lives in MediaContext (file `media.json`) so the drop
/// handler calls `media.setRegionPriority` instead of writing to the
/// shared library prefs.
const SortableMediaRegionRow: Component<{
  region: string;
  idx: number;
  onRemove: (region: string) => void;
}> = (props) => {
  const sortable = createSortable(props.region);
  return (
    <li
      ref={sortable.ref}
      style={transformStyle(sortable.transform)}
      class="flex items-center justify-between gap-2 rounded border border-white/5 bg-white/[0.02] px-3 py-1.5 text-xs transition"
      classList={{
        "hover:border-white/15": !sortable.isActiveDraggable,
        "border-(--color-system-accent) bg-(--color-system-accent)/10 z-10 shadow-lg":
          sortable.isActiveDraggable,
      }}
    >
      <div class="flex flex-1 items-center gap-2">
        <span
          class="select-none px-1 text-(--color-oa-ink-dim) hover:text-(--color-oa-ink)"
          classList={{
            "cursor-grab": !sortable.isActiveDraggable,
            "cursor-grabbing": sortable.isActiveDraggable,
          }}
          role="button"
          tabindex="-1"
          aria-label={`Drag handle for ${props.region}`}
          {...sortable.dragActivators}
        >
          ⋮⋮
        </span>
        <span class="w-6 text-right tabular-nums text-(--color-oa-ink-dim)">
          {props.idx + 1}.
        </span>
        <span class="text-(--color-oa-ink)">{props.region}</span>
      </div>
      <button
        type="button"
        onClick={(e) => {
          e.currentTarget.blur();
          props.onRemove(props.region);
        }}
        class="rounded border border-white/10 bg-white/[0.04] px-1.5 py-0.5 text-[0.6rem] text-(--color-oa-ink-dim) hover:bg-red-500/15 hover:text-(--color-oa-ink)"
        aria-label={`Remove ${props.region}`}
      >
        ×
      </button>
    </li>
  );
};

/// One row in the library-folders sortable list. Drag activators live
/// on the grip handle (not the whole row) so the Remove button's
/// click event isn't captured by the drag sensor. Order matters for
/// the scan-collision policy (first-folder-wins on duplicate ROM
/// filenames) so this drag-reorder is functional, not just cosmetic.
const SortableFolderRow: Component<{
  /// SQLite folder id (e.g. `folder-1ddb42376878b631`). Used as the
  /// solid-dnd sortable id so reorder maps cleanly to a single
  /// `reorder_folders` Tauri call.
  id: string;
  folder: string;
  onRemove: (folderId: string) => void;
}> = (props) => {
  const sortable = createSortable(props.id);
  return (
    <li
      ref={sortable.ref}
      style={transformStyle(sortable.transform)}
      class="flex items-center justify-between gap-3 rounded border border-white/5 bg-white/[0.02] px-3 py-2 text-xs transition"
      classList={{
        "hover:border-white/15": !sortable.isActiveDraggable,
        "border-(--color-system-accent) bg-(--color-system-accent)/10 z-10 shadow-lg":
          sortable.isActiveDraggable,
      }}
    >
      <span
        class="select-none px-1 text-(--color-oa-ink-dim) hover:text-(--color-oa-ink)"
        classList={{
          "cursor-grab": !sortable.isActiveDraggable,
          "cursor-grabbing": sortable.isActiveDraggable,
        }}
        role="button"
        tabindex="-1"
        aria-label={`Drag handle for ${props.folder}`}
        {...sortable.dragActivators}
      >
        ⋮⋮
      </span>
      <span class="flex-1 truncate font-mono text-(--color-oa-ink)" title={props.folder}>
        {props.folder}
      </span>
      <button
        type="button"
        onClick={(e) => {
          e.currentTarget.blur();
          props.onRemove(props.id);
        }}
        class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
      >
        Remove
      </button>
    </li>
  );
};

// After the menu-bar redesign, this page only renders the Library +
// Game-media tabs. The other tabs (Display / Audio / Gameplay / Shaders)
// became top-bar Settings menu dialogs; Presentation moved to View menu;
// Cores moved to the dedicated CoresPage. Library + Media stay full-page
// because of the wide editors (folder lists, per-system sync rows with
// progress bars, region priority editor, disk-usage panel).
const TABS = ["library", "views", "media"] as const;
type TabId = typeof TABS[number];
const TAB_LABELS: Record<TabId, string> = {
  library:      "Library",
  views:        "Views",
  media:        "Game media",
};
const TAB_HINTS: Record<TabId, string> = {
  library:      "Tracked folders, scanning, ingest",
  views:        "Sidebar tree views — create, rename, switch active",
  media:        "Cover art + snapshots + titles, libretro-thumbnails sync, region priority",
};

const LibraryManagerPage: Component<Props> = (props) => {
  // Tabbed layout — sidebar nav on the left, per-tab content on the right.
  // Persists the last-selected tab in localStorage so reopening the modal
  // lands you where you were.
  const TAB_STORAGE = "oa.library.activeTab";
  const initialTab: TabId = (() => {
    if (props.initialTab && TABS.includes(props.initialTab as TabId)) return props.initialTab as TabId;
    const saved = localStorage.getItem(TAB_STORAGE) as TabId | null;
    return saved && TABS.includes(saved) ? saved : "library";
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

  const systemIds = Object.keys(systemThemes) as SystemId[];

  // --- Library prefs: region + revision priority for the multi-variant
  //     grouping. Fetched once at mount; writes go straight through to
  //     `set_library_prefs` then re-fetch the groups so tiles re-rank.
  type RevisionPriority = "newest" | "oldest";
  type LibraryPrefs = { regionPriority: string[]; revisionPriority: RevisionPriority };
  const [libraryPrefs, setLibraryPrefs] = createSignal<LibraryPrefs>({
    regionPriority: ["USA", "World", "Europe", "Japan", "Asia", "Other"],
    revisionPriority: "newest",
  });
  onMount(() => {
    invoke<LibraryPrefs>("get_library_prefs")
      .then((p) => setLibraryPrefs(p))
      .catch((e) => console.warn("get_library_prefs failed:", e));
  });
  async function persistLibraryPrefs(next: LibraryPrefs) {
    setLibraryPrefs(next);
    try {
      await invoke("set_library_prefs", { prefs: next });
      // Re-rank tiles so the change is visible immediately.
      await props.library.refreshGroups();
    } catch (e) {
      console.warn("set_library_prefs failed:", e);
    }
  }
  /// solid-dnd onDragEnd for the region-priority list. `draggable.id`
  /// is the region being moved; `droppable.id` is the row it was
  /// dropped on. Swap them by removing-and-reinserting so the rest of
  /// the order stays stable (vs. a pairwise swap which would scramble
  /// when crossing multiple rows).
  const handleRegionDragEnd: DragEventHandler = ({ draggable, droppable }) => {
    if (!draggable || !droppable) return;
    const prefs = libraryPrefs();
    const fromIdx = prefs.regionPriority.indexOf(draggable.id as string);
    const toIdx = prefs.regionPriority.indexOf(droppable.id as string);
    if (fromIdx === -1 || toIdx === -1 || fromIdx === toIdx) return;
    const next = [...prefs.regionPriority];
    const [moved] = next.splice(fromIdx, 1);
    next.splice(toIdx, 0, moved);
    void persistLibraryPrefs({ ...prefs, regionPriority: next });
  };
  /// solid-dnd onDragEnd for the library-folders list. Drag ids are
  /// the SQLite folder ids (stable across reorder) so a reorder maps
  /// to a single `reorder_folders` Tauri call without ambiguity.
  const handleFolderDragEnd: DragEventHandler = ({ draggable, droppable }) => {
    if (!draggable || !droppable) return;
    const rows = props.settings.libraryFolderRows();
    const fromIdx = rows.findIndex((r) => r.id === draggable.id);
    const toIdx = rows.findIndex((r) => r.id === droppable.id);
    if (fromIdx === -1 || toIdx === -1 || fromIdx === toIdx) return;
    const next = [...rows];
    const [moved] = next.splice(fromIdx, 1);
    next.splice(toIdx, 0, moved);
    void props.settings.reorderLibraryFolderIds(next.map((r) => r.id));
  };
  function removeLibraryFolder(folderId: string) {
    void props.settings.removeLibraryFolderById(folderId);
  }
  function resetLibraryRegionPriority() {
    void persistLibraryPrefs({
      ...libraryPrefs(),
      regionPriority: ["USA", "World", "Europe", "Japan", "Asia", "Other"],
    });
  }

  // --- Game media (covers): sync, region priority, storage stats ---

  const media = useMedia();

  // Phase 3 (2026-05-23): art-pack importer dialog open-state. Pulled
  // up to LibraryManagerPage so the "Import art pack…" button in the
  // media tab can open it.
  const [importArtPackOpen, setImportArtPackOpen] = createSignal(false);

  // Per-system live sync progress, keyed by systemId. Updated by oa://library-sync events.
  const [syncProgress, setSyncProgress] = createSignal<Record<string, SyncProgressPayload>>({});
  // Per-system "is sync running" flag, used to disable the button.
  const [syncing, setSyncing] = createSignal<Record<string, boolean>>({});
  // Same shape for the per-system metadata sync — independent signals so
  // the two buttons can run independently and don't share progress state.
  const [metaProgress, setMetaProgress] = createSignal<Record<string, SyncProgressPayload>>({});
  const [metaSyncing, setMetaSyncing] = createSignal<Record<string, boolean>>({});
  // Per-system metadata-clear loading state. No progress bar — the
  // server-side call is O(N) over library_db ids and completes in well
  // under a second even on large libraries; a binary "clearing…" pill
  // is enough. metaClearStatus is the last-action line shown under the
  // button row (same shape as the other Sync status lines); empty
  // string = no status to show yet.
  const [metaClearing, setMetaClearing] = createSignal<Record<string, boolean>>({});
  const [metaClearStatus, setMetaClearStatus] = createSignal<Record<string, string>>({});
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
      console.warn("LibraryManagerPage: listen('oa://library-(metadata-)sync*') failed:", e);
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
      setSyncProgress((prev) => ({
        ...prev,
        [systemId]: { systemId, done: 0, total: entries.length, currentRomTitle: "", lastAction: `error: ${e}` },
      }));
    } finally {
      // H10 belt-and-braces — clear the loading state regardless of
      // outcome and regardless of whether the oa://library-sync-complete
      // event listener actually fires. Pre-fix, an early-return on the
      // Rust side (e.g. entries.len()=0 before the emit) or a dropped
      // event left the button stuck on "Syncing…" forever.
      setSyncing((prev) => ({ ...prev, [systemId]: false }));
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
    /// Library row count for this system at resolve time. Compared
    /// against `alreadyIdentified` to disambiguate the "0/0" no-op
    /// case (re-run on already-identified library → "all N identified")
    /// from "no games in library."
    libraryTotal: number;
    /// Subset of `libraryTotal` whose sha1 is already stamped from a
    /// prior successful Identify pass.
    alreadyIdentified: number;
  };
  const [hashSyncing, setHashSyncing] = createSignal<Record<string, boolean>>({});
  const [hashSyncSummary, setHashSyncSummary] = createSignal<Record<string, HashSyncSummaryPayload>>({});
  const [hashResolving, setHashResolving] = createSignal<Record<string, boolean>>({});
  const [hashResolveProgress, setHashResolveProgress] =
    createSignal<Record<string, HashResolveProgressPayload>>({});
  const [hashResolveSummary, setHashResolveSummary] =
    createSignal<Record<string, HashResolveSummaryPayload>>({});

  /// Cross-button gating (H9). Any per-system sync op in flight
  /// disables ALL of that system's buttons (Sync media, Sync metadata,
  /// Clear metadata, Sync hashes, Identify ROMs). The server-side
  /// per-system mutex (H11) already serializes concurrent ops, but
  /// disabling the UI prevents the operator from queuing up confusing
  /// button-mash sequences and stops fast-fingers from hitting the
  /// same op twice while the first is still in flight.
  function isSystemBusy(id: SystemId): boolean {
    return (
      syncing()[id] === true ||
      metaSyncing()[id] === true ||
      metaClearing()[id] === true ||
      hashSyncing()[id] === true ||
      hashResolving()[id] === true
    );
  }

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
        console.warn("LibraryManagerPage: hash listen failed:", e);
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
    } finally {
      // H10 belt-and-braces — see startSync. The
      // oa://rom-hashes-synced listener also clears this signal on the
      // happy path (line ~498), so this is just the safety net for
      // missed-event cases.
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
      // H10 — surface the error in the status line so the operator
      // sees that something went wrong, not just a button that
      // suddenly reverted to "Identify ROMs".
      setHashResolveProgress((p) => ({
        ...p,
        [systemId]: {
          systemId,
          done: 0,
          total: 0,
          currentTitle: "",
          lastAction: `error: ${e}`,
        },
      }));
    } finally {
      // H10 belt-and-braces — see startSync. The
      // oa://rom-hash-resolve-complete listener also clears this on
      // the happy path.
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
      setMetaProgress((prev) => ({
        ...prev,
        [systemId]: { systemId, done: 0, total: entries.length, currentRomTitle: "", lastAction: `error: ${e}` },
      }));
    } finally {
      // H10 belt-and-braces — see startSync.
      setMetaSyncing((prev) => ({ ...prev, [systemId]: false }));
    }
  }

  /// Wipe the `metadata` slot on every media_db entry whose game is
  /// tagged with `systemId`. Used to scrub the cross-system data
  /// contamination from the pre-2026-05-21 metadata-routing bug
  /// (Genesis games were getting PC Engine metadata stamped onto them).
  /// Cover-art / snap / title variants are NOT touched.
  async function startClearMetadata(systemId: SystemId) {
    const theme = systemThemes[systemId];
    const name = theme.displayName;
    const count = props.library.state.entries.filter(
      (e) => e.systemId === systemId && !e.seed,
    ).length;
    if (count === 0) {
      // No games for this system in the library yet — surface that as
      // an inline status rather than an alert (Tauri 2's dialog plugin
      // gates window.alert behind an ACL we don't always carry, and a
      // status line matches the other Sync buttons' feedback style).
      setMetaClearStatus((prev) => ({ ...prev, [systemId]: `no ${name} games in library` }));
      return;
    }
    if (
      !window.confirm(
        `Clear metadata (genre / developer / publisher / year / players) for ${count} ${name} game(s)?\n\n` +
          `Cover art, snapshots, and title screens will NOT be touched. ` +
          `Use this after the 2026-05-21 metadata-routing fix to scrub stale cross-system data; ` +
          `re-run "Sync metadata" afterwards to repopulate against the correct upstream catalog.`,
      )
    ) {
      return;
    }
    setMetaClearing((prev) => ({ ...prev, [systemId]: true }));
    setMetaClearStatus((prev) => ({ ...prev, [systemId]: "clearing…" }));
    try {
      const result = await invoke<{ systemId: string; scanned: number; cleared: number }>(
        "clear_metadata_for_system",
        { systemId },
      );
      await media.refreshAll();
      setMetaClearStatus((prev) => ({
        ...prev,
        [systemId]: `cleared ${result.cleared} of ${result.scanned} game(s)`,
      }));
    } catch (e) {
      console.warn("clear_metadata_for_system failed:", e);
      setMetaClearStatus((prev) => ({ ...prev, [systemId]: `error: ${String(e)}` }));
    } finally {
      setMetaClearing((prev) => ({ ...prev, [systemId]: false }));
    }
  }

  // Region priority list — bound to MediaContext (which hydrates from Rust).
  const [regionDraft, setRegionDraft] = createSignal<string[]>([]);
  createEffect(() => {
    // Mirror context value into local mutable draft for reordering UX.
    setRegionDraft(media.regionPriority());
  });
  function removeRegionByName(name: string) {
    setRegionDraft((prev) => {
      const next = prev.filter((r) => r !== name);
      void media.setRegionPriority(next);
      return next;
    });
  }
  const handleMediaRegionDragEnd: DragEventHandler = ({ draggable, droppable }) => {
    if (!draggable || !droppable) return;
    setRegionDraft((prev) => {
      const fromIdx = prev.indexOf(draggable.id as string);
      const toIdx = prev.indexOf(droppable.id as string);
      if (fromIdx === -1 || toIdx === -1 || fromIdx === toIdx) return prev;
      const next = [...prev];
      const [moved] = next.splice(fromIdx, 1);
      next.splice(toIdx, 0, moved);
      void media.setRegionPriority(next);
      return next;
    });
  };
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
    <>
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
              Library Manager
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
            <Show when={activeTab() === "views"}>
              <ViewsManagerTab views={props.views} library={props.library} />
            </Show>
            <Show when={activeTab() === "media"}>
            <div class="space-y-3">
              <div class="flex items-center justify-between">
                <h3 class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
                  Game media
                </h3>
                <div class="flex items-center gap-2">
                  <button
                    type="button"
                    onClick={(e) => {
                      e.currentTarget.blur();
                      setImportArtPackOpen(true);
                    }}
                    class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
                  >
                    Import art pack…
                  </button>
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
              </div>

              {/* Only-sync-identified gate. Default on — the fuzzy
                  filename matcher produced a lot of wrong-art mismatches
                  in the field, especially for repacked / renamed sets.
                  When this is on, only ROMs whose sha1 matched a
                  libretro-database canonical entry (via Identify ROMs)
                  get their art synced. Turn it off to fall back to
                  fuzzy filename matching against ALL library entries
                  at the strict 0.95 threshold. */}
              <SettingRow
                label="Only sync identified ROMs"
                hint="Recommended"
                inherited={null}
                overridden={false}
                toggle={{
                  checked: onlySyncIdentified(),
                  onChange: (v) => void setOnlySyncIdentifiedPref(v),
                }}
                description="Skip ROMs that haven't been hash-identified via Identify ROMs. Stops the fuzzy filename matcher from producing wrong-art mismatches on repacked or renamed sets."
              />

              {/* Kinds to fetch — controls which libretro-thumbnails subdirs
                  the sync pulls per ROM. Defaults to all three; users on
                  metered connections can drop the extras to cut sync
                  bandwidth ~3×. */}
              <div class="space-y-1">
                <p class="text-xs text-(--color-oa-ink-dim)">
                  Kinds to fetch (per-ROM downloads during sync)
                </p>
                <div class="flex flex-wrap gap-2">
                  <For each={["box-front", "screenshot-gameplay", "screenshot-title"] as const}>
                    {(k) => {
                      const checked = () => media.kindsToFetch().includes(k);
                      const label =
                        k === "box-front" ? "Boxart"
                        : k === "screenshot-gameplay" ? "Snapshots"
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
                            disabled={isSystemBusy(id)}
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
                            disabled={isSystemBusy(id)}
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
                            disabled={isSystemBusy(id)}
                            onClick={(e) => {
                              e.currentTarget.blur();
                              void startClearMetadata(id);
                            }}
                            class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink) disabled:cursor-not-allowed disabled:opacity-50"
                            title="Clear metadata (genre / developer / publisher / year / players) for every game in this system. Cover art is NOT touched. Use after the 2026-05-21 metadata-routing fix to scrub stale cross-system data."
                          >
                            {metaClearing()[id] ? "Clearing…" : "Clear metadata"}
                          </button>
                          <button
                            type="button"
                            disabled={isSystemBusy(id)}
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
                            disabled={isSystemBusy(id)}
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
                      <Show when={metaClearStatus()[id]}>
                        {(s) => (
                          <p class="truncate text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                            clear metadata · {s()}
                          </p>
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
                            {(() => {
                              const v = s();
                              if (v.canonicalEntries === 0) {
                                return "identify · libretro-database has no hash DB for this system";
                              }
                              if (v.libraryTotal === 0) {
                                return `identify · no games in library for this system (${v.canonicalEntries} canonical entries in DB)`;
                              }
                              if (v.scanned === 0 && v.alreadyIdentified === v.libraryTotal) {
                                return `identify · all ${v.libraryTotal} games already identified (re-runs are no-ops)`;
                              }
                              return `identify done · ${v.matched} matched · ${v.unmatched} unknown · ${v.skippedCd} CD-skipped · ${v.errors} errors · ${v.alreadyIdentified} of ${v.libraryTotal} stamped (${v.canonicalEntries} canonical entries in DB)`;
                            })()}
                          </p>
                        )}
                      </Show>
                    </div>
                  );
                }}
              </For>

              {/* Region priority — drag-reorder + remove + add. */}
              <div class="space-y-1">
                <p class="text-xs text-(--color-oa-ink-dim)">
                  Region priority (first match wins) — drag to reorder
                </p>
                <DragDropProvider
                  onDragEnd={handleMediaRegionDragEnd}
                  collisionDetector={closestCenter}
                >
                  <DragDropSensors />
                  <SortableProvider ids={regionDraft()}>
                    <ul class="space-y-1">
                      <For each={regionDraft()}>
                        {(region, i) => (
                          <SortableMediaRegionRow
                            region={region}
                            idx={i()}
                            onRemove={removeRegionByName}
                          />
                        )}
                      </For>
                    </ul>
                  </SortableProvider>
                </DragDropProvider>
                <select
                  onChange={(e) => {
                    const v = e.currentTarget.value;
                    if (v) addRegion(v);
                    e.currentTarget.value = "";
                  }}
                  class={selectClass("oa")}
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
                <DragDropProvider
                  onDragEnd={handleFolderDragEnd}
                  collisionDetector={closestCenter}
                >
                  <DragDropSensors />
                  <SortableProvider ids={props.settings.libraryFolderRows().map((r) => r.id)}>
                    <ul class="space-y-1">
                      <For each={props.settings.libraryFolderRows()}>
                        {(row) => (
                          <SortableFolderRow
                            id={row.id}
                            folder={row.path}
                            onRemove={removeLibraryFolder}
                          />
                        )}
                      </For>
                    </ul>
                  </SortableProvider>
                </DragDropProvider>
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

                {/* Hidden containers (v2.3) — operator can right-click a
                    container in the sidebar tree to hide it; this surfaces
                    the un-hide affordance so it's not a one-way trip. Walks
                    the active view at any depth so v3+ nested-container
                    views work too. Renders only when at least one container
                    is hidden. */}
                <Show when={collectHiddenContainers(props.views.activeView()?.root ?? {
                  id: "_empty", label: "", rule: null, accent: null, art: null, hidden: false, children: []
                }).length > 0}>
                  <div class="mt-4 space-y-1">
                    <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                      Hidden containers
                    </p>
                    <ul class="space-y-1">
                      <For each={collectHiddenContainers(props.views.activeView()?.root ?? {
                        id: "_empty", label: "", rule: null, accent: null, art: null, hidden: false, children: []
                      })}>
                        {(container) => (
                          <li class="flex items-center justify-between gap-3 rounded border border-white/5 bg-white/[0.02] px-3 py-2 text-xs">
                            <span class="flex-1 truncate text-(--color-oa-ink-dim)">
                              {container.label}
                            </span>
                            <button
                              type="button"
                              onClick={(e) => {
                                e.currentTarget.blur();
                                props.views.setNodeHidden(container.id, false);
                              }}
                              class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-(--color-system-accent)/15 hover:text-(--color-oa-ink)"
                            >
                              Show
                            </button>
                          </li>
                        )}
                      </For>
                    </ul>
                  </div>
                </Show>
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
                      /// PR-γ source of truth is the active view's per-leaf
                      /// `hidden` flag; the legacy flat `layout.hiddenSystems`
                      /// set is checked as a fallback so systems not present
                      /// in the active view's tree (custom v3+ views) still
                      /// honor the operator's hide intent. Writes go to both
                      /// to keep the two representations consistent during
                      /// the soft-migration window.
                      const hidden = () => {
                        const active = props.views.activeView();
                        if (active) {
                          const node = findNode(active, platformNodeIdFor(id));
                          if (node && "kind" in node && node.kind === "platform" && node.hidden) {
                            return true;
                          }
                        }
                        return props.layout.hiddenSystems().includes(id);
                      };
                      return (
                        <li class="flex items-center gap-3 rounded border border-white/5 bg-white/[0.02] px-3 py-2 text-xs">
                          <input
                            type="checkbox"
                            checked={!hidden()}
                            onChange={(e) => {
                              const list = props.layout.hiddenSystems();
                              const show = e.currentTarget.checked;
                              if (show) {
                                props.layout.setHiddenSystems(list.filter((s) => s !== id));
                              } else if (!list.includes(id)) {
                                props.layout.setHiddenSystems([...list, id]);
                              }
                              props.views.setNodeHidden(platformNodeIdFor(id), !show);
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

              {/* --- Region & version priority --- */}
              <div class="mt-6 space-y-2">
                <div class="flex items-center justify-between">
                  <h3 class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
                    Region & version priority
                  </h3>
                  <button
                    type="button"
                    onClick={(e) => {
                      e.currentTarget.blur();
                      resetLibraryRegionPriority();
                    }}
                    class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
                  >
                    Reset
                  </button>
                </div>
                <p class="text-[0.6rem] uppercase tracking-widest text-(--color-oa-ink-dim)">
                  When a game has dumps from multiple regions / revisions, the library tile shows
                  the variant matching the earliest region in this list. Per-system overrides
                  live in each system's settings panel.
                </p>
                <DragDropProvider
                  onDragEnd={handleRegionDragEnd}
                  collisionDetector={closestCenter}
                >
                  <DragDropSensors />
                  <SortableProvider ids={libraryPrefs().regionPriority}>
                    <ul class="space-y-1">
                      <For each={libraryPrefs().regionPriority}>
                        {(region, idx) => (
                          <SortableRegionRow region={region} idx={idx()} />
                        )}
                      </For>
                    </ul>
                  </SortableProvider>
                </DragDropProvider>
                <div class="mt-3">
                  <SettingRow
                    label="Revision tiebreaker"
                    inherited={null}
                    overridden={false}
                    select={{
                      value: libraryPrefs().revisionPriority,
                      options: [
                        { value: "newest", label: "Newest revision wins" },
                        { value: "oldest", label: "Oldest revision wins" },
                      ],
                      onChange: (v) =>
                        void persistLibraryPrefs({
                          ...libraryPrefs(),
                          revisionPriority: v as RevisionPriority,
                        }),
                    }}
                  />
                </div>
              </div>

              {/* --- Library cleanup --- */}
              <div class="mt-6 space-y-2">
                <h3 class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
                  Cleanup
                </h3>
                <SettingRow
                  label="Auto-remove on delete"
                  inherited={null}
                  overridden={false}
                  toggle={{
                    checked: props.settings.autoRemoveOnDelete(),
                    onChange: (v) => props.settings.setAutoRemoveOnDelete(v),
                  }}
                  description="Off (default) keeps library entries when files vanish — useful for moves / renames. On removes the matching row when the watcher reports the file gone."
                />

                <div class="mt-3 flex flex-wrap items-center gap-2">
                  <span class="text-xs text-(--color-oa-ink-dim)">Clear games for:</span>
                  <select
                    class={selectClass("oa")}
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
      <ImportArtPackDialog
        open={importArtPackOpen()}
        onClose={() => setImportArtPackOpen(false)}
        onImported={() => {
          // Re-hydrate the MediaIndex so the just-imported variants
          // show on the grid + Game Info modals without a restart.
          void media.refreshAll();
        }}
      />
    </>
  );
};

export default LibraryManagerPage;
