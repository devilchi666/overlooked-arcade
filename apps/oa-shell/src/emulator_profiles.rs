//! Emulator profile registry — virtual-library Phase C2
//! (docs/PLANS/launcher-abstraction.md, decision D4).
//!
//! External standalone emulators reach OA as operator-editable YAML
//! profiles under `config/emulators/<id>.yaml`, mirroring the
//! per-system descriptor pattern in [`crate::system_registry`].
//! Profiles are loaded once at startup ("reload on restart" per D4).
//!
//! Two appData-side preference files ride alongside (same shape +
//! lifecycle as `cores.json`):
//!
//! - **`emulators.json`** — `{ "<profile id>": "<binary path>" }`.
//!   The operator-set path to an existing install (Settings → Cores →
//!   External emulators). Takes precedence over the profile's own
//!   optional `binary_path` field, which Phase D's installer will
//!   write when OA manages the install itself.
//! - **`launchers.json`** — `{ "<system id>": "<profile id>" }`.
//!   The per-system default-launcher pref. A system with no entry
//!   launches through the in-process libretro pipeline — today's
//!   behavior, untouched.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use oa_core::LauncherCapabilities;

/// Expected executable filename for a profile, either a single name
/// shared across every OS (the original schema) or a per-OS map.
///
/// Schema accretion (ED4 — external-emulator-depth Slice 1): the same
/// emulator ships a different binary name per platform (`Dolphin.exe` /
/// `Dolphin.app/Contents/MacOS/Dolphin` / `dolphin-emu`), so a profile
/// may now declare a `{ windows, macos, linux }` map. A bare string
/// stays valid and applies to every OS — every shipped profile that
/// predates this field keeps parsing unchanged.
///
/// Resolution is "the entry for the OS we're running on" via
/// [`BinaryName::resolve`]. A map that omits the current OS resolves to
/// `None` — the binary-name sanity check is a warn-only soft check
/// (renamed builds and forks are legitimate), so "no expected name for
/// this OS" simply means "skip the check," never "reject."
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(untagged)]
pub enum BinaryName {
    /// One name for every platform — the original shape.
    Single(String),
    /// Per-OS names. Any subset may be present; an absent OS resolves to
    /// `None` (e.g. BizHawk has no native macOS build, so its profile
    /// omits the `macos` key).
    PerOs(BinaryNameMap),
}

/// The per-OS arm of [`BinaryName`]. Each field is optional so a profile
/// only lists the platforms the emulator actually ships on.
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
pub struct BinaryNameMap {
    #[serde(default)]
    pub windows: Option<String>,
    #[serde(default)]
    pub macos: Option<String>,
    #[serde(default)]
    pub linux: Option<String>,
}

impl BinaryName {
    /// The expected binary filename for the OS we're compiled for, or
    /// `None` when a per-OS map omits the current platform. The single
    /// string arm always resolves to `Some`.
    pub fn resolve(&self) -> Option<&str> {
        match self {
            BinaryName::Single(s) => Some(s.as_str()),
            BinaryName::PerOs(m) => m.current_os(),
        }
    }
}

impl BinaryNameMap {
    /// The entry for the OS this build targets. wasm/other platforms
    /// fall through to the Linux entry (closest "generic unix" match).
    fn current_os(&self) -> Option<&str> {
        let picked = if cfg!(target_os = "windows") {
            &self.windows
        } else if cfg!(target_os = "macos") {
            &self.macos
        } else {
            &self.linux
        };
        picked.as_deref()
    }
}

/// One `config/emulators/<id>.yaml` profile. Field set per decision D4.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct EmulatorProfile {
    /// Stable identifier (`"dolphin"`). Doubles as the launcher id and
    /// the `launchers.json` pref value. Should match the file stem.
    pub id: String,
    /// Operator-facing name ("Dolphin").
    pub display_name: String,
    /// Upstream project / team name.
    #[serde(default)]
    pub vendor: String,
    /// Where the operator downloads the emulator themselves (Phase C);
    /// Phase D's installer automates this.
    #[serde(default)]
    pub official_download_url: String,
    /// Expected executable filename (`"Dolphin.exe"`), or a per-OS map
    /// (ED4). Used to sanity-check the operator-picked binary path
    /// (warn, not reject — forks and renamed builds are legitimate).
    /// Resolve the current-OS name with [`BinaryName::resolve`].
    pub binary_name: BinaryName,
    #[serde(default)]
    pub schema_version: u32,
    /// OA system ids this emulator can run (`["gamecube"]`).
    pub supported_systems: Vec<String>,
    /// Argv template, one arg per element. `{content}` expands to the
    /// resolved game path at launch time.
    ///
    /// RESERVED SEAM (ED4 — documented, not built): the rare emulator
    /// whose file extension is genuinely ambiguous (e.g. a bare MSX
    /// `.rom`, which ares cannot always auto-detect) will eventually need
    /// a per-system disambiguation token — a `{system}` placeholder fed
    /// from a `system_aliases: { <oa id>: <emulator system name> }` map,
    /// expanded the same way `{content}` is. ares + BizHawk auto-detect
    /// every system in their shipped profiles (ED4, verified 2026-06-15),
    /// so nothing here consumes `{system}` today; this note marks where
    /// it lands when a real ambiguous case surfaces, so the schema can
    /// accrete it additively rather than be reshaped.
    pub launch_args_template: Vec<String>,
    /// Whether this emulator loads archive files (`.zip`/`.7z`) directly,
    /// without OA extracting them first. When `true`, an archived game
    /// launches by handing the **outer archive path** to the emulator (it
    /// picks/auto-loads the inner ROM itself — BizHawk and ares both do).
    /// When `false` (default), archived content is rejected on the
    /// external path, since OA does not yet extract-for-external
    /// (the in-process libretro path still extracts as before).
    #[serde(default)]
    pub accepts_archives: bool,
    /// What the external session supports of OA's QuickSettings
    /// surface. Omitted flags fail closed (all-false `Default`).
    #[serde(default)]
    pub capabilities: LauncherCapabilities,
    /// Optional install path baked into the profile (Phase D installer
    /// territory). The appData `emulators.json` pref wins over this.
    #[serde(default)]
    pub binary_path: Option<String>,
}

/// All profiles found at startup, keyed by id. Empty when no
/// `config/emulators/` directory resolves — every system then keeps
/// launching through libretro, so a missing directory is not an error.
#[derive(Debug, Default)]
pub struct EmulatorProfiles {
    by_id: BTreeMap<String, EmulatorProfile>,
}

impl EmulatorProfiles {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn get(&self, id: &str) -> Option<&EmulatorProfile> {
        self.by_id.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &EmulatorProfile> {
        self.by_id.values()
    }

    /// Walk `dir` for `*.yaml` profiles. Unparseable files are logged
    /// and skipped — one broken profile must not take down the others
    /// (or the libretro path).
    pub fn load_from_dir(dir: &Path) -> Self {
        let mut by_id = BTreeMap::new();
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                log::warn!("emulator_profiles: read_dir {}: {e}", dir.display());
                return Self::empty();
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("yaml") {
                continue;
            }
            let raw = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("emulator_profiles: read {}: {e}", path.display());
                    continue;
                }
            };
            let profile: EmulatorProfile = match serde_yaml::from_str(&raw) {
                Ok(p) => p,
                Err(e) => {
                    log::warn!("emulator_profiles: parse {}: {e}", path.display());
                    continue;
                }
            };
            if profile.schema_version > 1 {
                log::warn!(
                    "emulator_profiles: {} declares schema_version {} — newer than this build understands (1); loading anyway",
                    path.display(),
                    profile.schema_version
                );
            }
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if stem != profile.id {
                log::warn!(
                    "emulator_profiles: {} declares id '{}' (file stem wins nothing — id field is authoritative)",
                    path.display(),
                    profile.id
                );
            }
            log::info!(
                "emulator_profiles: loaded '{}' ({}) for systems {:?}",
                profile.id,
                profile.display_name,
                profile.supported_systems
            );
            by_id.insert(profile.id.clone(), profile);
        }
        Self { by_id }
    }

    /// Resolve + load the default registry. Companion to
    /// [`crate::system_registry::SystemRegistry::load_default`].
    pub fn load_default() -> Self {
        match resolve_config_emulators_dir() {
            Some(dir) => Self::load_from_dir(&dir),
            None => {
                log::info!("emulator_profiles: no config/emulators/ directory found — external launchers unavailable");
                Self::empty()
            }
        }
    }
}

/// Resolve `config/emulators/` at runtime. Same two-candidate walk as
/// [`crate::system_registry::resolve_config_systems_dir`]:
///
/// 1. `<exe_dir>/config/emulators/` — production install path.
/// 2. `<CARGO_MANIFEST_DIR>/../../config/emulators/` — source tree for
///    `cargo run` + `cargo tauri dev` + `cargo test`.
pub fn resolve_config_emulators_dir() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let p = exe_dir.join("config").join("emulators");
            if p.is_dir() {
                return Some(p);
            }
        }
    }
    let dev = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("config")
        .join("emulators");
    if dev.is_dir() {
        return Some(dev);
    }
    None
}

// ---------------------------------------------------------------------------
// appData prefs — emulators.json (binary paths) + launchers.json
// (per-system default launcher). Same read/write shape as cores.json.
// ---------------------------------------------------------------------------

/// Read `appDataDir/emulators.json` — operator-set binary paths keyed
/// by profile id. Missing or malformed yields an empty map.
pub fn read_binary_path_prefs(app_data_dir: &Path) -> BTreeMap<String, String> {
    read_json_map(&app_data_dir.join("emulators.json"))
}

pub fn write_binary_path_prefs(
    app_data_dir: &Path,
    prefs: &BTreeMap<String, String>,
) -> std::io::Result<()> {
    write_json_map(app_data_dir, "emulators.json", prefs)
}

/// Read `appDataDir/launchers.json` — per-system default-launcher pref
/// keyed by system id, value = profile id. A system with no entry uses
/// the in-process libretro launcher (today's behavior).
pub fn read_launcher_prefs(app_data_dir: &Path) -> BTreeMap<String, String> {
    read_json_map(&app_data_dir.join("launchers.json"))
}

pub fn write_launcher_prefs(
    app_data_dir: &Path,
    prefs: &BTreeMap<String, String>,
) -> std::io::Result<()> {
    write_json_map(app_data_dir, "launchers.json", prefs)
}

/// Effective binary path for a profile: the appData pref wins over the
/// profile's own optional `binary_path`. `None` = operator hasn't
/// pointed OA at an install yet.
pub fn effective_binary_path(
    profile: &EmulatorProfile,
    app_data_dir: &Path,
) -> Option<PathBuf> {
    read_binary_path_prefs(app_data_dir)
        .get(&profile.id)
        .cloned()
        .or_else(|| profile.binary_path.clone())
        .map(PathBuf::from)
}

/// Expand a launch_args_template against the resolved content path.
/// `{content}` is the only variable today; it substitutes inside an
/// arg (so `--exec={content}` works), not just as a whole element.
pub fn expand_args(template: &[String], content_path: &str) -> Vec<String> {
    template
        .iter()
        .map(|arg| arg.replace("{content}", content_path))
        .collect()
}

fn read_json_map(path: &Path) -> BTreeMap<String, String> {
    let raw = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return Default::default(),
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

fn write_json_map(
    app_data_dir: &Path,
    file_name: &str,
    prefs: &BTreeMap<String, String>,
) -> std::io::Result<()> {
    std::fs::create_dir_all(app_data_dir)?;
    let body = serde_json::to_string_pretty(prefs)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(app_data_dir.join(file_name), body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_tmp_dir(label: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "oa-emuprof-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("mkdir tmp");
        p
    }

    #[test]
    fn shipped_dolphin_profile_parses() {
        // The actual file we ship must round-trip through the loader.
        let dir = resolve_config_emulators_dir().expect("config/emulators resolves in-tree");
        let profiles = EmulatorProfiles::load_from_dir(&dir);
        let dolphin = profiles.get("dolphin").expect("dolphin profile loads");
        assert_eq!(dolphin.display_name, "Dolphin");
        // Single-string binary_name resolves to the same name on every OS.
        assert_eq!(dolphin.binary_name, BinaryName::Single("Dolphin.exe".into()));
        assert_eq!(dolphin.binary_name.resolve(), Some("Dolphin.exe"));
        assert_eq!(dolphin.supported_systems, vec!["gamecube".to_string()]);
        assert!(
            dolphin.launch_args_template.iter().any(|a| a.contains("{content}")),
            "template must reference the content path"
        );
        // D5: external = no OA capabilities in v1.
        assert_eq!(dolphin.capabilities, LauncherCapabilities::none());
        // D4: shipped profile carries no machine-specific install path.
        assert_eq!(dolphin.binary_path, None);
    }

    #[test]
    fn all_shipped_profiles_parse_and_hold_invariants() {
        // The loader skips a malformed profile with only a warn, so a
        // broken file would silently vanish rather than fail. Parse each
        // shipped *.yaml strictly here so authoring a bad profile fails
        // the build. Invariants every profile must hold:
        //   - parses against the EmulatorProfile schema
        //   - file stem == declared id (loader warns but doesn't reject;
        //     we hold the stricter line in the tree)
        //   - at least one supported system
        //   - the launch template references {content}
        //   - external profiles expose no OA capabilities in v1 (D5)
        //   - binary_name names at least one platform (Single, or a
        //     PerOs map that isn't entirely empty) — ED4 schema accretion
        let dir = resolve_config_emulators_dir().expect("config/emulators resolves in-tree");
        let mut ids = Vec::new();
        for entry in std::fs::read_dir(&dir).expect("read_dir config/emulators").flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("yaml") {
                continue;
            }
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
            let raw = std::fs::read_to_string(&path).expect("read profile");
            let profile: EmulatorProfile = serde_yaml::from_str(&raw)
                .unwrap_or_else(|e| panic!("profile {} failed to parse: {e}", path.display()));
            assert_eq!(profile.id, stem, "{}: id must match file stem", path.display());
            assert!(
                !profile.supported_systems.is_empty(),
                "{}: must declare at least one supported system",
                path.display()
            );
            assert!(
                profile.launch_args_template.iter().any(|a| a.contains("{content}")),
                "{}: launch template must reference {{content}}",
                path.display()
            );
            assert_eq!(
                profile.capabilities,
                LauncherCapabilities::none(),
                "{}: external profiles expose no OA capabilities in v1 (D5)",
                path.display()
            );
            // ED4: binary_name must name at least one platform's binary.
            match &profile.binary_name {
                BinaryName::Single(s) => assert!(
                    !s.is_empty(),
                    "{}: binary_name string must be non-empty",
                    path.display()
                ),
                BinaryName::PerOs(m) => assert!(
                    m.windows.is_some() || m.macos.is_some() || m.linux.is_some(),
                    "{}: per-OS binary_name map must list at least one platform",
                    path.display()
                ),
            }
            // Every shipped emulator ships a Windows build (the operator's
            // platform), so on Windows each profile must resolve a name.
            #[cfg(target_os = "windows")]
            assert!(
                profile.binary_name.resolve().is_some_and(|n| !n.is_empty()),
                "{}: must resolve a Windows binary name",
                path.display()
            );
            ids.push(stem);
        }
        assert!(
            ids.len() >= 9,
            "expected the shipped roster (dolphin + the verified batch + ares/bizhawk), found {}",
            ids.len()
        );
        // The two ED4 multi-system additions must be present and parse.
        for must in ["ares", "bizhawk"] {
            assert!(ids.iter().any(|i| i == must), "missing shipped profile '{must}'");
        }
    }

    #[test]
    fn binary_name_accepts_single_string_and_per_os_map() {
        // Single string — the original, backward-compatible shape.
        let single: EmulatorProfile = serde_yaml::from_str(
            r#"
id: single
display_name: Single
binary_name: foo.exe
supported_systems: [gamecube]
launch_args_template: ["{content}"]
"#,
        )
        .expect("parse single-string binary_name");
        assert_eq!(single.binary_name, BinaryName::Single("foo.exe".into()));
        assert_eq!(single.binary_name.resolve(), Some("foo.exe"));

        // Per-OS map — the ED4 accretion.
        let per_os: EmulatorProfile = serde_yaml::from_str(
            r#"
id: peros
display_name: Per OS
binary_name:
  windows: foo.exe
  macos: Foo.app/Contents/MacOS/Foo
  linux: foo
supported_systems: [gamecube]
launch_args_template: ["{content}"]
"#,
        )
        .expect("parse per-OS binary_name map");
        let expected = if cfg!(target_os = "windows") {
            Some("foo.exe")
        } else if cfg!(target_os = "macos") {
            Some("Foo.app/Contents/MacOS/Foo")
        } else {
            Some("foo")
        };
        assert_eq!(per_os.binary_name.resolve(), expected);
    }

    #[test]
    fn per_os_binary_name_omitting_current_os_resolves_none() {
        // BizHawk-shaped: Windows + Linux only, no macOS build. On macOS
        // resolve() is None (the soft name-check is then skipped).
        let profile: EmulatorProfile = serde_yaml::from_str(
            r#"
id: winlin
display_name: Win+Linux only
binary_name:
  windows: EmuHawk.exe
  linux: EmuHawkMono.sh
supported_systems: [nes]
launch_args_template: ["{content}"]
"#,
        )
        .expect("parse");
        #[cfg(target_os = "windows")]
        assert_eq!(profile.binary_name.resolve(), Some("EmuHawk.exe"));
        #[cfg(target_os = "linux")]
        assert_eq!(profile.binary_name.resolve(), Some("EmuHawkMono.sh"));
        #[cfg(target_os = "macos")]
        assert_eq!(profile.binary_name.resolve(), None);
    }

    #[test]
    fn omitted_capabilities_fail_closed() {
        let yaml = r#"
id: testemu
display_name: Test Emu
binary_name: test.exe
supported_systems: [gamecube]
launch_args_template: ["{content}"]
"#;
        let p: EmulatorProfile = serde_yaml::from_str(yaml).expect("parse");
        assert_eq!(p.capabilities, LauncherCapabilities::none());
        assert_eq!(p.binary_path, None);
        assert_eq!(p.vendor, "");
        // accepts_archives defaults false — an emulator must opt in before
        // OA hands it an archive instead of an extracted ROM.
        assert!(!p.accepts_archives);
    }

    #[test]
    fn archive_capable_profiles_opt_in() {
        // The two emulators that load archives natively must declare it,
        // or the external launch path will reject archived games (the
        // operator's first BizHawk playtest hit exactly this).
        let dir = resolve_config_emulators_dir().expect("config/emulators resolves in-tree");
        let profiles = EmulatorProfiles::load_from_dir(&dir);
        for id in ["ares", "bizhawk"] {
            assert!(
                profiles.get(id).expect("profile loads").accepts_archives,
                "{id} must set accepts_archives: true (it opens archives natively)"
            );
        }
        // The dedicated single-system emulators don't claim archive support
        // here — left false until each is verified.
        assert!(
            !profiles.get("dolphin").expect("dolphin loads").accepts_archives,
            "dolphin not opted into archive passthrough"
        );
    }

    #[test]
    fn expand_args_substitutes_inside_elements() {
        let template = vec!["--batch".to_string(), "--exec={content}".to_string()];
        let args = expand_args(&template, r"G:\gc\Melee.iso");
        assert_eq!(args, vec!["--batch".to_string(), r"--exec=G:\gc\Melee.iso".to_string()]);
    }

    #[test]
    fn binary_path_pref_wins_over_profile_field() {
        let dir = fresh_tmp_dir("binpath");
        let profile: EmulatorProfile = serde_yaml::from_str(
            r#"
id: testemu
display_name: Test Emu
binary_name: test.exe
supported_systems: [gamecube]
launch_args_template: ["{content}"]
binary_path: C:/from-profile/test.exe
"#,
        )
        .expect("parse");
        // No pref → profile field.
        assert_eq!(
            effective_binary_path(&profile, &dir),
            Some(PathBuf::from("C:/from-profile/test.exe"))
        );
        // Pref set → pref wins.
        let mut prefs = BTreeMap::new();
        prefs.insert("testemu".to_string(), "C:/operator/test.exe".to_string());
        write_binary_path_prefs(&dir, &prefs).expect("write");
        assert_eq!(
            effective_binary_path(&profile, &dir),
            Some(PathBuf::from("C:/operator/test.exe"))
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn launcher_prefs_round_trip() {
        let dir = fresh_tmp_dir("launchers");
        assert!(read_launcher_prefs(&dir).is_empty(), "missing file = empty map");
        let mut prefs = BTreeMap::new();
        prefs.insert("gamecube".to_string(), "dolphin".to_string());
        write_launcher_prefs(&dir, &prefs).expect("write");
        let back = read_launcher_prefs(&dir);
        assert_eq!(back.get("gamecube").map(String::as_str), Some("dolphin"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
