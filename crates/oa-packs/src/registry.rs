//! The registry schema — `registry.json`, one OA-curated list of every
//! installable pack (content-packs.md §4).
//!
//! **This type, together with [`crate::manifest::Manifest`], IS the
//! contract** between OA and every pack author (decision CP2). Changing a
//! required field later churns every already-published pack, so the design
//! care goes here. Two anti-lock-in postures are baked in:
//!
//! - **`pack_type` is an open `String`, never a closed enum** (CP3). New
//!   pack kinds (emulator-recipes, themes, cheats, metadata, per-system
//!   assets) slot in as new `type` values + a dispatch arm — additive
//!   data, never a schema break.
//! - **Unknown fields are ignored, not rejected.** No `deny_unknown_fields`
//!   here: a newer registry carrying a future field (e.g. the reserved
//!   `source` federation field, content-packs.md §4) still parses on an
//!   older OA. Forward-compatible by default.

use serde::{Deserialize, Serialize};

/// Top-level `registry.json` shape. Slice 1 never fetches this — the type
/// exists so the fetch slice (Slice 2) deserializes into a shared contract
/// rather than reinventing one.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {
    /// Schema version of the registry document itself. Bumped only on a
    /// breaking registry-envelope change (not on adding a pack `type`).
    pub registry_version: u32,
    /// ISO-8601 timestamp the registry was last regenerated. Optional so a
    /// hand-authored registry need not carry it.
    #[serde(default)]
    pub updated: Option<String>,
    /// Every pack OA knows how to offer. Defaults to empty so a malformed
    /// or partial registry still deserializes for diagnostics.
    #[serde(default)]
    pub packs: Vec<PackEntry>,
}

/// One installable pack, as listed in the registry. The `sha256` field is
/// the integrity trust anchor (content-packs.md §5); `url` is pinned to a
/// specific release and never moves.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackEntry {
    /// Globally unique pack id, slug form (e.g. `oa-editorial-baseline`).
    /// Doubles as the on-disk directory name under `community/`.
    pub id: String,

    /// Pack kind — an OPEN string, dispatched on at install/load time
    /// (CP3). Known values today: `editorial`, `emulator-recipes`; future:
    /// `theme`, `system-ui-assets`, `cheats`, `metadata`, … Adding one is
    /// data, not a schema change.
    #[serde(rename = "type")]
    pub pack_type: String,

    /// Human-readable display name. Operator-visible; must match the
    /// in-zip manifest's `name` on install.
    pub name: String,

    /// Semver. Used for update detection (Slice 3) and matched against the
    /// manifest on install.
    pub version: String,

    /// Direct download URL for the pack zip. Pinned to a release asset.
    /// Slice 1 ignores this (local-zip install only); Slice 2 fetches it.
    pub url: String,

    /// Expected sha256 (hex) of the downloaded zip. The integrity trust
    /// anchor — a download whose bytes don't match this is rejected.
    pub sha256: String,

    // ---- Optional / additive-friendly fields below this line. ----
    /// Uncompressed-or-not size of the zip, for progress + a sanity check.
    #[serde(default)]
    pub size_bytes: Option<u64>,

    /// Other pack ids this pack needs present to load. Install ordering
    /// follows the DAG (resolved in a later slice). Empty by default.
    #[serde(default)]
    pub depends_on: Vec<String>,

    /// Refuse to install if the running OA is older than this (semver).
    /// Gate enforced in [`crate::install`].
    #[serde(default)]
    pub min_oa_version: Option<String>,

    /// SPDX-ish license string. Operator-visible so they know what they're
    /// consuming. Optional in the type, but the OA registry PR review
    /// requires it in practice.
    #[serde(default)]
    pub license: Option<String>,

    /// Project / source homepage.
    #[serde(default)]
    pub homepage: Option<String>,

    /// One-line summary for the Available-packs list.
    #[serde(default)]
    pub summary: Option<String>,

    /// Maintainer handle (e.g. the GitHub org/user).
    #[serde(default)]
    pub maintainer: Option<String>,
}
