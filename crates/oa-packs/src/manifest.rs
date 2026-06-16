//! The pack `manifest.yml` schema — the file at the top of every pack zip
//! (content-packs.md §6).
//!
//! Together with [`crate::registry::PackEntry`] this is the load-bearing
//! contract (CP2). On install OA cross-checks the four **identity** fields
//! (`id` / `version` / `pack_type` / `name`) against the registry entry
//! that authorized the install and refuses any disagreement — that's how a
//! swapped-content / wrong-pack zip is caught even when its bytes hash
//! correctly to *some* registry row.
//!
//! ## `name` vs the §6 sketch
//!
//! content-packs.md §6's example `manifest.yml` omits `name` while §6's
//! prose says install validates `name`. We resolve that here by making
//! `name` a **required identity field** of the manifest — we own the
//! schema (CP2), and a self-describing manifest that carries its own
//! display name is what makes the name-match check meaningful. Pack
//! authors must include it.

use serde::{Deserialize, Serialize};

/// Parsed `manifest.yml`. Identity fields (`id`/`version`/`pack_type`/
/// `name`) are required; everything else is optional metadata. Unknown
/// fields (e.g. a type-specific `contents:` block, intentionally not
/// modeled until a consumer needs it) are ignored, keeping the schema
/// additive-friendly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// Must equal the registry entry's `id`.
    pub id: String,

    /// Must equal the registry entry's `version`.
    pub version: String,

    /// Must equal the registry entry's `type`. Open string (CP3).
    #[serde(rename = "type")]
    pub pack_type: String,

    /// Must equal the registry entry's `name`. Required here even though
    /// the §6 sketch omitted it — see the module doc.
    pub name: String,

    // ---- Optional metadata below this line. ----
    #[serde(default)]
    pub maintainer: Option<String>,

    #[serde(default)]
    pub license: Option<String>,

    #[serde(default)]
    pub license_url: Option<String>,

    #[serde(default)]
    pub summary: Option<String>,

    #[serde(default)]
    pub homepage: Option<String>,

    /// A pack may also self-declare its minimum OA version. The registry
    /// entry's `min_oa_version` is authoritative, but when both are present
    /// the stricter one wins (enforced in [`crate::install`]).
    #[serde(default)]
    pub min_oa_version: Option<String>,
}

impl Manifest {
    /// Parse a `manifest.yml` from raw bytes.
    pub fn from_yaml_bytes(bytes: &[u8]) -> crate::Result<Self> {
        Ok(serde_yaml::from_slice(bytes)?)
    }
}
