// Per-system UI configuration — Stage 1 surface.
//
// Drives the Per-System Custom UI feature documented in
// `docs/PLANS/per-system-ui.md`. Each system in OA's library carries a
// `SystemUIConfig` describing its character: audio profile, background
// shape, interaction feel, tile shape, transition timing, button-label
// convention, etc. Stage 1 is the polish layer — Stage 2 adds layout +
// behavior fields, Stage 3 adds in-game overlay theming. All additions
// are additive; this Stage 1 shape stays valid as the spec grows.
//
// Stored as a sibling to `themes/registry.ts` (not merged into
// `SystemTheme`) for Stage 1 rollout safety — the existing visual
// theme work (accent colors, default fonts, default shader preset,
// tile aspect) stays untouched while this new behavioral layer rolls
// in. Merge can happen later once the shape stabilizes.
//
// Slice 1 ships this file plus the `perSystemUiEnabled` master toggle
// in Settings → Display. No consumers wire up to the registry yet —
// they arrive in subsequent slices (per-system SFX, backgrounds, boot
// animations, tile flourishes, pilot full builds, etc.). Defining the
// registry up-front lets the data shape settle before consumers depend
// on it.

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

  /// Stage 3 escape hatch. When set, the named custom component
  /// renders the system's library view instead of the shared
  /// renderer. Shipped values land per-pilot as Stage 1+ rolls out;
  /// Vectrex is the canonical first user. The string-or-literal
  /// union keeps TypeScript narrowing for known components while
  /// staying open to future per-system overrides.
  customComponent?: "vectrex" | string;
}

/// Baseline config — every system in OA shares this shape; pilots
/// override specific fields below. Resolves to the universal soft-click
/// SFX, a single accent-color static gradient background, and the
/// existing tile aspect from `systemThemes`.
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

/// Per-system UI configurations. Most systems inherit `BASELINE_UI`
/// unchanged; the three Stage 1 pilots (Game Boy, NES, Vectrex)
/// override per `docs/PLANS/per-system-ui.md` §8. Stage 2+ adds
/// showcase tier for ~5-10 more systems.
///
/// Slice 1 ships the registry; consumers wire up in later slices.
/// Adding a new system: extend `SystemId` in `registry.ts` and add a
/// matching entry here (the TS `Record<SystemId, …>` shape forces it).
export const systemUIConfigs: Record<SystemId, SystemUIConfig> = {
  // --- Pilot 1: Game Boy — soft / minimal / personal -------------------
  gb: {
    ...BASELINE_UI,
    interactionStyle: "delayed", // LCD-persistence feel
    tileShape: "portrait-3:4", // handheld convention
    buttonLabels: "nintendo-handheld",
  },

  // --- Pilot 2: NES — classic / bright / instant -----------------------
  nes: {
    ...BASELINE_UI,
    audioProfile: "console", // toy-piano boops on nav
    background: "animated", // scrolling NES-palette pattern
    buttonLabels: "nintendo-console",
  },

  // --- Pilot 3: Vectrex — vector phosphor signature --------------------
  vectrex: {
    ...BASELINE_UI,
    audioProfile: "console", // synthesized vector blips
    background: "shader", // phosphor-screen WGSL shader
    interactionStyle: "physical", // vector-pulse + glow bloom
    tileShape: "square",
    transitionTiming: "slow", // cinematic feel
    buttonLabels: "vectrex-custom",
    customComponent: "vectrex", // escape-hatch render path
  },

  // --- Baseline systems ------------------------------------------------
  tg16: { ...BASELINE_UI },
  "pce-cd": { ...BASELINE_UI },
  lynx: { ...BASELINE_UI },
  snes: { ...BASELINE_UI },
  mame: { ...BASELINE_UI },
  atari7800: { ...BASELINE_UI },
  genesis: { ...BASELINE_UI },
  segacd: { ...BASELINE_UI },
  sega32x: { ...BASELINE_UI },
  sega32xcd: { ...BASELINE_UI },
  saturn: { ...BASELINE_UI },
  psx: { ...BASELINE_UI },
  neogeo: { ...BASELINE_UI },
  neocd: { ...BASELINE_UI },
  ngp: { ...BASELINE_UI },
  jaguar: { ...BASELINE_UI },
  jagcd: { ...BASELINE_UI },
  "3do": { ...BASELINE_UI },
  pcfx: { ...BASELINE_UI },
  n64: { ...BASELINE_UI },
  gamecube: { ...BASELINE_UI },
  dreamcast: { ...BASELINE_UI },
  psp: { ...BASELINE_UI },
  ps2: { ...BASELINE_UI },
  nds: { ...BASELINE_UI },
  sms: { ...BASELINE_UI },
  gamegear: { ...BASELINE_UI },
  gbc: { ...BASELINE_UI },
  gba: { ...BASELINE_UI },
  "2600": { ...BASELINE_UI },
  "5200": { ...BASELINE_UI },
  coleco: { ...BASELINE_UI },
  intv: { ...BASELINE_UI },
  o2: { ...BASELINE_UI },
  channelf: { ...BASELINE_UI },
  virtualboy: { ...BASELINE_UI },
  wonderswan: { ...BASELINE_UI },
  pokemini: { ...BASELINE_UI },
  msx: { ...BASELINE_UI },
  msx2: { ...BASELINE_UI },
  scummvm: { ...BASELINE_UI },
  dosbox: { ...BASELINE_UI },
};

/// Resolve the UI config for a system. Returns `BASELINE_UI` if the
/// id isn't in the registry (defensive — TypeScript already forces an
/// entry, but the runtime call site might receive a stringly-typed id
/// from external data).
export function uiConfigFor(systemId: SystemId): SystemUIConfig {
  return systemUIConfigs[systemId] ?? BASELINE_UI;
}
