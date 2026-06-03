# Per-track SHA-1 matching for disc-shape systems

**Status:** Planning locked 2026-06-02 (4 rounds of operator Q&A; all design questions answered). Research pass 2026-06-03 closed the three "resolve before Phase 2" open questions (Q1 hash convention, Q2 schema shape, Q3 chd-crate TOC). Execution in flight as Phase A1 of the [virtual library + launcher arc](virtual-library-and-launcher-arc.md).

**Owner-of-decisions:** the operator. This document records the
decisions that came out of the refinement Q&A. Implementation
should follow them unless a code-time issue forces a revisit (in
which case: check back in here first).

---

## Why this matters

Cart-shape ROMs get the full canonical-identification treatment
today: at "Identify ROMs" time the file's SHA-1 is computed, looked
up against libretro-database's no-intro dat, and the matched game's
title / serial / year / publisher get stamped onto the library row.
Operators with cart libraries see "Sonic the Hedgehog (USA)" instead
of `sonic.bin`; cover art lookups light up; year + publisher feed
the Game Info Panel + the Retroverse DISCOVER tab's data-driven
axes.

**Disc-shape systems don't get any of that.** PSX, Saturn, Sega CD,
Dreamcast, Neo Geo CD, PC Engine CD, PC-FX, 3DO, GameCube, PSP, PS2
— operator drops in 100 `.cue` files or 100 `.chd` files and gets
100 library rows named "Game (USA)" or whatever the cuesheet
basename was. Cover-art sync falls back to fuzzy filename match
against thumbnails (works for the common cases, misses on every
non-standard name). Year + publisher stay blank, so DISCOVER's "By
era" / "By publisher" axes are empty for the entire disc-game half
of the library.

We do have **disc-ID extraction**: reading IP.BIN / SYSTEM.CNF /
catalog-code headers from the first data track and matching against
the redump dat's `serial` field. That produces canonical titles for
games whose serial is in the dat — but it depends on every
operator-dumped disc having an exact matching serial in redump,
which not every dump does (homebrew, prototype, region variants
that overlap, dumps where the serial bytes got truncated by the
dump tool). The operator's hit rate via serial-lookup is good but
not complete.

**Per-track SHA-1 matching closes the gap.** Redump publishes
per-track SHA-1s for every disc they catalog. We hash each track
of the operator's disc, match the set against redump, and on a
hit stamp the canonical title alongside (or instead of) the serial.
Same machinery cart ROMs already have, extended to disc shape.

The operator-facing pitch this unlocks ("help new users out"):
"Drop your PSX dump folder, click Identify ROMs, get canonical
titles + cover art + year + publisher for every game — same way
your NES folder works." Without this work, that pitch needs an
asterisk: "...except disc-shape systems which sometimes work."

---

## Locked design decisions (from 4 rounds of refinement, 2026-06-02)

### Identification scope

- **Hash data tracks only; skip audio entirely for the
  identification gate.** Redump publishes per-track SHA-1s for
  every track including audio, but audio tracks (CD-DA) vary by
  dump-tool conventions (drive offset correction not applied by
  cdrdao raw / older EAC profiles) and cause false negatives on
  legitimately good dumps. Data tracks are byte-perfect across
  dump tools, so the *match decision* is based on data tracks
  only. Audio matches are scored opportunistically as a "bonus
  confidence" signal (DiscImageCreator + redumper dumps WILL
  match) but never block identification. See "Research pass —
  2026-06-03 findings" below for the redump-conventions
  reasoning.
- **Hash takes precedence over serial.** Hash result wins, always.
  The existing SYSTEM.CNF / IP.BIN serial-lookup path becomes the
  fallback when no hash match is found. Clean precedence hierarchy:
    1. Per-track SHA-1 match → canonical title
    2. Serial-lookup (disc-id extraction) → canonical title
    3. Filename-based (today's default) → cleaned filename

### Match strictness

- **Configurable in Settings, default Strict.** Three options:
    - **Strict** (default) — all data tracks must match
    - **Threshold-80%** — ≥80% of data tracks must match
    - **Lenient** — any one data track matches
- All matches under Strict show no badge. Threshold / Lenient
  matches show a ⚠ "Partial match" badge on the library tile
  + per-track-detail in the GameDetailPanel.

### Schema (3 new tables)

- **`rom_hashes_tracks`** — canonical SHA-1 → game lookup downloaded
  from the redump dat. Separate from existing `rom_hashes` (cart
  systems' single-file lookup stays untouched).

```sql
CREATE TABLE rom_hashes_tracks (
    sha1            TEXT NOT NULL,
    system_id       TEXT NOT NULL,
    game_name       TEXT NOT NULL,
    serial          TEXT,
    track_number    INTEGER NOT NULL,
    track_mode      TEXT NOT NULL,   -- "MODE1/2352", "MODE2/2352", "AUDIO"
    size_bytes      INTEGER NOT NULL,
    PRIMARY KEY (sha1, system_id)
);
CREATE INDEX idx_rom_hashes_tracks_by_game
    ON rom_hashes_tracks (system_id, game_name);
```

- **`game_disc_tracks`** — per-game cache of operator's computed
  per-track SHA-1s. Idempotent re-identification reads this + skips
  rehashing matched discs.

```sql
CREATE TABLE game_disc_tracks (
    game_id         TEXT NOT NULL,    -- FK to games.id
    track_number    INTEGER NOT NULL,
    sha1            TEXT NOT NULL,
    track_mode      TEXT NOT NULL,
    size_bytes      INTEGER NOT NULL,
    -- Cache invalidation: scan checks file mtime+size against these
    -- stamps; if they differ, cache rows for that game are deleted
    -- and re-identification is queued.
    file_mtime      INTEGER NOT NULL,
    file_size       INTEGER NOT NULL,
    last_hashed_at  INTEGER NOT NULL,
    PRIMARY KEY (game_id, track_number),
    FOREIGN KEY (game_id) REFERENCES games (id) ON DELETE CASCADE
);
CREATE INDEX idx_game_disc_tracks_by_game ON game_disc_tracks (game_id);
```

- **`disc_sets`** — multi-disc game grouping. When redump tells
  us Foo (Disc 1), Foo (Disc 2), Foo (Disc 3) belong to the same
  game, OA stamps a disc set; the LIBRARY tile becomes one tile
  per SET (rather than one tile per disc).

```sql
CREATE TABLE disc_sets (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    canonical_title     TEXT NOT NULL,
    system_id           TEXT NOT NULL,
    disc_count          INTEGER NOT NULL,
    created_at          INTEGER NOT NULL
);

-- New columns on the existing `games` table:
ALTER TABLE games ADD COLUMN disc_set_id    INTEGER;  -- FK to disc_sets.id; NULL = standalone
ALTER TABLE games ADD COLUMN disc_number    INTEGER;  -- 1-based; NULL for standalone
CREATE INDEX idx_games_disc_set ON games (disc_set_id);
```

### Eager auto-trigger via background-jobs dependency graph

- **At scan completion**, the wizard auto-fires the chain through
  the background-jobs dependency graph
  ([docs/PLANS/background-jobs-and-progress-bar.md](background-jobs-and-progress-bar.md)):
    - `folder_scan` completes →
    - `hash_resolve` auto-queues for cart-shape systems →
    - `disc_track_hash` auto-queues for disc-shape systems →
    - `artwork_sync` + `metadata_sync` auto-queue once identification
      lands
- The operator imports their folder, walks away, comes back to a
  fully-identified + cover-arted + metadata-stamped library.
- Auto-trigger respects per-kind opt-out: any kind the operator
  marked "prompt before resuming" in Download Settings asks first.

### Cache invalidation: auto-detect file changes

- **`file_mtime` + `file_size` stamps on every `game_disc_tracks`
  row.** Scan time: OA stat()s each disc file and compares to the
  cached stamp.
    - Stamps match → cache is valid, skip re-identification.
    - Stamps differ (operator replaced the dump) → cache rows for
      that game DELETE'd; `disc_track_hash` re-queues for that
      game; new run produces new cache.
- Magic + invisible: operator who swaps a disc for a better dump
  just gets identification re-run on the next scan with no manual
  step.

### Partial-match UX

- **Stamp the canonical title even on partial matches.** Operator
  gets the useful information (the title is probably right) AND a
  truthful indicator (something diverged).
- **⚠ "Partial match (N of M tracks)" badge** on the library tile
  for any threshold or lenient match.
- **GameDetailPanel detail:** "Partial match — track 3 (audio /
  data) didn't match the canonical SHA-1. Click to see candidate
  matches." Operator can drill into the why.
- Strict matches show no badge (full match is the silent default).

### Identification feedback

- **Post-completion toast / dialog: "Identified 47 of 50 PSX
  discs" + deep-link to filter library by unmatched.** This is the
  one place we deliberately violate the BackgroundJobs "toasts
  retire" rule, because:
    - This is informational + actionable (47/50 + click to see the
      3 that didn't), not progress (the bar handled progress).
    - The operator left the app to walk away during eager
      identification; they need a "here's what happened" marker
      when they return.
- The deep-link opens LIBRARY with the system selected + a filter
  applied: "Show unmatched discs." Operator can decide what to do
  with each.

### Unmatched discs

- **Library tile keeps the filename-based title** (today's
  pre-identification behavior).
- **"? Unidentified" chip on the tile** signals the row didn't
  match.
- **"Submit this dump to redump.org" link** in the
  GameDetailPanel's identification section, opens redump.org's
  submission page. Encourages ecosystem participation for legit
  homebrew / prototypes / region variants redump hasn't catalogued.
  (Outbound link only; OA doesn't auto-upload anything.)

### Multi-disc auto-grouping

- **When redump tells us a multi-disc set exists** (Foo (Disc 1) +
  Foo (Disc 2) + Foo (Disc 3) live under one game in the dat),
  OA auto-creates a `disc_sets` row with the canonical_title and
  stamps `games.disc_set_id` + `games.disc_number` on each disc.
- **Library tile becomes the SET, not the disc.** Foo shows as ONE
  tile labelled "Final Fantasy IX" (canonical set title). Tile
  click opens a **disc-picker sub-view** ("Disc 1 / Disc 2 /
  Disc 3 / Disc 4") → operator picks → launch.
- This work touches LIBRARY's grid rendering + the launch path
  (which today expects a single `games.id`); new disc-picker view
  is a substantial UI surface, breaking it out as Phase 4 of the
  arc.
- Standalone discs (no set, or set of one) launch directly on tile
  click (today's behavior — unchanged).

### Operator title edits vs canonical hash match

- **Operator edits always win** for the displayed title. Per
  today's Game Info Panel L3 override pattern.
- The canonical title from hash matching IS recorded in MediaDb
  (so it survives re-imports + appears in detail panel as
  "Canonical title: Foo (USA)" beneath the operator's edit).
- Re-identification doesn't overwrite operator edits. The canonical
  title quietly updates in MediaDb if redump publishes a revision;
  the operator's displayed title stays as-is.

### Manual re-identification trigger

- **Auto-detect file changes only.** No manual "Re-identify"
  button. The mtime+size cache invalidation handles the only
  legitimate "operator wants re-identification" case: they
  replaced a disc file with a different dump.
- Edge case: operator wants to force re-identification without
  changing the file (e.g. a new redump dat dropped with corrected
  hashes for their disc). Workaround: edit the file's mtime
  (touch). Acceptable for an edge case; can revisit if it bites.

### CHD per-track byte extraction

**Locked from 2026-06-03 research pass.** The `chd 0.3` crate
already in tree does NOT expose a `Track` abstraction. We parse
the CHT2 / CHTR / CHGD text metadata ourselves and walk hunks
manually.

- **Open** via `chd::Chd::open(reader, None)` — same as the
  existing `cd_id::chd_reader::read_data_track_header` pattern
  at `apps/oa-shell/src/cd_id.rs:533-609`.
- **Parse track metadata** from `Chd::metadata_refs()`. CHT2
  text format string (lifted from MAME `cdrom.cpp`):
  `"TRACK:%d TYPE:%s SUBTYPE:%s FRAMES:%d PREGAP:%d PGTYPE:%s PGSUB:%s POSTGAP:%d"`.
  Legacy CHTR omits PREGAP/POSTGAP; CHGD (GD-ROM) adds
  PADFRAMES.
- **Compute per-track phys ranges** with CHDMAN's 4-frame track
  padding (`TRACK_PADDING = 4` in MAME `cdrom.h`):
  `track[i].phys_start = sum over j<i of ceil(track[j].frames / 4) * 4`
  `track[i].phys_end = track[i].phys_start + track[i].frames`
  Padding frames are in the hunk stream but excluded from any
  track's hash.
- **Walk hunks** via `Chd::hunk(n)` + `Hunk::read_hunk_in`. For
  each frame in the hunk (CD: `unit_bytes = 2448`), attribute
  by global LBA to its track's SHA-1 hasher. Strip the 96-byte
  subchannel tail (redump hashes bytes 0..2352 per frame).
- **Hunk-straddles-track-boundary** is the caller's
  responsibility; per-frame dispatch handles it naturally.
- **DVD-shape CHDs** (GameCube / Wii / PS2): `unit_bytes = 2048`,
  no subchannel, no CdRomTrack metadata. Detect via
  `unit_bytes != 2448` and treat as single contiguous stream
  per the .iso convention.
- **Legacy CHCD format** (old CHDMAN): one combined blob; rare;
  treat as not-matchable in v1.
- Constant memory footprint: one hunk buffer (~20 KB) + N SHA-1
  contexts. Scales to PS2 / GameCube multi-GB CHDs.

### Performance budget

- **Acceptable as-is.** Plan estimates ~10 seconds per PSX disc /
  ~90 minutes for a 500-game PSX library, on a one-time eager
  pass. Re-runs are no-ops (cache hits). Operator imports, walks
  away, returns to identified library. Background-jobs bar
  surfaces progress; chime + post-completion toast acknowledge.

### Research pass — 2026-06-03 findings locked

Three open questions called out for "resolve before Phase 2" were
researched in parallel via subagents (web research + codebase
audit + chd-crate / libchdr survey). Outcomes:

#### Q1 — Hash convention (resolved, gate removed)

Redump hashes **the full raw 2352-byte sectors as written to
the .bin file**, for both MODE1/2352 and MODE2/2352 (both
forms). Audio is also full 2352 raw. The rule across all CD
sector formats reduces to: **hash the file byte-for-byte, no
preprocessing.** Verified empirically against the libretro PSX
dat (13,526 track entries, every size cleanly divides 2352).

Side findings:
- **Audio is opportunistically matchable**, not unmatchable.
  Tools that apply drive-offset correction (DiscImageCreator,
  redumper) produce byte-identical audio .bin across dumpers.
  Tools that don't (cdrdao raw, old EAC) fail audio matching
  even on otherwise-good dumps. Recommendation: data tracks
  gate identification; audio matches score as a "bonus
  confidence" signal but never block.
- **CHD files cannot be matched directly against redump.**
  chdman recompresses + writes its own container SHA-1. We hash
  the *decompressed raw frames* per track, never the .chd file
  bytes themselves.
- **MODE1/2048 cooked .iso for CD systems** is NOT in redump
  (that's TOSEC convention). If the operator hands us a
  2048-aligned CD .iso, we cannot match it against redump.
- **Single-bin .cue dumps** (one merged .bin with multiple
  INDEX 01 positions, common on PSX): split on-the-fly using
  cue INDEX 01 positions × 2352 to derive per-track byte ranges
  into the single .bin; stream slices per track for hashing.
  Don't require re-dumping.

Phase 1's "verification pass" is downgraded from a Phase 2
*gate* to a smoke test (5 PSX + 5 Saturn + 5 Dreamcast)
confirming our pipeline produces identical SHA-1s end-to-end.

#### Q2 — Schema shape (separate `rom_hashes_tracks` confirmed)

Plan-of-record (3 new tables, `rom_hashes` untouched) stands.
Codebase audit confirmed:
- Current `rom_hashes` PK is `sha1` alone
  (`apps/oa-shell/src/library_db.rs:1229`). Extending it would
  require the first PK-mutation migration in this codebase —
  every prior migration is append-only column adds or new
  tables.
- Migration precedent: v8→v9 added `game_serials` as a separate
  table for parallel disc-ID lookup rather than extending
  `games`. Same pattern fits here.
- Six consumer functions on `rom_hashes` stay untouched — zero
  regression risk on the cart path.
- Parser already emits one row per file in a multi-file game; we
  add a `RomTrackRow` variant + a parallel
  `replace_rom_hashes_tracks_for_system`.
- Dispatch cost: ~5–8 lines in
  `apps/oa-shell/src/scan_service.rs::apply_smart_classification`
  (line 619) and in `resolve_rom_hashes_for_system`
  (`apps/oa-shell/src/rom_hashes.rs:1351`) branching on
  disc-shape system_id.

New `rom_hashes_tracks` PK is `(sha1, system_id)` — tightens
the implicit "SHA-1 is globally unique" assumption that holds
in practice today but is unenforced.

#### Q3 — CHD per-track extraction (feasible via streaming + manual TOC parse)

`chd 0.3` (resolves 0.3.2) is already in tree at
`apps/oa-shell/Cargo.toml`. Existing pattern at
`apps/oa-shell/src/cd_id.rs:533-609`
(`chd_reader::read_data_track_header`) is the open + hunk-read
template.

Critical findings from the chd-crate + libchdr + MAME survey:
- **No `Track` struct, no `byte_range()`, no per-track hunk
  iterator in chd-rs.** The crate stops at "hunks + metadata
  refs." We parse CHT2 / CHTR / CHGD text blobs ourselves.
- **CHT2 format string** (lifted from MAME
  `src/lib/util/cdrom.cpp`):
  `"TRACK:%d TYPE:%s SUBTYPE:%s FRAMES:%d PREGAP:%d PGTYPE:%s PGSUB:%s POSTGAP:%d"`.
  Legacy CHTR omits PREGAP/POSTGAP; CHGD (GD-ROM) adds
  PADFRAMES.
- **4-frame track padding gotcha** — `TRACK_PADDING = 4` in
  MAME `cdrom.h`. CHDMAN pads each track to a 4-frame boundary;
  padding frames are in the hunk stream but excluded from any
  track's hash. Per-track byte-range math:
  `track[i].phys_start = sum over j<i of ceil(track[j].frames / 4) * 4`
  `track[i].phys_end = track[i].phys_start + track[i].frames`
- **Hunk-straddles-track-boundary is on the caller.** Walk per
  frame (2448 bytes for CD), attribute each frame to its track
  by global LBA, dispatch to that track's SHA-1 hasher.
- **Subchannel data (last 96 of each 2448 frame) is excluded
  from redump's hash.** Slice out bytes 0..2352 from each CD
  frame before feeding the hasher.
- **DVD-shape CHDs** (GameCube / Wii / PS2 .iso-style):
  `unit_bytes = 2048`, no subchannel, no CdRomTrack metadata.
  Detect via `unit_bytes != 2448` and treat as single contiguous
  stream (aligns with Q1's DVD `.iso` finding — no track
  abstraction).
- **Legacy CHCD blob format** (old CHDMAN, one combined entry):
  rare; treat as not-matchable in v1.

Constant memory: one hunk buffer (~20 KB) + N SHA-1 contexts.
Scales to PS2 / GameCube multi-GB CHDs without trouble.

---

## What redump publishes (technical reference)

The libretro-database redump dats (e.g.
`metadat/redump/Sony - PlayStation.dat`) are clrmamepro format. One
`game (...)` block per disc with multiple `rom (...)` entries —
one per track. Each track has its own SHA-1.

Example shape (synthesized; real entries are longer):

```
game (
    name "Tomb Raider (USA)"
    description "Tomb Raider (USA)"
    serial "SLUS-00152"
    rom ( name "Tomb Raider (USA) (Track 01).bin" size 622272 crc XXXXX sha1 11111... )
    rom ( name "Tomb Raider (USA) (Track 02).bin" size 33840960 crc YYYYY sha1 22222... )
    rom ( name "Tomb Raider (USA).cue" size 312 crc ZZZZZ sha1 33333... )
)
```

The `.cue` sidecar gets its own SHA-1 — but the operator's
cuesheet content varies by dump tool, so we don't match on that.
What we match is the per-track `.bin` SHA-1 set (data tracks only
per the locked decision).

Multi-disc games have a redump "parent" relationship indicated by
the title pattern `Foo (Disc 1)` + `Foo (Disc 2)` + ... The
`disc_sets` auto-grouping reads these title suffixes (regex
`(.*) \(Disc (\d+)\)`) + groups them under the parent name.

---

## What we compute on the operator's side

For each disc image the operator has:

### Parse the tracks

- **`.cue + split .bin` (one .bin per track):** cuesheet already
  parsed by `cd_id::cue::parse` (Slice 2 used this for the
  data-track disc-id peek). Each track points at a `.bin` file
  with a mode + sector size. Hash each per-track .bin file
  directly.
- **`.cue + single merged .bin`** (common on PSX): parse INDEX 01
  positions from the cuesheet; multiply by 2352 to derive
  per-track byte offsets into the single .bin; stream byte
  ranges per track.
- **`.chd`:** decompress hunk-by-hunk; parse the embedded
  CHT2 / CHTR / CHGD text metadata for per-track frame counts;
  account for CHDMAN's 4-frame track padding; dispatch frames to
  per-track hashers (subchannel-stripped). See "CHD per-track
  byte extraction" below for the API + the 4-frame padding
  details locked in the 2026-06-03 research pass.
- **`.gdi`:** Dreamcast cuesheet shape
  (`track_no track_lba mode_str sector_size file_offset`).
  Parser analogous to `.cue`.
- **`.iso`:** single track, full file is the data bytes.
  Trivial. Note: only valid for DVD-shape systems (GameCube /
  Wii / PS2 / PSP). MODE1/2048 cooked `.iso` for CD systems is
  NOT in redump — see "Out of scope" for the reject path.

### Compute the per-track SHA-1

**Locked from 2026-06-03 research pass:** Redump hashes the full
raw .bin bytes per track, no preprocessing — across MODE1,
MODE2, and audio. The rule reduces to: **hash the file
byte-for-byte.** Verified empirically against the libretro PSX
dat (13,526 track entries, every size cleanly divides 2352).

What this means for each container shape we have to handle:
- **Split `.bin` per track** (the natural shape): stream the
  per-track .bin file directly through SHA-1. No
  mode-awareness needed at hash time.
- **Single `.bin` for whole disc** (common on PSX): parse cue
  INDEX 01 positions × 2352 to derive per-track byte ranges
  into the single .bin; stream slices per track.
- **CHD**: decompress hunk-by-hunk, walk frames, slice off the
  96-byte subchannel tail per frame (CD: 2352 of 2448 keeps
  the user-data bytes redump hashes), dispatch each frame's
  bytes to the right track's hasher per the manual TOC walk.
  See "CHD per-track byte extraction" above.

Audio tracks are hashed the same way (full 2352 raw per sector).
Identification gating uses data tracks only — see the locked
decision in "Identification scope"; audio matches score
opportunistically.

### Match all data tracks against a redump game entry

A disc matches a redump entry per the operator's chosen strictness:
- **Strict** — every data track's SHA-1 in `game_disc_tracks` is
  in `rom_hashes_tracks WHERE system_id = ? AND game_name = ?`.
- **Threshold-80%** — at least 80% of the operator's data tracks
  match.
- **Lenient** — at least one data track matches.

First check: look up the FIRST data track's SHA-1 in
`rom_hashes_tracks` to find candidate (system_id, game_name). Then
verify per strictness against the candidate's full track set.

---

## Sync flow

Today's `sync_rom_hashes_for_system` pulls
`libretro_dat_refs_for_system_resolved` and parses each dat into
the `rom_hashes` table. Adding per-track support:

1. `parse_libretro_dat` (existing) walks every `rom (...)` in a
   game block. Today: one `RomHashRow` per file. Extension:
   parse per-track + emit per-track `RomTrackRow` entries with
   `track_number` derived from filename (`(Track NN)` convention)
   or the rom's position within the game block.
2. Bulk-replace into `rom_hashes_tracks` parallel to existing
   `replace_rom_hashes_for_system`.
3. Cache + TTL same as today (24h).

Detect multi-disc sets at sync time: for each system, group
`rom_hashes_tracks` entries by `regex.captures(game_name,
r"^(.*) \(Disc (\d+)\)")`. Stamp a per-set canonical title for
later disc_set creation.

---

## Lookup + identification flow

Eager auto-trigger fires after scan via the background-jobs
dependency graph:

1. `disc_track_hash` job created for the system with N discs to
   identify.
2. For each game in the library missing canonical identification:
    - Stat the file; compare mtime+size against `game_disc_tracks`
      cache rows (if any). Stamps match → cache valid → skip.
    - Parse tracks via the container-specific parser.
    - Compute each data track's SHA-1 (with per-track progress
      emit; the bar shows "Hashing Tomb Raider (USA) — Track 03
      of 5...").
    - Persist `game_disc_tracks` row per track (with mtime+size
      stamp).
    - First data track SHA-1 lookup → candidate game.
    - Strictness verification against candidate's track set.
    - Match → stamp canonical title (if operator hasn't edited) +
      stamp `disc_set_id` if multi-disc parent → fire
      `artwork_sync` + `metadata_sync` dependents.
3. Per-disc progress events sized for the BackgroundJobsBar:
   the disc_track_hash job's done/total reports
   `n_discs_completed / n_discs_total`; nested progress events
   carry per-track byte progress for the disc currently being
   hashed (the bar's expanded row shows both).
4. On job completion: post-completion toast/dialog (the exception
   to "toasts retire" — informational, not progress).

---

## Operations to consolidate

| Operation | Job kind | New / existing |
| --- | --- | --- |
| Per-disc per-track SHA-1 hashing | `disc_track_hash` (defined in BackgroundJobs plan) | NEW |
| Per-system redump sync extension | `dat_sync` (existing) | EXTENDED |
| Disc-set grouping | (no new job — fires at end of `disc_track_hash`) | NEW |
| Library tile + disc-picker sub-view | (UI; no job) | NEW |
| Post-completion identification toast | (one-shot notification) | NEW |

---

## Sizing

Rough phasing — ~4-5 weeks total (grew from the original 3-4 week
estimate because of the multi-disc disc-set work added in Round 3):

- **Phase 1 — schema + sync + smoke verification** (~1 week):
  Three new tables (`rom_hashes_tracks`, `game_disc_tracks`,
  `disc_sets`). `parse_libretro_dat` extension to emit
  `RomTrackRow` per track. Sync flow update + parallel
  `replace_rom_hashes_tracks_for_system`. The hash-convention
  question is **resolved** per the 2026-06-03 research pass
  (full 2352 bytes / hash file byte-for-byte); what remains is
  a smoke test against 5 PSX + 5 Saturn + 5 Dreamcast known
  dumps confirming the end-to-end pipeline reproduces redump
  SHA-1s. No longer a Phase 2 gate.

- **Phase 2 — per-track hashing engine** (~1.5 weeks):
  Container-specific track parsing: `.cue + .bin` (extend
  existing cd_id parser), `.chd` (stream-per-track via TOC;
  needs chd-crate TOC API investigation), `.gdi`, `.iso`.
  Mode-aware sector unwrapping. Cancellable streaming SHA-1 per
  track. Strictness implementation (Strict / Threshold-80% /
  Lenient).

- **Phase 3 — Identify flow + cache + auto-trigger** (~1 week):
  `disc_track_hash` job kind wired into BackgroundJobsBar.
  Eager auto-trigger via dep graph at scan completion.
  Per-track cache write + mtime/size invalidation. Per-disc
  nested progress UI. Post-completion identification
  toast/dialog. Deep-link to filter library by unmatched.
  Per-game "Submit to redump.org" link in detail panel.

- **Phase 4 — Multi-disc disc-set support** (~1 week):
  `disc_sets` schema + auto-detection from redump titles.
  LIBRARY grid: render one tile per set instead of one per disc.
  Disc-picker overlay sub-view. Launch-path update to support
  "launch disc N of set X." Migration handling (existing
  multi-disc libraries get their sets auto-grouped on next
  scan).

- **Phase 5 — Validation + docs** (~3-4 days):
  Operator playtest on real PSX / Saturn / Dreamcast folders.
  Hit-rate measurement (target 95%+ of redump-cataloged dumps
  identify cleanly). Per-system `docs/cores/<id>/README.md`
  mention. Settings panel polish (the Strict/Threshold/Lenient
  picker lands in Download Settings).

---

## Risks

- **~~Track-hashing convention drift.~~** **Resolved 2026-06-03
  research pass.** Redump hashes the full raw per-track .bin
  byte-for-byte, no preprocessing — verified empirically against
  the libretro PSX dat (13,526 track entries, every size cleanly
  divides 2352). MODE1/2352, MODE2/2352 Form 1, MODE2/2352
  Form 2, and audio all use the same convention. The 15-dump
  check stays as a Phase 1 smoke test but no longer gates
  Phase 2.

- **~~`.chd` TOC complexity.~~** **Resolved 2026-06-03 research
  pass.** The `chd 0.3` crate doesn't expose a per-track
  abstraction — we parse CHT2 text metadata + walk hunks
  frame-by-frame with a manual 4-frame padding accounting.
  Worst-case "decompress whole disc" fallback is NOT needed. See
  "CHD per-track byte extraction" for the locked approach.
  Constant memory (~20 KB hunk buffer + N SHA-1 contexts)
  regardless of disc size.

- **Hash compute time on big libraries.** 500-disc PSX library is
  ~5 GB of disc data = ~90 minutes of SHA-1 work. Operator
  accepted this in Round 3, but real-world testing may surface
  pain we didn't anticipate (slow disks, background CPU
  pressure). Mitigations available: per-track caching (already
  in plan), background-jobs pause/resume, eventually a "skip
  identification for this system" Download Settings toggle if
  needed.

- **`.iso` ambiguity.** `.iso` files for disc-shape systems
  (GameCube, PSP, PS2) are usually single-track, full-file
  user data. But some `.iso` dumps include the data-track
  header bytes; redump's convention varies. Treat as edge case;
  start with the common shape; Phase 1 verification catches
  format-specific issues.

- **Redump coverage isn't 100%.** Some homebrew + prototypes +
  obscure regional variants aren't in redump. These will
  legitimately not match. The "Submit to redump.org" link is
  the long-term mitigation (encouraging contribution); the
  short-term mitigation is the "? Unidentified" chip making
  it visible to the operator.

- **Disc-set auto-grouping false positives.** Redump's title
  pattern matching ("Foo (Disc 1)" + "Foo (Disc 2)") could group
  unrelated discs that happen to share a prefix. Mitigation:
  only group when redump's GAME entry explicitly signals
  multi-disc (parent-game pattern, not just title regex);
  Phase 1 sync extension establishes the data source for the
  parent relationship.

- **Operator confusion when 3 of 5 tracks match (Threshold
  default → no match; Lenient → match).** Strictness setting
  toggling can dramatically change hit rates. Mitigation: clear
  Settings copy ("Strict: trust only exact matches. Lenient: trust
  partial matches — may identify your dumps as similar but
  not-quite-identical games"). Default Strict for safety.

---

## Out of scope (won't do here)

- **Pre-burning per-track SHA-1s into shipped OA data.** Same
  reasoning as the existing rom_hashes sync — fetch on demand +
  cache.
- **Per-track repair / re-encoding.** We hash + match, we don't
  fix bad dumps.
- **`.bin` files dropped without their `.cue` companion.** Already
  filtered out by the Slice 2 dedupe (data-track rows without a
  playlist sibling get dropped at scan time).
- **Disc-shape systems redump doesn't catalog.** 3DO is the
  notable case — no per-track redump data exists. These stay on
  the existing fuzzy-filename + serial-lookup path; no per-track
  hashing attempt.
- **Manual disc-set grouping UI.** Multi-disc auto-grouping only
  fires from redump data. Operator-curated grouping (e.g. mod /
  ROM-hack collections) stays out of scope; existing .m3u
  playlist workflow covers it.
- **CD-DA-only audio discs (arcade music CDs).** Edge case;
  out of scope.
- **MODE1/2048 cooked `.iso` for CD systems** (e.g. a PSX disc
  ripped with 2048-byte sectors). Not in redump (TOSEC
  convention). When detected, the library row stays on the
  filename-based title path; GameDetailPanel surfaces a hint
  that a raw 2352-byte re-rip would unlock identification.

---

## When this arc starts

This plan is in flight as Phase A1 of the
[virtual library + launcher arc](virtual-library-and-launcher-arc.md).
The executing session should:

1. **Re-read this plan in full.**
2. **Hard dependency check:** confirm the background-jobs
   foundation has shipped (or is shipping in parallel). The
   `disc_track_hash` kind, the dependency graph, the
   BackgroundJobsBar UI, and the eager auto-trigger flow all
   come from
   [docs/PLANS/background-jobs-and-progress-bar.md](background-jobs-and-progress-bar.md).
   This arc CAN proceed without the bar (the dat-sync extension +
   per-track hashing can land independently) but the eager
   auto-trigger + post-completion toast both want the bar
   present. (Background-jobs foundation shipped 2026-06-03;
   prerequisite met.)
3. **Phase 1 verification** is a smoke test, not a gate (per
   the 2026-06-03 research pass). The track-hash convention is
   resolved; the 15-disc sample (5 PSX + 5 Saturn + 5 Dreamcast)
   confirms our end-to-end pipeline reproduces redump SHA-1s
   but does not block Phase 2.
4. **Branch as `feat/virtual-library-phase-a1-disc-track-sha1`**
   per the standard workflow.

---

*Plan refined 2026-06-02 across 4 rounds of operator Q&A. 15
design decisions locked. Original framing: "needed to help new
users out" — disc-shape systems are the weakest part of the
canonical-identification story today, and a new operator's first
PSX folder import is the moment to fix it.*
