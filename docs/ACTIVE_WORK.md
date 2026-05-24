# Active Work Streams

Free-form list of what's in flight. Read the linked stream's README + recent
SESSION_LOG entry to pick up where the last session left off.

Replaces the older `docs/ACTIVE_CORE.md` (single-string "which core is active")
because cross-cutting work didn't fit that model — the 2026-05-22 sidebar work
spanned every system but was filed under whichever core happened to be active.

---

## In flight

(Nothing actively in flight.)

## Recently completed (this session)

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
