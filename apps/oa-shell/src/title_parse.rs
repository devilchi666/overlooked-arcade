//! Parse No-Intro / Redump canonical title strings into structured
//! metadata so the library can group multi-region / multi-revision dumps
//! of the same underlying game as one library item.
//!
//! No-Intro names follow a regular pattern:
//!   `<Base Title> (Region[, Region]*) [(Rev N|A|v1.1)] [(Flag)]* [Bracket]*`
//!
//! Examples (every shape seen across NES/SNES/TG-16/Lynx/Atari 7800):
//!   - `Castlevania (USA)`
//!   - `Castlevania (Japan)`
//!   - `Super Mario Bros. (World) (Rev A)`
//!   - `Final Fantasy III (USA) (Rev 1)`
//!   - `Bonk's Adventure (USA, Europe)`
//!   - `Sonic the Hedgehog (USA, Europe) (En,Fr,De,Es,It)`
//!   - `Pokemon - Crystal Version (USA, Europe) (Rev 1)`
//!   - `Castlevania (Japan) (Beta)`
//!   - `Some Game (USA) (Proto)`
//!   - `Test Title (USA) [!]`            (GoodTools-style bracket flag)
//!   - `Hacked Title (USA) [h]`
//!
//! Trailing parens we don't recognise (language list, publisher tag,
//! arbitrary annotation) are stripped from the base but captured as
//! flags so we don't silently lose them.

/// Structured view of a canonical title. The `base` is the grouping
/// key — everything else is metadata used by the priority resolver
/// and the UI surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedTitle {
    /// Title with all parenthesised / bracketed annotations stripped.
    /// Used as the group key alongside `system_id`.
    pub base: String,
    /// First recognised region in the parens (e.g. "USA", "Japan").
    /// `None` when no parens look region-shaped.
    pub region: Option<String>,
    /// All regions when the dump is multi-region (e.g. `(USA, Europe)` →
    /// `["USA", "Europe"]`). The first element matches `region`.
    pub regions: Vec<String>,
    /// Numeric revision. `(Rev 1)` → `1`; `(Rev A)` → `1` (A=1, B=2…);
    /// `(v1.1)` → `1` (the integer part). Defaults to 0 for the
    /// unrevised release — newer-revision-wins still picks the higher
    /// revision over the base release.
    pub revision: u32,
    /// Catch-all for non-region / non-revision parens + brackets, in
    /// the order they appeared. `(Beta)`, `(Proto)`, `(Demo)`, `(Sample)`,
    /// `[!]`, `[h]`, `[t]`, language lists, etc.
    pub flags: Vec<String>,
}

impl ParsedTitle {
    /// True if this dump is flagged as a non-release variant we should
    /// usually deprioritise (Beta, Proto, Demo, Sample, Hack). The
    /// priority resolver consults this when picking a default variant
    /// — a release dump outranks a beta of the same region+revision.
    pub fn is_prerelease(&self) -> bool {
        self.flags.iter().any(|f| {
            matches!(f.as_str(), "Beta" | "Proto" | "Prototype" | "Demo" | "Sample" | "Alpha")
        })
    }
}

/// Parse a No-Intro / Redump style title. Always returns a `ParsedTitle`;
/// if no annotations are found the result is `{ base: input, region:
/// None, regions: [], revision: 0, flags: [] }` — i.e. user-renamed
/// games still form their own one-entry group cleanly.
pub fn parse_canonical_title(s: &str) -> ParsedTitle {
    let trimmed = s.trim();
    // First pass — split off annotations. Annotations are spans
    // delimited by `(...)` or `[...]` at the END of the title.
    let (base, annotations) = split_base_and_annotations(trimmed);

    let mut region: Option<String> = None;
    let mut regions: Vec<String> = Vec::new();
    let mut revision: u32 = 0;
    let mut flags: Vec<String> = Vec::new();

    for ann in annotations {
        if let Some(parsed_regions) = classify_region_list(&ann) {
            if region.is_none() {
                region = parsed_regions.first().cloned();
                regions = parsed_regions;
            } else {
                // Second region paren — rare; keep as flag for transparency.
                flags.push(ann);
            }
        } else if let Some(rev) = classify_revision(&ann) {
            // First revision wins; subsequent (Rev …) parens become flags.
            if revision == 0 {
                revision = rev;
            } else {
                flags.push(ann);
            }
        } else {
            flags.push(ann);
        }
    }

    ParsedTitle {
        base: base.trim().to_string(),
        region,
        regions,
        revision,
        flags,
    }
}

/// Split `Castlevania (USA) (Rev 1) [!]` into `("Castlevania", ["USA",
/// "Rev 1", "!"])`. Annotations are taken from the END of the string
/// — a paren in the middle of a title (e.g. `Pac-Man (32k) Edition`)
/// stays part of the base. We stop walking back the moment we hit a
/// non-annotation token.
fn split_base_and_annotations(s: &str) -> (String, Vec<String>) {
    let mut annotations: Vec<String> = Vec::new();
    let mut remaining = s.to_string();
    loop {
        let trimmed = remaining.trim_end();
        let bytes = trimmed.as_bytes();
        if bytes.is_empty() {
            break;
        }
        let last = *bytes.last().unwrap() as char;
        let (open, close) = match last {
            ')' => ('(', ')'),
            ']' => ('[', ']'),
            _ => break,
        };
        // Walk backwards from the closing delimiter to its matching
        // opener, respecting nesting (rare but possible —
        // `(USA, Europe) (En,Fr,De)`-style chains have no nesting, but
        // some titles use `( ... ( ... ) ... )`).
        let close_idx = trimmed.len() - 1;
        let mut depth = 1;
        let mut open_idx = None;
        // Iterate over (byte_idx, char) walking backwards. Skip the
        // closing delimiter itself.
        let mut i = close_idx;
        while i > 0 {
            i -= 1;
            let c = trimmed.as_bytes()[i] as char;
            if c == close {
                depth += 1;
            } else if c == open {
                depth -= 1;
                if depth == 0 {
                    open_idx = Some(i);
                    break;
                }
            }
        }
        let Some(open_idx) = open_idx else { break };
        // Extract the inner content (without the delimiters) and the
        // new remaining string (everything before the opener).
        let inner = trimmed[open_idx + 1..close_idx].trim().to_string();
        let new_remaining = trimmed[..open_idx].to_string();
        annotations.push(inner);
        remaining = new_remaining;
    }
    annotations.reverse(); // we collected from right to left
    (remaining.trim().to_string(), annotations)
}

/// Decide whether an annotation looks like a region list. Returns the
/// region names in input order; `None` when this annotation isn't a
/// region declaration.
///
/// Recognised regions are conservative — we only return `Some` when
/// EVERY token in the annotation is a known region. A list like
/// `(En,Fr,De,Es)` (language codes, not regions) returns `None` and
/// becomes a flag.
fn classify_region_list(ann: &str) -> Option<Vec<String>> {
    let tokens: Vec<&str> = ann.split(',').map(|t| t.trim()).collect();
    if tokens.is_empty() {
        return None;
    }
    let mut out = Vec::with_capacity(tokens.len());
    for t in tokens {
        let canonical = canonical_region_name(t)?;
        out.push(canonical);
    }
    Some(out)
}

/// Map a region token to its canonical name, or `None` if it isn't a
/// region we recognise. Canonical names match the defaults in
/// `regionPriority` so the priority resolver can do plain `==` matches.
fn canonical_region_name(token: &str) -> Option<String> {
    Some(match token {
        "USA" | "U" | "US" => "USA",
        "Europe" | "E" | "EUR" | "EU" => "Europe",
        "Japan" | "J" | "JP" | "JPN" => "Japan",
        "World" | "W" => "World",
        "Asia" | "A" => "Asia",
        "Korea" | "K" | "KR" => "Korea",
        "Brazil" | "B" | "BR" => "Brazil",
        "Australia" | "AU" | "AUS" => "Australia",
        "China" | "CN" => "China",
        "Taiwan" | "TW" => "Taiwan",
        "Spain" | "S" | "ES" => "Spain",
        "France" | "F" | "FR" => "France",
        "Germany" | "G" | "DE" => "Germany",
        "Italy" | "I" | "IT" => "Italy",
        "Netherlands" | "NL" => "Netherlands",
        "Sweden" | "SE" => "Sweden",
        "Canada" | "CA" => "Canada",
        "Russia" | "RU" => "Russia",
        "Hong Kong" | "HK" => "Hong Kong",
        // Multi-region shorthands seen in No-Intro:
        "Unl" | "Unlicensed" => return None, // not a region — let it become a flag
        _ => return None,
    }
    .to_string())
}

/// Decide whether an annotation encodes a revision number. Recognises:
///   - `Rev 1`, `Rev 2`, ... `Rev N`  → N
///   - `Rev A`, `Rev B`, ...           → 1, 2, ... (No-Intro/GoodTools alpha)
///   - `v1.0`, `v1.1`, `v2`            → integer part
///   - `Revision 1` (rare, mostly Redump) → N
fn classify_revision(ann: &str) -> Option<u32> {
    let lower = ann.to_ascii_lowercase();
    let body = lower
        .strip_prefix("rev ")
        .or_else(|| lower.strip_prefix("revision "))
        .or_else(|| lower.strip_prefix('v'))?;
    let body = body.trim();
    // Alpha form: a single letter A-Z → 1-26.
    if body.len() == 1 {
        let c = body.chars().next().unwrap();
        if c.is_ascii_alphabetic() {
            return Some((c.to_ascii_uppercase() as u32) - ('A' as u32) + 1);
        }
    }
    // Numeric or v1.1 form: parse the leading integer.
    let mut digits = String::new();
    for c in body.chars() {
        if c.is_ascii_digit() {
            digits.push(c);
        } else {
            break;
        }
    }
    if digits.is_empty() {
        return None;
    }
    digits.parse::<u32>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> ParsedTitle {
        parse_canonical_title(s)
    }

    #[test]
    fn user_rename_with_no_annotations_groups_alone() {
        let p = parse("My favourite game");
        assert_eq!(p.base, "My favourite game");
        assert_eq!(p.region, None);
        assert!(p.regions.is_empty());
        assert_eq!(p.revision, 0);
        assert!(p.flags.is_empty());
    }

    // --- MAME clone grouping --------------------------------------------
    //
    // MAME ROM-sets carry their region / revision / variant in trailing
    // parenthesized annotations: "Street Fighter II (World, set 1)" vs
    // "Street Fighter II (Japan)" vs "Street Fighter II (US set 1)".
    // For variant-grouping to collapse clones under one library tile,
    // every variant must produce the SAME `base` after parsing. These
    // tests pin that behaviour so the existing library_groups infra
    // handles MAME clones without per-system special-casing.

    #[test]
    fn mame_world_set_strips_to_base() {
        let p = parse("Street Fighter II: Champion Edition (World 920313)");
        assert_eq!(p.base, "Street Fighter II: Champion Edition");
    }

    #[test]
    fn mame_world_set_and_japan_share_base() {
        let world = parse("Street Fighter II: Champion Edition (World 920313)").base;
        let japan = parse("Street Fighter II: Champion Edition (Japan)").base;
        assert_eq!(world, japan,
            "MAME clones must share base title so library_groups collapses them");
    }

    #[test]
    fn mame_us_set_1_strips_set_annotation() {
        let p = parse("Donkey Kong (US set 1)");
        assert_eq!(p.base, "Donkey Kong");
    }

    #[test]
    fn mame_licensee_annotation_strips() {
        // "(Midway)" / "(Capcom)" / "(Taito license)" are MAME's
        // licensee tags. They strip alongside region tags.
        let p = parse("Pac-Man (Midway)");
        assert_eq!(p.base, "Pac-Man");
    }

    #[test]
    fn single_region_usa() {
        let p = parse("Castlevania (USA)");
        assert_eq!(p.base, "Castlevania");
        assert_eq!(p.region.as_deref(), Some("USA"));
        assert_eq!(p.regions, vec!["USA".to_string()]);
        assert_eq!(p.revision, 0);
    }

    #[test]
    fn single_region_japan() {
        let p = parse("Castlevania (Japan)");
        assert_eq!(p.region.as_deref(), Some("Japan"));
        assert_eq!(p.base, "Castlevania");
    }

    #[test]
    fn multi_region_usa_europe() {
        let p = parse("Bonk's Adventure (USA, Europe)");
        assert_eq!(p.region.as_deref(), Some("USA"));
        assert_eq!(p.regions, vec!["USA".to_string(), "Europe".to_string()]);
    }

    #[test]
    fn revision_numeric() {
        let p = parse("Final Fantasy III (USA) (Rev 1)");
        assert_eq!(p.base, "Final Fantasy III");
        assert_eq!(p.region.as_deref(), Some("USA"));
        assert_eq!(p.revision, 1);
    }

    #[test]
    fn revision_alpha_a_is_one_b_is_two() {
        // No-Intro uses (Rev A) / (Rev B) as letter-revision shorthand.
        // Treat them as 1/2 so newest-wins prefers B > A.
        assert_eq!(parse("Super Mario Bros. (World) (Rev A)").revision, 1);
        assert_eq!(parse("Super Mario Bros. (World) (Rev B)").revision, 2);
    }

    #[test]
    fn revision_v_prefix() {
        // GBA / homebrew style versioning.
        assert_eq!(parse("Pokemon - Emerald (USA) (v1.0)").revision, 1);
        assert_eq!(parse("Pokemon - Emerald (USA) (v1.1)").revision, 1);
        assert_eq!(parse("Pokemon - Emerald (USA) (v2)").revision, 2);
    }

    #[test]
    fn language_list_becomes_flag_not_region() {
        // `(En,Fr,De,Es,It)` is a language list, not a region. Must not
        // shadow the actual region paren, and must survive as a flag.
        let p = parse("Sonic the Hedgehog (USA, Europe) (En,Fr,De,Es,It)");
        assert_eq!(p.base, "Sonic the Hedgehog");
        assert_eq!(p.region.as_deref(), Some("USA"));
        assert_eq!(p.regions, vec!["USA".to_string(), "Europe".to_string()]);
        assert!(p.flags.iter().any(|f| f.contains("En,Fr")));
    }

    #[test]
    fn beta_flag_recognised_as_prerelease() {
        let p = parse("Mystery Game (Japan) (Beta)");
        assert_eq!(p.base, "Mystery Game");
        assert_eq!(p.region.as_deref(), Some("Japan"));
        assert!(p.flags.iter().any(|f| f == "Beta"));
        assert!(p.is_prerelease());
    }

    #[test]
    fn proto_demo_sample_all_prerelease() {
        assert!(parse("X (USA) (Proto)").is_prerelease());
        assert!(parse("X (USA) (Demo)").is_prerelease());
        assert!(parse("X (USA) (Sample)").is_prerelease());
    }

    #[test]
    fn release_dump_is_not_prerelease() {
        assert!(!parse("Castlevania (USA)").is_prerelease());
        assert!(!parse("Castlevania (USA) (Rev 1)").is_prerelease());
    }

    #[test]
    fn goodtools_bracket_flag_preserved() {
        let p = parse("Test Title (USA) [!]");
        assert_eq!(p.base, "Test Title");
        assert_eq!(p.region.as_deref(), Some("USA"));
        assert!(p.flags.iter().any(|f| f == "!"));
    }

    #[test]
    fn paren_inside_title_stays_part_of_base() {
        // Annotation extraction only walks from the END. A paren that
        // appears mid-title (and isn't followed by another annotation)
        // should remain in the base.
        let p = parse("Pac-Man (32k) Edition (USA)");
        // The trailing `(USA)` is consumed; "(32k) Edition" stays in
        // the base because nothing after it gets parsed.
        assert_eq!(p.base, "Pac-Man (32k) Edition");
        assert_eq!(p.region.as_deref(), Some("USA"));
    }

    #[test]
    fn nested_parens_consume_as_one_annotation() {
        // Rare but real: `(Foo (Bar))`. The outer paren is one
        // annotation; inner `(Bar)` is part of its inner text.
        let p = parse("Title (Foo (Bar))");
        // The outer annotation isn't a known region → becomes a flag.
        assert!(p.flags.iter().any(|f| f.starts_with("Foo")));
    }

    #[test]
    fn unmatched_paren_doesnt_panic() {
        // Defensive: malformed input shouldn't crash. We just don't
        // strip the unmatched chunk.
        let p = parse("Title (USA");
        // No clean close → no annotations consumed; base preserves the input.
        assert_eq!(p.base, "Title (USA");
    }

    #[test]
    fn three_letter_codes_map_to_canonical() {
        // Some upstream dats use J/U/E single-letter codes. The
        // canonical names normalise to USA/Japan/Europe.
        assert_eq!(parse("X (U)").region.as_deref(), Some("USA"));
        assert_eq!(parse("X (J)").region.as_deref(), Some("Japan"));
        assert_eq!(parse("X (E)").region.as_deref(), Some("Europe"));
    }

    #[test]
    fn group_key_is_base_lowercased_for_robustness() {
        // `Castlevania (USA)` and `castlevania (Japan)` should group
        // together for users who care about title-case quirks. The
        // base field stores the natural form; the aggregator
        // case-folds at grouping time.
        let a = parse("Castlevania (USA)");
        let b = parse("castlevania (Japan)");
        assert_eq!(a.base.to_lowercase(), b.base.to_lowercase());
    }
}
