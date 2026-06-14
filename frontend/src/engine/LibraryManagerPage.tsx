import {
  createEffect,
  createResource,
  createSignal,
  For,
  onMount,
  Show,
  type Component,
} from "solid-js";
import * as libraryApi from "@oa/platform/api/libraryApi";
import { confirm } from "@oa/platform/lib/confirm";
import { pushToast } from "@oa/platform/lib/toast";
import { open as pickDirectory } from "@tauri-apps/plugin-dialog";
import {
  getOnlySyncIdentified,
  setOnlySyncIdentified,
  mediaStorageStats,
  openMediaFolder,
} from "@oa/platform/api/mediaApi";
import {
  closestCenter,
  createSortable,
  DragDropProvider,
  DragDropSensors,
  SortableProvider,
  transformStyle,
  type DragEventHandler,
} from "@thisbeyond/solid-dnd";
import type { LibraryStore } from "@oa/platform/library/store";
import { useMedia } from "@oa/platform/library/media";
import type { SettingsStore } from "@oa/platform/settings/store";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import SettingRow, { selectClass } from "@oa/platform/components/SettingRow";
import { ImportArtPackDialog } from "./ImportArtPackDialog";
import { PlatformMediaDialog } from "./PlatformMediaDialog";

type Props = {
  settings: SettingsStore;
  library: LibraryStore;
  onAddLibraryFolder: () => void;
  onRescanLibraryFolders: () => void;
  /// Optional deep-link target — chooses which tab the page lands on
  /// when first mounted. Falls back to the persisted last-visited tab.
  initialTab?: "library" | "media";
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
  onRelink: (folderId: string) => void;
}> = (props) => {
  const sortable = createSortable(props.id);
  return (
    <li
      ref={sortable.ref}
      style={transformStyle(sortable.transform)}
      class="flex items-center justify-between gap-3 rounded border border-white/5 bg-white/[0.02] px-3 py-1.5 text-xs transition"
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
          props.onRelink(props.id);
        }}
        title="Moved these ROMs? Point OA at the new folder — keeps all covers, metadata, favorites and play-time."
        class="rounded border border-white/10 bg-white/[0.04] px-2 py-0.5 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
      >
        Relink…
      </button>
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
const TABS = ["library", "media"] as const;
type TabId = typeof TABS[number];
const TAB_LABELS: Record<TabId, string> = {
  library:      "Library",
  media:        "Game media",
};
// TAB_HINTS dropped 2026-05-31 alongside the page-mode header that
// rendered them (subtitle next to the Library Manager title).
// Re-add if a future surface needs short per-tab hint strings.

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

  // Local Esc handler dropped 2026-05-31 with the variant="page" mode.
  // Retroverse SETTINGS owns Esc semantics for the embedding category
  // pane; the legacy page-mode equivalent (which set
  // `setCurrentView({ kind: "all" })`) is no longer reachable.

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
    libraryApi.getLibraryPrefs<LibraryPrefs>()
      .then((p) => setLibraryPrefs(p))
      .catch((e) => console.warn("get_library_prefs failed:", e));
  });
  async function persistLibraryPrefs(next: LibraryPrefs) {
    setLibraryPrefs(next);
    try {
      await libraryApi.setLibraryPrefs(next);
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
  /// Relink a moved folder: the user picks the new location, OA previews how
  /// many tracked ROMs are present there, then rebases the folder + its games'
  /// paths IN PLACE on confirm — covers / metadata / favorites / play-time are
  /// keyed by game id, not path, so they all survive. The App.tsx watcher
  /// effect (keyed on settings.libraryFolders()) re-registers automatically
  /// after refreshLibraryFolders().
  async function relinkFolder(folderId: string) {
    const picked = await pickDirectory({ directory: true, multiple: false });
    if (!picked || typeof picked !== "string") return;
    let preview: libraryApi.RepointPreview;
    try {
      preview = await libraryApi.previewRepointFolder(folderId, picked);
    } catch (e) {
      pushToast("error", `Couldn't read that folder: ${e}`);
      return;
    }
    const allFound = preview.matched === preview.total;
    const sample = preview.sampleMissing.slice(0, 3).join(", ");
    const message = allFound
      ? `All ${preview.total} ROMs were found at the new location. Relink this folder? Covers, metadata, favorites and play-time are all kept.`
      : `${preview.matched} of ${preview.total} ROMs found at the new location`
        + (preview.missing > 0 ? ` (${preview.missing} missing${sample ? `, e.g. ${sample}` : ""})` : "")
        + `. Relink anyway? Missing ROMs will point at the new folder until you re-scan.`;
    if (!(await confirm(message, {
      title: "Relink folder",
      confirmLabel: "Relink",
      danger: preview.matched === 0,
    }))) return;
    try {
      const res = await libraryApi.repointFolder(folderId, picked);
      await props.settings.refreshLibraryFolders();
      await props.library.refreshGroups();
      pushToast(
        "success",
        `Relinked — ${res.gamesUpdated} game${res.gamesUpdated === 1 ? "" : "s"} now point at the new folder.`,
      );
    } catch (e) {
      pushToast("error", `Relink failed: ${e}`);
    }
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
  // Phase 6 (2026-05-23): platform-media dialog open-state — same
  // top-of-component pattern so a button in the media tab can open it.
  const [platformMediaOpen, setPlatformMediaOpen] = createSignal(false);

  // "Only sync identified ROMs" pref — file-backed on the Rust side
  // alongside the other media prefs. Hydrated once on mount; setter
  // writes through.
  const [onlySyncIdentified, setOnlySyncIdentifiedLocal] = createSignal<boolean>(true);
  onMount(() => {
    void getOnlySyncIdentified()
      .then((v) => setOnlySyncIdentifiedLocal(v))
      .catch((e) => console.warn("get_only_sync_identified failed:", e));
  });
  async function setOnlySyncIdentifiedPref(v: boolean) {
    setOnlySyncIdentifiedLocal(v);
    try {
      await setOnlySyncIdentified(v);
    } catch (e) {
      console.warn("set_only_sync_identified failed:", e);
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

  // Disk usage — fetched on page mount.
  const [storageStats] = createResource(
    async (): Promise<MediaStorageStats | null> => {
      try {
        return await mediaStorageStats();
      } catch (e) {
        console.warn("media_storage_stats failed:", e);
        return null;
      }
    },
  );

  return (
    <>
    <div
      class="flex w-full flex-col"
      role="region"
    >
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
            <Show when={activeTab() === "media"}>
            <div class="space-y-5">
              {/* Header — page title + top-level entry-point buttons. */}
              <div class="flex items-center justify-between gap-3">
                <h3 class="text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
                  Game media
                </h3>
                <div class="flex items-center gap-2">
                  <button
                    type="button"
                    onClick={(e) => {
                      e.currentTarget.blur();
                      setPlatformMediaOpen(true);
                    }}
                    class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
                  >
                    Platform media…
                  </button>
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
                      void openMediaFolder();
                    }}
                    class="rounded-md border border-white/10 bg-white/[0.04] px-2 py-1 text-[0.6rem] uppercase tracking-wider text-(--color-oa-ink-dim) transition hover:bg-white/[0.08] hover:text-(--color-oa-ink)"
                  >
                    Open folder
                  </button>
                </div>
              </div>

              {/* Preferences card — the things that are settings, not
                  actions. Hoisted here on 2026-06-03 (declutter arc) so
                  the per-system action area below stays a clean grid. */}
              <div class="space-y-3 rounded-lg border border-white/10 bg-white/[0.02] px-4 py-3">
                <h4 class="text-[0.6rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
                  Preferences
                </h4>

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

                <div class="space-y-1">
                  <p class="text-[0.7rem] text-(--color-oa-ink-dim)">
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

                {/* Region priority — collapsed by default so the
                    preferences card stays compact. Operator expands when
                    they need to reorder. */}
                <details class="group">
                  <summary class="cursor-pointer select-none list-none text-[0.7rem] text-(--color-oa-ink-dim) transition hover:text-(--color-oa-ink) [&::-webkit-details-marker]:hidden">
                    <span class="mr-1 inline-block transition group-open:rotate-90" aria-hidden="true">▸</span>
                    Region priority (first match wins) — drag to reorder
                  </summary>
                  <div class="mt-2 space-y-1">
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
                </details>
              </div>

              {/* Disk usage. */}
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
                <h3 class="flex items-center gap-2 text-[0.65rem] uppercase tracking-[0.4em] text-(--color-oa-ink-dim)">
                  Library folders
                  <span class="rounded bg-white/[0.06] px-1.5 py-0.5 text-[0.6rem] tabular-nums">
                    {props.settings.libraryFolders().length}
                  </span>
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
                    <ul class="max-h-56 space-y-1 overflow-y-auto pr-1">
                      <For each={props.settings.libraryFolderRows()}>
                        {(row) => (
                          <SortableFolderRow
                            id={row.id}
                            folder={row.path}
                            onRemove={removeLibraryFolder}
                            onRelink={relinkFolder}
                          />
                        )}
                      </For>
                    </ul>
                  </SortableProvider>
                </DragDropProvider>
              </Show>

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
                      if (!(await confirm(
                        `Remove all ${theme.displayName} games from the library? Files on disk are NOT touched.`,
                        { title: "Remove system games", confirmLabel: "Remove", danger: true },
                      ))) return;
                      const n = await props.library.clearForSystem(id);
                      pushToast("success", `Removed ${n} game${n === 1 ? "" : "s"} from ${theme.displayName}.`);
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
                      if (!(await confirm(
                        "Reset the entire library? Every game row will be removed from the database. Files on disk are NOT deleted.",
                        { title: "Reset library", confirmLabel: "Reset library", danger: true },
                      ))) return;
                      await props.library.clear();
                      pushToast("success", "Library reset. Re-scan a folder to rebuild.");
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
      <PlatformMediaDialog
        open={platformMediaOpen()}
        onClose={() => setPlatformMediaOpen(false)}
      />
    </>
  );
};

export default LibraryManagerPage;
