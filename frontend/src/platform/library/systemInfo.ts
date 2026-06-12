// Frontend bindings for the System Info Panel v1 Tauri commands.
//
// Rust side: apps/oa-shell/src/system_info.rs (data model + merge layer)
// + apps/oa-shell/src/main.rs (Tauri command handlers + the
// bake_system_info_on_launch path that fills L1+L2 tables on startup).
//
// Three-layer data model: L1 (MAME baseline, baked from
// assets/mame-source/), L2 (curated, baked from
// docs/cores/<id>/system-info.yaml), L3 (per-install operator overrides
// in SQLite). The Rust merge function resolves the three layers via
// field-typed precedence (L3 > L2 > L1) and the result is what
// `getSystemInfo` returns.

// Theming Phase 4: the Tauri command wrappers for this domain now live in
// platform/api/ and are re-exported here so existing consumers' import paths
// are untouched. `refresh_mame_system_info` is the media domain (mediaApi,
// Slice 3, D15); the six system-info commands moved to systemApi (Slice 6,
// D15). The shared TYPES still live in this module (below), pulled into
// systemApi via `import type`.
export { refreshMameSystemInfo } from "@oa/platform/api/mediaApi";
export {
  getSystemInfo,
  getSystemInfoOverride,
  getSystemInfoCurated,
  setSystemInfoOverride,
  deleteSystemInfoOverride,
  resetSystemInfoToDefault,
  listSystemInfoOverridden,
} from "@oa/platform/api/systemApi";

/// One operator-facing peripheral row, rendered in SUPPORTED
/// PERIPHERALS as `glyph name`. Matches the Rust `Peripheral` struct
/// (snake_case serde — both fields are lowercase so no rename happens).
export type Peripheral = {
  name: string;
  glyph: string;
};

/// Final per-system record after the field-typed merge of L1 (MAME),
/// L2 (curated YAML), and L3 (operator overrides). Frontend renders
/// this directly — no further merge logic.
///
/// All optional fields fall back to `undefined` when no layer supplied
/// a value; the UI renders "—" for those rows so partial entries
/// degrade gracefully.
export type MergedSystemInfo = {
  systemId: string;

  // SYSTEM INFORMATION section
  manufacturer?: string;
  systemType?: string;
  generation?: string;
  releaseDate?: string;
  discontinued?: string;
  unitsSold?: string;
  media?: string;
  cpu?: string;
  sound?: string;
  resolution?: string;
  colorPalette?: string;
  displayRatio?: string;

  // TECHNICAL DETAILS section
  architecture?: string;
  maxPlayers?: string;
  multiplayer?: string;
  region?: string;
  storage?: string;
  ram?: string;
  videoOutput?: string;
  aspectRatio?: string;
  refreshRate?: string;

  // PERIPHERALS — always populated (could be empty Vec), so the
  // frontend doesn't need to null-check before iterating.
  peripherals: Peripheral[];
  /// Raw MAME control-kind hints ("joy", "lightgun", "trackball").
  /// L1-only; reserved for future filter UIs.
  peripheralHints: string[];

  // Hero extras (HomePage spotlight)
  releaseFlag?: string;
  tagline?: string;
  blurb?: string;
  sidebarSubline?: string;

  /// `history.xml` prose description, L1-only. Renders in the per-system
  /// Settings drill-in as "MAME's bio" — long-form; not shown on HOME.
  description?: string;

  /// True when the operator's L3 carries at least one non-default
  /// field. Drives the "(edited)" indicator in the per-system
  /// Settings drill-in.
  hasLocalEdits: boolean;
};

/// Operator's local overrides — Layer 3 of the data model. Every L2
/// field plus the peripherals list and hero extras can be overridden;
/// `undefined` means "no override, fall through to L2 → L1"; a value
/// (including empty string) wins over both lower layers.
export type SystemInfoOverride = {
  manufacturer?: string;
  systemType?: string;
  generation?: string;
  releaseDate?: string;
  discontinued?: string;
  unitsSold?: string;
  media?: string;
  cpu?: string;
  sound?: string;
  resolution?: string;
  colorPalette?: string;
  displayRatio?: string;
  architecture?: string;
  maxPlayers?: string;
  multiplayer?: string;
  region?: string;
  storage?: string;
  ram?: string;
  videoOutput?: string;
  aspectRatio?: string;
  refreshRate?: string;
  /// `undefined` = no override; `[]` = operator explicitly cleared
  /// the L2 list; `[...]` = operator replaced the L2 list entirely.
  peripherals?: Peripheral[];
  releaseFlag?: string;
  tagline?: string;
  blurb?: string;
  sidebarSubline?: string;
};

/// Empty default — matches Rust's `SystemInfoOverride::default()`.
/// Passing this to `setSystemInfoOverride` deletes the row so the
/// table stays sparse.
export const EMPTY_SYSTEM_INFO_OVERRIDE: SystemInfoOverride = {};

/// One L2 curated record — the per-system YAML at
/// `docs/cores/<id>/system-info.yaml` after parsing + storage in the
/// `system_info_curated` SQLite table. Returned by `getSystemInfoCurated`
/// for the Phase 4 edit UI's provenance-badge logic ("curated" vs
/// "L1 default" distinction). All fields camelCase to match the rest
/// of the wire shape; the underlying YAML stays snake_case via serde
/// aliases.
export type SystemInfoCurated = {
  systemId: string;
  manufacturer?: string;
  systemType?: string;
  generation?: string;
  releaseDate?: string;
  discontinued?: string;
  unitsSold?: string;
  media?: string;
  cpu?: string;
  sound?: string;
  resolution?: string;
  colorPalette?: string;
  displayRatio?: string;
  architecture?: string;
  maxPlayers?: string;
  multiplayer?: string;
  region?: string;
  storage?: string;
  ram?: string;
  videoOutput?: string;
  aspectRatio?: string;
  refreshRate?: string;
  peripherals: Peripheral[];
  releaseFlag?: string;
  tagline?: string;
  blurb?: string;
  sidebarSubline?: string;
};

/// Result of the SETTINGS → Storage → "Refresh MAME system info"
/// action. Frontend renders this as a toast message; `missingSystems`
/// drives a follow-up "N OA systems aren't covered by your MAME
/// release" hint when populated.
export type MameRefreshReport = {
  /// MAME version captured from `mame -version`, e.g. `"0.288"`.
  mameVersion: string;
  /// OA-slug rows written to system_info_mame after the refresh.
  systemsRefreshed: number;
  /// OA slugs the operator's `mame -listxml` didn't cover. In v1
  /// this typically includes `3do`, `msx`, `msx2` — see
  /// `tools/mame-extractor/README.md` "Known L1 description gaps".
  missingSystems: string[];
  /// True when a `history.xml` was found + parsed for descriptions;
  /// false when only the structured fields were updated (operator
  /// hasn't dropped a community-maintained history.xml next to MAME).
  historyPresent: boolean;
  /// Absolute path of the MAME binary used — surfaced in the toast
  /// for transparency ("Refreshed from C:\Emulators\MAME\mame.exe").
  mamePath: string;
};
