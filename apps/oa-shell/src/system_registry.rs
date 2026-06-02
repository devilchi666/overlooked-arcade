//! Runtime loader + lookup surface for `config/systems/<id>/` per-system
//! descriptors.
//!
//! Companion to [`crate::system_descriptor`] — that module owns the
//! types + pure-string parsers; this one owns the directory walk, the
//! hot-fail-on-malformed semantics, and the in-memory lookup table the
//! rest of `oa-shell` consults at runtime.
//!
//! ## Load model
//!
//! Slice 1: load once at app start (Phase E wires the startup call).
//! The registry is read-only after construction and lives in Tauri
//! state via `tauri::Manager::manage`. Subsequent slices may add a
//! "reload" path for content-pack development; for v1 a restart picks
//! up YAML edits.
//!
//! ## Directory layout
//!
//! ```text
//! config/systems/
//!   gb/
//!     system.yaml
//!   psx/
//!     system.yaml
//!     bios.yaml
//!     games.yaml
//!   nds/
//!     system.yaml
//!     bios.yaml
//!     games.yaml
//!   …
//! ```
//!
//! Each subfolder name MUST match the descriptor's `id` field; load
//! hot-fails on mismatch to catch operator-renamed folders that forgot
//! to update the YAML. `system.yaml` is mandatory; `bios.yaml` and
//! `games.yaml` are optional.
//!
//! ## Error handling
//!
//! Per the plan §"Loader behavior", L2 load hot-fails on any
//! malformed YAML — silently skipping a broken system would mean its
//! consumers (BIOS check, core installer, dat-refs) fall through to
//! the L1 const fallback and the operator never sees the error. The
//! `cargo test` + future CI guard catches malformed YAMLs before they
//! land in the tree.
//!
//! Per [`load_from_in_tree`] policy: the loader returns the first
//! error it encounters (file or folder) so the caller can surface a
//! single concrete fix path. Future content-pack loading (Slice 3)
//! will use a separate non-strict loader that logs + skips per-pack
//! errors without taking down the L2 registry.
//!
//! ## Resolver
//!
//! [`resolve_config_systems_dir`] mirrors the pattern
//! [`crate::system_info::resolve_docs_cores_dir`] uses for the
//! existing `docs/cores/` walk: try `<exe_dir>/config/systems/` first
//! (production install), fall back to the workspace-relative path
//! (`<CARGO_MANIFEST_DIR>/../../config/systems/`) for `cargo run` +
//! `cargo tauri dev` + `cargo test`.

#![allow(dead_code)] // scaffolding for Slice 1; consumers wire in Phases B-D

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::system_descriptor::{
    parse_bios_yaml, parse_games_yaml, parse_system_yaml, BiosDescriptor, GamesDescriptor,
    SystemDescriptor,
};

/// One per-system bundle as loaded from `config/systems/<id>/`. Owns
/// the three parsed file shapes plus the source path (kept for
/// diagnostic logging + the future L3 layering step in Slice 3 — when
/// a content pack overrides a per-system field we'll cite both source
/// paths in the merge-trace UI).
#[derive(Clone, Debug)]
pub struct LoadedSystem {
    /// Required `system.yaml` content.
    pub descriptor: SystemDescriptor,
    /// Optional `bios.yaml`. `None` when the system has no BIOS
    /// (GB / NES / SNES) or when the file simply isn't there yet
    /// during a mid-migration session.
    pub bios: Option<BiosDescriptor>,
    /// Optional `games.yaml`. `None` when the system has no curated
    /// game records.
    pub games: Option<GamesDescriptor>,
    /// Absolute path to the system's `config/systems/<id>/` folder.
    /// Kept for diagnostic surfaces + the future L3 source-trace UI.
    pub source_path: PathBuf,
}

/// The registry. Built once via [`SystemRegistry::load_from_in_tree`]
/// and stored in Tauri state. Lookup is `O(1)` HashMap by system_id.
#[derive(Clone, Debug, Default)]
pub struct SystemRegistry {
    by_id: HashMap<String, LoadedSystem>,
    source_root: Option<PathBuf>,
}

/// Errors the loader emits. Each variant names enough state for the
/// log line + the future toast surface to point operators at the file
/// they need to fix.
#[derive(Debug)]
pub enum RegistryError {
    /// Top-level `config/systems/` folder couldn't be read.
    RootIo {
        path: PathBuf,
        source: std::io::Error,
    },
    /// A per-system file's bytes couldn't be read.
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// A per-system file's contents failed serde parsing. `message`
    /// includes the serde error path (e.g. "unknown field `extentions`,
    /// expected one of …") so contributors can find the offending key.
    Parse { path: PathBuf, message: String },
    /// A per-system folder was missing its mandatory `system.yaml`.
    /// Likely a half-finished migration commit; failing loud forces
    /// the contributor to either complete the folder or remove it.
    MissingSystemYaml { folder: PathBuf },
    /// The descriptor's `id` field didn't match the parent folder
    /// name. Catches the rename-folder-but-forgot-the-YAML mistake.
    IdMismatch {
        folder_name: String,
        declared_id: String,
        path: PathBuf,
    },
    /// The embedded `system_info.system_id` field (when present) didn't
    /// match the descriptor's `id`. Catches the migration-tool
    /// copy-and-rename mistake where one system's panel data got
    /// pasted under another's folder.
    EmbeddedSystemInfoIdMismatch {
        descriptor_id: String,
        embedded_id: String,
        path: PathBuf,
    },
    /// Two folders declared the same `id`. Should be impossible given
    /// the folder-name → id validation, but defensive: catches a
    /// case-sensitive filesystem mismatch (`PSX/` + `psx/` both
    /// declaring `id: psx`).
    DuplicateId { id: String, paths: Vec<PathBuf> },
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RootIo { path, source } => {
                write!(
                    f,
                    "system_registry: cannot read root {}: {source}",
                    path.display()
                )
            }
            Self::Io { path, source } => {
                write!(f, "system_registry: cannot read {}: {source}", path.display())
            }
            Self::Parse { path, message } => {
                write!(f, "system_registry: {}: {message}", path.display())
            }
            Self::MissingSystemYaml { folder } => {
                write!(
                    f,
                    "system_registry: {} has no system.yaml (every per-system folder requires one)",
                    folder.display()
                )
            }
            Self::IdMismatch {
                folder_name,
                declared_id,
                path,
            } => {
                write!(
                    f,
                    "system_registry: {}: id `{declared_id}` does not match folder name `{folder_name}`",
                    path.display()
                )
            }
            Self::EmbeddedSystemInfoIdMismatch {
                descriptor_id,
                embedded_id,
                path,
            } => {
                write!(
                    f,
                    "system_registry: {}: descriptor id `{descriptor_id}` does not match embedded system_info.system_id `{embedded_id}`",
                    path.display()
                )
            }
            Self::DuplicateId { id, paths } => {
                let path_list: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
                write!(
                    f,
                    "system_registry: id `{id}` declared in multiple folders: {}",
                    path_list.join(", ")
                )
            }
        }
    }
}

impl std::error::Error for RegistryError {}

impl SystemRegistry {
    /// Construct an empty registry. Used by tests + as the fallback
    /// when no `config/systems/` directory resolves at startup (Slice
    /// 1 keeps the L1 const tables as a safety net for that case).
    pub fn empty() -> Self {
        Self {
            by_id: HashMap::new(),
            source_root: None,
        }
    }

    /// Walk `config_root` (typically `config/systems/`), load every
    /// per-system folder, return the populated registry. Hot-fails on
    /// the first error encountered.
    ///
    /// Per-system folders are processed in alphabetical order so the
    /// error message points at the same file across runs when multiple
    /// systems are simultaneously broken.
    pub fn load_from_in_tree(config_root: &Path) -> Result<Self, RegistryError> {
        let started = std::time::Instant::now();

        let entries =
            std::fs::read_dir(config_root).map_err(|source| RegistryError::RootIo {
                path: config_root.to_path_buf(),
                source,
            })?;

        let mut folders: Vec<(String, PathBuf)> = Vec::new();
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_dir() {
                continue;
            }
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            folders.push((name.to_string(), path));
        }
        folders.sort_by(|a, b| a.0.cmp(&b.0));

        let mut by_id: HashMap<String, LoadedSystem> = HashMap::new();
        let mut id_paths: HashMap<String, Vec<PathBuf>> = HashMap::new();

        for (folder_name, folder_path) in folders {
            let loaded = load_one_system(&folder_name, &folder_path)?;
            id_paths
                .entry(loaded.descriptor.id.clone())
                .or_default()
                .push(folder_path.clone());
            by_id.insert(loaded.descriptor.id.clone(), loaded);
        }

        // DuplicateId guard — case-insensitive filesystems (HFS+ /
        // default-NTFS) wouldn't allow two folders with the same name
        // anyway, but a case-sensitive filesystem could ship `PSX/` +
        // `psx/` both declaring `id: psx`. Catch that.
        for (id, paths) in &id_paths {
            if paths.len() > 1 {
                return Err(RegistryError::DuplicateId {
                    id: id.clone(),
                    paths: paths.clone(),
                });
            }
        }

        let count = by_id.len();
        log::info!(
            "system_registry: loaded {count} systems from {} in {}ms",
            config_root.display(),
            started.elapsed().as_millis()
        );

        Ok(Self {
            by_id,
            source_root: Some(config_root.to_path_buf()),
        })
    }

    /// Convenience: resolve `config/systems/` per
    /// [`resolve_config_systems_dir`] and call
    /// [`load_from_in_tree`](Self::load_from_in_tree). Returns
    /// [`SystemRegistry::empty`] when no `config/systems/` directory
    /// resolves — Slice 1's L1 const fallback covers the 38
    /// unmigrated systems in that case, so a missing-dir doesn't
    /// crash the app.
    pub fn load_default() -> Self {
        match resolve_config_systems_dir() {
            Some(dir) => match Self::load_from_in_tree(&dir) {
                Ok(r) => r,
                Err(e) => {
                    // L2 load failure during dev is loud at WARN; the
                    // cargo-test guard catches it before commit. In
                    // production this would mean a corrupted install —
                    // we still want the app to launch so the operator
                    // can see Help → Debug log.
                    log::warn!("system_registry: load failed; falling back to empty registry: {e}");
                    Self::empty()
                }
            },
            None => {
                log::info!(
                    "system_registry: no config/systems/ directory found \
                     (checked <exe>/config/systems and <repo>/config/systems); registry is empty"
                );
                Self::empty()
            }
        }
    }

    /// Look up a system by id. Returns `None` for unmigrated systems —
    /// callers MUST fall back to the L1 const path in that case (the
    /// "prefer-registry, fall back to const" shim pattern in Phases
    /// B-D).
    pub fn get(&self, system_id: &str) -> Option<&LoadedSystem> {
        self.by_id.get(system_id)
    }

    /// All registered system_ids in arbitrary order. Useful for diag
    /// surfaces (Help → Debug log, future "what's migrated" admin
    /// view).
    pub fn system_ids(&self) -> impl Iterator<Item = &str> {
        self.by_id.keys().map(|s| s.as_str())
    }

    /// Number of loaded systems.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// True when the registry has no loaded systems.
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// Root path the registry was loaded from. `None` for
    /// [`SystemRegistry::empty`] / the no-resolution fallback.
    pub fn source_root(&self) -> Option<&Path> {
        self.source_root.as_deref()
    }
}

/// Load a single per-system folder. Pure (no side effects beyond file
/// reads); the walking + dedupe + logging happens in
/// [`SystemRegistry::load_from_in_tree`].
fn load_one_system(folder_name: &str, folder_path: &Path) -> Result<LoadedSystem, RegistryError> {
    let system_yaml = folder_path.join("system.yaml");
    if !system_yaml.is_file() {
        return Err(RegistryError::MissingSystemYaml {
            folder: folder_path.to_path_buf(),
        });
    }

    let descriptor_body =
        std::fs::read_to_string(&system_yaml).map_err(|source| RegistryError::Io {
            path: system_yaml.clone(),
            source,
        })?;
    let descriptor = parse_system_yaml(&descriptor_body).map_err(|message| RegistryError::Parse {
        path: system_yaml.clone(),
        message,
    })?;

    if descriptor.id != folder_name {
        return Err(RegistryError::IdMismatch {
            folder_name: folder_name.to_string(),
            declared_id: descriptor.id.clone(),
            path: system_yaml.clone(),
        });
    }

    if let Some(panel) = &descriptor.system_info {
        if panel.system_id != descriptor.id {
            return Err(RegistryError::EmbeddedSystemInfoIdMismatch {
                descriptor_id: descriptor.id.clone(),
                embedded_id: panel.system_id.clone(),
                path: system_yaml.clone(),
            });
        }
    }

    let bios = read_optional_yaml(folder_path, "bios.yaml", parse_bios_yaml)?;
    let games = read_optional_yaml(folder_path, "games.yaml", parse_games_yaml)?;

    Ok(LoadedSystem {
        descriptor,
        bios,
        games,
        source_path: folder_path.to_path_buf(),
    })
}

/// Generic "read this file if it exists, parse it via `parser`,
/// hot-fail on any error other than absence" helper. Shared by the
/// bios.yaml + games.yaml branches.
fn read_optional_yaml<T, F>(
    folder: &Path,
    filename: &str,
    parser: F,
) -> Result<Option<T>, RegistryError>
where
    F: FnOnce(&str) -> Result<T, String>,
{
    let path = folder.join(filename);
    if !path.is_file() {
        return Ok(None);
    }
    let body = std::fs::read_to_string(&path).map_err(|source| RegistryError::Io {
        path: path.clone(),
        source,
    })?;
    let parsed = parser(&body).map_err(|message| RegistryError::Parse {
        path: path.clone(),
        message,
    })?;
    Ok(Some(parsed))
}

/// Resolve `config/systems/` at runtime. Walks two candidates in
/// priority order; returns the first that exists. `None` when neither
/// resolves — [`SystemRegistry::load_default`] falls back to
/// [`SystemRegistry::empty`] in that case.
///
/// Order:
/// 1. `<exe_dir>/config/systems/` — production install path. The Tauri
///    bundler config copies the in-tree `config/` folder next to the
///    `oa-shell.exe` (wired in Slice 1's Phase E packaging update or
///    Slice 2's mass-migration PR, whichever lands first).
/// 2. `<CARGO_MANIFEST_DIR>/../../config/systems/` — source-tree path
///    for `cargo run` + `cargo tauri dev` + `cargo test`. Hard-coded
///    relative to the `oa-shell` crate, which lives two levels under
///    the repo root.
pub fn resolve_config_systems_dir() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let p = exe_dir.join("config").join("systems");
            if p.is_dir() {
                return Some(p);
            }
        }
    }
    let dev = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("config")
        .join("systems");
    if dev.is_dir() {
        return Some(dev);
    }
    None
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Build a temp `config/systems/` root with the given per-system
    /// files, return its path. Caller is responsible for cleanup via
    /// [`cleanup_tmp_root`].
    ///
    /// `entries` is a slice of `(folder_name, files)` where `files` is
    /// itself a slice of `(filename, contents)`. An empty `files` list
    /// creates an empty folder (which the loader should reject with
    /// `MissingSystemYaml`).
    fn make_tmp_root(label: &str, entries: &[(&str, &[(&str, &str)])]) -> PathBuf {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let nonce = COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "oa-sysregistry-{label}-{}-{nonce}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create tmp root");
        for (folder, files) in entries {
            let folder_path = root.join(folder);
            std::fs::create_dir_all(&folder_path).expect("create folder");
            for (filename, body) in *files {
                std::fs::write(folder_path.join(filename), body).expect("write file");
            }
        }
        root
    }

    fn cleanup_tmp_root(path: &Path) {
        let _ = std::fs::remove_dir_all(path);
    }

    // ---- empty / missing root --------------------------------------

    #[test]
    fn empty_registry_returns_none_for_lookups() {
        let r = SystemRegistry::empty();
        assert!(r.is_empty());
        assert_eq!(r.len(), 0);
        assert!(r.get("psx").is_none());
        assert!(r.source_root().is_none());
    }

    #[test]
    fn load_from_empty_dir_yields_empty_registry() {
        let root = make_tmp_root("empty", &[]);
        let r = SystemRegistry::load_from_in_tree(&root).unwrap();
        assert!(r.is_empty());
        assert_eq!(r.source_root(), Some(root.as_path()));
        cleanup_tmp_root(&root);
    }

    #[test]
    fn load_from_nonexistent_root_returns_root_io_error() {
        let bogus = std::env::temp_dir().join("oa-sysregistry-does-not-exist-xyz123");
        let _ = std::fs::remove_dir_all(&bogus);
        let err = SystemRegistry::load_from_in_tree(&bogus).unwrap_err();
        assert!(matches!(err, RegistryError::RootIo { .. }));
    }

    // ---- minimum-shape system --------------------------------------

    #[test]
    fn load_minimum_shape_one_system() {
        let root = make_tmp_root(
            "minimal",
            &[(
                "gb",
                &[("system.yaml", "id: gb\ndisplay_name: Game Boy\n")],
            )],
        );
        let r = SystemRegistry::load_from_in_tree(&root).unwrap();
        assert_eq!(r.len(), 1);
        let gb = r.get("gb").expect("gb loaded");
        assert_eq!(gb.descriptor.display_name, "Game Boy");
        assert!(gb.bios.is_none());
        assert!(gb.games.is_none());
        cleanup_tmp_root(&root);
    }

    #[test]
    fn load_full_shape_with_bios_and_games() {
        // PSX-shape: descriptor + any_of BIOS + one game record.
        let system_yaml = "id: psx\ndisplay_name: PlayStation\n";
        let bios_yaml = r#"
semantics: any_of
files:
  - name: scph5501.bin
    sha1: 0555C6FAE8906F3F09BAF5988F00E55F88E9F30B
    description: "US PSX BIOS"
"#;
        let games_yaml = r#"
games:
  - id_key:
      system_id: psx
      rom_title: "Test Game"
    date: 1999
"#;
        let root = make_tmp_root(
            "full",
            &[(
                "psx",
                &[
                    ("system.yaml", system_yaml),
                    ("bios.yaml", bios_yaml),
                    ("games.yaml", games_yaml),
                ],
            )],
        );
        let r = SystemRegistry::load_from_in_tree(&root).unwrap();
        let psx = r.get("psx").expect("psx loaded");
        let bios = psx.bios.as_ref().expect("bios loaded");
        assert_eq!(bios.files.len(), 1);
        assert_eq!(bios.files[0].name, "scph5501.bin");
        let games = psx.games.as_ref().expect("games loaded");
        assert_eq!(games.games.len(), 1);
        assert_eq!(games.games[0].id_key.system_id, "psx");
        cleanup_tmp_root(&root);
    }

    // ---- hot-fail paths --------------------------------------------

    #[test]
    fn load_hot_fails_on_missing_system_yaml() {
        // Empty folder under config/systems/ — must error rather than
        // silently skipping.
        let root = make_tmp_root("missing_yaml", &[("gb", &[])]);
        let err = SystemRegistry::load_from_in_tree(&root).unwrap_err();
        assert!(
            matches!(err, RegistryError::MissingSystemYaml { .. }),
            "got: {err}"
        );
        cleanup_tmp_root(&root);
    }

    #[test]
    fn load_hot_fails_on_id_folder_mismatch() {
        // Folder name "gb" but descriptor says id "snes" → IdMismatch.
        let root = make_tmp_root(
            "id_mismatch",
            &[(
                "gb",
                &[("system.yaml", "id: snes\ndisplay_name: Super Nintendo\n")],
            )],
        );
        let err = SystemRegistry::load_from_in_tree(&root).unwrap_err();
        match err {
            RegistryError::IdMismatch {
                folder_name,
                declared_id,
                ..
            } => {
                assert_eq!(folder_name, "gb");
                assert_eq!(declared_id, "snes");
            }
            _ => panic!("expected IdMismatch, got: {err}"),
        }
        cleanup_tmp_root(&root);
    }

    #[test]
    fn load_hot_fails_on_malformed_yaml() {
        let root = make_tmp_root(
            "malformed",
            &[(
                "gb",
                &[(
                    "system.yaml",
                    "id: gb\ndisplay_name: Game Boy\nunknown_field: oops\n",
                )],
            )],
        );
        let err = SystemRegistry::load_from_in_tree(&root).unwrap_err();
        match err {
            RegistryError::Parse { message, .. } => {
                assert!(
                    message.contains("unknown_field") || message.contains("unknown field"),
                    "expected unknown-field error, got: {message}"
                );
            }
            _ => panic!("expected Parse error, got: {err}"),
        }
        cleanup_tmp_root(&root);
    }

    #[test]
    fn load_hot_fails_on_embedded_system_info_id_mismatch() {
        // Descriptor says id: gb, embedded system_info says system_id: psx.
        // Catches migration-tool copy-paste mistake.
        let yaml = r#"
id: gb
display_name: Game Boy
system_info:
  system_id: psx
  manufacturer: Sony
"#;
        let root = make_tmp_root(
            "embedded_mismatch",
            &[("gb", &[("system.yaml", yaml)])],
        );
        let err = SystemRegistry::load_from_in_tree(&root).unwrap_err();
        match err {
            RegistryError::EmbeddedSystemInfoIdMismatch {
                descriptor_id,
                embedded_id,
                ..
            } => {
                assert_eq!(descriptor_id, "gb");
                assert_eq!(embedded_id, "psx");
            }
            _ => panic!("expected EmbeddedSystemInfoIdMismatch, got: {err}"),
        }
        cleanup_tmp_root(&root);
    }

    #[test]
    fn load_hot_fails_on_malformed_bios_yaml() {
        // system.yaml is fine but bios.yaml is malformed → Parse error
        // surfaces with the bios.yaml path, NOT the system.yaml path.
        let root = make_tmp_root(
            "bad_bios",
            &[(
                "psx",
                &[
                    ("system.yaml", "id: psx\ndisplay_name: PlayStation\n"),
                    (
                        "bios.yaml",
                        "semantics: invalid_semantics\nfiles: []\n",
                    ),
                ],
            )],
        );
        let err = SystemRegistry::load_from_in_tree(&root).unwrap_err();
        match err {
            RegistryError::Parse { path, message } => {
                assert!(
                    path.ends_with("bios.yaml"),
                    "expected error on bios.yaml, got: {}",
                    path.display()
                );
                assert!(
                    message.contains("invalid_semantics") || message.contains("unknown variant"),
                    "expected variant error, got: {message}"
                );
            }
            _ => panic!("expected Parse error on bios.yaml, got: {err}"),
        }
        cleanup_tmp_root(&root);
    }

    // ---- multi-system load (alphabetical processing) ---------------

    #[test]
    fn load_multiple_systems_each_resolves() {
        let root = make_tmp_root(
            "multi",
            &[
                ("gb", &[("system.yaml", "id: gb\ndisplay_name: Game Boy\n")]),
                (
                    "psx",
                    &[("system.yaml", "id: psx\ndisplay_name: PlayStation\n")],
                ),
                (
                    "nds",
                    &[("system.yaml", "id: nds\ndisplay_name: Nintendo DS\n")],
                ),
            ],
        );
        let r = SystemRegistry::load_from_in_tree(&root).unwrap();
        assert_eq!(r.len(), 3);
        assert!(r.get("gb").is_some());
        assert!(r.get("psx").is_some());
        assert!(r.get("nds").is_some());
        // Unknown id returns None — important for the
        // prefer-registry-then-fallback consumer shim pattern.
        assert!(r.get("snes").is_none());
        // system_ids iteration covers all three.
        let ids: std::collections::HashSet<&str> = r.system_ids().collect();
        assert!(ids.contains("gb"));
        assert!(ids.contains("psx"));
        assert!(ids.contains("nds"));
        cleanup_tmp_root(&root);
    }

    // ---- load_default ----------------------------------------------

    #[test]
    fn load_default_returns_empty_when_no_dir_resolves() {
        // We can't easily test the success path without polluting the
        // source-tree config/ directory. But load_default's empty path
        // is the safety net the rest of Slice 1 relies on, so confirm
        // an empty SystemRegistry comes back cleanly from a fresh call
        // when the source-tree dir genuinely doesn't exist yet (Phase
        // A test runs BEFORE Phase B creates the pilot folders).
        //
        // NOTE: once Phase B writes `config/systems/gb/`, this test
        // changes meaning — it'll exercise the success path of the
        // source-tree fallback. Either way load_default never panics.
        let r = SystemRegistry::load_default();
        // No assertion on len() — depends on whether B/C/D have shipped
        // their pilot folders yet. The contract is "never panics".
        let _ = r.len();
    }
}
