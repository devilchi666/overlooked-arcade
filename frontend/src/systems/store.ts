import { createEffect } from "solid-js";
import { createStore } from "solid-js/store";
import type { SystemId } from "../themes/registry";

// Per-system settings live at localStorage[oa.core.<id>.v1], one bucket per
// core. This file is the OA-wide skeleton; each core's actual schema lives
// next to the system page that owns it (e.g., frontend/src/systems/tg16/).
// See feedback memory: settings split into three tiers (OA-wide / per-system /
// per-game). This is the per-system tier.

export type SystemSettings = {
  // Intentionally empty for now. Per-system fields land here as their UI
  // sections come online: buttonMapping, region, shaderPreset, etc.
};

function storageKey(systemId: SystemId): string {
  return `oa.core.${systemId}.v1`;
}

function load(systemId: SystemId): SystemSettings {
  try {
    const raw = localStorage.getItem(storageKey(systemId));
    if (!raw) return {};
    const parsed = JSON.parse(raw) as unknown;
    if (parsed && typeof parsed === "object") return parsed as SystemSettings;
    return {};
  } catch {
    return {};
  }
}

function save(systemId: SystemId, state: SystemSettings): void {
  try {
    localStorage.setItem(storageKey(systemId), JSON.stringify(state));
  } catch {
    // Storage disabled / quota — keep in-memory only.
  }
}

export function createSystemStore(systemId: SystemId) {
  const [state, setState] = createStore<SystemSettings>(load(systemId));

  createEffect(() => {
    save(systemId, { ...state });
  });

  return { systemId, state, setState };
}

export type SystemStore = ReturnType<typeof createSystemStore>;
