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
    use crate::job_registry::{JobKind, JobRegistry};

    let dest_dir = cores_dir.0.clone();

    // === Background-jobs registration (soft-fail). ===
    // -1 sentinel from start_bulk_core_install means the registry
    // wasn't managed (bar shows nothing) — treat as None.
    let parent_id_normalized = parentJobId.filter(|id| *id > 0);
    let registry_state = app.try_state::<JobRegistry>();
    let job_id: Option<i64> = registry_state.as_ref().and_then(|state| {
        let label = format!("Downloading {}", catalog_display_name_for(&base));
        match state.create_job(
            JobKind::CoreDownload {
                base: base.clone(),
            },
            label,
            None,
            parent_id_normalized,
            false,
            "bytes",
            None,
        ) {
            Ok(id) => {
                let _ = state.mark_running(id);
                Some(id)
            }
            Err(e) => {
                log::warn!("background_jobs: create_job(core_download) failed: {e}");
                None
            }
        }
    });
    let handle = job_id.and_then(|id| registry_state.as_ref().and_then(|s| s.handle(id)));

    // === Run the actual download/extract/install via the shared inner
    //     fn. Wrapped so finalization runs exactly once regardless of
    //     which error path took us out, AND so the resumer (Phase 3a
    //     §CoreDownloadResumer below) can share the same flow without
    //     duplicating ~200 lines of body. ===
    let result = run_download_core_inner(
        &base,
        &dest_dir,
        &app,
        registry_state.as_deref(),
        job_id,
        handle.as_ref(),
    )
    .await;

    // === Finalize the job row. ===
    if let (Some(state), Some(id)) = (registry_state.as_ref(), job_id) {
        let was_cancelled = handle.as_ref().is_some_and(|h| h.is_cancelled());
        match (&result, was_cancelled) {
            (Ok(_), _) => {
                let _ = state.mark_completed(id);
            }
            (Err(_), true) => {
                // Per-kind cancel-cleanup contract (plan §"Cancel
                // cleanup"): core_download cancel = delete the
                // `.partial` file. Re-derive its path here from
                // dest_dir + base (the in-closure `partial_path` is
                // out of scope by now). The remove is best-effort —
                // ENOENT (the more common case if cancel fired during
                // the chunk loop, before extract wrote `.partial`)
                // and any IO error both silently no-op.
                let ext = dylib_ext();
                let file_name = core_filename_for_host(&base);
                let partial_path = dest_dir
                    .join(&file_name)
                    .with_extension(format!("{ext}.partial"));
                let _ = std::fs::remove_file(&partial_path);
                let _ = state.mark_cancelled(id);
            }
            (Err(e), false) => {
                let _ = state.mark_failed(id, e.clone());
            }
        }
    }

    result
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
    let url = buildbot_url_for(base).ok_or_else(|| {
        format!(
            "buildbot has no build for this OS/ARCH ({}/{})",
            std::env::consts::OS,
            std::env::consts::ARCH,
        )
    })?;
    let file_name = core_filename_for_host(base);
    let final_path = dest_dir.join(&file_name);

    if let Err(e) = std::fs::create_dir_all(dest_dir) {
        return Err(format!("create cores dir {}: {e}", dest_dir.display()));
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(180))
        .build()
        .map_err(|e| format!("reqwest client: {e}"))?;

    emit_progress(app, &file_name, 0, None, "downloading", None);

    let resp = client
        .get(&url)
        .header("User-Agent", "OverlookedArcade")
        .send()
        .await
        .map_err(|e| {
            emit_progress(app, &file_name, 0, None, "error", Some(&format!("{e}")));
            format!("GET {url}: {e}")
        })?;
    if !resp.status().is_success() {
        let status = resp.status();
        emit_progress(app, &file_name, 0, None, "error", Some(&format!("HTTP {status}")));
        return Err(format!("buildbot HTTP {status} for {url}"));
    }
    let total = resp.content_length();

    use futures::StreamExt;
    let mut downloaded: u64 = 0;
    let mut zip_bytes: Vec<u8> = Vec::with_capacity(total.unwrap_or(0) as usize);
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        if let Some(h) = handle {
            if h.is_cancelled() {
                return Err("cancelled".into());
            }
            let mut was_paused = false;
            while h.is_paused() {
                if !was_paused {
                    was_paused = true;
                    if let (Some(state), Some(id)) = (registry, job_id) {
                        let _ = state.mark_paused(id);
                    }
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
                if h.is_cancelled() {
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
        zip_bytes.extend_from_slice(&chunk);
        downloaded += chunk.len() as u64;
        emit_progress(app, &file_name, downloaded, total, "downloading", None);
        if let (Some(state), Some(id)) = (registry, job_id) {
            let _ = state.progress(id, downloaded as i64, total.map(|t| t as i64));
        }
    }

    emit_progress(app, &file_name, downloaded, total, "extracting", None);

    let ext = dylib_ext();
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

    let partial_path = final_path.with_extension(format!("{ext}.partial"));
    std::fs::write(&partial_path, &payload).map_err(|e| {
        emit_progress(
            app,
            &file_name,
            downloaded,
            total,
            "error",
            Some(&format!("{e}")),
        );
        format!("write {}: {e}", partial_path.display())
    })?;

    if let Err(e) = oa_libretro::probe(&partial_path) {
        let _ = std::fs::remove_file(&partial_path);
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
    if let Err(e) = std::fs::rename(&partial_path, &final_path) {
        let _ = std::fs::remove_file(&partial_path);
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

    emit_progress(app, &file_name, downloaded, total, "done", None);
    log::info!(
        "core_installer: installed {} from {url}",
        final_path.display()
    );
    Ok(final_path.to_string_lossy().into_owned())
}

/// Phase 3a resumer for `core_download` jobs. Registered in
/// `main.rs::setup()` after the JobRegistry lands in Tauri state and
/// before `resume_interrupted_jobs` dispatches. The lock-file
/// detection promotes any `state='running'` rows from the previous
/// crashed run to `state='interrupted'`; the dispatcher then routes
/// each to this struct's `resume`.
///
/// Phase 3a strategy: **restart from zero.** Drops the leftover
/// `.partial` file (if any) and re-runs the full
/// `run_download_core_inner` flow from the buildbot URL. The
/// existing chunk-loop buffers the entire .zip in RAM before writing
/// .partial; a future Phase 3b will refactor to streaming-write +
/// HTTP Range so resume is byte-level. For Phase 3a a fresh
/// re-download is acceptable — cores are <10 MB and crash recovery
/// is rare enough that the bandwidth tradeoff is fine.
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

            // Phase 3a: drop any leftover .partial from the crashed
            // run. Best-effort; ENOENT is the common case.
            let ext = dylib_ext();
            let file_name = core_filename_for_host(&base);
            let partial_path = cores_dir
                .join(&file_name)
                .with_extension(format!("{ext}.partial"));
            let _ = std::fs::remove_file(&partial_path);

            // Re-attach handle (fresh cancel + pause flags) so the
            // bar can route Pause / Cancel into the resumed worker.
            let handle = registry.attach_handle(snapshot.id, "core_download".to_string());
            if let Err(e) = registry.mark_running(snapshot.id) {
                log::warn!(
                    "background_jobs: mark_running on resumed job {} failed: {e}",
                    snapshot.id
                );
            }

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
                (Ok(_), _) => {
                    let _ = registry.mark_completed(snapshot.id);
                }
                (Err(_), true) => {
                    let _ = std::fs::remove_file(&partial_path);
                    let _ = registry.mark_cancelled(snapshot.id);
                }
                (Err(e), false) => {
                    let _ = registry.mark_failed(snapshot.id, e.clone());
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
