// Per-system status stats for the Systems hub. Self-contained hook reading the
// platform library store + MediaDb (the same singletons LibraryManagerPage uses)
// so the hub needs no props threaded through SettingsPanel. The Game-media grid
// in LibraryManagerPage keeps its own local copy until that tab is removed in S5
// — this is the shared home going forward (Per-System Settings Hub arc, S1).

import { createMemo, type Accessor } from "solid-js";
import { useMedia } from "@oa/platform/library/media";
import { usePlatform } from "@oa/platform/platformContext";
import { systemThemes, type SystemId } from "@oa/platform/themes/registry";

export type MediaCardStats = {
  total: number;
  identified: number;
  covered: number;
  metadataed: number;
};

export type RowState = "ok" | "partial" | "none";

export const STATE_GLYPH: Record<RowState, string> = { ok: "✓", partial: "⚠", none: "✗" };
export const STATE_CLASS: Record<RowState, string> = {
  ok: "text-emerald-300",
  partial: "text-amber-300",
  none: "text-rose-300",
};

/// Status of one count against its total. 0/0 (empty system) reads "none" so
/// pre-import systems render sensibly under Show-all.
export function rowState(n: number, total: number): RowState {
  if (total === 0 || n === 0) return n === 0 ? "none" : "partial";
  if (n >= total) return "ok";
  return "partial";
}

const EMPTY: MediaCardStats = { total: 0, identified: 0, covered: 0, metadataed: 0 };

export type SystemsStats = {
  /// Stats for one system (empty zeroes for systems with no games).
  statsFor: (id: SystemId) => MediaCardStats;
  /// Systems with ≥1 non-seed entry, alphabetical by displayName.
  librarySystems: Accessor<SystemId[]>;
  /// All known systems, alphabetical by displayName (for the Show-all toggle).
  allSystems: Accessor<SystemId[]>;
  /// How many library systems are behind on identified / covers / metadata.
  incompleteCount: Accessor<number>;
};

export function useSystemsStats(): SystemsStats {
  const platform = usePlatform();
  const media = useMedia();

  const stats = createMemo<Map<SystemId, MediaCardStats>>(() => {
    const out = new Map<SystemId, MediaCardStats>();
    for (const e of platform.library.state.entries) {
      if (e.seed) continue;
      const id = e.systemId as SystemId;
      let s = out.get(id);
      if (!s) {
        s = { total: 0, identified: 0, covered: 0, metadataed: 0 };
        out.set(id, s);
      }
      s.total += 1;
      if (e.sha1) s.identified += 1;
      const m = media.media(e.id);
      if ((m?.boxFront && m.boxFront.length > 0) || (m?.boxart && m.boxart.length > 0)) {
        s.covered += 1;
      }
      const md = m?.metadata;
      if (md && (md.year || md.genre || md.developer || md.publisher)) {
        s.metadataed += 1;
      }
    }
    return out;
  });

  const byName = (a: SystemId, b: SystemId): number =>
    (systemThemes[a]?.displayName ?? a).localeCompare(systemThemes[b]?.displayName ?? b);

  const librarySystems = createMemo<SystemId[]>(() => Array.from(stats().keys()).sort(byName));
  const allSystems = createMemo<SystemId[]>(() => (Object.keys(systemThemes) as SystemId[]).sort(byName));
  const incompleteCount = createMemo<number>(() => {
    let k = 0;
    for (const s of stats().values()) {
      if (s.total === 0) continue;
      if (s.identified < s.total || s.covered < s.total || s.metadataed < s.total) k += 1;
    }
    return k;
  });

  return {
    statsFor: (id) => stats().get(id) ?? EMPTY,
    librarySystems,
    allSystems,
    incompleteCount,
  };
}
