// Typed Tauri bridge — emulator-session domain (launch lifecycle / external
// launchers / disc control / region priority).
//
// Theming Phase 4 Slice 5 (PR A, with rewindTasApi). The launch boundary: load
// / unload a ROM, the bootless (no-content) launch path, the external
// standalone-emulator profile registry + per-system launcher pref + active-
// launcher capability probe, the multi-disc swap/eject control, the per-ROM
// selected-variant write, and the global region-priority list. Same convention
// as the other platform/api modules (see docs/PLANS/theming-platform-api-bridge.md):
// one typed named export per command, thin pass-through, no error handling here
// (call sites keep their own try/catch), command-name string lives ONLY here.
//
// `listEmulatorProfiles` is generic (D14) — CoresPage reads the full profile
// shape, PerSystemSettingsBody a subset. The component-local contract types
// (DiscInfo, ActiveLauncherInfo, …) live here per D16 (the platform↛components
// boundary forbids importing them from a component); their sole consumers keep
// their structurally-identical local copies, which stay assignable.

import { invoke } from "@tauri-apps/api/core";
import type { RomEntry } from "@oa/platform/library/types";

// --- Backend-contract types this domain owns ----------------------------

/// The `launch_rom` payload. Mirrors the object launch.ts builds from a
/// RomEntry; every field is forwarded unchanged.
export type LaunchRomArgs = {
  path: string;
  slot: number | null;
  stateFile: string | null;
  coreOverride: string | null;
  archiveInnerPath: string | null;
  entryId: string;
  systemId: string;
};

/// One external-emulator profile (`list_emulator_profiles`). The full shape the
/// Cores Manager renders; the per-system settings body reads a subset and
/// passes its narrower local type to `listEmulatorProfiles`.
export type EmulatorProfileInfo = {
  id: string;
  displayName: string;
  vendor: string;
  officialDownloadUrl: string;
  binaryName: string;
  supportedSystems: string[];
  binaryPath: string | null;
};

/// Which in-game features the ACTIVE launcher supports. Mirror of Rust
/// `LauncherCapabilities` (oa_core). In-process libretro reports all-true; an
/// external standalone emulator reports all-false in v1.
export type LauncherCapabilities = {
  supportsSavestate: boolean;
  supportsRewind: boolean;
  supportsRunAhead: boolean;
  supportsInputRemap: boolean;
  supportsShaders: boolean;
  supportsCoreOptions: boolean;
  supportsDiscControl: boolean;
  supportsFrameCapture: boolean;
};

/// The active-launcher identity + capability set
/// (`get_active_launcher_capabilities`). Mirror of Rust `ActiveLauncherInfo`.
export type ActiveLauncherInfo = {
  launcherId: string;
  launcherName: string;
  isExternal: boolean;
  capabilities: LauncherCapabilities;
};

/// Multi-disc state for the current session (`get_disc_state`). v1 cores leave
/// `paths` empty; `labels` is the primary visible string.
export type DiscInfo = {
  numDiscs: number;
  currentIndex: number;
  ejected: boolean;
  labels: string[];
  paths: string[];
};

// --- Launch lifecycle ---------------------------------------------------

/// Load a ROM in-process (or hand off to the system's external launcher).
export function launchRom(args: LaunchRomArgs): Promise<void> {
  return invoke("launch_rom", args);
}

/// Unload the current ROM / terminate the active session.
export function unloadRom(title: string | null): Promise<void> {
  return invoke("unload_rom", { title });
}

/// Whether the system's in-process core supports `retro_load_game(NULL)`.
export function systemSupportsBootless(systemId: string): Promise<boolean> {
  return invoke<boolean>("system_supports_bootless", { systemId });
}

/// Boot a system's core with no content (DOSBox-Pure / ScummVM built-in menu).
export function bootWithoutGame(systemId: string): Promise<void> {
  return invoke("boot_without_game", { systemId });
}

// --- External launchers -------------------------------------------------

/// The external-emulator profile registry. Generic (D14): canonical shape is
/// `EmulatorProfileInfo`; the per-system settings body reads a subset.
export function listEmulatorProfiles<T = EmulatorProfileInfo>(): Promise<T[]> {
  return invoke<T[]>("list_emulator_profiles");
}

/// Set (or clear, with null) the operator-set binary path for a profile.
export function setEmulatorBinaryPath(profileId: string, path: string | null): Promise<void> {
  return invoke("set_emulator_binary_path", { profileId, path });
}

/// The per-system default-launcher pref profile id, or null = libretro core.
export function getLauncherPref(systemId: string): Promise<string | null> {
  return invoke<string | null>("get_launcher_pref", { systemId });
}

/// Set (or clear, with null) the per-system default-launcher pref.
export function setLauncherPref(systemId: string, profileId: string | null): Promise<void> {
  return invoke("set_launcher_pref", { systemId, profileId });
}

/// The active launcher's identity + capabilities (governs QuickSettings gating).
export function getActiveLauncherCapabilities(): Promise<ActiveLauncherInfo> {
  return invoke<ActiveLauncherInfo>("get_active_launcher_capabilities");
}

// --- Disc control -------------------------------------------------------

/// Multi-disc state for the current session, or null when not a multi-disc game.
export function getDiscState(): Promise<DiscInfo | null> {
  return invoke<DiscInfo | null>("get_disc_state");
}

/// Swap to disc `index` in a multi-disc set.
export function setDiscImage(index: number): Promise<void> {
  return invoke("set_disc_image", { index });
}

/// Eject / insert the current disc (the swap UI ejects → set_disc_image →
/// inserts).
export function setDiscEject(ejected: boolean): Promise<void> {
  return invoke("set_disc_eject", { ejected });
}

/// The member ROMs of a disc set (the disc-picker overlay).
export function listDiscSetMembers(discSetId: number): Promise<RomEntry[]> {
  return invoke<RomEntry[]>("list_disc_set_members", { discSetId });
}

/// Persist the operator's selected art/title variant for a ROM.
export function setSelectedVariant(romId: string, kind: string, index: number): Promise<void> {
  return invoke("set_selected_variant", { romId, kind, index });
}

// --- Region priority ----------------------------------------------------

/// The global region-priority list used for art/metadata variant ranking.
export function getRegionPriority(): Promise<string[]> {
  return invoke<string[]>("get_region_priority");
}

/// Persist the global region-priority list.
export function setRegionPriority(regions: string[]): Promise<void> {
  return invoke("set_region_priority", { regions });
}
