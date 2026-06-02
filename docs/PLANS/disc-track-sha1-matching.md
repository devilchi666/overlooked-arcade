# Per-track SHA-1 matching for disc-shape systems

**Status:** Planning (operator-requested 2026-06-02, execution deferred to a future session). No code yet.

**Owner-of-decisions:** the operator. This document records the
shape of the work + design questions still open. Revisit before
kicking off.

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
basename was. Cover art sync falls back to fuzzy filename match
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

## What this is + what it isn't

**This arc:**
- Per-track SHA-1 hashing of disc images (`.cue + .bin`, `.chd`,
  `.gdi`, possibly `.iso` for single-track disc systems).
- A track-set lookup against the redump dat's per-track SHA-1
  entries. Match when ALL tracks in the operator's image match
  ALL tracks in a redump entry.
- Schema extension to the local `rom_hashes` table (or a new
  `rom_hashes_tracks` sibling table) so the lookup index can be
  per-track.
- Sync flow that fetches the per-track SHA-1s from redump alongside
  the existing single-file SHA-1 sync.
- Integration into the existing "Identify ROMs" flow so disc-shape
  systems get the same per-system Identify button + progress emits
  + post-commit stamping that cart systems do.

**Not this arc:**
- Whole-disc SHA-1 against cart-style identification. Useless
  shape: `.cue` / `.chd` / `.gdi` containers vary by dump-tool
  conventions (track interleaving, padding, header bytes,
  compression algorithms) and don't match the canonical bytes
  upstream catalogs index. The per-track shape is the only one
  that round-trips reliably.
- Cover-art lookup for matched games. Falls out of the existing
  cover-sync pipeline once the games have canonical titles; no
  changes needed in this arc.
- DOSBox / ScummVM identification. These are engine launchers, not
  disc-shape — separate problem (parking-lot item if it ever
  becomes worth pursuing).
- Track-level rewriting. We hash + match; we don't repair or
  re-encode operator dumps. Bad dumps just don't match, same as
  bad cart dumps.

---

## Technical shape

### What redump publishes

The libretro-database redump dats (e.g. `metadat/redump/Sony - PlayStation.dat`)
are clrmamepro format. One `game (...)` block per disc with multiple
`rom (...)` entries — one per track. Each track has its own SHA-1.

Example shape (synthesized — real entries are longer):

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

The `.cue` sidecar gets its own SHA-1 — but the operator's cuesheet
content varies by dump tool, so we can't reliably match on that.
What we CAN match reliably is the per-track `.bin` SHA-1 set.

### What we need to compute on the operator's side

For each disc image the operator has:
- **Parse the tracks** out of the container.
  - `.cue + .bin`: cuesheet already parsed by `cd_id::cue::parse`
    (Slice 2 used this for the data-track disc-id peek). Each
    track points at a `.bin` file with a mode + sector size.
  - `.chd`: decompress, walk the embedded TOC, extract per-track
    byte ranges. The `chd` crate exposes the TOC; the per-track
    extraction is new.
  - `.gdi`: another cuesheet shape (`track_no track_lba mode_str sector_size file_offset`).
    Parser + extractor analogous to `.cue`.
  - `.iso`: single track, full file is the bytes. Trivial case.

- **Compute the per-track SHA-1.** Mode-aware:
  - `MODE1/2048`: file bytes ARE the user data; hash the file
    contents directly.
  - `MODE1/2352`: each 2352-byte sector has 16 header bytes
    (sync + address + mode) + 2048 user bytes + 288 ECC bytes.
    What redump hashes varies by track type — some tracks hash
    the full 2352, some just the 2048 user payload. We need to
    match redump's convention exactly per track (likely full
    2352 for data + 2352 for audio — needs verification against
    a real dat entry).
  - `AUDIO`: 2352-byte sectors of raw PCM. Full file bytes.

- **Match all tracks against a redump game entry.** A disc matches
  a redump entry only when EVERY track's SHA-1 is in the entry's
  track set. Partial matches (1 of 5 tracks matches) aren't useful
  — could mean the operator has a different game's track that
  shares a coincidental SHA-1.

### Schema

Local SQLite tables today:
- `rom_hashes (sha1, system_id, game_name, serial, crc32, size_bytes)` —
  cart-shape: one row per file per game. SHA-1 is the primary
  lookup key.

New table proposal — `rom_hashes_tracks`:
```
CREATE TABLE rom_hashes_tracks (
    sha1            TEXT NOT NULL,
    system_id       TEXT NOT NULL,
    game_name       TEXT NOT NULL,
    serial          TEXT,
    track_number    INTEGER NOT NULL,
    track_mode      TEXT NOT NULL,
    size_bytes      INTEGER NOT NULL,
    PRIMARY KEY (sha1, system_id)
);
CREATE INDEX idx_rom_hashes_tracks_by_game ON rom_hashes_tracks (system_id, game_name);
```

The (system_id, game_name) index supports the "all-tracks-match"
query: lookup by first track's SHA-1 to get a candidate game
name, then verify every other track's SHA-1 is in the set for that
game.

Alternative shape: extend `rom_hashes` itself with `track_number INTEGER`
+ `track_mode TEXT` and let single-file rows store `NULL` for both.
Cleaner if we're confident the per-track and single-file lookups
won't interfere; might be messier if downstream consumers branch on
schema shape.

### Sync flow

Today's `sync_rom_hashes_for_system` pulls
`libretro_dat_refs_for_system_resolved` (the per-system dat refs
from `config/systems/<id>/system.yaml`) and parses each into the
`rom_hashes` table. Adding per-track support:

1. `parse_libretro_dat` (existing) already walks every `rom (...)`
   in a game block. Today it emits one `RomHashRow` per file
   regardless of track count. Extend to also emit per-track
   `RomTrackRow` entries with the track number derived from the
   filename (`(Track NN)` convention) or the `rom`'s position
   within the game block.
2. Bulk-replace into `rom_hashes_tracks` parallel to the existing
   `replace_rom_hashes_for_system`.
3. Cache + TTL same as today (24h).

### Lookup flow

Operator clicks "Identify ROMs" for a disc-shape system:

1. For each game in the library missing a serial/title (analog to
   `list_games_missing_hash` today):
   - Parse the disc's tracks.
   - Compute each track's SHA-1 (with progress emit per track —
     these can be big files; an audio track on a PSX disc is
     often 50+ MB).
   - Lookup the first data track's SHA-1 against
     `rom_hashes_tracks`. Get candidate (game_name, system_id).
   - Verify EVERY other track's SHA-1 is in
     `rom_hashes_tracks WHERE game_name = ? AND system_id = ?`.
     All match → it's that game.
   - Stamp the canonical title onto the library row.

2. Per-disc progress events (`oa://identify-disc-progress`) so the
   UI can render "Hashing Tomb Raider (USA) — Track 03 of 5...".

3. Cancellable per file (small loop in `resolve_disc_hashes_for_system`)
   so the operator can bail mid-pass.

### Performance budget

Hashing a multi-GB disc is meaningful work. Rough cost model:

- PSX disc: 600 MB typical. SHA-1 is ~500 MB/s on modern CPU → ~1.2s
  per disc.
- Dreamcast GD-ROM: ~1 GB. ~2s per disc.
- PS2 DVD: 4.5 GB common. ~9s per disc.
- 100-disc PSX library: ~2 minutes.
- 100-disc PS2 library: ~15 minutes.

Decompressing `.chd` on the way in adds ~10-30% depending on
compression ratio. Numbers above assume `.cue + .bin`; `.chd` is
slower.

Mitigations:
- Stream the SHA-1 (no need to load full file into memory; the
  existing `stream_sha1_of_file` already does this for cart ROMs
  >64 MB). Per-track variant: stream the .bin from offset N to
  offset M.
- Hash incrementally per track so progress events emit frequently.
- Cache the per-track SHA-1s in the local `games` row (new
  columns `track_1_sha1`, `track_2_sha1`, ... or a JSON blob)
  so the operator doesn't re-hash the whole disc on every
  Identify run.

---

## Open design questions

1. **Schema shape — separate table vs extended `rom_hashes`?**
   Pro-separate: clean dual-mode storage, easier downstream
   consumers. Pro-extended: fewer JOINs, fewer migrations to
   write. Need to look at how many consumers touch `rom_hashes`
   today before deciding.

2. **Track-hash convention exactly matches redump?** Need to
   confirm against a real dat entry whether redump hashes
   MODE1/2352 data tracks as full 2352 bytes or just the 2048
   user data. Same question for MODE2 / audio. Empirical
   verification with a known disc + a known SHA-1 from redump is
   the diagnostic.

3. **What about `.chd` of multi-disc games?** Some operators
   pack each disc as its own `.chd`, some pack all discs of a
   set into one big `.chd` with a multi-disc TOC. The
   chd-crate API needs investigation.

4. **What does the operator see when a partial-match happens?**
   E.g. 4 of 5 tracks match a redump entry but one .bin's SHA-1
   doesn't. Options: flag as "possibly Foo (USA), audio track 3
   diverges from canonical" + don't stamp the title? Or treat as
   no-match? Both are reasonable; needs operator input.

5. **Where does the per-disc progress UI live?**
   Today's per-system "Identify ROMs" button has a progress bar.
   For disc-shape, do we keep the same progress bar but with
   slower advancement (each disc takes seconds), or do we add a
   per-disc nested progress (outer: "23 of 100 discs", inner:
   "Track 3 of 5")? Probably nested.

6. **How does this interact with the disc-id serial-lookup path
   we already have?** Two non-exclusive options: (a) per-track
   hash runs FIRST as the more reliable check; serial-lookup
   becomes the fallback for partial-hash matches. (b) Both run
   in parallel; if both match the same game, great; if they
   disagree, hash wins. Probably (a).

---

## Sizing

Rough phasing — ~3-4 weeks total:

- **Phase 1 — schema + parser extension** (~1 week):
  `rom_hashes_tracks` table + migration. Extend
  `parse_libretro_dat` to emit per-track rows. Bulk-replace
  function. Cache + TTL. Tests against synthetic dat input.

- **Phase 2 — per-track hashing engine** (~1 week):
  Per-track byte extraction for `.cue + .bin`, `.chd`, `.gdi`.
  Mode-aware sector unwrapping. Cancellable streaming SHA-1
  per track. Tests with synthetic disc images at each mode.

- **Phase 3 — Identify flow integration** (~1 week):
  New `resolve_disc_hashes_for_system` Tauri command (parallel
  to existing `resolve_rom_hashes_for_system`). Per-disc
  progress events. Library write that stamps canonical title +
  serial + per-track SHA-1 cache on the game row. Wizard +
  per-system Settings button wired.

- **Phase 4 — validation + docs** (~3-4 days):
  Operator playtest on real PSX / Saturn / Dreamcast folders.
  Hit-rate measurement (how many redump-cataloged dumps actually
  match? Goal: 95%+). Documentation in
  `docs/cores/<id>/README.md` for disc-shape systems. Update
  `docs/cores/SCHEMA.md` if new fields land on the games table.

---

## Risks

- **Track-hashing convention drift.** Redump's per-track SHA-1
  isn't documented as a clean convention — it's "whatever bytes
  redump's dumping process produced." If different game eras
  have different conventions, our matcher could miss valid
  dumps. Mitigation: verify against a small set of known-good
  dumps for each disc-shape system before kicking off Phase 3.

- **`.chd` TOC complexity.** The `chd` crate has a TOC accessor
  but per-track byte extraction may need additional work. Worst
  case: we re-encode `.chd` → raw `.bin` per track in-memory for
  hashing. Memory cost is the disc size, which on PS2 is bad.
  Mitigation: investigate chd-crate API + the libchdr C
  reference implementation before committing to Phase 2.

- **Hash compute time on big libraries.** A 500-game PSX
  library is ~5 GB of disc images = ~10 seconds of SHA-1 work
  per disc → ~90 minutes total. Operators may not have patience
  for that as a "click Identify and wait" UI flow. Mitigation:
  (a) cache per-track SHA-1s on the game row so re-runs are
  no-ops; (b) make Identify a background task they can leave
  running; (c) auto-trigger at scan time so first scan stamps
  everything.

- **`.iso` ambiguity.** `.iso` files for disc-shape systems
  (GameCube, PSP, PS2) are usually single-track, full-file
  user data. But some `.iso` dumps include the data track
  header bytes; redump's convention varies. Treat as edge case;
  start with the common shape.

- **Redump coverage isn't 100%.** Some homebrew + prototypes +
  obscure regional variants aren't in redump. These will
  legitimately not match. Operator-facing UX needs to handle
  this gracefully — "23 of 100 discs identified" is a
  reasonable shape; "0 of 100 — Identify failed" is not.

---

## Out of scope (won't do here)

- **Pre-burning the per-track SHA-1s into shipped data.** Same
  reasoning as the existing rom_hashes sync — we don't ship a
  copy of redump; we fetch on demand + cache.
- **Per-track repair / re-encoding.** We hash + match, we don't
  fix.
- **`.bin` files dropped without their `.cue` companion.**
  Already filtered out by the Slice 2 dedupe (data-track rows
  without a playlist sibling get dropped at scan time).
- **Disc-shape systems redump doesn't catalog.** 3DO is the
  notable case — no per-track redump data exists. These stay on
  the existing fuzzy-filename + serial-lookup path.

---

## When this arc starts

This plan is approved + queued (2026-06-02) but deferred. The
executing session should:

1. **Re-read this plan in full.**
2. **Confirm operator still wants the per-track approach** vs an
   alternative (whole-disc-canonical-form normalization, online
   lookup service, etc.). Per-track is the most-likely-correct
   path but worth a fresh check.
3. **Verify the track-hashing convention** against a small set of
   known dumps + redump dat entries BEFORE writing the matcher.
   Closing the question in §"Open design questions" #2 is the
   gate to Phase 2.
4. **Branch as `feat/disc-track-sha1-matching-phase-1`** per the
   standard workflow.
5. **Plan per-phase commits** per §Sizing — schema first, hashing
   engine second, Identify-flow third, validation fourth.

---

*Plan written 2026-06-02 after the Slice 2 closure + the
Dreamcast-classified-as-MAME fix landed. Operator framing:
"needed to help new users out" — disc-shape systems are the
weakest part of the canonical-identification story today, and a
new operator's first PSX folder import is the moment to fix it.*
