// Shared per-system game-media operations (Identify / Sync covers / Sync
// metadata / Clear metadata / Refresh hash DB / Freshen) + busy state + the
// background-event listeners that clear it. Lifted out of LibraryManagerPage so
// the Systems hub's Media domain can run the same ops. Self-contained: reads the
// platform library store + MediaDb + per-system stats internally. Persistence
// and the underlying Tauri commands are unchanged.
//
// (Lives under engine/ — it depends on useSystemsStats, which is engine; the
// platform↛engine lint boundary forbids putting it in platform/.)

import { createSignal, onMount, type Accessor } from "solid-js";
import { confirm } from "@oa/platform/lib/confirm";
import {
  clearMetadataForSystem,
  getOnlySyncIdentified,
  resolveRomHashesForSystem,
  setOnlySyncIdentified,
  syncMediaForSystem,
  syncMetadataForSystem,
  syncRomHashesForSystem,
} from "@oa/platform/api/mediaApi";
import { listenScoped, OA_EVENTS } from "@oa/platform/api/eventsApi";
import { useMedia } from "@oa/platform/library/media";
import { usePlatform } from "@oa/platform/platformContext";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";
import { useSystemsStats } from "./systemsHubStats";

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
type HashSyncSummaryPayload = {
  systemId: string;
  upstreamEntries: number;
  written: number;
  fromCache: boolean;
};
type HashResolveSummaryPayload = { systemId: string; scanned: number; matched: number };

type RomLite = {
  id: string;
  title: string;
  filePath: string;
  systemId: string;
  sha1?: string;
};

export type GameMediaOps = {
  startSync: (systemId: SystemId) => Promise<void>;
  startMetadataSync: (systemId: SystemId) => Promise<void>;
  startClearMetadata: (systemId: SystemId) => Promise<void>;
  startHashSync: (systemId: SystemId) => Promise<void>;
  startHashResolve: (systemId: SystemId) => Promise<void>;
  /// Smart freshen — runs Identify / Sync covers / Sync metadata in sequence,
  /// skipping any already 100% complete for this system.
  startFreshen: (systemId: SystemId) => Promise<void>;
  /// True while any op for this system is in flight (cross-button gating).
  isSystemBusy: (id: SystemId) => boolean;
  /// "Only sync identified ROMs" pref (file-backed).
  onlySyncIdentified: Accessor<boolean>;
  setOnlySyncIdentifiedPref: (v: boolean) => Promise<void>;
};

export function useGameMediaOps(): GameMediaOps {
  const platform = usePlatform();
  const media = useMedia();
  const stats = useSystemsStats();

  const entriesFor = (systemId: SystemId): RomLite[] =>
    platform.library.state.entries
      .filter((e) => e.systemId === systemId && !e.seed)
      .map((e) => ({
        id: e.id,
        title: e.title,
        filePath: e.filePath,
        systemId: e.systemId,
        sha1: e.sha1,
      }));

  const [syncing, setSyncing] = createSignal<Record<string, boolean>>({});
  const [metaSyncing, setMetaSyncing] = createSignal<Record<string, boolean>>({});
  const [metaClearing, setMetaClearing] = createSignal<Record<string, boolean>>({});
  const [hashSyncing, setHashSyncing] = createSignal<Record<string, boolean>>({});
  const [hashResolving, setHashResolving] = createSignal<Record<string, boolean>>({});

  // Background-job completion events clear the per-system busy flags (the
  // happy-path counterpart to the finally-blocks below, for ops that complete
  // out-of-band).
  listenScoped<SyncSummaryPayload>(OA_EVENTS.librarySyncComplete, (ev) => {
    setSyncing((p) => ({ ...p, [ev.payload.systemId]: false }));
  });
  listenScoped<MetadataSyncSummaryPayload>(OA_EVENTS.libraryMetadataSyncComplete, (ev) => {
    setMetaSyncing((p) => ({ ...p, [ev.payload.systemId]: false }));
  });
  listenScoped<HashSyncSummaryPayload>(OA_EVENTS.romHashesSynced, (ev) => {
    setHashSyncing((p) => ({ ...p, [ev.payload.systemId]: false }));
  });
  listenScoped<HashResolveSummaryPayload>(OA_EVENTS.romHashResolveComplete, (ev) => {
    setHashResolving((p) => ({ ...p, [ev.payload.systemId]: false }));
  });
  // Progress events are surfaced by the global BackgroundJobsBar; we don't
  // re-render per-row progress here, so they're intentionally not subscribed.

  function isSystemBusy(id: SystemId): boolean {
    return (
      syncing()[id] === true ||
      metaSyncing()[id] === true ||
      metaClearing()[id] === true ||
      hashSyncing()[id] === true ||
      hashResolving()[id] === true
    );
  }

  async function startSync(systemId: SystemId): Promise<void> {
    const entries = entriesFor(systemId);
    if (entries.length === 0) return;
    setSyncing((p) => ({ ...p, [systemId]: true }));
    try {
      await syncMediaForSystem(systemId, entries);
      await media.refreshAll();
    } catch (e) {
      console.warn("[oa-media] sync_media_for_system failed:", e);
    } finally {
      setSyncing((p) => ({ ...p, [systemId]: false }));
    }
  }

  async function startMetadataSync(systemId: SystemId): Promise<void> {
    const entries = entriesFor(systemId);
    if (entries.length === 0) return;
    setMetaSyncing((p) => ({ ...p, [systemId]: true }));
    try {
      await syncMetadataForSystem(systemId, entries);
      await media.refreshAll();
    } catch (e) {
      console.warn("[oa-media] sync_metadata_for_system failed:", e);
    } finally {
      setMetaSyncing((p) => ({ ...p, [systemId]: false }));
    }
  }

  async function startClearMetadata(systemId: SystemId): Promise<void> {
    const name = systemThemes[systemId]?.displayName ?? systemId;
    const count = entriesFor(systemId).length;
    if (count === 0) return;
    if (
      !(await confirm(
        `Clear metadata (genre / developer / publisher / year / players) for ${count} ${name} game(s)?\n\n` +
          `Cover art, snapshots, and title screens will NOT be touched. ` +
          `Re-run "Sync metadata" afterwards to repopulate against the correct upstream catalog.`,
        { title: "Clear metadata", confirmLabel: "Clear metadata", danger: true },
      ))
    ) {
      return;
    }
    setMetaClearing((p) => ({ ...p, [systemId]: true }));
    try {
      await clearMetadataForSystem(systemId);
      await media.refreshAll();
    } catch (e) {
      console.warn("[oa-media] clear_metadata_for_system failed:", e);
    } finally {
      setMetaClearing((p) => ({ ...p, [systemId]: false }));
    }
  }

  async function startHashSync(systemId: SystemId): Promise<void> {
    setHashSyncing((p) => ({ ...p, [systemId]: true }));
    try {
      await syncRomHashesForSystem(systemId);
    } catch (e) {
      console.warn("[oa-media] sync_rom_hashes_for_system failed:", e);
    } finally {
      setHashSyncing((p) => ({ ...p, [systemId]: false }));
    }
  }

  async function startHashResolve(systemId: SystemId): Promise<void> {
    setHashResolving((p) => ({ ...p, [systemId]: true }));
    try {
      await resolveRomHashesForSystem(systemId);
      await platform.library.refresh();
    } catch (e) {
      console.warn("[oa-media] resolve_rom_hashes_for_system failed:", e);
    } finally {
      setHashResolving((p) => ({ ...p, [systemId]: false }));
    }
  }

  async function startFreshen(systemId: SystemId): Promise<void> {
    const s = stats.statsFor(systemId);
    if (s.total === 0) return;
    if (s.identified < s.total) await startHashResolve(systemId);
    if (s.covered < s.total) await startSync(systemId);
    if (s.metadataed < s.total) await startMetadataSync(systemId);
  }

  const [onlySyncIdentified, setOnlySyncIdentifiedLocal] = createSignal<boolean>(true);
  onMount(() => {
    void getOnlySyncIdentified()
      .then((v) => setOnlySyncIdentifiedLocal(v))
      .catch((e) => console.warn("[oa-media] get_only_sync_identified failed:", e));
  });
  async function setOnlySyncIdentifiedPref(v: boolean): Promise<void> {
    setOnlySyncIdentifiedLocal(v);
    try {
      await setOnlySyncIdentified(v);
    } catch (e) {
      console.warn("[oa-media] set_only_sync_identified failed:", e);
    }
  }

  return {
    startSync,
    startMetadataSync,
    startClearMetadata,
    startHashSync,
    startHashResolve,
    startFreshen,
    isSystemBusy,
    onlySyncIdentified,
    setOnlySyncIdentifiedPref,
  };
}
