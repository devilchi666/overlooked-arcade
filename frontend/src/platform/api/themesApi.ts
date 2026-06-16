// Typed Tauri bridge — themes domain (Theming ARC 2 "P" P.1 S3).
//
// One command: list the on-disk `.oatheme` themes the Rust loader discovers
// under `<exe_dir>/themes/community/<id>/` (+ the reserved `themes/dev/` path).
// The returned descriptors mirror `platform/theme/diskTheme.ts::DiskThemeDescriptor`
// 1:1 (the Rust struct serializes to exactly that shape), so callers map each
// through `diskThemeToPackage` and merge into the active-theme registry alongside
// `BUILTIN_THEMES`. Local + read-only — no network, no mutation.

import { invoke } from "@tauri-apps/api/core";
import type { DiskThemeDescriptor } from "@oa/platform/theme/diskTheme";

/// Discover on-disk themes next to the exe. Returns an empty array when none are
/// installed; the loader skips malformed themes (logged in the Rust log), so a
/// bad theme never surfaces here.
export function listDiskThemes(): Promise<DiskThemeDescriptor[]> {
  return invoke<DiskThemeDescriptor[]>("oa_themes_list_disk");
}
