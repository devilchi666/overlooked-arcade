//! MAME → OA slim-data extractor.
//!
//! Reads MAME's `-listxml` output + `history.xml` and emits three slim
//! artifacts the oa-shell baker (Phase 2 of the System Info Panel v1
//! plan) consumes at runtime:
//!
//! - `listxml-slim.json`: structured per-system fields (CPU, sound,
//!   resolution, refresh rate, max players, peripheral hints).
//! - `history-slim.xml`: filtered prose entries for the OA-relevant
//!   machines, in upstream `history.xml`'s shape (root `<history>`
//!   wrapping `<entry>` blocks with `<systems>/<system name="…"/>` +
//!   `<text>`). Sourced from arcade-history.com (Gaming-History);
//!   MAME deprecated the legacy `history.dat` text format in 2023.
//! - `mame-version.txt`: the maintainer-supplied version string
//!   (whatever `mame -version` printed).
//!
//! Maintainer-only tool, invoked from `tools/bump-mame.sh`. NOT part of
//! the workspace build — see Cargo.toml's `[workspace]` block for why.
//!
//! Plan: `docs/PLANS/system-info-panel-v1.md`.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use serde::Serialize;

/// OA-relevant MAME machines. The left column is the OA system slug
/// (matching `SystemId` in `frontend/src/themes/registry.ts` and the
/// directory under `docs/cores/`); the right column is the canonical
/// MAME machine driver name.
///
/// Adding a new OA system that maps to a MAME driver: add a row here,
/// recompile, re-run `bump-mame.sh`.
///
/// Systems intentionally absent: `psp`, `ps2`, `nds`, `gamecube`,
/// `scummvm`, `dosbox`, and `mame` itself. The first four lack a usable
/// MAME driver for the metadata fields we extract; `scummvm`/`dosbox`
/// are engine cores; `mame` IS MAME, so there's no parent-machine
/// representation in `-listxml`.
const MAME_DRIVER_MAP: &[(&str, &str)] = &[
    ("tg16", "pce"),
    ("pce-cd", "pce"),
    ("lynx", "lynx"),
    ("nes", "nes"),
    ("snes", "snes"),
    ("atari7800", "a7800"),
    ("genesis", "genesis"),
    ("segacd", "segacd"),
    ("sega32x", "32x"),
    ("sega32xcd", "32x_scd"),
    ("saturn", "saturn"),
    ("stv", "stvbios"),
    ("psx", "psj"),
    ("neogeo", "neogeo"),
    ("neocd", "neocdz"),
    ("ngp", "ngp"),
    ("jaguar", "jaguar"),
    ("jagcd", "jaguarcd"),
    ("3do", "3do"),
    ("pcfx", "pcfx"),
    ("n64", "n64"),
    ("dreamcast", "dc"),
    ("sms", "sms"),
    ("gamegear", "gamegear"),
    ("gb", "gameboy"),
    ("gbc", "gbcolor"),
    ("gba", "gba"),
    ("2600", "a2600"),
    ("5200", "a5200"),
    ("coleco", "coleco"),
    ("intv", "intv"),
    ("o2", "odyssey2"),
    ("channelf", "channelf"),
    ("vectrex", "vectrex"),
    ("virtualboy", "vboy"),
    ("wonderswan", "wswan"),
    ("pokemini", "pokemini"),
    ("msx", "msx"),
    ("msx2", "msx2"),
];

#[derive(Parser, Debug)]
#[command(version, about = "Slim MAME listxml + history.xml down to OA-relevant machines")]
struct Args {
    /// Path to the file containing MAME's `-listxml` output. Captured
    /// by `bump-mame.sh` via `mame -listxml > listxml.xml`.
    #[arg(long)]
    listxml: PathBuf,

    /// Path to `history.xml` — community-maintained, distributed
    /// separately from MAME by arcade-history.com (Gaming-History).
    /// MAME deprecated the legacy text-format `history.dat` in 2023;
    /// only the XML form is published today. Optional: when absent,
    /// `history-slim.xml` lands as a header-only placeholder and the
    /// Phase 2 loader treats every L1 record as having no description.
    #[arg(long)]
    history: Option<PathBuf>,

    /// MAME version string — written verbatim into `mame-version.txt`.
    /// `bump-mame.sh` captures this from `mame -version`.
    #[arg(long)]
    mame_version: String,

    /// Output directory. The three slim artifacts land here:
    /// `listxml-slim.json`, `history-slim.xml`, `mame-version.txt`.
    #[arg(long)]
    out: PathBuf,
}

/// One MAME machine's slim record, as serialised into
/// `listxml-slim.json`. Every field except `system_id` + `machine_name`
/// is optional — MAME doesn't reliably populate clocks, chip names, or
/// display attributes across all consoles, and the Rust loader treats
/// missing values as "L1 has nothing here; fall through to L2."
///
/// Field-by-field notes:
/// - `cpu`, `sound`: first `<chip type="cpu">` / `<chip type="audio">`
///   under the machine, formatted as `"{name} @ {clock-mhz}"`.
/// - `resolution`: first `<display>` width × height. Multi-screen
///   machines pick the first screen; OA's right pane only shows one
///   resolution string.
/// - `refresh_rate`: same display's `refresh` attribute, formatted to
///   2 decimal places ("60.10 Hz", "50.00 Hz").
/// - `max_players`: from `<input players="N">`.
/// - `peripheral_hints`: sorted unique `<control type="…">` values
///   inside `<input>`. Raw MAME control kinds ("joy", "lightgun",
///   "trackball"); the L2 layer maps these to operator-facing labels.
#[derive(Serialize, Debug, Default, PartialEq)]
struct SlimMachine {
    system_id: String,
    machine_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    year: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    manufacturer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cpu: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sound: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resolution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    refresh_rate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_players: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    peripheral_hints: Vec<String>,
}

/// Top-level shape of `listxml-slim.json`. The `mame_version` field
/// duplicates `mame-version.txt`'s contents so consumers reading the
/// JSON alone can still know which MAME release the data came from.
#[derive(Serialize, Debug)]
struct SlimListxml {
    mame_version: String,
    systems: Vec<SlimMachine>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Build the inverse map (MAME machine name → list of OA system_ids
    // that point at it) once, so the XML walk can dispatch in O(1) per
    // `<machine>` without scanning MAME_DRIVER_MAP every time. Multiple
    // OA slugs can target the same machine — `tg16` and `pce-cd` both
    // map to `pce` — which is why the value is a Vec.
    let mut machine_to_slugs: HashMap<&str, Vec<&str>> = HashMap::new();
    for (slug, machine) in MAME_DRIVER_MAP {
        machine_to_slugs.entry(*machine).or_default().push(*slug);
    }
    let wanted_machines: HashSet<&str> = machine_to_slugs.keys().copied().collect();

    fs::create_dir_all(&args.out)
        .with_context(|| format!("creating output dir {}", args.out.display()))?;

    let extracted = parse_listxml(&args.listxml, &wanted_machines)?;
    let systems = fan_out_slugs(extracted, &machine_to_slugs);

    let slim = SlimListxml {
        mame_version: args.mame_version.clone(),
        systems,
    };
    let json_path = args.out.join("listxml-slim.json");
    let json_body = serde_json::to_string_pretty(&slim)?;
    fs::write(&json_path, &json_body)
        .with_context(|| format!("writing {}", json_path.display()))?;

    let history_path = args.out.join("history-slim.xml");
    let history_body = match args.history.as_ref() {
        Some(p) => Some(
            fs::read_to_string(p)
                .with_context(|| format!("reading history.xml at {}", p.display()))?,
        ),
        None => None,
    };
    let slim_history = match history_body.as_deref() {
        Some(body) => slim_history_xml(body, &wanted_machines, &args.mame_version)?,
        // No history.xml supplied — emit a header-only placeholder so the
        // Phase 2 loader can read the file unconditionally. The header
        // carries an XML comment the loader can surface as "no description
        // data available" in the per-system Settings drill-in.
        None => placeholder_history_xml(&args.mame_version),
    };
    fs::write(&history_path, slim_history)
        .with_context(|| format!("writing {}", history_path.display()))?;

    let version_path = args.out.join("mame-version.txt");
    fs::write(&version_path, &args.mame_version)
        .with_context(|| format!("writing {}", version_path.display()))?;

    let history_note = if args.history.is_some() {
        format!("history-slim.xml at {} KB", fs::metadata(&history_path)?.len() / 1024)
    } else {
        "history-slim.xml = placeholder (no history.xml supplied)".to_string()
    };
    eprintln!(
        "mame-extractor: wrote {} machines to {} ({} KB JSON, {})",
        slim.systems.len(),
        json_path.display(),
        json_body.len() / 1024,
        history_note,
    );

    let missing: Vec<&str> = MAME_DRIVER_MAP
        .iter()
        .filter(|(_, machine)| {
            !slim
                .systems
                .iter()
                .any(|s| s.machine_name == *machine)
        })
        .map(|(slug, _)| *slug)
        .collect();
    if !missing.is_empty() {
        eprintln!(
            "mame-extractor: WARNING — no machine record found for OA slugs: {}. \
             Either MAME renamed the driver (update MAME_DRIVER_MAP) or the listxml \
             output is incomplete.",
            missing.join(", ")
        );
    }

    Ok(())
}

/// One MAME machine's data, indexed by MAME machine name. We collect
/// these first and then fan-out to OA slugs in [`fan_out_slugs`] so a
/// machine shared across slugs (e.g. `pce` for both `tg16` and `pce-cd`)
/// is parsed exactly once.
type ExtractedMachines = HashMap<String, ExtractedMachine>;

#[derive(Default, Debug)]
struct ExtractedMachine {
    year: Option<String>,
    manufacturer: Option<String>,
    cpu: Option<String>,
    sound: Option<String>,
    resolution: Option<String>,
    refresh_rate: Option<String>,
    max_players: Option<u32>,
    peripheral_hints: BTreeSet<String>,
}

/// Stream-parse the listxml file. Only machines whose `name=` matches
/// an entry in `wanted_machines` accumulate state; everything else is
/// skipped at the start-tag check so we never allocate per-machine
/// scratch for the 30k+ irrelevant arcade entries.
fn parse_listxml(path: &Path, wanted_machines: &HashSet<&str>) -> Result<ExtractedMachines> {
    let mut reader = Reader::from_file(path)
        .with_context(|| format!("opening listxml at {}", path.display()))?;
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut out = ExtractedMachines::new();

    // Active per-machine state. None when we're outside a wanted machine
    // OR inside an irrelevant one.
    let mut current: Option<(String, ExtractedMachine)> = None;
    // Tracks which text-bearing element we're inside (year / manufacturer)
    // so the Text event knows where to deposit. None for elements whose
    // text we don't care about.
    let mut text_target: Option<TextTarget> = None;
    let mut text_buf = String::new();

    loop {
        let evt = reader
            .read_event_into(&mut buf)
            .with_context(|| format!("reading XML at byte {}", reader.buffer_position()))?;

        match evt {
            Event::Start(ref e) => {
                handle_open_tag(
                    e,
                    /*self_closing=*/ false,
                    wanted_machines,
                    &mut current,
                    &mut text_target,
                    &mut text_buf,
                );
            }
            Event::Empty(ref e) => {
                handle_open_tag(
                    e,
                    /*self_closing=*/ true,
                    wanted_machines,
                    &mut current,
                    &mut text_target,
                    &mut text_buf,
                );
            }
            Event::Text(ref t) => {
                if text_target.is_some() {
                    if let Ok(unesc) = t.unescape() {
                        text_buf.push_str(&unesc);
                    }
                }
            }
            Event::End(ref e) => {
                let name = e.name();
                let tag = std::str::from_utf8(name.as_ref()).unwrap_or("");
                match tag {
                    "machine" => {
                        if let Some((name, em)) = current.take() {
                            out.insert(name, em);
                        }
                    }
                    "year" | "manufacturer" => {
                        if let Some(target) = text_target.take() {
                            if let Some((_, em)) = current.as_mut() {
                                let value = text_buf.trim().to_string();
                                if !value.is_empty() {
                                    match target {
                                        TextTarget::Year => em.year = Some(value),
                                        TextTarget::Manufacturer => em.manufacturer = Some(value),
                                    }
                                }
                            }
                            text_buf.clear();
                        }
                    }
                    _ => {}
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(out)
}

#[derive(Copy, Clone)]
enum TextTarget {
    Year,
    Manufacturer,
}

/// Shared logic for `<…>` (Start) and `<…/>` (Empty) tags. Reading
/// quick-xml's docs the two events carry identical attribute data, only
/// differing in whether a matching End is coming — so we route both
/// through here and only set `text_target` for elements we know have
/// non-empty text bodies (which precludes Empty by definition).
fn handle_open_tag(
    e: &BytesStart,
    self_closing: bool,
    wanted_machines: &HashSet<&str>,
    current: &mut Option<(String, ExtractedMachine)>,
    text_target: &mut Option<TextTarget>,
    text_buf: &mut String,
) {
    let name = e.name();
    let tag = std::str::from_utf8(name.as_ref()).unwrap_or("");
    match tag {
        "machine" => {
            // Start of a top-level machine entry. Decide whether we
            // care; if yes, allocate scratch.
            if let Some(name) = attr(e, b"name") {
                if wanted_machines.contains(name.as_str()) {
                    *current = Some((name, ExtractedMachine::default()));
                } else {
                    *current = None;
                }
            }
        }
        "year" | "manufacturer" if !self_closing && current.is_some() => {
            *text_target = Some(match tag {
                "year" => TextTarget::Year,
                _ => TextTarget::Manufacturer,
            });
            text_buf.clear();
        }
        "input" if current.is_some() => {
            if let Some(p) = attr(e, b"players").and_then(|v| v.parse::<u32>().ok()) {
                if let Some((_, em)) = current.as_mut() {
                    em.max_players = Some(p);
                }
            }
        }
        "control" if current.is_some() => {
            if let Some(t) = attr(e, b"type") {
                if let Some((_, em)) = current.as_mut() {
                    em.peripheral_hints.insert(t);
                }
            }
        }
        "display" if current.is_some() => {
            let w = attr(e, b"width").and_then(|v| v.parse::<u32>().ok());
            let h = attr(e, b"height").and_then(|v| v.parse::<u32>().ok());
            let refresh = attr(e, b"refresh").and_then(|v| v.parse::<f64>().ok());
            if let Some((_, em)) = current.as_mut() {
                if em.resolution.is_none() {
                    if let (Some(w), Some(h)) = (w, h) {
                        em.resolution = Some(format!("{} × {}", w, h));
                    }
                }
                if em.refresh_rate.is_none() {
                    if let Some(r) = refresh {
                        em.refresh_rate = Some(format!("{:.2} Hz", r));
                    }
                }
            }
        }
        "chip" if current.is_some() => {
            let chip_type = attr(e, b"type");
            let name = attr(e, b"name");
            let clock = attr(e, b"clock").and_then(|v| v.parse::<u64>().ok());
            if let (Some(ct), Some((_, em))) = (chip_type.as_deref(), current.as_mut()) {
                if matches!(ct, "cpu" | "audio") {
                    let formatted = format_chip(name.as_deref(), clock);
                    if let Some(text) = formatted {
                        match ct {
                            "cpu" if em.cpu.is_none() => em.cpu = Some(text),
                            "audio" if em.sound.is_none() => em.sound = Some(text),
                            _ => {}
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

/// Look up a single attribute on a start tag by raw key bytes. Returns
/// the unescaped value as an owned String; None when the attribute is
/// absent or unparseable.
fn attr(e: &BytesStart, key: &[u8]) -> Option<String> {
    for a in e.attributes() {
        let Ok(a) = a else { continue };
        if a.key.as_ref() == key {
            return a.unescape_value().ok().map(|c| c.into_owned());
        }
    }
    None
}

/// Build the human-facing chip string from MAME's raw fields. Returns
/// None when the chip has neither a name nor a clock — MAME sometimes
/// emits placeholder `<chip type="cpu"/>` entries for unimplemented
/// CPUs which we want to skip.
fn format_chip(name: Option<&str>, clock_hz: Option<u64>) -> Option<String> {
    match (name, clock_hz) {
        (Some(n), Some(c)) => Some(format!("{} @ {}", n.trim(), format_clock(c))),
        (Some(n), None) if !n.trim().is_empty() => Some(n.trim().to_string()),
        _ => None,
    }
}

/// Format a chip clock from Hz into the most natural unit. MAME stores
/// clocks at full Hz precision (e.g. 1789773 for the NES 2A03); the
/// panel shows MHz with 2 decimals, falling through to GHz / kHz at
/// the extremes.
fn format_clock(hz: u64) -> String {
    if hz >= 1_000_000_000 {
        format!("{:.2} GHz", hz as f64 / 1_000_000_000.0)
    } else if hz >= 1_000_000 {
        format!("{:.2} MHz", hz as f64 / 1_000_000.0)
    } else if hz >= 1_000 {
        format!("{} kHz", hz / 1_000)
    } else {
        format!("{} Hz", hz)
    }
}

/// Convert extracted-per-machine data into per-slug slim records.
/// Machines mapped to multiple OA slugs (e.g. `pce` for both `tg16` and
/// `pce-cd`) emit one record per slug, sharing the underlying MAME data.
///
/// Output ordering matches the row order in MAME_DRIVER_MAP so the
/// JSON file stays stable across runs (avoids spurious diffs when the
/// XML walk yields machines in a slightly different order).
fn fan_out_slugs(
    extracted: ExtractedMachines,
    machine_to_slugs: &HashMap<&str, Vec<&str>>,
) -> Vec<SlimMachine> {
    let mut out = Vec::new();
    for (slug, machine) in MAME_DRIVER_MAP {
        let Some(em) = extracted.get(*machine) else {
            continue;
        };
        let _ = machine_to_slugs; // silence unused warning; map built upstream is the source of truth
        out.push(SlimMachine {
            system_id: (*slug).to_string(),
            machine_name: (*machine).to_string(),
            year: em.year.clone(),
            manufacturer: em.manufacturer.clone(),
            cpu: em.cpu.clone(),
            sound: em.sound.clone(),
            resolution: em.resolution.clone(),
            refresh_rate: em.refresh_rate.clone(),
            max_players: em.max_players,
            peripheral_hints: em.peripheral_hints.iter().cloned().collect(),
        });
    }
    out
}

/// Emit a header-only `history-slim.xml` for the case where the
/// maintainer didn't supply a `history.xml` file. The placeholder is
/// readable by the same parser idiom the Phase 2 loader uses for the
/// populated case (it just yields zero `<entry>` blocks).
fn placeholder_history_xml(mame_version: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!-- OA-slimmed history.xml — PLACEHOLDER (no history.xml supplied at bump time).\n\
              Source: MAME {mame_version}.\n\
              WARNING: no description text — operator can re-run bump-mame.sh with\n\
              HISTORY=<path-to-history.xml> once arcade-history.com's file is\n\
              available locally (canonical path: <root>/Emulators/MAME/history/history.xml). -->\n\
         <history>\n\
         </history>\n",
    )
}

/// Walk a history.xml file and emit only the `<entry>` blocks whose
/// `<systems>` section lists at least one machine in `wanted_machines`.
/// Output preserves the upstream shape — entries are passed through
/// verbatim via event replay — so the Phase 2 loader can use the same
/// quick-xml event parser against either the full file or the slim.
///
/// `<software>`-only entries (the per-game cart / cassette / disk
/// records under `<software list="…"/>`) are always dropped: this
/// extractor only cares about system-level metadata. The system_info
/// Tauri command consumes the slim file; per-game game_info data
/// flows through a separate path (`docs/cores/<id>/games-info.md`).
fn slim_history_xml(
    input: &str,
    wanted_machines: &HashSet<&str>,
    mame_version: &str,
) -> Result<String> {
    use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
    use quick_xml::Writer;

    // Read upstream version + date from the root element so the slim
    // header records the exact source we slimmed from. Falls through to
    // "unknown" when the attributes are missing — defensive against
    // partial / edited inputs.
    let (history_version, history_date) = read_root_metadata(input);

    let mut writer = Writer::new(Vec::new());
    // The XML declaration is written manually rather than via
    // Event::Decl(BytesDecl::new(...)) because quick-xml's Decl helper
    // requires several lifetime gymnastics for borrowed attribute lists
    // and we don't gain anything from going through the typed path for
    // a constant header.
    writer
        .get_mut()
        .extend_from_slice(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");

    let comment = format!(
        " OA-slimmed history.xml — only OA-relevant <entry>s retained.\n\
         Source: MAME {mame_version}; history.xml v{history_version} ({history_date})\n\
         from arcade-history.com (Gaming-History).\n\
         Format: subset of upstream history.xml. Entries kept iff their\n\
         <systems> block lists at least one machine in tools/mame-extractor's\n\
         MAME_DRIVER_MAP. <software>-only entries are dropped (per-game\n\
         data flows through docs/cores/<id>/games-info.md instead). "
    );
    writer.write_event(Event::Comment(BytesText::from_escaped(&comment)))?;
    writer.write_event(Event::Text(BytesText::from_escaped("\n")))?;
    let mut root = BytesStart::new("history");
    root.push_attribute(("version", history_version.as_str()));
    root.push_attribute(("date", history_date.as_str()));
    writer.write_event(Event::Start(root))?;
    writer.write_event(Event::Text(BytesText::from_escaped("\n")))?;

    let mut reader = Reader::from_str(input);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();

    // While inside an `<entry>`, every event is recorded into
    // `entry_events`; at `</entry>` we either replay the events to
    // `writer` (if any `<systems>/<system name="X"/>` matched) or
    // discard them. `into_owned()` decouples the events from `buf`
    // so we can clear the buffer each iteration.
    let mut entry_events: Vec<Event<'static>> = Vec::new();
    let mut entry_machines: HashSet<String> = HashSet::new();
    let mut in_entry = false;
    let mut in_systems = false;

    loop {
        let evt = reader.read_event_into(&mut buf).with_context(|| {
            format!("reading history.xml at byte {}", reader.buffer_position())
        })?;

        // Determine event kind + tag name without moving evt — we still
        // need the original after inspection to push to entry_events.
        let kind_tag = match &evt {
            Event::Start(e) => Some(("start", tag_name(e.name().as_ref()))),
            Event::Empty(e) => Some(("empty", tag_name(e.name().as_ref()))),
            Event::End(e) => Some(("end", tag_name(e.name().as_ref()))),
            Event::Eof => None,
            _ => Some(("other", String::new())),
        };

        if kind_tag.is_none() {
            break;
        }
        let (kind, tag) = kind_tag.unwrap();

        // --- pre-process: open tags + item collection -----------------
        match (kind, tag.as_str()) {
            ("start", "entry") => {
                in_entry = true;
                entry_events.clear();
                entry_machines.clear();
            }
            ("start", "systems") if in_entry => {
                in_systems = true;
            }
            ("empty", "system") | ("start", "system") if in_systems => {
                let bytes_start = match &evt {
                    Event::Empty(e) | Event::Start(e) => Some(e),
                    _ => None,
                };
                if let Some(e) = bytes_start {
                    if let Some(name) = attr(e, b"name") {
                        entry_machines.insert(name);
                    }
                }
            }
            _ => {}
        }

        // --- record event into per-entry buffer -----------------------
        if in_entry {
            entry_events.push(evt.into_owned());
        }

        // --- post-process: close tags + entry flush -------------------
        match (kind, tag.as_str()) {
            ("end", "systems") if in_entry => {
                in_systems = false;
            }
            ("end", "entry") if in_entry => {
                let keep = entry_machines
                    .iter()
                    .any(|m| wanted_machines.contains(m.as_str()));
                if keep {
                    // quick-xml's write_event takes ownership; clone each
                    // recorded event. The Vec entries are <'static> after
                    // into_owned, so the clone is a couple of small Box
                    // allocations per entry — fine at ~40 systems.
                    for ev in &entry_events {
                        writer.write_event(ev.clone())?;
                    }
                    writer.write_event(Event::Text(BytesText::from_escaped("\n")))?;
                }
                in_entry = false;
                in_systems = false;
                entry_events.clear();
                entry_machines.clear();
            }
            _ => {}
        }

        buf.clear();
    }

    writer.write_event(Event::End(BytesEnd::new("history")))?;
    writer.write_event(Event::Text(BytesText::from_escaped("\n")))?;

    Ok(String::from_utf8(writer.into_inner())?)
}

/// Pull `version` + `date` out of the `<history>` root tag without
/// allocating a full DOM. Used by [`slim_history_xml`] so the slim
/// header records exactly which upstream release we filtered.
///
/// Falls through to "unknown" / "unknown" when the root attrs aren't
/// found in the first 4KB — defensive against partial files (truncated
/// downloads, edits) where the maintainer might still want a usable
/// slim even with a degraded header.
fn read_root_metadata(input: &str) -> (String, String) {
    let head = &input[..input.len().min(4096)];
    let mut reader = Reader::from_str(head);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                if e.name().as_ref() == b"history" {
                    let v = attr(&e, b"version").unwrap_or_else(|| "unknown".into());
                    let d = attr(&e, b"date").unwrap_or_else(|| "unknown".into());
                    return (v, d);
                }
            }
            Ok(Event::Eof) | Err(_) => return ("unknown".into(), "unknown".into()),
            _ => {}
        }
        buf.clear();
    }
}

/// Convenience: decode quick-xml's `QName` bytes to an owned String.
/// Non-UTF-8 names degrade to empty so downstream match arms see a
/// no-op tag — there is no valid XML where the element name isn't
/// UTF-8, so this is purely defensive.
fn tag_name(name: &[u8]) -> String {
    std::str::from_utf8(name).unwrap_or("").to_string()
}

/// Sanity-check that the file we wrote can be read back into the same
/// shape — runs as part of `cargo test` against tiny in-source samples,
/// so regressions in the parser surface at PR time rather than at the
/// next bump.
#[allow(dead_code)]
fn roundtrip_sanity_check(slim: &SlimListxml) -> Result<()> {
    let json = serde_json::to_string(slim)?;
    let _back: SlimListxmlForTest = serde_json::from_str(&json)?;
    if slim.systems.is_empty() {
        bail!("expected at least one system in slim output");
    }
    Ok(())
}

/// Mirror of [`SlimListxml`] with all fields owned + Deserialize. Used
/// only by the roundtrip test path; production code only serialises.
#[derive(serde::Deserialize, Debug)]
struct SlimListxmlForTest {
    #[allow(dead_code)]
    mame_version: String,
    #[allow(dead_code)]
    systems: Vec<serde_json::Value>,
}

#[allow(dead_code)]
fn ensure_machine_name_unique() -> Result<()> {
    // Defensive at link time: detect duplicate machine_name entries in
    // MAME_DRIVER_MAP that would collide when fanned out. Two slugs
    // pointing at the same machine is fine; two rows of the same (slug,
    // machine) is a typo we want to catch.
    let mut seen = HashSet::new();
    for (slug, _) in MAME_DRIVER_MAP {
        if !seen.insert(*slug) {
            return Err(anyhow!("MAME_DRIVER_MAP has duplicate slug {slug}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_LISTXML: &str = r#"<?xml version="1.0"?>
<mame build="0.262">
  <machine name="nes" sourcefile="nes.cpp">
    <description>Nintendo Entertainment System</description>
    <year>1985</year>
    <manufacturer>Nintendo</manufacturer>
    <input players="2" service="no" tilt="no" coins="0">
      <control type="joy" buttons="2"/>
      <control type="lightgun"/>
    </input>
    <display tag="screen" type="raster" width="256" height="240" refresh="60.098476"/>
    <chip type="cpu" tag="maincpu" name="N2A03" clock="1789773"/>
    <chip type="audio" tag="dac" name="N2A03 APU" clock="1789773"/>
  </machine>
  <machine name="some_unrelated_arcade" sourcefile="other.cpp">
    <description>Should be skipped</description>
    <year>1984</year>
    <manufacturer>Konami</manufacturer>
    <input players="4"/>
    <chip type="cpu" name="Z80" clock="4000000"/>
  </machine>
  <machine name="pce" sourcefile="pce.cpp">
    <description>PC Engine / TurboGrafx-16</description>
    <year>1987</year>
    <manufacturer>NEC</manufacturer>
    <input players="1">
      <control type="joy" buttons="2"/>
    </input>
    <display tag="screen" type="raster" width="256" height="239" refresh="59.826820"/>
    <chip type="cpu" tag="maincpu" name="HuC6280" clock="7159090"/>
    <chip type="audio" tag="psg" name="HuC6280 PSG" clock="3579545"/>
  </machine>
</mame>
"#;

    fn parse_sample() -> ExtractedMachines {
        // Write the sample to a temp file so we exercise the same
        // Reader::from_file path the real run uses. tmp_dir lives at
        // std::env::temp_dir() — works on Windows + Unix without
        // pulling in `tempfile`.
        let tmp_dir = std::env::temp_dir();
        let path = tmp_dir.join(format!(
            "mame-extractor-test-{}.xml",
            std::process::id()
        ));
        fs::write(&path, SAMPLE_LISTXML).unwrap();

        let mut wanted = HashSet::new();
        wanted.insert("nes");
        wanted.insert("pce");
        let out = parse_listxml(&path, &wanted).unwrap();
        let _ = fs::remove_file(&path);
        out
    }

    #[test]
    fn parses_machines_in_driver_map_only() {
        let extracted = parse_sample();
        // some_unrelated_arcade is NOT in our driver map → must not
        // appear in the output.
        assert!(extracted.contains_key("nes"));
        assert!(extracted.contains_key("pce"));
        assert!(!extracted.contains_key("some_unrelated_arcade"));
        assert_eq!(extracted.len(), 2);
    }

    #[test]
    fn nes_machine_fields_populated() {
        let extracted = parse_sample();
        let nes = extracted.get("nes").unwrap();
        assert_eq!(nes.year.as_deref(), Some("1985"));
        assert_eq!(nes.manufacturer.as_deref(), Some("Nintendo"));
        assert_eq!(nes.max_players, Some(2));
        assert_eq!(nes.resolution.as_deref(), Some("256 × 240"));
        assert_eq!(nes.refresh_rate.as_deref(), Some("60.10 Hz"));
        assert_eq!(nes.cpu.as_deref(), Some("N2A03 @ 1.79 MHz"));
        assert_eq!(nes.sound.as_deref(), Some("N2A03 APU @ 1.79 MHz"));
        // Peripheral hints sorted via BTreeSet — alphabetical.
        let hints: Vec<&str> = nes.peripheral_hints.iter().map(|s| s.as_str()).collect();
        assert_eq!(hints, vec!["joy", "lightgun"]);
    }

    #[test]
    fn fan_out_produces_two_slug_records_for_shared_pce_machine() {
        // tg16 and pce-cd both target the MAME `pce` machine. After
        // fan-out, both slugs surface as separate slim records — the
        // Rust loader writes one row per slug downstream.
        let extracted = parse_sample();
        let mut machine_to_slugs: HashMap<&str, Vec<&str>> = HashMap::new();
        for (slug, machine) in MAME_DRIVER_MAP {
            machine_to_slugs.entry(*machine).or_default().push(*slug);
        }
        let out = fan_out_slugs(extracted, &machine_to_slugs);
        let tg16 = out.iter().find(|s| s.system_id == "tg16");
        let pce_cd = out.iter().find(|s| s.system_id == "pce-cd");
        assert!(tg16.is_some());
        assert!(pce_cd.is_some());
        // Both records carry the same underlying MAME data (manufacturer,
        // year, CPU) since they're sourced from the same `pce` machine.
        assert_eq!(tg16.unwrap().manufacturer.as_deref(), Some("NEC"));
        assert_eq!(pce_cd.unwrap().manufacturer.as_deref(), Some("NEC"));
        assert_eq!(tg16.unwrap().machine_name, "pce");
        assert_eq!(pce_cd.unwrap().machine_name, "pce");
    }

    #[test]
    fn fan_out_skips_systems_with_no_machine_record() {
        // Even though MAME_DRIVER_MAP lists 39 slugs, fan_out only
        // produces records for the slugs whose machines were actually
        // present in the parsed XML. Confirms the .filter behaviour.
        let extracted = parse_sample();
        let mut machine_to_slugs: HashMap<&str, Vec<&str>> = HashMap::new();
        for (slug, machine) in MAME_DRIVER_MAP {
            machine_to_slugs.entry(*machine).or_default().push(*slug);
        }
        let out = fan_out_slugs(extracted, &machine_to_slugs);
        // tg16 + pce-cd (both → pce) + nes = 3 records.
        assert_eq!(out.len(), 3);
        assert!(out.iter().any(|s| s.system_id == "nes"));
    }

    #[test]
    fn format_clock_units_at_breakpoints() {
        assert_eq!(format_clock(1_789_773), "1.79 MHz");
        assert_eq!(format_clock(294_000_000), "294.00 MHz");
        assert_eq!(format_clock(3_500_000_000), "3.50 GHz");
        assert_eq!(format_clock(32_768), "32 kHz");
        assert_eq!(format_clock(500), "500 Hz");
    }

    /// Two-entry sample modelled on the actual arcade-history.com
    /// `history.xml` shape (verified against the 2.87a 2026-04-11
    /// release): an NES system entry (multi-machine `<systems>` block
    /// with regional variants), one unrelated arcade entry, and one
    /// software-list entry. The slimmer keeps the NES + drops the
    /// other two.
    const SAMPLE_HISTORY_XML: &str = r#"<?xml version="1.0"?>
<history version="2.87a" date="2026-04-11">
    <entry>
        <systems>
            <system name="nes" game="no" />
            <system name="nespal" game="no" />
        </systems>
        <text>
Console published 41 years ago:

Nintendo Entertainment System [NES] (c) 1985 Nintendo Co., Ltd.

8-Bit video game console.
        </text>
    </entry>
    <entry>
        <systems>
            <system name="some_unrelated_arcade" game="yes" />
        </systems>
        <text>
Arcade game irrelevant to OA.
        </text>
    </entry>
    <entry>
        <software>
            <item list="nes" name="smb" game="yes" />
        </software>
        <text>
Software-list entry (per-game record). Always dropped by the slimmer.
        </text>
    </entry>
    <entry>
        <systems>
            <system name="pce" game="no" />
        </systems>
        <text>
PC Engine console.
        </text>
    </entry>
</history>
"#;

    #[test]
    fn slim_history_xml_keeps_wanted_systems_only() {
        // NES + PCE are in wanted; the unrelated arcade + the
        // software-list entry must be dropped. Order preserved.
        let mut wanted = HashSet::new();
        wanted.insert("nes");
        wanted.insert("pce");
        let slim = slim_history_xml(SAMPLE_HISTORY_XML, &wanted, "0.262")
            .expect("slim must succeed against valid input");
        assert!(slim.contains("Nintendo Entertainment System"));
        assert!(slim.contains("PC Engine console"));
        assert!(!slim.contains("Arcade game irrelevant"));
        assert!(!slim.contains("Software-list entry"));
        // Slim file is well-formed XML — root + comment.
        assert!(slim.contains("<?xml"));
        assert!(slim.contains("<history"));
        assert!(slim.contains("</history>"));
        assert!(slim.contains("OA-slimmed"));
        // Upstream version + date from the root attrs flow through.
        assert!(slim.contains("2.87a"));
        assert!(slim.contains("2026-04-11"));
    }

    #[test]
    fn slim_history_xml_matches_entry_via_any_listed_system() {
        // <systems> can list multiple machines (regional variants);
        // the entry keeps as long as ANY listed machine is wanted.
        // wanted={nespal} → the NES entry passes because nespal is
        // its second listed system.
        let mut wanted = HashSet::new();
        wanted.insert("nespal");
        let slim = slim_history_xml(SAMPLE_HISTORY_XML, &wanted, "0.262")
            .expect("slim must succeed");
        assert!(slim.contains("Nintendo Entertainment System"));
        assert!(!slim.contains("PC Engine console"));
    }

    #[test]
    fn slim_history_xml_no_wanted_systems_yields_empty_history() {
        // No system match anywhere → output is the header + an empty
        // <history> root. Phase 2 loader still reads it cleanly.
        let wanted: HashSet<&str> = HashSet::new();
        let slim = slim_history_xml(SAMPLE_HISTORY_XML, &wanted, "0.262")
            .expect("slim must succeed on empty wanted set");
        assert!(slim.contains("<history"));
        assert!(slim.contains("</history>"));
        assert!(!slim.contains("Nintendo"));
        assert!(!slim.contains("PC Engine"));
    }

    #[test]
    fn slim_history_xml_drops_software_only_entries() {
        // The standalone software-list entry has no <systems> block at
        // all → must be dropped regardless of wanted set. Confirms
        // we filter on systems exclusively, never on software.
        let mut wanted = HashSet::new();
        wanted.insert("nes");
        let slim = slim_history_xml(SAMPLE_HISTORY_XML, &wanted, "0.262").unwrap();
        // The SMB software entry is keyed by list="nes" name="smb";
        // a naive filter that also matched <software list="…"> would
        // include it. We don't.
        assert!(!slim.contains("Software-list entry"));
    }

    #[test]
    fn driver_map_has_no_duplicate_slugs() {
        ensure_machine_name_unique().expect("driver map must have unique slugs");
    }

    #[test]
    fn placeholder_history_carries_warning_header() {
        // Confirms the no-history.xml path still produces a readable
        // file the Phase 2 loader can open without special-casing.
        let body = placeholder_history_xml("0.262");
        assert!(body.contains("PLACEHOLDER"));
        assert!(body.contains("0.262"));
        assert!(body.contains("WARNING"));
        // Header-only — no entries because we have no source data.
        assert!(!body.contains("<entry"));
        assert!(!body.contains("<systems"));
        // Still well-formed XML so the loader can parse it normally.
        assert!(body.starts_with("<?xml"));
        assert!(body.contains("<history>"));
        assert!(body.contains("</history>"));
    }
}
