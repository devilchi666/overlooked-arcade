//! Hash-based ROM identification — bind ROMs in the library to their
//! canonical entries in libretro-database via SHA-1.
//!
//! Three commands, three stages:
//!
//! 1. **`sync_rom_hashes_for_system`** — fetch `dat/<system>.dat` from
//!    libretro-database, parse the clrmamepro text format, populate the
//!    `rom_hashes` table. Per-system 24h cache (same TTL as the existing
//!    metadat sync).
//!
//! 2. **`resolve_rom_hashes_for_system`** — for every game of the
//!    system that doesn't have a sha1 stamped on it, hash the ROM
//!    (skipping CD images — the `.cue` / `.chd` content hash isn't in
//!    libretro-database), look up the sha1 in `rom_hashes`, on hit
//!    overwrite `games.title` with the canonical name and stamp the
//!    serial.
//!
//! 3. **`lookup_rom_hash`** — single-shot diagnostic that takes a sha1
//!    and returns the matching `RomHashRow`. Used by per-game UI
//!    surfaces that want to explain "why is this game named X?".
//!
//! ## Why SHA-1 only
//!
//! libretro-database keys each ROM entry on SHA-1, CRC32, and MD5.
//! SHA-1 is the canonical lookup (collision-resistant + the most
//! widely-attested upstream). CRC32 is useful when the user only has a
//! cataloged-but-not-hashed entry (rare); MD5 adds nothing SHA-1 doesn't.
//! Slice ships SHA-1 only — adding CRC32/MD5 fallbacks is a small follow-up.
//!
//! ## Why we skip CD images
//!
//! The .cue / .chd / .toc / .m3u file on disk doesn't have a meaningful
//! hash for the user's game — that depends on track contents, not the
//! container. CD identification is a separate game-ID database problem
//! (PSX serial via "SLUS_xxx.xx" matching, redump-style); out of scope
//! for this slice.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::Serialize;
use sha1::{Digest, Sha1};
use tauri::Emitter;

use crate::archive;
use crate::library_db::{LibraryDb, RomHashRow};

const HASH_CACHE_TTL_SECS: u64 = 86_400; // 24h, matches metadat cache
const HASH_DB_CACHE_DIR: &str = "library-db/hashes";

/// Map an OA SystemId to the libretro-database system name (the basename
/// of the .dat file under `dat/`). `None` means we have no upstream
/// .dat for the system — sync is a no-op rather than an error.
///
/// Names match libretro-database's `dat/` directory listings; revisit
/// when adding new systems.
fn libretro_db_name_for_system(system_id: &str) -> Option<&'static str> {
    Some(match system_id {
        "tg16"     => "NEC - PC Engine - TurboGrafx 16",
        "pce-cd"   => return None, // CD images aren't hash-matched (see module docs)
        "lynx"     => "Atari - Lynx",
        "nes"      => "Nintendo - Nintendo Entertainment System",
        "snes"     => "Nintendo - Super Nintendo Entertainment System",
        _ => return None,
    })
}

/// Extensions we deliberately skip when computing hashes — CD-container
/// formats whose content hash means something different from "the
/// canonical ROM bytes." See module docs.
fn is_cd_container_ext(ext: &str) -> bool {
    matches!(ext, "cue" | "chd" | "ccd" | "toc" | "m3u" | "iso" | "bin")
}

fn hash_cache_path(app_data_dir: &Path, system_id: &str) -> PathBuf {
    app_data_dir
        .join(HASH_DB_CACHE_DIR)
        .join(format!("{}.json", system_id.replace(['/', '\\'], "_")))
}

#[derive(serde::Deserialize, serde::Serialize)]
struct CachedHashDb {
    fetched_at_unix_secs: u64,
    entries: Vec<RomHashRow>,
}

/// Parse the libretro-database clrmamepro-format .dat. Each `game (...)`
/// block carries a `name "..."` (canonical name we adopt as title), an
/// optional `serial "..."`, and one or more `rom ( ... sha1 XXXX ... )`
/// entries. Multi-disc games may have multiple rom blocks per game;
/// each gets its own row in our table because the user's local ROM is
/// hashed individually.
///
/// Robust to upstream comments + indentation variations. Lines we don't
/// recognize inside a game block are ignored — we only consume `name`,
/// `serial`, and the contents of `rom (...)` parens.
pub fn parse_libretro_dat(content: &str, system_id: &str) -> Vec<RomHashRow> {
    /// One pending rom entry while we're inside a `game (...)` block.
    /// game.serial may appear before OR after the `rom ( ... )` line
    /// depending on the upstream dat, so we accumulate and stamp the
    /// final serial onto every rom in the block at the closing `)`.
    struct PendingRom {
        sha1: String,
        crc32: Option<String>,
        size_bytes: Option<i64>,
    }

    let mut out: Vec<RomHashRow> = Vec::new();
    let mut in_game = false;
    let mut current_name: Option<String> = None;
    let mut current_serial: Option<String> = None;
    let mut pending: Vec<PendingRom> = Vec::new();
    for raw in content.lines() {
        let line = raw.trim();
        if line.starts_with("game (") {
            in_game = true;
            current_name = None;
            current_serial = None;
            pending.clear();
            continue;
        }
        if !in_game {
            continue;
        }
        if line == ")" {
            in_game = false;
            if let Some(name) = current_name.take() {
                for r in pending.drain(..) {
                    out.push(RomHashRow {
                        sha1: r.sha1,
                        system_id: system_id.to_string(),
                        game_name: name.clone(),
                        serial: current_serial.clone(),
                        crc32: r.crc32,
                        size_bytes: r.size_bytes,
                    });
                }
            }
            current_serial = None;
            pending.clear();
            continue;
        }

        if let Some(rest) = line.strip_prefix("name ") {
            current_name = Some(unquote(rest));
            continue;
        }
        if let Some(rest) = line.strip_prefix("serial ") {
            current_serial = Some(unquote(rest));
            continue;
        }
        if let Some(rest) = line.strip_prefix("rom (") {
            // Format: `rom ( name "X" size N crc YYYY md5 ZZZZ sha1 WWWW )`
            // Keys/values can come in any order; we want sha1 + crc + size.
            // Trailing `)` is on the same line for libretro-database. Strip
            // both ends defensively.
            let body = rest.trim_end_matches(')').trim();
            let mut sha1: Option<String> = None;
            let mut crc32: Option<String> = None;
            let mut size: Option<i64> = None;
            let mut tokens = TokenIter::new(body);
            while let Some((key, value)) = tokens.next_pair() {
                match key.as_str() {
                    "sha1"   => sha1 = Some(value.to_ascii_lowercase()),
                    "crc"    => crc32 = Some(value.to_ascii_lowercase()),
                    "size"   => size = value.parse::<i64>().ok(),
                    _ => {}
                }
            }
            if let Some(sha1) = sha1 {
                pending.push(PendingRom { sha1, crc32, size_bytes: size });
            }
        }
    }
    out
}

fn unquote(s: &str) -> String {
    s.trim().trim_matches('"').to_string()
}

/// Tiny `key value [key value]…` tokenizer with quote-aware values. The
/// `rom (...)` body never nests parens, and value lengths are bounded,
/// so a hand-rolled iterator outperforms regex / nom for this loop.
struct TokenIter<'a> {
    chars: std::str::Chars<'a>,
    peek: Option<char>,
}

impl<'a> TokenIter<'a> {
    fn new(s: &'a str) -> Self {
        let mut it = s.chars();
        let peek = it.next();
        Self { chars: it, peek }
    }

    fn advance(&mut self) -> Option<char> {
        let cur = self.peek;
        self.peek = self.chars.next();
        cur
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek {
            if c.is_ascii_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_word(&mut self) -> String {
        let mut out = String::new();
        while let Some(c) = self.peek {
            if c.is_ascii_whitespace() {
                break;
            }
            out.push(c);
            self.advance();
        }
        out
    }

    fn read_value(&mut self) -> String {
        self.skip_ws();
        if self.peek == Some('"') {
            self.advance(); // consume opening quote
            let mut out = String::new();
            while let Some(c) = self.peek {
                if c == '"' {
                    self.advance();
                    return out;
                }
                out.push(c);
                self.advance();
            }
            out
        } else {
            self.read_word()
        }
    }

    fn next_pair(&mut self) -> Option<(String, String)> {
        self.skip_ws();
        if self.peek.is_none() { return None; }
        let key = self.read_word();
        if key.is_empty() { return None; }
        let value = self.read_value();
        Some((key, value))
    }
}

/// Fetch `dat/<system>.dat` from libretro-database via the raw GitHub URL.
/// 404 = upstream has no .dat for the system; we return Ok(None) so the
/// caller can clean up without surfacing as an error.
async fn fetch_libretro_dat(
    client: &reqwest::Client,
    libretro_db_name: &str,
) -> Result<Option<String>, String> {
    let url = format!(
        "https://raw.githubusercontent.com/libretro/libretro-database/master/dat/{}.dat",
        urlencoding::encode(libretro_db_name),
    );
    let resp = client
        .get(&url)
        .header("User-Agent", "OverlookedArcade")
        .send()
        .await
        .map_err(|e| format!("fetch {url}: {e}"))?;
    let status = resp.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        log::debug!("rom_hashes: {url} 404 (no dat for this system)");
        return Ok(None);
    }
    if !status.is_success() {
        return Err(format!("rom_hashes dat fetch {url} status {status}"));
    }
    let body = resp.text().await.map_err(|e| format!("dat body: {e}"))?;
    Ok(Some(body))
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RomHashSyncSummary {
    pub system_id: String,
    pub upstream_entries: usize,
    pub written: usize,
    pub from_cache: bool,
}

/// Fetch + parse + bulk-upsert the libretro-database `dat/<system>.dat`
/// into our local `rom_hashes` table. 24h cache to avoid re-pulling
/// the same file every session.
#[tauri::command]
#[allow(non_snake_case)]
pub async fn sync_rom_hashes_for_system(
    systemId: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::media::MediaState>,
    db: tauri::State<'_, LibraryDb>,
) -> Result<RomHashSyncSummary, String> {
    let app_data_dir = state.app_data_dir.clone();
    let Some(name) = libretro_db_name_for_system(&systemId) else {
        log::info!("rom_hashes: no libretro-database mapping for {systemId}; skipping sync");
        return Ok(RomHashSyncSummary {
            system_id: systemId,
            upstream_entries: 0,
            written: 0,
            from_cache: false,
        });
    };

    // Cache lookup — same TTL as the metadat cache.
    let cache_path = hash_cache_path(&app_data_dir, &systemId);
    if let Ok(bytes) = std::fs::read(&cache_path) {
        if let Ok(cached) = serde_json::from_slice::<CachedHashDb>(&bytes) {
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            if !cached.entries.is_empty()
                && now.saturating_sub(cached.fetched_at_unix_secs) < HASH_CACHE_TTL_SECS
            {
                log::info!(
                    "rom_hashes: {systemId} cache hit ({} entries)",
                    cached.entries.len()
                );
                let upstream_entries = cached.entries.len();
                let written = db.upsert_rom_hashes(&cached.entries)?;
                let _ = app.emit("oa://rom-hashes-synced", &RomHashSyncSummary {
                    system_id: systemId.clone(),
                    upstream_entries,
                    written,
                    from_cache: true,
                });
                return Ok(RomHashSyncSummary {
                    system_id: systemId,
                    upstream_entries,
                    written,
                    from_cache: true,
                });
            }
        }
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| format!("reqwest client: {e}"))?;

    let dat_text = match fetch_libretro_dat(&client, name).await? {
        Some(s) => s,
        None => {
            log::warn!("rom_hashes: upstream has no dat for {systemId} ({name})");
            return Ok(RomHashSyncSummary {
                system_id: systemId,
                upstream_entries: 0,
                written: 0,
                from_cache: false,
            });
        }
    };
    let entries = parse_libretro_dat(&dat_text, &systemId);
    let upstream_entries = entries.len();
    log::info!("rom_hashes: parsed {upstream_entries} entries for {systemId}");
    let written = db.upsert_rom_hashes(&entries)?;

    // Cache write (24h reuse).
    if !entries.is_empty() {
        if let Some(parent) = cache_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let cached = CachedHashDb { fetched_at_unix_secs: now, entries };
        let _ = std::fs::write(&cache_path, serde_json::to_vec(&cached).unwrap_or_default());
    }

    let summary = RomHashSyncSummary {
        system_id: systemId.clone(),
        upstream_entries,
        written,
        from_cache: false,
    };
    let _ = app.emit("oa://rom-hashes-synced", &summary);
    Ok(summary)
}

/// Compute the SHA-1 of a ROM. For archived entries, peek into the
/// archive at the recorded `inner_path` (no extraction-to-disk). For
/// raw byte-source ROMs, hash the file in 64 KiB chunks. CD images are
/// caller-filtered before we get here.
///
/// `file_path` follows the library's encoded shape: archived entries
/// look like `"<archive_path>#<inner>"` (matches `encode_file_path` in
/// archive.rs); raw ROMs are a plain path. We always run the input
/// through `archive::decode_file_path` first so callers don't have to
/// duplicate the split logic.
fn sha1_of_rom(file_path: &str, archive_inner: Option<&str>) -> Result<String, String> {
    let (real_path, decoded_inner) = archive::decode_file_path(file_path);
    let inner = archive_inner.map(|s| s.to_string()).or_else(|| {
        if decoded_inner.is_empty() { None } else { Some(decoded_inner) }
    });
    if let Some(inner) = inner {
        // Archived entry — load the inner bytes via the existing archive
        // module, which already handles every archive kind we support.
        let bytes = archive::read_inner_to_bytes(&real_path, &inner).map_err(|e| {
            format!("archive read {}#{inner}: {e}", real_path.display())
        })?;
        let mut hasher = Sha1::new();
        hasher.update(&bytes);
        return Ok(format!("{:x}", hasher.finalize()));
    }
    use std::io::Read;
    let mut file = std::fs::File::open(&real_path)
        .map_err(|e| format!("open {}: {e}", real_path.display()))?;
    let mut hasher = Sha1::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).map_err(|e| format!("read {}: {e}", real_path.display()))?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RomResolveProgress {
    pub system_id: String,
    pub done: usize,
    pub total: usize,
    pub current_title: String,
    pub last_action: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RomResolveSummary {
    pub system_id: String,
    pub scanned: usize,
    pub matched: usize,
    pub unmatched: usize,
    pub skipped_cd: usize,
    pub errors: usize,
}

/// Hash every ROM in `system_id` that doesn't have a sha1 yet, look it
/// up in `rom_hashes`, and stamp the canonical title + serial on a
/// match. CD images are skipped (see module docs).
#[tauri::command]
#[allow(non_snake_case)]
pub async fn resolve_rom_hashes_for_system(
    systemId: String,
    app: tauri::AppHandle,
    db: tauri::State<'_, LibraryDb>,
) -> Result<RomResolveSummary, String> {
    let games = db.list_games_missing_hash(&systemId)?;
    let total = games.len();
    let mut summary = RomResolveSummary {
        system_id: systemId.clone(),
        scanned: 0,
        matched: 0,
        unmatched: 0,
        skipped_cd: 0,
        errors: 0,
    };
    // Tiny in-process cache so re-hashing identical files in this batch
    // (e.g. user has the same ROM in two folders) doesn't pay for it
    // twice. Keyed on `(file_path, archive_inner)` since that's the
    // unique "thing to hash."
    let mut hash_cache: HashMap<(String, Option<String>), String> = HashMap::new();

    let mut done = 0usize;
    for g in games {
        done += 1;
        let ext = std::path::Path::new(g.archive_inner_path.as_deref().unwrap_or(&g.file_path))
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        if is_cd_container_ext(&ext) {
            summary.skipped_cd += 1;
            let _ = app.emit("oa://rom-hash-resolve-progress", &RomResolveProgress {
                system_id: systemId.clone(),
                done,
                total,
                current_title: g.title.clone(),
                last_action: format!("skipped CD container .{ext}"),
            });
            continue;
        }

        let key = (g.file_path.clone(), g.archive_inner_path.clone());
        let sha1 = match hash_cache.get(&key) {
            Some(h) => h.clone(),
            None => match sha1_of_rom(&g.file_path, g.archive_inner_path.as_deref()) {
                Ok(h) => {
                    hash_cache.insert(key, h.clone());
                    h
                }
                Err(e) => {
                    log::warn!("rom_hashes: hash {} failed: {e}", g.file_path);
                    summary.errors += 1;
                    let _ = app.emit("oa://rom-hash-resolve-progress", &RomResolveProgress {
                        system_id: systemId.clone(),
                        done,
                        total,
                        current_title: g.title.clone(),
                        last_action: format!("hash error: {e}"),
                    });
                    continue;
                }
            },
        };

        summary.scanned += 1;
        match db.lookup_rom_hash(&sha1)? {
            Some(row) => {
                summary.matched += 1;
                if let Err(e) = db.apply_rom_hash(
                    &g.id,
                    &sha1,
                    Some(&row.game_name),
                    row.serial.as_deref(),
                ) {
                    log::warn!("rom_hashes: apply {} failed: {e}", g.id);
                    summary.errors += 1;
                }
                let _ = app.emit("oa://rom-hash-resolve-progress", &RomResolveProgress {
                    system_id: systemId.clone(),
                    done,
                    total,
                    current_title: g.title.clone(),
                    last_action: format!("matched → {}", row.game_name),
                });
            }
            None => {
                // Stamp the sha1 anyway so re-runs don't re-hash this file.
                // Title stays as-is.
                if let Err(e) = db.apply_rom_hash(&g.id, &sha1, None, None) {
                    log::warn!("rom_hashes: apply-no-match {} failed: {e}", g.id);
                    summary.errors += 1;
                }
                summary.unmatched += 1;
                let _ = app.emit("oa://rom-hash-resolve-progress", &RomResolveProgress {
                    system_id: systemId.clone(),
                    done,
                    total,
                    current_title: g.title.clone(),
                    last_action: "no match".to_string(),
                });
            }
        }
    }

    log::info!(
        "rom_hashes: resolve_rom_hashes_for_system({systemId}) — scanned={} matched={} unmatched={} skipped_cd={} errors={}",
        summary.scanned, summary.matched, summary.unmatched, summary.skipped_cd, summary.errors,
    );
    let _ = app.emit("oa://rom-hash-resolve-complete", &summary);
    Ok(summary)
}

/// Diagnostic / debugger surface — look up a single sha1 in
/// `rom_hashes` and return the matched canonical entry (if any). Used
/// by per-game UI surfaces that want to explain "why is this named X?".
#[tauri::command]
pub fn lookup_rom_hash(
    sha1: String,
    db: tauri::State<'_, LibraryDb>,
) -> Result<Option<RomHashRow>, String> {
    db.lookup_rom_hash(&sha1)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_DAT: &str = r#"clrmamepro (
	name "NEC - PC Engine - TurboGrafx 16"
)

game (
	name "Bonk's Adventure (USA)"
	description "Bonk's Adventure (USA)"
	rom ( name "Bonk's Adventure (USA).pce" size 393216 crc 4F0BB6D2 md5 4F0BB6D24F0BB6D24F0BB6D24F0BB6D2 sha1 E22B706B22B706B22B706B22B706B22B706B22B7 )
	serial "TGX040080"
)

game (
	name "Air Zonk (USA)"
	description "Air Zonk (USA)"
	rom ( name "Air Zonk (USA).pce" size 524288 crc DEADBEEF sha1 abc123abc123abc123abc123abc123abc123abcd )
)
"#;

    #[test]
    fn parser_extracts_sha1_and_serial() {
        let entries = parse_libretro_dat(SAMPLE_DAT, "tg16");
        assert_eq!(entries.len(), 2, "expected 2 entries, got {entries:?}");
        let bonk = &entries[0];
        assert_eq!(bonk.game_name, "Bonk's Adventure (USA)");
        assert_eq!(bonk.serial.as_deref(), Some("TGX040080"));
        assert_eq!(bonk.sha1.len(), 40);
        assert_eq!(bonk.crc32.as_deref(), Some("4f0bb6d2"));
        assert_eq!(bonk.size_bytes, Some(393_216));
        let zonk = &entries[1];
        assert_eq!(zonk.game_name, "Air Zonk (USA)");
        assert_eq!(zonk.serial, None);
    }

    #[test]
    fn parser_ignores_clrmamepro_header() {
        let entries = parse_libretro_dat(SAMPLE_DAT, "tg16");
        // Sample has 1 clrmamepro block + 2 game blocks. If the parser
        // ate the header, name="NEC - PC Engine - TurboGrafx 16" would
        // show up as a row.
        assert!(entries.iter().all(|e| e.game_name != "NEC - PC Engine - TurboGrafx 16"));
    }

    #[test]
    fn parser_skips_rom_blocks_without_sha1() {
        let dat = r#"
game (
	name "No Hash Entry"
	rom ( name "X.pce" size 1024 crc 11111111 )
)
"#;
        let entries = parse_libretro_dat(dat, "tg16");
        assert!(entries.is_empty(), "rom block without sha1 should be skipped");
    }

    #[test]
    fn token_iter_handles_quoted_values() {
        let mut it = TokenIter::new(r#"name "Bonk's Adventure (USA).pce" size 393216 sha1 ABC"#);
        let (k, v) = it.next_pair().unwrap();
        assert_eq!(k, "name");
        assert_eq!(v, "Bonk's Adventure (USA).pce");
        let (k, v) = it.next_pair().unwrap();
        assert_eq!(k, "size");
        assert_eq!(v, "393216");
        let (k, v) = it.next_pair().unwrap();
        assert_eq!(k, "sha1");
        assert_eq!(v, "ABC");
        assert!(it.next_pair().is_none());
    }

    #[test]
    fn cd_container_extensions_are_filtered() {
        for ext in ["cue", "chd", "ccd", "toc", "m3u", "iso", "bin"] {
            assert!(is_cd_container_ext(ext), "{ext} should be a CD container");
        }
        for ext in ["pce", "nes", "smc", "sfc", "lnx"] {
            assert!(!is_cd_container_ext(ext), "{ext} should NOT be a CD container");
        }
    }

    #[test]
    fn sha1_of_raw_file_hashes_bytes() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-romhash-raw-{}-{}.bin",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        std::fs::write(&tmp, b"abc").unwrap();
        let got = sha1_of_rom(tmp.to_str().unwrap(), None).expect("hash");
        // sha1("abc") = a9993e364706816aba3e25717850c26c9cd0d89d
        assert_eq!(got, "a9993e364706816aba3e25717850c26c9cd0d89d");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn sha1_of_archived_entry_decodes_encoded_path() {
        use std::io::Write;
        // Build a tiny in-memory zip on disk containing one file with
        // known content, then verify sha1_of_rom both with the encoded
        // `<zip>#<inner>` shape AND with the inner_path argument set.
        let tmp = std::env::temp_dir().join(format!(
            "oa-romhash-zip-{}-{}.zip",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        {
            let file = std::fs::File::create(&tmp).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            zip.start_file::<_, ()>("Xexyz (USA).nes", zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"abc").unwrap();
            zip.finish().unwrap();
        }
        let encoded = format!("{}#Xexyz (USA).nes", tmp.display());
        // The library stores BOTH the encoded path (in file_path) AND
        // the inner (in archive_inner_path). Make sure both routings work.
        let with_inner = sha1_of_rom(&encoded, Some("Xexyz (USA).nes")).expect("hash with inner");
        assert_eq!(with_inner, "a9993e364706816aba3e25717850c26c9cd0d89d");
        let only_encoded = sha1_of_rom(&encoded, None).expect("hash encoded only");
        assert_eq!(only_encoded, "a9993e364706816aba3e25717850c26c9cd0d89d");
        let _ = std::fs::remove_file(&tmp);
    }
}
