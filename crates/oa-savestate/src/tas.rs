//! TAS (Tool-Assisted Speedrun) recording + deterministic replay.
//!
//! A recording is an initial save-state blob plus a per-frame stream of
//! input bits for each port. Replaying a recording from the same core +
//! ROM reproduces the gameplay frame-for-frame, assuming the core itself
//! is deterministic given `(state, input)` (true for every libretro core
//! we ship today).
//!
//! File format (on-disk):
//!
//! ```text
//! offset bytes  meaning
//! 0      5      magic "OATAS"
//! 5      2      u16 LE: format version (currently 1)
//! 7      ..     zstd-compressed payload (see [`write_payload`])
//! ```
//!
//! The payload is hand-rolled binary (length-prefixed strings + u32-LE
//! length-prefixed byte buffers + fixed-width primitives). Hand-rolled
//! beats serde-derive here because the bulk of the file is opaque save-
//! state bytes — serde encoding wouldn't compress better and would pull
//! in `bincode` as a new dep for what's structurally five fields and a
//! Vec.
//!
//! Replay safety:
//!
//! Replaying a recording against a different ROM than was used to make
//! it produces garbage at best and crashes the core at worst, so the
//! file records `core_file_name` + `rom_sha1_hex` + `system_id`. The
//! caller can sanity-check before dispatching the recording to the
//! emu thread; the load itself doesn't enforce it.

#![allow(missing_docs)]

use std::io::{self, Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

const MAGIC: &[u8; 5] = b"OATAS";
const VERSION: u16 = 1;
const ZSTD_LEVEL: i32 = 3;

/// One frame's worth of input bits, indexed by port. Frame number is
/// implicit (position in the recording's `input_frames` vec).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TasInputFrame {
    pub port0: u32,
    pub port1: u32,
    pub port2: u32,
    pub port3: u32,
}

/// Metadata header that prefixes the binary payload. Cheap to copy +
/// human-readable shape for Tauri serialization of "list recordings"
/// command (where we only return the headers, not the heavy state blobs).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TasHeader {
    pub system_id: String,
    /// Filename of the libretro core the recording was made against
    /// (e.g. "mednafen_pce_fast_libretro.dll"). Replay safety.
    pub core_file_name: String,
    /// SHA-1 of the ROM bytes, hex-encoded uppercase. Replay safety —
    /// playing back against a different ROM is undefined behavior.
    pub rom_sha1_hex: String,
    /// Core-reported fps at recording time. Used by the replay UI to
    /// show "X / Y seconds" timing.
    pub fps: f64,
    /// Unix milliseconds when recording stopped.
    pub recorded_at_unix_ms: i64,
    /// Operator-chosen display name. Empty string is allowed.
    pub display_name: String,
    /// Number of input frames in the recording. Convenience for UIs
    /// that want the count without parsing the full file.
    pub frame_count: u64,
}

/// Full on-disk shape. Initial state is the blob `Core::save_state`
/// produced at recording start; input_frames are dispatched 1:1 from
/// the start of replay.
#[derive(Clone, Debug)]
pub struct TasRecording {
    pub header: TasHeader,
    pub initial_state: Vec<u8>,
    pub input_frames: Vec<TasInputFrame>,
}

impl TasRecording {
    /// Build a fresh recording. Caller fills `input_frames` as recording
    /// progresses, then writes via [`write_to`].
    pub fn new(
        system_id: String,
        core_file_name: String,
        rom_sha1_hex: String,
        fps: f64,
        display_name: String,
        initial_state: Vec<u8>,
    ) -> Self {
        let header = TasHeader {
            system_id,
            core_file_name,
            rom_sha1_hex,
            fps,
            recorded_at_unix_ms: 0, // filled in at write time
            display_name,
            frame_count: 0, // filled in at write time
        };
        Self {
            header,
            initial_state,
            input_frames: Vec::new(),
        }
    }

    /// Serialize + write to a file. Compresses the payload via zstd.
    pub fn write_to(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let f = std::fs::File::create(path)?;
        let mut w = std::io::BufWriter::new(f);
        w.write_all(MAGIC)?;
        w.write_all(&VERSION.to_le_bytes())?;
        // Compress everything past the header into the rest of the file.
        let mut enc = zstd::Encoder::new(w, ZSTD_LEVEL)?;
        write_payload(&mut enc, self)?;
        enc.finish()?.flush()?;
        Ok(())
    }

    /// Read + decompress + parse. Errors on bad magic / unsupported
    /// version / truncated payload. Doesn't enforce that
    /// `header.core_file_name` matches an installed core — that's the
    /// caller's job (replay safety check happens in the shell).
    pub fn read_from(path: &Path) -> io::Result<Self> {
        let f = std::fs::File::open(path)?;
        let mut r = std::io::BufReader::new(f);
        let mut magic = [0u8; 5];
        r.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("not a TAS recording (bad magic: {magic:02x?})"),
            ));
        }
        let mut version_bytes = [0u8; 2];
        r.read_exact(&mut version_bytes)?;
        let version = u16::from_le_bytes(version_bytes);
        if version != VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported TAS recording version {version} (expected {VERSION})"),
            ));
        }
        let mut dec = zstd::Decoder::new(r)?;
        read_payload(&mut dec)
    }

    /// Header-only read — skips parsing the bulky initial_state +
    /// input_frames. Useful for "list recordings" UI that just wants the
    /// metadata fields. We still need to decompress through the binary
    /// header section, but stop after the header rather than reading the
    /// whole tail.
    pub fn read_header_only(path: &Path) -> io::Result<TasHeader> {
        let f = std::fs::File::open(path)?;
        let mut r = std::io::BufReader::new(f);
        let mut magic = [0u8; 5];
        r.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "not a TAS recording",
            ));
        }
        let mut version_bytes = [0u8; 2];
        r.read_exact(&mut version_bytes)?;
        let version = u16::from_le_bytes(version_bytes);
        if version != VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported TAS recording version {version}"),
            ));
        }
        let mut dec = zstd::Decoder::new(r)?;
        read_header(&mut dec)
    }
}

fn write_payload<W: Write>(w: &mut W, rec: &TasRecording) -> io::Result<()> {
    write_header(w, &rec.header)?;
    write_bytes(w, &rec.initial_state)?;
    // Each input frame = 4 × u32 LE = 16 bytes. We emit raw bytes for
    // density (saves the per-element length tag).
    w.write_all(&(rec.input_frames.len() as u64).to_le_bytes())?;
    for f in &rec.input_frames {
        w.write_all(&f.port0.to_le_bytes())?;
        w.write_all(&f.port1.to_le_bytes())?;
        w.write_all(&f.port2.to_le_bytes())?;
        w.write_all(&f.port3.to_le_bytes())?;
    }
    Ok(())
}

fn read_payload<R: Read>(r: &mut R) -> io::Result<TasRecording> {
    let header = read_header(r)?;
    let initial_state = read_bytes(r)?;
    let count = read_u64(r)? as usize;
    let mut input_frames = Vec::with_capacity(count.min(1024 * 1024));
    let mut buf = [0u8; 16];
    for _ in 0..count {
        r.read_exact(&mut buf)?;
        let port0 = u32::from_le_bytes(buf[0..4].try_into().unwrap());
        let port1 = u32::from_le_bytes(buf[4..8].try_into().unwrap());
        let port2 = u32::from_le_bytes(buf[8..12].try_into().unwrap());
        let port3 = u32::from_le_bytes(buf[12..16].try_into().unwrap());
        input_frames.push(TasInputFrame { port0, port1, port2, port3 });
    }
    Ok(TasRecording {
        header,
        initial_state,
        input_frames,
    })
}

fn write_header<W: Write>(w: &mut W, h: &TasHeader) -> io::Result<()> {
    write_str(w, &h.system_id)?;
    write_str(w, &h.core_file_name)?;
    write_str(w, &h.rom_sha1_hex)?;
    w.write_all(&h.fps.to_le_bytes())?;
    w.write_all(&h.recorded_at_unix_ms.to_le_bytes())?;
    write_str(w, &h.display_name)?;
    w.write_all(&h.frame_count.to_le_bytes())?;
    Ok(())
}

fn read_header<R: Read>(r: &mut R) -> io::Result<TasHeader> {
    Ok(TasHeader {
        system_id: read_str(r)?,
        core_file_name: read_str(r)?,
        rom_sha1_hex: read_str(r)?,
        fps: read_f64(r)?,
        recorded_at_unix_ms: read_i64(r)?,
        display_name: read_str(r)?,
        frame_count: read_u64(r)?,
    })
}

fn write_str<W: Write>(w: &mut W, s: &str) -> io::Result<()> {
    write_bytes(w, s.as_bytes())
}

fn read_str<R: Read>(r: &mut R) -> io::Result<String> {
    let bytes = read_bytes(r)?;
    String::from_utf8(bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid utf8 in TAS string field"))
}

fn write_bytes<W: Write>(w: &mut W, b: &[u8]) -> io::Result<()> {
    w.write_all(&(b.len() as u32).to_le_bytes())?;
    w.write_all(b)
}

fn read_bytes<R: Read>(r: &mut R) -> io::Result<Vec<u8>> {
    let len = read_u32(r)? as usize;
    // Cap to a sane ceiling so a corrupt length field can't try to
    // allocate Vec<u8>::with_capacity(usize::MAX). 64 MiB is bigger than
    // any realistic field — even Saturn save states are ~3 MiB.
    if len > 64 * 1024 * 1024 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("TAS byte field claims length {len}; refusing"),
        ));
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    Ok(buf)
}

fn read_u32<R: Read>(r: &mut R) -> io::Result<u32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}

fn read_u64<R: Read>(r: &mut R) -> io::Result<u64> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b)?;
    Ok(u64::from_le_bytes(b))
}

fn read_f64<R: Read>(r: &mut R) -> io::Result<f64> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b)?;
    Ok(f64::from_le_bytes(b))
}

fn read_i64<R: Read>(r: &mut R) -> io::Result<i64> {
    let mut b = [0u8; 8];
    r.read_exact(&mut b)?;
    Ok(i64::from_le_bytes(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_recording() -> TasRecording {
        let mut rec = TasRecording::new(
            "tg16".into(),
            "mednafen_pce_fast_libretro.dll".into(),
            "ABC123".into(),
            59.826,
            "Bonk Lv1".into(),
            vec![1u8, 2, 3, 4, 5, 6, 7, 8], // pretend save-state blob
        );
        rec.header.recorded_at_unix_ms = 1_700_000_000_000;
        rec.input_frames = vec![
            TasInputFrame { port0: 0x0001, ..Default::default() },
            TasInputFrame { port0: 0x0011, port1: 0x0001, ..Default::default() },
            TasInputFrame { port0: 0x0000, ..Default::default() },
        ];
        rec.header.frame_count = rec.input_frames.len() as u64;
        rec
    }

    #[test]
    fn round_trip_through_disk() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-tas-roundtrip-{}-{}.tas",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let original = sample_recording();
        original.write_to(&tmp).expect("write");
        let read = TasRecording::read_from(&tmp).expect("read");
        assert_eq!(read.header, original.header);
        assert_eq!(read.initial_state, original.initial_state);
        assert_eq!(read.input_frames, original.input_frames);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn header_only_read_skips_payload() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-tas-header-only-{}-{}.tas",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let original = sample_recording();
        original.write_to(&tmp).expect("write");
        let header = TasRecording::read_header_only(&tmp).expect("read header");
        assert_eq!(header, original.header);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn rejects_bad_magic() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-tas-badmagic-{}-{}.tas",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::write(&tmp, b"NOPE!\x01\x00\x00\x00\x00\x00\x00\x00").expect("write garbage");
        let err = TasRecording::read_from(&tmp).expect_err("must reject");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn rejects_unsupported_version() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-tas-badversion-{}-{}.tas",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&999u16.to_le_bytes());
        std::fs::write(&tmp, &bytes).expect("write garbage");
        let err = TasRecording::read_from(&tmp).expect_err("must reject");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn empty_input_frames_round_trip() {
        // A recording stopped immediately after start should still be
        // valid — header + initial state + zero input frames.
        let tmp = std::env::temp_dir().join(format!(
            "oa-tas-empty-{}-{}.tas",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let mut rec = sample_recording();
        rec.input_frames.clear();
        rec.header.frame_count = 0;
        rec.write_to(&tmp).expect("write");
        let read = TasRecording::read_from(&tmp).expect("read");
        assert!(read.input_frames.is_empty());
        assert_eq!(read.header.frame_count, 0);
        let _ = std::fs::remove_file(&tmp);
    }
}
