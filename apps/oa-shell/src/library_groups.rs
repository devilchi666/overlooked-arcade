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

use crate::library_db::GameRow;
use crate::library_prefs::RevisionPriority;
use crate::title_parse::{parse_canonical_title, ParsedTitle};

/// One library tile after grouping. The default variant's metadata
/// (title / cover / etc.) is what the tile renders; `variants` carries
/// every dump the user has installed, in the same ranking order the
/// priority resolver produced.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameGroup {
    pub system_id: String,
    /// Lowercased base title — the group key. Stable across renames /
    /// case differences.
    pub base_title_key: String,
    /// Pretty base title taken from the default variant's parsed name
    /// (i.e. `Castlevania`, not `castlevania`). For display only.
    pub display_base_title: String,
    pub default_variant_id: String,
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
}

/// Build groups out of a flat list. `defaults_by_base_title` maps a
/// group's `base_title_key` to a pinned `game_id` (from the
/// `game_group_defaults` table for the same system).
///
/// `games` may span multiple systems — they group per `(system_id,
/// base_title_key)` so different systems never bleed into one tile.
pub fn build_groups(
    games: Vec<GameRow>,
    region_priority: &[String],
    revision_priority: RevisionPriority,
    defaults_by_base_title: &std::collections::HashMap<(String, String), String>,
) -> Vec<GameGroup> {
    use std::collections::HashMap;

    // Bucket games by (system_id, base_title_key, disc_number). The
    // disc_number is part of the key so multi-disc games stay in
    // separate per-disc buckets — operator clicking the "Run version"
    // submenu on FF7 Disc 1 sees regional variants of Disc 1 only,
    // not Disc 1 + Disc 2 + Disc 3 as if they were regional dumps of
    // the same disc (Phase A1 Sub-phase 4 hotfix 2026-06-04).
    // Single-disc games (disc_number = None) keep the original
    // bucketing behaviour.
    //
    // Note: this requires v20 migration to have stamped disc_number
    // on pre-existing multi-disc identifications; pre-v20 games stay
    // grouped via the title-parser's flags path, which is the
    // pre-Sub-phase-4 behaviour.
    struct Bucket {
        system_id: String,
        base_key: String,
        display_base: String,
        entries: Vec<(GameRow, ParsedTitle)>,
    }
    let mut buckets: HashMap<(String, String, Option<u32>), Bucket> = HashMap::new();
    for g in games {
        let parsed = parse_canonical_title(&g.title);
        let base_key = parsed.base.to_lowercase();
        let key = (g.system_id.clone(), base_key.clone(), g.disc_number);
        let bucket = buckets.entry(key).or_insert_with(|| Bucket {
            system_id: g.system_id.clone(),
            base_key: base_key.clone(),
            display_base: parsed.base.clone(),
            entries: Vec::new(),
        });
        bucket.entries.push((g, parsed));
    }

    let mut out = Vec::with_capacity(buckets.len());
    for (_, mut bucket) in buckets {
        // Rank inside the bucket. We keep the parsed metadata around so
        // we can both decide the default AND surface the parsed shape
        // back to the frontend without re-parsing.
        bucket
            .entries
            .sort_by(|(a_row, a), (b_row, b)| {
                compare_variants(a, b, a_row, b_row, region_priority, revision_priority)
            });

        // Per-group pin overrides the ranking: if the pinned game is
        // present in this bucket, lift it to position 0; otherwise
        // ignore the pin (it points at a deleted/removed game).
        if let Some(pinned_id) = defaults_by_base_title
            .get(&(bucket.system_id.clone(), bucket.base_key.clone()))
        {
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
            })
            .collect();

        let default_variant_id = variants[0].id.clone();
        // Re-derive the display title from the default variant so the
        // tile reads naturally (matters when the alphabetical first
        // bucket entry parses to "castlevania" but the default
        // variant has the better-cased "Castlevania").
        let display_base_title = bucket
            .entries
            .first()
            .map(|(_, p)| p.base.clone())
            .unwrap_or(bucket.display_base);

        out.push(GameGroup {
            system_id: bucket.system_id,
            base_title_key: bucket.base_key,
            display_base_title,
            default_variant_id,
            variants,
        });
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
        let mut defaults = HashMap::new();
        defaults.insert(
            ("tg16".to_string(), "castlevania".to_string()),
            "g_jp".to_string(),
        );
        let groups = build_groups(
            vec![
                row("g_us", "Castlevania (USA)"),
                row("g_jp", "Castlevania (Japan)"),
            ],
            &default_regions(),
            RevisionPriority::Newest,
            &defaults,
        );
        // Without the pin USA would win. The pin lifts JP to position 0.
        assert_eq!(groups[0].variants[0].id, "g_jp");
        assert!(groups[0].variants[0].is_default);
    }

    #[test]
    fn pin_pointing_at_deleted_game_is_silently_ignored() {
        let mut defaults = HashMap::new();
        defaults.insert(
            ("tg16".to_string(), "castlevania".to_string()),
            "g_DOES_NOT_EXIST".to_string(),
        );
        let groups = build_groups(
            vec![
                row("g_us", "Castlevania (USA)"),
                row("g_jp", "Castlevania (Japan)"),
            ],
            &default_regions(),
            RevisionPriority::Newest,
            &defaults,
        );
        // Falls back to priority rules → USA wins.
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
}
