// Typed Tauri bridge for the OA-wide nav bindings (input→verb map).
// Phase 4 convention: the command name lives in exactly one place; the api
// layer owns the backend-contract shape. Persisted in appData as an opaque
// JSON document (`nav_bindings.json`) — the backend stores/returns it verbatim
// and never reasons about the binding shape (that lives in TS).
//
// See platform/nav/navBindings.ts for the consumer + defaults.

import { invoke } from "@tauri-apps/api/core";
import type { NavBindings } from "@oa/platform/nav/navBindings";

/// Read persisted bindings. `null` = no file yet (caller falls back to
/// defaults). The backend round-trips an arbitrary JSON object, so the shape is
/// only trusted to the extent the consumer normalizes it.
export async function getNavBindings(): Promise<NavBindings | null> {
  return invoke<NavBindings | null>("get_nav_bindings");
}

/// Persist the bindings map to appData.
export async function saveNavBindings(bindings: NavBindings): Promise<void> {
  await invoke("set_nav_bindings", { bindings });
}
