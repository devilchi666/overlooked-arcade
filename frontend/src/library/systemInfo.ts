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

import { invoke } from "@tauri-apps/api/core";

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

/// Resolve the merged System Info record for one system. Always
/// returns a record — even when L1/L2/L3 are all absent, the merged
/// shape is a systemId-stamped struct with every field undefined,
/// which the panel renders as "—" rows.
export async function getSystemInfo(args: {
  systemId: string;
}): Promise<MergedSystemInfo> {
  return invoke<MergedSystemInfo>("get_system_info", args);
}

/// Read just the operator's raw L3 overrides for one system (without
/// merging in L1+L2). Drives the per-system Settings drill-in form,
/// where each field shows its CURRENT override value alongside a
/// "(L1)" / "(curated)" / "(edited)" provenance hint.
export async function getSystemInfoOverride(args: {
  systemId: string;
}): Promise<SystemInfoOverride> {
  return invoke<SystemInfoOverride>("get_system_info_override", args);
}

/// UPSERT (or DELETE if every field is empty) the operator's L3
/// overrides for one system. Replace semantics — the backend does
/// not merge the incoming payload with any existing row.
export async function setSystemInfoOverride(args: {
  systemId: string;
  overrideRecord: SystemInfoOverride;
}): Promise<void> {
  return invoke("set_system_info_override", args);
}

/// Convenience: blank one operator override. Equivalent to
/// `setSystemInfoOverride(systemId, EMPTY_SYSTEM_INFO_OVERRIDE)` but
/// reads more explicitly at call sites.
export async function deleteSystemInfoOverride(args: {
  systemId: string;
}): Promise<void> {
  return invoke("delete_system_info_override", args);
}

/// Alias for `deleteSystemInfoOverride` with a more explicit verb.
/// The per-system Settings drill-in's "Reset all overrides for this
/// system" affordance uses this name to make the destructive intent
/// obvious to the operator.
export async function resetSystemInfoToDefault(args: {
  systemId: string;
}): Promise<void> {
  return invoke("reset_system_info_to_default", args);
}
