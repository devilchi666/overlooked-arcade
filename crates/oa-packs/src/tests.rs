//! Slice 1 acceptance tests. All filesystem I/O stays inside a temp dir
//! that's removed on drop — no network anywhere.
//!
//! Coverage (per the Slice 1 spec):
//! - good hash + matching manifest installs into the per-type layout;
//! - a tampered zip (wrong sha256) is rejected;
//! - a manifest that disagrees with its registry entry is rejected;
//! - a failed install leaves no partial directory at the destination;
//! - the `min_oa_version` gate refuses an OA that's too old;
//! - per-type baseline modelling (CP4) + the on-disk layout.
//!
//! (Numeric version comparison has its own unit tests in `version.rs`.)

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::registry::PackEntry;
use crate::{install_from_local_zip, sha256_hex, verify, PackError};

// ---------------------------------------------------------------------------
// Test scaffolding
// ---------------------------------------------------------------------------

/// A unique temp directory removed on drop. Avoids pulling the `tempfile`
/// crate for what Slice 1 needs.
struct TestDir(PathBuf);

impl TestDir {
    fn new(tag: &str) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "oa-packs-test-{}-{tag}-{n}",
            std::process::id()
        ));
        if path.exists() {
            let _ = std::fs::remove_dir_all(&path);
        }
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Build an in-memory pack zip containing the given `manifest.yml` body
/// plus a token content file, so the zip is a realistic multi-entry pack.
fn build_pack_zip(manifest_yaml: &str) -> Vec<u8> {
    use zip::write::SimpleFileOptions;
    let mut buf = Vec::new();
    {
        let mut zw = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let opts =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        zw.start_file("manifest.yml", opts).unwrap();
        zw.write_all(manifest_yaml.as_bytes()).unwrap();
        zw.start_file("articles/intro.md", opts).unwrap();
        zw.write_all(b"# Intro\n").unwrap();
        zw.finish().unwrap();
    }
    buf
}

/// A well-formed editorial manifest matching [`sample_entry`].
fn sample_manifest_yaml() -> String {
    "\
id: oa-editorial-baseline
version: 0.3.0
type: editorial
name: OA Editorial Baseline
maintainer: overlooked-arcade
license: CC-BY-SA-4.0
min_oa_version: \"0.9.0\"
summary: Twenty-odd articles.
"
    .to_string()
}

/// A registry entry whose identity fields match [`sample_manifest_yaml`].
/// `sha256` is filled in by the caller once the zip bytes are known.
fn sample_entry(sha256: &str) -> PackEntry {
    PackEntry {
        id: "oa-editorial-baseline".into(),
        pack_type: "editorial".into(),
        name: "OA Editorial Baseline".into(),
        version: "0.3.0".into(),
        url: "https://example.invalid/pack.zip".into(),
        sha256: sha256.into(),
        size_bytes: None,
        depends_on: vec![],
        min_oa_version: Some("0.9.0".into()),
        license: Some("CC-BY-SA-4.0".into()),
        homepage: None,
        summary: None,
        maintainer: None,
    }
}

/// Write `bytes` to `<dir>/<name>` and return the path.
fn write_zip(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, bytes).unwrap();
    p
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn good_hash_and_manifest_installs_into_per_type_layout() {
    let td = TestDir::new("install-ok");
    let zip = build_pack_zip(&sample_manifest_yaml());
    let entry = sample_entry(&sha256_hex(&zip));
    let zip_path = write_zip(td.path(), "pack.zip", &zip);
    let dest_root = td.path().join("packroot");

    let installed = install_from_local_zip(&zip_path, &entry, &dest_root, "1.0.0").unwrap();

    // Lands at <dest_root>/<type>/community/<id>/ (content-packs.md §7).
    let expected = dest_root
        .join("editorial")
        .join("community")
        .join("oa-editorial-baseline");
    assert_eq!(installed, expected);
    assert!(installed.join("manifest.yml").is_file());
    assert!(installed.join("articles").join("intro.md").is_file());
    // Staging is fully consumed — no leftover scratch dir.
    assert!(!dest_root.join(".staging").join("oa-editorial-baseline").exists());
}

#[test]
fn wrong_hash_is_rejected_via_verify() {
    let zip = build_pack_zip(&sample_manifest_yaml());
    // verify() is the unit under test: real bytes, a hash that isn't theirs.
    let err = verify(&zip, &"0".repeat(64)).unwrap_err();
    assert!(matches!(err, PackError::Sha256Mismatch { .. }));
    // Matching hash passes (case-insensitive on the expected side).
    assert!(verify(&zip, &sha256_hex(&zip).to_uppercase()).is_ok());
}

#[test]
fn tampered_zip_is_rejected_and_leaves_no_partial_dir() {
    let td = TestDir::new("tampered");
    let zip = build_pack_zip(&sample_manifest_yaml());
    // Registry pins the *original* hash; the on-disk zip is then tampered.
    let entry = sample_entry(&sha256_hex(&zip));
    let mut tampered = zip.clone();
    *tampered.last_mut().unwrap() ^= 0xFF;
    let zip_path = write_zip(td.path(), "pack.zip", &tampered);
    let dest_root = td.path().join("packroot");

    let err = install_from_local_zip(&zip_path, &entry, &dest_root, "1.0.0").unwrap_err();
    assert!(matches!(err, PackError::Sha256Mismatch { .. }));

    // Nothing was created at the destination, and no staging leaked.
    assert!(!dest_root.join("editorial").exists());
    assert_no_staging_leak(&dest_root);
}

#[test]
fn manifest_disagreeing_with_registry_is_rejected() {
    let td = TestDir::new("manifest-mismatch");
    let zip = build_pack_zip(&sample_manifest_yaml());
    // Same bytes (hash matches) but the registry claims a different
    // version than the manifest declares → swapped/wrong pack.
    let mut entry = sample_entry(&sha256_hex(&zip));
    entry.version = "0.4.0".into();
    let zip_path = write_zip(td.path(), "pack.zip", &zip);
    let dest_root = td.path().join("packroot");

    let err = install_from_local_zip(&zip_path, &entry, &dest_root, "1.0.0").unwrap_err();
    match err {
        PackError::ManifestMismatch { field, .. } => assert_eq!(field, "version"),
        other => panic!("expected version mismatch, got {other:?}"),
    }
    // Validation failed after staging — destination untouched, no partial.
    assert!(!dest_root.join("editorial").exists());
    assert_no_staging_leak(&dest_root);
}

#[test]
fn min_oa_version_gate_refuses_old_oa() {
    let td = TestDir::new("oa-too-old");
    let zip = build_pack_zip(&sample_manifest_yaml()); // requires OA >= 0.9.0
    let entry = sample_entry(&sha256_hex(&zip));
    let zip_path = write_zip(td.path(), "pack.zip", &zip);
    let dest_root = td.path().join("packroot");

    let err = install_from_local_zip(&zip_path, &entry, &dest_root, "0.8.0").unwrap_err();
    assert!(matches!(err, PackError::OaVersionTooOld { .. }));
    assert!(!dest_root.join("editorial").exists());
    assert_no_staging_leak(&dest_root);
}

#[test]
fn missing_manifest_is_rejected() {
    let td = TestDir::new("no-manifest");
    // A zip with content but no manifest.yml.
    let mut buf = Vec::new();
    {
        use zip::write::SimpleFileOptions;
        let mut zw = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        zw.start_file("articles/intro.md", SimpleFileOptions::default())
            .unwrap();
        zw.write_all(b"# Intro\n").unwrap();
        zw.finish().unwrap();
    }
    let entry = sample_entry(&sha256_hex(&buf));
    let zip_path = write_zip(td.path(), "pack.zip", &buf);
    let dest_root = td.path().join("packroot");

    let err = install_from_local_zip(&zip_path, &entry, &dest_root, "1.0.0").unwrap_err();
    assert!(matches!(err, PackError::ManifestMissing));
    assert_no_staging_leak(&dest_root);
}

#[test]
fn update_replaces_a_prior_install_atomically() {
    let td = TestDir::new("update");
    let dest_root = td.path().join("packroot");

    // v0.3.0 installed first.
    let zip_v3 = build_pack_zip(&sample_manifest_yaml());
    let entry_v3 = sample_entry(&sha256_hex(&zip_v3));
    let path_v3 = write_zip(td.path(), "v3.zip", &zip_v3);
    let installed = install_from_local_zip(&path_v3, &entry_v3, &dest_root, "1.0.0").unwrap();
    assert!(installed.join("articles").join("intro.md").is_file());

    // v0.4.0 over the top — a content file unique to v4 proves a full
    // replace, not a merge into the old dir.
    let manifest_v4 = sample_manifest_yaml().replace("0.3.0", "0.4.0");
    let zip_v4 = {
        use zip::write::SimpleFileOptions;
        let mut buf = Vec::new();
        {
            let mut zw = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = SimpleFileOptions::default();
            zw.start_file("manifest.yml", opts).unwrap();
            zw.write_all(manifest_v4.as_bytes()).unwrap();
            zw.start_file("articles/new-in-v4.md", opts).unwrap();
            zw.write_all(b"# New\n").unwrap();
            zw.finish().unwrap();
        }
        buf
    };
    let mut entry_v4 = sample_entry(&sha256_hex(&zip_v4));
    entry_v4.version = "0.4.0".into();
    let path_v4 = write_zip(td.path(), "v4.zip", &zip_v4);

    let installed = install_from_local_zip(&path_v4, &entry_v4, &dest_root, "1.0.0").unwrap();
    assert!(installed.join("articles").join("new-in-v4.md").is_file());
    // The old v3-only file is gone — atomic replace, not overlay.
    assert!(!installed.join("articles").join("intro.md").is_file());
    assert_no_staging_leak(&dest_root);
}

#[test]
fn per_type_baseline_is_modelled_not_global() {
    use crate::{baseline_for_type, community_pack_dir, default_pack_type_specs};

    let specs = default_pack_type_specs();
    // CP4: recipes ship a bundled baseline; editorial does not; an unknown
    // (future) type defaults to no baseline — additive-safe.
    assert!(baseline_for_type(&specs, "emulator-recipes"));
    assert!(!baseline_for_type(&specs, "editorial"));
    assert!(!baseline_for_type(&specs, "some-future-type"));

    // The community install dir is the same shape regardless of type.
    let root = Path::new("/root");
    assert_eq!(
        community_pack_dir(root, "emulator-recipes", "p"),
        root.join("emulator-recipes").join("community").join("p")
    );
}

#[test]
fn registry_and_manifest_parse_from_their_wire_formats() {
    // Registry is JSON; tolerate unknown future fields (forward-compatible).
    let json = r#"{
        "registry_version": 1,
        "updated": "2026-06-15T00:00:00Z",
        "packs": [{
            "id": "oa-editorial-baseline",
            "type": "editorial",
            "name": "OA Editorial Baseline",
            "version": "0.3.0",
            "url": "https://example.invalid/pack.zip",
            "sha256": "abc",
            "future_field": "ignored"
        }]
    }"#;
    let reg: crate::Registry = serde_json::from_str(json).unwrap();
    assert_eq!(reg.packs.len(), 1);
    assert_eq!(reg.packs[0].pack_type, "editorial");

    // Manifest is YAML.
    let m = crate::Manifest::from_yaml_bytes(sample_manifest_yaml().as_bytes()).unwrap();
    assert_eq!(m.id, "oa-editorial-baseline");
    assert_eq!(m.name, "OA Editorial Baseline");
}

/// Assert no staging scratch dir leaked under `<dest_root>/.staging/`.
fn assert_no_staging_leak(dest_root: &Path) {
    let staging = dest_root.join(".staging");
    if let Ok(rd) = std::fs::read_dir(&staging) {
        let leaked: Vec<_> = rd.flatten().map(|e| e.path()).collect();
        assert!(leaked.is_empty(), "staging dir leaked: {leaked:?}");
    }
}
