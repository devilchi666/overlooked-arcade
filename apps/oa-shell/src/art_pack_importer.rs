//! LaunchBox / EmuMovies art-pack importer.
//!
//! Operators point this at a folder of community-curated art (LaunchBox
//! Images dir, an EmuMovies download, a manually-built art bundle) and
//! the importer fuzzy-matches each art file's stem against library
//! titles, copying matches into the canonical LaunchBox-shape layout
//! at `media/<systemId>/<kind>/<rom_stem>.<ext>`.
//!
//! Two layouts supported:
//! - **Multi-platform** (most common): the source folder's direct
//!   children are platform names (`"Sega Genesis"`, `"Super Nintendo
//!   Entertainment System"`, ...). Each platform sub-folder contains
//!   kind sub-folders (`"Box - Front"`, `"Screenshot - Gameplay"`,
//!   ...) holding the art files. system_id is inferred from the
//!   platform folder name.
//! - **Single-platform**: the source folder's direct children ARE the
//!   kind folders. system_id is supplied by the operator via
//!   `system_id_override`.
//!
//! Importer uses `ingest_manual_for_slot` under the hood, so:
//! - Imported variants are marked `MediaSource::Manual`.
//! - The Phase 2 eviction logic kicks in: if a prior synced variant
//!   owns the primary path, it's renamed to `-02` before the imported
//!   art lands at primary.
//! - The fresh import always claims index 0 (operator's most-recent
//!   choice wins region-priority resolution).
//!
//! Dry-run mode runs the same scan + fuzzy-match logic but skips the
//! file writes + db mutations — operator can preview what would
//! land before committing.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::Emitter;

use crate::library_db::{GameRow, LibraryDb};
use crate::media::{
    ingest_manual_for_slot, rom_stem_from_path, write_media_db, MediaKind, MediaState,
};
use crate::normalize::{match_score, normalize_title};

/// Fuzzy match threshold for art-pack imports. Matches the libretro-
/// thumbnails sync (0.95) — high enough that "Sonic" doesn't catch
/// "Sonic 2". Identified ROMs would still match on exact stem; for
/// the unidentified case we rely on a tight fuzzy match.
const IMPORT_FUZZY_THRESHOLD: f64 = 0.95;

/// LaunchBox platform-folder name → OA `system_id`. Returns None for
/// folder names that don't correspond to a system OA currently
/// supports (those subfolders are silently skipped). Multiple
/// LaunchBox names can map to the same OA `system_id` (e.g. "Sony
/// Playstation" and "Sony PlayStation" both → `psx`; Wii art rides
/// `gamecube` because OA wraps both under Dolphin).
pub fn launchbox_platform_to_system_id(name: &str) -> Option<&'static str> {
    match name {
        "Sega Genesis"                              => Some("genesis"),
        "Sega Master System"                        => Some("sms"),
        "Sega Game Gear"                            => Some("gamegear"),
        "Sega 32X"                                  => Some("sega32x"),
        "Sega CD"                                   => Some("segacd"),
        "Sega Saturn"                               => Some("saturn"),
        "Sega Dreamcast"                            => Some("dreamcast"),
        "Nintendo Entertainment System"             => Some("nes"),
        "Super Nintendo Entertainment System"       => Some("snes"),
        "Nintendo 64"                               => Some("n64"),
        "Nintendo GameCube"                         => Some("gamecube"),
        // Wii art rides "gamecube" — both run on Dolphin under one OA system_id.
        "Nintendo Wii"                              => Some("gamecube"),
        "Nintendo Game Boy"                         => Some("gb"),
        "Nintendo Game Boy Color"                   => Some("gbc"),
        "Nintendo Game Boy Advance"                 => Some("gba"),
        "Nintendo DS"                               => Some("nds"),
        "Nintendo Virtual Boy"                      => Some("virtualboy"),
        "Nintendo Pokemon Mini"                     => Some("pokemini"),
        "Sony Playstation" | "Sony PlayStation"     => Some("psx"),
        "Sony Playstation 2" | "Sony PlayStation 2" => Some("ps2"),
        "Sony PSP"                                  => Some("psp"),
        "Atari 2600"                                => Some("2600"),
        "Atari 5200"                                => Some("5200"),
        "Atari 7800"                                => Some("atari7800"),
        "Atari Lynx"                                => Some("lynx"),
        "Atari Jaguar"                              => Some("jaguar"),
        "MAME"                                      => Some("mame"),
        "NEC TurboGrafx-16" | "NEC PC Engine"       => Some("tg16"),
        "NEC TurboGrafx-CD" | "NEC PC Engine CD"    => Some("pce-cd"),
        "NEC PC-FX"                                 => Some("pcfx"),
        "Bandai WonderSwan" | "Bandai WonderSwan Color" => Some("wonderswan"),
        "SNK Neo Geo" | "SNK Neo Geo AES" | "SNK Neo Geo MVS" => Some("neogeo"),
        "SNK Neo Geo CD"                            => Some("neocd"),
        "SNK Neo Geo Pocket" | "SNK Neo Geo Pocket Color" => Some("ngp"),
        "3DO Interactive Multiplayer"               => Some("3do"),
        "GCE Vectrex"                               => Some("vectrex"),
        "Mattel Intellivision"                      => Some("intv"),
        "Magnavox Odyssey 2"                        => Some("o2"),
        "Fairchild Channel F"                       => Some("channelf"),
        "Coleco ColecoVision"                       => Some("coleco"),
        "Microsoft MSX"                             => Some("msx"),
        "Microsoft MSX2"                            => Some("msx2"),
        _ => None,
    }
}

/// LaunchBox kind-folder name → MediaKind. Returns None for folder
/// names that don't match a known slot (silently skipped). The set
/// here mirrors the 26-slot LaunchBox taxonomy that landed in
/// MediaKind in Phase 1.
pub fn launchbox_kind_to_media_kind(name: &str) -> Option<MediaKind> {
    match name {
        "Box - Front"                       => Some(MediaKind::BoxFront),
        "Box - Back"                        => Some(MediaKind::BoxBack),
        "Box - 3D"                          => Some(MediaKind::Box3d),
        "Box - Spine"                       => Some(MediaKind::BoxSpine),
        "Box - Full"                        => Some(MediaKind::BoxFull),
        "Cart - Front"                      => Some(MediaKind::CartFront),
        "Cart - Back"                       => Some(MediaKind::CartBack),
        "Cart - 3D"                         => Some(MediaKind::Cart3d),
        "Disc"                              => Some(MediaKind::Disc),
        "Screenshot - Gameplay"             => Some(MediaKind::ScreenshotGameplay),
        "Screenshot - Game Title"           => Some(MediaKind::ScreenshotTitle),
        "Screenshot - Game Select"          => Some(MediaKind::ScreenshotSelect),
        "Banner"                            => Some(MediaKind::Banner),
        "Clear Logo"                        => Some(MediaKind::ClearLogo),
        "Fanart - Background"               => Some(MediaKind::FanartBackground),
        "Fanart - Disc"                     => Some(MediaKind::FanartDisc),
        "Advertisement Flyer - Front"       => Some(MediaKind::AdvertFront),
        "Advertisement Flyer - Back"        => Some(MediaKind::AdvertBack),
        "Arcade - Cabinet"                  => Some(MediaKind::ArcadeCabinet),
        "Arcade - Marquee"                  => Some(MediaKind::ArcadeMarquee),
        "Arcade - Control Panel"            => Some(MediaKind::ArcadeControlpanel),
        "Arcade - Controls Information"     => Some(MediaKind::ArcadeControlsinfo),
        "Arcade - Player Selection"         => Some(MediaKind::ArcadePlayerselect),
        "Advertisement Flyer"               => Some(MediaKind::ArcadeFlyer),
        "Manual"                            => Some(MediaKind::Manual),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub enum ArtPackLayout {
    /// Root contains kind dirs directly (e.g. operator pointed at a
    /// single-platform folder). `system_id` must come from the
    /// operator via `system_id_override`.
    SinglePlatform,
    /// Root contains platform dirs, each with kind dirs underneath.
    /// `system_id` is inferred per-platform from the dir name.
    MultiPlatform,
    /// Neither layout detected — operator pointed at the wrong folder
    /// or the folder structure isn't recognized.
    Unknown,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    /// "single-platform" | "multi-platform" | "unknown"
    pub layout: String,
    pub platforms: Vec<PlatformReport>,
    pub total_imported: usize,
    pub total_skipped_no_match: usize,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlatformReport {
    pub platform_dir: String,
    /// None when the operator pointed at a single-platform folder
    /// without supplying `system_id_override`.
    pub system_id: Option<String>,
    /// The LaunchBox folder name (for multi-platform layout). None for
    /// single-platform where we just used the supplied system_id.
    pub launchbox_name: Option<String>,
    pub library_entries: usize,
    pub by_kind: BTreeMap<String, KindReport>,
    pub total_imported: usize,
    pub total_skipped_no_match: usize,
    /// Set when the platform couldn't be processed (no system_id
    /// mapping, no library entries, scan I/O error). Other counters
    /// stay 0 in that case.
    pub error: Option<String>,
}

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct KindReport {
    pub kind: String,
    pub source_files: usize,
    pub imported: usize,
    pub skipped_no_match: usize,
}

/// Progress event payload — fired after each (platform × kind) batch
/// finishes so the UI can show "Importing Genesis box-front..." style
/// progress without N×M per-entry events.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ImportProgress {
    pub platform_index: usize,
    pub platform_total: usize,
    pub current_platform: String,
    pub current_kind: String,
    pub imported_in_kind: usize,
}

/// Walk a directory once, return (name, path) for each subdirectory.
/// Skips files, symlinks, and unreadable entries silently.
fn collect_subdirs(dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<(String, PathBuf)> = rd
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            if !p.is_dir() {
                return None;
            }
            e.file_name().to_str().map(|n| (n.to_string(), p))
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Walk a directory once, return PathBufs for png/jpg/jpeg/webp files.
fn collect_image_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    rd.flatten()
        .filter_map(|e| {
            let p = e.path();
            if !p.is_file() {
                return None;
            }
            let ext = p
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_ascii_lowercase());
            match ext.as_deref() {
                Some("png") | Some("jpg") | Some("jpeg") | Some("webp") => Some(p),
                _ => None,
            }
        })
        .collect()
}

/// Inspect the source dir's direct children. If at least one matches a
/// known kind-folder name, treat as SinglePlatform. Else if at least
/// one matches a known platform-folder name, treat as MultiPlatform.
/// Else Unknown.
pub fn classify_layout(source_dir: &Path) -> ArtPackLayout {
    let subdirs = collect_subdirs(source_dir);
    let has_kind = subdirs
        .iter()
        .any(|(n, _)| launchbox_kind_to_media_kind(n).is_some());
    if has_kind {
        return ArtPackLayout::SinglePlatform;
    }
    let has_platform = subdirs
        .iter()
        .any(|(n, _)| launchbox_platform_to_system_id(n).is_some());
    if has_platform {
        ArtPackLayout::MultiPlatform
    } else {
        ArtPackLayout::Unknown
    }
}

/// Filter library entries to those tagged with `system_id`. Used to
/// scope the fuzzy match — art-pack imports for Genesis only consider
/// Genesis library entries, never accidentally apply art to a Game Boy
/// game with a similar title.
fn entries_for_system(library: &LibraryDb, system_id: &str) -> Vec<GameRow> {
    library
        .list_games_for_system(system_id)
        .unwrap_or_default()
}

/// Run a single (platform × kind) batch. Returns the per-kind report.
/// Mutates `db` in place when `dry_run = false`; in dry-run mode just
/// counts what would happen without touching the filesystem or db.
fn import_one_kind(
    kind_dir: &Path,
    kind: MediaKind,
    library_entries: &[GameRow],
    lib_normalized: &[(usize, String)],
    state: &MediaState,
    dry_run: bool,
) -> KindReport {
    let art_files = collect_image_files(kind_dir);
    let source_files = art_files.len();
    let mut report = KindReport {
        kind: kind.as_str().to_string(),
        source_files,
        imported: 0,
        skipped_no_match: 0,
    };
    if art_files.is_empty() {
        return report;
    }

    // Precompute normalized stems for art files.
    let art_normalized: Vec<(String, PathBuf)> = art_files
        .iter()
        .filter_map(|p| {
            p.file_stem()
                .and_then(|s| s.to_str())
                .map(|stem| (normalize_title(stem), p.clone()))
        })
        .filter(|(n, _)| !n.is_empty())
        .collect();

    if art_normalized.is_empty() {
        return report;
    }

    // For each library entry, pick the best-matching art file above
    // threshold (highest fuzzy score). One library entry → at most one
    // imported art file per kind in this pass; the unmatched ones go
    // into skipped_no_match.
    for (entry_idx, entry_norm) in lib_normalized {
        let best: Option<(f64, &Path)> = art_normalized
            .iter()
            .map(|(an, ap)| (match_score(entry_norm, an), ap.as_path()))
            .filter(|(s, _)| *s >= IMPORT_FUZZY_THRESHOLD)
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let Some((_score, art_path)) = best else {
            report.skipped_no_match += 1;
            continue;
        };

        if dry_run {
            report.imported += 1;
            continue;
        }

        // Live import: route through ingest_manual_for_slot so Phase 2's
        // eviction logic kicks in (prior synced variant at primary
        // moves to -02 before the imported art claims primary).
        let entry = &library_entries[*entry_idx];
        let raw_path = entry.archive_inner_path.as_deref().unwrap_or(&entry.file_path);
        let rom_stem = rom_stem_from_path(raw_path);

        let r = {
            let mut db = match state.db.write() {
                Ok(g) => g,
                Err(_) => {
                    log::warn!("art-pack import: media db lock poisoned, aborting kind");
                    return report;
                }
            };
            ingest_manual_for_slot(
                &state.app_data_dir,
                &rom_stem,
                &entry.system_id,
                kind,
                art_path,
                &mut db,
                &entry.id,
            )
        };

        match r {
            Ok(_) => report.imported += 1,
            Err(e) => {
                log::warn!(
                    "art-pack import: {} [{}] failed: {e}",
                    entry.title,
                    kind.as_str()
                );
                // Counted as not-imported (skipped) so the report is
                // honest about the file landing or not. Detailed
                // errors live in the log; the UI surfaces aggregate
                // counts.
                report.skipped_no_match += 1;
            }
        }
    }

    report
}

/// Run an import for one platform folder. Iterates the kind dirs
/// inside, calling `import_one_kind` for each recognized one. Emits
/// per-kind progress events. Returns the per-platform report.
fn import_one_platform(
    platform_dir: &Path,
    system_id: &str,
    launchbox_name: Option<&str>,
    library: &LibraryDb,
    state: &MediaState,
    dry_run: bool,
    platform_index: usize,
    platform_total: usize,
    app: Option<&tauri::AppHandle>,
) -> PlatformReport {
    let mut report = PlatformReport {
        platform_dir: platform_dir.display().to_string(),
        system_id: Some(system_id.to_string()),
        launchbox_name: launchbox_name.map(|s| s.to_string()),
        library_entries: 0,
        by_kind: BTreeMap::new(),
        total_imported: 0,
        total_skipped_no_match: 0,
        error: None,
    };

    let entries = entries_for_system(library, system_id);
    report.library_entries = entries.len();
    if entries.is_empty() {
        report.error = Some(format!(
            "no library entries for system '{system_id}' — nothing to match against",
        ));
        return report;
    }

    let lib_normalized: Vec<(usize, String)> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| (i, normalize_title(&e.title)))
        .filter(|(_, n)| !n.is_empty())
        .collect();

    for (kind_name, kind_dir) in collect_subdirs(platform_dir) {
        let Some(kind) = launchbox_kind_to_media_kind(&kind_name) else {
            continue;
        };
        let kind_report = import_one_kind(
            &kind_dir,
            kind,
            &entries,
            &lib_normalized,
            state,
            dry_run,
        );
        report.total_imported += kind_report.imported;
        report.total_skipped_no_match += kind_report.skipped_no_match;

        if let Some(app) = app {
            let _ = app.emit(
                "oa://art-pack-progress",
                &ImportProgress {
                    platform_index,
                    platform_total,
                    current_platform: launchbox_name
                        .unwrap_or(system_id)
                        .to_string(),
                    current_kind: kind.as_str().to_string(),
                    imported_in_kind: kind_report.imported,
                },
            );
        }

        report.by_kind.insert(kind.as_str().to_string(), kind_report);
    }

    report
}

/// Top-level import entry point. Walks the source dir, classifies
/// layout, runs per-platform imports, and returns a structured report.
/// Live-mode persists the MediaDb to disk at the end (one flush, not
/// per entry).
#[tauri::command]
#[allow(non_snake_case)]
pub async fn import_art_pack(
    sourceDir: String,
    systemIdOverride: Option<String>,
    dryRun: bool,
    state: tauri::State<'_, MediaState>,
    library: tauri::State<'_, LibraryDb>,
    app: tauri::AppHandle,
) -> Result<ImportReport, String> {
    let src = PathBuf::from(&sourceDir);
    if !src.is_dir() {
        return Err(format!("source dir not found: {sourceDir}"));
    }
    log::info!(
        "art-pack import: source={} dry_run={dryRun} system_override={:?}",
        src.display(),
        systemIdOverride,
    );

    let layout = classify_layout(&src);
    let layout_label = match layout {
        ArtPackLayout::SinglePlatform => "single-platform",
        ArtPackLayout::MultiPlatform => "multi-platform",
        ArtPackLayout::Unknown => "unknown",
    };

    let mut platforms: Vec<PlatformReport> = Vec::new();

    match layout {
        ArtPackLayout::SinglePlatform => {
            let Some(system_id) = systemIdOverride.as_deref() else {
                platforms.push(PlatformReport {
                    platform_dir: src.display().to_string(),
                    system_id: None,
                    launchbox_name: None,
                    library_entries: 0,
                    by_kind: BTreeMap::new(),
                    total_imported: 0,
                    total_skipped_no_match: 0,
                    error: Some(
                        "single-platform layout requires system_id_override".to_string(),
                    ),
                });
                return Ok(ImportReport {
                    layout: layout_label.to_string(),
                    platforms,
                    total_imported: 0,
                    total_skipped_no_match: 0,
                });
            };
            let report = import_one_platform(
                &src,
                system_id,
                None,
                &library,
                &state,
                dryRun,
                0,
                1,
                Some(&app),
            );
            platforms.push(report);
        }
        ArtPackLayout::MultiPlatform => {
            let subs = collect_subdirs(&src);
            let recognized: Vec<(String, &'static str, PathBuf)> = subs
                .into_iter()
                .filter_map(|(n, p)| {
                    launchbox_platform_to_system_id(&n).map(|sid| (n, sid, p))
                })
                .collect();
            let total = recognized.len();
            for (idx, (lb_name, system_id, platform_dir)) in recognized.iter().enumerate() {
                let report = import_one_platform(
                    platform_dir,
                    system_id,
                    Some(lb_name),
                    &library,
                    &state,
                    dryRun,
                    idx,
                    total,
                    Some(&app),
                );
                platforms.push(report);
            }
        }
        ArtPackLayout::Unknown => {
            platforms.push(PlatformReport {
                platform_dir: src.display().to_string(),
                system_id: None,
                launchbox_name: None,
                library_entries: 0,
                by_kind: BTreeMap::new(),
                total_imported: 0,
                total_skipped_no_match: 0,
                error: Some(
                    "source folder doesn't look like a LaunchBox / EmuMovies art pack \
                     (no recognized kind or platform subfolders)"
                        .to_string(),
                ),
            });
        }
    }

    let total_imported: usize = platforms.iter().map(|p| p.total_imported).sum();
    let total_skipped_no_match: usize = platforms.iter().map(|p| p.total_skipped_no_match).sum();

    // Live-mode flush — one media.json write covers all the per-entry
    // mutations done by ingest_manual_for_slot above. Pre-flush risk:
    // a crash mid-import would lose the in-memory variants we just
    // ingested. Acceptable trade-off vs writing media.json once per
    // entry (multi-MB write × N at scale).
    if !dryRun && total_imported > 0 {
        if let Ok(db) = state.db.read() {
            if let Err(e) = write_media_db(&state.app_data_dir, &db) {
                log::warn!("art-pack import: media.json flush failed: {e}");
            }
        }
        // Emit a media-updated batch so the frontend re-pulls touched
        // entries. Cheap to over-fire — frontend's MediaProvider just
        // refetches the get_media_index snapshot.
        let _ = app.emit(
            "oa://media-updated",
            serde_json::json!({ "batch": true, "count": total_imported }),
        );
    }

    log::info!(
        "art-pack import: done — layout={layout_label} platforms={} imported={total_imported} skipped={total_skipped_no_match}",
        platforms.len(),
    );

    Ok(ImportReport {
        layout: layout_label.to_string(),
        platforms,
        total_imported,
        total_skipped_no_match,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launchbox_platform_known_names_map_correctly() {
        assert_eq!(launchbox_platform_to_system_id("Sega Genesis"), Some("genesis"));
        assert_eq!(launchbox_platform_to_system_id("Super Nintendo Entertainment System"), Some("snes"));
        assert_eq!(launchbox_platform_to_system_id("Atari 7800"), Some("atari7800"));
        assert_eq!(launchbox_platform_to_system_id("MAME"), Some("mame"));
        assert_eq!(launchbox_platform_to_system_id("Bandai WonderSwan"), Some("wonderswan"));
        assert_eq!(launchbox_platform_to_system_id("Bandai WonderSwan Color"), Some("wonderswan"));
    }

    #[test]
    fn launchbox_platform_handles_capitalization_variants() {
        // LaunchBox's own data has both spellings in the wild.
        assert_eq!(launchbox_platform_to_system_id("Sony Playstation"), Some("psx"));
        assert_eq!(launchbox_platform_to_system_id("Sony PlayStation"), Some("psx"));
        assert_eq!(launchbox_platform_to_system_id("Sony Playstation 2"), Some("ps2"));
        assert_eq!(launchbox_platform_to_system_id("Sony PlayStation 2"), Some("ps2"));
    }

    #[test]
    fn launchbox_platform_unknown_returns_none() {
        assert_eq!(launchbox_platform_to_system_id("Made Up System"), None);
        assert_eq!(launchbox_platform_to_system_id(""), None);
        // Case sensitive — typo-protected.
        assert_eq!(launchbox_platform_to_system_id("sega genesis"), None);
    }

    #[test]
    fn launchbox_kind_known_names_map_correctly() {
        assert_eq!(launchbox_kind_to_media_kind("Box - Front"), Some(MediaKind::BoxFront));
        assert_eq!(launchbox_kind_to_media_kind("Screenshot - Gameplay"), Some(MediaKind::ScreenshotGameplay));
        assert_eq!(launchbox_kind_to_media_kind("Clear Logo"), Some(MediaKind::ClearLogo));
        assert_eq!(launchbox_kind_to_media_kind("Manual"), Some(MediaKind::Manual));
        assert_eq!(launchbox_kind_to_media_kind("Arcade - Marquee"), Some(MediaKind::ArcadeMarquee));
    }

    #[test]
    fn launchbox_kind_unknown_returns_none() {
        assert_eq!(launchbox_kind_to_media_kind("Made Up Kind"), None);
        assert_eq!(launchbox_kind_to_media_kind(""), None);
        // Case sensitive.
        assert_eq!(launchbox_kind_to_media_kind("box - front"), None);
    }

    /// Create a fresh tmp dir for filesystem-based layout tests.
    fn fresh_tmp_dir(label: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "oa-apimport-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&p).expect("mkdir tmp");
        p
    }

    #[test]
    fn classify_layout_detects_single_platform() {
        let root = fresh_tmp_dir("classify-single");
        std::fs::create_dir_all(root.join("Box - Front")).expect("mkdir kind dir");
        std::fs::create_dir_all(root.join("Screenshot - Gameplay")).expect("mkdir kind dir");
        std::fs::create_dir_all(root.join("Some Random Dir")).expect("mkdir unknown dir");
        assert!(matches!(classify_layout(&root), ArtPackLayout::SinglePlatform));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn classify_layout_detects_multi_platform() {
        let root = fresh_tmp_dir("classify-multi");
        std::fs::create_dir_all(root.join("Sega Genesis").join("Box - Front")).expect("mkdir");
        std::fs::create_dir_all(root.join("Super Nintendo Entertainment System").join("Clear Logo")).expect("mkdir");
        assert!(matches!(classify_layout(&root), ArtPackLayout::MultiPlatform));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn classify_layout_unknown_when_no_recognized_subfolders() {
        let root = fresh_tmp_dir("classify-unknown");
        std::fs::create_dir_all(root.join("Random Folder")).expect("mkdir");
        std::fs::create_dir_all(root.join("Another One")).expect("mkdir");
        assert!(matches!(classify_layout(&root), ArtPackLayout::Unknown));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn classify_layout_prefers_single_when_root_has_kind_dirs() {
        // Edge case: root has BOTH a recognized platform name AND a
        // recognized kind name as direct children. The kind name takes
        // priority (single-platform interpretation).
        let root = fresh_tmp_dir("classify-mixed");
        std::fs::create_dir_all(root.join("Box - Front")).expect("mkdir kind");
        std::fs::create_dir_all(root.join("Sega Genesis")).expect("mkdir platform");
        assert!(matches!(classify_layout(&root), ArtPackLayout::SinglePlatform));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn collect_image_files_filters_extensions() {
        let dir = fresh_tmp_dir("collect-img");
        std::fs::write(dir.join("a.png"), b"x").expect("png");
        std::fs::write(dir.join("b.jpg"), b"x").expect("jpg");
        std::fs::write(dir.join("c.webp"), b"x").expect("webp");
        std::fs::write(dir.join("d.JPEG"), b"x").expect("upper-jpeg");
        std::fs::write(dir.join("readme.txt"), b"x").expect("txt");
        std::fs::write(dir.join("art.psd"), b"x").expect("psd");
        std::fs::create_dir_all(dir.join("subdir")).expect("mkdir");
        let files = collect_image_files(&dir);
        assert_eq!(files.len(), 4); // png, jpg, webp, JPEG
        let _ = std::fs::remove_dir_all(&dir);
    }
}
