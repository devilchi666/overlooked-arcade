//! ScummVM auto-detection via sentinel-filename heuristic.
//!
//! LaunchBox's ScummVM importer shells out to a standalone ScummVM
//! install's `--detect` CLI to fingerprint game directories and write
//! `.scummvm` descriptor files automatically. OA's approach: ship a
//! curated table of well-known SCUMM-engine + other-engine games with
//! unambiguous main-data-file signatures, so no external dependency is
//! needed at runtime. Operators with games outside the curated set
//! fill in the descriptor manually in the same UI (or hand-craft a
//! `.scummvm` file the way they did before this feature shipped).
//!
//! ## What's in the table
//!
//! High-confidence matches for the most-played classics:
//!
//! - **SCUMM v3-v8** — Monkey Island 1 & 2, Indiana Jones 3 & 4,
//!   Day of the Tentacle, Sam & Max Hit the Road, Full Throttle,
//!   The Dig, Curse of Monkey Island, Loom (CD), Zak McKracken
//!   (Enhanced).
//! - **Other engines (ScummVM freewares)** — Beneath a Steel Sky
//!   (sky engine), Flight of the Amazon Queen (queen engine),
//!   Lure of the Temptress (lure engine), Drascula (drascula
//!   engine), Soltys (cge engine).
//!
//! Floppy-original LFL-shape SCUMM games (Maniac Mansion, Loom floppy,
//! Zak floppy, Monkey Island floppy DOS) share the `00.LFL` / `01.LFL`
//! sentinel and aren't disambiguable from filename alone — those stay
//! manual. Sierra AGI/SCI titles (King's Quest, Space Quest, Police
//! Quest series) all use `resource.*` file patterns and are ambiguous
//! likewise.
//!
//! ## Extending the table
//!
//! When operators surface a missing game, add a row to `KNOWN_GAMES`
//! with a verified sentinel filename + the ScummVM-canonical
//! `gameid:engine` string. ScummVM compatibility list: https://www.scummvm.org/compatibility/

use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// One entry in the curated detection table.
#[derive(Debug, Clone, Copy)]
pub struct ScummvmGame {
    /// Filename to look for in the game directory. Match is
    /// case-insensitive against the directory's direct children
    /// (no subdir walk — keeps detection fast + avoids false
    /// positives from data files in nested asset folders).
    pub sentinel: &'static str,
    /// The single-line text content that goes into the `.scummvm`
    /// file. Format is `gameid:engineid` (e.g. `"monkey:scumm"`).
    /// ScummVM core opens the descriptor, reads the line, looks up
    /// the game in its built-in detection table, and loads the
    /// engine accordingly.
    pub descriptor: &'static str,
    /// Human label for the operator confirmation UI.
    pub label: &'static str,
    /// Short note shown next to the match — release year, version
    /// hint, freeware flag.
    pub notes: &'static str,
}

/// Curated detection table. Sentinels are unambiguous per their
/// game's data layout. Order is "popular SCUMM first" so any future
/// table-walk that returns "first match" gets the most-likely game
/// when overlap exists. (Today every sentinel is globally unique,
/// so order doesn't actually matter — but the convention helps.)
pub const KNOWN_GAMES: &[ScummvmGame] = &[
    // ── SCUMM v5 ─────────────────────────────────────────────────
    ScummvmGame {
        sentinel: "MONKEY.000",
        descriptor: "monkey:scumm",
        label: "The Secret of Monkey Island",
        notes: "1990 LucasArts · SCUMM v5 · CD/Enhanced",
    },
    ScummvmGame {
        sentinel: "MONKEY2.000",
        descriptor: "monkey2:scumm",
        label: "Monkey Island 2: LeChuck's Revenge",
        notes: "1991 LucasArts · SCUMM v5",
    },
    ScummvmGame {
        sentinel: "ATLANTIS.000",
        descriptor: "atlantis:scumm",
        label: "Indiana Jones and the Fate of Atlantis",
        notes: "1992 LucasArts · SCUMM v5",
    },
    ScummvmGame {
        sentinel: "LOOM.000",
        descriptor: "loomcd:scumm",
        label: "Loom (CD/Enhanced)",
        notes: "1990 LucasArts · SCUMM v4 · CD Talkie",
    },
    ScummvmGame {
        sentinel: "ZAK.000",
        descriptor: "zak:scumm",
        label: "Zak McKracken and the Alien Mindbenders (Enhanced)",
        notes: "1988 LucasArts · SCUMM v3 · FM Towns / Enhanced",
    },
    // ── SCUMM v6 ─────────────────────────────────────────────────
    ScummvmGame {
        sentinel: "TENTACLE.000",
        descriptor: "tentacle:scumm",
        label: "Day of the Tentacle",
        notes: "1993 LucasArts · SCUMM v6",
    },
    ScummvmGame {
        sentinel: "SAMNMAX.000",
        descriptor: "samnmax:scumm",
        label: "Sam & Max Hit the Road",
        notes: "1993 LucasArts · SCUMM v6",
    },
    // ── SCUMM v7 ─────────────────────────────────────────────────
    ScummvmGame {
        sentinel: "FT.LA0",
        descriptor: "ft:scumm",
        label: "Full Throttle",
        notes: "1995 LucasArts · SCUMM v7",
    },
    ScummvmGame {
        sentinel: "DIG.LA0",
        descriptor: "dig:scumm",
        label: "The Dig",
        notes: "1995 LucasArts · SCUMM v7",
    },
    // ── SCUMM v8 ─────────────────────────────────────────────────
    ScummvmGame {
        sentinel: "COMI.LA0",
        descriptor: "comi:scumm",
        label: "The Curse of Monkey Island",
        notes: "1997 LucasArts · SCUMM v8",
    },
    // ── SCUMM v3 (LFL-shape, unambiguous prefix) ─────────────────
    ScummvmGame {
        sentinel: "INDY3.LFL",
        descriptor: "indy3:scumm",
        label: "Indiana Jones and the Last Crusade",
        notes: "1989 LucasArts · SCUMM v3 · LFL files",
    },
    // ── Revolution Software (Beneath a Steel Sky — FREEWARE) ─────
    ScummvmGame {
        sentinel: "SKY.DNR",
        descriptor: "sky:sky",
        label: "Beneath a Steel Sky",
        notes: "1994 Revolution · Sky engine · FREEWARE",
    },
    ScummvmGame {
        sentinel: "SKY.DSK",
        descriptor: "sky:sky",
        label: "Beneath a Steel Sky",
        notes: "1994 Revolution · Sky engine · FREEWARE",
    },
    // ── Interactive Binary Illusions (Flight of the Amazon Queen — FREEWARE) ──
    ScummvmGame {
        sentinel: "QUEEN.1",
        descriptor: "queen:queen",
        label: "Flight of the Amazon Queen",
        notes: "1995 IBI · Queen engine · FREEWARE",
    },
    ScummvmGame {
        sentinel: "QUEEN.1C",
        descriptor: "queen:queen",
        label: "Flight of the Amazon Queen (CD)",
        notes: "1995 IBI · Queen engine · CD Talkie · FREEWARE",
    },
    // ── Revolution Software (Lure of the Temptress — FREEWARE) ───
    ScummvmGame {
        sentinel: "DISK1.VGA",
        descriptor: "lure:lure",
        label: "Lure of the Temptress",
        notes: "1992 Revolution · Lure engine · FREEWARE",
    },
    // ── Alcachofa Soft (Drascula — FREEWARE) ─────────────────────
    ScummvmGame {
        sentinel: "DRASCULA.000",
        descriptor: "drascula:drascula",
        label: "Drascula: The Vampire Strikes Back",
        notes: "1996 Alcachofa Soft · Drascula engine · FREEWARE",
    },
    // ── L.K. Avalon (Soltys — FREEWARE) ──────────────────────────
    ScummvmGame {
        sentinel: "VOL.CAT",
        descriptor: "soltys:cge",
        label: "Soltys",
        notes: "1997 L.K. Avalon · CGE engine · FREEWARE",
    },
];

/// Detect the ScummVM game inside a single directory by walking its
/// direct children for any sentinel-table match. Returns the first
/// match (sentinels are globally unique today, so "first" == "the
/// match"). Case-insensitive filename compare.
///
/// Scans only the directory's direct children — game data subdirs
/// (e.g. `MUSIC/`, `SAVES/`) are skipped. ScummVM games conventionally
/// keep all main data files at the top level of the game directory.
pub fn detect_in_directory(dir: &Path) -> Option<&'static ScummvmGame> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else { continue };
        if !file_type.is_file() {
            continue;
        }
        let Ok(filename_os) = entry.file_name().into_string() else { continue };
        for game in KNOWN_GAMES {
            if filename_os.eq_ignore_ascii_case(game.sentinel) {
                return Some(game);
            }
        }
    }
    None
}

/// One row in the detection report. Always populated per scanned
/// subdirectory (`matched` may be `None` for unrecognized games —
/// operator can fill in the descriptor manually in the UI).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionResult {
    /// Absolute path to the game directory.
    pub directory: PathBuf,
    /// Directory basename — operator-facing label, also the
    /// `.scummvm` filename stem.
    pub directory_name: String,
    /// `Some(...)` when the curated table matched; `None` when the
    /// directory had no recognized sentinel.
    pub matched: Option<MatchedGame>,
    /// Where the `.scummvm` descriptor would be written. Sits next
    /// to the game directory per the LaunchBox convention (e.g.
    /// `<library>/ScummVM/Monkey Island/` →
    /// `<library>/ScummVM/Monkey Island.scummvm`).
    pub descriptor_path: PathBuf,
    /// True if a `.scummvm` file already exists at
    /// `descriptor_path` — the UI flags these so operators don't
    /// accidentally overwrite an operator-curated descriptor.
    pub already_exists: bool,
}

/// The matched portion of a [`DetectionResult`], flattened for the
/// frontend (which doesn't need direct access to the static table
/// pointer — just the strings).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchedGame {
    pub descriptor: String,
    pub label: String,
    pub notes: String,
}

impl MatchedGame {
    fn from_static(g: &'static ScummvmGame) -> Self {
        Self {
            descriptor: g.descriptor.to_string(),
            label: g.label.to_string(),
            notes: g.notes.to_string(),
        }
    }
}

/// Walk `parent_dir` one level deep for subdirectories and run
/// [`detect_in_directory`] against each. Returns one
/// [`DetectionResult`] per subdir (matched or unmatched) sorted
/// alphabetically by directory name so the UI lists them
/// deterministically.
///
/// Hidden dirs (`.git/`, OS folders starting with `.`) are skipped.
/// Symlinks are followed (`entry.file_type().is_dir()` follows on
/// Windows; matches the existing scan_service convention).
pub fn detect_in_parent(parent_dir: &Path) -> io::Result<Vec<DetectionResult>> {
    let mut out: Vec<DetectionResult> = Vec::new();
    let entries = std::fs::read_dir(parent_dir)?;
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else { continue };
        if !file_type.is_dir() {
            continue;
        }
        let Ok(name) = entry.file_name().into_string() else { continue };
        if name.starts_with('.') {
            continue;
        }
        let dir_path = entry.path();
        let matched = detect_in_directory(&dir_path).map(MatchedGame::from_static);
        // Descriptor sits at <parent>/<dir_name>.scummvm — same dir
        // as the game directory itself, NEXT TO it (per LaunchBox
        // ScummVM importer convention).
        let descriptor_path = parent_dir.join(format!("{name}.scummvm"));
        let already_exists = descriptor_path.is_file();
        out.push(DetectionResult {
            directory: dir_path,
            directory_name: name,
            matched,
            descriptor_path,
            already_exists,
        });
    }
    out.sort_by(|a, b| {
        a.directory_name
            .to_ascii_lowercase()
            .cmp(&b.directory_name.to_ascii_lowercase())
    });
    Ok(out)
}

/// One descriptor-write instruction from the frontend. Operator
/// confirms the per-row `descriptor` (possibly editing an unmatched
/// row's manual entry) and OA writes the file at `path`.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DescriptorWrite {
    /// Absolute path to write to. Frontend supplies what came back
    /// in `DetectionResult.descriptor_path` so OA doesn't have to
    /// recompute (and any operator manual override of the location
    /// works automatically).
    pub path: PathBuf,
    /// `gameid:engineid` text to write as the file's single line.
    pub descriptor: String,
    /// True if OA should overwrite an existing file at `path`. UI
    /// defaults this to false for `already_exists` rows so operators
    /// have to explicitly opt in to clobber operator-curated files.
    pub overwrite: bool,
}

/// Write the operator-confirmed descriptors to disk. Returns the
/// count of files actually written (skipped-because-exists rows are
/// not counted; the frontend reads that distinction from the input
/// list's `overwrite` flag).
///
/// Failures on individual writes are logged + counted as
/// "not written" rather than aborting the batch — operator gets a
/// partial-success result and can retry failing rows.
pub fn write_descriptors(writes: &[DescriptorWrite]) -> usize {
    let mut written = 0;
    for w in writes {
        if w.path.is_file() && !w.overwrite {
            log::info!(
                "scummvm_detect: skip write — {} already exists",
                w.path.display()
            );
            continue;
        }
        // Single-line text content. ScummVM core reads the descriptor,
        // splits on `:`, looks up gameid → engine in its internal
        // detection table. No trailing newline needed but harmless.
        let body = format!("{}\n", w.descriptor.trim());
        match std::fs::write(&w.path, body.as_bytes()) {
            Ok(()) => {
                log::info!(
                    "scummvm_detect: wrote descriptor {} -> {}",
                    w.path.display(),
                    w.descriptor.trim()
                );
                written += 1;
            }
            Err(e) => {
                log::warn!(
                    "scummvm_detect: write {} failed: {e}",
                    w.path.display()
                );
            }
        }
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_tmp_dir(label: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "oa-scummvm-detect-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("mkdir tmp");
        p
    }

    #[test]
    fn detect_matches_monkey_island_sentinel() {
        let dir = fresh_tmp_dir("monkey");
        std::fs::write(dir.join("MONKEY.000"), b"x").expect("seed");
        std::fs::write(dir.join("MONKEY.001"), b"x").expect("seed");
        let g = detect_in_directory(&dir).expect("should match");
        assert_eq!(g.descriptor, "monkey:scumm");
        assert_eq!(g.label, "The Secret of Monkey Island");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detect_is_case_insensitive() {
        // Operators on case-sensitive filesystems (Linux, modern macOS
        // case-sensitive APFS) may have lowercase filenames. Detection
        // should match either case.
        let dir = fresh_tmp_dir("case-mix");
        std::fs::write(dir.join("tentacle.000"), b"x").expect("seed");
        let g = detect_in_directory(&dir).expect("should match");
        assert_eq!(g.descriptor, "tentacle:scumm");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detect_returns_none_for_unrecognized_dir() {
        let dir = fresh_tmp_dir("random");
        std::fs::write(dir.join("README.TXT"), b"x").expect("seed");
        std::fs::write(dir.join("install.exe"), b"x").expect("seed");
        assert!(detect_in_directory(&dir).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn detect_skips_subdirectory_files() {
        // Sentinel files in nested asset folders shouldn't trigger
        // detection — only top-level direct children count.
        let dir = fresh_tmp_dir("nested");
        std::fs::create_dir_all(dir.join("MUSIC")).expect("mkdir");
        std::fs::write(dir.join("MUSIC").join("MONKEY.000"), b"x").expect("seed");
        assert!(detect_in_directory(&dir).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn freeware_sky_dnr_matches() {
        // BASS ships with both SKY.DNR (data signature) and SKY.DSK
        // (the disk image). Either should trigger detection.
        let dir = fresh_tmp_dir("sky-dnr");
        std::fs::write(dir.join("SKY.DNR"), b"x").expect("seed");
        let g = detect_in_directory(&dir).expect("should match");
        assert_eq!(g.descriptor, "sky:sky");
        let _ = std::fs::remove_dir_all(&dir);

        let dir2 = fresh_tmp_dir("sky-dsk-only");
        std::fs::write(dir2.join("SKY.DSK"), b"x").expect("seed");
        let g2 = detect_in_directory(&dir2).expect("should match");
        assert_eq!(g2.descriptor, "sky:sky");
        let _ = std::fs::remove_dir_all(&dir2);
    }

    #[test]
    fn detect_in_parent_walks_one_level_deep() {
        // Standard operator layout: parent contains per-game
        // subdirectories. Should detect each subdir independently.
        let parent = fresh_tmp_dir("parent");
        let mi = parent.join("Monkey Island");
        std::fs::create_dir_all(&mi).expect("mkdir");
        std::fs::write(mi.join("MONKEY.000"), b"x").expect("seed");
        let dott = parent.join("Day of the Tentacle");
        std::fs::create_dir_all(&dott).expect("mkdir");
        std::fs::write(dott.join("TENTACLE.000"), b"x").expect("seed");
        let mystery = parent.join("Unknown Game");
        std::fs::create_dir_all(&mystery).expect("mkdir");
        std::fs::write(mystery.join("data.dat"), b"x").expect("seed");

        let results = detect_in_parent(&parent).expect("parent walk");
        assert_eq!(results.len(), 3);
        // Sorted alphabetically by directory name (case-insensitive).
        assert_eq!(results[0].directory_name, "Day of the Tentacle");
        assert_eq!(results[1].directory_name, "Monkey Island");
        assert_eq!(results[2].directory_name, "Unknown Game");
        assert_eq!(
            results[0].matched.as_ref().map(|m| m.descriptor.as_str()),
            Some("tentacle:scumm"),
        );
        assert_eq!(
            results[1].matched.as_ref().map(|m| m.descriptor.as_str()),
            Some("monkey:scumm"),
        );
        assert!(results[2].matched.is_none());
        // Descriptor paths sit NEXT TO each game dir.
        assert_eq!(results[1].descriptor_path, parent.join("Monkey Island.scummvm"));
        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn detect_in_parent_flags_existing_descriptors() {
        let parent = fresh_tmp_dir("existing-desc");
        let mi = parent.join("Monkey Island");
        std::fs::create_dir_all(&mi).expect("mkdir");
        std::fs::write(mi.join("MONKEY.000"), b"x").expect("seed");
        // Pre-existing descriptor — operator already created one.
        std::fs::write(parent.join("Monkey Island.scummvm"), b"monkey:scumm\n").expect("seed");
        let results = detect_in_parent(&parent).expect("parent walk");
        assert_eq!(results.len(), 1);
        assert!(results[0].already_exists);
        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn write_descriptors_writes_files_and_respects_overwrite() {
        let dir = fresh_tmp_dir("write-test");
        let path = dir.join("Monkey Island.scummvm");
        let writes = vec![DescriptorWrite {
            path: path.clone(),
            descriptor: "monkey:scumm".into(),
            overwrite: false,
        }];
        assert_eq!(write_descriptors(&writes), 1);
        let body = std::fs::read_to_string(&path).expect("read back");
        assert_eq!(body.trim(), "monkey:scumm");

        // Second call without overwrite — should skip.
        let writes2 = vec![DescriptorWrite {
            path: path.clone(),
            descriptor: "DIFFERENT:gameid".into(),
            overwrite: false,
        }];
        assert_eq!(write_descriptors(&writes2), 0);
        let body2 = std::fs::read_to_string(&path).expect("read back 2");
        assert_eq!(body2.trim(), "monkey:scumm", "file should be unchanged");

        // Third call WITH overwrite — should clobber.
        let writes3 = vec![DescriptorWrite {
            path: path.clone(),
            descriptor: "tentacle:scumm".into(),
            overwrite: true,
        }];
        assert_eq!(write_descriptors(&writes3), 1);
        let body3 = std::fs::read_to_string(&path).expect("read back 3");
        assert_eq!(body3.trim(), "tentacle:scumm");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn known_games_table_has_expected_landmark_entries() {
        let ids: Vec<&str> = KNOWN_GAMES.iter().map(|g| g.descriptor).collect();
        for expected in &[
            "monkey:scumm",
            "monkey2:scumm",
            "atlantis:scumm",
            "tentacle:scumm",
            "samnmax:scumm",
            "ft:scumm",
            "dig:scumm",
            "comi:scumm",
            "indy3:scumm",
            "sky:sky",
            "queen:queen",
            "lure:lure",
            "drascula:drascula",
            "soltys:cge",
        ] {
            assert!(ids.contains(expected), "missing landmark entry: {expected}");
        }
    }
}
