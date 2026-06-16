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

/** The distinct screens of the library journey a theme composes + styles
 * (Theming ARC 2 L2 / D32). A theme MAY declare a layout per view and vary it
 * per system. ARC 2 validates the full set here; the engine *honors* a growing
 * subset (the L3 resolver wires `game-browse` first) — "reserve the contract,
 * defer the body", like D20b's surfaces. The union extends (home /
 * collections-browse / now-playing) when those views are honored. */
export type ViewType =
  | "manufacturer-browse"
  | "system-browse"
  | "game-browse"
  | "game-details";

/** The layout primitives a view can mount — the S5.5 nav-primitive set. The L3
 * resolver maps each to its `@oa/platform/nav` component; the manifest names them
 * as plain string-literals so this contract stays decoupled from the nav layer
 * (like `glyph_set`). `wheel` is the reserved radial primitive (S5.5 stub until
 * L4). */
export type LayoutPrimitive = "list" | "grid" | "carousel" | "wheel" | "custom";

/** Per-view layout declaration: a default `layout`, optionally overridden per
 * system (D32 — e.g. TG-16 → wheel, Lynx → grid). `per_system` keys are SystemIds
 * (kept loose `string` here so the manifest type doesn't couple to the registry;
 * the validator checks them against `SYSTEM_PALETTES` / SystemId). */
export type ViewLayoutConfig = {
  /** The theme's default layout for this view. OPTIONAL (L3b / D40): a theme may
   * declare only `per_system` overrides and leave the view-wide default to the
   * consumer's own fallback (e.g. game-browse falls back to the global
   * capsule/list `viewMode` toggle — the "coexist" model). Omit to NOT override
   * that default for undeclared systems. */
  layout?: LayoutPrimitive;
  per_system?: Record<string, LayoutPrimitive>;
};

/** A theme's per-view layout map (Theming ARC 2 L2). Optional — a theme that
 * declares nothing falls back to the engine default per view (set by the L3
 * resolver). Omit a view to inherit its engine default. */
export type ThemeViews = Partial<Record<ViewType, ViewLayoutConfig>>;

/** All view types the contract names — the validator's allow-list + the L3
 * resolver's iteration set. */
export const VIEW_TYPES: readonly ViewType[] = [
  "manufacturer-browse",
  "system-browse",
  "game-browse",
  "game-details",
];

/** All layout primitives the contract names — the validator's allow-list. */
export const LAYOUT_PRIMITIVES: readonly LayoutPrimitive[] = [
  "list",
  "grid",
  "carousel",
  "wheel",
  "custom",
];

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

/** The view-transition presets the declarative shell can play when a view
 * changes (Theming ARC 3 Thrust M, M1). `none` = no animation (the floor a
 * theme that doesn't declare motion inherits); `fade` / `slide` / `scale` are
 * the M1 starter set. The union widens (M2/M3) without a contract break — a
 * theme declaring an unknown preset is a disqualifying validator ERROR, so a
 * typo fails loud rather than silently rendering motionless. */
export type ViewTransitionPreset = "none" | "fade" | "slide" | "scale";

/** All view-transition presets the contract names — the validator's allow-list
 * + the M1 resolver's recognized set. */
export const VIEW_TRANSITION_PRESETS: readonly ViewTransitionPreset[] = [
  "none",
  "fade",
  "slide",
  "scale",
];

/** A declarative view-transition declaration (M1). DATA only (D50) — a preset
 * selection plus optional timing; the engine `DeclarativeShell` honors it on
 * the UI/DOM layer (D51), interruptibly, and downgrades to a short fade under
 * `prefers-reduced-motion`. `duration` / `easing` are CSS value strings; when
 * omitted the resolver falls back to the motion-token defaults
 * (`--motion-medium` / a concrete decelerate curve). NOTE: `easing` should be a
 * concrete CSS easing (a keyword or `cubic-bezier(...)`) — it drives the Web
 * Animations API, which does not resolve `var()`. */
export type ViewTransitionConfig = {
  preset: ViewTransitionPreset;
  /** CSS time string (e.g. "250ms", "0.4s"). Optional — defaults to the
   * standard motion duration. */
  duration?: string;
  /** Concrete CSS easing (keyword or cubic-bezier). Optional — defaults to a
   * decelerate curve. */
  easing?: string;
};

/** A theme's declarative motion declaration (Theming ARC 3 Thrust M / D52).
 * Optional — a theme that declares nothing animates nothing (the `none` floor).
 * M1 names exactly one field (`view_transition`); the keyframe/parallax model
 * (M3) widens this additively, like every other manifest contract. snake_case
 * key mirrors the `theme.toml` `[motion.view_transition]` table. */
export type ThemeMotion = {
  view_transition?: ViewTransitionConfig;
};

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
  /** Per-view layout map (Theming ARC 2 L2 / D32): which layout primitive each
   * library-journey view mounts, optionally varied per system. Optional — omit a
   * view to inherit the engine default. **Contract only in L2a; no consumer until
   * the L3 resolver reads it.** Validator checks known view types / primitives /
   * system ids; a malformed entry is a disqualifying ERROR (a broken layout map
   * is worse than none, like `settings_schema`). */
  views?: ThemeViews;
  /** Declarative appearance/options the engine renders generically in
   * Settings → Themes / Appearance, bound to per-theme storage. Optional. The
   * S4 validator checks unique keys + type-correct defaults / ranges / options;
   * a malformed control is a disqualifying ERROR (a broken options panel is
   * worse than none). */
  settings_schema?: ThemeSettingsSchema;
  /** Declarative motion (Theming ARC 3 Thrust M / D52): DATA-only cinematic
   * fields the `DeclarativeShell` honors on the UI/DOM layer (D51). M1 names
   * one field — `view_transition` (the preset played on a view change,
   * interruptible, reduced-motion-aware). Optional — omit to animate nothing.
   * The validator checks the preset against `VIEW_TRANSITION_PRESETS` + the
   * timing strings; a malformed declaration is a disqualifying ERROR (a broken
   * motion decl is worse than none, like `views` / `settings_schema`). */
  motion?: ThemeMotion;
};
