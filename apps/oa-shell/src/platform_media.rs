//! Per-system "platform media" — hardware photos, controllers,
//! marquees, wheel art, banners. One image per slot per system (not
//! per ROM), so the data model is `Option<MediaVariant>` per slot
//! rather than the `Vec<MediaVariant>` shape `GameMedia` uses for
//! regional variants.
//!
//! Storage:
//! - File: `media/platform/<slot>/<systemId>.<ext>`
//! - Index: `<data_dir>/library/platform-media.json`
//!   (`BTreeMap<system_id, PlatformMedia>`)
//!
//! Same atomic-write + .corrupt-backup pattern as `media::write_media_db`.
//!
//! Phase 6 (2026-05-23) of the media-taxonomy plan. Operator surfaces
//! these via the new PlatformMediaDialog frontend; the kiosk shell
//! (separate work stream) will consume `wheel/<systemId>.png` for
//! its tile UI.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::media::{atomic_write_bytes_pub, MediaSource, MediaVariant};

/// 9 platform-level media slots — hardware photos, controllers,
/// banners, etc. One image per slot per system. Field names match
/// the kebab-case folder names exactly (see slot_kebab_name).
#[derive(Serialize, Deserialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlatformMedia {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub banner: Option<MediaVariant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clear_logo: Option<MediaVariant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub console: Option<MediaVariant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controller: Option<MediaVariant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fanart: Option<MediaVariant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marquee: Option<MediaVariant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub photo: Option<MediaVariant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wheel: Option<MediaVariant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<MediaVariant>,
}

/// Per-system slot identifier. Maps to a PlatformMedia field +
/// kebab-case folder name exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlatformSlot {
    Banner,
    ClearLogo,
    Console,
    Controller,
    Fanart,
    Marquee,
    Photo,
    Wheel,
    Background,
}

impl PlatformSlot {
    /// Every slot in stable order — used by tests + future iteration
    /// loops (e.g. a "clear all platform media for system X" flow).
    #[allow(dead_code)]
    pub const ALL: &'static [PlatformSlot] = &[
        PlatformSlot::Banner,
        PlatformSlot::ClearLogo,
        PlatformSlot::Console,
        PlatformSlot::Controller,
        PlatformSlot::Fanart,
        PlatformSlot::Marquee,
        PlatformSlot::Photo,
        PlatformSlot::Wheel,
        PlatformSlot::Background,
    ];

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "banner"     => Some(PlatformSlot::Banner),
            "clear-logo" => Some(PlatformSlot::ClearLogo),
            "console"    => Some(PlatformSlot::Console),
            "controller" => Some(PlatformSlot::Controller),
            "fanart"     => Some(PlatformSlot::Fanart),
            "marquee"    => Some(PlatformSlot::Marquee),
            "photo"      => Some(PlatformSlot::Photo),
            "wheel"      => Some(PlatformSlot::Wheel),
            "background" => Some(PlatformSlot::Background),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            PlatformSlot::Banner     => "banner",
            PlatformSlot::ClearLogo  => "clear-logo",
            PlatformSlot::Console    => "console",
            PlatformSlot::Controller => "controller",
            PlatformSlot::Fanart     => "fanart",
            PlatformSlot::Marquee    => "marquee",
            PlatformSlot::Photo      => "photo",
            PlatformSlot::Wheel      => "wheel",
            PlatformSlot::Background => "background",
        }
    }

    fn get<'a>(&self, pm: &'a PlatformMedia) -> &'a Option<MediaVariant> {
        match self {
            PlatformSlot::Banner     => &pm.banner,
            PlatformSlot::ClearLogo  => &pm.clear_logo,
            PlatformSlot::Console    => &pm.console,
            PlatformSlot::Controller => &pm.controller,
            PlatformSlot::Fanart     => &pm.fanart,
            PlatformSlot::Marquee    => &pm.marquee,
            PlatformSlot::Photo      => &pm.photo,
            PlatformSlot::Wheel      => &pm.wheel,
            PlatformSlot::Background => &pm.background,
        }
    }

    fn set(&self, pm: &mut PlatformMedia, v: Option<MediaVariant>) {
        match self {
            PlatformSlot::Banner     => pm.banner = v,
            PlatformSlot::ClearLogo  => pm.clear_logo = v,
            PlatformSlot::Console    => pm.console = v,
            PlatformSlot::Controller => pm.controller = v,
            PlatformSlot::Fanart     => pm.fanart = v,
            PlatformSlot::Marquee    => pm.marquee = v,
            PlatformSlot::Photo      => pm.photo = v,
            PlatformSlot::Wheel      => pm.wheel = v,
            PlatformSlot::Background => pm.background = v,
        }
    }
}

pub type PlatformMediaDb = BTreeMap<String /* system_id */, PlatformMedia>;

/// Shared in-memory PlatformMediaDb + the resolved app-data-dir, both
/// for the protocol handler's read path and command handlers.
pub struct PlatformMediaState {
    pub db: Arc<RwLock<PlatformMediaDb>>,
    pub app_data_dir: PathBuf,
}

// ---- persistence ----

fn db_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("library").join("platform-media.json")
}

pub fn read_db(app_data_dir: &Path) -> PlatformMediaDb {
    let p = db_path(app_data_dir);
    match std::fs::read(&p) {
        Ok(bytes) => match serde_json::from_slice::<PlatformMediaDb>(&bytes) {
            Ok(db) => db,
            Err(e) => {
                // Same .corrupt-backup pattern as media::read_media_db
                // — never silently wipe operator state on parse failure.
                log::warn!(
                    "oa-shell: platform-media.json malformed ({e:?}); starting empty"
                );
                let mut backup = p.as_os_str().to_owned();
                backup.push(".corrupt");
                if let Err(e) = std::fs::rename(&p, std::path::Path::new(&backup)) {
                    log::warn!("oa-shell: platform-media.json backup-rename failed: {e}");
                }
                PlatformMediaDb::new()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => PlatformMediaDb::new(),
        Err(e) => {
            log::warn!(
                "oa-shell: platform-media.json read failed ({e:?}); starting empty"
            );
            PlatformMediaDb::new()
        }
    }
}

pub fn write_db(app_data_dir: &Path, db: &PlatformMediaDb) -> std::io::Result<()> {
    let p = db_path(app_data_dir);
    let bytes = serde_json::to_vec_pretty(db)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    atomic_write_bytes_pub(&p, &bytes)
}

// ---- Tauri commands ----

#[tauri::command]
pub fn get_platform_media_index(
    state: tauri::State<'_, PlatformMediaState>,
) -> PlatformMediaDb {
    state.db.read().map(|db| db.clone()).unwrap_or_default()
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn set_platform_media(
    systemId: String,
    slot: String,
    sourcePath: String,
    state: tauri::State<'_, PlatformMediaState>,
    app: tauri::AppHandle,
) -> Result<PlatformMedia, String> {
    let slot = PlatformSlot::parse(&slot)
        .ok_or_else(|| format!("unknown platform slot: {slot}"))?;
    let source = std::path::PathBuf::from(&sourcePath);
    let bytes = std::fs::read(&source)
        .map_err(|e| format!("read source {}: {e}", source.display()))?;
    let (_format, ext) = crate::media::detect_format_pub(&bytes)?;
    let sha = crate::media::sha1_hex_pub(&bytes);

    // Canonical path: media/platform/<slot>/<systemId>.<ext>.
    // systemId is an internal slug (registry-controlled) so it's safe
    // as-is; the source ext is one of png/jpg/webp from detect_format.
    let cover_rel = format!("media/platform/{}/{}.{}", slot.as_str(), systemId, ext);
    let cover_abs = state.app_data_dir.join(&cover_rel);
    if let Some(parent) = cover_abs.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir slot dir: {e}"))?;
    }
    std::fs::write(&cover_abs, &bytes).map_err(|e| format!("write cover: {e}"))?;

    let variant = MediaVariant {
        source: MediaSource::Manual,
        region: None,
        path: cover_rel,
        thumb_path: None, // platform media uses the full file directly; no thumb generation
        width: None,
        height: None,
        sha1: Some(sha),
        bytes: Some(bytes.len() as u64),
    };

    let updated = {
        let mut db = state.db.write().map_err(|_| "platform-media lock poisoned".to_string())?;
        let pm = db.entry(systemId.clone()).or_insert_with(PlatformMedia::default);
        slot.set(pm, Some(variant));
        let snap = pm.clone();
        write_db(&state.app_data_dir, &db)
            .map_err(|e| format!("write platform-media.json: {e}"))?;
        snap
    };
    use tauri::Emitter;
    let _ = app.emit(
        "oa://platform-media-updated",
        serde_json::json!({ "systemId": &systemId, "slot": slot.as_str(), "media": &updated }),
    );
    log::info!(
        "oa-shell: set_platform_media({systemId}, {}) from {}",
        slot.as_str(), source.display()
    );
    Ok(updated)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn clear_platform_media(
    systemId: String,
    slot: String,
    state: tauri::State<'_, PlatformMediaState>,
    app: tauri::AppHandle,
) -> Result<PlatformMedia, String> {
    let slot = PlatformSlot::parse(&slot)
        .ok_or_else(|| format!("unknown platform slot: {slot}"))?;
    let (updated, removed_path) = {
        let mut db = state.db.write().map_err(|_| "platform-media lock poisoned".to_string())?;
        let pm = db.entry(systemId.clone()).or_insert_with(PlatformMedia::default);
        let prior = slot.get(pm).clone();
        slot.set(pm, None);
        let snap = pm.clone();
        write_db(&state.app_data_dir, &db)
            .map_err(|e| format!("write platform-media.json: {e}"))?;
        (snap, prior.map(|v| v.path))
    };
    // Best-effort file delete; ignore failure (the db is already updated,
    // and the file is orphan-cleanable via the media folder browse view).
    if let Some(rel) = removed_path {
        let abs = state.app_data_dir.join(&rel);
        if abs.exists() {
            if let Err(e) = std::fs::remove_file(&abs) {
                log::warn!("oa-shell: clear_platform_media remove {}: {e}", abs.display());
            }
        }
    }
    use tauri::Emitter;
    let _ = app.emit(
        "oa://platform-media-updated",
        serde_json::json!({ "systemId": &systemId, "slot": slot.as_str(), "media": &updated }),
    );
    log::info!("oa-shell: clear_platform_media({systemId}, {})", slot.as_str());
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_slot_parse_round_trip() {
        for &s in PlatformSlot::ALL {
            assert_eq!(PlatformSlot::parse(s.as_str()), Some(s));
        }
        // Locks the count — bumping the enum forces an ALL update.
        assert_eq!(PlatformSlot::ALL.len(), 9);
    }

    #[test]
    fn platform_slot_parse_rejects_unknown() {
        assert_eq!(PlatformSlot::parse("box-front"), None); // wrong family
        assert_eq!(PlatformSlot::parse(""), None);
        // Case-sensitive — typo protection.
        assert_eq!(PlatformSlot::parse("Banner"), None);
    }

    #[test]
    fn platform_media_default_is_all_none() {
        let pm = PlatformMedia::default();
        for &slot in PlatformSlot::ALL {
            assert!(slot.get(&pm).is_none(), "{:?} should default to None", slot);
        }
    }

    #[test]
    fn platform_slot_get_set_round_trip_every_slot() {
        for &slot in PlatformSlot::ALL {
            let mut pm = PlatformMedia::default();
            let v = MediaVariant {
                source: MediaSource::Manual,
                region: None,
                path: format!("media/platform/{}/test.png", slot.as_str()),
                thumb_path: None,
                width: None, height: None,
                sha1: Some(format!("sha-{}", slot.as_str())),
                bytes: None,
            };
            slot.set(&mut pm, Some(v.clone()));
            let read_back = slot.get(&pm).clone().expect("read back");
            assert_eq!(read_back.path, v.path);
            // Clearing works.
            slot.set(&mut pm, None);
            assert!(slot.get(&pm).is_none());
        }
    }

    #[test]
    fn platform_media_serializes_to_camel_case() {
        let mut pm = PlatformMedia::default();
        pm.clear_logo = Some(MediaVariant {
            source: MediaSource::Manual,
            region: None,
            path: "media/platform/clear-logo/genesis.png".into(),
            thumb_path: None,
            width: None, height: None, sha1: None, bytes: None,
        });
        let json = serde_json::to_string(&pm).expect("serialize");
        assert!(json.contains(r#""clearLogo""#));
        // Unset slots skipped via skip_serializing_if.
        assert!(!json.contains(r#""banner""#));
        assert!(!json.contains(r#""wheel""#));
    }

    #[test]
    fn platform_media_legacy_unknown_field_ignored() {
        // Forward-compat: an unknown future slot in the JSON shouldn't
        // block parsing (we ignore unknown fields by default).
        let json = r#"{
            "clearLogo": {"source":{"kind":"manual"},"path":"x.png"},
            "futureSlot": {"source":{"kind":"manual"},"path":"y.png"}
        }"#;
        let pm: PlatformMedia = serde_json::from_str(json).expect("parses");
        assert!(pm.clear_logo.is_some());
    }

    fn fresh_tmp_dir(label: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "oa-pm-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("mkdir tmp");
        p
    }

    #[test]
    fn read_db_returns_empty_when_missing() {
        let dir = fresh_tmp_dir("read-missing");
        let db = read_db(&dir);
        assert!(db.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_then_read_round_trip() {
        let dir = fresh_tmp_dir("write-read");
        let mut db: PlatformMediaDb = Default::default();
        let mut pm = PlatformMedia::default();
        pm.wheel = Some(MediaVariant {
            source: MediaSource::Manual,
            region: None,
            path: "media/platform/wheel/genesis.png".into(),
            thumb_path: None,
            width: None, height: None,
            sha1: Some("test".into()), bytes: None,
        });
        db.insert("genesis".into(), pm);

        write_db(&dir, &db).expect("write");
        let read = read_db(&dir);
        assert_eq!(read.len(), 1);
        assert_eq!(
            read.get("genesis").unwrap().wheel.as_ref().unwrap().path,
            "media/platform/wheel/genesis.png"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_json_backs_up_and_returns_empty() {
        let dir = fresh_tmp_dir("corrupt");
        let p = db_path(&dir);
        std::fs::create_dir_all(p.parent().unwrap()).expect("mkdir");
        std::fs::write(&p, b"{ not json").expect("seed corrupt");
        let db = read_db(&dir);
        assert!(db.is_empty());
        // Backup file present.
        let mut backup = p.as_os_str().to_owned();
        backup.push(".corrupt");
        assert!(std::path::Path::new(&backup).exists(), "corrupt backup should exist");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
