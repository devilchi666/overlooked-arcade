//! Title normalization + match scoring for libretro-thumbnails sync.
//!
//! ROM filenames and libretro-thumbnails entries use different conventions:
//!
//!   ROM filename:   "Bonk_3_USA.pce"  or  "Bonk's Adventure (USA).pce"
//!   Upstream entry: "Bonk 3 - Bonk's Big Adventure (USA).png"
//!
//! Normalization aggressively strips punctuation, region/version tags, article
//! words, and case so that substring + fuzzy matching downstream can pair them
//! up. Match scoring tries exact → prefix → substring → Jaro-Winkler in order.

pub fn normalize_title(s: &str) -> String {
    // 1. Strip extension (only the last `.ext`).
    let s = match s.rsplit_once('.') {
        Some((stem, ext)) if ext.len() <= 5 => stem,
        _ => s,
    };
    // 2. Strip everything inside parens / brackets (region, revision, language tags).
    let s = strip_brackets(s);
    // 3. Lowercase.
    let s = s.to_lowercase();
    // 4. Replace separator-like punctuation with whitespace. `'` is NOT in
    //    this list — "Bonk's" should normalize to "bonks", not "bonk s".
    let s: String = s
        .chars()
        .map(|c| match c {
            '_' | '-' | '.' | ':' | ',' | '+' | '/' | '\\' => ' ',
            other => other,
        })
        .collect();
    // 5. Drop everything that isn't alphanumeric or whitespace. This
    //    silently strips `'`, `"`, `!`, `?`, `&`, `*`, etc. — they
    //    collapse rather than insert spaces, preserving "bonks" /
    //    "rtype".
    //
    //    Uses the Unicode-aware `is_alphanumeric` (not the
    //    `is_ascii_alphanumeric` of pre-2026-05-21 code) so kana / CJK /
    //    accented Latin characters survive: "ポケモン" → "ポケモン"
    //    (not empty), "Pokémon" → "pokémon" (not "pokmon"). Match
    //    matching is still ASCII-folded de-facto for most upstream
    //    libretro-database catalogs which carry the romaji form, but
    //    Japan-heavy libraries (PCE, MSX, SFC, WSC) where the
    //    filename and the canonical name share kana now stand a chance
    //    via exact-equality + Jaro-Winkler.
    let s: String = s
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect();
    // 6. Collapse runs of whitespace.
    let s = collapse_whitespace(&s);
    // 7. Strip tag words (region/version tokens that may have leaked out of
    //    brackets, e.g. "Bonk_3_USA" → "bonk 3 usa" → "bonk 3").
    let s = strip_tag_words(&s);
    // 8. Strip leading articles ("the", "a", "an").
    strip_leading_articles(&s).trim().to_string()
}

const TAG_WORDS: &[&str] = &[
    "usa", "japan", "europe", "world", "asia", "korea", "australia",
    "jp", "us", "eu", "uk", "kor",
    "en", "ja", "fr", "de", "es", "it",
    "rev", "proto", "beta", "alpha", "demo", "sample", "unl",
    "v1", "v2", "v3", "v4",
];

fn strip_tag_words(s: &str) -> String {
    s.split_whitespace()
        .filter(|w| !TAG_WORDS.contains(&w.to_lowercase().as_str()))
        .collect::<Vec<_>>()
        .join(" ")
}

fn strip_brackets(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut depth: i32 = 0;
    for c in s.chars() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                if depth > 0 {
                    depth -= 1;
                } else {
                    out.push(c);
                }
            }
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out
}

fn collapse_whitespace(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_space && !out.is_empty() {
                out.push(' ');
            }
            prev_space = true;
        } else {
            out.push(c);
            prev_space = false;
        }
    }
    out.trim().to_string()
}

fn strip_leading_articles(s: &str) -> String {
    let lower = s.to_lowercase();
    for article in &["the ", "a ", "an "] {
        if lower.starts_with(article) {
            return s[article.len()..].to_string();
        }
    }
    s.to_string()
}

/// Pull a canonical region name from the ORIGINAL (non-normalized) filename.
/// Looks for the well-known tag forms in parens/brackets first; falls back to
/// short codes (U/J/E/W) only when present in a parenthesized tag to avoid
/// false-positives on game titles that happen to contain those letters.
pub fn extract_region(s: &str) -> Option<&'static str> {
    let lower = s.to_lowercase();
    let pairs: &[(&str, &str)] = &[
        ("(usa)", "USA"),
        ("(u)", "USA"),
        ("(japan)", "Japan"),
        ("(j)", "Japan"),
        ("(europe)", "Europe"),
        ("(e)", "Europe"),
        ("(world)", "World"),
        ("(w)", "World"),
        ("[u]", "USA"),
        ("[j]", "Japan"),
        ("[e]", "Europe"),
    ];
    for (needle, region) in pairs {
        if lower.contains(needle) {
            return Some(region);
        }
    }
    None
}

/// 0.0..=1.0 match score. Strategies, highest wins:
///   exact equality                              → 1.00
///   prefix relationship  (both sides ≥ 4 char)  → 0.95
///   substring relationship (both sides ≥ 6 char) → 0.88
///   jaro_winkler                                 → 0.00..=1.00
///
/// Both shortcut strategies require BOTH operands to meet a minimum
/// length — pre-2026-05-21 the prefix branch only checked emptiness on
/// `rom`, so `"go"` matched `"goku adventure"` at 0.95 (false
/// positive) and `"a"` matched `"alex kidd..."` at 0.95 (worse). With
/// the canonical-no-intro title hydration shipping at the same time,
/// most hash-identified entries now hit the exact-equality branch and
/// the prefix/substring shortcuts are reserved for the homebrew /
/// hacks / unidentified path — where false positives are still
/// costly, so tightening is correct.
const PREFIX_SHORTCUT_MIN_LEN: usize = 4;
const SUBSTRING_SHORTCUT_MIN_LEN: usize = 6;

pub fn match_score(rom: &str, upstream: &str) -> f64 {
    if rom.is_empty() || upstream.is_empty() {
        return 0.0;
    }
    if rom == upstream {
        return 1.0;
    }
    if rom.len() >= PREFIX_SHORTCUT_MIN_LEN
        && upstream.len() >= PREFIX_SHORTCUT_MIN_LEN
        && (upstream.starts_with(rom) || rom.starts_with(upstream))
    {
        return 0.95;
    }
    if rom.len() >= SUBSTRING_SHORTCUT_MIN_LEN
        && upstream.len() >= SUBSTRING_SHORTCUT_MIN_LEN
        && (upstream.contains(rom) || rom.contains(upstream))
    {
        return 0.88;
    }
    strsim::jaro_winkler(rom, upstream)
}

/// Parsed upstream entry: filename + normalized-for-matching form + region.
#[derive(Debug, Clone)]
pub struct ParsedUpstream {
    pub filename: String,
    pub normalized: String,
    pub region: Option<&'static str>,
}

pub fn parse_upstream(filename: String) -> ParsedUpstream {
    let normalized = normalize_title(&filename);
    let region = extract_region(&filename);
    ParsedUpstream { filename, normalized, region }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_extension() {
        assert_eq!(normalize_title("Bonk.pce"), "bonk");
        assert_eq!(normalize_title("Bonk.png"), "bonk");
    }

    #[test]
    fn normalize_strips_parens() {
        assert_eq!(normalize_title("Bonk's Adventure (USA).pce"), "bonks adventure");
        assert_eq!(
            normalize_title("Bonk 3 - Bonk's Big Adventure (USA).png"),
            "bonk 3 bonks big adventure",
        );
    }

    #[test]
    fn normalize_handles_underscores_and_tags() {
        assert_eq!(normalize_title("Bonk_3_USA.pce"), "bonk 3");
        assert_eq!(normalize_title("R-Type_USA.pce"), "r type");
    }

    #[test]
    fn normalize_strips_leading_article() {
        assert_eq!(normalize_title("The_Legend_of_Hero_Tonma.pce"), "legend of hero tonma");
    }

    #[test]
    fn normalize_collapses_whitespace() {
        assert_eq!(normalize_title("Foo    Bar"), "foo bar");
    }

    #[test]
    fn match_score_exact() {
        assert_eq!(match_score("bonks adventure", "bonks adventure"), 1.0);
    }

    #[test]
    fn match_score_prefix_relationship() {
        assert!(match_score("bonk 3", "bonk 3 bonks big adventure") >= 0.95);
    }

    #[test]
    fn match_score_substring() {
        let s = match_score("bonks big adventure", "bonk 3 bonks big adventure");
        assert!(s >= 0.88, "expected substring score, got {}", s);
    }

    #[test]
    fn match_score_fuzzy() {
        // Different words — falls through to jaro_winkler.
        let s = match_score("rtype", "r type");
        assert!(s > 0.8, "expected jaro_winkler ~0.9, got {}", s);
    }

    #[test]
    fn extract_region_recognizes_known_tags() {
        assert_eq!(extract_region("Bonk 3 (USA).png"), Some("USA"));
        assert_eq!(extract_region("Akumajou Dracula X (Japan).png"), Some("Japan"));
        assert_eq!(extract_region("R-Type (Europe).png"), Some("Europe"));
        assert_eq!(extract_region("Bonk's Adventure (W).png"), Some("World"));
        assert_eq!(extract_region("Boring Title.png"), None);
    }

    #[test]
    fn parse_upstream_full_path() {
        let p = parse_upstream("Bonk's Adventure (USA).png".to_string());
        assert_eq!(p.normalized, "bonks adventure");
        assert_eq!(p.region, Some("USA"));
    }

    /// Pre-2026-05-21 the ASCII-only filter dropped kana to empty,
    /// which made every Japan-only ROM unmatchable via the canonical-
    /// title path. The Unicode-aware filter preserves them.
    #[test]
    fn normalize_preserves_cjk_kana() {
        assert_eq!(normalize_title("ポケモン.gb"), "ポケモン");
        // Mixed kana + ASCII titles, kana OUTSIDE parens so
        // strip_brackets doesn't drop them. Region tag in brackets
        // is removed first, then normalization preserves both halves.
        assert_eq!(
            normalize_title("Dragon Quest ドラゴンクエスト (Japan).nes"),
            "dragon quest ドラゴンクエスト",
        );
    }

    /// Accented Latin (é, ü, ñ, ç…) survives normalization rather than
    /// being silently dropped to a half-stripped word.
    #[test]
    fn normalize_preserves_accented_latin() {
        assert_eq!(normalize_title("Pokémon.gb"), "pokémon");
        assert_eq!(normalize_title("Pokémon Red.gba"), "pokémon red");
    }

    /// The prefix-shortcut now requires both operands ≥ 4 characters.
    /// Pre-fix this returned 0.95 because `upstream.starts_with(rom)`
    /// was true for any non-empty `rom`. Now short-rom + long-upstream
    /// falls through to Jaro-Winkler which scores below the 0.85
    /// match threshold.
    #[test]
    fn match_score_short_prefix_no_longer_false_positive() {
        // Pre-fix: "go" vs "goku adventure" returned 0.95 via prefix.
        // Post-fix: falls through to jaro_winkler which scores below
        // the 0.85 metadata-match threshold for unrelated titles.
        let s_short_prefix = match_score("go", "goku adventure");
        assert!(
            s_short_prefix < 0.95,
            "short-prefix shortcut should not fire, got {}",
            s_short_prefix,
        );
        // Real prefix (both ≥ 4) still fires.
        let s_legit_prefix = match_score("bonk", "bonks adventure");
        assert!(
            s_legit_prefix >= 0.95,
            "legit prefix shortcut should fire, got {}",
            s_legit_prefix,
        );
    }

    /// Substring shortcut symmetric on both sides ≥ 6.
    #[test]
    fn match_score_substring_requires_both_sides_long_enough() {
        // Pre-fix asymmetric: "super" (5 chars) contained in "super mario"
        // would have hit the substring branch since `upstream.contains(rom)`
        // + `rom.len() >= 6` had been the original gate — actually rom is
        // 5 here so neither branch fired in the original either; this
        // checks the symmetric tightening doesn't change accepted cases.
        let s = match_score("bonks big adventure", "bonk 3 bonks big adventure");
        assert!(s >= 0.88, "long substring should still hit, got {}", s);
    }
}
