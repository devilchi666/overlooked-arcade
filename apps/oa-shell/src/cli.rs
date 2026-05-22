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

use std::collections::HashSet;
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

    /// Headless maintenance: clear the `metadata` slot on every media
    /// entry whose game is tagged with this system_id, then exit.
    /// Use after the 2026-05-21 metadata-routing fix to scrub
    /// cross-system contamination from libraries that synced under
    /// the old PCE-only wildcard. Does not touch cover-art / snap /
    /// title variants — only the metadata payload (genre / developer
    /// / publisher / year / players).
    #[arg(long, value_name = "SYSTEM_ID")]
    pub clear_metadata: Option<String>,

    /// Force the frontend's presentation mode to Cabinet at startup. Pure
    /// runtime override — the operator's `presentation.json` on disk is
    /// preserved so the next library-mode launch honors it.
    ///
    /// Phase 0 hook for the kiosk shell (UI_POLISH_PLAN.md §E.1 +
    /// KIOSK_PLAN.md). Stand-alone today; pairs cleanly with `--rom` for
    /// the museum / arcade-cabinet boot-into-game flow.
    #[arg(long)]
    pub kiosk: bool,
}

/// Top-level CLI result. Bundles the resolved direct-launch config (if any)
/// with global flags that aren't tied to a specific ROM. `--kiosk` is the
/// first global of this shape — it can be combined with `--rom` or used
/// alone to force the frontend into Cabinet presentation for testing.
#[derive(Debug, Clone)]
pub struct CliConfig {
    pub direct_launch: Option<DirectLaunchConfig>,
    pub kiosk: bool,
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
    /// Populated by Phase D hash-lookup.
    pub matched_entry_id: Option<String>,
    /// Posix-style path inside the archive at `rom_path` when the outer
    /// file is a .zip/.7z that wraps a single cart ROM. `None` means
    /// `rom_path` is the ROM (raw cart / CD image / MAME romset).
    /// Populated by Phase H archive resolution.
    pub archive_inner_path: Option<String>,
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
    pub archive_inner_path: Option<String>,
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
            archive_inner_path: c.archive_inner_path.clone(),
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
    ArchiveEmpty { archive: PathBuf },
    ArchiveMultipleRoms { archive: PathBuf, paths: Vec<String> },
    ArchiveReadFailed { archive: PathBuf, error: String },
    ArchiveInnerNotFound { archive: PathBuf, inner: String, available: Vec<String> },
    StateFileMissing(PathBuf),
}

impl CliError {
    /// Display the error to the user. Always writes a multi-line banner to
    /// stderr (visible in debug builds, in `cargo tauri dev`, and when the
    /// release binary is launched from a console that managed to inherit
    /// stderr). On Windows release builds the binary uses `windows_subsystem
    /// = "windows"` so stderr is silently dropped — fall back to a native
    /// MessageBox so the operator gets visible feedback regardless of how
    /// they spawned the process (cmd, PowerShell, LaunchBox, double-click).
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

        #[cfg(all(target_os = "windows", not(debug_assertions)))]
        win_msgbox::error(&heading, &format!("{heading}\n\n{body}"));
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
            Self::AmbiguousExtension { ext, candidates } => {
                let mut body = format!(
                    ".{ext} matches multiple systems. Supply --system to choose.\n\
                     Candidates: {}",
                    candidates.join(", ")
                );
                // .zip / .7z are also the wrapper around extract-and-launch
                // cart games for systems that don't open archives natively
                // (SNES / NES / GBA / Genesis / etc.). Direct-launch v1
                // doesn't unpack — flag this explicitly so the user knows
                // "--system snes foo.zip" wouldn't work either.
                if ext == "zip" || ext == "7z" {
                    body.push_str(
                        "\n\nNote: direct-launch v1 does NOT auto-extract archives.\n\
                         - For MAME / Neo Geo romsets, --system mame (or neogeo) launches the .zip as-is.\n\
                         - For cart ROMs (SNES / NES / Genesis / etc.) packaged inside .zip,\n  \
                           extract the inner ROM file first and launch that.\n\
                         - Or scan the archive via the Import Wizard; OA hash-matches at launch."
                    );
                }
                ("oa-shell: ambiguous ROM extension".to_string(), body)
            }
            Self::UnknownSystem(s) => (
                "oa-shell: unknown system slug".to_string(),
                format!(
                    "--system {s} is not a recognized system. Common values:\n  \
                     tg16, pce-cd, nes, snes, genesis, segacd, saturn, psx, n64,\n  \
                     gba, gb, lynx, atari7800, mame, neogeo, dreamcast, ps2, psp."
                ),
            ),
            Self::ArchiveEmpty { archive } => (
                "oa-shell: archive contains no ROMs".to_string(),
                format!(
                    "Archive: {}\n\n\
                     No files with recognized cart-ROM extensions inside. \
                     If this is a MAME or Neo Geo romset, pass\n  \
                     --system mame  (or --system neogeo)\n\
                     to launch the archive directly.",
                    archive.display()
                ),
            ),
            Self::ArchiveMultipleRoms { archive, paths } => {
                let listed: String = paths
                    .iter()
                    .take(8)
                    .map(|p| format!("  - {p}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                let more = if paths.len() > 8 {
                    format!("\n  \u{2026} and {} more", paths.len() - 8)
                } else {
                    String::new()
                };
                (
                    "oa-shell: archive contains multiple ROMs".to_string(),
                    format!(
                        "Archive: {}\n\nFound {} ROM files:\n{listed}{more}\n\n\
                         Direct-launch picks single-ROM archives transparently; \
                         multi-ROM archives need to be scanned into the library \
                         first (each inner file gets its own entry — then OA \
                         hash-matches at launch).",
                        archive.display(),
                        paths.len(),
                    ),
                )
            }
            Self::ArchiveReadFailed { archive, error } => (
                "oa-shell: archive read failed".to_string(),
                format!("Archive: {}\n\nError: {error}", archive.display()),
            ),
            Self::ArchiveInnerNotFound { archive, inner, available } => {
                let listed: String = available
                    .iter()
                    .take(8)
                    .map(|p| format!("  - {p}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                let more = if available.len() > 8 {
                    format!("\n  \u{2026} and {} more", available.len() - 8)
                } else {
                    String::new()
                };
                (
                    "oa-shell: inner ROM not found in archive".to_string(),
                    format!(
                        "Archive: {}\nRequested inner: {inner}\n\nAvailable:\n{listed}{more}",
                        archive.display()
                    ),
                )
            }
            Self::StateFileMissing(p) => (
                "oa-shell: --state-file not found".to_string(),
                format!("Path: {}\n\nCheck the file path and try again.", p.display()),
            ),
        }
    }
}

/// Native Windows MessageBox for CLI errors on release builds. main.rs sets
/// `windows_subsystem = "windows"` for release, which detaches stderr — any
/// `eprintln!` is silently dropped, so launcher-spawned and double-click
/// invocations would see "nothing happens" on error without this fallback.
#[cfg(all(target_os = "windows", not(debug_assertions)))]
mod win_msgbox {
    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(
            hwnd: *mut std::ffi::c_void,
            text: *const u16,
            caption: *const u16,
            type_: u32,
        ) -> i32;
    }

    const MB_OK: u32 = 0x0000_0000;
    const MB_ICONERROR: u32 = 0x0000_0010;
    const MB_ICONINFORMATION: u32 = 0x0000_0040;

    fn show(title: &str, message: &str, type_: u32) {
        let title_w: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        let msg_w: Vec<u16> = message.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                msg_w.as_ptr(),
                title_w.as_ptr(),
                MB_OK | type_,
            );
        }
    }

    pub fn error(title: &str, message: &str) {
        show(title, message, MB_ICONERROR);
    }

    pub fn info(title: &str, message: &str) {
        show(title, message, MB_ICONINFORMATION);
    }
}

/// Unambiguous extension → system slug. Mirrors `frontend/src/themes/registry.ts`
/// (`SystemTheme.extensions`). CD-shaped and archive extensions return None —
/// the caller is expected to disambiguate (CD via `--system`; archive via
/// `resolve_archive` which peeks inside).
///
/// **Keep in sync with `frontend/src/themes/registry.ts`.**
fn slug_for_ext(ext: &str) -> Option<&'static str> {
    match ext {
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
    }
}

/// Set of cart-ROM extensions used by `resolve_archive` to filter the
/// contents of a .zip/.7z. Restricted to single-file cart shapes — CD
/// images inside archives need multi-file extract-to-temp handling and
/// are out of scope for direct-launch v1 (Phase H).
pub(crate) fn accepted_rom_extensions() -> HashSet<String> {
    [
        "pce", "lnx", "lyx", "nes", "fds", "unf", "unif", "sfc", "smc", "fig", "swc", "a78", "md",
        "smd", "gen", "68k", "32x", "n64", "z64", "v64", "nds", "j64", "jag", "gb", "gbc", "vec",
        "gam", "vb", "ws", "wsc", "col", "cv", "int", "o2", "chf", "a26", "a52", "min", "gba",
        "sms", "gg", "ngp", "ngc", "neo",
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect()
}

fn is_archive_extension(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .as_deref(),
        Some("zip") | Some("7z")
    )
}

/// Public wrapper retained for tests + future callers. Takes a path,
/// returns the resolved slug or an `AmbiguousExtension` / `UnknownExtension`
/// error. `resolve()` no longer calls this on archive extensions — the
/// archive path is `resolve_archive`.
pub fn infer_system_from_extension(path: &Path) -> Result<&'static str, CliError> {
    let Some(ext_os) = path.extension() else {
        return Err(CliError::UnknownExtension(String::new()));
    };
    let ext = ext_os.to_string_lossy().to_lowercase();
    if let Some(s) = slug_for_ext(&ext) {
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

/// Peek inside a .zip/.7z archive and decide what direct-launch should do.
///
/// Three paths, in priority order:
/// 1. **MAME / Neo Geo passthrough.** If `--system mame` (or `neogeo`) was
///    supplied, treat the archive as the romset itself — no inner extraction.
/// 2. **Neo Geo auto-detection.** If `--system` wasn't supplied AND the
///    archive contains the `.p1` + `.s1` Neo Geo signature, pick `neogeo`
///    automatically.
/// 3. **Single-ROM extraction.** Peek the archive's contents filtered to
///    cart-ROM extensions. Exactly one match → use it (system inferred from
///    the inner extension, or honored from `--system` if supplied). Zero
///    or multiple matches → error.
///
/// Returns `(system_id, archive_inner_path)`. `archive_inner_path` is `None`
/// for MAME / Neo Geo passthrough; `Some(inner)` for single-ROM extraction.
fn resolve_archive(
    rom_path: &Path,
    explicit_system: Option<&str>,
) -> Result<(String, Option<String>), CliError> {
    if matches!(explicit_system, Some("mame") | Some("neogeo")) {
        log::info!(
            "oa-shell: direct-launch archive {} \u{2192} --system {} passthrough",
            rom_path.display(),
            explicit_system.unwrap()
        );
        return Ok((explicit_system.unwrap().to_string(), None));
    }

    if explicit_system.is_none() && crate::archive::peek_zip_for_neogeo(rom_path) {
        log::info!(
            "oa-shell: direct-launch archive {} \u{2192} Neo Geo (p1+s1 signature)",
            rom_path.display()
        );
        return Ok(("neogeo".to_string(), None));
    }

    // Accept both cart-shaped + CD-entry-point extensions when peeking.
    // Cart inners auto-infer the system; CD inners require --system because
    // their formats are ambiguous (.cue could be PCE-CD / segacd / saturn / psx / …).
    let mut accepted = accepted_rom_extensions();
    for ext in ["cue", "ccd", "toc", "m3u"] {
        accepted.insert(ext.to_string());
    }
    let entries =
        crate::archive::list_rom_contents(rom_path, &accepted).map_err(|e| CliError::ArchiveReadFailed {
            archive: rom_path.to_path_buf(),
            error: e,
        })?;

    match entries.len() {
        0 => Err(CliError::ArchiveEmpty {
            archive: rom_path.to_path_buf(),
        }),
        1 => {
            let entry = &entries[0];
            let is_cd = matches!(entry.extension.as_str(), "cue" | "ccd" | "toc" | "m3u");
            let system_id = match explicit_system {
                Some(s) => s.to_string(),
                None => {
                    if is_cd {
                        return Err(CliError::AmbiguousExtension {
                            ext: entry.extension.clone(),
                            candidates: vec![
                                "pce-cd", "segacd", "saturn", "psx", "neocd", "3do", "pcfx",
                            ],
                        });
                    }
                    slug_for_ext(&entry.extension)
                        .ok_or_else(|| CliError::UnknownExtension(entry.extension.clone()))?
                        .to_string()
                }
            };
            log::info!(
                "oa-shell: direct-launch archive {} contains 1 {} ROM: {} (system={})",
                rom_path.display(),
                if is_cd { "CD" } else { "cart" },
                entry.inner_path,
                system_id,
            );
            Ok((system_id, Some(entry.inner_path.clone())))
        }
        _ => Err(CliError::ArchiveMultipleRoms {
            archive: rom_path.to_path_buf(),
            paths: entries.iter().map(|e| e.inner_path.clone()).collect(),
        }),
    }
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

/// Tauri 2 bundle identifier — must match `tauri.conf.json` `identifier`.
/// Used to derive `app_data_dir` in headless maintenance mode (when no
/// Tauri runtime is spun up, so the usual `app.path().app_data_dir()`
/// path resolver isn't available).
const TAURI_IDENTIFIER: &str = "dev.overlookedarcade.shell";

/// Resolve the Tauri-managed app_data_dir without spinning up a Tauri
/// runtime. Mirrors `dirs`/Tauri's per-platform convention.
///
/// - Windows: `%APPDATA%\<identifier>`
/// - macOS:   `~/Library/Application Support/<identifier>`
/// - Linux:   `${XDG_DATA_HOME:-~/.local/share}/<identifier>`
fn resolve_app_data_dir() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var("APPDATA")
            .map_err(|e| format!("APPDATA env not set: {e}"))?;
        Ok(PathBuf::from(appdata).join(TAURI_IDENTIFIER))
    }
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME")
            .map_err(|e| format!("HOME env not set: {e}"))?;
        Ok(PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join(TAURI_IDENTIFIER))
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            return Ok(PathBuf::from(xdg).join(TAURI_IDENTIFIER));
        }
        let home = std::env::var("HOME")
            .map_err(|e| format!("HOME env not set: {e}"))?;
        Ok(PathBuf::from(home)
            .join(".local")
            .join("share")
            .join(TAURI_IDENTIFIER))
    }
}

/// Headless --clear-metadata <SYSTEM_ID> entry point. Opens library_db
/// to enumerate game ids for the system, mutates media.json's
/// `metadata` slot to None for matching entries, writes back, prints
/// a summary, and returns. Caller (main) exits 0 / 1 based on Ok / Err.
///
/// Runs without a Tauri runtime — no window, no emu thread, no
/// settings store. Single-purpose maintenance mode.
pub fn run_clear_metadata_headless(system_id: &str) -> Result<(), String> {
    let app_data_dir = resolve_app_data_dir()?;
    eprintln!("oa-shell: clear-metadata {system_id}");
    eprintln!("oa-shell: app_data_dir = {}", app_data_dir.display());

    let library = crate::library_db::LibraryDb::open(&app_data_dir)
        .map_err(|e| format!("library_db open: {e}"))?;
    let ids = library.list_game_ids_for_system(system_id)?;
    if ids.is_empty() {
        eprintln!("oa-shell: no games tagged with system_id={system_id}");
        return Ok(());
    }
    let mut media_db = crate::media::read_media_db(&app_data_dir);
    let mut cleared = 0usize;
    for id in &ids {
        if let Some(gm) = media_db.get_mut(id) {
            if gm.metadata.is_some() {
                gm.metadata = None;
                cleared += 1;
            }
        }
    }
    if cleared > 0 {
        crate::media::write_media_db(&app_data_dir, &media_db)
            .map_err(|e| format!("write media.json: {e}"))?;
    }
    eprintln!(
        "oa-shell: cleared metadata on {cleared} of {} game id(s) for {system_id}",
        ids.len()
    );
    Ok(())
}

/// Parse argv via clap and resolve a direct-launch config when a ROM is
/// supplied. `Ok(None)` = library mode. Validation errors are surfaced to
/// the caller, which banner-prints them and exits with status 2.
///
/// `--help` / `--version` are intercepted here: clap renders them as
/// special "errors" that this function catches and displays via stderr
/// (debug / console-inheritable) AND via a Windows MessageBox on release
/// builds (where `windows_subsystem = "windows"` detaches stdout/stderr).
/// On success the process exits 0 directly — never returns to main().
pub fn parse_and_resolve() -> Result<CliConfig, CliError> {
    use clap::error::ErrorKind;

    match Cli::try_parse() {
        Ok(cli) => {
            // Headless maintenance mode short-circuits before any
            // direct-launch resolution. The --clear-metadata flag is
            // mutually exclusive with --rom in the sense that a
            // headless run never spins up a window — passing both is
            // a user error but we tolerate it by giving --clear-metadata
            // precedence and exiting after the maintenance runs.
            if let Some(sys) = cli.clear_metadata.as_deref() {
                match run_clear_metadata_headless(sys) {
                    Ok(()) => std::process::exit(0),
                    Err(e) => {
                        eprintln!("oa-shell: clear-metadata failed: {e}");
                        #[cfg(all(target_os = "windows", not(debug_assertions)))]
                        win_msgbox::error(
                            "oa-shell — clear-metadata failed",
                            &format!("Could not clear metadata for {sys}:\n\n{e}"),
                        );
                        std::process::exit(1);
                    }
                }
            }
            let kiosk = cli.kiosk;
            let direct_launch = resolve(cli)?;
            Ok(CliConfig { direct_launch, kiosk })
        }
        Err(err) => match err.kind() {
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                let text = err.render().to_string();
                eprintln!("{text}");
                #[cfg(all(target_os = "windows", not(debug_assertions)))]
                {
                    let title = if matches!(err.kind(), ErrorKind::DisplayVersion) {
                        "oa-shell — version"
                    } else {
                        "oa-shell — help"
                    };
                    win_msgbox::info(title, &text);
                }
                std::process::exit(0);
            }
            _ => {
                // clap validation errors (unknown flag, bad value, etc.).
                // Render in clap's standard form, surface via stderr +
                // MessageBox, and exit 2 — never reach `resolve`.
                let text = err.render().to_string();
                eprintln!("{text}");
                #[cfg(all(target_os = "windows", not(debug_assertions)))]
                win_msgbox::error("oa-shell — invalid arguments", &text);
                std::process::exit(2);
            }
        },
    }
}

fn resolve(cli: Cli) -> Result<Option<DirectLaunchConfig>, CliError> {
    let raw_rom = match (cli.rom_positional.clone(), cli.rom.clone()) {
        (Some(_), Some(_)) => {
            return Err(CliError::Conflict(
                "Specify either a positional ROM path or --rom, not both.",
            ));
        }
        (Some(p), None) => p,
        (None, Some(p)) => p,
        (None, None) => return Ok(None),
    };

    // --slot and --state-file are mutually exclusive (RetroArch convention:
    // --load-state PATH overrides --state-slot N; we error early instead of
    // silently picking one).
    if cli.slot.is_some() && cli.state_file.is_some() {
        return Err(CliError::Conflict(
            "Specify either --slot or --state-file, not both.",
        ));
    }
    if let Some(p) = cli.state_file.as_deref() {
        if !p.exists() {
            return Err(CliError::StateFileMissing(p.to_path_buf()));
        }
    }

    if let Some(slug) = cli.system.as_deref() {
        if !is_known_system(slug) {
            return Err(CliError::UnknownSystem(slug.to_string()));
        }
    }

    // Decode explicit `<archive>#<inner>` syntax. Power-user form for
    // multi-game archives — `oa-shell.exe "GoodSNES.zip#Super Mario World.sfc"`
    // — bypasses the Phase H single-ROM auto-extract requirement.
    let raw_rom_str = raw_rom.to_string_lossy().into_owned();
    let (rom_path, explicit_inner) = if raw_rom_str.contains('#') {
        let (archive, inner) = crate::archive::decode_file_path(&raw_rom_str);
        if inner.is_empty() {
            (raw_rom, None)
        } else {
            (archive, Some(inner))
        }
    } else {
        (raw_rom, None)
    };

    if !rom_path.exists() {
        return Err(CliError::RomFileMissing(rom_path));
    }

    let (system_id, archive_inner_path) = if let Some(inner) = explicit_inner.as_deref() {
        resolve_explicit_archive_inner(&rom_path, inner, cli.system.as_deref())?
    } else if is_archive_extension(&rom_path) {
        resolve_archive(&rom_path, cli.system.as_deref())?
    } else {
        let system_id = match cli.system.as_deref() {
            Some(slug) => slug.to_string(),
            None => infer_system_from_extension(&rom_path)?.to_string(),
        };
        (system_id, None)
    };

    Ok(Some(DirectLaunchConfig {
        rom_path,
        system_id,
        core_override: cli.core,
        slot: cli.slot,
        state_file: cli.state_file,
        tas_replay: cli.tas_replay,
        fullscreen: cli.fullscreen,
        matched_entry_id: None,
        archive_inner_path,
    }))
}

/// Resolve an explicit `<archive>#<inner>` invocation. The inner extension
/// determines the system (cart shape via `slug_for_ext`, CD shape requires
/// `--system`). The inner is validated against the archive's table of
/// contents so a typo errors before launch instead of at extract time.
fn resolve_explicit_archive_inner(
    archive_path: &Path,
    inner: &str,
    explicit_system: Option<&str>,
) -> Result<(String, Option<String>), CliError> {
    if !is_archive_extension(archive_path) {
        return Err(CliError::ArchiveReadFailed {
            archive: archive_path.to_path_buf(),
            error: format!(
                "explicit `#{inner}` inner-path syntax requires an archive outer (.zip / .7z)"
            ),
        });
    }

    let mut accepted = accepted_rom_extensions();
    // Allow CD entry-points too for the explicit form — caller supplies
    // --system for those (validated below).
    for ext in ["cue", "ccd", "toc", "m3u"] {
        accepted.insert(ext.to_string());
    }
    let entries = crate::archive::list_rom_contents(archive_path, &accepted).map_err(|e| {
        CliError::ArchiveReadFailed {
            archive: archive_path.to_path_buf(),
            error: e,
        }
    })?;
    if !entries.iter().any(|e| e.inner_path == inner) {
        return Err(CliError::ArchiveInnerNotFound {
            archive: archive_path.to_path_buf(),
            inner: inner.to_string(),
            available: entries.iter().map(|e| e.inner_path.clone()).collect(),
        });
    }

    let inner_ext = Path::new(inner)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    let is_cd = matches!(inner_ext.as_str(), "cue" | "ccd" | "toc" | "m3u");

    let system_id = match explicit_system {
        Some(s) => s.to_string(),
        None => {
            if is_cd {
                return Err(CliError::AmbiguousExtension {
                    ext: inner_ext.clone(),
                    candidates: vec!["pce-cd", "segacd", "saturn", "psx", "neocd", "3do", "pcfx"],
                });
            }
            slug_for_ext(&inner_ext)
                .ok_or_else(|| CliError::UnknownExtension(inner_ext.clone()))?
                .to_string()
        }
    };

    log::info!(
        "oa-shell: direct-launch explicit inner {}#{} (system={})",
        archive_path.display(),
        inner,
        system_id,
    );
    Ok((system_id, Some(inner.to_string())))
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
        archive_inner_path: None,
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
            clear_metadata: None,
            kiosk: false,
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
            clear_metadata: None,
            kiosk: false,
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
            clear_metadata: None,
            kiosk: false,
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
            clear_metadata: None,
            kiosk: false,
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
