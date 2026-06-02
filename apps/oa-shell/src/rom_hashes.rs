//! Hash-based ROM identification — bind ROMs in the library to their
//! canonical entries in libretro-database via SHA-1.
//!
//! Three commands, three stages:
//!
//! 1. **`sync_rom_hashes_for_system`** — fetch `dat/<system>.dat` from
//!    libretro-database, parse the clrmamepro text format, populate the
//!    `rom_hashes` table. Per-system 24h cache (same TTL as the existing
//!    metadat sync).
//!
//! 2. **`resolve_rom_hashes_for_system`** — for every game of the
//!    system that doesn't have a sha1 stamped on it, hash the ROM
//!    (skipping CD images — the `.cue` / `.chd` content hash isn't in
//!    libretro-database), look up the sha1 in `rom_hashes`, on hit
//!    overwrite `games.title` with the canonical name and stamp the
//!    serial.
//!
//! 3. **`lookup_rom_hash`** — single-shot diagnostic that takes a sha1
//!    and returns the matching `RomHashRow`. Used by per-game UI
//!    surfaces that want to explain "why is this game named X?".
//!
//! ## Why SHA-1 only
//!
//! libretro-database keys each ROM entry on SHA-1, CRC32, and MD5.
//! SHA-1 is the canonical lookup (collision-resistant + the most
//! widely-attested upstream). CRC32 is useful when the user only has a
//! cataloged-but-not-hashed entry (rare); MD5 adds nothing SHA-1 doesn't.
//! Slice ships SHA-1 only — adding CRC32/MD5 fallbacks is a small follow-up.
//!
//! ## Why we skip CD images
//!
//! The .cue / .chd / .toc / .m3u file on disk doesn't have a meaningful
//! hash for the user's game — that depends on track contents, not the
//! container. CD identification is a separate game-ID database problem
//! (PSX serial via "SLUS_xxx.xx" matching, redump-style); out of scope
//! for this slice.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use rayon::prelude::*;
use serde::Serialize;
use sha1::{Digest, Sha1};
use tauri::Emitter;

use crate::archive;
use crate::library_db::{GameSerialRow, LibraryDb, RomHashRow};
use crate::rom_header::{candidate_sha1s, HeaderRule, Sha1Candidate};

const HASH_CACHE_TTL_SECS: u64 = 86_400; // 24h, matches metadat cache
const HASH_DB_CACHE_DIR: &str = "library-db/hashes";

/// A reference to one libretro-database `.dat` file. The repo lays its
/// dat files out as `<subdir>/<basename>.dat` and the same system can
/// have multiple dat files across subdirs — `metadat/no-intro/<sys>.dat`
/// for the canonical unheadered hashes, plus an optional
/// `metadat/headered/<sys>.dat` carrying headered-variant hashes for the
/// systems (NES iNES, Lynx LNX, Atari 7800 A78) where the upstream
/// curator maintains both. Both subdirs land in our local `rom_hashes`
/// table — different sha1s, same `canonical_title` — so headered and
/// unheadered files both hit directly.
#[derive(Debug, Clone, Copy)]
pub struct DatRef {
    pub subdir: &'static str,
    pub basename: &'static str,
}

/// Map an OA SystemId to one or more libretro-database `.dat` files.
/// Empty slice = no upstream dat for the system (sync is a no-op rather
/// than an error). The `dat/` subdir we used to target is a small
/// curated set; the canonical sources live under `metadat/no-intro/`
/// (cart-based) and `metadat/redump/` (CD-based). For systems where the
/// upstream curator also maintains a `metadat/headered/<sys>.dat`, we
/// fetch that too so headered files match without needing our algorithmic
/// header-strip pass (see `rom_header.rs`).
///
/// **New-core onboarding checklist item:** every system_id registered in
/// `bindings.rs` (dispatch arms) **must** also get an arm here — even if
/// the answer is `&[]` — so the wildcard fallback only ever fires on
/// truly-unknown ids. Forgetting this leaves new-system ROMs unmatched
/// against libretro-database with only an info-level log explaining why.
/// The canonical onboarded list lives in `bindings.rs` test fixtures
/// (search for `"tg16", "pce-cd", "lynx"`); keep this match in sync.
///
/// Reference: https://github.com/libretro/libretro-database/tree/master/metadat
fn libretro_dat_refs_for_system(system_id: &str) -> &'static [DatRef] {
    match system_id {
        "tg16" => &[DatRef {
            subdir: "metadat/no-intro",
            basename: "NEC - PC Engine - TurboGrafx 16",
        }],
        // CD images aren't hash-matched against single-file sha1s — the
        // disc-id extractor (Phase 2b) keys against game_serials instead.
        // Once that lands, this arm should switch to the redump dat
        // (metadat/redump/NEC - PC Engine CD - TurboGrafx-CD.dat) which
        // populates game_serials via parse_libretro_dat's serial path.
        "pce-cd" => &[],
        "lynx" => &[
            DatRef { subdir: "metadat/no-intro", basename: "Atari - Lynx" },
            DatRef { subdir: "metadat/headered", basename: "Atari - Lynx" },
        ],
        "nes" => &[
            DatRef {
                subdir: "metadat/no-intro",
                basename: "Nintendo - Nintendo Entertainment System",
            },
            DatRef {
                subdir: "metadat/headered",
                basename: "Nintendo - Nintendo Entertainment System",
            },
        ],
        "snes" => &[DatRef {
            subdir: "metadat/no-intro",
            basename: "Nintendo - Super Nintendo Entertainment System",
        }],
        "atari7800" => &[
            DatRef { subdir: "metadat/no-intro", basename: "Atari - 7800" },
            DatRef { subdir: "metadat/headered", basename: "Atari - 7800" },
        ],
        // Arcade ROM identification is set-based (MAME's own DAT/zip
        // layout), not single-file sha1 against libretro-database.
        "mame" => &[],
        // Mega Drive / Genesis. libretro-database keeps the no-intro
        // Sega MD dat in `metadat/no-intro/Sega - Mega Drive - Genesis.dat`.
        // SMD-format dumps would hash to different sha1s after
        // deinterleaving; no separate metadat/headered dat exists for
        // MD (modern dump sets all ship .md/.bin raw), so first-pass
        // identification will miss .smd files until rom_header.rs grows
        // an SMD-deinterleaver in a follow-up. Plain .md dumps match.
        "genesis" => &[DatRef {
            subdir: "metadat/no-intro",
            basename: "Sega - Mega Drive - Genesis",
        }],
        // Sega CD / Mega-CD. CD-shape — hash-based identification keys
        // against redump rather than no-intro because CD images aren't
        // single-file sha1-matched. cd_id.rs::extractors::sega_cd reads
        // the serial at offset 0x180 of the data track (after the
        // "SEGADISCSYSTEM" signature); the redump dat's `serial` fields
        // populate game_serials via parse_libretro_dat's serial path.
        "segacd" => &[DatRef {
            subdir: "metadat/redump",
            basename: "Sega - Mega-CD - Sega CD",
        }],
        // Sega 32X. Cart-shape addon — single no-intro dat covers the
        // small library (~36 official cart releases + homebrew).
        // Headerless raw .32x dumps; raw sha1 matches directly.
        "sega32x" => &[DatRef {
            subdir: "metadat/no-intro",
            basename: "Sega - 32X",
        }],
        // Sega Saturn. CD-shape — disc-id extraction via cd_id.rs reads
        // the product number at offset 0x20 of the IP.BIN header (after
        // the "SEGA SEGASATURN" signature). The redump dat's `serial`
        // fields populate game_serials so the lookup chain hits.
        "saturn" => &[DatRef {
            subdir: "metadat/redump",
            basename: "Sega - Saturn",
        }],
        // Sony PlayStation. CD-shape — cd_id.rs::extractors::psx_family
        // scans the first 32 KB of the data track for the SYSTEM.CNF
        // BOOT line and normalizes the catalog code (SLUS_001.67 →
        // SLUS-00167). The redump dat's `serial` field is the canonical
        // shape libretro-database uses for cover-art keying.
        "psx" => &[DatRef {
            subdir: "metadat/redump",
            basename: "Sony - PlayStation",
        }],
        // SNK Neo Geo. Cart-shape ROM-sets. libretro-database catalogs
        // the Neo Geo AES home + MVS arcade library under a single
        // no-intro dat. ROM-set hash matching is set-based (multiple
        // .p1/.c1/.m1 files per game), so the simple sha1 path matches
        // .neo single-file dumps cleanly but .zip ROM-set dumps will
        // need set-level matching — same gap MAME has. First-pass
        // matches .neo; ROM-set support is a Phase 2 polish.
        "neogeo" => &[DatRef {
            subdir: "metadat/no-intro",
            basename: "SNK - Neo Geo",
        }],
        // SNK Neo Geo CD. CD-shape — cd_id.rs::extractors::neo_geo_cd
        // scans for SNK catalog code prefixes (NGCD-/ADCD-/NCDZ-/TBCD-)
        // in the data track. The redump dat's `serial` field stores the
        // canonical form (e.g. "NGCD-030", "ADCD-103").
        "neocd" => &[DatRef {
            subdir: "metadat/redump",
            basename: "SNK - Neo Geo CD",
        }],
        // SNK Neo Geo Pocket / Color. Single slug covers both NGP +
        // NGPC; libretro-database keeps them in separate no-intro
        // dats — we merge into one local corpus via fetch_and_parse_all
        // (same gb/WonderSwan pattern).
        "ngp" => &[
            DatRef { subdir: "metadat/no-intro", basename: "SNK - Neo Geo Pocket" },
            DatRef { subdir: "metadat/no-intro", basename: "SNK - Neo Geo Pocket Color" },
        ],
        // Atari Jaguar. Cart-shape; single no-intro dat covers retail
        // + homebrew. Headerless raw .j64 / .jag dumps; raw sha1
        // matches directly.
        "jaguar" => &[DatRef {
            subdir: "metadat/no-intro",
            basename: "Atari - Jaguar",
        }],
        // 3DO Interactive Multiplayer. CD-shape — STAYS empty even
        // though the libretro-database 3DO dat exists, because that dat
        // carries NO `serial` fields (3DO never standardized a catalog
        // code). Disc-id lookup is structurally impossible — callers
        // fall back to filename + fuzzy title matching. The library can
        // still ID 3DO games by file SHA-1 against the redump dat if we
        // ever wire that path (currently CD-shape sha1 matching is
        // deferred because per-track .bin hashes vary by dump quality).
        "3do" => &[],
        // NEC PC-FX. CD-shape — cd_id.rs::extractors::pcfx scans for
        // FX-prefixed catalog codes (FXNHE742 + optional -N disc suffix)
        // after the "PC-FX:" signature. The redump dat's `serial` field
        // is the canonical key.
        "pcfx" => &[DatRef {
            subdir: "metadat/redump",
            basename: "NEC - PC-FX",
        }],
        // Nintendo 64. Cart-shape; single no-intro dat. .n64/.z64/.v64
        // are different byte-order conventions for the same canonical
        // content; the dat keys against the canonical Big-Endian (.z64)
        // sha1. `rom_header::header_rules_for("n64")` adds two
        // `HeaderRule::ByteSwap` candidates (Pairs16 for .v64, Words32
        // for .n64) so dumps in either non-canonical byte order match
        // the dat by normalizing to the .z64 BE SHA-1 at hash time.
        "n64" => &[DatRef {
            subdir: "metadat/no-intro",
            basename: "Nintendo - Nintendo 64",
        }],
        // Nintendo GameCube. Disc-image-shape — cd_id.rs::extractors::gamecube
        // reads the 6-byte header at offset 0 (console + game + region +
        // maker), synthesizes the canonical "DL-DOL-XXXX-REG" serial
        // libretro-database uses, and the redump dat's `serial` field
        // populates game_serials for lookup.
        "gamecube" => &[DatRef {
            subdir: "metadat/redump",
            basename: "Nintendo - GameCube",
        }],
        // Sega Dreamcast. GD-ROM disc-shape — cd_id.rs::extractors::dreamcast
        // reads the product number at offset 0x40 of IP.BIN (after the
        // "SEGA SEGAKATANA" signature). Wider window captures PAL "-50"
        // suffix that overflows the on-disc 10-byte field. The redump
        // dat's `serial` field populates game_serials.
        "dreamcast" => &[DatRef {
            subdir: "metadat/redump",
            basename: "Sega - Dreamcast",
        }],
        // Sony PlayStation Portable. UMD-shape (.iso/.cso/.pbp).
        // Single-file dumps can match against the no-intro PSP dat —
        // .iso/.cso are single-file containers that .pbp also wraps.
        "psp" => &[DatRef {
            subdir: "metadat/no-intro",
            basename: "Sony - PlayStation Portable",
        }],
        // Sony PlayStation 2. DVD-shape — disc-id extraction shares the
        // PSX SYSTEM.CNF pattern (cd_id.rs::extractors::psx_family
        // accepts both PSX SLUS/SLES and PS2 SLPM/SLPS prefixes). The
        // redump dat's `serial` field populates game_serials.
        "ps2" => &[DatRef {
            subdir: "metadat/redump",
            basename: "Sony - PlayStation 2",
        }],
        // Nintendo DS. Cart-shape (.nds single-file). Headerless raw
        // dumps key directly against the no-intro NDS dat.
        "nds" => &[DatRef {
            subdir: "metadat/no-intro",
            basename: "Nintendo - Nintendo DS",
        }],
        // Sega Master System. libretro-database catalogs both the
        // Western SMS lineup and the Japanese Mark III variants under
        // the same no-intro dat. Plain .sms dumps are headerless raw,
        // so the Raw candidate sha1 hits directly without header strip.
        "sms" => &[DatRef {
            subdir: "metadat/no-intro",
            basename: "Sega - Master System - Mark III",
        }],
        // Sega Game Gear. Headerless .gg dumps, raw sha1 hits directly.
        "gamegear" => &[DatRef {
            subdir: "metadat/no-intro",
            basename: "Sega - Game Gear",
        }],
        // Game Boy (DMG) and Game Boy Color (CGB) are now separate OA
        // slugs after the sidebar-tier registry split. Each routes to
        // its own libretro-database no-intro dat — .gb dumps match against
        // the DMG corpus, .gbc dumps against the CGB corpus. (Before the
        // split, both dats were merged into a combined `gb` corpus.)
        "gb" => &[
            DatRef { subdir: "metadat/no-intro", basename: "Nintendo - Game Boy" },
        ],
        "gbc" => &[
            DatRef { subdir: "metadat/no-intro", basename: "Nintendo - Game Boy Color" },
        ],
        // Game Boy Advance — single no-intro dat covers the entire library.
        // GBA dumps are headerless raw .gba files; the raw sha1 candidate
        // matches directly without header strip.
        "gba" => &[DatRef {
            subdir: "metadat/no-intro",
            basename: "Nintendo - Game Boy Advance",
        }],
        // Atari 2600 — single no-intro dat. 2600 dumps are headerless
        // raw cart bytes; raw sha1 matches directly. NOTE: a small
        // number of bankswitching schemes use a 256-byte header on
        // disk for the "Supercharger" cassette / multicart formats;
        // those would need a header-strip pass to match, but they're
        // a niche subset of the 2600 corpus. First-pass identification
        // matches stock .a26 dumps; the Supercharger / multicart pass
        // is a follow-up.
        "2600" => &[DatRef {
            subdir: "metadat/no-intro",
            basename: "Atari - 2600",
        }],
        // ColecoVision — single no-intro dat. Headerless raw cart dumps.
        "coleco" => &[DatRef {
            subdir: "metadat/no-intro",
            basename: "Coleco - ColecoVision",
        }],
        // Mattel Intellivision — single no-intro dat. Headerless raw dumps.
        "intv" => &[DatRef {
            subdir: "metadat/no-intro",
            basename: "Mattel - Intellivision",
        }],
        // Magnavox Odyssey² + Videopac G7000. The libretro-database dat
        // covers the US Odyssey² + EU Videopac G7000 + Videopac+ G7400
        // libraries in one file.
        "o2" => &[DatRef {
            subdir: "metadat/no-intro",
            basename: "Magnavox - Odyssey2",
        }],
        // Fairchild Channel F — tiny library (~26 official titles +
        // homebrew). Single no-intro dat.
        "channelf" => &[DatRef {
            subdir: "metadat/no-intro",
            basename: "Fairchild - Channel F",
        }],
        // GCE Vectrex — tiny library (~30 official titles + active
        // homebrew). Single no-intro dat.
        "vectrex" => &[DatRef {
            subdir: "metadat/no-intro",
            basename: "GCE - Vectrex",
        }],
        // Nintendo Virtual Boy — small library (~22 official titles).
        // Headerless raw dumps; raw sha1 matches directly.
        "virtualboy" => &[DatRef {
            subdir: "metadat/no-intro",
            basename: "Nintendo - Virtual Boy",
        }],
        // Bandai WonderSwan + WonderSwan Color. Single slug covers both
        // hardware variants; libretro-database keeps them in separate
        // dats — we merge into one local corpus via fetch_and_parse_all.
        "wonderswan" => &[
            DatRef { subdir: "metadat/no-intro", basename: "Bandai - WonderSwan" },
            DatRef { subdir: "metadat/no-intro", basename: "Bandai - WonderSwan Color" },
        ],
        // Atari 5200 SuperSystem. Cart-shape; headerless raw .a52 / .bin
        // dumps. Atari800 reads them directly.
        "5200" => &[DatRef {
            subdir: "metadat/no-intro",
            basename: "Atari - 5200",
        }],
        // Nintendo Pokémon Mini. Cart-shape; raw .min dumps.
        "pokemini" => &[DatRef {
            subdir: "metadat/no-intro",
            basename: "Nintendo - Pokemon Mini",
        }],
        // ScummVM — engine launcher, not a hardware platform. Game data
        // files vary per release (different revisions, language packs,
        // fan translations) so libretro-database doesn't ship a canonical
        // SHA-1 set. The cover-sync pipeline falls back to fuzzy filename
        // match at the 0.95 threshold, which works fine because operators
        // (or LaunchBox's ScummVM importer) name `.scummvm` files after
        // the canonical title ("Monkey Island.scummvm").
        "scummvm" => &[],
        // DOSBox — DOS-game runner, also no hardware platform. DOS
        // games shipped on multiple media (floppy / CD / GOG re-releases)
        // and through fan-curated patch sets; the directory contents
        // vary enormously across releases. Cover sync falls back to
        // fuzzy filename matching against the directory basename at
        // the 0.95 threshold.
        "dosbox" => &[],
        _ => &[],
    }
}

/// "Prefer-registry, fall back to const" shim for
/// [`libretro_dat_refs_for_system`]. When
/// `config/systems/<id>/system.yaml` carries a `libretro_dat_refs` block,
/// use those refs; otherwise fall through to the const arm.
///
/// Slice 2 of the per-system descriptor consolidation
/// (`docs/PLANS/per-system-descriptors.md`). Slice 1 already wired the
/// BIOS check + game info + system info paths through the registry;
/// this closes the dat-refs path. Phase D deletes the const arms +
/// this shim's fallback; the resolved fn becomes a direct registry
/// lookup.
///
/// **Implementation note (cache + leak):** the existing [`DatRef`]
/// carries `&'static str` fields because every consumer downstream
/// (`fetch_libretro_dat`, `fetch_and_parse_all`) treats refs as if they
/// were process-lifetime data. Converting from the registry's owned
/// `String` to `&'static str` requires either changing `DatRef` to
/// `Cow<'static, str>` (~30 callsite changes downstream) or `Box::leak`
/// (bounded one-time allocation per system — ~50 bytes × ~40 systems
/// = ~2 KB lifetime). Slice 2 takes the leak approach for minimal
/// disturbance; Phase D's `DatRef` shape can pivot to whatever the
/// post-const world prefers.
pub fn libretro_dat_refs_for_system_resolved(system_id: &str) -> &'static [DatRef] {
    use std::sync::{Mutex, OnceLock};

    static CACHE: OnceLock<Mutex<HashMap<String, &'static [DatRef]>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(&refs) = cache.lock().unwrap().get(system_id) {
        return refs;
    }

    let registry = crate::system_registry::global_registry();
    if let Some(loaded) = registry.get(system_id) {
        let refs: Vec<DatRef> = loaded
            .descriptor
            .libretro_dat_refs
            .iter()
            .map(|r| DatRef {
                subdir: Box::leak(r.subdir.clone().into_boxed_str()),
                basename: Box::leak(r.basename.clone().into_boxed_str()),
            })
            .collect();
        let leaked: &'static [DatRef] = Box::leak(refs.into_boxed_slice());
        cache.lock().unwrap().insert(system_id.to_string(), leaked);
        return leaked;
    }

    libretro_dat_refs_for_system(system_id)
}

/// Extensions we deliberately skip when computing hashes — CD-container
/// formats whose content hash means something different from "the
/// canonical ROM bytes." See module docs.
///
/// `.bin` and `.iso` are special: they're CD-container extensions ONLY
/// on systems that use disc-based media. On cart systems (Atari 2600,
/// ColecoVision, Intellivision, Odyssey², Channel F, etc.) a `.bin`
/// file is the raw cart dump and SHOULD be hashed normally. Pre-fix
/// the function treated `.bin` as CD universally, silently routing
/// 2600/Coleco/Intv/O2 `.bin` ROMs to the disc-id peek (which
/// returns "no signature" → skipped_cd) instead of hashing them
/// against the no-intro DAT.
pub(crate) fn is_cd_container_ext(ext: &str, system_id: &str) -> bool {
    // Universal CD-container extensions — these only ever appear on
    // disc-based systems and always mean "skip the hash, do disc-id".
    if matches!(ext, "cue" | "chd" | "ccd" | "toc" | "m3u") {
        return true;
    }
    // `.bin` and `.iso` only count as CD on systems that actually use
    // disc media. Cart systems' `.bin` dumps go through the normal
    // hash path.
    if matches!(ext, "bin" | "iso") {
        return matches!(
            system_id,
            "pce-cd"
                | "segacd"
                | "saturn"
                | "psx"
                | "ps2"
                | "neocd"
                | "pcfx"
                | "gamecube"
                | "dreamcast"
                | "3do"
                | "psp"
        );
    }
    false
}

fn hash_cache_path(app_data_dir: &Path, system_id: &str) -> PathBuf {
    app_data_dir
        .join(HASH_DB_CACHE_DIR)
        .join(format!("{}.json", system_id.replace(['/', '\\'], "_")))
}

#[derive(serde::Deserialize, serde::Serialize)]
struct CachedHashDb {
    fetched_at_unix_secs: u64,
    entries: Vec<RomHashRow>,
    /// Serial → canonical title pivot, parallel to `entries`. Default is
    /// empty so caches written by older builds (no `serials` field on
    /// disk) deserialize cleanly; first sync after upgrade refills it.
    #[serde(default)]
    serials: Vec<GameSerialRow>,
}

/// Output of one `parse_libretro_dat` pass — both shapes the upstream
/// `.dat` produces. `rom_hashes` is one row per `rom ( ... sha1 ... )`;
/// `game_serials` is one row per `game (... serial ...)` block (zero
/// rows for blocks with no serial).
pub struct ParsedDat {
    pub rom_hashes: Vec<RomHashRow>,
    pub game_serials: Vec<GameSerialRow>,
}

/// Parse the libretro-database clrmamepro-format .dat. Each `game (...)`
/// block carries a `name "..."` (canonical name we adopt as title), an
/// optional `serial "..."`, and one or more `rom ( ... sha1 XXXX ... )`
/// entries. Multi-disc games may have multiple rom blocks per game;
/// each gets its own row in our table because the user's local ROM is
/// hashed individually.
///
/// Robust to upstream comments + indentation variations. Lines we don't
/// recognize inside a game block are ignored — we only consume `name`,
/// `serial`, and the contents of `rom (...)` parens.
pub fn parse_libretro_dat(content: &str, system_id: &str) -> ParsedDat {
    /// One pending rom entry while we're inside a `game (...)` block.
    /// game.serial may appear before OR after the `rom ( ... )` line
    /// depending on the upstream dat, so we accumulate and stamp the
    /// final serial onto every rom in the block at the closing `)`.
    struct PendingRom {
        sha1: String,
        crc32: Option<String>,
        size_bytes: Option<i64>,
    }

    let mut rom_hashes: Vec<RomHashRow> = Vec::new();
    let mut game_serials: Vec<GameSerialRow> = Vec::new();
    let mut in_game = false;
    let mut current_name: Option<String> = None;
    let mut current_serial: Option<String> = None;
    let mut pending: Vec<PendingRom> = Vec::new();
    for raw in content.lines() {
        let line = raw.trim();
        if line.starts_with("game (") {
            in_game = true;
            current_name = None;
            current_serial = None;
            pending.clear();
            continue;
        }
        if !in_game {
            continue;
        }
        if line == ")" {
            in_game = false;
            if let Some(name) = current_name.take() {
                for r in pending.drain(..) {
                    rom_hashes.push(RomHashRow {
                        sha1: r.sha1,
                        system_id: system_id.to_string(),
                        game_name: name.clone(),
                        serial: current_serial.clone(),
                        crc32: r.crc32,
                        size_bytes: r.size_bytes,
                    });
                }
                // Emit one game_serials row per game block that carries
                // a serial. Title is the same canonical `name` we'd have
                // stamped via apply_rom_hash on a sha1 hit. Region stays
                // None for now — extracting it from "(USA)" / "(Japan)"
                // suffixes is a Phase 2b polish item.
                if let Some(serial) = current_serial.take() {
                    if !serial.is_empty() {
                        game_serials.push(GameSerialRow {
                            system_id: system_id.to_string(),
                            serial,
                            canonical_title: name,
                            region: None,
                        });
                    }
                }
            }
            current_serial = None;
            pending.clear();
            continue;
        }

        if let Some(rest) = line.strip_prefix("name ") {
            current_name = Some(unquote(rest));
            continue;
        }
        if let Some(rest) = line.strip_prefix("serial ") {
            current_serial = Some(unquote(rest));
            continue;
        }
        if let Some(rest) = line.strip_prefix("rom (") {
            // Format: `rom ( name "X" size N crc YYYY md5 ZZZZ sha1 WWWW )`
            // Keys/values can come in any order; we want sha1 + crc + size.
            // Trailing `)` is on the same line for libretro-database. Strip
            // both ends defensively.
            let body = rest.trim_end_matches(')').trim();
            let mut sha1: Option<String> = None;
            let mut crc32: Option<String> = None;
            let mut size: Option<i64> = None;
            let mut tokens = TokenIter::new(body);
            while let Some((key, value)) = tokens.next_pair() {
                match key.as_str() {
                    "sha1"   => sha1 = Some(value.to_ascii_lowercase()),
                    "crc"    => crc32 = Some(value.to_ascii_lowercase()),
                    "size"   => size = value.parse::<i64>().ok(),
                    _ => {}
                }
            }
            if let Some(sha1) = sha1 {
                pending.push(PendingRom { sha1, crc32, size_bytes: size });
            }
        }
    }
    // De-dupe game_serials by (system_id, serial). Some upstream .dats
    // ship multiple regional dumps under the same serial; first
    // occurrence wins (matches what INSERT-OR-REPLACE would do on a
    // database round-trip, just without the round-trip).
    let mut seen = HashMap::new();
    let mut deduped = Vec::with_capacity(game_serials.len());
    for row in game_serials {
        if seen.insert((row.system_id.clone(), row.serial.clone()), ()).is_none() {
            deduped.push(row);
        }
    }
    ParsedDat { rom_hashes, game_serials: deduped }
}

/// Parse a MAME-style clrmamepro dat into name-keyed title rows.
/// Source format from libretro-database `metadat/mame/MAME.dat`:
///
/// ```text
/// game (
///     name "Street Fighter II: Champion Edition (World 920313)"
///     year "1992"
///     developer "Capcom"
///     rom ( name sf2ce.zip size 5042192 crc XXXXX md5 XXXXX sha1 XXXXX )
/// )
/// ```
///
/// Each entry produces one [`MameTitleRow`] keyed by the .zip basename
/// (lowercased, extension stripped). Games with multiple `rom (...)`
/// lines emit one row per .zip — covers MAME's "merged" sets where a
/// parent + clones share a single game block.
pub fn parse_mame_dat(content: &str) -> Vec<crate::library_db::MameTitleRow> {
    let mut out = Vec::new();
    let mut in_game = false;
    let mut current_name: Option<String> = None;
    let mut current_year: Option<String> = None;
    let mut current_dev: Option<String> = None;
    let mut pending_zips: Vec<String> = Vec::new();
    for raw in content.lines() {
        let line = raw.trim();
        if line.starts_with("game (") {
            in_game = true;
            current_name = None;
            current_year = None;
            current_dev = None;
            pending_zips.clear();
            continue;
        }
        if !in_game {
            continue;
        }
        if line == ")" {
            in_game = false;
            if let Some(title) = current_name.take() {
                for zip in pending_zips.drain(..) {
                    let stem = std::path::Path::new(&zip)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_ascii_lowercase())
                        .unwrap_or_else(|| zip.to_ascii_lowercase());
                    if stem.is_empty() {
                        continue;
                    }
                    out.push(crate::library_db::MameTitleRow {
                        rom_set: stem,
                        title: title.clone(),
                        year: current_year.clone(),
                        developer: current_dev.clone(),
                    });
                }
            }
            current_year = None;
            current_dev = None;
            pending_zips.clear();
            continue;
        }
        if let Some(rest) = line.strip_prefix("name ") {
            current_name = Some(unquote(rest));
            continue;
        }
        if let Some(rest) = line.strip_prefix("year ") {
            current_year = Some(unquote(rest));
            continue;
        }
        if let Some(rest) = line.strip_prefix("developer ") {
            current_dev = Some(unquote(rest));
            continue;
        }
        if let Some(rest) = line.strip_prefix("rom (") {
            // Extract the `name` field — that's the .zip filename.
            let body = rest.trim_end_matches(')').trim();
            let mut tokens = TokenIter::new(body);
            while let Some((key, value)) = tokens.next_pair() {
                if key == "name" && value.ends_with(".zip") {
                    pending_zips.push(value);
                    break;
                }
            }
        }
    }
    out
}

/// Strip at most one leading and one trailing `"` from a trimmed
/// string. Pre-fix this used `trim_matches('"')` which strips ALL
/// leading + trailing quotes — a value like `""` becomes `""`
/// (correct) but `"Foo\"bar\""` with naïvely-escaped inner quotes
/// would round-trip to `Foo\"bar\"` rather than the intended
/// `Foo"bar"`. clrmamepro dats are well-formed in practice and
/// this is a defensive tightening rather than a bug fix — but the
/// stricter form matches the intent (treat the outer quotes as
/// delimiters, leave inner bytes alone).
fn unquote(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

/// Tiny `key value [key value]…` tokenizer with quote-aware values. The
/// `rom (...)` body never nests parens, and value lengths are bounded,
/// so a hand-rolled iterator outperforms regex / nom for this loop.
struct TokenIter<'a> {
    chars: std::str::Chars<'a>,
    peek: Option<char>,
}

impl<'a> TokenIter<'a> {
    fn new(s: &'a str) -> Self {
        let mut it = s.chars();
        let peek = it.next();
        Self { chars: it, peek }
    }

    fn advance(&mut self) -> Option<char> {
        let cur = self.peek;
        self.peek = self.chars.next();
        cur
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek {
            if c.is_ascii_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_word(&mut self) -> String {
        let mut out = String::new();
        while let Some(c) = self.peek {
            if c.is_ascii_whitespace() {
                break;
            }
            out.push(c);
            self.advance();
        }
        out
    }

    fn read_value(&mut self) -> String {
        self.skip_ws();
        if self.peek == Some('"') {
            self.advance(); // consume opening quote
            let mut out = String::new();
            while let Some(c) = self.peek {
                if c == '"' {
                    self.advance();
                    return out;
                }
                out.push(c);
                self.advance();
            }
            out
        } else {
            self.read_word()
        }
    }

    fn next_pair(&mut self) -> Option<(String, String)> {
        self.skip_ws();
        if self.peek.is_none() { return None; }
        let key = self.read_word();
        if key.is_empty() { return None; }
        let value = self.read_value();
        Some((key, value))
    }
}

/// Fetch one `<subdir>/<basename>.dat` from libretro-database via the
/// raw GitHub URL. 404 = upstream has no .dat at that path; we return
/// Ok(None) so the caller can fall through to the next ref (or report
/// "system has no dats") without surfacing as a hard error.
/// MAME title-lookup sync. Fetches `metadat/mame/MAME.dat` from libretro-
/// database, parses the clrmamepro entries into name-keyed rows, and
/// bulk-inserts into the `mame_titles` table. Frontend calls this on the
/// first MAME ingest (or via a manual "Sync MAME titles" action); the
/// result is consulted on subsequent imports so library tiles show human
/// titles instead of .zip filenames.
#[tauri::command]
pub async fn sync_mame_titles(
    db: tauri::State<'_, LibraryDb>,
) -> Result<MameTitleSyncSummary, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(180))
        .build()
        .map_err(|e| format!("reqwest client: {e}"))?;
    let dat_ref = DatRef { subdir: "metadat/mame", basename: "MAME" };
    let body = match fetch_libretro_dat(&client, dat_ref).await? {
        Some(b) => b,
        None => {
            return Err("libretro-database has no MAME.dat at metadat/mame/MAME.dat".to_string());
        }
    };
    let entries = parse_mame_dat(&body);
    let upstream_entries = entries.len();
    let written = db.replace_mame_titles(&entries)?;
    log::info!("rom_hashes: mame_titles synced — {upstream_entries} parsed, {written} written");
    Ok(MameTitleSyncSummary { upstream_entries, written })
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MameTitleSyncSummary {
    pub upstream_entries: usize,
    pub written: usize,
}

/// Frontend ingest helper — look up a MAME ROM-set by .zip basename
/// (e.g. "sf2ce") and return the human title + year + developer. Returns
/// `null` if the rom_set isn't in the catalog (homebrew, hack, or an
/// older MAME set the catalog hasn't picked up yet).
#[tauri::command]
#[allow(non_snake_case)]
pub fn lookup_mame_title(
    romSet: String,
    db: tauri::State<'_, LibraryDb>,
) -> Result<Option<crate::library_db::MameTitleRow>, String> {
    db.lookup_mame_title(&romSet)
}

async fn fetch_libretro_dat(
    client: &reqwest::Client,
    dat_ref: DatRef,
) -> Result<Option<String>, String> {
    let url = format!(
        "https://raw.githubusercontent.com/libretro/libretro-database/master/{}/{}.dat",
        dat_ref.subdir,
        urlencoding::encode(dat_ref.basename),
    );
    // get_text_with_retry retries once on 5xx / network errors. A
    // single 404 (no dat at this subdir for this system) returns
    // Ok(None) and is logged at debug level.
    let result = crate::http_retry::get_text_with_retry(client, &url, "OverlookedArcade").await;
    if let Ok(None) = &result {
        log::debug!("rom_hashes: {url} 404 (no dat at this path)");
    }
    result
}

/// Fetch + parse every DatRef registered for a system, merging the
/// results into one `ParsedDat`. A 404 on any individual ref is logged
/// and skipped (some systems intentionally have only the primary `.dat`
/// and no headered variant). Returns `Ok(None)` only when EVERY ref
/// 404s — at which point the caller treats the system as having no
/// upstream coverage at all.
async fn fetch_and_parse_all(
    client: &reqwest::Client,
    refs: &[DatRef],
    system_id: &str,
) -> Result<Option<ParsedDat>, String> {
    let mut all_rom_hashes = Vec::new();
    let mut all_serials = Vec::new();
    let mut any_hit = false;
    for r in refs {
        match fetch_libretro_dat(client, *r).await? {
            Some(text) => {
                any_hit = true;
                let parsed = parse_libretro_dat(&text, system_id);
                log::info!(
                    "rom_hashes: {system_id} {}/{} → {} entries / {} serials",
                    r.subdir,
                    r.basename,
                    parsed.rom_hashes.len(),
                    parsed.game_serials.len(),
                );
                all_rom_hashes.extend(parsed.rom_hashes);
                all_serials.extend(parsed.game_serials);
            }
            None => {
                log::debug!(
                    "rom_hashes: {system_id} {}/{}: 404, skipping",
                    r.subdir,
                    r.basename
                );
            }
        }
    }
    if !any_hit {
        return Ok(None);
    }
    // Dedupe merged serials by (system_id, serial) — first wins, same as
    // the parser's per-dat dedupe. Across dats (no-intro vs headered) the
    // serial is identical for the same game; we just want one row.
    let mut seen = HashMap::new();
    all_serials.retain(|row| {
        seen.insert((row.system_id.clone(), row.serial.clone()), ()).is_none()
    });
    Ok(Some(ParsedDat {
        rom_hashes: all_rom_hashes,
        game_serials: all_serials,
    }))
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RomHashSyncSummary {
    pub system_id: String,
    pub upstream_entries: usize,
    pub written: usize,
    pub from_cache: bool,
}

/// Fetch + parse + bulk-upsert the libretro-database `dat/<system>.dat`
/// into our local `rom_hashes` table. 24h cache to avoid re-pulling
/// the same file every session.
#[tauri::command]
#[allow(non_snake_case)]
pub async fn sync_rom_hashes_for_system(
    systemId: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::media::MediaState>,
    db: tauri::State<'_, LibraryDb>,
) -> Result<RomHashSyncSummary, String> {
    // Per-system op gate (H11) — see sync_media_for_system in media.rs.
    let gate = state.gate_for(&systemId);
    let _gate_guard = gate.lock().await;

    let app_data_dir = state.app_data_dir.clone();
    let refs = libretro_dat_refs_for_system_resolved(&systemId);
    if refs.is_empty() {
        log::info!("rom_hashes: no libretro-database mapping for {systemId}; skipping sync");
        return Ok(RomHashSyncSummary {
            system_id: systemId,
            upstream_entries: 0,
            written: 0,
            from_cache: false,
        });
    }

    // Cache lookup — same TTL as the metadat cache.
    let cache_path = hash_cache_path(&app_data_dir, &systemId);
    if let Ok(bytes) = std::fs::read(&cache_path) {
        if let Ok(cached) = serde_json::from_slice::<CachedHashDb>(&bytes) {
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            if !cached.entries.is_empty()
                && now.saturating_sub(cached.fetched_at_unix_secs) < HASH_CACHE_TTL_SECS
            {
                log::info!(
                    "rom_hashes: {systemId} cache hit ({} entries, {} serials)",
                    cached.entries.len(),
                    cached.serials.len(),
                );
                let upstream_entries = cached.entries.len();
                // Wipe-and-replace per system so the local table mirrors
                // the cached upstream snapshot exactly — entries removed
                // upstream don't linger as orphans, and entries the user
                // hand-imported into the wrong system aren't preserved
                // through a sync.
                let written = db.replace_rom_hashes_for_system(&systemId, &cached.entries)?;
                // Older caches deserialize with serials=[] (serde
                // default). Replacing with an empty slice clears the
                // table for the system; the next fresh fetch fills it
                // back in once the TTL expires.
                let _ = db.replace_game_serials_for_system(&systemId, &cached.serials);
                let _ = app.emit("oa://rom-hashes-synced", &RomHashSyncSummary {
                    system_id: systemId.clone(),
                    upstream_entries,
                    written,
                    from_cache: true,
                });
                return Ok(RomHashSyncSummary {
                    system_id: systemId,
                    upstream_entries,
                    written,
                    from_cache: true,
                });
            }
        }
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| format!("reqwest client: {e}"))?;

    let ParsedDat { rom_hashes: entries, game_serials: serials } =
        match fetch_and_parse_all(&client, refs, &systemId).await? {
            Some(p) => p,
            None => {
                log::warn!(
                    "rom_hashes: upstream has no dat for {systemId} (every ref 404'd)"
                );
                return Ok(RomHashSyncSummary {
                    system_id: systemId,
                    upstream_entries: 0,
                    written: 0,
                    from_cache: false,
                });
            }
        };
    let upstream_entries = entries.len();
    let upstream_serials = serials.len();
    log::info!(
        "rom_hashes: merged {upstream_entries} entries / {upstream_serials} serials for {systemId}"
    );
    // Replace the system's corpus rather than merging — upstream is the
    // source of truth, and stale entries from a prior sync with a
    // narrower ref set (or a since-removed upstream row) should not
    // linger.
    let written = db.replace_rom_hashes_for_system(&systemId, &entries)?;
    let _ = db.replace_game_serials_for_system(&systemId, &serials);

    // Cache write (24h reuse).
    if !entries.is_empty() {
        if let Some(parent) = cache_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let cached = CachedHashDb { fetched_at_unix_secs: now, entries, serials };
        let _ = std::fs::write(&cache_path, serde_json::to_vec(&cached).unwrap_or_default());
    }

    let summary = RomHashSyncSummary {
        system_id: systemId.clone(),
        upstream_entries,
        written,
        from_cache: false,
    };
    let _ = app.emit("oa://rom-hashes-synced", &summary);
    Ok(summary)
}

/// Load a ROM's bytes for hashing. Archived entries peek the archive at
/// `inner_path` (no extraction-to-disk); raw entries `std::fs::read`.
/// Caps at `MAX_ROM_BYTES` so a misclassified disc-image doesn't OOM us
/// — every cart ROM we care about sits well under the cap (SNES is 8 MB,
/// Lynx 1 MB, Atari 7800 144 KB). Past the cap we surface an error and
/// the caller skips the row (the resolve loop already swallows hash
/// errors and increments `summary.errors`).
///
/// `file_path` follows the library's encoded shape: archived entries
/// look like `"<archive_path>#<inner>"` (matches `encode_file_path` in
/// archive.rs); raw ROMs are a plain path. We always run the input
/// through `archive::decode_file_path` first so callers don't have to
/// duplicate the split logic.
const MAX_ROM_BYTES: u64 = 64 * 1024 * 1024;

pub(crate) fn rom_bytes_for(file_path: &str, archive_inner: Option<&str>) -> Result<Vec<u8>, String> {
    let (real_path, decoded_inner) = archive::decode_file_path(file_path);
    let inner = archive_inner.map(|s| s.to_string()).or_else(|| {
        if decoded_inner.is_empty() { None } else { Some(decoded_inner) }
    });
    if let Some(inner) = inner {
        return archive::read_inner_to_bytes(&real_path, &inner).map_err(|e| {
            format!("archive read {}#{inner}: {e}", real_path.display())
        });
    }
    let meta = std::fs::metadata(&real_path)
        .map_err(|e| format!("stat {}: {e}", real_path.display()))?;
    if meta.len() > MAX_ROM_BYTES {
        return Err(format!(
            "{} is {} bytes, exceeds {} byte cap — refusing to load into memory",
            real_path.display(),
            meta.len(),
            MAX_ROM_BYTES
        ));
    }
    std::fs::read(&real_path).map_err(|e| format!("read {}: {e}", real_path.display()))
}

/// Compute the SHA-1 of a ROM the way pre-header-aware code did — a
/// single hash of the canonical bytes (equivalent to the `Raw` rule).
/// Test-only: production resolve flow goes through `candidate_sha1s`
/// which tries header-stripped variants too.
#[cfg(test)]
fn sha1_of_rom(file_path: &str, archive_inner: Option<&str>) -> Result<String, String> {
    let bytes = rom_bytes_for(file_path, archive_inner)?;
    let mut hasher = Sha1::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

/// Streaming SHA-1 for large disc image files (PS2 / GameCube / Dreamcast
/// .iso dumps that can be 1.5–8 GB). Reads in 1 MB chunks so we never
/// hold more than ~1 MB of file bytes in memory at once. Used by the
/// resolve loop as a fallback after disc-ID extraction misses — catches
/// dumps the redump dat has SHA-1s for but where the disc-ID extraction
/// returned no signature.
pub(crate) fn stream_sha1_of_file(path: &std::path::Path) -> Result<String, String> {
    use std::io::Read;
    let f = std::fs::File::open(path)
        .map_err(|e| format!("open {}: {e}", path.display()))?;
    let mut reader = std::io::BufReader::with_capacity(1024 * 1024, f);
    let mut hasher = Sha1::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("read {}: {e}", path.display()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RomResolveProgress {
    pub system_id: String,
    pub done: usize,
    pub total: usize,
    pub current_title: String,
    pub last_action: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RomResolveSummary {
    pub system_id: String,
    pub scanned: usize,
    pub matched: usize,
    pub unmatched: usize,
    pub skipped_cd: usize,
    pub errors: usize,
    /// Number of canonical hash entries the local rom_hashes table held
    /// for this system at resolve time. When 0, every game in the
    /// system shows up as "unknown" — the UI uses this to surface
    /// "no hash DB available" rather than "we tried 1904 and found 0."
    pub canonical_entries: i64,
    /// Library row count for this system at resolve time. Compared
    /// against `already_identified` so the UI can show "all N games
    /// already identified, nothing to do" when a re-run is a no-op.
    pub library_total: i64,
    /// Subset of `library_total` whose `games.sha1` is already
    /// stamped (i.e. were resolved in a previous Identify pass).
    /// `library_total - already_identified` is roughly the work the
    /// current run will attempt (CD-shaped games further subtract
    /// out at the CD-skip path).
    pub already_identified: i64,
}

/// Hash every ROM in `system_id` that doesn't have a sha1 yet, look it
/// up in `rom_hashes`, and stamp the canonical title + serial on a
/// match. CD images are skipped (see module docs).
///
/// Auto-syncs the libretro-database `dat/<system>.dat` into our local
/// `rom_hashes` table if the system currently has zero canonical
/// entries — otherwise every game ends up "unknown" and the user has
/// no good signal as to why. The explicit `sync_rom_hashes_for_system`
/// command stays available for power users who want to force a refresh.
/// Run a one-shot upstream-dat fetch + parse + populate for `system_id`,
/// but only if the local `rom_hashes` table is currently empty for that
/// system. Mirrors the inline auto-sync that used to live at the head of
/// [`resolve_rom_hashes_for_system`].
///
/// Extracted (2026-06-01, Phase 1B Slice 1) so the smart-scan path in
/// `scan_service` can populate the hash table at scan time — without it,
/// a first-time operator's first scan can never light up the `confidence:
/// hash` tier because the `rom_hashes` table is still empty when the
/// scanner queries it. Calling this from `start_background_scan` before
/// the walk + hash pass closes that gap.
///
/// Failures are non-fatal — caller continues with an empty table if the
/// upstream is offline or the system has no `libretro_dat_refs_for_system`
/// mapping. Emits `oa://rom-hashes-synced` on success so any frontend
/// listener picks up the populated state.
pub(crate) async fn auto_sync_rom_hashes_if_empty(
    system_id: &str,
    app: &tauri::AppHandle,
    app_data_dir: &Path,
    db: &LibraryDb,
) -> Result<(), String> {
    if db.count_rom_hashes(system_id)? > 0 {
        return Ok(());
    }
    let refs = libretro_dat_refs_for_system_resolved(system_id);
    if refs.is_empty() {
        log::info!("rom_hashes: no libretro-database mapping for {system_id}");
        return Ok(());
    }
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            log::warn!("rom_hashes: reqwest client build failed: {e}");
            return Ok(());
        }
    };
    match fetch_and_parse_all(&client, refs, system_id).await {
        Ok(Some(ParsedDat { rom_hashes: entries, game_serials: serials })) => {
            let n = entries.len();
            let s = serials.len();
            log::info!(
                "rom_hashes: auto-sync {system_id} fetched {n} entries / {s} serials"
            );
            let _ = db.replace_rom_hashes_for_system(system_id, &entries);
            let _ = db.replace_game_serials_for_system(system_id, &serials);
            if !entries.is_empty() {
                let cache_path = hash_cache_path(app_data_dir, system_id);
                if let Some(parent) = cache_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let now = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let cached = CachedHashDb {
                    fetched_at_unix_secs: now,
                    entries,
                    serials,
                };
                let _ = std::fs::write(
                    &cache_path,
                    serde_json::to_vec(&cached).unwrap_or_default(),
                );
            }
            let _ = app.emit(
                "oa://rom-hashes-synced",
                &RomHashSyncSummary {
                    system_id: system_id.to_string(),
                    upstream_entries: n,
                    written: n,
                    from_cache: false,
                },
            );
        }
        Ok(None) => {
            log::warn!(
                "rom_hashes: upstream has no dat for {system_id} (every ref 404'd)"
            );
        }
        Err(e) => {
            log::warn!("rom_hashes: auto-sync {system_id} fetch failed: {e}");
        }
    }
    Ok(())
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn resolve_rom_hashes_for_system(
    systemId: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::media::MediaState>,
    db: tauri::State<'_, LibraryDb>,
) -> Result<RomResolveSummary, String> {
    // Per-system op gate (H11) — held for the lifetime of this call.
    // resolve_rom_hashes_for_system also calls sync_rom_hashes_for_system
    // inline (auto-sync block below) — but that's a function inlined
    // here, not a separate Tauri command invocation, so it doesn't try
    // to re-acquire the gate. The auto-sync block bypasses the wrapper
    // function's own gate acquisition by inlining only the fetch +
    // parse + apply steps.
    let gate = state.gate_for(&systemId);
    let _gate_guard = gate.lock().await;

    // Auto-sync if our local rom_hashes table is empty for this system.
    // Without this, "Identify ROMs" against an unsynced system returns
    // N unknown / 0 matched with no obvious cause; auto-syncing turns
    // the typical one-click flow into "fetch then resolve" transparently.
    auto_sync_rom_hashes_if_empty(&systemId, &app, &state.app_data_dir, &db).await?;

    let canonical_entries = db.count_rom_hashes(&systemId)?;
    let library_total = db.count_games_for_system(&systemId)?;
    let already_identified = db.count_games_with_hash_for_system(&systemId)?;
    let games = db.list_games_missing_hash(&systemId)?;
    let total = games.len();
    let mut summary = RomResolveSummary {
        system_id: systemId.clone(),
        scanned: 0,
        matched: 0,
        unmatched: 0,
        skipped_cd: 0,
        errors: 0,
        canonical_entries,
        library_total,
        already_identified,
    };

    // Log the re-run no-op case explicitly. Without this, the operator
    // sees "0/0 scanned, 0 matched" in the UI and assumes Identify is
    // broken when in fact every game has been identified already and a
    // re-run has nothing to do. The matching frontend status line
    // also shows "N already identified" via the new summary fields.
    if total == 0 && library_total > 0 && already_identified == library_total {
        log::info!(
            "rom_hashes: resolve {systemId} — all {library_total} game(s) already identified; nothing to do",
        );
    } else if total == 0 && library_total == 0 {
        log::info!(
            "rom_hashes: resolve {systemId} — no games in library for this system",
        );
    }

    // Short-circuit: nothing to match against. Skip the per-game hash
    // pass entirely — better to stamp 0 sha1s than to burn cycles on a
    // database the user can't possibly hit.
    if canonical_entries == 0 {
        log::info!(
            "rom_hashes: resolve {systemId} — no canonical entries available, returning early",
        );
        let _ = app.emit("oa://rom-hash-resolve-complete", &summary);
        return Ok(summary);
    }
    // Tiny in-process cache so re-hashing identical files in this batch
    // (e.g. user has the same ROM in two folders) doesn't pay for it
    // twice. Keyed on `(file_path, archive_inner)`; value is the full
    // candidate set so a re-hit doesn't pay for header-stripping either.
    //
    // Pre-populated in parallel — the cartridge-game read+hash work is
    // the dominant cost of resolve_rom_hashes_for_system, and was the
    // single biggest serial bottleneck in OA's library scan. Rayon
    // par_iter over the unique cart-game keys saturates all cores;
    // failed reads aren't inserted (the for-loop below will retry them
    // and surface the error through its existing progress-emission
    // path). CD games skip the cache entirely (they go through
    // peek_disc_id, not rom_bytes_for + candidate_sha1s).
    let mut hash_cache: HashMap<(String, Option<String>), Vec<Sha1Candidate>> = HashMap::new();
    {
        // Collect deduped cart keys. Skipping any game whose extension
        // is a CD container keeps the parallel pass focused on the
        // path it actually accelerates.
        let mut seen: std::collections::HashSet<(String, Option<String>)> =
            std::collections::HashSet::new();
        let mut cart_keys: Vec<(String, Option<String>)> = Vec::new();
        for g in &games {
            let ext = std::path::Path::new(g.archive_inner_path.as_deref().unwrap_or(&g.file_path))
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_default();
            if is_cd_container_ext(&ext, &systemId) {
                continue;
            }
            let key = (g.file_path.clone(), g.archive_inner_path.clone());
            if seen.insert(key.clone()) {
                cart_keys.push(key);
            }
        }
        if !cart_keys.is_empty() {
            let n = cart_keys.len();
            let started = std::time::Instant::now();
            let system_for_hash = systemId.clone();
            // Run on Tokio's blocking pool so the async runtime isn't
            // blocked for the duration. rayon::par_iter inside the
            // closure spreads the work across a bounded CPU pool.
            //
            // Pool size is capped at min(4, num_cpus) to bound transient
            // memory. Each worker may transiently hold up to ~2× the ROM
            // bytes during header-rule candidate generation (N64 ByteSwap
            // rule allocates a swapped copy of the 64 MB ROM before
            // hashing). On a 16-core machine, unbounded rayon would
            // spawn 16 workers × ~128 MB each = ~2 GB transient — fine
            // on a desktop with 32+ GB RAM, painful on a Steam Deck or
            // laptop. The 4-worker cap keeps peak transient at ~512 MB
            // for N64 and far less for everything else, at minimal cost
            // to wall-clock time (the hash work is mostly memcpy + sha1
            // which doesn't scale past memory bandwidth on most CPUs).
            let cpu_count = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4);
            let pool_size = cpu_count.min(4).max(1);
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(pool_size)
                .thread_name(|i| format!("oa-hash-{i}"))
                .build()
                .map_err(|e| format!("rayon pool build: {e}"))?;
            let results: Vec<((String, Option<String>), Vec<Sha1Candidate>)> =
                tokio::task::spawn_blocking(move || {
                    pool.install(|| {
                        cart_keys
                            .into_par_iter()
                            .filter_map(|key| {
                                let (file_path, inner) = &key;
                                match rom_bytes_for(file_path, inner.as_deref()) {
                                    Ok(bytes) => {
                                        let candidates = candidate_sha1s(&bytes, &system_for_hash);
                                        Some((key, candidates))
                                    }
                                    // Skip failures here — the for-loop's
                                    // cache-miss path will retry and surface
                                    // the error through its existing
                                    // progress-emission flow.
                                    Err(_) => None,
                                }
                            })
                            .collect()
                    })
                })
                .await
                .map_err(|e| format!("parallel hash join: {e}"))?;
            for (k, v) in results {
                hash_cache.insert(k, v);
            }
            log::info!(
                "rom_hashes: parallel pre-hash {} cart key(s) for {} in {:?} ({} cached)",
                n, systemId, started.elapsed(), hash_cache.len(),
            );
        }
    }

    let mut done = 0usize;
    for g in games {
        done += 1;
        let ext = std::path::Path::new(g.archive_inner_path.as_deref().unwrap_or(&g.file_path))
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        if is_cd_container_ext(&ext, &systemId) {
            // CD path: peek the disc-id from the data track, look it up
            // in `game_serials`, stamp `games.disc_id` so re-scans skip
            // the peek. Archived CDs use `peek_disc_id_archived` which
            // pulls just the cue + first ~64 KB of the data-track .bin
            // through the existing archive reader — no full extract.
            let peek_result = if let Some(inner) = g.archive_inner_path.as_deref() {
                let (archive_path, _) = archive::decode_file_path(&g.file_path);
                crate::cd_id::peek_disc_id_archived(&archive_path, inner, &systemId)
            } else {
                crate::cd_id::peek_disc_id(std::path::Path::new(&g.file_path), &systemId)
            };
            match peek_result {
                Ok(Some(disc)) => {
                    summary.scanned += 1;
                    match db.lookup_game_serial(&systemId, &disc.game_id)? {
                        Some(row) => {
                            summary.matched += 1;
                            if let Err(e) = db.apply_disc_id(
                                &g.id,
                                &disc.game_id,
                                Some(&row.canonical_title),
                            ) {
                                log::warn!("rom_hashes: apply_disc_id {} failed: {e}", g.id);
                                summary.errors += 1;
                            }
                            let _ = app.emit("oa://rom-hash-resolve-progress", &RomResolveProgress {
                                system_id: systemId.clone(),
                                done,
                                total,
                                current_title: g.title.clone(),
                                last_action: format!("matched (disc-id) → {}", row.canonical_title),
                            });
                        }
                        None => {
                            // Stamp the disc-id on the row anyway so
                            // re-scans don't re-peek; title stays as-is.
                            if let Err(e) = db.apply_disc_id(&g.id, &disc.game_id, None) {
                                log::warn!("rom_hashes: apply_disc_id (no-match) {} failed: {e}", g.id);
                                summary.errors += 1;
                            }
                            summary.unmatched += 1;
                            let _ = app.emit("oa://rom-hash-resolve-progress", &RomResolveProgress {
                                system_id: systemId.clone(),
                                done,
                                total,
                                current_title: g.title.clone(),
                                last_action: format!("disc-id {} — no match", disc.game_id),
                            });
                        }
                    }
                }
                Ok(None) => {
                    // No signature recognised in the data track. Try
                    // file-SHA-1 fallback for .iso disc images on the
                    // disc systems whose redump dat carries per-file
                    // hashes (PS2 / GameCube / Dreamcast). .chd dumps
                    // compress the underlying content so the file SHA
                    // doesn't match redump anyway — those stay
                    // disc-id-only.
                    if ext == "iso"
                        && g.archive_inner_path.is_none()
                        && matches!(systemId.as_str(), "ps2" | "gamecube" | "dreamcast")
                    {
                        // PS2/GameCube/Dreamcast .iso dumps are 1.4–8 GB.
                        // Streaming SHA-1 reads + hashes the entire file
                        // sequentially — multi-second to multi-minute
                        // work that would block this async task's runtime
                        // worker for the whole duration. spawn_blocking
                        // moves it to Tokio's blocking pool so the rest
                        // of the async runtime stays responsive.
                        let path_for_task = g.file_path.clone();
                        let sha_result = tokio::task::spawn_blocking(move || {
                            stream_sha1_of_file(std::path::Path::new(&path_for_task))
                        })
                        .await
                        .map_err(|e| format!("stream_sha1 join: {e}"))?;
                        match sha_result {
                            Ok(sha) => {
                                match db.lookup_rom_hash(&sha)? {
                                    Some(row) if row.system_id == systemId => {
                                        summary.matched += 1;
                                        // apply_rom_hash signature is
                                        // (id, sha1, canonical_title, serial).
                                        // Pre-fix this call swapped the two
                                        // tail args — stored serial as title
                                        // and title as serial, corrupting
                                        // both columns on every PS2/GC/DC
                                        // .iso match via this fallback. Cart
                                        // loop below (line ~1431) was always
                                        // correct.
                                        if let Err(e) = db.apply_rom_hash(
                                            &g.id,
                                            &sha,
                                            Some(&row.game_name),
                                            row.serial.as_deref(),
                                        ) {
                                            log::warn!("rom_hashes: apply_rom_hash {} failed: {e}", g.id);
                                            summary.errors += 1;
                                        }
                                        let _ = app.emit("oa://rom-hash-resolve-progress", &RomResolveProgress {
                                            system_id: systemId.clone(),
                                            done,
                                            total,
                                            current_title: g.title.clone(),
                                            last_action: format!("matched (file-sha1) → {}", row.game_name),
                                        });
                                        continue;
                                    }
                                    _ => {
                                        summary.unmatched += 1;
                                        let _ = app.emit("oa://rom-hash-resolve-progress", &RomResolveProgress {
                                            system_id: systemId.clone(),
                                            done,
                                            total,
                                            current_title: g.title.clone(),
                                            last_action: format!("file-sha1 {sha} — no match"),
                                        });
                                        continue;
                                    }
                                }
                            }
                            Err(e) => {
                                log::warn!("rom_hashes: stream_sha1_of_file {} failed: {e}", g.file_path);
                                // Fall through to skipped — the file
                                // read itself failed, not a catalog miss.
                            }
                        }
                    }
                    summary.skipped_cd += 1;
                    let _ = app.emit("oa://rom-hash-resolve-progress", &RomResolveProgress {
                        system_id: systemId.clone(),
                        done,
                        total,
                        current_title: g.title.clone(),
                        last_action: format!("no disc-id signature in .{ext}"),
                    });
                }
                Err(e) => {
                    // Distinct from `Ok(None)` (no signature found) —
                    // this is a hard read failure: corrupt CHD, missing
                    // .bin sidecar for a .cue, permission error, etc.
                    // Pre-fix all of these silently fell into
                    // `skipped_cd` alongside the legitimate
                    // no-signature case, masking real setup mistakes
                    // the operator could fix (e.g., a half-downloaded
                    // multi-disc set with one .bin missing). Now they
                    // count as `errors` and the verbose message in the
                    // progress emit + log surfaces what went wrong.
                    log::warn!(
                        "rom_hashes: peek_disc_id {} failed: {e}",
                        g.file_path
                    );
                    summary.errors += 1;
                    let _ = app.emit("oa://rom-hash-resolve-progress", &RomResolveProgress {
                        system_id: systemId.clone(),
                        done,
                        total,
                        current_title: g.title.clone(),
                        last_action: format!("disc-id read failed: {e}"),
                    });
                }
            }
            continue;
        }

        let key = (g.file_path.clone(), g.archive_inner_path.clone());
        let candidates = match hash_cache.get(&key) {
            Some(c) => c.clone(),
            None => {
                let bytes = match rom_bytes_for(&g.file_path, g.archive_inner_path.as_deref()) {
                    Ok(b) => b,
                    Err(e) => {
                        log::warn!("rom_hashes: read {} failed: {e}", g.file_path);
                        summary.errors += 1;
                        let _ = app.emit("oa://rom-hash-resolve-progress", &RomResolveProgress {
                            system_id: systemId.clone(),
                            done,
                            total,
                            current_title: g.title.clone(),
                            last_action: format!("read error: {e}"),
                        });
                        continue;
                    }
                };
                let candidates = candidate_sha1s(&bytes, &systemId);
                hash_cache.insert(key, candidates.clone());
                candidates
            }
        };

        // The Raw candidate is guaranteed to be present and first — use
        // it as the persisted sha1 on miss so re-scans skip rehashing.
        // `candidate_sha1s` panics-by-design if Raw is dropped (would
        // mean a bug in this module).
        let raw_sha1 = candidates
            .iter()
            .find(|c| c.rule == HeaderRule::Raw)
            .expect("candidate_sha1s always yields Raw first")
            .sha1
            .clone();

        summary.scanned += 1;
        // Try every candidate against the DB; first hit wins. Logging the
        // rule on hit is genuinely diagnostic — it tells the operator
        // whether their library is headered or not.
        let mut matched: Option<(RomHashRow, HeaderRule, String)> = None;
        for c in &candidates {
            if let Some(row) = db.lookup_rom_hash(&c.sha1)? {
                matched = Some((row, c.rule, c.sha1.clone()));
                break;
            }
        }

        match matched {
            Some((row, rule, sha1)) => {
                summary.matched += 1;
                if let Err(e) = db.apply_rom_hash(
                    &g.id,
                    &sha1,
                    Some(&row.game_name),
                    row.serial.as_deref(),
                ) {
                    log::warn!("rom_hashes: apply {} failed: {e}", g.id);
                    summary.errors += 1;
                }
                let last_action = match rule {
                    HeaderRule::Raw => format!("matched → {}", row.game_name),
                    _ => format!("matched (header-stripped) → {}", row.game_name),
                };
                let _ = app.emit("oa://rom-hash-resolve-progress", &RomResolveProgress {
                    system_id: systemId.clone(),
                    done,
                    total,
                    current_title: g.title.clone(),
                    last_action,
                });
            }
            None => {
                // Stamp the Raw sha1 so re-runs don't re-hash this file.
                // Title stays as-is.
                if let Err(e) = db.apply_rom_hash(&g.id, &raw_sha1, None, None) {
                    log::warn!("rom_hashes: apply-no-match {} failed: {e}", g.id);
                    summary.errors += 1;
                }
                summary.unmatched += 1;
                let _ = app.emit("oa://rom-hash-resolve-progress", &RomResolveProgress {
                    system_id: systemId.clone(),
                    done,
                    total,
                    current_title: g.title.clone(),
                    last_action: "no match".to_string(),
                });
            }
        }
    }

    log::info!(
        "rom_hashes: resolve_rom_hashes_for_system({systemId}) — scanned={} matched={} unmatched={} skipped_cd={} errors={}",
        summary.scanned, summary.matched, summary.unmatched, summary.skipped_cd, summary.errors,
    );
    let _ = app.emit("oa://rom-hash-resolve-complete", &summary);
    Ok(summary)
}

/// Diagnostic / debugger surface — look up a single sha1 in
/// `rom_hashes` and return the matched canonical entry (if any). Used
/// by per-game UI surfaces that want to explain "why is this named X?".
#[tauri::command]
pub fn lookup_rom_hash(
    sha1: String,
    db: tauri::State<'_, LibraryDb>,
) -> Result<Option<RomHashRow>, String> {
    db.lookup_rom_hash(&sha1)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_DAT: &str = r#"clrmamepro (
	name "NEC - PC Engine - TurboGrafx 16"
)

game (
	name "Bonk's Adventure (USA)"
	description "Bonk's Adventure (USA)"
	rom ( name "Bonk's Adventure (USA).pce" size 393216 crc 4F0BB6D2 md5 4F0BB6D24F0BB6D24F0BB6D24F0BB6D2 sha1 E22B706B22B706B22B706B22B706B22B706B22B7 )
	serial "TGX040080"
)

game (
	name "Air Zonk (USA)"
	description "Air Zonk (USA)"
	rom ( name "Air Zonk (USA).pce" size 524288 crc DEADBEEF sha1 abc123abc123abc123abc123abc123abc123abcd )
)
"#;

    #[test]
    fn parser_extracts_sha1_and_serial() {
        let ParsedDat { rom_hashes: entries, .. } = parse_libretro_dat(SAMPLE_DAT, "tg16");
        assert_eq!(entries.len(), 2, "expected 2 entries, got {entries:?}");
        let bonk = &entries[0];
        assert_eq!(bonk.game_name, "Bonk's Adventure (USA)");
        assert_eq!(bonk.serial.as_deref(), Some("TGX040080"));
        assert_eq!(bonk.sha1.len(), 40);
        assert_eq!(bonk.crc32.as_deref(), Some("4f0bb6d2"));
        assert_eq!(bonk.size_bytes, Some(393_216));
        let zonk = &entries[1];
        assert_eq!(zonk.game_name, "Air Zonk (USA)");
        assert_eq!(zonk.serial, None);
    }

    #[test]
    fn parser_emits_game_serials_for_blocks_with_serials() {
        let ParsedDat { game_serials, .. } = parse_libretro_dat(SAMPLE_DAT, "tg16");
        // Sample has 2 game blocks; only Bonk has a serial.
        assert_eq!(game_serials.len(), 1);
        let bonk = &game_serials[0];
        assert_eq!(bonk.system_id, "tg16");
        assert_eq!(bonk.serial, "TGX040080");
        assert_eq!(bonk.canonical_title, "Bonk's Adventure (USA)");
        assert!(bonk.region.is_none());
    }

    #[test]
    fn parser_dedupes_game_serials_by_serial() {
        // Two game blocks sharing the same serial → one game_serials row.
        // Mirrors libretro-database's habit of cataloging multiple
        // regional/revision dumps under one publisher catalog code.
        let dat = r#"
game (
	name "Foo (USA)"
	rom ( name "Foo.pce" size 1024 sha1 1111111111111111111111111111111111111111 )
	serial "TGX040000"
)

game (
	name "Foo (Japan)"
	rom ( name "Foo.pce" size 1024 sha1 2222222222222222222222222222222222222222 )
	serial "TGX040000"
)
"#;
        let ParsedDat { rom_hashes, game_serials } = parse_libretro_dat(dat, "tg16");
        assert_eq!(rom_hashes.len(), 2, "both rom rows survive");
        assert_eq!(game_serials.len(), 1, "duplicate serials dedupe");
    }

    #[test]
    fn parser_ignores_clrmamepro_header() {
        let ParsedDat { rom_hashes: entries, .. } = parse_libretro_dat(SAMPLE_DAT, "tg16");
        // Sample has 1 clrmamepro block + 2 game blocks. If the parser
        // ate the header, name="NEC - PC Engine - TurboGrafx 16" would
        // show up as a row.
        assert!(entries.iter().all(|e| e.game_name != "NEC - PC Engine - TurboGrafx 16"));
    }

    #[test]
    fn parser_skips_rom_blocks_without_sha1() {
        let dat = r#"
game (
	name "No Hash Entry"
	rom ( name "X.pce" size 1024 crc 11111111 )
)
"#;
        let ParsedDat { rom_hashes: entries, .. } = parse_libretro_dat(dat, "tg16");
        assert!(entries.is_empty(), "rom block without sha1 should be skipped");
    }

    #[test]
    fn token_iter_handles_quoted_values() {
        let mut it = TokenIter::new(r#"name "Bonk's Adventure (USA).pce" size 393216 sha1 ABC"#);
        let (k, v) = it.next_pair().unwrap();
        assert_eq!(k, "name");
        assert_eq!(v, "Bonk's Adventure (USA).pce");
        let (k, v) = it.next_pair().unwrap();
        assert_eq!(k, "size");
        assert_eq!(v, "393216");
        let (k, v) = it.next_pair().unwrap();
        assert_eq!(k, "sha1");
        assert_eq!(v, "ABC");
        assert!(it.next_pair().is_none());
    }

    /// Every system registered in `bindings.rs` dispatch must also have
    /// an explicit decision recorded in `libretro_dat_refs_for_system` —
    /// either one or more `DatRef`s, or an empty slice (listed in
    /// `NO_DAT_SYSTEMS` below) with the reason it's skipped. Forgetting
    /// this leaves a new system silently unmatched at "Identify ROMs"
    /// time (info-log only).
    ///
    /// If you're adding a new core and this test fails: find the system
    /// under https://github.com/libretro/libretro-database/tree/master/metadat
    /// (no-intro for cart-based, redump for CD-based) and add a DatRef
    /// to `libretro_dat_refs_for_system`. If the system genuinely has no
    /// hash-based identification path, add it to `NO_DAT_SYSTEMS` below
    /// with the reason.
    #[test]
    fn every_onboarded_system_has_an_explicit_rom_hashes_decision() {
        // Keep in sync with the canonical onboarded list in
        // `bindings.rs` tests (e.g. `default_pads_round_trip_to_button`).
        const ONBOARDED_SYSTEMS: &[&str] = &[
            "tg16", "pce-cd", "lynx", "nes", "snes", "mame", "atari7800",
            "genesis", "segacd", "sega32x", "saturn", "psx",
            "neogeo", "neocd", "ngp",
            "jaguar", "3do", "pcfx",
            "n64", "gamecube", "dreamcast",
            "psp", "ps2", "nds",
            "sms", "gamegear", "gb", "gbc", "gba", "2600",
            "coleco", "intv", "o2", "channelf",
            "vectrex", "virtualboy", "wonderswan",
            "5200", "pokemini",
            "scummvm", "dosbox",
        ];
        // Systems whose `libretro_dat_refs_for_system` returns an empty
        // slice on purpose. Document the reason next to the id.
        const NO_DAT_SYSTEMS: &[&str] = &[
            "pce-cd", // PCE-CD discs use catalog codes (Hu7-series) not present in libretro-database as standalone dat — game_serials is populated via the no-intro PCE dat which doesn't cover CD-shape titles.
            "3do",    // libretro-database 3DO dat carries NO `serial` fields — 3DO never standardized a catalog code, disc-id lookup is structurally impossible.
            "mame",   // arcade ROM identification is set-based, not single-file.
            "scummvm",// engine launcher, not hardware. Game data files vary by release/translation; cover sync falls back to fuzzy filename match at the 0.95 threshold.
            "dosbox", // DOS-game runner, also no hardware. Directory contents vary by media (floppy/CD/GOG re-release) + fan patches; fuzzy filename match at the 0.95 threshold against directory basename.
        ];
        for sys in ONBOARDED_SYSTEMS {
            let refs = libretro_dat_refs_for_system(sys);
            if NO_DAT_SYSTEMS.contains(sys) {
                assert!(
                    refs.is_empty(),
                    "{sys} is in NO_DAT_SYSTEMS but libretro_dat_refs_for_system returned refs — pick one"
                );
            } else {
                assert!(
                    !refs.is_empty(),
                    "{sys} is onboarded but libretro_dat_refs_for_system returned no refs. \
                     Add a DatRef pointing at the right metadat/ subdir + basename, or add \
                     {sys} to NO_DAT_SYSTEMS with the reason."
                );
                // Sanity: subdirs should be one of the known layouts.
                for r in refs {
                    assert!(
                        matches!(
                            r.subdir,
                            "dat" | "metadat/no-intro" | "metadat/redump" | "metadat/headered"
                        ),
                        "{sys}: unexpected subdir {:?} — extend the allowlist if libretro-database added a new layout",
                        r.subdir
                    );
                    assert!(!r.basename.is_empty(), "{sys}: empty basename");
                }
            }
        }
    }

    #[test]
    fn cd_container_extensions_are_filtered() {
        // Universal CD extensions are CD on EVERY system.
        for ext in ["cue", "chd", "ccd", "toc", "m3u"] {
            assert!(
                is_cd_container_ext(ext, "psx"),
                "{ext} should be a CD container on psx",
            );
            assert!(
                is_cd_container_ext(ext, "2600"),
                "{ext} is always a CD container even on cart systems (universal arm)",
            );
        }
        // Cart-system extensions are never CD.
        for ext in ["pce", "nes", "smc", "sfc", "lnx"] {
            assert!(
                !is_cd_container_ext(ext, "psx"),
                "{ext} should NOT be a CD container",
            );
        }
    }

    /// `.bin` and `.iso` only count as CD on disc systems. Pre-fix
    /// `is_cd_container_ext("bin")` returned true universally, causing
    /// 2600/Coleco/Intv/O2 `.bin` cart dumps to be routed to
    /// peek_disc_id (which fails) instead of getting hashed normally.
    #[test]
    fn bin_and_iso_only_cd_on_disc_systems() {
        // Disc systems — .bin and .iso are CD containers.
        for system in ["pce-cd", "segacd", "saturn", "psx", "ps2", "neocd", "pcfx",
                       "gamecube", "dreamcast", "3do", "psp"] {
            assert!(
                is_cd_container_ext("bin", system),
                ".bin should be CD on {system}",
            );
            assert!(
                is_cd_container_ext("iso", system),
                ".iso should be CD on {system}",
            );
        }
        // Cart systems — .bin is the raw cart dump, must hash normally.
        for system in ["2600", "coleco", "intv", "o2", "channelf", "vectrex",
                       "virtualboy", "nes", "snes", "genesis"] {
            assert!(
                !is_cd_container_ext("bin", system),
                ".bin should NOT be a CD container on cart system {system}",
            );
            assert!(
                !is_cd_container_ext("iso", system),
                ".iso should NOT be a CD container on cart system {system}",
            );
        }
    }

    #[test]
    fn sha1_of_raw_file_hashes_bytes() {
        let tmp = std::env::temp_dir().join(format!(
            "oa-romhash-raw-{}-{}.bin",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        std::fs::write(&tmp, b"abc").unwrap();
        let got = sha1_of_rom(tmp.to_str().unwrap(), None).expect("hash");
        // sha1("abc") = a9993e364706816aba3e25717850c26c9cd0d89d
        assert_eq!(got, "a9993e364706816aba3e25717850c26c9cd0d89d");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn sha1_of_archived_entry_decodes_encoded_path() {
        use std::io::Write;
        // Build a tiny in-memory zip on disk containing one file with
        // known content, then verify sha1_of_rom both with the encoded
        // `<zip>#<inner>` shape AND with the inner_path argument set.
        let tmp = std::env::temp_dir().join(format!(
            "oa-romhash-zip-{}-{}.zip",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        {
            let file = std::fs::File::create(&tmp).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            zip.start_file::<_, ()>("Xexyz (USA).nes", zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"abc").unwrap();
            zip.finish().unwrap();
        }
        let encoded = format!("{}#Xexyz (USA).nes", tmp.display());
        // The library stores BOTH the encoded path (in file_path) AND
        // the inner (in archive_inner_path). Make sure both routings work.
        let with_inner = sha1_of_rom(&encoded, Some("Xexyz (USA).nes")).expect("hash with inner");
        assert_eq!(with_inner, "a9993e364706816aba3e25717850c26c9cd0d89d");
        let only_encoded = sha1_of_rom(&encoded, None).expect("hash encoded only");
        assert_eq!(only_encoded, "a9993e364706816aba3e25717850c26c9cd0d89d");
        let _ = std::fs::remove_file(&tmp);
    }

    // ---- Slice 2 Phase C — registry round-trip for dat refs ----

    #[test]
    fn libretro_dat_refs_resolved_matches_legacy_for_all_systems() {
        // Slice 2 Phase C (2026-06-02): every system in
        // libretro_dat_refs_for_system's match arms must round-trip
        // through libretro_dat_refs_for_system_resolved (which prefers
        // the registry's config/systems/<id>/system.yaml
        // libretro_dat_refs block). Phase D deletes the const arms
        // + this test once direct registry lookup is the only path.
        //
        // The shipping registry should cover every system the const
        // function arms over. List drift in either direction is the
        // gap this test catches.
        let registry = crate::system_registry::global_registry();
        for sys_id in registry.system_ids() {
            let const_refs = libretro_dat_refs_for_system(sys_id);
            let resolved = libretro_dat_refs_for_system_resolved(sys_id);
            let const_pairs: std::collections::HashSet<(&str, &str)> = const_refs
                .iter()
                .map(|r| (r.subdir, r.basename))
                .collect();
            let resolved_pairs: std::collections::HashSet<(&str, &str)> = resolved
                .iter()
                .map(|r| (r.subdir, r.basename))
                .collect();
            assert_eq!(
                const_pairs, resolved_pairs,
                "libretro_dat_refs for {sys_id} differ between const + registry path"
            );
        }
    }
}
