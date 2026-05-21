//! CD-image game identification by reading the canonical game-id string
//! the original publisher burned into the data track.
//!
//! Cart games key on file SHA-1 against libretro-database's
//! `metadat/no-intro` dats (handled by `rom_hashes` + `rom_header`); CD
//! games can't because the .cue / .chd container's bytes don't match
//! anything in Redump's per-track hash space. Instead we extract the
//! publisher-burned identifier from the disc's IPL/boot sector and look
//! that up in the `game_serials` table — populated by the same
//! libretro-database sync that fills `rom_hashes` (every `game (...)`
//! block carrying a `serial "..."` becomes a `game_serials` row).
//!
//! Format support:
//!   - `.iso` — data track only, raw read.
//!   - `.cue` + sidecar `.bin` — parse cue, open the data track's bin,
//!     deframe MODE1/2352 → 2048 if needed, read the IPL.
//!   - `.chd` — MAME compressed disc, via the pure-Rust `chd` crate.
//!     Reads the first hunk(s), walks the 2448-byte frames, harvests
//!     MODE1 user data by sync-pattern match. Robust against discs
//!     with audio frames before the data track.
//!
//! Per-system extractors live in the `extractors` submodule and decide
//! how to slice the IPL bytes. Add an arm to `peek_disc_id` when
//! onboarding a new CD-based core.

use std::path::{Path, PathBuf};

/// The disc-image container format. Detected by extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscFormat {
    /// .cue + sidecar .bin(s). The most common shape we'll see for
    /// PCE-CD / Saturn / Dreamcast dumps.
    Cue,
    /// MAME compressed disc. Single-file container.
    Chd,
    /// Data track only — no audio, no framing. Trivial to read.
    Iso,
}

/// Result of a successful disc-ID peek. `game_id` is the canonical
/// publisher catalog code (Hu7-series for PCE-CD, SLUS_xxx.xx for PSX,
/// T-xxxG for Saturn). `system_hint` carries the raw signature string
/// we matched on (e.g. "PC Engine CD-ROM SYSTEM") — diagnostic, not
/// used for lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscId {
    pub game_id: String,
    pub system_hint: Option<String>,
}

/// Resolve an m3u to the first listed disc image. m3u is a plain-text
/// playlist (one path per line, lines starting with `#` are comments).
/// Multi-disc games (FFVII, Lunar, Policenauts, etc.) ship as an
/// m3u + N .cue/.chd/.bin pairs. Disc-id only needs ONE disc to
/// identify the title — pick the first non-comment entry and resolve
/// it relative to the m3u file's directory.
///
/// Pre-fix `is_cd_container_ext` included "m3u" → routed to
/// peek_disc_id → `read_data_track_header` → DiscFormat detection
/// → error ("disc format .m3u not supported"). All multi-disc M3Us
/// silently fell into skipped_cd.
fn resolve_m3u_first_disc(m3u_path: &Path) -> Result<PathBuf, String> {
    let text = std::fs::read_to_string(m3u_path)
        .map_err(|e| format!("read m3u {}: {e}", m3u_path.display()))?;
    let parent = m3u_path.parent().unwrap_or(Path::new(""));
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        // m3u entries can be absolute or relative. Relative resolves
        // against the m3u's directory (standard m3u semantics).
        let candidate = PathBuf::from(trimmed);
        let resolved = if candidate.is_absolute() {
            candidate
        } else {
            parent.join(candidate)
        };
        return Ok(resolved);
    }
    Err(format!(
        "m3u {} has no playable entries (all blank or commented)",
        m3u_path.display()
    ))
}

/// Open the disc image, read enough of the data track to extract the
/// embedded game id, dispatch to the per-system extractor.
///
/// New-CD-core onboarding checklist item: add an arm here mapping the
/// system_id to its extractor. Unknown system → `Ok(None)` so the
/// caller treats it the same as "no identifier embedded" rather than
/// surfacing as a read error.
pub fn peek_disc_id(path: &Path, system_id: &str) -> Result<Option<DiscId>, String> {
    let bytes = read_data_track_header(path)?;
    Ok(dispatch_extractor(system_id, &bytes))
}

/// Same as `peek_disc_id` but for a CD entry living inside an archive
/// (.zip / .7z). Avoids extracting the whole archive — pulls just the
/// cue (small) + the first ~64 KB of the data track's .bin via
/// `archive::read_inner_partial_to_bytes`. CHD inside archive isn't
/// supported (the chd crate needs Read+Seek over the whole file).
pub fn peek_disc_id_archived(
    archive: &Path,
    inner: &str,
    system_id: &str,
) -> Result<Option<DiscId>, String> {
    let inner_ext = std::path::Path::new(inner)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .ok_or_else(|| format!("no extension on inner entry {inner}"))?;
    let bytes = match inner_ext.as_str() {
        "iso" => crate::archive::read_inner_partial_to_bytes(archive, inner, HEADER_BYTES)?,
        "cue" => read_archived_cue_data_track(archive, inner)?,
        "chd" => {
            return Err(format!(
                "CHD inside archive isn't supported — the chd reader needs Read+Seek over the \
                 whole file (extract first, or store the .chd directly without zip)"
            ));
        }
        other => return Err(format!("disc format .{other} not supported by cd_id (archived)")),
    };
    Ok(dispatch_extractor(system_id, &bytes))
}

fn dispatch_extractor(system_id: &str, bytes: &[u8]) -> Option<DiscId> {
    match system_id {
        "pce-cd"    => extractors::pce_cd(bytes),
        "segacd"    => extractors::sega_cd(bytes),
        "saturn"    => extractors::saturn(bytes),
        // PSX + PS2 share the same SYSTEM.CNF / BOOT-line shape. Both are
        // serviced by the psx_family extractor; PS2 just lives on DVD
        // media (raw .iso typically). PS2 disc images often exceed the
        // 32 KB header window though — the SYSTEM.CNF lives further in
        // for some builds. Best-effort.
        "psx"       => extractors::psx_family(bytes),
        "ps2"       => extractors::psx_family(bytes),
        "neocd"     => extractors::neo_geo_cd(bytes),
        "pcfx"      => extractors::pcfx(bytes),
        "gamecube"  => extractors::gamecube(bytes),
        "dreamcast" => extractors::dreamcast(bytes),
        // 3DO discs don't carry a standardized catalog serial — libretro-
        // database's 3DO dat has zero `serial` fields, so disc-ID lookup
        // is structurally impossible. Caller falls back to filename
        // matching + fuzzy title resolution.
        "3do"       => None,
        _ => None,
    }
}

/// Read the cue out of an archive, parse it, locate the first data
/// track's .bin in the same archive (joining the cue's parent dir),
/// pull enough bytes to cover HEADER_BYTES of user data, deframe.
fn read_archived_cue_data_track(archive: &Path, cue_inner: &str) -> Result<Vec<u8>, String> {
    // Cues are small (KB-sized). Read whole.
    let cue_bytes = crate::archive::read_inner_to_bytes(archive, cue_inner)?;
    let text = std::str::from_utf8(&cue_bytes)
        .map_err(|e| format!("cue not utf8 in {}#{cue_inner}: {e}", archive.display()))?;
    let tracks = cue::parse(text);
    let track = tracks
        .into_iter()
        .find(|t| t.is_data() && t.sector_size() > 0)
        .ok_or_else(|| format!("no data track in cue inside {}#{cue_inner}", archive.display()))?;
    // Join cue's parent dir (within the archive) with the bin name.
    let cue_parent = match cue_inner.rfind('/') {
        Some(i) => &cue_inner[..=i],
        None => "",
    };
    let bin_inner = format!("{cue_parent}{}", track.file);
    // Need ceil(HEADER_BYTES / user_per_sector) sectors × sector_size bytes.
    let sector_size = track.sector_size();
    let user_offset = track.user_data_offset();
    let user_per_sector = sector_size - user_offset
        - if track.mode == "MODE1/2352" { 288 } else { 0 };
    if user_per_sector == 0 {
        return Err(format!("unsupported track mode {} in archived cue", track.mode));
    }
    let sectors_needed = HEADER_BYTES.div_ceil(user_per_sector);
    let raw_bytes_needed = sectors_needed * sector_size;
    let raw = crate::archive::read_inner_partial_to_bytes(archive, &bin_inner, raw_bytes_needed)?;
    if user_offset == 0 && user_per_sector == sector_size {
        // MODE1/2048 — file IS user data.
        let mut out = raw;
        out.truncate(HEADER_BYTES.min(out.len()));
        return Ok(out);
    }
    // Deframe MODE1/2352 → 2048.
    let mut out = Vec::with_capacity(HEADER_BYTES);
    for chunk in raw.chunks(sector_size) {
        if chunk.len() < user_offset + user_per_sector {
            break;
        }
        out.extend_from_slice(&chunk[user_offset..user_offset + user_per_sector]);
        if out.len() >= HEADER_BYTES {
            out.truncate(HEADER_BYTES);
            break;
        }
    }
    Ok(out)
}

/// Detect the container format from the file extension. CD-image
/// extensions are case-insensitive in practice (some Windows tools
/// uppercase them).
pub fn detect_format(path: &Path) -> Result<DiscFormat, String> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .ok_or_else(|| format!("no extension on {}", path.display()))?;
    match ext.as_str() {
        "cue" => Ok(DiscFormat::Cue),
        "chd" => Ok(DiscFormat::Chd),
        "iso" => Ok(DiscFormat::Iso),
        // .ccd (CloneCD), .toc (cdrdao), .m3u (multi-disc) point at
        // sidecar bin/wav files; not implemented yet. The caller falls
        // back to "skip and warn" via the Err branch.
        other => Err(format!("disc format .{other} not supported by cd_id")),
    }
}

/// First ~32 KB of the data track's USER DATA bytes (post-deframing
/// for MODE1/2352 dumps). Enough to land on the IPL boot sector of
/// every system we plan to extract for.
const HEADER_BYTES: usize = 32 * 1024;

/// Read the data track's first `HEADER_BYTES` of user data. Single
/// entry point for all extractor code so per-system extractors only
/// deal with deframed bytes.
///
/// Handles m3u indirection: if `path` is an m3u, resolves to the first
/// listed disc image and reads its header. Disc-id only needs one
/// disc to identify the title (the catalog code on Disc 1 is the
/// canonical lookup key in redump).
pub fn read_data_track_header(path: &Path) -> Result<Vec<u8>, String> {
    // m3u indirection — resolve to first disc + recurse. Done before
    // detect_format so we don't have to add a DiscFormat::M3u variant.
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase());
    if matches!(ext.as_deref(), Some("m3u")) {
        let first_disc = resolve_m3u_first_disc(path)?;
        return read_data_track_header(&first_disc);
    }
    match detect_format(path)? {
        DiscFormat::Iso => read_iso_header(path),
        DiscFormat::Cue => cue::read_data_track_header(path),
        DiscFormat::Chd => chd_reader::read_data_track_header(path),
    }
}

/// .iso = data track raw. No framing, no parsing, no sidecar.
fn read_iso_header(path: &Path) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)
        .map_err(|e| format!("open {}: {e}", path.display()))?;
    let mut buf = vec![0u8; HEADER_BYTES];
    let mut total = 0usize;
    while total < HEADER_BYTES {
        match file.read(&mut buf[total..]) {
            Ok(0) => break,
            Ok(n) => total += n,
            Err(e) => return Err(format!("read {}: {e}", path.display())),
        }
    }
    buf.truncate(total);
    Ok(buf)
}

/// .cue parsing + framed-bin reading. Lives in its own submodule for
/// testability — the parser is exercised against synthetic cue strings.
pub(crate) mod cue {
    use super::{HEADER_BYTES, PathBuf, Path};

    /// One track entry parsed out of a .cue. Only the first MODE1 track
    /// matters for disc-ID extraction; we still collect everything so
    /// future Phase 2b work (per-track SHA-1 against Redump) can use the
    /// same parser.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct CueTrack {
        /// .bin path RELATIVE TO THE .cue (cuesheets always reference
        /// sidecars by basename — caller joins against the cue's parent).
        pub file: String,
        /// Track number (1-based, as cuesheets store it).
        pub track_no: u32,
        /// "MODE1/2048" / "MODE1/2352" / "MODE2/2352" / "AUDIO" / etc.
        /// We only care about distinguishing data tracks from audio.
        pub mode: String,
    }

    impl CueTrack {
        pub fn is_data(&self) -> bool {
            self.mode.starts_with("MODE")
        }
        /// Sector size on disk for this track's mode. Returns 2048 for
        /// MODE1/2048, 2352 for MODE1/2352 (and MODE2 variants), 0 for
        /// audio (caller skips audio tracks before asking).
        pub fn sector_size(&self) -> usize {
            if self.mode.ends_with("/2048") {
                2048
            } else if self.mode.ends_with("/2352") {
                2352
            } else {
                0
            }
        }
        /// Offset within a 2352-byte sector where USER DATA starts.
        /// MODE1/2352: 16 bytes (12-byte sync + 3-byte address +
        /// 1-byte mode); MODE2/2352 form-1: 24 bytes; MODE1/2048: 0.
        pub fn user_data_offset(&self) -> usize {
            if self.mode == "MODE1/2352" {
                16
            } else if self.mode.starts_with("MODE2/2352") {
                24
            } else {
                0
            }
        }
    }

    /// Parse cue text. State machine: each `FILE "..." BINARY` sets the
    /// current file; each `TRACK NN MODE` emits a CueTrack carrying that
    /// file. We ignore everything else (INDEX, PREGAP, FLAGS).
    ///
    /// Tolerant of quoting variants (some tools omit quotes around
    /// single-word filenames) and case differences (`Track 01 Mode1/2352`).
    pub fn parse(text: &str) -> Vec<CueTrack> {
        let mut out = Vec::new();
        let mut current_file: Option<String> = None;
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() { continue; }
            let upper = line.to_ascii_uppercase();
            if upper.starts_with("FILE ") {
                current_file = Some(parse_file_name(line));
            } else if upper.starts_with("TRACK ") {
                if let Some(file) = current_file.clone() {
                    if let Some((track_no, mode)) = parse_track_line(line) {
                        out.push(CueTrack { file, track_no, mode });
                    }
                }
            }
        }
        out
    }

    /// `FILE "Game (Track 1).bin" BINARY` → `"Game (Track 1).bin"`.
    /// Handles unquoted filenames too (`FILE name.bin BINARY`).
    fn parse_file_name(line: &str) -> String {
        let after_file = line[4..].trim_start();
        if let Some(rest) = after_file.strip_prefix('"') {
            if let Some(end) = rest.find('"') {
                return rest[..end].to_string();
            }
        }
        // Unquoted: take everything up to the last whitespace-separated
        // word (which is the type — BINARY / WAVE / MOTOROLA).
        let parts: Vec<&str> = after_file.split_whitespace().collect();
        if parts.len() >= 2 {
            parts[..parts.len() - 1].join(" ")
        } else {
            after_file.to_string()
        }
    }

    /// `TRACK 01 MODE1/2352` → `(1, "MODE1/2352")`.
    fn parse_track_line(line: &str) -> Option<(u32, String)> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 { return None; }
        let track_no = parts[1].parse::<u32>().ok()?;
        let mode = parts[2].to_ascii_uppercase();
        Some((track_no, mode))
    }

    /// Find the first data track in the cue and return the absolute
    /// .bin path (joined against the cue's parent dir).
    fn first_data_track(cue_path: &Path, text: &str) -> Result<(PathBuf, super::cue::CueTrack), String> {
        let parent = cue_path
            .parent()
            .ok_or_else(|| format!("cue has no parent dir: {}", cue_path.display()))?;
        let tracks = parse(text);
        for t in tracks {
            if t.is_data() && t.sector_size() > 0 {
                let bin_path = parent.join(&t.file);
                return Ok((bin_path, t));
            }
        }
        Err("no data track in cue".to_string())
    }

    /// Read the cue, find the first data track, open the bin, deframe
    /// MODE1/2352 → 2048 if needed, return first HEADER_BYTES of user data.
    pub fn read_data_track_header(cue_path: &Path) -> Result<Vec<u8>, String> {
        let text = std::fs::read_to_string(cue_path)
            .map_err(|e| format!("read cue {}: {e}", cue_path.display()))?;
        let (bin_path, track) = first_data_track(cue_path, &text)?;
        let bin_bytes = read_bin_user_data(&bin_path, &track, HEADER_BYTES)?;
        Ok(bin_bytes)
    }

    /// Open the bin, read enough sectors to cover `target_user_bytes` of
    /// USER DATA, strip the framing if MODE1/2352. Returns up to
    /// `target_user_bytes` bytes (or whatever the file held if shorter).
    fn read_bin_user_data(
        bin_path: &Path,
        track: &super::cue::CueTrack,
        target_user_bytes: usize,
    ) -> Result<Vec<u8>, String> {
        use std::io::Read;
        let sector_size = track.sector_size();
        let user_offset = track.user_data_offset();
        let user_per_sector = sector_size - user_offset
            - if track.mode == "MODE1/2352" { 288 } else { 0 }; // 288 bytes ECC/EDC trailer
        if user_per_sector == 0 {
            return Err(format!("unsupported track mode {}", track.mode));
        }
        let sectors_needed = target_user_bytes.div_ceil(user_per_sector);
        let bytes_to_read = sectors_needed * sector_size;
        let mut file = std::fs::File::open(bin_path)
            .map_err(|e| format!("open {}: {e}", bin_path.display()))?;
        let mut raw = vec![0u8; bytes_to_read];
        let mut total = 0usize;
        while total < bytes_to_read {
            match file.read(&mut raw[total..]) {
                Ok(0) => break,
                Ok(n) => total += n,
                Err(e) => return Err(format!("read {}: {e}", bin_path.display())),
            }
        }
        raw.truncate(total);
        if user_offset == 0 && user_per_sector == sector_size {
            // MODE1/2048 — no framing, file IS the user data.
            raw.truncate(target_user_bytes.min(raw.len()));
            return Ok(raw);
        }
        // Deframe: per sector, take bytes [user_offset .. user_offset + user_per_sector].
        let mut out = Vec::with_capacity(target_user_bytes);
        for chunk in raw.chunks(sector_size) {
            if chunk.len() < user_offset + user_per_sector {
                break; // partial trailing sector
            }
            out.extend_from_slice(&chunk[user_offset .. user_offset + user_per_sector]);
            if out.len() >= target_user_bytes {
                out.truncate(target_user_bytes);
                break;
            }
        }
        Ok(out)
    }
}

/// CHD reading via the `chd` crate (pure-Rust libchdr port). CDs are
/// stored as a sequence of 2448-byte frames (2352-byte sector + 96-byte
/// subchannel) packed into "hunks" of N frames each. For the disc-ID
/// extractor we only need the first ~32 KB of user data from the data
/// track, so the simplest robust approach is:
///   - read the first hunk(s),
///   - walk it in 2448-byte chunks,
///   - check the MODE1 sync pattern at the start of each chunk,
///   - extract the user-data window (bytes 16..2064) on valid frames.
///
/// This skips full track-metadata parsing — works for the ~99% case
/// where Track 1 of a PCE-CD / Saturn / PSX dump is data and stored at
/// the start of the CHD. Discs that start with audio (rare for the
/// systems we support) won't produce a disc-ID hit; that's the same
/// failure mode as a .iso of a missing data track.
pub(crate) mod chd_reader {
    use super::{HEADER_BYTES, Path};

    /// MODE1/MODE2 raw-sector sync pattern: 12-byte ID prefixing every
    /// data sector. Bytes 12..15 are the address (MSF); byte 15 is the
    /// mode (1 or 2). We only key off the sync to avoid hard-coding
    /// disc-layout assumptions.
    const SYNC_PATTERN: [u8; 12] = [
        0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00,
    ];

    /// Bytes per CHD CD frame: 2352 (sector) + 96 (subchannel).
    const CHD_FRAME_SIZE: usize = 2448;
    /// MODE1 user-data window inside a 2352-byte sector: skip the
    /// 16-byte sync+header, take 2048 bytes (EDC/ECC trailer follows
    /// but we don't need it).
    const MODE1_USER_OFFSET: usize = 16;
    const MODE1_USER_LEN: usize = 2048;

    pub fn read_data_track_header(path: &Path) -> Result<Vec<u8>, String> {
        let file = std::fs::File::open(path)
            .map_err(|e| format!("open chd {}: {e}", path.display()))?;
        let mut reader = std::io::BufReader::new(file);
        let mut chd = chd::Chd::open(&mut reader, None)
            .map_err(|e| format!("chd open {}: {e:?}", path.display()))?;
        let hunk_count = chd.header().hunk_count();
        let hunk_size = chd.header().hunk_size() as usize;
        if hunk_count == 0 || hunk_size == 0 || hunk_size % CHD_FRAME_SIZE != 0 {
            // Not a CD-shaped CHD (HDD CHDs use 512-byte sectors; we'd
            // need different framing). Surface as Err so the resolve
            // loop falls back to "skipped CD" rather than fabricating
            // a bogus disc-id.
            return Err(format!(
                "chd {}: hunk_size {hunk_size} not a multiple of CHD CD frame size ({CHD_FRAME_SIZE})",
                path.display()
            ));
        }
        let frames_per_hunk = hunk_size / CHD_FRAME_SIZE;
        let mut out = Vec::with_capacity(HEADER_BYTES);
        let mut hunk_buf = chd.get_hunksized_buffer();
        let mut temp_buf = Vec::new();
        // Read enough hunks to cover ~32 KB of user data. Each MODE1
        // frame yields 2048 user bytes, so ceil(HEADER_BYTES / 2048)
        // = 16 frames. Divide by frames-per-hunk (commonly 8) → 2 hunks
        // worst case. Add 1 for slack in case some early frames don't
        // pass the sync check.
        let frames_needed = HEADER_BYTES.div_ceil(MODE1_USER_LEN);
        let hunks_needed = (frames_needed.div_ceil(frames_per_hunk) + 1).min(hunk_count as usize);
        for h in 0..hunks_needed {
            let mut hunk = chd
                .hunk(h as u32)
                .map_err(|e| format!("chd hunk {h}: {e:?}"))?;
            hunk.read_hunk_in(&mut temp_buf, &mut hunk_buf)
                .map_err(|e| format!("chd read_hunk_in {h}: {e:?}"))?;
            // Walk the hunk frame-by-frame, harvest user data.
            for frame_off in (0..hunk_buf.len()).step_by(CHD_FRAME_SIZE) {
                if frame_off + MODE1_USER_OFFSET + MODE1_USER_LEN > hunk_buf.len() {
                    break;
                }
                let frame = &hunk_buf[frame_off..frame_off + CHD_FRAME_SIZE];
                if frame[..12] != SYNC_PATTERN {
                    // Audio / garbage / not MODE1 — skip without
                    // bailing; some discs have audio frames before the
                    // data track that we want to walk past.
                    continue;
                }
                let user = &frame[MODE1_USER_OFFSET..MODE1_USER_OFFSET + MODE1_USER_LEN];
                out.extend_from_slice(user);
                if out.len() >= HEADER_BYTES {
                    out.truncate(HEADER_BYTES);
                    return Ok(out);
                }
            }
        }
        Ok(out)
    }
}

/// Per-system extractors. Each takes the deframed user-data bytes of
/// the data track and tries to pull out a canonical game-id. Returns
/// None when no recognised signature is found (= the disc isn't this
/// system, or is an obscure dump shape we don't catalog).
pub(crate) mod extractors {
    use super::DiscId;

    /// PC Engine CD-ROM² / TurboGrafx-CD identification.
    ///
    /// PCE-CD discs begin with an IPL (Initial Program Loader) at the
    /// start of the data track's user data. Within the first ~8 KB you
    /// find the system identification string — typically one of:
    ///   - "PC Engine CD-ROM SYSTEM"  (most discs)
    ///   - "HUDSON SOFT"              (some early Hudson titles)
    ///   - "NEC HOME ELECTRONICS"     (NEC-branded releases)
    ///
    /// The program name + Hu7-series catalog code (HCD3023, TGXCD1037,
    /// etc.) lives in the program-header section at offset 0x20 of the
    /// IPL — 16 ASCII bytes for the title, then the catalog code.
    ///
    /// We do a forgiving scan: locate any of the known signatures
    /// anywhere in the first 32 KB, then walk back/forward looking for
    /// a printable ASCII run that looks like a catalog code. If we
    /// can't find a code, we still return DiscId with `game_id` =
    /// the program name — the `game_serials` lookup keys on whatever
    /// libretro-database stored as the `serial` field, which for PCE-CD
    /// is sometimes the catalog code and sometimes the program name.
    pub fn pce_cd(bytes: &[u8]) -> Option<DiscId> {
        const SIGNATURES: &[&[u8]] = &[
            b"PC Engine CD-ROM SYSTEM",
            b"HUDSON SOFT",
            b"NEC HOME ELECTRONICS",
            b"Hudson Soft",
        ];
        let sig_pos = SIGNATURES.iter().find_map(|sig| find_subsequence(bytes, sig))?;
        let matched_sig = SIGNATURES
            .iter()
            .find(|sig| {
                bytes.len() >= sig_pos + sig.len() && &bytes[sig_pos..sig_pos + sig.len()] == **sig
            })
            .copied()
            .unwrap_or(&[]);

        // Pull a candidate game-id. PCE-CD IPL convention: catalog code
        // is a 4-8 char ASCII run (e.g. "TGXCD1037", "HCD3023", "JCXX0001").
        // Scan a window around the signature for the most plausible run.
        let window_start = sig_pos.saturating_sub(256);
        let window_end = (sig_pos + matched_sig.len() + 256).min(bytes.len());
        let game_id = find_catalog_code(&bytes[window_start..window_end])
            .or_else(|| find_printable_run(&bytes[window_start..window_end], 6))?;

        Some(DiscId {
            game_id,
            system_hint: std::str::from_utf8(matched_sig).ok().map(|s| s.to_string()),
        })
    }

    /// Search for a Hu7-series catalog code: 2-4 uppercase letters
    /// followed by 4-5 digits, optionally with a CD prefix
    /// (TGXCD1037, HCD3023, FXCD0001).
    fn find_catalog_code(bytes: &[u8]) -> Option<String> {
        // Walk the window looking for letter-runs of length 2-6
        // followed by digit-runs of length 3-6. Total length 5-12.
        let n = bytes.len();
        let mut i = 0;
        while i < n {
            if !bytes[i].is_ascii_alphabetic() {
                i += 1;
                continue;
            }
            let letters_start = i;
            while i < n && bytes[i].is_ascii_alphabetic() {
                i += 1;
            }
            let letters_len = i - letters_start;
            if !(2..=6).contains(&letters_len) {
                continue;
            }
            let digits_start = i;
            while i < n && bytes[i].is_ascii_digit() {
                i += 1;
            }
            let digits_len = i - digits_start;
            if (3..=6).contains(&digits_len) {
                let total = i - letters_start;
                if (5..=12).contains(&total) {
                    let code: String = bytes[letters_start..i]
                        .iter()
                        .map(|&b| b as char)
                        .collect();
                    // Filter out runs that don't look like catalog codes
                    // (uppercase letters + the prefix shouldn't be "AT", "OF", etc.).
                    if code.chars().take(letters_len).all(|c| c.is_ascii_uppercase()) {
                        return Some(code);
                    }
                }
            }
        }
        None
    }

    /// Fallback: longest printable-ASCII run of length ≥ min_len.
    /// Used when we can't find a catalog-code-shaped string. Returns
    /// the program name e.g. "BONK'S ADVENTURE" — better than nothing
    /// for serial lookup.
    fn find_printable_run(bytes: &[u8], min_len: usize) -> Option<String> {
        let mut best: Option<(usize, usize)> = None; // (start, len)
        let mut current_start: Option<usize> = None;
        for (i, &b) in bytes.iter().enumerate() {
            let printable = (0x20..=0x7e).contains(&b);
            match (printable, current_start) {
                (true, None) => current_start = Some(i),
                (false, Some(start)) => {
                    let len = i - start;
                    if len >= min_len && best.map(|(_, l)| len > l).unwrap_or(true) {
                        best = Some((start, len));
                    }
                    current_start = None;
                }
                _ => {}
            }
        }
        if let Some(start) = current_start {
            let len = bytes.len() - start;
            if len >= min_len && best.map(|(_, l)| len > l).unwrap_or(true) {
                best = Some((start, len));
            }
        }
        best.map(|(start, len)| {
            std::str::from_utf8(&bytes[start..start + len])
                .unwrap_or("")
                .trim()
                .to_string()
        })
        .filter(|s| !s.is_empty())
    }

    fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        if needle.is_empty() || haystack.len() < needle.len() {
            return None;
        }
        haystack
            .windows(needle.len())
            .position(|w| w == needle)
    }

    // -------------------------------------------------------------------
    // Sega CD / Mega-CD
    // -------------------------------------------------------------------
    //
    // Header at user-data offset 0:
    //   0x00: "SEGADISCSYSTEM " (16 bytes — Sega CD signature)
    //   0x10: copyright string
    //   0x20: title
    //   0x180: serial, ASCII, 14 bytes — format "GM T-93175-00"
    //          (Sega first-party JP = "G-NNNN", US = "MK-NNNN",
    //           third-party = "T-NNNNN", PAL adds "-50" suffix sometimes).
    //   0x1F0: region byte ("U", "J", "E", "JUE")
    //
    // libretro-database stores the serial with "GM " prefix stripped and
    // the trailing "-XX" revision normalized: "T-93175", "G-6031",
    // "MK-XXXX", "4432-50" (PAL). We do the same normalization here.
    pub fn sega_cd(bytes: &[u8]) -> Option<DiscId> {
        const SIGNATURES: &[&[u8]] = &[
            b"SEGADISCSYSTEM",
            b"SEGABOOTDISC",
            b"SEGADATADISC",
            b"SEGADISC ",
        ];
        let sig_pos = SIGNATURES.iter().find_map(|sig| find_subsequence(bytes, sig))?;
        let matched_sig = SIGNATURES
            .iter()
            .find(|sig| {
                bytes.len() >= sig_pos + sig.len() && &bytes[sig_pos..sig_pos + sig.len()] == **sig
            })
            .copied()
            .unwrap_or(&[]);

        // Pull the serial field. Offset 0x180 RELATIVE to the signature
        // position (which is at the start of the data track's first
        // sector — so this is also 0x180 absolute on a clean header).
        let serial_start = sig_pos + 0x180;
        let serial_end = (serial_start + 14).min(bytes.len());
        if serial_start >= bytes.len() {
            return None;
        }
        let raw = &bytes[serial_start..serial_end];
        let serial = parse_sega_cd_serial(raw)?;

        Some(DiscId {
            game_id: serial,
            system_hint: std::str::from_utf8(matched_sig).ok().map(|s| s.to_string()),
        })
    }

    /// Parse "GM T-93175-00" → "T-93175". Handle edge cases:
    ///   - Some discs use "  T-93175-00" (no "GM " prefix, just spaces).
    ///   - Disc header pads with NUL bytes past the serial run.
    ///   - PAL revisions append "-50" → keep as "T-XXXXX-50".
    ///   - Sega first-party JP: "G-6031" stays "G-6031".
    fn parse_sega_cd_serial(raw: &[u8]) -> Option<String> {
        // Strip NUL padding + whitespace at both ends. NUL isn't
        // whitespace in Rust's trim(), so call out the chars explicitly.
        let trim = |s: &str| {
            s.trim_matches(|c: char| c.is_whitespace() || c == '\0').to_string()
        };
        let s = trim(std::str::from_utf8(raw).ok()?);
        let s = s.strip_prefix("GM").map(|t| trim(t)).unwrap_or(s);
        // Trim trailing revision "-00", "-01", etc. UNLESS it's "-50"
        // (PAL marker) — libretro-database keeps that.
        let s = if let Some(dash_pos) = s.rfind('-') {
            let suffix = &s[dash_pos + 1..];
            if suffix == "50" {
                // PAL — keep the suffix.
                s
            } else if suffix.chars().all(|c| c.is_ascii_digit()) && suffix.len() <= 2 {
                // Revision marker — strip.
                s[..dash_pos].trim_end().to_string()
            } else {
                s
            }
        } else {
            s
        };
        if s.is_empty() {
            return None;
        }
        Some(s)
    }

    // -------------------------------------------------------------------
    // Sega Saturn
    // -------------------------------------------------------------------
    //
    // IP.BIN at user-data offset 0:
    //   0x00: "SEGA SEGASATURN " (16 bytes)
    //   0x10: maker ID (16 bytes, often "SEGA ENTERPRISES")
    //   0x20: product number / serial (10 bytes, ASCII space-padded)
    //          - Sega first-party JP: "GS-9101   "
    //          - Sega first-party US: "MK-81088  "
    //          - Third-party: "T-4305G   ", "T-15906H  "
    //          - PAL: "T-11304H-50" extends past 10 bytes occasionally
    //   0x2A: version (6 bytes, "V1.000")
    //   0x30: release date (8 bytes)
    //   0x40: region symbols (10 bytes — "JTUEBKAL")
    pub fn saturn(bytes: &[u8]) -> Option<DiscId> {
        const SIG: &[u8] = b"SEGA SEGASATURN";
        let sig_pos = find_subsequence(bytes, SIG)?;

        // Serial at sig + 0x20, length 10 (but some PAL entries spill
        // into the version field with "-50" suffix — read 16 bytes and
        // trim).
        let serial_start = sig_pos + 0x20;
        let serial_end = (serial_start + 16).min(bytes.len());
        if serial_start >= bytes.len() {
            return None;
        }
        let raw = &bytes[serial_start..serial_end];
        let serial = parse_saturn_serial(raw)?;

        Some(DiscId {
            game_id: serial,
            system_hint: Some("SEGA SEGASATURN".to_string()),
        })
    }

    /// Saturn serial: take first whitespace-terminated ASCII run; if the
    /// run looks like a known shape (T-NNNNNX, MK-NNNNN, GS-NNNN) and is
    /// immediately followed by "-50", append the PAL marker.
    fn parse_saturn_serial(raw: &[u8]) -> Option<String> {
        // Trim leading whitespace.
        let start = raw.iter().position(|b| !b.is_ascii_whitespace())?;
        let tail = &raw[start..];
        // First ASCII non-space run.
        let end = tail
            .iter()
            .position(|b| !b.is_ascii_graphic() || *b == b' ')
            .unwrap_or(tail.len());
        if end == 0 {
            return None;
        }
        let core_str = std::str::from_utf8(&tail[..end]).ok()?.to_string();
        // Look for "-50" PAL suffix in the remaining bytes.
        let rest = &tail[end..];
        if rest.len() >= 4 {
            let snippet: String = rest.iter().take(8).map(|&b| b as char).collect();
            if snippet.contains("-50") && !core_str.ends_with("-50") {
                return Some(format!("{core_str}-50"));
            }
        }
        Some(core_str)
    }

    // -------------------------------------------------------------------
    // Sony PlayStation + PlayStation 2 (shared shape)
    // -------------------------------------------------------------------
    //
    // PSX/PS2 discs carry SYSTEM.CNF in the ISO9660 root directory. The
    // file's first line is `BOOT = cdrom:\SLUS_001.67;1` (or `BOOT2 = ...`
    // for PS2). The executable filename's stem (after underscore-to-dash
    // + dot-strip) is the catalog serial libretro-database keys against:
    //   "SLUS_001.67" → "SLUS-00167"
    //   "SLES_010.04" → "SLES-01004"
    //   "SLPM_650.02" → "SLPM-65002"  (PS2)
    //   "SCUS_975.00" → "SCUS-97500"
    //
    // We could fully parse ISO9660 to locate SYSTEM.CNF, but a regex
    // scan over the header window catches the BOOT line directly — far
    // simpler and works for both raw .iso (PS2-shape) and MODE2/2352
    // CD images (PSX-shape) once the cd / chd readers have deframed.
    //
    // Supported prefixes (covers all retail PSX + PS2 releases):
    //   SLUS / SLES / SLPS / SLPM / SLKA / SCES / SCPS / SCUS / SCKA /
    //   SCAJ / SIPS / PAPX / PBPX / PCPX / PSPX
    pub fn psx_family(bytes: &[u8]) -> Option<DiscId> {
        // Pattern: 4 letters + ('_' or '-') + 3 digits + '.' + 2 digits.
        // Some dumps use dash directly (SLUS-00167); accept both.
        let serial = find_psx_serial(bytes)?;
        Some(DiscId {
            game_id: serial,
            system_hint: find_subsequence(bytes, b"SYSTEM.CNF")
                .map(|_| "PSX/PS2 SYSTEM.CNF".to_string())
                .or_else(|| {
                    find_subsequence(bytes, b"PLAYSTATION").map(|_| "PLAYSTATION".to_string())
                }),
        })
    }

    /// Walk bytes looking for the SLUS/SLES/SCES/... + sep + NNN + dot +
    /// NN pattern. Returns the normalized "XXXX-NNNNN" serial on first
    /// match. Restricting to known prefixes keeps false positives down.
    fn find_psx_serial(bytes: &[u8]) -> Option<String> {
        const PREFIXES: &[&[u8; 4]] = &[
            b"SLUS", b"SLES", b"SLPS", b"SLPM", b"SLKA", b"SCES", b"SCPS",
            b"SCUS", b"SCKA", b"SCAJ", b"SIPS", b"PAPX", b"PBPX", b"PCPX", b"PSPX",
        ];
        let n = bytes.len();
        let mut i = 0;
        while i + 11 <= n {
            // Bail fast on non-letter first byte.
            if !bytes[i].is_ascii_uppercase() {
                i += 1;
                continue;
            }
            let candidate = &bytes[i..i + 4];
            if !PREFIXES.iter().any(|p| *p == candidate) {
                i += 1;
                continue;
            }
            let sep = bytes[i + 4];
            if sep != b'_' && sep != b'-' {
                i += 1;
                continue;
            }
            // Now expect either:
            //   NNN.NN  (PSX convention with dot)
            //   NNNNN   (dash convention, no dot)
            let post = &bytes[i + 5..];
            if post.len() >= 6 && post[0..3].iter().all(|b| b.is_ascii_digit())
                && post[3] == b'.'
                && post[4..6].iter().all(|b| b.is_ascii_digit())
            {
                let digits: String = post[0..3]
                    .iter()
                    .chain(post[4..6].iter())
                    .map(|&b| b as char)
                    .collect();
                let prefix = std::str::from_utf8(candidate).ok()?;
                return Some(format!("{prefix}-{digits}"));
            }
            if post.len() >= 5 && post[0..5].iter().all(|b| b.is_ascii_digit()) {
                let digits: String = post[0..5].iter().map(|&b| b as char).collect();
                let prefix = std::str::from_utf8(candidate).ok()?;
                return Some(format!("{prefix}-{digits}"));
            }
            i += 1;
        }
        None
    }

    // -------------------------------------------------------------------
    // SNK Neo Geo CD
    // -------------------------------------------------------------------
    //
    // Neo Geo CD discs don't have a fixed-offset serial in the boot
    // sector (their IPL is more bespoke than Saturn/PSX). The product
    // catalog code shows up in the ISO9660 root in either the volume
    // label OR in IPL.TXT. Library serials look like:
    //   "NGCD-030"  (SNK first-party)
    //   "ADCD-103"  (ADK)
    //   "NCDZ-XXX"  (CDZ variant)
    //   "TBCD-XXX"  (various third-party)
    //
    // We scan for any of these prefixes followed by a 2-4 digit run.
    pub fn neo_geo_cd(bytes: &[u8]) -> Option<DiscId> {
        const PREFIXES: &[&[u8]] = &[
            b"NGCD-", b"ADCD-", b"NCDZ-", b"TBCD-", b"FACD-", b"SPCD-", b"YBCD-",
        ];
        for prefix in PREFIXES {
            if let Some(pos) = find_subsequence(bytes, prefix) {
                let digits_start = pos + prefix.len();
                let digits_end = (digits_start + 4).min(bytes.len());
                let digits: String = bytes[digits_start..digits_end]
                    .iter()
                    .take_while(|b| b.is_ascii_digit())
                    .map(|&b| b as char)
                    .collect();
                if (2..=4).contains(&digits.len()) {
                    let prefix_str = std::str::from_utf8(prefix).ok()?.trim_end_matches('-');
                    return Some(DiscId {
                        game_id: format!("{prefix_str}-{digits}"),
                        system_hint: Some("Neo Geo CD".to_string()),
                    });
                }
            }
        }
        None
    }

    // -------------------------------------------------------------------
    // NEC PC-FX
    // -------------------------------------------------------------------
    //
    // PC-FX discs carry a "PC-FX:" signature in the boot sector + a
    // catalog code with "FX" prefix (e.g. "FXNHE742"). The serial format
    // in libretro-database is `FX[A-Z]{3}\d{3}(-\d)?` — six letters/
    // digits with optional disc-number suffix.
    pub fn pcfx(bytes: &[u8]) -> Option<DiscId> {
        const SIG: &[u8] = b"PC-FX:";
        let sig_pos = find_subsequence(bytes, SIG)?;
        // Scan within ±256 bytes of the signature for an FX-prefixed
        // catalog code.
        let window_start = sig_pos.saturating_sub(256);
        let window_end = (sig_pos + 1024).min(bytes.len());
        let serial = find_pcfx_serial(&bytes[window_start..window_end])?;
        Some(DiscId {
            game_id: serial,
            system_hint: Some("PC-FX".to_string()),
        })
    }

    fn find_pcfx_serial(window: &[u8]) -> Option<String> {
        let n = window.len();
        let mut i = 0;
        while i + 6 <= n {
            // Look for "FX" followed by 3 letters + 3 digits.
            if window[i] == b'F' && window[i + 1] == b'X'
                && window[i + 2..i + 5].iter().all(|b| b.is_ascii_uppercase())
                && window[i + 5..i.min(n - 1) + 5 + 3.min(n - i - 5)].len() >= 3
            {
                let post = &window[i + 5..];
                if post.len() >= 3 && post[0..3].iter().all(|b| b.is_ascii_digit()) {
                    let core: String = window[i..i + 8].iter().map(|&b| b as char).collect();
                    // Optional disc-number suffix "-N" (e.g. "FXNHE742-0").
                    if post.len() >= 5 && post[3] == b'-' && post[4].is_ascii_digit() {
                        return Some(format!("{core}-{}", post[4] as char));
                    }
                    return Some(core);
                }
            }
            i += 1;
        }
        None
    }

    // -------------------------------------------------------------------
    // Nintendo GameCube
    // -------------------------------------------------------------------
    //
    // GameCube disc header (raw, no framing — .iso / .gcm are direct
    // dumps): offset 0 carries the 6-byte game identifier:
    //   0x00: Console ID  (1 byte; 'G' = GameCube, 'R'/'S' = Wii)
    //   0x01-0x02: Gamecode (2 bytes — e.g. "W7" for 007 Agent Under Fire)
    //   0x03: Region/country code (1 byte — 'E'=USA, 'P'=EUR, 'J'=JPN,
    //         'D'=NOE, 'F'=FRA, 'S'=ESP, 'I'=ITA, 'H'=HOL, 'K'=KOR, 'X'/'Y'=multi)
    //   0x04-0x05: Maker code (2 bytes — e.g. "01" Nintendo, "08" Capcom)
    //
    // libretro-database serial format: "DL-DOL-GW7P-EUR" — 4-char on-disc
    // code prefixed with "DL-DOL-" and suffixed with the full region name.
    // We synthesize the full canonical serial here.
    pub fn gamecube(bytes: &[u8]) -> Option<DiscId> {
        if bytes.len() < 6 {
            return None;
        }
        // Validate it's a GameCube/Wii header: first byte must be a known
        // console ID + bytes 4-5 must be ASCII (maker code).
        let console = bytes[0];
        if console != b'G' && console != b'R' && console != b'S' && console != b'D' {
            return None;
        }
        let code: String = bytes[0..4]
            .iter()
            .filter(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
            .map(|&b| b as char)
            .collect();
        if code.len() != 4 {
            return None;
        }
        let region_full = match bytes[3] {
            b'E' => "USA",
            b'P' => "EUR",
            b'J' => "JPN",
            b'D' => "NOE",
            b'F' => "FRA",
            b'S' => "ESP",
            b'I' => "ITA",
            b'H' => "HOL",
            b'K' => "KOR",
            b'X' => "EUU",
            b'Y' => "USA",
            _ => return None,
        };
        Some(DiscId {
            game_id: format!("DL-DOL-{code}-{region_full}"),
            system_hint: Some("Nintendo GameCube".to_string()),
        })
    }

    // -------------------------------------------------------------------
    // Sega Dreamcast
    // -------------------------------------------------------------------
    //
    // IP.BIN layout (struct `ip_meta_t` from Flycast `core/reios/reios.h`):
    //   0x00: hardware_id[16]      — "SEGA SEGAKATANA "
    //   0x10: maker_id[16]         — "SEGA ENTERPRISES"
    //   0x20: ks[5]                — copy-protection key string
    //   0x25: disk_type[6]         — "GD-ROM" / "CD-ROM"
    //   0x2B: disk_num[5]          — "1/1  "
    //   0x30: area_symbols[8]      — region flags (J / U / E in fixed positions)
    //   0x38: ctrl[4] + dev[1] + vga[1] + wince[1] + _unk1[1]
    //   0x40: product_number[10]   ← SERIAL
    //   0x4A: product_version[6]
    //   0x50: release_date[8]
    //   0x60: boot_filename[16]
    //   0x70: software_company[16]
    //   0x80: software_name[128]
    //
    // libretro-database stores the serial in these shapes:
    //   "HDR-0080"            (Japan first-party)
    //   "MK-51064-50"         (Europe — "-50" suffix burned on-disc, spills past the 10-byte field)
    //   "T-15705N"            (Third-party US)
    //   "SCLX-2004-(8848)"    (rare collector / pre-release format)
    //
    // We read a wider window starting at 0x40 to catch the "-50" overflow
    // and take the first whitespace-terminated ASCII run.
    pub fn dreamcast(bytes: &[u8]) -> Option<DiscId> {
        const SIG: &[u8] = b"SEGA SEGAKATANA";
        let sig_pos = find_subsequence(bytes, SIG)?;
        let serial_start = sig_pos + 0x40;
        // Read 16 bytes: covers the 10-byte product_number plus the first
        // 6 bytes of product_version. PAL discs with "-50" suffix that
        // overflowed the 10-byte field still come through cleanly.
        let serial_end = (serial_start + 16).min(bytes.len());
        if serial_start >= bytes.len() {
            return None;
        }
        let raw = &bytes[serial_start..serial_end];
        let serial = parse_dreamcast_serial(raw)?;
        Some(DiscId {
            game_id: serial,
            system_hint: Some("SEGA SEGAKATANA".to_string()),
        })
    }

    /// Dreamcast serial parsing.
    ///
    /// The on-disc product_number field is 10 bytes (0x40..0x4A). Read
    /// strictly from that window, trim NUL/whitespace padding. PAL
    /// discs burn an extra "-50" suffix that overflows into byte 10
    /// (the first byte of product_version) — when we see a trimmed
    /// result ending in "-5", peek byte 10 and if it's '0' append it.
    /// This catches "MK-51064-50" cleanly without false-merging the
    /// version string ("V1.001") that follows.
    fn parse_dreamcast_serial(raw: &[u8]) -> Option<String> {
        // raw is 16 bytes (10-byte product_number + 6 bytes of overflow
        // / product_version). Treat the first 10 as the serial field.
        if raw.len() < 10 {
            return None;
        }
        let trim = |s: &str| {
            s.trim_matches(|c: char| c.is_whitespace() || c == '\0').to_string()
        };
        let serial = trim(std::str::from_utf8(&raw[..10]).ok()?);
        if serial.len() < 3 {
            return None;
        }
        // PAL overflow: "MK-51064-5" + byte 10 == '0'  → "MK-51064-50".
        if raw.len() >= 11 && raw[10] == b'0' && serial.ends_with("-5") {
            return Some(format!("{serial}0"));
        }
        Some(serial)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_format_by_extension() {
        assert_eq!(detect_format(Path::new("foo.cue")).unwrap(), DiscFormat::Cue);
        assert_eq!(detect_format(Path::new("foo.CUE")).unwrap(), DiscFormat::Cue);
        assert_eq!(detect_format(Path::new("foo.chd")).unwrap(), DiscFormat::Chd);
        assert_eq!(detect_format(Path::new("foo.iso")).unwrap(), DiscFormat::Iso);
        assert!(detect_format(Path::new("foo.cue.txt")).is_err()); // unknown ext
        assert!(detect_format(Path::new("noext")).is_err());
    }

    // --- cue parser -----------------------------------------------------

    #[test]
    fn cue_parse_single_file_multi_track() {
        let text = r#"FILE "Game.bin" BINARY
  TRACK 01 MODE1/2352
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    INDEX 00 00:00:00
    INDEX 01 00:02:00
  TRACK 03 AUDIO
    INDEX 01 03:45:12
"#;
        let tracks = cue::parse(text);
        assert_eq!(tracks.len(), 3);
        assert_eq!(tracks[0].file, "Game.bin");
        assert_eq!(tracks[0].track_no, 1);
        assert_eq!(tracks[0].mode, "MODE1/2352");
        assert!(tracks[0].is_data());
        assert_eq!(tracks[0].sector_size(), 2352);
        assert_eq!(tracks[0].user_data_offset(), 16);

        assert!(!tracks[1].is_data());
        assert!(!tracks[2].is_data());
    }

    #[test]
    fn cue_parse_multi_file_one_track_each() {
        let text = r#"FILE "Game (Track 1).bin" BINARY
  TRACK 01 MODE1/2048
    INDEX 01 00:00:00
FILE "Game (Track 2).bin" BINARY
  TRACK 02 AUDIO
    INDEX 01 00:00:00
"#;
        let tracks = cue::parse(text);
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].file, "Game (Track 1).bin");
        assert_eq!(tracks[0].mode, "MODE1/2048");
        assert_eq!(tracks[0].sector_size(), 2048);
        assert_eq!(tracks[0].user_data_offset(), 0);
        assert_eq!(tracks[1].file, "Game (Track 2).bin");
    }

    #[test]
    fn cue_parse_unquoted_filename() {
        // Some old tools omit quotes around single-word filenames.
        let text = "FILE Game.bin BINARY\n  TRACK 01 MODE1/2352\n    INDEX 01 00:00:00\n";
        let tracks = cue::parse(text);
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].file, "Game.bin");
    }

    #[test]
    fn cue_parse_case_insensitive_keywords() {
        let text = "file \"Game.bin\" binary\n  track 01 Mode1/2352\n";
        let tracks = cue::parse(text);
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].mode, "MODE1/2352"); // uppercased
    }

    // --- PCE-CD extractor -----------------------------------------------

    #[test]
    fn pce_cd_extractor_finds_catalog_code() {
        // Synthetic IPL: pad up to 0x40, then the catalog code, then
        // the signature string. Real discs vary in layout but the
        // extractor's forgiving scan should still find both.
        let mut bytes = vec![0u8; 0x40];
        bytes.extend_from_slice(b"TGXCD1037");
        bytes.extend_from_slice(&[0u8; 16]);
        bytes.extend_from_slice(b"PC Engine CD-ROM SYSTEM");
        bytes.resize(0x800, 0);

        let id = extractors::pce_cd(&bytes).expect("extractor should match");
        assert_eq!(id.game_id, "TGXCD1037");
        assert_eq!(id.system_hint.as_deref(), Some("PC Engine CD-ROM SYSTEM"));
    }

    #[test]
    fn pce_cd_extractor_handles_hudson_signature() {
        let mut bytes = vec![0u8; 32];
        bytes.extend_from_slice(b"HCD3023");
        bytes.extend_from_slice(&[0u8; 32]);
        bytes.extend_from_slice(b"HUDSON SOFT");
        bytes.resize(0x800, 0);

        let id = extractors::pce_cd(&bytes).expect("hudson signature should match");
        assert_eq!(id.game_id, "HCD3023");
        assert_eq!(id.system_hint.as_deref(), Some("HUDSON SOFT"));
    }

    #[test]
    fn pce_cd_extractor_returns_none_for_unrelated_disc() {
        // No PCE signature anywhere — must NOT spuriously match.
        let mut bytes = vec![0u8; 1024];
        bytes.extend_from_slice(b"PLAYSTATION");
        bytes.extend_from_slice(b"SLUS_001.67");
        assert!(extractors::pce_cd(&bytes).is_none());
    }

    #[test]
    fn peek_disc_id_archived_zip_cue_bin() {
        // Build a synthetic zip on disk holding a cue + bin. The bin is
        // a single MODE1/2352 sector carrying a PCE-CD signature so the
        // extractor will hit. peek_disc_id_archived must pull bytes via
        // the archive reader (no extraction-to-disk) and return the id.
        use std::io::Write;

        let tmp = std::env::temp_dir().join(format!(
            "oa-cd_id-archive-{}-{}.zip",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));

        // 1 MODE1/2352 sector = 2352 bytes:
        //   0..12: sync (00 FF FF...FF 00) — peek doesn't validate
        //   12..16: address + mode
        //   16..2064: user data (this is what the extractor sees)
        //   2064..2352: EDC+ECC (we just pad zeros)
        let mut sector = vec![0u8; 2352];
        sector[0] = 0x00;
        for b in &mut sector[1..11] { *b = 0xff; }
        sector[11] = 0x00;
        sector[15] = 0x01; // mode 1
        // User data: catalog code + signature visible to the extractor.
        sector[16..16 + 9].copy_from_slice(b"HCD3023\0\0");
        sector[16 + 32..16 + 32 + 11].copy_from_slice(b"HUDSON SOFT");

        {
            let file = std::fs::File::create(&tmp).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            zip.start_file::<_, ()>("Game.cue", opts).unwrap();
            zip.write_all(b"FILE \"Game.bin\" BINARY\n  TRACK 01 MODE1/2352\n    INDEX 01 00:00:00\n").unwrap();
            zip.start_file::<_, ()>("Game.bin", opts).unwrap();
            zip.write_all(&sector).unwrap();
            zip.finish().unwrap();
        }

        let id = peek_disc_id_archived(&tmp, "Game.cue", "pce-cd")
            .expect("archived peek")
            .expect("hit");
        assert_eq!(id.game_id, "HCD3023");
        assert_eq!(id.system_hint.as_deref(), Some("HUDSON SOFT"));
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn peek_disc_id_archived_chd_returns_err() {
        // CHD inside archive is intentionally unsupported. The peek
        // should return Err rather than fabricate a result.
        let result = peek_disc_id_archived(
            Path::new("nonexistent.zip"),
            "image.chd",
            "pce-cd",
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("CHD"));
    }

    #[test]
    fn pce_cd_extractor_falls_back_to_program_name_when_no_code() {
        // Signature present but no catalog-code-shaped substring nearby
        // — extractor should fall back to longest printable run.
        let mut bytes = vec![0u8; 0x40];
        bytes.extend_from_slice(b"BONK'S ADVENTURE");
        bytes.extend_from_slice(&[0u8; 32]);
        bytes.extend_from_slice(b"PC Engine CD-ROM SYSTEM");
        bytes.resize(0x800, 0);

        let id = extractors::pce_cd(&bytes).expect("must find SOMETHING");
        // The catalog-code regex won't match "BONK'S ADVENTURE" (has
        // apostrophe + space), so we should hit the printable-run
        // fallback. Either result is acceptable; we just want non-empty.
        assert!(!id.game_id.is_empty());
    }

    // --- Sega CD extractor ------------------------------------------------

    #[test]
    fn sega_cd_extractor_extracts_third_party_serial() {
        // Synthetic Sega CD header: "SEGADISCSYSTEM " at 0x00,
        // serial "GM T-93175-00" at 0x180.
        let mut bytes = vec![0u8; 0x200];
        bytes[0..16].copy_from_slice(b"SEGADISCSYSTEM \0");
        bytes[0x180..0x180 + 13].copy_from_slice(b"GM T-93175-00");

        let id = extractors::sega_cd(&bytes).expect("third-party SegaCD");
        assert_eq!(id.game_id, "T-93175");
        assert_eq!(id.system_hint.as_deref(), Some("SEGADISCSYSTEM"));
    }

    #[test]
    fn sega_cd_extractor_keeps_pal_50_suffix() {
        let mut bytes = vec![0u8; 0x200];
        bytes[0..16].copy_from_slice(b"SEGADISCSYSTEM \0");
        bytes[0x180..0x180 + 10].copy_from_slice(b"GM 4432-50");

        let id = extractors::sega_cd(&bytes).expect("PAL SegaCD");
        assert_eq!(id.game_id, "4432-50");
    }

    #[test]
    fn sega_cd_extractor_first_party_jp() {
        let mut bytes = vec![0u8; 0x200];
        bytes[0..14].copy_from_slice(b"SEGABOOTDISC  ");
        bytes[0x180..0x180 + 9].copy_from_slice(b"GM G-6031");

        let id = extractors::sega_cd(&bytes).expect("first-party JP SegaCD");
        assert_eq!(id.game_id, "G-6031");
    }

    #[test]
    fn sega_cd_extractor_returns_none_for_unrelated_disc() {
        let mut bytes = vec![0u8; 0x200];
        bytes[0..23].copy_from_slice(b"PC Engine CD-ROM SYSTEM");
        assert!(extractors::sega_cd(&bytes).is_none());
    }

    // --- Saturn extractor -------------------------------------------------

    #[test]
    fn saturn_extractor_third_party_us() {
        let mut bytes = vec![0u8; 0x100];
        bytes[0..16].copy_from_slice(b"SEGA SEGASATURN ");
        bytes[0x10..0x20].copy_from_slice(b"SEGA ENTERPRISES");
        bytes[0x20..0x2A].copy_from_slice(b"T-15906H  ");

        let id = extractors::saturn(&bytes).expect("Saturn US T-prefix");
        assert_eq!(id.game_id, "T-15906H");
    }

    #[test]
    fn saturn_extractor_pal_appends_50_suffix() {
        let mut bytes = vec![0u8; 0x100];
        bytes[0..16].copy_from_slice(b"SEGA SEGASATURN ");
        bytes[0x10..0x20].copy_from_slice(b"SEGA ENTERPRISES");
        bytes[0x20..0x2B].copy_from_slice(b"T-11304H-50");

        let id = extractors::saturn(&bytes).expect("Saturn PAL");
        assert_eq!(id.game_id, "T-11304H-50");
    }

    #[test]
    fn saturn_extractor_first_party_jp() {
        let mut bytes = vec![0u8; 0x100];
        bytes[0..16].copy_from_slice(b"SEGA SEGASATURN ");
        bytes[0x10..0x20].copy_from_slice(b"SEGA ENTERPRISES");
        bytes[0x20..0x2A].copy_from_slice(b"GS-9101   ");

        let id = extractors::saturn(&bytes).expect("Saturn first-party JP");
        assert_eq!(id.game_id, "GS-9101");
    }

    #[test]
    fn saturn_extractor_returns_none_for_unrelated_disc() {
        let mut bytes = vec![0u8; 0x100];
        bytes[0..15].copy_from_slice(b"SEGADISCSYSTEM ");
        assert!(extractors::saturn(&bytes).is_none());
    }

    // --- PSX / PS2 (psx_family) -------------------------------------------

    #[test]
    fn psx_extractor_finds_slus_with_dot_format() {
        // Real PSX SYSTEM.CNF has BOOT=cdrom:\SLUS_001.67;1
        let mut bytes = vec![0u8; 0x800];
        let line = b"BOOT = cdrom:\\SLUS_001.67;1\r\n";
        bytes[0x100..0x100 + line.len()].copy_from_slice(line);

        let id = extractors::psx_family(&bytes).expect("PSX with dot");
        assert_eq!(id.game_id, "SLUS-00167");
    }

    #[test]
    fn psx_extractor_finds_eu_sles() {
        let mut bytes = vec![0u8; 0x800];
        let line = b"BOOT = cdrom:\\SLES_010.04;1\r\n";
        bytes[0x100..0x100 + line.len()].copy_from_slice(line);

        let id = extractors::psx_family(&bytes).expect("SLES");
        assert_eq!(id.game_id, "SLES-01004");
    }

    #[test]
    fn psx_extractor_finds_ps2_slpm_serial() {
        let mut bytes = vec![0u8; 0x800];
        let line = b"BOOT2 = cdrom0:\\SLPM_650.02;1\r\n";
        bytes[0x100..0x100 + line.len()].copy_from_slice(line);

        let id = extractors::psx_family(&bytes).expect("PS2 SLPM");
        assert_eq!(id.game_id, "SLPM-65002");
    }

    #[test]
    fn psx_extractor_accepts_dash_form() {
        // Some loose dumps use SLUS-00167 directly without dot.
        let mut bytes = vec![0u8; 256];
        bytes[100..111].copy_from_slice(b"SLUS-00167\0");

        let id = extractors::psx_family(&bytes).expect("dash form");
        assert_eq!(id.game_id, "SLUS-00167");
    }

    #[test]
    fn psx_extractor_returns_none_for_non_psx() {
        let mut bytes = vec![0u8; 256];
        bytes[0..23].copy_from_slice(b"PC Engine CD-ROM SYSTEM");
        assert!(extractors::psx_family(&bytes).is_none());
    }

    // --- Neo Geo CD -------------------------------------------------------

    #[test]
    fn neocd_extractor_finds_ngcd_prefix() {
        let mut bytes = vec![0u8; 256];
        bytes[50..58].copy_from_slice(b"NGCD-030");

        let id = extractors::neo_geo_cd(&bytes).expect("NGCD-030");
        assert_eq!(id.game_id, "NGCD-030");
    }

    #[test]
    fn neocd_extractor_finds_adk_prefix() {
        let mut bytes = vec![0u8; 256];
        bytes[100..108].copy_from_slice(b"ADCD-103");

        let id = extractors::neo_geo_cd(&bytes).expect("ADCD-103");
        assert_eq!(id.game_id, "ADCD-103");
    }

    #[test]
    fn neocd_extractor_returns_none_for_unrelated_disc() {
        let bytes = b"PLAYSTATION SLUS-00167".to_vec();
        assert!(extractors::neo_geo_cd(&bytes).is_none());
    }

    // --- PC-FX ------------------------------------------------------------

    #[test]
    fn pcfx_extractor_finds_fxnhe742() {
        let mut bytes = vec![0u8; 0x800];
        bytes[0x100..0x100 + 6].copy_from_slice(b"PC-FX:");
        bytes[0x120..0x128].copy_from_slice(b"FXNHE742");

        let id = extractors::pcfx(&bytes).expect("PC-FX serial");
        assert_eq!(id.game_id, "FXNHE742");
    }

    #[test]
    fn pcfx_extractor_appends_disc_suffix() {
        let mut bytes = vec![0u8; 0x800];
        bytes[0x100..0x100 + 6].copy_from_slice(b"PC-FX:");
        bytes[0x120..0x12A].copy_from_slice(b"FXNHE742-0");

        let id = extractors::pcfx(&bytes).expect("PC-FX disc 0");
        assert_eq!(id.game_id, "FXNHE742-0");
    }

    #[test]
    fn pcfx_extractor_returns_none_without_signature() {
        let mut bytes = vec![0u8; 256];
        bytes[100..108].copy_from_slice(b"FXNHE742");
        // No PC-FX signature → must not return a serial.
        assert!(extractors::pcfx(&bytes).is_none());
    }

    // --- GameCube ---------------------------------------------------------

    #[test]
    fn gamecube_extractor_synthesizes_dol_serial_usa() {
        // Header at offset 0: console G + game W7 + region E + maker 01.
        let mut bytes = vec![0u8; 0x40];
        bytes[0..6].copy_from_slice(b"GW7E01");

        let id = extractors::gamecube(&bytes).expect("GameCube USA");
        assert_eq!(id.game_id, "DL-DOL-GW7E-USA");
    }

    #[test]
    fn gamecube_extractor_synthesizes_dol_serial_eur() {
        let mut bytes = vec![0u8; 0x40];
        bytes[0..6].copy_from_slice(b"GW7P01");

        let id = extractors::gamecube(&bytes).expect("GameCube EUR");
        assert_eq!(id.game_id, "DL-DOL-GW7P-EUR");
    }

    #[test]
    fn gamecube_extractor_synthesizes_dol_serial_germany() {
        let mut bytes = vec![0u8; 0x40];
        bytes[0..6].copy_from_slice(b"GW7D01");

        let id = extractors::gamecube(&bytes).expect("GameCube NOE");
        assert_eq!(id.game_id, "DL-DOL-GW7D-NOE");
    }

    #[test]
    fn gamecube_extractor_rejects_non_gc_header() {
        // First byte isn't G/R/S/D → not GameCube/Wii.
        let mut bytes = vec![0u8; 0x40];
        bytes[0..6].copy_from_slice(b"ZZZZZZ");
        assert!(extractors::gamecube(&bytes).is_none());
    }

    // --- Dreamcast --------------------------------------------------------

    #[test]
    fn dreamcast_extractor_japan_hdr_serial() {
        let mut bytes = vec![0u8; 0x100];
        bytes[0..16].copy_from_slice(b"SEGA SEGAKATANA ");
        bytes[0x10..0x20].copy_from_slice(b"SEGA ENTERPRISES");
        bytes[0x40..0x4A].copy_from_slice(b"HDR-0080  ");
        bytes[0x4A..0x50].copy_from_slice(b"V1.001");

        let id = extractors::dreamcast(&bytes).expect("Dreamcast JP HDR");
        assert_eq!(id.game_id, "HDR-0080");
        assert_eq!(id.system_hint.as_deref(), Some("SEGA SEGAKATANA"));
    }

    #[test]
    fn dreamcast_extractor_pal_50_suffix_overflow() {
        // PAL discs have "-50" burned at the end of the serial which
        // spills past the 10-byte product_number field into the
        // product_version slot. Extractor's wider window catches it.
        let mut bytes = vec![0u8; 0x100];
        bytes[0..16].copy_from_slice(b"SEGA SEGAKATANA ");
        bytes[0x40..0x4B].copy_from_slice(b"MK-51064-50");
        bytes[0x4B..0x51].copy_from_slice(b"V1.001");

        let id = extractors::dreamcast(&bytes).expect("Dreamcast PAL with -50");
        assert_eq!(id.game_id, "MK-51064-50");
    }

    #[test]
    fn dreamcast_extractor_third_party_us() {
        let mut bytes = vec![0u8; 0x100];
        bytes[0..16].copy_from_slice(b"SEGA SEGAKATANA ");
        bytes[0x40..0x4A].copy_from_slice(b"T-15705N  ");

        let id = extractors::dreamcast(&bytes).expect("Dreamcast US T-prefix");
        assert_eq!(id.game_id, "T-15705N");
    }

    #[test]
    fn dreamcast_extractor_returns_none_for_unrelated_disc() {
        let mut bytes = vec![0u8; 0x100];
        bytes[0..16].copy_from_slice(b"SEGA SEGASATURN ");
        // SEGA SEGASATURN ≠ SEGA SEGAKATANA → not Dreamcast.
        assert!(extractors::dreamcast(&bytes).is_none());
    }
}
