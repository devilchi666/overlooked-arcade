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
// MOTION (Theming ARC 3 Thrust M, M1). The `--motion-*` / `--ease-*`
// duration+easing tokens were RESERVED through ARC 1-2 (documented, not
// authorable). M1 ACTIVATES them as an authorable contract — see
// `ThemeMotionTokens` / `MOTION_TOKEN_VAR` below — so a theme can tune the
// global motion feel (the duration/easing every `var(--motion-*)`-reading CSS
// consumes: nav primitives, focus ring, the declarative view transition's
// defaults). They are a SEPARATE group from `ThemeTokens`, not folded in,
// because their injection MUST re-assert the `prefers-reduced-motion` floor
// inside the theme mount (the scoped override would otherwise out-specify the
// global `:root { --motion-*: 0ms }` a11y reset for theme-internal motion) —
// see `themeMotionTokensCss`. The view-transition PRESET selection is a
// manifest field (`manifest.motion`), not a token — see manifest.ts.

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

// ===========================================================================
// MOTION TOKENS (Theming ARC 3 Thrust M, M1)
// ===========================================================================

/// The motion half of the token contract — durations + easings a theme MAY
/// override to tune the global motion feel. Each key maps to one of the
/// `--motion-*` / `--ease-*` CSS custom properties index.css already defines
/// (and the `prefers-reduced-motion` reset already collapses). All values are
/// CSS value strings: durations (`"250ms"`, `"0.4s"`) for the `--motion-*`
/// keys, easings (`"ease-out"`, `"cubic-bezier(...)"`) for the `--ease-*` keys.
/// A theme provides any subset; omitted keys fall through to the `:root`
/// defaults. Authorable contract, NOT folded into `ThemeTokens` — see the
/// module header for why (the reduced-motion floor re-assertion).
export type ThemeMotionTokens = {
  /** Micro-interaction duration (taps/toggles). CSS: --motion-instant */
  instant: string;
  /** Fast transition duration. CSS: --motion-fast */
  fast: string;
  /** Standard transition duration. CSS: --motion-medium */
  medium: string;
  /** Deliberate / large-surface duration. CSS: --motion-slow */
  slow: string;
  /** Decelerate easing (enter / settle). CSS: --ease-out */
  easeOut: string;
  /** Symmetric easing (move). CSS: --ease-in-out */
  easeInOut: string;
  /** Overshoot easing (snap / pop). CSS: --ease-snap */
  easeSnap: string;
};

/// Maps each motion token key to its CSS custom property. Parallel to
/// `TOKEN_VAR`; the single source of truth tying the typed motion contract to
/// the stylesheet vars.
export const MOTION_TOKEN_VAR: Record<keyof ThemeMotionTokens, string> = {
  instant: "--motion-instant",
  fast: "--motion-fast",
  medium: "--motion-medium",
  slow: "--motion-slow",
  easeOut: "--ease-out",
  easeInOut: "--ease-in-out",
  easeSnap: "--ease-snap",
};

/// The four duration vars — re-zeroed under `prefers-reduced-motion` so a
/// theme's scoped duration overrides cannot defeat the a11y floor inside its
/// own mount (see `themeMotionTokensCss`). Easings are excluded: an easing
/// curve causes no motion on its own.
const MOTION_DURATION_VARS: readonly string[] = [
  "--motion-instant",
  "--motion-fast",
  "--motion-medium",
  "--motion-slow",
];

/// Emit the CSS that injects a theme's motion-token overrides, SCOPED to the
/// theme mount (the `scope` selector, e.g. `.oa-theme-mount`), plus a
/// `prefers-reduced-motion` block that RE-ZEROES the duration vars inside that
/// same scope. Why a `<style>` string and not the inline-var map
/// `themeTokensToCssVars` uses: an inline (or plain class) scoped override
/// out-specifies the global `:root { --motion-*: 0ms }` reduced-motion reset
/// for the mount's descendants — so without re-asserting the floor here, a
/// theme could re-enable token-driven motion for users who asked for none.
/// Returns "" when there's nothing to inject (so a theme with no motion tokens
/// emits no style). Easings pass through unchanged under reduced motion (they
/// move nothing).
export function themeMotionTokensCss(
  scope: string,
  tokens: Partial<ThemeMotionTokens> | undefined,
): string {
  if (!tokens) return "";
  const decls: string[] = [];
  for (const key of Object.keys(tokens) as (keyof ThemeMotionTokens)[]) {
    const value = tokens[key];
    if (value != null && value.trim().length > 0) {
      decls.push(`${MOTION_TOKEN_VAR[key]}: ${value};`);
    }
  }
  if (decls.length === 0) return "";
  const reZero = MOTION_DURATION_VARS.map((v) => `${v}: 0ms;`).join(" ");
  return (
    `${scope} { ${decls.join(" ")} }\n` +
    `@media (prefers-reduced-motion: reduce) { ${scope} { ${reZero} } }`
  );
}
