// Per-user per-system LAYOUT overrides (Theming ARC 2 L3 / D32).
//
// The user's "pick your view per system" choices — the top tier of the layout
// resolution cascade (D32): `user override → theme per_system → theme view
// default → engine default` (see layoutResolver.ts). A user can override the
// active layout per (theme, system, view) at runtime; the choice persists across
// the restart-based theme swap.
//
// SHAPE: one localStorage key (`oa.layoutOverrides`) holding
// `{ [themeId]: { [systemId]: { [view]: LayoutPrimitive } } }` — the
// `(theme_id, system_id, view) → layout` namespace D32 names, mirroring the
// per-theme settings store's localStorage pattern (frontend-owned, per-install,
// survives the swap). Keyed by theme id so each theme has its own override set
// (a layout override for Retroverse doesn't leak to CoverFlow).
//
// REACTIVITY: backed by a Solid `createStore`, so a `get` read tracks and a
// `set`/`clear` repaints consumers (the L3 resolver + the L5 override UI). L3a
// ships the store + resolver only — the LibraryView consumer + the editing UI
// arrive in L3b / L5.

import { createStore, produce } from "solid-js/store";
import type { LayoutPrimitive, ViewType } from "./manifest";
import type { SystemId } from "@oa/platform/themes/registry";

/// theme id → system id → view type → chosen layout primitive.
type LayoutOverrideMap = Record<
  string,
  Record<string, Partial<Record<ViewType, LayoutPrimitive>>>
>;

const STORAGE_KEY = "oa.layoutOverrides";

function load(): LayoutOverrideMap {
  if (typeof localStorage === "undefined") return {};
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as unknown;
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
      return parsed as LayoutOverrideMap;
    }
    return {};
  } catch {
    return {};
  }
}

const [store, setStore] = createStore<LayoutOverrideMap>(load());

function persist(): void {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(store));
  } catch {
    // Quota / storage disabled — accept the in-memory-only fallback.
  }
}

/// Read the user's layout override for (theme, system, view), reactively.
/// `undefined` means "no override" → the resolver falls through to the theme /
/// engine tiers.
export function getLayoutOverride(
  themeId: string,
  systemId: SystemId,
  view: ViewType,
): LayoutPrimitive | undefined {
  return store[themeId]?.[systemId]?.[view];
}

/// Set the user's layout override for (theme, system, view) + persist.
export function setLayoutOverride(
  themeId: string,
  systemId: SystemId,
  view: ViewType,
  layout: LayoutPrimitive,
): void {
  setStore(
    produce((s) => {
      if (!s[themeId]) s[themeId] = {};
      if (!s[themeId][systemId]) s[themeId][systemId] = {};
      s[themeId][systemId][view] = layout;
    }),
  );
  persist();
}

/// Clear the user's layout override for (theme, system, view) — falls back to the
/// theme / engine tiers. Prunes now-empty system / theme branches so the stored
/// blob doesn't accrete empty objects.
export function clearLayoutOverride(
  themeId: string,
  systemId: SystemId,
  view: ViewType,
): void {
  setStore(
    produce((s) => {
      const sys = s[themeId]?.[systemId];
      if (!sys) return;
      delete sys[view];
      if (Object.keys(sys).length === 0) delete s[themeId][systemId];
      if (s[themeId] && Object.keys(s[themeId]).length === 0) delete s[themeId];
    }),
  );
  persist();
}
