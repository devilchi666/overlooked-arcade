//! Per-track SHA-1 hashing for disc-shape ROMs.
//!
//! Phase A1 Sub-phase 2 of the virtual library + launcher arc. See
//! `docs/PLANS/disc-track-sha1-matching.md` for the locked plan; the
//! 2026-06-03 research pass locked the hash convention (raw bytes,
//! byte-for-byte — no preprocessing across MODE1/2352, MODE2/2352, or
//! audio) and the CHD walk strategy (manual CHT2 parse + 4-frame
//! CHDMAN padding accounting + 96-byte subchannel strip per CD frame).
//!
//! ## What this produces
//!
//! Given a disc image path, [`hash_disc`] returns one [`TrackHash`] per
//! non-sidecar track of the disc, ready to match against the
//! `rom_hashes_tracks` SQLite table populated by Sub-phase 1. The
//! caller (Sub-phase 3's identify flow) compares the computed SHA-1
//! set against the redump-synced canonical set via
//! [`evaluate_match`].
//!
//! ## Container coverage
//!
//! - `.iso` — single track, full file is the data. Trivial.
//! - `.cue + split .bin` (one .bin per track) — stream each .bin
//!   file directly.
//! - `.cue + merged .bin` (single .bin per disc, INDEX 01 positions
//!   carve per-track byte ranges) — slice the file by
//!   `index01_sectors × sector_size` and stream each slice.
//! - `.gdi` — Dreamcast cuesheet (`track_no track_lba sector_type
//!   sector_size filename file_offset`). One file per track; stream
//!   each.
//! - `.chd` — see [`hash_chd`]. Manual CHT2 metadata parse + per-frame
//!   walk through hunks (4-frame `TRACK_PADDING` accounting, 96-byte
//!   subchannel tail stripped). Constant memory.
//!
//! ## Cancellation
//!
//! Every hasher takes an optional `&dyn Fn() -> bool` cancel check.
//! The inner read loops poll it every 1 MiB; on a `true` return the
//! hasher exits with [`HashError::Cancelled`]. The convention lets
//! Sub-phase 3 wire BackgroundJobs cancel buttons without changing
//! these signatures.

// Production surface — every function in this module is a future
// Sub-phase 3 consumer (the identify flow wires hash_disc into
// resolve_rom_hashes_for_system + evaluate_match into the strictness
// dispatch). Suppress dead_code globally for the module so the cargo
// output stays readable until Sub-phase 3 lands.
#![allow(dead_code)]

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use sha1::{Digest, Sha1};

use crate::cd_id;
use crate::library_db::RomTrackRow;

/// Cancel-check interval — every 1 MiB of streamed bytes the hasher
/// polls the cancel closure (if any). 1 MiB matches roughly 1 ms of
/// SHA-1 work at GB/s rates; small enough for responsive cancel,
/// large enough not to slow the hash inner loop measurably.
const CANCEL_CHECK_BYTES: u64 = 1 << 20;

/// CHDMAN pads each CD track to a 4-frame boundary. Padding frames
/// land in the hunk stream but are NOT part of any track's hash.
/// Locked 2026-06-03 research pass — cite MAME `cdrom.h`
/// `TRACK_PADDING = 4`.
const CHD_CD_TRACK_PADDING: u64 = 4;

/// CD frame size as CHDMAN stores it: 2352 user bytes + 96 subchannel.
/// DVD CHDs use `unit_bytes = 2048` and have no track metadata.
const CHD_CD_FRAME_BYTES: usize = 2448;

/// User bytes per CD frame that redump hashes. Strip the 96-byte
/// subchannel tail of each 2448-byte CHD frame before feeding the
/// hasher.
const CD_USER_BYTES_PER_FRAME: usize = 2352;

/// One computed per-track hash + metadata. Lifted from the operator's
/// disc image at scan/identify time. Compare against
/// [`RomTrackRow`] entries fetched from `rom_hashes_tracks` to identify
/// the canonical title.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackHash {
    /// 1-based track number as the container's TOC reports it.
    pub track_number: u32,
    /// Track mode string — "MODE1/2352", "MODE1/2048", "MODE2/2352",
    /// "AUDIO", or the CHD's own form (e.g. "MODE2_FORM1"). Used by
    /// [`evaluate_match`] to skip audio tracks per the identification-
    /// scope decision.
    pub track_mode: String,
    /// Lowercase 40-char SHA-1 hex string of the raw track bytes.
    /// "Raw" means: exactly the bytes redump publishes per-track,
    /// with no preprocessing (no header strip, no ECC discard) per
    /// the 2026-06-03 research pass.
    pub sha1: String,
    /// Total user bytes hashed for this track. Used by Sub-phase 3's
    /// progress reporting + by [`evaluate_match`] as a soft tiebreaker.
    pub size_bytes: u64,
}

impl TrackHash {
    /// True for tracks whose mode string starts with "MODE" or "AUDIO"
    /// — the cases the matcher cares about. Used internally by
    /// [`evaluate_match`].
    pub fn is_audio(&self) -> bool {
        let mode_upper = self.track_mode.to_ascii_uppercase();
        mode_upper.starts_with("AUDIO") || mode_upper == "CDDA"
    }
}

/// Strictness mode for matching the operator's per-track hashes
/// against a canonical disc entry. Plan-locked at three steps.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Strictness {
    /// Every data track must match. The default — silent green
    /// confidence pill when it passes.
    Strict,
    /// At least N% of data tracks must match. Threshold(80) is the
    /// plan's standard mid-tier. Shows a ⚠ partial-match badge when
    /// it passes via this mode rather than Strict.
    Threshold(u8),
    /// At least one data track matches. Shows the same ⚠ partial-
    /// match badge. Useful for operators with mixed-quality dumps.
    Lenient,
}

/// Outcome of running [`evaluate_match`] against a candidate
/// canonical entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchResult {
    pub matched_tracks: u32,
    pub total_data_tracks: u32,
    pub passes_strictness: bool,
}

/// Errors a hasher can return. `Cancelled` is the cancel-token bail
/// path; the others map to standard I/O + parse failures.
#[derive(Debug)]
pub enum HashError {
    Cancelled,
    Io(std::io::Error),
    Parse(String),
}

impl std::fmt::Display for HashError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HashError::Cancelled => write!(f, "hashing cancelled by caller"),
            HashError::Io(e) => write!(f, "i/o: {e}"),
            HashError::Parse(msg) => write!(f, "parse: {msg}"),
        }
    }
}

impl std::error::Error for HashError {}

impl From<std::io::Error> for HashError {
    fn from(e: std::io::Error) -> Self {
        HashError::Io(e)
    }
}

/// Hash every track of a disc image. Dispatches on the file extension:
/// `.iso` / `.cue` / `.chd` / `.gdi`. Other extensions return
/// [`HashError::Parse`].
///
/// Returns per-track entries in container TOC order (track_number
/// 1, 2, 3, …). Audio tracks ARE included in the output (the matcher
/// gates identification on data-track matches but scores audio
/// opportunistically — see [`evaluate_match`]).
pub fn hash_disc(
    path: &Path,
    cancel: Option<&dyn Fn() -> bool>,
) -> Result<Vec<TrackHash>, HashError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .ok_or_else(|| HashError::Parse(format!("no file extension on {}", path.display())))?;
    match ext.as_str() {
        "iso" => hash_iso(path, cancel),
        "cue" => hash_cue(path, cancel),
        "chd" => hash_chd(path, cancel),
        "gdi" => hash_gdi(path, cancel),
        other => Err(HashError::Parse(format!(
            "unsupported disc container: .{other}"
        ))),
    }
}

// ---------- File streaming primitives -------------------------------------

/// Stream the entire file at `path` through SHA-1. Returns the hex
/// digest + total bytes read. Cancel check fires every 1 MiB.
fn stream_file_sha1(
    path: &Path,
    cancel: Option<&dyn Fn() -> bool>,
) -> Result<(String, u64), HashError> {
    let mut file = File::open(path)?;
    let mut hasher = Sha1::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut total: u64 = 0;
    let mut since_check: u64 = 0;
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        total += n as u64;
        since_check += n as u64;
        if since_check >= CANCEL_CHECK_BYTES {
            if let Some(c) = cancel {
                if c() {
                    return Err(HashError::Cancelled);
                }
            }
            since_check = 0;
        }
    }
    let digest = hasher.finalize();
    Ok((hex_lower(&digest), total))
}

/// Stream `[start_byte, end_byte)` of `path` through SHA-1.
/// `end_byte == 0` means "to end of file" (sentinel; cuesheets that
/// can't produce an explicit end use this). Cancel check fires every
/// 1 MiB.
fn stream_file_slice_sha1(
    path: &Path,
    start_byte: u64,
    end_byte: u64,
    cancel: Option<&dyn Fn() -> bool>,
) -> Result<(String, u64), HashError> {
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(start_byte))?;
    let want = if end_byte == 0 || end_byte <= start_byte {
        u64::MAX
    } else {
        end_byte - start_byte
    };
    let mut hasher = Sha1::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut total: u64 = 0;
    let mut since_check: u64 = 0;
    while total < want {
        let to_read = ((want - total).min(buf.len() as u64)) as usize;
        let n = file.read(&mut buf[..to_read])?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        total += n as u64;
        since_check += n as u64;
        if since_check >= CANCEL_CHECK_BYTES {
            if let Some(c) = cancel {
                if c() {
                    return Err(HashError::Cancelled);
                }
            }
            since_check = 0;
        }
    }
    let digest = hasher.finalize();
    Ok((hex_lower(&digest), total))
}

fn hex_lower(digest: &[u8]) -> String {
    let mut out = String::with_capacity(digest.len() * 2);
    for b in digest {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

// ---------- .iso ----------------------------------------------------------

/// Hash a single-track .iso file. Used by DVD-shape systems
/// (GameCube / Wii / PS2 / PSP) where the entire .iso IS the
/// canonical track data — no framing, no audio, no track switches.
///
/// Reports `track_number: 1`, `track_mode: "MODE1/2048"`. Note:
/// MODE1/2048 cooked .iso for CD systems (PSX etc.) is NOT in
/// redump's per-track table (TOSEC convention) — the caller decides
/// whether to attempt matching.
fn hash_iso(
    path: &Path,
    cancel: Option<&dyn Fn() -> bool>,
) -> Result<Vec<TrackHash>, HashError> {
    let (sha1, size_bytes) = stream_file_sha1(path, cancel)?;
    Ok(vec![TrackHash {
        track_number: 1,
        track_mode: "MODE1/2048".to_string(),
        sha1,
        size_bytes,
    }])
}

// ---------- .cue (split-bin + merged-bin) ---------------------------------

/// Hash all tracks of a .cue-containered disc. Detects per-file
/// layout: a FILE block referenced by a single TRACK is split-bin
/// (whole-file hash); a FILE block referenced by multiple TRACKs is
/// merged-bin (slice by INDEX 01 positions × sector_size).
///
/// Per the 2026-06-03 research pass: hash the raw bytes
/// byte-for-byte, no preprocessing across MODE1/2352, MODE2/2352, or
/// audio.
fn hash_cue(
    cue_path: &Path,
    cancel: Option<&dyn Fn() -> bool>,
) -> Result<Vec<TrackHash>, HashError> {
    let text = std::fs::read_to_string(cue_path)?;
    let tracks = cd_id::cue::parse(&text);
    if tracks.is_empty() {
        return Err(HashError::Parse(format!(
            "no tracks in cue {}",
            cue_path.display()
        )));
    }
    let cue_parent = cue_path.parent().ok_or_else(|| {
        HashError::Parse(format!("cue has no parent dir: {}", cue_path.display()))
    })?;

    // Group track refs by FILE while preserving track order. Vec-of-pairs
    // keeps insertion order without an extra index map.
    let mut by_file: Vec<(String, Vec<usize>)> = Vec::new();
    for (i, t) in tracks.iter().enumerate() {
        if let Some((_, group)) = by_file.iter_mut().find(|(f, _)| f == &t.file) {
            group.push(i);
        } else {
            by_file.push((t.file.clone(), vec![i]));
        }
    }

    let mut out: Vec<TrackHash> = Vec::with_capacity(tracks.len());
    for (file, indices) in by_file {
        let bin_path = cue_parent.join(&file);
        if indices.len() == 1 {
            // Split-bin: whole .bin file is this track's bytes.
            let track = &tracks[indices[0]];
            let (sha1, size_bytes) = stream_file_sha1(&bin_path, cancel)?;
            out.push(TrackHash {
                track_number: track.track_no,
                track_mode: track.mode.clone(),
                sha1,
                size_bytes,
            });
        } else {
            // Merged-bin: derive per-track byte ranges from INDEX 01
            // positions × sector_size. All tracks in a single .bin
            // share the same sector size (practical CD reality), so
            // use the first track's sector_size for boundary math.
            let sector_size = tracks[indices[0]].sector_size().max(2352) as u64;
            let file_size = std::fs::metadata(&bin_path)
                .map_err(|e| {
                    HashError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("stat {}: {e}", bin_path.display()),
                    ))
                })?
                .len();
            let mut sorted_indices = indices.clone();
            sorted_indices.sort_by_key(|i| tracks[*i].track_no);
            for (i, &idx) in sorted_indices.iter().enumerate() {
                let track = &tracks[idx];
                let start_byte = track.index01_sectors * sector_size;
                let end_byte = if i + 1 < sorted_indices.len() {
                    tracks[sorted_indices[i + 1]].index01_sectors * sector_size
                } else {
                    file_size
                };
                if end_byte < start_byte {
                    return Err(HashError::Parse(format!(
                        "cue {}: track {} start {} > end {} (malformed INDEX 01)",
                        cue_path.display(),
                        track.track_no,
                        start_byte,
                        end_byte
                    )));
                }
                let (sha1, size_bytes) =
                    stream_file_slice_sha1(&bin_path, start_byte, end_byte, cancel)?;
                out.push(TrackHash {
                    track_number: track.track_no,
                    track_mode: track.mode.clone(),
                    sha1,
                    size_bytes,
                });
            }
        }
    }
    // Final output in track-number order (cues are usually sequential
    // already, but multi-file cues may interleave — sort for
    // deterministic matcher input).
    out.sort_by_key(|t| t.track_number);
    Ok(out)
}

// ---------- .gdi (Dreamcast) ----------------------------------------------

/// One track parsed out of a .gdi file. The format is one
/// space-separated line per track: `track_no track_lba sector_type
/// sector_size filename file_offset`. Quoted filenames carry
/// embedded spaces; we handle either.
#[derive(Debug, Clone, PartialEq, Eq)]
struct GdiTrack {
    track_no: u32,
    /// sector_type 4 = MODE1 data, 0 = AUDIO, others undefined for
    /// modern Dreamcast dumps.
    sector_type: u32,
    sector_size: u32,
    file: String,
}

impl GdiTrack {
    fn mode_string(&self) -> &'static str {
        match (self.sector_type, self.sector_size) {
            (4, 2352) => "MODE1/2352",
            (4, 2048) => "MODE1/2048",
            (0, _) => "AUDIO",
            _ => "DATA",
        }
    }
}

/// Parse a .gdi file. First line is the track count; subsequent lines
/// describe one track each. Tolerant of: quoted filenames with
/// spaces, missing trailing fields (file_offset defaults to 0), and
/// extra whitespace.
fn parse_gdi(text: &str) -> Vec<GdiTrack> {
    let mut lines = text.lines();
    let _count_line = lines.next();
    let mut out = Vec::new();
    for raw in lines {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(track) = parse_gdi_line(line) {
            out.push(track);
        }
    }
    out
}

fn parse_gdi_line(line: &str) -> Option<GdiTrack> {
    // Five fields: `track_no track_lba sector_type sector_size
    // filename [file_offset]`. Filename may be double-quoted to
    // contain whitespace (`"My Game.bin"`); the others are bare
    // numerics. Roll our own whitespace split that respects quotes
    // to avoid pulling in a regex dep.
    let parts = split_respecting_quotes(line);
    if parts.len() < 5 {
        return None;
    }
    let track_no = parts[0].parse::<u32>().ok()?;
    let _track_lba = parts[1].parse::<u64>().ok()?;
    let sector_type = parts[2].parse::<u32>().ok()?;
    let sector_size = parts[3].parse::<u32>().ok()?;
    let file = parts[4].trim_matches('"').to_string();
    Some(GdiTrack {
        track_no,
        sector_type,
        sector_size,
        file,
    })
}

/// Split a string on whitespace, treating double-quoted regions as
/// single fields. Returns owned strings to avoid lifetime juggling.
fn split_respecting_quotes(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    for c in line.chars() {
        if c == '"' {
            in_quote = !in_quote;
            current.push(c);
            continue;
        }
        if !in_quote && c.is_whitespace() {
            if !current.is_empty() {
                out.push(std::mem::take(&mut current));
            }
        } else {
            current.push(c);
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// Hash a .gdi-containered disc (Dreamcast). Each track is its own
/// file — stream + hash each.
fn hash_gdi(
    gdi_path: &Path,
    cancel: Option<&dyn Fn() -> bool>,
) -> Result<Vec<TrackHash>, HashError> {
    let text = std::fs::read_to_string(gdi_path)?;
    let parsed = parse_gdi(&text);
    if parsed.is_empty() {
        return Err(HashError::Parse(format!(
            "no tracks in gdi {}",
            gdi_path.display()
        )));
    }
    let gdi_parent = gdi_path.parent().ok_or_else(|| {
        HashError::Parse(format!("gdi has no parent dir: {}", gdi_path.display()))
    })?;
    let mut out = Vec::with_capacity(parsed.len());
    for track in &parsed {
        let bin_path = gdi_parent.join(&track.file);
        let (sha1, size_bytes) = stream_file_sha1(&bin_path, cancel)?;
        out.push(TrackHash {
            track_number: track.track_no,
            track_mode: track.mode_string().to_string(),
            sha1,
            size_bytes,
        });
    }
    out.sort_by_key(|t| t.track_number);
    Ok(out)
}

// ---------- .chd ----------------------------------------------------------

/// One CHD track as parsed from CHT2 / CHTR / CHGD text metadata.
/// `phys_start` is the cumulative position in CHD frames after
/// 4-frame `TRACK_PADDING` accounting; `frames` is the hashable
/// extent. Padding frames between tracks are NOT in any track's
/// hash range.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ChdTrack {
    track_number: u32,
    mode: String,
    frames: u64,
    phys_start: u64,
}

/// CHD metadata FourCC tags for CD-ROM track entries (big-endian
/// ASCII u32). CHTR = legacy CHDMAN; CHT2 = modern (includes
/// pregap/postgap fields); CHGD = GD-ROM single-track entry. CHCD /
/// CHGT (combined blobs from very old CHDMAN) are deliberately
/// NOT matched — those are treated as "not per-track-matchable in
/// v1" per the plan's locked decision.
fn is_cd_track_metadata_tag(tag: u32) -> bool {
    const CHTR: u32 = u32::from_be_bytes(*b"CHTR");
    const CHT2: u32 = u32::from_be_bytes(*b"CHT2");
    const CHGD: u32 = u32::from_be_bytes(*b"CHGD");
    matches!(tag, CHTR | CHT2 | CHGD)
}

/// Parse a CHD track-metadata text blob. Format string lifted from
/// MAME `src/lib/util/cdrom.cpp`:
///
/// ```text
/// CHT2: TRACK:%d TYPE:%s SUBTYPE:%s FRAMES:%d PREGAP:%d PGTYPE:%s PGSUB:%s POSTGAP:%d
/// CHTR: TRACK:%d TYPE:%s SUBTYPE:%s FRAMES:%d
/// CHGD: TRACK:%d TYPE:%s SUBTYPE:%s FRAMES:%d PADFRAMES:%d PREGAP:%d ...
/// ```
///
/// We only extract `TRACK`, `TYPE`, and `FRAMES` — the rest are
/// informational for our hashing path. Returns `None` if any
/// required field is missing or unparseable.
fn parse_chd_track_metadata(text: &str) -> Option<(u32, String, u64)> {
    let mut track_no: Option<u32> = None;
    let mut mode: Option<String> = None;
    let mut frames: Option<u64> = None;
    for token in text.split_whitespace() {
        if let Some((key, value)) = token.split_once(':') {
            match key {
                "TRACK" => track_no = value.parse().ok(),
                "TYPE" => mode = Some(value.to_string()),
                "FRAMES" => frames = value.parse().ok(),
                _ => {}
            }
        }
    }
    Some((track_no?, mode?, frames?))
}

/// Find the index of the track owning the given global frame.
/// Returns `None` when the frame is in a padding span between tracks.
fn chd_track_for_frame(tracks: &[ChdTrack], frame: u64) -> Option<usize> {
    for (i, t) in tracks.iter().enumerate() {
        if frame >= t.phys_start && frame < t.phys_start + t.frames {
            return Some(i);
        }
    }
    None
}

/// Hash a `.chd` (MAME's Compressed Hunks of Data) disc image.
/// Implementation per the 2026-06-03 research pass:
///
/// 1. Open via `chd::Chd::open`.
/// 2. Enumerate `metadata_refs()`, parse CHT2/CHTR/CHGD text blobs to
///    derive per-track frame counts. CHCD legacy blob → not matched.
/// 3. Compute per-track `phys_start` accounting for CHDMAN's 4-frame
///    `TRACK_PADDING` (frames between tracks that aren't part of any
///    hash).
/// 4. Walk hunks via `Chd::hunk(n)` + `Hunk::read_hunk_in`. For each
///    CD frame in the decompressed hunk (`unit_bytes = 2448`), strip
///    the 96-byte subchannel tail (keep bytes 0..2352), dispatch to
///    the owning track's SHA-1 hasher.
/// 5. DVD-shape (`unit_bytes = 2048`, no CD-ROM metadata) hashes as a
///    single contiguous stream (matches the redump `.iso` shape).
///
/// Constant memory: one hunk buffer (~20 KB) + N SHA-1 contexts.
fn hash_chd(
    path: &Path,
    cancel: Option<&dyn Fn() -> bool>,
) -> Result<Vec<TrackHash>, HashError> {
    let file = File::open(path)?;
    let mut reader = std::io::BufReader::new(file);
    let mut chd = chd::Chd::open(&mut reader, None)
        .map_err(|e| HashError::Parse(format!("chd open {}: {e:?}", path.display())))?;

    let header = chd.header();
    let hunk_count = header.hunk_count();
    let hunk_size = header.hunk_size() as usize;
    let unit_bytes = header.unit_bytes() as usize;

    if hunk_count == 0 || hunk_size == 0 {
        return Err(HashError::Parse(format!(
            "chd {}: empty hunk layout",
            path.display()
        )));
    }

    // Read track metadata via a SEPARATE file handle. chd::Chd holds
    // a mutable borrow on the primary reader for the duration of the
    // hunk-walking pass; MetadataRef::read also wants a &mut F.
    // Opening the same .chd a second time sidesteps the borrow
    // conflict — both readers see identical bytes at identical
    // offsets, and metadata reads are tiny (a few KB at most).
    let mut tracks = read_chd_track_metadata(&mut chd, path)?;

    if tracks.is_empty() {
        // DVD-shape: no CD-ROM metadata, unit_bytes should be 2048.
        // Hash as a single contiguous stream — matches the redump
        // `.iso` convention for GameCube / Wii / PS2.
        if unit_bytes != 2048 {
            return Err(HashError::Parse(format!(
                "chd {}: no CD track metadata and unit_bytes={} (expected 2048 for DVD-shape; legacy CHCD blob not supported)",
                path.display(),
                unit_bytes
            )));
        }
        return hash_chd_dvd_shape(&mut chd, hunk_count, cancel);
    }

    // CD-shape: hunks contain 2448-byte frames. Compute per-track
    // phys ranges with CHDMAN's 4-frame TRACK_PADDING accounting.
    if hunk_size % CHD_CD_FRAME_BYTES != 0 {
        return Err(HashError::Parse(format!(
            "chd {}: CD-shape but hunk_size {hunk_size} not a multiple of {CHD_CD_FRAME_BYTES}",
            path.display()
        )));
    }
    tracks.sort_by_key(|t| t.track_number);
    let mut phys = 0u64;
    for t in &mut tracks {
        t.phys_start = phys;
        phys += t.frames.div_ceil(CHD_CD_TRACK_PADDING) * CHD_CD_TRACK_PADDING;
    }

    hash_chd_cd_shape(&mut chd, hunk_count, hunk_size, &tracks, cancel)
}

/// Helper: open the chd file a second time, walk metadata refs, parse
/// the CD-ROM track entries.
fn read_chd_track_metadata<R: Read + Seek>(
    chd: &mut chd::Chd<R>,
    path: &Path,
) -> Result<Vec<ChdTrack>, HashError> {
    let meta_file = File::open(path)?;
    let mut meta_reader = std::io::BufReader::new(meta_file);
    let mut tracks = Vec::new();
    for meta_ref in chd.metadata_refs() {
        let meta = meta_ref.read(&mut meta_reader).map_err(|e| {
            HashError::Parse(format!("chd metadata read {}: {e:?}", path.display()))
        })?;
        if !is_cd_track_metadata_tag(meta.metatag) {
            continue;
        }
        let text = std::str::from_utf8(&meta.value)
            .unwrap_or("")
            .trim_end_matches('\0')
            .trim();
        if let Some((track_no, mode, frames)) = parse_chd_track_metadata(text) {
            tracks.push(ChdTrack {
                track_number: track_no,
                mode,
                frames,
                phys_start: 0,
            });
        }
    }
    Ok(tracks)
}

/// Walk hunks, dispatch per-frame to per-track SHA-1 hashers.
fn hash_chd_cd_shape<R: Read + Seek>(
    chd: &mut chd::Chd<R>,
    hunk_count: u32,
    hunk_size: usize,
    tracks: &[ChdTrack],
    cancel: Option<&dyn Fn() -> bool>,
) -> Result<Vec<TrackHash>, HashError> {
    let frames_per_hunk = hunk_size / CHD_CD_FRAME_BYTES;
    let mut hunk_buf = chd.get_hunksized_buffer();
    let mut temp_buf = Vec::new();
    let mut hashers: Vec<Sha1> = tracks.iter().map(|_| Sha1::new()).collect();
    let mut sizes: Vec<u64> = vec![0; tracks.len()];
    let mut since_check: u64 = 0;
    for h in 0..hunk_count {
        let mut hunk = chd.hunk(h).map_err(|e| {
            HashError::Parse(format!("chd hunk {h}: {e:?}"))
        })?;
        hunk.read_hunk_in(&mut temp_buf, &mut hunk_buf).map_err(|e| {
            HashError::Parse(format!("chd read_hunk_in {h}: {e:?}"))
        })?;
        for f in 0..frames_per_hunk {
            let global_frame = (h as u64) * (frames_per_hunk as u64) + (f as u64);
            if let Some(track_idx) = chd_track_for_frame(tracks, global_frame) {
                let frame_start = f * CHD_CD_FRAME_BYTES;
                let user_end = frame_start + CD_USER_BYTES_PER_FRAME;
                hashers[track_idx].update(&hunk_buf[frame_start..user_end]);
                sizes[track_idx] += CD_USER_BYTES_PER_FRAME as u64;
            }
            // else: padding frame between tracks — skip.
        }
        since_check += hunk_size as u64;
        if since_check >= CANCEL_CHECK_BYTES {
            if let Some(c) = cancel {
                if c() {
                    return Err(HashError::Cancelled);
                }
            }
            since_check = 0;
        }
    }
    let mut out: Vec<TrackHash> = Vec::with_capacity(tracks.len());
    for ((track, hasher), size) in tracks.iter().zip(hashers).zip(sizes) {
        let digest = hasher.finalize();
        out.push(TrackHash {
            track_number: track.track_number,
            track_mode: track.mode.clone(),
            sha1: hex_lower(&digest),
            size_bytes: size,
        });
    }
    out.sort_by_key(|t| t.track_number);
    Ok(out)
}

/// DVD-shape (`unit_bytes = 2048`, no track metadata): hash every
/// hunk as a single contiguous stream.
fn hash_chd_dvd_shape<R: Read + Seek>(
    chd: &mut chd::Chd<R>,
    hunk_count: u32,
    cancel: Option<&dyn Fn() -> bool>,
) -> Result<Vec<TrackHash>, HashError> {
    let mut hunk_buf = chd.get_hunksized_buffer();
    let mut temp_buf = Vec::new();
    let mut hasher = Sha1::new();
    let mut total: u64 = 0;
    let mut since_check: u64 = 0;
    for h in 0..hunk_count {
        let mut hunk = chd.hunk(h).map_err(|e| {
            HashError::Parse(format!("chd hunk {h}: {e:?}"))
        })?;
        hunk.read_hunk_in(&mut temp_buf, &mut hunk_buf).map_err(|e| {
            HashError::Parse(format!("chd read_hunk_in {h}: {e:?}"))
        })?;
        hasher.update(&hunk_buf[..]);
        total += hunk_buf.len() as u64;
        since_check += hunk_buf.len() as u64;
        if since_check >= CANCEL_CHECK_BYTES {
            if let Some(c) = cancel {
                if c() {
                    return Err(HashError::Cancelled);
                }
            }
            since_check = 0;
        }
    }
    let digest = hasher.finalize();
    Ok(vec![TrackHash {
        track_number: 1,
        track_mode: "MODE1/2048".to_string(),
        sha1: hex_lower(&digest),
        size_bytes: total,
    }])
}

// ---------- Matcher / strictness ------------------------------------------

/// Compare an operator-side per-track set against a candidate
/// canonical entry's per-track set. Audio tracks are skipped from the
/// match decision (operator's audio dumps vary by tool — see
/// [`docs/PLANS/disc-track-sha1-matching.md`]); the matcher operates
/// only on data tracks.
///
/// Strictness modes match the plan:
/// - [`Strictness::Strict`]: every data track must match.
/// - [`Strictness::Threshold(N)`]: ≥ N% of data tracks must match.
/// - [`Strictness::Lenient`]: ≥ 1 data track matches.
pub fn evaluate_match(
    operator_tracks: &[TrackHash],
    canonical_tracks: &[RomTrackRow],
    strictness: Strictness,
) -> MatchResult {
    let canonical_data: std::collections::HashSet<String> = canonical_tracks
        .iter()
        .filter(|t| !is_audio_mode(&t.track_mode))
        .map(|t| t.sha1.to_ascii_lowercase())
        .collect();
    let mut matched: u32 = 0;
    let mut total_data: u32 = 0;
    for op in operator_tracks {
        if op.is_audio() {
            continue;
        }
        total_data += 1;
        if canonical_data.contains(&op.sha1.to_ascii_lowercase()) {
            matched += 1;
        }
    }
    let passes = match strictness {
        Strictness::Strict => matched > 0 && matched == total_data,
        Strictness::Threshold(pct) => {
            total_data > 0 && (matched as u64 * 100) >= (total_data as u64 * pct as u64)
        }
        Strictness::Lenient => matched > 0,
    };
    MatchResult {
        matched_tracks: matched,
        total_data_tracks: total_data,
        passes_strictness: passes,
    }
}

fn is_audio_mode(mode: &str) -> bool {
    let upper = mode.to_ascii_uppercase();
    upper.starts_with("AUDIO") || upper == "CDDA"
}

// ---------- Tests ---------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;

    fn tmp_dir(label: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "oa-disc-track-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("mkdir");
        p
    }

    fn write_file(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let p = dir.join(name);
        let mut f = File::create(&p).expect("create");
        f.write_all(bytes).expect("write");
        p
    }

    /// Reference SHA-1: empty input → da39a3ee5e6b4b0d3255bfef95601890afd80709.
    /// Test that our streaming hasher reproduces it.
    #[test]
    fn stream_file_sha1_empty_file_matches_known_digest() {
        let dir = tmp_dir("sha1-empty");
        let p = write_file(&dir, "empty.bin", &[]);
        let (digest, size) = stream_file_sha1(&p, None).expect("hash");
        assert_eq!(digest, "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(size, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// "abc" → a9993e364706816aba3e25717850c26c9cd0d89d.
    #[test]
    fn stream_file_sha1_abc_matches_known_digest() {
        let dir = tmp_dir("sha1-abc");
        let p = write_file(&dir, "abc.bin", b"abc");
        let (digest, size) = stream_file_sha1(&p, None).expect("hash");
        assert_eq!(digest, "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(size, 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stream_file_sha1_respects_cancel() {
        let dir = tmp_dir("sha1-cancel");
        // 2 MiB so we cross the CANCEL_CHECK_BYTES threshold at least once.
        let p = write_file(&dir, "big.bin", &vec![0u8; 2 * 1024 * 1024]);
        let cancelled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let cancel_fn = {
            let cancelled = cancelled.clone();
            move || cancelled.load(std::sync::atomic::Ordering::Relaxed)
        };
        let result = stream_file_sha1(&p, Some(&cancel_fn));
        assert!(matches!(result, Err(HashError::Cancelled)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn iso_hasher_returns_single_track() {
        let dir = tmp_dir("iso");
        let p = write_file(&dir, "game.iso", b"abc");
        let tracks = hash_iso(&p, None).expect("hash");
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].track_number, 1);
        assert_eq!(tracks[0].track_mode, "MODE1/2048");
        assert_eq!(tracks[0].sha1, "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(tracks[0].size_bytes, 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cue_split_bin_hashes_each_track_separately() {
        let dir = tmp_dir("cue-split");
        write_file(&dir, "track01.bin", b"data-track-01");
        write_file(&dir, "track02.bin", b"audio-track-02");
        let cue = "\
FILE \"track01.bin\" BINARY
  TRACK 01 MODE1/2352
    INDEX 01 00:00:00
FILE \"track02.bin\" BINARY
  TRACK 02 AUDIO
    INDEX 01 00:00:00
";
        let cue_path = write_file(&dir, "game.cue", cue.as_bytes());
        let tracks = hash_cue(&cue_path, None).expect("hash");
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].track_number, 1);
        assert_eq!(tracks[0].track_mode, "MODE1/2352");
        assert_eq!(tracks[0].size_bytes, "data-track-01".len() as u64);
        assert_eq!(tracks[1].track_number, 2);
        assert_eq!(tracks[1].track_mode, "AUDIO");
        assert_eq!(tracks[1].size_bytes, "audio-track-02".len() as u64);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cue_merged_bin_carves_by_index01_positions() {
        let dir = tmp_dir("cue-merged");
        // Two-track merged bin: 2 sectors of 2352-byte data, then 1
        // sector of audio. Track 2 starts at INDEX 01 02:00:00 (2 sec).
        // We're going to do something simpler: contrived payload of
        // 2 sectors data + 2 sectors audio, where each sector is 16
        // bytes (so the math is easy to verify by eye). Actual
        // 2352-byte sectors would need megabyte-scale fixtures.
        //
        // We can't change the real sector size without forking
        // CueTrack::sector_size; instead, run a real-shape test by
        // using a small file with INDEX boundaries at sector-aligned
        // positions. Pick a 5-sector merged .bin (2 data + 3 audio,
        // each 2352 bytes = 11760 total). Use minimal content
        // because we hash the bytes regardless of meaning.
        let sector = 2352usize;
        let mut bin = Vec::with_capacity(5 * sector);
        // Sector 0 + 1: data track (Track 1).
        for s in 0..2 {
            for _ in 0..sector {
                bin.push((s as u8).wrapping_mul(0xAA));
            }
        }
        // Sector 2-4: audio (Track 2).
        for s in 2..5 {
            for _ in 0..sector {
                bin.push((s as u8).wrapping_mul(0x55));
            }
        }
        write_file(&dir, "game.bin", &bin);
        let cue = "\
FILE \"game.bin\" BINARY
  TRACK 01 MODE1/2352
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    INDEX 01 00:00:02
";
        let cue_path = write_file(&dir, "game.cue", cue.as_bytes());
        let tracks = hash_cue(&cue_path, None).expect("hash");
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].size_bytes, (2 * sector) as u64);
        assert_eq!(tracks[1].size_bytes, (3 * sector) as u64);
        // Round-trip: hashing the same byte range directly should
        // produce the same digest as the merged-bin slicer.
        let direct1 = {
            let mut h = Sha1::new();
            h.update(&bin[..2 * sector]);
            hex_lower(&h.finalize())
        };
        let direct2 = {
            let mut h = Sha1::new();
            h.update(&bin[2 * sector..]);
            hex_lower(&h.finalize())
        };
        assert_eq!(tracks[0].sha1, direct1);
        assert_eq!(tracks[1].sha1, direct2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_gdi_handles_standard_format() {
        let text = "\
3
1 0 4 2352 track01.bin 0
2 750 0 2352 track02.raw 0
3 45000 4 2048 track03.iso 0
";
        let tracks = parse_gdi(text);
        assert_eq!(tracks.len(), 3);
        assert_eq!(tracks[0].track_no, 1);
        assert_eq!(tracks[0].mode_string(), "MODE1/2352");
        assert_eq!(tracks[0].file, "track01.bin");
        assert_eq!(tracks[1].mode_string(), "AUDIO");
        assert_eq!(tracks[2].mode_string(), "MODE1/2048");
    }

    #[test]
    fn parse_gdi_handles_quoted_filename_with_spaces() {
        let text = "\
1
1 0 4 2352 \"My Game (USA) Track 01.bin\" 0
";
        let tracks = parse_gdi(text);
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].file, "My Game (USA) Track 01.bin");
    }

    #[test]
    fn gdi_hasher_streams_each_per_track_file() {
        let dir = tmp_dir("gdi");
        write_file(&dir, "track01.bin", b"data-bytes");
        write_file(&dir, "track02.raw", b"audio-bytes");
        let gdi = "\
2
1 0 4 2352 track01.bin 0
2 750 0 2352 track02.raw 0
";
        let gdi_path = write_file(&dir, "game.gdi", gdi.as_bytes());
        let tracks = hash_gdi(&gdi_path, None).expect("hash");
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].track_number, 1);
        assert_eq!(tracks[0].track_mode, "MODE1/2352");
        assert_eq!(tracks[0].size_bytes, "data-bytes".len() as u64);
        assert_eq!(tracks[1].track_mode, "AUDIO");
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn op_track(no: u32, mode: &str, sha1: &str) -> TrackHash {
        TrackHash {
            track_number: no,
            track_mode: mode.into(),
            sha1: sha1.into(),
            size_bytes: 0,
        }
    }
    fn canon_row(no: u32, mode: &str, sha1: &str) -> RomTrackRow {
        RomTrackRow {
            sha1: sha1.into(),
            system_id: "psx".into(),
            game_name: "Test Game".into(),
            serial: None,
            track_number: no,
            track_mode: mode.into(),
            size_bytes: 0,
        }
    }

    #[test]
    fn evaluate_match_strict_requires_all_data_tracks() {
        let op = vec![
            op_track(1, "MODE1/2352", "aa"),
            op_track(2, "AUDIO", "bb"),
            op_track(3, "MODE1/2352", "cc"),
        ];
        let canon = vec![
            canon_row(1, "MODE1/2352", "aa"),
            canon_row(2, "AUDIO", "different"),
            canon_row(3, "MODE1/2352", "cc"),
        ];
        let r = evaluate_match(&op, &canon, Strictness::Strict);
        assert_eq!(r.matched_tracks, 2);
        assert_eq!(r.total_data_tracks, 2);
        assert!(
            r.passes_strictness,
            "strict passes when all data tracks match (audio mismatch ignored)"
        );
    }

    #[test]
    fn evaluate_match_strict_fails_when_any_data_track_misses() {
        let op = vec![
            op_track(1, "MODE1/2352", "aa"),
            op_track(2, "MODE1/2352", "wrong"),
        ];
        let canon = vec![
            canon_row(1, "MODE1/2352", "aa"),
            canon_row(2, "MODE1/2352", "cc"),
        ];
        let r = evaluate_match(&op, &canon, Strictness::Strict);
        assert_eq!(r.matched_tracks, 1);
        assert_eq!(r.total_data_tracks, 2);
        assert!(!r.passes_strictness);
    }

    #[test]
    fn evaluate_match_threshold_uses_percentage() {
        let op = vec![
            op_track(1, "MODE1/2352", "aa"),
            op_track(2, "MODE1/2352", "bb"),
            op_track(3, "MODE1/2352", "cc"),
            op_track(4, "MODE1/2352", "wrong"),
            op_track(5, "MODE1/2352", "wrong2"),
        ];
        let canon = vec![
            canon_row(1, "MODE1/2352", "aa"),
            canon_row(2, "MODE1/2352", "bb"),
            canon_row(3, "MODE1/2352", "cc"),
            canon_row(4, "MODE1/2352", "different"),
            canon_row(5, "MODE1/2352", "different2"),
        ];
        let r80 = evaluate_match(&op, &canon, Strictness::Threshold(80));
        assert_eq!(r80.matched_tracks, 3);
        assert!(!r80.passes_strictness, "3/5 = 60% < 80%");
        let r50 = evaluate_match(&op, &canon, Strictness::Threshold(50));
        assert!(r50.passes_strictness, "3/5 = 60% >= 50%");
    }

    #[test]
    fn evaluate_match_lenient_passes_on_one_track() {
        let op = vec![
            op_track(1, "MODE1/2352", "aa"),
            op_track(2, "MODE1/2352", "wrong"),
        ];
        let canon = vec![
            canon_row(1, "MODE1/2352", "aa"),
            canon_row(2, "MODE1/2352", "bb"),
        ];
        let r = evaluate_match(&op, &canon, Strictness::Lenient);
        assert!(r.passes_strictness);
    }

    #[test]
    fn evaluate_match_strict_fails_on_zero_total() {
        // All-audio disc — no data tracks to gate on. Strict fails.
        let op = vec![op_track(1, "AUDIO", "aa")];
        let canon = vec![canon_row(1, "AUDIO", "aa")];
        let r = evaluate_match(&op, &canon, Strictness::Strict);
        assert_eq!(r.total_data_tracks, 0);
        assert!(!r.passes_strictness);
    }

    // ---- .chd helper tests (no chd fixture needed) ----

    #[test]
    fn parse_chd_track_metadata_handles_cht2_format() {
        let text = "TRACK:1 TYPE:MODE1_RAW SUBTYPE:NONE FRAMES:333000 PREGAP:0 PGTYPE:MODE1 PGSUB:NONE POSTGAP:0";
        let (track_no, mode, frames) = parse_chd_track_metadata(text).expect("parse");
        assert_eq!(track_no, 1);
        assert_eq!(mode, "MODE1_RAW");
        assert_eq!(frames, 333000);
    }

    #[test]
    fn parse_chd_track_metadata_handles_legacy_chtr_format() {
        // Legacy CHTR omits PREGAP/POSTGAP/PGTYPE/PGSUB.
        let text = "TRACK:2 TYPE:AUDIO SUBTYPE:NONE FRAMES:7350";
        let (track_no, mode, frames) = parse_chd_track_metadata(text).expect("parse");
        assert_eq!(track_no, 2);
        assert_eq!(mode, "AUDIO");
        assert_eq!(frames, 7350);
    }

    #[test]
    fn parse_chd_track_metadata_handles_chgd_with_padframes() {
        // GD-ROM CHGD adds PADFRAMES between FRAMES and PREGAP.
        let text = "TRACK:3 TYPE:MODE1_RAW SUBTYPE:NONE FRAMES:540000 PADFRAMES:225 PREGAP:0 PGTYPE:MODE1 PGSUB:NONE POSTGAP:0";
        let (track_no, mode, frames) = parse_chd_track_metadata(text).expect("parse");
        assert_eq!(track_no, 3);
        assert_eq!(mode, "MODE1_RAW");
        assert_eq!(frames, 540000);
    }

    #[test]
    fn parse_chd_track_metadata_returns_none_on_missing_fields() {
        // No FRAMES — can't compute phys ranges.
        let text = "TRACK:1 TYPE:AUDIO";
        assert!(parse_chd_track_metadata(text).is_none());
    }

    #[test]
    fn is_cd_track_metadata_tag_recognizes_chtr_cht2_chgd() {
        // FourCC big-endian.
        assert!(is_cd_track_metadata_tag(u32::from_be_bytes(*b"CHTR")));
        assert!(is_cd_track_metadata_tag(u32::from_be_bytes(*b"CHT2")));
        assert!(is_cd_track_metadata_tag(u32::from_be_bytes(*b"CHGD")));
        // CHCD / CHGT (combined blobs) deliberately not recognised
        // per the plan's "not matched in v1" decision.
        assert!(!is_cd_track_metadata_tag(u32::from_be_bytes(*b"CHCD")));
        assert!(!is_cd_track_metadata_tag(u32::from_be_bytes(*b"CHGT")));
        assert!(!is_cd_track_metadata_tag(0));
    }

    #[test]
    fn chd_phys_start_applies_4_frame_padding() {
        // Three tracks with frame counts that exercise padding math:
        // track 1 = 333000 frames (exact 4-multiple) → no padding added
        // track 2 = 7351 frames (rounds up to 7352)
        // track 3 = 1 frame (rounds up to 4)
        let mut tracks = vec![
            ChdTrack { track_number: 1, mode: "MODE1_RAW".into(), frames: 333000, phys_start: 0 },
            ChdTrack { track_number: 2, mode: "AUDIO".into(), frames: 7351, phys_start: 0 },
            ChdTrack { track_number: 3, mode: "AUDIO".into(), frames: 1, phys_start: 0 },
        ];
        let mut phys = 0u64;
        for t in &mut tracks {
            t.phys_start = phys;
            phys += t.frames.div_ceil(CHD_CD_TRACK_PADDING) * CHD_CD_TRACK_PADDING;
        }
        assert_eq!(tracks[0].phys_start, 0);
        assert_eq!(tracks[1].phys_start, 333000); // exact 4-multiple → no padding
        assert_eq!(tracks[2].phys_start, 333000 + 7352); // 7351 → 7352 (padded)
        // Total disc phys size: 333000 + 7352 + 4 = 340356
        assert_eq!(phys, 333000 + 7352 + 4);
    }

    #[test]
    fn chd_track_for_frame_finds_owning_track_or_returns_none_for_padding() {
        let tracks = vec![
            ChdTrack { track_number: 1, mode: "MODE1_RAW".into(), frames: 10, phys_start: 0 },
            ChdTrack { track_number: 2, mode: "AUDIO".into(), frames: 7, phys_start: 12 }, // 2-frame padding gap
        ];
        assert_eq!(chd_track_for_frame(&tracks, 0), Some(0));
        assert_eq!(chd_track_for_frame(&tracks, 9), Some(0)); // last frame of track 1
        assert_eq!(chd_track_for_frame(&tracks, 10), None);   // padding
        assert_eq!(chd_track_for_frame(&tracks, 11), None);   // padding
        assert_eq!(chd_track_for_frame(&tracks, 12), Some(1)); // first frame of track 2
        assert_eq!(chd_track_for_frame(&tracks, 18), Some(1)); // last frame of track 2
        assert_eq!(chd_track_for_frame(&tracks, 19), None);   // beyond disc
    }

    #[test]
    fn extract_cue_index01_positions() {
        // Direct unit test of the cue parser's INDEX 01 capture.
        let cue = "\
FILE \"merged.bin\" BINARY
  TRACK 01 MODE1/2352
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    PREGAP 00:02:00
    INDEX 01 02:00:00
  TRACK 03 AUDIO
    INDEX 01 05:30:00
";
        let tracks = cd_id::cue::parse(cue);
        assert_eq!(tracks.len(), 3);
        assert_eq!(tracks[0].index01_sectors, 0);
        // 2 min × 60 × 75 = 9000 sectors
        assert_eq!(tracks[1].index01_sectors, 9000);
        // 5 min × 60 × 75 + 30 × 75 = 22500 + 2250 = 24750 sectors
        assert_eq!(tracks[2].index01_sectors, 24750);
    }

}
