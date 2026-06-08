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
//!
//! ## Typed flag fields (Virtual Library Phase A2)
//!
//! Beyond region/revision, the parser also classifies common dump-
//! quality and provenance tags into typed fields on `ParsedTitle` —
//! `dump_status`, `is_hack`, `is_translation`, `is_pirate`, `is_bios`,
//! `is_homebrew`, `translation_languages`. Decoder covers the three
//! big naming conventions:
//!
//! - **GoodTools** brackets:
//!   - `[!]` Verified good dump
//!   - `[b]` `[b1]` `[b2]` Bad dump
//!   - `[o]` `[o1]` Over-dump (file larger than the cart's true capacity)
//!   - `[f]` `[f1]` "Fixed" — patched to run on specific hardware
//!   - `[h]` `[h1]` `[hI]` `[hIR]` Hack (intro hacks, intro-removed hacks, etc.)
//!   - `[p]` `[p1]` Pirate
//!   - `[T+Eng]` Current translation; `[T-Eng]` superseded translation;
//!     `[T+Eng,Fra]` multi-language translation
//! - **No-Intro / Redump** parens: `(Hack)`, `(Pirate)`, `(BIOS)`,
//!   `(Homebrew)`, `(Aftermarket)`, `(Unl)` / `(Unlicensed)`
//! - **TOSEC** parens: `(Cracked)`, `(Pirate)`, `(Hack)`, `(BIOS)`,
//!   `(Homebrew)`, `(Aftermarket)`
//!
//! All recognised tokens still land in `flags` for transparency — the
//! typed fields are additive, never substitutive. A `[T+Eng]` annotation
//! sets `is_translation = true` AND `translation_languages = ["Eng"]`
//! AND adds `"T+Eng"` to `flags`.

/// Dump-quality status from GoodTools-style bracket flags. Default
/// `Unknown` covers everything that doesn't carry an explicit dump-
/// quality marker (No-Intro / Redump don't tag verified dumps because
/// every entry in those DATs IS a verified dump; the `Unknown` value
/// for those isn't a problem, it's the absence of a problem).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DumpStatus {
    /// `[!]` — GoodTools verified good dump. Gold standard.
    Verified,
    /// `[b]` `[b1]` `[b2]` — Bad dump (known checksum mismatch / corruption).
    BadDump,
    /// `[o]` `[o1]` — Over-dump (file padded beyond the cart's true capacity).
    OverDump,
    /// `[f]` `[f1]` — "Fixed" — patched to run on specific hardware (e.g. flash
    /// carts, emulator quirks). Useful but not the canonical dump.
    Fixed,
    /// No dump-status flag present. Default for No-Intro / Redump
    /// names (every entry in those DATs is verified by the maintainers
    /// upstream, so an explicit verified marker isn't part of the
    /// naming convention).
    Unknown,
}

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
    /// `[!]`, `[h]`, `[t]`, language lists, etc. Recognised tags (the
    /// ones that set `dump_status` / `is_hack` / `is_translation` /
    /// etc.) still appear here for transparency — the typed fields are
    /// additive, not substitutive.
    pub flags: Vec<String>,
    /// Dump-quality classification from GoodTools brackets. See
    /// `DumpStatus`. Defaults to `Unknown` for No-Intro / Redump.
    pub dump_status: DumpStatus,
    /// True if any recognised hack flag is present: GoodTools `[h]` /
    /// `[h1]` / `[hI]` / `[hIR]` etc.; No-Intro `(Hack)`; TOSEC
    /// `(Hack)`. Hacks include intro-screen modifications, content
    /// edits, ROM hacks of any kind.
    pub is_hack: bool,
    /// True for fan-translation dumps. GoodTools `[T+Xxx]` (current
    /// translation) and `[T-Xxx]` (superseded translation) both set
    /// this; `translation_languages` carries the parsed language list.
    pub is_translation: bool,
    /// True if any recognised pirate / cracked flag is present.
    /// GoodTools `[p]` / `[p1]`; No-Intro `(Pirate)`; TOSEC `(Pirate)`,
    /// `(Cracked)`. Useful as a filter for operators who want a pure-
    /// preservation library.
    pub is_pirate: bool,
    /// True for BIOS / boot-ROM files (not games). No-Intro / TOSEC
    /// `(BIOS)`. The Preservation Vault filter excludes BIOS by default
    /// in the Casual view; opt-in to surface them.
    pub is_bios: bool,
    /// True for homebrew / aftermarket releases. No-Intro / TOSEC
    /// `(Homebrew)`, `(Aftermarket)`. `(Unl)` / `(Unlicensed)` is
    /// intentionally NOT folded in here — unlicensed commercial games
    /// (Wisdom Tree, Color Dreams, Camerica) sit in a different
    /// preservation category from amateur homebrew. The `Unl` token
    /// stays visible in `flags` for that purpose.
    pub is_homebrew: bool,
    /// Languages parsed from a translation tag: `[T+Eng]` →
    /// `["Eng"]`; `[T+Eng,Fra]` → `["Eng", "Fra"]`. Empty when
    /// `is_translation` is false. Codes are preserved as-written
    /// (GoodTools uses 3-letter ISO-like codes: Eng, Fra, Deu, Spa,
    /// Ita, Jpn, etc.) — no canonicalisation in v1 since the operator
    /// surface is "filter by language" not "look up language metadata".
    pub translation_languages: Vec<String>,
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
/// None, regions: [], revision: 0, flags: [], dump_status: Unknown,
/// all bool fields false, translation_languages: [] }` — i.e. user-
/// renamed games still form their own one-entry group cleanly.
pub fn parse_canonical_title(s: &str) -> ParsedTitle {
    let trimmed = s.trim();
    // First pass — split off annotations. Annotations are spans
    // delimited by `(...)` or `[...]` at the END of the title.
    let (base, annotations) = split_base_and_annotations(trimmed);

    let mut region: Option<String> = None;
    let mut regions: Vec<String> = Vec::new();
    let mut revision: u32 = 0;
    let mut flags: Vec<String> = Vec::new();
    let mut dump_status = DumpStatus::Unknown;
    let mut is_hack = false;
    let mut is_translation = false;
    let mut is_pirate = false;
    let mut is_bios = false;
    let mut is_homebrew = false;
    let mut translation_languages: Vec<String> = Vec::new();

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
            // Phase A2 — classify into typed flag fields. Each annotation
            // can contribute to MULTIPLE fields (rare but possible —
            // e.g. `[T+Eng]` is both translation and a language list).
            // Then ALWAYS push to flags for transparency.
            if dump_status == DumpStatus::Unknown {
                if let Some(status) = classify_dump_status(&ann) {
                    dump_status = status;
                }
            }
            if classify_translation(&ann, &mut translation_languages) {
                is_translation = true;
            }
            if is_hack_flag(&ann) {
                is_hack = true;
            }
            if is_pirate_flag(&ann) {
                is_pirate = true;
            }
            if is_bios_flag(&ann) {
                is_bios = true;
            }
            if is_homebrew_flag(&ann) {
                is_homebrew = true;
            }
            flags.push(ann);
        }
    }

    ParsedTitle {
        base: base.trim().to_string(),
        region,
        regions,
        revision,
        flags,
        dump_status,
        is_hack,
        is_translation,
        is_pirate,
        is_bios,
        is_homebrew,
        translation_languages,
    }
}

/// Classify a GoodTools-style dump-quality bracket flag. Recognises
/// the four canonical statuses + their numeric variants (`[b]` and
/// `[b1]` both mean bad-dump). Returns `None` for everything else,
/// including the `Verified` default — callers initialise to `Unknown`
/// and only override when this returns `Some`.
fn classify_dump_status(ann: &str) -> Option<DumpStatus> {
    // Verified is the bare `!` — no suffix variants in GoodTools.
    if ann == "!" {
        return Some(DumpStatus::Verified);
    }
    // Single-char status with optional numeric suffix: b / b1 / b2 / o / o1 / f / f1.
    let mut chars = ann.chars();
    let first = chars.next()?;
    let rest = chars.as_str();
    if !rest.is_empty() && !rest.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    match first {
        'b' => Some(DumpStatus::BadDump),
        'o' => Some(DumpStatus::OverDump),
        'f' => Some(DumpStatus::Fixed),
        _ => None,
    }
}

/// Classify GoodTools translation tags. `[T+Eng]` = current
/// translation; `[T-Eng]` = old/superseded translation; both set
/// `is_translation = true`. Multi-language: `[T+Eng,Fra]` →
/// languages = `["Eng", "Fra"]`. Tags may also carry version/group
/// suffixes (`[T+Eng1.0_Aeon]`) — those get stripped from the
/// language part.
///
/// Returns true when the annotation looks like a translation tag.
/// Appends parsed languages to `out_langs` in input order.
fn classify_translation(ann: &str, out_langs: &mut Vec<String>) -> bool {
    let body = match ann.strip_prefix("T+").or_else(|| ann.strip_prefix("T-")) {
        Some(b) => b,
        None => return false,
    };
    // The body is `Lang[,Lang2,...][Version_Group]`. Stop at the first
    // version/group separator — GoodTools puts version after the
    // language list with no separator other than the start of digits
    // (e.g. `Eng1.0_Aeon` is "Eng" + "1.0_Aeon"). Trim each language
    // token down to its leading alpha run.
    for token in body.split(',') {
        let mut lang = String::new();
        for c in token.chars() {
            if c.is_ascii_alphabetic() {
                lang.push(c);
            } else {
                break;
            }
        }
        if !lang.is_empty() {
            out_langs.push(lang);
        }
    }
    true
}

/// True when the annotation flags this dump as a hack. Recognises:
///   - GoodTools `[h]` / `[h1]` / `[h2]` (numeric suffix)
///   - GoodTools `[hI]` / `[hIR]` / `[hM]` (intro hack, intro-removed,
///     menu hack — letter suffixes after the leading `h`)
///   - No-Intro / TOSEC `(Hack)` (paren form, exact match)
fn is_hack_flag(ann: &str) -> bool {
    if ann.eq_ignore_ascii_case("Hack") {
        return true;
    }
    let mut chars = ann.chars();
    if chars.next() != Some('h') {
        return false;
    }
    // Suffix may be empty, all-digit, or alpha (intro/menu hack codes).
    // Reject anything that mixes other punctuation to avoid eating
    // unrelated tokens that happen to start with 'h'.
    chars.all(|c| c.is_ascii_alphanumeric())
}

/// True for GoodTools `[p]` / `[p1]` pirate marker, or
/// No-Intro / TOSEC `(Pirate)` / TOSEC `(Cracked)`.
fn is_pirate_flag(ann: &str) -> bool {
    if ann.eq_ignore_ascii_case("Pirate") || ann.eq_ignore_ascii_case("Cracked") {
        return true;
    }
    let mut chars = ann.chars();
    if chars.next() != Some('p') {
        return false;
    }
    let rest = chars.as_str();
    rest.is_empty() || rest.chars().all(|c| c.is_ascii_digit())
}

/// True for No-Intro / TOSEC `(BIOS)` or `[BIOS]`. The token is
/// case-insensitive for robustness; canonical sources always upper-
/// case it.
fn is_bios_flag(ann: &str) -> bool {
    ann.eq_ignore_ascii_case("BIOS")
}

/// True for No-Intro / TOSEC `(Homebrew)` or `(Aftermarket)`. Note:
/// `(Unl)` / `(Unlicensed)` is intentionally NOT folded in — see
/// `ParsedTitle::is_homebrew` doc for why.
fn is_homebrew_flag(ann: &str) -> bool {
    ann.eq_ignore_ascii_case("Homebrew") || ann.eq_ignore_ascii_case("Aftermarket")
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

    // --- Phase A2: typed flag fields ------------------------------------
    //
    // The classifier additions in Phase A2 give the Preservation Vault
    // its filterable fields. Each test pins one branch of the decode
    // table; together they cover GoodTools + No-Intro + TOSEC vocab.

    // Defaults — every typed field is false / Unknown / empty when no
    // recognised flags appear.

    #[test]
    fn defaults_when_no_typed_flags() {
        let p = parse("Castlevania (USA)");
        assert_eq!(p.dump_status, DumpStatus::Unknown);
        assert!(!p.is_hack);
        assert!(!p.is_translation);
        assert!(!p.is_pirate);
        assert!(!p.is_bios);
        assert!(!p.is_homebrew);
        assert!(p.translation_languages.is_empty());
    }

    #[test]
    fn defaults_for_user_renamed_title() {
        // No annotations at all → all typed fields default. Pins the
        // user-rename path so future classifier work can't accidentally
        // start tagging arbitrary user titles.
        let p = parse("My favourite game");
        assert_eq!(p.dump_status, DumpStatus::Unknown);
        assert!(!p.is_hack);
        assert!(!p.is_translation);
        assert!(p.translation_languages.is_empty());
    }

    // Dump status — `[!]` Verified.

    #[test]
    fn goodtools_bang_means_verified() {
        let p = parse("Test Title (USA) [!]");
        assert_eq!(p.dump_status, DumpStatus::Verified);
        // Still in flags for transparency.
        assert!(p.flags.iter().any(|f| f == "!"));
    }

    // Dump status — `[b]` Bad dump, with + without numeric suffix.

    #[test]
    fn goodtools_b_means_bad_dump() {
        let p = parse("Test Title (USA) [b]");
        assert_eq!(p.dump_status, DumpStatus::BadDump);
    }

    #[test]
    fn goodtools_b1_means_bad_dump_too() {
        let p = parse("Test Title (USA) [b1]");
        assert_eq!(p.dump_status, DumpStatus::BadDump);
    }

    #[test]
    fn goodtools_b2_means_bad_dump_too() {
        let p = parse("Test Title (USA) [b2]");
        assert_eq!(p.dump_status, DumpStatus::BadDump);
    }

    // Dump status — `[o]` Over-dump.

    #[test]
    fn goodtools_o_means_over_dump() {
        let p = parse("Test Title (USA) [o]");
        assert_eq!(p.dump_status, DumpStatus::OverDump);
    }

    #[test]
    fn goodtools_o1_means_over_dump_too() {
        let p = parse("Test Title (USA) [o1]");
        assert_eq!(p.dump_status, DumpStatus::OverDump);
    }

    // Dump status — `[f]` Fixed.

    #[test]
    fn goodtools_f_means_fixed() {
        let p = parse("Test Title (USA) [f]");
        assert_eq!(p.dump_status, DumpStatus::Fixed);
    }

    #[test]
    fn goodtools_f1_means_fixed_too() {
        let p = parse("Test Title (USA) [f1]");
        assert_eq!(p.dump_status, DumpStatus::Fixed);
    }

    // Dump status — only first match wins. A `[b]` followed by `[!]`
    // (perverse input) keeps BadDump; verified-on-top would lie about
    // the file integrity.

    #[test]
    fn first_dump_status_wins() {
        let p = parse("Test Title (USA) [b] [!]");
        assert_eq!(p.dump_status, DumpStatus::BadDump);
    }

    // Hack — GoodTools bracket forms.

    #[test]
    fn goodtools_h_means_hack() {
        let p = parse("Test Title (USA) [h]");
        assert!(p.is_hack);
    }

    #[test]
    fn goodtools_h1_means_hack() {
        let p = parse("Test Title (USA) [h1]");
        assert!(p.is_hack);
    }

    #[test]
    fn goodtools_intro_hack_codes() {
        // [hI] = intro hack, [hIR] = intro removed, [hM] = menu hack.
        // All are forms of hack; the decoder accepts any [h<alpha>]
        // suffix as long as the suffix is purely alphanumeric.
        assert!(parse("X (USA) [hI]").is_hack);
        assert!(parse("X (USA) [hIR]").is_hack);
        assert!(parse("X (USA) [hM]").is_hack);
    }

    #[test]
    fn nointro_hack_paren_form() {
        let p = parse("Test Title (USA) (Hack)");
        assert!(p.is_hack);
    }

    // Hack — false-positive guards. Single 'h' isn't enough; the
    // classifier must reject titles that legitimately START with h.

    #[test]
    fn arbitrary_h_token_doesnt_match_hack() {
        // `[h-something]` with non-alphanumeric content shouldn't
        // count. Defends against future tag vocab adding `h-foo`
        // patterns that aren't hacks.
        let p = parse("Test Title (USA) [h-not-a-hack]");
        assert!(!p.is_hack);
    }

    // Translation — `[T+Eng]` current; `[T-Eng]` superseded.

    #[test]
    fn translation_t_plus_eng() {
        let p = parse("Castlevania (Japan) [T+Eng]");
        assert!(p.is_translation);
        assert_eq!(p.translation_languages, vec!["Eng".to_string()]);
    }

    #[test]
    fn translation_t_minus_eng_also_sets_flag() {
        // `T-Xxx` is a superseded translation. Still a translation;
        // user filters get to decide whether to surface old vs new.
        let p = parse("Castlevania (Japan) [T-Eng]");
        assert!(p.is_translation);
        assert_eq!(p.translation_languages, vec!["Eng".to_string()]);
    }

    #[test]
    fn translation_multi_language() {
        // `[T+Eng,Fra]` is a single multi-language translation tag.
        let p = parse("Castlevania (Japan) [T+Eng,Fra]");
        assert!(p.is_translation);
        assert_eq!(
            p.translation_languages,
            vec!["Eng".to_string(), "Fra".to_string()]
        );
    }

    #[test]
    fn translation_strips_version_and_group_suffix() {
        // GoodTools translation tags carry version + author suffix:
        // `[T+Eng1.0_Aeon]`. We keep only the leading alpha run as
        // the language code.
        let p = parse("Castlevania (Japan) [T+Eng1.0_Aeon]");
        assert!(p.is_translation);
        assert_eq!(p.translation_languages, vec!["Eng".to_string()]);
    }

    // Pirate — GoodTools + No-Intro/TOSEC forms.

    #[test]
    fn goodtools_p_means_pirate() {
        let p = parse("Test Title (USA) [p]");
        assert!(p.is_pirate);
    }

    #[test]
    fn goodtools_p1_means_pirate() {
        let p = parse("Test Title (USA) [p1]");
        assert!(p.is_pirate);
    }

    #[test]
    fn nointro_pirate_paren_form() {
        let p = parse("Test Title (USA) (Pirate)");
        assert!(p.is_pirate);
    }

    #[test]
    fn tosec_cracked_means_pirate() {
        let p = parse("Test Title (USA) (Cracked)");
        assert!(p.is_pirate);
    }

    // BIOS — paren form (both No-Intro and TOSEC use this).

    #[test]
    fn nointro_bios_paren_form() {
        let p = parse("PSX BIOS (USA) (BIOS)");
        assert!(p.is_bios);
    }

    #[test]
    fn bios_case_insensitive() {
        let p = parse("Some Boot ROM [bios]");
        assert!(p.is_bios);
    }

    // Homebrew — Homebrew + Aftermarket; intentionally NOT Unl.

    #[test]
    fn nointro_homebrew_paren_form() {
        let p = parse("Indie Game (USA) (Homebrew)");
        assert!(p.is_homebrew);
    }

    #[test]
    fn tosec_aftermarket_means_homebrew() {
        // Post-original-era unofficial production (e.g. Sunday-cart
        // NES releases). Catalogued as preservation alongside
        // homebrew in the typed field.
        let p = parse("New NES Game (Unknown) (Aftermarket)");
        assert!(p.is_homebrew);
    }

    #[test]
    fn unl_is_not_homebrew() {
        // Unlicensed commercial games (Wisdom Tree, Color Dreams,
        // Camerica) sit in a different preservation category from
        // amateur homebrew. `Unl` becomes a plain flag, never sets
        // is_homebrew. This pins that distinction so a future
        // classifier widening can't quietly collapse the two.
        let p = parse("Bible Adventures (USA) (Unl)");
        assert!(!p.is_homebrew);
        // Still preserved in flags for the operator's visibility.
        assert!(p.flags.iter().any(|f| f == "Unl"));
    }

    // Multi-annotation interaction — a single dump can carry multiple
    // typed flags. Pins that no annotation shadows another.

    #[test]
    fn hack_plus_translation_both_set() {
        let p = parse("Castlevania (Japan) [T+Eng] [h]");
        assert!(p.is_translation);
        assert_eq!(p.translation_languages, vec!["Eng".to_string()]);
        assert!(p.is_hack);
    }

    #[test]
    fn bad_dump_plus_hack_both_set() {
        let p = parse("Castlevania (USA) [b1] [h2]");
        assert_eq!(p.dump_status, DumpStatus::BadDump);
        assert!(p.is_hack);
    }

    // Recognised typed annotations still appear in `flags` for
    // transparency. Pins the additive (not substitutive) policy.

    #[test]
    fn typed_annotations_stay_in_flags_vec() {
        let p = parse("Test Title (USA) [!] [h] (Pirate)");
        // Typed fields populated.
        assert_eq!(p.dump_status, DumpStatus::Verified);
        assert!(p.is_hack);
        assert!(p.is_pirate);
        // AND every one of them still in flags.
        assert!(p.flags.iter().any(|f| f == "!"));
        assert!(p.flags.iter().any(|f| f == "h"));
        assert!(p.flags.iter().any(|f| f == "Pirate"));
    }

    // Existing prerelease semantics (Beta / Proto / Demo / Sample) are
    // independent of Phase A2's typed flags. Pin that nothing crossed.

    #[test]
    fn beta_doesnt_set_phase_a2_flags() {
        let p = parse("Mystery Game (Japan) (Beta)");
        assert!(p.is_prerelease());
        assert_eq!(p.dump_status, DumpStatus::Unknown);
        assert!(!p.is_hack);
        assert!(!p.is_translation);
    }
}
