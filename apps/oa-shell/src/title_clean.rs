//! Filename-to-title cleanup used at scan time, before any canonical
//! identification has happened. The output is a best-effort label for a
//! row that hasn't yet been hashed (and may never be hashed — homebrew,
//! hacks, anything not in libretro-database). When a SHA-1 lookup
//! succeeds later in the scan pipeline, the canonical `game_name` from
//! [`crate::library_db::RomHashRow`] supersedes this output.
//!
//! Mirrors `titleFromFileName` from `frontend/src/library/ingest.ts`
//! (line 182) one-to-one. Tests below cover the same input shapes the
//! TS function accepts.
//!
//! Region tags / bracket flags (`(USA)`, `[!]`, `(Rev A)`, etc.) are
//! intentionally preserved — [`crate::title_parse::parse_canonical_title`]
//! is the structured parser that handles those, and it expects them to
//! still be present.

/// Strip the extension and replace `_` with spaces.
///
/// - Extension is everything after the last `.`. A leading-dot file
///   (e.g. `.thing`) keeps its name unchanged — strip only fires when
///   the dot is NOT at index 0.
/// - Multiple underscores collapse to one space (the JS `replace(/_+/g, " ")`).
/// - Leading + trailing whitespace trimmed.
pub fn title_from_file_name(name: &str) -> String {
    let base = match name.rfind('.') {
        Some(i) if i > 0 => &name[..i],
        _ => name,
    };
    let mut out = String::with_capacity(base.len());
    let mut prev_was_underscore = false;
    for c in base.chars() {
        if c == '_' {
            if !prev_was_underscore {
                out.push(' ');
            }
            prev_was_underscore = true;
        } else {
            out.push(c);
            prev_was_underscore = false;
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_extension() {
        assert_eq!(title_from_file_name("Castlevania.nes"), "Castlevania");
    }

    #[test]
    fn underscore_becomes_space() {
        assert_eq!(
            title_from_file_name("Super_Mario_World.smc"),
            "Super Mario World"
        );
    }

    #[test]
    fn multiple_underscores_collapse_to_single_space() {
        assert_eq!(title_from_file_name("Foo__Bar___Baz.bin"), "Foo Bar Baz");
    }

    #[test]
    fn no_extension_input() {
        assert_eq!(title_from_file_name("HiddenGame"), "HiddenGame");
    }

    #[test]
    fn embedded_dots_preserved_in_base() {
        // Only the LAST dot is the extension boundary.
        assert_eq!(title_from_file_name("Foo.Bar.smc"), "Foo.Bar");
    }

    #[test]
    fn leading_dotfile_left_alone() {
        // A hidden dotfile has no extension — the dot is at index 0.
        // Underscore translation still applies.
        assert_eq!(title_from_file_name(".thing"), ".thing");
        assert_eq!(title_from_file_name(".my_file"), ".my file");
    }

    #[test]
    fn preserves_region_tags_and_bracket_flags() {
        // title_parse::parse_canonical_title handles these downstream.
        assert_eq!(
            title_from_file_name("Super Mario World (USA) [!].smc"),
            "Super Mario World (USA) [!]"
        );
    }

    #[test]
    fn trims_leading_and_trailing_whitespace() {
        assert_eq!(title_from_file_name("  Foo  .nes"), "Foo");
    }

    #[test]
    fn empty_input_yields_empty() {
        assert_eq!(title_from_file_name(""), "");
    }
}
