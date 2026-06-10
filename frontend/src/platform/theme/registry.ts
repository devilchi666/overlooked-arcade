// Active-theme registry — the machinery that picks + switches the active
// whole-shell theme.
//
// Theming Substrate ARC 1 Phase 3 S2 (the walking skeleton / swap gate).
//
// BOUNDARY: this module lives in platform/, which must NEVER import concrete
// themes (themes/* is theme-layer code; platform ↛ theme). So platform owns
// only the MACHINERY — the active-id signal, boot seeding from LibraryPrefs,
// the picker's lightweight {id,name} list, and the persist-then-restart
// switch. The concrete ThemePackage list is INJECTED from above: App.tsx
// imports themes/index.ts and calls `registerThemes(BUILTIN_THEMES)` at boot.
// Same inversion-avoiding pattern as platform/libraryAdmin.ts and
// platform/engineSurface.ts (platform owns a stable surface; App wires the
// concrete behaviour into it — DECISIONS D13).
//
// Persistence (DECISIONS, S2 sign-off): the active id lives on LibraryPrefs
// (`active_theme_id`), the OA-wide install-level prefs bag, read at boot
// before any theme mounts. Switching = read-merge-write that field, then
// restart (D5). Build-time bundled only (D6) — no .oatheme zips in ARC 1.

import { createSignal } from "solid-js";
import { getLibraryPrefs, setLibraryPrefs } from "@oa/platform/api/libraryApi";
import { restartApp } from "@oa/platform/api/shellApi";
import type { ThemePackage } from "./types";

/// Lightweight view of a registered theme for the Settings → Appearance
/// picker — just what the picker renders, never the entry component.
export type ThemeChoice = { id: string; name: string };

const [registered, setRegistered] = createSignal<ThemePackage[]>([]);
const [activeId, setActiveId] = createSignal<string | null>(null);
const [resolved, setResolved] = createSignal(false);

/// Inject the build-time theme list. Called once by App.tsx at boot with
/// themes/index.ts's BUILTIN_THEMES. Idempotent (replaces).
export function registerThemes(themes: ThemePackage[]): void {
  setRegistered(themes);
}

/// The picker's option list (id + display name per registered theme).
export function availableThemes(): ThemeChoice[] {
  return registered().map((t) => ({ id: t.manifest.id, name: t.manifest.name }));
}

/// The persisted active theme id (`null` until the boot seed resolves, or
/// when no preference is stored — meaning "use the default").
export const activeThemeId = (): string | null => activeId();

/// True once the boot-time LibraryPrefs read has completed. App.tsx gates
/// the theme mount on this so it renders the persisted theme on first paint
/// rather than flashing the default then swapping.
export const activeThemeResolved = (): boolean => resolved();

/// Resolve the active ThemePackage: the registered theme matching the
/// persisted id, else the first registered theme (the default — Retroverse).
/// Returns undefined only before `registerThemes` has run.
export function activeTheme(): ThemePackage | undefined {
  const list = registered();
  if (list.length === 0) return undefined;
  const id = activeId();
  return list.find((t) => t.manifest.id === id) ?? list[0];
}

/// Seed `activeId` from LibraryPrefs at boot. Failure falls back to the
/// default (null → first registered theme). Always flips `resolved` so the
/// mount gate releases even if the read errors.
export async function initActiveTheme(): Promise<void> {
  try {
    const prefs = await getLibraryPrefs<{ activeThemeId?: string | null }>();
    setActiveId(prefs.activeThemeId ?? null);
  } catch {
    setActiveId(null);
  } finally {
    setResolved(true);
  }
}

/// Switch the active theme: persist the id into LibraryPrefs (read-merge-write
/// so other prefs survive — LibraryPrefs has non-defaulted fields), then
/// restart the app (D5). The returned Promise never resolves past the restart
/// call (the process is replaced). No-op if `id` isn't a registered theme or
/// already active.
export async function setActiveTheme(id: string): Promise<void> {
  if (!registered().some((t) => t.manifest.id === id)) return;
  if (id === activeTheme()?.manifest.id) return;
  // Read-merge-write: getLibraryPrefs returns the full camelCase blob;
  // spreading preserves every field (incl. the non-serde-default ones the
  // Rust struct requires) while we override just active_theme_id.
  const prefs = await getLibraryPrefs<Record<string, unknown>>();
  await setLibraryPrefs({ ...prefs, activeThemeId: id });
  await restartApp();
}
