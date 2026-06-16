// Typed Tauri bridge — content-packs domain.
//
// oa-packs arc Slice 3. The pack-manager surface: prefs (config registry URL
// + the master network toggle), the installed-pack inventory under
// `<exe_dir>/<type>/community/`, the registry browse + download/install/update
// flow, and rollback retention. The pure verify/validate/install logic lives
// in the `oa-packs` Rust crate; these are thin pass-throughs to the shell's
// `packs.rs` commands.
//
// Same convention as coresApi / settingsApi (docs/PLANS/theming-platform-api-
// bridge.md): one typed named export per command, no error handling here (call
// sites keep their own try/catch), and the command-name string lives ONLY in
// this file.
//
// Note the casing split: prefs / installed / rollback shapes are camelCase
// (their Rust structs derive `rename_all = "camelCase"`), but `Registry` /
// `PackEntry` mirror the on-disk `registry.json` wire format, which is
// snake_case (content-packs.md §4) — `size_bytes`, `min_oa_version`,
// `depends_on`, and `type`.

import { invoke } from "@tauri-apps/api/core";

// --- Prefs --------------------------------------------------------------

/// `appDataDir/packs/prefs.json`. `registryUrl` is config, not a constant
/// (CP1); `allowNetwork` is the master toggle (content-packs.md §9).
export type PacksPrefs = {
  registryUrl: string;
  allowNetwork: boolean;
  lastChecked?: string;
};

export function getPrefs(): Promise<PacksPrefs> {
  return invoke<PacksPrefs>("oa_packs_get_prefs");
}

/// Repoint the registry URL. Empty string resets to the seeded default.
export function setRegistryUrl(url: string): Promise<PacksPrefs> {
  return invoke<PacksPrefs>("oa_packs_set_registry_url", { url });
}

/// Flip the master network toggle.
export function setAllowNetwork(allow: boolean): Promise<PacksPrefs> {
  return invoke<PacksPrefs>("oa_packs_set_allow_network", { allow });
}

// --- Installed (local) --------------------------------------------------

/// One installed pack (`<exe_dir>/<type>/community/<id>/`).
export type InstalledPack = {
  id: string;
  packType: string;
  name: string;
  version: string;
  license?: string;
  path: string;
};

export function listInstalled(): Promise<InstalledPack[]> {
  return invoke<InstalledPack[]>("oa_packs_list");
}

/// Uninstall — moves the pack into rollback retention (§8), not a hard delete.
export function uninstall(packId: string): Promise<void> {
  return invoke("oa_packs_uninstall", { packId });
}

// --- Rollback retention (local) -----------------------------------------

/// A recoverable prior version under `<data_dir>/packs-rollback/`.
export type RollbackEntry = {
  id: string;
  packType: string;
  name: string;
  version: string;
  archivedAt?: string;
  path: string;
};

export function listRollbacks(): Promise<RollbackEntry[]> {
  return invoke<RollbackEntry[]>("oa_packs_list_rollbacks");
}

/// Restore a retained version (a reversible swap with the active version).
export function rollback(packId: string, version: string): Promise<InstalledPack> {
  return invoke<InstalledPack>("oa_packs_rollback", { packId, version });
}

/// Permanently discard a retained version.
export function discardRollback(packId: string, version: string): Promise<void> {
  return invoke("oa_packs_discard_rollback", { packId, version });
}

// --- Registry + network (gated by allowNetwork) -------------------------

/// One pack as listed in `registry.json`. Snake_case to match the wire
/// format. Optional fields mirror the additive-friendly Rust schema.
export type PackEntry = {
  id: string;
  type: string;
  name: string;
  version: string;
  url: string;
  sha256: string;
  size_bytes?: number;
  depends_on?: string[];
  min_oa_version?: string;
  license?: string;
  homepage?: string;
  summary?: string;
  maintainer?: string;
};

export type Registry = {
  registry_version: number;
  updated?: string;
  packs: PackEntry[];
};

/// Fetch + parse the registry from the configured URL. Network-gated —
/// rejects with a `NETWORK_DISABLED:`-prefixed error when the toggle is OFF.
/// Operator-initiated only (never call on mount).
export function fetchRegistry(): Promise<Registry> {
  return invoke<Registry>("oa_packs_fetch_registry");
}

/// Download + verify + install a registry pack by id. Network-gated.
export function install(packId: string): Promise<InstalledPack> {
  return invoke<InstalledPack>("oa_packs_install", { packId });
}

/// Result of an update check + apply.
export type UpdateOutcome = {
  updated: boolean;
  fromVersion: string;
  toVersion: string;
};

/// Update an installed pack to the registry version if newer. Network-gated.
export function update(packId: string): Promise<UpdateOutcome> {
  return invoke<UpdateOutcome>("oa_packs_update", { packId });
}

// --- Network audit log (local) ------------------------------------------

/// One logged network call (content-packs.md §9). Newest-first from
/// `getNetworkLog`.
export type NetLogEntry = {
  at?: string;
  /// `registry` | `install:<id>` | `update:<id>`.
  action: string;
  url: string;
  /// `ok` | `error`.
  outcome: string;
  detail?: string;
};

/// The per-call network audit trail, newest first. Local (no network).
export function getNetworkLog(): Promise<NetLogEntry[]> {
  return invoke<NetLogEntry[]>("oa_packs_get_network_log");
}

/// Erase the network audit log. Local.
export function clearNetworkLog(): Promise<void> {
  return invoke("oa_packs_clear_network_log");
}

// --- Emulator-recipe overrides (local; Slice 5) -------------------------

/// One active recipe override from an installed `emulator-recipes` pack.
export type RecipeOverride = {
  /// The emulator profile id (`bizhawk`).
  id: string;
  /// The pack that provided it.
  packId: string;
  /// True when it replaced a bundled baseline recipe; false when the pack
  /// introduced a new emulator.
  replacedBaseline: boolean;
};

/// Two packs provided the same emulator id; `winner` is in effect.
export type RecipeConflict = {
  id: string;
  winner: string;
  losers: string[];
};

export type RecipeOverridesReport = {
  overrides: RecipeOverride[];
  conflicts: RecipeConflict[];
};

/// The active emulator-recipe overrides + conflicts. Local, no network.
export function recipeOverrides(): Promise<RecipeOverridesReport> {
  return invoke<RecipeOverridesReport>("oa_packs_recipe_overrides");
}

/// Hot-reload the recipe override tier after a pack change (re-reads the
/// bundled baseline + installed `emulator-recipes` packs and swaps the
/// in-memory snapshot), so overrides update without an app restart. Returns
/// the refreshed report. Local, no network.
export function reloadRecipes(): Promise<RecipeOverridesReport> {
  return invoke<RecipeOverridesReport>("oa_packs_reload_recipes");
}

/// True if `err` is the synchronous network-disabled refusal from any gated
/// command — lets a panel route the operator to the network toggle instead
/// of showing a raw error.
export function isNetworkDisabled(err: unknown): boolean {
  const msg = err instanceof Error ? err.message : String(err);
  return msg.includes("NETWORK_DISABLED");
}
