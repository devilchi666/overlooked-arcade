# Active Work Streams

Free-form list of what's in flight. Read the linked stream's README + recent
SESSION_LOG entry to pick up where the last session left off.

Replaces the older `docs/ACTIVE_CORE.md` (single-string "which core is active")
because cross-cutting work didn't fit that model — the 2026-05-22 sidebar work
spanned every system but was filed under whichever core happened to be active.

---

## In flight

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
