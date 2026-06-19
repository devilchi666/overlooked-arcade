// Unified Navigation Tree Slice 1 — filterEntries view-slicing seam.
//
// Verifies the new `viewRomIds` parameter: a game-set node (collection /
// filter) slices the library by an explicit rom-id set and takes precedence
// over the system filter; the pre-existing `viewSystemIds` path is unchanged
// when `viewRomIds` is null.

import { describe, expect, it } from "vitest";
import { filterEntries } from "./filter";
import type { RomEntry } from "./types";
import type { SystemId } from "@oa/platform/themes/registry";
import type { SidebarView } from "@oa/platform/layout/types";

function entry(id: string, systemId: string, title = id): RomEntry {
  return {
    id,
    title,
    systemId: systemId as SystemId,
    filePath: `/roms/${id}`,
    addedAt: 0,
  };
}

const ALL: RomEntry[] = [
  entry("a", "nes", "Alpha"),
  entry("b", "nes", "Bravo"),
  entry("c", "snes", "Charlie"),
];

const VIEW: SidebarView = { kind: "all" };

describe("filterEntries", () => {
  it("slices by system when viewSystemIds is set and viewRomIds is null", () => {
    const out = filterEntries(ALL, VIEW, "", ["nes" as SystemId], null);
    expect(out.map((e) => e.id)).toEqual(["a", "b"]);
  });

  it("slices by explicit rom-id set when viewRomIds is non-null", () => {
    const ids = new Set(["a", "c"]);
    const out = filterEntries(ALL, VIEW, "", null, ids);
    expect(out.map((e) => e.id)).toEqual(["a", "c"]);
  });

  it("viewRomIds takes precedence over viewSystemIds", () => {
    // A collection node resolves to games; the caller passes systemIds=null,
    // but even if both were set the rom-id slice must win.
    const ids = new Set(["c"]);
    const out = filterEntries(ALL, VIEW, "", ["nes" as SystemId], ids);
    expect(out.map((e) => e.id)).toEqual(["c"]);
  });

  it("an empty rom-id set yields no entries", () => {
    const out = filterEntries(ALL, VIEW, "", null, new Set());
    expect(out).toEqual([]);
  });

  it("applies the search query on top of the rom-id slice", () => {
    const ids = new Set(["a", "b"]);
    const out = filterEntries(ALL, VIEW, "alpha", null, ids);
    expect(out.map((e) => e.id)).toEqual(["a"]);
  });

  it("no view filter (both null) returns everything", () => {
    const out = filterEntries(ALL, VIEW, "", null, null);
    expect(out.map((e) => e.id)).toEqual(["a", "b", "c"]);
  });
});
