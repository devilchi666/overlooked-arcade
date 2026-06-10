// Typed Tauri bridge — cores domain (installed cores / catalog / core-options
// / BIOS).
//
// Theming Phase 4 Slice 4. The libretro-core lifecycle surface: the installed-
// core inventory (`<exe_dir>/cores/*.dll`), the buildbot catalog + download/
// install/remove flow, the per-system default-core pref, the RetroArch-parity
// core-options table (per-system + per-game overrides), and the BIOS inventory
// + install path. Same convention as settingsApi / libraryApi / mediaApi (see
// docs/PLANS/theming-platform-api-bridge.md): one typed named export per
// command, thin pass-through, no error handling here (call sites keep their own
// try/catch), and the command-name string lives ONLY in this file
// (grep-verifiable).
//
// Shape-divergent reads follow D14 — generic with a canonical default — where
// multiple call sites hold their own partial view of the same backend payload
// (`listCores`, `availableCores`, `getBiosStatus`). The api layer can't import
// component-local types (the enforced platform↛components boundary), so the
// canonical defaults live here; call sites keep their exact local view via the
// type arg. Single-consumer shapes are defined here outright and imported back
// by the (one) consumer.

import { invoke } from "@tauri-apps/api/core";
import type { CoreEntry } from "@oa/platform/settings/store";

// --- Backend-contract types this domain owns ----------------------------

/// One catalog entry `core_installer::available_cores` returns. The richest
/// view (the Cores Manager card). The readiness checklist / bulk prompt read a
/// structural subset — they pass their narrower local type to `availableCores`.
export type AvailableCore = {
  base: string;
  fileName: string;
  displayName: string;
  blurb: string;
  systems: string[];
  installed: boolean;
  installedVersion?: string;
  supportedOnHost: boolean;
  buildbotUrl?: string;
  recommended: boolean;
  biosRequired?: string;
};

/// One selectable value for a libretro core option.
export type CoreOptionValue = {
  value: string;
  label: string | null;
};

/// One libretro core option (RetroArch SET_CORE_OPTIONS parity).
export type CoreOption = {
  key: string;
  desc: string;
  info: string | null;
  categoryKey: string | null;
  defaultValue: string;
  values: CoreOptionValue[];
};

/// A core-options category header.
export type CoreOptionCategory = {
  key: string;
  desc: string;
  info: string | null;
};

/// The per-system core-options snapshot (`list_core_options`): the schema +
/// the current per-system / per-game override values + the core's dynamically
/// hidden keys (SET_CORE_OPTIONS_DISPLAY parity).
export type CoreOptionsSnapshot = {
  schema: CoreOption[];
  categories: CoreOptionCategory[];
  systemValues: Record<string, string>;
  gameValues: Record<string, string>;
  hiddenKeys: string[];
};

/// Guided Setup Phase 2 — the tier-aware recommendation
/// `recommended_core_for_system` returns. Mirror of the Rust
/// `CoreRecommendation` struct (camelCase via serde).
export type CoreRecommendation = {
  core: string;
  tier: "high" | "mid" | "low";
  tierSource: "detected" | "cached" | "override";
  cameFrom: "tierTable" | "systemDefault";
};

/// Result of a manual BIOS install (`install_bios_file`).
export type InstallResult = {
  status: "ok" | "unknownHash";
  sha1: string;
  destination: string;
};

/// Canonical BIOS-status shape (`get_bios_status`). The default for
/// `getBiosStatus`; rollup call sites read a narrower entry shape and pass
/// their own type arg.
export type BiosStatusResponse = {
  systemDir: string;
  entries: unknown[];
};

// --- Installed cores ----------------------------------------------------

/// List the cores installed in `<exe_dir>/cores/`. Generic (D14): the contract
/// shape is `CoreEntry`, but several call sites read a narrower local view.
export function listCores<T = CoreEntry>(): Promise<T[]> {
  return invoke<T[]>("list_cores");
}

/// The per-system default-core pref filename, or null if unset (auto-detect).
export function getCorePref(systemId: string): Promise<string | null> {
  return invoke<string | null>("get_core_pref", { systemId });
}

/// Set (or clear, with null) the per-system default-core pref.
export function setCorePref(systemId: string, fileName: string | null): Promise<void> {
  return invoke("set_core_pref", { systemId, fileName });
}

/// Validate + copy a hand-picked .dll into `<exe_dir>/cores/`. Returns a
/// human-readable status string.
export function installCoreFromPath(path: string): Promise<string> {
  return invoke<string>("install_core_from_path", { path });
}

/// Delete an installed core .dll from `<exe_dir>/cores/`.
export function removeInstalledCore(fileName: string): Promise<void> {
  return invoke("remove_installed_core", { fileName });
}

// --- Buildbot catalog ---------------------------------------------------

/// The buildbot catalog of downloadable cores with install state. Generic
/// (D14): the canonical shape is `AvailableCore`, but the readiness checklist /
/// bulk prompt read a structural subset.
export function availableCores<T = AvailableCore>(): Promise<T[]> {
  return invoke<T[]>("available_cores");
}

/// Download + install one catalog core by its `base` name. `parentJobId` ties
/// the child download to a bulk-install parent row when present. Returns the
/// download job id.
export function downloadCore(base: string, parentJobId?: number): Promise<string> {
  return invoke<string>("download_core", { base, parentJobId });
}

/// Create the parent "Installing N cores" job for a bulk install. Returns the
/// parent job id (≤0 = JobRegistry not managed; downloads still proceed).
export function startBulkCoreInstall(n: number): Promise<number> {
  return invoke<number>("start_bulk_core_install", { n });
}

/// The tier-aware recommended core for a system (Guided Setup Phase 2).
export function recommendedCoreForSystem(systemId: string): Promise<CoreRecommendation> {
  return invoke<CoreRecommendation>("recommended_core_for_system", { systemId });
}

// --- Core options -------------------------------------------------------

/// The core-options snapshot for a system (optionally scoped to a game for the
/// per-game override view).
export function listCoreOptions(systemId: string, gameId: string | null): Promise<CoreOptionsSnapshot> {
  return invoke<CoreOptionsSnapshot>("list_core_options", { systemId, gameId });
}

/// Whether the system's effective core ships a core-options schema.
export function hasCoreOptionsSchema(systemId: string): Promise<boolean> {
  return invoke<boolean>("has_core_options_schema", { systemId });
}

/// Set (or clear, with null) a per-system core-option value.
export function setSystemCoreOption(systemId: string, key: string, value: string | null): Promise<void> {
  return invoke("set_system_core_option", { systemId, key, value });
}

/// Set (or clear, with null) a per-game core-option override.
export function setGameCoreOption(gameId: string, key: string, value: string | null): Promise<void> {
  return invoke("set_game_core_option", { gameId, key, value });
}

/// Push the resolved per-game core options to the running emulator at launch.
export function applyGameCoreOptions(gameId: string): Promise<void> {
  return invoke("apply_game_core_options", { gameId });
}

// --- BIOS ---------------------------------------------------------------

/// The BIOS inventory across systems. Generic (D14): the canonical shape is
/// `BiosStatusResponse`, but rollup call sites read a narrower entry view.
export function getBiosStatus<T = BiosStatusResponse>(): Promise<T> {
  return invoke<T>("get_bios_status");
}

/// Install a hand-picked BIOS file for a system, validating its SHA-1.
export function installBiosFile(
  sourcePath: string,
  targetFilename: string,
  systemId: string,
): Promise<InstallResult> {
  return invoke<InstallResult>("install_bios_file", { sourcePath, targetFilename, systemId });
}

/// Reveal the active `<exe_dir>/system/` BIOS folder in the OS file manager.
export function openBiosFolder(): Promise<void> {
  return invoke("open_bios_folder");
}
