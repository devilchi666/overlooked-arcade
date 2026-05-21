//! Per-system ROM header handling. Catalogs (No-Intro / Redump /
//! libretro-database) hash the canonical *unheadered* ROM, but files in
//! the wild often carry a header (iNES, SMC, LNX, A78, etc.). For each
//! system we produce 1+ candidate sha1s — raw + stripped — and the
//! lookup loop tries each.
//!
//! ## New-core onboarding checklist
//!
//! When onboarding a system whose ROMs may ship with a header convention,
//! add an arm to `header_rules_for`. Systems whose ROMs are always
//! headerless (Lynx via libretro typically ships raw, GB/GBA, SMS/GG,
//! WonderSwan, etc.) get the default `&[Raw]` fallback — no change needed.
//!
//! The added cost of a wrong rule is one extra sha1 computation per
//! candidate per ROM during the resolve pass. Cheap, so when in doubt
//! include the strip variant.

use sha1::{Digest, Sha1};

/// A transform producing one candidate-hash view of a ROM's bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderRule {
    /// Hash the file as-is. Always present (first) in every system's rule set.
    Raw,
    /// Skip the first `skip` bytes IF `magic_bytes` appears at
    /// `magic_offset` in the file. Pattern: iNES (NES), LNX (Lynx), A78
    /// (Atari 7800) — the header carries a known signature so presence is
    /// directly detectable.
    SkipMagic {
        skip: usize,
        magic_offset: usize,
        magic_bytes: &'static [u8],
    },
    /// Skip the first `skip` bytes IF `file_size % modulus == skip`.
    /// Pattern: SMC (SNES) — no header signature, presence inferred from
    /// the file's size mod power-of-two.
    SkipModulo { skip: usize, modulus: usize },
}

/// Header rules per system. Order matters only for logging — the lookup
/// tries every candidate and the first DB hit wins. The default `&[Raw]`
/// is fine for systems with no header convention.
pub fn header_rules_for(system_id: &str) -> &'static [HeaderRule] {
    use HeaderRule::*;
    match system_id {
        "nes" => &[
            Raw,
            SkipMagic { skip: 16, magic_offset: 0, magic_bytes: b"NES\x1a" },
        ],
        "snes" => &[Raw, SkipModulo { skip: 512, modulus: 1024 }],
        "lynx" => &[
            Raw,
            SkipMagic { skip: 64, magic_offset: 0, magic_bytes: b"LYNX" },
        ],
        "atari7800" => &[
            Raw,
            // A78 magic is "ATARI7800" at offset 1 (offset 0 is the
            // header-version byte; the spec keeps the magic one byte in).
            SkipMagic { skip: 128, magic_offset: 1, magic_bytes: b"ATARI7800" },
        ],
        "tg16" => &[Raw, SkipModulo { skip: 512, modulus: 1024 }],
        _ => &[Raw],
    }
}

/// One candidate produced by applying a header rule to the file bytes.
/// Carrying the rule alongside the sha1 lets the lookup loop log which
/// transform produced a hit ("matched via SkipMagic{16}") — useful when a
/// user's library is mostly headered and we want to surface that fact.
#[derive(Debug, Clone)]
pub struct Sha1Candidate {
    pub rule: HeaderRule,
    pub sha1: String,
}

/// Produce 1+ candidate sha1s for the ROM bytes under each header rule.
/// Returns at minimum the `Raw` candidate (rules guarantee `Raw` first).
/// Skip variants that don't apply (magic absent / size mismatch) are
/// silently dropped.
pub fn candidate_sha1s(bytes: &[u8], system_id: &str) -> Vec<Sha1Candidate> {
    let mut out = Vec::with_capacity(2);
    for rule in header_rules_for(system_id) {
        let view: Option<&[u8]> = match *rule {
            HeaderRule::Raw => Some(bytes),
            HeaderRule::SkipMagic { skip, magic_offset, magic_bytes } => {
                let end = magic_offset.saturating_add(magic_bytes.len());
                if bytes.len() >= end
                    && &bytes[magic_offset..end] == magic_bytes
                    && bytes.len() > skip
                {
                    Some(&bytes[skip..])
                } else {
                    None
                }
            }
            HeaderRule::SkipModulo { skip, modulus } => {
                if bytes.len() > skip && bytes.len() % modulus == skip {
                    Some(&bytes[skip..])
                } else {
                    None
                }
            }
        };
        if let Some(view) = view {
            let mut h = Sha1::new();
            h.update(view);
            out.push(Sha1Candidate { rule: *rule, sha1: format!("{:x}", h.finalize()) });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_system_only_produces_raw() {
        let bytes = vec![0u8; 4096];
        let candidates = candidate_sha1s(&bytes, "wonderswan");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].rule, HeaderRule::Raw);
    }

    #[test]
    fn ines_headered_nes_rom_produces_raw_and_stripped() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"NES\x1a"); // iNES magic
        bytes.extend_from_slice(&[0u8; 12]); // remainder of 16-byte header
        bytes.extend_from_slice(&[0x42u8; 16 * 1024]); // fake PRG
        let candidates = candidate_sha1s(&bytes, "nes");
        assert_eq!(candidates.len(), 2, "expected raw + iNES-stripped");
        assert_eq!(candidates[0].rule, HeaderRule::Raw);
        assert!(matches!(
            candidates[1].rule,
            HeaderRule::SkipMagic { skip: 16, .. }
        ));
        assert_ne!(candidates[0].sha1, candidates[1].sha1);
    }

    #[test]
    fn unheadered_nes_rom_produces_one_candidate() {
        // No iNES magic at offset 0 → SkipMagic is dropped; only Raw remains.
        let bytes = vec![0x42u8; 16 * 1024];
        let candidates = candidate_sha1s(&bytes, "nes");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].rule, HeaderRule::Raw);
    }

    #[test]
    fn smc_headered_snes_rom_detected_by_size_modulo() {
        // 512-byte SMC header + 1 MB ROM → size % 1024 == 512.
        let bytes = vec![0u8; 512 + 1024 * 1024];
        let candidates = candidate_sha1s(&bytes, "snes");
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].rule, HeaderRule::Raw);
        assert!(matches!(
            candidates[1].rule,
            HeaderRule::SkipModulo { skip: 512, modulus: 1024 }
        ));
    }

    #[test]
    fn unheadered_snes_rom_skipmodulo_drops() {
        // Pure 1 MB ROM, no SMC header → size % 1024 == 0, not 512.
        let bytes = vec![0u8; 1024 * 1024];
        let candidates = candidate_sha1s(&bytes, "snes");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].rule, HeaderRule::Raw);
    }

    #[test]
    fn a78_headered_atari7800_rom_produces_two_candidates() {
        // A78 header: byte 0 = version, bytes 1..10 = "ATARI7800", then
        // padding up to 128 bytes total.
        let mut bytes = vec![0u8; 128];
        bytes[0] = 1;
        bytes[1..10].copy_from_slice(b"ATARI7800");
        bytes.extend_from_slice(&[0x42u8; 48 * 1024]); // fake cart body
        let candidates = candidate_sha1s(&bytes, "atari7800");
        assert_eq!(candidates.len(), 2);
        assert!(matches!(
            candidates[1].rule,
            HeaderRule::SkipMagic { skip: 128, magic_offset: 1, .. }
        ));
    }

    #[test]
    fn lynx_headered_lnx_rom_produces_two_candidates() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"LYNX");      // LNX magic
        bytes.extend_from_slice(&[0u8; 60]);    // remainder of 64-byte header
        bytes.extend_from_slice(&[0x77u8; 256 * 1024]);
        let candidates = candidate_sha1s(&bytes, "lynx");
        assert_eq!(candidates.len(), 2);
        assert!(matches!(
            candidates[1].rule,
            HeaderRule::SkipMagic { skip: 64, .. }
        ));
    }

    #[test]
    fn truncated_file_below_skip_doesnt_panic() {
        // File too small to apply the strip — must drop the variant
        // rather than panic on slicing.
        let bytes = b"NES\x1a".to_vec(); // exactly 4 bytes, < 16
        let candidates = candidate_sha1s(&bytes, "nes");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].rule, HeaderRule::Raw);
    }

    #[test]
    fn raw_candidate_sha1_matches_direct_sha1() {
        // The Raw candidate must produce the same sha1 the existing
        // sha1_of_rom would produce on the bytes — otherwise we'd break
        // every already-stamped row in the DB.
        let bytes = b"abc"; // sha1 = a9993e364706816aba3e25717850c26c9cd0d89d
        let candidates = candidate_sha1s(bytes, "wonderswan");
        assert_eq!(candidates[0].sha1, "a9993e364706816aba3e25717850c26c9cd0d89d");
    }
}
