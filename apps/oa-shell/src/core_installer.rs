//! Core installer — buildbot.libretro.com nightly catalog browser + downloader.
//!
//! Power users can drop libretro `.dll` / `.so` / `.dylib` files into
//! `<exe_dir>/cores/` by hand; this module turns that into a click. We
//! ship a curated catalog of ~30 popular cores covering the first-wave
//! Overlooked Arcade systems plus a handful of common Genesis / SNES /
//! NES alternates, fetch them on demand from the libretro nightly
//! buildbot, and write the resulting `.dll` (or `.so` / `.dylib` on
//! other hosts) straight into the cores folder.
//!
//! ## URL shape
//!
//! ```text
//! https://buildbot.libretro.com/nightly/<host>/<arch>/latest/<base>_libretro.<ext>.zip
//! ```
//!
//! - **windows / x86_64** → `windows/x86_64/`
//! - **linux / x86_64**   → `linux/x86_64/`
//! - **macos / x86_64**   → `apple/osx/x86_64/`
//! - **macos / arm64**    → `apple/osx/arm64/`
//!
//! Each zip contains exactly one `.dll`/`.so`/`.dylib` at the top level.
//! After extraction we run `oa_libretro::probe` against the new file to
//! validate the libretro ABI before reporting success — if it doesn't
//! probe, we delete the half-installed file and surface the error.
//!
//! ## Progress events
//!
//! Long-running downloads emit `oa://core-download-progress` with the
//! payload below so the UI can render a progress bar:
//!
//! ```text
//! { fileName, downloadedBytes, totalBytes, phase: "downloading" | "extracting" | "done" | "error" }
//! ```
//!
//! `totalBytes` may be `null` if the server didn't send a Content-Length.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

/// One curated catalog entry. Cross-platform: `base` is the buildbot
/// basename (no extension) and `systems` lists the Overlooked Arcade
/// `SystemId`s the core is appropriate for. The host-specific filename
/// is derived at runtime via [`core_filename_for_host`].
pub struct CatalogEntry {
    pub base: &'static str,
    pub display_name: &'static str,
    /// Short blurb shown under the title — what the core is, what's
    /// special about it.
    pub blurb: &'static str,
    /// System slugs this core can drive. Used by the frontend to bucket
    /// entries: slugs that match `systemThemes` keys go under the
    /// matching system; multi-entry rows go in the "Multi-system"
    /// section; slugs that don't match any wired OA system go in the
    /// "Not yet wired" section. NEVER empty — at minimum, name a system
    /// slug even if OA's registry doesn't have it yet (that's how this
    /// section gets populated).
    pub systems: &'static [&'static str],
    /// True for the OA-tested / first-pick core for each system. The
    /// frontend renders a "recommended" chip next to these so users
    /// who don't want to read every blurb know which to install. When
    /// multiple cores in the same system are marked recommended (rare —
    /// only when a system has distinct emulator families), the first
    /// one in catalog order wins for default-picking heuristics.
    pub recommended: bool,
    /// Required BIOS / firmware filename (relative to `<exe_dir>/system/`)
    /// when the core needs one. None for cores that ship complete.
    /// Surfaced in the UI as a warning chip before install.
    pub bios_required: Option<&'static str>,
}

/// Curated catalog of cores we expose in the installer. Adding a core
/// here is one struct row; the runtime derives `<base>_libretro.<ext>`
/// for the host's platform automatically. The frontend groups by the
/// `systems` field: slugs that match `systemThemes` keys (currently
/// `tg16 | pce-cd | lynx | nes | snes`) go under the wired section;
/// everything else goes under "Not yet wired" as a punch list.
///
/// Curation policy:
/// - **Included:** every system-emulator core covering hardware OA
///   targets now or plans to target. Vintage computers are deferred
///   (atari800 / vice_* / fuse / hatari / cap32 / px68k / x1 / etc.).
/// - **Special-case included:** ScummVM + DOSBox-Pure — not strictly
///   "system" emulators but standard parts of any retro library.
/// - **Excluded:** fantasy consoles + interpreters + one-game cores
///   (TIC-80 / WASM-4 / vircon32 / Anarch / ChaiLove / lutro / 2048 /
///   Dinothawr / Jumpnbump / NXEngine / OpenLara / etc.).
///
/// Recommendation policy: exactly one `recommended: true` per system
/// where a clear first pick exists (the OA-tested core or the broad
/// libretro consensus pick); multiple recommendations are acceptable
/// only when a system has genuinely distinct families (e.g. SNES has
/// snes9x for compat + bsnes for accuracy — both reasonable defaults).
pub const CATALOG: &[CatalogEntry] = &[
    // ════════════════════════════════════════════════════════════════
    // Wired in OA today (tg16 / pce-cd / lynx / nes / snes)
    // ════════════════════════════════════════════════════════════════

    // -------- TG-16 / PC Engine family --------
    CatalogEntry {
        base: "mednafen_pce_fast_libretro",
        display_name: "Beetle PCE Fast",
        blurb: "Mednafen-derived. Fast, plays HuCard + CD. OA default.",
        systems: &["tg16", "pce-cd"],
        recommended: true,
        bios_required: None,
    },
    CatalogEntry {
        base: "mednafen_pce_libretro",
        display_name: "Beetle PCE (full Mednafen)",
        blurb: "Higher-accuracy alternative. Heavier, same library coverage.",
        systems: &["tg16", "pce-cd"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "mednafen_supergrafx_libretro",
        display_name: "Beetle SuperGrafx",
        blurb: "SGX-only — the four SuperGrafx-enhanced HuCards.",
        systems: &["tg16"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "geargrafx_libretro",
        display_name: "GearGrafx",
        blurb: "Modern PC Engine core (gear* family). Active development.",
        systems: &["tg16", "pce-cd"],
        recommended: false,
        bios_required: None,
    },

    // -------- Atari Lynx --------
    CatalogEntry {
        base: "mednafen_lynx_libretro",
        display_name: "Beetle Lynx",
        blurb: "Atari Lynx. Needs lynxboot.img in <exe_dir>/system/.",
        systems: &["lynx"],
        recommended: true,
        bios_required: Some("lynxboot.img"),
    },
    CatalogEntry {
        base: "handy_libretro",
        display_name: "Handy",
        blurb: "Older Lynx core, slightly different audio character.",
        systems: &["lynx"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "gearlynx_libretro",
        display_name: "GearLynx",
        blurb: "Modern Lynx core (gear* family). Active development.",
        systems: &["lynx"],
        recommended: false,
        bios_required: Some("lynxboot.img"),
    },
    CatalogEntry {
        base: "holani_libretro",
        display_name: "Holani",
        blurb: "High-accuracy Lynx — newer entrant focused on chip-level accuracy.",
        systems: &["lynx"],
        recommended: false,
        bios_required: Some("lynxboot.img"),
    },

    // -------- NES / Famicom --------
    CatalogEntry {
        base: "fceumm_libretro",
        display_name: "FCEUmm",
        blurb: "FCEU + community mapper updates. Wide mapper coverage.",
        systems: &["nes"],
        recommended: true,
        bios_required: None,
    },
    CatalogEntry {
        base: "mesen_libretro",
        display_name: "Mesen",
        blurb: "Higher-accuracy NES. Cycle-stepped PPU.",
        systems: &["nes"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "nestopia_libretro",
        display_name: "Nestopia UE",
        blurb: "Long-running cycle-accurate NES core.",
        systems: &["nes"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "quicknes_libretro",
        display_name: "QuickNES",
        blurb: "Performance-focused NES. Lower compatibility, very fast.",
        systems: &["nes"],
        recommended: false,
        bios_required: None,
    },

    // -------- SNES / Super Famicom --------
    CatalogEntry {
        base: "snes9x_libretro",
        display_name: "Snes9x",
        blurb: "Standard SNES core. Fast, broad compatibility.",
        systems: &["snes"],
        recommended: true,
        bios_required: None,
    },
    CatalogEntry {
        base: "bsnes_libretro",
        display_name: "bsnes",
        blurb: "Higher-accuracy SNES (byuu-derived).",
        systems: &["snes"],
        recommended: true,
        bios_required: None,
    },
    CatalogEntry {
        base: "mesen-s_libretro",
        display_name: "Mesen-S",
        blurb: "Mesen author's SNES core, accuracy-focused.",
        systems: &["snes"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "bsnes-jg_libretro",
        display_name: "bsnes-jg",
        blurb: "Modernized bsnes maintenance fork.",
        systems: &["snes"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "bsnes_hd_beta_libretro",
        display_name: "bsnes-HD (beta)",
        blurb: "bsnes with mode-7 hi-res rendering. Looks gorgeous on F-Zero etc.",
        systems: &["snes"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "mednafen_supafaust_libretro",
        display_name: "Beetle Supafaust",
        blurb: "Performance-focused SNES (Mednafen Faust fork).",
        systems: &["snes"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "snes9x2010_libretro",
        display_name: "Snes9x 2010",
        blurb: "Snes9x mid-2010 snapshot — lighter for older hardware.",
        systems: &["snes"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "snes9x2005_plus_libretro",
        display_name: "Snes9x 2005 Plus",
        blurb: "Even lighter Snes9x for ultra-low-end devices.",
        systems: &["snes"],
        recommended: false,
        bios_required: None,
    },

    // ════════════════════════════════════════════════════════════════
    // Queued systems (registry entry not yet added in OA)
    // ════════════════════════════════════════════════════════════════

    // -------- Atari 2600 --------
    CatalogEntry {
        base: "stella_libretro",
        display_name: "Stella",
        blurb: "Atari 2600 — the standard pick.",
        systems: &["2600"],
        recommended: true,
        bios_required: None,
    },
    CatalogEntry {
        base: "stella2014_libretro",
        display_name: "Stella 2014",
        blurb: "Older Stella snapshot.",
        systems: &["2600"],
        recommended: false,
        bios_required: None,
    },

    // -------- Atari 5200 --------
    CatalogEntry {
        base: "a5200_libretro",
        display_name: "Atari800-derived 5200",
        blurb: "Atari 5200. BIOS required.",
        systems: &["5200"],
        recommended: true,
        bios_required: Some("5200.rom"),
    },

    // -------- Atari 7800 --------
    CatalogEntry {
        base: "prosystem_libretro",
        display_name: "ProSystem",
        blurb: "Atari 7800. BIOS optional but recommended.",
        systems: &["atari7800"],
        recommended: true,
        bios_required: None,
    },

    // -------- Atari Jaguar --------
    CatalogEntry {
        base: "virtualjaguar_libretro",
        display_name: "Virtual Jaguar",
        blurb: "Atari Jaguar. Compatibility is patchy by game.",
        systems: &["jaguar", "jagcd"],
        recommended: true,
        bios_required: None,
    },

    // -------- Sega Master System / Game Gear / Genesis / Sega CD / 32X --------
    CatalogEntry {
        base: "genesis_plus_gx_libretro",
        display_name: "Genesis Plus GX",
        blurb: "SMS / GG / Mega Drive / Sega CD — the standard multi-Sega pick.",
        systems: &["sms", "gamegear", "genesis", "segacd"],
        recommended: true,
        bios_required: None,
    },
    CatalogEntry {
        base: "genesis_plus_gx_wide_libretro",
        display_name: "Genesis Plus GX Wide",
        blurb: "Genesis Plus GX with widescreen hacks.",
        systems: &["genesis", "segacd"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "picodrive_libretro",
        display_name: "PicoDrive",
        blurb: "Lighter Mega Drive / 32X / Sega CD.",
        systems: &["genesis", "segacd", "sega32x", "sega32xcd"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "blastem_libretro",
        display_name: "BlastEm",
        blurb: "Higher-accuracy Mega Drive — chip-level focus.",
        systems: &["genesis"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "clownmdemu_libretro",
        display_name: "ClownMDEmu",
        blurb: "Modern Mega Drive core, active development.",
        systems: &["genesis"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "smsplus_libretro",
        display_name: "SMS Plus GX",
        blurb: "SMS / Game Gear-only alternative.",
        systems: &["sms", "gamegear"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "gearsystem_libretro",
        display_name: "Gearsystem",
        blurb: "Modern SMS / GG / SG-1000 / ColecoVision (gear* family).",
        systems: &["sms", "gamegear"],
        recommended: false,
        bios_required: None,
    },

    // -------- Sega Saturn --------
    CatalogEntry {
        base: "mednafen_saturn_libretro",
        display_name: "Beetle Saturn",
        blurb: "Sega Saturn — Mednafen-derived, high accuracy. Demanding.",
        systems: &["saturn"],
        recommended: true,
        bios_required: Some("sega_101.bin (or equivalent)"),
    },
    CatalogEntry {
        base: "kronos_libretro",
        display_name: "Kronos",
        blurb: "Faster Saturn core. Lower accuracy, broader hardware reach.",
        systems: &["saturn"],
        recommended: false,
        bios_required: Some("sega_101.bin (or equivalent)"),
    },
    CatalogEntry {
        base: "yabasanshiro_libretro",
        display_name: "YabaSanshiro",
        blurb: "Yabause-derived Saturn core.",
        systems: &["saturn"],
        recommended: false,
        bios_required: Some("sega_101.bin (or equivalent)"),
    },
    CatalogEntry {
        base: "yabause_libretro",
        display_name: "Yabause",
        blurb: "Older Saturn core, original Yabause branch.",
        systems: &["saturn"],
        recommended: false,
        bios_required: Some("sega_101.bin (or equivalent)"),
    },

    // -------- Sega Dreamcast --------
    CatalogEntry {
        base: "flycast_libretro",
        display_name: "Flycast",
        blurb: "Dreamcast / Naomi / Atomiswave. The standard DC pick.",
        systems: &["dreamcast"],
        recommended: true,
        bios_required: Some("dc_boot.bin + dc_flash.bin"),
    },

    // -------- 3DO Interactive Multiplayer --------
    CatalogEntry {
        base: "opera_libretro",
        display_name: "Opera (4DO)",
        blurb: "3DO. Renamed from 4DO; the standard libretro 3DO pick.",
        systems: &["3do"],
        recommended: true,
        bios_required: Some("panafz1.bin / panafz10.bin / goldstar.bin / sanyotry.bin"),
    },

    // -------- Game Boy / GBC --------
    CatalogEntry {
        base: "gambatte_libretro",
        display_name: "Gambatte",
        blurb: "GB / GBC — very high accuracy, the standard pick.",
        systems: &["gb", "gbc"],
        recommended: true,
        bios_required: None,
    },
    CatalogEntry {
        base: "sameboy_libretro",
        display_name: "SameBoy",
        blurb: "Highest-accuracy GB / GBC core. Recommended for cycle-perfect.",
        systems: &["gb", "gbc"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "gearboy_libretro",
        display_name: "Gearboy",
        blurb: "Modern GB / GBC (gear* family).",
        systems: &["gb", "gbc"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "tgbdual_libretro",
        display_name: "TGB Dual",
        blurb: "Two GB / GBC instances simultaneously — link-cable simulation.",
        systems: &["gb", "gbc"],
        recommended: false,
        bios_required: None,
    },

    // -------- Game Boy Advance --------
    CatalogEntry {
        base: "mgba_libretro",
        display_name: "mGBA",
        blurb: "GBA + GB / GBC — the standard pick.",
        systems: &["gba", "gb", "gbc"],
        recommended: true,
        bios_required: None,
    },
    CatalogEntry {
        base: "vba_next_libretro",
        display_name: "VBA Next",
        blurb: "Lighter VBA-derived GBA core.",
        systems: &["gba"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "vbam_libretro",
        display_name: "VBA-M",
        blurb: "VisualBoyAdvance-M — community fork.",
        systems: &["gba"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "mednafen_gba_libretro",
        display_name: "Beetle GBA",
        blurb: "Mednafen GBA — VBA-derived, Mednafen wrapping.",
        systems: &["gba"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "gpsp_libretro",
        display_name: "gpSP",
        blurb: "Performance-focused GBA, originally for PSP.",
        systems: &["gba"],
        recommended: false,
        bios_required: Some("gba_bios.bin"),
    },

    // -------- Sony PlayStation --------
    CatalogEntry {
        base: "mednafen_psx_hw_libretro",
        display_name: "Beetle PSX HW",
        blurb: "Sony PlayStation, HW renderer. High accuracy.",
        systems: &["psx"],
        recommended: true,
        bios_required: Some("scph5500.bin / scph5501.bin / scph5502.bin"),
    },
    CatalogEntry {
        base: "swanstation_libretro",
        display_name: "SwanStation",
        blurb: "Faster PSX, fork of DuckStation.",
        systems: &["psx"],
        recommended: true,
        bios_required: Some("scph5500.bin / scph5501.bin / scph5502.bin"),
    },
    CatalogEntry {
        base: "mednafen_psx_libretro",
        display_name: "Beetle PSX (software)",
        blurb: "Beetle PSX without HW renderer. Lower system requirements.",
        systems: &["psx"],
        recommended: false,
        bios_required: Some("scph5500.bin / scph5501.bin / scph5502.bin"),
    },
    CatalogEntry {
        base: "pcsx_rearmed_libretro",
        display_name: "PCSX-ReARMed",
        blurb: "Lightweight PSX, originally for ARM. Strong perf-per-watt.",
        systems: &["psx"],
        recommended: false,
        bios_required: Some("scph5500.bin / scph5501.bin / scph5502.bin"),
    },

    // -------- Sony PlayStation 2 --------
    CatalogEntry {
        base: "pcsx2_libretro",
        display_name: "PCSX2",
        blurb: "PS2 — official PCSX2 libretro core. Demanding.",
        systems: &["ps2"],
        recommended: true,
        bios_required: Some("PS2 BIOS (ps2-0230a-20080220.bin etc.)"),
    },
    CatalogEntry {
        base: "play_libretro",
        display_name: "Play!",
        blurb: "Lighter PS2 core. Lower compatibility.",
        systems: &["ps2"],
        recommended: false,
        bios_required: None,
    },

    // -------- Sony PSP --------
    CatalogEntry {
        base: "ppsspp_libretro",
        display_name: "PPSSPP",
        blurb: "PlayStation Portable. The standard pick, runs most games well.",
        systems: &["psp"],
        recommended: true,
        bios_required: None,
    },

    // -------- Nintendo 64 --------
    CatalogEntry {
        base: "mupen64plus_next_libretro",
        display_name: "Mupen64Plus-Next",
        blurb: "N64 — GLideN64-based renderer. The standard pick.",
        systems: &["n64"],
        recommended: true,
        bios_required: None,
    },
    CatalogEntry {
        base: "parallel_n64_libretro",
        display_name: "ParaLLEl N64",
        blurb: "N64 with ParaLLEl-RDP/RSP. Accuracy-focused, demands a strong GPU.",
        systems: &["n64"],
        recommended: false,
        bios_required: None,
    },

    // -------- Nintendo DS --------
    CatalogEntry {
        base: "melonds_libretro",
        display_name: "melonDS",
        blurb: "Nintendo DS — high accuracy, modern.",
        systems: &["nds"],
        recommended: true,
        bios_required: None,
    },
    CatalogEntry {
        base: "melondsds_libretro",
        display_name: "melonDS DS",
        blurb: "melonDS variant with multi-screen UI options.",
        systems: &["nds"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "desmume_libretro",
        display_name: "DeSmuME",
        blurb: "Long-running DS core. Stable but slower than melonDS.",
        systems: &["nds"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "desmume2015_libretro",
        display_name: "DeSmuME 2015",
        blurb: "Older DeSmuME snapshot, lighter perf profile.",
        systems: &["nds"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "noods_libretro",
        display_name: "NooDS",
        blurb: "Modern DS core. Active development.",
        systems: &["nds"],
        recommended: false,
        bios_required: None,
    },

    // -------- Nintendo 3DS --------
    CatalogEntry {
        base: "citra_libretro",
        display_name: "Citra",
        blurb: "Nintendo 3DS — the standard pick. Heavy.",
        systems: &["3ds"],
        recommended: true,
        bios_required: None,
    },
    CatalogEntry {
        base: "citra2018_libretro",
        display_name: "Citra 2018",
        blurb: "Older Citra snapshot.",
        systems: &["3ds"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "panda3ds_libretro",
        display_name: "Panda3DS",
        blurb: "Modern 3DS core, active development.",
        systems: &["3ds"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "azahar_libretro",
        display_name: "Azahar",
        blurb: "Citra-derived 3DS fork.",
        systems: &["3ds"],
        recommended: false,
        bios_required: None,
    },

    // -------- Nintendo GameCube / Wii --------
    CatalogEntry {
        base: "dolphin_libretro",
        display_name: "Dolphin",
        blurb: "GameCube + Wii. Very demanding — requires a strong host.",
        systems: &["gamecube", "wii"],
        recommended: true,
        bios_required: None,
    },

    // -------- Nintendo Virtual Boy --------
    CatalogEntry {
        base: "mednafen_vb_libretro",
        display_name: "Beetle VB",
        blurb: "Mednafen Virtual Boy. Anaglyph or split-screen 3D.",
        systems: &["virtualboy"],
        recommended: true,
        bios_required: None,
    },

    // -------- Nintendo Pokémon Mini --------
    CatalogEntry {
        base: "pokemini_libretro",
        display_name: "PokeMini",
        blurb: "Pokémon Mini handheld.",
        systems: &["pokemini"],
        recommended: true,
        bios_required: None,
    },

    // -------- Bandai WonderSwan / WS Color --------
    CatalogEntry {
        base: "mednafen_wswan_libretro",
        display_name: "Beetle WonderSwan",
        blurb: "WonderSwan + WonderSwan Color.",
        systems: &["wonderswan"],
        recommended: true,
        bios_required: None,
    },

    // -------- SNK Neo Geo Pocket / Color --------
    CatalogEntry {
        base: "mednafen_ngp_libretro",
        display_name: "Beetle NeoPop",
        blurb: "Neo Geo Pocket + NGP Color. Mednafen-wrapped NeoPop.",
        systems: &["ngp"],
        recommended: true,
        bios_required: None,
    },
    CatalogEntry {
        base: "race_libretro",
        display_name: "RACE",
        blurb: "Lighter Neo Geo Pocket core.",
        systems: &["ngp"],
        recommended: false,
        bios_required: None,
    },

    // -------- SNK Neo Geo CD --------
    CatalogEntry {
        base: "neocd_libretro",
        display_name: "NeoCD",
        blurb: "Neo Geo CD.",
        systems: &["neocd"],
        recommended: true,
        bios_required: Some("neocd_z.rom + neocd_sp.rom + neocd_f.rom"),
    },

    // -------- NEC PC-FX --------
    CatalogEntry {
        base: "mednafen_pcfx_libretro",
        display_name: "Beetle PC-FX",
        blurb: "NEC PC-FX — Mednafen-derived. Sister system to PCE-CD.",
        systems: &["pcfx"],
        recommended: true,
        bios_required: Some("pcfx.rom"),
    },

    // -------- MSX / MSX2 / Turbo R --------
    CatalogEntry {
        base: "bluemsx_libretro",
        display_name: "blueMSX",
        blurb: "MSX / MSX2 / MSX Turbo R. Broad coverage.",
        systems: &["msx", "msx2"],
        recommended: true,
        bios_required: None,
    },
    CatalogEntry {
        base: "fmsx_libretro",
        display_name: "fMSX",
        blurb: "Lighter MSX alternative.",
        systems: &["msx"],
        recommended: false,
        bios_required: None,
    },

    // -------- ColecoVision --------
    CatalogEntry {
        base: "gearcoleco_libretro",
        display_name: "GearColeco",
        blurb: "ColecoVision (gear* family).",
        systems: &["coleco"],
        recommended: true,
        bios_required: Some("colecovision.col"),
    },

    // -------- Vectrex --------
    CatalogEntry {
        base: "vecx_libretro",
        display_name: "Vecx",
        blurb: "Vectrex with optional overlay support.",
        systems: &["vectrex"],
        recommended: true,
        bios_required: None,
    },

    // -------- Magnavox Odyssey² / Philips Videopac --------
    CatalogEntry {
        base: "o2em_libretro",
        display_name: "O2EM",
        blurb: "Magnavox Odyssey² / Philips Videopac G7000.",
        systems: &["o2"],
        recommended: true,
        bios_required: Some("o2rom.bin"),
    },

    // -------- Mattel Intellivision --------
    CatalogEntry {
        base: "freeintv_libretro",
        display_name: "FreeIntv",
        blurb: "Intellivision.",
        systems: &["intv"],
        recommended: true,
        bios_required: Some("exec.bin + grom.bin"),
    },

    // -------- Fairchild Channel F --------
    CatalogEntry {
        base: "freechaf_libretro",
        display_name: "FreeChaF",
        blurb: "Fairchild Channel F.",
        systems: &["channelf"],
        recommended: true,
        bios_required: None,
    },

    // ════════════════════════════════════════════════════════════════
    // Multi-system / arcade / DOS / engines
    // ════════════════════════════════════════════════════════════════

    // -------- MAME family --------
    CatalogEntry {
        base: "mame_libretro",
        display_name: "MAME (latest)",
        blurb: "Latest MAME. Heaviest, broadest hardware coverage.",
        systems: &["mame", "stv"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "mame2003_plus_libretro",
        display_name: "MAME 2003 Plus",
        blurb: "MAME 2003 with community additions. Strong compat/perf balance.",
        systems: &["mame", "stv"],
        recommended: true,
        bios_required: None,
    },
    CatalogEntry {
        base: "mame2010_libretro",
        display_name: "MAME 2010",
        blurb: "MAME 2010 snapshot. Lighter than latest, narrower coverage.",
        systems: &["mame", "stv"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "mame2003_libretro",
        display_name: "MAME 2003",
        blurb: "MAME 2003 baseline. Lower system requirements.",
        systems: &["mame", "stv"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "mame2000_libretro",
        display_name: "MAME 2000",
        blurb: "Very old MAME snapshot for ultra-low-end devices.",
        systems: &["mame", "stv"],
        recommended: false,
        bios_required: None,
    },

    // -------- FinalBurn family --------
    CatalogEntry {
        base: "fbneo_libretro",
        display_name: "FinalBurn Neo",
        blurb: "Arcade — Neo Geo, CPS1/2/3, Cave, more. The standard arcade pick.",
        systems: &["fbneo", "neogeo"],
        recommended: true,
        bios_required: None,
    },
    CatalogEntry {
        base: "fbalpha2012_libretro",
        display_name: "FB Alpha 2012",
        blurb: "Older FB Alpha snapshot.",
        systems: &["fbneo"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "fbalpha2012_cps1_libretro",
        display_name: "FB Alpha 2012 (CPS-1)",
        blurb: "Capcom Play System 1 only — lighter for older devices.",
        systems: &["fbneo"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "fbalpha2012_cps2_libretro",
        display_name: "FB Alpha 2012 (CPS-2)",
        blurb: "Capcom Play System 2 only.",
        systems: &["fbneo"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "fbalpha2012_cps3_libretro",
        display_name: "FB Alpha 2012 (CPS-3)",
        blurb: "Capcom Play System 3 only (Street Fighter III, Jojo).",
        systems: &["fbneo"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "fbalpha2012_neogeo_libretro",
        display_name: "FB Alpha 2012 (Neo Geo)",
        blurb: "Neo Geo only — lighter for older devices.",
        systems: &["fbneo", "neogeo"],
        recommended: false,
        bios_required: Some("neogeo.zip"),
    },

    // -------- DOS / ScummVM --------
    CatalogEntry {
        base: "dosbox_pure_libretro",
        display_name: "DOSBox-Pure",
        blurb: "DOS games — the standard libretro DOSBox. Mounts .zip / .iso directly.",
        systems: &["dosbox"],
        recommended: true,
        bios_required: None,
    },
    CatalogEntry {
        base: "dosbox_core_libretro",
        display_name: "DOSBox (vanilla)",
        blurb: "Closer-to-mainline DOSBox port.",
        systems: &["dosbox"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "dosbox_svn_libretro",
        display_name: "DOSBox SVN",
        blurb: "SVN-based DOSBox snapshot.",
        systems: &["dosbox"],
        recommended: false,
        bios_required: None,
    },
    CatalogEntry {
        base: "scummvm_libretro",
        display_name: "ScummVM",
        blurb: "Point-and-click adventure runtime — Lucasarts, Sierra, etc.",
        systems: &["scummvm"],
        recommended: true,
        bios_required: None,
    },
];

// =====================================================================
// Tier preference table — Guided Setup Phase 2
// =====================================================================
//
// Drives the curated core recommendation per detected CPU tier
// (High / Mid / Low) on top of the CATALOG above. Decision-locked
// 2026-06-06 per `docs/PLANS/guided-setup.md` §7: the heuristic is
// SOURCE OF TRUTH, not a placeholder until benchmarks land.
//
// Only systems with multiple viable cores need entries here — systems
// with one obvious pick (TG-16's Beetle PCE Fast, NES's FCEUmm,
// GameBoy's Gambatte, Lynx's Handy, Vectrex's vecx, etc.) use their
// existing `default_core_dll_for_system` registry value and skip this
// table. `recommended_core_for_tier(system_id, tier)` returns `None`
// for those systems so the caller can fall through.

/// One per-system tier-preference row. `high`/`mid`/`low` are buildbot
/// basenames matching entries in the [`CATALOG`] above. The host-
/// specific filename is derived at runtime via
/// [`core_filename_for_host`] just like CATALOG entries.
pub struct TierPreference {
    pub system_id: &'static str,
    pub high: &'static str,
    pub mid: &'static str,
    pub low: &'static str,
}

/// Per-system tier preferences. Order doesn't matter — lookup is by
/// linear scan over `system_id`. Each `high`/`mid`/`low` value
/// references a `CATALOG` row's `base`. Two patterns:
///
/// - **Distinct cores per tier** (psx / saturn / genesis): each tier
///   picks a meaningfully different core. High picks the most
///   demanding-but-accurate; Low picks the lightest. Mid sits between.
///
/// - **Same core for High + Mid, lighter for Low** (snes / n64 / ps2 /
///   nds): the accuracy-focused core handles High AND Mid fine; only
///   Low hardware needs a lighter alternative. Avoids over-recommending
///   the lightweight core to operators whose hardware would handle the
///   accurate one with headroom.
pub const TIER_PREFERENCES: &[TierPreference] = &[
    // PSX — three meaningfully different tiers:
    // - High: Beetle PSX HW (Vulkan HW renderer, PGXP geometry
    //   correction) — needs a real GPU + ~3 GHz CPU.
    // - Mid: SwanStation (DuckStation fork) — strong balance, runs
    //   most games great on modest hardware.
    // - Low: PCSX-ReARMed — lightest, originally for ARM. Loses HW
    //   renderer + PGXP.
    TierPreference {
        system_id: "psx",
        high: "mednafen_psx_hw_libretro",
        mid: "swanstation_libretro",
        low: "pcsx_rearmed_libretro",
    },
    // SNES — accuracy ladder:
    // - High: bsnes (cycle-accurate, demands CPU). 6+ cores at 3 GHz
    //   handle it without breaking sweat.
    // - Mid + Low: snes9x. Long-running compatibility core, runs the
    //   full SNES library at full speed on essentially any host.
    TierPreference {
        system_id: "snes",
        high: "bsnes_libretro",
        mid: "snes9x_libretro",
        low: "snes9x_libretro",
    },
    // N64 — renderer-path split. Mupen64Plus-Next is the standard;
    // ParaLLEl is the lightweight fallback for hardware that can't
    // sustain HLE GPU plugin throughput.
    // - High + Mid: mupen64plus_next (GLideN64-based renderer).
    // - Low: parallel_n64 (lighter).
    TierPreference {
        system_id: "n64",
        high: "mupen64plus_next_libretro",
        mid: "mupen64plus_next_libretro",
        low: "parallel_n64_libretro",
    },
    // Genesis / Mega Drive — Genesis Plus GX is the accuracy default;
    // PicoDrive is the lightweight alternative. Most modern hardware
    // runs GPGX at full speed; Low picks PicoDrive specifically because
    // GPGX's accuracy enhancements can briefly stall on very weak chips.
    // - High + Mid: genesis_plus_gx.
    // - Low: picodrive.
    TierPreference {
        system_id: "genesis",
        high: "genesis_plus_gx_libretro",
        mid: "genesis_plus_gx_libretro",
        low: "picodrive_libretro",
    },
    // Saturn — three viable cores with real tier differentiation:
    // - High: Beetle Saturn (Mednafen-derived, high accuracy + GPU
    //   demand).
    // - Mid: Kronos (faster, broader hardware reach, some accuracy
    //   compromises).
    // - Low: YabaSanshiro (lightest of the three).
    TierPreference {
        system_id: "saturn",
        high: "mednafen_saturn_libretro",
        mid: "kronos_libretro",
        low: "yabasanshiro_libretro",
    },
    // PS2 — PCSX2 is the only practical option for compatibility; Play!
    // is much lighter but runs few games. Low tier hardware can't run
    // PS2 well regardless, so Play! gives operators on weak machines
    // something rather than nothing.
    // - High + Mid: pcsx2.
    // - Low: play.
    TierPreference {
        system_id: "ps2",
        high: "pcsx2_libretro",
        mid: "pcsx2_libretro",
        low: "play_libretro",
    },
    // NDS — melonDS is the accuracy + features standard; DeSmuME is
    // the older, lighter alternative.
    // - High + Mid: melonds.
    // - Low: desmume.
    TierPreference {
        system_id: "nds",
        high: "melonds_libretro",
        mid: "melonds_libretro",
        low: "desmume_libretro",
    },
];

/// Look up the recommended core base name for `(system_id, tier)`.
/// Returns `None` when the system isn't in [`TIER_PREFERENCES`] — the
/// caller should fall through to whatever per-system default the
/// registry carries (`default_core_dll_for_system_resolved` in
/// `main.rs`). Most systems take this fallthrough path because they
/// have a single obvious core pick that's right at every tier.
pub fn recommended_core_for_tier(
    system_id: &str,
    tier: crate::cpu_tier::CpuTier,
) -> Option<&'static str> {
    let entry = TIER_PREFERENCES.iter().find(|p| p.system_id == system_id)?;
    let base = match tier {
        crate::cpu_tier::CpuTier::High => entry.high,
        crate::cpu_tier::CpuTier::Mid => entry.mid,
        crate::cpu_tier::CpuTier::Low => entry.low,
    };
    Some(base)
}

/// The platform-specific dynamic-library extension for the running host.
/// Mirrors `core_extension_for_host` in main.rs — kept duplicated here so
/// the installer module doesn't import shell-private helpers.
fn dylib_ext() -> &'static str {
    if cfg!(windows)          { "dll"   }
    else if cfg!(target_os = "macos") { "dylib" }
    else                              { "so"    }
}

/// Render the on-disk filename for a catalog entry on the current host
/// (e.g. `mednafen_pce_fast_libretro.dll` on Windows).
fn core_filename_for_host(base: &str) -> String {
    format!("{base}.{}", dylib_ext())
}

/// Resolve the buildbot URL prefix for the running host + arch. Returns
/// `None` for unsupported combinations (the catalog still renders; the
/// install button just refuses with a clean error).
fn buildbot_path_segment() -> Option<&'static str> {
    let arch = std::env::consts::ARCH;
    if cfg!(target_os = "windows") && arch == "x86_64" {
        Some("windows/x86_64")
    } else if cfg!(target_os = "linux") && arch == "x86_64" {
        Some("linux/x86_64")
    } else if cfg!(target_os = "macos") && arch == "x86_64" {
        Some("apple/osx/x86_64")
    } else if cfg!(target_os = "macos") && (arch == "aarch64" || arch == "arm64") {
        Some("apple/osx/arm64")
    } else {
        None
    }
}

/// Final buildbot URL for a catalog `base`. None when the platform isn't
/// in [`buildbot_path_segment`].
///
/// Note: every catalog `base` already includes the `_libretro` suffix
/// (e.g. `mednafen_pce_fast_libretro`). The URL is just `{base}.{ext}.zip`,
/// not `{base}_libretro.{ext}.zip` — the latter produced double-libretro
/// 404s for every entry until 2026-05-19.
fn buildbot_url_for(base: &str) -> Option<String> {
    let segment = buildbot_path_segment()?;
    let ext = dylib_ext();
    Some(format!(
        "https://buildbot.libretro.com/nightly/{segment}/latest/{base}.{ext}.zip",
    ))
}

/// Frontend-visible catalog entry. Combines a [`CatalogEntry`] with
/// installed-state info so the UI can render "Install" vs
/// "Update (current v1.2.3)" in one pass.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AvailableCore {
    pub base: String,
    pub file_name: String,
    pub display_name: String,
    pub blurb: String,
    pub systems: Vec<String>,
    pub installed: bool,
    /// Installed library version (from `retro_get_system_info`), if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_version: Option<String>,
    /// Whether the buildbot supports the current OS+ARCH at all. When
    /// false, the UI greys out the install button.
    pub supported_on_host: bool,
    /// The URL the installer will hit. Surfaced for diagnostics only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buildbot_url: Option<String>,
    /// True for the OA-tested / first-pick core for its system(s). The
    /// frontend renders a "recommended" chip when set.
    pub recommended: bool,
    /// BIOS / firmware filename(s) the core requires under
    /// `<exe_dir>/system/`. Surfaced as a warning chip before install.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bios_required: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct CoreDownloadProgress<'a> {
    file_name: &'a str,
    downloaded_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_bytes: Option<u64>,
    /// `"downloading"`, `"extracting"`, `"done"`, `"error"`.
    phase: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<&'a str>,
}

fn emit_progress(
    app: &AppHandle,
    file_name: &str,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
    phase: &str,
    message: Option<&str>,
) {
    let payload = CoreDownloadProgress {
        file_name,
        downloaded_bytes,
        total_bytes,
        phase,
        message,
    };
    if let Err(e) = app.emit("oa://core-download-progress", payload) {
        log::warn!("core_installer: emit progress failed: {e:?}");
    }
}

/// Walk `<exe_dir>/cores/` and probe every dylib. Returns a filename →
/// (version, valid_extensions) map so [`available_cores`] can stamp
/// "installed (v…)" chips.
fn installed_index(cores_dir: &Path) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    let Ok(rd) = std::fs::read_dir(cores_dir) else { return out };
    let ext = dylib_ext();
    for entry in rd.flatten() {
        let p = entry.path();
        if !p.is_file() { continue; }
        if p.extension().and_then(|s| s.to_str()) != Some(ext) { continue; }
        let Some(name) = p.file_name().and_then(|s| s.to_str()) else { continue };
        match oa_libretro::probe(&p) {
            Ok(info) => { out.insert(name.to_string(), info.library_version); }
            Err(_) => { out.insert(name.to_string(), String::new()); }
        }
    }
    out
}

/// Catalog merged with installed-state. The frontend renders this
/// directly — one row per catalog entry, with Install / Update buttons
/// driven by `installed` + `installed_version`.
#[tauri::command]
pub fn available_cores(cores_dir: tauri::State<'_, CoresDir>) -> Vec<AvailableCore> {
    let installed = installed_index(&cores_dir.0);
    let host_supported = buildbot_path_segment().is_some();
    CATALOG
        .iter()
        .map(|c| {
            let file_name = core_filename_for_host(c.base);
            let installed_version = installed.get(&file_name).cloned();
            AvailableCore {
                base: c.base.to_string(),
                file_name: file_name.clone(),
                display_name: c.display_name.to_string(),
                blurb: c.blurb.to_string(),
                systems: c.systems.iter().map(|s| s.to_string()).collect(),
                installed: installed.contains_key(&file_name),
                installed_version: installed_version.filter(|v| !v.is_empty()),
                supported_on_host: host_supported,
                buildbot_url: buildbot_url_for(c.base),
                recommended: c.recommended,
                bios_required: c.bios_required.map(|s| s.to_string()),
            }
        })
        .collect()
}

/// Tauri-managed state holder so the commands don't have to call
/// `resolve_cores_dir` on every invoke. Set up once at app start.
pub struct CoresDir(pub PathBuf);

/// Catalog lookup — returns the human display_name for `base` when
/// CATALOG has a row for it, falling back to the bare base for ad-hoc
/// downloads (e.g. core slugs the catalog doesn't carry yet). Only
/// used to shape the JobRegistry label, so a fallback to base is
/// acceptable.
fn catalog_display_name_for(base: &str) -> &str {
    CATALOG
        .iter()
        .find(|e| e.base == base)
        .map(|e| e.display_name)
        .unwrap_or(base)
}

/// Download the buildbot nightly zip for `base`, extract the single
/// dylib inside, validate it via libretro probe, and place it at
/// `<exe_dir>/cores/<base>.<ext>`. Emits `oa://core-download-progress`
/// throughout. If a file with the same name already exists, it gets
/// overwritten (Update path).
///
/// Phase 1 of the background-jobs arc wires this as the pilot kind.
/// On every invocation we register a `core_download` job in the
/// JobRegistry (if the state is managed — soft-fail otherwise),
/// poll the JobHandle's cancel/pause flags from inside the chunk
/// loop, and emit per-chunk progress through the registry's debounced
/// `progress()` call. The existing `oa://core-download-progress`
/// emits stay intact for Guided Setup's listener (see plan §"Out of
/// scope" — today's event protocols emit unchanged). The completion /
/// failure / cancel finalizers run in one place at the bottom of the
/// function, regardless of which error path took us out.
#[tauri::command]
#[allow(non_snake_case)]
pub async fn download_core(
    base: String,
    // Phase 4b — when invoked from Guided Setup's MissingCoreBulkPrompt,
    // the frontend creates a `bulk_core_install` parent first via
    // `start_bulk_core_install` and passes its id as parentJobId so the
    // bar shows a parent row aggregating the N children. Standalone
    // download_core invocations (right-click "Install core…", etc.)
    // omit it; the resulting CoreDownload row sits at the top level.
    parentJobId: Option<i64>,
    cores_dir: tauri::State<'_, CoresDir>,
    app: AppHandle,
) -> Result<String, String> {
    use crate::job_registry::{JobKind, JobRegistry, JobScope};

    let dest_dir = cores_dir.0.clone();

    // === Background-jobs registration via JobScope (soft-fail). ===
    // -1 sentinel from start_bulk_core_install means the registry
    // wasn't managed (bar shows nothing) — treat as None.
    let parent_id_normalized = parentJobId.filter(|id| *id > 0);
    let registry_state = app.try_state::<JobRegistry>();
    let scope = JobScope::start(
        registry_state.as_deref(),
        JobKind::CoreDownload { base: base.clone() },
        format!("Downloading {}", catalog_display_name_for(&base)),
        None,
        parent_id_normalized,
        false,
        "bytes",
        None,
    );
    let job_id = scope.id();
    let handle = job_id.and_then(|id| registry_state.as_ref().and_then(|s| s.handle(id)));

    // === Run the actual download/extract/install via the shared inner
    //     fn. JobScope's Drop catches any `?`-early-return inside
    //     run_download_core_inner so the row never leaks `running`. ===
    let result = run_download_core_inner(
        &base,
        &dest_dir,
        &app,
        registry_state.as_deref(),
        job_id,
        handle.as_ref(),
    )
    .await;

    // === Finalize the job row through the scope. ===
    let was_cancelled = handle.as_ref().is_some_and(|h| h.is_cancelled());
    match (&result, was_cancelled) {
        (Ok(_), _) => scope.complete(),
        (Err(_), true) => {
            // Per-kind cancel-cleanup contract (plan §"Cancel cleanup"):
            // core_download cancel = drop the partial download state so
            // the next attempt starts fresh. Phase 3b: now TWO partials
            // to clean — the streaming .zip.partial holds the in-flight
            // zip bytes, the .dll.partial only exists if extract ran
            // (post-chunk-loop). Cancel during the chunk loop only has
            // the former; cancel during extract has both.
            let ext = dylib_ext();
            let file_name = core_filename_for_host(&base);
            let zip_partial_path = dest_dir
                .join(&file_name)
                .with_extension(format!("{ext}.zip.partial"));
            let dll_partial_path = dest_dir
                .join(&file_name)
                .with_extension(format!("{ext}.partial"));
            let _ = std::fs::remove_file(&zip_partial_path);
            let _ = std::fs::remove_file(&dll_partial_path);
            scope.cancel();
        }
        (Err(e), false) => scope.fail(e.clone()),
    }

    result
}

/// Phase 4c — sleep for `total_ms` while polling the cancel flag
/// every 100 ms so an operator-triggered cancel during the retry
/// backoff takes effect within the current tick. Returns true on
/// natural completion, false if the cancel flag flipped during the
/// sleep.
async fn sleep_with_cancel_check(
    handle: Option<&crate::job_registry::JobHandle>,
    total_ms: u64,
) -> bool {
    let mut remaining = total_ms;
    while remaining > 0 {
        if let Some(h) = handle {
            if h.is_cancelled() {
                return false;
            }
        }
        let tick = remaining.min(100);
        tokio::time::sleep(Duration::from_millis(tick)).await;
        remaining = remaining.saturating_sub(tick);
    }
    true
}

/// Shared download/extract/install body. Both `download_core` (the
/// Tauri command — creates a fresh job row) and `CoreDownloadResumer`
/// (Phase 3a auto-resume — attaches to an existing interrupted job
/// row) call this. Keeps the ~200-line flow + cancel/pause polling
/// + state-bridge in one place.
///
/// `registry` / `job_id` / `handle` are all Option so the soft-fail
/// path through `download_core` still works when JobRegistry isn't
/// managed at startup. The resumer always passes Some for all three.
async fn run_download_core_inner(
    base: &str,
    dest_dir: &std::path::Path,
    app: &AppHandle,
    registry: Option<&crate::job_registry::JobRegistry>,
    job_id: Option<i64>,
    handle: Option<&crate::job_registry::JobHandle>,
) -> Result<String, String> {
    use tokio::io::AsyncWriteExt;

    let url = buildbot_url_for(base).ok_or_else(|| {
        format!(
            "buildbot has no build for this OS/ARCH ({}/{})",
            std::env::consts::OS,
            std::env::consts::ARCH,
        )
    })?;
    let file_name = core_filename_for_host(base);
    let final_path = dest_dir.join(&file_name);
    let ext = dylib_ext();

    // Phase 3b — stream .zip bytes through to a `.zip.partial` file as
    // they arrive (replaces the pre-3b in-RAM Vec<u8> buffer). On
    // restart, the partial's size tells us the resume point; we send
    // an HTTP Range request for the remainder. The .dll.partial step
    // (extract + write) stays at the END so the dll only ever lands
    // on disk after the full zip is verified.
    let zip_partial_path = dest_dir
        .join(&file_name)
        .with_extension(format!("{ext}.zip.partial"));
    let dll_partial_path = final_path.with_extension(format!("{ext}.partial"));

    if let Err(e) = std::fs::create_dir_all(dest_dir) {
        return Err(format!("create cores dir {}: {e}", dest_dir.display()));
    }

    // Existing .zip.partial → potential resume. Read its size up
    // front; if non-zero, append + send Range. If 0 / missing, fresh
    // download.
    let existing_size: u64 = std::fs::metadata(&zip_partial_path)
        .map(|m| m.len())
        .unwrap_or(0);
    if existing_size > 0 {
        log::info!(
            "core_installer: resuming download of {} ({} bytes already on disk)",
            base,
            existing_size
        );
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(180))
        .build()
        .map_err(|e| format!("reqwest client: {e}"))?;

    emit_progress(app, &file_name, existing_size, None, "downloading", None);

    // Phase 4c — retry policy. Plan §"Failure handling": 3 attempts
    // with 1s/5s/30s backoff on 5xx + network errors; permanent
    // 4xx errors (other than 416 which is handled specially below)
    // and the success codes (2xx, 206 partial content) return
    // immediately. Cancel/pause flags are polled between attempts so
    // an operator-triggered abort during the backoff sleep takes
    // effect within the current sleep tick.
    const BACKOFFS_MS: [u64; 3] = [1_000, 5_000, 30_000];
    let resp = {
        let mut attempt: usize = 0;
        loop {
            // Build the request fresh each attempt — reqwest's
            // RequestBuilder isn't Clone in older versions and the
            // overhead of rebuilding is nil.
            let mut req = client
                .get(&url)
                .header("User-Agent", "OverlookedArcade");
            if existing_size > 0 {
                req = req.header("Range", format!("bytes={}-", existing_size));
            }
            match req.send().await {
                Ok(r) => {
                    let status = r.status();
                    if status.is_success()
                        || status == reqwest::StatusCode::PARTIAL_CONTENT
                        || status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE
                        || status.is_client_error()
                    {
                        // Success or definitive failure — stop retrying.
                        // (4xx non-416 will surface as the permanent
                        // !is_success error below; 416 has its own
                        // branch right after; 2xx + 206 are success.)
                        break r;
                    }
                    // 5xx (server error). Retry if we have attempts left.
                    if attempt + 1 >= BACKOFFS_MS.len() + 1 {
                        emit_progress(
                            app,
                            &file_name,
                            existing_size,
                            None,
                            "error",
                            Some(&format!("HTTP {status} after {} attempts", attempt + 1)),
                        );
                        return Err(format!(
                            "buildbot HTTP {status} for {url} after {} attempts",
                            attempt + 1
                        ));
                    }
                    let sleep_ms = BACKOFFS_MS[attempt];
                    log::warn!(
                        "core_installer: HTTP {status} for {base} (attempt {}/{}); retrying in {}s",
                        attempt + 1,
                        BACKOFFS_MS.len() + 1,
                        sleep_ms / 1000
                    );
                    if !sleep_with_cancel_check(handle, sleep_ms).await {
                        return Err("cancelled".into());
                    }
                    attempt += 1;
                }
                Err(e) => {
                    // Network error — retry.
                    if attempt + 1 >= BACKOFFS_MS.len() + 1 {
                        emit_progress(
                            app,
                            &file_name,
                            existing_size,
                            None,
                            "error",
                            Some(&format!("{e}")),
                        );
                        return Err(format!(
                            "GET {url}: {e} (after {} attempts)",
                            attempt + 1
                        ));
                    }
                    let sleep_ms = BACKOFFS_MS[attempt];
                    log::warn!(
                        "core_installer: network error for {base} (attempt {}/{}): {e}; retrying in {}s",
                        attempt + 1,
                        BACKOFFS_MS.len() + 1,
                        sleep_ms / 1000
                    );
                    if !sleep_with_cancel_check(handle, sleep_ms).await {
                        return Err("cancelled".into());
                    }
                    attempt += 1;
                }
            }
        }
    };

    // 416 = our partial is bigger than the server's current file (the
    // upstream zip was updated or our partial is corrupt). Drop the
    // partial and ask the operator to retry; recursion in async is
    // awkward and a stale partial is rare enough to surface explicitly.
    if resp.status() == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
        log::warn!(
            "core_installer: server returned 416 for {}; clearing stale partial",
            base
        );
        let _ = std::fs::remove_file(&zip_partial_path);
        return Err(
            "downloaded partial is stale (server file changed); retry to start fresh".into(),
        );
    }
    if !resp.status().is_success() {
        let status = resp.status();
        emit_progress(
            app,
            &file_name,
            existing_size,
            None,
            "error",
            Some(&format!("HTTP {status}")),
        );
        return Err(format!("buildbot HTTP {status} for {url}"));
    }

    // 206 = resume worked; the response body is the remainder.
    // 200 = server ignored Range (or we didn't send one); body is the
    //       whole file from byte 0 — truncate our existing partial.
    let resumed = resp.status() == reqwest::StatusCode::PARTIAL_CONTENT;
    let response_content_length = resp.content_length();
    // Full file size = remainder + already-downloaded (if resumed),
    // or just remainder (if not resumed since it's the whole file).
    let total: Option<u64> = match (resumed, response_content_length) {
        (true, Some(remaining)) => Some(existing_size + remaining),
        (false, Some(len)) => Some(len),
        (_, None) => None,
    };
    let mut downloaded: u64 = if resumed { existing_size } else { 0 };
    if !resumed && existing_size > 0 {
        log::info!(
            "core_installer: server didn't honor Range for {} (status {}); restarting from 0",
            base,
            resp.status()
        );
    }

    let mut file = if resumed {
        tokio::fs::OpenOptions::new()
            .append(true)
            .open(&zip_partial_path)
            .await
            .map_err(|e| format!("open {} for append: {e}", zip_partial_path.display()))?
    } else {
        tokio::fs::File::create(&zip_partial_path)
            .await
            .map_err(|e| format!("create {}: {e}", zip_partial_path.display()))?
    };

    use futures::StreamExt;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        if let Some(h) = handle {
            if h.is_cancelled() {
                // Flush partial state before returning so the resumer
                // can pick up from the latest byte boundary.
                let _ = file.flush().await;
                return Err("cancelled".into());
            }
            let mut was_paused = false;
            while h.is_paused() {
                if !was_paused {
                    was_paused = true;
                    if let (Some(state), Some(id)) = (registry, job_id) {
                        let _ = state.mark_paused(id);
                    }
                    // Flush partial so a kill-during-pause still
                    // resumes cleanly from this point.
                    let _ = file.flush().await;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
                if h.is_cancelled() {
                    let _ = file.flush().await;
                    return Err("cancelled".into());
                }
            }
            if was_paused {
                if let (Some(state), Some(id)) = (registry, job_id) {
                    let _ = state.mark_running(id);
                }
            }
        }
        let chunk = chunk.map_err(|e| {
            emit_progress(
                app,
                &file_name,
                downloaded,
                total,
                "error",
                Some(&format!("{e}")),
            );
            format!("download chunk: {e}")
        })?;
        file.write_all(&chunk).await.map_err(|e| {
            emit_progress(
                app,
                &file_name,
                downloaded,
                total,
                "error",
                Some(&format!("{e}")),
            );
            format!("write {}: {e}", zip_partial_path.display())
        })?;
        downloaded += chunk.len() as u64;
        emit_progress(app, &file_name, downloaded, total, "downloading", None);
        if let (Some(state), Some(id)) = (registry, job_id) {
            let _ = state.progress(id, downloaded as i64, total.map(|t| t as i64));
        }
    }
    file.flush()
        .await
        .map_err(|e| format!("flush {}: {e}", zip_partial_path.display()))?;
    drop(file);

    emit_progress(app, &file_name, downloaded, total, "extracting", None);

    // Read the completed zip back from disk for extraction. (A
    // streaming extractor would let us skip this read, but the .zip
    // format requires the central directory at the END of the file —
    // which we can't process until the download finishes anyway.)
    let zip_bytes = std::fs::read(&zip_partial_path).map_err(|e| {
        emit_progress(
            app,
            &file_name,
            downloaded,
            total,
            "error",
            Some(&format!("{e}")),
        );
        format!("read {}: {e}", zip_partial_path.display())
    })?;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&zip_bytes)).map_err(|e| {
        emit_progress(
            app,
            &file_name,
            downloaded,
            total,
            "error",
            Some(&format!("{e}")),
        );
        format!("open zip: {e}")
    })?;
    let mut payload: Option<Vec<u8>> = None;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("read zip entry {i}: {e}"))?;
        if !entry.is_file() {
            continue;
        }
        let name = entry.name().to_string();
        if !name.to_ascii_lowercase().ends_with(&format!(".{ext}")) {
            continue;
        }
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry
            .read_to_end(&mut buf)
            .map_err(|e| format!("read zip body {name}: {e}"))?;
        payload = Some(buf);
        break;
    }
    let Some(payload) = payload else {
        emit_progress(
            app,
            &file_name,
            downloaded,
            total,
            "error",
            Some("zip had no .{ext} entry"),
        );
        return Err(format!("zip {url} contained no .{ext} entry"));
    };

    std::fs::write(&dll_partial_path, &payload).map_err(|e| {
        emit_progress(
            app,
            &file_name,
            downloaded,
            total,
            "error",
            Some(&format!("{e}")),
        );
        format!("write {}: {e}", dll_partial_path.display())
    })?;

    if let Err(e) = oa_libretro::probe(&dll_partial_path) {
        let _ = std::fs::remove_file(&dll_partial_path);
        emit_progress(
            app,
            &file_name,
            downloaded,
            total,
            "error",
            Some(&format!("probe: {e}")),
        );
        return Err(format!("downloaded file failed libretro probe: {e}"));
    }

    if final_path.exists() {
        if let Err(e) = std::fs::remove_file(&final_path) {
            emit_progress(
                app,
                &file_name,
                downloaded,
                total,
                "error",
                Some("existing core in use; restart Overlooked Arcade and retry"),
            );
            return Err(format!(
                "remove existing {}: {e} (likely still loaded by the running process; restart and retry)",
                final_path.display(),
            ));
        }
    }
    if let Err(e) = std::fs::rename(&dll_partial_path, &final_path) {
        let _ = std::fs::remove_file(&dll_partial_path);
        emit_progress(
            app,
            &file_name,
            downloaded,
            total,
            "error",
            Some(&format!("{e}")),
        );
        return Err(format!("rename to {}: {e}", final_path.display()));
    }

    // Success — drop the zip.partial since we're done with it. (The
    // dll.partial got moved to final_path via rename.)
    let _ = std::fs::remove_file(&zip_partial_path);

    emit_progress(app, &file_name, downloaded, total, "done", None);
    log::info!(
        "core_installer: installed {} from {url}",
        final_path.display()
    );
    Ok(final_path.to_string_lossy().into_owned())
}

/// Resumer for `core_download` jobs. Registered in
/// `main.rs::setup()` after the JobRegistry lands in Tauri state and
/// before `resume_interrupted_jobs` dispatches. The lock-file
/// detection promotes any `state='running'` rows from the previous
/// crashed run to `state='interrupted'`; the dispatcher then routes
/// each to this struct's `resume`.
///
/// Phase 3b strategy: **byte-level Range resume.** The streaming
/// chunk loop in `run_download_core_inner` writes through to
/// `<base>.dll.zip.partial` as bytes arrive — so a crashed run
/// leaves an on-disk partial we can resume from. On resume the
/// inner reads `metadata.len()` of the partial, sends `Range:
/// bytes={size}-`, and appends the response body. The Phase 3a
/// restart-from-zero fallback (drop the partial up front) is gone;
/// the inner handles 416 Range Not Satisfiable + 200 Forced Restart
/// edge cases internally.
pub struct CoreDownloadResumer {
    cores_dir: PathBuf,
}

impl CoreDownloadResumer {
    pub fn new(cores_dir: PathBuf) -> Self {
        Self { cores_dir }
    }
}

impl crate::job_registry::JobResumer for CoreDownloadResumer {
    fn kind(&self) -> &'static str {
        "core_download"
    }

    fn resume(
        &self,
        snapshot: crate::job_registry::JobSnapshot,
        registry: crate::job_registry::JobRegistry,
        app: AppHandle,
    ) -> tauri::async_runtime::JoinHandle<()> {
        let cores_dir = self.cores_dir.clone();
        tauri::async_runtime::spawn(async move {
            let Some(base) = snapshot.target_id.clone() else {
                log::warn!(
                    "background_jobs: core_download resume for job {} missing target_id; marking failed",
                    snapshot.id
                );
                let _ = registry
                    .mark_failed(snapshot.id, "missing target_id on resume".into());
                return;
            };

            // Phase 3b: do NOT drop the .zip.partial. The inner reads
            // its size and sends Range. (Phase 3a dropped it for
            // restart-from-zero; Phase 3b's whole point is to skip
            // the re-download cost.)
            //
            // The .dll.partial — the EXTRACTED dll bytes written
            // post-chunk-loop — is a different file. If it exists,
            // it means the previous run got past the chunk loop and
            // started extracting; drop it because a fresh extract
            // happens after the (Range-resumed) zip completes.
            let ext = dylib_ext();
            let file_name = core_filename_for_host(&base);
            let dll_partial_path = cores_dir
                .join(&file_name)
                .with_extension(format!("{ext}.partial"));
            let _ = std::fs::remove_file(&dll_partial_path);

            // Re-attach via JobScope::resume so the worker gets the
            // same Drop-on-`?` protection download_core has, AND the
            // bar can route Pause / Cancel into the resumed worker
            // through the freshly-attached handle.
            let scope = crate::job_registry::JobScope::resume(
                Some(&registry),
                snapshot.id,
                "core_download".to_string(),
                None,
            );
            let handle = registry
                .handle(snapshot.id)
                .expect("attach_handle just inserted it");

            let result = run_download_core_inner(
                &base,
                &cores_dir,
                &app,
                Some(&registry),
                Some(snapshot.id),
                Some(&handle),
            )
            .await;

            // Finalize — identical shape to download_core's tail.
            let was_cancelled = handle.is_cancelled();
            match (&result, was_cancelled) {
                (Ok(_), _) => scope.complete(),
                (Err(_), true) => {
                    // Cancel cleanup — drop both partials.
                    let zip_partial_path = cores_dir
                        .join(&file_name)
                        .with_extension(format!("{ext}.zip.partial"));
                    let _ = std::fs::remove_file(&zip_partial_path);
                    let _ = std::fs::remove_file(&dll_partial_path);
                    scope.cancel();
                }
                (Err(e), false) => scope.fail(e.clone()),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::cpu_tier::CpuTier;

    #[test]
    fn recommended_core_for_tier_distinct_psx_tiers() {
        // PSX is the canonical three-meaningfully-different-cores
        // case. Each tier picks a real winner.
        assert_eq!(
            recommended_core_for_tier("psx", CpuTier::High),
            Some("mednafen_psx_hw_libretro"),
        );
        assert_eq!(
            recommended_core_for_tier("psx", CpuTier::Mid),
            Some("swanstation_libretro"),
        );
        assert_eq!(
            recommended_core_for_tier("psx", CpuTier::Low),
            Some("pcsx_rearmed_libretro"),
        );
    }

    #[test]
    fn recommended_core_for_tier_same_core_for_high_and_mid() {
        // The accuracy-focused core handles High AND Mid; only Low
        // picks the lighter alternative. Don't over-recommend the
        // weak core to operators whose hardware would handle the
        // accurate one with headroom. SNES + N64 + PS2 + NDS follow
        // this pattern.
        assert_eq!(
            recommended_core_for_tier("snes", CpuTier::High),
            Some("bsnes_libretro"),
        );
        assert_eq!(
            recommended_core_for_tier("snes", CpuTier::Mid),
            Some("snes9x_libretro"),
        );
        assert_eq!(
            recommended_core_for_tier("snes", CpuTier::Low),
            Some("snes9x_libretro"),
        );

        for tier in [CpuTier::High, CpuTier::Mid] {
            assert_eq!(
                recommended_core_for_tier("n64", tier),
                Some("mupen64plus_next_libretro"),
            );
            assert_eq!(
                recommended_core_for_tier("ps2", tier),
                Some("pcsx2_libretro"),
            );
            assert_eq!(
                recommended_core_for_tier("nds", tier),
                Some("melonds_libretro"),
            );
        }
        assert_eq!(
            recommended_core_for_tier("n64", CpuTier::Low),
            Some("parallel_n64_libretro"),
        );
        assert_eq!(
            recommended_core_for_tier("ps2", CpuTier::Low),
            Some("play_libretro"),
        );
        assert_eq!(
            recommended_core_for_tier("nds", CpuTier::Low),
            Some("desmume_libretro"),
        );
    }

    #[test]
    fn recommended_core_for_tier_single_core_systems_return_none() {
        // Systems with one obvious pick (TG-16, NES, GB, Lynx, etc.)
        // skip the tier table entirely. Caller falls through to the
        // registry's `default_core_dll_for_system_resolved`.
        for sys in &["tg16", "nes", "gb", "gba", "lynx", "atari7800", "vectrex", "virtualboy"] {
            assert_eq!(
                recommended_core_for_tier(sys, CpuTier::High),
                None,
                "{sys} should fall through to the registry default",
            );
            assert_eq!(recommended_core_for_tier(sys, CpuTier::Low), None);
        }
    }

    #[test]
    fn recommended_core_for_tier_unknown_system_returns_none() {
        assert_eq!(recommended_core_for_tier("not-a-real-system", CpuTier::High), None);
    }

    #[test]
    fn tier_preference_bases_all_exist_in_catalog() {
        // Every base in TIER_PREFERENCES must be a real CATALOG entry —
        // otherwise the runtime would dispatch an install request for a
        // core that doesn't exist on the buildbot. Sanity check the
        // referential integrity at compile time (well, test time).
        let catalog_bases: std::collections::HashSet<&str> =
            CATALOG.iter().map(|e| e.base).collect();
        for pref in TIER_PREFERENCES {
            for (tier_name, base) in
                [("high", pref.high), ("mid", pref.mid), ("low", pref.low)]
            {
                assert!(
                    catalog_bases.contains(base),
                    "{}/{} = {base} not in CATALOG",
                    pref.system_id,
                    tier_name,
                );
            }
        }
    }

    #[test]
    fn buildbot_path_segment_for_current_host() {
        // Just ensure the resolver returns something on the platforms we
        // actually develop on. CI matrix already runs Windows + Linux +
        // macOS (both archs).
        let seg = buildbot_path_segment();
        if cfg!(any(target_os = "windows", target_os = "linux", target_os = "macos")) {
            assert!(seg.is_some(), "no buildbot mapping for {} {}", std::env::consts::OS, std::env::consts::ARCH);
        } else {
            // Other OSes are allowed to be unsupported.
            let _ = seg;
        }
    }

    #[test]
    fn url_for_known_core_contains_buildbot_host() {
        // Skip when the host isn't on the buildbot at all.
        if buildbot_path_segment().is_none() { return; }
        let url = buildbot_url_for("mednafen_pce_fast_libretro").unwrap();
        assert!(url.starts_with("https://buildbot.libretro.com/"), "{url}");
        assert!(url.contains("mednafen_pce_fast_libretro"), "{url}");
        assert!(url.ends_with(".zip"), "{url}");
        // Regression guard (2026-05-19): URL builder used to append
        // `_libretro` to a base that already ends with `_libretro`,
        // producing `mednafen_pce_fast_libretro_libretro.dll.zip` — 404
        // for every entry. Buildbot serves just `{base}.{ext}.zip`.
        assert!(!url.contains("_libretro_libretro"), "double-libretro: {url}");
        let ext = dylib_ext();
        assert!(
            url.ends_with(&format!("mednafen_pce_fast_libretro.{ext}.zip")),
            "expected tail mednafen_pce_fast_libretro.{ext}.zip in {url}"
        );
    }

    #[test]
    fn no_catalog_url_double_libretro() {
        // Every catalog entry should produce a buildbot URL that does
        // NOT contain `_libretro_libretro` — guards against catalog
        // authors accidentally bare-naming a core (omitting the
        // `_libretro` suffix) under the wrong assumption the URL
        // builder adds it.
        if buildbot_path_segment().is_none() { return; }
        for entry in CATALOG {
            let url = buildbot_url_for(entry.base).unwrap();
            assert!(
                !url.contains("_libretro_libretro"),
                "{} produces double-libretro URL: {url}", entry.base
            );
        }
    }

    #[test]
    fn catalog_has_no_duplicate_bases() {
        let mut seen = std::collections::HashSet::new();
        for entry in CATALOG {
            assert!(seen.insert(entry.base), "duplicate base {}", entry.base);
        }
    }
}
