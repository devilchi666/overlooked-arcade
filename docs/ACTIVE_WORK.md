# Active Work Streams

Free-form list of what's in flight. Read the linked stream's README + recent
SESSION_LOG entry to pick up where the last session left off.

Replaces the older `docs/ACTIVE_CORE.md` (single-string "which core is active")
because cross-cutting work didn't fit that model — the 2026-05-22 sidebar work
spanned every system but was filed under whichever core happened to be active.

---

## In flight

- **Virtual library + preservation architecture + launcher-agnostic frontend**
  — major multi-month arc planned 2026-06-03. 8 phases (A → E → B → C
  → D → F → G; Phase H deferred). Plan at
  [PLANS/virtual-library-and-launcher-arc.md](PLANS/virtual-library-and-launcher-arc.md).
  **Current slice: Phase A1 — disc-track SHA-1 matching.**
  - **Phase 0 ✅ shipped 2026-06-03** (merged from
    `feat/virtual-library-arc-foundation`, merge `dd430e4`-ish — Phase 0
    + the subsequent docs cleanup merged to main together). DECISIONS
    reversal of the 2026-05-16 libretro-only stance + partial un-park
    of the 2026-06-02 plugin-API entry + CLAUDE.md softening + plan
    committed at the path above.
  - **Phase A — identification depth (~3–4 weeks, in flight via A1):** disc-track SHA-1
    (A1) + filename tag decode (A2, hacks/translations/bad dumps) +
    Tier 5 deep-dive (A3) + MAME parent/clone bridge (A4).
    - **A1 Sub-phase 1 ✅ shipped 2026-06-03** (merge `1c319f8`).
      Schema v18→v19 (rom_hashes_tracks + game_disc_tracks +
      disc_sets + games.disc_set_id/disc_number), parser per-track
      + multi-disc-parent emission, sync flow dispatch, full
      disc-shape helper surface in library_db. 665 oa-shell tests
      pass (660 baseline + 5 new); frontend typecheck silent.
      Cart-shape `rom_hashes` path untouched. Plan + design
      decisions in [PLANS/disc-track-sha1-matching.md](PLANS/disc-track-sha1-matching.md).
    - **A1 Sub-phase 2 ✅ shipped 2026-06-03** (merge `dc2a257`).
      Per-track hashing engine in `apps/oa-shell/src/disc_track_hash.rs`:
      `.iso` / `.cue` (split-bin + merged-bin via INDEX 01) / `.gdi` /
      `.chd` (CHT2 parse + 4-frame padding + subchannel strip).
      Streaming SHA-1 with 1 MiB cancel-check cadence. `evaluate_match`
      across Strict / Threshold / Lenient. `cd_id::cue::parse` extended
      to capture INDEX 01 positions for merged-bin slicing. 687 tests
      pass (665 baseline + 22 new); frontend typecheck silent.
    - **A1 Sub-phase 3 ✅ shipped 2026-06-03** (merge `d2bf7db`).
      Backend identify flow + game_disc_tracks cache + mtime/size
      invalidation. `JobKind::DiscTrackHash`, `DiscTrackStrictness`,
      per-track try block in `resolve_rom_hashes_for_system`.
    - **A1 Pivot ✅ shipped 2026-06-03** (merge `c4aec19`).
      Per-track moved behind `LibraryPrefs.disc_track_experimental_enabled`
      (default OFF) after operator playtest measured 0% match rate
      on real library (Dreamcast CHD: chdman extract is 225 sectors
      short of redump's DiscImageCreator source dump; archived PSX
      ZIP: per-track skipped per Sub-phase 3 deferral). New primary
      identification: filename-fuzzy match against canonical disc
      titles in `rom_hashes_tracks` — cheap, works on any container
      shape. 697 tests pass.
    - **A1 follow-up — frontend experimental checkbox** (small).
      Settings → Display → Experimental → Per-track SHA-1 disc
      identification checkbox that toggles
      `LibraryPrefs.disc_track_experimental_enabled`. Backend is
      wired; just needs the frontend control.
    - **A1 hit-rate measurement** (operator-facing next step).
      Operator rebuilds + runs Identify ROMs on a disc system. The
      fuzzy index builds at resolve start (logged with canonical
      count). Per-game progress shows `matched (filename) →
      <canonical>` for hits. Misses fall to `peek_disc_id` (existing
      serial-lookup path). Hit rate measurement determines whether
      Sub-phase 4 (multi-disc grouping) is built on top of fuzzy
      or whether further architectural work is needed.
    - **A1 Sub-phase 4 — multi-disc disc-set wiring** (deferred).
      Was built on top of per-track stamping `games.disc_set_id`.
      With fuzzy as primary, the canonical `game_name` carries the
      "(Disc N)" suffix and grouping can move to display-time
      rather than data-model-time. Re-evaluate after the operator
      reports fuzzy hit rate from real libraries.
  - **Phase E — schema promotion (~3–4 weeks):** new
    `game_identities` SQLite table; per-group MediaDb keys; per-group
    metadata + play_time + favorites.
  - **Phase B — two-mode UX + Collection Health (~2 weeks):** global
    Casual / Preservation toggle; Variants tab on GameDetailPanel;
    System Health Overview gains % verified / % covers / % metadata
    rollups.
  - **Phase C — launcher abstraction (~2–3 weeks):** `oa-core::Core`
    → `Launcher` trait refactor; `LibretroLauncher` +
    `ExternalProcessLauncher` impls; `config/emulators/<id>.yaml`
    profile registry.
  - **Phase D — external emulator install pipeline (~2–3 weeks):**
    download + setup for v1 pilot trio (Cemu / RPCS3 / Lime3DS) from
    official release endpoints; plugin-style updater; legal posture
    locked (zero ROMs / zero BIOS, ever).
  - **Phase F — Preservation Vault polish (~1–2 weeks):** dedicated
    surface with deep variant tree filter ribbon.
  - **Phase G — `crates/oa-preserve` workspace split (~1–2 weeks):**
    refactor identification + grouping + DAT parsing into a
    standalone crate.
  - **Phase H — `oa-preserve-cli`:** deferred — back burner.
  - Two strategic shifts: (1) virtual library moves from runtime
    grouping to SQLite schema; (2) external standalone emulators
    (Cemu / RPCS3 / Lime3DS / Ryujinx / Suyu / Dolphin standalone)
    join libretro cores via the `Launcher` trait. Reverses the
    2026-05-16 libretro-only DECISIONS entry; partially un-parks the
    2026-06-02 plugin-API PARKING_LOT entry. Driven by 2026-06-03
    advisor proposal (ChatGPT + Gemini) + three rounds of operator
    Q&A. Total estimate ~14–22 weeks.

- **Retroverse UI rollout** — all six top-toolbar tabs operator-
  facing with real bodies. 2026-05-28 shipped Phases A-C4 + HOME v2
  + SETTINGS expansion; 2026-05-29 closed the unified controller
  pipeline + menu/dialog polish + Slice 12 custom collections +
  Per-system SETTINGS drill-in + Now-playing chip + DISCOVER body.
  Toggle Settings → Display → Experimental → Retroverse UI ON to
  enter; flag OFF stays byte-identical with the legacy Shell apart
  from the heart overlay on tiles + the custom-collections submenu
  in TileContextMenu (Retroverse-only).

  **Current state (2026-05-29 end of day):**
  - HOME — v2 dense mockup (system spotlight + carousel arrows +
    dot pagination + Recently Played panel; right pane = SYSTEM
    INFORMATION / TECHNICAL DETAILS / PERIPHERALS / ACHIEVEMENTS).
  - LIBRARY — header card + system-label tile headers; reuses
    LeftSidebar + VirtualLibraryGrid + GameDetailPanel.
  - COLLECTIONS — 3-pane; ALL 6 smart-lists wired (Favorites /
    Recently played / Completed / Multi-player / Hidden gems /
    Last played) PLUS Slice 12 custom collections (create / rename
    / delete / membership submenu in TileContextMenu).
  - PLAY NOW — hero + WHY-line + 3 rails + 9-mood sidebar (For
    you / Continue / With a friend / Nostalgia / Quick / Marathon
    / Challenge / Surprise me / Daily roulette with UTC-day lock).
  - DISCOVER — 3-pane with 4 data-driven axes (By era / By genre /
    By publisher / By developer) reading from `useMedia().media(
    romId)?.metadata`; 5 editorial axes (Featured / On this day /
    System dive / Cult classics / Lost games) render empty-state
    cards pointing at Phase C6 content-packs.
  - SETTINGS — ALL 15 top-level categories have real bodies PLUS
    the Per-system drill-in (sidebar group expands to 45-system
    picker; center pane composes Display / Rewind / Shaders /
    Default core inline + Bindings / Core options launchers).
    Section bodies shared with legacy SystemSettingsDialog via
    `components/perSystemSections.tsx`.
  - Now-playing chip in HintBar shows current platform-music
    system with animated equalizer bars.
  - Operator-locked controller-nav: L1/R1 cycle tabs; DPad
    LEFT/RIGHT transfers regions; stick walks within (LIBRARY
    sidebar containers expand/collapse on stick L/R via
    source-gated `onDirection`). See
    [features/retroverse-ui/DECISIONS.md](features/retroverse-ui/DECISIONS.md).
  - 660+ oa-shell tests green; frontend `npm run typecheck` silent.

  **Genuinely open work** (full §10 list in
  [PLANS/retroverse-ui-rollout.md](PLANS/retroverse-ui-rollout.md)
  §10, audited 2026-05-29):
  - Phase C6 — content-packs infrastructure (substantial; unlocks
    DISCOVER's 5 stub axes + curated COLLECTIONS + theme packs).
  - RetroAchievements integration OR local milestone tracking
    (HOME ACHIEVEMENTS card + GameDetailPanel / SystemInfoPanel
    sections are placeholders).
  - Per-System UI Stage 2 + Stage 3 — separate plan.
  - Flag deprecation endpoint — eventual.

  **Content workstream (operator-side):** per-system hero art (drop
  console + fanart into existing PlatformMedia slots),
  `systemMetadataStubs.ts` refinement for ~38 systems beyond the 7
  priority stubs, per-system blurbs.

- **Per-System Custom UI Stage 1 — code arc complete; content-side
  pause** ([features/per-system-ui/](features/per-system-ui/)).
  Slices 1-5 merged to main 2026-05-26 / 2026-05-27: the foundation
  + the four consumer-side mechanisms (per-system SFX wiring,
  background renderer, boot animation framework, tile flourish
  system). Master toggle ON gives every system a visibly distinct
  feel via the registry alone — operator playtested across the
  Stage 1 pilots and confirmed the per-system differences read.
  Remaining slices 6-9 are content-heavy: GB / NES / Vectrex
  full pilot builds (SFX recordings, background assets, boot
  animation keyframes, plus a Vectrex custom-component escape
  hatch) + per-core README "Per-system UI" sections. Held pending
  operator content production (CC0 audio curation, DMG gradient,
  AI-generated Vectrex vector blips, etc. — see plan §9 for
  sourcing strategy). Resumes when operator green-lights with
  content in hand. See
  [features/per-system-ui/ROADMAP.md](features/per-system-ui/ROADMAP.md)
  for the slice breakdown and
  [features/per-system-ui/ASSETS.md](features/per-system-ui/ASSETS.md)
  for the operator-facing asset catalog (where every sound /
  background / boot animation file goes on disk).

---

## Recently completed

Compressed log. Full per-arc detail lives in `docs/_archive/` — see
[_archive/INDEX.md](_archive/INDEX.md) for the searchable manifest.

**2026-06-03**
- SETTINGS declutter — System Health hub + Game-media status-first
  cards. Sidebar shrank 16 → 12. Merge `dd430e4`. Plan:
  [_archive/PLANS/settings-declutter-system-health.md](_archive/PLANS/settings-declutter-system-health.md).
- Background jobs registry + persistent progress bar — 7-phase arc
  + polish complete. Plan:
  [_archive/PLANS/background-jobs-and-progress-bar.md](_archive/PLANS/background-jobs-and-progress-bar.md);
  feature folder:
  [_archive/features/background-jobs/](_archive/features/background-jobs/).

**2026-06-02**
- Per-system descriptor consolidation — Slices 1+2 shipped (~2,750
  LOC removed; 46 systems load from `config/systems/<id>/`). Slice 3
  (L3 content packs + L4 SQLite + JSON Schema + CI lint) still
  queued. Plan: [PLANS/per-system-descriptors.md](PLANS/per-system-descriptors.md).

**2026-06-01**
- Guided Setup Phase 1B — wizard upgrade closed (6 slices in a
  single day). Phase 2 (curated CPU-tier core selection) queued.
  Feature: [features/guided-setup/](features/guided-setup/).
- MAME ROM-set name resolution (listxml metadata pass).
- System Info Panel v1 — 4-table SQLite schema + per-system YAML
  curation. Plan:
  [_archive/PLANS/system-info-panel-v1.md](_archive/PLANS/system-info-panel-v1.md).

**2026-05-30 / 2026-05-31**
- NDS per-game touch hotspots overlay.
- Legacy Shell deletion (-1,860 lines across 13 files).
- Retroverse migration follow-ups (drop overlay + header
  affordances + Help-dialog Retroverse home).
- Gameplay fixes batch — NDS multi-touch + lightgun gun-side
  buttons + SNES Super Multitap.
- Game Info Panel v1 — 3-layer data model + 6 Tauri commands +
  4-tab modal. Plan:
  [_archive/PLANS/game-info-panel.md](_archive/PLANS/game-info-panel.md).
- libretro env-callback batch (four gaps closed).

**2026-05-26 / 2026-05-27**
- Three new systems — jagcd / sega32xcd / stv (Phase 0 wiring done;
  operator playtest pending BIOS + ROM acquisition).
- Per-System Custom UI Stage 1 Slices 1-5 (code arc complete;
  Slices 6-9 await operator content).
- Controller-nav v2 polish + completion pass + Phase 0 primitives.
  Feature: [_archive/features/controller-nav/](_archive/features/controller-nav/).

**Earlier (2026-05-20 → 2026-05-25)**
- ColecoVision keypad reference + GameCube Wii peripherals.
- System fixes pass — MAME / light-gun IS_OFFSCREEN / Saturn 3D Pad
  + Atari 7800 twin-stick labels / NDS stylus reticle.
- DOSBox + ScummVM onboarding plan locked.
  Feature: [features/dosbox-and-scummvm/](features/dosbox-and-scummvm/).
- Media taxonomy (5-slot → 26-slot LaunchBox shape).
  Feature: [_archive/features/media-taxonomy/](_archive/features/media-taxonomy/).
- Window geometry persistence + tile-size slider.
- Portable install (`<exe_dir>/settings/` via `portable.txt`).
  Feature: [_archive/features/portable-install/](_archive/features/portable-install/).
- Docs audit + reorg (the originating cleanup arc).
- Sidebar tier + view editor + UI polish.
  Features:
  [_archive/features/sidebar/](_archive/features/sidebar/),
  [_archive/features/ui-polish/](_archive/features/ui-polish/).

For full per-arc detail, see [_archive/INDEX.md](_archive/INDEX.md).

---

## Cores

No core is in active deep-integration work today. The 2026-05-20 POINTER
infrastructure batch (psp + ps2 + nds) was the most recent cross-core focus.

Per-core status surfaces:
- High-priority next work — [NEXT.md](NEXT.md) HIGH/MEDIUM bands
- Per-system status — `docs/cores/<id>/ROADMAP.md`
- **5200 + pokemini** Phase 0 fully wired 2026-05-20 (default core,
  BIOS check, bindings, registry, theme). Phase 1 = operator
  playtest only (drop .dll + BIOS, scan library, launch flagship
  titles per the ROADMAP). No more code work on these two from this
  side until playtest surfaces a Phase 2 polish need.
- **scummvm + dosbox** — engine cores, plan locked 2026-05-24
  ([features/dosbox-and-scummvm/](features/dosbox-and-scummvm/)).
  5-phase implementation pending operator green-light. Both ship as
  ordinary OA systems alongside consoles; scummvm scans for
  `.scummvm` descriptor files, dosbox scans for one-level-deep
  subdirectories. No new UI surface beyond the existing sidebar.

## Picking next work

When this stream wraps and there's no clear next ask: read [NEXT.md](NEXT.md)
HIGH/MEDIUM bands first, then [PARKING_LOT.md](PARKING_LOT.md). Confirm the
pick with the operator before starting.
