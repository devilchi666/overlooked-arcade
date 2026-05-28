// HOME v2 — per-system metadata stub data.
//
// Operator-authored hardcoded values matching the operator-supplied
// HOME mockup. Real metadata schema (released_year / cpu / sound /
// peripherals / etc.) doesn't exist on SystemTheme today — this file
// stands in until either: (a) the operator fills out a richer registry
// or (b) a metadata-enrichment workstream syncs the data from an
// authoritative source.
//
// All fields are optional. SystemInfoPanel renders a "—" for any field
// that's null/undefined, so partial entries gracefully degrade. The
// SNES entry matches the mockup verbatim; the other priority systems
// are hand-typed approximations the operator can refine.

import type { SystemId } from "../../themes/registry";

export type SystemPeripheral = {
  name: string;
  /// Single character / emoji to render as the peripheral's icon.
  glyph: string;
};

export type SystemSpecs = {
  // ===== SYSTEM INFORMATION section =====
  manufacturer?: string;
  type?: string;
  generation?: string;
  releaseDate?: string;
  discontinued?: string;
  unitsSold?: string;
  media?: string;
  cpu?: string;
  sound?: string;
  resolution?: string;
  colorPalette?: string;
  displayRatio?: string;

  // ===== TECHNICAL DETAILS section =====
  architecture?: string;
  maxPlayers?: string;
  coOpSupport?: string;
  region?: string;
  storage?: string;
  ram?: string;
  videoOutput?: string;
  aspectRatio?: string;
  inputLatency?: string;
  emulatorCore?: string;

  // ===== SUPPORTED PERIPHERALS section =====
  peripherals?: readonly SystemPeripheral[];

  // ===== Hero header extras =====
  /// Country-of-origin flag emoji rendered next to the release date in
  /// the hero. Optional.
  releaseFlag?: string;
  /// Short subtitle under the system name (e.g. "16-BIT HOME CONSOLE").
  tagline?: string;
  /// 2-4 sentence description shown below the release line in the hero.
  blurb?: string;
  /// Compact subline shown beneath the system name in the SYSTEMS
  /// sidebar — e.g. "16-BIT · 1990". Defaults to "" (no subline) when
  /// missing.
  sidebarSubline?: string;
};

// Defaults — for systems we haven't authored stubs for. Most fields
// stay undefined so the rendered panel shows "—" gracefully.
const DEFAULT_SPECS: SystemSpecs = {};

const SYSTEM_SPECS: Partial<Record<SystemId, SystemSpecs>> = {
  snes: {
    manufacturer: "Nintendo",
    type: "Home Console",
    generation: "4th Generation",
    releaseDate: "August 23, 1990",
    discontinued: "September 25, 1997",
    unitsSold: "49.10 Million",
    media: "Cartridge",
    cpu: "Ricoh 5A22 @ 3.58 MHz",
    sound: "Sony SPC700",
    resolution: "256 x 224",
    colorPalette: "32,768 Colors",
    displayRatio: "8:7 (4:3)",
    architecture: "16-Bit",
    maxPlayers: "1-2 Players",
    coOpSupport: "Yes",
    region: "NTSC / PAL",
    storage: "Cartridge (4MB Max)",
    ram: "128KB",
    videoOutput: "Composite / RGB",
    aspectRatio: "4:3",
    inputLatency: "Low",
    emulatorCore: "snes9x - Current",
    peripherals: [
      { name: "SNES Controller", glyph: "🎮" },
      { name: "Super Scope", glyph: "🔫" },
      { name: "Mouse", glyph: "🖱️" },
      { name: "Multitap (X4)", glyph: "🔗" },
      { name: "Wireless Adapter", glyph: "📡" },
    ],
    releaseFlag: "🇺🇸",
    tagline: "16-BIT HOME CONSOLE",
    blurb:
      "Nintendo's second home console and the successor to the NES. Famous for its vast library of RPGs, platformers, and classics that defined a generation of gaming.",
    sidebarSubline: "16-BIT · 1990",
  },
  nes: {
    manufacturer: "Nintendo",
    type: "Home Console",
    generation: "3rd Generation",
    releaseDate: "July 15, 1983",
    discontinued: "August 14, 2003",
    unitsSold: "61.91 Million",
    media: "Cartridge",
    cpu: "Ricoh 2A03 @ 1.79 MHz",
    sound: "Ricoh 2A03 APU",
    resolution: "256 x 240",
    colorPalette: "54 Colors",
    displayRatio: "8:7 (4:3)",
    architecture: "8-Bit",
    maxPlayers: "1-2 Players",
    coOpSupport: "Yes",
    region: "NTSC / PAL",
    storage: "Cartridge (1MB Max)",
    ram: "2KB",
    videoOutput: "RF / Composite",
    aspectRatio: "4:3",
    inputLatency: "Low",
    emulatorCore: "fceumm - Current",
    peripherals: [
      { name: "NES Controller", glyph: "🎮" },
      { name: "Zapper", glyph: "🔫" },
      { name: "Power Pad", glyph: "👣" },
      { name: "Four Score", glyph: "🔗" },
      { name: "R.O.B.", glyph: "🤖" },
    ],
    releaseFlag: "🇯🇵",
    tagline: "8-BIT HOME CONSOLE",
    blurb:
      "Nintendo's flagship console of the 1980s and the system that rebuilt the home video-game industry after the 1983 crash. Defined platformers, RPGs, and action games for a generation.",
    sidebarSubline: "8-BIT · 1983",
  },
  genesis: {
    manufacturer: "Sega",
    type: "Home Console",
    generation: "4th Generation",
    releaseDate: "October 29, 1988",
    discontinued: "April 30, 1997",
    unitsSold: "30.75 Million",
    media: "Cartridge",
    cpu: "Motorola 68000 @ 7.6 MHz",
    sound: "Yamaha YM2612 + SN76489",
    resolution: "320 x 224",
    colorPalette: "512 Colors",
    displayRatio: "32:35",
    architecture: "16-Bit",
    maxPlayers: "1-2 Players",
    coOpSupport: "Yes",
    region: "NTSC / PAL",
    storage: "Cartridge (4MB Max)",
    ram: "64KB",
    videoOutput: "Composite / RGB",
    aspectRatio: "4:3",
    inputLatency: "Low",
    emulatorCore: "genesis_plus_gx - Current",
    peripherals: [
      { name: "3-Button Pad", glyph: "🎮" },
      { name: "6-Button Pad", glyph: "🎮" },
      { name: "Menacer", glyph: "🔫" },
      { name: "Team Player", glyph: "🔗" },
      { name: "Activator", glyph: "📡" },
    ],
    releaseFlag: "🇯🇵",
    tagline: "16-BIT HOME CONSOLE",
    blurb:
      "Sega's answer to Nintendo's dominance. A sharper, faster machine famous for arcade-quality ports, Sonic the Hedgehog, and a fierce mascot rivalry that defined the 90s.",
    sidebarSubline: "16-BIT · 1988",
  },
  psx: {
    manufacturer: "Sony",
    type: "Home Console",
    generation: "5th Generation",
    releaseDate: "December 3, 1994",
    discontinued: "March 23, 2006",
    unitsSold: "102.50 Million",
    media: "CD-ROM",
    cpu: "MIPS R3000A @ 33.86 MHz",
    sound: "SPU 24-channel",
    resolution: "640 x 480 max",
    colorPalette: "16.7 Million",
    displayRatio: "4:3",
    architecture: "32-Bit",
    maxPlayers: "1-2 Players (Multitap up to 8)",
    coOpSupport: "Yes",
    region: "NTSC / PAL / NTSC-J",
    storage: "Memory Card (128KB)",
    ram: "2MB",
    videoOutput: "Composite / S-Video / RGB",
    aspectRatio: "4:3",
    inputLatency: "Low",
    emulatorCore: "beetle_psx_hw - Current",
    peripherals: [
      { name: "Digital Controller", glyph: "🎮" },
      { name: "DualShock", glyph: "🎮" },
      { name: "Multitap", glyph: "🔗" },
      { name: "GunCon", glyph: "🔫" },
      { name: "Mouse", glyph: "🖱️" },
    ],
    releaseFlag: "🇯🇵",
    tagline: "32-BIT HOME CONSOLE",
    blurb:
      "Sony's first console, and the system that brought 3D gaming to the mainstream. Its CD-ROM media unlocked cinematic storytelling and a JRPG renaissance.",
    sidebarSubline: "32-BIT · 1994",
  },
  ps2: {
    manufacturer: "Sony",
    type: "Home Console",
    generation: "6th Generation",
    releaseDate: "March 4, 2000",
    discontinued: "January 7, 2013",
    unitsSold: "155.00 Million",
    media: "DVD-ROM",
    cpu: "Emotion Engine @ 294 MHz",
    sound: "SPU2 48-channel",
    resolution: "1080i max",
    colorPalette: "16.7 Million",
    displayRatio: "4:3 / 16:9",
    architecture: "128-Bit",
    maxPlayers: "1-2 Players (Multitap up to 8)",
    coOpSupport: "Yes",
    region: "NTSC / PAL / NTSC-J",
    storage: "Memory Card (8MB)",
    ram: "32MB",
    videoOutput: "Composite / Component / RGB",
    aspectRatio: "4:3 / 16:9",
    inputLatency: "Low",
    emulatorCore: "pcsx2 - Current",
    peripherals: [
      { name: "DualShock 2", glyph: "🎮" },
      { name: "EyeToy", glyph: "📷" },
      { name: "Buzz! Buzzers", glyph: "🎚️" },
      { name: "Network Adapter", glyph: "🌐" },
      { name: "Multitap", glyph: "🔗" },
    ],
    releaseFlag: "🇯🇵",
    tagline: "128-BIT HOME CONSOLE",
    blurb:
      "The best-selling home console of all time. Sony's DVD-ROM machine dominated the 2000s with a sprawling library spanning arcade ports, JRPGs, and genre-defining originals.",
    sidebarSubline: "128-BIT · 2000",
  },
  n64: {
    manufacturer: "Nintendo",
    type: "Home Console",
    generation: "5th Generation",
    releaseDate: "June 23, 1996",
    discontinued: "April 30, 2002",
    unitsSold: "32.93 Million",
    media: "Cartridge",
    cpu: "NEC VR4300 @ 93.75 MHz",
    sound: "Reality Coprocessor (RSP)",
    resolution: "640 x 480 max",
    colorPalette: "16.7 Million",
    displayRatio: "4:3",
    architecture: "64-Bit",
    maxPlayers: "1-4 Players",
    coOpSupport: "Yes",
    region: "NTSC / PAL",
    storage: "Cartridge + Controller Pak",
    ram: "4MB (8MB w/ Expansion Pak)",
    videoOutput: "Composite / S-Video / RGB (mod)",
    aspectRatio: "4:3",
    inputLatency: "Low",
    emulatorCore: "mupen64plus_next - Current",
    peripherals: [
      { name: "N64 Controller", glyph: "🎮" },
      { name: "Rumble Pak", glyph: "📳" },
      { name: "Controller Pak", glyph: "💾" },
      { name: "Transfer Pak", glyph: "🔗" },
      { name: "Expansion Pak", glyph: "⚡" },
    ],
    releaseFlag: "🇯🇵",
    tagline: "64-BIT HOME CONSOLE",
    blurb:
      "Nintendo's first true 3D platform, defined by its analog stick, four-player split-screen, and a slate of genre-defining first-party games (Mario 64, Zelda: OoT, Goldeneye).",
    sidebarSubline: "64-BIT · 1996",
  },
  gb: {
    manufacturer: "Nintendo",
    type: "Handheld",
    generation: "4th Generation (handheld)",
    releaseDate: "April 21, 1989",
    discontinued: "March 23, 2003",
    unitsSold: "118.69 Million",
    media: "Cartridge",
    cpu: "Sharp LR35902 @ 4.19 MHz",
    sound: "4-channel PSG",
    resolution: "160 x 144",
    colorPalette: "4 Shades of Green",
    displayRatio: "10:9",
    architecture: "8-Bit",
    maxPlayers: "1-2 (Link Cable) / 4 (Adapter)",
    coOpSupport: "Yes",
    region: "Worldwide",
    storage: "Cartridge",
    ram: "8KB",
    videoOutput: "Built-in LCD",
    aspectRatio: "10:9",
    inputLatency: "Low",
    emulatorCore: "gambatte - Current",
    peripherals: [
      { name: "Link Cable", glyph: "🔗" },
      { name: "Camera", glyph: "📷" },
      { name: "Printer", glyph: "🖨️" },
      { name: "Super Game Boy", glyph: "🎮" },
      { name: "Four Player Adapter", glyph: "👥" },
    ],
    releaseFlag: "🇯🇵",
    tagline: "8-BIT HANDHELD",
    blurb:
      "The handheld that took gaming on the road. Modest hardware, a 4-shade green screen, and an unmatched library — Tetris, Pokémon, Zelda: Link's Awakening — that justified every pixel.",
    sidebarSubline: "8-BIT · 1989",
  },
};

export function getSystemSpecs(systemId: SystemId): SystemSpecs {
  return SYSTEM_SPECS[systemId] ?? DEFAULT_SPECS;
}
