# mame-extractor

Maintainer-only utility that slims MAME's `-listxml` output and
`history.xml` (from arcade-history.com / Gaming-History) down to the
~40 machines that map to OA-supported systems. Produces the three
files in `assets/mame-source/` that `oa-shell` bakes into SQLite on
launch (System Info Panel v1, Phase 2).

Invoked exclusively via `tools/bump-mame.sh`. Standalone Cargo workspace —
NOT a member of OA's root workspace, so `cargo build --workspace` /
`cargo test --workspace` from the repo root do not touch it.

Plan reference: `docs/PLANS/system-info-panel-v1.md`.

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
`MAME_DRIVER_MAP` has no duplicate slugs.
