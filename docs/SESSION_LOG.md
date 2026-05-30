# Session Log

Project-wide milestone log. Per-core day-to-day work goes in `docs/cores/<core>/SESSION_LOG.md`. This file is for cross-cutting milestones (phase boundaries, shell-level shipped features, new systems coming online).

Format: date + three lines — **Shipped / Almost / Next**.

---

## 2026-05-30 — libretro env-callback batch (four gaps closed)

Closes four high-leverage libretro `cb_environment` arms that were previously
unhandled or accept-ignored. Single feature branch
`feat/libretro-env-callbacks-batch`, four phase commits, merged `--no-ff` as
`3b35a41`.

- **Shipped (`SET_MEMORY_MAPS` storage):**
  - New `oa_core::MemoryDescriptor` (metadata only — `flags` / `offset` /
    `start` / `select` / `disconnect` / `len` / `addrspace`) + `Core::memory_map()`
    trait method.
  - `crates/oa-libretro/src/state.rs` parses the descriptor array; metadata
    stored in `State.memory_descriptors`, host base pointers separately in
    `State.memory_map_ptrs` as `usize` so State stays `Send`.
  - Cleared on `load_rom` alongside rotation so back-to-back swaps don't
    inherit stale state.
  - 3 unit tests cover null pointer / zero count / 2-region NES-shape map.
  - Unblocks future RetroAchievements rcheevos integration, cheat-search
    address translation, AI/scripting memory reads.
- **Shipped (`SET_MESSAGE` / `SET_MESSAGE_EXT` → toast):**
  - New `oa_core::CoreMessage` + `CoreMessageLevel` + `Core::drain_messages()`.
  - Env arms for env 6 (legacy frames-based) + env 60 (modern with level /
    target / priority); `GET_MESSAGE_INTERFACE_VERSION` (env 59) returns v1
    so modern cores prefer the richer path.
  - Shell drains per render frame in `run_emu_render`, emits each entry as
    `oa://toast` via existing `emit_toast(level, system, text)`.
  - `target=LOG` messages log-only (skip toast); cores' OSD on save state /
    disc swap / cheat apply / BIOS fallback now surface visually.
- **Shipped (`SET_SUPPORT_NO_GAME` + `load_no_rom()`):**
  - Env arm 18 captures the bool into `State.supports_no_game`;
    `LibretroCore::supports_no_game()` accessor + `LibretroCore::load_no_rom()`
    calls `retro_load_game(NULL)` for DOSBox-Pure / ScummVM bootless mode.
  - Refactored shared post-load work into `finish_load()` so `load_rom` and
    `load_no_rom` stay in lockstep.
- **Shipped (disc-control v2 extras):**
  - `LibretroCore::add_disc_image()`, `replace_disc_image(idx, path)`,
    `set_initial_disc_image(idx, path)`, `disc_image_path(idx)`.
  - `oa_core::DiscInfo` gains `paths: Vec<String>` populated from
    `get_image_path` for v2 cores; v1 fallback returns empty.
  - `read_disc_string_field` helper collapses label / path buffer-fill
    duplication.
  - Frontend `QuickSettings.tsx` `DiscInfo` type extended with `paths`
    field for future tooltip polish.
- **Almost:** UI hook for `load_no_rom()` — bootless launch button for DOSBox
  / ScummVM. Infrastructure is in; operator-facing wiring is its own ~30-line
  follow-up if the bootless workflow becomes a real ask.
- **Next:** the remaining big libretro infra gap is `SET_HW_RENDER` — the
  multi-week task that unblocks Beetle PSX HW / Mupen64Plus-Next /
  PPSSPP / Beetle Saturn HW / Flycast at their real quality tier.

---

## 2026-05-21 — Direct-launch Phase I — explicit #inner, CD-in-archive, --state-file restore

Three follow-ups to direct-launch shipped on top of `main`. Closes
out the load-bearing PARKING_LOT items for the CLI feature.

- **Shipped (explicit `<archive>#<inner>` syntax):**
  - `resolve_explicit_archive_inner` in cli.rs — bypasses Phase H's
    single-ROM requirement; the operator can address one ROM out of a
    multi-game archive without scanning the library first.
  - Inner is validated against `archive::list_rom_contents`; typos
    error with the available-inner list (new
    `CliError::ArchiveInnerNotFound`).
  - Cart inners auto-infer the system via `slug_for_ext`; CD inners
    require `--system`.
- **Shipped (CD-in-archive auto-extract):**
  - `resolve_archive` peek filter extended to accept .cue / .ccd /
    .toc / .m3u in the accepted-extensions set.
  - Single CD inner with `--system` → `archive_inner_path` set;
    `launch_rom`'s existing `is_cd_entry_extension` branch fires
    `archive::extract_to_temp` to `appData/temp/<entryId>/`.
  - Synthesized RomEntry's id + filePath fold the inner path in
    (`<archive>#<inner>` encoding) so different CDs in the same
    archive get distinct entryIds and reuse-then-clean their own
    temp dirs.
- **Shipped (`--state-file PATH` actual restore):**
  - `EmuCommand::LoadRom.restore_state_path: Option<PathBuf>` added.
  - `launch_rom` Tauri command takes `stateFile: Option<String>`,
    threaded through `launchRom` JS → `handleLaunch` → cascade.
  - Emu thread's LoadRom handler reads + `core.load_state` from the
    absolute path after the rom load completes, atomically. Toast on
    read/deserialize failure.
  - CLI parse: `--slot` and `--state-file` mutually exclusive
    (RetroArch convention). State-file existence validated upfront
    so a missing file errors before any subprocess work
    (new `CliError::StateFileMissing`).
- **PARKING_LOT swept:** five direct-launch items closed (Phase H
  + Phase I); three new deferrals added for the CLI v2 batches the
  operator chose to skip (launcher-parity flags, kiosk / arcade,
  diagnostics).
- 309/309 tests green. tsc --noEmit clean.

---

## 2026-05-21 — Direct-launch Phase H — archive auto-extract + Windows-release error visibility

Two same-week fast follow-ups on the direct-launch branch driven by
operator real-world testing.

- **Shipped (Windows release error visibility):**
  - `windows_subsystem = "windows"` (release builds) means stderr is
    silently dropped — operators spawning the .exe from cmd / LaunchBox
    / a double-click saw "nothing happens" on CLI validation errors.
  - New `win_msgbox::error` Windows-FFI shim (linked against user32) +
    `CliError::emit_banner` always pops a native MessageBox on Windows
    release. Debug builds keep using the stderr banner.
- **Shipped (Phase H — single-ROM archive auto-extract):**
  - `.zip` / `.7z` direct-launch now peeks inside. Exactly one cart-ROM
    file → transparently used; system inferred from inner extension
    (or honored from `--system`). MAME / Neo Geo pass the archive
    through as-is via `--system mame` (or `neogeo`) or the `.p1+.s1`
    Neo Geo signature auto-detection.
  - Empty / multi-ROM archives error out with a list (and remediation
    hint pointing at the Import Wizard).
  - `DirectLaunchConfig.archive_inner_path` + DTO mirror flow through
    to the frontend's synthesized RomEntry, which forwards it to
    `launch_rom` so the existing `archive::extract_for_launch`
    plumbing runs identically to a library launch.
  - Hash-lookup hashes the inner ROM bytes (via
    `archive::read_inner_to_bytes`) to match the library DB's sha1
    convention — per-game overrides apply for scanned archived games.
  - `accepted_rom_extensions()` restricted to cart shapes only —
    CD-in-archive support is a separate v2 enhancement.
- **Almost:** Multi-ROM-archive launching via explicit
  `<path>#<inner>` syntax. CD images inside archives.
- **Next:** Operator plays through end-to-end on
  `feat/direct-launch-cli` (positional .nes, positional .zip wrapping
  a .sfc, --system mame on a MAME romset, explicit-error paths). Merge
  to main after thumbs-up.

---

## 2026-05-20 — Direct-launch CLI mode (LaunchBox / BigBox / EmulationStation compat)

External-frontend integration ships. `oa-shell.exe "C:\ROMs\game.nes"`
boots straight into the game with no library UI, the way standalone
emulators do. Default zero-arg behavior unchanged.

- **Shipped:**
  - New `apps/oa-shell/src/cli.rs` module (clap derive) parsing
    positional ROM / `--rom` / `--core` / `--system` / `--slot` /
    `--state-file` / `--tas-replay` / `--fullscreen`. Unambiguous
    cart extensions auto-infer the system; CD-shaped extensions
    require `--system`. Error banners + `process::exit(2)` on
    validation failures.
  - `DirectLaunchConfig` on `AppState`; new Tauri commands
    `get_direct_launch_config` + `get_game(id)`.
  - Forced single-window at runtime when direct-launch is set;
    operator's `OA_SHELL_MODE` / `shell.json` preference preserved
    on disk.
  - `library_db::find_game_by_sha1` (uses existing `idx_games_sha1`)
    + boot-time SHA-1 lookup for cart-shaped ROMs. Matched library
    rows carry their per-game overrides (patches, custom core options,
    shader, rewind config, analog routing, bezel) through the
    standard launch cascade.
  - Frontend: `directLaunchConfig` resource + `isDirectLaunch` memo
    + `Shell.fullBleed` wiring + JSX `<Show>` guards collapse chrome
    to game surface + Quick Settings / Save Slots / Game Info /
    Performance HUD / Toast Stack.
  - `createLibraryStore({ shouldBootstrap })` short-circuits
    `list_games` / `list_game_groups` / migration / seed insertion
    in direct-launch.
  - Auto-launch effect re-uses existing `handleLaunch` cascade so
    per-game / per-system / OA-wide settings, milestones, cheats,
    analog routing all arm normally.
  - Exit-on-unload: emu thread emits `oa://rom-unloaded` after the
    UnloadRom drain; frontend listener calls `quit_app` in
    direct-launch. Quick Settings "Exit to library" relabels to
    "Quit".
  - `OA_ROM` env-var still honored as silent fallback; CLI args win
    when both set.
  - Pre-existing build blocker fixed: removed stale `#[cfg(test)]`
    gate on `sha1::Sha1` import in `rom_hashes.rs`.
  - `docs/direct-launch.md` operator usage doc.
  - 9 new cli.rs unit tests; `cargo test -p oa-shell` 309/309 green.
- **Almost:** `--state-file PATH` accepted by clap but not wired
  yet (frontend logs a warning; operators should use `--slot`).
  Future work: a `restore_state_file` Tauri command, then plumb.
- **Next:** Operator play-tests the branch (`feat/direct-launch-cli`)
  end-to-end — positional launch, --system + CD launch, hash-matched
  per-game overrides applying, Quick Settings overlays working,
  close-window-exits, LaunchBox / EmulationStation real-world
  invocation. Merge to main after thumbs-up.

---

## 2026-05-20 — Sony+Nintendo handheld pass + POINTER infra (systems #34-36: psp + ps2 + nds)

Seventh paired pass of the day. **Second cross-cutting input
infrastructure** of the session — the POINTER device dispatch (mouse-
as-touch) joins this morning's analog input infra to round out OA's
modern-controller input model. NDS, the platform that requires it
most, ships immediately playable. PS2 slots into the CD-launch BIOS
dispatch arm as the 9th system; PSP is BIOS-free.

- **Shipped (cross-cutting POINTER infra):**
  - `oa_core::InputState` extended with `pointer: (i16, i16, bool)`
    field — x, y normalized to libretro POINTER range
    (-32768..32767), plus the pressed flag.
  - `oa-libretro::ffi` — new RETRO_DEVICE_POINTER (6) constant +
    RETRO_DEVICE_INDEX_ANALOG_POINTER_LEFT/RIGHT/BUTTON +
    RETRO_DEVICE_ID_POINTER_X/Y/PRESSED/COUNT.
  - `oa-libretro::state::State` — new `input_pointer: [(i16, i16, bool); 5]`
    field (per-port pointer state).
  - `cb_input_state` extended to dispatch RETRO_DEVICE_POINTER
    queries to the stored pointer state per port/id (X/Y axes,
    pressed flag, count).
  - `LibretroCore::set_input` stores `input.pointer`.
  - `oa-input::InputPoller::poll` — new `poll_pointer()` helper reads
    mouse position via device_query (the same DeviceState the
    keyboard polling uses) + left-button state. Normalizes screen
    coordinates to libretro range (assumes 1920×1080 at Phase 0;
    window-relative pixel-perfect mapping is Phase 2.5).
  - Emu thread updated to plumb `polled.pointer` through to
    `core.set_input`; TAS replay paths set pointer to defaults
    (TAS pointer recording is Phase 2.5).
- **Shipped (Rust core):** Three new `oa_core::SystemId` variants
  (`Psp`, `Ps2`, `Nds`) + parse_system_id arms with aliases.
- **Shipped (bindings):** Three new modules.
  - `psp` — 12 digital buttons (PSX-shape: d-pad + 4 face diamond +
    L/R + START + SELECT). **No L2/R2** — PSP hardware lacks them.
    Single analog stick via shared infra.
  - `ps2` — 16 digital buttons (DualShock 2: PSX-shape + L3/R3 stick
    clicks). Dual analog sticks via shared infra. Pressure-sensitive
    face buttons + analog L2/R2 = Phase 2.5.
  - `nds` — 12 digital buttons (Nintendo diamond: A east PRIMARY,
    B south secondary, X north, Y west; matches nes/snes/gb/gba
    precedent). Touch screen via new POINTER infra.
  Nine new tests lock the dispatch.
- **Shipped (default cores):** psp → `ppsspp_libretro.dll` (BIOS-
  free). ps2 → `pcsx2_libretro.dll` (LRPS2, BIOS-required). nds →
  `melonds_libretro.dll`.
- **Shipped (BIOS pre-checks):**
  - `check_ps2_bios` + `PS2_BIOS_KNOWN_HASHES` (6 entries covering
    JP launch / US fat / US-EU slim variants). Slotted into CD-launch
    dispatch as 9th CD-shape system (pce-cd / segacd / saturn / psx /
    neocd / 3do / pcfx / dreamcast / ps2).
  - `check_nds_bios` + `NDS_BIOS_KNOWN_HASHES` (**new multi-file
    BIOS shape** — requires ALL THREE files: bios7.bin + bios9.bin +
    firmware.bin). Cart-shape pre-check arm in main.rs next to neogeo.
    First multi-file BIOS check in OA's lineup.
  - psp is BIOS-free.
- **Shipped (media + rom_hashes):** Three new repo arms
  (`Sony_-_PlayStation_Portable`, `Sony_-_PlayStation_2`,
  `Nintendo_-_Nintendo_DS`). rom_hashes: psp + nds → single-file
  no-intro dats (.iso/.cso/.pbp for psp; .nds for nds); ps2 → `&[]`
  with NO_DAT_SYSTEMS entry (DVD images deferred).
- **Shipped (frontend):** SystemId union extended. Three new
  `systemThemes` entries. Three new CSS blocks (Plan A — Sony
  cool cluster + Nintendo handheld pearl):
  - **psp:** cool cyan `oklch(0.65 0.18 200)` — middle of the new
    Sony cluster (psx 180° / psp 200° / ps2 215°).
  - **ps2:** deep cobalt `oklch(0.45 0.22 215)` — bottom of the
    cool cluster lightness ladder; period-correct to PS2 blue logo
    + dark-hardware-era marketing.
  - **nds:** pearl yellow-green `oklch(0.78 0.14 95)` — Nintendo
    handheld pearl pattern (matches ngp 105° / WS 305°).
- **Shipped (docs):** ACTIVE_CORE → `nds` (POINTER infra leadership).
  Three full per-core docs sets at `docs/cores/{psp,ps2,nds}/` (15
  doc files). Decisions captured: PPSSPP/LRPS2/melonDS defaults,
  Sony cool cluster theme, Nintendo handheld pearl theme, NDS A is
  PRIMARY per Nintendo convention, multi-file BIOS check pattern
  (3-file NDS check) + 6-entry PS2 BIOS table + POINTER-infra
  rationale.
- **Plan:** Flipped 3 rows ⬜ → ✅; bumped count from 33 to 36.
  **Wave 4 (Nintendo handhelds) COMPLETE + Wave 5 (Sony) COMPLETE.**
  Order-of-attack reduced to 3 groups remaining (scummvm+dosbox,
  5200, pokemini).
- **Validation:** `cargo test -p oa-shell --bin oa-shell` 269/269
  green (was 260; +9 tests across 3 systems × 3 each). `cargo check`
  on workspace clean (POINTER trait changes affect oa-core /
  oa-libretro / oa-input cleanly). Frontend `npm run typecheck`
  silent.
- **Almost:** Phase 1 operator validation. **psp** — drop `ppsspp_libretro.dll`
  + gamepad → God of War: Chains of Olympus. **ps2** — drop
  `pcsx2_libretro.dll` + regional BIOS → Shadow of the Colossus.
  **nds** — drop `melonds_libretro.dll` + 3 BIOS files → Phantom
  Hourglass (canonical "POINTER infra works" test — mouse should
  control Link's stylus).
- **Next:** 36 systems shipped (over 100% of original 34-plan —
  scope expansion landed faster than anticipated). Order-of-attack
  next pick is **`scummvm` + `dosbox`** — engine cores that need a
  folder-as-game scanner extension before they slot in cleanly.

---

---

Older entries (everything 2026-05-20 and earlier) live in [SESSION_LOG_ARCHIVE.md](SESSION_LOG_ARCHIVE.md).
