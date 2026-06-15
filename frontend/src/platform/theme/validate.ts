// Theme load-time validator — turns THEME_CONTRACT.md §6 from a documented
// contract into a machine-checked one.
//
// Theming Substrate ARC 1 Phase 3 S4 (the versioned manifest + load-time
// validator, docs/PLANS/theming-substrate.md §13.3 + scope-calls #2/#7).
//
// `validateTheme(pkg)` is a PURE function over a ThemePackage's declarative
// surface — its manifest + its token overrides. It returns structured
// errors/warnings; it does NOT throw and does NOT touch the DOM. Two callers:
//   1. registry.ts at registration time (dev-loud + drives the fallback to the
//      default theme when the persisted active theme is invalid).
//   2. The Vitest CI gate (validate.test.ts) over BUILTIN_THEMES + the `bare`
//      fixture, so a built-in theme drifting from the contract fails the build.
//
// SCOPE OF ENFORCEMENT (DECISIONS D24, THEME_CONTRACT.md §6). The validator
// checks the DECLARATIVE surface only — the manifest fields and the typed
// `tokens` object. That covers §6's manifest/schema/surfaces/capabilities/token
// checks AND the data half of the "no engine-var override" rule: a theme can
// only set token keys that map to known, sibling-scoped CSS vars (TOKEN_VAR),
// so even a hostile token VALUE cannot escape the theme mount.
//
// What the validator CANNOT see: a theme's entry is an opaque Solid component.
// A `<style>:root{…}</style>` in its JSX, a `document.head.appendChild`, or an
// imported global stylesheet are invisible to a package-object validator. The
// real protection against those is STRUCTURAL — the S3 sibling-scope mount (a
// theme's scoped tokens physically cannot reach engine territory) plus the
// ESLint layer boundary (a theme can't import engine/). Static source
// inspection of that bypass is a deferred Phase-5 concern (untrusted on-disk
// authors); built-in themes are reviewed. See THEME_CONTRACT.md §6.

import type { ThemePackage } from "./types";
import { TOKEN_VAR, type ThemeTokens } from "./tokens";
import {
  LAYOUT_PRIMITIVES,
  MAX_SCHEMA_VERSION,
  SUPPORTED_SCHEMA_VERSIONS,
  VIEW_TYPES,
  type ThemeSurface,
} from "./manifest";
import {
  PALETTE_VAR,
  SYSTEM_PALETTES,
  type SystemPalette,
} from "@oa/platform/themes/systemPalettes";
import { GLYPH_SETS } from "@oa/platform/nav";

/// Surfaces the shell honors today. ARC 1 = exactly `main` (D20b). The named
/// surfaces seam widens this when multi-monitor lands; a theme declaring a
/// surface outside this set is rejected so it can't silently no-op.
export const HONORED_SURFACES: ReadonlySet<ThemeSurface> = new Set<ThemeSurface>(["main"]);

/// Engine capabilities this build advertises. A theme's
/// `required_engine_capabilities` must be a subset of this. ARC 1 advertises
/// NONE — attract-mode / multi-monitor / CRT-chrome are deferred platform
/// features (D20), so the only valid `required_engine_capabilities` value is
/// `[]`. When a capability ships, add its id here.
export const ENGINE_CAPABILITIES: ReadonlySet<string> = new Set<string>([]);

/// Disqualifying (error) vs. non-fatal (warning) issue codes. An ERROR excludes
/// the theme from the valid set → it can't be the active theme (fallback to the
/// default). A WARNING loads the theme but is surfaced (dev console / toast).
export type ThemeIssueCode =
  // --- errors ---
  | "MISSING_FIELD" // a required manifest field is absent or empty
  | "UNSUPPORTED_SCHEMA_VERSION" // schema_version ∉ SUPPORTED_SCHEMA_VERSIONS
  | "EMPTY_SURFACES" // surfaces must list at least one surface
  | "UNKNOWN_SURFACE" // a declared surface ∉ HONORED_SURFACES
  | "UNADVERTISED_CAPABILITY" // a required capability ∉ ENGINE_CAPABILITIES
  | "UNKNOWN_TOKEN_KEY" // a tokens key ∉ ThemeTokens (TOKEN_VAR)
  | "EMPTY_TOKEN_VALUE" // a tokens value is empty / blank
  | "UNKNOWN_SYSTEM_ID" // a perSystemTokens key ∉ SystemId (SYSTEM_PALETTES)
  | "UNKNOWN_PALETTE_KEY" // a perSystemTokens sub-key ∉ SystemPalette (PALETTE_VAR)
  | "EMPTY_PALETTE_VALUE" // a perSystemTokens value is empty / blank
  | "SETTING_KEY_INVALID" // a settings_schema control key is missing / empty / duplicated
  | "SETTING_CONTROL_INVALID" // a settings_schema control is malformed (type / label / default / range / options)
  | "INVALID_VIEWS" // views (or a view entry / per_system) is the wrong shape
  | "UNKNOWN_VIEW_TYPE" // a views key ∉ VIEW_TYPES
  | "INVALID_VIEW_LAYOUT" // a view layout (or per_system value) ∉ LAYOUT_PRIMITIVES
  // --- warnings ---
  | "INVALID_ID" // id is not lowercase / directory-safe
  | "DEFAULT_ROUTE_NOT_IN_ROUTES" // default_route ∉ routes
  | "UNKNOWN_GLYPH_SET" // glyph_set ∉ GLYPH_SETS (hints fall back to default)
  | "INVALID_PER_SYSTEM_UI"; // per_system_ui not { tiles?, sfx? } booleans (→ OFF)

export type ThemeIssue = {
  code: ThemeIssueCode;
  /** Manifest field or token key the issue concerns, when applicable. */
  field?: string;
  /** Human-readable explanation (dev console / surfaced text). */
  message: string;
};

export type ThemeValidation = {
  /** The theme's declared id (or "<unknown>" if even that is missing). */
  themeId: string;
  /** True when `errors` is empty — the theme may be activated. */
  ok: boolean;
  errors: ThemeIssue[];
  warnings: ThemeIssue[];
};

/// Required string manifest fields that must be present + non-empty.
const REQUIRED_STRING_FIELDS = [
  "id",
  "name",
  "version",
  "oa_version",
  "entry",
  "entry_export",
  "default_route",
  "reserves_corner",
] as const;

const ID_PATTERN = /^[a-z0-9][a-z0-9_-]*$/;

function isNonEmptyString(v: unknown): v is string {
  return typeof v === "string" && v.trim().length > 0;
}

/// Validate a theme package against THEME_CONTRACT.md §6. Pure; never throws.
export function validateTheme(pkg: ThemePackage): ThemeValidation {
  const errors: ThemeIssue[] = [];
  const warnings: ThemeIssue[] = [];
  const m = pkg.manifest;
  const themeId = isNonEmptyString(m?.id) ? m.id : "<unknown>";

  // --- required string fields present + non-empty ---
  for (const field of REQUIRED_STRING_FIELDS) {
    if (!isNonEmptyString(m?.[field])) {
      errors.push({
        code: "MISSING_FIELD",
        field,
        message: `manifest.${field} is required and must be a non-empty string`,
      });
    }
  }

  // --- schema_version supported ---
  if (typeof m?.schema_version !== "number") {
    errors.push({
      code: "MISSING_FIELD",
      field: "schema_version",
      message: "manifest.schema_version is required and must be a number",
    });
  } else if (!SUPPORTED_SCHEMA_VERSIONS.has(m.schema_version)) {
    const tooNew = m.schema_version > MAX_SCHEMA_VERSION;
    errors.push({
      code: "UNSUPPORTED_SCHEMA_VERSION",
      field: "schema_version",
      message: tooNew
        ? `manifest.schema_version ${m.schema_version} targets a newer manifest schema than this OA build supports (up to ${MAX_SCHEMA_VERSION}) — update OA`
        : `manifest.schema_version ${m.schema_version} is not a supported manifest schema (supported: ${[...SUPPORTED_SCHEMA_VERSIONS].join(", ")})`,
    });
  }

  // --- surfaces: at least one, all honored ---
  if (!Array.isArray(m?.surfaces) || m.surfaces.length === 0) {
    errors.push({
      code: "EMPTY_SURFACES",
      field: "surfaces",
      message: "manifest.surfaces must list at least one surface (ARC 1: [\"main\"])",
    });
  } else {
    for (const surface of m.surfaces) {
      if (!HONORED_SURFACES.has(surface)) {
        errors.push({
          code: "UNKNOWN_SURFACE",
          field: "surfaces",
          message: `surface "${surface}" is not honored by this OA build (honored: ${[...HONORED_SURFACES].join(", ")})`,
        });
      }
    }
  }

  // --- required_engine_capabilities ⊆ advertised ---
  if (!Array.isArray(m?.required_engine_capabilities)) {
    errors.push({
      code: "MISSING_FIELD",
      field: "required_engine_capabilities",
      message: "manifest.required_engine_capabilities is required (use [] for a theme that runs anywhere)",
    });
  } else {
    for (const cap of m.required_engine_capabilities) {
      if (!ENGINE_CAPABILITIES.has(cap)) {
        errors.push({
          code: "UNADVERTISED_CAPABILITY",
          field: "required_engine_capabilities",
          message: `required engine capability "${cap}" is not advertised by this OA build${
            ENGINE_CAPABILITIES.size === 0 ? " (ARC 1 advertises none)" : ""
          }`,
        });
      }
    }
  }

  // --- routes / default_route coherence (warnings) ---
  if (!isNonEmptyString(m?.id)) {
    // id missing is already an error above; skip the id-shape warning.
  } else if (!ID_PATTERN.test(m.id)) {
    warnings.push({
      code: "INVALID_ID",
      field: "id",
      message: `manifest.id "${m.id}" should be lowercase + directory-safe (a-z, 0-9, "-", "_")`,
    });
  }
  if (
    isNonEmptyString(m?.default_route) &&
    Array.isArray(m?.routes) &&
    m.routes.length > 0 &&
    !m.routes.includes(m.default_route)
  ) {
    warnings.push({
      code: "DEFAULT_ROUTE_NOT_IN_ROUTES",
      field: "default_route",
      message: `manifest.default_route "${m.default_route}" is not listed in manifest.routes`,
    });
  }

  // --- glyph_set known (warning — cosmetic; hints fall back to the default) ---
  if (isNonEmptyString(m?.glyph_set) && !(m.glyph_set in GLYPH_SETS)) {
    warnings.push({
      code: "UNKNOWN_GLYPH_SET",
      field: "glyph_set",
      message: `manifest.glyph_set "${m.glyph_set}" is not a known glyph set (${Object.keys(GLYPH_SETS).join(", ")}); the HintBar falls back to the default`,
    });
  }

  // --- per_system_ui (ARC 2 L1, D33/D34): optional { tiles?, sfx? } booleans.
  //     Warning (not error) + falls back to OFF on a malformed value — a theme
  //     shouldn't be disqualified over a consumption flag; the safe default is a
  //     uniform grid. ---
  if (m?.per_system_ui != null) {
    const psu: unknown = m.per_system_ui;
    const ok =
      typeof psu === "object" &&
      !Array.isArray(psu) &&
      ["tiles", "sfx"].every((k) => {
        const v = (psu as Record<string, unknown>)[k];
        return v === undefined || typeof v === "boolean";
      });
    if (!ok) {
      warnings.push({
        code: "INVALID_PER_SYSTEM_UI",
        field: "per_system_ui",
        message:
          "manifest.per_system_ui must be an object of optional booleans { tiles?, sfx? }; falling back to OFF",
      });
    }
  }

  // --- settings_schema (Settings IA Slice 3): unique non-empty keys +
  //     type-correct defaults / ranges / options. A malformed control is a
  //     disqualifying ERROR — a broken Appearance panel is worse than none. ---
  if (m?.settings_schema != null) {
    if (!Array.isArray(m.settings_schema)) {
      errors.push({
        code: "SETTING_CONTROL_INVALID",
        field: "settings_schema",
        message: "manifest.settings_schema must be an array of controls",
      });
    } else {
      const seenKeys = new Set<string>();
      m.settings_schema.forEach((c, i) => {
        const at = `settings_schema[${i}]`;
        if (!isNonEmptyString(c?.key)) {
          errors.push({
            code: "SETTING_KEY_INVALID",
            field: at,
            message: `${at}.key is required and must be a non-empty string`,
          });
        } else if (seenKeys.has(c.key)) {
          errors.push({
            code: "SETTING_KEY_INVALID",
            field: at,
            message: `${at}.key "${c.key}" is duplicated — control keys must be unique within a theme`,
          });
        } else {
          seenKeys.add(c.key);
        }
        if (!isNonEmptyString(c?.label)) {
          errors.push({
            code: "SETTING_CONTROL_INVALID",
            field: at,
            message: `${at}.label is required and must be a non-empty string`,
          });
        }
        switch (c.type) {
          case "toggle":
            if (typeof c.default !== "boolean") {
              errors.push({
                code: "SETTING_CONTROL_INVALID",
                field: at,
                message: `${at}.default must be a boolean for a toggle`,
              });
            }
            break;
          case "slider":
            if (
              typeof c.min !== "number" ||
              typeof c.max !== "number" ||
              typeof c.step !== "number" ||
              c.min >= c.max ||
              c.step <= 0
            ) {
              errors.push({
                code: "SETTING_CONTROL_INVALID",
                field: at,
                message: `${at} slider needs numeric min < max and step > 0`,
              });
            } else if (typeof c.default !== "number" || c.default < c.min || c.default > c.max) {
              errors.push({
                code: "SETTING_CONTROL_INVALID",
                field: at,
                message: `${at}.default must be a number within [${c.min}, ${c.max}]`,
              });
            }
            break;
          case "select":
            if (!Array.isArray(c.options) || c.options.length === 0) {
              errors.push({
                code: "SETTING_CONTROL_INVALID",
                field: at,
                message: `${at} select needs a non-empty options array`,
              });
            } else if (!c.options.some((o: { value?: unknown }) => o?.value === c.default)) {
              errors.push({
                code: "SETTING_CONTROL_INVALID",
                field: at,
                message: `${at}.default "${c.default}" must match one of the option values`,
              });
            }
            break;
          default:
            errors.push({
              code: "SETTING_CONTROL_INVALID",
              field: at,
              message: `${at}.type must be one of: toggle | slider | select`,
            });
        }
      });
    }
  }

  // --- tokens: keys ∈ ThemeTokens, values non-empty ---
  if (pkg.tokens != null) {
    for (const key of Object.keys(pkg.tokens)) {
      if (!(key in TOKEN_VAR)) {
        errors.push({
          code: "UNKNOWN_TOKEN_KEY",
          field: `tokens.${key}`,
          message: `tokens.${key} is not a recognized design token (see ThemeTokens / TOKEN_VAR)`,
        });
        continue;
      }
      const value = pkg.tokens[key as keyof ThemeTokens];
      // A present key with an empty/blank value is a mistake — it would inject
      // an empty CSS var rather than fall through to :root (omit the key for
      // inheritance). `undefined`/`null` are fine: themeTokensToCssVars drops
      // them, so a Partial that names a key but leaves it unset still inherits.
      if (value != null && !isNonEmptyString(value)) {
        errors.push({
          code: "EMPTY_TOKEN_VALUE",
          field: `tokens.${key}`,
          message: `tokens.${key} must be a non-empty CSS value string (omit the key to inherit the :root default)`,
        });
      }
    }
  }

  // --- perSystemTokens (S5.2): system ids ∈ SystemId, sub-keys ∈ SystemPalette,
  //     values non-empty. Same data-half rule as tokens: a theme can only set
  //     known palette vars on known systems, so even a hostile value injects a
  //     known, mount-scoped CSS var rather than escaping the theme. ---
  if (pkg.perSystemTokens != null) {
    for (const systemId of Object.keys(pkg.perSystemTokens)) {
      if (!(systemId in SYSTEM_PALETTES)) {
        errors.push({
          code: "UNKNOWN_SYSTEM_ID",
          field: `perSystemTokens.${systemId}`,
          message: `perSystemTokens.${systemId} is not a known system id (see SystemId / SYSTEM_PALETTES)`,
        });
        continue;
      }
      const pal = pkg.perSystemTokens[systemId as keyof typeof pkg.perSystemTokens];
      if (pal == null) continue;
      for (const paletteKey of Object.keys(pal)) {
        if (!(paletteKey in PALETTE_VAR)) {
          errors.push({
            code: "UNKNOWN_PALETTE_KEY",
            field: `perSystemTokens.${systemId}.${paletteKey}`,
            message: `perSystemTokens.${systemId}.${paletteKey} is not a palette key (accent / soft / glow)`,
          });
          continue;
        }
        const value = pal[paletteKey as keyof SystemPalette];
        if (value != null && !isNonEmptyString(value)) {
          errors.push({
            code: "EMPTY_PALETTE_VALUE",
            field: `perSystemTokens.${systemId}.${paletteKey}`,
            message: `perSystemTokens.${systemId}.${paletteKey} must be a non-empty CSS value string (omit the key to inherit the baseline)`,
          });
        }
      }
    }
  }

  // --- perSystemUiConfigs (L2b / D34): per-system experiential overrides keyed
  //     by SystemId. Validate the keys are known systems; the field VALUES are
  //     enum-typed Partial<SystemUIConfig> (deep value validation waits for
  //     on-disk themes). Mirrors the perSystemTokens key check. ---
  if (pkg.perSystemUiConfigs != null) {
    for (const systemId of Object.keys(pkg.perSystemUiConfigs)) {
      if (!(systemId in SYSTEM_PALETTES)) {
        errors.push({
          code: "UNKNOWN_SYSTEM_ID",
          field: `perSystemUiConfigs.${systemId}`,
          message: `perSystemUiConfigs.${systemId} is not a known system id (see SystemId / SYSTEM_PALETTES)`,
        });
      }
    }
  }

  // --- views (ARC 2 L2 / D32): per-view layout map. Each key ∈ VIEW_TYPES, each
  //     `layout` ∈ LAYOUT_PRIMITIVES, optional `per_system` overrides keyed by
  //     SystemId → LayoutPrimitive. Malformed = ERROR (a broken layout map is
  //     worse than none, like settings_schema). Contract only in L2a — the L3
  //     resolver is the first consumer. ---
  if (m?.views != null) {
    const views: unknown = m.views;
    if (typeof views !== "object" || Array.isArray(views)) {
      errors.push({
        code: "INVALID_VIEWS",
        field: "views",
        message: "manifest.views must be an object mapping view types to { layout, per_system? }",
      });
    } else {
      const layoutSet = new Set<string>(LAYOUT_PRIMITIVES);
      const viewSet = new Set<string>(VIEW_TYPES);
      for (const view of Object.keys(views as Record<string, unknown>)) {
        if (!viewSet.has(view)) {
          errors.push({
            code: "UNKNOWN_VIEW_TYPE",
            field: `views.${view}`,
            message: `views.${view} is not a known view type (${VIEW_TYPES.join(", ")})`,
          });
          continue;
        }
        const cfg = (views as Record<string, unknown>)[view];
        if (typeof cfg !== "object" || cfg === null || Array.isArray(cfg)) {
          errors.push({
            code: "INVALID_VIEWS",
            field: `views.${view}`,
            message: `views.${view} must be an object { layout, per_system? }`,
          });
          continue;
        }
        const layout = (cfg as Record<string, unknown>).layout;
        if (typeof layout !== "string" || !layoutSet.has(layout)) {
          errors.push({
            code: "INVALID_VIEW_LAYOUT",
            field: `views.${view}.layout`,
            message: `views.${view}.layout must be one of ${LAYOUT_PRIMITIVES.join(", ")}`,
          });
        }
        const perSystem = (cfg as Record<string, unknown>).per_system;
        if (perSystem != null) {
          if (typeof perSystem !== "object" || Array.isArray(perSystem)) {
            errors.push({
              code: "INVALID_VIEWS",
              field: `views.${view}.per_system`,
              message: `views.${view}.per_system must be an object of SystemId → layout primitive`,
            });
          } else {
            for (const sysId of Object.keys(perSystem as Record<string, unknown>)) {
              if (!(sysId in SYSTEM_PALETTES)) {
                errors.push({
                  code: "UNKNOWN_SYSTEM_ID",
                  field: `views.${view}.per_system.${sysId}`,
                  message: `views.${view}.per_system.${sysId} is not a known system id (see SystemId / SYSTEM_PALETTES)`,
                });
                continue;
              }
              const pl = (perSystem as Record<string, unknown>)[sysId];
              if (typeof pl !== "string" || !layoutSet.has(pl)) {
                errors.push({
                  code: "INVALID_VIEW_LAYOUT",
                  field: `views.${view}.per_system.${sysId}`,
                  message: `views.${view}.per_system.${sysId} must be one of ${LAYOUT_PRIMITIVES.join(", ")}`,
                });
              }
            }
          }
        }
      }
    }
  }

  return { themeId, ok: errors.length === 0, errors, warnings };
}

/// Format a validation result as a single multi-line string for the dev
/// console / a toast body. Returns "" when there's nothing to report.
export function formatThemeValidation(v: ThemeValidation): string {
  if (v.errors.length === 0 && v.warnings.length === 0) return "";
  const lines: string[] = [];
  for (const e of v.errors) lines.push(`  ✗ [${e.code}] ${e.message}`);
  for (const w of v.warnings) lines.push(`  ⚠ [${w.code}] ${w.message}`);
  return `theme "${v.themeId}":\n${lines.join("\n")}`;
}
