# Virtual library + preservation architecture + launcher-agnostic frontend

**Status:** Planning locked 2026-06-03. Three rounds of operator Q&A
(plus a ChatGPT + Gemini external proposal pasted into the session).
Execution starts Phase 0 immediately; Phase A queued behind operator
review of the foundation.

**Owner-of-decisions:** the operator. This document records the
decisions that came out of the refinement Q&A. Implementation should
follow them unless a code-time issue forces a revisit — in which case,
check back in here first.

Originating advisor-session source files:
[~/.claude/plans/iridescent-riding-cocke.md](off-tree backup of this plan).

---

## Context

OA today handles ROM identification + grouping + BIOS resolution +
metadata sync + per-system theming as a libretro-only frontend. The
foundation is more capable than the external AI advisors realized:

- 4-tier identification (Hash / Header / Extension / Hint) shipped in
  `apps/oa-shell/src/scan_service.rs`.
- Multi-region / multi-revision variant grouping shipped in
  `apps/oa-shell/src/library_groups.rs` with operator-pinnable default
  via the `game_group_defaults` SQLite table.
- Per-system descriptor consolidation
  (`config/systems/<id>/{system,bios,games}.yaml`) shipped 2026-06-02.
- Background jobs registry + persistent progress bar shipped 2026-06-03.
- BIOS resolution with per-file picker shipped 2026-06-01; grouped
  Issues / Ready UI shipped 2026-06-03.
- System Health Overview rollup shipped 2026-06-03.

Two strategic shifts pull the next arc:

1. **The virtual library + preservation depth the advisor proposal
   described needs to be promoted from "runtime view" to "schema model"**
   so per-group metadata, per-group artwork, cross-file search, and
   per-group stats track canonical games rather than individual file
   rows. The grouping logic exists; the schema doesn't yet match.

2. **OA's role is shifting from "premium libretro frontend" to
   "premium frontend for retro emulation, period"** — including
   standalone external emulators (Cemu / RPCS3 / Lime3DS / Ryujinx /
   Suyu / etc.) that don't ship as libretro `.dll`s. The 2026-05-16
   libretro-only DECISIONS entry + the 2026-06-02 PARKING_LOT
   plugin-API rejection are both being reversed here. The launcher
   abstraction needs to be in the architecture before the variant
   model + per-game settings model crystallize on libretro-only
   assumptions.

The intended outcome: every ROM gets the cleanest possible canonical
identity (Tier 1–5 chain, full tag decode), every game shows up as one
parent in the Casual view, every variant is preserved + filterable in
the Preservation view, every system launches through a unified
abstraction whether the core is a libretro `.dll` or an external
standalone process.

---

## Strategic decisions locked

### S1 — Reversal of "libretro-only" architecture

The 2026-05-16 DECISIONS.md entry locking OA into a libretro-only
frontend is **reversed**. OA will support both libretro cores AND
external standalone emulators via a `Launcher` trait abstraction. A
new DECISIONS entry records the reversal + the new rationale (vendor
coverage gap: Wii U / PS3 / 3DS / Switch / modern Mac emulation
targets have no production-grade libretro path).

### S2 — Legal posture for external emulator install

OA downloads + sets up emulator binaries where legally clean (plugin
profiles point at each emulator's official release endpoint:
GitHub Releases for Cemu / RPCS3 / Lime3DS / etc.; vendor sites where
applicable). OA **never** downloads or installs ROMs or BIOS files.
"Emulation is legal; redistribution of copyrighted ROMs / BIOS is not
unless the user owns them, and OA cannot guarantee that." Plugin
profiles update on a configurable cadence (default weekly check, manual
override).

### S3 — Schema promotion is centerpiece, but identification depth precedes it

Phase order pulls schema promotion (Phase E in the original sketch)
up to second position behind identification depth (Phase A). Reason:
deferred-schema-decision risk — every later phase reads cleaner if the
`game_identities` schema is the canonical model. UX layer + launcher
abstraction + Preservation Vault all hang off it.

### S4 — Per-variant artwork stays a feature

When the schema promotes to `game_identities`, per-group MediaDb keys
get added for canonical artwork (the parent "Pokémon Red" cover used in
Casual mode), but **per-variant artwork is preserved**. Japan boxart,
USA boxart, European boxart all retained per-file. The Casual view
picks the canonical; the Preservation view shows variant-specific.
Multi-region boxart is a *feature* for collectors, not a deduplication
target.

### S5 — Documented-decision-reversal lands as Phase 0

A `docs/DECISIONS.md` entry and a `docs/PARKING_LOT.md` update fold in
as Phase 0 (paperwork-only) so the architectural shift is explicit.
Without this, the next contributor reads CLAUDE.md ("**libretro is the
only FFI boundary**") and assumes the old stance is still in force.

### S6 — Identity scope: per-system

`game_identities` is keyed strictly per-system (`(system_id,
normalized_title)`). "Pokémon Red Game Boy" and "Pokémon Red Switch
Online" are separate identities. Cross-system identity model deferred
to a possible v2 migration if operator demand surfaces.

### S7 — Two-mode UX is a global toggle

Casual / Preservation is one OA-wide pref in Settings → Display →
Library presentation. No per-system or per-collection overrides in v1.

### S8 — External-emulator v1 pilot trio

Phase C/D ships **Cemu** (Wii U) + **RPCS3** (PS3) + **Lime3DS** (3DS).
Each has well-known official release endpoints, broad system coverage,
and clean redistribution stance. Ryujinx / Suyu / Dolphin standalone
expand after operator validates the abstraction end-to-end.

---

## Phases

### Phase 0 — DECISIONS reversal + plan commitment (~1 day)

Paperwork only. Adds entries to:
- `docs/DECISIONS.md` — reversal of 2026-05-16 libretro-only
  locking decision; new rationale (vendor coverage gap, operator
  request).
- `docs/PARKING_LOT.md` — un-park the 2026-06-02 plugin-API rejection;
  note the install pipeline isn't a generic plugin API (different shape).
- `CLAUDE.md` — soften "libretro is the only FFI boundary" to
  "libretro is the primary FFI boundary; external-process emulators
  via the Launcher abstraction" — and reference the new DECISIONS
  entry.
- This plan file lives at `docs/PLANS/virtual-library-and-launcher-arc.md`
  so it persists across context-cleared restarts.
- `docs/ACTIVE_WORK.md` — add the new in-flight stream.
- `docs/NEXT.md` — reshuffle: disc-track SHA-1 stays HIGH (folds into
  Phase A); MAME parent/clone bridge moves out of LOWER into Phase A.

### Phase A — Identification depth (~3–4 weeks)

**Goal:** every ROM gets the cleanest possible canonical identity.
Tier 1 fully closes for cart + disc systems; tag decoding gives the
Preservation Vault its filterable fields.

**Slices:**

- **A1 — Disc-track SHA-1** (~1 week): ship the queued plan at
  `docs/PLANS/disc-track-sha1-matching.md`. Hashes data tracks for
  `.cue+.bin` / `.chd` / `.gdi` / `.iso`, matches against redump's
  per-track SHA-1 fields, stamps canonical title on the library row.
  Closes Tier 1 for PSX / Saturn / Sega CD / Dreamcast / NeoCD /
  PCE-CD / PC-FX / 3DO / GameCube / PSP / PS2.

- **A2 — Filename tag decode** (~3–4 days): extend
  `apps/oa-shell/src/title_parse.rs` with a decode table for
  bracket flags. Net-new typed fields on `ParsedTitle`:
  `dump_status: { Verified, BadDump, Unknown, OverDump, Fixed }`,
  `is_hack: bool`, `is_translation: bool`, `is_pirate: bool`,
  `is_bios: bool`, `is_homebrew: bool`,
  `translation_languages: Vec<String>` (parsed from `[T+Eng,Fra]`).
  Decode table covers No-Intro + GoodSet + TOSEC conventions.
  Library_groups.rs adds filter predicates on the new fields.

- **A3 — Tier 5 deep-dive** (~1 week): for ROMs that miss all four
  existing tiers, attempt structural matching. Slices:
  - Internal title-string extraction at known header offsets
    (SNES $FFC0 21-byte ASCII, Genesis $0150 / $0120, PSX exec
    header). Match against `mame_games` + libretro-database title
    lists.
  - Generic archive introspection (`.zip` / `.7z` / `.rar`) — peek
    manifest, sum file sizes, identify nested ROMs (today only Neo
    Geo does this).
  - Trimmed-CRC32 fallback (strip header + padding, try canonical
    hash against trimmed form).
  - Filesize + extension as a soft tiebreaker rank.

- **A4 — MAME parent/clone bridge** (~3 days): wire `mame_games.cloneof`
  into `library_groups.rs` so MAME clones (`mspacman` = clone of
  `pacman`) optionally group under the parent canonical title.
  Operator toggle in Settings → System Health → MAME ("Group clones
  under parents").

**Exit criteria:**
- Operator imports a folder; every ROM lands with the most specific
  identity possible (canonical title where DAT-matched, otherwise
  structural-match candidate with confidence score).
- The Confidence pill in the Import Wizard results table extends from
  4 levels to 5 (Hash / Header / Extension / Hint / Deep-dive).
- Background Jobs surfaces the Tier 5 deep-dive pass when triggered;
  pause / resume / cancel work uniformly.

### Phase E — Schema promotion to game_identities (~3–4 weeks)

**Pulled forward per operator decision 2026-06-03.** Without this,
later phases keep building on the per-file model.

**Schema migration v18+ → v19:**
- New `game_identities` table — `(id PK, system_id, canonical_title,
  normalized_title, year, genre, developer, publisher, players,
  rating, canonical_cover_path)`. Keyed by `(system_id,
  normalized_title)`. Replaces `game_group_defaults` (which becomes
  a foreign-key column on `games`).
- `games` table gains `identity_id` FK column. Population: on
  migration, run `library_groups::build_groups` once; bucket entries
  into identities; insert rows.
- `media.json` MediaDb gains a parallel `identity_media` keyspace for
  canonical (parent) artwork. Per-file MediaDb stays — variants keep
  their own art (S4 decision).

**Code changes:**
- `crates/oa-libretro::LibretroCore` unchanged (operates on a single
  ROM path).
- `apps/oa-shell/src/library_db.rs` — new identity CRUD; games-table
  reads JOIN onto identities for canonical metadata.
- `apps/oa-shell/src/library_groups.rs` — `GameGroup` is now backed
  by an identity row rather than computed in-memory. Ranking logic
  preserved.
- `frontend/src/components/LibraryTile.tsx` + grid — render from
  identities; per-variant launch via the existing `Run version ▸`
  submenu.
- `frontend/src/components/GameDetailPanel.tsx` — header shows
  canonical metadata; existing variant tab shows per-variant
  metadata.
- Cross-file search now keys on canonical title.
- Per-identity stats: play_time aggregates across variants;
  favorite + completed apply to the identity (toggleable per-variant
  via Preservation view).

**Exit criteria:**
- Existing libraries migrate cleanly (idempotent — re-run produces
  same identity ids).
- Every library tile renders from an identity row.
- Per-identity metadata, per-variant metadata, per-identity stats
  all queryable + editable.
- 660+ oa-shell tests stay green throughout.

### Phase B — Two-mode UX + Collection Health (~2 weeks)

**Two-mode toggle:**
- New OA-wide pref — `library_mode: "casual" | "preservation"`.
  Default Casual. Stored in `settings.json`; togglable from
  Settings → Display → Library presentation.
- Casual mode — library grid renders one tile per identity; the
  default variant launches on click. Right-click → Run version ▸
  still works for power users who want a specific variant.
- Preservation mode — library grid shows multi-variant identities
  with an explicit variant ribbon (e.g., `[USA] [JP] [EU] [Rev A]`
  inline on the tile). Variant-tree expander on GameDetailPanel
  becomes the primary view.

**Variants tab on GameDetailPanel:**
- New tab adjacent to the existing Overview / Game info / Controls.
- Lists every variant under this identity with: region / revision /
  dump_status / hack / translation flags from Phase A2 decode.
- Per-variant: launch button, mark-as-default button (writes
  `game_identities.default_variant_id`), MediaDb thumbnail.
- Filter affordances (dump_status, region, language) for
  Preservation users.

**Collection Health:**
- Extend `SystemHealthPage::OverviewBody` with three new rollup
  cards: % verified dumps (computed from `parsed_title.dump_status`),
  % with covers (existing per-system stat rolled up), % with
  metadata (existing per-system stat rolled up).
- Each card has a CTA that scrolls to a per-system breakdown grid
  (analogous to the Per-system readiness checklist already there).
- Tracking incentive: dashboard numbers tick up as the operator
  cleans up bad dumps + sources missing covers.

**Exit criteria:**
- Library tile in Casual mode shows "Pokémon Red" without region /
  revision noise.
- Same library in Preservation mode shows every variant explicitly.
- Collection Health dashboard renders three new rollup cards with
  real % numbers + drill-in CTAs.
- 660+ oa-shell tests + frontend typecheck silent.

### Phase C — Launcher abstraction (~2–3 weeks)

**The critical architecture work.** Refactor `crates/oa-core::Core`
trait into a `Launcher` trait that supports both libretro-loaded
cores AND external-process emulators.

**Trait shape (provisional):**
- `prepare(rom_path, system_id) -> LaunchPrepared` — pre-flight
  validation, BIOS resolution, controller binding setup.
- `launch(prepared) -> LaunchedSession` — boots the core / spawns
  the process; returns a session handle.
- `is_alive(session) -> bool` — for external processes, polls the
  child PID; for libretro, always true while loaded.
- `terminate(session)` — graceful shutdown; for external, sends
  shutdown signal then SIGKILL fallback after timeout.
- Per-launcher capability flags: `supports_rewind`, `supports_savestate`,
  `supports_run_ahead`, `supports_input_remap`, etc. Today's QuickSettings
  toggles gate themselves on launcher capabilities.

**Two impls:**
- `LibretroLauncher` — wraps today's `oa-libretro::LibretroCore`.
  Behavior preserved bit-for-bit.
- `ExternalProcessLauncher` — spawns a configured emulator binary
  via `tokio::process::Command` with the ROM path argument.
  Captures stdout / stderr to the OA debug log. Stays alive until
  the process exits or operator terminates.

**Profile registry:**
- New `apps/oa-shell/src/emulator_profiles.rs` — `EmulatorProfile`
  struct describing one external emulator. Fields:
  `id, display_name, vendor, official_download_url, binary_name,
  supported_systems: Vec<SystemId>, launch_args_template,
  settings_dir_relative, capabilities: LauncherCapabilities`.
- Profiles ship in `config/emulators/<id>.yaml` (mirror the per-system
  descriptor pattern). 2026-06-03 v1 pilot set: Cemu (Wii U), RPCS3
  (PS3), Lime3DS (3DS).
- Operator can edit per-emulator profile YAML; reload-on-restart.

**Per-system default launcher pref:**
- Settings → Per-system → Default launcher (libretro core vs.
  external emulator). Falls through to OA-wide default if unset.

**Exit criteria:**
- Operator picks an external emulator profile, points it at an
  installed binary (Phase C does NOT yet auto-install), launches
  a game.
- Variant tree from Phase B works identically whether launching via
  libretro or external (one parent `Pokémon Red`, one click,
  external emulator process spawns).
- QuickSettings toggles correctly disable for capabilities the
  external launcher doesn't support (e.g., rewind grayed out for
  Cemu).

### Phase D — External emulator install pipeline (~2–3 weeks)

**Install profile shape:**
- `EmulatorProfile.installable: Option<InstallableProfile>` —
  per-emulator install metadata.
- `InstallableProfile { download_url_template, asset_pattern,
  install_method: { Portable, Installer }, version_check_url,
  current_version, last_checked }`.
- Profiles fetch their own latest release URLs from each emulator's
  official endpoint (GitHub Releases API for most; vendor download
  endpoint where applicable).

**Install flow:**
- Settings → Emulators category (or sub-tab under System Health) —
  one card per installable emulator profile.
- Per-card: current version status + "Install" / "Update" / "Open
  install folder" buttons. Background-Jobs-tracked download +
  extraction.
- Default install location: `<exe_dir>/Emulators/<id>/` per the
  existing pattern (MAME already lives there).

**Plugin-style updater:**
- Configurable update cadence (default: weekly check, manual override).
- Update notifications surface via the BackgroundJobsBar + a
  notification chip in the Emulators settings card.
- Operator can pin a specific version (prevents auto-update from
  breaking their setup).

**Legal posture (S2 reaffirmed):**
- Profiles for emulators with clear official redistribution rights
  ship in v1.
- Emulators with ambiguous redistribution stance (e.g., some Switch
  emulators may not survive future legal challenges) require explicit
  operator action: "Point OA at an existing install" rather than
  auto-fetch.
- Zero ROM downloads. Zero BIOS downloads. Per-emulator install
  cards include explicit "OA does not provide ROMs or BIOS files"
  language.

**Exit criteria:**
- Operator clicks "Install Cemu" → Background Jobs runs the download
  → emulator binary lands in `<exe_dir>/Emulators/cemu/`.
- Library plays a Wii U title through the installed Cemu via the
  `ExternalProcessLauncher`.
- Update cadence + version pinning work.

### Phase F — Preservation Vault polish (~1–2 weeks)

**New dedicated surface.** Either a new top-level Retroverse tab OR
a sub-section under Library. Operator picks at execution time.

**Vault features:**
- Tree view rooted at canonical identity → variant rows.
- Filter ribbon: dump_status (Verified / Bad / Unknown / Over /
  Fixed), is_hack, is_translation, is_homebrew, is_pirate, regions,
  languages, dump-source attribution.
- Per-variant editorial action menu: mark-as-canonical, override
  metadata, attach custom note, flag for re-dump, etc.
- Bulk operations: "Run Tier 5 deep-dive on every Unknown",
  "Export variant inventory to JSON", "Show stats: 14 bad dumps
  across 9 systems."

**Builds on:**
- Phase A2 decoded tags (filter fields).
- Phase E identities + per-variant rows (data source).
- Phase B Casual / Preservation toggle (mode-aware default landing).

### Phase G — `crates/oa-preserve` workspace split (~1–2 weeks)

**Refactor.** Extract identification + grouping + DAT parsing into a
standalone crate so the engine can later be wrapped by an
`oa-preserve-cli` or other consumers.

**Move into `crates/oa-preserve`:**
- `apps/oa-shell/src/scan_service.rs`
- `apps/oa-shell/src/rom_header.rs`
- `apps/oa-shell/src/rom_hashes.rs`
- `apps/oa-shell/src/title_parse.rs`
- `apps/oa-shell/src/library_groups.rs`
- `apps/oa-shell/src/cd_id.rs`
- `apps/oa-shell/src/mame_games.rs`
- `apps/oa-shell/src/system_registry.rs` (probably — close to the line)

**Keep in `apps/oa-shell`:**
- Tauri commands wrapping the crate (consumer-facing IPC layer).
- `library_db.rs` (SQLite-specific; depends on app-level migrations).
- `system_info.rs` (mixed concern).

**Exit criteria:**
- `cargo build -p oa-preserve` succeeds standalone (no Tauri deps).
- 660+ oa-shell tests still pass.
- Public API documented enough that a future CLI can wrap it.

### Phase H — `oa-preserve-cli` (deferred — back burner)

Per operator direction 2026-06-03 — defer to a later cycle. No
implementation here; the workspace split in Phase G keeps it possible.

---

## Critical files (anchor points for execution)

- `apps/oa-shell/src/library_db.rs` — schema migrations + games / identities tables
- `apps/oa-shell/src/library_groups.rs` — variant grouping + ranking
- `apps/oa-shell/src/title_parse.rs` — canonical-title parser + tag decode (Phase A2)
- `apps/oa-shell/src/scan_service.rs` — Tier 1–5 identification pipeline
- `apps/oa-shell/src/rom_header.rs` + `rom_hashes.rs` + `cd_id.rs` — Tier 1–2 + disc serial
- `apps/oa-shell/src/mame_games.rs` — MAME parent/clone bridge target (A4)
- `apps/oa-shell/src/system_registry.rs` — per-system descriptor loader
- `crates/oa-core/src/lib.rs` — `Core` trait → `Launcher` trait refactor (Phase C)
- `crates/oa-libretro/src/lib.rs` — `LibretroLauncher` impl
- `apps/oa-shell/src/emulator_profiles.rs` (new — Phase C/D)
- `config/emulators/<id>.yaml` (new — Phase C profiles)
- `frontend/src/components/LibraryTile.tsx` + `LibraryGrid.tsx` — render from identities (Phase E)
- `frontend/src/components/GameDetailPanel.tsx` — Variants tab (Phase B)
- `frontend/src/routes/retroverse/SystemHealthPage.tsx` — Collection Health rollups (Phase B)

---

## Verification

- After each phase: `cargo test -p oa-shell` (660+ tests) + frontend
  `npm run typecheck` silent + operator smoke playtest of the
  visible-surface changes before merge.
- Schema migrations (Phase E) require an extra step: take a snapshot
  of an existing operator-built `library_db.sqlite`, run the migration
  on a copy, confirm idempotent + lossless. Add a SQL fixture test
  that asserts the post-migration shape.
- Phase D install pipeline: integration-test the actual download +
  extraction for at least one emulator (Cemu suggested — open-source,
  clean licensing, well-known release endpoint).
- Phase C launcher abstraction: regression-test that every existing
  libretro path still works byte-for-byte after the trait refactor.

---

## Open questions deferred to execution time

These don't gate the plan; lock at the start of the relevant phase.

- **Phase F surface placement** — Preservation Vault as a new
  top-level Retroverse tab, or a sub-section under Library, or only
  reachable via the Preservation-mode toggle from Phase B. IA
  decision that benefits from seeing the Phase B + Phase E surfaces
  built.

- **Phase A confidence-pill UI** — fifth tier ("Deep-dive") shows
  as a fifth distinct pill, or as a badge layered on the existing
  four. Visual decision; small.

- **Phase G crate-split scope** — does `system_registry.rs` move
  into `oa-preserve` (close to the line) or stay in `oa-shell`?
  Decide when Phase G is sized.

---

## Reference

- 2026-06-03 advisor proposal (ChatGPT + Gemini chained) — pasted
  into the session that locked this plan. The "Casual vs Preservation
  user" framing + the Global Game Identity concept + the
  launcher-abstraction reframing came out of that conversation.
- 2026-05-16 DECISIONS — libretro-only stance, **reversed in Phase 0**
  (see new DECISIONS entry).
- 2026-06-02 PARKING_LOT — Plugin/Extension API rejection,
  **partially un-parked in Phase 0** (install pipeline is not a generic
  plugin API; per-emulator profile shape is constrained).
- `docs/PLANS/disc-track-sha1-matching.md` — folds into Phase A1.
- `docs/PLANS/per-system-descriptors.md` — Slices 1+2 shipped; Slice 3
  (L3 content packs + L4 SQLite + JSON Schema) is independent and can
  pipeline alongside Phase A.
