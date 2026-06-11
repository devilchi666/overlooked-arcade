// Per-system palette — the typed single source of truth.
//
// Theming Substrate ARC 1 Phase 3 S5.2 (the palette substrate). Replaces the
// hand-authored `themes/systems.css` (retired): the 46 per-system OKLCH accent
// palettes now live as a typed, greppable, validator-readable map, and the
// global `[data-system]` baseline CSS is DERIVED from it at boot
// (`ensureSystemPaletteBaseline`). Per-system palette is frontend-only data
// (no Rust reader), so a typed TS map is its right home — vs `config/*.json` +
// a cross-language build step, which would add a generated-file that can drift.
//
// WHY A SINGLE MAP: a future Theme Studio (ARC 3) round-trips this shape
// directly; the S4 validator checks a theme's per-system overrides against it;
// and the per-theme override seam (`perSystemTokens`, below) composes over it
// (D19's optional sub-cascade — a theme MAY recolor a system, scoped to its own
// mount so engine territory keeps the baseline; D2).
//
// FULL HUE RATIONALE: the detailed per-system "why this hue / how it separates
// from its neighbors in a mixed library" notes lived as block comments in the
// old `themes/systems.css`; they're preserved in git history (the file deleted
// in the S5.2 commit). Each entry below keeps a one-line identity note; the
// `glow` var is always the accent at 0.35 alpha (the invariant across all 46).

import type { SystemId } from "./registry";

/// A system's three accent CSS vars. Maps 1:1 to PALETTE_VAR. Values are CSS
/// value strings (baseline = `oklch(...)`; a theme override may use any CSS
/// color). The per-theme override seam takes a `Partial<SystemPalette>`.
export type SystemPalette = {
  /** --color-system-accent — the system's primary identity color. */
  accent: string;
  /** --color-system-accent-soft — pale variant for text-on-color / highlights. */
  soft: string;
  /** --color-system-glow — accent at low alpha, for hover halos. */
  glow: string;
};

/// Maps each palette key to its CSS custom property — the single tie between
/// the typed shape and the stylesheet vars (peer of TOKEN_VAR in tokens.ts).
export const PALETTE_VAR: Record<keyof SystemPalette, string> = {
  accent: "--color-system-accent",
  soft: "--color-system-accent-soft",
  glow: "--color-system-glow",
};

/// Build a baseline palette from its accent + soft. `glow` is derived as the
/// accent at 0.35 alpha — the invariant every baseline followed in systems.css
/// (an override theme can still set `glow` explicitly via `perSystemTokens`).
function p(accent: string, soft: string): SystemPalette {
  return { accent, soft, glow: accent.replace(/\)\s*$/, " / 0.35)") };
}

/// The 46 per-system baseline palettes (one-line identity notes; full rationale
/// in git history of the retired `themes/systems.css`). `Record<SystemId, …>`
/// forces a palette for every system — adding a system makes this fail to
/// typecheck until its palette is added (the parity guarantee systems.css's
/// comment used to ask for by hand).
export const SYSTEM_PALETTES: Record<SystemId, SystemPalette> = {
  tg16: p("oklch(0.74 0.18 55)", "oklch(0.94 0.04 80)"), // HuCard warm orange (55°)
  "pce-cd": p("oklch(0.72 0.14 220)", "oklch(0.93 0.04 220)"), // Duo silver-cyan (220°)
  lynx: p("oklch(0.65 0.22 290)", "oklch(0.93 0.05 290)"), // Epyx violet (290°)
  nes: p("oklch(0.62 0.22 28)", "oklch(0.92 0.05 28)"), // Big Box crimson (28°)
  snes: p("oklch(0.62 0.18 270)", "oklch(0.93 0.05 270)"), // mid violet (270°)
  mame: p("oklch(0.64 0.24 12)", "oklch(0.93 0.06 12)"), // arcade neon scarlet (12°)
  atari7800: p("oklch(0.78 0.18 80)", "oklch(0.94 0.05 80)"), // bright gold (80°)
  genesis: p("oklch(0.62 0.22 245)", "oklch(0.93 0.05 245)"), // electric cobalt (245°)
  segacd: p("oklch(0.55 0.20 235)", "oklch(0.92 0.05 235)"), // sapphire (235°)
  sega32x: p("oklch(0.68 0.22 42)", "oklch(0.94 0.05 42)"), // neon orange (42°)
  sega32xcd: p("oklch(0.60 0.22 42)", "oklch(0.93 0.06 42)"), // deeper 32X orange (42°)
  saturn: p("oklch(0.45 0.18 275)", "oklch(0.91 0.06 275)"), // deepest night purple (275°)
  stv: p("oklch(0.55 0.18 220)", "oklch(0.93 0.05 220)"), // Saturn-arcade cyan-blue (220°)
  psx: p("oklch(0.65 0.16 180)", "oklch(0.93 0.05 180)"), // PS1 cool teal-cyan (180°)
  neogeo: p("oklch(0.50 0.27 18)", "oklch(0.91 0.07 18)"), // SNK arcade cherry (18°)
  neocd: p("oklch(0.55 0.18 50)", "oklch(0.93 0.05 50)"), // Neo Geo CD gold-bronze (50°)
  ngp: p("oklch(0.80 0.12 105)", "oklch(0.95 0.04 105)"), // NGPC pearl yellow-green (105°)
  jaguar: p("oklch(0.65 0.22 65)", "oklch(0.93 0.06 65)"), // 1993 saturated gold (65°)
  jagcd: p("oklch(0.58 0.20 75)", "oklch(0.93 0.06 75)"), // deeper Jaguar amber (75°)
  "3do": p("oklch(0.55 0.22 297)", "oklch(0.92 0.06 297)"), // 3DO purple-magenta (297°)
  pcfx: p("oklch(0.62 0.24 320)", "oklch(0.93 0.06 320)"), // anime pink-magenta (320°)
  n64: p("oklch(0.55 0.22 268)", "oklch(0.92 0.06 268)"), // Atomic Purple (268°)
  gamecube: p("oklch(0.48 0.22 280)", "oklch(0.92 0.06 280)"), // GC Indigo (280°)
  dreamcast: p("oklch(0.55 0.27 32)", "oklch(0.92 0.07 32)"), // DC orange swirl (32°)
  psp: p("oklch(0.65 0.18 200)", "oklch(0.93 0.05 200)"), // Sony cool cyan (200°)
  ps2: p("oklch(0.45 0.22 215)", "oklch(0.92 0.06 215)"), // deep PS cobalt-cyan (215°)
  nds: p("oklch(0.78 0.14 95)", "oklch(0.94 0.05 95)"), // pearl yellow-green (95°)
  sms: p("oklch(0.65 0.22 340)", "oklch(0.93 0.06 340)"), // Western Big Box neon magenta (340°)
  gamegear: p("oklch(0.72 0.18 130)", "oklch(0.93 0.05 130)"), // GG yellow-green (130°)
  gb: p("oklch(0.62 0.13 145)", "oklch(0.93 0.04 145)"), // DMG muted pea-green (145°)
  gbc: p("oklch(0.68 0.20 320)", "oklch(0.94 0.05 320)"), // Atomic-Purple-era berry (320°)
  gba: p("oklch(0.55 0.20 285)", "oklch(0.93 0.06 285)"), // clear-indigo (285°)
  "2600": p("oklch(0.60 0.07 60)", "oklch(0.93 0.03 60)"), // wood-veneer brown (60°, low C)
  "5200": p("oklch(0.62 0.20 18)", "oklch(0.93 0.06 18)"), // premium red (18°)
  coleco: p("oklch(0.72 0.16 195)", "oklch(0.93 0.05 195)"), // bright cyan (195°)
  intv: p("oklch(0.50 0.17 260)", "oklch(0.92 0.05 260)"), // Mattel deep navy (260°)
  o2: p("oklch(0.62 0.18 325)", "oklch(0.93 0.06 325)"), // Odyssey² rose-fuchsia (325°)
  channelf: p("oklch(0.45 0.06 25)", "oklch(0.92 0.03 25)"), // red-cedar wood-grain (25°, low C)
  vectrex: p("oklch(0.80 0.16 165)", "oklch(0.94 0.05 165)"), // bright phosphor green (165°)
  virtualboy: p("oklch(0.55 0.26 7)", "oklch(0.93 0.07 7)"), // VB neon LED red (7°)
  wonderswan: p("oklch(0.70 0.14 305)", "oklch(0.93 0.05 305)"), // pearl lavender (305°)
  pokemini: p("oklch(0.85 0.16 95)", "oklch(0.95 0.06 95)"), // sunny yellow, lightest tile (95°)
  msx: p("oklch(0.55 0.18 250)", "oklch(0.92 0.05 250)"), // ASCII/MS royal blue (250°)
  msx2: p("oklch(0.65 0.18 250)", "oklch(0.93 0.05 250)"), // brighter MSX2 sibling (250°)
  scummvm: p("oklch(0.62 0.16 195)", "oklch(0.93 0.05 195)"), // adventure teal-cyan (195°)
  dosbox: p("oklch(0.65 0.18 55)", "oklch(0.94 0.05 55)"), // DOS amber phosphor (55°)
};

/// Render a palette (or partial) into a CSS declaration body (no braces),
/// e.g. `--color-system-accent:oklch(...);--color-system-accent-soft:...`.
/// Omitted keys are skipped (partial overrides inherit the lower tier).
function paletteDecls(pal: Partial<SystemPalette>): string {
  let out = "";
  for (const key of Object.keys(PALETTE_VAR) as (keyof SystemPalette)[]) {
    const value = pal[key];
    if (value != null && value !== "") out += `${PALETTE_VAR[key]}:${value};`;
  }
  return out;
}

/// The global per-system baseline CSS: one `[data-system="<id>"]{…}` rule per
/// system. Consumed by BOTH engine territory (per-system settings, info panels)
/// and theme territory — so it's injected GLOBALLY (the same scope the old
/// systems.css had), not into a theme mount.
export function systemPaletteBaselineCss(): string {
  let css = "";
  for (const id of Object.keys(SYSTEM_PALETTES) as SystemId[]) {
    css += `[data-system="${id}"]{${paletteDecls(SYSTEM_PALETTES[id])}}`;
  }
  return css;
}

/// The DOM id of the injected baseline `<style>` — stable so injection is
/// idempotent (HMR / double-import won't duplicate it).
const BASELINE_STYLE_ID = "oa-system-palettes-baseline";

/// Inject the global per-system baseline CSS into `document.head` once. Called
/// at app entry (index.tsx) BEFORE first render so `[data-system]` accents are
/// in the DOM before any component mounts (no flash) — the runtime equivalent
/// of the retired `@import "./themes/systems.css"`. Idempotent + DOM-guarded so
/// it's a no-op in a non-DOM context (it's never called from tests).
export function ensureSystemPaletteBaseline(): void {
  if (typeof document === "undefined") return;
  if (document.getElementById(BASELINE_STYLE_ID)) return;
  const style = document.createElement("style");
  style.id = BASELINE_STYLE_ID;
  style.textContent = systemPaletteBaselineCss();
  document.head.appendChild(style);
}

/// Per-theme per-system palette overrides — D19's optional sub-cascade. A theme
/// declares `perSystemTokens` on its ThemePackage; App.tsx injects the result
/// of `perSystemOverrideCss` as a `<style>` SCOPED to the theme-mount wrapper,
/// so the override beats the baseline INSIDE the theme but engine territory (a
/// sibling of the mount) still reads the baseline (the D2 guarantee, same
/// structural sibling-scope the S3 token layer uses). Omit a system to inherit;
/// omit a key within a system to inherit that var.
export type PerSystemTokens = Partial<Record<SystemId, Partial<SystemPalette>>>;

/// Build the scoped override CSS for a theme's `perSystemTokens`. Each rule is
/// `<scope> [data-system="<id>"]{…}` — higher specificity (scope class +
/// attribute) than the global `[data-system]` baseline, so it wins inside the
/// scope only. `scopeSelector` is the theme-mount selector (e.g.
/// `.oa-theme-mount`). Returns "" when there are no overrides.
export function perSystemOverrideCss(
  scopeSelector: string,
  overrides: PerSystemTokens,
): string {
  let css = "";
  for (const id of Object.keys(overrides) as SystemId[]) {
    const pal = overrides[id];
    if (!pal) continue;
    const decls = paletteDecls(pal);
    if (decls) css += `${scopeSelector} [data-system="${id}"]{${decls}}`;
  }
  return css;
}
