// Retroverse — the DEFAULT whole-shell theme (the dogfood).
//
// Theming Substrate ARC 1 Phase 6 (the acceptance gate). Retroverse is now a
// REAL theme: its whole implementation — RetroverseShell + the five route
// pages + GameDetailPanel / SystemInfoPanel + the private tab-routing signal
// (currentRoute) — lives physically under themes/retroverse/ and consumes ONLY
// platform (@oa/platform/*). The S2 thin-wrapper-pointing-at-layout/routes is
// gone, and with it the two eslint `except: ['./retroverse']` boundary
// exceptions — Retroverse proves the SDK hosts the flagship with zero
// exceptions. This entry just declares the manifest + mounts RetroverseShell.

import { onMount } from "solid-js";
import type { ThemeEntry, ThemePackage } from "@oa/platform/theme/types";
import type { ThemeManifest } from "@oa/platform/theme/manifest";
import { usePlatform } from "@oa/platform/platformContext";
import { useThemeSettings } from "@oa/platform/theme/themeSettings";
import {
  LIBRARY_TILE_SIZE_MIN,
  LIBRARY_TILE_SIZE_MAX,
  LIBRARY_TILE_SIZE_STEP,
  LIBRARY_TILE_SIZE_DEFAULT,
} from "@oa/platform/layout/state";
import RetroverseShell from "./RetroverseShell";
import { RETROVERSE_SYSTEM_UI } from "./systemUiConfigs";

/// Authored inline as a typed object in S2 (the manifest reader lands in
/// Phase 5 / S4). Mirrors what Retroverse's `theme.toml` will declare.
const RETROVERSE_MANIFEST: ThemeManifest = {
  id: "retroverse",
  name: "Retroverse",
  version: "1.0.0",
  schema_version: 1,
  oa_version: "^0.x",
  entry: "./index.tsx",
  entry_export: "retroverse",
  default_route: "home",
  routes: ["home", "library", "collections", "play-now", "discover"],
  context_slots: ["library", "customCollections", "layout", "views", "settings"],
  required_engine_capabilities: [],
  reserves_corner: "top-right",
  surfaces: ["main"],
  // Theming ARC 2 L1 (D33/D34): Retroverse is the flagship that consumes the
  // full per-system experience on the shared grid — per-system tile flourishes
  // + nav SFX. CoverFlow / bare omit this and get a uniform grid.
  per_system_ui: { tiles: true, sfx: true },
  // Theming ARC 2 L3b (D32/D40): per-system layout for the game-browse view.
  // NO view-wide `layout` here — that would override every system's global
  // capsule/list toggle (the "coexist" model keeps the toggle as the default).
  // Only `per_system` overrides: NES browses as a text list (demo proving the
  // cascade end-to-end; per-system layout is curated for real in L6). Other
  // systems keep the operator's global viewMode.
  views: {
    "game-browse": { per_system: { nes: "list" } },
  },
  // Settings IA Slice 3 — Retroverse declares its library browse-appearance.
  // The engine renders these in Settings → Themes / Appearance; LibraryView /
  // GridControls read the same per-theme keys. Defaults mirror the pre-Slice-3
  // global defaults so behavior is unchanged out of the box.
  settings_schema: [
    {
      key: "tileSize",
      type: "slider",
      label: "Tile size",
      hint: "Cover width in the capsule grid",
      default: LIBRARY_TILE_SIZE_DEFAULT,
      min: LIBRARY_TILE_SIZE_MIN,
      max: LIBRARY_TILE_SIZE_MAX,
      step: LIBRARY_TILE_SIZE_STEP,
      unit: "px",
    },
    {
      key: "sortKey",
      type: "select",
      label: "Sort by",
      default: "title",
      options: [
        { value: "title", label: "Name" },
        { value: "addedAt", label: "Date added" },
        { value: "year", label: "Year" },
      ],
    },
    {
      key: "groupBy",
      type: "select",
      label: "Group by",
      default: "none",
      options: [
        { value: "none", label: "None" },
        { value: "letter", label: "A–Z" },
        { value: "system", label: "System" },
      ],
    },
    {
      key: "viewMode",
      type: "select",
      label: "Layout",
      default: "capsule",
      options: [
        { value: "capsule", label: "Capsule grid" },
        { value: "list", label: "Detail list" },
      ],
    },
  ],
};

const RetroverseEntry: ThemeEntry = (_props) => {
  // Settings IA Slice 3 — one-time migration of the formerly-GLOBAL library
  // appearance (tile size / sort / group / view mode) into Retroverse's own
  // per-theme namespace, so it becomes theme-specific WITHOUT anything jumping
  // on first run. Seed each key from the live global value only if unset
  // (sentinel: get(key, null) === null). After this, LibraryPage + the engine
  // Appearance panel both read the per-theme keys and stay in sync.
  const platform = usePlatform();
  const ts = useThemeSettings();
  onMount(() => {
    const layout = platform.layout;
    if (ts.get<number | null>("tileSize", null) === null) {
      ts.set("tileSize", layout.libraryTileSize());
    }
    if (ts.get<string | null>("sortKey", null) === null) {
      ts.set("sortKey", layout.sortKey());
    }
    if (ts.get<string | null>("groupBy", null) === null) {
      ts.set("groupBy", layout.groupBy());
    }
    if (ts.get<string | null>("viewMode", null) === null) {
      ts.set("viewMode", layout.viewMode());
    }
  });
  // ARC 1 honors only the "main" surface; RetroverseShell IS that surface.
  // (When multi-monitor surfaces land per D20b, switch on props.surface.)
  return <RetroverseShell />;
};

export const retroverse: ThemePackage = {
  manifest: RETROVERSE_MANIFEST,
  entry: RetroverseEntry,
  // ARC 2 L2b (D34): per-system experiential character (the gb/nes/vectrex
  // pilots) is Retroverse content, merged over BASELINE_UI by uiConfigFor. Only
  // consumed because Retroverse opts into per-system UI (per_system_ui, L1).
  perSystemUiConfigs: RETROVERSE_SYSTEM_UI,
};
