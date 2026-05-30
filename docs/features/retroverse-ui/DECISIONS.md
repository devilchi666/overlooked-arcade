# Retroverse UI — Decisions Log

Append-only. Newest at the bottom. Per-stream decisions; project-wide
decisions live in `docs/DECISIONS.md`.

---

## 2026-05-29 — Unified controller pipeline: DPad transfers, stick walks

**Decided.** The DPad and the left stick carry distinct semantics across
the Retroverse shell. DPad LEFT/RIGHT transfers focus between regions
(sidebar ↔ center ↔ right pane). The stick walks within a region.
DPad UP/DOWN walks within a vertical region too (no neighbour
declared), so a stickless controller stays usable.

**Why.** Operator complaint mid-2026-05-29 session: "DPad does nothing
on LIBRARY." Root cause was a Phase 5 fix that had over-rotated and
made BOTH sources walk-only. The earlier "shoulder bumpers transfer,
DPad walks" model had been operator-rejected in favor of "DPad is the
transfer gesture you reach for." Once we tried to wire stick-only
expand-collapse on container rows it collided with DPad transfer
again — surfaced the need for source-awareness in the framework.

**What we considered and rejected.**
- Shoulder-bumper transfers (rejected pre-merge — DPad is faster).
- Single-direction handler with no source info (rejected when stick
  expand/collapse needed to coexist with DPad transfer).
- `onStickDirection` separate callback (rejected — overhead vs.
  passing `source` to the existing handler).

**Implementation.** `nav/focus.ts` `onDirection` signature widened to
`(direction, currentIndex, source: "dpad" | "stick-left") => boolean`.
Handlers gate on source when behavior should differ; existing handlers
(legacy LeftSidebar internal group, QuickSettings RewindScrubber)
ignore the new arg and continue firing for both sources unchanged.

Cross-reference: [[retroverse-controller-nav-spec]] memory; merged in
`a211a2e` (menu/dialog polish branch) + refined in `021852f` (stick-
only gate fix).

---

## 2026-05-29 — Custom collections: two-table schema with FK CASCADE

**Decided.** Slice 12's persistence uses two SQLite tables —
`custom_collections` (parent rows, ordered) + `custom_collection_members`
(junction, composite PK on (collection_id, rom_id), FK ON DELETE
CASCADE to the parent). Member rom_ids are NOT FK-linked to `games`
— rom rows come and go independently of operator curation.

**Why.**
- Junction shape lets a future drag-reorder land on a `sort_order`
  column without rewriting parent rows.
- FK CASCADE on the parent side means `DELETE FROM custom_collections
  WHERE id = ?` cleans up memberships in one statement.
- No FK on the games side means a `delete_game` call doesn't need to
  worry about dependent membership rows blocking it — the membership
  becomes orphan, swept on the next `delete_game` (via explicit
  `DELETE FROM custom_collection_members WHERE rom_id = ?`) and
  filtered out at list-time via INNER JOIN against `games`.

**What we considered and rejected.**
- Single denormalized JSON blob on the parent (rejected — drag-
  reorder and "is X in collection Y" lookups want indexed access).
- FK from members → games (rejected — would block legitimate game
  deletes; the orphan-tolerant model is simpler).

**Implementation.** v14 migration in `library_db.rs`. Member-count
queried at list-time via LEFT JOIN against games so dangling
memberships from deleted games don't inflate the sidebar count.
Merged in `d07cdff`.

---

## 2026-05-29 — Per-system SETTINGS section components shared with legacy dialog

**Decided.** The per-system override section JSX (Display / Rewind /
Shaders / Default-core) lives in `components/perSystemSections.tsx`
as four standalone components plus a `usePerSystemOverrides` hook
that owns the fetch + patch cycle. Both the legacy `SystemSettingsDialog`
(one-section-at-a-time modal) and the new Retroverse
`PerSystemSettingsBody` (all four inline on one scroll page) consume
the same component set.

**Why.** The override semantics are load-bearing — bugs would
otherwise have to be fixed in two places. The legacy dialog is being
phased out alongside the flag-deprecation endpoint but stays the
canonical surface for legacy-Shell operators until then; both code
paths need to behave identically.

**What we considered and rejected.**
- Copy the JSX into a new Retroverse component (rejected — two
  copies of the patch helper is a maintenance trap).
- Lift the entire dialog into the Retroverse pane and drop the
  legacy dialog (rejected — legacy Shell still needs it until flag
  deprecation).

**Implementation.** Section components take an `api` prop (the hook
result) + the OA-wide settings store + section-specific resources
(monitors / cores) as accessors so the parent owns the lazy-fetch
decisions. Merged in `406527d`.

---

## 2026-05-29 — DISCOVER v1: 4 data-driven axes + 5 stubs awaiting C6

**Decided.** DISCOVER ships with 4 axes powered by the existing
MediaDb metadata (By era / By genre / By publisher / By developer)
plus 5 axes (Featured / On this day / System dive / Cult classics /
Lost games) that render an empty-state card pointing at Phase C6
content-packs. Sidebar surfaces all 9 axes; the stubs dim to 60% with
a "soon" pill so the operator sees at a glance what's available now.

**Why.** The full DISCOVER design (per
`docs/PLANS/discover-tab-retroverse.md`) needs editorial content
that ships via Phase C6 content packs. Holding the entire tab as a
StubPage until C6 lands would leave a tab unreachable for weeks;
shipping the data-driven half means operators get a functional
"explore the library" surface immediately.

**What we considered and rejected.**
- "By region" axis (rejected — `GameMetadata` carries publisher but
  not region; region is per-cover-art-variant in `MediaVariant`).
- Reading axes from `games`-table columns (year/genre/developer)
  (rejected after the post-merge bug pass — those columns exist in
  the v1 schema but metadata sync never writes to them; the
  canonical source is MediaDb).
- Wait for C6 (rejected — multi-week block on a useful surface).

**Implementation.** `DiscoverPage.tsx` reads facets from
`useMedia().media(romId)?.metadata`. Era bucketing uses 7 hardcoded
hardware-generation ranges; entries without a `year` land in
"Unknown / not enriched." Merged in `b2fe206`; metadata-source
correction landed in the same merge after the post-merge playtest.

Cross-reference: [[reference_metadata_lives_in_mediadb_not_games_table]].

---

## 2026-05-29 — Now-playing chip tracks dispatch, not Rust-side state

**Decided.** The HintBar now-playing chip's source signal
(`lib/audio.ts::nowPlaying`) is written by the frontend dispatch
helpers (`dispatchPlatformMusic` on non-null resolve;
`stopAudio("platform-music")` on explicit stop). It tracks what we
ASKED the Rust side to play, not what the rodio mixer is actually
running.

**Why.** Rust doesn't currently emit "playback failed" or "track
ended" events back to the frontend. Implementing a feedback channel
is its own arc. The dispatch-tracking model is correct in the
happy path (Rust opens the file, music plays, chip displays); the
edge case where Rust fails to open the file leaves the chip
displaying for a few seconds with no audible audio, which is
visually misleading but not destructive.

**What we considered and rejected.**
- Subscribe to a future Rust event (deferred — needs an event +
  emitter on the audio_player side; out of scope for the small
  chip).
- Poll a `get_audio_state` command at 1 Hz (rejected — IPC churn
  for a passive UI cue).

**Implementation.** `nowPlaying` accessor exported from
`lib/audio.ts`; HintBar subscribes. Merged in `cbbd818`.

**Superseded same-day** — see next entry.

---

## 2026-05-29 — Now-playing chip subscribes to Rust playback-failed event

**Decided.** Supersedes the prior "tracks dispatch only" decision.
The audio thread now holds an `Option<AppHandle>` and emits
`oa://audio-playback-failed { bus, reason }` on file-open / decode
/ sink-alloc failures. Frontend `lib/audio.ts` listens at module
load and clears the `nowPlaying` signal when payload.bus matches
`"platform-music"`. The chip disappears at the moment playback
actually fails rather than sitting on indefinitely.

**Why.** The prior decision noted "needs an event + emitter on the
audio_player side; out of scope for the small chip." Operator
revisited that scope decision in the same session — the wiring is
small (~30 lines Rust + ~20 lines frontend) and the failure case
is operator-visible enough to justify it now rather than later.

**What we considered and rejected.**
- Poll a `get_audio_state` command at 1 Hz (still rejected — IPC
  churn for a passive UI cue, especially now that the event path
  exists).
- Emit a generic "audio-state-changed" event with the full bus
  state (rejected — bigger payload, more churn; the chip only
  cares about failures).
- Track per-bus playback state in Rust as the canonical source +
  expose via a query command (rejected — same churn problem, plus
  the dispatch-tracking model is correct in the happy path).

**Implementation.** `audio_thread_main` gains an `emit_failed`
closure. `AudioPlayerHandle::spawn` widened to
`spawn(Option<AppHandle>)`; `None` keeps emission off for headless
test contexts. Merged in `388a90a`.
