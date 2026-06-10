// Retroverse — the DEFAULT whole-shell theme (the dogfood).
//
// Theming Substrate ARC 1 Phase 3 S2 (the walking skeleton). Per the S2
// sign-off this is a THIN WRAPPER: it points at the existing Retroverse
// implementation (layout/retroverse/RetroverseShell + routes/retroverse/*),
// which stay exactly where they are. The full physical move of those files
// INTO themes/retroverse/ is Phase 6 proper — S2's job is only to prove the
// swap gate, not to relocate ~20 files.
//
// The wrapper imports from layout/retroverse (a layer the CoverFlow theme may
// NOT touch). That's the one deliberate exception encoded in the eslint
// `themes ↛ routes` zone (it `except`s this folder) — Retroverse-the-theme
// is defined as a pointer at the existing routes during S2.

import type { ThemeEntry, ThemePackage } from "@oa/platform/theme/types";
import type { ThemeManifest } from "@oa/platform/theme/manifest";
import RetroverseShell from "../../layout/retroverse/RetroverseShell";

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
