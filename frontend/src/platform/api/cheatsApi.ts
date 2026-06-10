// Typed Tauri bridge — cheats domain (cheat CRUD / arming / per-system formats
// / live memory search / patch-file picker).
//
// Theming Phase 4 Slice 5 (PR B, with milestonesApi + captureApi). The cheat
// surface: the per-game cheat list + CRUD, arming cheats into the running core,
// the per-system cheat-format catalog (Game Genie / GameShark / …), the live
// memory-search engine (start/peek/filter/end), the memory-region inspector,
// and the soft-patch file picker. Same convention as the other platform/api
// modules (see docs/PLANS/theming-platform-api-bridge.md): one typed named
// export per command, thin pass-through, no error handling here, command-name
// string lives ONLY in this file.
//
// Per D16 the backend-contract types live here (the platform↛components
// boundary forbids importing them from GameDialogs / QuickSettings); the sole
// consumers keep their structurally-identical local copies, which stay
// assignable. The cheat-search `filter` discriminated union is forwarded as a
// generic `F` so this module needn't pull the component's union type in.

import { invoke } from "@tauri-apps/api/core";

// --- Backend-contract types this domain owns ----------------------------

/// One cheat row (`list_cheats` / add / update). `id` absent = new.
export type Cheat = {
  id?: number;
  gameId: string;
  name: string;
  description: string;
  region: string;
  offset: number;
  width: number;
  value: number;
  enabled: boolean;
  kind: string;
  code?: string | null;
};

/// One per-system cheat-format declaration (`list_cheat_formats`).
export type CheatFormat = {
  id: string;
  label: string;
  hint: string;
  validationRegex: string;
  isMemoryPoke: boolean;
};

/// A memory-search result snapshot (`start/peek/filter_cheat_search`).
export type CheatSearchSummary = {
  region: string;
  width: number;
  candidateCount: number;
  top: Array<{ offset: number; currentValue: number; previousValue: number }>;
};

/// A live memory-region window (`read_memory_region`).
export type MemoryRegionInfo = {
  region: string;
  available: boolean;
  totalSize: number;
  offset: number;
  bytes: number[];
};

// --- Cheat CRUD + arming ------------------------------------------------

/// The cheats saved for a game.
export function listCheats(gameId: string): Promise<Cheat[]> {
  return invoke<Cheat[]>("list_cheats", { gameId });
}

/// Insert a new cheat; returns the new row id.
export function addCheat(cheat: Cheat): Promise<number> {
  return invoke<number>("add_cheat", { cheat });
}

/// Update an existing cheat.
export function updateCheat(cheat: Cheat): Promise<void> {
  return invoke("update_cheat", { cheat });
}

/// Delete a cheat by id.
export function deleteCheat(id: number): Promise<void> {
  return invoke("delete_cheat", { id });
}

/// Push the game's enabled cheats into the running core; returns the count.
export function armCheats(gameId: string): Promise<number> {
  return invoke<number>("arm_cheats", { gameId });
}

// --- Formats + memory search --------------------------------------------

/// The cheat-format catalog for a system.
export function listCheatFormats(systemId: string): Promise<CheatFormat[]> {
  return invoke<CheatFormat[]>("list_cheat_formats", { systemId });
}

/// Start a fresh memory search over a region.
export function startCheatSearch(region: string): Promise<CheatSearchSummary> {
  return invoke<CheatSearchSummary>("start_cheat_search", { region });
}

/// Re-read the current candidate set without filtering.
export function peekCheatSearch(): Promise<CheatSearchSummary> {
  return invoke<CheatSearchSummary>("peek_cheat_search");
}

/// Narrow the candidate set by a filter predicate. `filter` is forwarded
/// verbatim (the discriminated union lives in the caller).
export function filterCheatSearch<F>(filter: F): Promise<CheatSearchSummary> {
  return invoke<CheatSearchSummary>("filter_cheat_search", { filter });
}

/// Discard the active memory search.
export function endCheatSearch(): Promise<void> {
  return invoke("end_cheat_search");
}

/// Read a live memory-region window (the QuickSettings memory inspector).
export function readMemoryRegion(
  region: string,
  offset: number,
  length: number,
): Promise<MemoryRegionInfo> {
  return invoke<MemoryRegionInfo>("read_memory_region", { region, offset, length });
}

// --- Patch files --------------------------------------------------------

/// Open a file picker for a soft-patch (IPS/BPS/…); null = cancelled.
export function pickPatchFile(): Promise<string | null> {
  return invoke<string | null>("pick_patch_file");
}
