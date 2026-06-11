// Per-theme settings namespace — a collision-free slice of persisted prefs that
// a theme owns (S5.4, scope-call #9).
//
// Theming Substrate ARC 1 Phase 3 S5.4. The OA-wide settings store
// (platform/settings/store.ts) holds engine/operator prefs; a THEME often wants
// its own small prefs (a density toggle, a layout choice) that must NOT collide
// with OA settings or with another theme's prefs. This module is that namespace.
//
// SHAPE: one localStorage key (`oa.themeSettings`) holding `{ [themeId]: { [key]:
// value } }` — mirroring the settings store's localStorage pattern (frontend-
// owned, per-install per-user, survives the restart-based theme swap). A theme
// reaches ONLY its own slice via `useThemeSettings()`, which auto-binds the
// ACTIVE theme's id — so a theme can't read or clobber another theme's prefs (the
// binding IS the collision rule; a theme never passes a themeId).
//
// REACTIVITY: backed by a Solid `createStore`, so `get(...)` reads track and a
// `set(...)` repaints consumers. Loaded from localStorage at module init (ready
// before any theme mounts); each `set` persists synchronously.

import { createStore, produce } from "solid-js/store";
import { activeTheme } from "./registry";

/// The persisted shape: theme id → that theme's own key/value bag. Values are
/// opaque JSON (each theme declares the shape it reads via the `get` type arg).
type ThemeSettingsMap = Record<string, Record<string, unknown>>;

const STORAGE_KEY = "oa.themeSettings";

function load(): ThemeSettingsMap {
  if (typeof localStorage === "undefined") return {};
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as unknown;
    // Shallow shape guard: a top-level object of objects. Anything malformed
    // resets to empty rather than throwing into module init.
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
      return parsed as ThemeSettingsMap;
    }
    return {};
  } catch {
    return {};
  }
}

const [store, setStore] = createStore<ThemeSettingsMap>(load());

function persist(): void {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(store));
  } catch {
    // Quota / storage disabled — accept the in-memory-only fallback.
  }
}

/// Read a theme's pref reactively. `fallback` is returned when the theme has no
/// value for `key` yet. Generic on the value type — the caller declares what it
/// stored (the namespace is opaque JSON, like the nav-bindings blob).
export function getThemePref<T>(themeId: string, key: string, fallback: T): T {
  const slice = store[themeId];
  const value = slice ? slice[key] : undefined;
  return value === undefined ? fallback : (value as T);
}

/// Write a theme's pref + persist. Creates the theme's slice on first write.
export function setThemePref(themeId: string, key: string, value: unknown): void {
  setStore(
    produce((s) => {
      if (!s[themeId]) s[themeId] = {};
      s[themeId][key] = value;
    }),
  );
  persist();
}

/// Theme-facing accessor — get/set BOUND to the active theme's id. A theme calls
/// `const settings = useThemeSettings()` then `settings.get(key, fallback)` /
/// `settings.set(key, value)`; it never names a theme id, so it can only touch
/// its own slice (the collision-free guarantee). `get` is reactive.
export function useThemeSettings() {
  const themeId = (): string => activeTheme()?.manifest.id ?? "<unknown>";
  return {
    get: <T>(key: string, fallback: T): T => getThemePref(themeId(), key, fallback),
    set: (key: string, value: unknown): void => setThemePref(themeId(), key, value),
  };
}
