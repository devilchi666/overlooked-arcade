// Typed Tauri bridge — views + layout domain.
//
// Theming Phase 4 Slice 2. The sidebar-views config (`views.json`) and the
// layout/geometry prefs (`layout.json`) round-trip through these four
// commands. Same convention as settingsApi (see
// docs/PLANS/theming-platform-api-bridge.md): one typed named export per
// command, thin pass-through, command string lives only here.

import { invoke } from "@tauri-apps/api/core";
import type { ViewsConfig } from "@oa/platform/views/types";
import type { LayoutPrefs } from "@oa/platform/layout/state";

/// Read the persisted views config (`null` when no file exists yet — the
/// store then seeds defaults / migrates from legacy layout.systemOrder).
export function getViews(): Promise<ViewsConfig | null> {
  return invoke<ViewsConfig | null>("get_views");
}

/// Write the full views config back to disk.
export function setViews(config: ViewsConfig): Promise<void> {
  return invoke("set_views", { config });
}

/// Read the layout/geometry prefs. `LayoutPrefs` is the full shape; the
/// views store reads only `{ systemOrder }` during migration and passes its
/// own narrower `T`.
export function getLayout<T = LayoutPrefs>(): Promise<T> {
  return invoke<T>("get_layout");
}

/// Write the full layout prefs back to disk.
export function setLayout(prefs: LayoutPrefs): Promise<void> {
  return invoke("set_layout", { prefs });
}
