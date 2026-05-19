export type SystemId = "tg16" | "pce-cd" | "lynx" | "nes" | "snes" | "mame";

export type SystemTheme = {
  id: SystemId;
  displayName: string;
  shortName: string;
  extensions: string[];
  /// CSS aspect-ratio for library + region-picker tiles ("4/3", "3/4", "1/1").
  /// Defaults to "3/4" (portrait) when unset. TG-16 box scans are landscape so
  /// "4/3" fits the whole cover without letterboxing.
  tileAspect?: string;
  /// Phase 3 slice C polish — shipped-default shader preset for this system.
  /// Consulted when the OA-wide setting is `"system-default"` (the new
  /// default for fresh installs). Per-game / per-system overrides still
  /// win. Picked per-system because what looks right varies by source
  /// resolution: low-res LCD-era systems like Lynx (160×102) get visibly
  /// crisper at default scanline intensity; CRT-era home consoles like
  /// NES (256×224) lean into the CRT-Lite preset that ships sample-rate
  /// scanlines + radial vignette.
  defaultShaderPreset?: string;
};

// Add a system: extend SystemId + add to systemThemes + add [data-system="..."] block in systems.css.
export const systemThemes: Record<SystemId, SystemTheme> = {
  tg16: {
    id: "tg16",
    displayName: "TurboGrafx-16 / PC Engine",
    shortName: "TG-16",
    // .pce = HuCard cart. CD images live under the separate `pce-cd` system —
    // they share the libretro core (Beetle PCE Fast, validated 2026-05-18) but
    // have distinct library/sidebar/theme treatment because cart and CD games
    // play very differently and deserve their own visual home.
    extensions: ["pce"],
    tileAspect: "4/3",
    // PCE HuCard art is sharp and color-rich; default to plain so the user
    // sees the source. Users wanting CRT vibes can toggle per-system.
    defaultShaderPreset: "plain",
  },
  "pce-cd": {
    id: "pce-cd",
    displayName: "TurboGrafx-CD / PC Engine CD-ROM²",
    shortName: "TG-CD",
    // CD image containers. cue/chd/ccd/toc/m3u/iso. Needs a PCE-CD BIOS
    // (syscard3.pce preferred) in <exe_dir>/system/ — the launch path
    // pre-checks it against the canonical Mednafen SHA-1s and refuses
    // missing/wrong BIOSes before the core gets a chance to crash on init.
    extensions: ["cue", "chd", "ccd", "toc", "m3u", "iso"],
    tileAspect: "4/3",
    // CDDA/FMV-heavy. Plain default keeps video crisp; users wanting CRT
    // halo on the games can drop into per-system.
    defaultShaderPreset: "plain",
  },
  lynx: {
    id: "lynx",
    displayName: "Atari Lynx",
    shortName: "Lynx",
    // .lnx is the canonical Handy-style dump. .lyx is a less common variant
    // some dumpers wrote. The mednafen_lynx libretro core handles both via
    // identical headerless framing. Lynx needs `lynxboot.img` in
    // <exe_dir>/system/ — same convention as PCE-CD's syscard3.pce.
    extensions: ["lnx", "lyx"],
    // Lynx box scans are landscape — match TG-16 + the home-console family.
    tileAspect: "4/3",
    // 160×102 source paints visibly chunky pixels — crt-lite's vignette +
    // saturation lift compensate for the perceived dimming from scanlines
    // on so few source rows.
    defaultShaderPreset: "crt-lite",
  },
  nes: {
    id: "nes",
    displayName: "Nintendo Entertainment System",
    shortName: "NES",
    // .nes  = iNES (the standard headered NES dump).
    // .fds  = Famicom Disk System (needs `disksys.rom` BIOS in
    //          <exe_dir>/system/).
    // .unf / .unif = UNIF format (some homebrew + obscure mappers).
    // NSF audio-only files are intentionally NOT scanned — they aren't
    // games. A future "tracks" UI surface might pick them up.
    extensions: ["nes", "fds", "unf", "unif"],
    // NES boxes were landscape in the West (Big Box era) but Famicom
    // boxes shipped vertical in Japan. The portrait default is closer to
    // what most modern scans show; landscape NES box art still fits.
    tileAspect: "3/4",
    // NES is the CRT-aesthetic poster child — 256×224 on a CRT TV is what
    // every screenshot remembers. crt-lite default.
    defaultShaderPreset: "crt-lite",
  },
  snes: {
    id: "snes",
    displayName: "Super Nintendo Entertainment System",
    shortName: "SNES",
    // .sfc  = canonical Super Famicom/SNES dump.
    // .smc  = with a 512-byte copier header (some old dumps).
    // .fig / .swc = less common copier formats; libretro cores handle both.
    extensions: ["sfc", "smc", "fig", "swc"],
    // SNES boxes are landscape (cartridge slip-case era).
    tileAspect: "4/3",
    // SNES games shipped with developers' awareness of CRT aliasing — many
    // titles look noticeably better through scanlines + bloom. crt-lite
    // default matches the era.
    defaultShaderPreset: "crt-lite",
  },
  mame: {
    id: "mame",
    displayName: "Arcade (MAME)",
    shortName: "MAME",
    // MAME ROM-sets are .zip archives keyed by short game name (e.g.
    // pacman.zip). The archive layer peeks inside .zip files first and
    // reclassifies to NES / SNES / etc. when it finds a recognized inner
    // extension; MAME zips contain hardware-specific binary blobs with
    // no standard extension, so they fall through to MAME by elimination.
    // `.chd` also appears for CD-/DVD-/HDD-backed arcade games (Killer
    // Instinct, etc.) — listed here so the scanner doesn't classify
    // those as PCE-CD.
    extensions: ["zip", "chd"],
    // Arcade flyer art is typically landscape — instruction cards and
    // marquees both lean wide.
    tileAspect: "4/3",
    // CRT-lite reads as period-correct for late-'80s / '90s arcade
    // hardware (15 kHz CRT monitors with visible scanlines on tubes).
    defaultShaderPreset: "crt-lite",
  },
};

export const DEFAULT_TILE_ASPECT = "3/4";

export function applySystemTheme(id: SystemId | null): void {
  const root = document.documentElement;
  if (id === null) {
    delete root.dataset.system;
    return;
  }
  root.dataset.system = id;
}

export function activeSystemTheme(): SystemTheme | null {
  const id = document.documentElement.dataset.system as SystemId | undefined;
  return id ? (systemThemes[id] ?? null) : null;
}

export function systemForExtension(extension: string): SystemId | null {
  const normalized = extension.replace(/^\./, "").toLowerCase();
  for (const theme of Object.values(systemThemes)) {
    if (theme.extensions.includes(normalized)) return theme.id;
  }
  return null;
}

/// Resolve a shader-preset string against per-system defaults. Used by the
/// launch path + per-system / per-game pages when the user's effective
/// value is the `"system-default"` sentinel — pick the system's shipped
/// recommendation if it has one, else fall back to `"plain"`.
export function resolveShaderPreset(value: string | null | undefined, systemId: SystemId | null): string {
  if (value && value !== "system-default") return value;
  if (systemId) {
    const fromSys = systemThemes[systemId]?.defaultShaderPreset;
    if (fromSys) return fromSys;
  }
  return "plain";
}

export function allSupportedExtensions(): string[] {
  const set = new Set<string>();
  for (const theme of Object.values(systemThemes)) {
    for (const ext of theme.extensions) set.add(ext);
  }
  return [...set];
}
