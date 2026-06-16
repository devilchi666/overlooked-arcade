//! Validate + install a pack from a local zip — the offline half of the
//! pipeline (content-packs.md §5–§8 minus the download). Slice 2 hands this
//! crate the bytes it fetched; Slice 1 reads them straight off disk.
//!
//! The install is **verify → stage → validate → atomic move**, in that
//! order, so the destination only ever sees a fully-checked pack:
//!
//! 1. Read the zip + sha256-verify against the registry entry (§5).
//! 2. Extract into a staging dir on the *same volume* as the destination.
//! 3. Parse `manifest.yml` + cross-check it against the registry entry,
//!    plus the `min_oa_version` gate (§6).
//! 4. Atomically `rename` staging → `<dest_root>/<type>/community/<id>/`.
//!
//! Any failure before step 4 removes the staging dir and leaves **no
//! partial directory** at the destination — the destination is untouched
//! until the single atomic rename.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

use crate::error::{PackError, Result};
use crate::manifest::Manifest;
use crate::registry::PackEntry;
use crate::version::version_at_least;

/// Per-pack-type install/load policy (decision CP4).
///
/// `pack_type` is an OPEN string (CP3); this is plain data, not a closed
/// enum. `has_bundled_baseline` records whether the type ships a working
/// baseline *inside the OA install* that community packs then **override**
/// (emulator-recipes: the bundled `config/emulators/*.yaml`), versus being
/// empty until a pack is installed (editorial: DISCOVER is blank until you
/// install a pack).
///
/// Slice 1 doesn't load anything, so the flag changes no install path here
/// — community packs always land under `community/<id>/`. It exists so the
/// layout doesn't bake in editorial's "zero builtin" assumption: the loader
/// (a later slice) reads this to decide whether to load a baseline tier
/// *beneath* `community/`. Modeling it now is the whole point of CP4.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackTypeSpec {
    pub pack_type: String,
    pub has_bundled_baseline: bool,
}

impl PackTypeSpec {
    pub fn new(pack_type: impl Into<String>, has_bundled_baseline: bool) -> Self {
        Self {
            pack_type: pack_type.into(),
            has_bundled_baseline,
        }
    }
}

/// Seed specs for the pack types known today. This is a **default seed,
/// not a closed set** — callers may supply their own, and an unknown type
/// is not an error (CP3). Use [`baseline_for_type`] to resolve a type's
/// baseline policy with the right default for unknowns.
pub fn default_pack_type_specs() -> Vec<PackTypeSpec> {
    vec![
        // Emulator recipes ship bundled in the install and treat pack
        // updates as an override of a working baseline (CP4/CP5).
        PackTypeSpec::new("emulator-recipes", true),
        // Editorial / DISCOVER content is empty until a pack is installed.
        PackTypeSpec::new("editorial", false),
    ]
}

/// Whether `pack_type` has a bundled baseline. Unknown types default to
/// `false` (no baseline) — the safe, additive default: a brand-new type is
/// empty-until-installed unless it explicitly opts into a baseline.
pub fn baseline_for_type(specs: &[PackTypeSpec], pack_type: &str) -> bool {
    specs
        .iter()
        .find(|s| s.pack_type == pack_type)
        .map(|s| s.has_bundled_baseline)
        .unwrap_or(false)
}

/// On-disk install directory for a pack:
/// `<dest_root>/<type>/community/<pack_id>/` (content-packs.md §7). The
/// `community/` tier is where every installed pack lands regardless of
/// whether the type also has a bundled baseline — baseline lives elsewhere
/// (e.g. recipes' baseline is the install's `config/emulators/`), and the
/// loader layers them.
pub fn community_pack_dir(dest_root: &Path, pack_type: &str, pack_id: &str) -> PathBuf {
    dest_root
        .join(pack_type)
        .join("community")
        .join(pack_id)
}

/// Cross-check a parsed manifest against the registry entry that
/// authorized the install, then apply the `min_oa_version` gate. The four
/// identity fields must match exactly; a disagreement means the zip is not
/// the pack the registry described (swapped content, wrong release) and is
/// refused (content-packs.md §6).
///
/// `running_oa_version` is threaded in by the caller — the crate has no
/// knowledge of the app version (it's not a constant here).
pub fn validate_manifest_against_registry(
    manifest: &Manifest,
    entry: &PackEntry,
    running_oa_version: &str,
) -> Result<()> {
    check_field("id", &entry.id, &manifest.id)?;
    check_field("version", &entry.version, &manifest.version)?;
    check_field("type", &entry.pack_type, &manifest.pack_type)?;
    check_field("name", &entry.name, &manifest.name)?;

    // min_oa_version gate. The registry entry is authoritative; if the
    // manifest also declares one, the stricter (higher) bound wins so a
    // pack can't weaken its own floor via a laxer registry row.
    for required in [entry.min_oa_version.as_deref(), manifest.min_oa_version.as_deref()]
        .into_iter()
        .flatten()
    {
        if !version_at_least(running_oa_version, required)? {
            return Err(PackError::OaVersionTooOld {
                required: required.to_string(),
                running: running_oa_version.to_string(),
            });
        }
    }
    Ok(())
}

fn check_field(field: &'static str, registry: &str, manifest: &str) -> Result<()> {
    if registry != manifest {
        return Err(PackError::ManifestMismatch {
            field,
            registry: registry.to_string(),
            manifest: manifest.to_string(),
        });
    }
    Ok(())
}

/// Install a pack from a local zip at `zip_path`, authorized by `entry`,
/// into `<dest_root>/<type>/community/<pack_id>/`. No network. Returns the
/// final install directory on success.
///
/// On any failure the staging dir is removed and the destination is left
/// untouched (no partial install).
pub fn install_from_local_zip(
    zip_path: &Path,
    entry: &PackEntry,
    dest_root: &Path,
    running_oa_version: &str,
) -> Result<PathBuf> {
    // --- 1. Read + verify the zip bytes against the registry sha256. ---
    let zip_bytes = std::fs::read(zip_path)?;
    crate::verify::verify(&zip_bytes, &entry.sha256)?;

    // --- 2. Stage into a sibling dir under dest_root so the final move is
    //        a same-volume rename (cross-volume rename fails on Windows). ---
    let final_dir = community_pack_dir(dest_root, &entry.pack_type, &entry.id);
    let staging = StagingDir::create(dest_root, &entry.id)?;
    extract_zip_to(zip_path, staging.path())?;

    // --- 3. Parse + validate the manifest. Any error here drops out with
    //        the StagingDir guard cleaning up — destination untouched. ---
    let manifest_path = staging.path().join("manifest.yml");
    if !manifest_path.is_file() {
        return Err(PackError::ManifestMissing);
    }
    let manifest = Manifest::from_yaml_bytes(&std::fs::read(&manifest_path)?)?;
    validate_manifest_against_registry(&manifest, entry, running_oa_version)?;

    // --- 4. Atomic move into place. Clear any prior install first (an
    //        update replaces in full), then rename staging → final. The
    //        rename is the single moment the destination changes. ---
    if let Some(parent) = final_dir.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if final_dir.exists() {
        std::fs::remove_dir_all(&final_dir)
            .map_err(|e| PackError::DestinationBusy(final_dir.clone(), e.to_string()))?;
    }
    std::fs::rename(staging.path(), &final_dir)?;
    // The rename consumed the staging dir; disarm the cleanup guard so its
    // Drop doesn't try to remove the now-moved (and now-final) directory.
    staging.disarm();

    Ok(final_dir)
}

/// A staging directory that removes itself on drop unless [`disarm`]ed.
/// This is what guarantees "no partial dir on failure": every early return
/// between create and the final rename runs this Drop.
///
/// [`disarm`]: StagingDir::disarm
struct StagingDir {
    path: PathBuf,
    armed: bool,
}

impl StagingDir {
    /// Create `<dest_root>/.staging/<pack_id>-<unique>/`. Staging lives
    /// under `dest_root` (not the OS temp dir) so the final step is a
    /// same-volume rename.
    fn create(dest_root: &Path, pack_id: &str) -> Result<Self> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        // Process id + a monotonic counter — unique within and across
        // concurrent installs without pulling a uuid/rand dependency.
        let unique = format!(
            "{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, AtomicOrdering::Relaxed)
        );
        let path = dest_root
            .join(".staging")
            .join(format!("{pack_id}-{unique}"));
        // Fresh dir — clear any leftover from a crashed prior run.
        if path.exists() {
            std::fs::remove_dir_all(&path)?;
        }
        std::fs::create_dir_all(&path)?;
        Ok(Self { path, armed: true })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    /// Give up ownership of the dir — call after a successful rename so the
    /// guard doesn't delete the now-final install.
    fn disarm(mut self) {
        self.armed = false;
    }
}

impl Drop for StagingDir {
    fn drop(&mut self) {
        if self.armed && self.path.exists() {
            if let Err(e) = std::fs::remove_dir_all(&self.path) {
                log::warn!(
                    "oa-packs: failed to clean staging dir {}: {e}",
                    self.path.display()
                );
            }
        }
    }
}

/// Extract every entry of the zip at `zip_path` into `dest`, refusing any
/// entry whose path escapes `dest` (zip-slip / `..` traversal).
fn extract_zip_to(zip_path: &Path, dest: &Path) -> Result<()> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        // `enclosed_name` returns None for any path that would escape the
        // root — our zip-slip guard.
        let Some(rel) = entry.enclosed_name() else {
            return Err(PackError::UnsafeZipPath(entry.name().to_string()));
        };
        let out_path = dest.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = std::fs::File::create(&out_path)?;
            std::io::copy(&mut entry, &mut out)?;
        }
    }
    Ok(())
}
