// Per-system UI configuration — the platform CONTRACT (Theming ARC 2 L2b / D34).
//
// Per-system UI is a platform *capability*; the per-system *content* (which
// systems get which experiential config) is THEME content. So this file holds:
//   - the `SystemUIConfig` type + the `UI*` enums (the contract a theme fills),
//   - `BASELINE_UI` (the default every system falls back to),
//   - `systemSupportsTouch()` — the one FACTUAL per-system datum (does the
//     hardware have a touchscreen; gates the stylus/touch overlays regardless of
//     theme), and
//   - the active-theme bridge: App.tsx pushes `activeTheme().perSystemUiConfigs`
//     here; `uiConfigFor()` merges that over `BASELINE_UI`.
//
// The per-system EXPERIENTIAL values (the gb/nes/vectrex overrides) used to live
// in a global map here; D34 moved them to `themes/retroverse/systemUiConfigs.ts`
// (theme content). A theme that declares nothing → every system reads
// `BASELINE_UI`. Consumed (gated by the L1 opt-in) by `LibraryTile` /
// `VirtualLibraryGrid` (tileShape/interactionStyle) + `systemUiSound`
// (audioProfile). Per-system *layout* is the separate `views` contract (L2a).

import { createSignal } from "solid-js";
import type { SystemId } from "./registry";

/// Library grid shape. Stage 1: `grid` for every system; Stage 2 unlocks
/// the rest.
export type UILayout = "grid" | "carousel" | "list" | "wheel";

/// Cursor / focus movement behavior between tiles.
export type UINavigation = "free" | "snap" | "paged";

/// What's prominent on each library tile.
export type UIEmphasis = "boxart" | "title" | "screenshot";

/// Background shape behind the library.
export type UIBackground = "static" | "animated" | "shader";

/// Audio character class. "none" mutes all per-system SFX, "ambient"
/// is the universal soft-click baseline, "console" lets the system
/// express more characterful SFX (NES boops, Vectrex blips).
export type UIAudioProfile = "none" | "ambient" | "console";

/// Selection / focus feel.
export type UIInteractionStyle = "instant" | "delayed" | "physical";

/// Tile aspect override. "auto" defers to the existing
/// `systemThemes[id].tileAspect` so this field never has to fight the
/// existing visual-theme registry; concrete values force a specific
/// shape regardless of cover-art geometry.
export type UITileShape =
  | "auto"
  | "square"
  | "portrait-3:4"
  | "landscape-4:3"
  | "wide-16:9"
  | "circle";

/// Animation timing for nav + transitions. Maps to a duration
/// multiplier in the consumer. "instant" = no animation; "cinematic"
/// = ~500-1000 ms long-form transitions.
export type UITransitionTiming = "instant" | "fast" | "standard" | "slow" | "cinematic";

/// Button-label convention for the on-screen hint bar. "auto" defers
/// to a per-system-family resolver in the consumer (Nintendo systems
/// → nintendo-{handheld,console}; Sony → playstation; etc.). Pilots
/// can pin a specific convention.
export type UIButtonLabels =
  | "auto"
  | "xbox"
  | "playstation"
  | "nintendo-handheld"
  | "nintendo-console"
  | "saturn"
  | "vectrex-custom";

/// Per-system sound-effect bank. Each event maps to an audio file
/// resolved against `<exe_dir>/assets/system-ui/<system>/sounds/`.
/// Fields are individually optional — anything missing falls through
/// to the audioProfile's baseline. `bootOutro` is Stage 3 surface;
/// shipped systems don't have to populate it for Stage 1.
export type UISoundEffects = Partial<{
  navigate: string;
  select: string;
  back: string;
  launch: string;
  bootIntro: string;
  bootOutro: string;
}>;

export interface SystemUIConfig {
  layout: UILayout;
  navigation: UINavigation;
  emphasis: UIEmphasis;
  background: UIBackground;
  audioProfile: UIAudioProfile;
  interactionStyle: UIInteractionStyle;
  tileShape: UITileShape;
  transitionTiming: UITransitionTiming;
  buttonLabels: UIButtonLabels;

  /// Optional per-system background asset path (relative to
  /// `<exe_dir>/assets/system-ui/<system>/backgrounds/`). When unset,
  /// the `background` enum's default rendering applies — `static`
  /// falls back to the existing accent-color gradient driven by
  /// `systemThemes[id]`.
  backgroundAsset?: string;

  /// Optional per-system SFX bank. When unset, the `audioProfile`
  /// enum's baseline sounds apply.
  soundEffects?: UISoundEffects;
}

/// A theme's per-system experiential overrides (D34) — a partial config per
/// system, merged over `BASELINE_UI` by `uiConfigFor`. This is THEME content
/// (lives on `ThemePackage.perSystemUiConfigs`, peer of `perSystemTokens`); a
/// theme that ships none gets `BASELINE_UI` everywhere.
export type PerSystemUiConfigs = Partial<Record<SystemId, Partial<SystemUIConfig>>>;

/// Baseline config — every system falls back to this; a theme overrides
/// specific fields per system via `perSystemUiConfigs`. Resolves to the
/// universal soft-click SFX, a single accent-color static gradient background,
/// and the existing tile aspect from `systemThemes`.
export const BASELINE_UI: SystemUIConfig = {
  layout: "grid",
  navigation: "snap",
  emphasis: "boxart",
  background: "static",
  audioProfile: "ambient",
  interactionStyle: "instant",
  tileShape: "auto",
  transitionTiming: "fast",
  buttonLabels: "auto",
};

/// FACTUAL per-system data (D34): systems whose hardware uses a primary
/// pointer/touch input mode. Drives the StylusOverlay reticle, the
/// TouchHotspotOverlay per-game labels, and the QuickSettings "Show touch
/// hints" toggle — all theme-INDEPENDENT (NDS has a touchscreen under any
/// theme), so this stays platform, NOT theme content. Light-gun systems use
/// the cursor differently (AIM not TAP) and stay out.
const SYSTEM_TOUCH_INPUT: ReadonlySet<SystemId> = new Set<SystemId>(["nds"]);

/// Whether a system has primary touch/stylus input (factual hardware fact).
export function systemSupportsTouch(systemId: SystemId): boolean {
  return SYSTEM_TOUCH_INPUT.has(systemId);
}

/// Active-theme per-system experiential overrides, bridged from App.tsx
/// (`setThemeSystemUiConfigs(activeTheme()?.perSystemUiConfigs)`) — the same
/// pattern as the glyph-set / per-system-UI-opt-in bridges. Defaults to empty:
/// a theme that declares nothing → every system reads `BASELINE_UI`.
const [themeUiConfigsSig, setThemeUiConfigsSig] = createSignal<PerSystemUiConfigs>({});

/// Bridge for the App.tsx createEffect watching the active theme's
/// `perSystemUiConfigs`.
export function setThemeSystemUiConfigs(map: PerSystemUiConfigs | undefined): void {
  setThemeUiConfigsSig(map ?? {});
}

/// Resolve the effective UI config for a system: `BASELINE_UI` overlaid with
/// the active theme's per-system override (if any). Reactive — reading the
/// theme bridge signal, so consumers re-resolve when the active theme changes.
export function uiConfigFor(systemId: SystemId): SystemUIConfig {
  const override = themeUiConfigsSig()[systemId];
  return override ? { ...BASELINE_UI, ...override } : BASELINE_UI;
}
