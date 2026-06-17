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
import { pushToast } from "@oa/platform/lib/toast";
import type { ThemePackage } from "./types";
import { formatThemeValidation, validateTheme } from "./validate";

/// Lightweight view of a registered theme for the Settings → Appearance
/// picker — just what the picker renders, never the entry component.
export type ThemeChoice = { id: string; name: string };

const [registered, setRegistered] = createSignal<ThemePackage[]>([]);
const [activeId, setActiveId] = createSignal<string | null>(null);
const [resolved, setResolved] = createSignal(false);

// Validity verdict per theme id, computed once at registration (S4). Only VALID
// themes are pickable / activatable; an invalid theme is excluded so it can
// never become the active shell (the fallback policy, DECISIONS D24).
let validity = new Map<string, boolean>();

/// Inject the build-time theme list. Called once by App.tsx at boot with
/// themes/index.ts's BUILTIN_THEMES. Idempotent (replaces).
///
/// S4: every theme is validated here against THEME_CONTRACT.md §6. Errors
/// exclude the theme from the valid set (it can't be activated); they're logged
/// to the console always (an operator may wonder why a theme vanished — the log
/// is captured by the three-output logger). Warnings are dev-loud only. The
/// hard CI gate that stops a drifted built-in from ever shipping is the Vitest
/// suite (validate.test.ts), not this runtime pass.
export function registerThemes(themes: ThemePackage[]): void {
  validity = new Map();
  for (const t of themes) {
    const v = validateTheme(t);
    validity.set(t.manifest.id, v.ok);
    if (!v.ok) {
      console.error(`[oa-theme] invalid theme excluded from the picker:\n${formatThemeValidation(v)}`);
    } else if (v.warnings.length > 0 && import.meta.env.DEV) {
      console.warn(`[oa-theme] theme validation warnings:\n${formatThemeValidation(v)}`);
    }
  }
  setRegistered(themes);
}

/// The registered themes that passed validation — the only ones that can be
/// the active shell or appear in the picker.
function validThemes(): ThemePackage[] {
  return registered().filter((t) => validity.get(t.manifest.id) === true);
}

/// The picker's option list (id + display name per VALID registered theme).
/// [GRAPHICS-LAB] strip-on-ship: experimental themes stay registered + valid +
/// activatable, but are omitted here so the normal Appearance picker hides them
/// (they're reached only via Settings → Experimental). See
/// docs/features/theming-substrate/GRAPHICS_LAB_TESTBED.md.
export function availableThemes(): ThemeChoice[] {
  return validThemes()
    .filter((t) => !t.experimental)
    .map((t) => ({ id: t.manifest.id, name: t.manifest.name }));
}

/// The persisted active theme id (`null` until the boot seed resolves, or
/// when no preference is stored — meaning "use the default").
export const activeThemeId = (): string | null => activeId();

/// True once the boot-time LibraryPrefs read has completed. App.tsx gates
/// the theme mount on this so it renders the persisted theme on first paint
/// rather than flashing the default then swapping.
export const activeThemeResolved = (): boolean => resolved();

/// Resolve the active ThemePackage: the VALID registered theme matching the
/// persisted id, else the first valid theme (the default — Retroverse). An
/// invalid/unknown persisted id therefore falls back to the default (the S4
/// fallback policy; the boot notice is raised in `initActiveTheme`). Returns
/// undefined only before `registerThemes` has run or if NO theme is valid
/// (impossible past the CI gate, which guarantees the default validates).
export function activeTheme(): ThemePackage | undefined {
  const list = validThemes();
  if (list.length === 0) return undefined;
  const id = activeId();
  return list.find((t) => t.manifest.id === id) ?? list[0];
}

/// Seed `activeId` from LibraryPrefs at boot. Failure falls back to the
/// default (null → first registered theme). Always flips `resolved` so the
/// mount gate releases even if the read errors.
export async function initActiveTheme(): Promise<void> {
  let persistedId: string | null = null;
  try {
    const prefs = await getLibraryPrefs<{ activeThemeId?: string | null }>();
    persistedId = prefs.activeThemeId ?? null;
    setActiveId(persistedId);
  } catch {
    setActiveId(null);
  } finally {
    setResolved(true);
  }

  // Fallback notice (S4): the operator persisted a theme that is no longer a
  // valid choice — either renamed/removed (e.g. the wheel→coverflow rename) or
  // now failing validation. `activeTheme()` already falls back to the default;
  // this just makes the silent fallback loud so the operator knows why the
  // shell isn't the one they picked. ARC-1 loudness is a toast + console (the
  // Phase-5 §6 persistent banner is deferred to when real on-disk themes can
  // fail in the field, DECISIONS D24).
  if (persistedId != null && !validThemes().some((t) => t.manifest.id === persistedId)) {
    const fallback = activeTheme();
    pushToast(
      "warn",
      `Theme "${persistedId}" couldn't load — using ${fallback?.manifest.name ?? "the default theme"}. Re-pick a theme in Settings → Themes.`,
    );
  }
}

/// Switch the active theme: persist the id into LibraryPrefs (read-merge-write
/// so other prefs survive — LibraryPrefs has non-defaulted fields), then
/// restart the app (D5). The returned Promise never resolves past the restart
/// call (the process is replaced). No-op if `id` isn't a registered theme or
/// already active.
export async function setActiveTheme(id: string): Promise<void> {
  // Only a VALID registered theme can be activated — the picker already only
  // offers those, this guards a stale/programmatic id.
  if (!validThemes().some((t) => t.manifest.id === id)) return;
  if (id === activeTheme()?.manifest.id) return;
  // Read-merge-write: getLibraryPrefs returns the full camelCase blob;
  // spreading preserves every field (incl. the non-serde-default ones the
  // Rust struct requires) while we override just active_theme_id.
  const prefs = await getLibraryPrefs<Record<string, unknown>>();
  await setLibraryPrefs({ ...prefs, activeThemeId: id });
  await restartApp();
}
