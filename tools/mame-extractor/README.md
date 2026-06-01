# mame-extractor

Maintainer-only utility that slims MAME's `-listxml` output and
`history.xml` (from arcade-history.com / Gaming-History) down to:
(1) the ~40 machines that map to OA-supported systems (per-system
metadata), and (2) every playable arcade-game machine MAME knows
about (~25-30k records, used for ROM-set name resolution in the
library). Produces the four files in `assets/mame-source/` that
`oa-shell` bakes into SQLite on launch.

Invoked exclusively via `tools/bump-mame.sh`. Standalone Cargo workspace —
NOT a member of OA's root workspace, so `cargo build --workspace` /
`cargo test --workspace` from the repo root do not touch it.

Plan references:
- `docs/PLANS/system-info-panel-v1.md` (per-system slim, originally
  the only artifact).
- `C:/Users/Devilchi/.claude/plans/glittery-kindling-blum.md`
  (mame-games-slim, added 2026-06-01).

## Usage

```bash
# From the repo root.
tools/bump-mame.sh
```

The script auto-detects a local MAME install + a local `history.xml`,
runs `mame -listxml`, builds + invokes this extractor, and writes the
slim artifacts to `assets/mame-source/`. Override MAME / history paths
via the `MAME` / `HISTORY` env vars.

### Where to put MAME

OA's project-wide convention for third-party emulator binaries (the
ones OA shells out to rather than loading as libretro `.dll`s) is a
top-level `Emulators/` directory at the repo / install root. For MAME
specifically:

```
<root>/
  Emulators/
    MAME/
      mame.exe          (or mame on Linux)
      history/
        history.xml     (downloaded separately from arcade-history.com)
      …                 (other MAME files MAME itself needs)
```

`bump-mame.sh` probes `Emulators/MAME/` first; the shipped install
applies the same convention at `<exe_dir>/Emulators/MAME/` when Phase 5
wires the in-app "Refresh MAME system info" button. Other external
emulators (DOSBox-X standalone, ScummVM standalone, anything else OA
might call out to) follow the same shape: `<root>/Emulators/<name>/`.

The `Emulators/` directory is gitignored — drop your install there
without worrying about accidentally committing copyrighted binaries.

## Outputs

### `listxml-slim.json`

Structured per-system fields filtered to OA-relevant MAME machines. One
JSON record per OA system slug (so `tg16` and `pce-cd` each get their
own record, both sourced from MAME's `pce` machine).

```json
{
  "mame_version": "0.262",
  "systems": [
    {
      "system_id": "nes",
      "machine_name": "nes",
      "year": "1985",
      "manufacturer": "Nintendo",
      "cpu": "N2A03 @ 1.79 MHz",
      "sound": "N2A03 APU @ 1.79 MHz",
      "resolution": "256 × 240",
      "refresh_rate": "60.10 Hz",
      "max_players": 2,
      "peripheral_hints": ["joy", "lightgun"]
    }
  ]
}
```

Every field except `system_id` + `machine_name` is optional — MAME
doesn't reliably populate clocks, chip names, or display attributes
across every console driver, and the consumer (`oa-shell`'s
`system_info` module, landed in Phase 2) treats missing values as
"L1 has nothing here; the L2 YAML or L3 operator override fills the
gap."

Field-by-field source:

| Field              | MAME source                                            |
|--------------------|--------------------------------------------------------|
| `system_id`        | The OA slug from `MAME_DRIVER_MAP` in `src/main.rs`    |
| `machine_name`     | The MAME machine driver (`<machine name="…">`)         |
| `year`             | `<year>…</year>` child text                            |
| `manufacturer`     | `<manufacturer>…</manufacturer>` child text            |
| `cpu`              | First `<chip type="cpu">`, formatted `{name} @ {clk}`  |
| `sound`            | First `<chip type="audio">`, formatted similarly       |
| `resolution`       | First `<display width="…" height="…">` (W × H)         |
| `refresh_rate`     | Same display's `refresh="…"`, 2-decimal Hz             |
| `max_players`      | `<input players="…">`                                  |
| `peripheral_hints` | Sorted unique `<control type="…">` under `<input>`     |

### `mame-games-slim.json`

Per-machine arcade-game catalog. One JSON record per machine that
satisfies ALL of:

- `runnable != "no"` (machine is playable, not a placeholder stub),
- `isbios != "yes"` (not a BIOS-only ROM-set like `neogeo`),
- `isdevice != "yes"` (not a CPU / chip device entry),
- has at least one `<rom>` child element,
- has a non-empty `<description>` (every real machine does).

Both parents AND clones emit their own row with their own
`<description>`. The clone's `cloneof` field points at the parent's
machine name; the parent's `cloneof` is omitted. This means an
operator with `sf2ce.zip` gets "Street Fighter II': Champion Edition"
— NOT the parent `sf2`'s "Street Fighter II: The World Warrior".

```json
{
  "mame_version": "0.262",
  "schema_version": 1,
  "machines": [
    {"name": "dkong", "description": "Donkey Kong (US set 1)", "year": "1981", "manufacturer": "Nintendo"},
    {"name": "sf2", "description": "Street Fighter II: The World Warrior (World 910522)", "year": "1991", "manufacturer": "Capcom"},
    {"name": "sf2ce", "description": "Street Fighter II': Champion Edition (World 920313)", "year": "1992", "manufacturer": "Capcom", "cloneof": "sf2"}
  ]
}
```

Field-by-field source:

| Field          | MAME source                                            |
|----------------|--------------------------------------------------------|
| `name`         | `<machine name="…">` attribute (matches `.zip` stem)   |
| `description`  | `<description>…</description>` child text              |
| `year`         | `<year>…</year>` child text (optional in MAME)         |
| `manufacturer` | `<manufacturer>…</manufacturer>` child text            |
| `cloneof`      | `<machine cloneof="…">` attribute (omitted when None)  |

Output is sorted by `name` ascending and **minified** (no
pretty-printing) — at ~25-30k records, pretty-printing balloons the
file to 4-5 MB, and the install-size cost outweighs the human-readability
benefit for an artifact this large. Diff stability across MAME version
bumps comes from the deterministic sort, not from pretty formatting.

`schema_version` lets the consumer detect and reject incompatible
artifacts on future schema bumps (Phase 2 SQLite loader will refuse
to bake an artifact whose `schema_version` exceeds what it understands).

### `history-slim.xml`

Filtered `<entry>` blocks from upstream `history.xml`. Output is a
subset of the upstream shape — wrapped in `<?xml ?><history>…</history>`
with an OA-authored comment header — so the Phase 2 loader can use the
same quick-xml event parser against either the full file or the slim.

Filter rule: an `<entry>` is kept iff its `<systems>` block lists at
least one machine in `MAME_DRIVER_MAP`. The `<software>`-only entries
(per-game cart / cassette / disk records under `<software list="…"/>`)
are always dropped — per-game data flows through
`docs/cores/<id>/games-info.md` instead.

**`history.xml` is optional.** MAME builds don't bundle it; it's
community-maintained by arcade-history.com (Gaming-History). MAME
deprecated the legacy text-format `history.dat` in 2023; only the
XML form is published today. When `bump-mame.sh` can't find a local
copy, `--history` is omitted and the extractor writes a header-only
placeholder `history-slim.xml` with an XML comment carrying a
`WARNING` line. The Phase 2 SQLite loader reads the file
unconditionally — placeholder simply yields zero description rows.
Drop a history.xml at `<root>/Emulators/MAME/history/history.xml` and
re-run bump-mame.sh later to populate descriptions.

**Known L1 description gaps (real, not bugs):**
- `3do` — history.xml only has model-specific entries (`3do_fz1`,
  `3do_gdo101`, etc.), no generic `3do` entry.
- `msx` / `msx2` — history.xml has no MSX system-level entries at all
  (MSX records are software-list-only).

These three systems get L1 records with `listxml-slim.json` fields
populated but no description; the L2 YAML supplies the description
copy. Same recipe DOSBox / ScummVM / PSP / PS2 / NDS / GameCube use
for systems entirely absent from MAME.

### `mame-version.txt`

Plain text with the MAME version string (e.g. `0.262`). Used by the
`oa-shell` startup hash to detect when the bundled data changes
between OA releases and a rebake is needed.

## Driver map

OA system slug → MAME machine name lives as a hardcoded constant
`MAME_DRIVER_MAP` in `src/main.rs`. Adding a new OA system that has a
MAME driver: add the row, recompile, rerun `bump-mame.sh`. Systems
that MAME doesn't cover (PSP / PS2 / NDS / DOSBox / ScummVM, plus
`mame` itself) stay out of the map; their L1 row in the SQLite table
is a stub and all values come from L2 YAML.

Multiple OA slugs are allowed to point at the same MAME machine
(`tg16` + `pce-cd` both → `pce`). The extractor fans out one record
per slug downstream so the SQLite loader can store per-slug rows.

## Tests

```bash
cargo test --manifest-path tools/mame-extractor/Cargo.toml
```

The unit tests exercise the listxml parser against an embedded sample,
the history.xml slimmer against a four-`<entry>` sample mirroring the
real upstream shape (NES multi-machine `<systems>` block, an unrelated
arcade machine, a software-list-only entry that must be dropped, and a
single-machine PCE entry), the chip-clock formatter at MHz / GHz / kHz /
Hz breakpoints, the placeholder-history path, and a sanity check that
`MAME_DRIVER_MAP` has no duplicate slugs. The arcade-games slim path
adds five more tests against an embedded seven-machine sample:
parent + clone preserve their own descriptions; BIOS / device /
runnable=no / ROM-less entries are excluded; output sorts
alphabetically; the per-system path is unaffected by the new arcade
walk; and the `ArcadeGameRecord` serialised JSON shape stays locked.
