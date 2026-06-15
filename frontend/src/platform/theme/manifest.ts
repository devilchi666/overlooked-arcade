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

/** Named surfaces a theme provides a layout for (DECISIONS D20b seam).
 * ARC 1 honors exactly one — `main`, the primary single-monitor shell.
 * Multi-monitor surfaces (marquee / manual / control-panel) widen this
 * union in a later arc; existing themes just declare more entries, no
 * rewrite. The theme entry component is handed the surface it should
 * render via `ThemeEntryProps.surface` (platform/theme/types.ts). */
export type ThemeSurface = "main";

/** Manifest schema revisions this OA build can load. ARC 1 ships exactly
 * one (`1`). A theme declaring a `schema_version` outside this set is
 * rejected by the S4 validator (`UNSUPPORTED_SCHEMA_VERSION`) and falls back
 * to the default theme. A single set (not a min/max range) is deliberate for
 * ARC 1 — when a breaking schema change lands, add the new revision here and,
 * if old themes should keep loading, a migration; the validator already
 * distinguishes "too new" (declared > max known) from "unknown" in its
 * message. Source of truth for the current revision is the `schema_version`
 * field doc below. */
export const SUPPORTED_SCHEMA_VERSIONS: ReadonlySet<number> = new Set([1]);

/** The newest manifest schema revision this build understands — used only to
 * phrase the validator's mismatch message ("targets a newer schema; update
 * OA" vs. "unsupported schema"). */
export const MAX_SCHEMA_VERSION = Math.max(...SUPPORTED_SCHEMA_VERSIONS);

/** One declarative control in a theme's `settings_schema` (Settings IA Slice 3).
 * The engine renders it generically in Settings → Themes / Appearance, bound to
 * the theme's own per-theme storage (`useThemeSettings()`) under `key`. A
 * discriminated union on `type`; the engine renderer + the S4 validator both
 * switch on it. The point: a theme — including a community one OA has never
 * seen — declares its options here and they surface automatically, with zero
 * engine code per option. */
export type ThemeSettingControl =
  | {
      key: string;
      type: "toggle";
      label: string;
      hint?: string;
      default: boolean;
    }
  | {
      key: string;
      type: "slider";
      label: string;
      hint?: string;
      default: number;
      min: number;
      max: number;
      step: number;
      /** Optional unit suffix shown in the value readout (e.g. "px"). */
      unit?: string;
    }
  | {
      key: string;
      type: "select";
      label: string;
      hint?: string;
      default: string;
      options: ReadonlyArray<{ value: string; label: string }>;
    };

/** A theme's declared appearance/options surface. Optional on the manifest —
 * a theme with nothing to configure omits it. */
export type ThemeSettingsSchema = ReadonlyArray<ThemeSettingControl>;

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
  /** Named surfaces the theme renders (DECISIONS D20b). ARC 1 themes
   * declare `["main"]`; the field exists now so multi-monitor surfaces
   * are an additive declaration later, not a contract rewrite. */
  surfaces: ThemeSurface[];
  /** Controller-glyph set the HintBar paints for this theme (S5.3,
   * scope-call #4). One of the built-in `GlyphSetId`s ("xbox" | "playstation");
   * omit to inherit the default ("xbox"). Kept a loose `string` here (like
   * `routes`) so the manifest type doesn't couple to the nav layer; the S4
   * validator checks it against the `GLYPH_SETS` registry and an unknown value
   * is a non-fatal WARNING (hints fall back to the default — a cosmetic
   * mismatch must not disqualify a theme). The user-facing picker + controller
   * auto-detect are deferred. */
  glyph_set?: string;
  /** Per-system UI consumption opt-in (Theming ARC 2 L1; DECISIONS D33/D34).
   * Per-system UI is a platform *capability*; whether a theme *consumes* it on
   * the shared library grid is the theme's choice. `tiles` = per-system
   * `tileShape` + `interactionStyle` on `LibraryTile`; `sfx` = per-system nav
   * sounds on the grid. Omit the field, or a sub-flag, to inherit OFF — the D33
   * "uniformly theme-opt-in" default (matching how backgrounds + boot are
   * opt-in by component mount). The user's master toggle (Settings) gates ABOVE
   * this as a global off-switch. Per-system *layout* (D32) is a separate `views`
   * field (L2), NOT this. Validator warns + falls back to OFF on a malformed
   * value (a consumption flag shouldn't disqualify a theme). */
  per_system_ui?: { tiles?: boolean; sfx?: boolean };
  /** Declarative appearance/options the engine renders generically in
   * Settings → Themes / Appearance, bound to per-theme storage. Optional. The
   * S4 validator checks unique keys + type-correct defaults / ranges / options;
   * a malformed control is a disqualifying ERROR (a broken options panel is
   * worse than none). */
  settings_schema?: ThemeSettingsSchema;
};
