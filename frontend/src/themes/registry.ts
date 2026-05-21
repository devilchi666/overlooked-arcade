export type SystemId = "tg16" | "pce-cd" | "lynx" | "nes" | "snes" | "mame" | "atari7800" | "genesis" | "segacd" | "sega32x" | "saturn" | "psx" | "neogeo" | "neocd" | "ngp" | "jaguar" | "3do" | "pcfx" | "n64" | "gamecube" | "dreamcast" | "psp" | "ps2" | "nds" | "sms" | "gamegear" | "gb" | "gba" | "2600" | "5200" | "coleco" | "intv" | "o2" | "channelf" | "vectrex" | "virtualboy" | "wonderswan" | "pokemini";

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
  atari7800: {
    id: "atari7800",
    displayName: "Atari 7800 ProSystem",
    shortName: "Atari 7800",
    // .a78 is the canonical Atari 7800 dump — 128-byte header carries
    // mapper / region / BIOS-required flags the ProSystem core reads at
    // load time. Some old dumps are headerless `.bin`; intentionally
    // NOT included here to avoid colliding with future Atari 2600 / SMS
    // / other systems that all reuse `.bin`. Users with headerless 7800
    // dumps can rename to .a78 — the ProSystem core autodetects.
    extensions: ["a78"],
    // 7800 boxes (Big Box era, Atari Corp 1986+) shipped portrait — vertical
    // slip-cases with the title across the top. 3/4 fits the whole scan.
    tileAspect: "3/4",
    // 320×240 NTSC source running on early-'80s CRTs. crt-lite picks up
    // the visible scanlines + bloom these games were designed against.
    defaultShaderPreset: "crt-lite",
  },
  genesis: {
    id: "genesis",
    displayName: "Sega Mega Drive / Genesis",
    shortName: "Genesis",
    // .md  = canonical raw MD dump (most modern dump sets ship this).
    // .smd = Super Magic Drive interleaved format — cores deinterleave
    //         on load.
    // .gen = Kega Fusion-era alternate extension; same raw bytes as .md.
    // .68k = rare; some homebrew dumps lean on the CPU name.
    // .bin is INTENTIONALLY excluded — collides with PCE-CD track files
    // and future Atari 2600 / Sega CD audio tracks. Users with .bin MD
    // dumps rename to .md (same convention Atari 7800 uses for headerless
    // .bin → .a78).
    extensions: ["md", "smd", "gen", "68k"],
    // Mega Drive boxes are landscape (Sega's 16-bit-era retail boxes
    // were all wide). 4/3 fits the whole scan without letterboxing.
    tileAspect: "4/3",
    // Genesis games shipped on CRT TVs and the saturated MD palette
    // (Sonic's hyper-blue, Streets of Rage's neon, the YM2612-era
    // dithering) reads as period-correct under crt-lite scanlines +
    // soft vignette.
    defaultShaderPreset: "crt-lite",
  },
  segacd: {
    id: "segacd",
    displayName: "Sega CD / Mega-CD",
    shortName: "Sega CD",
    // CD image containers — same shape as PCE-CD. Disambiguation against
    // PCE-CD's claim happens via per-folder Import Wizard rules (the same
    // path PCE-CD navigated): operator marks a folder as `segacd` and
    // .cue/.chd/.iso/.m3u inside that folder route to Sega CD. Needs a
    // regional BIOS (bios_CD_E.bin / bios_CD_U.bin / bios_CD_J.bin) in
    // <exe_dir>/system/ — the launch path pre-checks SHA-1 against the
    // canonical Genesis Plus GX-blessed dumps (see check_sega_cd_bios in
    // apps/oa-shell/src/main.rs) and refuses missing BIOS before the
    // core gets a chance to crash on init.
    extensions: ["cue", "chd", "ccd", "toc", "m3u", "iso"],
    // Sega CD boxes shipped landscape (CD jewel-case sleeves wider than
    // tall in the retail era).
    tileAspect: "4/3",
    // CDDA + FMV-heavy library (Lunar, Sonic CD, Snatcher). Plain default
    // keeps video crisp; users wanting CRT halo on the games drop into
    // per-system Display.
    defaultShaderPreset: "plain",
  },
  sega32x: {
    id: "sega32x",
    displayName: "Sega 32X",
    shortName: "32X",
    // .32x = canonical headerless 32X cart dump (No-Intro standard).
    // PicoDrive reads this directly. .bin intentionally NOT registered —
    // same collision rationale Genesis uses (PCE-CD track files, future
    // Atari sets, audio tracks). Operators with .bin 32X dumps rename
    // to .32x; PicoDrive doesn't care about the extension.
    extensions: ["32x"],
    // 32X boxes were landscape, matching the Mega Drive family retail
    // packaging.
    tileAspect: "4/3",
    // 32X enhanced-mode 320×224 on CRT TVs — same crt-lite preset
    // Genesis ships, period-correct for the 1994 hardware.
    defaultShaderPreset: "crt-lite",
  },
  saturn: {
    id: "saturn",
    displayName: "Sega Saturn",
    shortName: "Saturn",
    // CD image containers — same shape as segacd/pce-cd. Disambiguation
    // via per-folder Import Wizard rule (operator marks a folder as
    // saturn; matching extensions inside route here). Requires a
    // regional Saturn BIOS in <exe_dir>/system/ — pre-checked by
    // check_saturn_bios in main.rs.
    extensions: ["cue", "chd", "ccd", "toc", "m3u", "iso"],
    // Saturn boxes shipped landscape — wide CD jewel-case sleeves.
    tileAspect: "4/3",
    // Saturn games shipped on CRT TVs; the VDP1/VDP2 dual-pipeline
    // rendering reads as period-correct under crt-lite scanlines.
    defaultShaderPreset: "crt-lite",
  },
  neogeo: {
    id: "neogeo",
    displayName: "SNK Neo Geo",
    shortName: "Neo Geo",
    // .neo = No-Intro Neo Geo single-file dump (modern canonical format).
    // .zip = MAME-compatible ROM-set; the library scanner runs a content-
    // peek (peek_zip_for_neogeo in archive.rs) to disambiguate Neo Geo
    // zips against MAME zips, emitting a systemHint when the .p1+.s1
    // signature matches. Without the hint, .zip files default to mame.
    // Requires neogeo.zip BIOS in <exe_dir>/system/.
    extensions: ["neo", "zip"],
    // Neo Geo AES + MVS boxes were landscape — Big Box era CD-jewel-case
    // wide for AES, marquee-style for MVS.
    tileAspect: "4/3",
    // Neo Geo arcade-era CRT visuals are exactly the genre crt-lite
    // was built for (saturated colors, hard sprite edges, designed for
    // CRT bloom).
    defaultShaderPreset: "crt-lite",
  },
  neocd: {
    id: "neocd",
    displayName: "SNK Neo Geo CD",
    shortName: "Neo Geo CD",
    // CD image containers — same shape as segacd/saturn/psx/pce-cd.
    // Disambiguation via per-folder Import Wizard rule. Requires
    // neocd_z.rom (top-loader) or neocd_t.rom (front-loader) BIOS in
    // <exe_dir>/system/ — pre-checked by check_neocd_bios.
    extensions: ["cue", "chd", "ccd", "toc", "m3u", "iso"],
    tileAspect: "4/3",
    defaultShaderPreset: "crt-lite",
  },
  n64: {
    id: "n64",
    displayName: "Nintendo 64",
    shortName: "N64",
    // .n64 / .z64 / .v64 are different byte-order conventions for the
    // same canonical content — Mupen64Plus-Next auto-detects.
    extensions: ["n64", "z64", "v64"],
    // N64 boxes were landscape Big Box era (1996-2001).
    tileAspect: "4/3",
    defaultShaderPreset: "crt-lite",
  },
  psp: {
    id: "psp",
    displayName: "Sony PlayStation Portable",
    shortName: "PSP",
    // .iso / .cso (compressed ISO) / .pbp (PSN-format EBOOT). PPSSPP
    // reads all three directly. No BIOS required (PPSSPP synthesizes
    // firmware behavior).
    extensions: ["iso", "cso", "pbp"],
    // PSP boxes were landscape UMD-jewel-case shape.
    tileAspect: "4/3",
    // PSP's 480×272 LCD source — handheld pearl shader convention
    // matches gb/gba/gg/ws (crt-lite as temporary handheld default
    // until the dedicated lcd-handheld preset lands).
    defaultShaderPreset: "crt-lite",
  },
  ps2: {
    id: "ps2",
    displayName: "Sony PlayStation 2",
    shortName: "PS2",
    // DVD-shape (most PS2 games shipped on DVD; some used CD). .iso
    // is the single-file container; .chd cross-collides with other CD
    // systems (per-folder Import Wizard rule). Regional BIOS required
    // (scph10000 JP / scph39001 US fat / scph70000 US-EU slim) in
    // <exe_dir>/system/ — pre-checked by check_ps2_bios; slots into
    // the CD-launch dispatch arm as 9th CD-shape system.
    extensions: ["iso", "chd"],
    tileAspect: "4/3",
    defaultShaderPreset: "crt-lite",
  },
  nds: {
    id: "nds",
    displayName: "Nintendo DS",
    shortName: "NDS",
    // .nds = headerless raw NDS dump (No-Intro standard).
    // 3-file BIOS required (bios7.bin + bios9.bin + firmware.bin) in
    // <exe_dir>/system/ — pre-checked by check_nds_bios (multi-file
    // shape). Touch screen flows through the new POINTER input infra
    // (mouse-as-touch at Phase 0; window-relative pixel-perfect
    // mapping is Phase 2.5).
    extensions: ["nds"],
    // NDS boxes were portrait clamshell sleeves (matches gb/gba
    // handheld family).
    tileAspect: "3/4",
    defaultShaderPreset: "crt-lite",
  },
  dreamcast: {
    id: "dreamcast",
    displayName: "Sega Dreamcast",
    shortName: "Dreamcast",
    // .cdi + .gdi are Dreamcast-unique GD-ROM container formats.
    // .chd is shared with other CD systems via per-folder Import
    // Wizard disambiguation. Regional BIOS required (dc_boot.bin +
    // dc_flash.bin per region) in <exe_dir>/system/; pre-checked by
    // check_dreamcast_bios.
    extensions: ["cdi", "gdi", "chd"],
    // DC boxes were landscape Big Box era (1998-2001).
    tileAspect: "4/3",
    // 480p/480i CRT era; the Dreamcast was the first console with
    // native 480p support. crt-lite reads as period-correct for the
    // bulk of the library.
    defaultShaderPreset: "crt-lite",
  },
  gamecube: {
    id: "gamecube",
    displayName: "Nintendo GameCube + Wii",
    shortName: "GC / Wii",
    // Single slug covers both GameCube + Wii via Dolphin's runtime
    // auto-detect. Extension union: .iso/.gcm/.gcz/.rvz for GameCube,
    // .wbfs for Wii (which also uses .iso). Per-folder Import Wizard
    // disambiguates against PSX/3DO/Saturn .iso collisions. Wii Remote
    // / motion-controls deferred to Phase 2.5 alongside full per-system
    // analog Bindings UI.
    extensions: ["iso", "gcm", "gcz", "rvz", "wbfs"],
    tileAspect: "4/3",
    // GC/Wii era was the early-HD CRT transition. crt-lite reads as
    // period-correct for the bulk of the library; users wanting clean
    // HD output toggle to plain via per-system Display.
    defaultShaderPreset: "crt-lite",
  },
  jaguar: {
    id: "jaguar",
    displayName: "Atari Jaguar",
    shortName: "Jaguar",
    // .j64 = canonical Atari Jaguar dump (No-Intro standard).
    // .jag = alternate extension some older dumpers used.
    // .rom intentionally NOT registered — too generic, collides with
    // multiple systems. Operators with .rom Jaguar dumps rename to .j64.
    extensions: ["j64", "jag"],
    // Jaguar boxes were landscape, Big Box era (1993-1996).
    tileAspect: "4/3",
    defaultShaderPreset: "crt-lite",
  },
  "3do": {
    id: "3do",
    displayName: "3DO Interactive Multiplayer",
    shortName: "3DO",
    // CD image containers — same shape as other CD-shape systems.
    // Disambiguation via per-folder Import Wizard rule. Requires a
    // regional/manufacturer BIOS (panafz1.bin / panafz10.bin /
    // goldstar.bin / sanyotry.bin) in <exe_dir>/system/ — pre-checked
    // by check_3do_bios.
    extensions: ["cue", "chd", "ccd", "toc", "m3u", "iso"],
    tileAspect: "4/3",
    defaultShaderPreset: "crt-lite",
  },
  pcfx: {
    id: "pcfx",
    displayName: "NEC PC-FX",
    shortName: "PC-FX",
    // CD image containers. PC-FX was Japan-only (1994-1998); single
    // canonical pcfx.rom BIOS required in <exe_dir>/system/.
    extensions: ["cue", "chd", "ccd", "toc", "m3u", "iso"],
    // PC-FX boxes were landscape Big Box era.
    tileAspect: "4/3",
    // PC-FX games leaned heavily on anime FMV; plain shader keeps
    // video crisp.
    defaultShaderPreset: "plain",
  },
  ngp: {
    id: "ngp",
    displayName: "SNK Neo Geo Pocket Color",
    shortName: "NGP/C",
    // .ngp = mono Neo Geo Pocket (1998); .ngc = Neo Geo Pocket Color
    // (1999). Single slug covers both — Beetle NeoPop auto-detects
    // hardware variant from the ROM header (same pattern as Gambatte
    // covering DMG+CGB and Beetle WonderSwan covering WS+WSC).
    // No BIOS required.
    extensions: ["ngp", "ngc"],
    // NGPC pack-in boxes were vertical clamshell sleeves — matches the
    // gb/gba/wonderswan handheld convention.
    tileAspect: "3/4",
    // 160×152 LCD source. Same temporary handheld default as gb/gba/
    // gamegear/wonderswan — crt-lite until the dedicated lcd-handheld
    // preset lands.
    defaultShaderPreset: "crt-lite",
  },
  psx: {
    id: "psx",
    displayName: "Sony PlayStation",
    shortName: "PS1",
    // CD image containers + .pbp (PSP-format PS1 EBOOT — common on
    // PSP-converted PS1 libraries). The .cue/.chd/.m3u set collides
    // with PCE-CD/segacd/saturn — same per-folder Import Wizard
    // disambiguation. .pbp is PSX-unique (no collision). Requires a
    // regional PSX BIOS in <exe_dir>/system/ — pre-checked by
    // check_psx_bios. Beetle PSX HW shipped as default; Beetle PSX
    // SW available as a catalog peer alternate (per-system Cores).
    extensions: ["cue", "chd", "ccd", "toc", "m3u", "iso", "pbp"],
    // PSX boxes shipped landscape — CD jewel-case sleeves.
    tileAspect: "4/3",
    // PSX games shipped on CRT TVs; 240p / 480i content reads as
    // period-correct under crt-lite scanlines. Beetle PSX HW's
    // upscaling stacks on top of this for the visually-premium output.
    defaultShaderPreset: "crt-lite",
  },
  gb: {
    id: "gb",
    displayName: "Nintendo Game Boy / Game Boy Color",
    shortName: "Game Boy",
    // Single slug covers both DMG (.gb) and CGB (.gbc) — Gambatte
    // auto-detects from the ROM header. No BIOS required for either
    // (optional `dmg_boot.bin` / `cgb_boot.bin` in <exe_dir>/system/
    // just enables the era-correct boot logo).
    extensions: ["gb", "gbc"],
    // Game Boy pack-in boxes were vertical clamshell sleeves — the
    // iconic Nintendo Big Box era for handhelds. 3/4 fits the whole
    // scan without letterboxing.
    tileAspect: "3/4",
    // 160×144 LCD source. Same compromise as Lynx and Game Gear —
    // crt-lite reads as a softened look that compensates for the chunky
    // pixel ratio at modern scales without faking CRT artifacts the
    // hardware never had. A dedicated `lcd-handheld` preset (subpixel
    // grid, no scanlines) is the follow-up.
    defaultShaderPreset: "crt-lite",
  },
  vectrex: {
    id: "vectrex",
    displayName: "GCE Vectrex",
    shortName: "Vectrex",
    // .vec = canonical Vectrex dump extension. .gam = alternate used by
    // some older dump sets. Both readable by vecx.
    extensions: ["vec", "gam"],
    // Vectrex boxes were landscape, showing the unit + the screen.
    tileAspect: "4/3",
    // Vector display on a green-phosphor CRT — crt-lite captures the
    // soft glow + scanline-less vector beam look acceptably for Phase 0.
    // A dedicated `vector-phosphor` shader preset (gaussian glow on
    // vector lines, no scanlines) is a Phase 2 polish item.
    defaultShaderPreset: "crt-lite",
  },
  virtualboy: {
    id: "virtualboy",
    displayName: "Nintendo Virtual Boy",
    shortName: "VB",
    // .vb = canonical Virtual Boy dump (No-Intro standard). Headerless.
    extensions: ["vb"],
    // VB box art was landscape, showing the headset unit.
    tileAspect: "4/3",
    // The Virtual Boy was monochrome RED on black — the OA dark surface
    // already provides the black; CRT scanlines + bloom would muddy the
    // crisp red-on-black LED palette. Plain default keeps the era-correct
    // monochrome aesthetic visible. Operators wanting anaglyph 3D mode
    // configure that via Beetle VB's core options.
    defaultShaderPreset: "plain",
  },
  wonderswan: {
    id: "wonderswan",
    displayName: "Bandai WonderSwan",
    shortName: "WonderSwan",
    // .ws = WonderSwan; .wsc = WonderSwan Color. Both routed to one slug
    // per the gb-pattern decision (single core covers both hardware
    // variants; Beetle WS auto-detects from ROM header).
    extensions: ["ws", "wsc"],
    // WS pack-in boxes were vertical handheld-style clamshells. 3/4
    // matches the GB/GBA family convention.
    tileAspect: "3/4",
    // 224×144 LCD source. Same compromise as Lynx/GG/GB/GBA — crt-lite
    // is the temporary handheld default until `lcd-handheld` preset
    // lands.
    defaultShaderPreset: "crt-lite",
  },
  coleco: {
    id: "coleco",
    displayName: "ColecoVision",
    shortName: "Coleco",
    // .col + .cv are both canonical ColecoVision dump extensions; some
    // dump sets ship one or the other. blueMSX reads both directly.
    // .bin is intentionally NOT registered — see 2600's rationale.
    // Operators with .bin-shaped Coleco libraries configure a per-folder
    // `*.bin → coleco` rule via the Import Wizard.
    extensions: ["col", "cv"],
    tileAspect: "3/4",
    defaultShaderPreset: "crt-lite",
  },
  intv: {
    id: "intv",
    displayName: "Mattel Intellivision",
    shortName: "Intv",
    // .int = canonical Intellivision dump (No-Intro standard).
    // .bin intentionally excluded; per-folder rule mechanism.
    extensions: ["int"],
    tileAspect: "3/4",
    defaultShaderPreset: "crt-lite",
  },
  o2: {
    id: "o2",
    displayName: "Magnavox Odyssey²",
    shortName: "Odyssey²",
    // .o2 = synthetic extension (no widely-standardized extension
    // exists; the O2 community ships almost entirely .bin). Users with
    // .o2 files (rare; some OpenEmu / RetroPie sets use the convention)
    // auto-scan; users with the common .bin shape configure a per-folder
    // `*.bin → o2` rule.
    extensions: ["o2"],
    tileAspect: "3/4",
    defaultShaderPreset: "crt-lite",
  },
  channelf: {
    id: "channelf",
    displayName: "Fairchild Channel F",
    shortName: "Channel F",
    // .chf = the Channel F community / FreeChaF homebrew extension.
    // Rare in older dump sets but present in modern homebrew. .bin
    // dominates older dumps and is excluded per the cross-system rule;
    // per-folder rule covers .bin-shaped libraries.
    extensions: ["chf"],
    tileAspect: "3/4",
    defaultShaderPreset: "crt-lite",
  },
  "2600": {
    id: "2600",
    displayName: "Atari 2600",
    shortName: "2600",
    // .a26 = canonical headerless raw 2600 dump. The 2600 community
    // never standardized on .a26 (most real libraries are .bin), so
    // the import wizard's per-folder rule mechanism is the documented
    // disambiguation for users with .bin-shaped collections.
    // Intentionally NOT registering .bin globally — collides with
    // PCE-CD track files + future Sega CD audio tracks +
    // Coleco/Intv/O2/ChannelF dumps.
    extensions: ["a26"],
    // 2600 box art was vertical Big Box era — saturated illustrations
    // on portrait clamshell sleeves. 3/4 fits the whole scan.
    tileAspect: "3/4",
    // 160×192 NTSC source on early-'80s CRT TVs. The canonical
    // CRT-era system; visible scanlines + soft bloom were designed
    // into the games. crt-lite is the strongest default.
    defaultShaderPreset: "crt-lite",
  },
  "5200": {
    id: "5200",
    displayName: "Atari 5200 SuperSystem",
    shortName: "5200",
    // .a52 = canonical Atari 5200 dump (.bin clashes with too many
    // siblings — operators with .bin libraries go through the Import
    // Wizard per-folder rule). Atari800 also reads .car (cartridge
    // container with header) but .a52 is the dominant format.
    extensions: ["a52"],
    // 5200 packaging was landscape Big Box era — wide gatefold
    // illustrations on the iconic black-with-silver sleeves. 4/3 fits
    // the whole scan.
    tileAspect: "4/3",
    // 320×192 NTSC source. Same CRT-era family as the 2600 — visible
    // scanlines + soft bloom designed into the visual style.
    defaultShaderPreset: "crt-lite",
  },
  pokemini: {
    id: "pokemini",
    displayName: "Nintendo Pokémon Mini",
    shortName: "PokeMini",
    // .min = canonical Pokémon Mini cart dump. Single canonical
    // extension; PokeMini libretro reads it directly.
    extensions: ["min"],
    // Tiny portrait Nintendo handheld — small landscape boxes shipped
    // with a hangtag. Portrait tile aspect 3/4 matches the GB family.
    tileAspect: "3/4",
    // 96×64 monochrome 2-bit LCD source. The PokeMini's actual screen
    // was a peculiar shape (close to square-ish) — crt-lite is the
    // best general default until a dedicated lcd-handheld preset lands.
    defaultShaderPreset: "crt-lite",
  },
  gba: {
    id: "gba",
    displayName: "Nintendo Game Boy Advance",
    shortName: "GBA",
    // .gba = headerless raw GBA dump. mGBA reads this directly. No
    // header strip needed (the cartridge ROM image is the canonical
    // bytes upstream catalogs as sha1).
    extensions: ["gba"],
    // GBA boxes were landscape clamshells, but most modern scan sets
    // ship the box art in portrait orientation (matching the GB/GBC
    // family). Portrait keeps the GBA family-aligned with `gb` in the
    // library view.
    tileAspect: "3/4",
    // 240×160 LCD. Same compromise as Lynx / GG / GB — crt-lite is a
    // temporary handheld default until the dedicated `lcd-handheld`
    // preset lands.
    defaultShaderPreset: "crt-lite",
  },
  sms: {
    id: "sms",
    displayName: "Sega Master System",
    shortName: "SMS",
    // .sms = headerless raw SMS dump. Genesis Plus GX and PicoDrive both
    // read this format directly. The optional `bios.sms` lives in
    // `<exe_dir>/system/` — without it games still run, they just skip
    // the era-correct boot logo.
    extensions: ["sms"],
    // SMS Western Big Box era (1986-1990) shipped landscape boxes —
    // the iconic black-with-neon-grid floor art is wide. 4/3 fits the
    // whole scan without letterboxing.
    tileAspect: "4/3",
    // SMS games on CRT. crt-lite picks up the visible scanlines and
    // soft vignette of period US/EU NTSC + PAL hardware.
    defaultShaderPreset: "crt-lite",
  },
  gamegear: {
    id: "gamegear",
    displayName: "Sega Game Gear",
    shortName: "Game Gear",
    // .gg = headerless raw GG dump. Same Genesis Plus GX core covers
    // both this and SMS — operators install one .dll, get both systems.
    // Optional `bios.gg` in `<exe_dir>/system/`.
    extensions: ["gg"],
    // GG boxes were small landscape clamshells — 4/3 fits the whole scan.
    tileAspect: "4/3",
    // 160×144 LCD source running on the unit's small backlit screen.
    // crt-lite reads as a softened look that compensates for the chunky
    // pixel ratio at modern scales without faking CRT artifacts the
    // hardware never had.
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
