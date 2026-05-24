//! Per-system cheat-code format declarations.
//!
//! The shipped cheat infrastructure (`library_db::Cheat` + `apply_cheats`
//! + `cheat_runtime` dispatch in main.rs) already routes both shapes of
//! cheats end-to-end:
//!
//! - **`kind = "memory_poke"`** — `(region, offset, width, value)` written
//!   to core memory every frame via `Core::memory_region_mut` (apply_cheats).
//! - **Any other `kind` value** — `code` string passed through
//!   `Core::cheat_set(idx, enabled, code)` → libretro's `retro_cheat_set`.
//!   Each core decodes the code per its own format conventions.
//!
//! This module names the per-system code formats so the frontend
//! CheatsDialog can:
//!
//! 1. Show a per-system picker ("Game Genie" / "Action Replay" /
//!    "CodeBreaker" etc.) instead of a single opaque "Code" option.
//! 2. Validate operator input against the format's shape at save time
//!    rather than silently passing malformed codes to the core.
//! 3. Render format-name labels in the cheat list so operators can tell
//!    at a glance what each cheat is.
//!
//! At dispatch time the named formats are equivalent to a generic
//! libretro code — the core decodes the actual format. The kind value
//! is preserved verbatim through the database and through to
//! `retro_cheat_set` (some cores key on the kind for ABI variant
//! selection; most ignore everything except the raw code string).
//!
//! ## Validation regex format
//!
//! `validation_regex` is a JavaScript-flavored regex pattern (consumed
//! by the frontend via the `RegExp` constructor). Backend never compiles
//! it — Rust only ships the string. Case-insensitive flag is implicit;
//! the frontend appends `i` when constructing the RegExp.
//!
//! ## Adding a new system's formats
//!
//! 1. Verify the libretro core for that system documents which code
//!    formats it accepts (check the core's README + retro_cheat_set
//!    source).
//! 2. Add an arm to `cheat_formats_for` returning a Vec<CheatFormat>
//!    in priority order (most-common-format first).
//! 3. Add to the test fixture so the round-trip stays sound.
//!
//! Systems not declared here fall through to the generic
//! `memory_poke` + `libretro_code` pair — operators can still enter any
//! raw code; they just don't get format-specific labels or validation.

use serde::Serialize;

/// One supported cheat format on a given system. The frontend's
/// CheatsDialog renders an entry per `CheatFormat` in the Type picker.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheatFormat {
    /// Stable id stored in `Cheat.kind`. Conventions:
    /// - `"memory_poke"` — always present, raw address+value (special-cased)
    /// - `"libretro_code"` — always present last, generic catch-all
    /// - `"<format>_<system>"` — named formats per system (e.g.
    ///   `"game_genie_nes"`, `"action_replay_gba"`, `"gameshark_gb"`)
    pub id: &'static str,
    /// Human label shown in the picker (e.g. "Game Genie", "Action Replay v3").
    pub label: &'static str,
    /// Short hint shown next to the code input. Describes the expected
    /// shape ("e.g. SXIOPO or AEAAGZ", "8 hex digits", etc.).
    pub hint: &'static str,
    /// JavaScript-flavored regex pattern for input validation. The
    /// frontend constructs `new RegExp(pattern, "i")` and rejects
    /// invalid codes at save time. Empty string skips validation
    /// (used for `memory_poke` + `libretro_code` generic formats).
    pub validation_regex: &'static str,
    /// True for the kind that the apply_cheats dispatch handles
    /// (writes value to memory every frame). False for code formats
    /// passed through to libretro's retro_cheat_set.
    pub is_memory_poke: bool,
}

const MEMORY_POKE: CheatFormat = CheatFormat {
    id: "memory_poke",
    label: "Memory poke (raw address + value)",
    hint: "Set region / offset / width / value below",
    validation_regex: "",
    is_memory_poke: true,
};

const LIBRETRO_RAW: CheatFormat = CheatFormat {
    id: "libretro_code",
    label: "Generic libretro code",
    hint: "Paste whatever format the core accepts (verbatim)",
    validation_regex: "",
    is_memory_poke: false,
};

/// Resolve a system_id to its supported cheat formats. Returns at
/// minimum `memory_poke` + `libretro_code`; systems with documented
/// named formats include those between the two.
///
/// Order is operator-facing — the first entry is the default
/// selection in a fresh-add UI, the last is the catch-all.
pub fn cheat_formats_for(system_id: &str) -> Vec<CheatFormat> {
    let mut out = vec![MEMORY_POKE];
    out.extend(named_formats_for(system_id));
    out.push(LIBRETRO_RAW);
    out
}

/// Named per-system code formats. Order matters — the picker renders
/// in declaration order with the most-common format first per system.
fn named_formats_for(system_id: &str) -> Vec<CheatFormat> {
    match system_id {
        // NES — FCEUmm + Mesen both accept Game Genie codes (6 or 8
        // chars from the GG alphabet APZLGITYEOXUKSVN) and the
        // historical Pro Action Replay format (8 hex). FCEUmm also
        // accepts the raw `XXXX:YY` Pro Action Replay v2 form.
        "nes" => vec![
            CheatFormat {
                id: "game_genie_nes",
                label: "Game Genie",
                hint: "6 or 8 chars from APZLGITYEOXUKSVN (e.g. SXIOPO, AEAAGZ)",
                validation_regex: r"^[APZLGITYEOXUKSVN]{6}([APZLGITYEOXUKSVN]{2})?$",
                is_memory_poke: false,
            },
            CheatFormat {
                id: "pro_action_replay_nes",
                label: "Pro Action Replay",
                hint: "8 hex digits OR AAAA:VV (4-hex-address colon 2-hex-value)",
                validation_regex: r"^([0-9A-F]{8}|[0-9A-F]{4}:[0-9A-F]{2})$",
                is_memory_poke: false,
            },
        ],
        // SNES — Snes9x + bsnes accept Game Genie (XXXX-YYYY 9-char
        // dashed format) and Pro Action Replay (8 hex address+value).
        "snes" => vec![
            CheatFormat {
                id: "game_genie_snes",
                label: "Game Genie",
                hint: "XXXX-YYYY (8 hex with dash, e.g. DD62-D7D9)",
                validation_regex: r"^[0-9A-F]{4}-[0-9A-F]{4}$",
                is_memory_poke: false,
            },
            CheatFormat {
                id: "pro_action_replay_snes",
                label: "Pro Action Replay",
                hint: "8 hex digits (e.g. 7E001050)",
                validation_regex: r"^[0-9A-F]{8}$",
                is_memory_poke: false,
            },
        ],
        // Genesis / Mega Drive — Game Genie (XXXX-YYYY, alphanumeric)
        // + Pro Action Replay / Master Code (6-hex-address colon
        // 2-hex-value, or compact 8-hex). ClownMDEmu, GPGX, PicoDrive
        // all accept both via retro_cheat_set.
        "genesis" => vec![
            CheatFormat {
                id: "game_genie_genesis",
                label: "Game Genie",
                hint: "XXXX-YYYY (alphanumeric with dash, e.g. RHGT-A6WT)",
                validation_regex: r"^[A-Z0-9]{4}-[A-Z0-9]{4}$",
                is_memory_poke: false,
            },
            CheatFormat {
                id: "pro_action_replay_genesis",
                label: "Pro Action Replay / Master Code",
                hint: "AAAAAA:VV (6-hex-address colon 2-hex-value) OR 8 hex",
                validation_regex: r"^([0-9A-F]{6}:[0-9A-F]{2}|[0-9A-F]{8})$",
                is_memory_poke: false,
            },
        ],
        // Sega CD + 32X share the Genesis controller AND share
        // Genesis Plus GX's cheat decoder — same formats apply.
        "segacd" | "sega32x" => vec![
            CheatFormat {
                id: "game_genie_genesis",
                label: "Game Genie",
                hint: "XXXX-YYYY (alphanumeric with dash)",
                validation_regex: r"^[A-Z0-9]{4}-[A-Z0-9]{4}$",
                is_memory_poke: false,
            },
            CheatFormat {
                id: "pro_action_replay_genesis",
                label: "Pro Action Replay / Master Code",
                hint: "AAAAAA:VV OR 8 hex",
                validation_regex: r"^([0-9A-F]{6}:[0-9A-F]{2}|[0-9A-F]{8})$",
                is_memory_poke: false,
            },
        ],
        // SMS / Game Gear — Genesis Plus GX accepts Game Genie format
        // (XXX-XXX-XXX, 11 chars with two dashes — different shape
        // from MD's 9-char dashed format).
        "sms" | "gamegear" => vec![
            CheatFormat {
                id: "game_genie_sms",
                label: "Game Genie",
                hint: "XXX-XXX-XXX (9 hex with two dashes, e.g. 00C-200-F7A)",
                validation_regex: r"^[0-9A-F]{3}-[0-9A-F]{3}-[0-9A-F]{3}$",
                is_memory_poke: false,
            },
        ],
        // Game Boy + Game Boy Color — Gambatte accepts both Game Genie
        // (ABC-DEF-GHI 9-char dashed format) and GameShark (8 hex).
        "gb" | "gbc" => vec![
            CheatFormat {
                id: "game_genie_gb",
                label: "Game Genie",
                hint: "ABC-DEF-GHI (9 hex with two dashes)",
                validation_regex: r"^[0-9A-F]{3}-[0-9A-F]{3}-[0-9A-F]{3}$",
                is_memory_poke: false,
            },
            CheatFormat {
                id: "gameshark_gb",
                label: "GameShark",
                hint: "8 hex digits (TTYYAAAA format)",
                validation_regex: r"^[0-9A-F]{8}$",
                is_memory_poke: false,
            },
        ],
        // GBA — mGBA accepts Game Genie (xxxx-xxxx-xxxx 12-char triple-
        // dashed), Action Replay v3 (8-hex 8-hex space-separated pair),
        // and CodeBreaker (8-hex 4-hex pair).
        "gba" => vec![
            CheatFormat {
                id: "game_genie_gba",
                label: "Game Genie",
                hint: "XXXX-XXXX-XXXX (12 hex with two dashes)",
                validation_regex: r"^[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}$",
                is_memory_poke: false,
            },
            CheatFormat {
                id: "action_replay_gba",
                label: "Action Replay v3",
                hint: "AAAAAAAA YYYYYYYY (two 8-hex blocks, space-separated)",
                validation_regex: r"^[0-9A-F]{8}\s+[0-9A-F]{8}$",
                is_memory_poke: false,
            },
            CheatFormat {
                id: "codebreaker_gba",
                label: "CodeBreaker",
                hint: "AAAAAAAA YYYY (8 hex space 4 hex)",
                validation_regex: r"^[0-9A-F]{8}\s+[0-9A-F]{4}$",
                is_memory_poke: false,
            },
        ],
        // Atari 2600 — Stella accepts the BB:XX:DD format (3 pairs of
        // hex, colon-separated) for cheats. Not a "Game Genie" per se;
        // operators typically lift codes from the AtariAge community.
        "2600" => vec![
            CheatFormat {
                id: "stella_2600",
                label: "Stella cheat",
                hint: "BB:XX:DD (bank:address:data, hex)",
                validation_regex: r"^[0-9A-F]{2}:[0-9A-F]{2,4}:[0-9A-F]{2}$",
                is_memory_poke: false,
            },
        ],
        // N64 — Mupen64Plus-Next accepts GameShark-format codes
        // (AAAAAAAA YYYY or AAAAAAAA YYYYYYYY depending on the GS code
        // variant). Most community codes use the 8+4 form.
        "n64" => vec![
            CheatFormat {
                id: "gameshark_n64",
                label: "GameShark",
                hint: "AAAAAAAA YYYY (8 hex space 4 hex) OR 8+8 hex",
                validation_regex: r"^[0-9A-F]{8}\s+([0-9A-F]{4}|[0-9A-F]{8})$",
                is_memory_poke: false,
            },
        ],
        _ => Vec::new(),
    }
}

/// True if the given `kind` string represents a memory-poke cheat
/// (apply_cheats frame-loop dispatch). All other kinds — named formats
/// and the generic `libretro_code` — route through `Core::cheat_set`.
pub fn is_memory_poke_kind(kind: &str) -> bool {
    kind == "memory_poke"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_system_returns_memory_poke_plus_libretro_raw() {
        // The two universal kinds bracket every per-system list.
        for sys in &[
            "nes", "snes", "genesis", "segacd", "sega32x", "sms",
            "gamegear", "gb", "gbc", "gba", "2600", "n64",
            "tg16", "lynx", "pcfx", "scummvm", "dosbox",
        ] {
            let formats = cheat_formats_for(sys);
            assert_eq!(
                formats.first().map(|f| f.id),
                Some("memory_poke"),
                "{sys}: memory_poke must be first",
            );
            assert_eq!(
                formats.last().map(|f| f.id),
                Some("libretro_code"),
                "{sys}: libretro_code must be last",
            );
        }
    }

    #[test]
    fn nes_advertises_game_genie_and_pro_action_replay() {
        let ids: Vec<&str> = cheat_formats_for("nes").iter().map(|f| f.id).collect();
        assert!(ids.contains(&"game_genie_nes"), "nes must list Game Genie");
        assert!(ids.contains(&"pro_action_replay_nes"), "nes must list PAR");
    }

    #[test]
    fn gba_advertises_three_named_formats() {
        let ids: Vec<&str> = cheat_formats_for("gba").iter().map(|f| f.id).collect();
        assert!(ids.contains(&"game_genie_gba"));
        assert!(ids.contains(&"action_replay_gba"));
        assert!(ids.contains(&"codebreaker_gba"));
    }

    #[test]
    fn unknown_system_falls_through_to_memory_poke_plus_libretro_raw() {
        let formats = cheat_formats_for("not-a-real-system");
        assert_eq!(formats.len(), 2);
        assert_eq!(formats[0].id, "memory_poke");
        assert_eq!(formats[1].id, "libretro_code");
    }

    #[test]
    fn is_memory_poke_kind_only_matches_the_one_kind() {
        assert!(is_memory_poke_kind("memory_poke"));
        assert!(!is_memory_poke_kind("libretro_code"));
        assert!(!is_memory_poke_kind("game_genie_nes"));
        assert!(!is_memory_poke_kind(""));
        assert!(!is_memory_poke_kind("memory_pokes"));
    }

    /// Validation regexes are consumed by the frontend (JavaScript
    /// RegExp). Backend never compiles them — the patterns just travel
    /// over the wire as strings. We sanity-check that the strings have
    /// the basic well-formed-regex shape we expect (anchors at both
    /// ends, character class brackets balanced) so a typo doesn't
    /// silently disable per-format validation on the frontend side.
    #[test]
    fn every_named_format_regex_is_anchored_at_both_ends() {
        for sys in &[
            "nes", "snes", "genesis", "segacd", "sega32x", "sms",
            "gamegear", "gb", "gbc", "gba", "2600", "n64",
        ] {
            for format in cheat_formats_for(sys) {
                if format.validation_regex.is_empty() {
                    continue;
                }
                assert!(
                    format.validation_regex.starts_with('^'),
                    "{}/{}: regex missing leading ^ anchor",
                    sys, format.id,
                );
                assert!(
                    format.validation_regex.ends_with('$'),
                    "{}/{}: regex missing trailing $ anchor",
                    sys, format.id,
                );
                let opens = format.validation_regex.matches('[').count();
                let closes = format.validation_regex.matches(']').count();
                assert_eq!(
                    opens, closes,
                    "{}/{}: unbalanced character-class brackets",
                    sys, format.id,
                );
                let parens_open = format.validation_regex.matches('(').count();
                let parens_close = format.validation_regex.matches(')').count();
                assert_eq!(
                    parens_open, parens_close,
                    "{}/{}: unbalanced group parens",
                    sys, format.id,
                );
            }
        }
    }
}
