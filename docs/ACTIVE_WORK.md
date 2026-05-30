# Active Work Streams

Free-form list of what's in flight. Read the linked stream's README + recent
SESSION_LOG entry to pick up where the last session left off.

Replaces the older `docs/ACTIVE_CORE.md` (single-string "which core is active")
because cross-cutting work didn't fit that model — the 2026-05-22 sidebar work
spanned every system but was filed under whichever core happened to be active.

---

## In flight

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
  - 506 oa-shell tests green; frontend `npm run typecheck` silent.

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

- **ColecoVision keypad reference + GameCube Wii peripherals** —
  branch `feat/coleco-keypad-and-gamecube-wii`, two commits.
  - **Coleco visual keypad reference** (`c4746d9`): new
    `KeypadReference` component renders the physical 3×4 Coleco
    controller keypad in the per-game Input dialog with each KP
    labeled by its current per-system keyboard / gamepad mapping.
    Bridges "keypad_layout_note says KP1" to "physical key 'Q'
    fires KP1." Visible only for Coleco today; Intv shares the 3×4
    shape and can adopt later. Per-game bindings override stays
    ⬜ stretch (`coleco/ROADMAP.md:36`); current design keeps
    bindings system-wide per the `GameOverrides.keypad_layout_note`
    doc comment.
  - **GameCube Wii peripheral subclasses**: un-deferred after
    Dolphin libretro source research confirmed the 5 selectable
    SUBCLASS values from `Source/Core/DolphinLibretro/Input.cpp:48-54`.
    New `DEVICE_ID_OPTIONS_GAMECUBE` table in `GameDialogs.tsx`
    adds Wii Remote (sideways) / + Nunchuk / + Classic Controller /
    + Classic Pro / GC Controller (Wii mode). `deviceOptionsForSystem`
    helper gates them so they only show in the dropdown for
    GameCube system games. Per-game hint block names the Wii titles
    each peripheral is for. No Rust changes — `arm_libretro_device`
    already dispatches arbitrary u32s. `library_db.rs::GameOverrides::libretro_device`
    doc comment extended with the Dolphin subclass table.
    `gamecube/ROADMAP.md:44` flipped ⬜→✅.

  **Dreamcast VMU peripheral** — still deferred to its own run
  (Phase 2.5 secondary-screen plumbing).

- **System fixes pass — MAME / light-gun IS_OFFSCREEN / Saturn 3D Pad +
  Atari 7800 twin-stick labels / NDS stylus reticle** — branch
  `feat/system-fixes-mame-lightgun-analog`, five commits closing a
  handful of small-to-medium code gaps across the per-core ROADMAPs:
  - **MAME Phase 1.5 clarification**: ROADMAP line 30 reframed from
    "verify dispatch" ⬜ to clarified ✅ — MAME consumes Service /
    Tab / P2 inputs via the keyboard-passthrough pump, not the
    RetroPad bits the four MAME_BUTTONS entries map to. RetroPad
    mappings stay as future-proofing; no code change.
  - **Light-gun IS_OFFSCREEN flag (`696dc87`)**: real code gap. New
    `in_viewport` field on `InputState.pointer` flows from
    `InputPoller::poll_pointer` → `cb_input_state` →
    `lightgun_field_value` → `RETRO_DEVICE_ID_LIGHTGUN_IS_OFFSCREEN`.
    Shoot-off-screen-to-reload now works across all 6 light-gun
    systems (NES Zapper, SMS Phaser, Saturn Virtua Gun, PS1 GunCon
    / Justifier, Dreamcast HotD / Confidential Mission, Atari 7800
    XEGS Light Gun). Closes the 2026-05-25 light-gun arc's
    remaining ⬜ note; 577 workspace tests green.
  - **System-specific device-type labels (`5e60ae6`)**: cosmetic
    polish — the per-game Input dialog dropdown now shows
    "3D Pad / Analog" for Saturn, "Virtua Gun" for Saturn/STV
    light-gun device, "Zapper" for NES, "Super Scope" for SNES,
    "DualShock / Analog" + "GunCon / Justifier" for PSX, etc.
    Plus help-text blocks in the Additional Ports section for
    atari7800 (Robotron twin-stick recipe) + saturn / stv
    (3D Pad setup). Underlying dispatch unchanged.
  - **NDS stylus reticle (`9d04815`)**: visual cursor + tap
    feedback overlay. Hollow accent-colored ring follows the OS
    cursor while an NDS game is running; fills in + scales down
    on left-mouse-down so the operator sees explicit stylus-tap
    feedback the OS cursor doesn't provide. Per-game touch-area
    hotspots remain ⬜ as a separate larger feature.

  Per-core ROADMAP flips (saturn / atari7800 / nds / dreamcast / psx /
  mame) reflect each of these closures. NEXT.md cross-system
  inventory updated to record the in_viewport plumbing.

- **Three new systems — jagcd / sega32xcd / stv** — merged to
  main 2026-05-27 (`--no-ff` from `feat/new-systems-jagcd-32xcd-stv`,
  merge `189c448`). Three phase commits + a docs commit lifting
  all three from the `docs/NEXT.md` DEFERRED band.
  - **jagcd** (`5ec0a57`): new oa-core `SystemId::JaguarCd`
    variant + frontend slug + theme + BIOS check (jagboot.rom +
    jagcd.rom) + bindings shared with cart Jaguar + Atari_-_Jaguar_CD
    thumbnails repo.
  - **sega32xcd** (`de4335b`): new frontend slug routing to
    `oa_core::SystemId::SegaCd` (stacked-override pattern, no new
    Rust variant). Default core swapped to PicoDrive — the only
    libretro core with 32X+CD combined-mode support. BIOS check
    reuses `check_sega_cd_bios`.
  - **stv** (`f700f64`): pure alias slug routing to
    `oa_core::SystemId::Mame`. MAME's stv driver handles BIOS
    lookup + ROM-set loading internally; no separate BIOS check
    function in OA. Bindings + media sync share MAME's.

  Phase 0 (slug wiring) is ✅ for all three. Phase 1 (operator
  playtest) is ⬜ for all three — operator needs to legally
  acquire BIOSes + ROMs first. Per-core ROADMAPs flip Phase 1 ✅
  as each is validated end-to-end.

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

## Recently completed (this session)

- **Controller navigation v2 polish**
  ([features/controller-nav/](features/controller-nav/)) — merged to
  main 2026-05-26 (`--no-ff` from `feat/controller-nav-v2-polish`).
  Three phase commits + a docs commit closing three of the four
  LOWER-band #1 items NEXT.md carried as "Controller-nav v2 polish
  (operator-driven)." Slice 1 QuickSettings sub-views (`b87493d`):
  rewind / TAS / video / memory / disc panels each gain a focus group
  + back handler; most via a new `useDomQueryFocusGroup` helper in
  `frontend/src/nav/focus.ts` (DOM-query + MutationObserver +
  identity-tracked focused element, generalized from the MenuBar
  pattern); the rewind scrubber uses an `onDirection` override so DPad
  left/right scrubs the timeline. Slice 2 right-sidebar widget DPad
  browse (`c883af3`): sidebar body becomes one DOM-query group keyed
  by `data-oa-sidebar-row`, widget wrappers participate alongside the
  action row, R1 from grid still lands on Play (createEffect snaps
  focus to first action while inactive). Slice 3 MenuBar identity-
  tracked focus (`567d0de`): closes Slice K's known limitation by
  tracking the focused button by element identity through rebinds.
  Pin toggle + sidebar-hide button in the right-sidebar header stay
  mouse-only by design (utility chrome, not the play path) — one
  of the four LOWER-band #1 bullets remains ⬜ for that reason. With
  controller-nav now fully shipped (Phase 0 + completion pass + v2
  polish), Per-System UI Stage 1 is the next major arc per the
  pipelined sequence in `docs/NEXT.md`.

- **Controller navigation completion pass**
  ([features/controller-nav/](features/controller-nav/)) — merged to
  main 2026-05-26 (`--no-ff` from `feat/controller-nav-completion`).
  Extends Phase 0 (A–E, merged earlier today) to cover every remaining
  interactive surface so the operator can run the whole shell from a
  pad. Ten commits on the branch: seven feature slices plus three
  post-test fixes. Slices: F critical polish + global back-stack
  (`102eef8`); G context + overlay menus — TileContextMenu /
  SystemContextMenu / SaveSlotsModal / QuickSettings (`8254aa1`);
  H GameInfoModal + universal Dialog B-close via the primitive
  (`6cb86d9`); a fix that gates the frontend Web Gamepad poller while
  gilrs owns input via DOM focus on the library WebView (`662cd5a`);
  K top toolbar menu bar with Start-to-open + L1/R1 menu cycling
  (`d68ab7f`); L chained CorePicker + RegionPicker popovers
  (`8180a0e`); M right sidebar widget actions row with R1-from-grid
  transfer (`e721e7d`). Post-test fixes: docs (`4079d20`), library
  grid DPad left/right wrap-across-rows (`792f17d`), and menu bar
  focus ring + disabled filter + dynamic content + a cross-cutting
  `data-oa-focus-active` CSS broadening (`dc25ab4`). Read-only
  widgets, utility chrome, and QuickSettings sub-views deliberately
  stay mouse + keyboard in v1. With Phase 0 + the completion pass
  shipped, Per-System UI Stage 1 unblocks next per the pipelined
  sequence in `docs/NEXT.md`.

- **Controller navigation primitives (Phase 0)**
  ([features/controller-nav/](features/controller-nav/)) — merged to
  main 2026-05-26 (`--no-ff` from `feat/controller-nav-primitives`).
  5 phase commits shipping the shared foundation for the Guided Setup
  + Per-System UI arcs: Slice A `nav/gamepad.ts` Web Gamepad API
  poller (`ca3dff9`); Slice B `nav/focus.ts` useFocusGroup hook with
  vertical/horizontal/grid orientations + L1/R1 neighbour transfer
  (`d8a5ffb`); Slice C `nav/HintBar.tsx` persistent footer + module-
  stack HintRegion (`a3a54b3`); Slice D wired VirtualLibraryGrid +
  LeftSidebar with DPad nav + A/X buttons + shoulder bumpers
  (`49522ab`); Slice E `Settings → Display → Controller navigation`
  panel with master toggle / source picker / A↔B swap / animation
  budget (`f2501fb`). Operator-confirmed working before merge.
  Phase 0 closes; Per-System UI Stage 1 unblocks per the pipelined
  sequence in `docs/NEXT.md`.

- **DOSBox + ScummVM onboarding** ([features/dosbox-and-scummvm/](features/dosbox-and-scummvm/))
  — shipped 2026-05-24 across two `--no-ff` merges. Phase 1
  scummvm (`0b56bd8`, branch `feat/dosbox-and-scummvm`) wired
  the descriptor-file engine launcher; Phase 2 dosbox (`b6fea2c`,
  branch `feat/dosbox-onboarding`) wired the directory-path engine
  launcher and added new infrastructure that future engine cores
  will reuse: `is_directory_path_system` helper, `run_dir_scan_blocking`
  + `start_background_directory_scan`, `systemHint`-aware classification
  in the Import Wizard, `GameOverrides.dosbox_entry_point` field.
  Cross-stream SESSION_LOG + commit shas at
  [docs/features/dosbox-and-scummvm/SESSION_LOG.md](features/dosbox-and-scummvm/SESSION_LOG.md).
  Per-core ROADMAP Phase 1 entries flip ✅ when operator playtest
  validates each (gated on having game data on hand).

- **Media taxonomy** ([features/media-taxonomy/](features/media-taxonomy/)) —
  merged to main 2026-05-24 (`--no-ff` from `feat/media-taxonomy`).
  7 phase commits implementing the full LaunchBox-shape art/audio
  taxonomy locked in the 2026-05-23 plan:
  - **Phase 1** (`7c1b0e9`) data model + folder layout: MediaKind
    5 → 27 variants, GameMedia/SelectedMedia per-slot fields,
    sanitize/path-builder helpers, set_manual_cover writes to new
    layout via library_db.find_game_by_id rom_stem lookup.
  - **Phase 2** (`c2d0976`) libretro-thumbnails sync to new layout
    + operator-art-wins guard (sha-based cache, next-variant
    suffix), ingest_manual_for_slot eviction logic.
  - **Phase 3** (`2edfc1d`) LaunchBox/EmuMovies art-pack importer
    (auto-detects single- vs multi-platform layouts, fuzzy
    matches against library titles at 0.95 threshold) +
    ImportArtPackDialog UI.
  - **Phase 4** (`b71057c`) 4-bus audio mixer over rodio/symphonia
    (platform-music / ui-sounds / ceremony / snap-audio) +
    SystemSettings audio override fields + GameOverrides
    platform_music_path + frontend audio dispatch service.
  - **Phase 5** (`92c2403`) existing-install migration: walks
    pre-Phase-1 MediaDb, moves manual covers / copies synced art
    to canonical kind dirs, sentinel-guarded one-shot pass.
  - **Phase 6** (`d8dd98a`) per-system PlatformMedia (9 slots —
    banner, clear-logo, console, controller, fanart, marquee,
    photo, wheel, background) + PlatformMediaDialog UI.
  - **Phase 7** docs + SESSION_LOG (this entry).
  cargo test workspace 430 oa-shell + 64 others all green.
- **Window geometry persistence + tile-size slider**
  ([features/ui-polish/](features/ui-polish/)) — merged to main
  2026-05-23 as `6cf6acb`. 3 phase commits on
  `feat/window-and-tile-prefs`: `LayoutPrefs.windows` map with
  per-label geometry + first-launch maximize + debounced flusher
  thread; `library_tile_size` + GridControls slider + hybrid ±20%
  scaling in VirtualLibraryGrid; SESSION_LOG entry.
- **Portable install** ([features/portable-install/](features/portable-install/)) —
  merged to main 2026-05-23 as `993ca6a`. 5 commits: data_dir
  resolver + marker file, asset-protocol runtime scope + frontend
  getDataDir helper, AppData→portable auto-migration with sentinel,
  CLAUDE.md + docs, and a follow-up `npm --prefix` fix to
  tauri.conf.json so `cargo tauri build` works end-to-end.
- **Docs audit + reorg** — branch `feat/docs-audit-and-reorg`, 5 commits.
  Phase 1 fixed stale references across the docs tree; Phase 2 introduced
  `INDEX.md` + `ACTIVE_WORK.md` + `docs/features/<name>/` skeleton, moved
  executed plans into their feature folders, re-filed cross-cutting
  session entries out of per-core SESSION_LOGs, and capped the long
  SESSION_LOGs with sibling ARCHIVE files. Merged to main.
- **Sidebar v3.4 PARKING_LOT entry** — small doc-cleanup PR merged
  to main 2026-05-23 as `c700641`.

## Recently completed (last 1–2 sessions; reference for context)

- **Sidebar tier + view editor** ([features/sidebar/](features/sidebar/)) —
  PR-α/β/γ shipped 2026-05-21; v2.1–v3.5 shipped 2026-05-22. Tier plan and
  View Editor plan are now historical reference under features/sidebar/.
  Outstanding: v3.4 per-container art slots (parked in PARKING_LOT.md).
- **UI polish** ([features/ui-polish/](features/ui-polish/)) — Phases A–E
  shipped 2026-05-22. Menu-bar IA operationalized via dialog refactor.

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
