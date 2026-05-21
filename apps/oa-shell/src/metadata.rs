//! Per-game metadata sync from libretro-database (metadat/ plain-text DATs).
//!
//! Fetches genre / developer / publisher / releaseyear / users (= players)
//! from `metadat/<kind>/<system>.dat`, merges them into one
//! `UpstreamMetadat` per game, and matches against ROM titles via the
//! existing `normalize::match_score` fuzzy pipeline. Matches populate
//! `GameMedia.metadata` in MediaDb — same on-disk store as boxart, so a
//! single `oa://media-updated` event covers both surfaces.
//!
//! libretro-database is offline-after-fetch: we sync the system's .dat
//! files once (24h cache), then resolve every ROM locally. No per-launch
//! API hits, no rate-limit anxiety, matches the libretro-thumbnails posture.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use crate::media::{
    write_media_db, GameMedia, GameMetadata, MediaState, SyncRomEntry,
};

/// Metadat .dat-file kinds we fetch per system. Each is a subdir under
/// libretro-database/metadat/. Files that 404 (kinds the system doesn't
/// have data for) are silently skipped — the rest still populate.
#[derive(Clone, Copy)]
enum MetadatKind {
    Genre,
    Developer,
    Publisher,
    ReleaseYear,
    Users,
}

impl MetadatKind {
    const ALL: [MetadatKind; 5] = [
        MetadatKind::Genre,
        MetadatKind::Developer,
        MetadatKind::Publisher,
        MetadatKind::ReleaseYear,
        MetadatKind::Users,
    ];

    fn subdir(&self) -> &'static str {
        match self {
            MetadatKind::Genre       => "genre",
            MetadatKind::Developer   => "developer",
            MetadatKind::Publisher   => "publisher",
            MetadatKind::ReleaseYear => "releaseyear",
            // Player count's metadat subdir is `maxusers/` even though the
            // inner field inside game blocks is `users` (see value_key).
            // Don't unify these — they really do diverge upstream.
            MetadatKind::Users       => "maxusers",
        }
    }

    /// The field name inside `game (...)` blocks. Matches the subdir for
    /// most kinds, but diverges for Users (subdir = "maxusers", field key
    /// = "users").
    fn value_key(&self) -> &'static str {
        match self {
            MetadatKind::Genre       => "genre",
            MetadatKind::Developer   => "developer",
            MetadatKind::Publisher   => "publisher",
            MetadatKind::ReleaseYear => "releaseyear",
            MetadatKind::Users       => "users",
        }
    }
}

/// Map a ROM extension to the libretro-database system name. Mirrors
/// `media::repo_for_extension` but uses libretro-database's space-separated
/// naming (vs libretro-thumbnails' underscore-separated repo names).
///
/// **PCE-CD note:** libretro-database has no PCE-CD-specific .dat — only
/// HuCard (`NEC - PC Engine - TurboGrafx 16`) and SuperGrafx. CD-extension
/// entries map to a sentinel name that 404s every kind, producing a clean
/// "no metadat for system" message rather than partial false matches
/// against the cart catalog (PCE-CD and HuCard share essentially no
/// titles, so cross-matching would be misleading).
fn metadat_system_name_for_extension(ext: &str) -> &'static str {
    match ext {
        "sgx" => "NEC - PC Engine SuperGrafx",
        "cue" | "chd" | "ccd" | "toc" | "m3u" | "iso" => "NEC - PC Engine CD - TurboGrafx-CD",
        _ => "NEC - PC Engine - TurboGrafx 16",
    }
}

/// One game's combined metadata across every kind we fetched. `name` is
/// the upstream entry's full name (used for fuzzy matching against ROM
/// titles); `normalized` is the precomputed normalized form so the match
/// loop doesn't re-normalize on every score.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct UpstreamMetadat {
    name: String,
    normalized: String,
    #[serde(default, skip_serializing_if = "Option::is_none")] genre: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] developer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] publisher: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] release_year: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")] users: Option<u32>,
}

#[derive(Serialize, Deserialize)]
struct CachedMetadat {
    fetched_at_unix_secs: u64,
    entries: Vec<UpstreamMetadat>,
}

const METADAT_CACHE_TTL_SECS: u64 = 86_400; // 24h

fn metadat_cache_path(app_data_dir: &Path, system_name: &str) -> PathBuf {
    app_data_dir
        .join("media")
        .join("cache")
        .join("metadat")
        .join(format!("{}.json", system_name.replace(['/', '\\'], "_")))
}

/// Parse a clrmamepro-style metadat .dat. Pulls one value per game block
/// (the field named by `value_key`) paired with that game's title — read
/// from `comment "..."`, NOT `name`. The top-level `name "<system>"` lives
/// inside the `clrmamepro (...)` header block which we skip entirely (the
/// `in_game` gate only enters on `game (`).
///
/// Game blocks lacking the value field are silently skipped — we don't
/// want to fabricate empty values for kinds the upstream chose not to
/// publish for that title.
fn parse_metadat_dat(content: &str, value_key: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut in_game = false;
    let mut current_title: Option<String> = None;
    let mut current_value: Option<String> = None;
    for raw in content.lines() {
        let line = raw.trim();
        if line.starts_with("game (") {
            in_game = true;
            current_title = None;
            current_value = None;
            continue;
        }
        if !in_game {
            continue;
        }
        if line == ")" {
            in_game = false;
            if let (Some(t), Some(v)) = (current_title.take(), current_value.take()) {
                out.push((t, v));
            }
            continue;
        }
        // Title field is `comment "..."` in metadat (the `name "..."` line
        // is the system header at the top of the file — handled by the
        // `in_game` gate above).
        if let Some(rest) = line.strip_prefix("comment ") {
            current_title = Some(unquote(rest));
        } else if line.starts_with(value_key) && line[value_key.len()..].starts_with(' ') {
            // Value can be quoted (`genre "Action"`) or unquoted
            // (`users 2`); `unquote` handles both via trim_matches('"').
            let rest = &line[value_key.len() + 1..];
            current_value = Some(unquote(rest));
        }
    }
    out
}

fn unquote(s: &str) -> String {
    s.trim().trim_matches('"').to_string()
}

async fn fetch_metadat_dat(
    client: &reqwest::Client,
    system_name: &str,
    kind: MetadatKind,
) -> Result<Option<String>, String> {
    let url = format!(
        "https://raw.githubusercontent.com/libretro/libretro-database/master/metadat/{}/{}.dat",
        kind.subdir(),
        urlencoding::encode(system_name),
    );
    let resp = client
        .get(&url)
        .header("User-Agent", "OverlookedArcade")
        .send()
        .await
        .map_err(|e| format!("fetch {url}: {e}"))?;
    let status = resp.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        log::debug!("oa-shell: metadat {url} 404 (kind absent for system)");
        return Ok(None);
    }
    if !status.is_success() {
        return Err(format!("metadat fetch {url} status {status}"));
    }
    let text = resp.text().await.map_err(|e| format!("metadat body: {e}"))?;
    Ok(Some(text))
}

/// Fetch + merge all metadat kinds for a system into one `Vec<UpstreamMetadat>`.
/// Cached on disk for 24h per system; the cache stores the merged parsed
/// JSON (not raw .dat) so a cache hit avoids re-parsing on every sync.
async fn get_system_metadat_cached(
    client: &reqwest::Client,
    app_data_dir: &Path,
    system_name: &str,
) -> Result<Vec<UpstreamMetadat>, String> {
    let cache_path = metadat_cache_path(app_data_dir, system_name);
    if let Ok(bytes) = std::fs::read(&cache_path) {
        if let Ok(cached) = serde_json::from_slice::<CachedMetadat>(&bytes) {
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            // Skip empty caches — they're either from a prior buggy parser
            // run (pre-comment-vs-name fix) or from a system with no
            // upstream coverage. Either way, a refetch is what we want;
            // the new code won't re-cache empties.
            if cached.entries.is_empty() {
                log::info!(
                    "oa-shell: stale empty cache for {system_name}; refetching"
                );
            } else if now.saturating_sub(cached.fetched_at_unix_secs) < METADAT_CACHE_TTL_SECS {
                log::info!(
                    "oa-shell: metadat for {system_name} from cache ({} entries)",
                    cached.entries.len()
                );
                return Ok(cached.entries);
            }
        }
    }
    // Fetch fresh — one request per kind. Skip kinds that 404.
    let mut by_name: BTreeMap<String, UpstreamMetadat> = BTreeMap::new();
    for kind in MetadatKind::ALL {
        let dat = match fetch_metadat_dat(client, system_name, kind).await {
            Ok(Some(s)) => s,
            Ok(None) => continue,
            Err(e) => {
                log::warn!(
                    "oa-shell: metadat {} for {system_name} failed: {e} — skipping kind",
                    kind.subdir()
                );
                continue;
            }
        };
        let pairs = parse_metadat_dat(&dat, kind.value_key());
        for (name, value) in pairs {
            let normalized = crate::normalize::normalize_title(&name);
            let entry = by_name
                .entry(name.clone())
                .or_insert_with(|| UpstreamMetadat {
                    name: name.clone(),
                    normalized,
                    genre: None,
                    developer: None,
                    publisher: None,
                    release_year: None,
                    users: None,
                });
            match kind {
                MetadatKind::Genre       => entry.genre = Some(value),
                MetadatKind::Developer   => entry.developer = Some(value),
                MetadatKind::Publisher   => entry.publisher = Some(value),
                MetadatKind::ReleaseYear => entry.release_year = value.parse::<u32>().ok(),
                MetadatKind::Users       => entry.users = value.parse::<u32>().ok(),
            }
        }
    }
    let entries: Vec<UpstreamMetadat> = by_name.into_values().collect();
    // Don't cache empty results — they typically mean the system has no
    // metadat at all (every kind 404'd, see e.g. PCE-CD which libretro-
    // database doesn't carry). Caching the empty would lock the user out
    // for 24h; leaving the cache absent lets a retry re-fetch immediately
    // if upstream publishes data later. Non-empty caches still last the
    // full TTL.
    if entries.is_empty() {
        log::warn!(
            "oa-shell: no metadat available for system {system_name} (all kinds 404 or empty); skipping cache write"
        );
    } else {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let cached = CachedMetadat {
            fetched_at_unix_secs: now,
            entries: entries.clone(),
        };
        if let Some(parent) = cache_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(
            &cache_path,
            serde_json::to_vec_pretty(&cached).unwrap_or_default(),
        );
        log::info!(
            "oa-shell: metadat for {system_name} fetched fresh ({} entries)",
            entries.len()
        );
    }
    Ok(entries)
}

const MATCH_THRESHOLD: f64 = 0.85;

fn metadata_from(upstream: &UpstreamMetadat) -> GameMetadata {
    GameMetadata {
        year: upstream.release_year,
        genre: upstream.genre.clone(),
        developer: upstream.developer.clone(),
        publisher: upstream.publisher.clone(),
        players: upstream.users,
        description: None,
    }
}

fn metadata_differs(prior: Option<&GameMetadata>, next: &GameMetadata) -> bool {
    let Some(p) = prior else { return true; };
    p.year != next.year
        || p.genre != next.genre
        || p.developer != next.developer
        || p.publisher != next.publisher
        || p.players != next.players
        || p.description != next.description
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MetadataSyncProgress {
    pub system_id: String,
    pub done: usize,
    pub total: usize,
    pub current_rom_title: String,
    pub last_action: String,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MetadataSyncSummary {
    pub system_id: String,
    pub total: usize,
    pub matched: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub unmatched: usize,
    pub errors: usize,
}

/// Sync per-game metadata from libretro-database for the supplied entries.
/// Groups by system name (HuCard / CD / SGX use different .dat files),
/// fetches each system's merged metadat once (24h cache), then matches
/// every ROM via the same fuzzy scoring used for cover sync. Updates
/// GameMedia.metadata in-place, emits `oa://media-updated` per ROM with a
/// changed value, and emits `oa://library-metadata-sync` per ROM + a final
/// `oa://library-metadata-sync-complete` summary.
#[tauri::command]
#[allow(non_snake_case)]
pub async fn sync_metadata_for_system(
    systemId: String,
    entries: Vec<SyncRomEntry>,
    state: tauri::State<'_, MediaState>,
    library: tauri::State<'_, crate::library_db::LibraryDb>,
    app: tauri::AppHandle,
) -> Result<MetadataSyncSummary, String> {
    use tauri::Emitter;
    log::info!(
        "oa-shell: sync_metadata_for_system({systemId}) — {} entries",
        entries.len()
    );

    let app_data_dir = state.app_data_dir.clone();
    let db = state.db.clone();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(45))
        .build()
        .map_err(|e| format!("reqwest client: {e}"))?;

    // Hydrate canonical no-intro titles for entries with a stamped
    // sha1 — these match upstream metadat by exact (post-normalize)
    // title, jumping match rate from ~10% (filename fuzzy) to ~95%
    // (canonical) on a typical library. Filename-derived title is
    // the fallback for entries the rom_hashes resolve couldn't
    // identify (homebrew, hacks, unstamped). Same lookup pattern as
    // sync_media_for_system's canonical_by_id.
    let mut canonical_title_by_id: std::collections::HashMap<String, String> =
        Default::default();
    for e in entries.iter() {
        if let Some(sha) = library.find_sha1_by_id(&e.id).ok().flatten() {
            if !sha.is_empty() {
                if let Ok(Some(row)) = library.lookup_rom_hash(&sha) {
                    canonical_title_by_id.insert(e.id.clone(), row.game_name);
                }
            }
        }
    }

    // Group by libretro-database system name (cart / CD / SGX use distinct
    // .dat files, exactly like the libretro-thumbnails repos).
    let mut by_system: BTreeMap<&'static str, Vec<SyncRomEntry>> = BTreeMap::new();
    for e in entries.iter() {
        let ext = std::path::Path::new(&e.file_path)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        by_system
            .entry(metadat_system_name_for_extension(&ext))
            .or_default()
            .push(e.clone());
    }

    let total = entries.len();
    let mut summary = MetadataSyncSummary {
        system_id: systemId.clone(),
        total,
        matched: 0,
        updated: 0,
        unchanged: 0,
        unmatched: 0,
        errors: 0,
    };
    let mut done = 0usize;

    for (system_name, system_entries) in by_system {
        let upstream = match get_system_metadat_cached(&client, &app_data_dir, system_name).await {
            Ok(u) => u,
            Err(e) => {
                log::warn!(
                    "oa-shell: metadata — system {system_name} unavailable: {e}"
                );
                summary.errors += system_entries.len();
                for entry in &system_entries {
                    done += 1;
                    let _ = app.emit(
                        "oa://library-metadata-sync",
                        &MetadataSyncProgress {
                            system_id: systemId.clone(),
                            done,
                            total,
                            current_rom_title: entry.title.clone(),
                            last_action: format!("system unavailable: {system_name}"),
                        },
                    );
                }
                continue;
            }
        };

        if upstream.is_empty() {
            summary.unmatched += system_entries.len();
            for entry in &system_entries {
                done += 1;
                let _ = app.emit(
                    "oa://library-metadata-sync",
                    &MetadataSyncProgress {
                        system_id: systemId.clone(),
                        done,
                        total,
                        current_rom_title: entry.title.clone(),
                        last_action: "no metadat for system".into(),
                    },
                );
            }
            continue;
        }

        for entry in system_entries {
            done += 1;
            // Prefer the canonical no-intro title for matching when the
            // entry has been identified via sha1; fall back to the
            // user's filename-derived title otherwise.
            let match_title = canonical_title_by_id
                .get(&entry.id)
                .map(|s| s.as_str())
                .unwrap_or(&entry.title);
            let rom_norm = crate::normalize::normalize_title(match_title);
            let action: String = if rom_norm.is_empty() {
                summary.unmatched += 1;
                "no match".into()
            } else {
                let mut best: Option<(&UpstreamMetadat, f64)> = None;
                for u in &upstream {
                    let s = crate::normalize::match_score(&rom_norm, &u.normalized);
                    match best {
                        Some((_, bs)) if s <= bs => {}
                        _ => best = Some((u, s)),
                    }
                }
                match best {
                    Some((u, score)) if score >= MATCH_THRESHOLD => {
                        let next_meta = metadata_from(u);
                        let (changed, applied) = {
                            let mut db_w = db
                                .write()
                                .map_err(|_| "media db lock poisoned".to_string())?;
                            let gm = db_w
                                .entry(entry.id.clone())
                                .or_insert_with(GameMedia::default);
                            let changed = metadata_differs(gm.metadata.as_ref(), &next_meta);
                            if changed {
                                gm.metadata = Some(next_meta);
                            }
                            let cloned = gm.clone();
                            if changed {
                                if let Err(e) = write_media_db(&app_data_dir, &db_w) {
                                    log::warn!("oa-shell: write media.json failed: {e}");
                                }
                            }
                            (changed, cloned)
                        };
                        if changed {
                            let _ = app.emit(
                                "oa://media-updated",
                                serde_json::json!({ "romId": &entry.id, "media": &applied }),
                            );
                            summary.matched += 1;
                            summary.updated += 1;
                            format!("matched → {} ({:.2})", u.name, score)
                        } else {
                            summary.matched += 1;
                            summary.unchanged += 1;
                            "unchanged".into()
                        }
                    }
                    _ => {
                        summary.unmatched += 1;
                        "no match".into()
                    }
                }
            };
            let _ = app.emit(
                "oa://library-metadata-sync",
                &MetadataSyncProgress {
                    system_id: systemId.clone(),
                    done,
                    total,
                    current_rom_title: entry.title.clone(),
                    last_action: action,
                },
            );
        }
    }

    let _ = app.emit("oa://library-metadata-sync-complete", &summary);
    log::info!(
        "oa-shell: sync_metadata_for_system({systemId}) done — matched {}, updated {}, unchanged {}, unmatched {}, errors {}",
        summary.matched, summary.updated, summary.unchanged, summary.unmatched, summary.errors
    );
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_metadat_pulls_comment_and_value() {
        // Real libretro-database format: title is `comment "..."`; the
        // top-level `name "..."` inside `clrmamepro (...)` is the system
        // header and must NOT be picked up.
        let dat = r#"clrmamepro (
	name "NEC - PC Engine - TurboGrafx 16"
	description "NEC - PC Engine - TurboGrafx 16"
)

game (
	comment "Bonk's Adventure (USA)"
	genre "Platformer"
	rom ( crc DEADBEEF )
)

game (
	comment "Air Zonk (USA)"
	genre "Shoot 'Em Up"
	rom ( crc CAFEBABE )
)
"#;
        let parsed = parse_metadat_dat(dat, "genre");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0], ("Bonk's Adventure (USA)".into(), "Platformer".into()));
        assert_eq!(parsed[1], ("Air Zonk (USA)".into(), "Shoot 'Em Up".into()));
    }

    #[test]
    fn parse_metadat_skips_game_without_value_field() {
        // Games missing the value field (only have comment + rom record)
        // are skipped — we don't want to fabricate empty values.
        let dat = r#"game (
	comment "Foo"
	rom ( crc 11111111 )
)
game (
	comment "Bar"
	genre "Action"
	rom ( crc 22222222 )
)
"#;
        let parsed = parse_metadat_dat(dat, "genre");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0], ("Bar".into(), "Action".into()));
    }

    #[test]
    fn parse_metadat_releaseyear_round_trips_to_u32() {
        // releaseyear values are quoted in the actual files.
        let dat = r#"game (
	comment "Foo"
	releaseyear "1990"
)
"#;
        let parsed = parse_metadat_dat(dat, "releaseyear");
        assert_eq!(parsed[0].1.parse::<u32>().ok(), Some(1990));
    }

    #[test]
    fn parse_metadat_unquoted_users_value() {
        // `users` (inside the maxusers/ .dat) is unquoted in upstream:
        //     users 2
        // The unquote helper trim_matches('"') is a no-op on unquoted input.
        let dat = r#"game (
	comment "Foo"
	users 4
)
game (
	comment "Bar"
	users 1
)
"#;
        let parsed = parse_metadat_dat(dat, "users");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].1.parse::<u32>().ok(), Some(4));
        assert_eq!(parsed[1].1.parse::<u32>().ok(), Some(1));
    }

    #[test]
    fn parse_metadat_skips_system_header_name_field() {
        // The top-level clrmamepro header has `name "<system>"` — our
        // parser must NOT mistake that for a game title (`in_game` gate).
        let dat = r#"clrmamepro (
	name "Some System"
)

game (
	comment "Real Game"
	genre "Action"
)
"#;
        let parsed = parse_metadat_dat(dat, "genre");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].0, "Real Game");
        assert_ne!(parsed[0].0, "Some System");
    }

    #[test]
    fn metadata_differs_detects_any_field_change() {
        let prior = GameMetadata {
            year: Some(1990),
            genre: Some("Platformer".into()),
            developer: None,
            publisher: None,
            players: Some(1),
            description: None,
        };
        let next_with_developer = GameMetadata {
            year: Some(1990),
            genre: Some("Platformer".into()),
            developer: Some("Red".into()),
            publisher: None,
            players: Some(1),
            description: None,
        };
        assert!(metadata_differs(Some(&prior), &next_with_developer));
        assert!(!metadata_differs(Some(&prior), &prior));
        assert!(metadata_differs(None, &prior));
    }
}
