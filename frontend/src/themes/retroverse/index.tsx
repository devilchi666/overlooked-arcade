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

import type { ThemeEntry, ThemePackage } from "@oa/platform/theme/types";
import type { ThemeManifest } from "@oa/platform/theme/manifest";
import RetroverseShell from "./RetroverseShell";

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
};

const RetroverseEntry: ThemeEntry = (_props) => {
  // ARC 1 honors only the "main" surface; RetroverseShell IS that surface.
  // (When multi-monitor surfaces land per D20b, switch on props.surface.)
  return <RetroverseShell />;
};

export const retroverse: ThemePackage = {
  manifest: RETROVERSE_MANIFEST,
  entry: RetroverseEntry,
};
