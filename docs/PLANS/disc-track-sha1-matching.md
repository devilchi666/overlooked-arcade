# Per-track SHA-1 matching for disc-shape systems

**Status:** Planning locked 2026-06-02 (4 rounds of operator Q&A; all design questions answered). Execution deferred to a future session.

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

- **Hash data tracks only; skip audio entirely.** Redump publishes
  per-track SHA-1s for every track including audio, but audio
  tracks (CD-DA) vary by dump-tool conventions (pre-emphasis
  flags, encoder differences) and cause false negatives on
  legit dumps. Data tracks are byte-perfect across dump tools, so
  the match decision is based on data tracks only. Audio sectors
  are not read.
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

- **Stream per-track using the CHD's embedded TOC.**
    - Use the `chd` crate's TOC accessor to find each track's
      hunk range.
    - Decompress one hunk at a time; accumulate the current track's
      user-data bytes; hash inline; free hunk memory immediately.
    - Constant memory footprint (~one hunk = ~4 KB).
- Matches the existing `cd_id::chd_reader::read_data_track_header`
  pattern. The chd crate's TOC API needs investigation to confirm
  it exposes per-track LBA ranges; Phase 1's verification pass
  covers this.

### Performance budget

- **Acceptable as-is.** Plan estimates ~10 seconds per PSX disc /
  ~90 minutes for a 500-game PSX library, on a one-time eager
  pass. Re-runs are no-ops (cache hits). Operator imports, walks
  away, returns to identified library. Background-jobs bar
  surfaces progress; chime + post-completion toast acknowledge.

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

- **`.cue + .bin`:** cuesheet already parsed by `cd_id::cue::parse`
  (Slice 2 used this for the data-track disc-id peek). Each
  track points at a `.bin` file with a mode + sector size.
- **`.chd`:** decompress (streaming, per-hunk), walk the embedded
  TOC, extract per-track byte ranges.
- **`.gdi`:** Dreamcast cuesheet shape
  (`track_no track_lba mode_str sector_size file_offset`).
  Parser analogous to `.cue`.
- **`.iso`:** single track, full file is the data bytes. Trivial.

### Compute the per-track SHA-1

Mode-aware:
- **`MODE1/2048`:** file bytes ARE the user data; hash the file
  contents directly.
- **`MODE1/2352`:** each 2352-byte sector has 16 header bytes
  (sync + address + mode) + 2048 user bytes + 288 ECC bytes.
  Redump's convention varies — Phase 1 verification confirms
  whether redump hashes the full 2352 or just the 2048 user
  payload (likely full 2352 to preserve dump fidelity).
- **`MODE2/2352`** (Form 1 / Form 2): 24-byte header + 2048
  user bytes + 280 ECC, OR 24-byte header + 2324 user bytes + 4
  ECC depending on form. Same verification gate as MODE1/2352.

Audio tracks are skipped entirely per the locked decision.

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

- **Phase 1 — schema + sync + verification** (~1 week):
  Three new tables (`rom_hashes_tracks`, `game_disc_tracks`,
  `disc_sets`). `parse_libretro_dat` extension. Sync flow update.
  **Critical verification pass** — confirm redump's track-hash
  convention (MODE1/2352 as full 2352 bytes vs just 2048 user
  data; MODE2 variants) against a small set of known dumps.
  Hand-pick 5 known PSX dumps + 5 Saturn + 5 Dreamcast; verify
  hash convention before Phase 2.

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

- **Track-hashing convention drift.** Redump's per-track SHA-1
  isn't documented as a clean convention — it's "whatever bytes
  redump's dumping process produced." If different game eras have
  different conventions, the matcher could miss valid dumps.
  Mitigation: **Phase 1 verification pass is the gate to Phase
  2.** Hand-pick 15 known dumps across systems; verify hash
  output matches redump dat entries before writing the matcher.

- **`.chd` TOC complexity.** The `chd` crate has a TOC accessor
  but per-track byte-range extraction may need additional work.
  Worst case: re-encode `.chd` → raw `.bin` per track in-memory
  for hashing. Memory cost is the disc size, which on PS2 is bad.
  Mitigation: Phase 1 investigates chd-crate API + libchdr C
  reference implementation before committing to the streaming
  approach.

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

---

## When this arc starts

This plan is approved + queued (planning locked 2026-06-02) but
deferred. The executing session should:

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
   present.
3. **Phase 1 verification** is the gate to Phase 2. Don't write
   the matcher before confirming redump's track-hash convention
   against real dumps. 15-disc sample (5 PSX + 5 Saturn + 5
   Dreamcast) is the smoke test.
4. **Branch as `feat/disc-track-sha1-phase-1`** per the standard
   workflow.

---

*Plan refined 2026-06-02 across 4 rounds of operator Q&A. 15
design decisions locked. Original framing: "needed to help new
users out" — disc-shape systems are the weakest part of the
canonical-identification story today, and a new operator's first
PSX folder import is the moment to fix it.*
