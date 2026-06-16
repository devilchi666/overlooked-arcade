//! Content-pack manager — the **network + Tauri glue** around the pure
//! `oa-packs` crate (Slice 2 of the oa-packs arc).
//!
//! `oa-packs` does the trust-critical work offline (sha256 verify, manifest
//! validation, staged atomic install — all unit-tested there). This module
//! adds only what *must* live in the shell: fetching `registry.json` from a
//! config-supplied URL, downloading the pack zip, and exposing the lot as
//! Tauri commands. We reuse the existing `http_retry` helper rather than
//! rebuilding a fetcher (the `core_installer.rs` downloader is the richer
//! cousin — job rows, resume, progress events; pack download grows those in
//! Slice 3 when the UI needs them).
//!
//! ## Network discipline (content-packs.md §3, §9)
//!
//! Every command that touches the network calls [`ensure_network_allowed`]
//! **first, synchronously**, and returns [`NETWORK_DISABLED`] before any
//! request goes out when the master toggle is OFF. Local commands (`list`,
//! `uninstall`, the prefs getters/setters) never hit the network and carry
//! no gate.
//!
//! ## On-disk layout (decision CP2)
//!
//! Packs install to `<exe_dir>/<type>/community/<pack_id>/` — the literal
//! `<type>` segment, per CP2 (this supersedes content-packs.md §7's older
//! semantic `discover/` naming). `<exe_dir>` is the [`PacksRoot`] dest_root;
//! `oa-packs` builds the rest of the path.
//!
//! ## Trust chain
//!
//! install/update fetch the registry **themselves** and look the entry up
//! by id — they never trust a frontend-supplied `sha256`. The chain is:
//! registry (authoritative) → `entry.sha256` → `oa_packs::verify` of the
//! downloaded bytes → `oa_packs::install_from_local_zip`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;

use crate::packs_prefs::{read_packs_prefs, write_packs_prefs, PacksPrefs};
use crate::AppDataDir;

/// Stable error string returned by every network command when the master
/// "Allow network calls" toggle is OFF. The frontend (Slices 3–4) matches
/// on it to route the operator to Settings → Privacy.
pub const NETWORK_DISABLED: &str =
    "NETWORK_DISABLED: pack network calls are off (Settings → Privacy → Allow network calls)";

const USER_AGENT: &str = "OverlookedArcade";

/// dest_root for pack installs — the directory next to `oa-shell.exe`.
/// Mirrors `resolve_cores_dir` in main.rs but stops at `<exe_dir>` (the
/// `<type>/community/` tail is `oa-packs`' job). Managed once at startup.
pub struct PacksRoot(pub PathBuf);

/// Resolve `<exe_dir>` the same way `resolve_cores_dir` does, minus the
/// `cores` join. Falls back to the cwd if the exe path can't be read.
pub fn resolve_packs_root() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

/// One installed pack, as surfaced to the frontend (Slice 3's Installed
/// list). Derived from the pack's on-disk `manifest.yml`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPack {
    pub id: String,
    pub pack_type: String,
    pub name: String,
    pub version: String,
    pub license: Option<String>,
    /// Absolute install path (`<exe_dir>/<type>/community/<id>`).
    pub path: String,
}

/// Result of an update check + apply.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOutcome {
    /// True if a newer version was downloaded + installed; false if the
    /// installed version was already current.
    pub updated: bool,
    pub from_version: String,
    pub to_version: String,
}

// ---------------------------------------------------------------------------
// Network gate + small shared helpers
// ---------------------------------------------------------------------------

/// The synchronous gate. Returns `Err(NETWORK_DISABLED)` when the master
/// toggle is OFF — called at the top of every network command, before any
/// request is built.
fn ensure_network_allowed(prefs: &PacksPrefs) -> Result<(), String> {
    if prefs.allow_network {
        Ok(())
    } else {
        Err(NETWORK_DISABLED.to_string())
    }
}

fn build_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(180))
        .build()
        .map_err(|e| format!("http client: {e}"))
}

/// Fetch + parse the registry from `prefs.registry_url`. Network-gated by
/// the caller. Shared by the fetch command and install/update.
async fn fetch_registry_inner(
    client: &reqwest::Client,
    prefs: &PacksPrefs,
) -> Result<oa_packs::Registry, String> {
    let body = crate::http_retry::get_text_with_retry(client, &prefs.registry_url, USER_AGENT)
        .await?
        .ok_or_else(|| format!("registry not found (404) at {}", prefs.registry_url))?;
    serde_json::from_str::<oa_packs::Registry>(&body)
        .map_err(|e| format!("parse registry.json: {e}"))
}

fn find_entry<'a>(
    registry: &'a oa_packs::Registry,
    pack_id: &str,
) -> Result<&'a oa_packs::PackEntry, String> {
    registry
        .packs
        .iter()
        .find(|p| p.id == pack_id)
        .ok_or_else(|| format!("pack `{pack_id}` is not listed in the registry"))
}

/// Download `url` to `dest` (full-buffer; packs are small — tens of MB).
/// Reuses the one-retry `http_retry` helper. The bytes are verified by
/// `oa_packs::install_from_local_zip` against the registry sha256 *after*
/// this returns, so a corrupt/short download is caught downstream.
async fn download_to_file(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
) -> Result<(), String> {
    let resp = crate::http_retry::get_with_retry(client, url, USER_AGENT)
        .await?
        .ok_or_else(|| format!("pack zip not found (404) at {url}"))?;
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("read pack body {url}: {e}"))?;
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create download dir: {e}"))?;
    }
    std::fs::write(dest, &bytes).map_err(|e| format!("write pack zip {}: {e}", dest.display()))?;
    Ok(())
}

/// The running OA version, for `oa-packs`' `min_oa_version` gate. Sourced
/// from the crate version (the workspace version today) — threaded into
/// `oa-packs` as data, never hard-coded there.
fn running_oa_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn now_rfc3339() -> Option<String> {
    use time::format_description::well_known::Rfc3339;
    time::OffsetDateTime::now_utc().format(&Rfc3339).ok()
}

/// Scan `<exe_dir>/*/community/*/manifest.yml` and return every installed
/// pack. Type-agnostic: any top-level dir with a `community/` child is a
/// pack-type root (CP3 — we don't hard-code the type set). Unreadable /
/// unparseable pack dirs are skipped with a warning, not fatal.
fn scan_installed(root: &Path) -> Vec<InstalledPack> {
    let mut out = Vec::new();
    let Ok(types) = std::fs::read_dir(root) else {
        return out;
    };
    for type_entry in types.flatten() {
        let type_dir = type_entry.path();
        let community = type_dir.join("community");
        let Ok(packs) = std::fs::read_dir(&community) else {
            continue; // not a pack-type root (no community/ child)
        };
        for pack_entry in packs.flatten() {
            let pack_dir = pack_entry.path();
            if !pack_dir.is_dir() {
                continue;
            }
            let manifest_path = pack_dir.join("manifest.yml");
            let Ok(bytes) = std::fs::read(&manifest_path) else {
                continue;
            };
            match oa_packs::Manifest::from_yaml_bytes(&bytes) {
                Ok(m) => out.push(InstalledPack {
                    id: m.id,
                    pack_type: m.pack_type,
                    name: m.name,
                    version: m.version,
                    license: m.license,
                    path: pack_dir.to_string_lossy().into_owned(),
                }),
                Err(e) => log::warn!(
                    "oa-packs: skipping unreadable manifest at {}: {e}",
                    manifest_path.display()
                ),
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Tauri commands — prefs (local, no network)
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn oa_packs_get_prefs(app_data_dir: tauri::State<'_, AppDataDir>) -> PacksPrefs {
    read_packs_prefs(&app_data_dir.0)
}

/// Repoint the registry URL (CP1 — the URL is config). An empty string
/// resets to the seeded default so the operator can always get back to a
/// working baseline.
#[tauri::command]
pub fn oa_packs_set_registry_url(
    url: String,
    app_data_dir: tauri::State<'_, AppDataDir>,
) -> Result<PacksPrefs, String> {
    let mut prefs = read_packs_prefs(&app_data_dir.0);
    let trimmed = url.trim();
    prefs.registry_url = if trimmed.is_empty() {
        crate::packs_prefs::DEFAULT_REGISTRY_URL.to_string()
    } else {
        trimmed.to_string()
    };
    write_packs_prefs(&app_data_dir.0, &prefs).map_err(|e| format!("write packs prefs: {e}"))?;
    Ok(prefs)
}

/// Flip the master network toggle (content-packs.md §9). The Privacy panel
/// (Slice 4) is the operator-facing surface; this command backs it.
#[tauri::command]
pub fn oa_packs_set_allow_network(
    allow: bool,
    app_data_dir: tauri::State<'_, AppDataDir>,
) -> Result<PacksPrefs, String> {
    let mut prefs = read_packs_prefs(&app_data_dir.0);
    prefs.allow_network = allow;
    write_packs_prefs(&app_data_dir.0, &prefs).map_err(|e| format!("write packs prefs: {e}"))?;
    Ok(prefs)
}

// ---------------------------------------------------------------------------
// Tauri commands — local
// ---------------------------------------------------------------------------

/// List installed packs by scanning the on-disk layout. No network.
#[tauri::command]
pub fn oa_packs_list(packs_root: tauri::State<'_, PacksRoot>) -> Vec<InstalledPack> {
    scan_installed(&packs_root.0)
}

/// Remove an installed pack's directory. No network. Rollback retention
/// (content-packs.md §8) is Slice 3 — this is the plain removal.
#[tauri::command]
pub fn oa_packs_uninstall(
    pack_id: String,
    packs_root: tauri::State<'_, PacksRoot>,
) -> Result<(), String> {
    let installed = scan_installed(&packs_root.0);
    let pack = installed
        .iter()
        .find(|p| p.id == pack_id)
        .ok_or_else(|| format!("pack `{pack_id}` is not installed"))?;
    std::fs::remove_dir_all(&pack.path)
        .map_err(|e| format!("remove pack dir {}: {e}", pack.path))?;
    log::info!("oa-packs: uninstalled `{pack_id}` from {}", pack.path);
    Ok(())
}

// ---------------------------------------------------------------------------
// Tauri commands — network (all gated)
// ---------------------------------------------------------------------------

/// Fetch + parse the registry, stamping `last_checked` on success.
#[tauri::command]
pub async fn oa_packs_fetch_registry(
    app_data_dir: tauri::State<'_, AppDataDir>,
) -> Result<oa_packs::Registry, String> {
    let prefs = read_packs_prefs(&app_data_dir.0);
    ensure_network_allowed(&prefs)?;
    let client = build_client()?;
    let registry = fetch_registry_inner(&client, &prefs).await?;

    // Best-effort: record when we last successfully checked. A write
    // failure here doesn't fail the fetch — the registry is already in hand.
    let mut stamped = prefs;
    stamped.last_checked = now_rfc3339();
    if let Err(e) = write_packs_prefs(&app_data_dir.0, &stamped) {
        log::warn!("oa-packs: failed to stamp last_checked: {e}");
    }
    Ok(registry)
}

/// Install a pack by id: gate → fetch registry → find entry → download →
/// verify + atomic install (in `oa-packs`). Returns the installed pack.
#[tauri::command]
pub async fn oa_packs_install(
    pack_id: String,
    app_data_dir: tauri::State<'_, AppDataDir>,
    packs_root: tauri::State<'_, PacksRoot>,
) -> Result<InstalledPack, String> {
    let prefs = read_packs_prefs(&app_data_dir.0);
    ensure_network_allowed(&prefs)?;
    let client = build_client()?;
    let registry = fetch_registry_inner(&client, &prefs).await?;
    let entry = find_entry(&registry, &pack_id)?.clone();

    let dest_root = packs_root.0.clone();
    let download_path = app_data_dir
        .0
        .join("packs")
        .join(".download")
        .join(format!("{pack_id}.zip"));
    download_to_file(&client, &entry.url, &download_path).await?;

    // verify (sha256 vs entry) + manifest-vs-registry validation + atomic
    // install all happen inside the pure crate.
    let result = oa_packs::install_from_local_zip(
        &download_path,
        &entry,
        &dest_root,
        running_oa_version(),
    )
    .map_err(|e| format!("install `{pack_id}`: {e}"));
    let _ = std::fs::remove_file(&download_path); // best-effort cleanup
    let installed_dir = result?;

    log::info!("oa-packs: installed `{pack_id}` v{} at {}", entry.version, installed_dir.display());
    Ok(InstalledPack {
        id: entry.id,
        pack_type: entry.pack_type,
        name: entry.name,
        version: entry.version,
        license: entry.license,
        path: installed_dir.to_string_lossy().into_owned(),
    })
}

/// Check the registry for a newer version of an installed pack and apply
/// it if found. Returns whether an update was applied. Rollback retention
/// (content-packs.md §8) is Slice 3; this replaces in place via the
/// crate's atomic install.
#[tauri::command]
pub async fn oa_packs_update(
    pack_id: String,
    app_data_dir: tauri::State<'_, AppDataDir>,
    packs_root: tauri::State<'_, PacksRoot>,
) -> Result<UpdateOutcome, String> {
    let prefs = read_packs_prefs(&app_data_dir.0);
    ensure_network_allowed(&prefs)?;

    let installed = scan_installed(&packs_root.0);
    let current = installed
        .iter()
        .find(|p| p.id == pack_id)
        .ok_or_else(|| format!("pack `{pack_id}` is not installed"))?;
    let from_version = current.version.clone();

    let client = build_client()?;
    let registry = fetch_registry_inner(&client, &prefs).await?;
    let entry = find_entry(&registry, &pack_id)?.clone();

    // Only download when the registry is strictly newer than what's on disk.
    let is_newer = oa_packs::compare_versions(&entry.version, &from_version)
        .map(|ord| ord == std::cmp::Ordering::Greater)
        .map_err(|e| format!("compare versions: {e}"))?;
    if !is_newer {
        return Ok(UpdateOutcome {
            updated: false,
            from_version,
            to_version: entry.version,
        });
    }

    let dest_root = packs_root.0.clone();
    let download_path = app_data_dir
        .0
        .join("packs")
        .join(".download")
        .join(format!("{pack_id}.zip"));
    download_to_file(&client, &entry.url, &download_path).await?;
    let result = oa_packs::install_from_local_zip(
        &download_path,
        &entry,
        &dest_root,
        running_oa_version(),
    )
    .map_err(|e| format!("update `{pack_id}`: {e}"));
    let _ = std::fs::remove_file(&download_path);
    result?;

    log::info!("oa-packs: updated `{pack_id}` {from_version} → {}", entry.version);
    Ok(UpdateOutcome {
        updated: true,
        from_version,
        to_version: entry.version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs_prefs::PacksPrefs;

    #[test]
    fn gate_blocks_when_network_off() {
        let mut prefs = PacksPrefs::default();
        prefs.allow_network = false;
        let err = ensure_network_allowed(&prefs).unwrap_err();
        assert!(err.starts_with("NETWORK_DISABLED"));
    }

    #[test]
    fn gate_passes_when_network_on() {
        assert!(ensure_network_allowed(&PacksPrefs::default()).is_ok());
    }

    #[test]
    fn scan_finds_packs_and_ignores_non_pack_dirs() {
        let root = std::env::temp_dir().join(format!(
            "oa-packs-scan-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&root);

        // A real pack at <root>/editorial/community/oa-ed/manifest.yml.
        let pack_dir = root.join("editorial").join("community").join("oa-ed");
        std::fs::create_dir_all(&pack_dir).unwrap();
        std::fs::write(
            pack_dir.join("manifest.yml"),
            "id: oa-ed\nversion: 1.0.0\ntype: editorial\nname: Ed\nlicense: CC-BY-SA-4.0\n",
        )
        .unwrap();
        // A non-pack sibling (e.g. cores/) with no community/ child.
        std::fs::create_dir_all(root.join("cores")).unwrap();

        let found = scan_installed(&root);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "oa-ed");
        assert_eq!(found[0].pack_type, "editorial");
        assert_eq!(found[0].version, "1.0.0");

        let _ = std::fs::remove_dir_all(&root);
    }
}
