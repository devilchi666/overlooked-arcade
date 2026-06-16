//! Error type for the pack pipeline.
//!
//! Every refusal the trust model makes (bad hash, manifest disagreeing
//! with its registry entry, an OA that's too old for the pack) surfaces
//! as a distinct [`PackError`] variant so callers — and tests — can
//! assert *why* an install was rejected, not just that it was.

use std::path::PathBuf;

/// All the ways pack verification / validation / install can fail.
#[derive(Debug, thiserror::Error)]
pub enum PackError {
    /// The downloaded bytes don't hash to the registry-listed sha256.
    /// The trust anchor (content-packs.md §5) — a mismatch always rejects.
    #[error("sha256 mismatch: expected {expected}, computed {actual}")]
    Sha256Mismatch { expected: String, actual: String },

    /// A required identity field on the in-zip `manifest.yml` disagrees
    /// with the registry entry that authorized the install
    /// (content-packs.md §6 — id/version/type/name must match).
    #[error("manifest field `{field}` disagrees with registry: registry={registry:?}, manifest={manifest:?}")]
    ManifestMismatch {
        field: &'static str,
        registry: String,
        manifest: String,
    },

    /// The running OA build is older than the pack's `min_oa_version` gate.
    #[error("pack requires OA >= {required}, but this build is {running}")]
    OaVersionTooOld { required: String, running: String },

    /// A version string couldn't be parsed for comparison.
    #[error("invalid version string: {0:?}")]
    InvalidVersion(String),

    /// The pack zip had no top-level `manifest.yml`.
    #[error("pack zip is missing a top-level manifest.yml")]
    ManifestMissing,

    /// A zip entry tried to escape the extraction root (zip-slip / `..`).
    #[error("pack zip contains an unsafe path that escapes the extract root: {0:?}")]
    UnsafeZipPath(String),

    /// The destination pack dir already exists and couldn't be cleared
    /// for an atomic replace.
    #[error("could not clear existing install at {0}: {1}")]
    DestinationBusy(PathBuf, String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("manifest parse error: {0}")]
    ManifestParse(#[from] serde_yaml::Error),
}

/// Crate-wide result alias.
pub type Result<T> = std::result::Result<T, PackError>;
