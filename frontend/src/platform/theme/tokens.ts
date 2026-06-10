// Design-token contract — the documented set a theme MAY override.
//
// Theming Substrate ARC 1 Phase 3 S3 (the token layer). This is the typed,
// greppable, validator-checkable surface of OA's existing CSS-variable token
// system (defined in frontend/src/index.css `@theme` + `:root`). S3 does NOT
// invent tokens — it FORMALIZES which ones are the theme-overridable contract
// and gives them a typed shape; the values still live as CSS custom properties.
//
// HOW OVERRIDES APPLY (DECISIONS D23): a theme declares `tokens?:
// Partial<ThemeTokens>` on its ThemePackage. App.tsx injects them as CSS
// custom properties SCOPED to the theme-mount wrapper (the S2 `isolate` div).
// Because the engine surface (Settings/Manager) is a SIBLING of that wrapper —
// not a descendant — scoped theme tokens never reach engine territory: the
// engine always reads the `:root` defaults. That sibling-scope IS the D2
// guarantee (a theme can't restyle Settings). The contract rule that makes it
// airtight: a theme styles via its own scoped classes + this token object, and
// NEVER writes a global `:root` / `<style>` token override. (The S4 validator
// will check that rule; for now it's documented in THEME_CONTRACT.md.)
//
// MOTION IS RESERVED. The `--motion-*` / `--ease-*` duration+easing tokens
// exist, but ARC 1 does NOT let themes drive animation — the cinematic/motion
// axis (transitions, video, attract, shader chrome) is ARC 2-3 (the BigBox-
// style capability the substrate builds toward). Motion is documented in
// THEME_CONTRACT.md as a reserved category so it slots in without a contract
// break; it is intentionally NOT part of ThemeTokens yet.

/// The theme-overridable token set. Each key maps to one CSS custom property
/// (see TOKEN_VAR). All values are CSS value strings (e.g. an `oklch(...)`
/// color, a font stack, a length). A theme provides any subset; omitted keys
/// fall through to the `:root` defaults from index.css.
export type ThemeTokens = {
  // --- palette ---
  /** App surface base. CSS: --color-oa-bg */
  bg: string;
  /** App surface deep (page background). CSS: --color-oa-bg-deep */
  bgDeep: string;
  /** Primary text. CSS: --color-oa-ink */
  ink: string;
  /** Dimmed/secondary text. CSS: --color-oa-ink-dim */
  inkDim: string;
  /** Base accent (the per-system [data-system] cascade still overrides this
   *  for elements that carry a data-system; this is the fallback a theme uses
   *  where no system identity applies). CSS: --color-system-accent */
  accent: string;
  /** Soft accent (highlights). CSS: --color-system-accent-soft */
  accentSoft: string;
  /** Accent glow (translucent). CSS: --color-system-glow */
  accentGlow: string;
  /** Focus-visible ring color the nav primitives consume. Defaults to the
   *  accent; a theme may set it independently. CSS: --oa-focus-ring */
  focusRing: string;

  // --- typography ---
  /** Display font stack (keep CJK fallbacks if you replace Inter — see
   *  index.css). CSS: --font-display */
  fontDisplay: string;

  // --- geometry ---
  /** Tile / card corner radius. CSS: --layout-tile-radius */
  tileRadius: string;
  /** Grid gap. CSS: --layout-grid-gap */
  gridGap: string;
  /** Section spacing rhythm. CSS: --layout-section-spacing */
  sectionSpacing: string;
};

/// Maps each token key to its CSS custom property name. The single source of
/// truth tying the typed contract to the stylesheet vars.
export const TOKEN_VAR: Record<keyof ThemeTokens, string> = {
  bg: "--color-oa-bg",
  bgDeep: "--color-oa-bg-deep",
  ink: "--color-oa-ink",
  inkDim: "--color-oa-ink-dim",
  accent: "--color-system-accent",
  accentSoft: "--color-system-accent-soft",
  accentGlow: "--color-system-glow",
  focusRing: "--oa-focus-ring",
  fontDisplay: "--font-display",
  tileRadius: "--layout-tile-radius",
  gridGap: "--layout-grid-gap",
  sectionSpacing: "--layout-section-spacing",
};

/// Convert a theme's typed token overrides into a CSS-custom-property style
/// map suitable for a Solid `style={...}` prop (scoped injection on the theme
/// mount). Omits null/undefined values so partial token sets fall through to
/// the `:root` defaults. Returns an empty object for `undefined` (the
/// no-overrides default theme — Retroverse).
export function themeTokensToCssVars(
  tokens: Partial<ThemeTokens> | undefined,
): Record<string, string> {
  const out: Record<string, string> = {};
  if (!tokens) return out;
  for (const key of Object.keys(tokens) as (keyof ThemeTokens)[]) {
    const value = tokens[key];
    if (value != null) out[TOKEN_VAR[key]] = value;
  }
  return out;
}
