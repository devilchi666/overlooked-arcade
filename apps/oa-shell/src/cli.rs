//! CLI surface for oa-shell.
//!
//! By default oa-shell launches into the library UI. When invoked with a ROM
//! argument (positional or via --rom), it enters *direct-launch* mode: the
//! library chrome is hidden and the shell boots straight into the game. This
//! lets external frontends (LaunchBox, BigBox, EmulationStation) treat
//! oa-shell like any standalone emulator.
//!
//! Precedence:
//!   1. Positional ROM arg or `--rom` flag → direct-launch.
//!   2. `OA_ROM` env var (legacy fallback) → direct-launch with permissive
//!      system inference (unknown ext falls back to `tg16` with a warning,
//!      preserving the existing dev loop).
//!   3. No ROM → library mode (default).

use std::path::{Path, PathBuf};

use clap::Parser;
use serde::Serialize;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "oa-shell",
    version,
    about = "Overlooked Arcade — emulator frontend",
    long_about = "Run with no arguments for the library UI, or pass a ROM path \
                  to launch directly into the game (LaunchBox / BigBox / \
                  EmulationStation compatible)."
)]
pub struct Cli {
    /// Positional ROM path (LaunchBox/EmulationStation compat).
    #[arg(value_name = "ROM_PATH")]
    pub rom_positional: Option<PathBuf>,

    /// Explicit ROM path. Alternative to the positional form.
    #[arg(long, value_name = "PATH")]
    pub rom: Option<PathBuf>,

    /// Override the libretro core .dll filename (resolved against <exe_dir>/cores/).
    #[arg(long, value_name = "DLL")]
    pub core: Option<String>,

    /// Force system slug when extension is ambiguous (.cue/.chd/.iso/.m3u/.pbp/.zip).
    /// e.g. --system tg16, --system psx, --system saturn
    #[arg(long, value_name = "SLUG")]
    pub system: Option<String>,

    /// Save-state slot to restore after the ROM loads (0-9).
    #[arg(long, value_parser = clap::value_parser!(u32).range(0..=9))]
    pub slot: Option<u32>,

    /// Restore a save-state file directly (full path; overrides --slot).
    #[arg(long, value_name = "PATH")]
    pub state_file: Option<PathBuf>,

    /// Play back a TAS replay .oatas file at launch.
    #[arg(long, value_name = "PATH")]
    pub tas_replay: Option<PathBuf>,

    /// Force fullscreen window mode at launch.
    #[arg(long)]
    pub fullscreen: bool,
}

/// Resolved direct-launch configuration. `None` on AppState means library mode.
#[derive(Debug, Clone)]
pub struct DirectLaunchConfig {
    pub rom_path: PathBuf,
    pub system_id: String,
    pub core_override: Option<String>,
    pub slot: Option<u32>,
    pub state_file: Option<PathBuf>,
    pub tas_replay: Option<PathBuf>,
    pub fullscreen: bool,
    /// Library-DB row id when the ROM's SHA-1 matched an existing entry.
    /// Populated by Phase D hash-lookup; None in Phase A.
    pub matched_entry_id: Option<String>,
}

/// camelCase mirror sent to the frontend by `get_direct_launch_config`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectLaunchConfigDto {
    pub rom_path: String,
    pub system_id: String,
    pub core_override: Option<String>,
    pub slot: Option<u32>,
    pub state_file: Option<String>,
    pub tas_replay: Option<String>,
    pub fullscreen: bool,
    pub matched_entry_id: Option<String>,
}

impl From<&DirectLaunchConfig> for DirectLaunchConfigDto {
    fn from(c: &DirectLaunchConfig) -> Self {
        DirectLaunchConfigDto {
            rom_path: c.rom_path.to_string_lossy().to_string(),
            system_id: c.system_id.clone(),
            core_override: c.core_override.clone(),
            slot: c.slot,
            state_file: c.state_file.as_ref().map(|p| p.to_string_lossy().to_string()),
            tas_replay: c.tas_replay.as_ref().map(|p| p.to_string_lossy().to_string()),
            fullscreen: c.fullscreen,
            matched_entry_id: c.matched_entry_id.clone(),
        }
    }
}

#[derive(Debug)]
pub enum CliError {
    Conflict(&'static str),
    RomFileMissing(PathBuf),
    UnknownExtension(String),
    AmbiguousExtension { ext: String, candidates: Vec<&'static str> },
    UnknownSystem(String),
}

impl CliError {
    /// Print a multi-line banner to stderr, mirroring the shell's existing
    /// "no libretro core found" formatting in main.rs.
    pub fn emit_banner(&self) {
        let (heading, body) = self.banner_body();
        let width = heading.len().max(40);
        let line = "─".repeat(width);
        eprintln!();
        eprintln!("┌─{line}─┐");
        eprintln!(
            "│ {heading}{pad} │",
            pad = " ".repeat(width.saturating_sub(heading.chars().count()))
        );
        eprintln!("└─{line}─┘");
        eprintln!("{body}");
        eprintln!();
    }

    fn banner_body(&self) -> (String, String) {
        match self {
            Self::Conflict(msg) => (
                "oa-shell: conflicting CLI arguments".to_string(),
                (*msg).to_string(),
            ),
            Self::RomFileMissing(p) => (
                "oa-shell: ROM file not found".to_string(),
                format!(
                    "Path: {}\n\nCheck the file path and try again.",
                    p.display()
                ),
            ),
            Self::UnknownExtension(ext) => (
                "oa-shell: unknown ROM extension".to_string(),
                format!(
                    ".{ext} is not a recognized ROM extension. Supply --system \
                     explicitly to override (e.g. --system tg16)."
                ),
            ),
            Self::AmbiguousExtension { ext, candidates } => (
                "oa-shell: ambiguous ROM extension".to_string(),
                format!(
                    ".{ext} matches multiple systems. Supply --system to choose.\n\
                     Candidates: {}",
                    candidates.join(", ")
                ),
            ),
            Self::UnknownSystem(s) => (
                "oa-shell: unknown system slug".to_string(),
                format!(
                    "--system {s} is not a recognized system. Common values:\n  \
                     tg16, pce-cd, nes, snes, genesis, segacd, saturn, psx, n64,\n  \
                     gba, gb, lynx, atari7800, mame, neogeo, dreamcast, ps2, psp."
                ),
            ),
        }
    }
}

/// Extension → unambiguous system slug. Mirrors `frontend/src/themes/registry.ts`
/// (`SystemTheme.extensions`). CD-shaped extensions (.cue/.chd/.iso/.m3u/.pbp/
/// .zip/.7z) intentionally return `AmbiguousExtension` and require the caller
/// to pass `--system`.
///
/// **Keep in sync with `frontend/src/themes/registry.ts`.**
pub fn infer_system_from_extension(path: &Path) -> Result<&'static str, CliError> {
    let Some(ext_os) = path.extension() else {
        return Err(CliError::UnknownExtension(String::new()));
    };
    let ext = ext_os.to_string_lossy().to_lowercase();
    let slug: Option<&'static str> = match ext.as_str() {
        "pce" => Some("tg16"),
        "lnx" | "lyx" => Some("lynx"),
        "nes" | "fds" | "unf" | "unif" => Some("nes"),
        "sfc" | "smc" | "fig" | "swc" => Some("snes"),
        "a78" => Some("atari7800"),
        "md" | "smd" | "gen" | "68k" => Some("genesis"),
        "32x" => Some("sega32x"),
        "n64" | "z64" | "v64" => Some("n64"),
        "nds" => Some("nds"),
        "j64" | "jag" => Some("jaguar"),
        "gb" | "gbc" => Some("gb"),
        "vec" | "gam" => Some("vectrex"),
        "vb" => Some("virtualboy"),
        "ws" | "wsc" => Some("wonderswan"),
        "col" | "cv" => Some("coleco"),
        "int" => Some("intv"),
        "o2" => Some("o2"),
        "chf" => Some("channelf"),
        "a26" => Some("2600"),
        "a52" => Some("5200"),
        "min" => Some("pokemini"),
        "gba" => Some("gba"),
        "sms" => Some("sms"),
        "gg" => Some("gamegear"),
        "ngp" | "ngc" => Some("ngp"),
        "cdi" | "gdi" => Some("dreamcast"),
        "gcm" | "gcz" | "rvz" | "wbfs" => Some("gamecube"),
        "cso" => Some("psp"),
        "neo" => Some("neogeo"),
        _ => None,
    };
    if let Some(s) = slug {
        return Ok(s);
    }

    let ambiguous: Option<Vec<&'static str>> = match ext.as_str() {
        "cue" | "ccd" | "toc" | "m3u" => Some(vec![
            "pce-cd", "segacd", "saturn", "psx", "neocd", "3do", "pcfx",
        ]),
        "chd" => Some(vec![
            "pce-cd", "segacd", "saturn", "psx", "neocd", "3do", "pcfx", "ps2", "dreamcast", "mame",
        ]),
        "iso" => Some(vec![
            "pce-cd", "segacd", "saturn", "psx", "neocd", "3do", "pcfx", "ps2", "psp", "dreamcast",
        ]),
        "pbp" => Some(vec!["psx", "psp"]),
        "zip" | "7z" => Some(vec!["mame", "neogeo"]),
        _ => None,
    };
    if let Some(candidates) = ambiguous {
        return Err(CliError::AmbiguousExtension { ext, candidates });
    }
    Err(CliError::UnknownExtension(ext))
}

/// Permissive check for a user-supplied `--system` slug. Mirrors the slug set
/// in `parse_system_id` (main.rs) — keep in sync.
fn is_known_system(slug: &str) -> bool {
    matches!(
        slug,
        "tg16"
            | "pce"
            | "pce-cd"
            | "pcecd"
            | "lynx"
            | "nes"
            | "snes"
            | "atari7800"
            | "genesis"
            | "sega32x"
            | "segacd"
            | "saturn"
            | "n64"
            | "nds"
            | "jaguar"
            | "gb"
            | "gbc"
            | "vectrex"
            | "virtualboy"
            | "wonderswan"
            | "coleco"
            | "intv"
            | "o2"
            | "channelf"
            | "2600"
            | "5200"
            | "pokemini"
            | "gba"
            | "sms"
            | "gamegear"
            | "ngp"
            | "ngpc"
            | "dreamcast"
            | "gamecube"
            | "psp"
            | "psx"
            | "ps2"
            | "neogeo"
            | "neocd"
            | "3do"
            | "pcfx"
            | "mame"
    )
}

/// Parse argv via clap and resolve a direct-launch config when a ROM is
/// supplied. `Ok(None)` = library mode. Errors should be banner-printed by
/// the caller and exit with status 2.
pub fn parse_and_resolve() -> Result<Option<DirectLaunchConfig>, CliError> {
    let cli = Cli::parse();
    resolve(cli)
}

fn resolve(cli: Cli) -> Result<Option<DirectLaunchConfig>, CliError> {
    let rom_path = match (cli.rom_positional.clone(), cli.rom.clone()) {
        (Some(_), Some(_)) => {
            return Err(CliError::Conflict(
                "Specify either a positional ROM path or --rom, not both.",
            ));
        }
        (Some(p), None) => p,
        (None, Some(p)) => p,
        (None, None) => return Ok(None),
    };

    // Validate user-supplied --system BEFORE touching the filesystem. The
    // slug error is more upstream than file-missing — fix the typo first.
    let system_id = match cli.system.as_deref() {
        Some(slug) => {
            if !is_known_system(slug) {
                return Err(CliError::UnknownSystem(slug.to_string()));
            }
            slug.to_string()
        }
        None => infer_system_from_extension(&rom_path)?.to_string(),
    };

    if !rom_path.exists() {
        return Err(CliError::RomFileMissing(rom_path));
    }

    Ok(Some(DirectLaunchConfig {
        rom_path,
        system_id,
        core_override: cli.core,
        slot: cli.slot,
        state_file: cli.state_file,
        tas_replay: cli.tas_replay,
        fullscreen: cli.fullscreen,
        matched_entry_id: None,
    }))
}

/// Legacy `OA_ROM` env-var fallback. Permissive — an unknown / ambiguous
/// extension logs a warning and defaults to `tg16` rather than aborting,
/// preserving the existing dev loop.
pub fn from_oa_rom_env(rom: &str) -> DirectLaunchConfig {
    let rom_path = PathBuf::from(rom);
    let system_id = match infer_system_from_extension(&rom_path) {
        Ok(slug) => slug.to_string(),
        Err(_) => {
            log::warn!(
                "oa-shell: OA_ROM={rom} has unknown/ambiguous extension; defaulting to tg16. \
                 Pass an OA_ROM path with a recognized extension or use the CLI args + --system."
            );
            "tg16".to_string()
        }
    };
    DirectLaunchConfig {
        rom_path,
        system_id,
        core_override: None,
        slot: None,
        state_file: None,
        tas_replay: None,
        fullscreen: false,
        matched_entry_id: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_unambiguous_carts() {
        assert_eq!(infer_system_from_extension(Path::new("foo.nes")).unwrap(), "nes");
        assert_eq!(infer_system_from_extension(Path::new("foo.pce")).unwrap(), "tg16");
        assert_eq!(infer_system_from_extension(Path::new("foo.sfc")).unwrap(), "snes");
        assert_eq!(infer_system_from_extension(Path::new("foo.A78")).unwrap(), "atari7800");
        assert_eq!(infer_system_from_extension(Path::new("foo.GBA")).unwrap(), "gba");
        assert_eq!(infer_system_from_extension(Path::new("foo.lnx")).unwrap(), "lynx");
    }

    #[test]
    fn infer_ambiguous_extensions() {
        let err = infer_system_from_extension(Path::new("foo.cue")).unwrap_err();
        match err {
            CliError::AmbiguousExtension { ext, candidates } => {
                assert_eq!(ext, "cue");
                assert!(candidates.contains(&"psx"));
                assert!(candidates.contains(&"pce-cd"));
            }
            _ => panic!("expected AmbiguousExtension"),
        }
        let err = infer_system_from_extension(Path::new("foo.zip")).unwrap_err();
        match err {
            CliError::AmbiguousExtension { ext, candidates } => {
                assert_eq!(ext, "zip");
                assert!(candidates.contains(&"mame"));
            }
            _ => panic!("expected AmbiguousExtension"),
        }
    }

    #[test]
    fn infer_unknown_extension() {
        let err = infer_system_from_extension(Path::new("foo.txt")).unwrap_err();
        assert!(matches!(err, CliError::UnknownExtension(_)));
        let err = infer_system_from_extension(Path::new("noext")).unwrap_err();
        assert!(matches!(err, CliError::UnknownExtension(_)));
    }

    #[test]
    fn resolve_no_args_returns_none() {
        let cli = Cli {
            rom_positional: None,
            rom: None,
            core: None,
            system: None,
            slot: None,
            state_file: None,
            tas_replay: None,
            fullscreen: false,
        };
        assert!(resolve(cli).unwrap().is_none());
    }

    #[test]
    fn resolve_both_rom_forms_conflicts() {
        let cli = Cli {
            rom_positional: Some(PathBuf::from("a.nes")),
            rom: Some(PathBuf::from("b.nes")),
            core: None,
            system: None,
            slot: None,
            state_file: None,
            tas_replay: None,
            fullscreen: false,
        };
        assert!(matches!(resolve(cli).unwrap_err(), CliError::Conflict(_)));
    }

    #[test]
    fn resolve_missing_rom_file() {
        let cli = Cli {
            rom_positional: Some(PathBuf::from("definitely-does-not-exist.nes")),
            rom: None,
            core: None,
            system: None,
            slot: None,
            state_file: None,
            tas_replay: None,
            fullscreen: false,
        };
        assert!(matches!(
            resolve(cli).unwrap_err(),
            CliError::RomFileMissing(_)
        ));
    }

    #[test]
    fn resolve_unknown_system_slug() {
        let cli = Cli {
            rom_positional: None,
            rom: Some(PathBuf::from("foo.nes")),
            core: None,
            system: Some("nonsense".to_string()),
            slot: None,
            state_file: None,
            tas_replay: None,
            fullscreen: false,
        };
        assert!(matches!(
            resolve(cli).unwrap_err(),
            CliError::UnknownSystem(_)
        ));
    }

    #[test]
    fn from_oa_rom_env_falls_back_to_tg16() {
        let cfg = from_oa_rom_env("/tmp/weird.unknownext");
        assert_eq!(cfg.system_id, "tg16");
        assert!(cfg.core_override.is_none());
        assert!(!cfg.fullscreen);
    }

    #[test]
    fn from_oa_rom_env_recognizes_known_extension() {
        let cfg = from_oa_rom_env("/tmp/Bonk.pce");
        assert_eq!(cfg.system_id, "tg16");
        let cfg = from_oa_rom_env("/tmp/Mario.sfc");
        assert_eq!(cfg.system_id, "snes");
    }
}
