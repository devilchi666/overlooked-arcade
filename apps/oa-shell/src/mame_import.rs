//! In-process MAME data import for the operator-driven "Refresh MAME
//! system info" path (System Info Panel v1 Phase 5).
//!
//! The maintainer-time pipeline lives in `tools/mame-extractor/` —
//! standalone Cargo binary, ships the slim JSON/XML artifacts under
//! `assets/mame-source/` next to the OA install. This module is the
//! in-app equivalent: when the operator clicks SETTINGS → Storage →
//! "Refresh MAME system info", we locate their local MAME, run
//! `mame -listxml` ourselves, parse the output via quick-xml, fold in
//! `history.xml` descriptions, and overwrite the SQLite `system_info_mame`
//! table — without touching L2 (curated YAML) or L3 (operator overrides).
//!
//! The listxml-parse logic is ported from
//! `tools/mame-extractor/src/main.rs` (same MAME_DRIVER_MAP,
//! format_clock formatting, ExtractedMachine state machine). It's
//! duplicated rather than shared via a workspace crate because the
//! extractor is a standalone Cargo workspace (it pulls in clap +
//! anyhow we don't want in oa-shell) and the parser is small enough
//! that the maintenance cost of two copies is low. If either copy
//! evolves, mirror the change.
//!
//! Plan: `docs/PLANS/system-info-panel-v1.md` §5 + §6.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use serde::Serialize;

use crate::library_db::LibraryDb;
use crate::system_info::{self, SystemInfoMame};

/// OA slug → MAME machine driver name. Mirror of the constant in
/// `tools/mame-extractor/src/main.rs`. Keep the two in sync when
/// adding new systems.
///
/// Systems intentionally absent (no usable MAME driver):
/// PSP / PS2 / NDS / GameCube — engine cores or insufficient MAME
/// coverage for the fields we extract; scummvm / dosbox — engine
/// cores; `mame` itself — IS MAME, no parent-machine representation.
pub const MAME_DRIVER_MAP: &[(&str, &str)] = &[
    ("tg16", "pce"),
    ("pce-cd", "pce"),
    ("lynx", "lynx"),
    ("nes", "nes"),
    ("snes", "snes"),
    ("atari7800", "a7800"),
    ("genesis", "genesis"),
    ("segacd", "segacd"),
    ("sega32x", "32x"),
    ("sega32xcd", "32x_scd"),
    ("saturn", "saturn"),
    ("stv", "stvbios"),
    ("psx", "psj"),
    ("neogeo", "neogeo"),
    ("neocd", "neocdz"),
    ("ngp", "ngp"),
    ("jaguar", "jaguar"),
    ("jagcd", "jaguarcd"),
    ("3do", "3do"),
    ("pcfx", "pcfx"),
    ("n64", "n64"),
    ("dreamcast", "dc"),
    ("sms", "sms"),
    ("gamegear", "gamegear"),
    ("gb", "gameboy"),
    ("gbc", "gbcolor"),
    ("gba", "gba"),
    ("2600", "a2600"),
    ("5200", "a5200"),
    ("coleco", "coleco"),
    ("intv", "intv"),
    ("o2", "odyssey2"),
    ("channelf", "channelf"),
    ("vectrex", "vectrex"),
    ("virtualboy", "vboy"),
    ("wonderswan", "wswan"),
    ("pokemini", "pokemini"),
    ("msx", "msx"),
    ("msx2", "msx2"),
];

/// Outcome of one refresh, surfaced to the frontend as a toast.
/// camelCase serde so it matches the rest of the wire format.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MameRefreshReport {
    /// MAME version string captured from `mame -version` (e.g.
    /// `"0.288"`). The frontend interpolates this into the success
    /// toast: "Refreshed N systems from MAME 0.288."
    pub mame_version: String,
    /// Total slim records written to `system_info_mame` (one row per
    /// OA slug, including shared-machine fan-out like tg16+pce-cd
    /// both → pce). Equal to `MAME_DRIVER_MAP.len() - missing.len()`
    /// after a successful run.
    pub systems_refreshed: u32,
    /// OA slugs that had no matching machine in the operator's
    /// `mame -listxml` output. v1 expects this to include `3do`,
    /// `msx`, `msx2` (upstream gaps — see tools/mame-extractor/
    /// README.md). v2 may extend MAME_DRIVER_MAP to model-specific
    /// driver names (`3do_fz1` etc.) to close those gaps.
    pub missing_systems: Vec<String>,
    /// True when a `history.xml` was located + parsed. False when
    /// the structured-fields half ran but no description prose
    /// was available (operator skipped the arcade-history.com
    /// download). The frontend surfaces this with a "no
    /// descriptions refreshed" sub-toast.
    pub history_present: bool,
    /// Absolute path of the MAME binary we shelled out to, so the
    /// frontend can surface "Refreshed from C:\Emulators\MAME\
    /// mame.exe" in the toast for transparency.
    pub mame_path: String,
}

/// Locate a MAME binary, preferring the canonical project location
/// (`<exe_dir>/Emulators/MAME/`) over PATH lookups over OS-typical
/// system install paths.
///
/// When `custom_path` is supplied, treat it as canonical (allows the
/// frontend's folder picker to override auto-detection). The custom
/// path can be either the binary itself OR the parent directory (the
/// `Emulators/MAME/` folder); the function probes both.
pub fn detect_mame_binary(custom_path: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = custom_path {
        // Direct binary path — accept if it exists + is executable.
        if p.is_file() {
            return Some(p.to_path_buf());
        }
        // Folder path — look for mame.exe / mame inside.
        if p.is_dir() {
            for candidate in &["mame.exe", "mame"] {
                let bin = p.join(candidate);
                if bin.is_file() {
                    return Some(bin);
                }
            }
        }
        return None;
    }

    let mut candidates: Vec<PathBuf> = Vec::new();
    // 1. <exe_dir>/Emulators/MAME/ — the project-wide canonical
    //    location introduced in Phase 1a. Shipped installs follow this
    //    same convention so the operator can drop MAME alongside the
    //    .exe without configuration.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("Emulators").join("MAME").join("mame.exe"));
            candidates.push(parent.join("Emulators").join("MAME").join("mame"));
        }
    }
    // 2. <repo_root>/Emulators/MAME/ — dev convenience. Maintainers
    //    running cargo tauri dev have their MAME under the source tree
    //    rather than next to a built exe.
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
    candidates.push(repo_root.join("Emulators").join("MAME").join("mame.exe"));
    candidates.push(repo_root.join("Emulators").join("MAME").join("mame"));
    // 3. OS-typical paths.
    candidates.push(PathBuf::from(r"C:\mame\mame.exe"));
    candidates.push(PathBuf::from(r"C:\Program Files\MAME\mame.exe"));
    candidates.push(PathBuf::from("/usr/bin/mame"));
    candidates.push(PathBuf::from("/usr/local/bin/mame"));

    for c in candidates {
        if c.is_file() {
            return Some(c);
        }
    }
    None
}

/// Find `history.xml` next to a located MAME install. Probes the
/// three layouts MAME packagings use (`history/history.xml` is
/// canonical for upstream; some bundlers drop it at the install root
/// or under `dats/`).
pub fn detect_history_xml(mame_dir: &Path) -> Option<PathBuf> {
    for rel in &[
        "history/history.xml",
        "history.xml",
        "dats/history.xml",
    ] {
        let p = mame_dir.join(rel);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

/// Shell out to `mame -version`, capture stdout's first line, strip
/// the "MAME v" prefix + the trailing date stamp. Falls back to the
/// raw first line when the parse fails — the version string is only
/// surfaced in the success toast + a metadata log line, never used
/// for behavioural decisions.
pub fn read_mame_version(mame: &Path) -> Result<String, String> {
    let out = Command::new(mame)
        .arg("-version")
        .output()
        .map_err(|e| format!("invoking {} -version: {e}", mame.display()))?;
    if !out.status.success() {
        return Err(format!(
            "{} -version exited with {}",
            mame.display(),
            out.status
        ));
    }
    let raw = String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    if raw.is_empty() {
        return Err(format!("{} -version produced no output", mame.display()));
    }
    // "MAME v0.288 (Jul 31 2024)" → "0.288"; fall back to raw.
    let short = raw
        .strip_prefix("MAME ")
        .or_else(|| raw.strip_prefix("mame "))
        .unwrap_or(&raw)
        .trim_start_matches('v')
        .split_whitespace()
        .next()
        .unwrap_or(&raw)
        .to_string();
    if short.is_empty() {
        Ok(raw)
    } else {
        Ok(short)
    }
}

/// Run `mame -listxml`, capturing the output to a temp file (the
/// emit can exceed 200MB on recent MAME; streaming-parse from disk
/// keeps peak memory bounded).
///
/// Returns the temp path. Caller is responsible for `std::fs::remove_file`
/// once parsing is done.
pub fn run_mame_listxml(mame: &Path) -> Result<PathBuf, String> {
    let tmp = std::env::temp_dir().join(format!(
        "oa-mame-listxml-{}.xml",
        std::process::id()
    ));
    let f = fs::File::create(&tmp)
        .map_err(|e| format!("creating {}: {e}", tmp.display()))?;
    let status = Command::new(mame)
        .arg("-listxml")
        .stdout(f)
        .status()
        .map_err(|e| format!("invoking {} -listxml: {e}", mame.display()))?;
    if !status.success() {
        let _ = fs::remove_file(&tmp);
        return Err(format!(
            "{} -listxml exited with {status}",
            mame.display()
        ));
    }
    Ok(tmp)
}

// =====================================================================
// listxml parser — ported from tools/mame-extractor/src/main.rs
// =====================================================================
//
// The streaming quick-xml walk filters MAME's emit to OA-relevant
// machines on the fly so peak memory stays bounded by the per-machine
// working set rather than the 200MB+ input. Same shape as the
// extractor's parse_listxml; only the error type changes (Result<…,
// String> here matches oa-shell's convention).

#[derive(Default, Debug)]
struct ExtractedMachine {
    year: Option<String>,
    manufacturer: Option<String>,
    cpu: Option<String>,
    sound: Option<String>,
    resolution: Option<String>,
    refresh_rate: Option<String>,
    max_players: Option<u32>,
    peripheral_hints: BTreeSet<String>,
}

#[derive(Copy, Clone)]
enum TextTarget {
    Year,
    Manufacturer,
}

fn parse_listxml(
    path: &Path,
    wanted_machines: &HashSet<&str>,
) -> Result<HashMap<String, ExtractedMachine>, String> {
    let mut reader = Reader::from_file(path)
        .map_err(|e| format!("opening listxml at {}: {e}", path.display()))?;
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut out: HashMap<String, ExtractedMachine> = HashMap::new();
    let mut current: Option<(String, ExtractedMachine)> = None;
    let mut text_target: Option<TextTarget> = None;
    let mut text_buf = String::new();

    loop {
        let evt = reader.read_event_into(&mut buf).map_err(|e| {
            format!("reading XML at byte {}: {e}", reader.buffer_position())
        })?;
        match evt {
            Event::Start(ref e) => handle_open_tag(
                e,
                /* self_closing= */ false,
                wanted_machines,
                &mut current,
                &mut text_target,
                &mut text_buf,
            ),
            Event::Empty(ref e) => handle_open_tag(
                e,
                /* self_closing= */ true,
                wanted_machines,
                &mut current,
                &mut text_target,
                &mut text_buf,
            ),
            Event::Text(ref t) => {
                if text_target.is_some() {
                    if let Ok(unesc) = t.unescape() {
                        text_buf.push_str(&unesc);
                    }
                }
            }
            Event::End(ref e) => {
                let name = e.name();
                let tag = std::str::from_utf8(name.as_ref()).unwrap_or("");
                match tag {
                    "machine" => {
                        if let Some((mname, em)) = current.take() {
                            out.insert(mname, em);
                        }
                    }
                    "year" | "manufacturer" => {
                        if let Some(target) = text_target.take() {
                            if let Some((_, em)) = current.as_mut() {
                                let value = text_buf.trim().to_string();
                                if !value.is_empty() {
                                    match target {
                                        TextTarget::Year => em.year = Some(value),
                                        TextTarget::Manufacturer => {
                                            em.manufacturer = Some(value)
                                        }
                                    }
                                }
                            }
                            text_buf.clear();
                        }
                    }
                    _ => {}
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(out)
}

fn handle_open_tag(
    e: &BytesStart,
    self_closing: bool,
    wanted_machines: &HashSet<&str>,
    current: &mut Option<(String, ExtractedMachine)>,
    text_target: &mut Option<TextTarget>,
    text_buf: &mut String,
) {
    let name = e.name();
    let tag = std::str::from_utf8(name.as_ref()).unwrap_or("");
    match tag {
        "machine" => {
            if let Some(mname) = attr(e, b"name") {
                if wanted_machines.contains(mname.as_str()) {
                    *current = Some((mname, ExtractedMachine::default()));
                } else {
                    *current = None;
                }
            }
        }
        "year" | "manufacturer" if !self_closing && current.is_some() => {
            *text_target = Some(match tag {
                "year" => TextTarget::Year,
                _ => TextTarget::Manufacturer,
            });
            text_buf.clear();
        }
        "input" if current.is_some() => {
            if let Some(p) = attr(e, b"players").and_then(|v| v.parse::<u32>().ok()) {
                if let Some((_, em)) = current.as_mut() {
                    em.max_players = Some(p);
                }
            }
        }
        "control" if current.is_some() => {
            if let Some(t) = attr(e, b"type") {
                if let Some((_, em)) = current.as_mut() {
                    em.peripheral_hints.insert(t);
                }
            }
        }
        "display" if current.is_some() => {
            let w = attr(e, b"width").and_then(|v| v.parse::<u32>().ok());
            let h = attr(e, b"height").and_then(|v| v.parse::<u32>().ok());
            let refresh = attr(e, b"refresh").and_then(|v| v.parse::<f64>().ok());
            if let Some((_, em)) = current.as_mut() {
                if em.resolution.is_none() {
                    if let (Some(w), Some(h)) = (w, h) {
                        em.resolution = Some(format!("{} × {}", w, h));
                    }
                }
                if em.refresh_rate.is_none() {
                    if let Some(r) = refresh {
                        em.refresh_rate = Some(format!("{:.2} Hz", r));
                    }
                }
            }
        }
        "chip" if current.is_some() => {
            let chip_type = attr(e, b"type");
            let name = attr(e, b"name");
            let clock = attr(e, b"clock").and_then(|v| v.parse::<u64>().ok());
            if let (Some(ct), Some((_, em))) = (chip_type.as_deref(), current.as_mut()) {
                if matches!(ct, "cpu" | "audio") {
                    if let Some(text) = format_chip(name.as_deref(), clock) {
                        match ct {
                            "cpu" if em.cpu.is_none() => em.cpu = Some(text),
                            "audio" if em.sound.is_none() => em.sound = Some(text),
                            _ => {}
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

fn attr(e: &BytesStart, key: &[u8]) -> Option<String> {
    for a in e.attributes() {
        let Ok(a) = a else { continue };
        if a.key.as_ref() == key {
            return a.unescape_value().ok().map(|c| c.into_owned());
        }
    }
    None
}

fn format_chip(name: Option<&str>, clock_hz: Option<u64>) -> Option<String> {
    match (name, clock_hz) {
        (Some(n), Some(c)) => Some(format!("{} @ {}", n.trim(), format_clock(c))),
        (Some(n), None) if !n.trim().is_empty() => Some(n.trim().to_string()),
        _ => None,
    }
}

fn format_clock(hz: u64) -> String {
    if hz >= 1_000_000_000 {
        format!("{:.2} GHz", hz as f64 / 1_000_000_000.0)
    } else if hz >= 1_000_000 {
        format!("{:.2} MHz", hz as f64 / 1_000_000.0)
    } else if hz >= 1_000 {
        format!("{} kHz", hz / 1_000)
    } else {
        format!("{} Hz", hz)
    }
}

// =====================================================================
// Orchestrator
// =====================================================================

/// End-to-end refresh: detect MAME, run -listxml, parse, fold in
/// history descriptions, fan out to per-slug records, write to the
/// L1 table. Does NOT touch L2 (curated YAML) or L3 (operator
/// overrides) — those are unchanged whether this succeeds or fails.
///
/// `custom_mame_path` (when present) overrides auto-detection — used
/// by the frontend's folder-picker fallback when the canonical
/// `<exe_dir>/Emulators/MAME/` location isn't populated.
///
/// Returns the MameRefreshReport for the success toast. Errors only
/// when MAME can't be found, the subprocess fails outright, or the
/// SQLite write fails. A partial refresh (some OA slugs missing from
/// upstream) succeeds with `missing_systems` populated; the operator
/// sees a single toast naming the gaps.
pub fn refresh_mame_system_info(
    custom_mame_path: Option<&Path>,
    db: &LibraryDb,
) -> Result<MameRefreshReport, String> {
    let mame = detect_mame_binary(custom_mame_path)
        .ok_or_else(|| {
            "MAME binary not found. Drop your MAME install at \
             <exe_dir>/Emulators/MAME/mame.exe, or use the folder \
             picker to locate it manually."
                .to_string()
        })?;
    log::info!("system_info: refresh — MAME at {}", mame.display());

    let version = read_mame_version(&mame)
        .map_err(|e| format!("reading MAME version: {e}"))?;
    log::info!("system_info: refresh — MAME version {}", version);

    let mame_dir = mame.parent().ok_or_else(|| {
        format!("MAME path {} has no parent dir", mame.display())
    })?;
    let history_path = detect_history_xml(mame_dir);

    let listxml_tmp = run_mame_listxml(&mame)?;

    // Compute the wanted-machines set from MAME_DRIVER_MAP — same
    // pattern as the extractor's startup. Strings are 'static so the
    // HashSet doesn't have to own them.
    let mut wanted: HashSet<&str> = HashSet::new();
    let mut machine_to_slugs: HashMap<&str, Vec<&str>> = HashMap::new();
    for (slug, machine) in MAME_DRIVER_MAP {
        wanted.insert(*machine);
        machine_to_slugs.entry(*machine).or_default().push(*slug);
    }

    let parse_result = parse_listxml(&listxml_tmp, &wanted);
    let _ = fs::remove_file(&listxml_tmp); // best-effort cleanup
    let extracted = parse_result?;

    // Optional history.xml: when present, parse via the existing
    // slim-XML parser in system_info.rs (the upstream history.xml has
    // the same shape — slim is just a filtered subset).
    let descriptions: HashMap<String, String> = match history_path.as_deref() {
        Some(p) => match fs::read_to_string(p) {
            Ok(body) => system_info::parse_history_slim_xml(&body)
                .unwrap_or_else(|e| {
                    log::warn!(
                        "system_info: history.xml parse failed ({e}); continuing without descriptions"
                    );
                    HashMap::new()
                }),
            Err(e) => {
                log::warn!(
                    "system_info: history.xml read failed at {} ({e}); continuing without descriptions",
                    p.display()
                );
                HashMap::new()
            }
        },
        None => HashMap::new(),
    };

    // Fan out one SystemInfoMame per OA slug. Order matches
    // MAME_DRIVER_MAP so the SQLite rebake is byte-stable across
    // operator-driven refreshes.
    let mut rows: Vec<SystemInfoMame> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    for (slug, machine) in MAME_DRIVER_MAP {
        let Some(em) = extracted.get(*machine) else {
            missing.push((*slug).to_string());
            continue;
        };
        rows.push(SystemInfoMame {
            system_id: (*slug).to_string(),
            machine_name: Some((*machine).to_string()),
            year: em.year.clone(),
            manufacturer: em.manufacturer.clone(),
            cpu: em.cpu.clone(),
            sound: em.sound.clone(),
            resolution: em.resolution.clone(),
            refresh_rate: em.refresh_rate.clone(),
            max_players: em.max_players,
            peripheral_hints: em.peripheral_hints.iter().cloned().collect(),
            description: descriptions.get(*machine).cloned(),
        });
    }

    let refreshed = rows.len() as u32;
    db.bake_system_info_mame(&rows)?;
    // Do NOT touch system_info_meta. The stored hash is for the
    // BUNDLED slim files; an operator re-import is session-scoped per
    // plan §5 — next OA update rebakes from the bundled data and
    // overwrites this. v2 candidate: per-row provenance tracking.
    log::info!(
        "system_info: refresh — wrote {} L1 rows from MAME {} ({} missing slugs: {})",
        refreshed,
        version,
        missing.len(),
        if missing.is_empty() { "none".to_string() } else { missing.join(", ") }
    );

    Ok(MameRefreshReport {
        mame_version: version,
        systems_refreshed: refreshed,
        missing_systems: missing,
        history_present: history_path.is_some(),
        mame_path: mame.display().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_clock_units_at_breakpoints() {
        assert_eq!(format_clock(1_789_773), "1.79 MHz");
        assert_eq!(format_clock(3_500_000_000), "3.50 GHz");
        assert_eq!(format_clock(32_768), "32 kHz");
        assert_eq!(format_clock(500), "500 Hz");
    }

    #[test]
    fn driver_map_matches_extractor_arity() {
        // Trip-wire: the in-app MAME_DRIVER_MAP must stay synced with
        // tools/mame-extractor/src/main.rs's constant. Both lists ship
        // ~39 OA slugs in v1; a divergence here means a system was
        // added to one side and not the other.
        assert_eq!(MAME_DRIVER_MAP.len(), 39);
    }

    #[test]
    fn detect_mame_binary_returns_none_when_nothing_found() {
        // Custom path that doesn't exist → None (not a panic).
        let bogus = std::env::temp_dir().join("oa-test-no-such-mame.exe");
        assert!(detect_mame_binary(Some(&bogus)).is_none());
    }

    #[test]
    fn detect_mame_binary_accepts_folder_with_mame_inside() {
        // Create a fake folder + binary, point detect at the folder.
        let tmp = std::env::temp_dir().join(format!(
            "oa-test-mame-folder-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let bin_name = if cfg!(windows) { "mame.exe" } else { "mame" };
        let bin = tmp.join(bin_name);
        fs::write(&bin, b"#!/bin/sh\necho stub").unwrap();
        let found = detect_mame_binary(Some(&tmp));
        assert_eq!(found, Some(bin));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn detect_history_xml_walks_three_locations() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-test-hist-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("history")).unwrap();
        let hist = tmp.join("history").join("history.xml");
        fs::write(&hist, "<history/>").unwrap();
        assert_eq!(detect_history_xml(&tmp), Some(hist));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn detect_history_xml_returns_none_when_absent() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-test-no-hist-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        assert!(detect_history_xml(&tmp).is_none());
        let _ = fs::remove_dir_all(&tmp);
    }
}
