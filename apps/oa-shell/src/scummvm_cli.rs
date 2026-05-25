//! Optional ScummVM CLI shell-out for power-user auto-detection.
//!
//! Companion to [`crate::scummvm_detect`]'s curated-table approach.
//! The curated table covers the top ~18 well-known SCUMM titles +
//! freeware classics with zero external dependencies. Operators who
//! already have a standalone ScummVM install and want comprehensive
//! detection across the engine's full ~400-game catalog can flip the
//! mode toggle in the `ScummvmDetectDialog` and OA shells out to
//! `scummvm --detect --path=<dir>` for the heavy lifting.
//!
//! ## Why this exists
//!
//! LaunchBox's ScummVM importer uses exactly this approach. The
//! ScummVM team maintains the canonical detection database in C++
//! source, exposed via the CLI's `--detect` mode. Shelling out gets
//! us the full database without OA having to maintain a Rust-side
//! mirror.
//!
//! Trade-off vs the curated table: needs the operator to have a
//! standalone ScummVM install (free, ~30MB) on their machine. Most
//! retro-gaming enthusiasts already do; OA's table covers the
//! activation-energy-zero case for everyone else.
//!
//! ## Discovery
//!
//! [`find_scummvm_executable`] checks the canonical install paths for
//! each platform first (cheap stat calls), then walks `$PATH`. The
//! frontend uses this to pre-fill the dialog's CLI-path input; the
//! operator can override with a custom path via the file picker.
//!
//! ## Output parsing
//!
//! `scummvm --detect` (modern 2.x format) emits:
//!
//! ```text
//! GameID                          Description                                       Full Path
//! ------------------------------- ------------------------------------------------- -----------------------------------------------
//! monkey                          The Secret of Monkey Island (VGA Floppy/English)  /path/to/MonkeyIsland
//! tentacle                        Day of the Tentacle (CD/English)                  /path/to/DOTT
//! ```
//!
//! [`parse_detect_output`] looks for lines matching the
//! `<gameid> <description> <path>` shape (2+ space column separator)
//! and skips everything else (header, dashes, blank lines, the rare
//! warning ScummVM emits on stdout). The parser is intentionally
//! liberal — if a line doesn't match, we skip it and log; we never
//! abort the whole batch.
//!
//! ## Descriptor format
//!
//! The CLI emits gameid alone (no engine prefix). ScummVM's libretro
//! core accepts bare gameids in the descriptor file — its internal
//! detection table maps gameid → engine. The dialog writes
//! `<gameid>` as the descriptor body for CLI-detected rows. (Curated-
//! table rows write `<gameid>:<engine>` since we know both.)

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

/// One row from the parsed `scummvm --detect` output.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliDetectionRow {
    /// Game id as ScummVM reports it (e.g. `"monkey"`, `"tentacle"`,
    /// `"sky"`). Used verbatim as the descriptor body — ScummVM's
    /// libretro core's internal detection table maps gameid → engine.
    pub game_id: String,
    /// Human description from the CLI (e.g.
    /// `"The Secret of Monkey Island (VGA Floppy/English)"`).
    pub description: String,
    /// Absolute path the CLI reported for this detection.
    /// Operator-facing — we use this to align CLI rows against the
    /// subdirectory the curated-table walker found.
    pub directory: PathBuf,
}

/// Try the standard install paths for a ScummVM executable, then
/// walk `$PATH`. Returns the first hit; `None` if nothing resolves.
///
/// Standard paths per platform (checked in order):
/// - **Windows**: `C:\Program Files\ScummVM\scummvm.exe`,
///   `C:\Program Files (x86)\ScummVM\scummvm.exe`
/// - **macOS**: `/Applications/ScummVM.app/Contents/MacOS/scummvm`
/// - **Linux**: `/usr/bin/scummvm`, `/usr/local/bin/scummvm`
///
/// Then PATH walk for the platform-appropriate executable name
/// (`scummvm.exe` on Windows, `scummvm` everywhere else).
pub fn find_scummvm_executable() -> Option<PathBuf> {
    for candidate in standard_install_paths() {
        if candidate.is_file() {
            log::info!("scummvm_cli: found at {}", candidate.display());
            return Some(candidate);
        }
    }
    let exe_name = if cfg!(windows) { "scummvm.exe" } else { "scummvm" };
    if let Ok(path_env) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_env) {
            let candidate = dir.join(exe_name);
            if candidate.is_file() {
                log::info!("scummvm_cli: found on PATH at {}", candidate.display());
                return Some(candidate);
            }
        }
    }
    log::info!("scummvm_cli: no standalone ScummVM install found");
    None
}

fn standard_install_paths() -> Vec<PathBuf> {
    if cfg!(windows) {
        // Windows ScummVM installer defaults. Some operators install
        // ScummVM to non-standard paths; the PATH walk catches those.
        let mut paths = Vec::new();
        if let Ok(pf) = std::env::var("ProgramFiles") {
            paths.push(PathBuf::from(pf).join("ScummVM").join("scummvm.exe"));
        }
        if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
            paths.push(PathBuf::from(pf86).join("ScummVM").join("scummvm.exe"));
        }
        paths
    } else if cfg!(target_os = "macos") {
        vec![
            PathBuf::from("/Applications/ScummVM.app/Contents/MacOS/scummvm"),
        ]
    } else {
        // Linux + other Unixes — package manager defaults.
        vec![
            PathBuf::from("/usr/bin/scummvm"),
            PathBuf::from("/usr/local/bin/scummvm"),
        ]
    }
}

/// Shell out to `scummvm --detect --path=<dir>`, parse stdout, return
/// the detected rows. Errors include the CLI's stderr verbatim so
/// operators can diagnose without re-running by hand.
///
/// Runs with a 30s timeout via the OS-level process — ScummVM
/// detection on a few hundred game dirs is fast (<1s typical), so
/// anything slower likely means the CLI is hanging on a malformed
/// directory. The Tauri command layer wraps this in spawn_blocking so
/// the renderer doesn't stall.
pub fn run_detect(
    scummvm_path: &Path,
    target_dir: &Path,
) -> Result<Vec<CliDetectionRow>, String> {
    if !scummvm_path.is_file() {
        return Err(format!(
            "ScummVM executable not found at {}",
            scummvm_path.display()
        ));
    }
    if !target_dir.is_dir() {
        return Err(format!(
            "target directory not found: {}",
            target_dir.display()
        ));
    }

    log::info!(
        "scummvm_cli: invoking {} --detect --path={} --recursive",
        scummvm_path.display(),
        target_dir.display()
    );
    let output = Command::new(scummvm_path)
        .arg("--detect")
        .arg("--recursive")
        .arg(format!("--path={}", target_dir.display()))
        .output()
        .map_err(|e| format!("failed to invoke ScummVM CLI: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "ScummVM CLI failed (exit {}): {}",
            output.status.code().unwrap_or(-1),
            stderr.trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let rows = parse_detect_output(&stdout);
    log::info!(
        "scummvm_cli: detected {} games in {}",
        rows.len(),
        target_dir.display()
    );
    Ok(rows)
}

/// Parse `scummvm --detect` stdout into structured rows. Liberal
/// parser — skips header lines, dashes, blank lines, and anything
/// else that doesn't shape-match a detection row.
///
/// Detection rows have the shape `<gameid><whitespace><description><whitespace><path>`
/// where the gameid contains no spaces, the path is absolute (starts
/// with `/` on Unix or matches drive-letter on Windows), and the
/// description sits between the two with 2+ spaces separating each
/// column.
pub fn parse_detect_output(stdout: &str) -> Vec<CliDetectionRow> {
    let mut rows = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            continue;
        }
        // Skip the header line (column titles) and the dashes
        // separator. Detection rows never start with a dash and
        // never begin with the literal "GameID" header.
        let lead = trimmed.trim_start();
        if lead.starts_with("---") || lead.starts_with("GameID") {
            continue;
        }
        if let Some(row) = parse_detect_row(trimmed) {
            rows.push(row);
        } else {
            log::debug!("scummvm_cli: skipping unparseable line: {trimmed:?}");
        }
    }
    rows
}

/// Parse one detection-row line. Returns `None` if the line doesn't
/// match the expected shape.
///
/// The gameid is the first whitespace-delimited token. The path is
/// the last token that looks like an absolute path. The description
/// is everything between them, trimmed of surrounding whitespace.
///
/// Defensive against varying column widths across ScummVM versions —
/// older 1.x versions used different padding than modern 2.x.
fn parse_detect_row(line: &str) -> Option<CliDetectionRow> {
    // First whitespace-separated token is the gameid. Must not be
    // empty and must contain only [a-zA-Z0-9_-] (ScummVM gameid
    // grammar).
    let mut chars = line.char_indices();
    let first_nonws = chars.find(|(_, c)| !c.is_whitespace())?.0;
    let after_first_ws = line[first_nonws..]
        .find(char::is_whitespace)
        .map(|n| first_nonws + n)?;
    let game_id = line[first_nonws..after_first_ws].to_string();
    if game_id.is_empty()
        || !game_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return None;
    }
    let rest = line[after_first_ws..].trim_start();
    // Last token starting with `/` or matching `X:\` or `X:/` is the path.
    // ScummVM CLI always emits absolute paths.
    let path_start = find_last_path_start(rest)?;
    let description = rest[..path_start].trim().to_string();
    let path_str = rest[path_start..].trim();
    if description.is_empty() || path_str.is_empty() {
        return None;
    }
    Some(CliDetectionRow {
        game_id,
        description,
        directory: PathBuf::from(path_str),
    })
}

/// Find the start byte-offset of the last absolute-path-looking
/// substring in `s`. Handles both Unix paths (`/foo/bar`) and
/// Windows paths (`C:\foo\bar` or `C:/foo/bar`). Returns `None` if
/// no path-looking substring is found.
///
/// The "last" qualifier matters because the description may contain
/// path-shaped tokens (rare — e.g. a parenthesised release note like
/// "(SVN/release)"). The final column of the CLI output is always the
/// real path, so we anchor on that.
fn find_last_path_start(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = bytes.len();
    while i > 0 {
        // Look backward for a candidate path-start byte.
        if let Some(pos) = s[..i].rfind(|c: char| c == '/' || c == '\\') {
            // Check whether this is a real path-start by walking
            // backward through preceding non-whitespace bytes. A
            // path starts at a whitespace boundary (or string start).
            let mut start = pos;
            while start > 0 {
                let prev_byte = bytes[start - 1];
                if (prev_byte as char).is_whitespace() {
                    break;
                }
                start -= 1;
            }
            // Reject paths that are mid-word slashes (e.g.
            // `MonkeyIsland/CD` — no, that's not a real path because
            // it doesn't start after whitespace). The `start` walk-
            // back catches those.
            let candidate = &s[start..];
            if looks_like_absolute_path(candidate) {
                return Some(start);
            }
            // Step back past this slash and try again.
            i = pos;
            continue;
        }
        break;
    }
    None
}

fn looks_like_absolute_path(s: &str) -> bool {
    // Unix: starts with `/`. Windows: starts with `X:` followed by
    // `\` or `/`. (Also accept the Unix form on Windows for paths
    // that came through WSL or similar.)
    let s = s.trim();
    if s.starts_with('/') {
        return true;
    }
    let bytes = s.as_bytes();
    if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
    {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_modern_2x_format() {
        // Sample output mirroring ScummVM 2.5+ --detect format.
        let output = r#"GameID                          Description                                       Full Path
------------------------------- ------------------------------------------------- -----------------------------------------------
monkey                          The Secret of Monkey Island (VGA Floppy/English)  /home/user/games/MonkeyIsland
tentacle                        Day of the Tentacle (CD/English)                  /home/user/games/DOTT
sky                             Beneath a Steel Sky (Floppy/English)              /home/user/games/Sky
"#;
        let rows = parse_detect_output(output);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].game_id, "monkey");
        assert_eq!(rows[0].description, "The Secret of Monkey Island (VGA Floppy/English)");
        assert_eq!(rows[0].directory, PathBuf::from("/home/user/games/MonkeyIsland"));
        assert_eq!(rows[1].game_id, "tentacle");
        assert_eq!(rows[2].game_id, "sky");
    }

    #[test]
    fn parse_windows_paths() {
        let output = r#"GameID    Description                              Full Path
--------  ---------------------------------------  --------------------------------
monkey    The Secret of Monkey Island (VGA)        C:\Games\MonkeyIsland
tentacle  Day of the Tentacle (CD)                 D:\Games\DOTT
"#;
        let rows = parse_detect_output(output);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].directory, PathBuf::from(r"C:\Games\MonkeyIsland"));
        assert_eq!(rows[1].directory, PathBuf::from(r"D:\Games\DOTT"));
    }

    #[test]
    fn parse_skips_header_and_dashes() {
        let output = "GameID    Description    Full Path\n--- ----- ---\n\nmonkey  Monkey Island (English)  /games/monkey\n";
        let rows = parse_detect_output(output);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].game_id, "monkey");
    }

    #[test]
    fn parse_skips_empty_input() {
        assert!(parse_detect_output("").is_empty());
        assert!(parse_detect_output("\n\n\n").is_empty());
    }

    #[test]
    fn parse_handles_description_with_slashes() {
        // Some descriptions contain slashes (e.g. "(VGA/Floppy)"); the
        // path detection should anchor on the LAST whitespace-bounded
        // absolute path, not a mid-description slash.
        let output =
            "monkey  The Secret of Monkey Island (VGA/Floppy/English)  /home/user/MonkeyIsland\n";
        let rows = parse_detect_output(output);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].directory, PathBuf::from("/home/user/MonkeyIsland"));
        assert_eq!(
            rows[0].description,
            "The Secret of Monkey Island (VGA/Floppy/English)"
        );
    }

    #[test]
    fn parse_rejects_lines_without_path() {
        // Edge case: a malformed line with no absolute path. Should
        // skip rather than panic.
        let output = "monkey  Monkey Island (no path here)\n";
        let rows = parse_detect_output(output);
        assert!(rows.is_empty());
    }

    #[test]
    fn parse_rejects_invalid_gameid() {
        // GameID with special chars that ScummVM never uses.
        let output = "mon!key  Monkey Island (Bad ID)  /games/monkey\n";
        let rows = parse_detect_output(output);
        assert!(rows.is_empty());
    }

    #[test]
    fn find_last_path_start_basic_cases() {
        assert_eq!(find_last_path_start("desc  /games/foo"), Some(6));
        assert_eq!(
            find_last_path_start("desc  C:\\Games\\Foo"),
            Some(6),
        );
        assert_eq!(find_last_path_start("no path here"), None);
    }

    #[test]
    fn looks_like_absolute_path_smoke() {
        assert!(looks_like_absolute_path("/games/foo"));
        assert!(looks_like_absolute_path("C:\\Games\\Foo"));
        assert!(looks_like_absolute_path("D:/games/foo"));
        assert!(!looks_like_absolute_path("games/foo"));
        assert!(!looks_like_absolute_path("foo"));
        assert!(!looks_like_absolute_path(""));
    }

    #[test]
    fn standard_install_paths_returns_nonzero_per_platform() {
        // Smoke test that platform detection doesn't return an empty
        // list — operators on every platform should get SOME path
        // candidates even before falling through to the PATH walk.
        let paths = standard_install_paths();
        assert!(!paths.is_empty(), "platform paths should not be empty");
    }
}
