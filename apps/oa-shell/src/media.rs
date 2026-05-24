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

/// Full LaunchBox-shape media slot set. v1 (pre 2026-05-23) shipped 5
/// slots: `boxart`/`snap`/`title`/`cart`/`disc`. Those keys are preserved
/// via `#[serde(alias = ...)]` on the renamed fields below — old
/// `media.json` files deserialize forward into the new shape with no
/// migration step. The 26-slot taxonomy ships in the data model from
/// day one; UI catches up incrementally (see Phase 6 of the
/// media-taxonomy plan).
#[derive(Serialize, Deserialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GameMedia {
    // ---- v1 slots, semantically renamed to map to LaunchBox names ----
    #[serde(default, skip_serializing_if = "Vec::is_empty", alias = "boxart")]
    pub box_front: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty", alias = "snap")]
    pub screenshot_gameplay: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty", alias = "title")]
    pub screenshot_title: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty", alias = "cart")]
    pub cart_front: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disc: Vec<MediaVariant>,

    // ---- new slots (all default-empty; old JSON omits them) ----
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub box_back: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub box_3d: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub box_spine: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub box_full: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub cart_back: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub cart_3d: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub screenshot_select: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub banner: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub clear_logo: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub fanart_background: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub fanart_disc: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub advert_front: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub advert_back: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub arcade_cabinet: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub arcade_marquee: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub arcade_controlpanel: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub arcade_controlsinfo: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub arcade_playerselect: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub arcade_flyer: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub video: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub music: Vec<MediaVariant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub manual: Vec<MediaVariant>,

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

/// Per-slot pinned variant index. v1 (pre 2026-05-23) shipped 3 indexes
/// keyed `boxartIndex` / `snapIndex` / `titleIndex`; the renamed fields
/// below preserve those keys via `#[serde(alias = ...)]`. New slots
/// default to None so old prefs roundtrip cleanly.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct SelectedMedia {
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "boxartIndex")]
    pub box_front_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "snapIndex")]
    pub screenshot_gameplay_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "titleIndex")]
    pub screenshot_title_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub cart_front_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub disc_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub box_back_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub box_3d_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub box_spine_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub box_full_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub cart_back_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub cart_3d_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub screenshot_select_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub banner_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub clear_logo_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub fanart_background_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub fanart_disc_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub advert_front_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub advert_back_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub arcade_cabinet_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub arcade_marquee_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub arcade_controlpanel_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub arcade_controlsinfo_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub arcade_playerselect_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub arcade_flyer_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub video_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub music_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub manual_index: Option<usize>,
}

impl SelectedMedia {
    /// Set the pinned-variant index for `kind`. None clears the pin.
    pub fn set_for_kind(&mut self, kind: MediaKind, idx: Option<usize>) {
        match kind {
            MediaKind::BoxFront => self.box_front_index = idx,
            MediaKind::ScreenshotGameplay => self.screenshot_gameplay_index = idx,
            MediaKind::ScreenshotTitle => self.screenshot_title_index = idx,
            MediaKind::CartFront => self.cart_front_index = idx,
            MediaKind::Disc => self.disc_index = idx,
            MediaKind::BoxBack => self.box_back_index = idx,
            MediaKind::Box3d => self.box_3d_index = idx,
            MediaKind::BoxSpine => self.box_spine_index = idx,
            MediaKind::BoxFull => self.box_full_index = idx,
            MediaKind::CartBack => self.cart_back_index = idx,
            MediaKind::Cart3d => self.cart_3d_index = idx,
            MediaKind::ScreenshotSelect => self.screenshot_select_index = idx,
            MediaKind::Banner => self.banner_index = idx,
            MediaKind::ClearLogo => self.clear_logo_index = idx,
            MediaKind::FanartBackground => self.fanart_background_index = idx,
            MediaKind::FanartDisc => self.fanart_disc_index = idx,
            MediaKind::AdvertFront => self.advert_front_index = idx,
            MediaKind::AdvertBack => self.advert_back_index = idx,
            MediaKind::ArcadeCabinet => self.arcade_cabinet_index = idx,
            MediaKind::ArcadeMarquee => self.arcade_marquee_index = idx,
            MediaKind::ArcadeControlpanel => self.arcade_controlpanel_index = idx,
            MediaKind::ArcadeControlsinfo => self.arcade_controlsinfo_index = idx,
            MediaKind::ArcadePlayerselect => self.arcade_playerselect_index = idx,
            MediaKind::ArcadeFlyer => self.arcade_flyer_index = idx,
            MediaKind::Video => self.video_index = idx,
            MediaKind::Music => self.music_index = idx,
            MediaKind::Manual => self.manual_index = idx,
        }
    }

    /// Read the pinned-variant index for `kind`.
    pub fn get_for_kind(&self, kind: MediaKind) -> Option<usize> {
        match kind {
            MediaKind::BoxFront => self.box_front_index,
            MediaKind::ScreenshotGameplay => self.screenshot_gameplay_index,
            MediaKind::ScreenshotTitle => self.screenshot_title_index,
            MediaKind::CartFront => self.cart_front_index,
            MediaKind::Disc => self.disc_index,
            MediaKind::BoxBack => self.box_back_index,
            MediaKind::Box3d => self.box_3d_index,
            MediaKind::BoxSpine => self.box_spine_index,
            MediaKind::BoxFull => self.box_full_index,
            MediaKind::CartBack => self.cart_back_index,
            MediaKind::Cart3d => self.cart_3d_index,
            MediaKind::ScreenshotSelect => self.screenshot_select_index,
            MediaKind::Banner => self.banner_index,
            MediaKind::ClearLogo => self.clear_logo_index,
            MediaKind::FanartBackground => self.fanart_background_index,
            MediaKind::FanartDisc => self.fanart_disc_index,
            MediaKind::AdvertFront => self.advert_front_index,
            MediaKind::AdvertBack => self.advert_back_index,
            MediaKind::ArcadeCabinet => self.arcade_cabinet_index,
            MediaKind::ArcadeMarquee => self.arcade_marquee_index,
            MediaKind::ArcadeControlpanel => self.arcade_controlpanel_index,
            MediaKind::ArcadeControlsinfo => self.arcade_controlsinfo_index,
            MediaKind::ArcadePlayerselect => self.arcade_playerselect_index,
            MediaKind::ArcadeFlyer => self.arcade_flyer_index,
            MediaKind::Video => self.video_index,
            MediaKind::Music => self.music_index,
            MediaKind::Manual => self.manual_index,
        }
    }
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
    // Kebab-case canonical names. Legacy "boxart"/"snap"/"title" still
    // parse via MediaKind::parse aliases, so old media-prefs.json files
    // keep working unchanged until next save flips them to the new shape.
    vec!["box-front".into(), "screenshot-gameplay".into(), "screenshot-title".into()]
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
    /// Per-system operation gate. sync_media / sync_metadata /
    /// resolve_rom_hashes / sync_rom_hashes / clear_metadata all
    /// acquire the per-system Mutex at function entry so two
    /// concurrent ops on the same system serialize naturally. Pre-
    /// 2026-05-21 the buttons in Settings → Library were independently
    /// gated per-button, so an operator could click "Sync media" then
    /// "Identify ROMs" and both ran in parallel — the mid-sync
    /// apply_rom_hash writes weren't visible to the in-flight
    /// sync_media's `only_identified` filter, causing the wrong-art
    /// bug.
    ///
    /// `tokio::sync::Mutex` (not std::sync::Mutex) because the gate
    /// is held across `.await` boundaries inside the sync functions.
    /// The outer Mutex is `std::sync::Mutex` for lazy slot creation —
    /// the lookup is O(1) and never held across an await.
    pub system_op_gates: Arc<std::sync::Mutex<
        std::collections::HashMap<String, Arc<tokio::sync::Mutex<()>>>,
    >>,
}

impl MediaState {
    /// Get-or-create the per-system gate Mutex. Holds the outer
    /// std::sync::Mutex briefly (no await inside) to populate the
    /// slot, then returns the inner tokio Mutex Arc which the caller
    /// awaits.
    pub fn gate_for(&self, system_id: &str) -> Arc<tokio::sync::Mutex<()>> {
        let mut map = self.system_op_gates.lock().expect("system_op_gates poisoned");
        map.entry(system_id.to_string())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }
}

// ---- persistence ----

/// Atomically write `bytes` to `path` — writes to `<path>.tmp` first,
/// then renames into place. On all platforms `std::fs::rename` is
/// atomic when the source and destination are on the same filesystem
/// (which is always the case here — we write within the app_data
/// directory). On Windows, rename also replaces an existing target
/// file silently (per `MoveFileEx`/`MOVEFILE_REPLACE_EXISTING`
/// semantics that std::fs::rename uses).
///
/// Pre-fix `write_media_db` and `write_media_prefs` did a direct
/// `std::fs::write`. A crash mid-write left the file truncated;
/// next launch's parser hit the truncated JSON, fell back to
/// `MediaDb::default()`, and silently lost all the operator's
/// custom cover selections, region priorities, and so on.
fn atomic_write_bytes(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // Append .tmp to the full filename (preserves extension visibility
    // and avoids collisions with any other transient files in the dir).
    let mut tmp_os = path.as_os_str().to_owned();
    tmp_os.push(".tmp");
    let tmp_path = PathBuf::from(tmp_os);
    std::fs::write(&tmp_path, bytes)?;
    match std::fs::rename(&tmp_path, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            // Rename failed — try to clean up the .tmp so we don't
            // leave debris on disk. Ignore cleanup errors; the
            // primary failure is what gets surfaced.
            let _ = std::fs::remove_file(&tmp_path);
            Err(e)
        }
    }
}

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
                // Preserve the bad file as .corrupt — pre-fix the
                // operator's media.json was silently wiped on parse
                // failure and the bad bytes (potentially recoverable)
                // were lost. Now they can grep the .corrupt file or
                // hand it back for analysis.
                log::warn!("oa-shell: media.json malformed ({e:?}); starting empty");
                let mut backup = p.as_os_str().to_owned();
                backup.push(".corrupt");
                if let Err(e) = std::fs::rename(&p, std::path::Path::new(&backup)) {
                    log::warn!("oa-shell: media.json backup-rename failed: {e}");
                } else {
                    log::warn!(
                        "oa-shell: media.json saved as {} for recovery",
                        std::path::Path::new(&backup).display()
                    );
                }
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
    let bytes = serde_json::to_vec_pretty(db)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    atomic_write_bytes(&p, &bytes)
}

pub fn read_media_prefs(app_data_dir: &Path) -> MediaPrefs {
    let p = media_prefs_path(app_data_dir);
    match std::fs::read(&p) {
        Ok(bytes) => match serde_json::from_slice::<MediaPrefs>(&bytes) {
            Ok(prefs) => prefs,
            Err(e) => {
                // Loud warning + preserve the bad file as .corrupt so the
                // operator can recover their custom region priority /
                // kinds_to_fetch / only_sync_identified settings.
                // Pre-fix this was a silent unwrap_or_default — a
                // mid-write crash silently wiped the operator's
                // customizations and the only signal was "wait, my
                // region priority is back to defaults?"
                log::warn!(
                    "oa-shell: media-prefs.json malformed ({e}); starting with defaults"
                );
                let mut backup = p.as_os_str().to_owned();
                backup.push(".corrupt");
                if let Err(e) = std::fs::rename(&p, std::path::Path::new(&backup)) {
                    log::warn!("oa-shell: media-prefs.json backup-rename failed: {e}");
                } else {
                    log::warn!(
                        "oa-shell: media-prefs.json saved as {} for recovery",
                        std::path::Path::new(&backup).display()
                    );
                }
                MediaPrefs::default()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => MediaPrefs::default(),
        Err(e) => {
            log::warn!("oa-shell: media-prefs.json read failed ({e}); starting with defaults");
            MediaPrefs::default()
        }
    }
}

pub fn write_media_prefs(app_data_dir: &Path, prefs: &MediaPrefs) -> std::io::Result<()> {
    let p = media_prefs_path(app_data_dir);
    let bytes = serde_json::to_vec_pretty(prefs)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    atomic_write_bytes(&p, &bytes)
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
            "sms", "gamegear", "gb", "gbc", "gba", "2600",
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

    /// Multi-variant systems (wonderswan = WS + WS Color) must return
    /// both repos so a Color-only title still resolves covers. gb (DMG)
    /// and gbc (CGB) are now separate OA slugs, each returning their
    /// own single repo — locked separately below.
    #[test]
    fn multi_variant_systems_return_both_repos() {
        let ws = super::repos_for_system_id("wonderswan");
        assert_eq!(
            ws,
            &["Bandai_-_WonderSwan", "Bandai_-_WonderSwan_Color"],
            "wonderswan must return WS + WS Color repos (mono first as primary)"
        );
    }

    /// gb (DMG) and gbc (CGB) are sibling slugs sharing the Gambatte
    /// core but resolving to distinct libretro-thumbnails repos. Locked
    /// here so a future edit doesn't accidentally re-merge them or drop
    /// the CGB-side repo.
    #[test]
    fn gb_and_gbc_resolve_to_their_own_thumbnails_repos() {
        assert_eq!(
            super::repos_for_system_id("gb"),
            &["Nintendo_-_Game_Boy"],
            "gb (DMG slug) must resolve to the DMG-only repo"
        );
        assert_eq!(
            super::repos_for_system_id("gbc"),
            &["Nintendo_-_Game_Boy_Color"],
            "gbc (CGB slug) must resolve to the CGB-only repo"
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

    /// `.rvz` with a Wii disc header (game-ID byte at offset 40 = 'R')
    /// routes to the Wii repo. Pre-fix this test would have failed
    /// because every RVZ was misclassified as Wii regardless of disc
    /// contents (RVZ container magic starts with 'R', so the old
    /// byte-0 peek always matched the Wii heuristic).
    #[test]
    fn rvz_with_wii_disc_header_routes_to_wii_repo() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-rvz-wii-{}-{}.rvz",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        // 48-byte synthetic RVZ header:
        //   bytes 0..4   = "RVZ\x01" magic
        //   bytes 4..40  = filler (version, sizes, offsets — not parsed)
        //   bytes 40..48 = embedded disc header; byte 40 is the game
        //                  ID's first character. 'R' = Wii.
        let mut header = [0u8; 48];
        header[..4].copy_from_slice(b"RVZ\x01");
        header[40] = b'R';
        std::fs::write(&tmp, header).expect("write tmp rvz");
        let e = gc_entry_with_path(tmp.to_str().unwrap());
        assert_eq!(super::repos_for_entry(&e), &["Nintendo_-_Wii"]);
        let _ = std::fs::remove_file(&tmp);
    }

    /// `.rvz` with a GameCube disc header (byte 40 = 'G') routes to
    /// the GameCube repo. Confirms the RVZ container header is being
    /// parsed correctly rather than the bug where every RVZ matched
    /// the Wii heuristic.
    #[test]
    fn rvz_with_gc_disc_header_routes_to_gc_repo() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-rvz-gc-{}-{}.rvz",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let mut header = [0u8; 48];
        header[..4].copy_from_slice(b"RVZ\x01");
        header[40] = b'G';
        std::fs::write(&tmp, header).expect("write tmp rvz");
        let e = gc_entry_with_path(tmp.to_str().unwrap());
        assert_eq!(super::repos_for_entry(&e), &["Nintendo_-_GameCube"]);
        let _ = std::fs::remove_file(&tmp);
    }

    /// Truncated `.rvz` (header missing) → falls back to GameCube.
    /// Mirrors the iso_unreachable_falls_back_to_gc_repo case.
    #[test]
    fn rvz_truncated_falls_back_to_gc_repo() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-rvz-short-{}-{}.rvz",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        // Only 8 bytes — too short for the 48-byte RVZ header read.
        std::fs::write(&tmp, b"RVZ\x01abcd").expect("write tmp rvz");
        let e = gc_entry_with_path(tmp.to_str().unwrap());
        assert_eq!(super::repos_for_entry(&e), &["Nintendo_-_GameCube"]);
        let _ = std::fs::remove_file(&tmp);
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

        // gb (DMG) now resolves to its own single repo; gbc is the
        // companion slug for CGB. Fallback path returns the per-slug repo.
        e.system_id = "gb".to_string();
        e.file_path = "/nope/some.gb".to_string();
        assert_eq!(
            super::repos_for_entry(&e),
            &["Nintendo_-_Game_Boy"]
        );

        e.system_id = "gbc".to_string();
        e.file_path = "/nope/some.gbc".to_string();
        assert_eq!(
            super::repos_for_entry(&e),
            &["Nintendo_-_Game_Boy_Color"]
        );
    }

    // ---- media-taxonomy Phase 1: path-builder + filename sanitizer +
    //      variant-suffix + MediaKind parse/as_str + serde-alias tests ----

    #[test]
    fn sanitize_strips_windows_forbidden_chars() {
        assert_eq!(super::sanitize_filename_stem("X: Beyond Frontier"), "X_ Beyond Frontier");
        // Only `?` is forbidden; `!` is fine on every supported OS.
        assert_eq!(super::sanitize_filename_stem("Whoa?!"), "Whoa_!");
        assert_eq!(super::sanitize_filename_stem("a/b\\c|d*e"), "a_b_c_d_e");
        assert_eq!(super::sanitize_filename_stem("Final Fantasy V <fan-dub>"), "Final Fantasy V _fan-dub_");
    }

    #[test]
    fn sanitize_handles_empty_and_trim_only_input() {
        assert_eq!(super::sanitize_filename_stem(""), "_");
        assert_eq!(super::sanitize_filename_stem("..."), "_");
        assert_eq!(super::sanitize_filename_stem("   "), "_");
    }

    #[test]
    fn sanitize_strips_trailing_dot_and_space() {
        assert_eq!(super::sanitize_filename_stem("Sonic the Hedgehog ."), "Sonic the Hedgehog");
        assert_eq!(super::sanitize_filename_stem("foo. . "), "foo");
    }

    #[test]
    fn sanitize_avoids_reserved_dos_names() {
        assert_eq!(super::sanitize_filename_stem("CON"), "CON_");
        assert_eq!(super::sanitize_filename_stem("con"), "con_");
        assert_eq!(super::sanitize_filename_stem("LPT1"), "LPT1_");
        // Plain words that happen to contain reserved prefixes are fine.
        assert_eq!(super::sanitize_filename_stem("CONTRA"), "CONTRA");
    }

    #[test]
    fn sanitize_strips_control_chars() {
        let s = format!("hello{}world", '\x07');
        assert_eq!(super::sanitize_filename_stem(&s), "hello_world");
    }

    #[test]
    fn rom_stem_strips_extension_and_directory() {
        assert_eq!(
            super::rom_stem_from_path("C:\\ROMs\\Genesis\\Sonic the Hedgehog (USA).md"),
            "Sonic the Hedgehog (USA)"
        );
        assert_eq!(
            super::rom_stem_from_path("/home/me/roms/snes/Super Metroid.smc"),
            "Super Metroid"
        );
    }

    #[test]
    fn rom_stem_sanitizes_in_one_pass() {
        // colon in title gets replaced; extension still stripped.
        assert_eq!(
            super::rom_stem_from_path("/roms/X: Beyond Frontier.iso"),
            "X_ Beyond Frontier"
        );
    }

    #[test]
    fn rom_stem_empty_path_returns_underscore() {
        assert_eq!(super::rom_stem_from_path(""), "_");
    }

    #[test]
    fn media_path_for_slot_primary() {
        assert_eq!(
            super::media_path_for_slot("genesis", super::MediaKind::BoxFront, "Sonic", "png", None),
            "media/genesis/box-front/Sonic.png"
        );
        // variant_n = 1 is treated as primary (no suffix).
        assert_eq!(
            super::media_path_for_slot("snes", super::MediaKind::ClearLogo, "Super Mario World", "png", Some(1)),
            "media/snes/clear-logo/Super Mario World.png"
        );
    }

    #[test]
    fn media_path_for_slot_variant_suffix() {
        assert_eq!(
            super::media_path_for_slot("genesis", super::MediaKind::BoxFront, "Sonic", "png", Some(2)),
            "media/genesis/box-front/Sonic-02.png"
        );
        assert_eq!(
            super::media_path_for_slot("tg16", super::MediaKind::ScreenshotGameplay, "Bonk", "jpg", Some(10)),
            "media/tg16/screenshot-gameplay/Bonk-10.jpg"
        );
    }

    #[test]
    fn next_variant_filename_returns_primary_when_empty() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-mt-empty-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp).expect("mkdir tmp");
        let got = super::next_variant_filename(&tmp, "Sonic", "png");
        assert_eq!(got, "Sonic.png");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn next_variant_filename_skips_existing() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-mt-suffix-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp).expect("mkdir tmp");
        std::fs::write(tmp.join("Sonic.png"), b"x").expect("primary");
        let got = super::next_variant_filename(&tmp, "Sonic", "png");
        assert_eq!(got, "Sonic-02.png");
        std::fs::write(tmp.join("Sonic-02.png"), b"x").expect("v2");
        let got = super::next_variant_filename(&tmp, "Sonic", "png");
        assert_eq!(got, "Sonic-03.png");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn media_kind_parse_accepts_kebab_case() {
        assert_eq!(super::MediaKind::parse("box-front"), Some(super::MediaKind::BoxFront));
        assert_eq!(super::MediaKind::parse("screenshot-gameplay"), Some(super::MediaKind::ScreenshotGameplay));
        assert_eq!(super::MediaKind::parse("clear-logo"), Some(super::MediaKind::ClearLogo));
        assert_eq!(super::MediaKind::parse("arcade-marquee"), Some(super::MediaKind::ArcadeMarquee));
        assert_eq!(super::MediaKind::parse("manual"), Some(super::MediaKind::Manual));
    }

    #[test]
    fn media_kind_parse_accepts_v1_aliases() {
        assert_eq!(super::MediaKind::parse("boxart"), Some(super::MediaKind::BoxFront));
        assert_eq!(super::MediaKind::parse("snap"), Some(super::MediaKind::ScreenshotGameplay));
        assert_eq!(super::MediaKind::parse("title"), Some(super::MediaKind::ScreenshotTitle));
        assert_eq!(super::MediaKind::parse("cart"), Some(super::MediaKind::CartFront));
        assert_eq!(super::MediaKind::parse("disc"), Some(super::MediaKind::Disc));
    }

    #[test]
    fn media_kind_parse_rejects_unknown() {
        assert_eq!(super::MediaKind::parse("unknown"), None);
        assert_eq!(super::MediaKind::parse(""), None);
        assert_eq!(super::MediaKind::parse("Box-Front"), None); // case-sensitive
    }

    #[test]
    fn media_kind_as_str_returns_kebab_case() {
        assert_eq!(super::MediaKind::BoxFront.as_str(), "box-front");
        assert_eq!(super::MediaKind::ScreenshotGameplay.as_str(), "screenshot-gameplay");
        assert_eq!(super::MediaKind::ClearLogo.as_str(), "clear-logo");
        assert_eq!(super::MediaKind::Disc.as_str(), "disc");
        assert_eq!(super::MediaKind::Manual.as_str(), "manual");
    }

    #[test]
    fn media_kind_all_covers_every_variant_via_round_trip() {
        // Locks the ALL invariant: parsing each ALL entry's as_str back
        // must produce the same variant. If a new MediaKind is added but
        // not appended to ALL, this test catches it because the round
        // trip works but the count check below fails.
        for &k in super::MediaKind::ALL {
            assert_eq!(super::MediaKind::parse(k.as_str()), Some(k));
        }
        // Expected count tracks the 26-slot LaunchBox taxonomy plus
        // multimedia + manual (= 27). Bumping the enum requires bumping
        // ALL and this constant in lockstep — intentional friction so
        // the data model stays in sync.
        assert_eq!(super::MediaKind::ALL.len(), 27);
    }

    #[test]
    fn libretro_thumbnails_subdir_only_three_kinds() {
        assert_eq!(super::MediaKind::BoxFront.libretro_thumbnails_subdir(), Some("Named_Boxarts"));
        assert_eq!(super::MediaKind::ScreenshotGameplay.libretro_thumbnails_subdir(), Some("Named_Snaps"));
        assert_eq!(super::MediaKind::ScreenshotTitle.libretro_thumbnails_subdir(), Some("Named_Titles"));
        // All other kinds have no upstream coverage.
        for &k in super::MediaKind::ALL {
            match k {
                super::MediaKind::BoxFront
                | super::MediaKind::ScreenshotGameplay
                | super::MediaKind::ScreenshotTitle => {}
                _ => assert_eq!(
                    k.libretro_thumbnails_subdir(),
                    None,
                    "{:?} unexpectedly has an upstream subdir",
                    k,
                ),
            }
        }
    }

    #[test]
    fn game_media_parses_v1_legacy_json() {
        // A v1 media.json entry with the old field names. The new
        // GameMedia must accept this via serde aliases and surface the
        // values on the renamed fields.
        let legacy = r#"{
            "boxart": [{"source":{"kind":"manual"},"path":"old/path.png"}],
            "snap":   [{"source":{"kind":"manual"},"path":"old/snap.png"}],
            "title":  [{"source":{"kind":"manual"},"path":"old/title.png"}],
            "cart":   [{"source":{"kind":"manual"},"path":"old/cart.png"}],
            "disc":   [{"source":{"kind":"manual"},"path":"old/disc.png"}]
        }"#;
        let gm: super::GameMedia = serde_json::from_str(legacy).expect("legacy parses");
        assert_eq!(gm.box_front.len(), 1);
        assert_eq!(gm.box_front[0].path, "old/path.png");
        assert_eq!(gm.screenshot_gameplay.len(), 1);
        assert_eq!(gm.screenshot_title.len(), 1);
        assert_eq!(gm.cart_front.len(), 1);
        assert_eq!(gm.disc.len(), 1);
        // New slots default to empty.
        assert!(gm.clear_logo.is_empty());
        assert!(gm.manual.is_empty());
        assert!(gm.banner.is_empty());
    }

    #[test]
    fn game_media_parses_new_camel_case_json() {
        // A new media.json entry with the camelCase serialized names.
        let modern = r#"{
            "boxFront": [{"source":{"kind":"manual"},"path":"media/genesis/box-front/Sonic.png"}],
            "clearLogo": [{"source":{"kind":"manual"},"path":"media/genesis/clear-logo/Sonic.png"}],
            "manual": [{"source":{"kind":"manual"},"path":"media/genesis/manual/Sonic.pdf"}]
        }"#;
        let gm: super::GameMedia = serde_json::from_str(modern).expect("modern parses");
        assert_eq!(gm.box_front.len(), 1);
        assert_eq!(gm.box_front[0].path, "media/genesis/box-front/Sonic.png");
        assert_eq!(gm.clear_logo.len(), 1);
        assert_eq!(gm.manual.len(), 1);
    }

    #[test]
    fn game_media_serializes_to_camel_case() {
        let mut gm = super::GameMedia::default();
        gm.box_front.push(super::MediaVariant {
            source: super::MediaSource::Manual,
            region: None,
            path: "media/genesis/box-front/Sonic.png".into(),
            thumb_path: None,
            width: None, height: None, sha1: None, bytes: None,
        });
        gm.clear_logo.push(super::MediaVariant {
            source: super::MediaSource::Manual,
            region: None,
            path: "media/genesis/clear-logo/Sonic.png".into(),
            thumb_path: None,
            width: None, height: None, sha1: None, bytes: None,
        });
        let json = serde_json::to_string(&gm).expect("serialize");
        // Empty slots skipped, present slots use camelCase keys.
        assert!(json.contains(r#""boxFront""#));
        assert!(json.contains(r#""clearLogo""#));
        // Legacy v1 names must NOT appear on serialize.
        assert!(!json.contains(r#""boxart""#));
        assert!(!json.contains(r#""snap""#));
        // Empty slots skipped via skip_serializing_if.
        assert!(!json.contains(r#""banner""#));
    }

    #[test]
    fn selected_media_parses_v1_legacy_indexes() {
        let legacy = r#"{"boxartIndex":1,"snapIndex":2,"titleIndex":3}"#;
        let sel: super::SelectedMedia = serde_json::from_str(legacy).expect("legacy parses");
        assert_eq!(sel.box_front_index, Some(1));
        assert_eq!(sel.screenshot_gameplay_index, Some(2));
        assert_eq!(sel.screenshot_title_index, Some(3));
        // Other slots default to None.
        assert_eq!(sel.cart_front_index, None);
        assert_eq!(sel.clear_logo_index, None);
    }

    #[test]
    fn selected_media_set_and_get_round_trip_every_kind() {
        // Lock: set_for_kind followed by get_for_kind returns the same
        // value for every MediaKind variant. Catches accidental
        // dispatch-table drift between the two helpers.
        for (i, &k) in super::MediaKind::ALL.iter().enumerate() {
            let mut sel = super::SelectedMedia::default();
            sel.set_for_kind(k, Some(i));
            assert_eq!(sel.get_for_kind(k), Some(i), "round-trip failed for {:?}", k);
            // Clearing works.
            sel.set_for_kind(k, None);
            assert_eq!(sel.get_for_kind(k), None, "clear failed for {:?}", k);
        }
    }
}

// ---- LaunchBox-shape path builders + filename sanitization ----

/// Characters Windows forbids in filenames (`< > : " / \ | ? *`) plus
/// any C0 control byte. Other platforms permit most of these but we
/// keep one shared sanitization rule so OA installs are portable across
/// OSes and a Windows-safe filename always works elsewhere.
fn is_forbidden_filename_char(c: char) -> bool {
    matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || (c as u32) < 0x20
}

const RESERVED_WIN_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL",
    "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
    "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Sanitize a filename stem (no extension) for cross-platform safety.
/// Forbidden chars become `_`. Trailing `.` / space — both of which
/// Windows treats specially — are trimmed. Reserved DOS names
/// (CON/PRN/AUX/NUL/COM1-9/LPT1-9, case-insensitive) get a `_`
/// appended so the resulting name is never reserved. Empty input
/// produces `_` (so we never emit a zero-length filename).
pub fn sanitize_filename_stem(stem: &str) -> String {
    if stem.is_empty() {
        return "_".to_string();
    }
    let mut out: String = stem
        .chars()
        .map(|c| if is_forbidden_filename_char(c) { '_' } else { c })
        .collect();
    // Trim trailing whitespace + dots — Windows silently strips both
    // from filenames at creation time, which would produce a different
    // filename than the caller intended.
    while out.ends_with('.') || out.ends_with(' ') {
        out.pop();
    }
    if out.is_empty() {
        return "_".to_string();
    }
    let upper = out.to_ascii_uppercase();
    if RESERVED_WIN_NAMES.iter().any(|r| *r == upper.as_str()) {
        out.push('_');
    }
    out
}

/// Derive the rom_stem from a ROM file path. Returns the file stem
/// (basename without extension), sanitized for cross-platform safety.
/// Examples:
///   "C:\\ROMs\\Genesis\\Sonic the Hedgehog (USA).md" -> "Sonic the Hedgehog (USA)"
///   "/roms/snes/Super Metroid.smc"                   -> "Super Metroid"
///   "/roms/pcecd/Y's IV.cue"                         -> "Y's IV"
pub fn rom_stem_from_path(file_path: &str) -> String {
    let p = std::path::Path::new(file_path);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    sanitize_filename_stem(stem)
}

/// Build the canonical media path for a slot.
/// Returns `"media/<system_id>/<kind>/<rom_stem>.<ext>"` for primary
/// variants. `variant_n` controls the -NN suffix:
///   - None or Some(1) → primary, no suffix
///   - Some(n) where n ≥ 2 → `-02` / `-03` / ... appended to the stem
///
/// `system_id` is NOT sanitized — it's an internal slug controlled by
/// the registry (always safe). `rom_stem` MUST already be sanitized
/// (callers get this from `rom_stem_from_path` or build with care).
/// `ext` is also passed through as-is; callers should use lowercase
/// short extensions like "png" / "jpg" / "webp" / "mp4" / "ogg".
pub fn media_path_for_slot(
    system_id: &str,
    kind: MediaKind,
    rom_stem: &str,
    ext: &str,
    variant_n: Option<u32>,
) -> String {
    let suffix = match variant_n {
        Some(n) if n >= 2 => format!("-{:02}", n),
        _ => String::new(),
    };
    format!(
        "media/{}/{}/{}{}.{}",
        system_id, kind.as_str(), rom_stem, suffix, ext
    )
}

/// Pick the next available variant filename in `dir`. Returns the
/// primary `<stem>.<ext>` when no collision; otherwise tries
/// `<stem>-02.<ext>`, `<stem>-03.<ext>`, ... up to -99. After that
/// (extremely unlikely in practice) falls back to a millisecond-stamped
/// suffix so we never overwrite an existing file silently.
///
/// Unused in Phase 1 — wired into the libretro-thumbnails sync's
/// operator-art-wins guard in Phase 2.
#[allow(dead_code)]
pub fn next_variant_filename(dir: &Path, stem: &str, ext: &str) -> String {
    let primary = format!("{stem}.{ext}");
    if !dir.join(&primary).exists() {
        return primary;
    }
    for n in 2u32..=99 {
        let candidate = format!("{stem}-{:02}.{ext}", n);
        if !dir.join(&candidate).exists() {
            return candidate;
        }
    }
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("{stem}-{ms}.{ext}")
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

/// Full LaunchBox-shape media slot taxonomy. v1 shipped 5 variants
/// (Boxart/Snap/Title/Cart/Disc); the 2026-05-23 media-taxonomy pivot
/// added the remaining ~21 slots from the LaunchBox naming. `parse`
/// accepts both the legacy 5 names (`"boxart"`/`"snap"`/`"title"`/
/// `"cart"`/`"disc"`) AND the new kebab-case names, so prefs files +
/// frontend URLs from before the migration keep working without a
/// flag day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MediaKind {
    BoxFront,
    BoxBack,
    Box3d,
    BoxSpine,
    BoxFull,
    CartFront,
    CartBack,
    Cart3d,
    Disc,
    ScreenshotGameplay,
    ScreenshotTitle,
    ScreenshotSelect,
    Banner,
    ClearLogo,
    FanartBackground,
    FanartDisc,
    AdvertFront,
    AdvertBack,
    ArcadeCabinet,
    ArcadeMarquee,
    ArcadeControlpanel,
    ArcadeControlsinfo,
    ArcadePlayerselect,
    ArcadeFlyer,
    Video,
    Music,
    Manual,
}

impl MediaKind {
    /// All variants in stable enumeration order. Use this for "iterate
    /// every slot" loops (e.g. importer scans, migration sweeps). Used
    /// today by tests; the Phase 3 art-pack importer + Phase 5 migration
    /// loops will consume it.
    #[allow(dead_code)]
    pub const ALL: &'static [MediaKind] = &[
        MediaKind::BoxFront, MediaKind::BoxBack, MediaKind::Box3d, MediaKind::BoxSpine, MediaKind::BoxFull,
        MediaKind::CartFront, MediaKind::CartBack, MediaKind::Cart3d,
        MediaKind::Disc,
        MediaKind::ScreenshotGameplay, MediaKind::ScreenshotTitle, MediaKind::ScreenshotSelect,
        MediaKind::Banner, MediaKind::ClearLogo,
        MediaKind::FanartBackground, MediaKind::FanartDisc,
        MediaKind::AdvertFront, MediaKind::AdvertBack,
        MediaKind::ArcadeCabinet, MediaKind::ArcadeMarquee, MediaKind::ArcadeControlpanel,
        MediaKind::ArcadeControlsinfo, MediaKind::ArcadePlayerselect, MediaKind::ArcadeFlyer,
        MediaKind::Video, MediaKind::Music, MediaKind::Manual,
    ];

    /// Parse a kind name. Accepts kebab-case (`"box-front"`) for new
    /// callers AND the v1 5-slot aliases (`"boxart"`/`"snap"`/`"title"`/
    /// `"cart"`/`"disc"`). Returns None for unknown input.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            // ---- v1 aliases ----
            "boxart" => Some(MediaKind::BoxFront),
            "snap"   => Some(MediaKind::ScreenshotGameplay),
            "title"  => Some(MediaKind::ScreenshotTitle),
            "cart"   => Some(MediaKind::CartFront),
            // ---- canonical kebab-case names (match folder names exactly) ----
            "box-front"             => Some(MediaKind::BoxFront),
            "box-back"              => Some(MediaKind::BoxBack),
            "box-3d"                => Some(MediaKind::Box3d),
            "box-spine"             => Some(MediaKind::BoxSpine),
            "box-full"              => Some(MediaKind::BoxFull),
            "cart-front"            => Some(MediaKind::CartFront),
            "cart-back"             => Some(MediaKind::CartBack),
            "cart-3d"               => Some(MediaKind::Cart3d),
            "disc"                  => Some(MediaKind::Disc),
            "screenshot-gameplay"   => Some(MediaKind::ScreenshotGameplay),
            "screenshot-title"      => Some(MediaKind::ScreenshotTitle),
            "screenshot-select"     => Some(MediaKind::ScreenshotSelect),
            "banner"                => Some(MediaKind::Banner),
            "clear-logo"            => Some(MediaKind::ClearLogo),
            "fanart-background"     => Some(MediaKind::FanartBackground),
            "fanart-disc"           => Some(MediaKind::FanartDisc),
            "advert-front"          => Some(MediaKind::AdvertFront),
            "advert-back"           => Some(MediaKind::AdvertBack),
            "arcade-cabinet"        => Some(MediaKind::ArcadeCabinet),
            "arcade-marquee"        => Some(MediaKind::ArcadeMarquee),
            "arcade-controlpanel"   => Some(MediaKind::ArcadeControlpanel),
            "arcade-controlsinfo"   => Some(MediaKind::ArcadeControlsinfo),
            "arcade-playerselect"   => Some(MediaKind::ArcadePlayerselect),
            "arcade-flyer"          => Some(MediaKind::ArcadeFlyer),
            "video"                 => Some(MediaKind::Video),
            "music"                 => Some(MediaKind::Music),
            "manual"                => Some(MediaKind::Manual),
            _ => None,
        }
    }

    /// Canonical kebab-case name. Matches folder names + new JSON keys.
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaKind::BoxFront             => "box-front",
            MediaKind::BoxBack              => "box-back",
            MediaKind::Box3d                => "box-3d",
            MediaKind::BoxSpine             => "box-spine",
            MediaKind::BoxFull              => "box-full",
            MediaKind::CartFront            => "cart-front",
            MediaKind::CartBack             => "cart-back",
            MediaKind::Cart3d               => "cart-3d",
            MediaKind::Disc                 => "disc",
            MediaKind::ScreenshotGameplay   => "screenshot-gameplay",
            MediaKind::ScreenshotTitle      => "screenshot-title",
            MediaKind::ScreenshotSelect     => "screenshot-select",
            MediaKind::Banner               => "banner",
            MediaKind::ClearLogo            => "clear-logo",
            MediaKind::FanartBackground     => "fanart-background",
            MediaKind::FanartDisc           => "fanart-disc",
            MediaKind::AdvertFront          => "advert-front",
            MediaKind::AdvertBack           => "advert-back",
            MediaKind::ArcadeCabinet        => "arcade-cabinet",
            MediaKind::ArcadeMarquee        => "arcade-marquee",
            MediaKind::ArcadeControlpanel   => "arcade-controlpanel",
            MediaKind::ArcadeControlsinfo   => "arcade-controlsinfo",
            MediaKind::ArcadePlayerselect   => "arcade-playerselect",
            MediaKind::ArcadeFlyer          => "arcade-flyer",
            MediaKind::Video                => "video",
            MediaKind::Music                => "music",
            MediaKind::Manual               => "manual",
        }
    }

    /// libretro-thumbnails subdirectory hosting this kind. Only the three
    /// kinds with upstream coverage map to a subdir; everything else
    /// returns None and the sync loop silently skips it.
    pub fn libretro_thumbnails_subdir(&self) -> Option<&'static str> {
        match self {
            MediaKind::BoxFront             => Some("Named_Boxarts"),
            MediaKind::ScreenshotGameplay   => Some("Named_Snaps"),
            MediaKind::ScreenshotTitle      => Some("Named_Titles"),
            _ => None,
        }
    }

    pub fn variants<'a>(&self, gm: &'a GameMedia) -> &'a [MediaVariant] {
        match self {
            MediaKind::BoxFront             => &gm.box_front,
            MediaKind::BoxBack              => &gm.box_back,
            MediaKind::Box3d                => &gm.box_3d,
            MediaKind::BoxSpine             => &gm.box_spine,
            MediaKind::BoxFull              => &gm.box_full,
            MediaKind::CartFront            => &gm.cart_front,
            MediaKind::CartBack             => &gm.cart_back,
            MediaKind::Cart3d               => &gm.cart_3d,
            MediaKind::Disc                 => &gm.disc,
            MediaKind::ScreenshotGameplay   => &gm.screenshot_gameplay,
            MediaKind::ScreenshotTitle      => &gm.screenshot_title,
            MediaKind::ScreenshotSelect     => &gm.screenshot_select,
            MediaKind::Banner               => &gm.banner,
            MediaKind::ClearLogo            => &gm.clear_logo,
            MediaKind::FanartBackground     => &gm.fanart_background,
            MediaKind::FanartDisc           => &gm.fanart_disc,
            MediaKind::AdvertFront          => &gm.advert_front,
            MediaKind::AdvertBack           => &gm.advert_back,
            MediaKind::ArcadeCabinet        => &gm.arcade_cabinet,
            MediaKind::ArcadeMarquee        => &gm.arcade_marquee,
            MediaKind::ArcadeControlpanel   => &gm.arcade_controlpanel,
            MediaKind::ArcadeControlsinfo   => &gm.arcade_controlsinfo,
            MediaKind::ArcadePlayerselect   => &gm.arcade_playerselect,
            MediaKind::ArcadeFlyer          => &gm.arcade_flyer,
            MediaKind::Video                => &gm.video,
            MediaKind::Music                => &gm.music,
            MediaKind::Manual               => &gm.manual,
        }
    }

    pub fn variants_mut<'a>(&self, gm: &'a mut GameMedia) -> &'a mut Vec<MediaVariant> {
        match self {
            MediaKind::BoxFront             => &mut gm.box_front,
            MediaKind::BoxBack              => &mut gm.box_back,
            MediaKind::Box3d                => &mut gm.box_3d,
            MediaKind::BoxSpine             => &mut gm.box_spine,
            MediaKind::BoxFull              => &mut gm.box_full,
            MediaKind::CartFront            => &mut gm.cart_front,
            MediaKind::CartBack             => &mut gm.cart_back,
            MediaKind::Cart3d               => &mut gm.cart_3d,
            MediaKind::Disc                 => &mut gm.disc,
            MediaKind::ScreenshotGameplay   => &mut gm.screenshot_gameplay,
            MediaKind::ScreenshotTitle      => &mut gm.screenshot_title,
            MediaKind::ScreenshotSelect     => &mut gm.screenshot_select,
            MediaKind::Banner               => &mut gm.banner,
            MediaKind::ClearLogo            => &mut gm.clear_logo,
            MediaKind::FanartBackground     => &mut gm.fanart_background,
            MediaKind::FanartDisc           => &mut gm.fanart_disc,
            MediaKind::AdvertFront          => &mut gm.advert_front,
            MediaKind::AdvertBack           => &mut gm.advert_back,
            MediaKind::ArcadeCabinet        => &mut gm.arcade_cabinet,
            MediaKind::ArcadeMarquee        => &mut gm.arcade_marquee,
            MediaKind::ArcadeControlpanel   => &mut gm.arcade_controlpanel,
            MediaKind::ArcadeControlsinfo   => &mut gm.arcade_controlsinfo,
            MediaKind::ArcadePlayerselect   => &mut gm.arcade_playerselect,
            MediaKind::ArcadeFlyer          => &mut gm.arcade_flyer,
            MediaKind::Video                => &mut gm.video,
            MediaKind::Music                => &mut gm.music,
            MediaKind::Manual               => &mut gm.manual,
        }
    }

    pub fn pinned_index(&self, sel: Option<&SelectedMedia>) -> Option<usize> {
        sel.and_then(|s| s.get_for_kind(*self))
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
    // Encode WebP to an in-memory buffer first, then atomically write
    // via the .tmp + rename helper. Pre-fix `save_with_format` wrote
    // directly to `dest`, leaving a window where two concurrent
    // sync_single_rom tasks targeting the same content-addressed
    // sha could torn-write (both saw `thumb_abs.exists() == false`,
    // both encoded, both wrote — last write wins but readers in the
    // middle could see a partial file). With the .tmp + rename
    // pattern each task writes to its own tmp path (path + ".tmp")
    // and the rename is atomic, so readers always see either the
    // old file or a complete new file, never a torn one.
    let mut buf: Vec<u8> = Vec::new();
    let dyn_img = image::DynamicImage::ImageRgba8(resized);
    dyn_img
        .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::WebP)
        .map_err(|e| format!("encode webp ({}x{} → {}x{}): {e}", orig_w, orig_h, new_w, new_h))?;
    atomic_write_bytes(dest, &buf)
        .map_err(|e| format!("write thumb {}: {e}", dest.display()))?;
    Ok((orig_w, orig_h))
}

/// Pure logic: writes a manual cover + thumbnail to disk for the given
/// slot + mutates the in-memory MediaDb. Caller is responsible for
/// persisting `db` to disk and emitting `oa://media-updated`.
///
/// File lands at `media/<system_id>/<kind>/<rom_stem>.<ext>` (new
/// LaunchBox-shape layout from the 2026-05-23 media-taxonomy pivot).
/// Thumbnail stays content-addressed at `media/thumbs/<system_id>/<sha[..16]>.webp`
/// so duplicate art across regional clones dedupes naturally.
///
/// Manual variants always sort to index 0 on their slot and clear any
/// prior pinned selection for that slot — the operator's most-recent
/// manual choice wins the default-display fallback. Any pre-existing
/// Manual variant on the same slot is replaced in place (only one
/// Manual per slot at a time; use the `-NN` suffix logic in Phase 2+
/// when we want multi-variant manual art).
fn ingest_manual_for_slot(
    app_data_dir: &Path,
    rom_stem: &str,
    system_id: &str,
    kind: MediaKind,
    source_path: &Path,
    db: &mut MediaDb,
    rom_id: &str,
) -> Result<GameMedia, String> {
    let bytes = std::fs::read(source_path).map_err(|e| format!("read source: {e}"))?;
    let (_format, ext) = detect_format(&bytes)?;
    let sha = sha1_hex(&bytes);

    let cover_rel = media_path_for_slot(system_id, kind, rom_stem, ext, None);
    let cover_abs = app_data_dir.join(&cover_rel);
    if let Some(parent) = cover_abs.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir slot dir: {e}"))?;
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
    let slot = kind.variants_mut(entry);
    slot.retain(|v| v.source != MediaSource::Manual);
    slot.insert(0, new_variant);
    if let Some(sel) = entry.selected.as_mut() {
        sel.set_for_kind(kind, None);
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
        // Game Boy (DMG) and Game Boy Color (CGB) are now separate OA
        // slugs (`gb` + `gbc`), one sidebar entry each — companion to the
        // libretro-thumbnails split between Nintendo_-_Game_Boy and
        // Nintendo_-_Game_Boy_Color repos. Each slug resolves to its own
        // single repo; cross-resolution (a CGB-only title scanned as `gb`)
        // isn't a concern now that the registry routes .gb → gb and
        // .gbc → gbc unambiguously.
        "gb" => &["Nintendo_-_Game_Boy"],
        "gbc" => &["Nintendo_-_Game_Boy_Color"],
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
        // Raw ISO: byte 0 of a GameCube/Wii disc image is the first
        // character of the 6-byte game ID. Dolphin's convention reserves
        // 'R' (Revolution = Wii) and 'S' (later Wii titles) as Wii
        // prefixes; everything else (G/D/P/etc.) routes to GameCube.
        Some("iso") => peek_dolphin_console_byte(path)
            .map(|b| matches!(b, b'R' | b'S'))
            .unwrap_or(false),
        // RVZ: Dolphin's compressed container. Byte 0 of the FILE is
        // always 'R' (from the "RVZ\x01" magic), which would
        // accidentally trip the Wii heuristic above for every RVZ
        // dump. Parse the RVZ header far enough to find the embedded
        // disc header's game-ID byte; if we can't (truncated file /
        // unknown future RVZ revision), default to GameCube — RVZ is
        // most commonly used for GameCube preservation (Wii dumps
        // overwhelmingly ship as WBFS, which is auto-routed above).
        Some("rvz") => peek_rvz_console_byte(path)
            .map(|b| matches!(b, b'R' | b'S'))
            .unwrap_or(false),
        _ => false,
    }
}

/// Read the first byte of a raw GameCube/Wii ISO dump (i.e. byte 0
/// of the disc image, which is the game ID's leading character).
/// Returns `None` on I/O failure — the caller treats that as "assume
/// GameCube" rather than erroring out.
fn peek_dolphin_console_byte(path: &std::path::Path) -> Option<u8> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).ok()?;
    let mut buf = [0u8; 1];
    f.read_exact(&mut buf).ok()?;
    Some(buf[0])
}

/// Peek the embedded disc-image game-ID byte inside an RVZ container.
///
/// RVZ layout (from the Dolphin RVZ format spec):
/// - bytes 0..4   : magic "RVZ\x01"
/// - bytes 4..8   : version (u32 LE)
/// - bytes 8..16  : reserved
/// - bytes 16..24 : data_size (u64 LE)
/// - bytes 24..32 : raw data offset
/// - bytes 32..40 : raw data size
/// - bytes 40..48 : disc header — first 8 bytes of the underlying
///                  GameCube/Wii disc image (per Dolphin's
///                  DiscIO/WIABlob.cpp WIAHeader2 layout). Byte 40
///                  is the disc game-ID's first character.
///
/// Returns `None` if the file is too short to be a valid RVZ, doesn't
/// have the magic, or if the read fails — the caller treats that as
/// "assume GameCube" which is the more common case for RVZ dumps.
fn peek_rvz_console_byte(path: &std::path::Path) -> Option<u8> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).ok()?;
    let mut buf = [0u8; 48];
    f.read_exact(&mut buf).ok()?;
    if &buf[0..4] != b"RVZ\x01" {
        return None;
    }
    Some(buf[40])
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
            MediaKind::BoxFront             => &self.boxarts,
            MediaKind::ScreenshotGameplay   => &self.snaps,
            MediaKind::ScreenshotTitle      => &self.titles,
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
    // get_with_retry classifies 5xx + network errors as transient and
    // retries once; 4xx as permanent. Pre-fix a transient 503 from
    // GitHub's API marked the whole repo as errors for the sync
    // (every entry would lose its variant for this run).
    //
    // The Accept header is required for the GitHub tree API (it
    // returns text/plain by default which is missing the `tree`
    // array). Append it after the helper's GET via a slight hack —
    // use the helper for response classification + retry, then
    // re-issue with the right header if we got a hit. Cleaner: just
    // accept that the github tree API also serves the right shape
    // without the Accept header in practice (it does; the header is
    // a hint, not a requirement). If a future api change breaks this,
    // pull the helper into a builder pattern that accepts extra
    // headers.
    let Some(resp) = crate::http_retry::get_with_retry(client, &url, "OverlookedArcade").await? else {
        return Err(format!("github tree: {url} returned 404 (repo missing?)"));
    };
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
        vec![MediaKind::BoxFront, MediaKind::ScreenshotGameplay, MediaKind::ScreenshotTitle]
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

    // Per-system op gate (H11) — serializes concurrent sync/resolve
    // operations on the same system_id so the "click Sync media then
    // Identify ROMs" race can't produce stale-sha1 filtering. The
    // gate is held for the lifetime of this function call.
    let gate = state.gate_for(&systemId);
    let _gate_guard = gate.lock().await;

    let enabled_kinds = enabled_sync_kinds(&state.prefs);
    let only_identified = state.prefs.read().ok()
        .map(|p| p.only_sync_identified)
        .unwrap_or(true);

    // Resolve canonical names server-side: bulk-hydrate sha1 +
    // canonical title from library_db in one SQL statement. Pre-2026-
    // 05-21 this was N×2 sequential SQLite lookups per entry; for
    // a 1160-entry sync that was ~11,400 lock cycles before any
    // network walk began, serializing against any concurrent
    // resolve_rom_hashes_for_system writes. The bulk LEFT JOIN
    // collapses that to one query + one lock acquisition.
    //
    // Entries whose game.sha1 is null (resolve hasn't stamped them
    // yet) drop out of `hydrated` and won't pass the
    // `only_identified` filter below — same behaviour as before, just
    // arrived at much faster.
    let hydrated = library
        .hydrate_sha1_and_canonical_for_system(&systemId)
        .map_err(|e| format!("hydrate sha1+canonical: {e}"))?;
    let mut canonical_by_id: std::collections::HashMap<String, String> = Default::default();
    for e in entries.iter() {
        if let Some((_sha, Some(game_name))) = hydrated.get(&e.id) {
            canonical_by_id.insert(e.id.clone(), game_name.clone());
        } else if let Some(sha) = e.sha1.as_deref() {
            // Fallback for callers that explicitly hydrated entry.sha1
            // before invoking (e.g. the store-level syncSystem path).
            // The bulk JOIN already covers the common case (entry.id is
            // in the library); this branch catches the rare ad-hoc
            // direct-launch path that might pass an entry not yet in
            // library_db.
            if !sha.is_empty() {
                if let Ok(Some(row)) = library.lookup_rom_hash(sha) {
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
    // BTreeMap (not HashMap) — iteration order is deterministic
    // (alphabetical by repo name) so for multi-repo systems (gb
    // DMG+CGB, wonderswan WS+WSC) the order variants land in
    // `GameMedia.boxart` is reproducible run-to-run. Pre-2026-05-21
    // a HashMap caused the default-displayed cover to flip between
    // runs when no region pin was active and multiple repos
    // contributed variants to the same game.
    let mut by_repo: std::collections::BTreeMap<&'static str, Vec<SyncRomEntry>> =
        std::collections::BTreeMap::new();
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

            // Per-kind dirty flag — set by any successful Downloaded
            // outcome inside the buffer_unordered stream below, then
            // checked once after the stream drains so we flush
            // media.json at most once per (repo × kind) batch.
            let kind_dirty = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let kind_dirty2 = kind_dirty.clone();
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
                    let kind_dirty = kind_dirty2.clone();
                    async move {
                        let canonical = canonical_by_id.get(&entry.id).map(|s| s.as_str());
                        let outcome = sync_single_rom(
                            &client, &parsed, repo, kind, &subdir, &app_data_dir, &db, &entry,
                            canonical,
                        ).await;
                        let d = done_ctr.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                        // Per-ROM log line. Pre-fix every outcome was
                        // info-level — a 1160-game Genesis sync × 3
                        // kinds = ~3500 info lines in oa-current.log
                        // per sync. Now: only Downloaded and Err
                        // surface at info (the operator-interesting
                        // events); Cached and NoMatch drop to debug
                        // (still recoverable with RUST_LOG=debug but
                        // not cluttering normal logs).
                        let outcome_str = match &outcome {
                            Ok(SyncOutcome::Downloaded { .. }) => "downloaded".to_string(),
                            Ok(SyncOutcome::Cached) => "cached".to_string(),
                            Ok(SyncOutcome::NoMatch) => "no match".to_string(),
                            Err(e) => format!("ERROR: {e}"),
                        };
                        match &outcome {
                            Ok(SyncOutcome::Downloaded { .. }) | Err(_) => {
                                log::info!(
                                    "oa-shell: sync [{d}/{total}] {} [{kind_label}] → {}",
                                    entry.title, outcome_str,
                                );
                            }
                            _ => {
                                log::debug!(
                                    "oa-shell: sync [{d}/{total}] {} [{kind_label}] → {}",
                                    entry.title, outcome_str,
                                );
                            }
                        }
                        let action: String = match &outcome {
                            Ok(SyncOutcome::Downloaded { variant }) => {
                                // Apply the variant under the write lock, but
                                // DON'T flush media.json here — the per-kind
                                // batch flush at end of buffer_unordered (line
                                // ~1930) writes once per kind, not once per
                                // ROM. Pre-2026-05-21 every successful download
                                // wrote the full media.json (multi-MB on a 5K+
                                // library) under contention from 8 concurrent
                                // tasks. Now: 1 write per (repo × kind), ~10-30
                                // writes total per sync vs ~1000s before.
                                let updated = {
                                    let mut db_w = match db.write() {
                                        Ok(g) => g,
                                        Err(_) => return,
                                    };
                                    apply_synced_variant(
                                        &mut db_w, &entry.id, kind, variant.clone(),
                                    )
                                };
                                let _ = app.emit(
                                    "oa://media-updated",
                                    serde_json::json!({ "romId": &entry.id, "media": &updated }),
                                );
                                kind_dirty.store(true, std::sync::atomic::Ordering::Release);
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

            // Per-kind batch flush — write media.json once per
            // (repo × kind) batch IF any successful download set the
            // dirty flag inside the stream. Cuts media.json writes
            // from ~1 per ROM (multi-MB write × 1000s) down to ~1
            // per kind per repo (~10-30 total per sync).
            if kind_dirty.load(std::sync::atomic::Ordering::Acquire) {
                if let Ok(db_r) = db.read() {
                    if let Err(e) = write_media_db(&app_data_dir, &db_r) {
                        log::warn!(
                            "oa-shell: media.json flush failed after {repo}/{kind_label}: {e}"
                        );
                    }
                }
            }
        }
    }

    // Final flush — also runs the no-downloads case (the dirty flag
    // gates per-kind writes, so a sync that only hits cached entries
    // doesn't touch the disk; this safety-net write captures any
    // accounting state we might have missed and keeps the on-disk
    // file fresh at end-of-sync regardless.
    if let Ok(db_r) = db.read() {
        if let Err(e) = write_media_db(&app_data_dir, &db_r) {
            log::warn!("oa-shell: final media.json flush failed: {e}");
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

/// Set a manual cover. v1 default — kind defaults to box-front (the
/// historical "set cover" gesture) so the frontend doesn't need to pass
/// `kind` until the UI starts surfacing other slots. Pass `kind` when
/// the UI needs to target a non-default slot (snapshots, clear-logo, etc.).
///
/// Looks up `file_path` from library_db so the on-disk filename uses
/// the human-readable rom stem (`Sonic the Hedgehog (USA).png`) rather
/// than the opaque `rom-<hash>.png` from the v1 layout.
#[tauri::command]
#[allow(non_snake_case)]
pub fn set_manual_cover(
    romId: String,
    systemId: String,
    sourcePath: String,
    kind: Option<String>,
    state: tauri::State<'_, MediaState>,
    library: tauri::State<'_, crate::library_db::LibraryDb>,
    app: tauri::AppHandle,
) -> Result<GameMedia, String> {
    let kind = match kind.as_deref() {
        None | Some("") => MediaKind::BoxFront,
        Some(k) => MediaKind::parse(k).ok_or_else(|| format!("unknown kind: {k}"))?,
    };
    // Look up the rom's file_path so we can name the on-disk art by
    // its human-readable stem. Falls back to the rom_id if the lookup
    // fails (lets the user drop covers on ad-hoc / pre-import entries
    // without crashing the flow).
    let rom_stem = match library.find_game_by_id(&romId) {
        Ok(Some(row)) => {
            let raw = row
                .archive_inner_path
                .as_deref()
                .unwrap_or(&row.file_path);
            rom_stem_from_path(raw)
        }
        Ok(None) | Err(_) => sanitize_filename_stem(&romId),
    };

    let updated = {
        let mut db = state.db.write().map_err(|_| "media db lock poisoned".to_string())?;
        let r = ingest_manual_for_slot(
            &state.app_data_dir,
            &rom_stem,
            &systemId,
            kind,
            Path::new(&sourcePath),
            &mut db,
            &romId,
        )?;
        write_media_db(&state.app_data_dir, &db).map_err(|e| format!("write media.json: {e}"))?;
        r
    };
    use tauri::Emitter;
    let _ = app.emit(
        "oa://media-updated",
        serde_json::json!({ "romId": &romId, "media": &updated }),
    );
    log::info!(
        "oa-shell: manual cover set for {romId} [{}] (stem={rom_stem}) from {sourcePath}",
        kind.as_str(),
    );
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

/// Clear the `metadata` slot on every media_db entry whose game is
/// tagged with `systemId` in library_db. Cover-art / snap / title
/// variants are *not* touched — only the metadata payload (genre /
/// developer / publisher / year / players). Used after the 2026-05-21
/// metadata-routing fix to scrub the cross-system data contamination
/// that the old wildcard-to-PCE routing left behind: a Genesis game
/// that accidentally fuzzy-matched a PCE catalog entry got stamped
/// with PCE metadata; this command nulls those stamps so a follow-up
/// sync against the correct upstream catalog refills them cleanly.
///
/// Returns the count of entries actually cleared (skips entries that
/// already had metadata = None). Emits one oa://media-updated per
/// cleared entry so the UI re-renders.
#[tauri::command]
#[allow(non_snake_case)]
pub async fn clear_metadata_for_system(
    systemId: String,
    state: tauri::State<'_, MediaState>,
    library: tauri::State<'_, crate::library_db::LibraryDb>,
    app: tauri::AppHandle,
) -> Result<MetadataClearSummary, String> {
    // Per-system op gate (H11) — serializes against any in-flight
    // sync_media / sync_metadata / resolve_rom_hashes / sync_rom_hashes
    // for the same system. Without the gate a "Clear metadata" mid-sync
    // could null entries the sync was about to update, causing
    // misleading "updated 0" summary numbers.
    let gate = state.gate_for(&systemId);
    let _gate_guard = gate.lock().await;

    let ids = library.list_game_ids_for_system(&systemId)?;
    // Pre-fix this loop cloned every cleared GameMedia under the write
    // lock and stored them in `updated_payloads` for the event emit
    // step. For a 5000-entry library that's 5000 clones (each
    // GameMedia carries multiple Vec<MediaVariant>) held alive while
    // the write lock blocks every concurrent reader. Now: the
    // mutating loop only collects ids that were cleared, the write
    // lock drops, the file write happens outside the lock, and the
    // per-row clones for the emit payload run under a READ lock
    // (multiple concurrent readers are fine).
    let cleared_ids: Vec<String> = {
        let mut db = state.db.write().map_err(|_| "media db lock poisoned".to_string())?;
        let mut out: Vec<String> = Vec::new();
        for id in &ids {
            if let Some(gm) = db.get_mut(id) {
                if gm.metadata.is_some() {
                    gm.metadata = None;
                    out.push(id.clone());
                }
            }
        }
        out
    }; // <-- write lock drops here

    let cleared = cleared_ids.len();
    if cleared > 0 {
        // File write under a brief read lock — atomic_write_bytes
        // makes this crash-safe via tmp+rename.
        let db_r = state.db.read().map_err(|_| "media db lock poisoned".to_string())?;
        write_media_db(&state.app_data_dir, &db_r)
            .map_err(|e| format!("write media.json: {e}"))?;
    }

    // Build emit payloads under a brief read lock — readers are
    // shareable, so this doesn't contend with other concurrent
    // sync_media / get_media_index callers.
    let updated_payloads: Vec<(String, GameMedia)> = {
        let db_r = state.db.read().map_err(|_| "media db lock poisoned".to_string())?;
        cleared_ids
            .iter()
            .filter_map(|id| db_r.get(id).map(|gm| (id.clone(), gm.clone())))
            .collect()
    };
    use tauri::Emitter;
    for (rom_id, gm) in &updated_payloads {
        let _ = app.emit(
            "oa://media-updated",
            serde_json::json!({ "romId": rom_id, "media": gm }),
        );
    }
    log::info!(
        "oa-shell: clear_metadata_for_system({systemId}) — cleared {cleared} of {} game ids in library",
        ids.len()
    );
    Ok(MetadataClearSummary {
        system_id: systemId,
        scanned: ids.len(),
        cleared,
    })
}

/// Summary returned by `clear_metadata_for_system`. `scanned` is the
/// number of library game ids inspected for the system; `cleared` is
/// the subset whose `metadata` slot was set to `None`.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataClearSummary {
    pub system_id: String,
    pub scanned: usize,
    pub cleared: usize,
}

/// Pin a specific variant index as the default-display choice for
/// `(rom_id, kind)`. Accepts any of the 27 MediaKind names (kebab-case
/// or v1 aliases). Pre-2026-05-23 this only worked for boxart / snap /
/// title; now every slot has its own pin.
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
            sel.set_for_kind(parsed, Some(index));
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
