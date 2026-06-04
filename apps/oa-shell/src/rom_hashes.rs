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

use rayon::prelude::*;
use serde::Serialize;
use sha1::{Digest, Sha1};
use tauri::Emitter;

use crate::archive;
use crate::library_db::{GameSerialRow, LibraryDb, RomHashRow, RomTrackRow};
use crate::rom_header::{candidate_sha1s, HeaderRule, Sha1Candidate};

const HASH_CACHE_TTL_SECS: u64 = 86_400; // 24h, matches metadat cache
const HASH_DB_CACHE_DIR: &str = "library-db/hashes";

/// A reference to one libretro-database `.dat` file. The repo lays its
/// dat files out as `<subdir>/<basename>.dat` and the same system can
/// have multiple dat files across subdirs — `metadat/no-intro/<sys>.dat`
/// for the canonical unheadered hashes, plus an optional
/// `metadat/headered/<sys>.dat` carrying headered-variant hashes for the
/// systems (NES iNES, Lynx LNX, Atari 7800 A78) where the upstream
/// curator maintains both. Both subdirs land in our local `rom_hashes`
/// table — different sha1s, same `canonical_title` — so headered and
/// unheadered files both hit directly.
#[derive(Debug, Clone, Copy)]
pub struct DatRef {
    pub subdir: &'static str,
    pub basename: &'static str,
}


/// libretro-database `.dat` references for `system_id`, sourced from
/// `config/systems/<id>/system.yaml::libretro_dat_refs`.
///
/// **Implementation note (cache + leak):** [`DatRef`] carries
/// `&'static str` fields because every consumer downstream
/// (`fetch_libretro_dat`, `fetch_and_parse_all`) treats refs as if
/// they were process-lifetime data. The registry returns owned
/// `String`s, so we `Box::leak` once per system into a
/// process-lifetime cache. Bounded one-time allocation per system
/// (~50 bytes × 41 systems = ~2 KB lifetime).
///
/// Returns an empty slice for unregistered system_id or systems
/// without a `libretro_dat_refs` block (the system has no upstream
/// dat).
pub fn libretro_dat_refs_for_system_resolved(system_id: &str) -> &'static [DatRef] {
    use std::sync::{Mutex, OnceLock};

    static CACHE: OnceLock<Mutex<HashMap<String, &'static [DatRef]>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(&refs) = cache.lock().unwrap().get(system_id) {
        return refs;
    }

    let resolved: &'static [DatRef] = crate::system_registry::global_registry()
        .get(system_id)
        .map(|loaded| {
            let refs: Vec<DatRef> = loaded
                .descriptor
                .libretro_dat_refs
                .iter()
                .map(|r| DatRef {
                    subdir: Box::leak(r.subdir.clone().into_boxed_str()),
                    basename: Box::leak(r.basename.clone().into_boxed_str()),
                })
                .collect();
            let slice: &'static [DatRef] = Box::leak(refs.into_boxed_slice());
            slice
        })
        .unwrap_or(&[]);
    cache.lock().unwrap().insert(system_id.to_string(), resolved);
    resolved
}

/// Extensions we deliberately skip when computing hashes — CD-container
/// formats whose content hash means something different from "the
/// canonical ROM bytes." See module docs.
///
/// `.bin` and `.iso` are special: they're CD-container extensions ONLY
/// on systems that use disc-based media. On cart systems (Atari 2600,
/// ColecoVision, Intellivision, Odyssey², Channel F, etc.) a `.bin`
/// file is the raw cart dump and SHOULD be hashed normally. Pre-fix
/// the function treated `.bin` as CD universally, silently routing
/// 2600/Coleco/Intv/O2 `.bin` ROMs to the disc-id peek (which
/// returns "no signature" → skipped_cd) instead of hashing them
/// against the no-intro DAT.
pub(crate) fn is_cd_container_ext(ext: &str, system_id: &str) -> bool {
    // Universal CD-container extensions — these only ever appear on
    // disc-based systems and always mean "skip the hash, do disc-id".
    if matches!(ext, "cue" | "chd" | "ccd" | "toc" | "m3u") {
        return true;
    }
    // `.bin` and `.iso` only count as CD on systems that actually use
    // disc media. Cart systems' `.bin` dumps go through the normal
    // hash path.
    if matches!(ext, "bin" | "iso") {
        return is_disc_shape_system(system_id);
    }
    false
}

/// Disc-shape (CD/DVD-based) systems. Drives:
/// - container-extension routing inside `is_cd_container_ext`,
/// - sync-time dispatch in `sync_rom_hashes_for_system` (Phase A1):
///   disc systems additionally populate `rom_hashes_tracks` +
///   `disc_sets` alongside the existing cart-shape `rom_hashes` write,
/// - per-track lookup branching in the identify flow (planned for
///   Sub-phase 3 of A1).
///
/// Membership matches the redump-catalogued disc lineup as of
/// 2026-06-03. Adding a new disc-shape system means registering it
/// here AND ensuring its `config/systems/<id>/system.yaml` lists a
/// redump `libretro_dat_refs` entry.
pub(crate) fn is_disc_shape_system(system_id: &str) -> bool {
    matches!(
        system_id,
        "pce-cd"
            | "segacd"
            | "saturn"
            | "psx"
            | "ps2"
            | "neocd"
            | "pcfx"
            | "gamecube"
            | "dreamcast"
            | "3do"
            | "psp"
    )
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
    /// Serial → canonical title pivot, parallel to `entries`. Default is
    /// empty so caches written by older builds (no `serials` field on
    /// disk) deserialize cleanly; first sync after upgrade refills it.
    #[serde(default)]
    serials: Vec<GameSerialRow>,
    /// Phase A1 — per-track canonical rows for disc-shape systems.
    /// Empty for cart-shape systems AND for caches written by builds
    /// before 2026-06-03. The first disc-shape sync after upgrade
    /// refills this; older caches still deserialize cleanly via the
    /// serde default.
    #[serde(default)]
    tracks: Vec<RomTrackRow>,
    /// Phase A1 — multi-disc parent groupings auto-detected from the
    /// `Foo (Disc N)` title pattern at parse time. Same backward-compat
    /// shape as `tracks` (default empty for older caches).
    #[serde(default)]
    disc_set_candidates: Vec<DiscSetCandidate>,
}

/// Output of one `parse_libretro_dat` pass — every shape the upstream
/// `.dat` produces. `rom_hashes` is one row per `rom ( ... sha1 ... )`
/// (consumed by cart-shape sync); `game_serials` is one row per
/// `game (... serial ...)` block; `rom_hashes_tracks` is one row per
/// non-sidecar track for disc-shape sync (Phase A1 of the virtual
/// library + launcher arc — see `docs/PLANS/disc-track-sha1-matching.md`);
/// `disc_set_candidates` flags multi-disc parent groupings detected
/// from the title pattern `Foo (Disc N)`.
///
/// `rom_hashes` + `rom_hashes_tracks` are both populated unconditionally;
/// the sync flow decides which to persist based on the system's
/// disc-shape-ness (cart systems write `rom_hashes`; disc systems write
/// `rom_hashes_tracks`). This avoids parse-time coupling to the
/// system-shape registry.
pub struct ParsedDat {
    pub rom_hashes: Vec<RomHashRow>,
    pub game_serials: Vec<GameSerialRow>,
    pub rom_hashes_tracks: Vec<RomTrackRow>,
    pub disc_set_candidates: Vec<DiscSetCandidate>,
}

/// One multi-disc set detected from `Foo (Disc N)` title patterns.
/// Emitted when a system has ≥ 2 distinct disc numbers under the same
/// base title. The sync flow persists these into the `disc_sets` table;
/// individual disc rows in `games` later attach via `disc_set_id`.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscSetCandidate {
    /// Base title with the `(Disc N)` suffix stripped.
    pub canonical_title: String,
    pub system_id: String,
    /// Max disc number observed in the set. Used as the canonical
    /// disc_count (covers dats that skip a disc number).
    pub disc_count: u32,
}

/// Extract a 1-based disc number from a game title carrying a
/// `(Disc N)` suffix. Returns the (base_title, disc_number) pair on
/// match; None otherwise. The base title is everything before the
/// `(Disc ` token, with trailing whitespace trimmed.
pub(crate) fn extract_disc_set_candidate(game_name: &str) -> Option<(String, u32)> {
    let start = game_name.find("(Disc ")?;
    let rest = &game_name[start + 6..];
    let end = rest.find(')')?;
    let n = rest[..end].trim().parse::<u32>().ok()?;
    let base = game_name[..start].trim_end().to_string();
    if base.is_empty() {
        return None;
    }
    Some((base, n))
}

/// Extract a 1-based track number from a rom filename. Looks for a
/// `(Track NN)` substring (redump convention). Falls back to
/// `position_in_block` (1-indexed) when no `(Track NN)` is present.
fn extract_track_number(filename: &str, position_in_block: u32) -> u32 {
    if let Some(start) = filename.find("(Track ") {
        let rest = &filename[start + 7..];
        if let Some(end) = rest.find(')') {
            if let Ok(n) = rest[..end].trim().parse::<u32>() {
                return n;
            }
        }
    }
    position_in_block
}

/// Derive a heuristic track_mode string from the rom filename + track
/// number. Returns `None` for sidecar files we deliberately skip from
/// the per-track table (.cue / .gdi / .toc / .ccd / .sub / .sbi / .sbx)
/// — their SHA-1s vary by dump tool and aren't useful for the
/// identification matcher.
///
/// Mode field is informational only — the operator-side hashing path
/// reads the real mode from cue / CHD metadata. This heuristic exists
/// so the canonical table's `track_mode` column is populated with
/// something sensible at sync time.
fn derive_track_mode(filename: &str, track_number: u32) -> Option<String> {
    let lower = filename.to_ascii_lowercase();
    for sidecar in &[".cue", ".gdi", ".toc", ".ccd", ".sub", ".sbi", ".sbx"] {
        if lower.ends_with(sidecar) {
            return None;
        }
    }
    if lower.ends_with(".iso") {
        return Some("MODE1/2048".to_string());
    }
    // Heuristic — track 01 is almost always the data track on CD
    // systems; tracks 2+ are usually audio. Wrong for some games (e.g.
    // PCE-CD games with multiple data tracks); refined at operator-side
    // hashing time.
    if track_number == 1 {
        Some("DATA".to_string())
    } else {
        Some("AUDIO".to_string())
    }
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
pub fn parse_libretro_dat(content: &str, system_id: &str) -> ParsedDat {
    /// One pending rom entry while we're inside a `game (...)` block.
    /// game.serial may appear before OR after the `rom ( ... )` line
    /// depending on the upstream dat, so we accumulate and stamp the
    /// final serial onto every rom in the block at the closing `)`.
    /// `name` is the rom's filename ("Tomb Raider (USA) (Track 01).bin")
    /// — needed for track_number + track_mode derivation in disc-shape
    /// outputs.
    struct PendingRom {
        name: String,
        sha1: String,
        crc32: Option<String>,
        size_bytes: Option<i64>,
    }

    let mut rom_hashes: Vec<RomHashRow> = Vec::new();
    let mut game_serials: Vec<GameSerialRow> = Vec::new();
    let mut rom_hashes_tracks: Vec<RomTrackRow> = Vec::new();
    // (system_id, base_title) → set of disc numbers observed. Finalized
    // after the parse loop; sets with ≥ 2 distinct disc numbers become
    // DiscSetCandidate rows.
    let mut disc_set_first_pass: HashMap<(String, String), Vec<u32>> =
        HashMap::new();
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
                let mut position_in_block: u32 = 0;
                for r in pending.drain(..) {
                    rom_hashes.push(RomHashRow {
                        sha1: r.sha1.clone(),
                        system_id: system_id.to_string(),
                        game_name: name.clone(),
                        serial: current_serial.clone(),
                        crc32: r.crc32.clone(),
                        size_bytes: r.size_bytes,
                    });
                    // Disc-shape per-track row. Skip sidecar files
                    // (.cue / .gdi / etc — derive_track_mode returns
                    // None) and rows where the dat omitted `size` (the
                    // schema requires NOT NULL).
                    position_in_block += 1;
                    let track_number = extract_track_number(&r.name, position_in_block);
                    if let Some(track_mode) = derive_track_mode(&r.name, track_number) {
                        if let Some(size_bytes) = r.size_bytes {
                            rom_hashes_tracks.push(RomTrackRow {
                                sha1: r.sha1,
                                system_id: system_id.to_string(),
                                game_name: name.clone(),
                                serial: current_serial.clone(),
                                track_number,
                                track_mode,
                                size_bytes,
                            });
                        }
                    }
                }
                // Emit one game_serials row per game block that carries
                // a serial. Title is the same canonical `name` we'd have
                // stamped via apply_rom_hash on a sha1 hit. Region stays
                // None for now — extracting it from "(USA)" / "(Japan)"
                // suffixes is a Phase 2b polish item.
                if let Some(serial) = current_serial.take() {
                    if !serial.is_empty() {
                        game_serials.push(GameSerialRow {
                            system_id: system_id.to_string(),
                            serial,
                            canonical_title: name.clone(),
                            region: None,
                        });
                    }
                }
                // Multi-disc parent detection from `Foo (Disc N)`
                // pattern. Finalized after the parse loop into
                // disc_set_candidates.
                if let Some((base_title, disc_n)) = extract_disc_set_candidate(&name) {
                    disc_set_first_pass
                        .entry((system_id.to_string(), base_title))
                        .or_default()
                        .push(disc_n);
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
            // Keys/values can come in any order; we want name + sha1 +
            // crc + size. Trailing `)` is on the same line for
            // libretro-database. Strip both ends defensively.
            let body = rest.trim_end_matches(')').trim();
            let mut name: Option<String> = None;
            let mut sha1: Option<String> = None;
            let mut crc32: Option<String> = None;
            let mut size: Option<i64> = None;
            let mut tokens = TokenIter::new(body);
            while let Some((key, value)) = tokens.next_pair() {
                match key.as_str() {
                    "name"   => name = Some(value.to_string()),
                    "sha1"   => sha1 = Some(value.to_ascii_lowercase()),
                    "crc"    => crc32 = Some(value.to_ascii_lowercase()),
                    "size"   => size = value.parse::<i64>().ok(),
                    _ => {}
                }
            }
            if let Some(sha1) = sha1 {
                pending.push(PendingRom {
                    name: name.unwrap_or_default(),
                    sha1,
                    crc32,
                    size_bytes: size,
                });
            }
        }
    }
    // De-dupe game_serials by (system_id, serial). Some upstream .dats
    // ship multiple regional dumps under the same serial; first
    // occurrence wins (matches what INSERT-OR-REPLACE would do on a
    // database round-trip, just without the round-trip).
    let mut seen = HashMap::new();
    let mut deduped = Vec::with_capacity(game_serials.len());
    for row in game_serials {
        if seen.insert((row.system_id.clone(), row.serial.clone()), ()).is_none() {
            deduped.push(row);
        }
    }
    // Finalize disc-set candidates. Same base title across regions
    // (Foo (Disc 1) (USA) + Foo (Disc 1) (Japan)) produces a single
    // disc_n entry per region — dedup so we count distinct discs, not
    // distinct dat entries. A "set" requires ≥ 2 distinct discs;
    // disc_count is the max observed disc number (covers dats that
    // skip a number, rare but possible).
    let mut disc_set_candidates: Vec<DiscSetCandidate> = disc_set_first_pass
        .into_iter()
        .filter_map(|((sys, base), mut nums)| {
            nums.sort_unstable();
            nums.dedup();
            if nums.len() < 2 {
                return None;
            }
            let disc_count = *nums.iter().max().unwrap_or(&0);
            Some(DiscSetCandidate {
                canonical_title: base,
                system_id: sys,
                disc_count,
            })
        })
        .collect();
    // Stable output ordering — simplifies snapshot-style tests + makes
    // the sync flow's bulk-replace deterministic.
    disc_set_candidates.sort_by(|a, b| {
        a.system_id
            .cmp(&b.system_id)
            .then_with(|| a.canonical_title.cmp(&b.canonical_title))
    });
    ParsedDat {
        rom_hashes,
        game_serials: deduped,
        rom_hashes_tracks,
        disc_set_candidates,
    }
}

/// Parse a MAME-style clrmamepro dat into name-keyed title rows.
/// Source format from libretro-database `metadat/mame/MAME.dat`:
///
/// ```text
/// game (
///     name "Street Fighter II: Champion Edition (World 920313)"
///     year "1992"
///     developer "Capcom"
///     rom ( name sf2ce.zip size 5042192 crc XXXXX md5 XXXXX sha1 XXXXX )
/// )
/// ```
///
/// Each entry produces one [`MameTitleRow`] keyed by the .zip basename
/// (lowercased, extension stripped). Games with multiple `rom (...)`
/// lines emit one row per .zip — covers MAME's "merged" sets where a
/// parent + clones share a single game block.
pub fn parse_mame_dat(content: &str) -> Vec<crate::library_db::MameTitleRow> {
    let mut out = Vec::new();
    let mut in_game = false;
    let mut current_name: Option<String> = None;
    let mut current_year: Option<String> = None;
    let mut current_dev: Option<String> = None;
    let mut pending_zips: Vec<String> = Vec::new();
    for raw in content.lines() {
        let line = raw.trim();
        if line.starts_with("game (") {
            in_game = true;
            current_name = None;
            current_year = None;
            current_dev = None;
            pending_zips.clear();
            continue;
        }
        if !in_game {
            continue;
        }
        if line == ")" {
            in_game = false;
            if let Some(title) = current_name.take() {
                for zip in pending_zips.drain(..) {
                    let stem = std::path::Path::new(&zip)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_ascii_lowercase())
                        .unwrap_or_else(|| zip.to_ascii_lowercase());
                    if stem.is_empty() {
                        continue;
                    }
                    out.push(crate::library_db::MameTitleRow {
                        rom_set: stem,
                        title: title.clone(),
                        year: current_year.clone(),
                        developer: current_dev.clone(),
                    });
                }
            }
            current_year = None;
            current_dev = None;
            pending_zips.clear();
            continue;
        }
        if let Some(rest) = line.strip_prefix("name ") {
            current_name = Some(unquote(rest));
            continue;
        }
        if let Some(rest) = line.strip_prefix("year ") {
            current_year = Some(unquote(rest));
            continue;
        }
        if let Some(rest) = line.strip_prefix("developer ") {
            current_dev = Some(unquote(rest));
            continue;
        }
        if let Some(rest) = line.strip_prefix("rom (") {
            // Extract the `name` field — that's the .zip filename.
            let body = rest.trim_end_matches(')').trim();
            let mut tokens = TokenIter::new(body);
            while let Some((key, value)) = tokens.next_pair() {
                if key == "name" && value.ends_with(".zip") {
                    pending_zips.push(value);
                    break;
                }
            }
        }
    }
    out
}

/// Strip at most one leading and one trailing `"` from a trimmed
/// string. Pre-fix this used `trim_matches('"')` which strips ALL
/// leading + trailing quotes — a value like `""` becomes `""`
/// (correct) but `"Foo\"bar\""` with naïvely-escaped inner quotes
/// would round-trip to `Foo\"bar\"` rather than the intended
/// `Foo"bar"`. clrmamepro dats are well-formed in practice and
/// this is a defensive tightening rather than a bug fix — but the
/// stricter form matches the intent (treat the outer quotes as
/// delimiters, leave inner bytes alone).
fn unquote(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
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

/// Fetch one `<subdir>/<basename>.dat` from libretro-database via the
/// raw GitHub URL. 404 = upstream has no .dat at that path; we return
/// Ok(None) so the caller can fall through to the next ref (or report
/// "system has no dats") without surfacing as a hard error.
/// MAME title-lookup sync. Fetches `metadat/mame/MAME.dat` from libretro-
/// database, parses the clrmamepro entries into name-keyed rows, and
/// bulk-inserts into the `mame_titles` table. Frontend calls this on the
/// first MAME ingest (or via a manual "Sync MAME titles" action); the
/// result is consulted on subsequent imports so library tiles show human
/// titles instead of .zip filenames.
#[tauri::command]
pub async fn sync_mame_titles(
    db: tauri::State<'_, LibraryDb>,
) -> Result<MameTitleSyncSummary, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(180))
        .build()
        .map_err(|e| format!("reqwest client: {e}"))?;
    let dat_ref = DatRef { subdir: "metadat/mame", basename: "MAME" };
    let body = match fetch_libretro_dat(&client, dat_ref).await? {
        Some(b) => b,
        None => {
            return Err("libretro-database has no MAME.dat at metadat/mame/MAME.dat".to_string());
        }
    };
    let entries = parse_mame_dat(&body);
    let upstream_entries = entries.len();
    let written = db.replace_mame_titles(&entries)?;
    log::info!("rom_hashes: mame_titles synced — {upstream_entries} parsed, {written} written");
    Ok(MameTitleSyncSummary { upstream_entries, written })
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MameTitleSyncSummary {
    pub upstream_entries: usize,
    pub written: usize,
}

/// Frontend ingest helper — look up a MAME ROM-set by .zip basename
/// (e.g. "sf2ce") and return the human title + year + developer. Returns
/// `null` if the rom_set isn't in the catalog (homebrew, hack, or an
/// older MAME set the catalog hasn't picked up yet).
#[tauri::command]
#[allow(non_snake_case)]
pub fn lookup_mame_title(
    romSet: String,
    db: tauri::State<'_, LibraryDb>,
) -> Result<Option<crate::library_db::MameTitleRow>, String> {
    db.lookup_mame_title(&romSet)
}

async fn fetch_libretro_dat(
    client: &reqwest::Client,
    dat_ref: DatRef,
) -> Result<Option<String>, String> {
    let url = format!(
        "https://raw.githubusercontent.com/libretro/libretro-database/master/{}/{}.dat",
        dat_ref.subdir,
        urlencoding::encode(dat_ref.basename),
    );
    // get_text_with_retry retries once on 5xx / network errors. A
    // single 404 (no dat at this subdir for this system) returns
    // Ok(None) and is logged at debug level.
    let result = crate::http_retry::get_text_with_retry(client, &url, "OverlookedArcade").await;
    if let Ok(None) = &result {
        log::debug!("rom_hashes: {url} 404 (no dat at this path)");
    }
    result
}

/// Fetch + parse every DatRef registered for a system, merging the
/// results into one `ParsedDat`. A 404 on any individual ref is logged
/// and skipped (some systems intentionally have only the primary `.dat`
/// and no headered variant). Returns `Ok(None)` only when EVERY ref
/// 404s — at which point the caller treats the system as having no
/// upstream coverage at all.
async fn fetch_and_parse_all(
    client: &reqwest::Client,
    refs: &[DatRef],
    system_id: &str,
) -> Result<Option<ParsedDat>, String> {
    let mut all_rom_hashes = Vec::new();
    let mut all_serials = Vec::new();
    let mut all_tracks = Vec::new();
    let mut all_disc_sets = Vec::new();
    let mut any_hit = false;
    for r in refs {
        match fetch_libretro_dat(client, *r).await? {
            Some(text) => {
                any_hit = true;
                let parsed = parse_libretro_dat(&text, system_id);
                log::info!(
                    "rom_hashes: {system_id} {}/{} → {} entries / {} serials / {} tracks / {} disc-sets",
                    r.subdir,
                    r.basename,
                    parsed.rom_hashes.len(),
                    parsed.game_serials.len(),
                    parsed.rom_hashes_tracks.len(),
                    parsed.disc_set_candidates.len(),
                );
                all_rom_hashes.extend(parsed.rom_hashes);
                all_serials.extend(parsed.game_serials);
                all_tracks.extend(parsed.rom_hashes_tracks);
                all_disc_sets.extend(parsed.disc_set_candidates);
            }
            None => {
                log::debug!(
                    "rom_hashes: {system_id} {}/{}: 404, skipping",
                    r.subdir,
                    r.basename
                );
            }
        }
    }
    if !any_hit {
        return Ok(None);
    }
    // Dedupe merged serials by (system_id, serial) — first wins, same as
    // the parser's per-dat dedupe. Across dats (no-intro vs headered) the
    // serial is identical for the same game; we just want one row.
    let mut seen = HashMap::new();
    all_serials.retain(|row| {
        seen.insert((row.system_id.clone(), row.serial.clone()), ()).is_none()
    });
    // Cross-dat dedupe for disc-set candidates by (system_id,
    // canonical_title), keeping the max observed disc_count. Most
    // systems have one redump dat so cross-dat overlap is rare, but
    // headered/unheadered split dats CAN produce duplicate entries
    // (same multi-disc parent across both); cleaner to merge here than
    // in the sync layer.
    let mut disc_by_key: HashMap<(String, String), u32> = HashMap::new();
    for cand in all_disc_sets {
        let key = (cand.system_id, cand.canonical_title);
        let entry = disc_by_key.entry(key).or_insert(0);
        if cand.disc_count > *entry {
            *entry = cand.disc_count;
        }
    }
    let mut merged_disc_sets: Vec<DiscSetCandidate> = disc_by_key
        .into_iter()
        .map(|((sys, title), count)| DiscSetCandidate {
            canonical_title: title,
            system_id: sys,
            disc_count: count,
        })
        .collect();
    merged_disc_sets.sort_by(|a, b| {
        a.system_id
            .cmp(&b.system_id)
            .then_with(|| a.canonical_title.cmp(&b.canonical_title))
    });
    Ok(Some(ParsedDat {
        rom_hashes: all_rom_hashes,
        game_serials: all_serials,
        rom_hashes_tracks: all_tracks,
        disc_set_candidates: merged_disc_sets,
    }))
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
    use tauri::Manager;
    // Per-system op gate (H11) — see sync_media_for_system in media.rs.
    let gate = state.gate_for(&systemId);
    let _gate_guard = gate.lock().await;

    // Phase 4a dat_sync — RAII guard owns the bar row. Drop
    // auto-fails on `?` early-return (2026-06-04 refactor).
    let registry_state = app.try_state::<crate::job_registry::JobRegistry>();
    let scope = crate::job_registry::JobScope::start(
        registry_state.as_deref(),
        crate::job_registry::JobKind::DatSync {
            system_id: systemId.clone(),
        },
        format!("Updating ROM database — {}", systemId),
        Some(systemId.clone()),
        None,
        false,
        "entries",
        None,
    );

    let app_data_dir = state.app_data_dir.clone();
    let refs = libretro_dat_refs_for_system_resolved(&systemId);
    if refs.is_empty() {
        log::info!("rom_hashes: no libretro-database mapping for {systemId}; skipping sync");
        scope.complete();
        return Ok(RomHashSyncSummary {
            system_id: systemId,
            upstream_entries: 0,
            written: 0,
            from_cache: false,
        });
    }

    // Cache lookup — same TTL as the metadat cache.
    let cache_path = hash_cache_path(&app_data_dir, &systemId);
    if let Ok(bytes) = std::fs::read(&cache_path) {
        if let Ok(cached) = serde_json::from_slice::<CachedHashDb>(&bytes) {
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            // Phase A1 fix — disc-shape systems whose cache file
            // was written before Sub-phase 1 deserialize with
            // `tracks: []` via the #[serde(default)]. The cache-hit
            // path would then wipe rom_hashes_tracks on every sync
            // within the 24h TTL window. Treat that as a stale cache
            // and fall through to fresh fetch instead.
            let cache_is_useful_for_disc = !is_disc_shape_system(&systemId)
                || !cached.tracks.is_empty();
            if !cached.entries.is_empty()
                && cache_is_useful_for_disc
                && now.saturating_sub(cached.fetched_at_unix_secs) < HASH_CACHE_TTL_SECS
            {
                log::info!(
                    "rom_hashes: {systemId} cache hit ({} entries, {} serials, {} tracks, {} disc-sets)",
                    cached.entries.len(),
                    cached.serials.len(),
                    cached.tracks.len(),
                    cached.disc_set_candidates.len(),
                );
                let upstream_entries = cached.entries.len();
                // Wipe-and-replace per system so the local table mirrors
                // the cached upstream snapshot exactly — entries removed
                // upstream don't linger as orphans, and entries the user
                // hand-imported into the wrong system aren't preserved
                // through a sync.
                let written = db.replace_rom_hashes_for_system(&systemId, &cached.entries)?;
                // Older caches deserialize with serials=[] (serde
                // default). Replacing with an empty slice clears the
                // table for the system; the next fresh fetch fills it
                // back in once the TTL expires.
                let _ = db.replace_game_serials_for_system(&systemId, &cached.serials);
                // Phase A1 disc-shape dispatch — write rom_hashes_tracks
                // + disc_sets for systems that hash per-track. Cached
                // files written by builds before 2026-06-03 have these
                // fields empty (serde default); the next 24h cache cycle
                // refills them via the fresh-fetch path below.
                if is_disc_shape_system(&systemId) {
                    let n_tracks = db.replace_rom_hashes_tracks_for_system(
                        &systemId,
                        &cached.tracks,
                    )?;
                    let disc_set_entries: Vec<(String, u32)> = cached
                        .disc_set_candidates
                        .iter()
                        .map(|c| (c.canonical_title.clone(), c.disc_count))
                        .collect();
                    let n_sets = db.upsert_disc_sets_for_system(
                        &systemId,
                        &disc_set_entries,
                    )?;
                    log::info!(
                        "rom_hashes: {systemId} disc-shape cache hit — wrote {n_tracks} tracks + {n_sets} disc-sets"
                    );
                }
                let _ = app.emit("oa://rom-hashes-synced", &RomHashSyncSummary {
                    system_id: systemId.clone(),
                    upstream_entries,
                    written,
                    from_cache: true,
                });
                scope.set_total(upstream_entries as i64);
                scope.complete();
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

    let ParsedDat {
        rom_hashes: entries,
        game_serials: serials,
        rom_hashes_tracks: tracks,
        disc_set_candidates,
    } = match fetch_and_parse_all(&client, refs, &systemId).await? {
        Some(p) => p,
        None => {
            log::warn!(
                "rom_hashes: upstream has no dat for {systemId} (every ref 404'd)"
            );
            scope.complete();
            return Ok(RomHashSyncSummary {
                system_id: systemId,
                upstream_entries: 0,
                written: 0,
                from_cache: false,
            });
        }
    };
    let upstream_entries = entries.len();
    let upstream_serials = serials.len();
    let upstream_tracks = tracks.len();
    let upstream_disc_sets = disc_set_candidates.len();
    log::info!(
        "rom_hashes: merged {upstream_entries} entries / {upstream_serials} serials / {upstream_tracks} tracks / {upstream_disc_sets} disc-sets for {systemId}"
    );
    // Replace the system's corpus rather than merging — upstream is the
    // source of truth, and stale entries from a prior sync with a
    // narrower ref set (or a since-removed upstream row) should not
    // linger.
    let written = db.replace_rom_hashes_for_system(&systemId, &entries)?;
    let _ = db.replace_game_serials_for_system(&systemId, &serials);
    // Phase A1 disc-shape dispatch — write rom_hashes_tracks + disc_sets
    // alongside the existing rom_hashes write. Cart-shape systems
    // (where is_disc_shape_system returns false) skip both, matching
    // pre-A1 behaviour exactly. Disc-shape systems get the per-track
    // canonical lookup plus multi-disc parent groupings.
    if is_disc_shape_system(&systemId) {
        let n_tracks = db.replace_rom_hashes_tracks_for_system(&systemId, &tracks)?;
        let disc_set_entries: Vec<(String, u32)> = disc_set_candidates
            .iter()
            .map(|c| (c.canonical_title.clone(), c.disc_count))
            .collect();
        let n_sets = db.upsert_disc_sets_for_system(&systemId, &disc_set_entries)?;
        log::info!(
            "rom_hashes: {systemId} disc-shape — wrote {n_tracks} tracks + {n_sets} disc-sets"
        );
    }

    // Cache write (24h reuse). Per-track + disc-set fields land in the
    // same file via the extended CachedHashDb shape; older builds
    // ignore them on read (serde defaults).
    if !entries.is_empty() {
        if let Some(parent) = cache_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let cached = CachedHashDb {
            fetched_at_unix_secs: now,
            entries,
            serials,
            tracks,
            disc_set_candidates,
        };
        let _ = std::fs::write(&cache_path, serde_json::to_vec(&cached).unwrap_or_default());
    }

    let summary = RomHashSyncSummary {
        system_id: systemId.clone(),
        upstream_entries,
        written,
        from_cache: false,
    };
    let _ = app.emit("oa://rom-hashes-synced", &summary);
    // Phase 4a dat_sync — finalize via the RAII guard. Any earlier
    // `?` propagation would auto-fail via Drop instead of leaving the
    // row stuck in `running`.
    scope.set_total(upstream_entries as i64);
    scope.complete();
    Ok(summary)
}

/// Load a ROM's bytes for hashing. Archived entries peek the archive at
/// `inner_path` (no extraction-to-disk); raw entries `std::fs::read`.
/// Caps at `MAX_ROM_BYTES` so a misclassified disc-image doesn't OOM us
/// — every cart ROM we care about sits well under the cap (SNES is 8 MB,
/// Lynx 1 MB, Atari 7800 144 KB). Past the cap we surface an error and
/// the caller skips the row (the resolve loop already swallows hash
/// errors and increments `summary.errors`).
///
/// `file_path` follows the library's encoded shape: archived entries
/// look like `"<archive_path>#<inner>"` (matches `encode_file_path` in
/// archive.rs); raw ROMs are a plain path. We always run the input
/// through `archive::decode_file_path` first so callers don't have to
/// duplicate the split logic.
const MAX_ROM_BYTES: u64 = 64 * 1024 * 1024;

pub(crate) fn rom_bytes_for(file_path: &str, archive_inner: Option<&str>) -> Result<Vec<u8>, String> {
    let (real_path, decoded_inner) = archive::decode_file_path(file_path);
    let inner = archive_inner.map(|s| s.to_string()).or_else(|| {
        if decoded_inner.is_empty() { None } else { Some(decoded_inner) }
    });
    if let Some(inner) = inner {
        return archive::read_inner_to_bytes(&real_path, &inner).map_err(|e| {
            format!("archive read {}#{inner}: {e}", real_path.display())
        });
    }
    let meta = std::fs::metadata(&real_path)
        .map_err(|e| format!("stat {}: {e}", real_path.display()))?;
    if meta.len() > MAX_ROM_BYTES {
        return Err(format!(
            "{} is {} bytes, exceeds {} byte cap — refusing to load into memory",
            real_path.display(),
            meta.len(),
            MAX_ROM_BYTES
        ));
    }
    std::fs::read(&real_path).map_err(|e| format!("read {}: {e}", real_path.display()))
}

/// Compute the SHA-1 of a ROM the way pre-header-aware code did — a
/// single hash of the canonical bytes (equivalent to the `Raw` rule).
/// Test-only: production resolve flow goes through `candidate_sha1s`
/// which tries header-stripped variants too.
#[cfg(test)]
fn sha1_of_rom(file_path: &str, archive_inner: Option<&str>) -> Result<String, String> {
    let bytes = rom_bytes_for(file_path, archive_inner)?;
    let mut hasher = Sha1::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

/// Streaming SHA-1 for large disc image files (PS2 / GameCube / Dreamcast
/// .iso dumps that can be 1.5–8 GB). Reads in 1 MB chunks so we never
/// hold more than ~1 MB of file bytes in memory at once. Used by the
/// resolve loop as a fallback after disc-ID extraction misses — catches
/// dumps the redump dat has SHA-1s for but where the disc-ID extraction
/// returned no signature.
pub(crate) fn stream_sha1_of_file(path: &std::path::Path) -> Result<String, String> {
    use std::io::Read;
    let f = std::fs::File::open(path)
        .map_err(|e| format!("open {}: {e}", path.display()))?;
    let mut reader = std::io::BufReader::with_capacity(1024 * 1024, f);
    let mut hasher = Sha1::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("read {}: {e}", path.display()))?;
        if n == 0 {
            break;
        }
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
    /// Number of canonical hash entries the local rom_hashes table held
    /// for this system at resolve time. When 0, every game in the
    /// system shows up as "unknown" — the UI uses this to surface
    /// "no hash DB available" rather than "we tried 1904 and found 0."
    pub canonical_entries: i64,
    /// Library row count for this system at resolve time. Compared
    /// against `already_identified` so the UI can show "all N games
    /// already identified, nothing to do" when a re-run is a no-op.
    pub library_total: i64,
    /// Subset of `library_total` whose `games.sha1` is already
    /// stamped (i.e. were resolved in a previous Identify pass).
    /// `library_total - already_identified` is roughly the work the
    /// current run will attempt (CD-shaped games further subtract
    /// out at the CD-skip path).
    pub already_identified: i64,
}

/// Hash every ROM in `system_id` that doesn't have a sha1 yet, look it
/// up in `rom_hashes`, and stamp the canonical title + serial on a
/// match. CD images are skipped (see module docs).
///
/// Auto-syncs the libretro-database `dat/<system>.dat` into our local
/// `rom_hashes` table if the system currently has zero canonical
/// entries — otherwise every game ends up "unknown" and the user has
/// no good signal as to why. The explicit `sync_rom_hashes_for_system`
/// command stays available for power users who want to force a refresh.
/// Run a one-shot upstream-dat fetch + parse + populate for `system_id`,
/// but only if the local `rom_hashes` table is currently empty for that
/// system. Mirrors the inline auto-sync that used to live at the head of
/// [`resolve_rom_hashes_for_system`].
///
/// Extracted (2026-06-01, Phase 1B Slice 1) so the smart-scan path in
/// `scan_service` can populate the hash table at scan time — without it,
/// a first-time operator's first scan can never light up the `confidence:
/// hash` tier because the `rom_hashes` table is still empty when the
/// scanner queries it. Calling this from `start_background_scan` before
/// the walk + hash pass closes that gap.
///
/// Failures are non-fatal — caller continues with an empty table if the
/// upstream is offline or the system has no `libretro_dat_refs_for_system`
/// mapping. Emits `oa://rom-hashes-synced` on success so any frontend
/// listener picks up the populated state.
/// Phase A1 fix — pure-Rust gate decision for `auto_sync_rom_hashes_if_empty`.
/// Extracted from the async function so the matrix can be unit-tested
/// without standing up a tokio runtime + Tauri state.
///
/// Returns `true` when the auto-sync should fetch fresh data. For
/// cart-shape systems this is "the cart table is empty"; for
/// disc-shape systems we ALSO require the per-track table to be
/// non-empty, otherwise the per-track identify path stays dormant
/// while the cart table looks populated (the bug Sub-phase 3.1 fixed).
fn auto_sync_needed(is_disc_shape: bool, cart_count: i64, track_count: i64) -> bool {
    if is_disc_shape {
        cart_count == 0 || track_count == 0
    } else {
        cart_count == 0
    }
}

pub(crate) async fn auto_sync_rom_hashes_if_empty(
    system_id: &str,
    app: &tauri::AppHandle,
    app_data_dir: &Path,
    db: &LibraryDb,
    // Phase 4c dependency graph — when the auto-sync is invoked from
    // resolve_rom_hashes_for_system, pass the HashResolve job id here
    // so the visible DatSync row in the bar is linked to its parent
    // (plan §"Job dependencies": auto-trigger prereqs). Pass None
    // when the caller has no parent (e.g. the scan_service pre-walk).
    parent_job_id: Option<i64>,
) -> Result<(), String> {
    use tauri::Manager;
    let cart_count = db.count_rom_hashes(system_id)?;
    let track_count = if is_disc_shape_system(system_id) {
        db.count_rom_hashes_tracks(system_id)?
    } else {
        // Cart-shape systems don't use the track table; treat as
        // "data present" so it doesn't drag the gate.
        1
    };
    if !auto_sync_needed(is_disc_shape_system(system_id), cart_count, track_count) {
        return Ok(());
    }
    let refs = libretro_dat_refs_for_system_resolved(system_id);
    if refs.is_empty() {
        log::info!("rom_hashes: no libretro-database mapping for {system_id}");
        return Ok(());
    }

    // Phase 4c — auto-sync as a visible DatSync prereq row in the
    // bar via a RAII guard. The parent_job_id link makes it clear
    // it's part of the HashResolve flow.
    let registry_state = app.try_state::<crate::job_registry::JobRegistry>();
    let label = if parent_job_id.is_some() {
        format!("Updating ROM database — {} (prereq)", system_id)
    } else {
        format!("Updating ROM database — {}", system_id)
    };
    let scope = crate::job_registry::JobScope::start(
        registry_state.as_deref(),
        crate::job_registry::JobKind::DatSync {
            system_id: system_id.to_string(),
        },
        label,
        Some(system_id.to_string()),
        parent_job_id,
        parent_job_id.is_some(),
        "entries",
        None,
    );

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            log::warn!("rom_hashes: reqwest client build failed: {e}");
            scope.fail(format!("reqwest client: {e}"));
            return Ok(());
        }
    };
    match fetch_and_parse_all(&client, refs, system_id).await {
        Ok(Some(ParsedDat {
            rom_hashes: entries,
            game_serials: serials,
            rom_hashes_tracks: tracks,
            disc_set_candidates,
        })) => {
            let n = entries.len();
            let s = serials.len();
            let nt = tracks.len();
            let nds = disc_set_candidates.len();
            log::info!(
                "rom_hashes: auto-sync {system_id} fetched {n} entries / {s} serials / {nt} tracks / {nds} disc-sets"
            );
            let _ = db.replace_rom_hashes_for_system(system_id, &entries);
            let _ = db.replace_game_serials_for_system(system_id, &serials);
            // Phase A1 disc-shape dispatch — same shape as the primary
            // sync_rom_hashes_for_system path. Cart-shape systems skip.
            if is_disc_shape_system(system_id) {
                let _ =
                    db.replace_rom_hashes_tracks_for_system(system_id, &tracks);
                let disc_set_entries: Vec<(String, u32)> = disc_set_candidates
                    .iter()
                    .map(|c| (c.canonical_title.clone(), c.disc_count))
                    .collect();
                let _ =
                    db.upsert_disc_sets_for_system(system_id, &disc_set_entries);
            }
            if !entries.is_empty() {
                let cache_path = hash_cache_path(app_data_dir, system_id);
                if let Some(parent) = cache_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let now = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let cached = CachedHashDb {
                    fetched_at_unix_secs: now,
                    entries,
                    serials,
                    tracks,
                    disc_set_candidates,
                };
                let _ = std::fs::write(
                    &cache_path,
                    serde_json::to_vec(&cached).unwrap_or_default(),
                );
            }
            let _ = app.emit(
                "oa://rom-hashes-synced",
                &RomHashSyncSummary {
                    system_id: system_id.to_string(),
                    upstream_entries: n,
                    written: n,
                    from_cache: false,
                },
            );
            scope.set_total(n as i64);
            scope.complete();
        }
        Ok(None) => {
            log::warn!(
                "rom_hashes: upstream has no dat for {system_id} (every ref 404'd)"
            );
            // No dat present upstream is a "success" from the prereq's
            // perspective — the parent (HashResolve) proceeds with an
            // empty hash table and lights up no canonical matches.
            scope.complete();
        }
        Err(e) => {
            log::warn!("rom_hashes: auto-sync {system_id} fetch failed: {e}");
            scope.fail(e);
        }
    }
    Ok(())
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn resolve_rom_hashes_for_system(
    systemId: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::media::MediaState>,
    db: tauri::State<'_, LibraryDb>,
) -> Result<RomResolveSummary, String> {
    use tauri::Manager;
    // Per-system op gate (H11) — held for the lifetime of this call.
    // resolve_rom_hashes_for_system also calls sync_rom_hashes_for_system
    // inline (auto-sync block below) — but that's a function inlined
    // here, not a separate Tauri command invocation, so it doesn't try
    // to re-acquire the gate. The auto-sync block bypasses the wrapper
    // function's own gate acquisition by inlining only the fetch +
    // parse + apply steps.
    let gate = state.gate_for(&systemId);
    let _gate_guard = gate.lock().await;

    // Read OA-wide prefs once at the top — used by the disc-shape
    // identification branches below.
    let library_prefs = crate::library_prefs::read_library_prefs(&state.app_data_dir);
    let disc_strictness = library_prefs.disc_track_strictness.to_engine();
    // Phase A1 pivot — per-track SHA-1 is gated behind an experimental
    // flag (default OFF) after the 2026-06-03 operator playtest
    // surfaced the chdman/redump byte divergence + archived-disc skip
    // limitation. The filename-fuzzy path below is the new primary
    // identification mechanism for disc-shape systems; per-track is
    // available for raw-dump libraries that opt in.
    let disc_track_experimental = library_prefs.disc_track_experimental_enabled;

    // Phase 4a hash_resolve wiring — register the job up front so the
    // bar surfaces "Identifying {system} ROMs" with a real n_hashed /
    // n_total. Total settles once `db.list_games_missing_hash` returns
    // (`total` local below); we tick progress after every per-game
    // iteration. Soft-fail when the registry isn't managed.
    //
    // Phase A1 Sub-phase 3 — disc-shape systems get a separate
    // `disc_track_hash` JobKind + a "discs" rather than "ROMs" label,
    // so the BackgroundJobsBar can surface the right progress shape
    // (per-disc nested progress lands in Sub-phase 3.5; this commit
    // wires the kind + label).
    let registry_state = app.try_state::<crate::job_registry::JobRegistry>();
    let disc_shape = is_disc_shape_system(&systemId);
    let (label, kind, unit) = if disc_shape {
        (
            format!("Identifying {} discs", systemId),
            crate::job_registry::JobKind::DiscTrackHash {
                system_id: systemId.clone(),
            },
            "discs",
        )
    } else {
        (
            format!("Identifying {} ROMs", systemId),
            crate::job_registry::JobKind::HashResolve {
                system_id: systemId.clone(),
            },
            "games",
        )
    };
    let scope = crate::job_registry::JobScope::start(
        registry_state.as_deref(),
        kind,
        label,
        Some(systemId.clone()),
        None,
        false,
        unit,
        None, // total set after list_games_missing_hash returns
    );
    let scope_id = scope.id();

    // Auto-sync if our local rom_hashes table is empty for this system.
    // Without this, "Identify ROMs" against an unsynced system returns
    // N unknown / 0 matched with no obvious cause; auto-syncing turns
    // the typical one-click flow into "fetch then resolve" transparently.
    // Phase 4c — pass the HashResolve job id as parent so the
    // auto-sync DatSync row in the bar links to its parent (visible
    // child of "Identifying {system} ROMs" rather than an
    // unrelated row).
    auto_sync_rom_hashes_if_empty(
        &systemId,
        &app,
        &state.app_data_dir,
        &db,
        scope_id,
    )
    .await?;

    let canonical_entries = db.count_rom_hashes(&systemId)?;
    let library_total = db.count_games_for_system(&systemId)?;
    let already_identified = db.count_games_with_hash_for_system(&systemId)?;
    // Phase A1 Sub-phase 3 fix — disc-shape systems pass true so the
    // per-track path retries games the old peek_disc_id flow stamped
    // with disc_id (which previously excluded them from re-resolution
    // forever). Cart-shape systems keep the exclusion: a sha1 OR
    // disc_id stamp is a terminal state for them.
    let games = db.list_games_missing_hash(&systemId, disc_shape)?;
    let total = games.len();
    let mut summary = RomResolveSummary {
        system_id: systemId.clone(),
        scanned: 0,
        matched: 0,
        unmatched: 0,
        skipped_cd: 0,
        errors: 0,
        canonical_entries,
        library_total,
        already_identified,
    };

    // Log the re-run no-op case explicitly. Without this, the operator
    // sees "0/0 scanned, 0 matched" in the UI and assumes Identify is
    // broken when in fact every game has been identified already and a
    // re-run has nothing to do. The matching frontend status line
    // also shows "N already identified" via the new summary fields.
    if total == 0 && library_total > 0 && already_identified == library_total {
        log::info!(
            "rom_hashes: resolve {systemId} — all {library_total} game(s) already identified; nothing to do",
        );
    } else if total == 0 && library_total == 0 {
        log::info!(
            "rom_hashes: resolve {systemId} — no games in library for this system",
        );
    }

    // Short-circuit: nothing to match against. Skip the per-game hash
    // pass entirely — better to stamp 0 sha1s than to burn cycles on a
    // database the user can't possibly hit.
    if canonical_entries == 0 {
        log::info!(
            "rom_hashes: resolve {systemId} — no canonical entries available, returning early",
        );
        let _ = app.emit("oa://rom-hash-resolve-complete", &summary);
        scope.complete();
        return Ok(summary);
    }
    scope.set_total(total as i64);

    // Phase A1 pivot — build the filename-fuzzy index for disc-shape
    // systems once at the top. Keyed on `disc_filename_fuzzy_key` of
    // the canonical game_name; lookup in the per-game loop is O(1).
    // For systems with no rom_hashes_tracks rows (or cart-shape
    // systems), the index is empty and the fuzzy try block below
    // short-circuits cleanly.
    let disc_fuzzy_index: HashMap<String, crate::library_db::RomTrackRow> = if disc_shape {
        match build_disc_fuzzy_index(&db, &systemId) {
            Ok(idx) => {
                log::info!(
                    "rom_hashes: resolve {systemId} fuzzy index built ({} canonical titles)",
                    idx.len()
                );
                idx
            }
            Err(e) => {
                log::warn!(
                    "rom_hashes: resolve {systemId} fuzzy index build failed: {e}"
                );
                HashMap::new()
            }
        }
    } else {
        HashMap::new()
    };
    // Tiny in-process cache so re-hashing identical files in this batch
    // (e.g. user has the same ROM in two folders) doesn't pay for it
    // twice. Keyed on `(file_path, archive_inner)`; value is the full
    // candidate set so a re-hit doesn't pay for header-stripping either.
    //
    // Pre-populated in parallel — the cartridge-game read+hash work is
    // the dominant cost of resolve_rom_hashes_for_system, and was the
    // single biggest serial bottleneck in OA's library scan. Rayon
    // par_iter over the unique cart-game keys saturates all cores;
    // failed reads aren't inserted (the for-loop below will retry them
    // and surface the error through its existing progress-emission
    // path). CD games skip the cache entirely (they go through
    // peek_disc_id, not rom_bytes_for + candidate_sha1s).
    let mut hash_cache: HashMap<(String, Option<String>), Vec<Sha1Candidate>> = HashMap::new();
    {
        // Collect deduped cart keys. Skipping any game whose extension
        // is a CD container keeps the parallel pass focused on the
        // path it actually accelerates.
        let mut seen: std::collections::HashSet<(String, Option<String>)> =
            std::collections::HashSet::new();
        let mut cart_keys: Vec<(String, Option<String>)> = Vec::new();
        for g in &games {
            let ext = std::path::Path::new(g.archive_inner_path.as_deref().unwrap_or(&g.file_path))
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_default();
            if is_cd_container_ext(&ext, &systemId) {
                continue;
            }
            let key = (g.file_path.clone(), g.archive_inner_path.clone());
            if seen.insert(key.clone()) {
                cart_keys.push(key);
            }
        }
        if !cart_keys.is_empty() {
            let n = cart_keys.len();
            let started = std::time::Instant::now();
            let system_for_hash = systemId.clone();
            // Run on Tokio's blocking pool so the async runtime isn't
            // blocked for the duration. rayon::par_iter inside the
            // closure spreads the work across a bounded CPU pool.
            //
            // Pool size is capped at min(4, num_cpus) to bound transient
            // memory. Each worker may transiently hold up to ~2× the ROM
            // bytes during header-rule candidate generation (N64 ByteSwap
            // rule allocates a swapped copy of the 64 MB ROM before
            // hashing). On a 16-core machine, unbounded rayon would
            // spawn 16 workers × ~128 MB each = ~2 GB transient — fine
            // on a desktop with 32+ GB RAM, painful on a Steam Deck or
            // laptop. The 4-worker cap keeps peak transient at ~512 MB
            // for N64 and far less for everything else, at minimal cost
            // to wall-clock time (the hash work is mostly memcpy + sha1
            // which doesn't scale past memory bandwidth on most CPUs).
            let cpu_count = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4);
            let pool_size = cpu_count.min(4).max(1);
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(pool_size)
                .thread_name(|i| format!("oa-hash-{i}"))
                .build()
                .map_err(|e| format!("rayon pool build: {e}"))?;
            let results: Vec<((String, Option<String>), Vec<Sha1Candidate>)> =
                tokio::task::spawn_blocking(move || {
                    pool.install(|| {
                        cart_keys
                            .into_par_iter()
                            .filter_map(|key| {
                                let (file_path, inner) = &key;
                                match rom_bytes_for(file_path, inner.as_deref()) {
                                    Ok(bytes) => {
                                        let candidates = candidate_sha1s(&bytes, &system_for_hash);
                                        Some((key, candidates))
                                    }
                                    // Skip failures here — the for-loop's
                                    // cache-miss path will retry and surface
                                    // the error through its existing
                                    // progress-emission flow.
                                    Err(_) => None,
                                }
                            })
                            .collect()
                    })
                })
                .await
                .map_err(|e| format!("parallel hash join: {e}"))?;
            for (k, v) in results {
                hash_cache.insert(k, v);
            }
            log::info!(
                "rom_hashes: parallel pre-hash {} cart key(s) for {} in {:?} ({} cached)",
                n, systemId, started.elapsed(), hash_cache.len(),
            );
        }
    }

    let mut done = 0usize;
    for g in games {
        done += 1;
        // Phase 4a hash_resolve — per-game tick. 1 Hz SQLite debounce
        // + 10 Hz Tauri event cap inside `progress` keeps the tight
        // loop's overhead nil.
        scope.tick(done as i64);
        let ext = std::path::Path::new(g.archive_inner_path.as_deref().unwrap_or(&g.file_path))
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        if is_cd_container_ext(&ext, &systemId) {
            // Phase A1 pivot 2026-06-03 — filename-fuzzy identification.
            // New primary disc-shape identification path after operator
            // playtest showed per-track SHA-1 vs redump structurally
            // fails on CHD (chdman extract bytes differ from
            // DiscImageCreator source) AND skips archived discs (most
            // libraries). Filename match works on any container with
            // any storage shape — it just compares names.
            //
            // Normalize the operator's filename (file_name of inner
            // archive entry if archived, else the outer file_path) via
            // `disc_filename_fuzzy_key` and look up in the index built
            // at the top of resolve. On hit: stamp canonical title +
            // serial + the canonical's first-track sha1 as the
            // identification marker.
            //
            // Empty index → noop, falls through to per-track (if
            // experimental ON) or peek_disc_id (existing path).
            if !disc_fuzzy_index.is_empty() {
                let raw_name = g
                    .archive_inner_path
                    .as_deref()
                    .unwrap_or(&g.file_path);
                let stem = std::path::Path::new(raw_name)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(raw_name);
                let key = disc_filename_fuzzy_key(stem);
                if let Some(candidate) = disc_fuzzy_index.get(&key) {
                    summary.scanned += 1;
                    summary.matched += 1;
                    if let Err(e) = db.apply_rom_hash(
                        &g.id,
                        &candidate.sha1,
                        Some(&candidate.game_name),
                        candidate.serial.as_deref(),
                    ) {
                        log::warn!(
                            "rom_hashes: apply_rom_hash (fuzzy) {} failed: {e}",
                            g.id
                        );
                        summary.errors += 1;
                    } else {
                        // Phase A1 Sub-phase 4 — stamp disc-set membership
                        // when the canonical title carries a `(Disc N)`
                        // suffix that matches a disc_sets row.
                        match maybe_stamp_disc_set_membership(
                            &db,
                            &g.id,
                            &systemId,
                            &candidate.game_name,
                        ) {
                            Ok(Some((set_id, disc_n))) => log::info!(
                                "rom_hashes: fuzzy match {} → '{}' (disc-set #{set_id} disc {disc_n})",
                                g.id, candidate.game_name
                            ),
                            Ok(None) => log::info!(
                                "rom_hashes: fuzzy match {} → '{}'",
                                g.id, candidate.game_name
                            ),
                            Err(e) => log::warn!(
                                "rom_hashes: disc-set stamp {} failed: {e}",
                                g.id
                            ),
                        }
                    }
                    let _ = app.emit(
                        "oa://rom-hash-resolve-progress",
                        &RomResolveProgress {
                            system_id: systemId.clone(),
                            done,
                            total,
                            current_title: g.title.clone(),
                            last_action: format!(
                                "matched (filename) → {}",
                                candidate.game_name
                            ),
                        },
                    );
                    continue;
                }
            }

            // Phase A1 Sub-phase 3 — per-track SHA-1 identification.
            // Gated behind the experimental flag since the 2026-06-03
            // pivot. Works correctly for raw split-bin libraries
            // (operator opts in via Settings → Experimental →
            // Per-track SHA-1 disc identification); falls back to
            // peek_disc_id below otherwise.
            //
            // Skips when: not disc-shape, experimental flag OFF,
            // archived disc (multi-GB stream through archive layer
            // not implemented), or rom_hashes_tracks empty.
            let try_per_track = disc_shape
                && disc_track_experimental
                && g.archive_inner_path.is_none()
                && matches!(db.count_rom_hashes_tracks(&systemId), Ok(n) if n > 0);
            if try_per_track {
                match try_identify_disc_via_track_hashes(
                    &g.id,
                    &g.file_path,
                    &systemId,
                    disc_strictness,
                    &db,
                )
                .await
                {
                    Ok(Some(canonical)) => {
                        summary.scanned += 1;
                        summary.matched += 1;
                        if let Err(e) = db.apply_rom_hash(
                            &g.id,
                            &canonical.sha1,
                            Some(&canonical.game_name),
                            canonical.serial.as_deref(),
                        ) {
                            log::warn!(
                                "rom_hashes: apply_rom_hash (per-track) {} failed: {e}",
                                g.id
                            );
                            summary.errors += 1;
                        } else {
                            // Phase A1 Sub-phase 4 — stamp disc-set
                            // membership on multi-disc canonical titles.
                            if let Err(e) = maybe_stamp_disc_set_membership(
                                &db,
                                &g.id,
                                &systemId,
                                &canonical.game_name,
                            ) {
                                log::warn!(
                                    "rom_hashes: disc-set stamp {} failed: {e}",
                                    g.id
                                );
                            }
                        }
                        let _ = app.emit(
                            "oa://rom-hash-resolve-progress",
                            &RomResolveProgress {
                                system_id: systemId.clone(),
                                done,
                                total,
                                current_title: g.title.clone(),
                                last_action: format!(
                                    "matched (per-track) → {}",
                                    canonical.game_name
                                ),
                            },
                        );
                        continue;
                    }
                    Ok(None) => {
                        // Fall through to serial-lookup path below.
                    }
                    Err(e) => {
                        // Soft-fail — log and fall through. A hash
                        // error shouldn't block the rest of the resolve
                        // pass.
                        log::warn!(
                            "rom_hashes: per-track identify {} failed: {e}",
                            g.id
                        );
                    }
                }
            }

            // CD path: peek the disc-id from the data track, look it up
            // in `game_serials`, stamp `games.disc_id` so re-scans skip
            // the peek. Archived CDs use `peek_disc_id_archived` which
            // pulls just the cue + first ~64 KB of the data-track .bin
            // through the existing archive reader — no full extract.
            let peek_result = if let Some(inner) = g.archive_inner_path.as_deref() {
                let (archive_path, _) = archive::decode_file_path(&g.file_path);
                crate::cd_id::peek_disc_id_archived(&archive_path, inner, &systemId)
            } else {
                crate::cd_id::peek_disc_id(std::path::Path::new(&g.file_path), &systemId)
            };
            match peek_result {
                Ok(Some(disc)) => {
                    summary.scanned += 1;
                    match db.lookup_game_serial(&systemId, &disc.game_id)? {
                        Some(row) => {
                            summary.matched += 1;
                            if let Err(e) = db.apply_disc_id(
                                &g.id,
                                &disc.game_id,
                                Some(&row.canonical_title),
                            ) {
                                log::warn!("rom_hashes: apply_disc_id {} failed: {e}", g.id);
                                summary.errors += 1;
                            }
                            let _ = app.emit("oa://rom-hash-resolve-progress", &RomResolveProgress {
                                system_id: systemId.clone(),
                                done,
                                total,
                                current_title: g.title.clone(),
                                last_action: format!("matched (disc-id) → {}", row.canonical_title),
                            });
                        }
                        None => {
                            // Stamp the disc-id on the row anyway so
                            // re-scans don't re-peek; title stays as-is.
                            if let Err(e) = db.apply_disc_id(&g.id, &disc.game_id, None) {
                                log::warn!("rom_hashes: apply_disc_id (no-match) {} failed: {e}", g.id);
                                summary.errors += 1;
                            }
                            summary.unmatched += 1;
                            let _ = app.emit("oa://rom-hash-resolve-progress", &RomResolveProgress {
                                system_id: systemId.clone(),
                                done,
                                total,
                                current_title: g.title.clone(),
                                last_action: format!("disc-id {} — no match", disc.game_id),
                            });
                        }
                    }
                }
                Ok(None) => {
                    // No signature recognised in the data track. Try
                    // file-SHA-1 fallback for .iso disc images on the
                    // disc systems whose redump dat carries per-file
                    // hashes (PS2 / GameCube / Dreamcast). .chd dumps
                    // compress the underlying content so the file SHA
                    // doesn't match redump anyway — those stay
                    // disc-id-only.
                    if ext == "iso"
                        && g.archive_inner_path.is_none()
                        && matches!(systemId.as_str(), "ps2" | "gamecube" | "dreamcast")
                    {
                        // PS2/GameCube/Dreamcast .iso dumps are 1.4–8 GB.
                        // Streaming SHA-1 reads + hashes the entire file
                        // sequentially — multi-second to multi-minute
                        // work that would block this async task's runtime
                        // worker for the whole duration. spawn_blocking
                        // moves it to Tokio's blocking pool so the rest
                        // of the async runtime stays responsive.
                        let path_for_task = g.file_path.clone();
                        let sha_result = tokio::task::spawn_blocking(move || {
                            stream_sha1_of_file(std::path::Path::new(&path_for_task))
                        })
                        .await
                        .map_err(|e| format!("stream_sha1 join: {e}"))?;
                        match sha_result {
                            Ok(sha) => {
                                match db.lookup_rom_hash(&sha)? {
                                    Some(row) if row.system_id == systemId => {
                                        summary.matched += 1;
                                        // apply_rom_hash signature is
                                        // (id, sha1, canonical_title, serial).
                                        // Pre-fix this call swapped the two
                                        // tail args — stored serial as title
                                        // and title as serial, corrupting
                                        // both columns on every PS2/GC/DC
                                        // .iso match via this fallback. Cart
                                        // loop below (line ~1431) was always
                                        // correct.
                                        if let Err(e) = db.apply_rom_hash(
                                            &g.id,
                                            &sha,
                                            Some(&row.game_name),
                                            row.serial.as_deref(),
                                        ) {
                                            log::warn!("rom_hashes: apply_rom_hash {} failed: {e}", g.id);
                                            summary.errors += 1;
                                        }
                                        let _ = app.emit("oa://rom-hash-resolve-progress", &RomResolveProgress {
                                            system_id: systemId.clone(),
                                            done,
                                            total,
                                            current_title: g.title.clone(),
                                            last_action: format!("matched (file-sha1) → {}", row.game_name),
                                        });
                                        continue;
                                    }
                                    _ => {
                                        summary.unmatched += 1;
                                        let _ = app.emit("oa://rom-hash-resolve-progress", &RomResolveProgress {
                                            system_id: systemId.clone(),
                                            done,
                                            total,
                                            current_title: g.title.clone(),
                                            last_action: format!("file-sha1 {sha} — no match"),
                                        });
                                        continue;
                                    }
                                }
                            }
                            Err(e) => {
                                log::warn!("rom_hashes: stream_sha1_of_file {} failed: {e}", g.file_path);
                                // Fall through to skipped — the file
                                // read itself failed, not a catalog miss.
                            }
                        }
                    }
                    summary.skipped_cd += 1;
                    let _ = app.emit("oa://rom-hash-resolve-progress", &RomResolveProgress {
                        system_id: systemId.clone(),
                        done,
                        total,
                        current_title: g.title.clone(),
                        last_action: format!("no disc-id signature in .{ext}"),
                    });
                }
                Err(e) => {
                    // Distinct from `Ok(None)` (no signature found) —
                    // this is a hard read failure: corrupt CHD, missing
                    // .bin sidecar for a .cue, permission error, etc.
                    // Pre-fix all of these silently fell into
                    // `skipped_cd` alongside the legitimate
                    // no-signature case, masking real setup mistakes
                    // the operator could fix (e.g., a half-downloaded
                    // multi-disc set with one .bin missing). Now they
                    // count as `errors` and the verbose message in the
                    // progress emit + log surfaces what went wrong.
                    log::warn!(
                        "rom_hashes: peek_disc_id {} failed: {e}",
                        g.file_path
                    );
                    summary.errors += 1;
                    let _ = app.emit("oa://rom-hash-resolve-progress", &RomResolveProgress {
                        system_id: systemId.clone(),
                        done,
                        total,
                        current_title: g.title.clone(),
                        last_action: format!("disc-id read failed: {e}"),
                    });
                }
            }
            continue;
        }

        let key = (g.file_path.clone(), g.archive_inner_path.clone());
        let candidates = match hash_cache.get(&key) {
            Some(c) => c.clone(),
            None => {
                let bytes = match rom_bytes_for(&g.file_path, g.archive_inner_path.as_deref()) {
                    Ok(b) => b,
                    Err(e) => {
                        log::warn!("rom_hashes: read {} failed: {e}", g.file_path);
                        summary.errors += 1;
                        let _ = app.emit("oa://rom-hash-resolve-progress", &RomResolveProgress {
                            system_id: systemId.clone(),
                            done,
                            total,
                            current_title: g.title.clone(),
                            last_action: format!("read error: {e}"),
                        });
                        continue;
                    }
                };
                let candidates = candidate_sha1s(&bytes, &systemId);
                hash_cache.insert(key, candidates.clone());
                candidates
            }
        };

        // The Raw candidate is guaranteed to be present and first — use
        // it as the persisted sha1 on miss so re-scans skip rehashing.
        // `candidate_sha1s` panics-by-design if Raw is dropped (would
        // mean a bug in this module).
        let raw_sha1 = candidates
            .iter()
            .find(|c| c.rule == HeaderRule::Raw)
            .expect("candidate_sha1s always yields Raw first")
            .sha1
            .clone();

        summary.scanned += 1;
        // Try every candidate against the DB; first hit wins. Logging the
        // rule on hit is genuinely diagnostic — it tells the operator
        // whether their library is headered or not.
        let mut matched: Option<(RomHashRow, HeaderRule, String)> = None;
        for c in &candidates {
            if let Some(row) = db.lookup_rom_hash(&c.sha1)? {
                matched = Some((row, c.rule, c.sha1.clone()));
                break;
            }
        }

        match matched {
            Some((row, rule, sha1)) => {
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
                let last_action = match rule {
                    HeaderRule::Raw => format!("matched → {}", row.game_name),
                    _ => format!("matched (header-stripped) → {}", row.game_name),
                };
                let _ = app.emit("oa://rom-hash-resolve-progress", &RomResolveProgress {
                    system_id: systemId.clone(),
                    done,
                    total,
                    current_title: g.title.clone(),
                    last_action,
                });
            }
            None => {
                // Stamp the Raw sha1 so re-runs don't re-hash this file.
                // Title stays as-is.
                if let Err(e) = db.apply_rom_hash(&g.id, &raw_sha1, None, None) {
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
    // Phase 4a hash_resolve — finalize via the RAII guard. Earlier
    // `?` propagations would auto-fail via Drop instead of leaving
    // the row stuck in `running`.
    scope.complete();
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

/// Phase A1 Sub-phase 4 — stamp `games.disc_set_id` + `games.disc_number`
/// when an identification result has a canonical title carrying a
/// `(Disc N)` suffix that matches an existing row in `disc_sets`.
/// Called from both the fuzzy-match Ok path AND the per-track-match
/// Ok(Some) path; no-op when the canonical title isn't multi-disc OR
/// no disc_sets row exists for the base title (covers single-disc
/// games and systems whose redump dat hasn't synced yet).
fn maybe_stamp_disc_set_membership(
    db: &LibraryDb,
    game_id: &str,
    system_id: &str,
    canonical_title: &str,
) -> Result<Option<(i64, u32)>, String> {
    let Some((base_title, disc_n)) = extract_disc_set_candidate(canonical_title) else {
        return Ok(None);
    };
    let Some(disc_set_id) = db.lookup_disc_set_id(system_id, &base_title)? else {
        return Ok(None);
    };
    db.apply_disc_set_membership(game_id, disc_set_id, disc_n)?;
    Ok(Some((disc_set_id, disc_n)))
}

/// Phase A1 pivot 2026-06-03 — normalize a filename for fuzzy
/// matching against canonical disc titles. Strips the extension,
/// lowercases, converts separator punctuation (`_-:,+/\\`) to spaces,
/// collapses whitespace. **Preserves bracketed region tags** so
/// "Tomb Raider (USA)" and "Tomb Raider (Japan)" don't collide on
/// the same normalized key.
///
/// This is intentionally simpler than [`crate::normalize::normalize_title`]
/// (which aggressively strips region/version tags + tag words +
/// articles): for disc identification we want regional precision,
/// not the broad coalescence that media-sync needs for fuzzy artwork
/// match. Operators with consistent redump-style filenames
/// (`Game (USA).cue` / `Game (USA).chd` / `Game (USA).zip`) will hit
/// canonical entries exactly via this key.
fn disc_filename_fuzzy_key(s: &str) -> String {
    let s = match s.rsplit_once('.') {
        Some((stem, ext)) if ext.len() <= 5 => stem,
        _ => s,
    };
    let s = s.to_lowercase();
    let s: String = s
        .chars()
        .map(|c| match c {
            '_' | '-' | '.' | ':' | ',' | '+' | '/' | '\\' => ' ',
            other => other,
        })
        .collect();
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Build a fuzzy-key → canonical-row index for a disc-shape system.
/// Called once per resolve call (not per-game) so the operator pays
/// O(N) up front and O(1) per game during the loop. Empty when the
/// system has no `rom_hashes_tracks` rows.
fn build_disc_fuzzy_index(
    db: &LibraryDb,
    system_id: &str,
) -> Result<HashMap<String, crate::library_db::RomTrackRow>, String> {
    let canonical = db.list_canonical_disc_titles(system_id)?;
    let mut by_key: HashMap<String, crate::library_db::RomTrackRow> =
        HashMap::with_capacity(canonical.len());
    for row in canonical {
        let key = disc_filename_fuzzy_key(&row.game_name);
        // First-write-wins on collision. The regional bracket suffix
        // disambiguates most pairs; surviving collisions are
        // alphabetical-first via SQL order.
        by_key.entry(key).or_insert(row);
    }
    Ok(by_key)
}

/// Phase A1 Sub-phase 3 — per-track SHA-1 identification for one
/// disc-shape game. Called from the resolve flow's CD-container
/// branch when the system is disc-shape and rom_hashes_tracks is
/// populated.
///
/// Returns the canonical [`RomTrackRow`] match on success, `None`
/// when no candidate was found or the strictness check failed. The
/// caller stamps `apply_rom_hash` with the returned values on
/// `Ok(Some(_))` and falls through to the existing serial-lookup
/// path on `Ok(None)`.
///
/// Cache behaviour: stats the disc file; if its mtime+size match the
/// last cached `game_disc_tracks` row, the cached track set is used
/// directly (no re-hash). Drift triggers a fresh hash + cache
/// refresh. First identification (no cache row) does the same fresh
/// hash + cache write.
///
/// Hashing runs on Tokio's blocking pool — disc hashes range from
/// seconds (PSX HuCard-shape) to multiple minutes (PS2/GameCube
/// multi-GB ISOs); blocking the async runtime is unacceptable.
async fn try_identify_disc_via_track_hashes(
    game_id: &str,
    file_path: &str,
    system_id: &str,
    strictness: crate::disc_track_hash::Strictness,
    db: &LibraryDb,
) -> Result<Option<crate::library_db::RomTrackRow>, String> {
    let path = std::path::PathBuf::from(file_path);
    let metadata = std::fs::metadata(&path)
        .map_err(|e| format!("stat {}: {e}", path.display()))?;
    let file_size = metadata.len() as i64;
    let file_mtime = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    // Cache lookup. Stamps match → use cached tracks; otherwise hash
    // fresh and refresh the cache.
    let cached = db.get_game_disc_tracks(game_id)?;
    let tracks = match cached {
        Some(c) if c.file_mtime == file_mtime && c.file_size == file_size => {
            log::debug!(
                "disc_track_hash: cache hit for {game_id} ({} tracks)",
                c.tracks.len()
            );
            c.tracks
        }
        _ => {
            let path_for_blocking = path.clone();
            let game_id_for_log = game_id.to_string();
            let fresh = tokio::task::spawn_blocking(move || {
                crate::disc_track_hash::hash_disc(&path_for_blocking, None)
            })
            .await
            .map_err(|e| format!("hash_disc join {game_id_for_log}: {e}"))?
            .map_err(|e| format!("hash_disc {game_id_for_log}: {e}"))?;
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            if let Err(e) = db.write_game_disc_tracks(
                game_id, &fresh, file_mtime, file_size, now,
            ) {
                log::warn!(
                    "disc_track_hash: write cache for {game_id} failed: {e}"
                );
            }
            fresh
        }
    };

    // Candidate selection: look up the first data track's SHA-1 in
    // the canonical rom_hashes_tracks. All-audio discs (CD-DA) have
    // no data track and are not identifiable via this path.
    let Some(first_data) = tracks.iter().find(|t| !t.is_audio()) else {
        log::debug!(
            "disc_track_hash: {game_id} all-audio disc, skip per-track match"
        );
        return Ok(None);
    };
    let candidate = match db.lookup_rom_hash_track(&first_data.sha1)? {
        Some(c) => c,
        None => return Ok(None),
    };

    // Verify candidate's full canonical track set against the
    // operator's per the chosen strictness.
    let canonical_tracks =
        db.lookup_rom_hashes_tracks_for_game(system_id, &candidate.game_name)?;
    let result = crate::disc_track_hash::evaluate_match(
        &tracks,
        &canonical_tracks,
        strictness,
    );
    if result.passes_strictness {
        log::info!(
            "disc_track_hash: {game_id} matched '{}' ({}/{} data tracks)",
            candidate.game_name,
            result.matched_tracks,
            result.total_data_tracks
        );
        Ok(Some(candidate))
    } else {
        log::debug!(
            "disc_track_hash: {game_id} partial match '{}' ({}/{} under {:?})",
            candidate.game_name,
            result.matched_tracks,
            result.total_data_tracks,
            strictness
        );
        Ok(None)
    }
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
        let ParsedDat { rom_hashes: entries, .. } = parse_libretro_dat(SAMPLE_DAT, "tg16");
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
    fn parser_emits_game_serials_for_blocks_with_serials() {
        let ParsedDat { game_serials, .. } = parse_libretro_dat(SAMPLE_DAT, "tg16");
        // Sample has 2 game blocks; only Bonk has a serial.
        assert_eq!(game_serials.len(), 1);
        let bonk = &game_serials[0];
        assert_eq!(bonk.system_id, "tg16");
        assert_eq!(bonk.serial, "TGX040080");
        assert_eq!(bonk.canonical_title, "Bonk's Adventure (USA)");
        assert!(bonk.region.is_none());
    }

    #[test]
    fn parser_dedupes_game_serials_by_serial() {
        // Two game blocks sharing the same serial → one game_serials row.
        // Mirrors libretro-database's habit of cataloging multiple
        // regional/revision dumps under one publisher catalog code.
        let dat = r#"
game (
	name "Foo (USA)"
	rom ( name "Foo.pce" size 1024 sha1 1111111111111111111111111111111111111111 )
	serial "TGX040000"
)

game (
	name "Foo (Japan)"
	rom ( name "Foo.pce" size 1024 sha1 2222222222222222222222222222222222222222 )
	serial "TGX040000"
)
"#;
        let ParsedDat { rom_hashes, game_serials, .. } = parse_libretro_dat(dat, "tg16");
        assert_eq!(rom_hashes.len(), 2, "both rom rows survive");
        assert_eq!(game_serials.len(), 1, "duplicate serials dedupe");
    }

    #[test]
    fn disc_filename_fuzzy_key_normalizes_redump_filenames() {
        // Identical canonical + operator names produce identical keys.
        assert_eq!(
            disc_filename_fuzzy_key("Tomb Raider (USA).cue"),
            disc_filename_fuzzy_key("Tomb Raider (USA)")
        );
        // Different container shapes for the same game collapse to
        // the same key — covers .cue / .chd / .gdi / .iso / .zip.
        let canonical_key = disc_filename_fuzzy_key("Tomb Raider (USA)");
        assert_eq!(canonical_key, disc_filename_fuzzy_key("Tomb Raider (USA).cue"));
        assert_eq!(canonical_key, disc_filename_fuzzy_key("Tomb Raider (USA).chd"));
        assert_eq!(canonical_key, disc_filename_fuzzy_key("Tomb Raider (USA).gdi"));
        assert_eq!(canonical_key, disc_filename_fuzzy_key("Tomb Raider (USA).iso"));
        assert_eq!(canonical_key, disc_filename_fuzzy_key("Tomb Raider (USA).zip"));
        // Underscores + dashes + dots get collapsed to whitespace.
        assert_eq!(canonical_key, disc_filename_fuzzy_key("Tomb_Raider_(USA).cue"));
        assert_eq!(canonical_key, disc_filename_fuzzy_key("Tomb-Raider-(USA).cue"));
        assert_eq!(canonical_key, disc_filename_fuzzy_key("Tomb.Raider.(USA).cue"));
        // Mixed separators + case variations collapse.
        assert_eq!(
            canonical_key,
            disc_filename_fuzzy_key("TOMB-Raider_(usa).Cue")
        );
    }

    #[test]
    fn disc_filename_fuzzy_key_preserves_regional_brackets() {
        // Critical: regional variants must NOT collide on the same
        // key. "Tomb Raider (USA)" and "Tomb Raider (Japan)" stamp
        // different canonical titles + serials.
        let usa = disc_filename_fuzzy_key("Tomb Raider (USA).cue");
        let japan = disc_filename_fuzzy_key("Tomb Raider (Japan).cue");
        let europe = disc_filename_fuzzy_key("Tomb Raider (Europe).cue");
        assert_ne!(usa, japan);
        assert_ne!(usa, europe);
        assert_ne!(japan, europe);
        // Multi-disc variants also stay distinct.
        let disc1 = disc_filename_fuzzy_key("Final Fantasy IX (USA) (Disc 1).cue");
        let disc2 = disc_filename_fuzzy_key("Final Fantasy IX (USA) (Disc 2).cue");
        assert_ne!(disc1, disc2);
    }

    #[test]
    fn disc_filename_fuzzy_key_handles_no_extension_and_short_word_suffixes() {
        // No extension at all — leave the whole string in.
        assert_eq!(
            disc_filename_fuzzy_key("Tomb Raider (USA)"),
            "tomb raider (usa)"
        );
        // Suffix longer than 5 chars isn't treated as an extension —
        // catches "Final Fantasy VII Bootleg" not stripping "Bootleg".
        let key = disc_filename_fuzzy_key("Foo Bar Baz.SomeLongSuffix");
        assert!(key.contains("somelongsuffix"), "got: {key}");
    }

    #[test]
    fn auto_sync_needed_cart_shape_fires_only_when_table_empty() {
        // Cart-shape systems gate on rom_hashes alone.
        assert!(auto_sync_needed(false, 0, 0));
        assert!(auto_sync_needed(false, 0, 999));
        assert!(!auto_sync_needed(false, 1, 0));
        assert!(!auto_sync_needed(false, 1234, 0));
    }

    #[test]
    fn auto_sync_needed_disc_shape_fires_when_either_table_empty() {
        // Disc-shape systems require BOTH tables. Critical regression
        // guard: pre-Sub-phase 1 caches populated rom_hashes from the
        // redump dat but left rom_hashes_tracks empty; pre-fix the
        // gate keyed on rom_hashes alone and the per-track identify
        // path stayed dormant forever.
        assert!(auto_sync_needed(true, 0, 0));
        assert!(
            auto_sync_needed(true, 1234, 0),
            "rom_hashes populated but rom_hashes_tracks empty MUST fire sync"
        );
        assert!(
            auto_sync_needed(true, 0, 1234),
            "rom_hashes empty MUST fire sync regardless of track count"
        );
        assert!(!auto_sync_needed(true, 1234, 1234));
    }

    #[test]
    fn parser_emits_per_track_rows_for_redump_style_dat() {
        // Redump dats list one `rom (...)` entry per track of a disc-
        // shape game. Per-track rows derive track_number from the
        // `(Track NN)` filename suffix, track_mode heuristically from
        // position (DATA for track 01, AUDIO for tracks 2+). The .cue
        // sidecar is skipped from the per-track output (sidecar SHA-1s
        // vary by dump tool and aren't useful for identification).
        let dat = r#"
game (
    name "Tomb Raider (USA)"
    description "Tomb Raider (USA)"
    serial "SLUS-00152"
    rom ( name "Tomb Raider (USA) (Track 01).bin" size 622272 crc AAAAAAAA sha1 1111111111111111111111111111111111111111 )
    rom ( name "Tomb Raider (USA) (Track 02).bin" size 33840960 crc BBBBBBBB sha1 2222222222222222222222222222222222222222 )
    rom ( name "Tomb Raider (USA).cue" size 312 crc CCCCCCCC sha1 3333333333333333333333333333333333333333 )
)
"#;
        let ParsedDat { rom_hashes_tracks, .. } = parse_libretro_dat(dat, "psx");
        assert_eq!(rom_hashes_tracks.len(), 2, "sidecar .cue is skipped");
        let t1 = &rom_hashes_tracks[0];
        assert_eq!(t1.game_name, "Tomb Raider (USA)");
        assert_eq!(t1.serial.as_deref(), Some("SLUS-00152"));
        assert_eq!(t1.track_number, 1);
        assert_eq!(t1.track_mode, "DATA");
        assert_eq!(t1.size_bytes, 622_272);
        let t2 = &rom_hashes_tracks[1];
        assert_eq!(t2.track_number, 2);
        assert_eq!(t2.track_mode, "AUDIO");
        assert_eq!(t2.size_bytes, 33_840_960);
    }

    #[test]
    fn parser_skips_all_sidecar_extensions_from_track_rows() {
        let dat = r#"
game (
    name "Sample Multi-Sidecar"
    rom ( name "Sample (Track 01).bin" size 1024 sha1 1111111111111111111111111111111111111111 )
    rom ( name "Sample.cue" size 100 sha1 2222222222222222222222222222222222222222 )
    rom ( name "Sample.gdi" size 100 sha1 3333333333333333333333333333333333333333 )
    rom ( name "Sample.toc" size 100 sha1 4444444444444444444444444444444444444444 )
    rom ( name "Sample.ccd" size 100 sha1 5555555555555555555555555555555555555555 )
    rom ( name "Sample.sub" size 100 sha1 6666666666666666666666666666666666666666 )
    rom ( name "Sample.sbi" size 100 sha1 7777777777777777777777777777777777777777 )
    rom ( name "Sample.sbx" size 100 sha1 8888888888888888888888888888888888888888 )
)
"#;
        let ParsedDat { rom_hashes_tracks, rom_hashes, .. } =
            parse_libretro_dat(dat, "psx");
        // Cart-shape rom_hashes keeps every entry (8 rows) — disc/cart
        // dispatch happens at sync time, not at parse time.
        assert_eq!(rom_hashes.len(), 8);
        // Disc-shape rom_hashes_tracks keeps only the .bin track.
        assert_eq!(rom_hashes_tracks.len(), 1);
        assert_eq!(rom_hashes_tracks[0].track_mode, "DATA");
    }

    #[test]
    fn parser_track_number_falls_back_to_position_in_block() {
        // Single-track .iso with no `(Track NN)` suffix — track_number
        // defaults to 1 (position in block). Mode is MODE1/2048 for
        // .iso entries.
        let dat = r#"
game (
    name "Quake III Arena (USA)"
    rom ( name "Quake III Arena (USA).iso" size 1024 sha1 1111111111111111111111111111111111111111 )
)
"#;
        let ParsedDat { rom_hashes_tracks, .. } =
            parse_libretro_dat(dat, "dreamcast");
        assert_eq!(rom_hashes_tracks.len(), 1);
        assert_eq!(rom_hashes_tracks[0].track_number, 1);
        assert_eq!(rom_hashes_tracks[0].track_mode, "MODE1/2048");
    }

    #[test]
    fn parser_detects_multi_disc_parent_groupings() {
        // Four discs sharing the base title "Final Fantasy IX (USA)"
        // collapse to one DiscSetCandidate with disc_count=4. The
        // single-disc "Standalone Game" entry produces no candidate.
        let dat = r#"
game (
    name "Final Fantasy IX (USA) (Disc 1)"
    rom ( name "FF9 (Disc 1) (Track 01).bin" size 1024 sha1 1111111111111111111111111111111111111111 )
)
game (
    name "Final Fantasy IX (USA) (Disc 2)"
    rom ( name "FF9 (Disc 2) (Track 01).bin" size 1024 sha1 2222222222222222222222222222222222222222 )
)
game (
    name "Final Fantasy IX (USA) (Disc 3)"
    rom ( name "FF9 (Disc 3) (Track 01).bin" size 1024 sha1 3333333333333333333333333333333333333333 )
)
game (
    name "Final Fantasy IX (USA) (Disc 4)"
    rom ( name "FF9 (Disc 4) (Track 01).bin" size 1024 sha1 4444444444444444444444444444444444444444 )
)
game (
    name "Standalone Game (USA)"
    rom ( name "Standalone (Track 01).bin" size 1024 sha1 5555555555555555555555555555555555555555 )
)
"#;
        let ParsedDat { disc_set_candidates, .. } =
            parse_libretro_dat(dat, "psx");
        assert_eq!(disc_set_candidates.len(), 1);
        let ff9 = &disc_set_candidates[0];
        assert_eq!(ff9.canonical_title, "Final Fantasy IX (USA)");
        assert_eq!(ff9.system_id, "psx");
        assert_eq!(ff9.disc_count, 4);
    }

    #[test]
    fn parser_ignores_clrmamepro_header() {
        let ParsedDat { rom_hashes: entries, .. } = parse_libretro_dat(SAMPLE_DAT, "tg16");
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
        let ParsedDat { rom_hashes: entries, .. } = parse_libretro_dat(dat, "tg16");
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

    /// Every onboarded system must have an explicit decision recorded
    /// in `config/systems/<id>/system.yaml::libretro_dat_refs` —
    /// either one or more `DatRef`s, or an empty list (listed in
    /// `NO_DAT_SYSTEMS` below) with the reason it's skipped.
    /// Forgetting this leaves a new system silently unmatched at
    /// "Identify ROMs" time (info-log only).
    ///
    /// If you're adding a new core and this test fails: find the
    /// system under
    /// https://github.com/libretro/libretro-database/tree/master/metadat
    /// (no-intro for cart-based, redump for CD-based) and add a DatRef
    /// to `config/systems/<id>/system.yaml`. If the system has no
    /// hash-based identification path, add it to `NO_DAT_SYSTEMS`
    /// below with the reason.
    #[test]
    fn every_onboarded_system_has_an_explicit_rom_hashes_decision() {
        // Keep in sync with the canonical onboarded list in
        // `bindings.rs` tests (e.g. `default_pads_round_trip_to_button`).
        const ONBOARDED_SYSTEMS: &[&str] = &[
            "tg16", "pce-cd", "lynx", "nes", "snes", "mame", "atari7800",
            "genesis", "segacd", "sega32x", "saturn", "psx",
            "neogeo", "neocd", "ngp",
            "jaguar", "3do", "pcfx",
            "n64", "gamecube", "dreamcast",
            "psp", "ps2", "nds",
            "sms", "gamegear", "gb", "gbc", "gba", "2600",
            "coleco", "intv", "o2", "channelf",
            "vectrex", "virtualboy", "wonderswan",
            "5200", "pokemini",
            "scummvm", "dosbox",
        ];
        // Systems whose libretro_dat_refs is empty on purpose. Document
        // the reason next to the id.
        const NO_DAT_SYSTEMS: &[&str] = &[
            // Phase A1 Sub-phase 3 update — pce-cd + 3do gained redump
            // dat refs because the per-track SHA-1 path doesn't need
            // serial data (the prior reason these were on this list).
            // They're still serial-lookup-unfriendly but per-track
            // matching covers them now.
            "mame",   // arcade ROM identification is set-based, not single-file.
            "scummvm",// engine launcher, not hardware. Game data files vary by release/translation; cover sync falls back to fuzzy filename match at the 0.95 threshold.
            "dosbox", // DOS-game runner, also no hardware. Directory contents vary by media (floppy/CD/GOG re-release) + fan patches; fuzzy filename match at the 0.95 threshold against directory basename.
        ];
        for sys in ONBOARDED_SYSTEMS {
            let refs = libretro_dat_refs_for_system_resolved(sys);
            if NO_DAT_SYSTEMS.contains(sys) {
                assert!(
                    refs.is_empty(),
                    "{sys} is in NO_DAT_SYSTEMS but config/systems/{sys}/system.yaml::libretro_dat_refs is non-empty — pick one"
                );
            } else {
                assert!(
                    !refs.is_empty(),
                    "{sys} is onboarded but config/systems/{sys}/system.yaml has no libretro_dat_refs. \
                     Add a libretro_dat_refs entry pointing at the right metadat/ subdir + basename, \
                     or add {sys} to NO_DAT_SYSTEMS with the reason."
                );
                // Sanity: subdirs should be one of the known layouts.
                for r in refs {
                    assert!(
                        matches!(
                            r.subdir,
                            "dat" | "metadat/no-intro" | "metadat/redump" | "metadat/headered"
                        ),
                        "{sys}: unexpected subdir {:?} — extend the allowlist if libretro-database added a new layout",
                        r.subdir
                    );
                    assert!(!r.basename.is_empty(), "{sys}: empty basename");
                }
            }
        }
    }

    #[test]
    fn cd_container_extensions_are_filtered() {
        // Universal CD extensions are CD on EVERY system.
        for ext in ["cue", "chd", "ccd", "toc", "m3u"] {
            assert!(
                is_cd_container_ext(ext, "psx"),
                "{ext} should be a CD container on psx",
            );
            assert!(
                is_cd_container_ext(ext, "2600"),
                "{ext} is always a CD container even on cart systems (universal arm)",
            );
        }
        // Cart-system extensions are never CD.
        for ext in ["pce", "nes", "smc", "sfc", "lnx"] {
            assert!(
                !is_cd_container_ext(ext, "psx"),
                "{ext} should NOT be a CD container",
            );
        }
    }

    /// `.bin` and `.iso` only count as CD on disc systems. Pre-fix
    /// `is_cd_container_ext("bin")` returned true universally, causing
    /// 2600/Coleco/Intv/O2 `.bin` cart dumps to be routed to
    /// peek_disc_id (which fails) instead of getting hashed normally.
    #[test]
    fn bin_and_iso_only_cd_on_disc_systems() {
        // Disc systems — .bin and .iso are CD containers.
        for system in ["pce-cd", "segacd", "saturn", "psx", "ps2", "neocd", "pcfx",
                       "gamecube", "dreamcast", "3do", "psp"] {
            assert!(
                is_cd_container_ext("bin", system),
                ".bin should be CD on {system}",
            );
            assert!(
                is_cd_container_ext("iso", system),
                ".iso should be CD on {system}",
            );
        }
        // Cart systems — .bin is the raw cart dump, must hash normally.
        for system in ["2600", "coleco", "intv", "o2", "channelf", "vectrex",
                       "virtualboy", "nes", "snes", "genesis"] {
            assert!(
                !is_cd_container_ext("bin", system),
                ".bin should NOT be a CD container on cart system {system}",
            );
            assert!(
                !is_cd_container_ext("iso", system),
                ".iso should NOT be a CD container on cart system {system}",
            );
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
