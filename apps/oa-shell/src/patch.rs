//! Soft-patching for ROM hacks + translation patches.
//!
//! RetroArch parity slice — applies IPS / UPS / BPS patches to ROM bytes
//! before they reach the libretro core's `retro_load_game`. Format
//! detected from the patch file's magic bytes:
//!
//! - **IPS** (`PATCH` header) — the classic format. 24-bit BE offsets,
//!   simple offset+data records, RLE on `size == 0`, `EOF` terminator.
//!   Limited to ~16 MB base files but trivial to implement.
//! - **UPS** (`UPS1` header) — newer, supports any base-file size, uses
//!   variable-length integers + XOR diff blocks. CRC32s at the end
//!   validate input/output/patch integrity (not enforced in v1).
//! - **BPS** (`BPS1` header) — beats UPS on delta size; modern
//!   translation projects (esp. for Sega CD / Saturn / large carts)
//!   ship in this format. Four opcodes drive byte-level reconstruction.
//!
//! Scope:
//! - Byte-source ROMs only. CD images (.cue / .chd / .m3u) are opened
//!   by the core directly; patching them in-place would require
//!   shadow-mounting, out of scope here.
//! - CRC32 validation is skipped — adding it needs a CRC crate. v2
//!   polish.
//! - Patches that grow the ROM beyond the base are allowed (UPS/BPS
//!   both carry `output_size` explicitly; IPS implicitly extends).

use std::io::Read;

/// Patch container — the union of every format's payload that mattered
/// to us. Kept opaque externally; consumers call [`apply`] directly.
#[derive(Debug)]
pub enum Patch {
    Ips(Vec<u8>),
    Ups(Vec<u8>),
    Bps(Vec<u8>),
}

/// Decode a patch file's header. Returns the typed `Patch` enum
/// (carrying the rest of the buffer) or an error string naming the
/// detected magic.
pub fn parse(buf: Vec<u8>) -> Result<Patch, String> {
    if buf.len() < 5 {
        return Err("patch file too short".into());
    }
    if &buf[0..5] == b"PATCH" {
        return Ok(Patch::Ips(buf));
    }
    if &buf[0..4] == b"UPS1" {
        return Ok(Patch::Ups(buf));
    }
    if &buf[0..4] == b"BPS1" {
        return Ok(Patch::Bps(buf));
    }
    Err(format!(
        "unrecognized patch magic: {:02X?}",
        &buf[..buf.len().min(4)]
    ))
}

/// Apply a patch to a ROM buffer. Returns the patched bytes.
pub fn apply(patch: &Patch, base: &[u8]) -> Result<Vec<u8>, String> {
    match patch {
        Patch::Ips(p) => apply_ips(p, base),
        Patch::Ups(p) => apply_ups(p, base),
        Patch::Bps(p) => apply_bps(p, base),
    }
}

/// Convenience: load + parse + apply in one step.
pub fn apply_from_path(path: &std::path::Path, base: &[u8]) -> Result<Vec<u8>, String> {
    let raw = std::fs::read(path).map_err(|e| format!("read patch: {e}"))?;
    let patch = parse(raw)?;
    apply(&patch, base)
}

// ---------- IPS ----------

fn apply_ips(patch: &[u8], base: &[u8]) -> Result<Vec<u8>, String> {
    if patch.len() < 5 || &patch[0..5] != b"PATCH" {
        return Err("not an IPS patch (missing PATCH header)".into());
    }
    let mut out = base.to_vec();
    let mut i = 5usize;
    while i + 3 <= patch.len() {
        // Check for EOF marker (3 bytes "EOF").
        if &patch[i..i + 3] == b"EOF" {
            i += 3;
            // Optional 3-byte truncate-length follows EOF.
            if i + 3 <= patch.len() {
                let trunc = u24_be(&patch[i..i + 3]);
                if (trunc as usize) < out.len() {
                    out.truncate(trunc as usize);
                }
            }
            return Ok(out);
        }
        let offset = u24_be(&patch[i..i + 3]) as usize;
        i += 3;
        if i + 2 > patch.len() {
            return Err("IPS: truncated record header".into());
        }
        let size = u16_be(&patch[i..i + 2]) as usize;
        i += 2;
        if size == 0 {
            // RLE block: u16 BE count + u8 fill.
            if i + 3 > patch.len() {
                return Err("IPS: truncated RLE record".into());
            }
            let rle_size = u16_be(&patch[i..i + 2]) as usize;
            i += 2;
            let fill = patch[i];
            i += 1;
            ensure_size(&mut out, offset + rle_size);
            for byte in out[offset..offset + rle_size].iter_mut() {
                *byte = fill;
            }
        } else {
            if i + size > patch.len() {
                return Err("IPS: truncated data record".into());
            }
            ensure_size(&mut out, offset + size);
            out[offset..offset + size].copy_from_slice(&patch[i..i + size]);
            i += size;
        }
    }
    Err("IPS: ran off the end without finding EOF".into())
}

fn u24_be(b: &[u8]) -> u32 {
    ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | (b[2] as u32)
}

fn u16_be(b: &[u8]) -> u16 {
    ((b[0] as u16) << 8) | (b[1] as u16)
}

fn ensure_size(buf: &mut Vec<u8>, len: usize) {
    if buf.len() < len {
        buf.resize(len, 0);
    }
}

// ---------- UPS ----------

/// Variable-length quantity used by UPS and BPS. Each byte's low 7 bits
/// are data; the high bit terminates. UPS additionally adds 1 to every
/// continuation chunk (so the encoding is "self-correcting" against
/// repeated zeros).
fn read_vlq_ups(buf: &[u8], pos: &mut usize) -> Result<u64, String> {
    let mut val: u64 = 0;
    let mut shift: u32 = 1;
    loop {
        if *pos >= buf.len() {
            return Err("VLQ: ran off end".into());
        }
        let b = buf[*pos];
        *pos += 1;
        val += ((b & 0x7F) as u64).wrapping_mul(shift as u64);
        if b & 0x80 != 0 {
            break;
        }
        shift <<= 7;
        val += shift as u64;
    }
    Ok(val)
}

fn apply_ups(patch: &[u8], base: &[u8]) -> Result<Vec<u8>, String> {
    if patch.len() < 4 + 12 || &patch[0..4] != b"UPS1" {
        return Err("not a UPS patch (missing UPS1 header)".into());
    }
    // Body runs from 4 to (len - 12); the trailing 12 bytes are three
    // little-endian u32 CRCs (input / output / patch). We don't verify.
    let body_end = patch.len() - 12;
    let mut p = 4usize;
    let input_size = read_vlq_ups(&patch[..body_end], &mut p)? as usize;
    let output_size = read_vlq_ups(&patch[..body_end], &mut p)? as usize;
    let _ = input_size; // not enforced; we use base.len() as-is
    let mut out = base.to_vec();
    ensure_size(&mut out, output_size);
    let mut o = 0usize;
    while p < body_end {
        let rel = read_vlq_ups(&patch[..body_end], &mut p)? as usize;
        o += rel;
        // XOR bytes until a 0x00 terminator. The terminator itself is
        // also XORed in then increments the cursor past it.
        loop {
            if p >= body_end {
                return Err("UPS: ran off body in diff block".into());
            }
            let b = patch[p];
            p += 1;
            if o < out.len() {
                out[o] ^= b;
            }
            o += 1;
            if b == 0 {
                break;
            }
        }
    }
    out.truncate(output_size);
    Ok(out)
}

// ---------- BPS ----------

/// BPS's VLQ is similar to UPS but does NOT add 1 on continuation.
fn read_vlq_bps(buf: &[u8], pos: &mut usize) -> Result<u64, String> {
    let mut val: u64 = 0;
    let mut shift: u32 = 0;
    loop {
        if *pos >= buf.len() {
            return Err("BPS VLQ: ran off end".into());
        }
        let b = buf[*pos];
        *pos += 1;
        val |= ((b & 0x7F) as u64) << shift;
        if b & 0x80 != 0 {
            break;
        }
        shift += 7;
        val += 1u64 << shift;
    }
    Ok(val)
}

fn apply_bps(patch: &[u8], base: &[u8]) -> Result<Vec<u8>, String> {
    if patch.len() < 4 + 12 || &patch[0..4] != b"BPS1" {
        return Err("not a BPS patch (missing BPS1 header)".into());
    }
    let body_end = patch.len() - 12;
    let mut p = 4usize;
    let _src_size = read_vlq_bps(&patch[..body_end], &mut p)? as usize;
    let dst_size = read_vlq_bps(&patch[..body_end], &mut p)? as usize;
    let meta_size = read_vlq_bps(&patch[..body_end], &mut p)? as usize;
    p += meta_size; // skip metadata blob (UTF-8 free-form)
    let mut out = vec![0u8; dst_size];
    let mut out_pos = 0usize;
    let mut src_rel_offset: i64 = 0;
    let mut tgt_rel_offset: i64 = 0;
    while p < body_end && out_pos < dst_size {
        let cmd = read_vlq_bps(&patch[..body_end], &mut p)?;
        let opcode = cmd & 0b11;
        let length = ((cmd >> 2) + 1) as usize;
        match opcode {
            0 => {
                // SourceRead — copy `length` bytes from base[out_pos..]
                let end = out_pos + length;
                if end > dst_size {
                    return Err("BPS SourceRead overruns dst".into());
                }
                for i in 0..length {
                    let src_idx = out_pos + i;
                    out[out_pos + i] = *base.get(src_idx).unwrap_or(&0);
                }
                out_pos = end;
            }
            1 => {
                // TargetRead — copy `length` bytes from the patch stream
                if p + length > body_end {
                    return Err("BPS TargetRead overruns patch".into());
                }
                let end = out_pos + length;
                if end > dst_size {
                    return Err("BPS TargetRead overruns dst".into());
                }
                out[out_pos..end].copy_from_slice(&patch[p..p + length]);
                out_pos = end;
                p += length;
            }
            2 => {
                // SourceCopy — signed VLQ offset; advance src_rel_offset
                // by it, then copy `length` bytes from base[src_rel_offset..]
                let raw = read_vlq_bps(&patch[..body_end], &mut p)?;
                let signed = signed_offset(raw);
                src_rel_offset += signed;
                let end = out_pos + length;
                if end > dst_size {
                    return Err("BPS SourceCopy overruns dst".into());
                }
                for i in 0..length {
                    let idx = src_rel_offset + i as i64;
                    out[out_pos + i] = if idx >= 0 && (idx as usize) < base.len() {
                        base[idx as usize]
                    } else {
                        0
                    };
                }
                src_rel_offset += length as i64;
                out_pos = end;
            }
            3 => {
                // TargetCopy — signed VLQ offset on tgt cursor; copy
                // `length` bytes from out[tgt_rel_offset..]
                let raw = read_vlq_bps(&patch[..body_end], &mut p)?;
                let signed = signed_offset(raw);
                tgt_rel_offset += signed;
                let end = out_pos + length;
                if end > dst_size {
                    return Err("BPS TargetCopy overruns dst".into());
                }
                // Byte-at-a-time because tgt_rel_offset advances
                // alongside out_pos — overlap is intentional + spec'd.
                for _ in 0..length {
                    let idx = tgt_rel_offset as usize;
                    let v = if idx < out.len() { out[idx] } else { 0 };
                    out[out_pos] = v;
                    tgt_rel_offset += 1;
                    out_pos += 1;
                }
            }
            _ => unreachable!(),
        }
    }
    Ok(out)
}

/// BPS uses an unusual signed encoding: low bit = sign, rest = magnitude.
fn signed_offset(raw: u64) -> i64 {
    let mag = (raw >> 1) as i64;
    if raw & 1 == 1 { -mag } else { mag }
}

// Convenience for callers that have a `Read` instead of bytes.
#[allow(dead_code)] // future use — Tauri command may pass a stream
pub fn parse_from_reader(mut r: impl Read) -> Result<Patch, String> {
    let mut buf = Vec::new();
    r.read_to_end(&mut buf).map_err(|e| format!("read patch: {e}"))?;
    parse(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ips(records: &[u8]) -> Vec<u8> {
        let mut v = b"PATCH".to_vec();
        v.extend_from_slice(records);
        v.extend_from_slice(b"EOF");
        v
    }

    #[test]
    fn ips_simple_offset_data() {
        // Replace bytes at offset 4: 0xAA 0xBB.
        let patch = ips(&[
            0x00, 0x00, 0x04, // offset 4
            0x00, 0x02, // size 2
            0xAA, 0xBB,
        ]);
        let base = vec![0u8; 8];
        let out = apply_ips(&patch, &base).expect("apply");
        assert_eq!(out, vec![0, 0, 0, 0, 0xAA, 0xBB, 0, 0]);
    }

    #[test]
    fn ips_rle_block() {
        // RLE at offset 0: 5 bytes of 0xFF.
        let patch = ips(&[
            0x00, 0x00, 0x00, // offset 0
            0x00, 0x00, // size 0 → RLE
            0x00, 0x05, // 5 bytes
            0xFF,
        ]);
        let base = vec![0u8; 5];
        let out = apply_ips(&patch, &base).expect("apply");
        assert_eq!(out, vec![0xFF; 5]);
    }

    #[test]
    fn ips_extends_rom() {
        // Patch at offset beyond base length should grow the buffer.
        let patch = ips(&[
            0x00, 0x00, 0x05, // offset 5
            0x00, 0x02, // size 2
            0x11, 0x22,
        ]);
        let base = vec![0u8; 3];
        let out = apply_ips(&patch, &base).expect("apply");
        assert_eq!(out, vec![0, 0, 0, 0, 0, 0x11, 0x22]);
    }

    #[test]
    fn ips_rejects_missing_header() {
        let bogus = vec![b'N', b'O', b'P', b'E', 0, 0];
        let err = apply_ips(&bogus, &[]).err().unwrap();
        assert!(err.contains("PATCH"));
    }

    #[test]
    fn parse_detects_format() {
        assert!(matches!(parse(b"PATCHEOF".to_vec()), Ok(Patch::Ips(_))));
        assert!(matches!(parse(b"UPS1XXXXXXXXXXXXXXX".to_vec()), Ok(Patch::Ups(_))));
        assert!(matches!(parse(b"BPS1XXXXXXXXXXXXXXX".to_vec()), Ok(Patch::Bps(_))));
        assert!(parse(b"NOPE".to_vec()).is_err());
    }

    #[test]
    fn bps_target_read_only() {
        // Smallest valid BPS: src_size = dst_size = 4, meta_size = 0,
        // one TargetRead action for 4 bytes, then 3 dummy CRC32s.
        // VLQ encoding (BPS): N is encoded as (N-1) shifted with 7-bit
        // groups, top bit terminates. For small numbers like 0 / 4:
        //   - 0     → 0x80
        //   - 4     → 0x84
        //   - cmd for TargetRead len 4: opcode=1 + ((4-1) << 2) = 13 = 0x8D
        let mut p = b"BPS1".to_vec();
        p.push(0x84); // src_size = 4
        p.push(0x84); // dst_size = 4
        p.push(0x80); // meta_size = 0
        p.push(0x8D); // TargetRead length 4
        p.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]); // 4 target bytes
        p.extend_from_slice(&[0u8; 12]); // dummy crcs
        let out = apply_bps(&p, &[0xFF; 4]).expect("apply");
        assert_eq!(out, vec![0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn bps_source_read_passthrough() {
        // BPS that copies the base ROM verbatim via SourceRead.
        // cmd: opcode=0 + ((4-1)<<2) = 12 = 0x8C
        let mut p = b"BPS1".to_vec();
        p.push(0x84); // src
        p.push(0x84); // dst
        p.push(0x80); // meta
        p.push(0x8C); // SourceRead len 4
        p.extend_from_slice(&[0u8; 12]);
        let base = vec![0x11, 0x22, 0x33, 0x44];
        let out = apply_bps(&p, &base).expect("apply");
        assert_eq!(out, base);
    }

    #[test]
    fn signed_offset_round_trips() {
        assert_eq!(signed_offset(0), 0);
        assert_eq!(signed_offset(2), 1); // (1 << 1) | 0 = 2
        assert_eq!(signed_offset(3), -1); // (1 << 1) | 1 = 3
        assert_eq!(signed_offset(4), 2);
        assert_eq!(signed_offset(5), -2);
    }
}
