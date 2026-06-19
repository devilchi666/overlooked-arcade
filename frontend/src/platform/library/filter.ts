// Filter / sort / group pipeline applied to the library's RomEntry list.
//
// Pure functions: caller wires them into a `createMemo` over the live
// signals so the virtualized grid receives an already-derived list. None of
// these functions allocate when there's nothing to do — empty queries,
// "none" group, and identity sorts short-circuit.

import type { GroupBy, SortKey } from "@oa/platform/layout/state";
import type { SidebarView } from "../layout/types";
import type { SystemId } from "@oa/platform/themes/registry";
import type { GameMetadata } from "./media";
import type { GameGroupInfo, RomEntry } from "./types";

/** First-letter-of-title bucket label. Numbers + non-letters fall into "#". */
function letterBucket(title: string): string {
  // Strip leading articles + punctuation the same way LaunchBox does.
  const trimmed = title.replace(/^(the|a|an)\s+/i, "").trim();
  const first = trimmed[0];
  if (!first) return "#";
  const upper = first.toUpperCase();
  return /^[A-Z]$/.test(upper) ? upper : "#";
}

/** Apply view + search-query filters. Pure.
 *
 *  `viewSystemIds` is the caller-resolved set of system ids the active
 *  view-node restricts to (null for `all` — meaning "no system-level
 *  filter"). The resolution lives in the caller (LibraryView) because
 *  the views model needs ViewsStore + resolver, which we keep out of
 *  this pure module. The `view` parameter is retained for future
 *  per-discriminant logic (e.g. date-range filters from a smart view).
 *
 *  `viewRomIds` (Unified Navigation Tree Slice 1) is the caller-resolved
 *  set of rom ids when the active node is a game-set node (a collection,
 *  and from Slice 2 a smart-list filter) rather than a system node. When
 *  non-null it takes precedence and slices by rom id; `viewSystemIds`
 *  should be null in that case. This is the seam this doc comment
 *  originally reserved — non-system membership now flows through here.
 */
export function filterEntries(
  entries: RomEntry[],
  view: SidebarView,
  query: string,
  viewSystemIds: ReadonlyArray<SystemId> | null,
  viewRomIds: ReadonlySet<string> | null,
  groupsByVariantId?: Map<string, GameGroupInfo>,
): RomEntry[] {
  // `view` is referenced via the parameter type so the discriminant
  // stays exhaustive in TypeScript — body-level branching on `view.kind`
  // lands when a smart-view rule needs more than the resolved ids.
  void view;
  const q = query.trim().toLowerCase();
  let result = entries;

  // View-driven slicing first — narrows the candidate list before search.
  // A game-set node (collection / filter) resolves to an explicit rom-id
  // set and takes precedence; otherwise fall back to the system filter.
  if (viewRomIds !== null) {
    result = result.filter((e) => viewRomIds.has(e.id));
  } else if (viewSystemIds !== null) {
    const allow = new Set<string>(viewSystemIds);
    result = result.filter((e) => allow.has(e.systemId));
  }

  if (q.length > 0) {
    // VL Phase E Sub-phase 3 — match the identity's canonical title in
    // addition to the per-file title, so searching "castlevania" hits a
    // group whose only dump is named "Akumajou Dracula (Japan)" once
    // the identity carries the canonical name. Per-file titles keep
    // matching too (searching a region tag like "(japan)" still works).
    result = result.filter((e) => {
      if (e.title.toLowerCase().includes(q)) return true;
      const canonical = groupsByVariantId?.get(e.id)?.displayBaseTitle;
      return canonical !== undefined && canonical.toLowerCase().includes(q);
    });
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

/**
 * Collapse same-variant-group entries down to their default variant.
 * Given a flat (already filtered + sorted) list and a Map keyed by
 * EVERY variant's id pointing at its group, replaces non-default
 * variants with their group's default and dedupes.
 *
 * Behaviour: if a search query matches "Castlevania (Japan)" but USA is
 * the default, the USA tile still surfaces — the matching variant's
 * group default replaces it. Search-by-region-name therefore "promotes"
 * the right group's tile rather than disappearing entirely.
 *
 * The Map only contains entries for groups with `variants.length > 1`;
 * single-file games are absent and pass through unchanged.
 */
/**
 * Phase A1 Sub-phase 4 — collapse multi-disc set members down to one
 * representative tile per set. The representative is the lowest-
 * `discNumber` entry; its title is stripped of the "(Disc N)" suffix
 * at render time. Standalone games (undefined `discSetId`) pass
 * through unchanged.
 *
 * The LibraryTile checks `discSetId` to decide whether to open the
 * disc-picker overlay on click instead of launching directly. This
 * collapse intentionally KEEPS the representative's other fields
 * (id, filePath, etc.) so existing context-menu actions (Show saves,
 * Show info, etc.) still target a real game.
 */
export function collapseDiscSets(entries: RomEntry[]): RomEntry[] {
  // First pass: bucket every disc-set member, ordered by discNumber.
  // Builds both the representative (lowest discNumber) AND a fallback
  // cover (first member with a non-empty coverPath in disc order) +
  // the member count for the LibraryTile badge.
  type SetState = {
    members: RomEntry[]; // sorted by discNumber ASC
  };
  const byDiscSet = new Map<number, SetState>();
  for (const entry of entries) {
    if (entry.discSetId === undefined) continue;
    let state = byDiscSet.get(entry.discSetId);
    if (!state) {
      state = { members: [] };
      byDiscSet.set(entry.discSetId, state);
    }
    state.members.push(entry);
  }
  for (const state of byDiscSet.values()) {
    state.members.sort(
      (a, b) =>
        (a.discNumber ?? Number.POSITIVE_INFINITY) -
        (b.discNumber ?? Number.POSITIVE_INFINITY),
    );
  }
  // Second pass: emit representatives once per set, standalone games
  // pass through in input order.
  const seen = new Set<number>();
  const out: RomEntry[] = [];
  for (const entry of entries) {
    if (entry.discSetId !== undefined) {
      if (seen.has(entry.discSetId)) continue;
      seen.add(entry.discSetId);
      const state = byDiscSet.get(entry.discSetId)!;
      const representative = state.members[0];
      // Cover fallback — if the lowest-disc entry has no coverPath,
      // scan subsequent discs in order and use the first non-empty
      // one. Saves the operator from "Disc 1 has no boxart so the
      // set tile is blank" when their Disc 2 dump happens to have
      // artwork attached.
      const coverProvider = representative.coverPath
        ? representative
        : state.members.find((m) => m.coverPath);
      out.push({
        ...representative,
        title: stripDiscSuffix(representative.title),
        coverPath: coverProvider?.coverPath ?? representative.coverPath,
        discSetMemberCount: state.members.length,
      });
    } else {
      out.push(entry);
    }
  }
  return out;
}

/// Strip the redump "(Disc N)" suffix from a multi-disc game title so
/// the collapsed set tile reads "Final Fantasy IX (USA)" rather than
/// "Final Fantasy IX (USA) (Disc 1)". Case-insensitive, tolerates one
/// or more spaces between "Disc" and the number.
function stripDiscSuffix(title: string): string {
  return title.replace(/\s*\(\s*Disc\s+\d+\s*\)/gi, "").trim();
}

export function collapseVariantGroups(
  entries: RomEntry[],
  groupsByVariantId: Map<string, GameGroupInfo>,
  entryById: Map<string, RomEntry>,
): RomEntry[] {
  const seen = new Set<string>();
  const out: RomEntry[] = [];
  for (const entry of entries) {
    const group = groupsByVariantId.get(entry.id);
    if (group) {
      if (seen.has(group.defaultVariantId)) continue;
      seen.add(group.defaultVariantId);
      const defaultEntry = entryById.get(group.defaultVariantId);
      // VL Phase E Sub-phase 3 — the tile shows the identity's
      // canonical title ("Castlevania"), not the default variant's
      // file title ("Castlevania (USA)"). The full per-file title
      // stays visible in the Run-version submenu, the disc picker,
      // and (come Phase B) the Variants tab. Multi-disc titles keep
      // their disc suffix here so collapseDiscSets can strip it after
      // picking the representative disc — for those, the canonical
      // title already has no "(Disc N)" so the rewrite is a no-op
      // there too.
      out.push({
        ...(defaultEntry ?? entry),
        title: group.displayBaseTitle,
      });
    } else {
      if (seen.has(entry.id)) continue;
      seen.add(entry.id);
      out.push(entry);
    }
  }
  return out;
}
