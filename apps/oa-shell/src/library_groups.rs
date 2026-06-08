//! Library aggregation. Takes a flat list of `GameRow`s (one per file)
//! and groups them by `(system_id, base_title)` so that "Castlevania
//! (USA)", "Castlevania (Japan)", and "Castlevania (USA) (Rev 1)"
//! render as one library tile with three variants behind it.
//!
//! Pure functions — no I/O. The Tauri command layer (`main.rs`) loads
//! the flat games + the per-group overrides + the effective priority
//! prefs, then calls into here.
//!
//! ## Default-variant selection
//!
//! For each group:
//!   1. If `game_group_defaults` has a pin for this group, use it.
//!   2. Otherwise rank variants by:
//!      - Release status: release > prerelease (Beta/Proto/Demo/Sample).
//!      - Region: position in `region_priority` (earlier = better).
//!         A variant with no recognised region ranks after every named
//!         region but before the catch-all "Other".
//!      - Revision: per `revision_priority` — Newest = higher revision
//!         wins; Oldest = lower revision wins.
//!      - Stable tiebreaker: `title` ascending (so the output is
//!         deterministic when everything else ties).
//!   3. The single best variant becomes the group's default.

use serde::Serialize;

use crate::library_db::{GameIdentityRow, GameRow};
use crate::library_prefs::RevisionPriority;
use crate::title_parse::{parse_canonical_title, DumpStatus, ParsedTitle};

/// One library tile after grouping. The default variant's metadata
/// (title / cover / etc.) is what the tile renders; `variants` carries
/// every dump the user has installed, in the same ranking order the
/// priority resolver produced.
///
/// VL Phase E Sub-phase 2 — groups are now *backed by identity rows*:
/// the group key is `games.identity_id`, the display title is the
/// identity's `canonical_title`, the pin comes from the identity's
/// `default_variant_id`, and the group carries the identity's canonical
/// metadata + per-identity stat aggregates. Games whose `identity_id`
/// is unstamped (transient state between an insert and the next
/// rebuild) fall back to the pre-Phase-E in-memory parse grouping.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameGroup {
    pub system_id: String,
    /// Lowercased base title — the group key. Stable across renames /
    /// case differences. Identity-backed groups carry the identity's
    /// `normalized_title` here (same normalization).
    pub base_title_key: String,
    /// Pretty base title. Identity-backed groups read the identity's
    /// `canonical_title`; fallback groups derive it from the default
    /// variant's parsed name. For display only.
    pub display_base_title: String,
    /// The `game_identities` row backing this group. `None` only for
    /// transient unstamped rows (heals on the next rebuild).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_id: Option<String>,
    pub default_variant_id: String,
    // --- Canonical metadata pass-through (identity row). NULL until
    // --- the Sub-phase 3 enrichment pass or an operator edit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub developer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub players: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<f64>,
    /// Identity's explicit parent cover when set; otherwise the default
    /// variant's per-file cover. What the tile should render.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_cover_path: Option<String>,
    // --- Per-identity stats, aggregated across ALL the identity's
    // --- files (not just this group's disc bucket — FF7 Disc 1/2/3
    // --- are three groups sharing one identity and one total).
    pub total_play_time_secs: i64,
    /// True when ANY variant is favorited.
    pub favorite: bool,
    /// True when ANY variant is marked completed.
    pub completed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_played_at: Option<i64>,
    /// Every dump in priority order. First entry === default variant
    /// (so frontends that just want "the one to launch" can use index 0).
    pub variants: Vec<GameVariant>,
}

/// One dump (one file) inside a group. The pieces the UI needs to
/// render a "Run version ▸" submenu entry: its `id` (to launch), its
/// canonical-or-raw title, plus parsed region/revision metadata.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameVariant {
    pub id: String,
    pub title: String,
    pub file_path: String,
    pub archive_inner_path: Option<String>,
    pub cover_path: Option<String>,
    pub region: Option<String>,
    pub regions: Vec<String>,
    pub revision: u32,
    pub flags: Vec<String>,
    pub is_prerelease: bool,
    /// True for the group's chosen default variant. The frontend uses
    /// this to render the ✓ marker in the right-click submenus.
    pub is_default: bool,
    // --- Phase A2 typed flag fields (Virtual Library arc) ----------
    // Mirror the new fields on ParsedTitle so the Preservation Vault
    // filter ribbon can read them without re-parsing the title.
    pub dump_status: DumpStatus,
    pub is_hack: bool,
    pub is_translation: bool,
    pub is_pirate: bool,
    pub is_bios: bool,
    pub is_homebrew: bool,
    pub translation_languages: Vec<String>,
    // --- VL Phase E Sub-phase 2 — per-variant stats ----------------
    // The group-level fields aggregate these; the Variants tab
    // (Phase B) renders them per-dump, and the Preservation view's
    // per-variant favorite toggle reads/writes the per-file truth.
    pub favorite: bool,
    pub completed: bool,
    pub play_time_secs: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_played_at: Option<i64>,
}

/// Per-variant filter predicates for the Preservation Vault. Each
/// exclude_* flag drops matching variants from the filtered list. All
/// defaults are `false` — an unset filter passes every variant
/// through. Phase A2 ships the fields; Phase F wires the ribbon UI to
/// drive them.
///
/// Filters are AND-combined: a variant is included only when EVERY
/// active filter passes. Want OR semantics? Compose multiple filter
/// passes at the call site.
// Phase F foundation — shipped in A2, consumed when Phase F wires the
// Preservation Vault filter ribbon. #[allow(dead_code)] on the struct +
// its impl + the predicate keeps the build clean without discarding the
// foundation ahead of its UI.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VariantFilters {
    /// Drop variants flagged as bad dumps (`[b]` / `[b1]` / …).
    pub exclude_bad_dumps: bool,
    /// Drop variants flagged as over-dumps (`[o]` / `[o1]` / …).
    pub exclude_overdumps: bool,
    /// Drop ROM hacks (`[h]` / `(Hack)`).
    pub exclude_hacks: bool,
    /// Drop fan translations (`[T+Xxx]` / `[T-Xxx]`).
    pub exclude_translations: bool,
    /// Drop pirate / cracked dumps.
    pub exclude_pirate: bool,
    /// Drop BIOS / boot-ROM files. The Casual view turns this ON by
    /// default; Preservation view turns it OFF.
    pub exclude_bios: bool,
    /// Drop homebrew / aftermarket releases.
    pub exclude_homebrew: bool,
    /// Drop beta / proto / demo / sample / alpha dumps.
    pub exclude_prerelease: bool,
}

#[allow(dead_code)]
impl VariantFilters {
    /// Default Casual-view filter set: exclude bad dumps + over-dumps
    /// + BIOS + prerelease. Hacks / translations / pirate / homebrew
    /// stay opt-in because operators with niche libraries (translation
    /// archives, homebrew collections) need them visible by default.
    pub fn casual_view_defaults() -> Self {
        Self {
            exclude_bad_dumps: true,
            exclude_overdumps: true,
            exclude_bios: true,
            exclude_prerelease: true,
            ..Self::default()
        }
    }

    /// Preservation-view defaults: show everything. The operator opts
    /// into hiding categories via the filter ribbon.
    pub fn preservation_view_defaults() -> Self {
        Self::default()
    }
}

/// True when `variant` passes every active filter in `filters`.
/// Variants with no filter exclusions always pass.
#[allow(dead_code)]
pub fn variant_passes_filters(variant: &GameVariant, filters: &VariantFilters) -> bool {
    if filters.exclude_bad_dumps && variant.dump_status == DumpStatus::BadDump {
        return false;
    }
    if filters.exclude_overdumps && variant.dump_status == DumpStatus::OverDump {
        return false;
    }
    if filters.exclude_hacks && variant.is_hack {
        return false;
    }
    if filters.exclude_translations && variant.is_translation {
        return false;
    }
    if filters.exclude_pirate && variant.is_pirate {
        return false;
    }
    if filters.exclude_bios && variant.is_bios {
        return false;
    }
    if filters.exclude_homebrew && variant.is_homebrew {
        return false;
    }
    if filters.exclude_prerelease && variant.is_prerelease {
        return false;
    }
    true
}

/// Build groups out of a flat list, backed by `game_identities` rows.
/// `identities` is keyed by identity id (`LibraryDb::list_identities`
/// → map at the call site); each game's `identity_id` stamp picks its
/// group, the identity's `default_variant_id` is the pin, and the
/// identity's canonical metadata rides along on the group.
///
/// Games with no (or an unknown) `identity_id` stamp fall back to the
/// pre-Phase-E in-memory parse grouping — a transient state that heals
/// on the next rebuild (`list_game_groups` triggers one lazily).
///
/// `games` may span multiple systems — identity ids are per-system by
/// construction, and the fallback key includes `system_id`, so
/// different systems never bleed into one tile.
pub fn build_groups(
    games: Vec<GameRow>,
    region_priority: &[String],
    revision_priority: RevisionPriority,
    identities: &std::collections::HashMap<String, GameIdentityRow>,
) -> Vec<GameGroup> {
    use std::collections::HashMap;

    // Bucket games by (group_key, disc_number). The group_key is the
    // identity id when stamped + known, else a parse-derived fallback.
    // The disc_number is part of the key so multi-disc games stay in
    // separate per-disc buckets — operator clicking the "Run version"
    // submenu on FF7 Disc 1 sees regional variants of Disc 1 only,
    // not Disc 1 + Disc 2 + Disc 3 as if they were regional dumps of
    // the same disc (Phase A1 Sub-phase 4 hotfix 2026-06-04).
    // Single-disc games (disc_number = None) keep the original
    // bucketing behaviour.
    struct Bucket {
        system_id: String,
        base_key: String,
        display_base: String,
        identity_id: Option<String>,
        entries: Vec<(GameRow, ParsedTitle)>,
    }
    // Per-identity stat totals, accumulated across ALL the identity's
    // files BEFORE disc bucketing splits them into per-disc groups.
    // Copy so multi-bucket identities (one per disc) each read the
    // same totals without ownership gymnastics.
    #[derive(Default, Clone, Copy)]
    struct Stats {
        play_time_secs: i64,
        favorite: bool,
        completed: bool,
        last_played_at: Option<i64>,
    }
    let mut stats: HashMap<String, Stats> = HashMap::new();
    let mut buckets: HashMap<(String, Option<u32>), Bucket> = HashMap::new();
    for g in games {
        let parsed = parse_canonical_title(&g.title);
        let identity = g
            .identity_id
            .as_ref()
            .and_then(|iid| identities.get(iid));
        // \x1f separators keep a pathological title from colliding
        // with a real identity id or another system's fallback key.
        let group_key = match identity {
            Some(i) => i.id.clone(),
            None => format!("fb\x1f{}\x1f{}", g.system_id, parsed.base.to_lowercase()),
        };

        let s = stats.entry(group_key.clone()).or_default();
        s.play_time_secs += g.play_time_secs;
        s.favorite |= g.favorite;
        s.completed |= g.completed;
        s.last_played_at = s.last_played_at.max(g.last_played_at);

        let bucket = buckets
            .entry((group_key, g.disc_number))
            .or_insert_with(|| match identity {
                Some(i) => Bucket {
                    system_id: g.system_id.clone(),
                    base_key: i.normalized_title.clone(),
                    display_base: i.canonical_title.clone(),
                    identity_id: Some(i.id.clone()),
                    entries: Vec::new(),
                },
                None => Bucket {
                    system_id: g.system_id.clone(),
                    base_key: parsed.base.to_lowercase(),
                    display_base: parsed.base.clone(),
                    identity_id: None,
                    entries: Vec::new(),
                },
            });
        bucket.entries.push((g, parsed));
    }

    let mut out = Vec::with_capacity(buckets.len());
    for ((group_key, _disc), mut bucket) in buckets {
        // Rank inside the bucket. We keep the parsed metadata around so
        // we can both decide the default AND surface the parsed shape
        // back to the frontend without re-parsing.
        bucket
            .entries
            .sort_by(|(a_row, a), (b_row, b)| {
                compare_variants(a, b, a_row, b_row, region_priority, revision_priority)
            });

        let identity = bucket
            .identity_id
            .as_ref()
            .and_then(|iid| identities.get(iid));

        // Per-group pin (identity.default_variant_id) overrides the
        // ranking: if the pinned game is present in this bucket, lift
        // it to position 0; otherwise ignore the pin (it points at a
        // deleted/removed game — or another disc's bucket).
        if let Some(pinned_id) = identity.and_then(|i| i.default_variant_id.as_ref()) {
            if let Some(pos) = bucket.entries.iter().position(|(g, _)| &g.id == pinned_id) {
                if pos != 0 {
                    let entry = bucket.entries.remove(pos);
                    bucket.entries.insert(0, entry);
                }
            }
        }

        let variants: Vec<GameVariant> = bucket
            .entries
            .iter()
            .enumerate()
            .map(|(idx, (row, parsed))| GameVariant {
                id: row.id.clone(),
                title: row.title.clone(),
                file_path: row.file_path.clone(),
                archive_inner_path: row.archive_inner_path.clone(),
                cover_path: row.cover_path.clone(),
                region: parsed.region.clone(),
                regions: parsed.regions.clone(),
                revision: parsed.revision,
                flags: parsed.flags.clone(),
                is_prerelease: parsed.is_prerelease(),
                is_default: idx == 0,
                dump_status: parsed.dump_status,
                is_hack: parsed.is_hack,
                is_translation: parsed.is_translation,
                is_pirate: parsed.is_pirate,
                is_bios: parsed.is_bios,
                is_homebrew: parsed.is_homebrew,
                translation_languages: parsed.translation_languages.clone(),
                favorite: row.favorite,
                completed: row.completed,
                play_time_secs: row.play_time_secs,
                last_played_at: row.last_played_at,
            })
            .collect();

        let default_variant_id = variants[0].id.clone();
        // Display title: the identity's canonical_title is the
        // authority for identity-backed groups. Fallback groups
        // re-derive from the default variant so the tile reads
        // naturally (matters when the alphabetical first bucket entry
        // parses to "castlevania" but the default variant has the
        // better-cased "Castlevania").
        let display_base_title = match identity {
            Some(i) => i.canonical_title.clone(),
            None => bucket
                .entries
                .first()
                .map(|(_, p)| p.base.clone())
                .unwrap_or(bucket.display_base),
        };
        // Tile artwork: explicit identity (parent) cover wins; the
        // default variant's per-file cover is the fallback.
        let canonical_cover_path = identity
            .and_then(|i| i.canonical_cover_path.clone())
            .or_else(|| bucket.entries.first().and_then(|(r, _)| r.cover_path.clone()));
        let group_stats = stats.get(&group_key).copied().unwrap_or_default();
        let group = GameGroup {
            system_id: bucket.system_id,
            base_title_key: bucket.base_key,
            display_base_title,
            identity_id: bucket.identity_id.clone(),
            default_variant_id,
            year: identity.and_then(|i| i.year),
            genre: identity.and_then(|i| i.genre.clone()),
            developer: identity.and_then(|i| i.developer.clone()),
            publisher: identity.and_then(|i| i.publisher.clone()),
            players: identity.and_then(|i| i.players),
            rating: identity.and_then(|i| i.rating),
            canonical_cover_path,
            total_play_time_secs: group_stats.play_time_secs,
            favorite: group_stats.favorite,
            completed: group_stats.completed,
            last_played_at: group_stats.last_played_at,
            variants,
        };
        out.push(group);
    }

    // Stable output order: by display_base_title (case-insensitive).
    out.sort_by(|a, b| {
        a.display_base_title
            .to_lowercase()
            .cmp(&b.display_base_title.to_lowercase())
    });
    out
}

/// Compare two variants under the priority rules. Returns Ordering::Less
/// when `a` should sort before `b` (i.e. `a` is the better default).
fn compare_variants(
    a: &ParsedTitle,
    b: &ParsedTitle,
    a_row: &GameRow,
    b_row: &GameRow,
    region_priority: &[String],
    revision_priority: RevisionPriority,
) -> std::cmp::Ordering {
    use std::cmp::Ordering::*;

    // 1. Release status: release wins over prerelease.
    match (a.is_prerelease(), b.is_prerelease()) {
        (false, true) => return Less,
        (true, false) => return Greater,
        _ => {}
    }

    // 2. Region position. Variants list each region they cover; pick
    // the best region each variant offers (lowest priority index).
    let a_rank = best_region_rank(&a.regions, region_priority);
    let b_rank = best_region_rank(&b.regions, region_priority);
    match a_rank.cmp(&b_rank) {
        Equal => {}
        other => return other,
    }

    // 3. Revision tiebreaker.
    match revision_priority {
        RevisionPriority::Newest => match b.revision.cmp(&a.revision) {
            Equal => {}
            other => return other,
        },
        RevisionPriority::Oldest => match a.revision.cmp(&b.revision) {
            Equal => {}
            other => return other,
        },
    }

    // 4. Stable tiebreaker: title ascending.
    a_row.title.cmp(&b_row.title)
}

/// Position of the best (lowest-index) region this variant offers in
/// the priority list. Variants with no recognised region get a slot
/// after every named region but before any "Other" catch-all.
fn best_region_rank(regions: &[String], priority: &[String]) -> usize {
    let other_idx = priority.iter().position(|r| r == "Other");
    let mut best = usize::MAX;
    for r in regions {
        if let Some(idx) = priority.iter().position(|p| p == r) {
            best = best.min(idx);
        }
    }
    if best != usize::MAX {
        return best;
    }
    // No regions parsed at all → rank just below the named regions but
    // above (or equal to) "Other". The intent: a release dump with no
    // region tag should beat a release dump explicitly marked Other.
    match other_idx {
        Some(idx) => idx.saturating_sub(1).max(priority.len().saturating_sub(2)),
        None => priority.len(), // no catch-all anywhere → push to bottom
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn row(id: &str, title: &str) -> GameRow {
        GameRow {
            id: id.to_string(),
            system_id: "tg16".to_string(),
            file_path: format!("/lib/{}.pce", title),
            title: title.to_string(),
            added_at: 0,
            cover_path: None,
            core_override: None,
            seed: false,
            archive_inner_path: None,
            sha1: None,
            serial: None,
            disc_id: None,
            favorite: false,
            completed: false,
            last_played_at: None,
            play_time_secs: 0,
            players: None,
            rating: None,
            disc_set_id: None,
            disc_number: None,
            identity_id: None,
        }
    }

    fn default_regions() -> Vec<String> {
        vec![
            "USA".to_string(),
            "World".to_string(),
            "Europe".to_string(),
            "Japan".to_string(),
            "Asia".to_string(),
            "Other".to_string(),
        ]
    }

    /// VL Phase E Sub-phase 2 — identity fixture. Mirrors what
    /// `rebuild_identities_for_system` writes; `pin` becomes
    /// `default_variant_id`.
    fn identity(base_key: &str, canonical: &str, pin: Option<&str>) -> GameIdentityRow {
        GameIdentityRow {
            id: format!("idn-test-{base_key}"),
            system_id: "tg16".to_string(),
            canonical_title: canonical.to_string(),
            normalized_title: base_key.to_string(),
            year: None,
            genre: None,
            developer: None,
            publisher: None,
            players: None,
            rating: None,
            canonical_cover_path: None,
            default_variant_id: pin.map(|s| s.to_string()),
        }
    }

    /// Stamp rows with an identity and produce the lookup map the new
    /// `build_groups` signature wants.
    fn stamp(
        rows: &mut [GameRow],
        idn: GameIdentityRow,
    ) -> HashMap<String, GameIdentityRow> {
        for r in rows.iter_mut() {
            r.identity_id = Some(idn.id.clone());
        }
        HashMap::from([(idn.id.clone(), idn)])
    }

    #[test]
    fn single_variant_group_passes_through() {
        let groups = build_groups(
            vec![row("g1", "Castlevania (USA)")],
            &default_regions(),
            RevisionPriority::Newest,
            &HashMap::new(),
        );
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].variants.len(), 1);
        assert_eq!(groups[0].variants[0].is_default, true);
        assert_eq!(groups[0].display_base_title, "Castlevania");
    }

    #[test]
    fn multi_disc_games_stay_in_separate_buckets_per_disc() {
        // Phase A1 Sub-phase 4 hotfix — FF7 Disc 1 / 2 / 3 used to
        // collapse into ONE variant group (the title parser strips
        // "(Disc N)" from the base). After including disc_number in
        // the bucket key, each disc gets its own group so the operator
        // can pick a specific disc via the existing "Run version" menu
        // AND the disc-set collapse / DiscPickerDialog flow can light
        // up via the disc_set_id linkage.
        let mut d1 = row("ff7-d1", "Final Fantasy VII (USA) (Disc 1)");
        d1.system_id = "psx".into();
        d1.disc_number = Some(1);
        let mut d2 = row("ff7-d2", "Final Fantasy VII (USA) (Disc 2)");
        d2.system_id = "psx".into();
        d2.disc_number = Some(2);
        let mut d3 = row("ff7-d3", "Final Fantasy VII (USA) (Disc 3)");
        d3.system_id = "psx".into();
        d3.disc_number = Some(3);
        let groups = build_groups(
            vec![d1, d2, d3],
            &default_regions(),
            RevisionPriority::Newest,
            &HashMap::new(),
        );
        assert_eq!(
            groups.len(),
            3,
            "three discs become three separate variant groups (one per disc), \
             NOT one 3-variant group where all three discs claim to be regional \
             variants of each other"
        );
        for g in &groups {
            assert_eq!(g.variants.len(), 1, "each disc bucket has one variant");
        }
    }

    #[test]
    fn multi_disc_multi_region_groups_per_disc_across_regions() {
        // FF7 Disc 1 in three regions should form ONE 3-variant group
        // (USA + Japan + Europe variants of Disc 1), and similarly for
        // Disc 2 and Disc 3. End state: 3 groups (one per disc) each
        // with 3 variants (one per region).
        let mut games = Vec::new();
        for (region_id, region) in [("us", "USA"), ("jp", "Japan"), ("eu", "Europe")] {
            for disc in [1u32, 2, 3] {
                let id = format!("ff7-{region_id}-d{disc}");
                let title = format!("Final Fantasy VII ({region}) (Disc {disc})");
                let mut g = row(&id, &title);
                g.system_id = "psx".into();
                g.disc_number = Some(disc);
                games.push(g);
            }
        }
        let groups = build_groups(
            games,
            &default_regions(),
            RevisionPriority::Newest,
            &HashMap::new(),
        );
        assert_eq!(groups.len(), 3, "one group per disc_number");
        for g in &groups {
            assert_eq!(
                g.variants.len(),
                3,
                "each disc bucket has 3 regional variants"
            );
            // Default variant ranking: USA wins per the default region
            // priority.
            assert!(
                g.variants[0].id.contains("-us-"),
                "USA picked as default within each disc bucket"
            );
        }
    }

    #[test]
    fn usa_wins_over_japan_by_default() {
        let groups = build_groups(
            vec![
                row("g_jp", "Castlevania (Japan)"),
                row("g_us", "Castlevania (USA)"),
            ],
            &default_regions(),
            RevisionPriority::Newest,
            &HashMap::new(),
        );
        assert_eq!(groups.len(), 1, "both should group as one");
        assert_eq!(groups[0].variants.len(), 2);
        assert_eq!(groups[0].variants[0].id, "g_us");
        assert_eq!(groups[0].variants[0].is_default, true);
        assert_eq!(groups[0].variants[1].id, "g_jp");
        assert_eq!(groups[0].variants[1].is_default, false);
    }

    #[test]
    fn newer_revision_wins_within_same_region() {
        let groups = build_groups(
            vec![
                row("g_v0", "Final Fantasy III (USA)"),
                row("g_v1", "Final Fantasy III (USA) (Rev 1)"),
            ],
            &default_regions(),
            RevisionPriority::Newest,
            &HashMap::new(),
        );
        assert_eq!(groups[0].variants[0].id, "g_v1");
    }

    #[test]
    fn revision_priority_oldest_inverts() {
        let groups = build_groups(
            vec![
                row("g_v0", "Final Fantasy III (USA)"),
                row("g_v1", "Final Fantasy III (USA) (Rev 1)"),
            ],
            &default_regions(),
            RevisionPriority::Oldest,
            &HashMap::new(),
        );
        assert_eq!(groups[0].variants[0].id, "g_v0");
    }

    #[test]
    fn release_outranks_beta_even_at_better_region() {
        // A USA Beta vs a Japan Release. The Release should win even
        // though USA is higher priority — release status outranks region.
        let groups = build_groups(
            vec![
                row("g_us_beta", "Castlevania (USA) (Beta)"),
                row("g_jp", "Castlevania (Japan)"),
            ],
            &default_regions(),
            RevisionPriority::Newest,
            &HashMap::new(),
        );
        assert_eq!(groups[0].variants[0].id, "g_jp");
        // ...but both still appear, so the user can pick the beta if they want.
        assert_eq!(groups[0].variants.len(), 2);
        assert!(groups[0].variants[1].is_prerelease);
    }

    #[test]
    fn per_group_pin_overrides_priority_rules() {
        let mut rows = vec![
            row("g_us", "Castlevania (USA)"),
            row("g_jp", "Castlevania (Japan)"),
        ];
        let identities = stamp(
            &mut rows,
            identity("castlevania", "Castlevania", Some("g_jp")),
        );
        let groups = build_groups(
            rows,
            &default_regions(),
            RevisionPriority::Newest,
            &identities,
        );
        // Without the pin USA would win. The identity's
        // default_variant_id lifts JP to position 0.
        assert_eq!(groups[0].variants[0].id, "g_jp");
        assert!(groups[0].variants[0].is_default);
    }

    #[test]
    fn pin_pointing_at_deleted_game_is_silently_ignored() {
        let mut rows = vec![
            row("g_us", "Castlevania (USA)"),
            row("g_jp", "Castlevania (Japan)"),
        ];
        let identities = stamp(
            &mut rows,
            identity("castlevania", "Castlevania", Some("g_DOES_NOT_EXIST")),
        );
        let groups = build_groups(
            rows,
            &default_regions(),
            RevisionPriority::Newest,
            &identities,
        );
        // Falls back to priority rules → USA wins.
        assert_eq!(groups[0].variants[0].id, "g_us");
    }

    // --- VL Phase E Sub-phase 2 — identity-backed groups ---------------

    #[test]
    fn identity_canonical_title_is_display_authority() {
        // The identity's canonical_title wins over whatever the default
        // variant's filename parses to — an operator-curated or
        // enrichment-set canonical survives badly-cased dumps.
        let mut rows = vec![row("g1", "CASTLEVANIA (USA) [b1]")];
        let identities = stamp(&mut rows, identity("castlevania", "Castlevania", None));
        let groups = build_groups(
            rows,
            &default_regions(),
            RevisionPriority::Newest,
            &identities,
        );
        assert_eq!(groups[0].display_base_title, "Castlevania");
        assert_eq!(groups[0].identity_id.as_deref(), Some("idn-test-castlevania"));
        assert_eq!(groups[0].base_title_key, "castlevania");
    }

    #[test]
    fn identity_metadata_and_cover_ride_on_the_group() {
        let mut rows = vec![row("g1", "Castlevania (USA)")];
        let mut idn = identity("castlevania", "Castlevania", None);
        idn.year = Some(1989);
        idn.genre = Some("Platformer".to_string());
        idn.canonical_cover_path = Some("/media/identity/castlevania.png".to_string());
        let identities = stamp(&mut rows, idn);
        let groups = build_groups(
            rows,
            &default_regions(),
            RevisionPriority::Newest,
            &identities,
        );
        assert_eq!(groups[0].year, Some(1989));
        assert_eq!(groups[0].genre.as_deref(), Some("Platformer"));
        assert_eq!(
            groups[0].canonical_cover_path.as_deref(),
            Some("/media/identity/castlevania.png"),
            "explicit identity cover wins"
        );
    }

    #[test]
    fn group_cover_falls_back_to_default_variant() {
        let mut us = row("g_us", "Castlevania (USA)");
        us.cover_path = Some("/covers/us.png".to_string());
        let jp = row("g_jp", "Castlevania (Japan)");
        let mut rows = vec![us, jp];
        let identities = stamp(&mut rows, identity("castlevania", "Castlevania", None));
        let groups = build_groups(
            rows,
            &default_regions(),
            RevisionPriority::Newest,
            &identities,
        );
        assert_eq!(
            groups[0].canonical_cover_path.as_deref(),
            Some("/covers/us.png"),
            "no explicit identity cover → default variant's (USA wins ranking)"
        );
    }

    #[test]
    fn identity_stats_aggregate_across_all_variants_and_discs() {
        // 2 discs + a regional variant of disc 1, all one identity.
        // Stats must aggregate across ALL THREE files and appear
        // identically on BOTH per-disc groups.
        let mut d1us = row("d1us", "Final Fantasy VII (USA) (Disc 1)");
        d1us.disc_number = Some(1);
        d1us.play_time_secs = 100;
        d1us.favorite = true;
        d1us.last_played_at = Some(1000);
        let mut d1jp = row("d1jp", "Final Fantasy VII (Japan) (Disc 1)");
        d1jp.disc_number = Some(1);
        d1jp.play_time_secs = 30;
        let mut d2 = row("d2", "Final Fantasy VII (USA) (Disc 2)");
        d2.disc_number = Some(2);
        d2.play_time_secs = 70;
        d2.completed = true;
        d2.last_played_at = Some(2000);

        let mut rows = vec![d1us, d1jp, d2];
        let identities = stamp(
            &mut rows,
            identity("final fantasy vii", "Final Fantasy VII", None),
        );
        let groups = build_groups(
            rows,
            &default_regions(),
            RevisionPriority::Newest,
            &identities,
        );
        assert_eq!(groups.len(), 2, "one group per disc, same identity");
        for g in &groups {
            assert_eq!(g.total_play_time_secs, 200, "summed across discs + regions");
            assert!(g.favorite, "any-variant favorite");
            assert!(g.completed, "any-variant completed");
            assert_eq!(g.last_played_at, Some(2000), "max across variants");
        }
        // Per-variant stats pass through untouched.
        let d1 = groups
            .iter()
            .find(|g| g.variants.iter().any(|v| v.id == "d1us"))
            .unwrap();
        let v = d1.variants.iter().find(|v| v.id == "d1us").unwrap();
        assert_eq!(v.play_time_secs, 100);
        assert!(v.favorite);
    }

    #[test]
    fn unstamped_games_fall_back_to_parse_grouping() {
        // identity_id = None (transient pre-rebuild state) — grouping
        // still works via the in-memory parse path, no identity fields.
        let groups = build_groups(
            vec![
                row("g_us", "Castlevania (USA)"),
                row("g_jp", "Castlevania (Japan)"),
            ],
            &default_regions(),
            RevisionPriority::Newest,
            &HashMap::new(),
        );
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].identity_id, None);
        assert_eq!(groups[0].display_base_title, "Castlevania");
        assert_eq!(groups[0].variants[0].id, "g_us");
    }

    #[test]
    fn cross_system_titles_dont_collide() {
        // Same title on two systems must produce two groups, not one.
        let mut a = row("g_a", "Castlevania (USA)");
        a.system_id = "nes".into();
        let mut b = row("g_b", "Castlevania (USA)");
        b.system_id = "snes".into();
        let groups = build_groups(
            vec![a, b],
            &default_regions(),
            RevisionPriority::Newest,
            &HashMap::new(),
        );
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn user_renamed_game_with_no_annotations_is_its_own_group() {
        let groups = build_groups(
            vec![row("g_user", "My favourite game")],
            &default_regions(),
            RevisionPriority::Newest,
            &HashMap::new(),
        );
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].variants.len(), 1);
        assert_eq!(groups[0].display_base_title, "My favourite game");
    }

    #[test]
    fn multi_region_dump_uses_best_named_region() {
        // `(USA, Europe)` should rank as USA (its best region), beating
        // a Japan-only dump.
        let groups = build_groups(
            vec![
                row("g_we", "Foo (USA, Europe)"),
                row("g_jp", "Foo (Japan)"),
            ],
            &default_regions(),
            RevisionPriority::Newest,
            &HashMap::new(),
        );
        assert_eq!(groups[0].variants[0].id, "g_we");
    }

    #[test]
    fn groups_output_order_is_alphabetical_by_base_title() {
        let groups = build_groups(
            vec![
                row("g_z", "Zelda (USA)"),
                row("g_a", "Astro Boy (Japan)"),
                row("g_m", "Mario Bros (USA)"),
            ],
            &default_regions(),
            RevisionPriority::Newest,
            &HashMap::new(),
        );
        assert_eq!(groups[0].display_base_title, "Astro Boy");
        assert_eq!(groups[1].display_base_title, "Mario Bros");
        assert_eq!(groups[2].display_base_title, "Zelda");
    }

    // --- Phase A2: typed flag propagation + VariantFilters --------------

    #[test]
    fn typed_flags_propagate_from_parsed_title_to_variant() {
        // Building a group from titles with Phase A2 flags should
        // produce variants that mirror the parsed fields. Pins the
        // wiring between title_parse and library_groups.
        let groups = build_groups(
            vec![
                row("g_verified", "Test Title (USA) [!]"),
                row("g_bad", "Test Title (USA) [b1]"),
                row("g_hack", "Test Title (USA) [h]"),
                row("g_tx", "Test Title (Japan) [T+Eng]"),
            ],
            &default_regions(),
            RevisionPriority::Newest,
            &HashMap::new(),
        );
        assert_eq!(groups.len(), 1, "all four parse to the same base title");
        let by_id: HashMap<&str, &GameVariant> = groups[0]
            .variants
            .iter()
            .map(|v| (v.id.as_str(), v))
            .collect();
        assert_eq!(by_id["g_verified"].dump_status, DumpStatus::Verified);
        assert_eq!(by_id["g_bad"].dump_status, DumpStatus::BadDump);
        assert!(by_id["g_hack"].is_hack);
        assert!(by_id["g_tx"].is_translation);
        assert_eq!(
            by_id["g_tx"].translation_languages,
            vec!["Eng".to_string()]
        );
    }

    /// Helper — build a single-variant group from a title and pull out
    /// the variant. Phase A2 filter tests use this to keep arrangement
    /// terse.
    fn variant_from(id: &str, title: &str) -> GameVariant {
        let g = build_groups(
            vec![row(id, title)],
            &default_regions(),
            RevisionPriority::Newest,
            &HashMap::new(),
        );
        g.into_iter().next().unwrap().variants.into_iter().next().unwrap()
    }

    #[test]
    fn default_filters_pass_every_variant() {
        // No filter active → every variant passes, including clearly
        // "bad" ones. Defaults are deliberately permissive so the
        // Preservation view shows everything by default.
        let filters = VariantFilters::default();
        assert!(variant_passes_filters(
            &variant_from("a", "X (USA)"),
            &filters
        ));
        assert!(variant_passes_filters(
            &variant_from("b", "X (USA) [b1]"),
            &filters
        ));
        assert!(variant_passes_filters(
            &variant_from("c", "X (USA) (Hack)"),
            &filters
        ));
        assert!(variant_passes_filters(
            &variant_from("d", "X (USA) (BIOS)"),
            &filters
        ));
    }

    #[test]
    fn exclude_bad_dumps_drops_bad_keeps_clean() {
        let filters = VariantFilters {
            exclude_bad_dumps: true,
            ..VariantFilters::default()
        };
        assert!(variant_passes_filters(
            &variant_from("clean", "X (USA)"),
            &filters
        ));
        assert!(!variant_passes_filters(
            &variant_from("bad", "X (USA) [b]"),
            &filters
        ));
        assert!(!variant_passes_filters(
            &variant_from("bad1", "X (USA) [b1]"),
            &filters
        ));
    }

    #[test]
    fn exclude_overdumps_drops_only_overdumps() {
        let filters = VariantFilters {
            exclude_overdumps: true,
            ..VariantFilters::default()
        };
        // OverDumps drop.
        assert!(!variant_passes_filters(
            &variant_from("o", "X (USA) [o]"),
            &filters
        ));
        // Bad dumps pass (this filter only excludes overdumps).
        assert!(variant_passes_filters(
            &variant_from("b", "X (USA) [b]"),
            &filters
        ));
    }

    #[test]
    fn exclude_hacks_drops_bracket_and_paren_forms() {
        let filters = VariantFilters {
            exclude_hacks: true,
            ..VariantFilters::default()
        };
        assert!(!variant_passes_filters(
            &variant_from("h_bracket", "X (USA) [h]"),
            &filters
        ));
        assert!(!variant_passes_filters(
            &variant_from("h_paren", "X (USA) (Hack)"),
            &filters
        ));
        // Non-hacks pass.
        assert!(variant_passes_filters(
            &variant_from("clean", "X (USA)"),
            &filters
        ));
    }

    #[test]
    fn exclude_translations_drops_t_plus_and_t_minus() {
        let filters = VariantFilters {
            exclude_translations: true,
            ..VariantFilters::default()
        };
        assert!(!variant_passes_filters(
            &variant_from("t_plus", "X (Japan) [T+Eng]"),
            &filters
        ));
        assert!(!variant_passes_filters(
            &variant_from("t_minus", "X (Japan) [T-Eng]"),
            &filters
        ));
        assert!(variant_passes_filters(
            &variant_from("clean", "X (USA)"),
            &filters
        ));
    }

    #[test]
    fn exclude_pirate_drops_all_pirate_forms() {
        let filters = VariantFilters {
            exclude_pirate: true,
            ..VariantFilters::default()
        };
        assert!(!variant_passes_filters(
            &variant_from("p", "X (USA) [p]"),
            &filters
        ));
        assert!(!variant_passes_filters(
            &variant_from("pirate", "X (USA) (Pirate)"),
            &filters
        ));
        assert!(!variant_passes_filters(
            &variant_from("cracked", "X (USA) (Cracked)"),
            &filters
        ));
    }

    #[test]
    fn exclude_bios_drops_bios_files() {
        let filters = VariantFilters {
            exclude_bios: true,
            ..VariantFilters::default()
        };
        assert!(!variant_passes_filters(
            &variant_from("bios", "PSX BIOS (USA) (BIOS)"),
            &filters
        ));
        // Real games pass.
        assert!(variant_passes_filters(
            &variant_from("game", "Some Game (USA)"),
            &filters
        ));
    }

    #[test]
    fn exclude_homebrew_drops_homebrew_and_aftermarket() {
        let filters = VariantFilters {
            exclude_homebrew: true,
            ..VariantFilters::default()
        };
        assert!(!variant_passes_filters(
            &variant_from("hb", "Indie Game (USA) (Homebrew)"),
            &filters
        ));
        assert!(!variant_passes_filters(
            &variant_from("am", "New NES Game (Unknown) (Aftermarket)"),
            &filters
        ));
        // Unl is NOT homebrew (pinned at the title_parse layer too).
        assert!(variant_passes_filters(
            &variant_from("unl", "Bible Adventures (USA) (Unl)"),
            &filters
        ));
    }

    #[test]
    fn exclude_prerelease_drops_beta_proto_demo() {
        let filters = VariantFilters {
            exclude_prerelease: true,
            ..VariantFilters::default()
        };
        assert!(!variant_passes_filters(
            &variant_from("beta", "X (Japan) (Beta)"),
            &filters
        ));
        assert!(!variant_passes_filters(
            &variant_from("proto", "X (USA) (Proto)"),
            &filters
        ));
        assert!(!variant_passes_filters(
            &variant_from("demo", "X (USA) (Demo)"),
            &filters
        ));
        assert!(variant_passes_filters(
            &variant_from("release", "X (USA)"),
            &filters
        ));
    }

    #[test]
    fn filters_are_and_combined() {
        // A variant excluded by ANY active filter is dropped. A clean
        // release passes all filters even when many are active.
        let filters = VariantFilters {
            exclude_bad_dumps: true,
            exclude_hacks: true,
            exclude_pirate: true,
            ..VariantFilters::default()
        };
        // Hits two filters → dropped.
        assert!(!variant_passes_filters(
            &variant_from("multi", "X (USA) [b1] [h]"),
            &filters
        ));
        // Hits zero filters → passes.
        assert!(variant_passes_filters(
            &variant_from("clean", "X (USA)"),
            &filters
        ));
    }

    #[test]
    fn casual_view_defaults_drop_noise_keep_real_games() {
        let filters = VariantFilters::casual_view_defaults();
        // Real released game passes.
        assert!(variant_passes_filters(
            &variant_from("game", "Castlevania (USA)"),
            &filters
        ));
        // Noise categories drop.
        assert!(!variant_passes_filters(
            &variant_from("bad", "X (USA) [b]"),
            &filters
        ));
        assert!(!variant_passes_filters(
            &variant_from("over", "X (USA) [o]"),
            &filters
        ));
        assert!(!variant_passes_filters(
            &variant_from("bios", "PSX BIOS (USA) (BIOS)"),
            &filters
        ));
        assert!(!variant_passes_filters(
            &variant_from("beta", "X (USA) (Beta)"),
            &filters
        ));
        // Hacks / translations / pirate / homebrew stay opt-in →
        // pass the Casual defaults.
        assert!(variant_passes_filters(
            &variant_from("hack", "X (USA) (Hack)"),
            &filters
        ));
        assert!(variant_passes_filters(
            &variant_from("tx", "X (Japan) [T+Eng]"),
            &filters
        ));
    }

    #[test]
    fn preservation_view_defaults_pass_everything() {
        // Preservation view starts permissive — operator opts in to
        // exclusions via the filter ribbon.
        let filters = VariantFilters::preservation_view_defaults();
        assert!(variant_passes_filters(
            &variant_from("game", "X (USA)"),
            &filters
        ));
        assert!(variant_passes_filters(
            &variant_from("bad", "X (USA) [b]"),
            &filters
        ));
        assert!(variant_passes_filters(
            &variant_from("bios", "PSX BIOS (USA) (BIOS)"),
            &filters
        ));
    }
}
