//! `oa-packs` — the pure core of Overlooked Arcade's content-pack
//! distribution channel.
//!
//! One operator-initiated mechanism distributes every optional pack-shaped
//! payload OA will ever offer — editorial DISCOVER content, emulator
//! recipes, themes, per-system asset bundles, cheats, metadata — so each
//! future pack type rides one foundation instead of N bespoke updaters.
//! See [`docs/PLANS/content-packs.md`] for the locked design and
//! [`docs/PLANS/oa-packs-infrastructure.md`] for the slice roadmap +
//! decisions CP1–CP5.
//!
//! ## What lives here (and what doesn't)
//!
//! This crate is **pure**: schema types, sha256 verification, manifest
//! validation, and install-from-a-local-zip. **No network. No Tauri.** The
//! download half already exists in the shell (`apps/oa-shell`'s
//! `core_installer.rs` + `http_retry.rs`) and is reused by the fetch slice
//! (Slice 2), which hands verified bytes to this crate. Keeping the trust
//! logic I/O-light (a temp dir is the only filesystem it touches) is what
//! makes it exhaustively unit-testable.
//!
//! ## The contract
//!
//! [`Registry`]/[`PackEntry`] (`registry.json`) and [`Manifest`]
//! (`manifest.yml`) together ARE the contract between OA and every pack
//! author (CP2). The early lock-in risk is these schemas + the on-disk
//! layout, *not* hosting — so they're additive-friendly (optional fields,
//! unknown fields ignored) and `type` is an open string dispatched at
//! install/load time (CP3), never a closed enum.
//!
//! ## Slice 1 surface
//!
//! - [`verify`] — sha256 the bytes against the registry hash; mismatch
//!   rejects (the v1 trust anchor).
//! - [`validate_manifest_against_registry`] — id/version/type/name must
//!   match + the `min_oa_version` gate.
//! - [`install_from_local_zip`] — verify → stage → validate → atomic move
//!   into `<dest_root>/<type>/community/<pack_id>/`.
//! - [`PackTypeSpec`] / [`baseline_for_type`] — model "has a bundled
//!   baseline" as a per-type property (CP4), not a global "zero builtin".

mod error;
mod install;
mod manifest;
mod registry;
mod verify;
mod version;

#[cfg(test)]
mod tests;

pub use error::{PackError, Result};
pub use install::{
    baseline_for_type, community_pack_dir, default_pack_type_specs,
    install_from_local_zip, validate_manifest_against_registry, PackTypeSpec,
};
pub use manifest::Manifest;
pub use registry::{PackEntry, Registry};
pub use verify::{sha256_hex, verify};
pub use version::{compare_versions, version_at_least};
