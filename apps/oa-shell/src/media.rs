//! Game-media data model + persistence + protocol delivery.
//!
//! - On-disk: `appDataDir/library/media.json` (MediaDb) + `media-prefs.json`
//!   (region priority). Cover bytes live separately under `appDataDir/media/`.
//! - In-memory: `Arc<RwLock<MediaDb>>` + `Arc<RwLock<MediaPrefs>>` in MediaState.
//! - Delivery to the WebView: the `oa-media://` custom protocol handler
//!   (registered in main.rs) reads through the same lock, no whole-db clones.
//! - Mutation: commands acquire write lock + flush to disk + emit
//!   `oa://media-updated` so the frontend can re-fetch the changed entry.
//!
//! Designed to scale to LaunchBox/BigBox parity — additional media kinds
//! (snap/title/cart/disc/video) and game metadata (year/genre/players) can
//! land without rewriting the data model. v1 ships boxart only.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GameMedia {
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub boxart: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub snap: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub title: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub cart: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub disc: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub selected: Option<SelectedMedia>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub metadata: Option<GameMetadata>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MediaVariant {
    pub source: MediaSource,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub region: Option<Region>,
    /// appData-relative path (e.g. `media/covers/tg16/rom-abc.png`).
    pub path: String,
    /// 300px-wide WebP thumbnail under `media/thumbs/<systemId>/<sha1[..16]>.webp`.
    /// Optional — the full image is the fallback if no thumb has been generated.
    #[serde(default, skip_serializing_if = "Option::is_none")] pub thumb_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub sha1: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub bytes: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum MediaSource {
    Manual,
    LibretroThumbnails,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(untagged)]
pub enum Region {
    /// Canonical: "USA", "Japan", "Europe", "World".
    Known(KnownRegion),
    /// Free-form for less-common regions (e.g. "Asia", "Brazil").
    Other(String),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum KnownRegion { USA, Japan, Europe, World }

impl Region {
    pub fn as_str(&self) -> String {
        match self {
            Region::Known(KnownRegion::USA) => "USA".into(),
            Region::Known(KnownRegion::Japan) => "Japan".into(),
            Region::Known(KnownRegion::Europe) => "Europe".into(),
            Region::Known(KnownRegion::World) => "World".into(),
            Region::Other(s) => s.clone(),
        }
    }

    pub fn parse(s: &str) -> Region {
        match s {
            "USA" => Region::Known(KnownRegion::USA),
            "Japan" => Region::Known(KnownRegion::Japan),
            "Europe" => Region::Known(KnownRegion::Europe),
            "World" => Region::Known(KnownRegion::World),
            other => Region::Other(other.to_string()),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct SelectedMedia {
    #[serde(default, skip_serializing_if = "Option::is_none")] pub boxart_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub snap_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub title_index: Option<usize>,
}

/// Placeholder for the LaunchBox-tier metadata layer. Lives here so the
/// data file already carries the shape — no migration needed when we wire
/// the UI for it.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct GameMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")] pub year: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub genre: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub developer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub publisher: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub players: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub description: Option<String>,
}

pub type MediaDb = BTreeMap<String /*rom_id*/, GameMedia>;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MediaPrefs {
    /// Ordered region priority used at lookup time when no pinned variant is set.
    /// First match wins. Default reflects a Western user audience.
    pub region_priority: Vec<String>,
    /// Which media kinds the libretro-thumbnails sync should fetch per ROM.
    /// Default is all three slottable kinds — Game Info modal needs snap +
    /// title to feel complete. Users on metered connections can drop the
    /// extras to cut sync bandwidth ~3×.
    #[serde(default = "default_kinds_to_fetch")]
    pub kinds_to_fetch: Vec<String>,
    /// When true, the media sync only fetches artwork for ROMs that
    /// have been hash-identified (i.e. games whose sha1 matched an
    /// entry in rom_hashes — that's how we learn the canonical title
    /// libretro-thumbnails actually keys its boxarts on). When false,
    /// unidentified games fall through to the fuzzy filename matcher
    /// at the high-confidence threshold (0.95) — fewer mismatches than
    /// the old 0.85, but still false-positive prone for repacked or
    /// regional names. Default on because the fuzzy fallback was the
    /// source of the "wrong art on the wrong game" complaints.
    #[serde(default = "default_only_sync_identified")]
    pub only_sync_identified: bool,
}

fn default_kinds_to_fetch() -> Vec<String> {
    vec!["boxart".into(), "snap".into(), "title".into()]
}

fn default_only_sync_identified() -> bool { true }

impl Default for MediaPrefs {
    fn default() -> Self {
        Self {
            region_priority: vec!["USA".into(), "World".into(), "Europe".into(), "Japan".into()],
            kinds_to_fetch: default_kinds_to_fetch(),
            only_sync_identified: default_only_sync_identified(),
        }
    }
}

/// Tauri state singleton. `app.manage(MediaState { ... })` once in setup;
/// commands and the protocol handler grab clones of the inner Arcs.
pub struct MediaState {
    pub db: Arc<RwLock<MediaDb>>,
    pub prefs: Arc<RwLock<MediaPrefs>>,
    pub app_data_dir: PathBuf,
}

// ---- persistence ----

fn media_db_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("library").join("media.json")
}

fn media_prefs_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("library").join("media-prefs.json")
}

pub fn read_media_db(app_data_dir: &Path) -> MediaDb {
    let p = media_db_path(app_data_dir);
    match std::fs::read(&p) {
        Ok(bytes) => match serde_json::from_slice::<MediaDb>(&bytes) {
            Ok(db) => db,
            Err(e) => {
                log::warn!("oa-shell: media.json malformed ({e:?}); starting empty");
                MediaDb::new()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => MediaDb::new(),
        Err(e) => {
            log::warn!("oa-shell: media.json read failed ({e:?}); starting empty");
            MediaDb::new()
        }
    }
}

pub fn write_media_db(app_data_dir: &Path, db: &MediaDb) -> std::io::Result<()> {
    let p = media_db_path(app_data_dir);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(db)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&p, &bytes)
}

pub fn read_media_prefs(app_data_dir: &Path) -> MediaPrefs {
    let p = media_prefs_path(app_data_dir);
    match std::fs::read(&p) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
        Err(_) => MediaPrefs::default(),
    }
}

pub fn write_media_prefs(app_data_dir: &Path, prefs: &MediaPrefs) -> std::io::Result<()> {
    let p = media_prefs_path(app_data_dir);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(prefs)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&p, &bytes)
}

// ---- active-variant resolver ----

/// Index into `variants` for the variant that should display by default.
/// Honors a pinned selection first (out-of-range pins are ignored), then
/// walks region priority, falling back to 0.
pub fn resolve_active_index(
    variants: &[MediaVariant],
    pinned: Option<usize>,
    region_priority: &[String],
) -> Option<usize> {
    if variants.is_empty() {
        return None;
    }
    if let Some(i) = pinned {
        if i < variants.len() {
            return Some(i);
        }
    }
    for region in region_priority {
        for (i, v) in variants.iter().enumerate() {
            let v_region = v.region.as_ref().map(|r| r.as_str()).unwrap_or_default();
            if v_region == *region {
                return Some(i);
            }
        }
    }
    Some(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn variant(region: Option<&str>) -> MediaVariant {
        MediaVariant {
            source: MediaSource::LibretroThumbnails,
            region: region.map(Region::parse),
            path: "x".into(),
            thumb_path: None,
            width: None,
            height: None,
            sha1: None,
            bytes: None,
        }
    }

    #[test]
    fn resolve_pinned_wins() {
        let variants = vec![variant(Some("Japan")), variant(Some("USA"))];
        let priority: Vec<String> = vec!["USA".into()];
        assert_eq!(resolve_active_index(&variants, Some(0), &priority), Some(0));
    }

    #[test]
    fn resolve_pinned_out_of_range_falls_through() {
        let variants = vec![variant(Some("Japan")), variant(Some("USA"))];
        let priority: Vec<String> = vec!["USA".into()];
        assert_eq!(resolve_active_index(&variants, Some(99), &priority), Some(1));
    }

    #[test]
    fn resolve_region_priority_picks_first_match() {
        let variants = vec![variant(Some("Japan")), variant(Some("Europe")), variant(Some("USA"))];
        let priority: Vec<String> = vec!["USA".into(), "Europe".into()];
        assert_eq!(resolve_active_index(&variants, None, &priority), Some(2));
    }

    #[test]
    fn resolve_fallback_index_zero_when_no_region_match() {
        let variants = vec![variant(Some("Japan")), variant(Some("Brazil"))];
        let priority: Vec<String> = vec!["USA".into()];
        assert_eq!(resolve_active_index(&variants, None, &priority), Some(0));
    }

    #[test]
    fn resolve_empty_returns_none() {
        let variants: Vec<MediaVariant> = vec![];
        let priority: Vec<String> = vec!["USA".into()];
        assert_eq!(resolve_active_index(&variants, None, &priority), None);
    }

    /// Every system registered in `bindings.rs` dispatch must have an
    /// explicit decision recorded in `repos_for_system_id` — either a
    /// non-empty slice of libretro-thumbnails repo names, or a listing
    /// in `NO_REPO_SYSTEMS` below with the reason. Forgetting this
    /// leaves new-system cover sync silently no-op (with a warn-level
    /// log explaining why).
    ///
    /// New core onboarding: find the system under
    /// https://github.com/libretro-thumbnails and add the arm to
    /// `repos_for_system_id`. If the system genuinely has no repo,
    /// add it to `NO_REPO_SYSTEMS` here with the reason.
    #[test]
    fn every_onboarded_system_has_an_explicit_thumbnails_decision() {
        // Keep in sync with `bindings.rs` test fixtures.
        const ONBOARDED_SYSTEMS: &[&str] = &[
            "tg16", "pce-cd", "lynx", "nes", "snes", "mame", "atari7800", "genesis",
            "segacd", "sega32x", "saturn", "psx",
            "neogeo", "neocd", "ngp",
            "jaguar", "3do", "pcfx",
            "n64", "gamecube", "dreamcast",
            "psp", "ps2", "nds",
            "sms", "gamegear", "gb", "gba", "2600",
            "coleco", "intv", "o2", "channelf",
            "vectrex", "virtualboy", "wonderswan",
            "5200", "pokemini",
        ];
        // Systems whose repos_for_system_id returns the empty slice on
        // purpose.
        const NO_REPO_SYSTEMS: &[&str] = &[
            // (none today — every onboarded system has a thumbnails repo)
        ];
        for sys in ONBOARDED_SYSTEMS {
            let repos = super::repos_for_system_id(sys);
            if NO_REPO_SYSTEMS.contains(sys) {
                assert!(
                    repos.is_empty(),
                    "{sys} is in NO_REPO_SYSTEMS but repos_for_system_id returned {} repos — pick one",
                    repos.len()
                );
            } else {
                assert!(
                    !repos.is_empty(),
                    "{sys} is onboarded but repos_for_system_id returned an empty slice. \
                     Add an arm pointing at the libretro-thumbnails repo name(s)."
                );
            }
        }
    }

    /// Multi-variant systems (gb = DMG + CGB; wonderswan = WS + WS
    /// Color) must return both repos so a Color-only title still
    /// resolves covers. Lock in the expected repo names so a future
    /// edit doesn't silently drop one half.
    #[test]
    fn multi_variant_systems_return_both_repos() {
        let gb = super::repos_for_system_id("gb");
        assert_eq!(
            gb,
            &["Nintendo_-_Game_Boy", "Nintendo_-_Game_Boy_Color"],
            "gb must return DMG + CGB repos (DMG first as primary)"
        );

        let ws = super::repos_for_system_id("wonderswan");
        assert_eq!(
            ws,
            &["Bandai_-_WonderSwan", "Bandai_-_WonderSwan_Color"],
            "wonderswan must return WS + WS Color repos (mono first as primary)"
        );
    }

    /// Single-repo systems must still return exactly one repo — guard
    /// against accidental empty-slice or duplicate-entry regressions.
    #[test]
    fn single_repo_systems_return_one_repo() {
        for sys in &["nes", "snes", "n64", "psx", "saturn", "dreamcast", "gba"] {
            let repos = super::repos_for_system_id(sys);
            assert_eq!(
                repos.len(),
                1,
                "{sys} is a single-repo system but returned {} repos",
                repos.len()
            );
        }
    }

    /// Unknown / unrecognized system_id must return the empty slice
    /// (not panic, not return a wrong repo).
    #[test]
    fn unknown_system_returns_empty_slice() {
        assert_eq!(super::repos_for_system_id("not-a-real-system"), &[] as &[&str]);
        assert_eq!(super::repos_for_system_id(""), &[] as &[&str]);
        // Dead aliases ("wonderswan-color" used to map standalone before
        // the multi-repo refactor) must NOT silently work — only the
        // canonical `wonderswan` id should resolve.
        assert_eq!(super::repos_for_system_id("wonderswan-color"), &[] as &[&str]);
    }

    /// Helper: make a minimal `SyncRomEntry` for the GC/Wii classifier
    /// tests. Only `system_id` + `file_path` matter to `repos_for_entry`.
    fn gc_entry_with_path(path: &str) -> super::SyncRomEntry {
        super::SyncRomEntry {
            id: "test".to_string(),
            title: "Test".to_string(),
            file_path: path.to_string(),
            system_id: "gamecube".to_string(),
            sha1: None,
        }
    }

    /// `.wbfs` is Wii-exclusive → repos_for_entry routes to Wii repo
    /// without any file I/O.
    #[test]
    fn wbfs_extension_routes_to_wii_repo() {
        // Note: file doesn't exist; extension is the signal.
        let e = gc_entry_with_path("/nope/SuperMarioGalaxy.wbfs");
        assert_eq!(super::repos_for_entry(&e), &["Nintendo_-_Wii"]);
    }

    /// `.wad` is Wii-exclusive (channel format) → Wii repo.
    #[test]
    fn wad_extension_routes_to_wii_repo() {
        let e = gc_entry_with_path("/nope/MyWiiChannel.wad");
        assert_eq!(super::repos_for_entry(&e), &["Nintendo_-_Wii"]);
    }

    /// `.gcm` / `.gcz` / `.ciso` are GC-exclusive containers → GC repo
    /// without any file peek.
    #[test]
    fn gc_exclusive_extensions_route_to_gc_repo() {
        for ext in &["gcm", "gcz", "ciso"] {
            let e = gc_entry_with_path(&format!("/nope/Zelda.{ext}"));
            assert_eq!(
                super::repos_for_entry(&e),
                &["Nintendo_-_GameCube"],
                ".{ext} should route to GC"
            );
        }
    }

    /// `.iso` with a Wii console byte at offset 0 → Wii repo. The
    /// classifier peeks the first byte of the file; 'R' is Wii per
    /// Dolphin convention.
    #[test]
    fn iso_with_wii_console_byte_routes_to_wii_repo() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-gcwii-iso-wii-{}-{}.iso",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        // First byte 'R' = Wii (Wii retail disc).
        std::fs::write(&tmp, b"R\x4E\x01\x45TEST WII GAME").expect("write tmp iso");
        let e = gc_entry_with_path(tmp.to_str().unwrap());
        assert_eq!(super::repos_for_entry(&e), &["Nintendo_-_Wii"]);
        let _ = std::fs::remove_file(&tmp);
    }

    /// `.iso` with a GameCube console byte at offset 0 → GC repo.
    /// 'G' = GameCube retail; 'D' = GameCube demo.
    #[test]
    fn iso_with_gc_console_byte_routes_to_gc_repo() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-gcwii-iso-gc-{}-{}.iso",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::write(&tmp, b"GW7E\x01\x08TEST GC GAME").expect("write tmp iso");
        let e = gc_entry_with_path(tmp.to_str().unwrap());
        assert_eq!(super::repos_for_entry(&e), &["Nintendo_-_GameCube"]);
        let _ = std::fs::remove_file(&tmp);
    }

    /// Unreachable file (path doesn't exist, can't open) → falls back
    /// to GameCube. Defensive — the sync will no-match and the
    /// operator can re-sync after the dump becomes available.
    #[test]
    fn iso_unreachable_falls_back_to_gc_repo() {
        let e = gc_entry_with_path("/this/path/genuinely/does/not/exist.iso");
        assert_eq!(super::repos_for_entry(&e), &["Nintendo_-_GameCube"]);
    }

    /// Entry for a non-gamecube system_id flows through the original
    /// `repos_for_system_id` path unchanged.
    #[test]
    fn non_gamecube_entry_uses_repos_for_system_id() {
        let mut e = gc_entry_with_path("/nope/some.nes");
        e.system_id = "nes".to_string();
        assert_eq!(
            super::repos_for_entry(&e),
            &["Nintendo_-_Nintendo_Entertainment_System"]
        );

        // Multi-repo systems still return both repos via the fallback.
        e.system_id = "gb".to_string();
        e.file_path = "/nope/some.gb".to_string();
        assert_eq!(
            super::repos_for_entry(&e),
            &["Nintendo_-_Game_Boy", "Nintendo_-_Game_Boy_Color"]
        );
    }
}

// ---- oa-media:// protocol handler ----
//
// URL shape: `oa-media://localhost/<systemId>/<rom_id>/<kind>/<size>.<ext>`
// where:
//   - systemId is reserved for future cross-system routing (today the variant
//     lookup is rom_id-keyed and ignores systemId; it's in the path so the
//     URL stays self-describing for browser DevTools).
//   - rom_id matches a key in MediaDb (frontend RomEntry.id).
//   - kind is "boxart" | "snap" | "title" | "cart" | "disc".
//   - size is "thumb" (300px WebP) | "full" (original PNG/JPEG/WebP).
//   - ext can be anything; the handler reads the file's true extension to
//     pick a Content-Type. It's in the URL purely so the WebView caches
//     differently per kind/size.
//
// Lookup: find the active variant via SelectedMedia + region priority, then
// serve `appDataDir / variant.thumb_path` (size=thumb) or `appDataDir /
// variant.path` (size=full). Missing file or any failed lookup yields 404.

#[derive(Clone, Copy)]
pub enum MediaKind { Boxart, Snap, Title, Cart, Disc }

impl MediaKind {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "boxart" => Some(MediaKind::Boxart),
            "snap"   => Some(MediaKind::Snap),
            "title"  => Some(MediaKind::Title),
            "cart"   => Some(MediaKind::Cart),
            "disc"   => Some(MediaKind::Disc),
            _ => None,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            MediaKind::Boxart => "boxart",
            MediaKind::Snap   => "snap",
            MediaKind::Title  => "title",
            MediaKind::Cart   => "cart",
            MediaKind::Disc   => "disc",
        }
    }

    /// libretro-thumbnails subdirectory hosting this kind. None for kinds
    /// the libretro-thumbnails repos don't carry (cart/disc) — the sync loop
    /// silently skips those.
    fn libretro_thumbnails_subdir(&self) -> Option<&'static str> {
        match self {
            MediaKind::Boxart => Some("Named_Boxarts"),
            MediaKind::Snap   => Some("Named_Snaps"),
            MediaKind::Title  => Some("Named_Titles"),
            _ => None,
        }
    }

    fn variants<'a>(&self, gm: &'a GameMedia) -> &'a [MediaVariant] {
        match self {
            MediaKind::Boxart => &gm.boxart,
            MediaKind::Snap   => &gm.snap,
            MediaKind::Title  => &gm.title,
            MediaKind::Cart   => &gm.cart,
            MediaKind::Disc   => &gm.disc,
        }
    }

    fn variants_mut<'a>(&self, gm: &'a mut GameMedia) -> &'a mut Vec<MediaVariant> {
        match self {
            MediaKind::Boxart => &mut gm.boxart,
            MediaKind::Snap   => &mut gm.snap,
            MediaKind::Title  => &mut gm.title,
            MediaKind::Cart   => &mut gm.cart,
            MediaKind::Disc   => &mut gm.disc,
        }
    }

    fn pinned_index(&self, sel: Option<&SelectedMedia>) -> Option<usize> {
        let s = sel?;
        match self {
            MediaKind::Boxart => s.boxart_index,
            MediaKind::Snap   => s.snap_index,
            MediaKind::Title  => s.title_index,
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
enum MediaSize { Thumb, Full }

fn content_type_for(path: &Path) -> &'static str {
    match path.extension().and_then(|s| s.to_str()).map(|s| s.to_ascii_lowercase()).as_deref() {
        Some("png")  => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    }
}

fn http_404() -> tauri::http::Response<Vec<u8>> {
    tauri::http::Response::builder()
        .status(tauri::http::StatusCode::NOT_FOUND)
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(b"oa-media: not found".to_vec())
        .expect("static 404 response")
}

fn http_400(reason: &str) -> tauri::http::Response<Vec<u8>> {
    tauri::http::Response::builder()
        .status(tauri::http::StatusCode::BAD_REQUEST)
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(format!("oa-media: {reason}").into_bytes())
        .expect("static 400 response")
}

pub fn handle_uri_request(
    state: &MediaState,
    request: &tauri::http::Request<Vec<u8>>,
) -> tauri::http::Response<Vec<u8>> {
    let path = request.uri().path();
    log::debug!("oa-media: request {}", request.uri());
    // Skip the leading '/' then split. Expected: [systemId, romId, kind, size.ext]
    let segments: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    if segments.len() < 4 {
        log::warn!("oa-media: bad request path {}", path);
        return http_400("expected oa-media://localhost/<systemId>/<romId>/<kind>/<size>.<ext>");
    }
    let _system_id = segments[0];
    let rom_id = segments[1];
    let Some(kind) = MediaKind::parse(segments[2]) else {
        return http_400("unknown kind");
    };
    let size = match segments[3].split('.').next().unwrap_or("") {
        "thumb" => MediaSize::Thumb,
        "full"  => MediaSize::Full,
        _ => return http_400("size must be thumb or full"),
    };

    // Optional `?i=<N>` query param overrides the active-variant resolution
    // — used by the RegionPicker UI to fetch each variant by explicit index.
    // No `i=` query → fall through to user pin + region priority.
    let explicit_index: Option<usize> = request
        .uri()
        .query()
        .and_then(|q| {
            q.split('&')
                .find_map(|p| p.strip_prefix("i=").and_then(|s| s.parse::<usize>().ok()))
        });

    // Resolve active variant under the read locks.
    let chosen: Option<MediaVariant> = (|| {
        let db = state.db.read().ok()?;
        let prefs = state.prefs.read().ok()?;
        let gm = db.get(rom_id)?;
        let variants = kind.variants(gm);
        let idx = if let Some(i) = explicit_index {
            if i < variants.len() { i } else { return None; }
        } else {
            let pinned = kind.pinned_index(gm.selected.as_ref());
            resolve_active_index(variants, pinned, &prefs.region_priority)?
        };
        Some(variants[idx].clone())
    })();

    let Some(variant) = chosen else {
        log::debug!("oa-media: no variant for {}/{}/{}", _system_id, rom_id, segments[2]);
        return http_404();
    };

    // Pick thumb path if requested + present, else fall back to full.
    let relative = match size {
        MediaSize::Thumb => variant.thumb_path.as_deref().unwrap_or(&variant.path),
        MediaSize::Full  => &variant.path,
    };
    let abs = state.app_data_dir.join(relative);
    let bytes = match std::fs::read(&abs) {
        Ok(b) => b,
        Err(e) => {
            log::warn!("oa-media: read {} failed: {e:?}", abs.display());
            return http_404();
        }
    };
    log::debug!("oa-media: served {} ({} bytes)", abs.display(), bytes.len());

    let etag = variant.sha1.clone().unwrap_or_default();

    let mut builder = tauri::http::Response::builder()
        .status(tauri::http::StatusCode::OK)
        .header("Content-Type", content_type_for(&abs))
        .header("Cache-Control", "max-age=31536000, immutable")
        .header("Access-Control-Allow-Origin", "*");
    if !etag.is_empty() {
        builder = builder.header("ETag", format!("\"{etag}\""));
    }
    builder.body(bytes).unwrap_or_else(|_| http_404())
}

// ---- thumbnail generation + manual cover ingest ----

const THUMB_WIDTH: u32 = 300;

/// Detect the source image's container format from its bytes (no path required).
/// Recognize a Git symlink response (small, all-ASCII, no path separators
/// that would let it escape the directory). Git stores symlinks as files
/// whose content is the target path; neither raw.githubusercontent.com nor
/// github.com/.../raw/... resolves them, so we follow them ourselves.
fn looks_like_same_dir_symlink_target(bytes: &[u8]) -> bool {
    if bytes.is_empty() || bytes.len() > 512 {
        return false;
    }
    let all_printable_ascii = bytes
        .iter()
        .all(|&b| b.is_ascii_graphic() || b == b' ');
    if !all_printable_ascii {
        return false;
    }
    let s = match std::str::from_utf8(bytes) {
        Ok(s) => s.trim(),
        Err(_) => return false,
    };
    // Same-dir symlinks have no path separators. We reject `..` / nested
    // paths to keep this safe — could be extended later if libretro-thumbnails
    // ever uses cross-directory symlinks for Named_Boxarts/.
    !s.contains('/') && !s.contains('\\') && !s.contains("..") && !s.is_empty()
}

async fn download_following_symlinks(
    client: &reqwest::Client,
    repo: &str,
    subdir: &str,
    initial_filename: &str,
    max_hops: usize,
) -> Result<Vec<u8>, String> {
    let mut filename = initial_filename.to_string();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for hop in 0..=max_hops {
        if !seen.insert(filename.clone()) {
            return Err(format!("symlink loop starting from {initial_filename}"));
        }
        let url = format!(
            "https://github.com/libretro-thumbnails/{repo}/raw/master/{subdir}/{}",
            urlencoding::encode(&filename)
        );
        log::debug!("oa-shell: download hop {hop}: {url}");
        let resp = client
            .get(&url)
            .header("User-Agent", "OverlookedArcade")
            .send()
            .await
            .map_err(|e| format!("download {url}: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!("download {url} status {}", resp.status()));
        }
        let bytes = resp.bytes().await.map_err(|e| format!("download body: {e}"))?.to_vec();
        if is_image_magic(&bytes) {
            return Ok(bytes);
        }
        if looks_like_same_dir_symlink_target(&bytes) {
            let target = std::str::from_utf8(&bytes)
                .map_err(|_| "symlink target not utf8".to_string())?
                .trim()
                .to_string();
            log::info!("oa-shell: following symlink {filename} → {target}");
            filename = target;
            continue;
        }
        let preview: String = bytes
            .iter()
            .take(48)
            .map(|b| if b.is_ascii_graphic() || *b == b' ' { *b as char } else { '.' })
            .collect();
        return Err(format!(
            "non-image response from {url} ({} bytes, head=`{preview}`)",
            bytes.len()
        ));
    }
    Err(format!(
        "too many symlink hops (>{max_hops}) starting from {initial_filename}"
    ))
}

/// Sniff for the standard image-format magic bytes. Used to reject
/// "downloaded" responses that are actually HTML / symlink-target text /
/// empty before we write them to disk.
fn is_image_magic(bytes: &[u8]) -> bool {
    // PNG: 89 50 4E 47 0D 0A 1A 0A
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") { return true; }
    // JPEG: FF D8 FF
    if bytes.starts_with(b"\xff\xd8\xff") { return true; }
    // GIF: "GIF87a" / "GIF89a"
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") { return true; }
    // WebP: "RIFF" .... "WEBP"
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" { return true; }
    // BMP: "BM"
    if bytes.starts_with(b"BM") && bytes.len() > 14 { return true; }
    false
}

fn detect_format(bytes: &[u8]) -> Result<(image::ImageFormat, &'static str), String> {
    let format = image::guess_format(bytes).map_err(|e| format!("not an image: {e}"))?;
    let ext = match format {
        image::ImageFormat::Png  => "png",
        image::ImageFormat::Jpeg => "jpg",
        image::ImageFormat::WebP => "webp",
        other => return Err(format!("unsupported image format: {other:?}")),
    };
    Ok((format, ext))
}

fn sha1_hex(bytes: &[u8]) -> String {
    use sha1::{Digest, Sha1};
    Sha1::digest(bytes).iter().map(|b| format!("{:02x}", b)).collect()
}

/// Decode bytes → resize to 300px wide preserving aspect (Triangle filter — the
/// right pick for boxart at thumbnail sizes; Lanczos overshoots sharp edges) →
/// encode WebP (lossless via the `image` crate's image-webp backend) → write
/// to `dest`. Returns (decoded_width, decoded_height) for variant metadata.
fn generate_thumbnail(bytes: &[u8], dest: &Path) -> Result<(u32, u32), String> {
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| format!("read: {e}"))?;
    let img = reader.decode().map_err(|e| {
        // First 32 bytes as ASCII when printable, hex otherwise — usually
        // enough to spot "404 not found" HTML responses or truncated downloads.
        let preview: String = bytes
            .iter()
            .take(32)
            .map(|b| if b.is_ascii_graphic() || *b == b' ' { *b as char } else { '.' })
            .collect();
        format!("decode ({} bytes, head=`{}`): {e}", bytes.len(), preview)
    })?;
    let (orig_w, orig_h) = (img.width(), img.height());
    let new_w = THUMB_WIDTH.min(orig_w).max(1);
    let new_h = ((orig_h as f64) * (new_w as f64 / orig_w as f64)).round().max(1.0) as u32;
    // Force RGBA8 BEFORE resizing — image-webp 0.x only encodes Rgba<u8> /
    // Rgb<u8> cleanly. PNGs from libretro-thumbnails come in mixed color
    // types (RGB, palette, grayscale-alpha). Without this normalization
    // the encode step errors mid-sync and the variant never lands in
    // MediaDb, so the tile never gets a cover.
    let rgba: image::RgbaImage = img.to_rgba8();
    let resized = image::imageops::resize(&rgba, new_w, new_h, image::imageops::FilterType::Triangle);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir thumb dir: {e}"))?;
    }
    resized
        .save_with_format(dest, image::ImageFormat::WebP)
        .map_err(|e| format!("encode webp ({}x{} → {}x{}): {e}", orig_w, orig_h, new_w, new_h))?;
    Ok((orig_w, orig_h))
}

/// Pure logic: writes a manual cover + thumbnail to disk + mutates the in-
/// memory MediaDb. Caller is responsible for persisting `db` to disk and
/// emitting `oa://media-updated`.
fn ingest_manual_cover(
    app_data_dir: &Path,
    rom_id: &str,
    system_id: &str,
    source_path: &Path,
    db: &mut MediaDb,
) -> Result<GameMedia, String> {
    let bytes = std::fs::read(source_path).map_err(|e| format!("read source: {e}"))?;
    let (_format, ext) = detect_format(&bytes)?;
    let sha = sha1_hex(&bytes);

    let cover_rel = format!("media/covers/{system_id}/{rom_id}.{ext}");
    let cover_abs = app_data_dir.join(&cover_rel);
    if let Some(parent) = cover_abs.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir covers: {e}"))?;
    }
    std::fs::write(&cover_abs, &bytes).map_err(|e| format!("write cover: {e}"))?;

    let thumb_rel = format!("media/thumbs/{system_id}/{}.webp", &sha[..sha.len().min(16)]);
    let thumb_abs = app_data_dir.join(&thumb_rel);
    let (w, h) = if thumb_abs.exists() {
        // Content-addressed thumbnail already exists — skip the decode/encode.
        (None, None)
    } else {
        let (w, h) = generate_thumbnail(&bytes, &thumb_abs)?;
        (Some(w), Some(h))
    };

    let new_variant = MediaVariant {
        source: MediaSource::Manual,
        region: None,
        path: cover_rel,
        thumb_path: Some(thumb_rel),
        width: w,
        height: h,
        sha1: Some(sha),
        bytes: Some(bytes.len() as u64),
    };

    let entry = db.entry(rom_id.to_string()).or_insert_with(GameMedia::default);
    // Manual replaces any prior manual variant + sorts to front so it wins
    // the region-priority fallback (index 0 is the final fallback in
    // resolve_active_index). Any pinned selection is cleared so a previous
    // pinned synced variant doesn't shadow the new manual choice.
    entry.boxart.retain(|v| v.source != MediaSource::Manual);
    entry.boxart.insert(0, new_variant);
    if let Some(sel) = entry.selected.as_mut() {
        sel.boxart_index = None;
    }
    Ok(entry.clone())
}

// ---- libretro-thumbnails sync ----

use std::time::{Duration, SystemTime};

/// Per-system → libretro-thumbnails GitHub subrepos. Repos live at
/// `github.com/libretro-thumbnails/<name>` and the names mirror
/// libretro-database's `metadat/no-intro/` basenames with spaces
/// replaced by underscores.
///
/// Most systems return a single-entry slice. Multi-variant systems
/// (`gb` = DMG + CGB, `wonderswan` = WS + WS Color) return two so
/// each entry's covers get attempted against both upstream catalogs —
/// a DMG-only title resolves in the DMG repo, a Color-only title in
/// the CGB repo, both via the same OA system_id. Multi-match results
/// land as independent variants on the same library entry; OA's
/// region-priority resolution picks which to surface.
///
/// Returning an empty slice is the "skip cover sync for this system"
/// path — the sync command treats it the same as "no entries for this
/// system" but emits a warn-level log so onboarding mistakes surface.
///
/// **New-core onboarding checklist item:** add an arm here when
/// onboarding a system that has a libretro-thumbnails repo (most do
/// — check https://github.com/libretro-thumbnails). Returning the
/// empty slice makes the sync a no-op for that system rather than
/// fetching from the wrong repo.
fn repos_for_system_id(system_id: &str) -> &'static [&'static str] {
    match system_id {
        // First wave + already-onboarded systems.
        "tg16"      => &["NEC_-_PC_Engine_-_TurboGrafx_16"],
        "pce-cd"    => &["NEC_-_PC_Engine_CD_-_TurboGrafx-CD"],
        "lynx"      => &["Atari_-_Lynx"],
        "nes"       => &["Nintendo_-_Nintendo_Entertainment_System"],
        "snes"      => &["Nintendo_-_Super_Nintendo_Entertainment_System"],
        "atari7800" => &["Atari_-_7800"],
        "mame"      => &["MAME"],
        "genesis"   => &["Sega_-_Mega_Drive_-_Genesis"],
        // Sega CD / Mega-CD. libretro-thumbnails ships one combined repo
        // covering both regional namings ("Mega-CD" JP / EU + "Sega CD"
        // US) under a single hyphenated path.
        "segacd"    => &["Sega_-_Mega-CD_-_Sega_CD"],
        // Sega 32X. Single thumbnails repo covers the small library.
        "sega32x"   => &["Sega_-_32X"],
        // Sega Saturn. Single thumbnails repo covers the full retail +
        // homebrew library.
        "saturn"    => &["Sega_-_Saturn"],
        // Sony PlayStation. The libretro-thumbnails repo follows the
        // "Sony_-_PlayStation" convention used elsewhere in the catalog.
        "psx"       => &["Sony_-_PlayStation"],
        // SNK Neo Geo (AES + MVS). Single thumbnails repo covers the
        // home + arcade library.
        "neogeo"    => &["SNK_-_Neo_Geo"],
        // SNK Neo Geo CD. Separate thumbnails repo from the cart AES.
        "neocd"     => &["SNK_-_Neo_Geo_CD"],
        // SNK Neo Geo Pocket Color. The single thumbnails repo covers
        // both NGP (mono) and NGPC (color) — same convention as the
        // libretro-thumbnails Sega_-_Game_Gear repo covering JP+US sets.
        "ngp"       => &["SNK_-_Neo_Geo_Pocket_Color"],
        // Atari Jaguar. Single thumbnails repo covers the small library
        // (~50 retail releases + homebrew).
        "jaguar"    => &["Atari_-_Jaguar"],
        // 3DO Interactive Multiplayer. libretro-thumbnails repo name
        // matches "The 3DO Company" branding.
        "3do"       => &["The_3DO_Company_-_3DO"],
        // NEC PC-FX. Japan-only platform; small library (~62 retail
        // releases). Single thumbnails repo.
        "pcfx"      => &["NEC_-_PC-FX"],
        // Nintendo 64. Single thumbnails repo covers the full retail
        // library (~390 releases).
        "n64"       => &["Nintendo_-_Nintendo_64"],
        // Nintendo GameCube + Wii. Single thumbnails repo covers
        // GameCube (operator-validated thumbnail set). Wii thumbnails
        // live in a separate `Nintendo_-_Wii` repo on libretro-thumbnails;
        // Phase 2.5 polish would split the slug + cover sync to point
        // at the right repo per-game-region.
        "gamecube"  => &["Nintendo_-_GameCube"],
        // Sega Dreamcast. Single thumbnails repo covers the full retail
        // library (~620 releases).
        "dreamcast" => &["Sega_-_Dreamcast"],
        // Sony PlayStation Portable. Single thumbnails repo covers
        // the full UMD library.
        "psp"       => &["Sony_-_PlayStation_Portable"],
        // Sony PlayStation 2. Single thumbnails repo covers the
        // ~2000-game library.
        "ps2"       => &["Sony_-_PlayStation_2"],
        // Nintendo DS. Single thumbnails repo covers both DS + DSi
        // (the DSi extensions ship as separate ROMs but use the same
        // base library).
        "nds"       => &["Nintendo_-_Nintendo_DS"],
        // First-wave systems waiting to be onboarded — mapping is ready
        // so cover sync works the moment they land.
        "sms"             => &["Sega_-_Master_System_-_Mark_III"],
        "gamegear"        => &["Sega_-_Game_Gear"],
        "msx"             => &["Microsoft_-_MSX"],
        "msx2"             => &["Microsoft_-_MSX2"],
        "coleco"          => &["Coleco_-_ColecoVision"],
        "intv"            => &["Mattel_-_Intellivision"],
        "o2"              => &["Magnavox_-_Odyssey2"],
        "channelf"        => &["Fairchild_-_Channel_F"],
        "vectrex"         => &["GCE_-_Vectrex"],
        "virtualboy"      => &["Nintendo_-_Virtual_Boy"],
        // WonderSwan + WonderSwan Color share the OA system_id
        // `wonderswan`; libretro-thumbnails splits the corpus across
        // two repos. Mono titles resolve in the WS repo, Color titles
        // in the WS Color repo — both attempted, both surfaced as
        // independent variants under the same OA library entry.
        "wonderswan"      => &[
            "Bandai_-_WonderSwan",
            "Bandai_-_WonderSwan_Color",
        ],
        // Game Boy + Game Boy Color share the OA system_id `gb`;
        // libretro-thumbnails keeps them in separate repos. DMG repo
        // listed first as primary (most retail-era GB libraries are
        // DMG-heavy) but a CGB-only title resolves the CGB repo via
        // the same `gb` system_id. Documented in docs/cores/gb/DECISIONS.md.
        "gb" => &[
            "Nintendo_-_Game_Boy",
            "Nintendo_-_Game_Boy_Color",
        ],
        // Game Boy Advance — single thumbnails repo; no multi-region or
        // hardware-variant split like `gb` had.
        "gba" => &["Nintendo_-_Game_Boy_Advance"],
        // Atari 2600 — single thumbnails repo covers the full library.
        "2600" => &["Atari_-_2600"],
        "5200" => &["Atari_-_5200"],
        "pokemini" => &["Nintendo_-_Pokemon_Mini"],
        _ => &[],
    }
}

/// Resolve a `SyncRomEntry` to the libretro-thumbnails repos that
/// hold its cover art. Wrapper around `repos_for_system_id` that adds
/// per-entry discrimination for systems where one OA `system_id`
/// covers multiple thumbnails-repo targets — currently just GameCube
/// vs Wii (both ride `system_id = "gamecube"` on Dolphin, but
/// libretro-thumbnails keeps Wii art in a separate repo).
///
/// Adding a new same-system-id split: extend the match here with the
/// per-entry classifier. Default falls through to `repos_for_system_id`.
fn repos_for_entry(entry: &SyncRomEntry) -> &'static [&'static str] {
    match entry.system_id.as_str() {
        "gamecube" => {
            if is_wii_dump(&entry.file_path) {
                &["Nintendo_-_Wii"]
            } else {
                &["Nintendo_-_GameCube"]
            }
        }
        _ => repos_for_system_id(&entry.system_id),
    }
}

/// Classify a Dolphin-loadable file as a Wii dump (true) or a
/// GameCube dump (false). Three tiers:
///
/// 1. Extension-only signal — `.wbfs` and `.wad` are Wii-exclusive
///    container formats. `.gcm` and `.gcz` and `.ciso` are
///    GameCube-only.
/// 2. Header peek for `.iso` and `.rvz` — both extensions cover
///    GameCube AND Wii dumps. The disc header's byte 0 is the
///    console-ID per Dolphin convention: `'G'` / `'D'` = GameCube,
///    `'R'` / `'S'` = Wii. Reads the first byte of the file only;
///    nothing else.
/// 3. Fallback when the file can't be read (deleted, permissions,
///    network mount offline): assume GameCube. The cover sync will
///    no-match Wii titles whose dumps are unreachable and the
///    operator can refresh once the dump is back.
///
/// The audit's "GC + Wii cover sync split" item is the entry point
/// for this classifier — gives Wii titles their own thumbnail set
/// without surfacing Wii as a separate OA `system_id`.
fn is_wii_dump(file_path: &str) -> bool {
    let path = std::path::Path::new(file_path);
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase());
    match ext.as_deref() {
        // Wii-exclusive containers.
        Some("wbfs") | Some("wad") => true,
        // GameCube-exclusive containers.
        Some("gcm") | Some("gcz") | Some("ciso") => false,
        // Shared containers — peek byte 0 to discriminate.
        Some("iso") | Some("rvz") => peek_dolphin_console_byte(path)
            .map(|b| matches!(b, b'R' | b'S'))
            .unwrap_or(false),
        _ => false,
    }
}

/// Read the first byte of a Dolphin-loadable disc dump. Returns
/// `None` on I/O failure (file gone, permission denied, etc.) — the
/// caller treats that as "assume GameCube" rather than erroring out.
fn peek_dolphin_console_byte(path: &std::path::Path) -> Option<u8> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).ok()?;
    let mut buf = [0u8; 1];
    f.read_exact(&mut buf).ok()?;
    Some(buf[0])
}

/// Back-compat alias around `repos_for_system_id` for the extension-only
/// callers that pre-date the system_id field on `SyncRomEntry`. Looks
/// at the few extensions that uniquely identify a system within the
/// PC Engine family, then falls back to the TG-16 repo.
#[allow(dead_code)]
fn repo_for_extension(ext: &str) -> &'static str {
    match ext {
        "sgx" => "NEC_-_PC_Engine_SuperGrafx",
        "cue" | "chd" | "ccd" | "toc" | "m3u" | "iso" => "NEC_-_PC_Engine_CD_-_TurboGrafx-CD",
        _ => "NEC_-_PC_Engine_-_TurboGrafx_16",
    }
}

/// One ROM in the frontend's library, shipped to the sync command. We only
/// need id + title + filePath + systemId — the full RomEntry would also work
/// but staying minimal keeps the IPC payload tight at scale.
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SyncRomEntry {
    pub id: String,
    pub title: String,
    pub file_path: String,
    pub system_id: String,
    /// Optional sha1 stamped on the games row by the rom_hashes resolve
    /// flow. When set we look the hash up in rom_hashes and use the
    /// canonical name for an exact (case-insensitive) match against
    /// libretro-thumbnails — bypassing the fuzzy filename matcher
    /// entirely. Repacked / renamed / no-intro-suffix ROMs all match
    /// reliably via this path when they're hash-identified.
    #[serde(default)]
    pub sha1: Option<String>,
}

/// Per-ROM progress event, fired as each entry completes (download / cached / no-match / error).
#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SyncProgress {
    pub system_id: String,
    pub done: usize,
    pub total: usize,
    pub current_rom_title: String,
    pub last_action: String,
}

/// Final tally fired on `oa://library-sync-complete`.
#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SyncSummary {
    pub system_id: String,
    pub total: usize,
    pub matched: usize,
    pub downloaded: usize,
    pub cached: usize,
    pub unmatched: usize,
    pub errors: usize,
}

/// One libretro-thumbnails repo's per-subdirectory file listing. v1 stored
/// just Named_Boxarts/ paths; v2 adds Named_Snaps + Named_Titles so the sync
/// loop can pull all three kinds from the same git/trees response.
#[derive(serde::Serialize, serde::Deserialize, Default, Clone)]
struct RepoTree {
    boxarts: Vec<String>,
    snaps: Vec<String>,
    titles: Vec<String>,
}

impl RepoTree {
    fn for_kind(&self, kind: MediaKind) -> &[String] {
        match kind {
            MediaKind::Boxart => &self.boxarts,
            MediaKind::Snap   => &self.snaps,
            MediaKind::Title  => &self.titles,
            _ => &[],
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CachedTreeV2 {
    fetched_at_unix_secs: u64,
    tree: RepoTree,
}

const TREE_CACHE_TTL_SECS: u64 = 86_400; // 24h

fn tree_cache_path(app_data_dir: &Path, repo: &str) -> PathBuf {
    // .v2 suffix invalidates the v1 cache automatically — v1 stored only
    // Named_Boxarts/ paths and the shape is incompatible. Old files become
    // orphans on disk; user can clear the cache folder if they want them gone.
    app_data_dir
        .join("media")
        .join("cache")
        .join(format!("index-{repo}.v2.json"))
}

async fn fetch_repo_tree(client: &reqwest::Client, repo: &str) -> Result<RepoTree, String> {
    let url = format!(
        "https://api.github.com/repos/libretro-thumbnails/{repo}/git/trees/master?recursive=1"
    );
    let resp = client
        .get(&url)
        .header("User-Agent", "OverlookedArcade")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("github tree request: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("github tree status: {}", resp.status()));
    }
    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("github tree json: {e}"))?;
    let tree = json
        .get("tree")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "github tree response missing 'tree' array".to_string())?;
    let mut out = RepoTree::default();
    for node in tree.iter() {
        let Some(path) = node.get("path").and_then(|p| p.as_str()) else { continue };
        if !path.ends_with(".png") { continue; }
        if let Some(name) = path.strip_prefix("Named_Boxarts/") {
            out.boxarts.push(name.to_string());
        } else if let Some(name) = path.strip_prefix("Named_Snaps/") {
            out.snaps.push(name.to_string());
        } else if let Some(name) = path.strip_prefix("Named_Titles/") {
            out.titles.push(name.to_string());
        }
    }
    Ok(out)
}

async fn get_repo_tree_cached(
    client: &reqwest::Client,
    app_data_dir: &Path,
    repo: &str,
) -> Result<RepoTree, String> {
    let cache_path = tree_cache_path(app_data_dir, repo);
    if let Ok(bytes) = std::fs::read(&cache_path) {
        if let Ok(cached) = serde_json::from_slice::<CachedTreeV2>(&bytes) {
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            if now.saturating_sub(cached.fetched_at_unix_secs) < TREE_CACHE_TTL_SECS {
                log::info!(
                    "oa-shell: libretro-thumbnails tree for {repo} from cache ({} boxarts / {} snaps / {} titles)",
                    cached.tree.boxarts.len(), cached.tree.snaps.len(), cached.tree.titles.len()
                );
                return Ok(cached.tree);
            }
        }
    }
    let tree = fetch_repo_tree(client, repo).await?;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let cached = CachedTreeV2 { fetched_at_unix_secs: now, tree: tree.clone() };
    if let Some(parent) = cache_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(
        &cache_path,
        serde_json::to_vec_pretty(&cached).unwrap_or_default(),
    );
    log::info!(
        "oa-shell: libretro-thumbnails tree for {repo} fetched fresh ({} boxarts / {} snaps / {} titles)",
        tree.boxarts.len(), tree.snaps.len(), tree.titles.len()
    );
    Ok(tree)
}

enum SyncOutcome {
    Downloaded { variant: MediaVariant },
    Cached,
    NoMatch,
}

/// Fuzzy fallback threshold for filename → canonical-name matching when
/// the ROM has no hash-identified canonical name. Set high enough that
/// almost-but-not-quite hits ("Bonk II" vs "Bonk's Adventure", Japanese
/// vs USA cuts of the same title) get rejected rather than producing
/// wrong-art mismatches. Was 0.85 — that produced the "wrong art on
/// wrong game" complaints in the field. Identified ROMs bypass fuzzy
/// matching entirely.
const FUZZY_MATCH_THRESHOLD: f64 = 0.95;

async fn sync_single_rom(
    client: &reqwest::Client,
    parsed: &[crate::normalize::ParsedUpstream],
    repo: &str,
    kind: MediaKind,
    subdir: &str,
    app_data_dir: &Path,
    db: &Arc<RwLock<MediaDb>>,
    entry: &SyncRomEntry,
    canonical_name: Option<&str>,
) -> Result<SyncOutcome, String> {
    // 1. Resolve target match. Prefer canonical name (hash-identified) →
    // exact case-insensitive filename match in the libretro-thumbnails
    // tree. Fall back to fuzzy filename matching with the much stricter
    // 0.95 threshold when the ROM is unidentified.
    let matched_ref: Option<&crate::normalize::ParsedUpstream>;
    let match_source: &str;
    let owned_score: f64;
    if let Some(canon) = canonical_name {
        let needle = canon.to_ascii_lowercase();
        let hit = parsed.iter().find(|u| {
            std::path::Path::new(&u.filename)
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|stem| stem.to_ascii_lowercase() == needle)
                .unwrap_or(false)
        });
        if let Some(u) = hit {
            matched_ref = Some(u);
            match_source = "canonical";
            owned_score = 1.0;
        } else {
            // Canonical name didn't appear in this repo's tree (rare —
            // typically means the canonical entry exists but lacks
            // artwork in libretro-thumbnails for this kind). Skip the
            // fuzzy fallback: a hash-identified ROM with no matching
            // tree entry is "no upstream art for this kind," not "let's
            // guess again."
            log::debug!(
                "oa-shell: canonical '{canon}' not in libretro-thumbnails {repo}/{subdir} — skipping fuzzy fallback"
            );
            return Ok(SyncOutcome::NoMatch);
        }
    } else {
        // Unidentified ROM — fuzzy match.
        let rom_norm = crate::normalize::normalize_title(&entry.title);
        if rom_norm.is_empty() {
            return Ok(SyncOutcome::NoMatch);
        }
        let mut best: Option<(&crate::normalize::ParsedUpstream, f64)> = None;
        for u in parsed {
            let score = crate::normalize::match_score(&rom_norm, &u.normalized);
            match best {
                Some((_, s)) if score <= s => {}
                _ => best = Some((u, score)),
            }
        }
        match best {
            Some((u, s)) if s >= FUZZY_MATCH_THRESHOLD => {
                matched_ref = Some(u);
                match_source = "fuzzy";
                owned_score = s;
            }
            _ => return Ok(SyncOutcome::NoMatch),
        }
    }

    let matched = matched_ref.expect("matched_ref set above");
    log::debug!(
        "oa-shell: sync match {} [{}] → {} (source={match_source}, score {:.3})",
        entry.title, kind.as_str(), matched.filename, owned_score,
    );

    // 3. Build destination paths. Subdir is part of the cache path so
    //    snap/title can coexist with boxart for the same ROM without
    //    filename collisions.
    let cache_rel = format!(
        "media/cache/libretro-thumbnails/{}/{}/{}",
        entry.system_id, subdir, matched.filename
    );
    let cache_abs = app_data_dir.join(&cache_rel);

    // 4. Cache check: if file already exists AND we have a recorded variant
    //    (on the matching kind's array) whose sha1 matches the on-disk
    //    content, skip the download entirely.
    let existing_sha: Option<String> = {
        let db_read = db.read().map_err(|_| "db lock".to_string())?;
        db_read
            .get(&entry.id)
            .and_then(|gm| {
                kind.variants(gm).iter().find(|v| {
                    matches!(v.source, MediaSource::LibretroThumbnails) && v.path == cache_rel
                }).cloned()
            })
            .and_then(|v| v.sha1)
    };
    if cache_abs.is_file() {
        if let Some(prior) = existing_sha {
            if let Ok(disk) = std::fs::read(&cache_abs) {
                let disk_sha = sha1_hex(&disk);
                if disk_sha == prior {
                    return Ok(SyncOutcome::Cached);
                }
            }
        }
    }

    // 5. Download — chasing Git symlinks ourselves. Neither raw.githubusercontent
    //    nor github.com/.../raw/... resolves them server-side; they both return
    //    the symlink target as plain-text bytes. libretro-thumbnails uses
    //    symlinks heavily (regional renames, beta/final aliases, etc.), so
    //    we recognize that response and re-fetch.
    let bytes_vec = download_following_symlinks(client, repo, subdir, &matched.filename, 3).await?;
    log::debug!(
        "oa-shell: sync downloaded {}/{} ({} bytes, first4 = {:02x?})",
        subdir, matched.filename,
        bytes_vec.len(),
        bytes_vec.iter().take(4).copied().collect::<Vec<_>>(),
    );

    // 6. Hash + write file + thumbnail. The thumbnail directory mirrors the
    //    subdir so the same content-addressed sha1 in two kinds doesn't
    //    collide (rare in practice — different art for the same game — but
    //    cheap insurance).
    let sha = sha1_hex(&bytes_vec);
    if let Some(parent) = cache_abs.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir cache: {e}"))?;
    }
    std::fs::write(&cache_abs, &bytes_vec).map_err(|e| format!("write cache: {e}"))?;

    let thumb_rel = format!(
        "media/thumbs/{}/{}/{}.webp",
        entry.system_id, subdir,
        &sha[..sha.len().min(16)]
    );
    let thumb_abs = app_data_dir.join(&thumb_rel);
    let (w, h) = if thumb_abs.exists() {
        (None, None)
    } else {
        // Move the decode → resize → WebP-encode pass onto Tokio's blocking
        // pool. Without this, every concurrent sync_single_rom holds an
        // async-runtime worker for 50-200ms per image while it does pure
        // CPU work — the upstream `buffer_unordered(8)` then can't actually
        // run 8 simultaneously and the decode step is the bottleneck of
        // first-scan-of-a-large-library. spawn_blocking moves it to the
        // blocking pool (~cpu_count threads), so CPU work parallelizes
        // across cores while runtime threads stay free for the next
        // batch of network fetches.
        let bytes_for_thumb = bytes_vec.clone();
        let thumb_abs_for_task = thumb_abs.clone();
        let (w, h) = tokio::task::spawn_blocking(move || {
            generate_thumbnail(&bytes_for_thumb, &thumb_abs_for_task)
        })
        .await
        .map_err(|e| format!("thumbnail join: {e}"))??;
        (Some(w), Some(h))
    };

    // 7. Build the new variant.
    let variant = MediaVariant {
        source: MediaSource::LibretroThumbnails,
        region: matched.region.map(|r| Region::parse(r)),
        path: cache_rel,
        thumb_path: Some(thumb_rel),
        width: w,
        height: h,
        sha1: Some(sha),
        bytes: Some(bytes_vec.len() as u64),
    };

    Ok(SyncOutcome::Downloaded { variant })
}

/// Insert/replace a libretro-thumbnails variant on a game's <kind> list. A
/// new variant with the same `path` replaces the prior one in place; new
/// paths append. Manual variants are never touched. Returns the updated GameMedia.
fn apply_synced_variant(
    db: &mut MediaDb,
    rom_id: &str,
    kind: MediaKind,
    variant: MediaVariant,
) -> GameMedia {
    let gm = db.entry(rom_id.to_string()).or_insert_with(GameMedia::default);
    let variants = kind.variants_mut(gm);
    let same_path = variants
        .iter()
        .position(|v| v.path == variant.path && matches!(v.source, MediaSource::LibretroThumbnails));
    match same_path {
        Some(i) => variants[i] = variant,
        None => variants.push(variant),
    }
    gm.clone()
}

/// Resolve the user's `kinds_to_fetch` pref into MediaKind values, filtering
/// to only kinds the libretro-thumbnails repos actually carry (boxart / snap /
/// title). Falls back to the full default if the prefs read errors or the
/// configured list is empty.
fn enabled_sync_kinds(prefs: &Arc<RwLock<MediaPrefs>>) -> Vec<MediaKind> {
    let names = prefs.read().ok()
        .map(|p| p.kinds_to_fetch.clone())
        .unwrap_or_else(default_kinds_to_fetch);
    let kinds: Vec<MediaKind> = names.iter()
        .filter_map(|s| MediaKind::parse(s))
        .filter(|k| k.libretro_thumbnails_subdir().is_some())
        .collect();
    if kinds.is_empty() {
        vec![MediaKind::Boxart, MediaKind::Snap, MediaKind::Title]
    } else {
        kinds
    }
}

/// Drive a libretro-thumbnails sync over the supplied library entries. Per
/// the user's `kinds_to_fetch` pref, fetches up to three kinds per ROM
/// (boxart / snap / title). Groups entries by their target repo (HuCard /
/// CD / SGX), fetches each repo's file tree once (24h cache, covers all
/// three subdirs), then loops kinds × repos × ROMs and downloads in
/// parallel (8-way `buffer_unordered` per kind).
#[tauri::command]
#[allow(non_snake_case)]
pub async fn sync_media_for_system(
    systemId: String,
    entries: Vec<SyncRomEntry>,
    state: tauri::State<'_, MediaState>,
    library: tauri::State<'_, crate::library_db::LibraryDb>,
    app: tauri::AppHandle,
) -> Result<SyncSummary, String> {
    use futures::stream::{self, StreamExt};
    use tauri::Emitter;

    let enabled_kinds = enabled_sync_kinds(&state.prefs);
    let only_identified = state.prefs.read().ok()
        .map(|p| p.only_sync_identified)
        .unwrap_or(true);

    // Resolve canonical names server-side: for every entry, look up
    // the authoritative sha1 from library_db (the frontend sends
    // SyncRomEntry payloads constructed before resolve_rom_hashes had
    // a chance to stamp the rows — entry.sha1 is typically None even
    // when the DB row has a fresh sha1). Falls back to the entry's
    // own sha1 field for callers that explicitly hydrate it (the
    // store-level syncSystem path does this; the ImportWizard path
    // doesn't). The hash-identified subset becomes the "trusted match"
    // set.
    let mut canonical_by_id: std::collections::HashMap<String, String> = Default::default();
    for e in entries.iter() {
        let sha = library
            .find_sha1_by_id(&e.id)
            .ok()
            .flatten()
            .or_else(|| e.sha1.clone());
        if let Some(sha) = sha {
            if !sha.is_empty() {
                if let Ok(Some(row)) = library.lookup_rom_hash(&sha) {
                    canonical_by_id.insert(e.id.clone(), row.game_name);
                }
            }
        }
    }

    // Optionally filter entries to only those that have a canonical
    // match — the recommended setting. Stops the sync from churning
    // through unidentified ROMs that would either no-match or
    // false-positive via fuzzy matching.
    let entries: Vec<SyncRomEntry> = if only_identified {
        let kept: Vec<SyncRomEntry> = entries
            .into_iter()
            .filter(|e| canonical_by_id.contains_key(&e.id))
            .collect();
        log::info!(
            "oa-shell: sync_media_for_system({systemId}) — only-identified ON: keeping {} hash-matched entries",
            kept.len()
        );
        kept
    } else {
        entries
    };

    log::info!(
        "oa-shell: sync_media_for_system({systemId}) — {} entries × {} kinds {:?} ({} canonical)",
        entries.len(), enabled_kinds.len(),
        enabled_kinds.iter().map(|k| k.as_str()).collect::<Vec<_>>(),
        canonical_by_id.len(),
    );

    let app_data_dir = state.app_data_dir.clone();
    let db = state.db.clone();
    let canonical_by_id = Arc::new(canonical_by_id);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(45))
        .build()
        .map_err(|e| format!("reqwest client: {e}"))?;

    // Group entries by their target libretro-thumbnails repo(s). An
    // entry whose system_id maps to multiple repos (gb = DMG + CGB,
    // wonderswan = WS + WS Color) gets pushed into each bucket — both
    // upstream catalogs are scanned independently, both matches land
    // as separate variants on the same library entry. Entries whose
    // system_id maps to the empty slice (unknown system) are skipped
    // rather than fetched from the wrong repo.
    //
    // GameCube + Wii special case: both run on Dolphin under
    // `system_id = "gamecube"`, but libretro-thumbnails keeps Wii art
    // in a separate `Nintendo_-_Wii` repo. `repos_for_entry` peeks
    // the dump (extension + first-byte header for ambiguous .iso/.rvz)
    // and routes Wii titles to the Wii repo while GC titles stay on
    // the GameCube repo.
    let mut by_repo: std::collections::HashMap<&'static str, Vec<SyncRomEntry>> =
        std::collections::HashMap::new();
    let mut skipped_no_repo = 0usize;
    for e in entries.iter() {
        let repos = repos_for_entry(e);
        if repos.is_empty() {
            skipped_no_repo += 1;
            continue;
        }
        for repo in repos {
            by_repo.entry(*repo).or_default().push(e.clone());
        }
    }
    if skipped_no_repo > 0 {
        log::warn!(
            "media sync: {skipped_no_repo} entries skipped (no libretro-thumbnails repo for their system_id)"
        );
    }

    // Total work units = sum of bucket sizes × enabled kinds (one
    // download attempt per ROM per repo per kind). Multi-repo entries
    // contribute one work unit per repo they appear in.
    let total = by_repo.values().map(|v| v.len()).sum::<usize>() * enabled_kinds.len();
    let summary = Arc::new(std::sync::Mutex::new(SyncSummary {
        system_id: systemId.clone(),
        total,
        matched: 0,
        downloaded: 0,
        cached: 0,
        unmatched: 0,
        errors: 0,
    }));
    let done = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    for (repo, repo_entries) in by_repo {
        // 1. Tree (cached 24h or fetched) — now carries all three subdirs.
        let tree = match get_repo_tree_cached(&client, &app_data_dir, repo).await {
            Ok(t) => t,
            Err(e) => {
                log::warn!("oa-shell: skipping {repo} — tree fetch failed: {e}");
                let mut s = summary.lock().expect("summary lock");
                let units = repo_entries.len() * enabled_kinds.len();
                s.errors += units;
                let mut d = done.fetch_add(units, std::sync::atomic::Ordering::SeqCst);
                for entry in &repo_entries {
                    for _ in &enabled_kinds {
                        d += 1;
                        let _ = app.emit("oa://library-sync", &SyncProgress {
                            system_id: systemId.clone(),
                            done: d,
                            total,
                            current_rom_title: entry.title.clone(),
                            last_action: format!("repo unavailable: {repo}"),
                        });
                    }
                }
                continue;
            }
        };

        // 2. Per kind: parse the upstream subdir's filenames once, then run
        //    the 8-way parallel per-ROM sync.
        for &kind in &enabled_kinds {
            let subdir = kind.libretro_thumbnails_subdir()
                .expect("filtered to subdir-having kinds in enabled_sync_kinds");
            let upstream_files = tree.for_kind(kind);
            if upstream_files.is_empty() {
                // Repo has no entries for this kind (e.g. some systems lack
                // Named_Titles/). Still walk every entry as "no match" so
                // the progress counter stays accurate.
                let mut s = summary.lock().expect("summary lock");
                s.unmatched += repo_entries.len();
                let mut d = done.fetch_add(repo_entries.len(), std::sync::atomic::Ordering::SeqCst);
                for entry in &repo_entries {
                    d += 1;
                    let _ = app.emit("oa://library-sync", &SyncProgress {
                        system_id: systemId.clone(),
                        done: d,
                        total,
                        current_rom_title: entry.title.clone(),
                        last_action: format!("{}: not in repo", kind.as_str()),
                    });
                }
                continue;
            }

            let parsed: Vec<crate::normalize::ParsedUpstream> = upstream_files
                .iter()
                .cloned()
                .map(crate::normalize::parse_upstream)
                .collect();
            let parsed = Arc::new(parsed);

            // Per-kind closure captures.
            let app2 = app.clone();
            let client2 = client.clone();
            let app_data_dir2 = app_data_dir.clone();
            let db2 = db.clone();
            let summary2 = summary.clone();
            let done2 = done.clone();
            let system_id2 = systemId.clone();
            let kind_label = kind.as_str();
            let subdir_owned = subdir.to_string();
            let entries_for_kind = repo_entries.clone();

            stream::iter(entries_for_kind)
                .map(|entry| {
                    let client = client2.clone();
                    let parsed = parsed.clone();
                    let app_data_dir = app_data_dir2.clone();
                    let db = db2.clone();
                    let app = app2.clone();
                    let summary = summary2.clone();
                    let done_ctr = done2.clone();
                    let system_id = system_id2.clone();
                    let subdir = subdir_owned.clone();
                    let canonical_by_id = canonical_by_id.clone();
                    async move {
                        let canonical = canonical_by_id.get(&entry.id).map(|s| s.as_str());
                        let outcome = sync_single_rom(
                            &client, &parsed, repo, kind, &subdir, &app_data_dir, &db, &entry,
                            canonical,
                        ).await;
                        let d = done_ctr.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                        log::info!(
                            "oa-shell: sync [{d}/{total}] {} [{kind_label}] → {}",
                            entry.title,
                            match &outcome {
                                Ok(SyncOutcome::Downloaded { .. }) => "downloaded".to_string(),
                                Ok(SyncOutcome::Cached) => "cached".to_string(),
                                Ok(SyncOutcome::NoMatch) => "no match".to_string(),
                                Err(e) => format!("ERROR: {e}"),
                            }
                        );
                        let action: String = match &outcome {
                            Ok(SyncOutcome::Downloaded { variant }) => {
                                let updated = {
                                    let mut db_w = match db.write() {
                                        Ok(g) => g,
                                        Err(_) => return,
                                    };
                                    let updated = apply_synced_variant(
                                        &mut db_w, &entry.id, kind, variant.clone(),
                                    );
                                    let _ = write_media_db(&app_data_dir, &db_w);
                                    updated
                                };
                                let _ = app.emit(
                                    "oa://media-updated",
                                    serde_json::json!({ "romId": &entry.id, "media": &updated }),
                                );
                                let mut s = summary.lock().expect("summary lock");
                                s.matched += 1;
                                s.downloaded += 1;
                                format!("{kind_label}: downloaded")
                            }
                            Ok(SyncOutcome::Cached) => {
                                // Echo the current GameMedia even on cache hit
                                // so a fresh MediaProvider learns about variants
                                // landed in a prior session.
                                if let Some(gm) = db.read().ok().and_then(|d| d.get(&entry.id).cloned()) {
                                    let _ = app.emit(
                                        "oa://media-updated",
                                        serde_json::json!({ "romId": &entry.id, "media": &gm }),
                                    );
                                }
                                let mut s = summary.lock().expect("summary lock");
                                s.matched += 1;
                                s.cached += 1;
                                format!("{kind_label}: cached")
                            }
                            Ok(SyncOutcome::NoMatch) => {
                                let mut s = summary.lock().expect("summary lock");
                                s.unmatched += 1;
                                format!("{kind_label}: no match")
                            }
                            Err(e) => {
                                log::warn!("oa-shell: sync {} [{kind_label}] failed: {e}", entry.title);
                                let mut s = summary.lock().expect("summary lock");
                                s.errors += 1;
                                format!("{kind_label}: error")
                            }
                        };
                        let _ = app.emit("oa://library-sync", &SyncProgress {
                            system_id,
                            done: d,
                            total,
                            current_rom_title: entry.title.clone(),
                            last_action: action,
                        });
                    }
                })
                .buffer_unordered(8)
                .for_each(|_| async {})
                .await;
        }
    }

    let final_summary = summary.lock().expect("summary lock").clone();
    let _ = app.emit("oa://library-sync-complete", &final_summary);
    log::info!(
        "oa-shell: sync_media_for_system({systemId}) done — matched {}, downloaded {}, cached {}, unmatched {}, errors {}",
        final_summary.matched, final_summary.downloaded, final_summary.cached, final_summary.unmatched, final_summary.errors,
    );
    Ok(final_summary)
}

// ---- Tauri commands ----

#[tauri::command]
pub fn get_media_index(state: tauri::State<'_, MediaState>) -> MediaDb {
    state.db.read().map(|db| db.clone()).unwrap_or_default()
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn set_manual_cover(
    romId: String,
    systemId: String,
    sourcePath: String,
    state: tauri::State<'_, MediaState>,
    app: tauri::AppHandle,
) -> Result<GameMedia, String> {
    let updated = {
        let mut db = state.db.write().map_err(|_| "media db lock poisoned".to_string())?;
        let r = ingest_manual_cover(
            &state.app_data_dir,
            &romId,
            &systemId,
            Path::new(&sourcePath),
            &mut db,
        )?;
        write_media_db(&state.app_data_dir, &db).map_err(|e| format!("write media.json: {e}"))?;
        r
    };
    use tauri::Emitter;
    let _ = app.emit(
        "oa://media-updated",
        serde_json::json!({ "romId": &romId, "media": &updated }),
    );
    log::info!("oa-shell: manual cover set for {romId} from {sourcePath}");
    Ok(updated)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn clear_media(
    romId: String,
    state: tauri::State<'_, MediaState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    {
        let mut db = state.db.write().map_err(|_| "media db lock poisoned".to_string())?;
        db.remove(&romId);
        write_media_db(&state.app_data_dir, &db).map_err(|e| format!("write media.json: {e}"))?;
    }
    use tauri::Emitter;
    let _ = app.emit("oa://media-updated", serde_json::json!({ "romId": &romId }));
    log::info!("oa-shell: media cleared for {romId}");
    Ok(())
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn set_selected_variant(
    romId: String,
    kind: String,
    index: usize,
    state: tauri::State<'_, MediaState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let parsed = MediaKind::parse(&kind).ok_or_else(|| format!("unknown kind: {kind}"))?;
    let updated = {
        let mut db = state.db.write().map_err(|_| "media db lock poisoned".to_string())?;
        let cloned = {
            let gm = db.get_mut(&romId).ok_or_else(|| format!("no media for rom_id: {romId}"))?;
            let sel = gm.selected.get_or_insert_with(SelectedMedia::default);
            match parsed {
                MediaKind::Boxart => sel.boxart_index = Some(index),
                MediaKind::Snap   => sel.snap_index   = Some(index),
                MediaKind::Title  => sel.title_index  = Some(index),
                _ => return Err(format!("selection not supported for kind: {kind}")),
            }
            gm.clone()
        };
        write_media_db(&state.app_data_dir, &db).map_err(|e| format!("write media.json: {e}"))?;
        cloned
    };
    use tauri::Emitter;
    let _ = app.emit(
        "oa://media-updated",
        serde_json::json!({ "romId": &romId, "media": &updated }),
    );
    log::info!("oa-shell: set_selected_variant({romId}, {kind}={index})");
    Ok(())
}

#[tauri::command]
pub fn get_region_priority(state: tauri::State<'_, MediaState>) -> Vec<String> {
    state
        .prefs
        .read()
        .map(|p| p.region_priority.clone())
        .unwrap_or_else(|_| MediaPrefs::default().region_priority)
}

#[tauri::command]
pub fn set_region_priority(
    regions: Vec<String>,
    state: tauri::State<'_, MediaState>,
) -> Result<(), String> {
    let mut prefs = state.prefs.write().map_err(|_| "media prefs lock poisoned".to_string())?;
    prefs.region_priority = regions;
    write_media_prefs(&state.app_data_dir, &prefs).map_err(|e| e.to_string())?;
    log::info!("oa-shell: region priority updated -> {:?}", prefs.region_priority);
    Ok(())
}

#[tauri::command]
pub fn get_media_kinds_to_fetch(state: tauri::State<'_, MediaState>) -> Vec<String> {
    state
        .prefs
        .read()
        .map(|p| p.kinds_to_fetch.clone())
        .unwrap_or_else(|_| default_kinds_to_fetch())
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn set_media_kinds_to_fetch(
    kinds: Vec<String>,
    state: tauri::State<'_, MediaState>,
) -> Result<(), String> {
    // Silently filter to known kinds; saving "boxart" "snap" "title" is the
    // expected v1 surface, but ignoring unknown strings keeps us forward-
    // compatible with frontends that pass an extra kind.
    let filtered: Vec<String> = kinds
        .into_iter()
        .filter(|s| MediaKind::parse(s).is_some())
        .collect();
    let mut prefs = state.prefs.write().map_err(|_| "media prefs lock poisoned".to_string())?;
    prefs.kinds_to_fetch = filtered;
    write_media_prefs(&state.app_data_dir, &prefs).map_err(|e| e.to_string())?;
    log::info!("oa-shell: media kinds_to_fetch updated -> {:?}", prefs.kinds_to_fetch);
    Ok(())
}

#[tauri::command]
pub fn get_only_sync_identified(state: tauri::State<'_, MediaState>) -> bool {
    state.prefs.read().map(|p| p.only_sync_identified).unwrap_or(true)
}

#[tauri::command]
pub fn set_only_sync_identified(
    enabled: bool,
    state: tauri::State<'_, MediaState>,
) -> Result<(), String> {
    let mut prefs = state.prefs.write().map_err(|_| "media prefs lock poisoned".to_string())?;
    prefs.only_sync_identified = enabled;
    write_media_prefs(&state.app_data_dir, &prefs).map_err(|e| e.to_string())?;
    log::info!("oa-shell: media only_sync_identified -> {enabled}");
    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaStorageStats {
    pub covers_bytes: u64,
    pub thumbs_bytes: u64,
    pub cache_bytes: u64,
    pub total_bytes: u64,
}

fn dir_size_recursive(p: &Path) -> u64 {
    let mut total = 0u64;
    let Ok(rd) = std::fs::read_dir(p) else { return 0; };
    for entry in rd.flatten() {
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_file() {
            total = total.saturating_add(meta.len());
        } else if meta.is_dir() {
            total = total.saturating_add(dir_size_recursive(&entry.path()));
        }
    }
    total
}

#[tauri::command]
pub fn media_storage_stats(state: tauri::State<'_, MediaState>) -> MediaStorageStats {
    let covers_bytes = dir_size_recursive(&state.app_data_dir.join("media").join("covers"));
    let thumbs_bytes = dir_size_recursive(&state.app_data_dir.join("media").join("thumbs"));
    let cache_bytes = dir_size_recursive(&state.app_data_dir.join("media").join("cache"));
    MediaStorageStats {
        total_bytes: covers_bytes + thumbs_bytes + cache_bytes,
        covers_bytes,
        thumbs_bytes,
        cache_bytes,
    }
}

#[tauri::command]
pub fn open_media_folder(state: tauri::State<'_, MediaState>) -> Result<(), String> {
    let folder = state.app_data_dir.join("media");
    // Ensure it exists so the shell-open doesn't pop an error dialog.
    let _ = std::fs::create_dir_all(&folder);
    #[cfg(target_os = "windows")]
    let r = std::process::Command::new("explorer").arg(&folder).spawn();
    #[cfg(target_os = "macos")]
    let r = std::process::Command::new("open").arg(&folder).spawn();
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let r = std::process::Command::new("xdg-open").arg(&folder).spawn();
    r.map(|_| ()).map_err(|e| format!("open folder: {e}"))
}
