// Unified Navigation Tree Slice 2 — shared smart-list predicates.

import { describe, expect, it } from "vitest";
import {
  asSmartListKind,
  evaluateSmartList,
  SMART_LISTS,
  SMART_LISTS_BY_KIND,
} from "./smartLists";
import type { RomEntry } from "./types";
import type { SystemId } from "@oa/platform/themes/registry";

function entry(id: string, over: Partial<RomEntry> = {}): RomEntry {
  return {
    id,
    title: id,
    systemId: "nes" as SystemId,
    filePath: `/lib/${id}`,
    addedAt: 0,
    ...over,
  };
}

const NOW = 1_000_000;
const ctx = { nowSecs: NOW };

describe("evaluateSmartList", () => {
  const entries: RomEntry[] = [
    entry("fav", { favorite: true }),
    entry("recent", { lastPlayedAt: NOW - 60 }),
    entry("stale", { lastPlayedAt: NOW - 40 * 24 * 60 * 60 }),
    entry("done", { completed: true }),
    entry("twoP", { players: 2 }),
    entry("plain"),
  ];

  it("favorites matches favorited entries", () => {
    expect([...evaluateSmartList("favorites", entries, ctx)]).toEqual(["fav"]);
  });

  it("recentlyPlayed honors the 30-day window via ctx.nowSecs", () => {
    // `recent` is inside the window; `stale` (40 days) is out.
    expect([...evaluateSmartList("recentlyPlayed", entries, ctx)]).toEqual(["recent"]);
  });

  it("completed matches completed entries", () => {
    expect([...evaluateSmartList("completed", entries, ctx)]).toEqual(["done"]);
  });

  it("multiPlayer matches players >= 2", () => {
    expect([...evaluateSmartList("multiPlayer", entries, ctx)]).toEqual(["twoP"]);
  });

  it("lastPlayed matches any entry with a lastPlayedAt", () => {
    expect([...evaluateSmartList("lastPlayed", entries, ctx)]).toEqual(["recent", "stale"]);
  });
});

describe("asSmartListKind", () => {
  it("narrows a known kind", () => {
    expect(asSmartListKind({ kind: "favorites" })).toBe("favorites");
    expect(asSmartListKind({ kind: "recentlyPlayed" })).toBe("recentlyPlayed");
  });

  it("rejects unknown / malformed specs", () => {
    expect(asSmartListKind({ kind: "dumpQuality" })).toBeNull();
    expect(asSmartListKind({ kind: 7 })).toBeNull();
    expect(asSmartListKind({})).toBeNull();
    expect(asSmartListKind(null)).toBeNull();
    expect(asSmartListKind(undefined)).toBeNull();
  });
});

describe("registry shape", () => {
  it("every list is reachable by kind", () => {
    for (const l of SMART_LISTS) {
      expect(SMART_LISTS_BY_KIND[l.kind]).toBe(l);
    }
  });
});
