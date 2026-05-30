//! Data-dir resolver — single source of truth for where OA user state lives.
//!
//! Two modes:
//! - **Portable**: a `portable.txt` marker file next to `oa-shell.exe` opts in;
//!   all user state lives under `<exe_dir>/settings/`. The whole install can
//!   be copied between machines or onto a USB stick as a unit. Matches the
//!   existing cores/BIOS-next-to-exe pattern.
//! - **AppData** (default): falls back to Tauri's `app_data_dir()` — on
//!   Windows `%APPDATA%\dev.overlookedarcade.shell\`.
//!
//! Same binary supports both modes. The marker file is the entire opt-in.
//! AppData → portable migration is handled separately (see Phase 3).

use std::path::{Path, PathBuf};

use tauri::Manager;

/// Marker filename next to oa-shell.exe that opts the install into portable mode.
pub const MARKER_FILENAME: &str = "portable.txt";

/// Subdirectory of <exe_dir> where user state lives in portable mode.
pub const PORTABLE_SUBDIR: &str = "settings";

/// Sentinel filename written inside the portable dir after a successful
/// migration from AppData. Presence means "migration already ran; do not
/// re-overwrite portable state from AppData even if portable looks empty."
const MIGRATION_SENTINEL: &str = ".migrated-from-appdata";

/// Tauri 2 bundle identifier — must match `tauri.conf.json` `identifier`.
/// Used to derive the AppData fallback path when no Tauri app handle is
/// available (headless CLI mode).
pub(crate) const TAURI_IDENTIFIER: &str = "dev.overlookedarcade.shell";

/// Resolved data-dir + which mode produced it. Callers that need to widen
/// the asset-protocol scope or log differently key off `portable`.
#[derive(Debug, Clone)]
pub struct DataDir {
    pub path: PathBuf,
    pub portable: bool,
}

/// Resolve where OA reads/writes user state. Tries portable mode first;
/// falls back to AppData if no marker is present.
///
/// `app: Some(&tauri::App)` — GUI mode; uses Tauri's `app_data_dir()` for
/// the AppData fallback (handles per-platform conventions natively).
///
/// `app: None` — headless / CLI mode; uses the manual env-var fallback
/// (same per-OS logic Tauri uses internally).
pub fn resolve_data_dir(app: Option<&tauri::App>) -> Result<DataDir, String> {
    if let Some(portable_dir) = detect_portable_dir() {
        // Ensure the portable settings/ dir exists. If we can't create it,
        // the install folder isn't writable (e.g. Program Files, read-only
        // network share) — log loudly and fall back to AppData rather than
        // silently corrupt state.
        match std::fs::create_dir_all(&portable_dir) {
            Ok(_) => {
                migrate_from_appdata_if_needed(&portable_dir);
                return Ok(DataDir { path: portable_dir, portable: true });
            }
            Err(e) => log::error!(
                "oa-shell: portable.txt detected but {} not writable ({e}); \
                 falling back to AppData. Move the install to a writable \
                 folder (Documents, Desktop, USB drive, etc.) to enable \
                 portable mode.",
                portable_dir.display()
            ),
        }
    }

    let appdata = match app {
        Some(app) => app
            .path()
            .app_data_dir()
            .map_err(|e| format!("Tauri app_data_dir() failed: {e:?}"))?,
        None => resolve_appdata_manually()?,
    };
    Ok(DataDir { path: appdata, portable: false })
}

/// Returns `Some(<exe_dir>/settings)` if the portable marker is present
/// next to the running .exe.
fn detect_portable_dir() -> Option<PathBuf> {
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    portable_dir_for_exe_dir(&exe_dir)
}

/// Pure function — given an exe_dir, return the portable settings dir if
/// the marker is present. Split out from `detect_portable_dir` so unit
/// tests can pass a tempdir.
fn portable_dir_for_exe_dir(exe_dir: &Path) -> Option<PathBuf> {
    if exe_dir.join(MARKER_FILENAME).exists() {
        Some(exe_dir.join(PORTABLE_SUBDIR))
    } else {
        None
    }
}

/// One-shot migration from the would-be AppData location → portable dir.
///
/// Runs on every portable-mode launch but exits early if any of:
/// - migration sentinel already exists (we ran before)
/// - AppData dir doesn't exist or is empty (nothing to migrate)
/// - portable dir already has user content (operator was already using
///   portable mode independently; don't overwrite)
///
/// On success, writes `.migrated-from-appdata` so subsequent launches skip
/// even if the operator manually empties the portable dir later. The
/// AppData dir is left in place — operator can delete it once they're
/// confident the migration took.
///
/// All errors logged, never propagated — migration failure should not
/// block app launch.
fn migrate_from_appdata_if_needed(portable_dir: &Path) {
    let sentinel = portable_dir.join(MIGRATION_SENTINEL);
    if sentinel.exists() {
        return;
    }
    if portable_has_user_content(portable_dir) {
        // First-portable-launch but operator already populated it without
        // running migration. Write the sentinel so we don't trip on this
        // every launch.
        if let Err(e) = std::fs::write(&sentinel, "skipped: portable dir already populated\n") {
            log::warn!("oa-shell: failed to write migration sentinel: {e}");
        }
        return;
    }
    let Ok(appdata) = resolve_appdata_manually() else {
        return;
    };
    if !appdata.is_dir() {
        return;
    }
    let Ok(mut entries) = std::fs::read_dir(&appdata) else {
        return;
    };
    if entries.next().is_none() {
        return; // empty AppData
    }

    log::info!(
        "oa-shell: migrating AppData → portable: {} → {}",
        appdata.display(),
        portable_dir.display()
    );
    match copy_dir_recursive(&appdata, portable_dir) {
        Ok((files, bytes)) => {
            log::info!(
                "oa-shell: migration complete: {files} files, {bytes} bytes copied. \
                 Original AppData left intact at {}; delete manually once verified.",
                appdata.display()
            );
            if let Err(e) = std::fs::write(&sentinel, format!("migrated {files} files / {bytes} bytes\n")) {
                log::warn!("oa-shell: failed to write migration sentinel: {e}");
            }
        }
        Err(e) => {
            log::error!(
                "oa-shell: migration failed mid-copy: {e}. Portable dir left as-is; \
                 next launch will retry. Investigate before continuing."
            );
        }
    }
}

/// True if `portable_dir` contains anything other than the marker file's
/// sibling (we don't expect any files in `portable_dir` itself on first
/// portable launch — the marker lives in the parent `<exe_dir>`).
fn portable_has_user_content(portable_dir: &Path) -> bool {
    match std::fs::read_dir(portable_dir) {
        Ok(mut entries) => entries.any(|e| {
            e.ok()
                .map(|e| e.file_name() != MIGRATION_SENTINEL)
                .unwrap_or(false)
        }),
        Err(_) => false,
    }
}

/// Recursive directory copy. Returns (file count, total bytes copied) on
/// success. Pure std::fs — no walkdir dep needed for a one-shot migration.
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<(u64, u64)> {
    std::fs::create_dir_all(dst)?;
    let mut files = 0u64;
    let mut bytes = 0u64;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let ft = entry.file_type()?;
        if ft.is_dir() {
            let (f, b) = copy_dir_recursive(&src_path, &dst_path)?;
            files += f;
            bytes += b;
        } else if ft.is_file() {
            let copied = std::fs::copy(&src_path, &dst_path)?;
            files += 1;
            bytes += copied;
        }
        // Symlinks and other special files are skipped — none of our
        // user-state subsystems write them.
    }
    Ok((files, bytes))
}

/// Per-OS manual AppData resolution. Mirrors what Tauri does internally,
/// used in headless mode where no Tauri app handle is available.
pub(crate) fn resolve_appdata_manually() -> Result<PathBuf, String> {
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

// ---- Media-taxonomy Phase 5 migration ----
//
// Pre-2026-05-23 layout used opaque djb2-hashed filenames:
//   - Manual:  media/covers/<systemId>/rom-<hash>.<ext>
//   - Synced:  media/cache/libretro-thumbnails/<systemId>/Named_X/<filename>
//
// Post-Phase-1 layout uses human-readable rom-stem filenames in a
// kind-folder shape:
//   - All variants:  media/<systemId>/<kind>/<rom_stem>.<ext>
//
// The data model deserializes old media.json forward via serde aliases
// (Phase 1) — the variants land in the right slots in memory. What's
// still broken on an upgraded install is the variant.path strings
// (point at the old paths) + the files-on-disk (sit at the old
// locations). This migration walks the MediaDb, moves/copies files,
// rewrites paths, and writes the new media.json + a sentinel so the
// migration runs at most once per install.

/// Sentinel filename written inside `<data_dir>` after a successful
/// media-taxonomy migration. Presence means "migration already ran;
/// skip it on subsequent launches." Distinct from the AppData→portable
/// migration sentinel `.migrated-from-appdata`.
const MEDIA_TAXONOMY_SENTINEL: &str = ".media-taxonomy-migrated";

/// Summary returned by `migrate_media_naming`. Logged at INFO so
/// operators can see what happened; structured for any future Tauri
/// command that surfaces the report in the UI.
#[derive(Debug, Default, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaTaxonomyMigrationReport {
    /// Manual variants whose files were renamed from media/covers/...
    /// to the new canonical layout.
    pub manual_renamed: usize,
    /// Synced variants whose files were copied from
    /// media/cache/libretro-thumbnails/... to the new canonical
    /// layout. Cache file kept in place so future re-syncs use the
    /// fast-path cache check.
    pub synced_copied: usize,
    /// Variants whose path was already in the new format — left
    /// untouched.
    pub skipped_already_new: usize,
    /// Variants whose rom_id wasn't found in library_db (orphan
    /// MediaDb entry). Path left as-is, file left as-is — operator
    /// can clean these out via media folder browse.
    pub skipped_lookup_failed: usize,
    /// Variants whose source file didn't exist at the old path
    /// (operator manually deleted, disk error). Path updated to the
    /// new target anyway so a future operator drop lands cleanly.
    pub skipped_file_missing: usize,
    /// True if the sentinel was already present at function entry
    /// (no work performed, no media.json write).
    pub already_migrated: bool,
}

/// One-shot media-taxonomy migration. Walks the in-memory MediaDb
/// (passed `&mut` so the caller's MediaState gets the migrated state
/// without a re-read), and for any variant whose `path` is in the
/// pre-2026-05-23 format:
///
/// - Looks up the rom's `file_path` via `library_db.find_game_by_id`
///   to derive `rom_stem` (the new filename base).
/// - Builds the canonical target path
///   `media/<system_id>/<kind>/<rom_stem>{-NN}.<ext>` via the same
///   `next_variant_filename` helper the live sync uses, so the
///   operator-art-wins guard kicks in for migrated data the same
///   way it does for new sync runs.
/// - Manual variants (MediaSource::Manual) → file renamed (moved).
/// - Synced variants (MediaSource::LibretroThumbnails) → file copied
///   (the cache stays for the fast-path cache check in future
///   re-syncs).
/// - variant.path is rewritten in memory.
///
/// Sentinel-guarded: if `<data_dir>/.media-taxonomy-migrated` exists,
/// no-op (the input `db` is left untouched). Sentinel is written
/// after a successful media.json write — so a crash mid-migration
/// leaves the next launch retrying.
///
/// All errors logged + counted; never propagated. Migration failure
/// should never block app launch.
pub fn migrate_media_naming(
    data_dir: &Path,
    library: &crate::library_db::LibraryDb,
    db: &mut crate::media::MediaDb,
) -> MediaTaxonomyMigrationReport {
    let sentinel = data_dir.join(MEDIA_TAXONOMY_SENTINEL);
    if sentinel.exists() {
        return MediaTaxonomyMigrationReport {
            already_migrated: true,
            ..Default::default()
        };
    }

    let mut report = MediaTaxonomyMigrationReport::default();
    if db.is_empty() {
        log::info!(
            "media-taxonomy migration: media.json empty; writing sentinel and skipping"
        );
        write_sentinel(&sentinel, &report);
        return report;
    }

    let mut any_changes = false;
    // Iterate every rom_id; for each, walk every MediaKind slot.
    let rom_ids: Vec<String> = db.keys().cloned().collect();
    for rom_id in &rom_ids {
        // Resolve rom_stem + system_id once per rom_id.
        let row = match library.find_game_by_id(rom_id) {
            Ok(Some(r)) => r,
            _ => {
                // Orphan MediaDb entry. Count every old-layout variant
                // in this rom as "lookup failed" so the operator can
                // see what's been left behind.
                if let Some(gm) = db.get(rom_id) {
                    for &kind in crate::media::MediaKind::ALL {
                        for v in kind.variants(gm) {
                            if classify_old_path(&v.path).is_some() {
                                report.skipped_lookup_failed += 1;
                            }
                        }
                    }
                }
                continue;
            }
        };
        let raw = row.archive_inner_path.as_deref().unwrap_or(&row.file_path);
        let rom_stem = crate::media::rom_stem_from_path(raw);
        let system_id = row.system_id.clone();

        // Snapshot the list of slots that have at least one variant
        // needing work, to avoid borrowing db both immutably + mutably
        // in the same loop.
        let slot_actions = plan_slot_migrations(
            db.get(rom_id).expect("rom_id present"),
            &system_id,
        );

        if slot_actions.is_empty() {
            continue;
        }

        for action in slot_actions {
            match action {
                VariantAction::AlreadyNew { .. } => {
                    report.skipped_already_new += 1;
                }
                VariantAction::Migrate { kind, variant_idx, old_path, source } => {
                    // Perform the file op + path rewrite.
                    let ext = std::path::Path::new(&old_path)
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("png");
                    let new_rel = match crate::media::next_sync_path_for_slot(
                        data_dir, &system_id, kind, &rom_stem, ext,
                    ) {
                        Ok(p) => p,
                        Err(e) => {
                            log::warn!(
                                "media-taxonomy migration: next_sync_path_for_slot {} [{}] failed: {e}",
                                rom_id, kind.as_str()
                            );
                            continue;
                        }
                    };
                    let old_abs = data_dir.join(&old_path);
                    let new_abs = data_dir.join(&new_rel);
                    if !old_abs.is_file() {
                        // Path was old-format but file gone (operator
                        // cleaned out covers manually). Still update
                        // variant.path so a future drop at the new
                        // location is reachable.
                        report.skipped_file_missing += 1;
                        if let Some(gm) = db.get_mut(rom_id) {
                            if let Some(v) = kind.variants_mut(gm).get_mut(variant_idx) {
                                v.path = new_rel;
                                any_changes = true;
                            }
                        }
                        continue;
                    }
                    let op_result = match source {
                        crate::media::MediaSource::Manual => {
                            // Move — operator's manual file has no cache copy.
                            std::fs::rename(&old_abs, &new_abs).map(|_| "renamed")
                        }
                        crate::media::MediaSource::LibretroThumbnails => {
                            // Copy — keep the cache copy at its old path so
                            // future re-syncs use the fast-path cache check.
                            std::fs::copy(&old_abs, &new_abs).map(|_| "copied")
                        }
                    };
                    match op_result {
                        Ok(verb) => {
                            log::info!(
                                "media-taxonomy migration: {} {} -> {}",
                                verb, old_path, new_rel
                            );
                            if let Some(gm) = db.get_mut(rom_id) {
                                if let Some(v) = kind.variants_mut(gm).get_mut(variant_idx) {
                                    v.path = new_rel;
                                    any_changes = true;
                                }
                            }
                            match source {
                                crate::media::MediaSource::Manual => report.manual_renamed += 1,
                                crate::media::MediaSource::LibretroThumbnails => report.synced_copied += 1,
                            }
                        }
                        Err(e) => {
                            log::warn!(
                                "media-taxonomy migration: file op {} -> {} failed: {e}",
                                old_path, new_rel
                            );
                        }
                    }
                }
            }
        }
    }

    if any_changes {
        if let Err(e) = crate::media::write_media_db(data_dir, db) {
            log::warn!(
                "media-taxonomy migration: media.json flush failed: {e}; \
                 sentinel NOT written, next launch will retry"
            );
            return report;
        }
    }
    write_sentinel(&sentinel, &report);
    log::info!(
        "media-taxonomy migration: done — manual renamed={} synced copied={} \
         already-new={} lookup-failed={} file-missing={}",
        report.manual_renamed, report.synced_copied,
        report.skipped_already_new, report.skipped_lookup_failed,
        report.skipped_file_missing,
    );
    report
}

/// Per-variant migration action. Computed up-front (read-only db
/// access) and applied later (write-only db access) to dodge the
/// "borrowed both immutably and mutably" issue in one pass.
enum VariantAction {
    AlreadyNew {
        #[allow(dead_code)]
        kind: crate::media::MediaKind,
    },
    Migrate {
        kind: crate::media::MediaKind,
        variant_idx: usize,
        old_path: String,
        source: crate::media::MediaSource,
    },
}

/// Detect old-layout paths. Returns the legacy-format tag for
/// diagnostics; None means the path is already in (or compatible
/// with) the new layout.
fn classify_old_path(path: &str) -> Option<&'static str> {
    if path.starts_with("media/covers/") {
        Some("manual-v1")
    } else if path.starts_with("media/cache/libretro-thumbnails/") {
        Some("synced-v1")
    } else {
        None
    }
}

/// Walk every slot of a GameMedia, returning the action list for
/// variants that need migrating. `_system_id` reserved for future use
/// when we want per-system gating.
fn plan_slot_migrations(
    gm: &crate::media::GameMedia,
    _system_id: &str,
) -> Vec<VariantAction> {
    let mut out: Vec<VariantAction> = Vec::new();
    for &kind in crate::media::MediaKind::ALL {
        for (idx, v) in kind.variants(gm).iter().enumerate() {
            match classify_old_path(&v.path) {
                None => out.push(VariantAction::AlreadyNew { kind }),
                Some(_) => out.push(VariantAction::Migrate {
                    kind,
                    variant_idx: idx,
                    old_path: v.path.clone(),
                    source: v.source.clone(),
                }),
            }
        }
    }
    out
}

fn write_sentinel(sentinel: &Path, report: &MediaTaxonomyMigrationReport) {
    let body = format!(
        "media-taxonomy migration completed\n\
         manual_renamed={}\n\
         synced_copied={}\n\
         skipped_already_new={}\n\
         skipped_lookup_failed={}\n\
         skipped_file_missing={}\n",
        report.manual_renamed, report.synced_copied,
        report.skipped_already_new, report.skipped_lookup_failed,
        report.skipped_file_missing,
    );
    if let Err(e) = std::fs::write(sentinel, body) {
        log::warn!(
            "media-taxonomy migration: failed to write sentinel {}: {e}; \
             migration may re-run on next launch",
            sentinel.display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_appdata_path_ends_with_identifier() {
        // Skip on platforms where the required env var isn't set (CI minimal
        // environments). The point of the test is that when resolution
        // succeeds, the suffix is correct.
        if let Ok(path) = resolve_appdata_manually() {
            assert!(
                path.ends_with(TAURI_IDENTIFIER),
                "manual appdata path should end with {TAURI_IDENTIFIER}, got {}",
                path.display()
            );
        }
    }

    #[test]
    fn portable_dir_returns_none_without_marker() {
        let tmp = tempdir_for("no-marker");
        assert!(portable_dir_for_exe_dir(&tmp).is_none());
        cleanup(&tmp);
    }

    #[test]
    fn portable_dir_returns_settings_subdir_with_marker() {
        let tmp = tempdir_for("with-marker");
        std::fs::write(tmp.join(MARKER_FILENAME), "").unwrap();
        let resolved = portable_dir_for_exe_dir(&tmp);
        assert_eq!(resolved, Some(tmp.join(PORTABLE_SUBDIR)));
        cleanup(&tmp);
    }

    fn tempdir_for(tag: &str) -> PathBuf {
        let tmp = std::env::temp_dir().join(format!("oa-shell-data-dir-{tag}"));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        tmp
    }

    fn cleanup(p: &Path) {
        let _ = std::fs::remove_dir_all(p);
    }

    #[test]
    fn copy_dir_recursive_copies_nested_files() {
        let src = tempdir_for("copy-src");
        let dst = tempdir_for("copy-dst");
        let _ = std::fs::remove_dir_all(&dst); // start with no dst
        std::fs::create_dir_all(src.join("sub/deep")).unwrap();
        std::fs::write(src.join("top.txt"), b"hello").unwrap();
        std::fs::write(src.join("sub/mid.txt"), b"world!!").unwrap();
        std::fs::write(src.join("sub/deep/leaf.bin"), b"\x00\x01\x02").unwrap();

        let (files, bytes) = copy_dir_recursive(&src, &dst).unwrap();
        assert_eq!(files, 3);
        assert_eq!(bytes, 5 + 7 + 3);
        assert_eq!(std::fs::read(dst.join("top.txt")).unwrap(), b"hello");
        assert_eq!(std::fs::read(dst.join("sub/mid.txt")).unwrap(), b"world!!");
        assert_eq!(std::fs::read(dst.join("sub/deep/leaf.bin")).unwrap(), b"\x00\x01\x02");

        cleanup(&src);
        cleanup(&dst);
    }

    #[test]
    fn portable_has_user_content_ignores_sentinel() {
        let dir = tempdir_for("has-content-sentinel-only");
        std::fs::write(dir.join(MIGRATION_SENTINEL), "").unwrap();
        assert!(!portable_has_user_content(&dir));
        std::fs::write(dir.join("audio.json"), "{}").unwrap();
        assert!(portable_has_user_content(&dir));
        cleanup(&dir);
    }

    #[test]
    fn portable_has_user_content_false_on_empty() {
        let dir = tempdir_for("has-content-empty");
        assert!(!portable_has_user_content(&dir));
        cleanup(&dir);
    }

    // ---- Phase 5 media-taxonomy migration tests ----

    /// Make a real 1×1 PNG via the image crate. Phase 2's tests use
    /// the same pattern — hand-rolled PNG bytes don't survive CRC
    /// validation by the image decoder used inside ingest paths.
    fn one_pixel_png() -> Vec<u8> {
        let img: image::RgbaImage = image::ImageBuffer::from_pixel(1, 1, image::Rgba([0u8, 0, 0, 255]));
        let mut buf: Vec<u8> = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .expect("encode 1px png");
        buf
    }

    fn fresh_library_for_data_dir(data_dir: &Path) -> crate::library_db::LibraryDb {
        crate::library_db::LibraryDb::open(data_dir).expect("open library db")
    }

    /// Seed `library` with a single ROM whose file_path basename
    /// matches `stem`. Returns the rom_id used.
    fn seed_one_rom(
        library: &crate::library_db::LibraryDb,
        rom_id: &str,
        title: &str,
        stem: &str,
    ) -> String {
        let row = crate::library_db::GameRow {
            id: rom_id.to_string(),
            title: title.to_string(),
            system_id: "genesis".to_string(),
            file_path: format!("/roms/genesis/{stem}.md"),
            added_at: 0,
            cover_path: None,
            core_override: None,
            seed: false,
            archive_inner_path: None,
            sha1: None,
            serial: None,
            disc_id: None,
            favorite: false,
            completed: false,
            last_played_at: None,
            play_time_secs: 0,
            players: None,
            rating: None,
            year: None,
            genre: None,
            region: None,
            developer: None,
        };
        library.add_games(&[row]).expect("seed row");
        rom_id.to_string()
    }

    fn write_pre_phase5_manual_file(data_dir: &Path, system_id: &str, rom_id: &str) -> PathBuf {
        let rel = format!("media/covers/{system_id}/{rom_id}.png");
        let abs = data_dir.join(&rel);
        std::fs::create_dir_all(abs.parent().unwrap()).expect("mkdir manual dir");
        std::fs::write(&abs, one_pixel_png()).expect("seed manual file");
        abs
    }

    fn write_pre_phase5_synced_file(
        data_dir: &Path,
        system_id: &str,
        subdir: &str,
        filename: &str,
    ) -> PathBuf {
        let rel = format!("media/cache/libretro-thumbnails/{system_id}/{subdir}/{filename}");
        let abs = data_dir.join(&rel);
        std::fs::create_dir_all(abs.parent().unwrap()).expect("mkdir cache dir");
        std::fs::write(&abs, one_pixel_png()).expect("seed cache file");
        abs
    }

    fn empty_db_dir(label: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "oa-mt5-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("mkdir tmp");
        p
    }

    #[test]
    fn migration_empty_db_writes_sentinel_and_noops() {
        let dir = empty_db_dir("empty-db");
        let library = fresh_library_for_data_dir(&dir);
        let mut db: crate::media::MediaDb = Default::default();
        let r = migrate_media_naming(&dir, &library, &mut db);
        assert_eq!(r.manual_renamed, 0);
        assert_eq!(r.synced_copied, 0);
        assert!(!r.already_migrated);
        assert!(dir.join(MEDIA_TAXONOMY_SENTINEL).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn migration_skips_when_sentinel_exists() {
        let dir = empty_db_dir("sentinel-present");
        std::fs::write(dir.join(MEDIA_TAXONOMY_SENTINEL), "from a prior run").expect("seed sentinel");
        let library = fresh_library_for_data_dir(&dir);
        // Even with an old-layout entry present, sentinel guards.
        let mut db: crate::media::MediaDb = Default::default();
        let mut gm = crate::media::GameMedia::default();
        gm.box_front.push(crate::media::MediaVariant {
            source: crate::media::MediaSource::Manual,
            region: None,
            path: "media/covers/genesis/rom-x.png".into(),
            thumb_path: None,
            width: None, height: None, sha1: None, bytes: None,
        });
        db.insert("rom-x".into(), gm);
        let r = migrate_media_naming(&dir, &library, &mut db);
        assert!(r.already_migrated);
        assert_eq!(r.manual_renamed, 0);
        // The variant.path was NOT touched (sentinel said skip).
        assert_eq!(
            db.get("rom-x").unwrap().box_front[0].path,
            "media/covers/genesis/rom-x.png"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn migration_renames_manual_file_and_updates_path() {
        let dir = empty_db_dir("manual-rename");
        let library = fresh_library_for_data_dir(&dir);
        // Seed library: rom_id "rom-sonic" maps to file Sonic the Hedgehog (USA).md.
        seed_one_rom(&library, "rom-sonic", "Sonic the Hedgehog (USA)", "Sonic the Hedgehog (USA)");
        // Seed the old-layout manual file on disk.
        let old_abs = write_pre_phase5_manual_file(&dir, "genesis", "rom-sonic");
        // Seed MediaDb with a Manual variant pointing at the old path.
        let mut db: crate::media::MediaDb = Default::default();
        let mut gm = crate::media::GameMedia::default();
        gm.box_front.push(crate::media::MediaVariant {
            source: crate::media::MediaSource::Manual,
            region: None,
            path: "media/covers/genesis/rom-sonic.png".into(),
            thumb_path: None,
            width: None, height: None,
            sha1: Some("test-sha".into()), bytes: None,
        });
        db.insert("rom-sonic".into(), gm);

        let r = migrate_media_naming(&dir, &library, &mut db);

        assert_eq!(r.manual_renamed, 1);
        assert_eq!(r.synced_copied, 0);
        assert!(!r.already_migrated);
        // Old file is gone (rename = move).
        assert!(!old_abs.exists());
        // New file is at canonical path.
        let new_abs = dir.join("media/genesis/box-front/Sonic the Hedgehog (USA).png");
        assert!(new_abs.is_file(), "new file should exist at {}", new_abs.display());
        // variant.path was rewritten.
        assert_eq!(
            db.get("rom-sonic").unwrap().box_front[0].path,
            "media/genesis/box-front/Sonic the Hedgehog (USA).png"
        );
        // Sentinel is present.
        assert!(dir.join(MEDIA_TAXONOMY_SENTINEL).exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn migration_copies_synced_file_and_keeps_cache() {
        let dir = empty_db_dir("synced-copy");
        let library = fresh_library_for_data_dir(&dir);
        seed_one_rom(&library, "rom-sonic", "Sonic", "Sonic");
        // Seed the old cache file.
        let cache_abs = write_pre_phase5_synced_file(
            &dir, "genesis", "Named_Boxarts", "Sonic.png",
        );
        // Variant whose path is the old cache path.
        let mut db: crate::media::MediaDb = Default::default();
        let mut gm = crate::media::GameMedia::default();
        gm.box_front.push(crate::media::MediaVariant {
            source: crate::media::MediaSource::LibretroThumbnails,
            region: None,
            path: "media/cache/libretro-thumbnails/genesis/Named_Boxarts/Sonic.png".into(),
            thumb_path: None,
            width: None, height: None,
            sha1: Some("test-sha".into()), bytes: None,
        });
        db.insert("rom-sonic".into(), gm);

        let r = migrate_media_naming(&dir, &library, &mut db);

        assert_eq!(r.manual_renamed, 0);
        assert_eq!(r.synced_copied, 1);
        // Cache file is STILL there (copy, not move) — keeps the
        // fast-path cache check warm for future re-syncs.
        assert!(cache_abs.exists(), "cache file should be kept");
        // Canonical kind dir file exists.
        let new_abs = dir.join("media/genesis/box-front/Sonic.png");
        assert!(new_abs.is_file());
        // variant.path was rewritten to canonical.
        assert_eq!(
            db.get("rom-sonic").unwrap().box_front[0].path,
            "media/genesis/box-front/Sonic.png"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn migration_mixed_old_and_new_only_touches_old() {
        let dir = empty_db_dir("mixed");
        let library = fresh_library_for_data_dir(&dir);
        seed_one_rom(&library, "rom-sonic", "Sonic", "Sonic");
        // Old manual at old path.
        write_pre_phase5_manual_file(&dir, "genesis", "rom-sonic");
        let mut db: crate::media::MediaDb = Default::default();
        let mut gm = crate::media::GameMedia::default();
        gm.box_front.push(crate::media::MediaVariant {
            source: crate::media::MediaSource::Manual,
            region: None,
            path: "media/covers/genesis/rom-sonic.png".into(),
            thumb_path: None,
            width: None, height: None, sha1: None, bytes: None,
        });
        // A NEW-format variant on a different slot — must be left alone.
        gm.screenshot_gameplay.push(crate::media::MediaVariant {
            source: crate::media::MediaSource::LibretroThumbnails,
            region: None,
            path: "media/genesis/screenshot-gameplay/Sonic.png".into(),
            thumb_path: None,
            width: None, height: None, sha1: None, bytes: None,
        });
        db.insert("rom-sonic".into(), gm);

        let r = migrate_media_naming(&dir, &library, &mut db);

        assert_eq!(r.manual_renamed, 1);
        assert_eq!(r.skipped_already_new, 1);
        // The new-format variant's path is untouched.
        assert_eq!(
            db.get("rom-sonic").unwrap().screenshot_gameplay[0].path,
            "media/genesis/screenshot-gameplay/Sonic.png"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn migration_orphan_media_entry_counted_as_lookup_failed() {
        let dir = empty_db_dir("orphan");
        let library = fresh_library_for_data_dir(&dir);
        // NO library entry for rom-ghost — orphan.
        let mut db: crate::media::MediaDb = Default::default();
        let mut gm = crate::media::GameMedia::default();
        gm.box_front.push(crate::media::MediaVariant {
            source: crate::media::MediaSource::Manual,
            region: None,
            path: "media/covers/genesis/rom-ghost.png".into(),
            thumb_path: None,
            width: None, height: None, sha1: None, bytes: None,
        });
        db.insert("rom-ghost".into(), gm);

        let r = migrate_media_naming(&dir, &library, &mut db);

        assert_eq!(r.manual_renamed, 0);
        assert_eq!(r.skipped_lookup_failed, 1);
        // variant.path NOT changed (we couldn't compute the new path).
        assert_eq!(
            db.get("rom-ghost").unwrap().box_front[0].path,
            "media/covers/genesis/rom-ghost.png"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn migration_missing_file_skipped_but_path_updated() {
        let dir = empty_db_dir("missing-file");
        let library = fresh_library_for_data_dir(&dir);
        seed_one_rom(&library, "rom-sonic", "Sonic", "Sonic");
        // Variant points at a path that doesn't exist on disk
        // (operator manually deleted the covers folder).
        let mut db: crate::media::MediaDb = Default::default();
        let mut gm = crate::media::GameMedia::default();
        gm.box_front.push(crate::media::MediaVariant {
            source: crate::media::MediaSource::Manual,
            region: None,
            path: "media/covers/genesis/rom-sonic.png".into(),
            thumb_path: None,
            width: None, height: None, sha1: None, bytes: None,
        });
        db.insert("rom-sonic".into(), gm);

        let r = migrate_media_naming(&dir, &library, &mut db);

        assert_eq!(r.skipped_file_missing, 1);
        // variant.path still updated to point at the new canonical
        // location — a future operator drop will land cleanly.
        assert_eq!(
            db.get("rom-sonic").unwrap().box_front[0].path,
            "media/genesis/box-front/Sonic.png"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn migration_re_run_after_success_noops_via_sentinel() {
        let dir = empty_db_dir("re-run");
        let library = fresh_library_for_data_dir(&dir);
        seed_one_rom(&library, "rom-sonic", "Sonic", "Sonic");
        write_pre_phase5_manual_file(&dir, "genesis", "rom-sonic");
        let mut db: crate::media::MediaDb = Default::default();
        let mut gm = crate::media::GameMedia::default();
        gm.box_front.push(crate::media::MediaVariant {
            source: crate::media::MediaSource::Manual,
            region: None,
            path: "media/covers/genesis/rom-sonic.png".into(),
            thumb_path: None,
            width: None, height: None, sha1: None, bytes: None,
        });
        db.insert("rom-sonic".into(), gm);

        let r1 = migrate_media_naming(&dir, &library, &mut db);
        assert_eq!(r1.manual_renamed, 1);
        // Second run — sentinel present.
        let r2 = migrate_media_naming(&dir, &library, &mut db);
        assert!(r2.already_migrated);
        assert_eq!(r2.manual_renamed, 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn classify_old_path_recognizes_both_legacy_shapes() {
        assert_eq!(
            classify_old_path("media/covers/genesis/rom-abc.png"),
            Some("manual-v1"),
        );
        assert_eq!(
            classify_old_path("media/cache/libretro-thumbnails/genesis/Named_Boxarts/Sonic.png"),
            Some("synced-v1"),
        );
        // New layout returns None.
        assert_eq!(
            classify_old_path("media/genesis/box-front/Sonic.png"),
            None,
        );
        assert_eq!(
            classify_old_path("media/snes/clear-logo/Super Metroid.png"),
            None,
        );
    }
}
