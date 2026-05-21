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
    /// Byte-swap a copy of the ROM before hashing, IF `magic_at_offset_0`
    /// matches the first 4 bytes. Used for N64 .v64 (16-bit pair-swap)
    /// and .n64 (32-bit word-swap) → canonical .z64 BE normalization,
    /// so a .v64/.n64 dump produces the same SHA-1 as the catalog's
    /// .z64-keyed entry. Both the .v64 and .n64 magic conventions key
    /// off the .z64 magic `80 37 12 40` permuted by the corresponding
    /// swap order.
    ///
    /// Unlike `SkipMagic` / `SkipModulo` this rule is the only one that
    /// allocates — the swapped bytes can't be a slice borrow into the
    /// original buffer, so we materialize a fresh `Vec<u8>` for hashing.
    /// Cheap because we only swap when the magic check passes, and only
    /// when the system actually opts in (today: n64).
    ByteSwap {
        magic_at_offset_0: &'static [u8],
        swap: ByteSwapKind,
    },
}

/// The byte-order permutation applied by a `HeaderRule::ByteSwap`
/// candidate. Both kinds describe an in-place transformation on the
/// ROM bytes (with an allocated copy at hash time).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteSwapKind {
    /// Swap each adjacent byte pair: `[a, b, c, d]` → `[b, a, d, c]`.
    /// Used for N64 .v64 (a.k.a. "byteswapped") dumps where every
    /// 16-bit word is in the opposite byte order from the canonical
    /// .z64 Big-Endian convention.
    Pairs16,
    /// Reverse each 4-byte word: `[a, b, c, d]` → `[d, c, b, a]`.
    /// Used for N64 .n64 (a.k.a. "little-endian" or "Wii VC") dumps
    /// where every 32-bit word is in the opposite byte order from
    /// the canonical .z64 BE convention.
    Words32,
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
        // Nintendo 64. The no-intro dat keys against the canonical
        // Big-Endian .z64 SHA-1 (magic `80 37 12 40` at offset 0). A
        // .v64 dump (`37 80 40 12`) is the same ROM with each adjacent
        // byte-pair swapped; a .n64 dump (`40 12 37 80`) is the same
        // ROM with each 4-byte word reversed. Both magic checks miss
        // on a .z64 file (first byte is `0x80`, not `0x37`/`0x40`), so
        // only the matching swap rule contributes a candidate per file.
        "n64" => &[
            Raw,
            ByteSwap {
                magic_at_offset_0: &[0x37, 0x80, 0x40, 0x12],
                swap: ByteSwapKind::Pairs16,
            },
            ByteSwap {
                magic_at_offset_0: &[0x40, 0x12, 0x37, 0x80],
                swap: ByteSwapKind::Words32,
            },
        ],
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
        // Two output paths: slice-borrow rules produce `Some(&[u8])`
        // and hash the borrowed view; `ByteSwap` allocates an owned
        // `Vec<u8>` because the swap can't be expressed as a slice
        // into the source bytes. Either path lands at the same
        // `Sha1Candidate` shape.
        let mut owned: Option<Vec<u8>> = None;
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
            HeaderRule::ByteSwap { magic_at_offset_0, swap } => {
                if bytes.len() >= magic_at_offset_0.len()
                    && &bytes[..magic_at_offset_0.len()] == magic_at_offset_0
                {
                    owned = Some(byte_swap(bytes, swap));
                }
                None
            }
        };
        let bytes_to_hash: Option<&[u8]> = view.or(owned.as_deref());
        if let Some(view) = bytes_to_hash {
            let mut h = Sha1::new();
            h.update(view);
            out.push(Sha1Candidate { rule: *rule, sha1: format!("{:x}", h.finalize()) });
        }
    }
    out
}

/// Apply a byte-swap permutation to a copy of the ROM. Used by
/// `HeaderRule::ByteSwap` to produce a hash-only view of N64 .v64/.n64
/// dumps that matches the canonical .z64 BE catalog SHA-1.
///
/// Both variants ignore trailing bytes that don't fit a complete
/// pair/word — N64 ROMs are always word-aligned in practice (they're
/// power-of-two sized), so the trim is a defensive no-op rather than
/// load-bearing. Tail bytes pass through unchanged so the swapped
/// buffer length matches the source length.
fn byte_swap(bytes: &[u8], swap: ByteSwapKind) -> Vec<u8> {
    match swap {
        ByteSwapKind::Pairs16 => {
            let mut out = bytes.to_vec();
            for chunk in out.chunks_exact_mut(2) {
                chunk.swap(0, 1);
            }
            out
        }
        ByteSwapKind::Words32 => {
            let mut out = bytes.to_vec();
            for chunk in out.chunks_exact_mut(4) {
                chunk.swap(0, 3);
                chunk.swap(1, 2);
            }
            out
        }
    }
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

    /// Canonical N64 .z64 ROM (Big-Endian magic 80 37 12 40) — should
    /// produce only the Raw candidate. Neither v64 nor n64 swap rule
    /// fires because the file's magic doesn't match the swapped
    /// patterns.
    #[test]
    fn n64_z64_rom_produces_only_raw_candidate() {
        let mut bytes = vec![0x80, 0x37, 0x12, 0x40]; // .z64 magic
        bytes.extend_from_slice(&[0x42u8; 1024 * 1024]);
        let candidates = candidate_sha1s(&bytes, "n64");
        assert_eq!(candidates.len(), 1, "expected only Raw — file is canonical .z64");
        assert_eq!(candidates[0].rule, HeaderRule::Raw);
    }

    /// N64 .v64 ROM (pair-swapped magic 37 80 40 12) — should produce
    /// Raw + Pairs16 candidates. The Pairs16 candidate's hash should
    /// match what you'd get from hashing the BE-normalized form
    /// directly.
    #[test]
    fn n64_v64_rom_produces_raw_plus_pairs16_candidate() {
        // Synthesize a .z64 ROM, then permute each pair to produce the
        // matching .v64 form. Hashing the .v64 ROM with the Pairs16
        // candidate should equal hashing the original .z64 buffer.
        let mut z64 = vec![0x80, 0x37, 0x12, 0x40];
        z64.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD, 0x11, 0x22, 0x33, 0x44]);
        z64.extend_from_slice(&[0u8; 1024 * 1024 - 12]);

        let mut v64 = z64.clone();
        for chunk in v64.chunks_exact_mut(2) {
            chunk.swap(0, 1);
        }
        // Sanity-check the .v64 magic is now the pair-swapped form.
        assert_eq!(&v64[..4], &[0x37, 0x80, 0x40, 0x12]);

        let candidates = candidate_sha1s(&v64, "n64");
        assert_eq!(candidates.len(), 2, "expected Raw + Pairs16 byte-swap");
        assert_eq!(candidates[0].rule, HeaderRule::Raw);
        assert!(matches!(
            candidates[1].rule,
            HeaderRule::ByteSwap { swap: ByteSwapKind::Pairs16, .. }
        ));

        // The byte-swap candidate's hash should match a fresh hash of
        // the original .z64 buffer (round-trip correctness).
        let mut h = Sha1::new();
        h.update(&z64);
        let z64_sha1 = format!("{:x}", h.finalize());
        assert_eq!(
            candidates[1].sha1, z64_sha1,
            "v64 → Pairs16 hash should equal hashing the canonical .z64 form"
        );
    }

    /// N64 .n64 ROM (word-swapped magic 40 12 37 80) — should produce
    /// Raw + Words32 candidates. The Words32 candidate's hash should
    /// match the BE-normalized form.
    #[test]
    fn n64_n64_rom_produces_raw_plus_words32_candidate() {
        let mut z64 = vec![0x80, 0x37, 0x12, 0x40];
        z64.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD, 0x11, 0x22, 0x33, 0x44]);
        z64.extend_from_slice(&[0u8; 1024 * 1024 - 12]);

        let mut n64 = z64.clone();
        for chunk in n64.chunks_exact_mut(4) {
            chunk.swap(0, 3);
            chunk.swap(1, 2);
        }
        // Sanity-check the .n64 magic is now the word-swapped form.
        assert_eq!(&n64[..4], &[0x40, 0x12, 0x37, 0x80]);

        let candidates = candidate_sha1s(&n64, "n64");
        assert_eq!(candidates.len(), 2, "expected Raw + Words32 byte-swap");
        assert_eq!(candidates[0].rule, HeaderRule::Raw);
        assert!(matches!(
            candidates[1].rule,
            HeaderRule::ByteSwap { swap: ByteSwapKind::Words32, .. }
        ));

        let mut h = Sha1::new();
        h.update(&z64);
        let z64_sha1 = format!("{:x}", h.finalize());
        assert_eq!(
            candidates[1].sha1, z64_sha1,
            "n64 → Words32 hash should equal hashing the canonical .z64 form"
        );
    }

    /// Truncated N64 file (smaller than the 4-byte magic) must not
    /// panic — drops both byte-swap rules cleanly.
    #[test]
    fn n64_truncated_file_below_magic_doesnt_panic() {
        let bytes = vec![0x37u8]; // 1 byte, below the 4-byte magic
        let candidates = candidate_sha1s(&bytes, "n64");
        // Raw still produces a candidate; both ByteSwap rules drop.
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].rule, HeaderRule::Raw);
    }

    /// N64 rule set must not fire byte-swap rules against a non-N64
    /// header (rare in practice — system-id mismatch would already be
    /// a bug — but defensive against accidental misclassification).
    #[test]
    fn n64_rule_set_skips_byte_swap_for_unrelated_magic() {
        let mut bytes = vec![0x4Eu8, 0x45, 0x53, 0x1A]; // iNES magic, not N64
        bytes.extend_from_slice(&[0u8; 1024]);
        let candidates = candidate_sha1s(&bytes, "n64");
        // Neither swap magic matches → only Raw remains.
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].rule, HeaderRule::Raw);
    }
}
