// Filter / sort / group pipeline applied to the library's RomEntry list.
//
// Pure functions: caller wires them into a `createMemo` over the live
// signals so the virtualized grid receives an already-derived list. None of
// these functions allocate when there's nothing to do — empty queries,
// "none" group, and identity sorts short-circuit.

import type { GroupBy, SortKey } from "../layout/state";
import type { SidebarView } from "../layout/LeftSidebar";
import type { GameMetadata } from "./media";
import type { RomEntry } from "./types";

/** First-letter-of-title bucket label. Numbers + non-letters fall into "#". */
function letterBucket(title: string): string {
  // Strip leading articles + punctuation the same way LaunchBox does.
  const trimmed = title.replace(/^(the|a|an)\s+/i, "").trim();
  const first = trimmed[0];
  if (!first) return "#";
  const upper = first.toUpperCase();
  return /^[A-Z]$/.test(upper) ? upper : "#";
}

/** Apply view + search-query filters. Pure. */
export function filterEntries(
  entries: RomEntry[],
  view: SidebarView,
  query: string,
): RomEntry[] {
  const q = query.trim().toLowerCase();
  let result = entries;

  // View-driven slicing first — narrows the candidate list before search.
  if (view.kind === "system") {
    result = result.filter((e) => e.systemId === view.id);
  }
  // home / all / favorites / recent / continue currently all show every
  // non-seed entry. Real Favorites/Recent/Continue logic ships when
  // play_history + favorites columns get a UI in Phase 4.

  if (q.length > 0) {
    result = result.filter((e) => e.title.toLowerCase().includes(q));
  }

  return result;
}

/** Apply the selected sort. Returns a NEW array (caller may pass a live store
 *  proxy; mutation would break Solid reactivity). */
export function sortEntries(
  entries: RomEntry[],
  key: SortKey,
  getYear: (romId: string) => number | undefined,
): RomEntry[] {
  if (entries.length <= 1) return entries.slice();
  const sorted = entries.slice();
  switch (key) {
    case "title":
      sorted.sort((a, b) =>
        a.title.localeCompare(b.title, undefined, { sensitivity: "base", numeric: true }),
      );
      break;
    case "addedAt":
      // Most-recent first. Seed entries (addedAt = 0) sink to the bottom.
      sorted.sort((a, b) => b.addedAt - a.addedAt);
      break;
    case "year":
      // Ascending year. Missing year sinks to the bottom; ties break by title.
      sorted.sort((a, b) => {
        const ya = getYear(a.id);
        const yb = getYear(b.id);
        if (ya === undefined && yb === undefined) return a.title.localeCompare(b.title);
        if (ya === undefined) return 1;
        if (yb === undefined) return -1;
        if (ya !== yb) return ya - yb;
        return a.title.localeCompare(b.title);
      });
      break;
  }
  return sorted;
}

export type EntryGroup = {
  /** Stable identifier used as group key (e.g. "tg16", "A", "1990"). */
  id: string;
  /** Human-readable label rendered as section header. */
  label: string;
  entries: RomEntry[];
};

/** Bucket an already-sorted list into named groups. group="none" returns a
 *  single virtual group. Caller decides whether to render headers. */
export function groupEntries(
  entries: RomEntry[],
  group: GroupBy,
  systemDisplayName: (id: string) => string,
): EntryGroup[] {
  if (group === "none" || entries.length === 0) {
    return [{ id: "all", label: "", entries }];
  }

  const buckets = new Map<string, EntryGroup>();
  const order: string[] = [];

  for (const entry of entries) {
    let id: string;
    let label: string;
    switch (group) {
      case "letter": {
        id = letterBucket(entry.title);
        label = id;
        break;
      }
      case "system": {
        id = entry.systemId;
        label = systemDisplayName(entry.systemId);
        break;
      }
    }
    let bucket = buckets.get(id);
    if (!bucket) {
      bucket = { id, label, entries: [] };
      buckets.set(id, bucket);
      order.push(id);
    }
    bucket.entries.push(entry);
  }

  // For letter grouping, force alphabetical order regardless of input order
  // (numbers/punctuation '#' first, then A-Z). For system grouping, preserve
  // first-seen order (matches user's sidebar ordering by extension).
  if (group === "letter") {
    order.sort((a, b) => {
      if (a === "#" && b !== "#") return -1;
      if (b === "#" && a !== "#") return 1;
      return a.localeCompare(b);
    });
  }

  return order.map((id) => buckets.get(id)!);
}

/** Helper for sortEntries' getYear closure when metadata is available. */
export function metadataYear(meta: GameMetadata | undefined): number | undefined {
  return meta?.year;
}
