# Overlooked Arcade — ChatGPT briefing

This document is a self-contained briefing for ChatGPT (or any other LLM
collaborator) about the Overlooked Arcade project. **Purpose:** give the
model enough context that it can help spot missing features, suggest
improvements OA hasn't considered, identify gaps in coverage, and reason
about new directions without us having to re-explain the project every
time.

Paste or upload this file into a fresh ChatGPT session. After reading
it, ChatGPT should be able to answer questions like:
- "What systems is OA missing that you'd expect from a serious retro
  frontend?"
- "What's a feature operators usually want that doesn't appear here?"
- "Given the philosophy, what's the next obvious arc after Phase 2?"
- "What corner cases in BIOS handling / save states / per-game settings
  might be unhandled?"

The document is in-repo at `docs/CHATGPT_BRIEFING.md`. Update it as the
project evolves — it represents the operator's mental model of the
project at the time of last edit (2026-06-01).

---

## 1. What Overlooked Arcade is

**Overlooked Arcade (OA)** is a premium desktop emulator frontend for
the cult and underserved consoles of gaming history — and, increasingly,
the entire retro catalogue. The name reflects the original focus:
**consoles modern emulators forgot**. TurboGrafx-16 + PC Engine CD,
Atari Lynx, Atari 7800, Sega Master System / Game Gear, MSX/MSX2,
ColecoVision, Vectrex, Virtual Boy, WonderSwan — systems whose
libraries deserve to be played in 2026 but whose existing emulators are
technically excellent and cosmetically punishing (Mednafen + friends).

Beyond the original "first wave," OA has grown to host **41 systems**
as of 2026-06-01 — every major cart-shape and CD-shape console,
handheld, plus engine launchers (ScummVM, DOSBox-Pure). The long-term
ambition is to host **almost all of retro gaming** under one polished
shell.

**It's non-commercial.** A gift to the retro community from someone who
got through depression with the help of that community.

### Audience priorities

1. **Couch gamers (primary).** Operator on a couch with a controller,
   monitor or TV across the room, wants OA to "just work" and stay out
   of the way. Will set up once and play often.
2. **Cabinet builders (secondary).** Eventually served by the future
   kiosk-shell mode (separate arc, design-only today).
3. **Desktop curators (tertiary).** Original OA audience — mouse +
   keyboard, organizing per-system collections. Already served by the
   shipped UI.

### Voice + design philosophy

- **"Warm + curator/enthusiast."** Knowledgeable-but-not-condescending,
  welcoming-but-not-saccharine. Example contrast (from the guided-setup
  voice card):
  - **Bad:** "240 files scanned, 12 systems detected."
  - **Good:** "Found 240 games across 12 systems. Quite a collection
    — let's get them ready."
- **"Guided Auto-Setup, not magic."** Every automation is *visible* —
  operator sees what was decided and why. Smart defaults for the 80%
  case; full customization escape hatch for the 20%.
- **Per-system theming inside a shared modern UI.** Identity comes from
  typography, accent color, era art — not console dioramas.
- **Playable, not cycle-accurate.** Top ~80% of each system's library
  running well. We don't need to beat Mesen on NES accuracy; we win on
  experience.
- **"Heroic Games Launcher visual ceiling."** Premium desktop feel
  (~15–25 MB binary, sub-1s cold start), polished UI, real curation.

---

## 2. Tech stack

| Layer | Tech | Notes |
| --- | --- | --- |
| Shell | **Rust + Tauri 2** | Single binary; one Cargo workspace under `apps/oa-shell/`. |
| UI | **Solid + TypeScript + Tailwind + Vite** | Frontend at `frontend/`; NOT a Cargo crate. |
| Rendering | **wgpu + WGSL** | Translates to DX12/Vulkan/Metal/GL/WebGPU from one shader pipeline. |
| Emulator cores | **libretro `.dll` / `.so` / `.dylib`** | Loaded via `libloading` (`oa-libretro` crate). Cores live next to the .exe in `<exe_dir>/cores/`. Operators use community-built buildbot nightlies OR our own builds of forked cores. |
| BIOS files | `<exe_dir>/system/` | Per-system BIOS dropped here; OA's per-system check helpers SHA-1-verify them. |
| User state | `appDataDir/` (default) or `<exe_dir>/settings/` (portable mode via `portable.txt` marker) | Per-user prefs, save states, bindings, audio config, library DB (SQLite). |
| License | **GPL-2.0** today | Will move permissive (MIT/Apache 2.0) once the install ships only our own .dll builds. GPL cores stay GPL in their .dll. |

### Architecture pillars (locked decisions)

- **Every core implements the `oa_core::Core` trait** (`reset`,
  `run_frame`, `framebuffer`, `drain_audio`, `set_input`, `save_state`,
  `load_state`). The shipped impl is `oa_libretro::LibretroCore`
  wrapping a loaded libretro .dll.
- **One binary** (`apps/oa-shell/`). Per-system Rust crates are NO
  LONGER added (since the 2026-05-16 libretro pivot) — new systems
  arrive as .dll files in `cores/`, registry entries in TypeScript,
  bindings in Rust.
- **Hot paths run on dedicated threads.** The emulator runs on its own
  thread; renderer pulls the latest framebuffer; audio is event-driven
  via cpal callback. UI thread never blocks.
- **No network calls from emulator code.** Fully offline at runtime.
  HTTP only for: libretro-database hash sync, libretro-thumbnails art
  sync, buildbot core downloads, ScummVM/MAME metadata fetches.
  All operator-initiated, never silent.
- **Shaders are WGSL only.** wgpu translates everywhere. Avoid features
  that don't translate cleanly to GL fallback unless behind a backend
  cap check.

---

## 3. Shipped features (current state, 2026-06-01)

### 3.1 System coverage — 41 systems

Wired in OA today with the full onboarding recipe (registry, theme,
bindings, BIOS check where required):

**Cart consoles:** NES, SNES, TG-16, Atari 7800, Atari 2600, Atari
5200, Atari Jaguar, Genesis, Sega 32X, Master System, Game Gear, Neo
Geo (cart), ColecoVision, Intellivision, Channel F, Odyssey², Vectrex,
Virtual Boy.

**CD/optical consoles:** PCE-CD, Sega CD, Sega 32X CD, Saturn,
Dreamcast, PSX, PS2, GameCube + Wii (combined slug), Neo Geo CD, 3DO,
PC-FX, Jaguar CD.

**Handhelds:** GB, GBC, GBA, NGP/NGPC (combined), NDS, PSP, Pokémon
Mini, WonderSwan / WonderSwan Color (combined).

**Computers / arcade / engines:** MSX, MSX2, MAME, ST-V (Sega Titan
Video — runs via MAME), ScummVM, DOSBox.

Each system has its own theme block (CSS accent + display name), its
own bindings module (default gamepad → libretro layout), its own
`docs/cores/<id>/` folder with README + ROADMAP + SESSION_LOG +
KNOWN_GAME_BUGS + DECISIONS, and BIOS check helpers where applicable.

### 3.2 Library + import

- **Folder watcher** auto-adds ROMs dropped into tracked folders.
- **Background scanner** (`scan_service.rs`) walks the tree async,
  emits per-folder + per-file progress events, supports cancel mid-
  scan. Hash + header detection via `rom_hashes.rs` / `rom_header.rs`.
- **Smart-scan emission** (Phase 1B Slice 1, 2026-06-01): per row
  emits `system_id`, `suggested_title`, `confidence` (Hash / Header /
  Extension / Hint), `sha1`. Auto-syncs libretro-database hash tables
  from upstream on first scan.
- **Import Wizard** (Phase 1B Slices 2-6, 2026-06-01): 4-step modal
  reachable via Settings → Library → "Set up your library". Step 1
  folder pick → Step 2 per-ROM results table (virtualized via
  `@tanstack/solid-virtual`, inline edit, bulk-select, sort + filter,
  Advanced extension-overrides expander) → Step 3 per-system readiness
  checklist → Step 4 confirm + sync toggles.
- **Per-system readiness checklist** with 5 pills per system row: Core
  installed / BIOS present / Bindings ready / Core options pre-tuned /
  Per-game overrides. Same component lives in Settings → Library
  ("System readiness" card) for the operator's shipped library. Inline
  Pick BIOS file picker, Install missing cores bulk modal, Open BIOS
  folder action.
- **First-launch hero** in `LibraryView::EmptyState` — "Welcome to
  Overlooked Arcade" + plan voice copy + primary CTA to the wizard.

### 3.3 Per-system experiences (Per-System UI Stage 1)

- **`SystemUIConfig` registry** drives per-system theming + behavior.
- **Per-system SFX** with a resolver cascade (operator override →
  per-system bundle → `_baseline` → silence). Library navigate / launch
  call sites dispatch via `playSystemUiSound`.
- **Per-system backgrounds** — static (gradient + optional image),
  animated (looping `<video>`), or shader (Slice 8 stretch). Source
  chain: hover → focused → activeView → pinned.
- **Boot animations** triggered by `activeSystemId` transitions, 1s
  full / 200ms cross-fade with reduced-motion floor. Skippable on any
  input. Per-system `boot-intro` SFX dispatched whenever fired.
- **Tile flourishes** — `tileShape` (rectangle / square / circle) +
  `interactionStyle` (instant / delayed LCD-feel / physical spring).
- **Master toggle** in Settings → Display → Per-system experiences.
  OFF gives uniform plain library (no audio, no animations, no
  flourishes).
- **Stages 2 + 3** (behavior layer, experience layer) are planned —
  per-system navigation, in-game overlays themed per system, library ↔
  game transitions.

### 3.4 Controller navigation (Phase 0 complete)

- **Web Gamepad API rAF poller** synthesizes UI events (button
  down/up, repeat, DPad/stick directions with deadzone).
- **Focus manager** with vertical / horizontal / grid orientations,
  L1/R1 neighbour transfer, roving-tabindex helpers.
- **Focus-ring CSS** — 2px outline on `[data-oa-focus="true"]`
  elements.
- **On-screen hint bar** per screen.
- **Operator-locked spec:** L1/R1 cycles top-toolbar tabs;
  DPad/stick UP-DOWN within region; LEFT-RIGHT between
  sidebar↔center↔right pane.
- **Coverage:** entire shell drivable from a pad except utility
  chrome + per-game settings drawer + cheat editor (those stay
  mouse/keyboard until kiosk shell ships).

### 3.5 Per-game settings (Phase D dialogs)

Right-click any tile → TileContextMenu surfaces every per-game
override:

- **Input mapping** (per-game keybindings, including light-gun gun-side
  buttons, multi-touch, special device IDs like Saturn 3D Pad)
- **Core options** (per-game core options that overlay the per-system
  defaults)
- **Display** (scaling mode, aspect override, overscan crop, scaling-
  filter override)
- **Shaders** (per-game shader preset override)
- **Cheats** (memory-poke cheats with width-1/2/3 + locked-value
  trainer support)
- **Rewind settings** (per-game ring-buffer size, density)
- **Milestones** (per-game custom milestones for local-only achievement
  tracking)
- **Core override** (per-game `libretro_core` selection, e.g.
  `mupen64plus_next_libretro.dll` for Goldeneye 007 vs default
  `parallel_n64_libretro.dll`)
- **Game properties** (full editor — title, year, publisher, region
  representative, etc.)
- **Per-game settings ▸** submenu (collapses the 7 above behind a
  single tile-menu entry when the menu gets long).

### 3.6 Save states, rewind, TAS

- **Save state slots** — per-game, file-based, with thumbnail capture.
- **Rewind** — circular save-state ring buffer; scrubbing UI with
  DPad-left/right timeline navigation.
- **TAS** — deterministic input recording + replay with named
  recordings, frame-by-frame stepping during replay.
- **Cheats applied per-frame** after each NORMAL / FAST-FORWARD /
  SLOW-MO run_frame. Skipped during TAS replay (would diverge from
  recorded outcome).

### 3.7 Cores + BIOS

- **Core catalog** (`core_installer::CATALOG`) — curated list of
  every libretro core we expose, with `{ base, display_name, blurb,
  systems: [], recommended, bios_required }` per entry. Every
  registry slug has at least one CATALOG entry.
- **Per-system default core** (`cores.json` under appDataDir) —
  operator-chosen default, layered over the catalog's `recommended`
  first-pick.
- **Bulk core install modal** — list of missing cores → parallel
  `download_core(base)` calls hitting `https://buildbot.libretro.com/nightly/`
  → atomic `.partial` swap into `/cores/` → libretro probe validates
  before activation.
- **Per-system BIOS check helpers** for 19+ systems. Hash-verified
  against canonical SHA-1 tables (sourced from libretro-database's
  `dat/System.dat`). Multi-file BIOSes (NDS = 3 files, Intv = 2,
  Channel F = 2 required + 1 optional, Dreamcast pair, Jaguar CD pair)
  + OR-of-regional-variants (PSX, Saturn, PCE-CD, Sega CD, 3DO, etc.)
  both supported via `AnyOf` / `AllRequired` semantics in `BiosCheck`.
- **`install_bios_file` Tauri command** — operator picks a file via
  `@tauri-apps/plugin-dialog`'s file picker; OA hashes, copies into
  `<exe_dir>/system/` under the canonical filename, reports back
  canonical-match vs unknown-hash. WARN semantics (copy regardless;
  pill flags unknown hash for operator verification).

### 3.8 Media taxonomy (LaunchBox-shape)

Full LaunchBox-shape art/audio/video/manual storage shipped 2026-05-24:

- **9 platform-media slots per system** — banner, clear-logo, console,
  controller, fanart, marquee, photo, wheel, background.
- **27 per-game media kinds** — covers, screenshots, fanart, manuals,
  videos, audio.
- **libretro-thumbnails sync** per system. Operator-art-wins guard
  (manual covers override synced art).
- **LaunchBox/EmuMovies art-pack importer** auto-detects single- vs
  multi-platform layouts, fuzzy matches against library titles at 0.95
  threshold.
- **4-bus audio mixer** (platform-music / ui-sounds / ceremony /
  snap-audio) over rodio/symphonia. Per-system audio override fields.
- **Per-system audio dispatch service** in the frontend.

### 3.9 Game / system info panels

- **Game info panel** (right pane in Retroverse): operator-note +
  controls + recommended-core (with Apply action) + known issues
  surfaces, plus a `⚠ N` + `✎` tile badge for games with overrides.
  4th "Game info" tab in GameInfoModal with inline editor.
- **System info panel** (HOME hero + GameDetailPanel): per-system
  technical details, release year, library size, peripherals,
  achievements. Maintained via three-layer data model (L1 default from
  MAME listxml / history.xml / OA-curated; L2 per-system YAML; L3
  per-system SQLite overrides).
- **Per-game data** in `docs/cores/<id>/games-info.md` (YAML records
  with operator-note, controls, recommended_core, bugs, controls,
  recommended_core). Sparse coverage today; operator-driven content
  over time.
- **MAME ROM-set name resolution** (2026-06-01): library tiles show
  "Donkey Kong (US set 1)" instead of `dkong`; year + manufacturer
  auto-surfaces via MediaDb GameMetadata enrichment.

### 3.10 Retroverse UI (top-toolbar IA)

Default shell since 2026-05-31. Six top-toolbar tabs:

- **HOME** — system spotlight carousel + Recently Played panel; right
  pane = SYSTEM INFORMATION / TECHNICAL DETAILS / PERIPHERALS /
  ACHIEVEMENTS
- **LIBRARY** — virtual library grid / detail list + LeftSidebar +
  GameDetailPanel
- **COLLECTIONS** — 6 smart-lists (Favorites / Recently played /
  Completed / Multi-player / Hidden gems / Last played) + custom
  collections (create / rename / delete / membership submenu)
- **PLAY NOW** — hero + WHY-line + 3 rails + 9-mood sidebar (For
  you / Continue / With a friend / Nostalgia / Quick / Marathon /
  Challenge / Surprise me / Daily roulette)
- **DISCOVER** — 3-pane with 4 data-driven axes (By era / By genre /
  By publisher / By developer) + 5 editorial axes (Featured / On this
  day / System dive / Cult classics / Lost games) currently stub →
  unblocked by Phase C6 content-packs
- **SETTINGS** — 15 categories incl. Display / Audio / Controller-nav /
  Library / Cores / BIOS / Performance / Themes / Help / About + a
  Per-system drill-in (45-system sidebar picker → inline Display /
  Rewind / Shaders / Default core sections + Bindings / Core options
  launchers)

### 3.11 Logging + debugging

- **Three-output logger:** stderr (cargo tauri dev), `oa-current.log`
  (stable path, truncated each launch), `oa-<YYYYMMDD-HHmmss>.log`
  (per-session archive, last 5 retained), plus in-app **Help → Debug
  log…** with a live filterable view (last 2000 entries).
- **Frontend logs** via `console.log("[oa-launch] …")` bracket prefix
  parsed into the unified stream.
- **Copy path** button in the debug log dialog returns the right
  log path regardless of portable vs AppData mode.

### 3.12 Other shipped infrastructure

- **Portable install mode** via `portable.txt` marker next to the
  .exe. Switches `appDataDir` lookup to `<exe_dir>/settings/`.
- **Auto-migration** AppData → portable on first launch with the
  marker present.
- **Window geometry persistence** per label (single-window vs
  two-window modes both persist size + position).
- **Tile-size slider** in GridControls with hybrid ±20% scaling.
- **Screenshot gallery** wired into TileContextMenu + QuickSettings.
- **Video capture** to PNG sequences + manifest (Phase 2 export to
  WebM is a stretch).
- **Per-system custom UI Stage 1** (audio, backgrounds, boot
  animations, tile flourishes) shipped; pilots GB + NES + Vectrex
  awaiting operator content production.

---

## 4. In flight / immediately queued

(See `docs/ACTIVE_WORK.md` for the live list; `docs/NEXT.md` for the
prioritized queue.)

### Currently in flight

- **Retroverse UI rollout** — Phase C6 content-packs infrastructure
  (substantial; unlocks DISCOVER's 5 stub axes + curated COLLECTIONS +
  theme packs). RetroAchievements integration OR local milestone
  tracking. Per-System UI Stage 2 + Stage 3 (separate plan).

### HIGH band — ready to ship next

- **Guided Setup Phase 2 — curated CPU-tier core selection.**
  `sysinfo` crate integration for CPU detection (brand / base clock /
  cores → High / Mid / Low tier bucket); per-system tier preference
  table next to `CATALOG`; surfaced on the readiness checklist row
  ("Using {core} for {system} ({tier}-tier pick)"); new Settings →
  Performance CPU-tier override. ~1 week. **Awaiting fresh operator
  green-light** — Phase 1B closure is a natural pause point.

### Planned (Phase 2C-2F, each ~3 days–1 week)

- **Phase 2C — Folder management.** Optional canonical
  `<root>/<system>/` layout proposed at wizard Step 2; atomic per-
  folder move/copy with progress + cancel; watcher conflict handling
  during moves; mode-aware default root (`<exe_dir>/roms/` portable;
  `~/Documents/OverlookedArcade/roms/` AppData).
- **Phase 2D — First-system bindings + KNOWN_GAME_BUGS overrides.**
  "Looks good?" bindings card per new system encountered during
  wizard; structured front-matter / sidecar format for
  `KNOWN_GAME_BUGS.md`; auto-application of per-game core overrides
  at commit; surfaced on readiness checklist.
- **Phase 2E — Help suppression registry.** Per-dialog "Don't show
  this again" checkbox; Settings → Help → Tips & Notifications with
  suppressed-tips registry; master "Expert mode" toggle suppressing
  all tier-1 tips at once. Load-bearing alerts never suppressible.
- **Phase 2F — Existing-operator re-entry diff view.** Settings →
  Library → Re-scan with smart detection (entry exists today) gains a
  "Detected improvements" diff view before commit; override-
  preservation logic (never destroy existing bindings/overrides).

### Major adjacent arcs

- **Per-System UI Stage 2 (Behavior layer, ~4-6 weeks).** Per-system
  navigation (grid / carousel / list / wheel); per-system interaction
  style (instant / delayed LCD / physical spring); per-system tile
  emphasis; 5-10 more systems tuned to showcase tier (Jaguar, PSX,
  Saturn, MAME, TG-16 candidates).
- **Per-System UI Stage 3 (Experience layer, ~6-10 weeks).** In-game
  overlays (pause, quick settings, save-state UI) themed per system.
  Library ↔ game transitions themed. Per-system metadata priorities.
  All ~40 systems tuned past baseline.
- **Game Info Panel v2 (~3-5 weeks, infra-heavy).** Scraper
  infrastructure (GitHub Actions on a separate data repo); daily
  auto-sync from data repo to OA installs; GitHub Issue → auto-PR
  community contribution flow; Wikipedia-style richer-source
  integration.
- **KNOWN_GAME_BUGS → games-info.md migration arc.** Migrate the
  free-form markdown KNOWN_GAME_BUGS files to structured `games-info.md`
  YAML records across all 45 systems (currently 2 systems covered).
  Wires the count Tauri command + the readiness checklist's Per-game
  overrides pill real status.

### Strategic decisions pending

- **RetroAchievements integration.** Close one of two big RetroArch
  gaps. Community demand is high. ~3-4 weeks. Pending strategic
  decision.
- **Netplay.** Close the other big RetroArch gap. Multi-month effort;
  risk of shipping a worse version for years. Pending strategic
  decision.
- **License pivot to permissive.** GPL-2.0 today; once the installer
  ships only our own .dll builds (forked-core builds), the binary-wide
  GPL propagation severs and we can move to MIT or Apache 2.0. GPL
  cores stay GPL in their .dll. Mission-aligned: encourages
  contributions + forks + ecosystem.

---

## 5. Explicitly out of scope (don't suggest these)

These have been considered + decided NOT-NOW or NEVER. ChatGPT should
not suggest pursuing them.

- **External drag-drop file import.** Won't fix per `docs/PARKING_LOT.md`
  2026-05-20. Tauri/wry/WebView2 stopped delivering drop paths reliably;
  the Import Wizard + Settings → Library → Add folder + per-file Pick
  BIOS picker cover the use case. **No new drop targets, ever** — not
  for ROMs, not for BIOS, not for cover art, not for save states.
  Internal HTML5 drag (sidebar reorder, region priority list) is
  unaffected.
- **Theme ecosystem.** Deferred per advisor session 2026-05-25.
  Dead-ecosystem trap (no users → no themes → no users).
  Reconsider if/when the kiosk shell launches and there's clear
  community pull.
- **Cycle-accurate emulation.** OA's posture is "playable, not
  cycle-accurate." Cores already encode chip-level correctness; we
  don't second-guess upstream.
- **Auto-download BIOS files.** Legally sketchy. Operators source
  BIOSes themselves. OA's `install_bios_file` only handles
  operator-supplied files; "where to get it" hints point at
  `docs/cores/<id>/README.md` for sourcing notes.
- **NES / SNES early.** The popular consoles already have great
  emulators (Mesen, bsnes, SameBoy). They joined OA when there was
  bandwidth, not because we needed the headline. The OVERLOOKED end
  of the catalogue is the differentiator.
- **Per-core ARCHITECTURE.md docs.** Chip behavior lives in upstream
  documentation. Per-core docs we DO keep: README (upstream info +
  our patches summary), ROADMAP, SESSION_LOG, KNOWN_GAME_BUGS,
  DECISIONS (integration choices).

---

## 6. Operating constraints + conventions

The rules of how OA is built. ChatGPT should respect these when
suggesting features.

### Decisions cadence

- **Operator owns all decisions.** ChatGPT (and Claude) are
  developers. The operator runs builds, tests gameplay, validates
  audio, gives creative direction. When designing, present options
  with trade-offs; don't decide unilaterally.
- **One major arc at a time.** Scope creep goes in
  `docs/PARKING_LOT.md`. Currently shipping: Retroverse UI rollout +
  per-system content workstream + Guided Setup (just closed Phase 1B).
- **Feature-branch workflow.** Pre-feature push → `feat/<name>`
  branch → phase commits → push for operator playtest → merge `--no-ff`
  after thumbs-up → delete branch both sides.
- **ROADMAP hygiene.** When a PR closes a per-core ROADMAP bullet,
  flip it `⬜→✅` in the same commit + add a code citation.

### Code conventions

- **Workspace prefix:** `oa-`.
- **Shared crates:** `oa-core`, `oa-render`, `oa-audio`, `oa-input`,
  `oa-platform`, `oa-content`, `oa-savestate`, `oa-cdrom`,
  `oa-libretro`.
- **Single Rust libretro frontend:** `crates/oa-libretro/`. Per-system
  Rust crates are no longer added.
- **No comments unless WHY is non-obvious.** Well-named identifiers
  do the WHAT. Comments capture hidden constraints, subtle invariants,
  workarounds for specific bugs.
- **No error handling for impossible scenarios.** Trust internal code
  and framework guarantees. Validate only at system boundaries (user
  input, external APIs).
- **No backwards-compatibility hacks** for already-removed surfaces.

### Settings split (THREE tiers — don't merge)

- **OA-wide** (e.g. controller-nav A/B swap, animation budget): lives
  in `Settings → <category>`.
- **Per-system** (e.g. SNES default core, NDS layout): lives in
  `Settings → Library → Per-system drill-in → <category>`.
- **Per-game** (e.g. Cheats for THIS game, scaling override for THIS
  game): lives in `TileContextMenu → Per-game settings ▸`.

### Audio architecture

- **4 buses:** `platform-music` / `ui-sounds` / `ceremony` /
  `snap-audio`. Mixed via rodio/symphonia.
- **Per-system SFX cascade:** operator override → per-system bundle
  → `_baseline` → silence. Resolved by `resolve_ui_sound` in Rust;
  dispatched by `playSystemUiSound` in TS.

### Controller nav

- **L1/R1 ALWAYS cycles top-toolbar tabs**, never transfers focus
  between regions.
- **DPad/stick LEFT-RIGHT** transfers between sidebar↔center↔right
  pane (DPad source-gated).
- **DPad/stick UP-DOWN** stays within current region.
- **A confirms, B cancels** (with A↔B swap toggle in Settings →
  Display → Controller-nav).

---

## 7. File / directory map (orientation)

```
G:\RustEmu\
├── apps\oa-shell\         Single Tauri+Rust binary
│   ├── src\main.rs        Most Tauri commands + BIOS checks + dispatch
│   ├── src\library_db.rs  SQLite (games, folders, rom_hashes, etc.)
│   ├── src\scan_service.rs  Async folder walker + smart-classify
│   ├── src\rom_hashes.rs  SHA-1 ID against libretro-database
│   ├── src\rom_header.rs  Per-system header rules (iNES, SMC, etc.)
│   ├── src\core_installer.rs  Catalog + download_core + available_cores
│   ├── src\core_options.rs  Per-system core-options schema + values
│   ├── src\bindings.rs    45-system default-binding dispatch
│   ├── src\media.rs       Media DB + libretro-thumbnails sync
│   └── ... (40+ other modules)
├── crates\
│   ├── oa-core\           Core trait + shared types
│   ├── oa-libretro\       libretro FFI + LibretroCore impl
│   ├── oa-render\         wgpu pipeline + WGSL shaders
│   ├── oa-audio\          cpal audio sink + mixer
│   ├── oa-input\          gilrs + keyboard
│   └── ...
├── frontend\src\
│   ├── App.tsx            Top-level Solid app + Retroverse mount
│   ├── components\        Modals, dialogs, widgets, ImportWizard
│   │   └── import-wizard\ Slice 2-6 components (results table, readiness, BIOS resolution, missing-core install)
│   ├── routes\retroverse\ 6-tab Retroverse Shell (HomePage, LibraryPage, etc.)
│   ├── library\           Store, ingest, types
│   ├── themes\registry.ts 41 systems with extensions + display names
│   ├── nav\               focus manager, gamepad poller, HintBar
│   └── settings\          Solid store + persisted prefs
└── docs\
    ├── CHATGPT_BRIEFING.md  ← THIS FILE
    ├── VISION.md          The pitch
    ├── ACTIVE_WORK.md     What's in flight + recently completed
    ├── NEXT.md            Cross-system priority queue (HIGH/MEDIUM/LOWER/DEFERRED bands)
    ├── DECISIONS.md       Append-only project decisions
    ├── PARKING_LOT.md     Out-of-scope ideas (some closed, some Won't fix)
    ├── INDEX.md           Routing table
    ├── PLANS\             Per-arc design plans (locked before code)
    │   ├── guided-setup.md  488-line plan; Phase 1B just shipped
    │   ├── per-system-ui.md  3-stage plan; Stage 1 shipped
    │   ├── retroverse-ui-rollout.md  6-tab IA rollout
    │   ├── content-packs.md  Phase C6 (DISCOVER tab content)
    │   └── ... (10+ more)
    ├── features\          Per-arc folders with README/ROADMAP/SESSION_LOG/DECISIONS
    │   ├── controller-nav\
    │   ├── guided-setup\  ← Phase 1B shipped today
    │   ├── per-system-ui\
    │   ├── retroverse-ui\
    │   ├── media-taxonomy\
    │   ├── library-import\
    │   └── ... 10 total
    └── cores\             45 per-system folders (README, ROADMAP, SESSION_LOG, KNOWN_GAME_BUGS, games-info.md)
```

---

## 8. Where ChatGPT can help

Useful prompts to give ChatGPT once it's read this briefing:

### Gap-spotting on shipped features

- "Looking at OA's shipped features, what's a feature operators of
  similar tools (RetroArch, LaunchBox, EmulationStation, Pegasus) get
  that doesn't appear in this briefing?"
- "What import-wizard edge cases might OA's Slice 1-6 implementation
  not handle? Operator-provided ROM in a weird archive format,
  symlinks, network drives, ROM-set vs flat-file confusion, etc."
- "What corner cases in BIOS handling might be unhandled? Partial
  dumps, byte-swapped variants, alternate filenames the curated SHA-1
  table doesn't know."
- "What systems on the long list (Amiga, ZX Spectrum, C64, X68000,
  PC-88, etc.) are operators most likely to want first, and what
  libretro cores exist for them?"

### Feature ideation

- "Given the 'overlooked first' philosophy and the 'curator-enthusiast'
  voice, what would a 'first-time-user playing a system they've never
  seen' onboarding mini-flow look like? Different from the wizard —
  this is in-library when launching a game of an unfamiliar system."
- "OA has per-system theming, per-system audio, per-system boot
  animations. What ELSE could be per-system that would feel
  curator-class? (Per-system save-state slot art? Per-system pause
  overlay? Per-system loading hints?)"
- "The Game Info Panel v1 ships with operator-edited per-game
  controls/recommended-core/bugs/notes. What additional fields would
  curators want? Soundtrack info? Composer? Engine? Predecessors /
  successors?"

### Risk-spotting

- "OA is GPL-2.0 with a planned permissive pivot once builds ship
  only operator-controlled .dlls. What's a gotcha in that transition?"
- "OA serves couch gamers primary, cabinet builders secondary,
  desktop tertiary. Are the shipped Settings flows controller-friendly
  enough? What about Per-system drill-in's 45-system picker?"
- "BIOS WARN semantics (copy regardless of hash; flag as 'unknown
  hash'): what's the risk profile for operators with downloaded BIOS
  files that *look* canonical but are subtly tampered?"

### Direction-setting

- "After Phase 2 (curated CPU-tier cores) ships, what's the highest-
  leverage next major arc? Per-System UI Stage 2 vs RetroAchievements
  vs content-packs Phase C6 vs starting on Amiga / C64?"
- "OA hasn't decided on Netplay yet (multi-month, risk of shipping a
  worse version for years). What's a smaller, lower-risk
  'multiplayer-adjacent' feature that captures some of the value?
  (Shared save states? Twitch-style 'crowd controls' for slow puzzle
  games?)"

### Inform per-system bring-up

- "OA is about to start serious bring-up on X system. What are the
  operator-experience pitfalls specific to that system that other
  emulators handle poorly? E.g. NDS dual-screen layout, GameCube
  high-res rendering perf tradeoffs, MAME ROM-set version drift."

---

## 9. Quick stats (snapshot 2026-06-01)

- **41 systems** wired end-to-end
- **615 oa-shell Rust tests** passing
- **`npm run typecheck` silent** across the frontend
- **6 phase arcs in active rotation** (Retroverse UI, Per-System UI,
  Game Info Panel, Guided Setup, KNOWN_GAME_BUGS migration,
  Content-packs)
- **5 SHIPPED phase arcs** today: Sidebar, UI-polish, Media-taxonomy,
  Portable-install, Controller-nav, Guided Setup Phase 1B
- **~25 in-progress per-core ROADMAPs**
- **License:** GPL-2.0 today; permissive (MIT/Apache 2.0) pivot
  planned post-forked-core-builds
- **Platforms supported:** Windows (primary), Linux + macOS targets
  exist but Windows is the only CI matrix as of 2026-06-01

---

*End of briefing. If ChatGPT wants more depth on any specific area,
the live docs at `docs/PLANS/` / `docs/cores/` / `docs/features/` are
the authoritative source. This briefing is a snapshot; refresh it as
the project evolves.*
