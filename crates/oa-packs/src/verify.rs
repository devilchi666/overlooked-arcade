//! sha256 integrity verification — the v1 trust anchor (content-packs.md §5).
//!
//! Cryptographic signing (minisign / sigstore) is explicitly deferred
//! (content-packs.md §11); for v1 the model is "OA team reviews registry
//! PRs; operator trusts the OA team," and the hash-from-registry check
//! catches transit corruption + a pack being swapped on its host after the
//! registry was last regenerated.

use sha2::{Digest, Sha256};

use crate::error::{PackError, Result};

/// Lowercase-hex sha256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for b in digest {
        // `{:02x}` — zero-padded lowercase hex, the form registries carry.
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// Verify `zip_bytes` hash to `expected_sha256`. Case- and
/// whitespace-insensitive on the expected side (registries occasionally
/// carry upper-case or padded hashes). Mismatch rejects — there is no
/// "close enough."
pub fn verify(zip_bytes: &[u8], expected_sha256: &str) -> Result<()> {
    let actual = sha256_hex(zip_bytes);
    let expected = expected_sha256.trim().to_ascii_lowercase();
    if actual != expected {
        return Err(PackError::Sha256Mismatch { expected, actual });
    }
    Ok(())
}
