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

### S9 — Media-on-disk: beside-the-ROM convention (Option B) + relative-path portability

**Operator decision 2026-06-11** (out of the LaunchBox competitive
research — `docs/features/guided-setup/LAUNCHBOX_RESEARCH_2026-06-11.md`
§10 Q1). Game media (boxart, clear logo, screenshots, fanart, video
snaps, manuals) is laid out **beside the ROMs following a
community-standard convention** (`<rom-dir>/media/<type>/<rom-basename>`,
ES-DE / ScreenScraper / Pegasus dialect — exact dialect TBD in S9a),
**not** a private app-managed media tree à la LaunchBox. Rationale:
- **Free scraper interop** — Skraper, ARRM, and ScreenScraper-based
  tools write directly into this layout, so OA inherits best-in-class
  third-party scraping without building it.
- **Portability** — art travels *with* the ROMs when a user moves a
  system folder between drives, between OSes (Windows desktop ↔ Linux
  cabinet), or desktop → cabinet. No export / re-link step.
- **DB-rebuild resilience** — art is resolved by convention, not a
  stored per-id path, so a fresh DB on a new machine re-discovers all
  art automatically.

This sits *on top of* the shipped `game_identities` + MediaDb keyspace
(Phase E, 2026-06-07): MediaDb becomes the **resolution + override +
canonical-art** layer over a convention-discovered base, not the sole
source of truth for where files live. OA-owned art with no on-disk
convention home (operator-pasted, AI-generated) stays in a managed
cache as a fallback tier — **hybrid: convention first, managed cache
second** (mirrors the S5.1 theming asset cascade shape).

**Hard constraint that falls out — a roots model, not absolute paths.**
"Ultra-portable off SQLite" does NOT mean "everything under the program
folder." ROMs commonly live on a separate drive, an external disk, or a
NAS/network share. The portable design is a **library-roots model**:
- The user registers one or more **roots** (content folders): local,
  external, or network (`D:\ROMs`, `\\NAS\retro`, `/mnt/nas/roms`).
- Every library entry is stored as **`(root_id, path-relative-to-root)`**,
  never an absolute path. The DB content is fully location-independent;
  the ONLY machine-specific state is a tiny **root → absolute-location
  mapping** (a few rows). Re-point one root → every ROM + every piece of
  art under it re-resolves at once. (Same shape as ES-DE `%ROMPATH%` /
  Pegasus per-collection dir / LaunchBox per-platform Games path.)
- **Media beside the ROM composes perfectly with NAS-hosted libraries** —
  art lives on the NAS with the ROMs and is shared across every machine
  that mounts it; a new cabinet just registers the root and re-discovers
  all art by convention.

Edge cases the resolver MUST handle (engineer around them — several are
classic LaunchBox-style bugs):
- **Root offline / NAS asleep / unmounted at launch** → mark entries
  **Unavailable**, NEVER delete. The "scan for removed ROMs" sweep must
  distinguish *root-unreachable* from *file-deleted* or it will purge a
  whole NAS library because the drive was spun down.
- **Drive-letter / mount change** → detect unreachable root, prompt
  re-point (one click re-resolves all). On Windows, optionally track the
  **volume GUID/label** to auto-relocate an external drive across letter
  changes.
- **Cross-OS root syntax** (`\\NAS\retro` ↔ `/mnt/nas/retro` ↔
  `smb://…`) lives only in the root row, re-mappable per machine — which
  is *why* roots are indirected rather than baked into every entry.
- **Network hashing is slow** → hash cache keyed by `(path, size, mtime)`
  so unchanged files aren't re-hashed each scan.
- **Forbidden:** the SQLite DB itself on a NAS for multi-machine sharing
  (SMB/NFS locking → corruption). Shared ROMs+media on NAS = fine; each
  machine keeps its OWN DB (its own stats/favorites) pointing at the
  shared roots.

Action items (land with the scraping/media slice + a portability pass):
- **Audit current path storage** (`games.cover_path`, identity
  `canonical_cover_path`, ROM application paths, BIOS resolution): are
  they absolute, single-relative, or already root-indirected? Migrate to
  the `(root_id, relative)` model. *(Verify — don't assume.)* Confirm
  whether OA already persists a multi-root concept or scans ad-hoc
  folders.
- Standardize media-folder **casing** on the chosen convention's exact
  spelling (Linux cabinets are case-sensitive; Windows isn't).
- Library DB + settings travel via the existing **portable mode**
  (`portable.txt` marker, `features/portable-install/`); per-user-only
  state may stay in appData when not portable. The roots table is the
  one piece that's intentionally machine-local.

**Open sub-decisions (S9a — defer to the scraping/media-management slice):**
- Exact convention dialect (ES-DE `media/` vs Pegasus
  `media/<type>/<game>` + `x-` keys vs ScreenScraper) — pick widest
  tool support; consider reading more than one on import.
- Whether OA *writes* the convention itself (own scraper) or only
  *reads* it (delegate scraping to Skraper/ARRM) in v1. **Operator
  leaning 2026-06-11 (LaunchBox research §10 Q4, undecided):** possibly
  **curated lists + a custom external scraper tool, self-hosted on a
  git repo** (the libretro-database / libretro-thumbnails model OA
  already consumes) rather than a live multi-source scraper. Not locked.
- Multi-region variant art (S4) composes (variants have distinct
  rom-basenames) but confirm under the chosen dialect.

### S10 — External-emulator integration: launch-and-return baseline + deep per-emulator profile control (NOT window-embedding)

**Operator decision 2026-06-11** (LaunchBox research §10 Q3). How OA
integrates external standalone emulators (the Launcher-trait / S8 pilot
trio Cemu / RPCS3 / Lime3DS, and beyond):

- **Windowing baseline = launch-and-return.** OA launches the external
  emulator as its own process/window; on exit, control returns to OA.
  Robust + cross-platform (roughly today's `ExternalProcessLauncher`).
  **Borderless-fullscreen takeover** (emulator visually fills the screen
  via a launch ceremony, no desktop flash) is the natural later *visual*
  upgrade — same model, nicer handoff.
- **True window embedding / reparenting is REJECTED** as a baseline.
  Hosting a foreign emulator window inside OA's window (Win32
  `SetParent`, X11 XEmbed) is fragile per-emulator, ~impossible on
  Wayland/macOS, and fights OA's cross-platform pillar. (Contrast
  libretro cores, which OA renders *truly* in-window via wgpu — that's
  why libretro stays the primary boundary; only libretro gets real
  in-window integration.)
- **The depth lives in per-emulator integration profiles, not in the
  window.** Each supported external emulator gets an OA profile (data +
  plugin/script — emulator-definitions-as-data, report rec #6 /
  Playnite-shaped) that lets OA **install, auto-configure, take over
  settings, and manage save files** for that emulator — "control as much
  of its functions as we can." A profile knows: official release
  endpoint (S2), install layout, launch-arg template, where its config +
  saves live, how to apply OA's controller / per-game settings into the
  emulator's own config, and how to surface its saves in OA's save
  handling. This is where OA decisively beats LaunchBox (auto-configures
  only RetroArch + a fixed list; never takes over settings/saves).

**Sequencing (operator triage of the §10 "general features" list,
2026-06-11):** external-emulator integration = **priority 1**;
**multi-user profiles** (expand OA's existing profile support) =
**priority 2**; **cross-machine sync, stats dashboard, and netplay are
parked.**

**How / open for next pass (records the "what", not the "how"):** profile/
manifest schema (extends `config/emulators/<id>.yaml`); the
settings-takeover + save-file-management seam; per-system (not
per-emulator) archive extraction + prominent bulk-capable per-game
overrides (report rec #6); reconcile with `oa-savestate` + a save-vault
UX (LaunchBox Save Management is the reference). Borderless takeover
scheduled after the launch-return baseline proves out.

---

## Phases

### Phase 0 — DECISIONS reversal + plan commitment (~1 day) — ✅ SHIPPED

**✅ Merged to main** (the DECISIONS reversal + plan commitment landed).

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

- **A1 — Disc-track SHA-1** ✅ **shipped** (~1 week): ship the queued plan at
  `docs/PLANS/disc-track-sha1-matching.md`. Hashes data tracks for
  `.cue+.bin` / `.chd` / `.gdi` / `.iso`, matches against redump's
  per-track SHA-1 fields, stamps canonical title on the library row.
  Closes Tier 1 for PSX / Saturn / Sega CD / Dreamcast / NeoCD /
  PCE-CD / PC-FX / 3DO / GameCube / PSP / PS2.

- **A2 — Filename tag decode** ✅ **shipped** (~3–4 days): extend
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

### Phase E — Schema promotion to game_identities (~3–4 weeks) — ✅ SHIPPED

**✅ Merged to main** (Sub-phases 1–3 complete — `game_identities` schema
v23, identity CRUD, identity-backed group read path + frontend).

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
- `frontend/src/platform/components/LibraryTile.tsx` +
  `frontend/src/platform/components/VirtualLibraryGrid.tsx` — render from
  identities; per-variant launch via the existing `Run version ▸`
  submenu.
- `frontend/src/themes/retroverse/GameDetailPanel.tsx` — header shows
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

**Status: code-complete on branch `feat/virtual-library-phase-b` (tip
`a475b29`) — RE-DERIVE, don't merge (reassessed 2026-06-15).** The branch is
171 commits behind main and every file it needs has since moved or been
rewritten, so merging would be a conflict slog. Re-implement on current main
using the branch as a design+code reference; it does not rot as a reference, so
there is no time pressure (slot after Theming ARC 2 L1). Re-home map:
- **Slice 1 toggle** → the new Settings IA **Library** group
  (`engine/SettingsSections.tsx` + the rewritten `platform/settings/store.ts`),
  NOT the old Display→Library-presentation location described below.
- **Slice 2 variant ribbon** → reconcile with **D33** (theming): it currently
  paints chrome on the shared `LibraryTile`/grid unconditionally — make it
  theme-opt-in rather than forced cross-theme.
- **Slice 3 Variants tab** → re-apply onto `platform/components/GameInfoModal.tsx`
  (relocated + rewritten since the branch).
- **Slice 4 Collection Health** → `engine/SystemHealthPage.tsx` (moved from
  `routes/retroverse/`).
No conceptual conflict with locked decisions — two-mode UX is core to this arc
and orthogonal to theming. (Section below is the original design intent; the
re-home map above supersedes its specific file/menu locations.)

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

### Phase C — Launcher abstraction (~2–3 weeks) — ✅ SHIPPED

**✅ Merged to main** (merge `28875d5`; Sub-phases C1–C3 — `Launcher`
trait + `LibretroLauncher` + `ExternalProcessLauncher` + profile registry
+ capability gating + per-system launcher pref + force-quit affordance).

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
- `frontend/src/platform/components/LibraryTile.tsx` + `VirtualLibraryGrid.tsx` — render from identities (Phase E)
- `frontend/src/themes/retroverse/GameDetailPanel.tsx` — Variants tab (Phase B)
- `frontend/src/engine/SystemHealthPage.tsx` — Collection Health rollups (Phase B)

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
