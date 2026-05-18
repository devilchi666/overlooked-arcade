// Archive support — zip / 7z (rar deliberately deferred). Two flavors of
// launch:
//
//   * Cart-format inner (.pce / .sgx / .nes / .sms / .gg / .ws / .lnx / ...):
//     extract bytes in memory and feed `RomSource::Bytes`. No filesystem
//     pollution; no cleanup needed; ~5-20ms cost on typical 1-8MB carts.
//
//   * CD-set inner (.cue / .m3u / .toc): cues reference relative .bin/.wav
//     siblings, so the core needs a real path. Extract all archive members
//     into `appData/temp/<rom_id>/`, point `RomSource::Path` at the entry.
//     Cleanup tied to unload_rom + a startup sweep for crash recovery.
//
// The scanner peeks inside archives at import time and emits one library
// entry per ROM-like inner file. Each entry's file_path is encoded as
// `<archive>#<inner>` so the UNIQUE constraint on games.file_path lets
// multiple inner ROMs share a single archive on disk. archive_inner_path
// holds the raw inner path (no `#` prefix) for the launch flow.

use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveKind {
    Zip,
    SevenZ,
}

impl ArchiveKind {
    /// Returns `Some(kind)` if the file extension matches a supported archive
    /// format. None means "treat as raw ROM."
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            "zip" => Some(Self::Zip),
            "7z" => Some(Self::SevenZ),
            _ => None,
        }
    }

    /// True for extensions we recognize as archives but don't handle yet.
    /// Used by the scanner to surface a "convert to zip" hint instead of
    /// silently skipping.
    pub fn is_unsupported_archive(ext: &str) -> bool {
        matches!(ext.to_ascii_lowercase().as_str(), "rar" | "tar" | "gz" | "bz2")
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveEntry {
    /// Posix-style relative path inside the archive.
    pub inner_path: String,
    pub size: u64,
    /// Lowercase extension without leading dot. Empty for directories
    /// (filtered out before this struct gets returned).
    pub extension: String,
}

/// CD-set entry-point extensions — these reference siblings, so the launch
/// path needs the whole archive extracted to a temp dir.
pub fn is_cd_entry_extension(ext: &str) -> bool {
    matches!(ext.to_ascii_lowercase().as_str(), "cue" | "m3u" | "toc" | "ccd")
}

/// Strip the path components down to the file's own extension (lowercased,
/// no leading dot).
fn extension_of(name: &str) -> String {
    Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default()
}

/// List ROM-like entries inside an archive. Filtered down to extensions the
/// caller cares about (the same set the folder scanner uses for raw files —
/// systems registry ∪ libretro cores' valid_extensions).
pub fn list_rom_contents(
    archive: &Path,
    accepted_exts: &HashSet<String>,
) -> Result<Vec<ArchiveEntry>, String> {
    let ext = extension_of(&archive.to_string_lossy());
    let Some(kind) = ArchiveKind::from_extension(&ext) else {
        return Err(format!("not an archive extension: {ext}"));
    };
    match kind {
        ArchiveKind::Zip => list_zip(archive, accepted_exts),
        ArchiveKind::SevenZ => list_sevenz(archive, accepted_exts),
    }
}

fn list_zip(archive: &Path, accepted: &HashSet<String>) -> Result<Vec<ArchiveEntry>, String> {
    let file = File::open(archive).map_err(|e| format!("open {}: {e}", archive.display()))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("read zip: {e}"))?;
    let mut out = Vec::new();
    for i in 0..zip.len() {
        let entry = zip.by_index(i).map_err(|e| format!("zip entry {i}: {e}"))?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        let inner_ext = extension_of(&name);
        if inner_ext.is_empty() || !accepted.contains(&inner_ext) {
            continue;
        }
        out.push(ArchiveEntry {
            inner_path: name,
            size: entry.size(),
            extension: inner_ext,
        });
    }
    Ok(out)
}

fn list_sevenz(archive: &Path, accepted: &HashSet<String>) -> Result<Vec<ArchiveEntry>, String> {
    let mut out = Vec::new();
    sevenz_rust::decompress_file_with_extract_fn(archive, std::env::temp_dir(), |entry, _reader, _dest| {
        // We're using decompress just to walk metadata — but sevenz-rust's
        // listing API requires the extract closure. Skip actual extraction
        // by returning Ok(false) which tells it not to write.
        if entry.is_directory() {
            return Ok(true); // continue
        }
        let inner_ext = extension_of(entry.name());
        if !inner_ext.is_empty() && accepted.contains(&inner_ext) {
            out.push(ArchiveEntry {
                inner_path: entry.name().to_string(),
                size: entry.size,
                extension: inner_ext,
            });
        }
        Ok(true) // continue without extracting
    })
    .map_err(|e| format!("read 7z: {e}"))?;
    Ok(out)
}

/// Read a single inner file as bytes. Used by the cart-format launch path.
pub fn read_inner_to_bytes(archive: &Path, inner: &str) -> Result<Vec<u8>, String> {
    let ext = extension_of(&archive.to_string_lossy());
    let Some(kind) = ArchiveKind::from_extension(&ext) else {
        return Err(format!("not an archive: {ext}"));
    };
    match kind {
        ArchiveKind::Zip => {
            let file = File::open(archive).map_err(|e| format!("open: {e}"))?;
            let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("read zip: {e}"))?;
            let mut entry = zip
                .by_name(inner)
                .map_err(|e| format!("zip inner {inner}: {e}"))?;
            let mut buf = Vec::with_capacity(entry.size() as usize);
            entry.read_to_end(&mut buf).map_err(|e| format!("read inner: {e}"))?;
            Ok(buf)
        }
        ArchiveKind::SevenZ => {
            let mut bytes: Option<Vec<u8>> = None;
            sevenz_rust::decompress_file_with_extract_fn(archive, std::env::temp_dir(), |entry, reader, _dest| {
                if entry.is_directory() || entry.name() != inner {
                    return Ok(true);
                }
                let mut buf = Vec::with_capacity(entry.size as usize);
                std::io::copy(reader, &mut buf).map_err(|e| std::io::Error::other(format!("read 7z inner: {e}")))?;
                bytes = Some(buf);
                Ok(false) // stop iterating
            })
            .map_err(|e| format!("walk 7z: {e}"))?;
            bytes.ok_or_else(|| format!("inner not found in 7z: {inner}"))
        }
    }
}

/// Extract the whole archive into `temp_root` (typically
/// `appData/temp/<rom_id>/`). Returns the absolute path of the inner entry
/// the core should be pointed at. Caller is responsible for calling
/// `cleanup_temp` after unload.
pub fn extract_to_temp(
    archive: &Path,
    inner_entry: &str,
    temp_root: &Path,
) -> Result<PathBuf, String> {
    std::fs::create_dir_all(temp_root).map_err(|e| format!("mkdir temp_root: {e}"))?;
    let ext = extension_of(&archive.to_string_lossy());
    let Some(kind) = ArchiveKind::from_extension(&ext) else {
        return Err(format!("not an archive: {ext}"));
    };
    match kind {
        ArchiveKind::Zip => extract_zip(archive, temp_root)?,
        ArchiveKind::SevenZ => extract_sevenz(archive, temp_root)?,
    }
    let entry_path = temp_root.join(inner_entry);
    if !entry_path.exists() {
        return Err(format!("entry path missing after extract: {}", entry_path.display()));
    }
    Ok(entry_path)
}

fn extract_zip(archive: &Path, dest: &Path) -> Result<(), String> {
    let file = File::open(archive).map_err(|e| format!("open: {e}"))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("read zip: {e}"))?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| format!("entry {i}: {e}"))?;
        // Refuse path traversal — entry names can contain `..` in malicious
        // archives. `enclosed_name()` returns None for any such case.
        let Some(rel) = entry.enclosed_name() else {
            log::warn!("archive: skipping suspicious zip entry: {}", entry.name());
            continue;
        };
        let dest_path = dest.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&dest_path)
                .map_err(|e| format!("mkdir {}: {e}", dest_path.display()))?;
        } else {
            if let Some(parent) = dest_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
            }
            let mut out = File::create(&dest_path)
                .map_err(|e| format!("create {}: {e}", dest_path.display()))?;
            std::io::copy(&mut entry, &mut out)
                .map_err(|e| format!("write {}: {e}", dest_path.display()))?;
        }
    }
    Ok(())
}

fn extract_sevenz(archive: &Path, dest: &Path) -> Result<(), String> {
    let dest_owned = dest.to_path_buf();
    sevenz_rust::decompress_file_with_extract_fn(archive, &dest_owned, |entry, reader, _hint| {
        if entry.is_directory() {
            let p = dest_owned.join(entry.name());
            std::fs::create_dir_all(&p)?;
            return Ok(true);
        }
        let rel = Path::new(entry.name());
        // Guard against path traversal — sevenz-rust doesn't enforce this.
        if rel.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
            log::warn!("archive: skipping suspicious 7z entry: {}", entry.name());
            return Ok(true);
        }
        let out_path = dest_owned.join(rel);
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = File::create(&out_path)?;
        std::io::copy(reader, &mut out)?;
        Ok(true)
    })
    .map_err(|e| format!("extract 7z: {e}"))
}

/// Delete a per-rom temp dir. Called after unload_rom. Non-fatal: a missing
/// dir is fine (cart-format games never created one).
pub fn cleanup_temp(temp_root: &Path, rom_id: &str) {
    let dir = temp_root.join(rom_id);
    if !dir.exists() {
        return;
    }
    if let Err(e) = std::fs::remove_dir_all(&dir) {
        log::warn!("archive: cleanup_temp({}) failed: {e}", dir.display());
    } else {
        log::debug!("archive: cleaned temp dir {}", dir.display());
    }
}

/// Sweep all leftover temp dirs. Called on app startup (recovers from
/// crashed sessions) and on graceful_exit (belt-and-braces). Returns the
/// count of dirs removed.
pub fn sweep_temp(temp_root: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(temp_root) else {
        return 0;
    };
    let mut removed = 0usize;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Err(e) = std::fs::remove_dir_all(&path) {
                log::warn!("archive: sweep_temp failed to remove {}: {e}", path.display());
            } else {
                removed += 1;
            }
        }
    }
    if removed > 0 {
        log::info!("archive: swept {removed} leftover temp dir(s) from {}", temp_root.display());
    }
    removed
}

/// Build the encoded library file_path from an archive path + inner path.
/// Format: `<archive>#<inner>` (posix-style inner). Used by both scanner
/// (when emitting RomEntries) and the launch path (when decoding).
pub fn encode_file_path(archive: &Path, inner: &str) -> String {
    format!("{}#{}", archive.display(), inner)
}

/// Reverse of `encode_file_path`. Returns `(archive_path, inner_path)`. If
/// the input has no `#`, returns `(input, "")` — caller treats empty inner
/// as "raw file, not archived."
pub fn decode_file_path(file_path: &str) -> (PathBuf, String) {
    if let Some((archive, inner)) = file_path.rsplit_once('#') {
        (PathBuf::from(archive), inner.to_string())
    } else {
        (PathBuf::from(file_path), String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_zip(dest: &Path, entries: &[(&str, &[u8])]) {
        let file = File::create(dest).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for (name, data) in entries {
            zip.start_file::<_, ()>(*name, opts).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    }

    fn tmp() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "oa-archive-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn detect_from_extension() {
        assert_eq!(ArchiveKind::from_extension("zip"), Some(ArchiveKind::Zip));
        assert_eq!(ArchiveKind::from_extension("ZIP"), Some(ArchiveKind::Zip));
        assert_eq!(ArchiveKind::from_extension("7z"), Some(ArchiveKind::SevenZ));
        assert_eq!(ArchiveKind::from_extension("pce"), None);
        assert!(ArchiveKind::is_unsupported_archive("rar"));
        assert!(!ArchiveKind::is_unsupported_archive("zip"));
    }

    #[test]
    fn list_zip_filters_to_accepted_extensions() {
        let dir = tmp();
        let z = dir.join("games.zip");
        make_zip(
            &z,
            &[
                ("bonk.pce", b"BONK"),
                ("manual.txt", b"manual text"),
                ("subdir/sub_game.sgx", b"SGX!"),
                ("noext", b"x"),
            ],
        );
        let mut accepted = HashSet::new();
        accepted.insert("pce".to_string());
        accepted.insert("sgx".to_string());
        let entries = list_rom_contents(&z, &accepted).expect("list");
        assert_eq!(entries.len(), 2);
        let names: Vec<_> = entries.iter().map(|e| e.inner_path.as_str()).collect();
        assert!(names.contains(&"bonk.pce"));
        assert!(names.contains(&"subdir/sub_game.sgx"));
    }

    #[test]
    fn read_inner_to_bytes_zip() {
        let dir = tmp();
        let z = dir.join("games.zip");
        make_zip(&z, &[("bonk.pce", b"BONK_BYTES")]);
        let bytes = read_inner_to_bytes(&z, "bonk.pce").expect("read");
        assert_eq!(bytes, b"BONK_BYTES");
    }

    #[test]
    fn extract_to_temp_writes_files() {
        let dir = tmp();
        let z = dir.join("cd.zip");
        make_zip(
            &z,
            &[
                ("Game.cue", b"FILE \"track01.bin\" BINARY"),
                ("track01.bin", b"binary track bytes"),
            ],
        );
        let temp_root = dir.join("rom-123");
        let entry = extract_to_temp(&z, "Game.cue", &temp_root).expect("extract");
        assert!(entry.ends_with("Game.cue"));
        assert!(temp_root.join("track01.bin").exists());
        // Cleanup test
        cleanup_temp(dir.as_path(), "rom-123");
        assert!(!temp_root.exists());
    }

    #[test]
    fn sweep_temp_removes_everything() {
        let dir = tmp();
        let temp_root = dir.join("temp");
        std::fs::create_dir_all(temp_root.join("game-a")).unwrap();
        std::fs::create_dir_all(temp_root.join("game-b")).unwrap();
        let removed = sweep_temp(&temp_root);
        assert_eq!(removed, 2);
        // Re-sweep is no-op.
        assert_eq!(sweep_temp(&temp_root), 0);
    }

    #[test]
    fn encode_decode_file_path_roundtrip() {
        let archive = PathBuf::from("G:\\ROMs\\games.zip");
        let inner = "Bonk (USA).pce";
        let encoded = encode_file_path(&archive, inner);
        let (a, i) = decode_file_path(&encoded);
        assert_eq!(a, archive);
        assert_eq!(i, inner);
    }

    #[test]
    fn decode_file_path_raw_returns_empty_inner() {
        let (a, i) = decode_file_path("G:\\ROMs\\bonk.pce");
        assert_eq!(a, PathBuf::from("G:\\ROMs\\bonk.pce"));
        assert!(i.is_empty());
    }

    #[test]
    fn cd_entry_detection() {
        assert!(is_cd_entry_extension("cue"));
        assert!(is_cd_entry_extension("CUE"));
        assert!(is_cd_entry_extension("m3u"));
        assert!(!is_cd_entry_extension("pce"));
        assert!(!is_cd_entry_extension("bin"));
    }

    #[test]
    fn extract_refuses_path_traversal() {
        let dir = tmp();
        let z = dir.join("evil.zip");
        // Manually craft a zip with `..` traversal — `enclosed_name()` should
        // detect and skip it. Using the regular ZipWriter would normalize.
        let file = File::create(&z).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zip.start_file::<_, ()>("../escaped.txt", opts).unwrap();
        zip.write_all(b"escape").unwrap();
        zip.start_file::<_, ()>("safe.pce", opts).unwrap();
        zip.write_all(b"safe").unwrap();
        zip.finish().unwrap();

        let temp_root = dir.join("extract_target");
        let res = extract_to_temp(&z, "safe.pce", &temp_root);
        // Extraction completes; the escaped.txt entry got skipped, safe.pce
        // landed in the temp_root. We don't expect ../escaped.txt anywhere
        // ABOVE temp_root.
        assert!(res.is_ok(), "extraction should succeed: {res:?}");
        assert!(temp_root.join("safe.pce").exists());
        let escaped = dir.join("escaped.txt");
        assert!(!escaped.exists(), "path-traversal entry must not escape temp_root");
    }
}
