//! migrate-systems — Slice 2 of the per-system descriptor consolidation arc.
//!
//! Walks OA's Rust source files (`apps/oa-shell/src/main.rs` for BIOS
//! const tables + `default_core_dll_for_system`; `core_installer.rs`
//! for `CATALOG`; `rom_hashes.rs` for `libretro_dat_refs_for_system`)
//! plus the in-tree `docs/cores/<id>/system-info.yaml` + `games-info.md`
//! and emits the consolidated `config/systems/<id>/{system,bios,games}.yaml`
//! triple per system.
//!
//! Run modes (CLI):
//!   migrate-systems                  # emit (write) all 41 systems
//!   migrate-systems --check          # diff against existing; exit 1 on drift
//!   migrate-systems --dry-run        # print what would write
//!   migrate-systems --systems gb,psx,nds  # restrict to a subset
//!
//! Phase A success criteria (round-trip): with `--systems gb,psx,nds
//! --check`, this tool should report no drift against the hand-written
//! Slice 1 YAMLs. Any reported drift is either a transcription error
//! we need to fix, or a schema improvement we want to capture for the
//! other 38 systems before Phase B's mass-emit.
//!
//! Deleted alongside the L1 const tables in Phase D once Slice 2 closes.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use regex::Regex;
use serde::{Deserialize, Serialize};

// =====================================================================
// CLI
// =====================================================================

#[derive(Parser, Debug)]
#[command(version, about = "Slice 2 system-descriptor migration tool")]
struct Args {
    /// Repository root (defaults to the parent of this tool's package).
    #[arg(long)]
    repo_root: Option<PathBuf>,

    /// Where to write the emitted YAMLs. Defaults to `<repo>/config/systems`.
    #[arg(long)]
    output_dir: Option<PathBuf>,

    /// Restrict emit/check to a comma-separated subset of system_ids.
    #[arg(long, value_delimiter = ',')]
    systems: Vec<String>,

    /// Check mode: diff against existing files; exit 1 on drift. No writes.
    #[arg(long)]
    check: bool,

    /// Dry-run mode: print what would write to stdout. No filesystem changes.
    #[arg(long)]
    dry_run: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let repo_root = args.repo_root.unwrap_or_else(default_repo_root);
    let output_dir = args
        .output_dir
        .clone()
        .unwrap_or_else(|| repo_root.join("config").join("systems"));

    let main_rs = read_source(&repo_root, "apps/oa-shell/src/main.rs")?;
    let core_installer_rs = read_source(&repo_root, "apps/oa-shell/src/core_installer.rs")?;
    let rom_hashes_rs = read_source(&repo_root, "apps/oa-shell/src/rom_hashes.rs")?;

    let default_cores = parse_default_core_map(&main_rs)
        .context("parsing default_core_dll_for_system")?;
    let bios_tables = parse_bios_tables(&main_rs)
        .context("parsing *_BIOS_KNOWN_HASHES const tables")?;
    let bios_dispatch = parse_bios_dispatch(&main_rs)
        .context("parsing known_hashes_for_system dispatcher (system_id → const table name)")?;
    let bios_semantics = parse_bios_semantics(&main_rs)
        .context("parsing per-system BiosSemantics from check_*_bios functions")?;
    let catalog_entries = parse_catalog(&core_installer_rs)
        .context("parsing core_installer::CATALOG")?;
    let dat_refs = parse_dat_refs(&rom_hashes_rs)
        .context("parsing libretro_dat_refs_for_system match arms")?;

    let docs_cores_dir = repo_root.join("docs").join("cores");

    let filter: Option<std::collections::HashSet<String>> = if args.systems.is_empty() {
        None
    } else {
        Some(args.systems.iter().cloned().collect())
    };

    let mut drift_count = 0usize;
    let mut emitted_count = 0usize;
    let mut skipped_no_filter_match = 0usize;

    for theme in themes::SYSTEM_THEMES {
        if let Some(f) = &filter {
            if !f.contains(theme.id) {
                skipped_no_filter_match += 1;
                continue;
            }
        }

        let plan = build_emit_plan(EmitInputs {
            theme,
            default_cores: &default_cores,
            bios_dispatch: &bios_dispatch,
            bios_tables: &bios_tables,
            bios_semantics: &bios_semantics,
            catalog_entries: &catalog_entries,
            dat_refs: &dat_refs,
            docs_cores_dir: &docs_cores_dir,
            output_dir: &output_dir,
        })?;

        let system_dir = output_dir.join(theme.id);

        if args.check {
            drift_count += check_one(&plan, &system_dir)?;
        } else if args.dry_run {
            print_one(&plan, &system_dir);
        } else {
            emit_one(&plan, &system_dir)?;
            emitted_count += 1;
        }
    }

    if args.check {
        if drift_count > 0 {
            eprintln!("migrate-systems: {drift_count} drift(s) detected; exit 1");
            std::process::exit(1);
        } else {
            println!("migrate-systems: no drift detected across all checked systems");
        }
    } else if !args.dry_run {
        println!(
            "migrate-systems: emitted {emitted_count} systems ({} skipped by filter)",
            skipped_no_filter_match,
        );
    }

    Ok(())
}

fn default_repo_root() -> PathBuf {
    // tools/migrate-systems/Cargo.toml lives at <repo>/tools/migrate-systems/.
    // CARGO_MANIFEST_DIR resolves at build time; walking two parents up
    // lands at the repo root.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn read_source(repo_root: &Path, relative: &str) -> Result<String> {
    let p = repo_root.join(relative);
    fs::read_to_string(&p).with_context(|| format!("read {}", p.display()))
}

// =====================================================================
// Themes — frontend systemThemes mirror
// =====================================================================

mod themes {
    /// Mirror of `frontend/src/themes/registry.ts::systemThemes`. Embedded
    /// here because reading TypeScript from Rust is fragile + the data is
    /// finite (41 systems) + this tool runs once and is deleted alongside
    /// the L1 const tables in Phase D of Slice 2.
    pub struct Theme {
        pub id: &'static str,
        pub display_name: &'static str,
        pub short_name: &'static str,
        pub extensions: &'static [&'static str],
        pub default_shader_preset: Option<&'static str>,
    }

    pub const SYSTEM_THEMES: &[Theme] = &[
        Theme { id: "tg16", display_name: "TurboGrafx-16 / PC Engine", short_name: "TG-16", extensions: &["pce"], default_shader_preset: Some("plain") },
        Theme { id: "pce-cd", display_name: "TurboGrafx-CD / PC Engine CD-ROM\u{00B2}", short_name: "TG-CD", extensions: &["cue","chd","ccd","toc","m3u","iso"], default_shader_preset: Some("plain") },
        Theme { id: "lynx", display_name: "Atari Lynx", short_name: "Lynx", extensions: &["lnx","lyx"], default_shader_preset: Some("crt-lite") },
        Theme { id: "nes", display_name: "Nintendo Entertainment System", short_name: "NES", extensions: &["nes","fds","unf","unif"], default_shader_preset: Some("crt-lite") },
        Theme { id: "snes", display_name: "Super Nintendo Entertainment System", short_name: "SNES", extensions: &["sfc","smc","fig","swc"], default_shader_preset: Some("crt-lite") },
        Theme { id: "atari7800", display_name: "Atari 7800 ProSystem", short_name: "Atari 7800", extensions: &["a78"], default_shader_preset: Some("crt-lite") },
        Theme { id: "genesis", display_name: "Sega Mega Drive / Genesis", short_name: "Genesis", extensions: &["md","smd","gen","68k"], default_shader_preset: Some("crt-lite") },
        Theme { id: "segacd", display_name: "Sega CD / Mega-CD", short_name: "Sega CD", extensions: &["cue","chd","ccd","toc","m3u","iso"], default_shader_preset: Some("plain") },
        Theme { id: "sega32x", display_name: "Sega 32X", short_name: "32X", extensions: &["32x"], default_shader_preset: Some("crt-lite") },
        Theme { id: "sega32xcd", display_name: "Sega 32X CD", short_name: "32X-CD", extensions: &["cue","chd","iso","m3u","ccd"], default_shader_preset: Some("plain") },
        Theme { id: "stv", display_name: "Sega Titan Video", short_name: "ST-V", extensions: &["zip","7z"], default_shader_preset: Some("crt-lite") },
        Theme { id: "saturn", display_name: "Sega Saturn", short_name: "Saturn", extensions: &["cue","chd","ccd","toc","m3u","iso"], default_shader_preset: Some("crt-lite") },
        Theme { id: "neogeo", display_name: "SNK Neo Geo", short_name: "Neo Geo", extensions: &["neo","zip"], default_shader_preset: Some("crt-lite") },
        Theme { id: "neocd", display_name: "SNK Neo Geo CD", short_name: "Neo Geo CD", extensions: &["cue","chd","ccd","toc","m3u","iso"], default_shader_preset: Some("crt-lite") },
        Theme { id: "n64", display_name: "Nintendo 64", short_name: "N64", extensions: &["n64","z64","v64"], default_shader_preset: Some("crt-lite") },
        Theme { id: "psp", display_name: "Sony PlayStation Portable", short_name: "PSP", extensions: &["iso","cso","pbp"], default_shader_preset: Some("lcd-handheld") },
        Theme { id: "ps2", display_name: "Sony PlayStation 2", short_name: "PS2", extensions: &["iso","chd"], default_shader_preset: Some("crt-lite") },
        Theme { id: "nds", display_name: "Nintendo DS", short_name: "NDS", extensions: &["nds"], default_shader_preset: Some("crt-lite") },
        Theme { id: "dreamcast", display_name: "Sega Dreamcast", short_name: "Dreamcast", extensions: &["cdi","gdi","chd"], default_shader_preset: Some("crt-lite") },
        Theme { id: "gamecube", display_name: "Nintendo GameCube + Wii", short_name: "GC / Wii", extensions: &["iso","gcm","gcz","rvz","wbfs"], default_shader_preset: Some("crt-lite") },
        Theme { id: "jaguar", display_name: "Atari Jaguar", short_name: "Jaguar", extensions: &["j64","jag"], default_shader_preset: Some("crt-lite") },
        Theme { id: "jagcd", display_name: "Atari Jaguar CD", short_name: "Jag CD", extensions: &["cue","chd","iso","m3u","ccd"], default_shader_preset: Some("crt-lite") },
        Theme { id: "3do", display_name: "3DO Interactive Multiplayer", short_name: "3DO", extensions: &["cue","chd","ccd","toc","m3u","iso"], default_shader_preset: Some("crt-lite") },
        Theme { id: "pcfx", display_name: "NEC PC-FX", short_name: "PC-FX", extensions: &["cue","chd","ccd","toc","m3u","iso"], default_shader_preset: Some("plain") },
        Theme { id: "ngp", display_name: "SNK Neo Geo Pocket Color", short_name: "NGP/C", extensions: &["ngp","ngc"], default_shader_preset: Some("lcd-handheld") },
        Theme { id: "psx", display_name: "Sony PlayStation", short_name: "PS1", extensions: &["cue","chd","ccd","toc","m3u","iso","pbp"], default_shader_preset: Some("crt-lite") },
        Theme { id: "gb", display_name: "Nintendo Game Boy", short_name: "Game Boy", extensions: &["gb"], default_shader_preset: Some("lcd-handheld") },
        Theme { id: "gbc", display_name: "Nintendo Game Boy Color", short_name: "Game Boy Color", extensions: &["gbc"], default_shader_preset: Some("lcd-handheld") },
        Theme { id: "vectrex", display_name: "GCE Vectrex", short_name: "Vectrex", extensions: &["vec","gam"], default_shader_preset: Some("vector-phosphor") },
        Theme { id: "virtualboy", display_name: "Nintendo Virtual Boy", short_name: "VB", extensions: &["vb"], default_shader_preset: Some("vb-monochrome") },
        Theme { id: "wonderswan", display_name: "Bandai WonderSwan", short_name: "WonderSwan", extensions: &["ws","wsc"], default_shader_preset: Some("lcd-handheld") },
        Theme { id: "coleco", display_name: "ColecoVision", short_name: "Coleco", extensions: &["col","cv"], default_shader_preset: Some("crt-lite") },
        Theme { id: "intv", display_name: "Mattel Intellivision", short_name: "Intv", extensions: &["int"], default_shader_preset: Some("crt-lite") },
        Theme { id: "o2", display_name: "Magnavox Odyssey\u{00B2}", short_name: "Odyssey\u{00B2}", extensions: &["o2"], default_shader_preset: Some("crt-lite") },
        Theme { id: "channelf", display_name: "Fairchild Channel F", short_name: "Channel F", extensions: &["chf"], default_shader_preset: Some("crt-lite") },
        Theme { id: "2600", display_name: "Atari 2600", short_name: "2600", extensions: &["a26"], default_shader_preset: Some("crt-lite") },
        Theme { id: "5200", display_name: "Atari 5200 SuperSystem", short_name: "5200", extensions: &["a52"], default_shader_preset: Some("crt-lite") },
        Theme { id: "pokemini", display_name: "Nintendo Pok\u{00E9}mon Mini", short_name: "PokeMini", extensions: &["min"], default_shader_preset: Some("lcd-handheld") },
        Theme { id: "gba", display_name: "Nintendo Game Boy Advance", short_name: "GBA", extensions: &["gba"], default_shader_preset: Some("lcd-handheld") },
        Theme { id: "sms", display_name: "Sega Master System", short_name: "SMS", extensions: &["sms"], default_shader_preset: Some("crt-lite") },
        Theme { id: "gamegear", display_name: "Sega Game Gear", short_name: "Game Gear", extensions: &["gg"], default_shader_preset: Some("lcd-handheld") },
        Theme { id: "mame", display_name: "Arcade (MAME)", short_name: "MAME", extensions: &["zip","chd"], default_shader_preset: Some("crt-lite") },
        Theme { id: "msx", display_name: "Microsoft MSX", short_name: "MSX", extensions: &[], default_shader_preset: Some("crt-lite") },
        Theme { id: "msx2", display_name: "Microsoft MSX2", short_name: "MSX2", extensions: &[], default_shader_preset: Some("crt-lite") },
        Theme { id: "scummvm", display_name: "ScummVM", short_name: "ScummVM", extensions: &["scummvm"], default_shader_preset: Some("plain") },
        Theme { id: "dosbox", display_name: "DOSBox", short_name: "DOSBox", extensions: &[], default_shader_preset: Some("crt-lite") },
    ];
}

// =====================================================================
// Rust source parsers
// =====================================================================

/// Parse `default_core_dll_for_system` match arms. Returns
/// `system_id → "core_libretro.dll"`. Tolerates either single-line
/// arms (`"slug" => "core.dll",`) or multi-line arms with comments
/// between the head and the closing comma.
fn parse_default_core_map(main_rs: &str) -> Result<HashMap<String, String>> {
    // Captures both single-slug arms (`"snes" => "snes9x_libretro.dll",`)
    // and combined arms (`"gb" | "gbc" => "gambatte_libretro.dll",`).
    // The LHS capture group holds the raw `"slug1" | "slug2"` form;
    // post-process splits it on `|` and dequotes each slug.
    let re = Regex::new(
        r#"(?m)^\s*("[a-z0-9\-]+"(?:\s*\|\s*"[a-z0-9\-]+")*)\s*=>\s*"([a-z0-9_]+\.dll)"\s*,"#,
    )?;
    let slug_re = Regex::new(r#""([a-z0-9\-]+)""#)?;
    let mut out = HashMap::new();
    for cap in re.captures_iter(main_rs) {
        let lhs = cap.get(1).unwrap().as_str();
        let core = cap.get(2).unwrap().as_str().to_string();
        for slug_cap in slug_re.captures_iter(lhs) {
            let slug = slug_cap.get(1).unwrap().as_str().to_string();
            // Default-core arms come first in main.rs by file order; if
            // a slug somehow appears twice (e.g. comment fixture below
            // the function), keep the first occurrence.
            out.entry(slug).or_insert_with(|| core.clone());
        }
    }
    if out.is_empty() {
        bail!("no default_core_dll_for_system arms parsed");
    }
    Ok(out)
}

/// One canonical BIOS file entry parsed from a `*_BIOS_KNOWN_HASHES`
/// const table tuple.
#[derive(Clone, Debug)]
struct BiosEntry {
    name: String,
    sha1: String,
    description: String,
}

/// One per-system BIOS table parsed from main.rs.
#[derive(Clone, Debug)]
struct BiosTable {
    files: Vec<BiosEntry>,
}

/// Parse every `*_BIOS_KNOWN_HASHES` const table. Key = const name.
fn parse_bios_tables(main_rs: &str) -> Result<HashMap<String, BiosTable>> {
    // Step 1: locate each const declaration block.
    let const_re = Regex::new(
        r#"(?ms)const\s+([A-Z0-9_]+_BIOS_KNOWN_HASHES)\s*:\s*&\[\s*\(\s*&str\s*,\s*&str\s*,\s*&str\s*\)\s*\]\s*=\s*&\[(.*?)\];"#,
    )?;
    // Step 2: per block, extract `("name", "sha1", "desc"),` tuples.
    let tuple_re = Regex::new(
        r#"\(\s*"([^"]+)"\s*,\s*"([^"]+)"\s*,\s*"([^"]*)"\s*\)\s*,"#,
    )?;

    let mut out = HashMap::new();
    for cap in const_re.captures_iter(main_rs) {
        let const_name = cap.get(1).unwrap().as_str().to_string();
        let body = cap.get(2).unwrap().as_str();
        let mut files = Vec::new();
        for tcap in tuple_re.captures_iter(body) {
            files.push(BiosEntry {
                name: tcap.get(1).unwrap().as_str().to_string(),
                sha1: tcap.get(2).unwrap().as_str().to_string(),
                description: tcap.get(3).unwrap().as_str().to_string(),
            });
        }
        if files.is_empty() {
            bail!("const {const_name} parsed but contained zero tuples — regex drift?");
        }
        out.insert(const_name, BiosTable { files });
    }
    if out.is_empty() {
        bail!("no *_BIOS_KNOWN_HASHES const tables parsed");
    }
    Ok(out)
}

/// Parse `known_hashes_for_system` match arms. Returns `system_id →
/// const_name` so we can join `parse_bios_tables` output back to a
/// per-system key.
fn parse_bios_dispatch(main_rs: &str) -> Result<HashMap<String, String>> {
    // Function shape:
    //   fn known_hashes_for_system(system_id: &str) -> &'static [(&'static str, &'static str, &'static str)] {
    //       match system_id {
    //           "pce-cd" => PCE_BIOS_KNOWN_HASHES,
    //           "segacd" => SEGA_CD_BIOS_KNOWN_HASHES,
    //           ...
    //           _ => &[],
    //       }
    //   }
    let scope_re = Regex::new(
        r#"(?ms)fn\s+known_hashes_for_system[^{]*\{(.*?)^\}"#,
    )?;
    let scope = scope_re
        .captures(main_rs)
        .ok_or_else(|| anyhow!("known_hashes_for_system not found"))?
        .get(1)
        .unwrap()
        .as_str();

    let arm_re = Regex::new(r#""([a-z0-9\-]+)"\s*=>\s*([A-Z0-9_]+_BIOS_KNOWN_HASHES)"#)?;
    let mut out = HashMap::new();
    for cap in arm_re.captures_iter(scope) {
        out.insert(
            cap.get(1).unwrap().as_str().to_string(),
            cap.get(2).unwrap().as_str().to_string(),
        );
    }
    if out.is_empty() {
        bail!("no known_hashes_for_system arms parsed");
    }
    Ok(out)
}

/// Parse per-system BIOS semantics by scanning each `check_*_bios`
/// function for its `bios_check_from_inventory(..., BiosSemantics::Xxx)`
/// call. Returns `system_id → "any_of" | "all_required"`.
///
/// The system_id is derived from the const name in the SAME function:
/// `let files = scan_bios_table(system_dir, PSX_BIOS_KNOWN_HASHES);`
/// followed by `bios_check_from_inventory(files, BiosSemantics::AnyOf)`.
/// We pair each scan_bios_table call with the subsequent
/// bios_check_from_inventory in the same function body.
fn parse_bios_semantics(main_rs: &str) -> Result<HashMap<String, String>> {
    // Match per-function: `fn check_xxx_bios(...) -> Result<BiosCheck, BiosError> { ... }`
    let fn_re = Regex::new(
        r#"(?ms)fn\s+check_[a-z0-9_]+_bios\s*\([^)]*\)\s*->\s*Result<BiosCheck,\s*BiosError>\s*\{(.*?)^\}"#,
    )?;
    let table_re = Regex::new(r#"scan_bios_table\([^,]+,\s*([A-Z0-9_]+_BIOS_KNOWN_HASHES)"#)?;
    let sem_re = Regex::new(r#"BiosSemantics::(AnyOf|AllRequired)"#)?;

    let mut out = HashMap::new();
    for fcap in fn_re.captures_iter(main_rs) {
        let body = fcap.get(1).unwrap().as_str();
        let const_name = match table_re.captures(body) {
            Some(c) => c.get(1).unwrap().as_str().to_string(),
            None => continue, // check_neogeo_bios + similar specials lack scan_bios_table
        };
        let sem = match sem_re.captures(body) {
            Some(c) => c.get(1).unwrap().as_str(),
            None => continue,
        };
        let yaml_sem = match sem {
            "AnyOf" => "any_of",
            "AllRequired" => "all_required",
            _ => continue,
        };
        out.insert(const_name, yaml_sem.to_string());
    }
    if out.is_empty() {
        bail!("no per-system BIOS semantics parsed from check_*_bios bodies");
    }
    Ok(out)
}

/// One CATALOG entry parsed from core_installer.rs.
#[derive(Clone, Debug)]
struct CatalogEntry {
    base: String,
    display_name: String,
    blurb: String,
    systems: Vec<String>,
    recommended: bool,
    bios_required: Option<String>,
}

/// Parse `CATALOG` entries from core_installer.rs. The shape is a struct
/// literal table; we scan each `CatalogEntry { ... }` block and pull
/// fields out by name.
fn parse_catalog(core_installer_rs: &str) -> Result<Vec<CatalogEntry>> {
    let entry_re = Regex::new(r#"(?s)CatalogEntry\s*\{(.*?)\},"#)?;
    let base_re = Regex::new(r#"base:\s*"([^"]+)""#)?;
    let display_re = Regex::new(r#"display_name:\s*"([^"]+)""#)?;
    let blurb_re = Regex::new(r#"blurb:\s*"((?:[^"\\]|\\.)*)""#)?;
    let systems_re = Regex::new(r#"systems:\s*&\[([^\]]+)\]"#)?;
    let system_id_re = Regex::new(r#""([a-z0-9\-]+)""#)?;
    let recommended_re = Regex::new(r#"recommended:\s*(true|false)"#)?;
    // bios_required is either Some("...") or None.
    let bios_some_re = Regex::new(r#"bios_required:\s*Some\(\s*"([^"]+)"\s*\)"#)?;
    let bios_none_re = Regex::new(r#"bios_required:\s*None"#)?;

    let mut out = Vec::new();
    for cap in entry_re.captures_iter(core_installer_rs) {
        let body = cap.get(1).unwrap().as_str();
        let base = base_re
            .captures(body)
            .ok_or_else(|| anyhow!("CatalogEntry block missing `base` field"))?
            .get(1)
            .unwrap()
            .as_str()
            .to_string();
        let display_name = display_re
            .captures(body)
            .ok_or_else(|| anyhow!("CatalogEntry {base} missing `display_name`"))?
            .get(1)
            .unwrap()
            .as_str()
            .to_string();
        let blurb_raw = blurb_re
            .captures(body)
            .ok_or_else(|| anyhow!("CatalogEntry {base} missing `blurb`"))?
            .get(1)
            .unwrap()
            .as_str();
        let blurb = unescape_rust_string(blurb_raw);
        let systems_body = systems_re
            .captures(body)
            .ok_or_else(|| anyhow!("CatalogEntry {base} missing `systems`"))?
            .get(1)
            .unwrap()
            .as_str();
        let systems: Vec<String> = system_id_re
            .captures_iter(systems_body)
            .map(|c| c.get(1).unwrap().as_str().to_string())
            .collect();
        let recommended = recommended_re
            .captures(body)
            .ok_or_else(|| anyhow!("CatalogEntry {base} missing `recommended`"))?
            .get(1)
            .unwrap()
            .as_str()
            == "true";
        let bios_required = if bios_none_re.is_match(body) {
            None
        } else if let Some(c) = bios_some_re.captures(body) {
            Some(c.get(1).unwrap().as_str().to_string())
        } else {
            None
        };
        out.push(CatalogEntry {
            base,
            display_name,
            blurb,
            systems,
            recommended,
            bios_required,
        });
    }
    if out.is_empty() {
        bail!("no CatalogEntry blocks parsed");
    }
    Ok(out)
}

/// One libretro-database DAT reference parsed from
/// `libretro_dat_refs_for_system`.
#[derive(Clone, Debug)]
struct DatRef {
    subdir: String,
    basename: String,
}

/// Parse `libretro_dat_refs_for_system` match arms. Returns `system_id
/// → Vec<DatRef>`. Arms with empty slices (`"3do" => &[]`) map to an
/// empty Vec.
fn parse_dat_refs(rom_hashes_rs: &str) -> Result<HashMap<String, Vec<DatRef>>> {
    // Function block — bounded to avoid matching DatRef literals in
    // doc comments elsewhere.
    let scope_re = Regex::new(
        r#"(?ms)fn\s+libretro_dat_refs_for_system[^{]*\{(.*?)^\}"#,
    )?;
    let scope = scope_re
        .captures(rom_hashes_rs)
        .ok_or_else(|| anyhow!("libretro_dat_refs_for_system not found"))?
        .get(1)
        .unwrap()
        .as_str();

    // Per-arm: `"slug" => &[DatRef {...}, DatRef {...}],` OR `"slug" => &[],`.
    // Capture the arm body inclusive of its closing `],` so we can find
    // multiple DatRef literals inside.
    let arm_re = Regex::new(r#"(?s)"([a-z0-9\-]+)"\s*=>\s*&\[(.*?)\]\s*,"#)?;
    let datref_re = Regex::new(
        r#"DatRef\s*\{\s*subdir:\s*"([^"]+)"\s*,\s*basename:\s*"([^"]+)"\s*,?\s*\}"#,
    )?;

    let mut out = HashMap::new();
    for arm in arm_re.captures_iter(scope) {
        let slug = arm.get(1).unwrap().as_str().to_string();
        let body = arm.get(2).unwrap().as_str();
        let refs: Vec<DatRef> = datref_re
            .captures_iter(body)
            .map(|c| DatRef {
                subdir: c.get(1).unwrap().as_str().to_string(),
                basename: c.get(2).unwrap().as_str().to_string(),
            })
            .collect();
        out.insert(slug, refs);
    }
    if out.is_empty() {
        bail!("no libretro_dat_refs_for_system arms parsed");
    }
    Ok(out)
}

/// Unescape a Rust string-literal body to its runtime form. Handles the
/// escapes that appear in CATALOG blurbs (\" and \\). Leaves the rest
/// untouched.
fn unescape_rust_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

// =====================================================================
// Emit shapes — YAML structures that match the descriptor schema
// =====================================================================

#[derive(Serialize, Default)]
#[serde(rename_all = "snake_case")]
struct SystemYaml {
    id: String,
    display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    short_name: Option<String>,
    schema_version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_core: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_shader_preset: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    extensions: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    libretro_dat_refs: Vec<DatRefYaml>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    cores: Vec<CoreEntryYaml>,
    /// Embedded L2 panel block. Round-tripped as opaque YAML so the
    /// SystemInfoCurated shape stays under the L2-format's own schema
    /// without our migrator needing to model every field.
    #[serde(skip_serializing_if = "Option::is_none")]
    system_info: Option<serde_yaml::Value>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct DatRefYaml {
    subdir: String,
    basename: String,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct CoreEntryYaml {
    base: String,
    display_name: String,
    blurb: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    recommended: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    bios_required: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct BiosYaml {
    schema_version: u32,
    semantics: String,
    required_for_launch: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    sourcing_hint: Option<String>,
    files: Vec<BiosFileYaml>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct BiosFileYaml {
    name: String,
    sha1: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    description: String,
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    optional: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct GamesYaml {
    schema_version: u32,
    games: Vec<serde_yaml::Value>,
}

// =====================================================================
// Plan-building + emit/check/dry-run
// =====================================================================

struct EmitInputs<'a> {
    theme: &'a themes::Theme,
    default_cores: &'a HashMap<String, String>,
    bios_dispatch: &'a HashMap<String, String>,
    bios_tables: &'a HashMap<String, BiosTable>,
    bios_semantics: &'a HashMap<String, String>,
    catalog_entries: &'a [CatalogEntry],
    dat_refs: &'a HashMap<String, Vec<DatRef>>,
    docs_cores_dir: &'a Path,
    /// Output directory (typically `<repo>/config/systems`). The
    /// migrator checks for an already-migrated `<output_dir>/<id>/system.yaml`
    /// and reads its `system_info:` block when present, so re-running
    /// the tool on already-migrated systems (GB / PSX / NDS in Slice 1)
    /// round-trips the panel content correctly. Without this, the
    /// migrator would emit empty `system_info` for those systems
    /// because Slice 1 deleted the legacy `docs/cores/<id>/system-info.yaml`.
    output_dir: &'a Path,
}

struct EmitPlan {
    system_yaml: String,
    bios_yaml: Option<String>,
    games_yaml: Option<String>,
}

fn build_emit_plan(inputs: EmitInputs) -> Result<EmitPlan> {
    let theme = inputs.theme;

    let default_core = inputs.default_cores.get(theme.id).cloned();

    let dat_refs: Vec<DatRefYaml> = inputs
        .dat_refs
        .get(theme.id)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|r| DatRefYaml {
            subdir: r.subdir,
            basename: r.basename,
        })
        .collect();

    let cores: Vec<CoreEntryYaml> = inputs
        .catalog_entries
        .iter()
        .filter(|e| e.systems.iter().any(|s| s == theme.id))
        .map(|e| CoreEntryYaml {
            base: e.base.clone(),
            display_name: e.display_name.clone(),
            blurb: e.blurb.clone(),
            recommended: e.recommended,
            bios_required: e.bios_required.clone(),
        })
        .collect();

    let system_info =
        load_existing_system_info(inputs.docs_cores_dir, inputs.output_dir, theme.id)?;

    let system_yaml_struct = SystemYaml {
        id: theme.id.to_string(),
        display_name: theme.display_name.to_string(),
        short_name: Some(theme.short_name.to_string()),
        schema_version: 1,
        default_core,
        default_shader_preset: theme.default_shader_preset.map(|s| s.to_string()),
        extensions: theme.extensions.iter().map(|s| s.to_string()).collect(),
        libretro_dat_refs: dat_refs,
        cores,
        system_info,
    };
    let system_yaml = serde_yaml::to_string(&system_yaml_struct)?;

    // BIOS YAML — only when this system has a const table per the dispatcher.
    let bios_yaml = if let Some(const_name) = inputs.bios_dispatch.get(theme.id) {
        let table = inputs
            .bios_tables
            .get(const_name)
            .ok_or_else(|| anyhow!("dispatcher references unknown const {const_name}"))?;
        let semantics = inputs
            .bios_semantics
            .get(const_name)
            .cloned()
            .unwrap_or_else(|| "any_of".to_string());
        let bios = BiosYaml {
            schema_version: 1,
            semantics,
            required_for_launch: true,
            sourcing_hint: None,
            files: table
                .files
                .iter()
                .map(|f| BiosFileYaml {
                    name: f.name.clone(),
                    sha1: f.sha1.clone(),
                    description: f.description.clone(),
                    optional: false,
                })
                .collect(),
        };
        Some(serde_yaml::to_string(&bios)?)
    } else {
        None
    };

    // Games YAML — only when docs/cores/<id>/games-info.md exists.
    let games_yaml = load_existing_games_info(inputs.docs_cores_dir, theme.id)?;

    Ok(EmitPlan {
        system_yaml,
        bios_yaml,
        games_yaml,
    })
}

/// Load the L2 System Info Panel block for a system. Tries two
/// sources in priority order:
///
/// 1. `<output_dir>/<id>/system.yaml` — already-migrated source. The
///    panel content lives nested under `system_info:`. Returns that
///    value. Slice 1 (GB / PSX / NDS) deleted the legacy
///    `docs/cores/<id>/system-info.yaml` files; the panel data now
///    lives in the config/systems/ tree only.
/// 2. `docs/cores/<id>/system-info.yaml` — legacy source for the 38
///    Slice 2 targets. Parses as a flat record + returns.
///
/// Returns `None` when neither source exists.
fn load_existing_system_info(
    docs_cores: &Path,
    output_dir: &Path,
    system_id: &str,
) -> Result<Option<serde_yaml::Value>> {
    let migrated = output_dir.join(system_id).join("system.yaml");
    if migrated.is_file() {
        let body = fs::read_to_string(&migrated)
            .with_context(|| format!("read {}", migrated.display()))?;
        let value: serde_yaml::Value = serde_yaml::from_str(&body)
            .with_context(|| format!("parse {}", migrated.display()))?;
        if let Some(panel) = value.get("system_info") {
            if !panel.is_null() {
                return Ok(Some(panel.clone()));
            }
        }
    }
    let legacy = docs_cores.join(system_id).join("system-info.yaml");
    if !legacy.is_file() {
        return Ok(None);
    }
    let body = fs::read_to_string(&legacy)
        .with_context(|| format!("read {}", legacy.display()))?;
    if body.trim().is_empty() {
        return Ok(None);
    }
    let value: serde_yaml::Value = serde_yaml::from_str(&body)
        .with_context(|| format!("parse {}", legacy.display()))?;
    Ok(Some(value))
}

/// Read `docs/cores/<id>/games-info.md` (multi-document YAML wrapped
/// in `.md` for readability) and convert to the new single-document
/// `games.yaml` shape (`{ schema_version, games: [...] }`).
fn load_existing_games_info(
    docs_cores: &Path,
    system_id: &str,
) -> Result<Option<String>> {
    let p = docs_cores.join(system_id).join("games-info.md");
    if !p.is_file() {
        return Ok(None);
    }
    let body = fs::read_to_string(&p)
        .with_context(|| format!("read {}", p.display()))?;
    // Strip leading prose: find the first line that starts with `---`
    // (the first YAML document separator).
    let trimmed = body.trim_start();
    let start = if trimmed.starts_with("---") {
        trimmed
    } else if let Some(pos) = body.find("\n---") {
        &body[pos + 1..]
    } else {
        return Ok(None);
    };
    // Parse each multi-document YAML record.
    let mut records: Vec<serde_yaml::Value> = Vec::new();
    for doc in serde_yaml::Deserializer::from_str(start) {
        let v = serde_yaml::Value::deserialize(doc)
            .with_context(|| format!("parse a record in {}", p.display()))?;
        if !v.is_null() {
            records.push(v);
        }
    }
    if records.is_empty() {
        return Ok(None);
    }
    let games = GamesYaml {
        schema_version: 1,
        games: records,
    };
    Ok(Some(serde_yaml::to_string(&games)?))
}

fn emit_one(plan: &EmitPlan, system_dir: &Path) -> Result<()> {
    fs::create_dir_all(system_dir)
        .with_context(|| format!("mkdir {}", system_dir.display()))?;
    write_atomic(&system_dir.join("system.yaml"), &plan.system_yaml)?;
    if let Some(b) = &plan.bios_yaml {
        write_atomic(&system_dir.join("bios.yaml"), b)?;
    }
    if let Some(g) = &plan.games_yaml {
        write_atomic(&system_dir.join("games.yaml"), g)?;
    }
    Ok(())
}

fn write_atomic(dest: &Path, content: &str) -> Result<()> {
    let tmp = dest.with_extension("yaml.partial");
    fs::write(&tmp, content).with_context(|| format!("write {}", tmp.display()))?;
    if dest.exists() {
        fs::remove_file(dest)
            .with_context(|| format!("remove existing {}", dest.display()))?;
    }
    fs::rename(&tmp, dest)
        .with_context(|| format!("rename {} → {}", tmp.display(), dest.display()))?;
    Ok(())
}

fn print_one(plan: &EmitPlan, system_dir: &Path) {
    println!("=== would write {} ===", system_dir.display());
    println!("--- system.yaml ---");
    println!("{}", plan.system_yaml);
    if let Some(b) = &plan.bios_yaml {
        println!("--- bios.yaml ---");
        println!("{}", b);
    }
    if let Some(g) = &plan.games_yaml {
        println!("--- games.yaml ---");
        println!("{}", g);
    }
}

fn check_one(plan: &EmitPlan, system_dir: &Path) -> Result<usize> {
    let mut drift = 0;
    drift += check_file(system_dir, "system.yaml", Some(&plan.system_yaml))?;
    drift += check_file(system_dir, "bios.yaml", plan.bios_yaml.as_deref())?;
    drift += check_file(system_dir, "games.yaml", plan.games_yaml.as_deref())?;
    Ok(drift)
}

fn check_file(system_dir: &Path, filename: &str, want: Option<&str>) -> Result<usize> {
    let p = system_dir.join(filename);
    let have = if p.is_file() {
        Some(fs::read_to_string(&p)?)
    } else {
        None
    };
    match (have.as_deref(), want) {
        (None, None) => Ok(0),
        (Some(a), Some(b)) if a == b => Ok(0),
        (Some(a), Some(b)) => {
            eprintln!(
                "DRIFT: {} differs from migrator output\n--- have ({} bytes) ---\n{}\n--- want ({} bytes) ---\n{}",
                p.display(),
                a.len(),
                a,
                b.len(),
                b,
            );
            Ok(1)
        }
        (Some(_), None) => {
            eprintln!(
                "DRIFT: {} exists on disk but migrator wouldn't emit it",
                p.display()
            );
            Ok(1)
        }
        (None, Some(_)) => {
            eprintln!(
                "DRIFT: migrator would emit {} but file is missing",
                p.display()
            );
            Ok(1)
        }
    }
}
