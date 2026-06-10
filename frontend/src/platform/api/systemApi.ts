// Typed Tauri bridge — system domain (System Info Panel v1 + system status +
// CPU tier + perf stats).
//
// Theming Phase 4 Slice 6 (the closer, with jobsApi + shellApi). The system-
// level surface: the three-layer System Info merge + the operator's L3
// override CRUD (moved here from platform/library/systemInfo per D15, which
// re-exports them for its existing consumers), the storage/system-health
// status rollup, the CPU-tier heuristic, and the live perf-stats poll. Same
// convention as the other platform/api modules (see
// docs/PLANS/theming-platform-api-bridge.md): one typed named export per
// command, thin pass-through, no error handling here, command-name string lives
// ONLY in this file.
//
// The System Info shapes live in platform/library/systemInfo (their domain
// home + widely imported); pulled here via `import type` (erased). The
// status/tier/perf reads are generic (D14) — each has a single consumer with
// its own local view, so the wrapper stays generic rather than naming a type.

import { invoke } from "@tauri-apps/api/core";
import type {
  MergedSystemInfo,
  SystemInfoOverride,
  SystemInfoCurated,
} from "@oa/platform/library/systemInfo";

// --- System Info Panel v1 (moved from systemInfo.ts per D15) -------------

/// Resolve the merged System Info record for one system (L3 > L2 > L1).
export function getSystemInfo(args: { systemId: string }): Promise<MergedSystemInfo> {
  return invoke<MergedSystemInfo>("get_system_info", args);
}

/// Read just the operator's raw L3 overrides (no merge).
export function getSystemInfoOverride(args: { systemId: string }): Promise<SystemInfoOverride> {
  return invoke<SystemInfoOverride>("get_system_info_override", args);
}

/// Read just the L2 curated record (no merge); null when none shipped.
export function getSystemInfoCurated(args: { systemId: string }): Promise<SystemInfoCurated | null> {
  return invoke<SystemInfoCurated | null>("get_system_info_curated", args);
}

/// UPSERT (or DELETE if every field is empty) the operator's L3 overrides.
export function setSystemInfoOverride(args: {
  systemId: string;
  overrideRecord: SystemInfoOverride;
}): Promise<void> {
  return invoke("set_system_info_override", args);
}

/// Blank one operator override (delete the L3 row).
export function deleteSystemInfoOverride(args: { systemId: string }): Promise<void> {
  return invoke("delete_system_info_override", args);
}

/// Reset all overrides for a system to default (explicit-verb alias).
export function resetSystemInfoToDefault(args: { systemId: string }): Promise<void> {
  return invoke("reset_system_info_to_default", args);
}

// --- Status / CPU tier / perf -------------------------------------------

/// The storage / system-health status rollup. Generic (D14): the two call
/// sites read the same `StorageSystemStatus` local shape.
export function getSystemStatus<T>(): Promise<T> {
  return invoke<T>("get_system_status");
}

/// The CPU-tier heuristic snapshot (Guided Setup / Settings perf tier).
export function detectCpuTier<T>(): Promise<T> {
  return invoke<T>("detect_cpu_tier");
}

/// Live emulator perf stats (the PerformanceHud poll).
export function getPerfStats<T>(): Promise<T> {
  return invoke<T>("get_perf_stats");
}
