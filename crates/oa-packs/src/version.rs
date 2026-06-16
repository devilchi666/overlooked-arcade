//! Dotted-numeric version comparison for the `min_oa_version` gate.
//!
//! We deliberately don't pull the `semver` crate: pack versions and the OA
//! version are plain `MAJOR.MINOR.PATCH` numbers, and the only question we
//! ever ask is "is the running OA at least this old?". A small comparator
//! keeps the crate dependency-light. Pre-release / build suffixes
//! (`-beta`, `+build`) are tolerated by comparing only the numeric core —
//! sufficient for the gate; revisit if real pre-release ordering ever
//! matters.

use std::cmp::Ordering;

use crate::error::{PackError, Result};

/// Parse the numeric core of a version string into its dotted segments.
/// `"1.2.3-beta+5"` → `[1, 2, 3]`.
fn parse_version(s: &str) -> Result<Vec<u64>> {
    let core = s.trim().split(['-', '+']).next().unwrap_or("");
    if core.is_empty() {
        return Err(PackError::InvalidVersion(s.to_string()));
    }
    let mut parts = Vec::new();
    for seg in core.split('.') {
        let n: u64 = seg
            .parse()
            .map_err(|_| PackError::InvalidVersion(s.to_string()))?;
        parts.push(n);
    }
    Ok(parts)
}

/// Compare two dotted-numeric versions. Shorter versions are zero-padded,
/// so `"1.2"` compares equal to `"1.2.0"`.
pub fn compare_versions(a: &str, b: &str) -> Result<Ordering> {
    let av = parse_version(a)?;
    let bv = parse_version(b)?;
    let len = av.len().max(bv.len());
    for i in 0..len {
        let x = av.get(i).copied().unwrap_or(0);
        let y = bv.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            Ordering::Equal => continue,
            non_eq => return Ok(non_eq),
        }
    }
    Ok(Ordering::Equal)
}

/// True when `running` is the same as or newer than `required` — i.e. the
/// `min_oa_version` gate passes.
pub fn version_at_least(running: &str, required: &str) -> Result<bool> {
    Ok(compare_versions(running, required)? != Ordering::Less)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_versions() {
        assert_eq!(compare_versions("1.2.3", "1.2.3").unwrap(), Ordering::Equal);
        // Zero-padding: trailing-zero segments don't change ordering.
        assert_eq!(compare_versions("1.2", "1.2.0").unwrap(), Ordering::Equal);
    }

    #[test]
    fn ordering_is_numeric_not_lexical() {
        // The classic lexical trap: "0.10" < "0.9" as strings, but 10 > 9.
        assert_eq!(compare_versions("0.10.0", "0.9.0").unwrap(), Ordering::Greater);
        assert_eq!(compare_versions("0.9.0", "0.10.0").unwrap(), Ordering::Less);
    }

    #[test]
    fn prerelease_suffix_is_ignored() {
        assert_eq!(
            compare_versions("1.0.0-beta", "1.0.0").unwrap(),
            Ordering::Equal
        );
    }

    #[test]
    fn gate_passes_when_equal_or_newer() {
        assert!(version_at_least("0.9.0", "0.9.0").unwrap());
        assert!(version_at_least("1.0.0", "0.9.0").unwrap());
        assert!(!version_at_least("0.8.0", "0.9.0").unwrap());
    }

    #[test]
    fn garbage_version_errors() {
        assert!(matches!(
            compare_versions("not-a-version", "1.0.0"),
            Err(PackError::InvalidVersion(_))
        ));
        assert!(matches!(parse_version(""), Err(PackError::InvalidVersion(_))));
    }
}
