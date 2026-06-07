// Theme manifest — the typed shape of a theme package's `theme.toml`.
//
// Theming Substrate ARC 1 Phase 2 Slice C. This is the SDK-layer
// contract only: field names mirror the TOML keys verbatim
// (snake_case) so the parsed document maps 1:1 onto this type with
// no casing transform. No loader exists yet — ARC 1 ships themes
// bundled at build time; Phase 5 wires the manifest reader and
// Phase 6 ports Retroverse onto it as the first real theme.
//
// Schema source of truth: docs/PLANS/theming-substrate.md §Phase 2.

/** Context slices a theme can declare it consumes. Mirrors the
 * top-level store keys on `ThemeContextValue` (routes/retroverse/
 * context.tsx — extracted to platform in Phase 5/6). */
export type ThemeContextSlot =
  | "library"
  | "customCollections"
  | "layout"
  | "views"
  | "settings";

/** Screen corner the engine summon icon occupies. ARC 1 always
 * reserves top-right; the union widens if a later arc lets themes
 * relocate it. */
export type ReservedCorner = "top-right";

export type ThemeManifest = {
  /** Stable identifier — directory-safe, lowercase (e.g. "retroverse"). */
  id: string;
  /** Display name shown in Settings → Appearance. */
  name: string;
  /** Theme's own semver (e.g. "1.0.0"). */
  version: string;
  /** Manifest schema revision this file targets. Bump only on
   * breaking manifest changes; current is 1. */
  schema_version: number;
  /** Semver range of the OA shell the theme supports (e.g. "^0.x"). */
  oa_version: string;
  /** Path to the built entry module, relative to the theme root
   * (e.g. "./dist/index.js"). */
  entry: string;
  /** Named export of the entry module that yields the theme's root
   * component. "default" for a default export. */
  entry_export: string;
  /** Route id the shell navigates to on theme mount. Must appear in
   * `routes`. */
  default_route: string;
  /** Route ids the theme registers (e.g. ["home", "library",
   * "collections", "play-now", "discover"]). */
  routes: string[];
  /** Context slices the theme consumes from the engine-provided
   * ThemeContext. */
  context_slots: ThemeContextSlot[];
  /** Engine capabilities the theme refuses to run without
   * (e.g. ["multi-monitor", "attract-mode"]). Empty for themes that
   * run anywhere. */
  required_engine_capabilities: string[];
  /** Corner reserved for the engine summon icon — the one piece of
   * engine-owned chrome every theme must leave room for. */
  reserves_corner: ReservedCorner;
};
