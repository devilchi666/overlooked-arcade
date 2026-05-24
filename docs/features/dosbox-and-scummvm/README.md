# DOSBox + ScummVM onboarding

> **Status: 📐 PLANNED, not implemented.** Design locked 2026-05-24 in
> conversation with operator after the media-taxonomy + cart-shape-BIOS
> work landed. Implementation is the next code session; pick up by
> reading this doc cover-to-cover, then confirming the 8 locked
> decisions before any code starts.

## Context

OA's 38 currently-live cores all share one shape: a "game" is a single
ROM file (or archive of one ROM). The shell scans for files matching
per-system extension allowlists; launch hands the file's bytes to
`retro_load_game`; per-game state (saves, overrides, cover art) keys
off the file path's stem.

Two cores from the original plan are deferred because their game
abstraction doesn't fit that shape:

- **ScummVM** — adventure-game engine. A "game" is a directory of
  data files (`MONKEY.000`, `MONKEY.001`, …) plus an engine selector
  (`scumm`, `lure`, `agi`, …). The libretro core loads a tiny
  `.scummvm` descriptor file that references the game by ID and tells
  the engine where to find the data.
- **DOSBox** (`dosbox-pure` libretro core) — DOS-game runner. A
  "game" is a directory containing an entry-point .exe + data files
  + optional `dosbox.conf`. The libretro core auto-detects the
  entry point at launch.

Both deferred at the 2026-05-20 system-wiring pass with the note
"PC games, different launch semantics from console emulation."

**Trigger (2026-05-24):** post-media-taxonomy + cart-bios-checks the
operator's instinct was right — every new piece of cross-system
infrastructure (audio override UI, kiosk-shell tile UI, etc.) adds
a per-core onboarding line item, so the per-core tax compounds.
Better to land the two remaining cores BEFORE the next infra wave
rather than pay the upgrade tax on each one separately later.

**Outcome:** two new cores wired into OA's existing sidebar / library
/ media / settings model — no separate "PC games" UI surface, no new
launcher app. The engine-launcher difference is contained inside
the launch dispatch (file vs. directory passed to `retro_load_game`)
and the per-system scanner (filename filter vs. directory-as-game
model). Everything downstream — cover art, save states, per-game
overrides, audio overrides — works the same as for console games.

## Locked design decisions

(All decided in conversation with operator 2026-05-24.)

| # | Decision |
|---|---|
| 1 | **Both ship as ordinary OA systems** in the existing sidebar / library / settings model. No separate "PC games" UI section, no separate launcher app, no new top-level concept. The operator sees two more entries in the sidebar alongside Genesis, SNES, etc. |
| 2 | **ScummVM scan = walk for `.scummvm` files** at any depth under the library folder. The `.scummvm` file is the operator-curated "this is a game" marker. OA does NOT auto-detect raw game directories — operators (or third-party tools like LaunchBox's ScummVM importer) create the `.scummvm` files; we just consume them. |
| 3 | **DOSBox scan = walk for subdirectories at exactly one nesting level deep** under the library folder. Each subdirectory = one game. Game title = directory name. Nested subdirectories are content (game data), not nested games. |
| 4 | **No engine-selection UI.** ScummVM's libretro core auto-detects engine from the `.scummvm` file's game ID. DOSBox-pure auto-detects entry point from the game directory contents. Operators don't pick engines in OA. |
| 5 | **No BIOS check for either.** Neither has a traditional BIOS — ScummVM ships its own .ini configuration; DOSBox-pure ships its own DOSBox runtime. The cart-shape BIOS dispatch arm gets two more "no-op" pass-through cases. |
| 6 | **GameRow.file_path semantics extended per system:** for scummvm, it's the absolute path to the `.scummvm` file; for dosbox, it's the absolute path to the game directory. Launch dispatch reads `system_id` and decides what to pass `retro_load_game`. |
| 7 | **Cover art via the existing media pipeline.** Same `media/<sys>/<kind>/<rom_stem>.<ext>` shape. `rom_stem` derivation: scummvm = `.scummvm` filename without extension; dosbox = directory basename. LaunchBox art packs key on "ScummVM" / "MS-DOS" platform names — both folded into Phase 3's `launchbox_platform_to_system_id` map. |
| 8 | **Bindings = mouse-primary + keyboard passthrough + optional gamepad fallback.** Both engines rely heavily on point-and-click. POINTER device infra (Phase 2.5) + keyboard passthrough (Phase 6) already cover this; per-system bindings modules just declare the surface. No new input infrastructure. |

## Storage model

```
<library-folder>/                              ← operator-chosen, ANY path
  ScummVM/                                     ← subfolder per system (convention, not requirement)
    Monkey Island/
      MONKEY.000
      MONKEY.001
      ...
    Monkey Island.scummvm                      ← single-line text: "monkey:scumm"
    Day of the Tentacle/
      TENTACLE.000
      ...
    Day of the Tentacle.scummvm                ← "tentacle:scumm"

  DOS Games/
    Doom/                                      ← scanned as one game
      DOOM.EXE                                 ← auto-detected entry point
      DOOM.WAD
      DOOM2.WAD
      ...
    Wing Commander/
      WC.EXE
      *.DAT
      DOSBOX.CONF                              ← optional, dosbox-pure honors per-game
    X-COM UFO Defense/
      ufo.exe
      ...
```

The `.scummvm` file lives **next to** its game data directory (per
LaunchBox convention). ScummVM core's `system_dir` gets pointed at the
parent so it finds the data — handled at launch time by setting the
core's system path to `<scummvm_file_dir>` before retro_load_game.

For DOSBox, the directory IS the input; dosbox-pure walks the
directory at load time and auto-detects boot path. Per-game
`dosbox.conf` (if present) overrides the auto-detection with
operator-specified `[autoexec]`.

## Data model changes

### `oa_core::SystemId`

```rust
ScummVm,
DosBox,
```

### `apps/oa-shell/src/main.rs`

- `parse_system_id` arms for `"scummvm"` / `"dosbox"` (+ aliases `"dos"`, `"ms-dos"`).
- `default_core_dll_for_system`:
  - `"scummvm" => "scummvm_libretro.dll"`
  - `"dosbox" => "dosbox_pure_libretro.dll"`
- **Launch dispatch (the engine-launcher special case):**
  - For `scummvm`: `file_path` is the `.scummvm` file → read its bytes → pass to `retro_load_game` as-is; ALSO set the core's `system_dir` to the file's parent directory so the engine finds game data.
  - For `dosbox`: `file_path` is the game directory → the core accepts a directory path directly; pass the path string (NOT bytes) via the libretro `path`-shape load arm.
  - Wrap the dispatch in a small `enum LaunchPayload { Bytes(Vec<u8>), DirectoryPath(PathBuf), DescriptorFile(PathBuf) }` so the launch site reads cleanly and future engine cores slot in without refactoring.
- No BIOS dispatch arms for either.

### `apps/oa-shell/src/bindings.rs`

- `bindings::scummvm` module:
  - 4 buttons: LMB / RMB / LMB-double / Escape (= "back to main menu" in most ScummVM titles)
  - POINTER device for cursor position via mouse / gamepad-right-stick
  - Keyboard passthrough for text input (sword-fighting insults, password prompts)
- `bindings::dosbox` module:
  - 8-button gamepad layout for arcade/action DOS games (Doom, Wolf3D, Commander Keen, Jazz Jackrabbit):
    A=jump/use, B=shoot, X=reload, Y=secondary, L/R=strafe, Start=ESC, Select=TAB
  - POINTER for mouse-driven games (X-COM, SimCity, Civilization)
  - Keyboard passthrough for everything that needs it (most DOS games)
- `parse_button_name`, `to_libretro_bits`, `bit_for`, `buttons_for` arms.

### `apps/oa-shell/src/scan_service.rs` (or wherever library scan lives)

- New per-system scan-mode dispatch:
  - `scummvm` → walk recursively for `.scummvm` extension, use file_path as the game identifier, title = filename stem.
  - `dosbox` → walk one level deep for subdirectories, use directory path as the game identifier, title = directory basename.
  - All other systems → existing file-extension scan model (unchanged).
- The scan model dispatcher is a small enum (`ScanMode::Files { extensions } | ScanMode::Descriptors { extension } | ScanMode::Directories { depth }`) so future engine cores slot in.

### `apps/oa-shell/src/library_db.rs::GameOverrides`

- New optional `dosbox_entry_point: Option<String>` field for the
  ~10% of DOS games dosbox-pure can't auto-detect. Value is a path
  relative to the game directory, e.g. `"INSTALL.EXE"` or
  `"DOSBOX/AUTOEXEC.BAT"`. Wired through to dosbox-pure's per-game
  conf at launch. Optional, default None → use auto-detection.

### Frontend

- `frontend/src/themes/registry.ts`:
  - `SystemId` union extended with `scummvm` and `dosbox`.
  - `systemThemes.scummvm`: extension `[".scummvm"]`, tile aspect 1/1
    (cover-art-shape), default shader `plain` (no CRT effects for
    pixel-art adventures shown at native resolution).
  - `systemThemes.dosbox`: extension `[]` (directory-based scan,
    handled via scan_service dispatch not extension matching), tile
    aspect 4/3 (most DOS games target 320×200 or 320×240), default
    shader `plain`.
  - Form-factor tags: both `"computer"` for the Computers sidebar group.
- `frontend/src/themes/systems.css`:
  - `scummvm`: teal-cyan accent (hue 195°, L=0.62, C=0.16) — adventure-game ocean / dialogue-box vibe.
  - `dosbox`: amber-on-black accent (hue 55°, L=0.65, C=0.18) — DOS prompt amber CRT vibe.
- `frontend/src/components/SystemHeader.tsx`: no changes — both systems work with existing wheel-art fallback to short-name chip.

### Art-pack importer

- `art_pack_importer::launchbox_platform_to_system_id` arms:
  - `"ScummVM" => "scummvm"`
  - `"MS-DOS" => "dosbox"`
  - `"DOS" => "dosbox"` (some packs use the older naming)

### libretro-thumbnails

- `media::repos_for_system_id`:
  - `"scummvm" => &["ScummVM"]`
  - `"dosbox" => &["DOS"]`
- Both repos exist at `github.com/libretro-thumbnails/<name>` per the
  upstream catalog convention. Existing cover-sync infrastructure
  works as-is.

### Hash matching

- `rom_hashes::libretro_dat_refs_for_system`: return empty for both.
  Neither has libretro-database canonical SHA-1 hashes — game data
  files vary (different revisions, different language packs, fan
  translations). Cover sync falls back to fuzzy filename matching at
  the 0.95 threshold; works fine because scummvm filename stems
  match the canonical game name exactly (`monkey:scumm` → operator
  names the file `Monkey Island.scummvm` → matches the LaunchBox
  ScummVM art pack's `Monkey Island.png`).

### Per-core docs

- `docs/cores/scummvm/{README,ROADMAP,SESSION_LOG,DECISIONS,KNOWN_GAME_BUGS}.md`
- `docs/cores/dosbox/{README,ROADMAP,SESSION_LOG,DECISIONS,KNOWN_GAME_BUGS}.md`

## File-by-file change list

### Existing files to modify (~600 lines total)

- **`crates/oa-core/src/lib.rs`** (~10 lines)
  - 2 new SystemId variants

- **`apps/oa-shell/src/main.rs`** (~250 lines)
  - parse_system_id + default_core_dll arms
  - Launch dispatch: LaunchPayload enum + per-system arm in launch_rom
  - System dir override for scummvm (point core at the .scummvm file's parent)

- **`apps/oa-shell/src/bindings.rs`** (~150 lines)
  - 2 new per-system bindings modules

- **`apps/oa-shell/src/scan_service.rs`** (~80 lines)
  - ScanMode enum + per-system dispatch
  - scummvm walker (recursive, .scummvm filter)
  - dosbox walker (1-level-deep directory enumeration)

- **`apps/oa-shell/src/library_db.rs`** (~10 lines)
  - GameOverrides.dosbox_entry_point field (default None)

- **`apps/oa-shell/src/media.rs`** (~20 lines)
  - repos_for_system_id arms

- **`apps/oa-shell/src/art_pack_importer.rs`** (~10 lines)
  - launchbox_platform_to_system_id arms

- **`apps/oa-shell/src/rom_hashes.rs`** (~5 lines)
  - libretro_dat_refs_for_system arms (return empty)

- **`frontend/src/themes/registry.ts`** (~50 lines)
  - SystemId union + 2 systemThemes entries

- **`frontend/src/themes/systems.css`** (~30 lines)
  - 2 accent color blocks

- **`frontend/src/library/ingest.ts`** (~15 lines, if needed)
  - Surface per-system scan-mode UI if the Import Wizard exposes it;
    likely no change since scan-mode is system_id-keyed in Rust.

### New files (~10 doc files)

- 5× `docs/cores/scummvm/*.md`
- 5× `docs/cores/dosbox/*.md`

## Phase plan

1. **Phase 1 — scummvm onboarding (~300 lines)**
   - SystemId variant, parse_system_id, default_core_dll
   - bindings::scummvm module
   - Theme registration + CSS block
   - Scan model: ScanMode enum + scummvm descriptor-file walker
   - Launch dispatch: read .scummvm file bytes + set core system_dir to file's parent
   - libretro-thumbnails repo name + art-pack importer mapping
   - Per-core docs scaffold (5 files)

2. **Phase 2 — dosbox onboarding (~350 lines)**
   - SystemId variant, parse_system_id, default_core_dll
   - bindings::dosbox module (more buttons than scummvm)
   - Theme registration + CSS block
   - Scan model: dosbox directory-1-level-deep walker (extends ScanMode)
   - Launch dispatch: pass directory path to core
   - GameOverrides.dosbox_entry_point field + per-game-conf injection
   - libretro-thumbnails repo name + art-pack importer mapping
   - Per-core docs scaffold (5 files)

3. **Phase 3 — cross-cutting wiring (~80 lines)**
   - art_pack_importer launchbox names
   - rom_hashes empty-refs arms
   - Frontend ingest.ts adjustments if any
   - Tests for both scan models (synthetic library folder + asserts)

4. **Phase 4 — docs + ROADMAPs (~50 lines)**
   - Flip Phase 0 ✅ entries in both new ROADMAPs after operator
     end-to-end playtest
   - Update docs/ACTIVE_WORK.md to move scummvm/dosbox from "deferred"
     to "shipped"
   - Update docs/NEXT.md to remove the deferral references
   - Update docs/VISION.md — note that all 40 cores are live (was 38)

5. **Phase 5 — SESSION_LOG + merge (~30 lines)**
   - SESSION_LOG entry under this feature folder
   - --no-ff merge after operator approval
   - Delete branch both sides

## Critical files to reference

- **`apps/oa-shell/src/main.rs`** — `default_core_dll_for_system`,
  `parse_system_id`, the launch_rom dispatch site (where bytes get
  loaded into the core today; this is where the engine-launcher
  special cases land).
- **`apps/oa-shell/src/scan_service.rs`** — existing per-system scan
  flow; the ScanMode dispatch lands here.
- **`apps/oa-shell/src/bindings.rs::pce` / `bindings.rs::nes`** —
  reference shape for a per-system bindings module; the scummvm /
  dosbox modules mirror this shape with the appropriate button set.
- **`apps/oa-shell/src/library_db.rs::GameOverrides`** — extension
  pattern for the new `dosbox_entry_point` field. Same pattern as
  the existing optional Option-shaped fields.
- **Existing onboarded systems' Phase 0 ROADMAP entries** —
  `docs/cores/pokemini/ROADMAP.md` is the most recent reference for
  Phase 0 onboarding shape; use it as the template for the two new
  per-core docs.

## Reuse / existing patterns

- **Per-system theming** — `systemThemes` entry + CSS block + sidebar
  registration is the same 6-line pattern every console followed.
- **Per-system bindings** — `bindings::<system>` module shape is
  standardized; copy from `bindings::pce` or `bindings::nes`.
- **Cover art** — media-taxonomy Phase 1+ shape works as-is. Each
  game's rom_stem (scummvm filename, dosbox directory name) keys
  the canonical art layout.
- **Per-game overrides** — `GameOverrides` already extensible; the
  new `dosbox_entry_point` field follows the existing optional shape.
- **Per-system settings** — `SystemSettings` already extensible;
  scummvm/dosbox per-system audio overrides, region priority, etc.
  all work out of the box.
- **POINTER device + keyboard passthrough** — both shipped
  cross-system; scummvm/dosbox bindings just declare the surface.

## Verification (when implementing)

End-to-end on Windows:

1. **scummvm scan + launch.** Operator drops `scummvm_libretro.dll`
   into `<exe_dir>/cores/`, drops a folder with `Monkey Island/`
   + `Monkey Island.scummvm` into a library folder, rescans. Tile
   appears in the ScummVM library page with the directory-stem title.
   Click launches Monkey Island via ScummVM.

2. **dosbox scan + launch.** Operator drops `dosbox_pure_libretro.dll`,
   drops a `Doom/` directory with `DOOM.EXE` + `DOOM.WAD` into a
   library folder, rescans. Tile appears in DOS library page with
   "Doom" title. Click launches DOOM.

3. **dosbox per-game entry-point override.** Operator sets
   `GameOverrides.dosbox_entry_point = "INSTALL.EXE"` on a game whose
   auto-detect picks the wrong .exe. Next launch boots INSTALL.EXE
   instead.

4. **Cover sync.** Operator runs media sync for both systems → covers
   land from the upstream `ScummVM` + `DOS` libretro-thumbnails repos.

5. **Art-pack import.** Operator points the art-pack importer at a
   LaunchBox Images folder containing `ScummVM/Box - Front/` and
   `MS-DOS/Box - Front/` → fuzzy-matches against library titles
   (scummvm filename stems + dosbox directory basenames) and lands
   covers at canonical paths.

6. **Save states.** F5 + F8 round-trip a save during a DOS game
   (Doom level mid-jump) and during a ScummVM game (Monkey Island
   mid-dialogue). Restore lands correctly.

7. **Per-system bindings + keyboard passthrough.** Operator types
   a sword-fighting insult in Monkey Island → reaches the engine.
   Operator presses ESC in Doom → reaches the game's pause menu.

8. **`cargo test --workspace`** — target ≥ 10 new tests across scan
   model dispatch, scummvm descriptor walker, dosbox directory walker,
   launch dispatch (the LaunchPayload enum + per-system arm).

## Out of scope (deferred)

- **Game scanning auto-detection for ScummVM.** ScummVM's command-line
  tool has a `--detect` flag that walks a directory and emits
  `.scummvm` files automatically. Wrapping this in OA's scan flow
  (so operators don't have to create `.scummvm` files manually) is a
  Phase 2 polish — for v1, operators drop pre-made `.scummvm` files.
- **DOSBox-pure per-game profile UI.** dosbox-pure supports per-game
  CPU cycles, sound card emulation, expanded memory tuning, etc.
  through core options. The existing per-game core-options drawer
  (slice 2.8.D) handles these — no new UI needed.
- **Per-game `dosbox.conf` editor.** Operators can hand-edit the
  `dosbox.conf` file inside the game directory today (dosbox-pure
  honors it). An in-app conf editor is a stretch UX polish — defer.
- **ScummVM Cloud sync.** ScummVM has cloud save sync built-in
  (Google Drive, Dropbox); irrelevant to OA's local-only model.
- **DOSBox Daum / DOSBox Staging cores.** Stock dosbox-pure is the
  default; community cores are operator-installed via the existing
  core picker (slice 2.8.C). No special wiring.
- **Engine-launcher abstraction generalization.** ScanMode enum
  + LaunchPayload enum are designed extensibly so a future
  Game.com / PuzzleScript / Twine engine launcher slots in cleanly,
  but adding more engine systems is a separate later decision.

## Branch + commit plan (when implementing)

1. Pre-feature push (main clean).
2. `git checkout -b feat/dosbox-and-scummvm`.
3. Phase commits in order (1 = scummvm; 2 = dosbox; 3 = cross-cutting;
   4 = docs/ROADMAPs; 5 = SESSION_LOG). Each phase is independent
   enough that operator can test + thumbs-up incrementally.
4. Push after each phase for operator playtest. Phase 1 + 2 each
   benefit from real-hardware playtest (actual ScummVM + DOSBox cores
   in `<exe_dir>/cores/`, actual game data on disk).
5. Final merge `--no-ff` after all 5 phases land + operator approves
   end-to-end. Same shape as the media-taxonomy 7-phase merge.

## Risk register

- **ScummVM `system_dir` setting.** The libretro core needs to know
  where its config + plugin files live. By default OA passes
  `<exe_dir>/system/` as the global system_dir; ScummVM expects to
  find its own `extra/` subdirectory there with engine plugins. We
  also need to override the system_dir per-launch so the engine
  finds GAME data (which lives next to the `.scummvm` file, not in
  `<exe_dir>/system/`). Approach: set the libretro `system_dir`
  global to `<exe_dir>/system/scummvm/` at startup; per-launch set
  the per-game "extrapath" core option to the `.scummvm` file's
  parent directory.

- **DOSBox-pure path encoding.** dosbox-pure accepts paths via the
  libretro `path` parameter (vs. the `data` byte buffer most cores
  use). The OA launch path always passes bytes today. Need to extend
  the launch enum (LaunchPayload — already in the plan above) so
  the dosbox path passes through without the bytes round-trip.

- **Library scan recursion depth.** ScummVM expects descriptor files
  at arbitrary depth (LaunchBox places them next to data folders);
  DOSBox expects exactly-1-deep directories (LaunchBox creates a
  parent `MS-DOS/` folder with each game as a subdirectory). The
  ScanMode enum disambiguates these per system. Don't conflate.

- **Cover art rom_stem mismatch.** ScummVM `.scummvm` filenames vary
  ("MonkeyIsland.scummvm" vs. "Monkey Island.scummvm" vs. "Secret
  of Monkey Island, The.scummvm"). LaunchBox art packs ship under
  the canonical no-intro name ("Monkey Island.png"). Fuzzy match
  at the 0.95 threshold should catch the variation; document the
  rom_stem-naming convention in the scummvm per-core README.

- **DOSBox per-game `dosbox.conf` ownership.** Operators with
  existing dosbox.conf files (typical of a Pure/Daum migration)
  should have those honored. dosbox-pure reads `dosbox.conf` from
  the game directory automatically — no extra wiring needed; just
  document the behavior in the dosbox per-core README so operators
  know they don't need to migrate their tuning.

- **CRC mismatch between dosbox-pure builds.** dosbox-pure ships
  in two variants (libretro-core vs. retroarch-bundled). Both load
  the same way; OA picks whichever .dll the operator drops into
  `<exe_dir>/cores/`. Same as every other core — no special handling.

- **`.exe` extension already used by Windows installers.** OA's
  scanner shouldn't grab arbitrary `.exe` files outside the
  dosbox directory model. The ScanMode::Directories mode for dosbox
  explicitly only looks for SUBDIRECTORIES of the library folder,
  not .exe files. The existing extension allowlist for other systems
  doesn't include .exe.

## Related

- [media-taxonomy/README.md](../media-taxonomy/README.md) — the
  per-system art-pack + audio-bus infrastructure that scummvm +
  dosbox lean on. Phase 3's `launchbox_platform_to_system_id` map
  is the join point for art-pack imports.
- [docs/VISION.md](../../VISION.md) — VISION's "Computers" section
  lists 9 vintage computer systems as long-term ambition. ScummVM
  + DOSBox aren't in that list because they aren't computer systems
  per se — they're engine launchers for PC-hosted games. The two
  lists don't overlap.
- [docs/PARKING_LOT.md](../../PARKING_LOT.md) — no entries for
  scummvm or dosbox today (the deferral was implicit in
  ACTIVE_WORK rather than parked formally). After this plan locks,
  ACTIVE_WORK gets the "in flight" entry and PARKING_LOT stays as
  the long-term-ambition surface for everything else.
