# Session Log

Project-wide milestone log. Per-core day-to-day work goes in `docs/cores/<core>/SESSION_LOG.md`. This file is for cross-cutting milestones (phase boundaries, shell-level shipped features, new systems coming online).

Format: date + three lines — **Shipped / Almost / Next**.

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

## 2026-05-20 — Sega family completion (system #33: dreamcast)

Sixth and final paired pass of the day (this one a solo). Completes
the Sega family — OA now hosts all 7 Sega home/handheld platforms
(SMS, Game Gear, Genesis, Sega CD, 32X, Saturn, Dreamcast). The
CD-launch BIOS dispatch arm grows to its 8th system; the analog
input infra from this morning carries the DC analog stick through
without additional plumbing.

- **Shipped (Rust core):** `oa_core::SystemId::Dreamcast` variant +
  parse_system_id arm (`dreamcast | dc | sega-dreamcast`).
- **Shipped (bindings):** `bindings::dreamcast` module — 11 digital
  buttons (d-pad + A/B/X/Y face diamond + L/R analog triggers +
  START). No SELECT (DC pad doesn't have one). Single analog stick
  flows via shared analog infra (gamepad LeftStick → axes[0..2]).
  Three new dispatch tests.
- **Shipped (default core):** dreamcast → `flycast_libretro.dll`
  (Flycast). Redream documented as per-system alternate.
- **Shipped (BIOS pre-check):** `check_dreamcast_bios` +
  `DREAMCAST_BIOS_KNOWN_HASHES` (4 entries: `dc_boot.bin` universal
  + `dc_flash.bin` US/JP/EU regional variants). Slotted into the
  CD-launch dispatch arm as the 8th CD-shape system (pce-cd / segacd
  / saturn / psx / neocd / 3do / pcfx / dreamcast).
- **Shipped (media + rom_hashes):** New repo arm
  (`Sega_-_Dreamcast`). rom_hashes → `&[]` with NO_DAT_SYSTEMS entry
  (GD-ROM CD images deferred to Phase 2 disc-id extraction via
  IP.BIN signature).
- **Shipped (frontend):** SystemId union extended with `"dreamcast"`.
  systemThemes entry (extensions `.cdi`/`.gdi`/`.chd`, landscape 4/3,
  crt-lite shader). New CSS block: **DC orange swirl
  `oklch(0.55 0.27 32)`** — highest chroma in the warm zone,
  period-correct to 9/9/99 launch marketing + iconic Dreamcast
  spiral logo. The warm zone now hosts 12 systems in 73° (most-
  crowded cluster in OA); each system stays visually distinct via
  L+C profile.
- **Shipped (docs):** ACTIVE_CORE → `dreamcast`. Full per-core docs
  set at `docs/cores/dreamcast/` (5 files). Decisions: Flycast
  default, 4-entry BIOS table (boot + 3 regional flash), period-
  correct orange swirl theme, analog-stick-via-shared-infra,
  no-SELECT-on-DC-pad.
- **Plan:** Flipped 1 row ⬜ → ✅; bumped "Already wired" count from
  32 to 33. **Wave 2 (Sega family completion) COMPLETE** — Sega's
  full home/handheld lineup now wired in OA. Order-of-attack
  reduced to 4 groups remaining (psp+ps2+nds, scummvm+dosbox,
  5200, pokemini).
- **Validation:** `cargo test -p oa-shell --bin oa-shell` 260/260
  green (was 257; +3 tests). Frontend `npm run typecheck` silent.
- **Almost:** Phase 1 operator validation. Drop
  `flycast_libretro.dll` + `dc_boot.bin` + a regional `dc_flash.bin`
  into `<exe_dir>/system/`, mark a DC folder via Import Wizard,
  launch Sonic Adventure / Crazy Taxi / Jet Set Radio / Soulcalibur.
  LeftStick should drive Sonic via the analog infra from earlier
  today.
- **Next:** 33 systems shipped (over 97% of original 34-plan; 82.5%
  if counting scope-expanded remaining). Order-of-attack next pick
  is **`psp` + `ps2` + `nds`** — Sony/Nintendo handheld + home
  heavyweights. All three use analog input (now unblocked) and PS2
  + DS slot CD-shape-style into the now-8-system BIOS dispatch arm.

---

## 2026-05-20 — Nintendo home heavyweights + analog input infra (systems #31-32: n64 + gamecube)

Fifth paired pass of the day, and the **largest infrastructure
investment** of any onboarding to date: the cross-cutting analog
input infrastructure (RETRO_DEVICE_ANALOG dispatch in oa-libretro +
gilrs analog axis polling in oa-input + `InputState.axes` flow
through the emu thread) ships as part of this Phase 0. Without it,
N64 and GameCube would be unplayable on gamepads (both systems use
analog sticks as primary movement input). The infra unblocks N64 +
GameCube immediately and is shared with PSX DualShock / Saturn 3D
Pad / VB right D-pad / Intv 16-way disc for their Phase 2 polish.

- **Shipped (cross-cutting analog infra):** Three crates touched.
  - `oa-libretro::ffi` — new constants for RETRO_DEVICE_ANALOG (5)
    + RETRO_DEVICE_INDEX_ANALOG_LEFT/RIGHT/BUTTON + RETRO_DEVICE_ID_ANALOG_X/Y.
  - `oa-libretro::state::State` — new `input_axes: [[i16; 4]; 5]`
    field (5 ports × 4 axes each: Left X, Left Y, Right X, Right Y).
  - `oa-libretro::state::cb_input_state` — extended to dispatch
    RETRO_DEVICE_ANALOG queries to the stored axes per port/index/id.
  - `oa-libretro::core::set_input` — stores `input.axes` alongside
    `input.buttons` on each frame.
  - `oa-input::InputPoller::poll` — extended to sample gilrs
    `Axis::LeftStickX/Y` + `Axis::RightStickX/Y`, scale to i16
    libretro range, populate `InputState.axes`. Y axes inverted
    (gilrs +Y up, libretro +Y down per convention).
  - The emu thread's `set_input` call site at main.rs:3990 already
    passed `polled.axes` through — no main.rs changes needed.
  - Net result: gamepad analog stick → libretro RETRO_DEVICE_ANALOG
    end-to-end, identity routing.
- **Shipped (Rust core):** Two new `oa_core::SystemId` variants
  (`N64`, `GameCube`). Single `gamecube` slug covers both Nintendo
  GameCube + Wii via Dolphin's auto-detect (operator chose pair-not-
  triple during onboarding).
- **Shipped (bindings):** Two new modules.
  - `n64` — 14 digital buttons (d-pad + A/B + L/R/Z + START + 4
    C-buttons). Main analog stick is NOT in the bit-table; flows
    via `InputState.axes[0..2]` (gamepad LeftStick).
  - `gamecube` — 12 digital buttons (d-pad + A/B/X/Y + L/R + Z +
    START). Both main stick AND C-stick flow via analog axes
    (LeftStick → main, RightStick → C-stick). No digital C-stick
    fallback at Phase 0 — Phase 2.5 polish adds per-axis keyboard
    binding.
  Six new tests lock the dispatch.
- **Shipped (default cores):** n64 → `mupen64plus_next_libretro.dll`
  (Mupen64Plus-Next with GLideN64 video plugin). gamecube →
  `dolphin_libretro.dll`. Both BIOS-free.
- **Shipped (media + rom_hashes):** New repo arms
  (`Nintendo_-_Nintendo_64`, `Nintendo_-_GameCube`). rom_hashes: n64
  → no-intro N64 dat (matches `.z64` directly; `.n64`/`.v64` need
  byte-swap pass in Phase 2); gamecube → `&[]` with NO_DAT_SYSTEMS
  (large multi-format disc images aren't single-file SHA-1 matched).
- **Shipped (frontend):** SystemId union extended. Two new
  `systemThemes` entries (n64 `.n64`/`.z64`/`.v64`; gamecube
  `.iso`/`.gcm`/`.gcz`/`.rvz`/`.wbfs` union covering GC + Wii). Two
  new CSS blocks (Plan A — Nintendo home cluster):
  - **n64:** Atomic Purple `oklch(0.55 0.22 268)` — period-correct
    to the iconic 1998 Atomic Purple transparent-shell N64; slots
    between Intv 260° and SNES 270° in the violet cluster.
  - **gamecube:** Indigo `oklch(0.48 0.22 280)` — period-correct to
    the 2001 Indigo GameCube launch shell; slots between Saturn 275°
    and GBA 285°.
  - Forms a 4-system Nintendo home-console cluster (SNES 270° / n64
    268° / gamecube 280° / GBA 285°) — operator accepted the
    cluster crowding for visual coherence.
- **Shipped (docs):** ACTIVE_CORE → `gamecube`. Two full per-core
  docs sets at `docs/cores/{n64,gamecube}/` (10 doc files).
  Decisions captured: Mupen64Plus-Next / Dolphin defaults, single-
  slug-covers-GC+Wii rationale, analog-infra-shipped-as-part-of-N64
  reasoning, Atomic-Purple / Indigo theme placements, GC C-stick is
  analog-only (no digital fallback at Phase 0).
- **Plan:** Flipped 2 rows ⬜ → ✅ in system-wiring-plan; bumped
  "Already wired" count from 30 to 32. **Wave 3 (Nintendo home
  post-SNES) COMPLETE.** Order-of-attack reduced to 5 groups
  remaining (dreamcast / psp+ps2+nds / scummvm+dosbox / 5200 /
  pokemini).
- **Validation:** `cargo test -p oa-shell --bin oa-shell` 257/257
  green (was 251; +6 tests across 2 systems × 3 each).
  `cargo check` on the workspace clean (analog trait changes
  ripple through oa-core / oa-libretro / oa-input cleanly).
  Frontend `npm run typecheck` silent.
- **Almost:** Phase 1 operator validation. n64: Mupen64Plus-Next .dll
  + gamepad → Super Mario 64 (LeftStick should drive Mario through
  Peach's Castle). gamecube: Dolphin .dll + gamepad → Super Smash
  Bros. Melee (LeftStick + RightStick test). Wii .wbfs files load
  via the same Dolphin .dll but motion-controls are unbound (Phase
  2.5).
- **Next:** 32 systems shipped (~94% of original 34-plan; 80% if
  counting the 8 scope-expanded heavyweight remaining). Order-of-
  attack next pick is **`dreamcast`** (Sega family completion;
  heavyweight CD-shape, BIOS-required, slots into the now-8th CD
  system in the launch-time BIOS dispatch arm).

---

## 2026-05-20 — Overlooked-console thesis triple-pair (systems #28-30: jaguar + 3do + pcfx)

Fourth paired pass of the day. The "overlooked consoles" Overlooked
Arcade was named for: Atari Jaguar (Atari Corp's last console), 3DO
Multiplayer (5th-gen pioneer that didn't survive), NEC PC-FX (Japan-
only anime-VN platform). Two new CD-shape systems slot into the
table-shaped BIOS dispatch arm (3DO + PCFX now alongside pce-cd /
segacd / saturn / psx / neocd); Jaguar is cart-shape with the largest
controller bindings module in OA's lineup (21 buttons including full
12-key numpad).

- **Shipped (Rust core):** Three new `oa_core::SystemId` variants
  (`Jaguar`, `ThreeDo`, `PcFx`) — variant names spell out the digit
  in `ThreeDo` since Rust identifiers can't start with one.
- **Shipped (bindings):** Three new modules with **largest variation
  in OA's lineup**:
  - `jaguar` — **21-button** (d-pad + A/B/C + OPTION + PAUSE + 12-key
    numpad). KP1-KP7 map to spare RetroPad bits (libretro X / L / R /
    L2 / R2 / L3 / R3); **KP8 / KP9 / KP_STAR / KP0 / KP_HASH live in
    shell-reserved high bits (1<<16 through 1<<20)** — surfaced in the
    per-system Bindings page for keyboard binding but require Phase 2
    keyboard-passthrough dispatch to reach Virtual Jaguar. `jaguar_to_libretro_bits`
    masks high bits off before reaching the core. Operator overrode
    the recommended Phase 0 8-button option in favor of full numpad
    coverage (Iron Soldier weapon select / AvP inventory / Cybermorph
    radar all lean on the keypad heavily).
  - `threedo` — 11-button (d-pad + A/B/C + L/R shoulders + STOP/PLAY +
    START). No SELECT — the 3DO standard controller doesn't have one.
  - `pcfx` — 12-button (d-pad + I-VI + RUN + SELECT). **Separate from
    the existing `pce::*` module** which is 2-button only — sharing
    would force tg16/pce-cd to either acknowledge the 6-button extras
    incorrectly or skip them, wasting bits. Each system's defaults
    stay clean.
  Nine new tests lock the dispatch (3 per system) including a Jaguar
  high-bit-masking test (`jaguar_remap_drops_high_bits`).
- **Shipped (default cores):** jaguar → `virtualjaguar_libretro.dll`,
  3do → `opera_libretro.dll` (Opera, formerly 4DO), pcfx →
  `mednafen_pcfx_libretro.dll` (Beetle PC-FX, Mednafen lineage with
  pce-cd / saturn / psx / vb / ws / lynx / ngp).
- **Shipped (BIOS pre-checks):** `check_3do_bios` +
  `THREEDO_BIOS_KNOWN_HASHES` (4 canonical regional/manufacturer
  entries: Panasonic FZ-1, FZ-10, GoldStar GDO-101M, Sanyo Try IMP-21J).
  `check_pcfx_bios` + `PCFX_BIOS_KNOWN_HASHES` (1 entry — single
  canonical `pcfx.rom`; PC-FX was Japan-only with no regional
  variants). Both slot into the CD-launch BIOS dispatch arm.
  **CD-launch dispatch now covers 7 CD-shape systems** (pce-cd /
  segacd / saturn / psx / neocd / 3do / pcfx) — table-shaped pattern
  validated; future CD systems just add a single match arm.
  Jaguar is BIOS-optional (`jagboot.rom` enables boot logo only) and
  doesn't get a pre-check at Phase 0.
- **Shipped (media + rom_hashes):** Three new repo arms
  (`Atari_-_Jaguar`, `The_3DO_Company_-_3DO`, `NEC_-_PC-FX`).
  rom_hashes: jaguar → no-intro Atari Jaguar dat; 3do + pcfx → `&[]`
  with NO_DAT_SYSTEMS entries. ONBOARDED_SYSTEMS fixtures bumped.
- **Shipped (frontend):** SystemId union extended. Three new
  `systemThemes` entries (jaguar `.j64`/`.jag`, 3do CD container set,
  pcfx CD container set). Three new CSS blocks (Plan A theme locked):
  - **jaguar:** saturated gold `oklch(0.65 0.22 65)` — open 65-75°
    band between 2600 wood-brown (60°) and A7800 gold (80°). Three
    Atari-era systems now share the warm zone with distinct
    lightness ladder: 2600 muted (L=0.60), Jaguar mid (L=0.65), A7800
    bright (L=0.78). Period-correct to JAGUAR logotype + jaguar-cat-fur.
  - **3do:** deep purple-magenta `oklch(0.55 0.22 297)` — tight
    Lynx 290° / WS 305° gap. Period-correct to 3DO swirl logo.
  - **pcfx:** saturated anime pink-magenta `oklch(0.62 0.24 320)` —
    tight WS 305° / O2 325° gap. Period-correct to PC-FX's
    anime-VN-platform identity (the marketing palette leaned heavily
    into vivid pinks).
- **Shipped (docs):** ACTIVE_CORE → `jaguar`. Three full per-core
  docs sets at `docs/cores/{jaguar,3do,pcfx}/` (15 doc files).
  Decisions captured: Virtual Jaguar / Opera / Beetle PCFX defaults,
  jaguar 21-button rationale + high-bit keypad Phase 2 path, 3DO 4-entry
  BIOS scope, PCFX single-canonical-BIOS scope, separate pcfx module
  (no pce::* sharing), theme placements.
- **Plan:** Flipped 3 rows ⬜ → ✅ in system-wiring-plan; bumped
  "Already wired" count from 27 to 30. Wave 6 (Other consoles)
  reduced to 5200 + pokemini. Order-of-attack reduced to 6 groups
  remaining.
- **Validation:** `cargo test -p oa-shell --bin oa-shell` 251/251
  green (was 242; +9 tests across 3 systems × 3 each, including
  Jaguar's high-bit-masking lock).
  Frontend `npm run typecheck` silent.
- **Almost:** Phase 1 operator validation. jaguar: Iron Soldier /
  Tempest 2000 / Rayman / AvP / Doom Jaguar. 3do: Star Control II /
  Road Rash / Need for Speed / Lemmings 3DO. pcfx: Battle Heat /
  Tyoushin Heiki Zeroigar.
- **Next:** 30 systems shipped (~88% of original 34-plan; 75% if
  counting scope-expanded remaining). Order-of-attack next pick is
  **`n64` + `gamecube`** (Nintendo home heavyweights — mupen64plus_next
  for N64, dolphin for GC; both cart-shape but GPU-intensive).

---

## 2026-05-20 — SNK family triple-pair (systems #25-27: neogeo + neocd + ngp)

Third paired pass of the day. **First content-peek classifier in the
scanner** — Neo Geo `.zip` ROM-sets disambiguated against MAME zips via
the `.p1+.s1` file signature. Two BIOS pre-checks added (existence-only
for cart neogeo.zip; SHA-1-based for neocd_z/t.rom slotting into the
CD-launch dispatch arm).

- **Shipped (Rust core):** Three new `oa_core::SystemId` variants
  (`NeoGeo`, `NeoGeoCd`, `NeoGeoPocket`). parse_system_id arms with
  aliases (aes, mvs, ngpc, etc.).
- **Shipped (bindings):** Two new modules. `bindings::neogeo` ships
  the **10-button arcade pad** (A/B/C/D + START + COIN + d-pad,
  COIN on Key5 matching MAME convention). neocd shares the controller
  via `"neogeo" | "neocd" => ...` dispatch arms (same precedent
  PCE-CD/TG-16 set, segacd/genesis set, sega32x/genesis set).
  `bindings::ngp` ships the **7-button handheld** (d-pad + A + B +
  OPTION) — simplest controller since channelf. Nine new tests lock
  the dispatch (3 per system).
- **Shipped (default cores):** neogeo → `fbneo_libretro.dll` (FBNeo).
  neocd → `neocd_libretro.dll` (NeoCD Redux). ngp →
  `mednafen_ngp_libretro.dll` (Beetle NeoPop, Mednafen lineage with
  the other Beetle cores).
- **Shipped (BIOS pre-checks):** Two new check functions in main.rs.
  `check_neogeo_bios` is **existence-only at Phase 0** — Neo Geo's
  multi-ROM `neogeo.zip` BIOS has too many legitimate variants
  (Universe BIOS / Unibios, MAME-revision-specific zips) to lock down
  a SHA-1 list. Phase 2 polish upgrades to zip-content peek. Cart
  pre-check fires after the CD-launch BIOS dispatch when system_id
  == "neogeo". `check_neocd_bios` + `NEOCD_BIOS_KNOWN_HASHES` (3
  entries: CDZ top-loader, CD front-loader, front-loader alternate
  naming) slots into the CD-launch dispatch arm next to pce-cd/segacd/
  saturn/psx. ngp is BIOS-free (Beetle NeoPop synthesizes firmware).
- **Shipped (content-peek scanner):** New `archive::peek_zip_for_neogeo`
  function — scans a `.zip` archive for files matching the Neo Geo
  ROM-set signature (`*.p1` AND `*.s1` extensions present). The
  scanner's archive branch now runs this check first for `.zip`
  files; matching zips emit a single ScannedRom for the whole zip
  with `system_hint = "neogeo"`, MAME zips fall through to the
  normal inner-file enumeration path. `ScannedRom` struct gained
  optional `system_hint` field; frontend ingest paths (both
  `ingestFolderPath` and `rescanFolders`) prefer the hint over
  extension-based mapping when present. **First per-file content
  classifier in OA's library scanner** — future systems with
  similar disambiguation needs (CPS-1/2/3 arcade .zip, etc.) can
  extend the `peek_zip_for_*` family.
- **Shipped (media + rom_hashes):** Three new repo arms (`SNK_-_Neo_Geo`,
  `SNK_-_Neo_Geo_CD`, `SNK_-_Neo_Geo_Pocket_Color`). rom_hashes:
  neogeo → no-intro SNK Neo Geo dat (matches .neo single-file dumps;
  .zip ROM-set hash matching is Phase 2). neocd → `&[]` with
  NO_DAT_SYSTEMS entry. ngp → two no-intro dats merged (NGP mono +
  NGPC color, same gb/wonderswan pattern). ONBOARDED_SYSTEMS fixtures
  bumped.
- **Shipped (frontend):** SystemId union extended with `neogeo | neocd
  | ngp`. Three `systemThemes` entries — neogeo `["neo", "zip"]` with
  content-peek disambiguation, neocd CD container set, ngp `["ngp",
  "ngc"]` single-slug-two-hardware. Three new CSS blocks (Plan A
  theme locked):
  - **neogeo:** deepest+most-saturated red `oklch(0.50 0.27 18)` —
    cluster bottom of VB 7° / MAME 12° / NES 28°, period-correct to
    SNK arcade marketing.
  - **neocd:** muted SNK gold `oklch(0.55 0.18 50)` — tight gap
    between sega32x 42° and TG-16 55°, distinct via L/C profile.
    Family-cousin to neogeo (red→gold warm zone preserves SNK
    arcade family identity).
  - **ngp:** pearl yellow-green `oklch(0.80 0.12 105)` — open
    95-125° band, evokes NGPC translucent yellow shell. Deliberately
    breaks free from the SNK arcade family to mark handheld as
    outlier (same WonderSwan-pearl-lavender precedent).
- **Shipped (docs):** ACTIVE_CORE → `neogeo`. Three full per-core
  docs sets at `docs/cores/{neogeo,neocd,ngp}/` (15 doc files).
  DECISIONS captured the FBNeo default, AES+MVS single-slug,
  separate-neocd-slug, .zip content-peek pattern, Phase 0
  existence-only BIOS check, deepest-red cluster placement,
  family-cousin theme strategy.
- **Plan:** Flipped 3 rows ⬜ → ✅ in system-wiring-plan; bumped
  "Already wired" count from 24 to 27. Wave 6 (Other consoles)
  reduced from 9 systems to 6 (jaguar/3do/pcfx/5200/pokemini remain).
  Order-of-attack re-numbered to 7 groups remaining.
- **Validation:** `cargo test -p oa-shell --bin oa-shell` 245/245
  green (was 236; +9 tests across 3 systems × 3 each).
  Frontend `npm run typecheck` silent.
- **Almost:** Phase 1 operator validation for all three systems.
  neogeo: drop `fbneo_libretro.dll` + `neogeo.zip` BIOS, scan
  Neo Geo folder, launch Metal Slug / KOF '97 / Samurai Shodown II.
  neocd: drop `neocd_libretro.dll` + `neocd_z.rom`, launch Samurai
  Shodown RPG / Metal Slug CD. ngp: drop `mednafen_ngp_libretro.dll`,
  scan NGP/NGPC ROMs, launch SNK vs Capcom: Card Fighter's Clash /
  Match of the Millennium / Sonic Pocket Adventure.
- **Next:** 27 systems shipped (~79% of plan). Order-of-attack next
  pick is **`jaguar` + `3do` + `pcfx`** (overlooked-console thesis
  lineup — Jaguar is cart-shape BIOS-optional, 3DO is CD-shape
  BIOS-required slotting into the CD-launch dispatch, PCFX is the
  PC Engine's CD successor sharing Beetle PCE family lineage).

---

## 2026-05-20 — Heavyweight CD-shape pair (systems #23-24: saturn + psx)

First heavyweight CD-shape onboarding pair post-segacd. Reused the
BIOS-pre-check dispatch pattern check_sega_cd_bios just generalized.
Each system adds a fresh ~13-button bindings module (Saturn 6-button
face pad vs PSX digital DualPad — distinct layouts, no shared MD-style
plumbing).

- **Shipped (Rust core):** Two new `oa_core::SystemId` variants
  (`Saturn`, `Playstation`). `parse_system_id` arms accepting regional
  aliases (saturn: `saturn | sat | ss | sega-saturn`; psx: `psx | ps1 |
  ps | playstation`).
- **Shipped (bindings):** Two new modules. `bindings::saturn` ships
  the **13-button Saturn 6-button face pad** (A/B/C bottom + X/Y/Z top
  + L/R shoulders + START + d-pad). Saturn C and Z legitimately live
  in libretro L2/R2 slots since the Xbox-diamond only holds 4 face
  buttons — that's Beetle Saturn's libretro mapping convention.
  `bindings::psx` ships the **14-button digital DualPad** (d-pad +
  Triangle/Circle/Cross/Square + L1/R1/L2/R2 + START + SELECT).
  DualShock analog sticks + L3/R3 deferred to Phase 2 alongside
  shared analog-input infra (same gate as Saturn 3D Pad, Virtual Boy
  right D-pad, Intv 16-way disc). Six new tests lock the dispatch
  (3 per system).
- **Shipped (default cores):** saturn →
  `mednafen_saturn_libretro.dll` (Beetle Saturn). psx →
  `mednafen_psx_hw_libretro.dll` (Beetle PSX HW). **First "catalog
  peer" pattern:** Beetle PSX SW (`mednafen_psx_libretro.dll`)
  pre-registered as a recommended alternate so operators hitting
  Vulkan/OpenGL-from-DX12-handoff issues can swap to software-renderer
  without manual .dll install. Documented in psx/DECISIONS.md.
- **Shipped (BIOS pre-checks):** Two new functions + tables in main.rs.
  `check_saturn_bios` + `SATURN_BIOS_KNOWN_HASHES` (5 canonical entries:
  JP v1.00 + v1.01, US/EU v1.00, EU PAL v1.01, generic saturn_bios.bin
  alias). `check_psx_bios` + `PSX_BIOS_KNOWN_HASHES` (6 canonical
  entries: JP/US/EU v3.0, US v4.1, US v4.4, US v2.2/PSone alias).
  The CD-launch path's BIOS dispatch arm extended with `saturn` and
  `psx` branches — the dispatch is now table-shaped and easy to
  extend (dreamcast / 3do / pcfx / neocd land in the same `match`).
- **Shipped (CD container handling):** `.pbp` extension added to
  `is_cd_extension` — the PSP-format PS1 EBOOT container needs the
  BIOS pre-check + path-based loading like other CD images. PSX-unique
  (no collision with other CD-shape systems). Standard CD container
  set (`.cue / .chd / .iso / .m3u / .ccd / .toc`) now claimed by 4
  systems (PCE-CD / segacd / saturn / psx); same per-folder Import
  Wizard disambiguation pattern.
- **Shipped (media + rom_hashes):** New repo arms — saturn →
  `Sega_-_Saturn`, psx → `Sony_-_PlayStation`. Both go to
  NO_DAT_SYSTEMS at Phase 0 (CD images aren't single-file SHA-1
  matched; disc-id extraction deferred to Phase 2). Both onboarded-
  systems test fixtures bumped.
- **Shipped (frontend):** `SystemId` union extended with `saturn | psx`.
  Two `systemThemes` entries — saturn uses landscape 4/3 + `crt-lite`,
  psx uses landscape 4/3 + `crt-lite` + extra `.pbp` extension. Two
  new `[data-system=…]` CSS blocks (Plan A theme locked):
  - **Saturn:** deepest purple `oklch(0.45 0.18 275)` — bottom of the
    SNES/Lynx/GBA violet cluster via lightness axis (L=0.45 vs SNES
    L=0.62, Lynx L=0.65, GBA L=0.55). Period-accurate to 1994-1996
    Saturn launch marketing palette. Operator accepted the cluster
    crowding.
  - **PSX:** teal cyan `oklch(0.65 0.16 180)` — open 175-185° band,
    no hue crowding. Evokes PS1 launch palette's cool blue/cyan/silver
    identity.
- **Shipped (docs):** ACTIVE_CORE → `saturn`. Two full per-core docs
  sets at `docs/cores/{saturn,psx}/` (10 doc files). Decisions
  captured the per-system nuances: saturn's Mednafen-default rationale
  + 6-button-face-with-trigger-spillover bindings + 3D Pad analog
  deferral; psx's HW/SW catalog peer + Z=Cross primary breaking PSX
  physical layout for cross-system consistency + DualShock deferral.
- **Plan:** Flipped 2 rows ⬜ → ✅ in system-wiring-plan; bumped
  "Already wired" count from 22 to 24. Wave 2 (Sega family) reduced
  to 1 system (Dreamcast); Wave 5 (Sony) reduced to 2 (psp + ps2).
  Order-of-attack re-numbered.
- **Validation:** `cargo test -p oa-shell --bin oa-shell` 236/236
  green (was 230; +6 tests across 2 systems × 3 each).
  Frontend `npm run typecheck` silent.
- **Almost:** Phase 1 operator validation. **Saturn** needs a real CD
  image + matching regional BIOS — drop
  `mednafen_saturn_libretro.dll` into `<exe_dir>/cores/` + one of
  `sega_100.bin / sega_101.bin / mpr-17933.bin / mpr-19367b.bin` into
  `<exe_dir>/system/`, mark a Saturn folder via Import Wizard,
  confirm deepest-purple-themed tiles, launch a known-good disc.
  Suggested: NiGHTS into Dreams, Guardian Heroes, Radiant Silvergun,
  Saturn Bomberman. **PSX** needs the same — drop
  `mednafen_psx_hw_libretro.dll` + a regional `scph*.bin`. Watch for
  Vulkan/OpenGL surface obtainment from wgpu DX12 host; if HW fails,
  swap to Beetle PSX SW via per-system Cores. Suggested: Castlevania:
  Symphony of the Night, Final Fantasy VII (3-disc .m3u test), Metal
  Gear Solid, Crash Bandicoot.
- **Next:** Pace is 24 systems shipped. Order-of-attack next pick is
  `neogeo + neocd + ngp` (SNK family — neogeo cart-shape leans on the
  MAME-style .zip ROM-set handling, neocd is CD-shape, ngp is a small
  handheld). 10 cores remaining on the plan; over 70% complete.

---

## 2026-05-20 — Sega family CD/32X pass (systems #21-22: segacd + sega32x)

Two-system paired pass. **First post-PCE-CD CD-shape onboarding** —
Sega CD adds BIOS pre-check + CD container disambiguation on top of
the console-shape recipe. 32X is cart-shape but routes through the
genesis 6-button MD controller. Both share the same dispatch arms in
`bindings.rs` (`"genesis" | "segacd" | "sega32x" => ...`), mirroring
the TG-16 / PCE-CD precedent.

- **Shipped (Rust core):** Two new `oa_core::SystemId` variants
  (`SegaCd`, `Sega32X`), each with parse_system_id arms accepting
  multiple regional aliases (segacd: `segacd | sega-cd | mega-cd |
  megacd | mcd`; sega32x: `sega32x | 32x | sega-32x`).
- **Shipped (bindings):** Shared 6-button MD controller dispatch.
  No new modules / no new BUTTONS tables — segacd + sega32x reuse
  `GENESIS_BUTTONS` + `default_genesis_bindings()` + `genesis_to_libretro_bits`
  via the dispatch arms in all four locations (`bit_for` /
  `buttons_for` / `to_libretro_bits` / `defaults_for`). Same pattern
  PCE-CD uses to share TG-16's controller. Six new tests lock the
  dispatch (3 per system: defaults_cover, remap_identity,
  dispatch_round_trips). Test fixtures bumped to include both slugs.
- **Shipped (default cores):** segacd →
  `genesis_plus_gx_libretro.dll` (same .dll already shipping for
  SMS + Game Gear — install-once value: one .dll, four Sega systems).
  sega32x → `picodrive_libretro.dll` (the only mainstream libretro
  core with 32X support; no practical alternate).
- **Shipped (BIOS pre-check):** New `check_sega_cd_bios` function +
  `SEGA_CD_BIOS_KNOWN_HASHES` table in main.rs (6 canonical entries
  spanning US Model 1 v1.10 / Model 2 v2.00 / Model 2 v2.00w, JP
  Mega-CD v1.00p / v1.00s, EU Mega-CD v1.00). Refactored the CD-launch
  path's BIOS dispatch — was hardcoded to `check_pce_cd_bios`; now
  matches on `system_id` and routes pce-cd → check_pce_cd_bios,
  segacd → check_sega_cd_bios. Other CD-shape systems (saturn /
  dreamcast / psx / 3do / pcfx / neocd) drop in here as they onboard.
  sega32x is BIOS-free (PicoDrive synthesizes the SH-2 boot vector).
- **Shipped (media + rom_hashes):** New
  `media::repo_for_system_id` arms — segacd →
  `Sega_-_Mega-CD_-_Sega_CD`, sega32x → `Sega_-_32X`. New
  `rom_hashes::libretro_dat_refs_for_system` arms — segacd → `&[]`
  with NO_DAT_SYSTEMS entry (CD images aren't single-file SHA-1
  matched; disc-id extraction via cd_id.rs Sega CD branch is Phase 2),
  sega32x → metadat/no-intro/Sega - 32X. Both onboarded-systems test
  fixtures bumped.
- **Shipped (frontend):** `SystemId` union extended with `segacd |
  sega32x`. Two `systemThemes` entries — segacd uses landscape 4/3
  + `plain` shader (FMV-heavy library), sega32x uses landscape 4/3 +
  `crt-lite` (period-correct CRT). Two new `[data-system=…]` CSS
  blocks:
  - **Sega CD:** sapphire blue 235°/L=0.55/C=0.20 — family-cousin to
    Genesis cobalt (245°) but visually distinct via lightness axis.
    Forms the Sega family cluster (PCE-CD 220° / segacd 235° /
    Genesis 245°) where each system holds a distinct L/C profile.
  - **Sega 32X:** neon orange 42°/L=0.68/C=0.22 — period-accurate to
    the 1994 32X marketing palette. Lands in the previously-empty
    35-50° hue band (13° from TG-16 55°; chroma + lightness separate
    them in mixed library). Deliberate departure from the Sega family
    cobalt cluster — 32X branding was orange-not-blue in the era.
- **Shipped (CD extension disambiguation):** segacd registers
  `.cue / .chd / .iso / .m3u / .ccd / .toc` — same set PCE-CD claims.
  Disambiguation at scan time via per-folder hint in the Import
  Wizard (same path PCE-CD navigated). Disc-id extraction via
  `cd_id.rs` deferred to Phase 2 polish. Documented in segacd
  DECISIONS.md.
- **Shipped (docs):** ACTIVE_CORE switched from `vectrex` to
  `segacd`. Two full per-core docs sets at
  `docs/cores/{segacd,sega32x}/` (10 doc files). Decisions captured
  the per-system nuances: segacd's Genesis Plus GX install-once
  rationale, the 6-canonical-BIOS table breadth choice, CD extension
  disambiguation via Import Wizard, disc-id deferral; sega32x's
  PicoDrive-as-only-option, slug-separation-forces-right-core
  rationale, .32x-only extension scope, BIOS-free cart-only path
  with 32X-CD games queued for Phase 3+ via stacked segacd override.
- **Plan:** Flipped 2 rows ⬜ → ✅ in system-wiring-plan; bumped
  "Already wired" count from 20 to 22. Wave 2 (Sega family completion)
  reduced from 3 systems to 2 (saturn + dreamcast remain).
  Order-of-attack re-numbered (8 groups remaining; was 9).
- **Validation:** `cargo test -p oa-shell --bin oa-shell` 230/230
  green (was 224; +6 tests across 2 systems × 3 each).
  Frontend `npm run typecheck` silent.
- **Almost:** Phase 1 operator validation. **Sega CD** needs a real
  CD image + matching regional BIOS — drop
  `genesis_plus_gx_libretro.dll` into `<exe_dir>/cores/` + one of
  `bios_CD_U.bin / bios_CD_J.bin / bios_CD_E.bin` into
  `<exe_dir>/system/`, mark a Sega CD folder via Import Wizard
  (disambiguates against PCE-CD's claim on the same extensions),
  confirm sapphire-themed tiles appear, launch a known-good disc.
  Suggested: Sonic CD (US v2.00), Lunar: The Silver Star Complete,
  Snatcher, Popful Mail. **Sega 32X** needs a real `.32x` cart
  launched — drop `picodrive_libretro.dll` into `<exe_dir>/cores/`,
  scan, confirm neon-orange tiles. Suggested: Knuckles' Chaotix,
  Virtua Racing Deluxe, Doom 32X, Star Wars Arcade, Kolibri.
- **Next:** Pace is now 22 systems shipped (was 20). Order-of-attack
  next pick is `saturn` + `psx` (heavyweight CD-shaped; both need
  BIOS validation passes — Sega CD's `check_sega_cd_bios` is the
  precedent the table dispatch generalizes from). 14 cores remaining
  on the plan; over 65% complete.

---

## 2026-05-20 — VISION first-wave remainder pass (systems #18-20: vectrex + virtualboy + wonderswan)

Three systems in one pass. Completes the VISION-document first-wave
lineup — the original "overlooked consoles" Overlooked Arcade was
designed around. All three were pre-registered Phase 0 placeholders
(SystemIds existed; parse_system_id arms wired; media repo arms
wired) — needed default_core_dll arms, bindings modules, rom_hashes,
frontend, and docs.

- **Shipped (Rust):** Three new `bindings.rs` modules:
  - `vectrex` — 8-button (D-pad + 4 face buttons B1/B2/B3/B4 in
    horizontal row).
  - `virtualboy` — 10-button (LEFT D-pad + A + B + L + R + START +
    SELECT). The unique RIGHT D-pad deferred to Phase 2 (Beetle VB
    routes it through libretro right analog stick by default; needs
    both core-option config + shared analog-input infra).
  - `wonderswan` — 7-button (D-pad + A + B + START). Single D-pad
    because Beetle WS handles the dual-physical-D-pad rotation
    (X-pad ↔ Y-pad) per-game-header automatically.
  All identity remaps, dispatch arms updated. Three new
  `default_core_dll_for_system` arms (vecx, mednafen_vb, mednafen_wswan).
- **Shipped (media + rom_hashes):** All three media repo arms were
  pre-wired from the Phase 0 placeholder seed. Three new rom_hashes
  DatRef arms — note WonderSwan uses TWO refs (Bandai - WonderSwan
  + Bandai - WonderSwan Color) merged into one corpus, same shape as
  `gb` covering DMG + CGB. Both onboarded-systems test fixtures
  bumped.
- **Shipped (frontend):** `SystemId` union extended with `vectrex |
  virtualboy | wonderswan`. Three `systemThemes` entries with
  notable per-system shader picks:
  - Vectrex: `crt-lite` as temporary compromise; dedicated
    `vector-phosphor` shader is Phase 2 polish.
  - **Virtual Boy: `plain` shader — first OA system to explicitly
    opt OUT of `crt-lite`.** The VB's monochrome-red LED display
    had no scanlines / no CRT artifacts; CRT shading would muddy
    the crisp red-on-black aesthetic. Documented as the precedent
    for future LED/VFD systems.
  - WonderSwan: `crt-lite` per the cross-handheld convention.
  Three new `[data-system=…]` CSS blocks:
  - Vectrex: bright phosphor green 165°/L=0.80/C=0.16 (highest
    lightness in the lineup — reads as "luminescent vector beam").
  - Virtual Boy: deep VB red 7°/L=0.55/C=0.26 (highest chroma red in
    the lineup; lightness-axis differentiates from MAME 12°/L=0.64
    and NES 28°/L=0.62).
  - WonderSwan: pearl lavender 305°/L=0.70/C=0.14 (open 295-320°
    range; sherbet/pearl shell vibe).
- **Shipped (docs):** `docs/ACTIVE_CORE.md` switched from `coleco` to
  `vectrex`. Three full per-core docs sets at
  `docs/cores/{vectrex,virtualboy,wonderswan}/` (15 doc files).
  Decisions captured the per-system nuances: Vectrex's deferred
  vector-phosphor shader + overlay rendering, VB's `plain` shader
  precedent + right-D-pad Phase 2 path, WS's single-slug-multi-
  hardware pattern + core-managed rotation + multi-repo cover
  follow-up (same gap as `gb` ↔ GBC).
- **Plan:** Flipped 3 rows ⬜ → ✅; bumped "Already wired" count from
  17 to 20. **Wave 1 (VISION first-wave remainder) now COMPLETE —
  all six originally-flagged systems shipped.** Order-of-attack
  re-numbered (9 groups remaining, was 10).
- **Validation:** `cargo test -p oa-shell --bin oa-shell` 224/224
  green (was 215; +9 tests across 3 systems × 3 each).
  Frontend `npm run typecheck` silent.
- **Almost:** Phase 1 operator validation across all three. Per
  system: drop the .dll into `<exe_dir>/cores/` (+ optional Vectrex
  BIOS for Mine Storm pack-in), scan the system folder, confirm
  themed tiles, launch a known-good ROM. Suggested test reels:
  Vectrex — Mine Storm + Berzerk + Star Trek + Spike. VB — Mario's
  Tennis + V-Tetris + Galactic Pinball (single-D-pad titles). WS —
  Klonoa: Moonlight Museum + Final Fantasy I (color) + GunPey.
- **Next:** Pace is now 12 systems shipped in 2 days — over half
  the original 34-system plan. Order-of-attack next pick is
  `segacd` + `sega32x` (Sega family completion; first CD-shape
  onboarding pair since the early-2026 TG-CD work, will need BIOS
  pre-check pass for Sega CD).

---

## 2026-05-19 — Obscure 70s/80s consoles pass (systems #14-17: coleco + intv + o2 + channelf)

Big batched onboarding session — four systems in one pass, matching the
order-of-attack's "obscure 70s/80s consoles" group. All four are
shape-similar (small TV consoles from the 1976-1983 era, small ROM
libraries, simple controllers in modern terms) but each has its own
quirks. Followed the standard 6-step recipe + oa-core variants × 4,
batched by step rather than per-system to keep moving.

- **Shipped (Rust, oa-core):** Three new SystemId variants —
  `Intellivision`, `Odyssey2`, `ChannelF`. Colecovision was already
  registered from the Phase 0 placeholder seed.
- **Shipped (Rust, main.rs):** Four `parse_system_id` arms (coleco
  already existed; added intv/o2/channelf with aliases). Four
  `default_core_dll_for_system` arms — `bluemsx` (coleco), `freeintv`
  (intv), `o2em` (o2), `freechaf` (channelf).
- **Shipped (Rust, bindings.rs):** Four modules with widely-varying
  controller shapes:
  - `coleco` — 16-button (D-pad + 2 fires + 10 keypad numbers KP0-KP9).
    Full keypad coverage in Phase 0 since Coleco launch-era games
    REQUIRE keypad input at game start.
  - `intv` — 10-button (D-pad disc-as-8-way + 4 side action buttons
    LOWER_L/R + UPPER_L/R + START/SELECT for keypad ENTER/CLEAR).
    Full 12-keypad and 16-direction disc analog are Phase 2 polish.
  - `o2` — 5-button (D-pad + single ACTION button). Second
    single-action system after 2600; the 47-key alphanumeric keyboard
    routes through libretro RETRO_DEVICE_KEYBOARD via OA's existing
    keyboard passthrough.
  - `channelf` — 9-button (4-axis plunger as D-pad + FIRE + 4 console
    switches MODE/TIME/START/HOLD with hardware-label keyboard
    bindings).
  All four with identity remaps, dispatch arms, default bindings.
  Documented two NEW single-action exceptions in the
  `z_is_the_primary_action_button_on_every_system` test header:
  o2 (true single-button) + channelf (effectively single-action;
  MODE/TIME/START/HOLD are console switches with hardware-label
  keyboards, not secondary game actions).
- **Shipped (media + rom_hashes):** Four `media::repo_for_system_id`
  arms (Coleco_-_ColecoVision, Mattel_-_Intellivision,
  Magnavox_-_Odyssey2, Fairchild_-_Channel_F). Four
  `rom_hashes::libretro_dat_refs_for_system` arms (one no-intro DatRef
  per system). Both onboarded-systems test fixtures bumped to include
  all four slugs.
- **Shipped (frontend):** `SystemId` union extended with
  `"coleco" | "intv" | "o2" | "channelf"`. Four `systemThemes` entries
  with extension policy: Coleco `["col", "cv"]`, Intv `["int"]`, O2
  `["o2"]` (synthetic — the .bin-dominant O2 community has no
  widely-standardized non-bin extension), Channel F `["chf"]`. All
  portrait 3/4, crt-lite shader. Four new `[data-system=…]` CSS
  blocks with the theme colors operator picked:
  - Coleco bright cyan 195°/L=0.72/C=0.16 (unclaimed teal-cyan range)
  - Intv deep Mattel navy 260°/L=0.50/C=0.17 (period-correct;
    lightness-axis separation from SNES violet + Genesis cobalt)
  - O2 rose-fuchsia 325°/L=0.62/C=0.18 (unclaimed; 15° from SMS magenta)
  - Channel F cedar-brown 25°/L=0.45/C=0.06 — sibling wood-grain
    to 2600's yellow-pine (60°/L=0.60/C=0.07). The two pioneer
    wood-grain consoles (1976 Channel F + 1977 2600 VCS) now read as
    a deliberate family in mixed library tiles.
- **Shipped (docs):** `docs/ACTIVE_CORE.md` switched from `2600` to
  `coleco`. Four full per-core docs sets at `docs/cores/{coleco,intv,o2,channelf}/`
  (README + ROADMAP + SESSION_LOG + KNOWN_GAME_BUGS + DECISIONS — 20
  doc files total). KNOWN_GAME_BUGS pre-populated where the deferred-
  input list is known up-front (Intv 16-direction disc games,
  Intv keypad-required games, O2 keyboard-required games, Channel F
  plunger-precision games). Flipped 4 rows ⬜ → ✅ in the plan;
  bumped "Already wired" count from 13 to 17 (over HALF the
  original 34-system plan now wired in one day).
- **Validation:** `cargo test -p oa-shell --bin oa-shell` 215/215
  green (was 203; +12 tests: 3 per system × 4 systems). Frontend
  `npm run typecheck` silent.
- **Almost:** Phase 1 operator validation across all four. Per
  system: drop the appropriate .dll into `<exe_dir>/cores/`, install
  the BIOS file(s) for systems that need them (`coleco.rom` for
  Coleco, `exec.bin` + `grom.bin` for Intv, `o2rom.bin` for O2,
  none for Channel F), scan the system folder, confirm themed tiles,
  launch a known-good ROM.
- **Next:** With 17/34 systems wired (50%), next pick per the
  order-of-attack is `vectrex` + `virtualboy` + `wonderswan` — the
  VISION first-wave remainder (the systems originally identified in
  `docs/VISION.md` as Overlooked Arcade's launch lineup that haven't
  shipped yet).

---

## 2026-05-19 — Atari 2600 Phase 0 onboarding (system #13)

Followed the standard 6-step recipe + oa-core SystemId variant. First
system in OA's lineup that's legitimately single-button — required
documenting an exception in the cross-system "Z is primary" test
fixture and pinning the Z=FIRE assertion to a per-system test
instead. Also first system in OA where the .bin extension question
was non-trivial enough to need operator decision (resolved: `.a26`
only at the global registry, `.bin` via per-folder rules).

- **Shipped (Rust):** `oa_core::SystemId::Atari2600` variant (Rust
  identifier can't start with a digit; string slug stays `"2600"`).
  `bindings.rs::atari2600` module — 7-button layout (4-way d-pad +
  FIRE + SELECT + RESET), identity libretro remap, `ATARI2600_BUTTONS`
  table, `default_atari2600_bindings()`, all dispatch arms keyed by
  string `"2600"`. `parse_system_id` arm covering `"2600" |
  "atari2600" | "vcs" → SystemId::Atari2600`.
  `default_core_dll_for_system("2600") → "stella_libretro.dll"`.
- **Shipped (media + rom_hashes):** `media::repo_for_system_id("2600")
  → "Atari_-_2600"`. `rom_hashes::libretro_dat_refs_for_system("2600")
  → metadat/no-intro/Atari - 2600`. Both onboarded-systems test
  fixtures bumped.
- **Shipped (frontend):** `SystemId` union extended in `themes/registry.ts`
  with `"2600"`; `systemThemes["2600"]` entry (extensions `["a26"]`
  ONLY — `.bin` deliberately excluded), portrait 3/4 tile aspect,
  `crt-lite` default shader preset. New `[data-system="2600"]` block
  in `themes/systems.css` — muted wood-grain brown at hue 60° / chroma
  0.07. Sits 5° from TG-16 orange (55°) but the chroma 0.07 vs 0.18
  separates them on saturation: TG-16 = bright orange, 2600 = quiet
  warm brown. Period-correct for the original wood-veneer VCS.
- **Shipped (docs):** `docs/ACTIVE_CORE.md` switched from `gba` to
  `2600`. New per-core docs at `docs/cores/2600/` (README + ROADMAP +
  SESSION_LOG + KNOWN_GAME_BUGS + DECISIONS). DECISIONS captures:
  Stella as default + no real alternate, `.a26`-only-with-per-folder-
  rules rationale + full collision analysis, wood-grain hue choice +
  why low-chroma separates it from TG-16, the 7-button single-button
  exception, why Difficulty / Color switches route through Stella's
  core options rather than bindings, paddle-controller deferral list.
  KNOWN_GAME_BUGS pre-populated with the 8 paddle-required titles
  (Breakout, Kaboom!, Warlords, Super Breakout, Night Driver, Indy
  500, Casino, Backgammon) documented as unplayable until shared
  analog-input infra lands. Flipped row ⬜ → ✅ in plan; bumped "Already
  wired" count to 13.
- **Validation:** `cargo test -p oa-shell --bin oa-shell` 203/203
  green (was 200; +3 atari2600 tests: defaults coverage incl. explicit
  Z=FIRE assertion, identity remap, dispatch round-trip). Frontend
  `npm run typecheck` silent.
- **Almost:** Phase 1 operator validation. Operator drops
  `stella_libretro.dll` into `<exe_dir>/cores/` (or configures a
  `*.bin → 2600` per-folder rule if their library is `.bin`-shaped),
  scans an Atari 2600 ROMs folder, confirms wood-grain themed tiles,
  launches a known-good joystick-only ROM (suggested: Adventure,
  Pitfall!, Yars' Revenge, River Raid, Asteroids).
- **Next:** Operator runs `Settings → Library → Identify ROMs` + `Sync
  media for Atari 2600` and confirms canonical title + cover download.
  Once Phase 1 ✅, next pick per the order-of-attack is the obscure
  70s/80s consoles pass — `coleco` + `intv` + `o2` + `channelf` —
  matching the Overlooked Arcade thesis perfectly.

---

## 2026-05-19 — Game Boy Advance Phase 0 onboarding (system #12)

Followed the post-libretro-pivot 6-step recipe + oa-core SystemId
variant. Fourth Phase-0 of the day; Wave 4 (Nintendo handhelds) now
1-of-2 remaining (only `nds` left, which is deferred until the touch-
screen input pass).

- **Shipped (Rust):** `oa_core::SystemId::Gba` variant. `bindings.rs::gba`
  module — 10-button layout (4-way d-pad + A + B + L + R + START +
  SELECT), identity libretro remap, `GBA_BUTTONS` table,
  `default_gba_bindings()`, all dispatch arms. `parse_system_id` arm
  covering `"gba" | "game-boy-advance" | "gameboyadvance" → SystemId::Gba`.
  `default_core_dll_for_system("gba") → "mgba_libretro.dll"`.
- **Shipped (media + rom_hashes):** `media::repo_for_system_id("gba")
  → "Nintendo_-_Game_Boy_Advance"`. `rom_hashes::libretro_dat_refs_for_system("gba")
  → metadat/no-intro/Nintendo - Game Boy Advance`. Both onboarded-systems
  test fixtures bumped.
- **Shipped (frontend):** `SystemId` union extended in `themes/registry.ts`
  with `"gba"`; `systemThemes.gba` entry (extensions `["gba"]`, portrait
  3/4 tile aspect, `crt-lite` default shader preset per the handheld
  convention). New `[data-system="gba"]` block in `themes/systems.css` —
  deep indigo at hue 285°, lightness 0.55, chroma 0.20. Sits between
  SNES violet (270°, L=0.62) and Lynx purple (290°, L=0.65) in hue —
  operator accepted the 15° crowding for period-correctness; the
  lightness axis separates the three purples (GBA = darkest, SNES =
  mid, Lynx = brightest).
- **Shipped (docs):** `docs/ACTIVE_CORE.md` switched from `gb` to
  `gba`. New per-core docs at `docs/cores/gba/` (README + ROADMAP +
  SESSION_LOG + KNOWN_GAME_BUGS + DECISIONS). DECISIONS captures:
  mGBA as default (VBA-Next / VBA-M as alternates), separate slug
  from `gb` rationale, indigo hue + lightness-axis separation
  reasoning, `.gba` only extension, BIOS-optional + Phase-2 BIOS-
  required pre-check plan. Flipped row ⬜ → ✅ in plan; bumped "Already
  wired" count to 12; Wave 4 now 1-of-2 remaining.
- **Validation:** `cargo test -p oa-shell --bin oa-shell` 200/200
  green (was 197; +3 gba tests: defaults coverage, identity remap,
  dispatch round-trip). Frontend `npm run typecheck` silent.
- **Almost:** Phase 1 operator validation. Operator drops
  `mgba_libretro.dll` into `<exe_dir>/cores/`, scans a GBA ROMs folder,
  confirms indigo-themed tiles, launches a known-good ROM (suggested:
  The Minish Cap, Pokémon FireRed / Emerald, Metroid Zero Mission,
  Advance Wars, Castlevania Aria of Sorrow).
- **Next:** Operator runs `Settings → Library → Identify ROMs` + `Sync
  media for Game Boy Advance` and confirms canonical title + cover
  download. Once Phase 1 ✅, next pick per the order-of-attack is
  `2600` (Atari — Stella).

---

## 2026-05-19 — Game Boy / Game Boy Color Phase 0 onboarding (system #11)

Followed the post-libretro-pivot 6-step recipe (plus the oa-core
SystemId variant since GB was a fresh add — no pre-wiring like
SMS+GG had). Single-slug-covers-both-hardware-variants — Gambatte
auto-detects DMG vs CGB from the ROM header at load time.

- **Shipped (Rust):** `oa_core::SystemId::Gb` variant.
  `bindings.rs::gb` module — 8-button NES-shape layout (4-way d-pad +
  A + B + START + SELECT), identity libretro remap, `GB_BUTTONS`
  table, `default_gb_bindings()`, all dispatch arms. `parse_system_id`
  arm covering `"gb" | "gbc" | "gameboy" | "game-boy" | "game-boy-color"
  → SystemId::Gb`. `default_core_dll_for_system("gb") → "gambatte_libretro.dll"`.
- **Shipped (media + rom_hashes):** `media::repo_for_system_id("gb")
  → "Nintendo_-_Game_Boy"` as primary cover repo. GBC-specific cover
  coverage (the parallel `Nintendo_-_Game_Boy_Color` repo) deferred as
  a documented multi-repo follow-up. `rom_hashes::libretro_dat_refs_for_system("gb")`
  returns TWO DatRefs — `metadat/no-intro/Nintendo - Game Boy` AND
  `metadat/no-intro/Nintendo - Game Boy Color` — merged into one local
  corpus by `fetch_and_parse_all`. Both onboarded-systems test fixtures
  bumped.
- **Shipped (frontend):** `SystemId` union extended in `themes/registry.ts`
  with `"gb"`; `systemThemes.gb` entry (extensions `["gb", "gbc"]`,
  portrait 3/4 tile aspect, `crt-lite` default shader preset per the
  handheld convention). New `[data-system="gb"]` block in
  `themes/systems.css` — muted DMG pea-green at hue 145°, chroma 0.13.
  Decisively distinct from Game Gear's bright yellow-green (130°, 0.18)
  by both hue (15° gap) and chroma (0.05 gap).
- **Shipped (docs):** `docs/ACTIVE_CORE.md` switched from `sms` to
  `gb`. New per-core docs at `docs/cores/gb/` (README + ROADMAP +
  SESSION_LOG + KNOWN_GAME_BUGS + DECISIONS). DECISIONS captures:
  Gambatte as default (SameBoy as accuracy-focused override), single
  slug for DMG + CGB, default cover repo choice + multi-repo follow-up,
  pea-green at hue 145° / chroma 0.13 with the cross-system hue map
  reasoning, `.bin` / `.cgb` / `.sgb` exclusions. Flipped row ⬜ → ✅
  in `docs/PLANS/system-wiring-plan.md`; bumped "Already wired" count
  to 11; **fixed last session's "gb + gbc" typo** in the order-of-attack
  list (single `gb` slug, not paired).
- **Validation:** `cargo test -p oa-shell --bin oa-shell` 197/197
  green (was 194; +3 gb tests: defaults coverage, identity remap,
  dispatch round-trip). Frontend `npm run typecheck` silent.
- **Almost:** Phase 1 operator validation. Operator drops
  `gambatte_libretro.dll` into `<exe_dir>/cores/`, scans a GB/GBC ROMs
  folder, confirms pea-green-themed tiles, launches known-good ROMs
  (suggested DMG: Tetris, Super Mario Land, Link's Awakening, Pokémon
  Red; suggested CGB: Pokémon Crystal, Link's Awakening DX, Wario
  Land 3).
- **Next:** Operator runs `Settings → Library → Identify ROMs` + `Sync
  media for Game Boy` and confirms canonical title + DMG cover
  download work. Once Phase 1 ✅, next pick per the order-of-attack
  is `gba` (mGBA).

---

## 2026-05-19 — SMS + Game Gear Phase 0 onboarding (systems #9 + #10, paired)

Operator picked Wave 1 item #1 from `docs/PLANS/system-wiring-plan.md` —
SMS + Game Gear together, single Genesis Plus GX .dll services both.
Followed the post-libretro-pivot 6-step recipe end to end, once per
system. Theme decisions made up-front via AskUserQuestion (SMS neon
magenta 340°, GG yellow-green 130°) so no hue collisions snuck in.

- **Shipped (Rust):** New `bindings.rs::sms` + `bindings.rs::gamegear`
  modules — 7-button SMS layout (4-way d-pad + B1 + B2 + PAUSE) and
  identical-shape GG layout with the operator-facing label "START"
  (matches hardware: SMS Pause sat on the console, GG Start sits on
  the unit). Both identity-mapped to libretro RetroPad bits, both
  follow the cross-system "Z is primary" rule. `SMS_BUTTONS` /
  `GAMEGEAR_BUTTONS` tables, `default_sms_bindings()` /
  `default_gamegear_bindings()`, `sms_to_libretro_bits` /
  `gamegear_to_libretro_bits`, all dispatch arms updated. `parse_system_id`
  already covered both slugs (`"gamegear" | "game-gear" → GameGear`).
  `default_core_dll_for_system("sms" | "gamegear") → "genesis_plus_gx_libretro.dll"`.
- **Shipped (media + rom_hashes):** `repo_for_system_id` arms for sms
  + gamegear were already wired ahead of onboarding; bumped the media
  + rom_hashes onboarded-systems test fixtures to include both slugs.
  Also caught up: `rom_hashes::ONBOARDED_SYSTEMS` was missing `genesis`
  from the prior session — added now alongside the new pair.
  `rom_hashes::libretro_dat_refs_for_system("sms") → metadat/no-intro/Sega - Master System - Mark III`;
  `("gamegear") → metadat/no-intro/Sega - Game Gear`.
- **Shipped (frontend):** `SystemId` union extended in `themes/registry.ts`
  with `"sms"` + `"gamegear"`; two new `systemThemes` entries (extensions
  `["sms"]` / `["gg"]`, landscape 4/3 tiles, `crt-lite` default shader
  preset for both). New `[data-system="sms"]` block — neon magenta at
  hue 340°, chroma 0.22 (period-correct for the SMS Western Big Box
  grid-floor era). New `[data-system="gamegear"]` block — yellow-green
  at hue 130°, chroma 0.18 (GG launch packaging palette). Both
  visually distinct from every prior hue.
- **Shipped (docs):** Per-core docs at `docs/cores/sms/` and
  `docs/cores/gamegear/` (README + ROADMAP + SESSION_LOG +
  KNOWN_GAME_BUGS + DECISIONS — matched the genesis structure). DECISIONS
  capture: Genesis Plus GX as the shared default (install-once value
  with the other Sega slug), 7-button layouts with the "PAUSE" vs "START"
  label diff matching hardware, the hue choices with rationale, and the
  `.bin` exclusion consistent with every other system. `docs/ACTIVE_CORE.md`
  switched from `genesis` to `sms`. Flipped both rows ⬜ → ✅ in
  `docs/PLANS/system-wiring-plan.md`; bumped "Already wired" count to 10
  and renumbered the order-of-attack list (now 14 items, 24 systems
  remaining).
- **Validation:** `cargo test -p oa-shell --bin oa-shell` 194/194
  green (was 188; +6 tests: defaults coverage ×2, identity remap ×2,
  dispatch round-trip ×2). Frontend `npm run typecheck` silent.
- **Almost:** Phase 1 operator validation for both systems. Operator
  drops `genesis_plus_gx_libretro.dll` into `<exe_dir>/cores/` (one
  .dll services both slugs), scans SMS + GG ROMs folders, confirms
  themed tiles appear, launches known-good ROMs (suggested SMS:
  Alex Kidd in Miracle World, Phantasy Star, Wonder Boy III; suggested
  GG: Sonic the Hedgehog (Game Gear), Shinobi, Tails Adventure).
- **Next:** Operator runs `Settings → Library → Identify ROMs` +
  `Sync media for SMS` + `Sync media for Game Gear` and confirms
  canonical title + cover download work. Once Phase 1 ✅, next
  Wave-1 pick per the order-of-attack is `gb` + `gba` (Gambatte + mGBA).

---

## 2026-05-19 — Genesis / Mega Drive Phase 0 onboarding (system #8)

Operator installed ClownMDEmu v1.6.11 (`clownmdemu_libretro.dll`) and
asked to wire it up. Followed the post-libretro-pivot 6-step recipe end
to end (see `feedback_multi_core_architecture_ready` memory) in a single
pass.

- **Shipped (Rust):** `oa_core::SystemId::Genesis` variant. New
  `bindings.rs::genesis` module — 12-button layout (4-way d-pad + 6
  face buttons A/B/C + X/Y/Z + START + MODE) laid out as libretro
  RetroPad bits so the remap is identity. `GENESIS_BUTTONS` table,
  `default_genesis_bindings()`, `genesis_to_libretro_bits`, all
  dispatch arms (`bit_for` / `buttons_for` / `to_libretro_bits` /
  `defaults_for`). `parse_system_id("genesis" | "megadrive" |
  "mega-drive") → SystemId::Genesis`. `default_core_dll_for_system("genesis") → "clownmdemu_libretro.dll"`.
- **Shipped (media + rom_hashes):** `repo_for_system_id("genesis") → Sega_-_Mega_Drive_-_Genesis`
  for libretro-thumbnails cover sync. `rom_hashes::libretro_dat_refs_for_system("genesis") → metadat/no-intro/Sega - Mega Drive - Genesis`
  for hash-based ROM identification. Onboarded systems test list bumped to include genesis.
- **Shipped (frontend):** `SystemId` union extended in `themes/registry.ts`;
  `systemThemes.genesis` entry (extensions `["md", "smd", "gen", "68k"]`,
  landscape 4/3 tile, `crt-lite` default shader preset). New `[data-system="genesis"]`
  block in `themes/systems.css` — cobalt blue at hue 245°, chroma 0.22.
  Deliberately distinct from PCE-CD's cyan-blue (220°, chroma 0.14) so
  a mixed library reads at a glance.
- **Shipped (docs):** `docs/ACTIVE_CORE.md` switched from `tg16` to
  `genesis`. New per-core docs at `docs/cores/genesis/` (README +
  ROADMAP + SESSION_LOG + KNOWN_GAME_BUGS + DECISIONS). DECISIONS
  captures: ClownMDEmu as default (operator pick), 6-button-MD layout
  default with the "B → East per OA console-pad convention" twist,
  cobalt blue at 245°-not-220°, `.md/.smd/.gen/.68k` registered with
  `.bin` excluded.
- **Validation:** `cargo test -p oa-shell --bin oa-shell` 188/188
  green (was 185; +3 genesis tests: defaults coverage, identity remap,
  dispatch round-trip). Frontend `npm run typecheck` silent.
- **Almost:** Phase 1 operator validation. Operator drops
  `clownmdemu_libretro.dll` into `<exe_dir>/cores/`, scans a Genesis
  ROMs folder, confirms cobalt-themed tiles, launches a known-good ROM
  (suggested: Sonic the Hedgehog, Streets of Rage 2, Phantasy Star IV,
  Gunstar Heroes).
- **Next:** Operator runs `Settings → Library → Identify ROMs` + `Sync
  media for Genesis` and confirms canonical title + cover download work.
  Once Phase 1 ✅, Phase 2 polish (3-button vs 6-button game map,
  MD-specific glyphs) opens.

---

## 2026-05-19 — Library infra sweep: hashing, multi-region grouping, CD disc-ID, display/overscan overrides, bezel UI

Cross-cutting library + media + per-system/per-game override work. Atari 7800 also onboarded as system #7 (separate per-core SESSION_LOG entry).

- **Shipped (rom_hashes architecture overhaul):** Header-aware hashing in `apps/oa-shell/src/rom_hashes.rs` — iNES / SMC / LNX / A78 / TG16 headers stripped before SHA-1 so dumps match libretro-database. Path fix from `dat/` to `metadat/no-intro` + `metadat/redump` + `metadat/headered` matches upstream's actual layout. Sync semantics changed from "append" to "wipe-and-replace" so re-syncs don't accumulate stale rows. New `game_serials` table + `games.disc_id` column for CD identification.
- **Shipped (multi-region grouping):** New `apps/oa-shell/src/title_parse.rs` extracts canonical title + region + revision tokens from filenames; `apps/oa-shell/src/library_groups.rs` aggregates rows that share a canonical title into a group. Schema v9→v10 adds `game_group_defaults` table. Per-system + per-game region/revision priority settings (`SystemSettings` + `GameOverrides`) feed a deterministic "preferred member" picker. UI: "Versions ·N" submenu in `TileContextMenu.tsx` + ▼N tile badge in `LibraryTile.tsx` + grouped tile shows the preferred member as primary.
- **Shipped (CD disc-ID extractor):** New `apps/oa-shell/src/cd_id.rs` module — PCE-CD signature extractor + cue + iso + chd format support (chd via the `chd` crate). Archived CDs supported via a new partial-read helper in `apps/oa-shell/src/archive.rs` so disc-ID extraction works without full extraction. Output threads into the new `games.disc_id` column at ingest time.
- **Shipped (display aspect override + overscan crop):** `SystemSettings.display_aspect_override: Option<f32>` + `SystemSettings.overscan_crop_override: Option<OverscanCropPrefs { top, bottom, left, right }>` (`apps/oa-shell/src/system_settings.rs`). Mirrored onto `GameOverrides` in `apps/oa-shell/src/library_db.rs`. Per-game → per-system → core-default resolution feeds the renderer viewport math via the existing Display launch-wiring chain. Closes both the 2026-05-15 PARKING_LOT "Per-system overscan / safe-area / aspect-correction quirks" entry and the tg16 ROADMAP "Pixel aspect ratio" / "Per-system aspect-ratio entry in the system registry" items.
- **Shipped (solid-dnd drag-reorder):** Replaced the ad-hoc HTML5 drag handlers in region priority (library + media), library folders, and the widget customizer with `solid-dnd` for consistent keyboard accessibility + animation. Sidebar systems list audited against `themes/registry.ts` to ensure no orphaned ordering.
- **Shipped (Auto-Identify after every ingest):** Every ingest path (folder pick, archive scan, watcher event, drag-drop fallback) now triggers `identify_roms` automatically against the new wipe-and-replace libretro-database sync — no operator step needed to populate canonical titles + publishers + years.
- **Shipped (cover sync via libretro-thumbnails infra):** `repo_for_system_id` in `apps/oa-shell/src/media.rs` gained explicit mappings for all 7 onboarded systems + the 9 first-wave systems waiting (sms / gamegear / msx / msx2 / coleco / vectrex / virtualboy / wonderswan / wonderswan-color). Per-core ROADMAPs updated from "needs configuration" to "needs operator validation" for lynx / nes / snes / mame / atari7800 / pce-cd.
- **Shipped (Bezel system UI):** File picker for bezel image per-system + per-game; new file-load Tauri command; new `EmuCommand` variant for bezel apply; launch-path wiring resolves per-game → per-system → none. Renderer-side bezel pipeline (Phase 3 slice B-2) already shipped — this slice is the missing UI half.
- **Almost:** Operator validation of cover-sync against real libretro-thumbnails repos for the 7 onboarded systems. Operator validation of A7800 Phase 1 (real `.a78` launch).
- **Next:** Phase 2 next-big-thing pick. Recommended: Curation Layer (VISION Pillar 4) — per-system welcome pages with era context, "recommended starts" lists, per-game curator notes. Alternates: HDR tone mapping (Phase 3 last item), rewind scrubber thumbnail strip, SMS/Game Gear onboarding as system #8, per-core cover-sync validation pass, MAME hardening continuation.

---

## 2026-05-18 — RetroArch parity slice 8: Game Genie / GameShark / Action Replay via libretro

The "raw memory poke" cheat path (slice 5) writes RAM bytes from our side, which doesn't work for codes that target ROM reads or rely on the core's internal cheat machinery (most published cheat codes for retro systems). This slice adds a SECOND cheat kind that pushes the raw user-entered code string through libretro's `retro_cheat_set(index, enabled, code)` callback and lets the CORE decode it per its system's conventions — Game Genie / GameShark / Pro Action Replay / Action Replay / raw `address:value`, whatever each core understands.

- **Shipped (FFI):** `retro_cheat_reset` + `retro_cheat_set` symbol resolution in `loader.rs`. New trait methods on `oa_core::Core`: `cheat_reset()` + `cheat_set(index, enabled, code)` with default no-op impls. `LibretroCore` impls call through; both check `rom_loaded` first to avoid invoking on an empty core.
- **Shipped (schema v6→v7):** Two new columns on the existing `cheats` table — `kind TEXT NOT NULL DEFAULT 'memory_poke'` + `code TEXT` (nullable). Migration is idempotent — `PRAGMA table_info(cheats)` check before each `ALTER TABLE ADD COLUMN` (SQLite doesn't support IF NOT EXISTS here), matches the same pattern slice B's archive_inner_path migration uses. v6 rows survive as `memory_poke` and keep applying via the existing per-frame poke path.
- **Shipped (runtime):** `EmuCommand::LoadCheats` now does two things: (1) populates `cheat_runtime` so `apply_cheats` continues writing memory-poke entries every frame, AND (2) calls `core.cheat_reset()` followed by `core.cheat_set(idx, enabled, code)` for each enabled `libretro_code` entry. The core's machinery applies its own decoder + frame-level patching from there. `apply_cheats` filters to `kind == "memory_poke"` so libretro-code entries don't double-fire.
- **Shipped (UI):** CheatsTab form gains a Type dropdown — Memory poke (existing region / width / offset / value fields) vs Code (single code-string input). Hint copy explains the format-per-core flexibility (Game Genie / GameShark / ProAR / raw, separated by `+` for multiple). List + toggle + delete + cheat search all work unchanged across both kinds.
- **Validation:** `cargo test --workspace` 130/130 green (existing `cheats_crud_roundtrip` expanded to add a third libretro-code cheat with `code = "SXIOPO"` and assert the column round-trips). Frontend `npm run build` clean — 56 modules, 932 ms.
- **Works because we don't decode codes ourselves.** Every libretro core that supports cheats already ships the per-system decoder logic. Beetle PCE Fast / Mednafen cores accept Game Genie / Pro Action Replay / raw. FCEUmm + Mesen accept NES Game Genie. Snes9x + bsnes accept SNES Game Genie. Each system community publishes codes in the format their preferred core wants — the user pastes verbatim, the core handles it.
- **Open: per-system code-format hint.** v1 shows a generic hint string in the form. A small follow-up: per-system the placeholder text could say "NES Game Genie format: 6 or 8 characters from APZLGITYEOXUKSVN" — purely UX polish.

---

## 2026-05-18 — RetroArch parity slice 7: run-ahead

Single-instance run-ahead — same algorithm RetroArch uses when not running a second core copy. Reduces perceived input latency by `N` frames (where most games' internal input-to-screen latency is 2-3 frames, so N=2 typically halves the feel).

- **Algorithm.** Per render frame, after the truly-normal `run_frame` produces frame X: (1) drain the real audio samples to a hoisted Vec, (2) `save_state` into a hoisted buffer, (3) call `run_frame` an additional `N` times to advance to frame X+N, (4) `renderer.present(framebuffer())` with X+N's pixels, (5) `load_state` back to post-X, (6) push the saved real audio. Cost per frame: 1 save + N run_frames + 1 load + 1 audio Vec clone. PCE save state is ~50 KB, run_frame ~0.1 ms — N=2 lands around 0.5 ms total. Heavier cores eat more budget; the renderer thread's frame-budget timer absorbs jitter but the audio ringbuf will drop if the loop exceeds 16 ms.
- **What gets fast-forwarded.** Only the truly-normal NORMAL play branch. SCRUB / TAS REPLAY / TAS RECORDING / PAUSE / FAST-FORWARD / SLOW-MOTION all skip — those branches have their own time semantics where peeking ahead either breaks determinism (TAS) or is meaningless (already running multiple frames per cycle).
- **Audio source of truth.** The user hears samples from the REAL run_frame, not the lookahead frames. The peek's audio is implicitly discarded by `drain_audio`'s clear-on-call semantics (we don't drain again after the load_state, so the future frames' samples disappear with the rollback). The user's "current" frame is sonically X, visually X+N — which matches how RetroArch users describe the experience.
- **Shipped (Rust):** New `EmuCommand::SetRunAhead(u32)` (clamped 0..=5). Hoisted `run_ahead_save_buf: Vec<u8>` + `run_ahead_audio_buf: Vec<i16>` outside the emu loop to avoid per-frame allocations. `ran_ahead: bool` declared at the top of the per-frame `if let Some(core_ref)` block; the post-frame present + drain block (line 3093) wraps in `if !ran_ahead { ... }` to avoid double-rendering. NEW `set_run_ahead(frames)` Tauri command.
- **Shipped (frontend):** `runAheadFrames: number` in `settings/store.ts` (default 0, persisted). `createEffect` pushes to `set_run_ahead` on change. Slider 0-5 in OA Settings → Display tab labeled "off" / "+1f" / "+2f" / etc. with hint copy about cost + skip conditions.
- **Validation:** `cargo test --workspace` 130/130 green (no new tests — the algorithm is integration code wired to a running core; pure-Rust tests would test the trait's no-op defaults). Frontend `npm run build` clean — 56 modules, 1.00 s.
- **Almost:** Operator validation. Easiest A/B test: pick a game with noticeable input lag (e.g. Castlevania, Ninja Gaiden), play with run-ahead = 0, then crank to 2, jump again — the response should feel tighter.
- **Per-system override deferred.** v1 is OA-wide only. Some heavy future cores (Saturn, PSX) will want lower defaults; per-system override slot already exists in the `SystemSettings` shape and adding it is mechanical when the time comes.

---

## 2026-05-18 — RetroArch parity slice 6: cheat search

Builds on slice 5 (raw memory-poke cheats) by adding the find-an-address workflow. Without a cheat code database for a given game, users typically use this to discover memory locations themselves: snapshot, do something in-game, filter against the previous state, repeat until a small candidate set remains.

- **Shipped (module):** New `apps/oa-shell/src/cheat_search.rs`. Owns the session shape (`CheatSearchSession { region, width, previous, candidates }`), the predicate enum (`Changed / Unchanged / Increased / Decreased / EqualToValue(N)`), and the in-place `apply_filter` that retains matching offsets + refreshes the baseline so chained filters compare against the new bytes. Plus a `summarize(session, current, limit)` that builds a wire-friendly `CheatSearchSummary` with the top-N candidates (offset + current + previous value) for the UI's list.
- **Shipped (Tauri):** `start_cheat_search(region)` writes a sentinel `Some(Vec::new())` into the matching memory_snapshot field to flip the per-frame snapshot gate on, then polls briefly (6 × 20 ms = 120 ms ceiling) for the emu thread to seed the real bytes. Returns the initial summary with every offset as a candidate. `filter_cheat_search(filter)` reads the current snapshot (already fresh via per-frame refresh) and narrows. `peek_cheat_search` returns the current candidate list without filtering. `end_cheat_search` drops the session.
- **Shipped (state):** `AppState.cheat_search: Arc<Mutex<Option<CheatSearchSession>>>`. None when no search active. Sits alongside `disc_state` / `memory_snapshot` / `rewind_state` in the same shared-state pattern.
- **Shipped (UI):** PerGameSettingsDrawer Cheats tab gains a "Cheat search" panel at the top. Region picker + Start button when idle; when active, six predicate buttons (≠ / = / ▲ / ▼ / = value input / ↻ peek) plus a candidate list (offset + current + previous) with per-row "Make cheat" buttons that pre-fill the cheat editor with the hit's address + value. The whole panel collapses back to "Start search" when the user hits End.
- **Workflow:** Start search → do something in-game that changes the target (e.g. take damage if you're looking for HP) → click the matching filter (Decreased / Changed / etc.) → repeat with another in-game change → typically 3-5 filters narrow thousands of candidates to a single digit. Click "Make cheat" on a hit → cheat row pre-filled → tweak the value (e.g. lock HP to 999) and save.
- **Validation:** `cargo test --workspace` 130/130 green (+7 in `cheat_search::tests` covering each predicate, baseline refresh, chained filters narrowing, and top-N capping). Frontend `npm run build` clean — 56 modules, 986 ms.
- **v1 scope notes:** Width is fixed to 1 byte; users can manually expand a found hit to 2/4 in the cheat editor afterward. Predicates are byte-level comparisons against the previous snapshot — RetroArch's more advanced "search by difference of N" / "by relative amount" / "fuzzy search" all sit on top of this same dispatch shape and would be a small follow-up.

---

## 2026-05-18 — RetroArch parity slice 5: cheats (memory pokes)

Trainer-style cheats — `(region, offset, width)` triple that gets the user's `value` written every frame the cheat is enabled. Layered on top of the memory-inspector + milestones plumbing that already existed (same `MemoryRegionId` tags, same per-frame snapshot path).

- **Shipped (FFI):** New `Core::memory_region_mut(id) -> Option<&mut [u8]>` trait method on `oa_core`, default None. `LibretroCore` impl gets the `*mut` pointer from `retro_get_memory_data` and builds a `&mut [u8]` slice aliasing through. Same safety story as the read-only `memory_region` — lifetime tied to `&mut self`; libretro guarantees pointer + size are stable between load_game / unload_game.
- **Shipped (schema v5→v6):** New `cheats` table — `id / game_id (FK CASCADE) / name / description / region / offset / width / value (i64) / enabled`. Index on `game_id` for the per-game list. `migrate_v5_to_v6` is additive; existing v5 DBs upgrade in place.
- **Shipped (CRUD):** `library_db::Cheat` struct with serde + five methods (`list_cheats / add_cheat / update_cheat / delete_cheat`, plus FK-cascade implicit delete-via-game-delete). Five Tauri commands wrap them; `arm_cheats(gameId)` returns count + sends `EmuCommand::LoadCheats(Vec)` to the emu thread.
- **Shipped (runtime):** `cheat_runtime: Vec<Cheat>` on the emu thread, cleared on LoadRom + loaded via `LoadCheats` from `arm_cheats` (called by `handleLaunch` right after `arm_milestones`). New `apply_cheats(core, cheats)` helper writes each enabled cheat's value to the configured memory region after every NORMAL / FAST-FORWARD / SLOW-MO `run_frame`. Width 1/2/4 → 1/2/4 bytes little-endian; other widths silently no-op (defensive against corrupted persisted rows). Skipped during TAS replay so deterministic playback isn't corrupted by memory writes.
- **Shipped (UI):** PerGameSettingsDrawer gains a new **"Cheats"** tab. List of existing cheats with enable checkbox + delete + edit; "+ Add cheat" reveals an inline form with name / region / width / offset (hex) / value (decimal). Add / Update / Toggle / Delete all re-call `arm_cheats` so live edits take effect on the running core without a relaunch.
- **Validation:** `cargo test --workspace` 123/123 green (+1 `cheats_crud_roundtrip` covering insert / update value+enabled / delete / FK cascade). Frontend `npm run build` clean — 56 modules, 1.06 s.
- **Game Genie / Action Replay / GameShark codes are NOT first-class** in v1 — those are system-specific encodings. Users translate to raw `(address, value)` via online tables for now. Per-system decoders (especially Game Genie for NES + SNES — RetroArch ships these) are a tractable follow-up; ~150 LOC per system.
- **No cheat search in v1.** The infrastructure to do "scan memory for value, then filter on increase / decrease / equal-to" sits on top of the snapshot machinery the memory inspector already runs every frame; it's the right thing for a polish session.

---

## 2026-05-18 — RetroArch parity slice 4: soft patching (IPS / UPS / BPS)

The unlock for ROM hacks + fan translations. PCE / Lynx / SNES communities all maintain a body of patches that only make sense applied to specific dumps — having to pre-patch with an external tool is friction. This slice runs the patch in-process before bytes reach `retro_load_game`.

- **Shipped (decoder):** New `apps/oa-shell/src/patch.rs` (~360 LOC) with three independent decoders. Auto-detects the format from the magic bytes (`PATCH` / `UPS1` / `BPS1`). IPS handles 24-bit BE offsets + RLE blocks + EOF terminator with optional truncate-length. UPS implements the self-correcting VLQ ("add 1 on continuation") + XOR diff blocks terminated by `0x00`. BPS implements VLQ + four opcodes (SourceRead / TargetRead / SourceCopy / TargetCopy) with signed offsets for the relative-cursor variants. CRC32 fields at the trailer are read past but not validated in v1 — adding a `crc32fast` dep is small but the patch ecosystem is rarely wrong about its own files.
- **Shipped (apply path):** `launch_rom` Tauri command reads the per-game `GameOverrides.patch_path` (new field in the existing `overrides_json` blob — no SQLite migration needed). For byte-source ROMs only, it loads + parses + applies the patch BEFORE sending the LoadRom EmuCommand. Failure surfaces as a clear error toast naming the patch and the failure reason ("IPS: ran off the end without finding EOF" etc.). CD images skip patching entirely since the core opens the `.cue` / `.chd` / `.m3u` directly — patching them would need shadow-mounting, which is out of scope.
- **Shipped (UI):** PerGameSettingsDrawer Core tab gains a "ROM patch" row: file picker (`pick_patch_file` Tauri command filtering for `.ips` / `.ups` / `.bps`), truncated filename display, and a Clear button. Hint copy explains the byte-source-only scope. The override chip shows "No patch" when unset.
- **Validation:** `cargo test --workspace` 122/122 green (+8 in `patch::tests`: IPS simple-record, IPS RLE block, IPS auto-extends ROM size, IPS header rejection, format auto-detect, BPS TargetRead, BPS SourceRead passthrough, signed-offset round-trip). Frontend `npm run build` clean — 56 modules, 1.00 s.
- **Almost:** Operator validation. Drop a `.ips` / `.ups` / `.bps` patch somewhere on disk, open the matching ROM's Properties → Core tab, click "ROM patch → Pick…", select the patch, relaunch. The bytes the core sees are the patched ROM. Try a real translation patch (e.g. the Cosmic Fantasy 2 fan English translation for PCE is `.ips`; many SNES patches are `.bps`).
- **Open polish:** CRC32 validation in a follow-up (small `crc32fast` dep, would reject patches against the wrong base ROM up-front). Patch path chooser in the right-sidebar quick-actions for power users.

---

## 2026-05-18 — RetroArch parity slice 3: gameplay hotkey bundle

Six universal hotkeys that every emulator user expects. F1 (soft reset) was already wired earlier; this slice adds the other five in one focused pass to the emu loop's frame body. All gated on the existing `enable` flag (game window focused + UI not intercepting), so they don't accidentally fire while the user is typing in the library / a binding-capture dialog.

| Hotkey | Action | Behavior |
|---|---|---|
| **F1** | Soft reset | Calls `Core::reset` (already shipped) |
| **F2** | Toggle pause | Edge-triggered; pause flag short-circuits `run_frame` in the NORMAL forward-play branch. Scrub / replay / rewind paths unaffected. |
| **F3** | Frame advance | Edge, only while paused. Runs exactly one `run_frame` then stays paused. TAS / debugging staple. |
| **F5 / F8** | Save / load state | Already shipped. |
| **F6** | Fast-forward (hold) | While held, calls `run_frame` 4× per render cycle → wall-clock seconds map to 4× game-seconds. Audio plays sped up (chipmunk); accept-and-document for v1. |
| **F7** | Slow-motion (hold) | While held, calls `run_frame` every other render cycle → 0.5× speed. Render rate stays constant; game time slows. |
| **F12** | Screenshot | Edge; writes `appData/screenshots/<rom-stem>/<timestamp>.png` via the same `write_thumbnail` path save-state thumbnails use. Toast confirms the filename. |

- **Shipped:** New state vars on the emu loop — `prev_f2 / prev_f3 / prev_f12 / paused / frame_advance_request / slow_mo_phase`. Two new tuning constants: `SLOW_MOTION_DIVISOR = 2` (half speed) and `FAST_FORWARD_BURST = 4` (4× speed). The NORMAL branch's `run_frame()` call wrapped in a four-way ladder (paused / fast-forward / slow-motion / normal); the other three play modes (SCRUB, REPLAY, HOLD-BACKSPACE rewind) stay untouched — those have their own time semantics. New `write_screenshot(app_data_dir, stem, w, h, rgba) -> PathBuf` helper that reuses the existing `write_thumbnail` PNG encoder.
- **Shipped (frontend):** `SUPPRESS_DEFAULT` in App.tsx extended with `F2 / F3 / F6 / F7 / F12` so the browser doesn't open Help / dev tools when the user hits these in single-window mode. Same Set-membership check as F1 / F5 / F8.
- **Audio behavior during fast-forward:** the core emits 4× the normal sample count per real second; our cpal ringbuf overflows and drops, producing partially-chipmunk audio. Accept-and-document for v1 — RetroArch users are accustomed to FF audio sounding off. A future polish could mute or downsample.
- **Validation:** `cargo test --workspace` 114/114 green (no new tests — the hotkey bundle is integration code wired to a running core; pure-Rust tests would test the trait's no-op defaults). Frontend `npm run build` clean — 56 modules, 49.74 kB CSS / 320.63 kB JS, 1.03 s.
- **Almost:** Operator validation. F2 pauses, F3 steps frames in paused state, F6 fast-forwards (4× game speed), F7 slow-mos (half speed), F12 writes a PNG. F1 (reset) already works.
- **RetroArch parity backlog after this slice:** Soft patching (IPS/UPS/BPS) · Cheats · Run-Ahead · RetroAchievements · Netplay (probably never).

---

## 2026-05-18 — RetroArch parity slice 2: disc control for multi-disc games

The companion to slice 1 (core options). Multi-disc CD games like Ys IV / Sakura Wars / Tengai Makyō II registered `.m3u` playlists with their cores but had no UI to swap discs mid-game. This slice wires the libretro `SET_DISK_CONTROL_INTERFACE` (v1) + `SET_DISK_CONTROL_EXT_INTERFACE` (v2) callbacks end-to-end so users can eject + insert different discs from the QuickSettings overlay.

- **Shipped (FFI):** New callback structs `retro_disk_control_callback` (v1: 7 function pointers) and `retro_disk_control_ext_callback` (v2: v1 + 3 ext getters for path / label / initial-image). All function pointers wrapped in `Option<...>` since cores can leave individual entries null. `state.rs` handles `SET_DISK_CONTROL_INTERFACE` / `SET_DISK_CONTROL_EXT_INTERFACE` by storing the registered struct on `State`. `GET_DISK_CONTROL_INTERFACE_VERSION` reports v1 (we accept v2 register but our read path only uses the universal-v1 callbacks plus v2's `get_image_label` when available).
- **Shipped (oa_core trait):** New `DiscInfo` type — `{num_discs, current_index, ejected, labels: Vec<String>}` — with serde derives so it round-trips through Tauri events. Three new `Core` trait methods with no-op defaults: `disc_state() -> Option<DiscInfo>`, `set_disc_eject(bool)`, `set_disc_image(u32)`. `LibretroCore` impl prefers the v2 callback struct (carries labels via `get_image_label`); falls back to v1 with empty labels (UI shows `Disc 1` / `Disc 2`).
- **Shipped (shell wiring):** `AppState` gains `disc_state: Arc<Mutex<Option<DiscInfo>>>` mirroring the rewind / TAS / video shared-state pattern. Emu thread refreshes it on every successful LoadRom AND after every `SetDiscEject` / `SetDiscImage` handler so the cached snapshot stays accurate. Three new Tauri commands: `get_disc_state()` reads the cache; `set_disc_eject(bool)` + `set_disc_image(u32)` send EmuCommand variants to the emu thread. Both new EmuCommand variants are no-ops when no core is loaded.
- **Shipped (UI):** QuickSettings overlay gains a 6th view (`"disc"`) alongside actions/rewind/tas/video/memory. The "Disc control…" ActionRow is only rendered when `discInfo.numDiscs > 1` (single-disc CD games and HuCards hide it entirely). DiscPanel lists every disc with the active one highlighted in the system accent + "Loaded" badge; inactive discs show an "Insert" button that runs the canonical eject → set_image_index → close sequence with a swap-in-flight lock so the protocol steps can't interleave. Labels fall back to "Disc N" when the core doesn't supply them (v1 callback).
- **Validation:** `cargo test --workspace` 114/114 green (no new tests — the disc-control surface needs a real core + multi-disc image to exercise end-to-end; pure-Rust tests would be testing the trait's no-op defaults). Frontend `npm run build` clean — 56 modules, 49.74 kB CSS / 320.22 kB JS, 1.04 s.
- **Almost:** Operator validation with a real multi-disc game. Recommended: Ys IV: The Dawn of Ys (`.m3u` with both `.chd` files), Cosmic Fantasy 4, or Tengai Makyō II — all PCE-CD titles with disc-swap prompts. Open the QuickSettings overlay (Esc during gameplay), click "Disc control…", verify both discs show, swap to disc 2, confirm the game continues from the right place.
- **Closes the two-feature parity sprint** (core options + disc control). The RetroArch-parity backlog still has fast-forward / frame-advance / screenshots / cheats / Run-Ahead / RetroAchievements queued — most are smaller now that the surface pattern is established.

---

## 2026-05-18 — RetroArch parity slice 1: core options exposure

Operator question — *what does RetroArch have per-core that we don't?* Biggest single gap was **core options**: every libretro core ships 15-30 per-core knobs (Mednafen PCE's CD audio interpolation + speed, Beetle Lynx's rotate + sound correction, FCEUmm's overclock, Snes9x's super-fx clock speed, etc.) that we previously declined wholesale via `RETRO_ENVIRONMENT_GET_VARIABLE` returning NULL. This slice surfaces them with a full three-tier inheritance model. Disc control ships in the next session.

- **Shipped (FFI):** New core-option structs in `crates/oa-libretro/src/ffi.rs` covering V2 + V1 + the legacy variables format: `retro_core_option_value`, `retro_core_option_definition`, `retro_core_option_v2_definition`, `retro_core_option_v2_category`, `retro_core_options_v2`, plus the matching `_intl` wrappers. Sentinel-terminated arrays (NULL key) plus embedded values arrays bounded by `RETRO_NUM_CORE_OPTION_VALUES_MAX = 128`.
- **Shipped (state.rs parsers):** Three parsers handle every format cores ship today: `parse_legacy_variables` splits the `"desc; opt1|opt2|opt3"` string format (first option = default); `parse_core_options_v1` walks the V1 definition array; `parse_core_options_v2` adds category_key handling. V2_INTL prefers the `local` struct when populated, else `us`. SET_CORE_OPTIONS_V2 also parses `parse_v2_categories` for UI grouping. SET_CORE_OPTIONS_DISPLAY is accept-and-ignore for v1 (a future polish respects per-option visibility).
- **Shipped (GET_VARIABLE response):** Previously returned NULL for every key. Now resolves: user-overridden value (from `option_values` map, stored as `CString` for stable C pointer lifetime) → schema default (synthesized lazily into the same map so the pointer survives the env call). GET_VARIABLE_UPDATE returns + clears `variables_updated`; flag starts `true` so the core's first poll triggers a re-read.
- **Shipped (oa_core trait):** New `CoreOption / CoreOptionValue / CoreOptionCategory` types with serde derives. Three new `Core` trait methods with default empty/no-op impls: `options() / option_categories() / set_option(key, value)`. `LibretroCore` implements all three by reading from the shared state.
- **Shipped (persistence):** New `apps/oa-shell/src/core_options.rs` module owns per-system files at `appData/core-options/<system>.json` with `{schema, categories, values}` shape. `refresh_schema` captures the schema on every successful core load, preserving user values whose keys still exist (a core update can remove options). Per-game overrides live in the existing `GameOverrides.core_options: HashMap<String, String>` (no new SQLite column — uses the existing `overrides_json` blob). Three tests cover the inheritance resolver, the merged effective-values builder, and stale-key dropping on schema refresh.
- **Shipped (commands + emu wiring):** New EmuCommand variants `SetCoreOption { key, value }` and `ApplyCoreOptions(HashMap)` — emu thread calls `core.set_option` on each. LoadRom handler captures schema to disk after load_game then applies per-system effective values inline (per-game overlay arrives via the apply command). Four new Tauri commands: `list_core_options(systemId, gameId?)` returns the full `{schema, categories, systemValues, gameValues}` snapshot; `set_system_core_option / set_game_core_option(key, value)` write to disk/SQLite + push to the running core; `apply_game_core_options(gameId)` is called from `handleLaunch` after `set_bloom_amount` so the merged values land on the first frame.
- **Shipped (UI):** New shared `CoreOptionsPanel` component in `frontend/src/components/CoreOptionsPanel.tsx`. Renders a filterable list of options, each with a dropdown of allowed values + an inherited-value chip ("per-system: …" / "default: …") + a Reset button that clears the override. Used in both the per-system Settings page (new "Core options" tab) and the per-game Settings drawer (new "Core options" tab, panel scoped to gameId). Empty state when no schema is captured yet ("launch a game once for this system, options appear after").
- **Validation:** `cargo test --workspace` 114/114 green (+3 new in `core_options::tests`). Frontend `npm run build` clean — 56 modules, 49.71 kB CSS / 317.62 kB JS, 924 ms.
- **Almost:** Operator validation. Launch a PCE game, open Settings → tg16 → Core options, see all the Mednafen PCE knobs. Change one (e.g. "PCE: CD Audio Volume" if exposed by the loaded core), confirm the live core re-reads on next frame. Same flow for per-game overrides via the game-properties drawer.
- **Deferred to next session:** Disc control end-to-end (FFI + Tauri + QuickSettings disc view). The two features were planned together but core options was the bigger lift; disc control gets a clean focused pass instead of being squeezed into the same turn.

---

## 2026-05-18 — Library mgmt: hide/show systems, auto-remove, bulk clear

Operator question — *the directory watcher detects removes but nothing happens with them; how do we hide / reorder / drop systems and ROMs?* Three behaviors landed in one pass, modeled after LaunchBox's Manage Platforms surface.

- **Shipped (1) — Hide/show systems.** `LayoutPrefs.hidden_systems: Vec<String>` (Rust) + `hiddenSystems: string[]` (frontend), plus `auto_hide_empty_systems: bool` (default true). LeftSidebar's `systemIds` memo filters the registry by both: explicit hide list AND zero-game systems when auto-hide is on. The currently-viewed system is always kept visible so it can't disappear under the user. `SystemContextMenu` gains a "Hide from sidebar" item; Settings → Library page gains a "Sidebar systems" section with a checkbox per system + auto-hide toggle. Right-clicking a hidden system isn't possible from the sidebar (it's not there), but the Settings checkbox un-hides it.
- **Shipped (2) — Auto-remove on file delete (opt-in).** New `autoRemoveOnDelete: boolean` in the OA-wide settings store (default false — matches the historical soft policy). New Tauri command `find_game_id_by_path(path) -> Option<id>`. App.tsx's `oa://library-watch-removed` listener now branches: off keeps the entry (old behavior, useful for moves / renames); on looks up the matching id and calls `library.remove(id)`. Setting surfaced as a checkbox in Settings → Library → Cleanup.
- **Shipped (3) — Bulk clear actions.** Two new SQL helpers + Tauri commands: `delete_games_for_system(system_id) -> usize` (per-system) + `delete_all_games() -> usize` (full reset). Both return the count removed for the success surface. `library.clearForSystem(id)` and `library.clear()` updated to use them — `clear()` previously ran a per-row delete loop, now one DELETE. Settings → Library → Cleanup gains a per-system picker ("Clear games for: …") + a red "Danger zone" button that resets the entire library after a `confirm()` dialog. Files on disk are never touched — the DB just forgets the rows.
- **Validation:** `cargo test --workspace` 111/111 green (+3 new in `library_db::tests`: `find_id_by_file_path_returns_match_or_none`, `delete_games_for_system_removes_only_that_system`, `delete_all_games_resets_library`). Frontend `npm run build` clean — 55 modules, 49.66 kB CSS / 313.17 kB JS, 997 ms.
- **Operator note:** The watcher's "file removed" event was wired but ignored before this slice. Turn on Settings → Library → Cleanup → "Auto-remove…" to opt into the new behavior; or just hit "Reset entire library" + re-scan for a clean slate.
- **Next:** Same backlog. Now would be a natural moment to do the per-core operator validation pass (Lynx / NES / SNES / pce-cd real-ROM smoke tests) — the library plumbing is in a tidy place to start dropping fresh ROM folders.

---

## 2026-05-18 — Cross-system d-pad fix: re-apply bindings on system swap

Same operator session that surfaced the Z=primary fix turned up a worse bug: arrow keys reading the wrong directions on NES/SNES/Lynx after launching a PCE game first. **Pressing down-arrow on an NES game flipped LEFT in the core; left flipped RIGHT; right flipped DOWN.** Same kind of permutation for the action buttons (Z and X read at unexpected bits). PCE-only sessions worked correctly because that's what the InputPoller was initialized with.

- **Root cause:** PCE's d-pad bit layout is *clockwise* — UP=4, RIGHT=5, DOWN=6, LEFT=7 — designed to round-trip through `pce_to_libretro_bits` to libretro's straight order. NES/SNES/Lynx use libretro's straight layout directly with identity remaps. The shell's startup wires the InputPoller with PCE bindings at bit positions 4/5/6/7 mapped to UP/RIGHT/DOWN/LEFT. When the user launches an NES game, `current_system_id` flips to `"nes"` so `to_libretro_bits` switches to identity — but the **InputPoller's bit-to-key table was never re-applied** for the new system. So arrow keys stayed at PCE's clockwise slots, and the NES identity remap read them as libretro's straight order: PCE-DOWN-bit (6) = libretro-LEFT (6). Result: down moves you left.
- **Shipped:** New `InputPoller::clear_port_bindings(port)` in `crates/oa-input/src/lib.rs` — zeros all 32 keyboard + 32 gamepad slots for a port. `apply_bindings_to_poller` in `apps/oa-shell/src/main.rs` calls this first so stale slots from another system can never leak through. The `LoadRom` handler now detects system changes (`current_system_id != system_id`) and re-applies the new system's bindings via `bindings::load + apply_bindings_to_poller`. Logged for diagnostics.
- **Shipped (regression test):** `dpad_lands_on_correct_libretro_bits_for_every_system` in `bindings.rs::tests`. Iterates every registered system, asserts `to_libretro_bits(sys, bit_for(sys, "UP"))` is libretro UP (1<<4); same for DOWN/LEFT/RIGHT. Catches the case where a future system's d-pad layout drifts from its remap.
- **Validation:** `cargo test --workspace` 108/108 green (+1 from the d-pad test, no other changes). No frontend changes.
- **Next:** Same backlog. Possible follow-up: bonk out the same fix for gamepad inputs (the same clear-on-swap helps gamepads too — `clear_port_bindings` handles both arrays, so it's already covered).

---

## 2026-05-18 — Keyboard default fix: Z = primary action on every system

Operator-reported regression: pressing Z on PCE plays the primary action (jump in Bonk's Adventure), but pressing Z on NES / SNES / Lynx plays the secondary action. The bindings for the three non-PCE systems were assigned per the PC-emulator tradition for each (Nestopia/FCEUX put NES `A` on `X`; ZSNES put SNES `A` on `X`; Lynx mirrored that). That's historically defensible per-system but indefensible across-systems: the launcher is a single muscle-memory surface and every system's "jump" button should sit on the same keyboard key.

- **Shipped:** `default_lynx_bindings`, `default_nes_bindings`, `default_snes_bindings` in `apps/oa-shell/src/bindings.rs` updated. Z is now the **primary action button** (libretro A; system-specific name varies — A on NES/SNES/Lynx, I on PCE) on every system. X is the **secondary** (libretro B; B on NES/SNES/Lynx, II on PCE). For SNES the top of the diamond (X button) stays on `S` and the left of the diamond (Y button) stays on `A`; only the A/B kb mapping swapped. Gamepad mappings unchanged — they were already consistent (East = primary, South = secondary) across systems.
- **Shipped:** New test `z_is_the_primary_action_button_on_every_system` in `bindings.rs` iterates every registered system, asserts `keyboard["I" or "A"] == "Z"` and `keyboard["II" or "B"] == "X"`. Future system additions that ship swapped defaults trip this test before they can land.
- **Existing users:** Per-system bindings files (`appData/bindings/<id>.json`) take precedence over defaults — anyone who already opened the bindings editor on one of these systems keeps whatever they saved. Fresh installs and untouched systems pick up the corrected defaults. Existing users wanting the new convention can click "Reset to defaults" in the bindings editor.
- **Validation:** `cargo test --workspace` 107/107 green (+1 from the new lock test). No frontend changes.
- **Next:** Same backlog (7800 onboarding / per-core validation / remaining medium-polish).

---

## 2026-05-18 — System page header + bindings deep-link

The per-system bindings editor (and the rest of `PerSystemSettingsPage`) was reachable today but heavily buried: only via the small ⚙ icon in the GridControls bar after left-clicking into a system, or via the right-click context menu's single "System settings…" entry. The design doc (`docs/PLANS/main-window.md`) calls out a "System page header → ⚙" pattern that hadn't been built; this entry ships it.

- **Shipped:** New `frontend/src/components/SystemHeader.tsx` — full-width header bar shown above the GridControls when the active view is system-filtered (`currentView.kind === "system"`). Renders the system's short-name chip in its accent color, the system's display name, the game count, and a row of quick-action buttons: **Bindings** (deep-links to the Input tab), **Cores**, **Shaders**, **Settings** (lands on the user's last-viewed tab). All buttons paint with the system's accent color via the `data-system` cascade. Sits above GridControls so the existing sort/group/view controls keep their place — the new header adds identity + quick-jump affordances without rearranging anything below.
- **Shipped:** `PerSystemSettingsPage` gained an `initialTab?: SystemSettingsTab` prop. When passed, it seeds the active tab to that value instead of the localStorage-persisted last choice — the new SystemHeader buttons + the SystemContextMenu's new "Edit bindings…" item both use this to deep-link to a specific tab. The localStorage persistence still applies once the user manually switches tabs inside the page.
- **Shipped:** `SidebarView`'s `system-settings` variant extended with `initialTab?: SystemSettingsTab`. `SystemSettingsTab` is the union of the seven existing tab ids exported from `layout/LeftSidebar.tsx` for reuse. App.tsx forwards the field through to PerSystemSettingsPage.
- **Shipped:** `SystemContextMenu` (right-click menu on left-sidebar system entries) gains an "Edit bindings…" item above the existing "System settings…" item. Bindings item passes `"input"` as the tab; Settings item passes undefined (respects last-viewed). The menu is now a more credible discovery surface for per-system settings — there's a verb-shaped entry that takes you directly to the most-requested action.
- **Validation:** Frontend `npm run build` clean — 55 modules (+1: SystemHeader), 49.22 kB CSS / 307.02 kB JS, 849 ms cold. No Rust changes.
- **Almost:** Operator visual validation. Click into TG-16 in the sidebar, see the new header with the orange short-name chip + Bindings/Cores/Shaders/Settings row. Click Bindings → lands on the Input tab. Right-click NES in the sidebar → "Edit bindings…" → lands on Input tab.
- **Next:** Same backlog as before — Atari 7800 next-system onboarding, per-core validation pass (Lynx/NES/SNES/pce-cd real-ROM smoke tests), or any of the remaining medium-polish items (audio device picker, rewind scrubbing thumbnails).

---

## 2026-05-18 — Cross-cutting polish pass: bloom UX, video WebM, system defaults, bindings chips

Six small wins in one pass — each item carrying as a backlog ⬜ before the session.

- **Shipped (1) — Live bloom slider during gameplay.** PerSystemSettingsPage + PerGameSettingsDrawer slider's `onInput` now fires `set_bloom_amount` directly in addition to persisting the override. User sees the bloom change while dragging, not on next launch.
- **Shipped (2) — OA-wide bloom slider in Settings → Display.** `bloomAmount` added to the OA-wide settings store (default 0.6, persisted in `localStorage[oa.settings.v1]`). Slider in `SettingsPage.tsx` Display tab. Push-to-renderer createEffect mirrors the existing scaling-mode pattern. App.tsx launch chain becomes per-game → per-system → OA-wide (was per-game → per-system → null). Full three-tier slider for the Phosphor composite weight now in place.
- **Shipped (3) — Focus-gated input properly logged as done.** Already shipped in code (the `Arc<AtomicBool>` driven by `WindowEvent::Focused` fans through to `InputPoller::set_enabled` each frame), but the carrying ⬜ in `docs/cores/tg16/ROADMAP.md` Phase 1.5 was stale. Closed out in the doc — no code change needed.
- **Shipped (4) — Per-system shader preset defaults via the registry.** `SystemTheme.defaultShaderPreset?: string` added in `frontend/src/themes/registry.ts`. Lynx + NES + SNES → `crt-lite` (matches CRT-era aesthetic at low source resolutions); tg16 + pce-cd → `plain` (HuCard art is sharp; CDDA/FMV stays crisp). New `resolveShaderPreset(value, systemId)` helper handles a `"system-default"` sentinel — that's the new OA-wide `DEFAULT_SHADER_PRESET`, so fresh installs fall through to the registry's per-system pick. Existing users with persisted `"plain"` stay on `"plain"` (their explicit value wins). App.tsx launch chain consults the helper.
- **Shipped (5) — ffmpeg WebM conversion for video clips.** New `convert_video_clip_to_webm(clipDir)` Tauri command (blocking, on the tokio command-thread pool; UI stays responsive). Reads fps + frame_pattern from the existing `manifest.json`; shells out to `ffmpeg -y -framerate FPS -i frame_%06d.png -c:v libvpx-vp9 -b:v 2M clip.webm`. ffmpeg-not-on-PATH returns a clear error pointing at https://ffmpeg.org/download.html. QuickSettings VideoPanel gains a per-clip "WebM" button with idle/in-flight/done states (`WebM` / `…WebM` / `✓ WebM` / `⚠ WebM`); all clip buttons disable while a conversion is in flight so only one ffmpeg fires at a time. Closes Phase 4 slice D-2.
- **Shipped (6) — Button-label chips in the bindings editor.** SystemBindingsEditor.tsx renders the leftmost button-name cell (UP/DOWN/I/II/A/B/RUN/SELECT/etc.) as a monospace chip with the system's accent border + soft fill + accent-soft text. CSS-only — no SVG glyph work. Visual upgrade to the per-system bindings UI without design assets.
- **Validation:** `cargo test --workspace` 106/106 green (no test count change — all six items are integration code that doesn't unit-test cleanly without a real ffmpeg / Tauri / browser environment). Frontend `npm run build` clean — 54 modules, 48.82 kB CSS / 303.13 kB JS, 1.10 s cold.
- **Almost:** Operator visual validation. Bloom slider dragging in real-time; per-system shader defaults visible on fresh install; ffmpeg conversion against a real clip; bindings chips against each system's accent palette.
- **Next:** Skipped from this polish pass: audio device picker (blocked on a live device-swap refactor in `oa-audio`), TG-CD theming polish (needs design assets), rewind scrubbing thumbnails (multi-crate refactor — deserves dedicated session). Logical next: Atari 7800 onboarding (first-wave queue), or the per-core validation pass (Lynx → NES → SNES → pce-cd real-ROM smoke tests).

---

## 2026-05-18 — Phase 3 slice D + slice-C polish: hot-reload + bloom slider

- **Shipped (slice D — hot-reload):** New `apps/oa-shell/src/shader_presets_watcher.rs` — `notify::RecommendedWatcher` on `<exe_dir>/shaders/presets/`, filters events to `*.preset.toml` files only, then (a) reloads the registry via `shader_presets::load_all`, (b) emits `oa://shader-presets-changed` with the fresh summary list, and (c) re-resolves + re-applies the currently-active preset via `EmuCommand::ApplyShaderPreset` so a `bloom_amount` tweak in `phosphor.preset.toml` takes effect on the next frame without a relaunch. `AppState` gained `active_shader_preset: Arc<Mutex<Option<String>>>` (set by `set_shader_preset` from the Tauri command path) + `shader_presets_watcher: Option<ShaderPresetsWatcher>` (held to keep the OS watcher alive — `Mutex<Option<Box<dyn Watcher + Send>>>` wrap so the struct is Sync per Tauri's `State<'_, T>` bound). Watcher dies cleanly when AppState drops; mkdir's the presets dir at startup so fresh installs have somewhere to drop user files.
- **Shipped (slice C polish — bloom slider):** Three-tier inheritance for the Phosphor composite weight: TOML preset default → per-system override → per-game override. `SystemSettings.bloom_amount: Option<f32>` + `GameOverrides.bloom_amount: Option<f32>` added (drops `Eq` derive from both — f32 isn't `Eq`; `PartialEq` is sufficient for the existing tests). New `EmuCommand::SetBloomAmount(f32)` → `renderer.set_bloom_amount`. New `set_bloom_amount(amount)` Tauri command. Per-system + per-game settings Shaders tabs gain a slider (range 0..1, step 0.05) with the standard SettingRow inheritance chip + a Reset button to clear the override. App.tsx's `handleLaunch` resolves `bloomAmount` per-game → per-system → null and calls `set_bloom_amount` AFTER `set_shader_preset` (awaited sequentially, not in `Promise.all`, since the EmuCommand channel order matters — the preset's TOML default lands first, then the override layers on top).
- **Shipped (frontend hot-reload listener):** `applyShaderPresetsUpdate(list)` exported from `settings/shader_presets.ts` and called from App.tsx's `onMount` via `listen("oa://shader-presets-changed", ...)`. Open dropdowns refresh instantly when a preset is added / renamed / removed. The Rust watcher handles the re-apply path on its own — no frontend re-invoke needed.
- **Validation:** `cargo test --workspace` 106/106 green (+4 new in `shader_presets_watcher::tests` covering the relevance filter: `.preset.toml` create/modify/remove all match; `.wgsl` and plain `.toml` files are ignored). Frontend `npm run build` clean — 54 modules, 48.68 kB CSS / 300.27 kB JS, 851 ms cold.
- **Almost:** Visual operator validation. The expected workflow: launch a phosphor-running ROM, edit `<exe_dir>/shaders/presets/phosphor.preset.toml`'s `bloom_amount`, save, see the bloom strength change live. The pure-function watcher tests prove the filter is right; rendering correctness still needs an eye on the screen.
- **Next:** Phase 3 slice E — HDR tone mapping (behind a setting where the display supports it; needs an HDR-aware swapchain format like `R16G16B16A16Float`). Or Phase 6 next-system (Atari 7800). Phase 3 acceptance gate ("per-system default presets ship; per-game override works; preset survives restart") is met today.

---

## 2026-05-18 — Phase 3 slice C: TOML preset registry

- **Shipped (Rust):** Four built-in presets ship as `shaders/presets/{plain,scanlines,crt-lite,phosphor}.preset.toml` at the workspace root, compiled into the binary via `include_str!`. Schema: `display_name` / `description` / `base` (renderer pipeline) / `[params]` (today: `bloom_amount`) / `[bezel]` (image path; relative paths resolve under `<exe_dir>/shaders/`). New `apps/oa-shell/src/shader_presets.rs` module: `ShaderPresetDef` (serde Deserialize), `builtins()` parses the four compiled-in TOMLs, `load_all(exe_dir)` overlays user files at `<exe_dir>/shaders/presets/<name>.preset.toml` by name (last-write-wins, sorted alphabetically), `summarize()` returns the `{name, displayName, description, base}` view for the frontend, `apply(def, exe_dir)` decodes any referenced bezel PNG via the `image` workspace crate and returns a `ResolvedPreset { base, bloom_amount, bezel }`. Malformed user files log a warning and are skipped — the registry never fails open. Renamed `EmuCommand::SetShaderPreset(enum)` → `ApplyShaderPreset(ResolvedPreset)` carrying the decoded shape; the emu thread now calls `renderer.set_shader_preset / set_bloom_amount / set_bezel_image` in turn (or `clear_bezel_image` when the TOML has no `[bezel]` block). `set_shader_preset(preset)` Tauri command updated; new `list_shader_presets() -> Vec<ShaderPresetSummary>` Tauri command. `toml = "0.8"` dep added (already in the workspace).
- **Shipped (frontend):** Broadened `ShaderPreset` type union → `string`. Dropped the hardcoded `SHADER_PRESET_OPTIONS` + `SHADER_PRESET_LABELS` exports from `settings/store.ts`. New `frontend/src/settings/shader_presets.ts` owns a Solid signal driven by the `list_shader_presets` Tauri call, with the four built-ins as the hardcoded fallback list shown during the brief mount-to-load window. `loadShaderPresets()` fires once from App.tsx's `onMount`. PerSystemSettingsPage + PerGameSettingsDrawer dropdowns iterate `shaderPresets()` (live signal); inheritance chips read `shaderPresetLabel(name)` instead of the static map. Drop a `*.preset.toml` file in `<exe_dir>/shaders/presets/` and it shows up in the dropdown on next launch.
- **Validation:** `cargo test --workspace` 102/102 green (+5 new in `shader_presets::tests`: builtin parse + name uniqueness, phosphor's `bloom_amount = 0.6`, no built-in ships a bezel, user-overlay-by-name precedence, malformed-file safety). Frontend `npm run build` clean — 54 modules, 48.49 kB CSS / 297.82 kB JS, 852 ms cold.
- **Almost:** A bloom-amount slider in the Phosphor section of the per-system / per-game settings drawers. The renderer has the API (`set_bloom_amount`), the TOML carries the default, the Tauri command applies the override on launch — but there's no UI surface for an end-user override yet. That + a default bezel asset are the natural slice-C polish items if needed before slice D.
- **Next:** Slice D — live shader hot-reload. `notify` watching `<exe_dir>/shaders/presets/` (the same dir slice C already scans), reload the registry on change, re-apply the active preset if its TOML changed. Cheap now that the registry is data-driven.

---

## 2026-05-18 — Phase 3 slice B-2: Phosphor composite + bezel overlay (renderer)

- **Shipped (Phosphor composite):** `blit.wgsl` gained a second texture binding pair (slots 3 + 4). The Phosphor branch — now `preset_id == 3` (was 0, was passing through after the chain wrote a bare blur) — samples slot 0 (source framebuffer) AND slot 3 (chain output blur) and returns `mix(source, blur, bloom_amount)`. The `bloom_amount` rides in the existing 16-byte uniform by repurposing the former `_pad0` slot as f32. New `Renderer::set_bloom_amount(f32)` / `bloom_amount() -> f32` accessors, default 0.6. New `final_blit_bgl` (5-entry layout — tex0 / sampler0 / uniform / tex1 / sampler1) hosts the final blit pipeline + `fb_texture.bind_group`; chain passes (`blur.wgsl` H + V) keep using the original 3-entry `bind_group_layout`. Single-pass presets (Plain / Scanlines / CrtLite) point slot 3 + 4 at the same fb_view + sampler so the binding is valid but the shader ignores it. Multi-pass path now binds slot 0 = source fb (was: chain output) + slot 3 = chain output (new) — load-bearing for the composite to see both textures.
- **Shipped (bezel overlay):** New `bezel.wgsl` (fullscreen-triangle sampler) + `bezel_bgl` (2-entry: tex + sampler, no uniform) + `bezel_pipeline` with `BlendState::ALPHA_BLENDING` (standard `src.a * src + (1 - src.a) * dst`). New API: `Renderer::set_bezel_image(rgba: &[u8], w, h) -> Result<(), String>` uploads RGBA8 sRGB bytes to a new texture; `clear_bezel_image()` drops it; `has_bezel()` + `bezel_dimensions()` for diagnostics + future UI. Dimension / byte-length validation rejects malformed inputs with clear error strings. `present()` runs an extra render pass with `LoadOp::Load` after the main blit when a bezel is loaded — preserves the game pixels, blends the bezel over them. Bezel covers the full surface (not the game viewport); users design bezels to match their window dimensions — RetroArch-style bezel artwork drops in directly.
- **Validation:** `cargo test -p oa-render` 15/15 green (no new tests for bezel path — `set_bezel_image` requires a real `Renderer`, which needs a wgpu device + surface; bezel correctness validates manually). `cargo test --workspace` 97/97 green. The `shader_preset_ids_are_stable` test now locks Phosphor.id() at 3 instead of 0 — catches future accidental reordering.
- **Almost:** Shell-side wiring. Today nothing in the UI loads a bezel — there's no Tauri command, no per-system/per-game bezel asset path, no PNG file loader, no bloom-amount slider. Per the ROADMAP that's slice C work ("pairs naturally with slice C's TOML preset format since the bezel needs an asset path and the composite needs a parameter slider"). The renderer side is complete and ready for slice C to call into.
- **Next:** Slice C — per-game shader preset TOML format. Replaces the slice-A hardcoded preset ids with structured `shaders/presets/<name>.preset.toml` files that pick which passes run + their parameters (bloom_amount, bezel asset path, vignette strength, etc.). Live hot-reload (slice D — `notify` on the presets dir) lands after. Slice B-2's renderer plumbing was the architectural unlock for both.

- **Shipped:** All four ⬜ items in the re-scoped Phase 5 ROADMAP entry closed in one session. (1) **Picked the .dll** — Beetle PCE Fast (`mednafen_pce_fast_libretro.dll`) — already shipped for TG-16 HuCards — handles CD too. Spike 2's hint about `pcecd.cpp` in `vendor/mednafen/pce_fast/` proved out. (2) **Rondo of Blood** validated operator-side end-to-end — BIOS SHA-1 check, CHD load via `RomSource::Path`, title-screen video, CDDA music, gameplay, audio. (3) **`oa-cdrom` build-out** deferred to Phase 5.5 hardening pending real-gap discovery (kept as the only remaining ⬜ in the Phase 5.5 list at `docs/cores/pce-cd/ROADMAP.md`). (4) **Registry split** — option (b) shipped: dedicated `pce-cd` SystemId in the frontend registry with its own sidebar entry, theme, and per-system settings file. Cart games (`.pce`) stay under `tg16`; CD images (`.cue` / `.chd` / `.ccd` / `.toc` / `.m3u` / `.iso`) live under `pce-cd`. Cyan-blue palette at 220° (distinct from TG-16 orange, SNES violet, Lynx purple). Shared core .dll + shared input pipeline — `bindings.rs::bit_for / buttons_for / defaults_for / to_libretro_bits` all dispatch `tg16` and `pce-cd` to the same PCE table and remap (they're the same controller). Library DB **v4 → v5 migration** retags existing `tg16` rows whose `file_path` or `archive_inner_path` ends in a CD container extension. One new oa-shell test (`v4_to_v5_retags_cd_games_to_pce_cd`) covers carts, bare CDs, archived inner-.cue, and the trick case of a .pce path containing the substring "cue". Per-core docs scaffolded at `docs/cores/pce-cd/` (README + ROADMAP + SESSION_LOG + KNOWN_GAME_BUGS + DECISIONS). tg16 DECISIONS + ROADMAP updated; project ROADMAP Phase 5 closed.
- **Validation:** `cargo test --workspace` 97/97 green (was 96; +1 = the new v4→v5 migration test). Frontend `npm run build` clean — 53 modules, 48.27 kB CSS (gzip 8.59 kB) + 296.99 kB JS (gzip 76.91 kB), 975 ms cold. No regressions. Operator end-to-end validation of Rondo on Fast was the gating signal that flipped item 1 to ✅ and unlocked items 2 + 4.
- **Next:** Phase 5.5 hardening (save-state mid-disc + multi-disc `.m3u` + the `oa-cdrom` build-out only if real gaps surface) — tracked in `docs/cores/pce-cd/ROADMAP.md`. OR the first-wave next-system queue (7800 → SMS/GG → MSX → Coleco → Vectrex → VB → WonderSwan). Operator call.

---

## 2026-05-18 — Phase 4 slices E + F: memory inspector + per-game milestones

One pass closes the last two Phase 4 slices. **Slice E** ships a hex-view memory inspector (Esc → Memory inspector); **slice F** layers per-game milestone tracking on top — memory predicates persist in SQLite, evaluate live during gameplay, and fire toasts when rising-edge triggers. Closes Phase 4.

- **Shipped (core trait + libretro plumbing — slice E):** `oa_core::MemoryRegionId` enum (SaveRam / Rtc / SystemRam / VideoRam) with `as_str()` / `parse()` for stable serde tagging. `oa_core::Core::memory_region(&self, id) -> Option<&[u8]>` trait method (default-None impl so test stubs don't need to implement). `oa-libretro` resolves the new `retro_get_memory_data` + `retro_get_memory_size` symbols; `LibretroCore::memory_region` returns an `&[u8]` aliasing through the live core memory — same `&self`-tied lifetime pattern as `framebuffer`. Safety reasoning: libretro guarantees pointer + size are stable between load_game / unload_game, and the borrow only lives until the next `&mut Core` method, which we control.
- **Shipped (memory snapshot + Tauri command — slice E):** `MemorySnapshot { save_ram: Option<Vec<u8>>, rtc, system_ram, video_ram }` struct on AppState behind `Arc<Mutex<>>`. Emu thread copies live core memory into the snapshot after each `run_frame` — but gated on `need_snapshot = milestone_runtime.is_empty() || (any snapshot field is Some)`, so when nothing's polling the snapshot stays empty and we pay only a single mutex check. First poll seeds the snapshot, subsequent frames refresh. New `read_memory_region(region, offset, length)` Tauri command returns `{ region, available, totalSize, offset, bytes: Vec<u8> }`. Out-of-bounds reads return the available subrange (no error). `length = 0` is "to end of region."
- **Shipped (SQLite schema v4 + Milestone CRUD — slice F):** New `milestones` table — `(id, game_id FK→games CASCADE, name, description, region TEXT tag, offset, width, op TEXT tag, target i64, edge_only bool, triggered_at_unix_ms i64 nullable)`. `migrate_v3_to_v4` is additive (CREATE TABLE IF NOT EXISTS) so re-runs are idempotent. `library_db::Milestone` type with full serde. Six methods: `list_milestones(game_id)` / `add_milestone(&Milestone) -> id` / `update_milestone(&Milestone)` / `delete_milestone(id)` / `mark_milestone_triggered(id, ts)` (with `WHERE triggered_at_unix_ms IS NULL` guard so the timestamp doesn't drift on subsequent triggers) / `reset_milestone_progress(id)` (clears the stamp). 1 new unit test rounds-trips all six methods + the trigger-guard semantics.
- **Shipped (runtime evaluator + emu thread integration — slice F):** Two new EmuCommand variants — `LoadMilestones(Vec<Milestone>)` parses the DB rows into `MilestoneRuntime` structs (with `MilestoneOp` enum + parsed `MemoryRegionId`); skips malformed rows with a warn instead of crashing the emu thread. `ClearMilestones` resets. State slots: `milestone_runtime: Vec<MilestoneRuntime>` + `milestone_prev_true: Vec<bool>` (parallel-indexed for edge-detection). Frame body's normal-play branch reads `read_memory_le(core, region, offset, width)` for each runtime entry, evaluates `op.eval(value, target)`, compares against `prev_true` for rising edges. On `should_fire`: emits `oa://milestone-triggered` event with `{ id, name, triggeredAtUnixMs }` payload, calls `app_handle.try_state::<LibraryDb>().mark_milestone_triggered(id, now_ms)` (the in-process LibraryDb singleton), fires a 🏆-prefixed success toast, sets `already_triggered = true` for edge-only milestones so they don't re-fire mid-session. LoadRom / UnloadRom clear both the snapshot and the runtime evaluator.
- **Shipped (Tauri commands — slice F):** 6 new — `list_milestones(game_id)` / `add_milestone(milestone)` / `update_milestone(milestone)` / `delete_milestone(id)` / `reset_milestone_progress(id)` (all CRUD wrappers) + `arm_milestones(game_id)` which reads SQLite + sends `LoadMilestones` to the emu thread (returns the loaded count for the toast log).
- **Shipped (frontend — QuickSettings memory inspector view, slice E):** `QuickView` extended to include `"memory"`. New "Memory inspector…" ActionRow (hint = "dev / power user"). `MemoryInspectorPanel` component renders the active region's bytes 8 per row in a black-bg monospace block with offset column. Region picker dropdown + hex/decimal offset input (accepts `0x...`, hex with a-f, or plain decimal) + Prev/Next page buttons (256-byte windows). Auto-polls `read_memory_region` at 4 Hz while the view is open; cleared when leaving.
- **Shipped (frontend — PerGameSettingsDrawer Milestones tab + auto-arm, slice F):** New "Milestones" tab on the per-game drawer. List view shows each milestone with a triggered chip (when `triggeredAtUnixMs != null`) + region/offset/width/op/target/edge-mode summary line + per-row Edit / Reset / Delete buttons. "Add milestone" button at the bottom opens an inline `MilestoneEditor` (name, description, region picker, offset input with hex parsing, width picker (u8/u16/u32), op picker (eq/neq/gt/lt/geq/leq), target input, edge-trigger checkbox). Drawer subscribes to `oa://milestone-triggered` and refreshes the list when an event fires (so the "Triggered" chip lights up live without re-opening). `App.handleLaunch` now calls `arm_milestones(gameId)` after every successful launch so the runtime evaluator picks up edits between launches.
- **Architecture invariant — runtime evaluator never blocks emulation:** the snapshot Mutex is taken briefly to write (cheap clone of opt<Vec<u8>>x4); reads happen on the worker thread, never holding the lock across frame boundaries. Milestone evaluation reads memory DIRECTLY from core, not through the snapshot, so the emu thread doesn't depend on the snapshot lock at all when no inspector view is open.
- **Architecture invariant — emit AND stamp:** when a milestone fires, the event payload carries `triggeredAtUnixMs` so the frontend can update UI immediately, AND the same timestamp gets stamped into SQLite via the shared LibraryDb handle. The two paths are independent (the toast fires even if the DB write fails, and the DB stamp survives a missed event). Reset zeroes only the DB stamp — `already_triggered` is reset by re-arming after a launch.
- **Validation:** `cargo test --workspace` 96/96 green — gained 1 oa-shell test on top of slice D's 95 (66 oa-shell + 15 oa-render + 14 oa-savestate + 1 oa-libretro doc-test). `cargo check -p oa-core -p oa-libretro -p oa-shell` clean. Frontend `npm run build`: 53 modules (unchanged), 48.12 kB CSS (+0.54 from slice D 47.58 — MemoryInspectorPanel + MilestonesTab/Editor styles), 296.87 kB JS (+16.61 from slice D 280.26 — the two new components + state machine + Tauri command bindings), 970 ms. TypeScript `--noEmit` clean.
- **Almost:** Operator runtime validation. (a) Launch any game; Esc → Memory inspector → switch region picker between System RAM / Save RAM → confirm bytes appear and change frame-to-frame as the game updates state. (b) Enter `0x1234` into offset input → confirm hex view jumps. Test decimal input (`4660`), 0x-prefixed (`0x1234`), and bare hex with letters (`1234abcd`). (c) Right-click a tile in the library → Game properties → Milestones tab → Add milestone → name "Test", region "system_ram", offset 0, width "u8", op "neq", target 0 → Add. (d) Launch the game; once a few frames pass + memory[0] becomes non-zero, confirm the 🏆-prefixed toast fires + the milestone row's Triggered chip lights up live (while the drawer is open). (e) Reset progress button clears the chip; the milestone re-arms on next launch. (f) Reload a different ROM → confirm in-flight milestone runtime is cleared (no spurious triggers from the new game's memory).
- **Phase 4 status:** ✅ **closed** as of this slice. All six planned slices shipped: A (rewind engine), B (scrubbing UI), C (TAS recording + replay), D (PNG frame video capture), E (memory inspector), F (per-game milestones). Slice D-2 (ffmpeg WebM conversion) remains on the ROADMAP as a polish follow-up. The differentiator-features phase the ROADMAP promised for 4-6 weeks landed in one day end-to-end.
- **Next:** Operator validates the end-to-end Phase 4 surface. After that the natural fork: **system #5 (Atari 7800 / N64)** — Phase 4 is system-agnostic so the new system inherits all of it; **Phase 3 slice B-2 + C bundle** — renderer-side bezel/Phosphor/TOML cleanup; **Phase 5 PCE-CD bringup** (Mednafen full + CHD); or **slice D-2** for ffmpeg-driven WebM conversion (polish, low scope).

---

## 2026-05-18 — Phase 4 slice D: frame-by-frame video capture (PNG sequence + manifest)

Esc → "Video capture…" → "Start capture" → play → "Stop & save". A directory of `frame_000000.png`, `frame_000001.png`, … lands under `appDataDir/clips/<rom-stem>/<timestamp>[-name]/`, with `manifest.json` recording fps / system / dimensions / drop count. The roadmap entry says "WebM" but slice D ships the foundation as a PNG sequence — lossless, no new native deps, convertible to WebM in one ffmpeg command. The actual WebM conversion is queued as slice D-2 to keep this slice's scope tight and the CI matrix dep-free.

- **Shipped (`apps/oa-shell/src/video_capture.rs`, new):** `VideoCaptureWorker` with bounded `mpsc::sync_channel<VideoFrame>(30)` + spawned encoder thread that pulls frames and writes each as a PNG via the `png` crate (already a workspace dep used by save-state thumbnails — no new deps). `VideoFrame { frame_idx, width, height, rgba: Vec<u8> }`. `try_submit` is non-blocking — when the channel's full (encoder can't keep up), the frame is dropped + `dropped_frame_count` increments. `stop_and_finalize(system_id, rom_stem, display_name, fps, first_w, first_h, discard)` drops the sender (worker drains remaining frames + exits), joins, writes `manifest.json` (or removes the dir if `discard = true`). `VideoManifest` is the serde struct serialized to disk. 3 new unit tests: round-trip-writes-png-files-and-manifest, discard-removes-directory, channel-overflow-drops-frames.
- **Shipped (emu thread integration):** Two new EmuCommand variants — `StartVideoCapture { display_name }`, `StopVideoCapture { discard }`. State slots: `video_capture: Option<VideoCaptureWorker>`, `video_first_size: (u32, u32)`, `video_display_name: String`, `video_frames_submitted: u64`. Frame body's NORMAL forward-play branch + TAS REPLAY branch both submit framebuffer to the worker after `run_frame` — replay-while-capturing produces a canonical video of a deterministic TAS playback, which is a power-user dream feature. SCRUB and HOLD-BACKSPACE rewind branches DO NOT submit (they'd produce duplicate / out-of-order frames). LoadRom + UnloadRom finalize-with-discard any in-flight capture (clip is bound to the current ROM state). `SharedVideoState { capturing, frame_count, dropped_frame_count, display_name, clip_dir }` published via `Arc<Mutex<>>` same pattern as `SharedRewindState` / `SharedTasState`; updated on every 30th submitted frame + on every state transition.
- **Shipped (Tauri commands, 6 new):** `get_video_state` (reads the published state for the UI), `start_video_capture(displayName)`, `stop_video_capture(discard)`, `list_video_clips(romPath)` (returns sorted-newest-first list of clip metadata from each `manifest.json`), `delete_video_clip(clipDir)` (with safety check — refuses to delete dirs without a manifest.json), `open_video_clip_folder(clipDir)` (cross-platform: Explorer / Finder / xdg-open). All registered in `invoke_handler!`.
- **Shipped (`QuickSettings.tsx` — fourth view):** Actions list gains "Video capture…" row with live hint ("record video clip" / "capturing · N frames" / "… (M dropped)"). Clicking enters the new `VideoPanel`:
  - **Idle:** "New capture" panel (optional label input + Enter-to-submit + Start button) above a scrollable clips list. Each row: display name / `formatDuration` / frame count / dimensions / drop count (if any) / `formatTimestamp` + open-folder button (📁) + delete (✕). Empty-state hint when no clips exist. Footer note explains the PNG-sequence + manifest format with an ffmpeg conversion hint.
  - **Capturing:** Accent-bordered banner explaining the per-frame write + drop semantics + Discard + "Stop & save" buttons.
  - Back button disabled while capturing. Polls `get_video_state` at 4 Hz while the panel is open.
- **Validation:** `cargo test --workspace` 95/95 green — gained 3 oa-shell tests on top of slice C's 92 (65 oa-shell + 15 oa-render + 14 oa-savestate + 1 oa-libretro doc-test). `cargo check -p oa-shell` clean. Frontend `npm run build`: 53 modules (unchanged), 47.58 kB CSS (unchanged — VideoPanel reuses existing utility classes), 280.26 kB JS (+6.23 from slice C 274.03 for VideoPanel + state machine + types), 932 ms. TypeScript `--noEmit` clean.
- **Almost:** Operator runtime validation. (a) Launch any game; Esc → Video capture → enter label → Start → play 5-10 s → Stop & save; confirm toast "Saved N frames"; reopen panel → confirm clip appears in the list with correct duration + dimensions. (b) Open clip folder (📁 button) → confirm Explorer/Finder opens the directory containing `frame_000000.png` ... + `manifest.json`. (c) Run `ffmpeg -framerate <fps> -i frame_%06d.png -c:v libvpx-vp9 -b:v 2M out.webm` from the clip dir → confirm the WebM plays back the gameplay. (d) During capture, watch the hint — if drops appear, the PNG encoder is saturated for this resolution (PCE/SNES at 60 fps should be drop-free on any modern CPU). (e) Try capturing during a TAS replay — confirm the resulting clip shows the replay playback. (f) LoadRom mid-capture → confirm "Video capture discarded — new ROM loaded" toast + the in-progress clip directory is removed.
- **Why PNG sequence over WebM today:** Pulling in a real video encoder is a big build-dep ask. `vpx-encode` needs libvpx + CMake + (on Windows) MSVC build of libvpx; cross-platform CI would need explicit dep install steps. `ffmpeg-sidecar` is cleaner but assumes ffmpeg in PATH (or downloads it at runtime). PNG sequence ships now with zero new deps, is lossless (good for video editing), and the conversion to WebM/MP4 is a one-line ffmpeg invocation. Slice D-2 will wrap that invocation behind a "Convert to WebM" button in the clips list when we're ready to take on the ffmpeg-detection UX.
- **Architecture invariant — frames captured BEFORE the renderer:** the video frames come from `core.framebuffer().pixels` (the raw libretro framebuffer) BEFORE the shader chain + scaling + bezel composite the renderer does. This is deliberate: the captured PNG sequence is the cleanest possible source for editing / re-encoding, with shader effects added as a post-process step if wanted (vs. baked into a video file that can't be undone).
- **Phase 4 status:** 🟨 in progress. Slices A + B + C + D ✅. **Remaining slices:** E (memory inspector — dev panel against retro_memory_data), F (per-game milestone tracking — reads memory regions per-snapshot, fires events). Slice D-2 (ffmpeg-driven WebM conversion) added to the ROADMAP as a polish follow-up.
- **Next:** Operator validates slice D. After that: **slice E (memory inspector)** is the natural next — independent + low-cost, and unlocks slice F. Alternatively: pause Phase 4 to clear Phase 3 slice B-2 / C (bezel + Phosphor composite + TOML preset) since those are renderer-side and don't conflict with Phase 4 code.

---

## 2026-05-18 — Phase 4 slice C: TAS recording + deterministic replay

The differentiator feature the rewind ring has been quietly preparing the ground for. Press Esc → "TAS recording…" → "Start recording", play through a section, "Stop & save". The file lives in `appDataDir/tas/<rom-stem>/<timestamp>.tas`. Re-open the panel → click any recording → click "Replay" — input frames dispatch from the recording at frame-perfect cadence, user input is suppressed, the game replays deterministically. Works across all 4 systems (PCE / Lynx / NES / SNES) because the recording captures libretro joypad bits (not per-system native bits) — same file replays correctly regardless of which per-system binding profile is active.

- **Shipped (`oa-savestate::tas` module, new):** Hand-rolled binary file format. Header: 5-byte magic `OATAS` + u16 LE version (= 1). Body: zstd-compressed (level 3) payload of `TasHeader` (system_id / core_file_name / rom_sha1_hex / fps / recorded_at_unix_ms / display_name / frame_count) + `initial_state: Vec<u8>` (the `Core::save_state` blob at record-start) + `Vec<TasInputFrame>` (each frame = 4× u32 LE = 16 bytes for port0/1/2/3). `TasRecording::{new, write_to, read_from, read_header_only}` — header-only read decompresses through the header block then stops, useful for the recordings-list UI without paying the full decode cost. 5 new unit tests: round-trip through disk, header-only read, bad-magic rejection, bad-version rejection, empty-input-frames round-trip. Cap on byte-field length (64 MiB) so a corrupt file can't try to allocate u32::MAX-sized Vec.
- **Shipped (`apps/oa-shell/src/main.rs` — state machine):** Four new EmuCommand variants — `StartTasRecording { display_name }`, `StopTasRecording { discard }`, `StartTasReplay(Box<TasRecording>)` (boxed to keep the variant small; `TasRecording` carries an unbounded `Vec<u8>` initial state), `StopTasReplay`. Emu thread state slots: `tas_recording: Option<TasRecording>`, `tas_replay: Option<TasRecording>`, `tas_replay_current_frame: u64`. Frame body now branches FOUR ways: SCRUB (slice B), REPLAY (new — dispatch recorded inputs, no user poll, no capture; auto-stops when `current_frame >= input_frames.len()` and emits `oa://tas-replay-complete`), HOLD-BACKSPACE rewind (slice A, now gated off during recording AND replay for clean v1 semantics — see DECISIONS for the v2 plan), NORMAL forward play (existing — but now also pushes the dispatched libretro-shape input bits into `tas_recording.input_frames` when recording). LoadRom + UnloadRom discard any in-progress recording with a warning toast. `set_input_remapped` helper retired — the remap is now inlined at the dispatch site so we can capture the libretro-shape bits for the recording without computing them twice.
- **Shipped (ROM SHA-1 stamping):** LoadRom path computes SHA-1 of the ROM bytes for Bytes-source loads (HuCard / cart — the typical TAS use case) and stores as `current_rom_sha1_hex` (uppercase hex). Path-source loads (CD `.cue` / `.chd`) leave it empty — hashing a 600 MB CHD at every load isn't worth the latency. Stamped into the recording's `TasHeader.rom_sha1_hex` field. Replay enforces SOFT — mismatching hash emits a warn-toast but proceeds (some cores tolerate close-enough ROMs).
- **Shipped (`SharedTasState` + Tauri commands):** New `Arc<Mutex<SharedTasState>>` on AppState, mirroring the `SharedRewindState` pattern. Fields: `mode: TasMode` (idle/recording/replaying), `frame: u64`, `total_frames: u64`, `display_name: String`. Emu thread updates on every state transition + every 30 frames during recording/replay (cheap mutex; not even close to contended). Seven new Tauri commands registered: `get_tas_state` / `start_tas_recording` / `stop_tas_recording` / `start_tas_replay` / `stop_tas_replay` / `list_tas_recordings(romPath)` / `delete_tas_recording`. `start_tas_replay` decodes the file server-side so malformed files produce a clean error instead of crashing the emu thread; `list_tas_recordings` reads via `read_header_only` for cheap directory scans.
- **Shipped (`frontend/src/components/QuickSettings.tsx`):** Third view added — `"tas"`. Actions list gains a TAS recording row showing live hint ("record or replay" / "recording · N frames" / "replaying · X / Y"). Clicking enters `TasPanel`. The panel renders three sub-states off `tasState.mode`:
  - **Idle:** "New recording" panel (optional label input — Enter submits — + Start button) above a scrollable per-game recordings list. Each row: display name / `formatDuration` / frame count / `formatTimestamp` + Replay button + Delete (✕) button. Empty-state hint when no recordings exist for this game.
  - **Recording:** Accent-bordered banner ("Recording inputs frame-by-frame…") + Discard + "Stop & save" buttons side-by-side.
  - **Replaying:** Progress bar driven by `frame() / totalFrames()` + a "Stop replay" button. Polled at 4 Hz so the progress bar advances smoothly.
  
  Back button disabled while recording or replaying so the user has to explicitly resolve the operation. TasPanel polls `get_tas_state` at 4 Hz only while the panel is open (cleared on view change / overlay close).
- **Validation:** `cargo test --workspace` 92/92 green — gained 5 oa-savestate tests on top of slice B's 87 (62 oa-shell + 15 oa-render + **14 oa-savestate** + 1 oa-libretro doc-test). `cargo check -p oa-savestate -p oa-shell` clean. Frontend `npm run build`: 53 modules (unchanged), 47.58 kB CSS (+0.31 from slice B 47.27), 274.03 kB JS (+9.28 from slice B 264.75 — TasPanel + state machine + state types), 2.58 s. TypeScript `--noEmit` clean.
- **Almost:** Operator runtime validation. (a) Launch any game; Esc → TAS recording → enter label → Start recording → play 5-10s → Stop & save; confirm toast "Saved N frames"; reopen panel → confirm recording appears in the list with correct duration. (b) Click Replay on that recording → confirm the game restarts from the recording's initial state and replays the exact inputs you fed it. (c) During replay, confirm gamepad/keyboard input has NO effect — game ignores you. (d) Click Stop replay mid-replay → confirm forward play resumes from the current replay frame (so you take over from there). (e) Recording then immediately reload a different ROM → confirm the toast "TAS recording discarded — new ROM loaded" appears. (f) Try replaying a recording made on a different ROM (rename a .pce, replay an old recording) → confirm a "ROM hash differs — replay may desync" warn-toast (and replay still runs, but the desync is your fault). (g) Discard button mid-recording → confirm no file is written.
- **Architecture invariant — libretro-shape bits in the recording:** the recording stores `bindings::to_libretro_bits(system_id, polled.buttons)`, not the per-system native bits the keyboard/gamepad mapping produces. This means a recording made on TG-16 with "Z=I, X=II" plays back faithfully even if the user later remaps to "Z=II, X=I" — the recording is a record of what the CORE received, not what the keyboard did. It also means recordings ARE cross-core safe within a system (e.g. a Snes9x recording will play on bsnes if you swap cores) since libretro joypad bits are stable across cores for the same system.
- **Phase 4 status:** 🟨 in progress. Slices A + B + C all ✅. **Remaining slices:** D (frame-by-frame WebM export — encoder thread + cpal-driven audio mux), E (memory inspector — dev panel against retro_memory_data), F (per-game milestone tracking — reads memory regions per-snapshot, fires events).
- **Next:** Operator validates slice C. After that the fork: **slice D** (WebM export — natural pair with TAS since you can now both record-inputs AND record-video for the same playthrough), **slice E** (memory inspector — independent + foundation for slice F), or pause Phase 4 to clear Phase 3 slice B-2 + C (bezel + Phosphor composite + TOML preset format).

---

## 2026-05-18 — Phase 4 slice B: rewind scrubbing UI in the Quick Settings overlay

Slice A's `RewindRing` gets its proper interactive surface. The Quick Settings overlay (Esc during gameplay) now has a "Rewind…" action that swaps the card into a timeline-scrubber mode: drag the strip to preview any captured frame, click "Resume from here" to commit (truncates the ring above the chosen point — the future is rewritten), or Cancel to restore the live edge with no history lost.

- **Shipped (`oa-savestate`):** `RewindRing` gains two new methods. `peek_at(steps_back)` — non-destructive index from the newest (0 = newest, len-1 = oldest); used during drag to preview without consuming snapshots. `truncate_above(steps_back)` — destructive drop of every snapshot newer than the target; used on commit. Two new unit tests cover index correctness + out-of-bounds + the no-op cases.
- **Shipped (`apps/oa-shell/src/main.rs` — emu thread state machine):** Three new `EmuCommand` variants — `StartRewindScrub` / `SetRewindScrubPosition { steps_back: u32 }` / `EndRewindScrub { commit: bool }`. Emu thread holds `scrubbing: bool` + `scrub_position: u32` + `scrub_dirty: bool` next to the slice-A ring. Frame body branches three ways now: SCRUB (peek at position, load_state, run_frame, no input, no capture), HOLD-BACKSPACE rewind (slice A), NORMAL forward play (slice A). Scrub mode pauses both forward play AND capture so the ring stays frozen while the user drags. `scrub_dirty` flag means we only re-apply the peek+load when the position actually changes (not every frame at 60 Hz). LoadRom / UnloadRom / SetRewindConfig (when toggling off) all clear scrubbing state alongside the ring clear.
- **Shipped (live ring stats publication):** New `SharedRewindState` struct — `{ enabled, snapshot_count, byte_size, capture_interval_frames, fps, scrubbing, scrub_position }`. Lives in `Arc<Mutex<SharedRewindState>>` on `AppState`. Emu thread writes after every capture / pop / scrub op via a `publish_rewind_state` closure that captures the writer side; Tauri commands read via `state.rewind_state.lock()`. Cheap Mutex — uncontended in practice. Plumbed through `main` → `setup_two_window` / `setup_single_window` → `run_emu_render` so both shell modes share the same channel.
- **Shipped (Tauri commands):** `get_rewind_state` returns the current `SharedRewindState` (used to hydrate the overlay's ring stats on open). `start_rewind_scrub` / `set_rewind_scrub_position(stepsBack)` / `end_rewind_scrub(commit)` map 1:1 to the new EmuCommand variants. All registered in `invoke_handler!`.
- **Shipped (`frontend/src/components/QuickSettings.tsx`):** Two-mode card with `view: "actions" | "rewind"`. Actions view now has a "Rewind…" `ActionRow` at the top showing live hint text ("Xs · N snaps" / "off" / "no history") computed from the polled `RewindState`. Clicking enters rewind view via `start_rewind_scrub`, which swaps the card to a new `RewindScrubber` component. The scrubber renders a 48px-tall horizontal strip — left=oldest, right=live — with an accent-glow vertical thumb, a filled bar from left to thumb (visualizing the "rewound past" span), endpoint labels, and a status row showing both `position` (e.g. `-1.23s`) and `total seconds buffered`. PointerEvents handle drag (`setPointerCapture` for reliable cross-element tracking; `xToStepsBack` maps client X to a clamped `steps_back` value; throttled-to-frame `set_rewind_scrub_position` invokes). Keyboard navigation: ←/→ step by 1 frame, Shift+arrow steps by 10, Home jumps to oldest, End to newest. ARIA `role="slider"` + valuemin/max/now for assistive tech. Below the strip: a Cancel button (restores live edge, returns to actions view) + a "Resume from here" button (truncates ring above current position, commits, closes overlay). Commit button is disabled when `scrubPosition === 0` (the live edge — no change to commit). Closing the overlay (Esc, backdrop click) while in rewind view auto-cancels server-side first so the emu thread doesn't get stranded in scrub mode with no UI driving it.
- **Validation:** `cargo test --workspace` 87/87 green — gained 2 oa-savestate tests (peek_at + truncate_above) on top of slice A's 85. `cargo check -p oa-savestate -p oa-shell` clean. Frontend `npm run build`: 53 modules (unchanged), 47.27 kB CSS (+0.81 from slice-A 46.46 — scrubber styles + two-mode card chrome), 264.75 kB JS (+5.79 — RewindScrubber component + state machine + pointer event handling), 1.50 s. TypeScript `--noEmit` clean.
- **Almost:** Operator runtime validation. (a) Enable rewind in OA Settings → Gameplay; launch any of the 4 systems (PCE / Lynx / NES / SNES — same code path); play 10+ seconds; Esc; confirm the Rewind action shows "X.Xs · N snaps" hint. (b) Click Rewind; confirm the card transitions to the timeline view; confirm the thumb starts at the right edge ("Live edge" label on the left); drag the thumb leftward; confirm the game framebuffer behind the dimmed backdrop reflects each new position. (c) Use ←/→ for fine stepping + Shift+← for fast scrubbing. (d) Click "Resume from here"; confirm the overlay closes + gameplay resumes from the chosen point + the ring's count drops (re-open overlay, check "N snaps" decreased). (e) Reopen scrubber + scrub back + click Cancel; confirm gameplay resumes from where the overlay was opened with NO change to the ring. (f) Scrub back + close via Esc; confirm same as Cancel (overlay-close = auto-cancel). (g) Disable rewind in Settings → Gameplay; reopen overlay; confirm the Rewind action shows "off" + is disabled.
- **Architecture invariant — destructive vs non-destructive scrub:** the scrubber uses `peek_at` for preview (non-destructive) and `truncate_above` for commit (destructive). Cancel just restores `peek_back()` (the live edge that hasn't moved). This is the same conceptual shape TAS replay (slice C) will need: preview a recording position with peek_at, scrub the input timeline, then commit by truncating.
- **Phase 4 status:** 🟨 in progress. Slices A + B both ✅. **Remaining slices:** C (TAS recording + deterministic replay — shares the snapshot format), D (frame-by-frame WebM export), E (memory inspector — dev panel against retro_memory_data), F (per-game milestone tracking — reads memory regions per-snapshot, fires events).
- **Next:** Operator validates slice B. After that the natural fork: **slice C** (TAS recording — the snapshot ring becomes the foundation for "record + replay" since recording a TAS effectively is "deterministic forward play that captures snapshots + input deltas"), OR **slice E** (memory inspector — independent of the rewind work; lowest scope), OR pause Phase 4 to clear Phase 3 slice B-2 + C (bezel + Phosphor composite + TOML preset format) since those are renderer-side and don't conflict with Phase 4 code.

---

## 2026-05-18 — Phase 4 opens: rewind engine (slice A) — `RewindRing` + hold-Backspace + three-tier inheritance

Phase 3's slices B-2 / C / D stay queued; Phase 4 opens on the differentiator features the ROADMAP has been promising. Slice A is the rewind engine — the substrate every subsequent slice (B scrubbing UI, C TAS recording, F per-game milestones) builds on. System-agnostic by construction: works across all 4 live cores (tg16 / Lynx / NES / SNES) without per-core code because the ring stores opaque `Core::save_state` blobs.

- **Shipped (`oa-savestate` crate, was a 5-line scaffold):** `RewindRing` — byte-bounded `VecDeque<Vec<u8>>` with LIFO `pop_back` + greedy front-eviction. `set_max_bytes(usize)` adjusts the cap live (drops oldest until under). `peek_back() / len() / byte_size() / is_empty() / clear()` round out the surface. Eviction policy retains at least 1 snapshot even if a single push busts the cap — losing all history to a momentary squeeze is worse than briefly exceeding. `seconds_held(fps, interval)` is the unit-aware accessor for the UI. `RewindConfig { enabled, capture_interval_frames, max_bytes }` is the runtime shape; defaults: off, 6 frames (~100 ms at 60 fps), 64 MiB. 7 new unit tests cover the entire surface.
- **Shipped (Rust shell wiring — `apps/oa-shell/src/main.rs`):** New `EmuCommand::SetRewindConfig(RewindConfig)` variant. Emu thread holds `rewind_ring: RewindRing` + `rewind_config: RewindConfig` next to the existing F5/F8 slot state. Frame body refactored: when Backspace is held AND `rewind_config.enabled` AND the ring has snapshots, pop the newest + `load_state` it + run exactly one forward frame to repaint the framebuffer (libretro `cb_video_refresh` only fires from `retro_run`). Input is intentionally NOT dispatched during rewind — user is steering history, not the game. When not rewinding, normal forward play, with a `save_state` capture every `capture_interval_frames` after `run_frame`. Ring clears on every `LoadRom` + `UnloadRom` (cross-core snapshot replay would corrupt the new core). New `set_rewind_config(enabled, captureIntervalFrames, maxMegabytes)` Tauri command + handler that constructs the `RewindConfig` server-side from human-readable units. Command registered in `invoke_handler!`.
- **Shipped (settings inheritance — three tiers):** `SystemSettings` (`appDataDir/systems/<id>.json`) gained `rewind_enabled` / `rewind_capture_interval_frames` / `rewind_buffer_megabytes`. `GameOverrides` (`games.overrides_json`) gained the same three fields. Both honor the existing `#[serde(skip_serializing_if = "Option::is_none")]` sparse-write policy + the existing `is_empty` check in `set_game_overrides`. Existing roundtrip tests updated.
- **Shipped (frontend wiring):** `settings/store.ts` gained `rewindEnabled` / `rewindCaptureIntervalFrames` / `rewindBufferMegabytes` signals (persisted in `oa.settings.v1`) + a `createEffect` that pushes `set_rewind_config` on any OA-wide change. `App.handleLaunch` extends the inheritance resolution from "shader + scaling + window + monitor" to include rewind config — per-game → per-system → OA-wide, pushed via `set_rewind_config` in the same `Promise.all` as the rest. `App.handleUnload` reverts to OA-wide.
- **Shipped (UI surfaces — three tabs):** New **"Gameplay" tab** on `SettingsPage.tsx` (OA-wide) with three controls: Enable rewind checkbox, Capture interval select (1 / 2 / 3 / 6 / 10 / 15 / 30 frames with ms equivalence labels), Buffer cap select (8 / 16 / 32 / 64 / 128 / 256 / 512 MB). New **"Rewind" tab** on `PerSystemSettingsPage.tsx` with the same three controls wrapped in `<SettingRow>` showing the OA-wide inherited value (with line-through when overridden). New **"Rewind" tab** on `PerGameSettingsDrawer.tsx` with the same three controls + the two-level inheritance chain (`Per-system` chip vs `OA default` chip).
- **Validation:** `cargo test --workspace` 85/85 green — gained 7 oa-savestate tests on top of the prior 78 (62 oa-shell + 15 oa-render + 1 oa-libretro doc-test). `cargo check -p oa-shell -p oa-savestate` clean. Frontend `npm run build`: 53 modules (unchanged), 46.46 kB CSS (+0.21 from baseline 46.25 — three new tab bodies), 258.96 kB JS (+10.77 — rewind signals + inheritance resolvers + three tab UIs), 814 ms cold build. TypeScript `--noEmit` clean. Stale `oa-libretro` doctest from the prior FCEUmm fix updated (the `name` param was added to `load_rom` but the doctest example wasn't bumped — now compiles).
- **Almost:** Operator runtime validation. Three-tier flow to walk: (a) OA Settings → Gameplay → Enable rewind + accept defaults; launch any game; play 30 s; hold Backspace; confirm visual reverse playback at ~5× speed (six frames backwards per render frame at default interval=6); release; confirm forward play resumes. (b) Open a tg16 game; PerGameSettingsDrawer → Rewind → set Capture interval = 1; confirm SettingRow shows OA-default chip with line-through; relaunch the game; hold Backspace; confirm the rewind is now ~1 frame back per render frame (10×+ smoother + far heavier RAM use — visible if you check Task Manager's process RAM). (c) PerSystemSettingsPage → snes Rewind tab → Enable: Off override; confirm any SNES launch ignores OA-wide enable. (d) Watch `cargo tauri dev` log for the "rewind reconfigured" line — confirms the launch-path push reaches the emu thread.
- **Architecture invariant — opaque snapshots:** the ring stores `Core::save_state` blobs without parsing them. Adding system #5 (Atari 7800, N64, anything) costs zero rewind-side code; the trait already has the methods. Same shape as save states (F5/F8 slots) — those use the same `save_state` → bytes machinery, just stamped on disk per slot instead of held in a memory ring.
- **Phase 4 status:** 🟨 in progress. Slice A ✅. **Remaining slices:** B (rewind scrubbing UI — visual timeline in Quick Settings, click/drag to scrub), C (TAS recording + deterministic replay — shares the snapshot format), D (frame-by-frame WebM export), E (memory inspector — dev panel against retro_memory_data), F (per-game milestone tracking — reads memory regions per-snapshot, fires events).
- **Next:** Operator validates slice A across all 4 systems (PCE / Lynx / NES / SNES — each has different `serialize_size` so the byte cap math + capture cost varies). After that the natural fork: **slice B** (the visual scrubbing UI — pairs directly with what's shipped today since the ring already exposes `peek_back` + `len` for thumbnail rendering), OR **slice C** (TAS recording — bigger payoff but more design surface — needs an on-disk `.tas` format + frame-perfect input dispatch).

---

## 2026-05-18 — FCEUmm bringup: `GET_GAME_INFO_EXT` fully populated (NES live)

Operator validation of NES (FCEUmm) shook out a libretro env-callback bug that had been latent since the very first PCE Fast bringup. The crash trace through three iterations is worth recording — it's the kind of cross-core fragility that bites every time a new libretro core's quirks differ from the original tested one.

- **Iteration 0 — what was shipped:** `GET_GAME_INFO_EXT` (env cmd 66) handler returned `true` with a struct that had `s.pending_name = "rom"` (the State::new() default, never repopulated from the real ROM) and `full_path`/`archive_path`/`archive_file`/`dir`/`meta` all set to `std::ptr::null()`. PCE Fast + Mednafen Lynx never trip this code path (they always read from `info.data` directly + ignore the env_ext surface) so the bug was dormant for two systems.
- **Crash 1 — FCEUmm 8 Eyes (USA).nes:** `STATUS_ACCESS_VIOLATION` immediately after env cmd 66 fires during `retro_load_game`. FCEUmm dereferences one of the NULL string pointers without null-checking. Spec says NULL is valid for these fields; FCEUmm disagrees.
- **Fix attempt #1 — decline `SET_CONTENT_INFO_OVERRIDE` (cmd 65):** Hypothesis: cores only call `GET_GAME_INFO_EXT` after the frontend acks `SET_CONTENT_INFO_OVERRIDE`. **Wrong** — FCEUmm calls cmd 66 unconditionally. Identical crash on retry.
- **Fix attempt #2 — decline `GET_GAME_INFO_EXT` outright:** Spec says returning false means "frontend doesn't support this; core should use `info.data` from `retro_load_game` parameter." **FCEUmm doesn't honor the spec fallback** — it just returns `false` from `retro_load_game` instead of falling back. Result: `retro_load_game returned false` error in the toast + black framebuffer left over from the previous core.
- **Fix attempt #3 — fully populate info_ext with valid non-NULL pointers (shipped):** Added a process-static `EMPTY_CSTR: LazyLock<CString>` holding `""`. The `GET_GAME_INFO_EXT` handler now fills every string field with a valid pointer — `name` + `ext` from the real ROM identity, everything else (`full_path` / `archive_path` / `archive_file` / `dir` / `meta`) pointing at the empty CString. `data` + `size` + `file_in_archive` + `persistent_data` work as before. Cores doing unchecked `strlen`/`strstr`/`strcpy` get harmless empty-string behavior; cores that DO null-check get a benign empty-string result either way. **NES launches end-to-end with FCEUmm.** SNES (Snes9x) already worked because it follows the spec — but the same fix is now ready for any future core that lands with FCEUmm-class assumptions.
- **API change to support the fix:** `LibretroCore::load_rom` gained a `name: &str` parameter (the ROM stem, no extension). Cores read it via `info_ext->name` for save filenames + display. Caller (`apps/oa-shell/src/main.rs`) computes `sanitize_stem(&path)` BEFORE the load call and passes it through. State's `pending_name` is now written every `load_rom` instead of carrying the State::new() default `"rom"` forever. Two call sites updated (`EmuCommand::LoadRom` handler + the OA_ROM env-var bootstrap path).
- **Validation:** `cargo test --workspace` 78/78 still green (no test surface changes — the unit tests don't exercise the env dispatcher). `cargo check -p oa-libretro -p oa-shell` clean.
- **What this teaches:** when bringing up a new core, libretro env callbacks are the highest-risk surface. The pattern "return spec-compliant NULL pointers / unsupported flags" works for the carefully-written cores (Mednafen family) and fails for the practically-written ones (FCEUmm). Defensive coding — empty-string pointers instead of NULL, valid struct layouts instead of "supported=false" — is cheap and broadly tolerated. Worth applying preemptively to any future env-callback addition.
- **Reference memory pending:** capture this as a `reference_libretro_*` memory so the next FCEUmm-class core doesn't trigger the same iteration cycle.

---

## 2026-05-18 — Sidebar right-click: SystemContextMenu

Quick follow-up after the NES/SNES + sidebar refactor session. The earlier slice made clicking a system in the sidebar filter the library, but left the per-system settings page only reachable via the ⚙ button in GridControls — which is only visible AFTER you've already left-clicked into the system. This slice adds a right-click context menu on sidebar system entries that surfaces both "Show library" and "System settings…" directly.

- **Shipped (frontend):** `components/SystemContextMenu.tsx` (~95 LOC) — small popover anchored at click coords. Header shows the system's display name + live game count (filters `library.state.entries` by systemId + non-seed). Two action rows: "Show library" (navigates `{ kind: "system", id }` — same as left-click; duplicated here for discoverability), "System settings…" (navigates `{ kind: "system-settings", id }` — the value-add). Window-event listeners mirror TileContextMenu: Esc closes, click-outside via `closest("[data-system-context-root]")` closes. `LeftSidebar` Props extended with `onSystemContext?: (id, position) => void`; SystemItem gained an `onContextMenu` prop that preventDefaults the native browser menu and forwards `{ x: clientX, y: clientY }` upward. App.tsx mounts the menu with a new `systemContextFor: { id, position } | null` signal driving open state; wires onShowLibrary + onOpenSystemSettings to the existing `setCurrentView` navigation handlers.
- **Validation:** `cargo test --workspace` 78/78 green (no Rust touched). Frontend build: 53 modules (+1 for SystemContextMenu, was 52), 46.21 kB CSS (+0.03), 248.23 kB JS (+2.72), 785 ms. TypeScript `--noEmit` clean.
- **Almost:** Operator runtime validation — right-click each system entry in the sidebar; confirm the menu opens at the cursor with the system's display name + game count in the header; click "System settings…" → confirms it navigates to PerSystemSettingsPage (same target as the ⚙ button in GridControls). The native browser context menu should NOT appear (preventDefault'd). Esc + click-outside both close.
- **Out of scope this slice:** the remaining main-window.md §4 right-click items — Hide from sidebar, Pin to top of group, Edit system display (theme override). Those each need a layout-store field for persistence + matching UI; deferred to a polish slice if the user wants them. The two actions shipped today cover the load-bearing path (discoverability of per-system settings).
- **Next:** Operator validates. After that: Phase 3 slice B-2 + C bundle (bezel + Phosphor composite + TOML preset format), system #5 (Atari 7800 or N64), or Phase 4 differentiators.

---

## 2026-05-18 — NES + SNES onboarded; sidebar refactor (system click → filtered library)

Two systems online + a structural refactor of the sidebar Systems section. Adding NES (system #3) and SNES (system #4) takes ~the same amount of code as Lynx took for #2 — the registry-driven architecture from `feedback_multi_core_architecture_ready` continues to pay off. The refactor moves "click a system in the sidebar" from "render SystemPage (input bindings + placeholders)" to "filter the library to that system's games" — which is what users expect from a left-rail system list. The SystemPage component is retired; its bindings editor extracted into a reusable `<SystemBindingsEditor>` that now lives on the PerSystemSettingsPage Input tab (which was previously scaffold-only).

- **Shipped (NES, system #3):** `oa_core::SystemId::Nes` variant. `bindings.rs::nes` module — A=1<<8, B=1<<0, SELECT=1<<2, START=1<<3, d-pad 4-7 (8 bits total, identity libretro remap). `NES_BUTTONS` table preserves NES-native names. `default_nes_bindings()` uses the Z/X face-button convention (matches PCE muscle memory + NES Big Box era PC emulator mappings). Dispatch arms in `bit_for`, `buttons_for`, `to_libretro_bits`, `defaults_for`. `parse_system_id("nes" | "famicom")`. `default_core_dll_for_system("nes") → "fceumm_libretro.dll"` (Mesen swap is via PerSystemSettingsPage → Cores). Frontend: `SystemId` extends to include `"nes"`; `systemThemes.nes` registered with `.nes` / `.fds` / `.unf` / `.unif` extensions + 3/4 portrait tile aspect; `[data-system="nes"]` CSS block with `oklch(0.62 0.22 28)` crimson (Big Box NES palette). NSF audio-only files intentionally excluded from the scanner — they're chiptune music, not games (Decisions doc explains).
- **Shipped (SNES, system #4):** `oa_core::SystemId::Snes` variant. `bindings.rs::snes` module — 12 buttons total: A=1<<8, B=1<<0, X=1<<9, Y=1<<1 (diamond), L=1<<10, R=1<<11 (shoulders), SELECT=1<<2, START=1<<3, d-pad 4-7 (identity libretro remap). `SNES_BUTTONS` table + `snes_bit_for` + `snes_to_libretro_bits`. `default_snes_bindings()` uses the ZSNES-derived diamond layout (Z/X on lower-row B/A, A/S on upper-row Y/X, Q/W on L/R shoulders). All four dispatch arms. `parse_system_id("snes" | "super-famicom")`. `default_core_dll_for_system("snes") → "snes9x_libretro.dll"` (bsnes swap via PerSystemSettingsPage → Cores). Frontend: `SystemId` includes `"snes"`; `systemThemes.snes` with `.sfc` / `.smc` / `.fig` / `.swc` extensions + 4/3 landscape tile aspect; `[data-system="snes"]` CSS block with `oklch(0.62 0.18 270)` violet (SNES launch palette — cooler than Lynx's 290° purple so they read distinct side-by-side).
- **Shipped (sidebar refactor):** **Removed App.tsx Match clause for `kind:"system"`** — the Switch fallback (LibraryView) now takes the route, and `library/filter.ts::filterEntries`'s existing `view.kind === "system" → filter by id` logic finally takes effect. Removed `systemPage` memo; the old import deleted. **GridControls** gained an optional `onOpenSystemSettings` callback; **LibraryView** passes it through only when `currentView.kind === "system"` (with the system id captured via an IIFE to preserve type narrowing). The new ⚙ Settings button sits at the front of GridControls' actions cluster and navigates to `{ kind: "system-settings", id }`. **Bindings editor extracted** from the old SystemPage into `components/SystemBindingsEditor.tsx` (~250 LOC) — same capture-key + capture-pad + applyBinding logic, presented as a self-contained article. **PerSystemSettingsPage Input tab** now embeds `<SystemBindingsEditor systemId={props.systemId} />` (replaces the previous scaffold placeholder + "Back to system" button). **`components/SystemPage.tsx` deleted** — the file's content is fully covered by the filtered library + the bindings editor + the per-system settings page's other tabs (Display / Audio / Cores / Shaders / Theme).
- **Validation:** `cargo test --workspace` 78/78 green (62 oa-shell — was 59, +3 NES/SNES + 15 oa-render + 1 oa-libretro doc-test). The 3 new oa-shell tests: `defaults_cover_every_nes_button`, `defaults_cover_every_snes_button`, `nes_and_snes_remap_is_identity` (locks the bit layout for both systems — same defense as Lynx's identity-remap test). Frontend build: 52 modules (was 53 — net delta: -1 for SystemPage delete + 1 for SystemBindingsEditor + … the count shifts), 46.18 kB CSS (-0.07 from slice 3.B's 46.25 — new theme blocks add +0.45, removed SystemPage inline styles save ~0.52), 245.51 kB JS (was 248.19; -2.68 — SystemPage was heavier than SystemBindingsEditor since it carried 3 placeholder articles), 883 ms. The extended bindings-defaults test (now iterates `defaults_for` over every registered system) catches any new default key/pad name that doesn't resolve through `device_query` / `gilrs` — would have flagged the new "A" / "S" / "Q" / "W" SNES defaults if they weren't valid Keycode names.
- **Almost:** Operator runtime validation on three threads. (a) NES: drop `fceumm_libretro.dll` into `<exe_dir>/cores/`, scan a folder of `.nes` ROMs, launch Super Mario Bros, confirm default Z/X controls work. FDS validation needs `disksys.rom` in `<exe_dir>/system/`. (b) SNES: drop `snes9x_libretro.dll`, scan a folder of `.sfc` ROMs, launch Super Mario World, confirm diamond layout (Z=B, X=A, A=Y, S=X). Special-chip games (SMRPG / Star Fox / Yoshi's Island) exercise the SA-1 / SuperFX paths in Snes9x. (c) Sidebar refactor: click TG-16 in the sidebar → library filters to TG-16 with the ⚙ Settings button visible in GridControls; click ⚙ → navigates to PerSystemSettingsPage; click Input tab → bindings editor renders with the existing capture flow.
- **Architecture exercise — lessons:** **Lynx took ~600 LOC; NES + SNES together took less.** The pattern is now mechanical:
  1. Add the `SystemId` variant in `oa_core`.
  2. Add `parse_system_id` + `default_core_dll_for_system` arms.
  3. Add the `bindings.rs::<sys>` module with libretro-aligned bits + button table + bit_for + identity remap + defaults function.
  4. Add the four dispatch arms (bit_for / buttons_for / to_libretro_bits / defaults_for).
  5. Add the registry entry in `themes/registry.ts` + the CSS theme block in `themes/systems.css`.
  6. Per-core docs.
  The renderer, audio, scanner, watcher, archive, library DB, settings drawers, sidebar, GridControls, save states, cover sync, metadata sync, shader chain — none of it needs to know about the new system. It just appears.
- **Next:** Operator validates NES + SNES + sidebar refactor. After that's confirmed: Phase 3 slice B-2 (bezel overlay + Phosphor composite + slice C TOML preset format) is the natural next renderer step, OR open system #5 (Atari 7800 follows the same recipe), OR Phase 4 differentiators (rewind / TAS / WebM export — system-agnostic so they work for all 4 systems at once).

---

## 2026-05-18 — Phase 3 slice B: multi-pass shader chain + Phosphor preset + Display runtime wiring

Adds the multi-pass infrastructure promised in slice 3.A and lights up the per-game/system Display overrides at runtime — both pending items from the prior slice. Ships a new Phosphor preset (2-pass separable Gaussian blur) as the first chain consumer. Bezel overlay + composite math still deferred (heavier on their own; pair well with the TOML preset format in slice 3.C).

- **Shipped (renderer multi-pass infrastructure):** `crates/oa-render/src/lib.rs` — `EffectPass` struct (pipeline + per-pass uniform buffer + label + uniform_bytes scratch for direction flag) + `IntermediateTexture` struct (width/height/texture/view). Renderer gains `effect_chain: Vec<EffectPass>` + `intermediates: Option<(IntermediateTexture, IntermediateTexture)>` (ping-pong pair, lazily allocated at framebuffer dimensions, re-allocated on fb mode change). `FbTexture` gained a `view` field exposed for chain passes that need it as input. New `build_effect_chain(preset)` constructs the per-preset chain — Phosphor returns `[H-blur, V-blur]`, everything else returns empty. New `create_blur_pass(label, direction_is_x)` builds the pipeline + uniform buffer using the new `shaders/blur.wgsl`. New `ensure_intermediates(w, h)` lazily allocates the pair. New `run_effect_chain()` ping-pongs through the chain: pass 0 reads `fb_texture.view` → writes `intermediate_a`; pass 1 reads `a.view` → writes `b`; pass 2 reads `b.view` → writes `a`; etc. Returns `last_written_was_a: bool` so the caller picks the right input view for the final blit. `present()` refactored: writes the final-blit uniform → uploads fb pixels → runs chain (if any) → builds the final-blit bind group (either the cached `fb_tex.bind_group` for single-pass or a fresh bind group against the last intermediate's view for multi-pass) → executes the final blit pass into the swapchain with viewport math. Intermediate textures are `Rgba8Unorm` (linear) — pre-blit effects work in linear space; the final blit handles the sRGB encode into the swapchain.
- **Shipped (Phosphor preset):** `crates/oa-render/shaders/blur.wgsl` (~50 LOC). 5-tap Gaussian (weights `{0.0613, 0.2447, 0.3880, 0.2447, 0.0613}` for σ=1), branched on `direction_is_x` to sample along x or y. Per-pixel UV step = `1 / fb_dim` so the kernel size stays in source-pixel space regardless of output resolution. `ShaderPreset::Phosphor` variant added; `id() == 0` (final blit passes through unchanged; the chain did the work); `parse("phosphor") / as_str() == "phosphor"`; new `is_multipass()` method returns true only for Phosphor. 2 new oa-render unit tests: `is_multipass_separates_chain_from_branch` (locks the single-pass vs multi-pass split) + `phosphor_string_round_trips` (parse/as_str symmetry).
- **Shipped (frontend):** `settings/store.ts` — `ShaderPreset` union extends to include `"phosphor"`, `SHADER_PRESET_OPTIONS` array + `SHADER_PRESET_LABELS` record both gain the new entry ("Phosphor (soft bloom)"). PerSystemSettingsPage + PerGameSettingsDrawer Shaders tabs automatically pick up the new option via the existing For loop over `SHADER_PRESET_OPTIONS` — no UI changes needed. The settings page's persistence + SettingRow inheritance UI both work for the new preset out of the box (one of those moments where the Phase 2.8 scaffolding pays off).
- **Shipped (Display runtime wiring):** `App.tsx::handleLaunch` extended from "resolve shader preset" to "resolve shader preset + scaling + windowMode + monitorIndex" in a single `Promise.all` over `get_system_settings` + `get_game_overrides`. Pushes via parallel `set_shader_preset` + `set_scaling_mode` + `set_window_mode` invokes before `launchRom`. `handleUnload` reverts all three to OA-wide settings after the unload completes — so the NEXT launch (which may have no per-game override) doesn't inherit stale state from the game that just unloaded. Region override stays scaffold-only (per-core BIOS region wiring is per-core work) but the resolved value is logged so future per-core consumers can read it. Closes the "Display + Region overrides persist but don't yet take effect at runtime" item that's been carrying through slice notes since Phase 2.8.C.
- **Validation:** `cargo test --workspace` 75/75 green (59 oa-shell + 15 oa-render — was 13, +2 for Phosphor + is_multipass + 1 oa-libretro doc-test). `cargo check -p oa-render` clean. Frontend build: 53 modules (unchanged), 46.25 kB CSS (unchanged), 248.19 kB JS (+0.84 from slice 3.A's 247.35 for the expanded handleLaunch + handleUnload wiring), 860 ms. TypeScript `--noEmit` clean.
- **Almost:** Operator runtime validation. (a) Set OA-wide shader preset to Phosphor; launch a TG-16 game; confirm the bloom is visible (looks like a soft CRT — entire image gently smeared by ~2px radius). (b) Same on Lynx (160×102 framebuffer — kernel is in source-pixel space so blur looks proportionally bigger to the eye but stays anchored to source pixels). (c) Set per-game shader override to Phosphor on one game; launch; confirm bloom applies only to that game. (d) Set per-system scaling override to "pixel-perfect" on tg16; launch; confirm renderer applies pixel-perfect scaling on launch (was previously persisting but ignored at launch — now takes effect). (e) Unload + launch another game with no override; confirm renderer reverts to OA-wide scaling. (f) Same flow for window mode + monitor. **Known limitations:** Phosphor uses a 5-tap kernel — adequate but not ideal for high-resolution sources (a 9-tap variant lands in slice B-2 alongside parameterization through TOML). The intermediate textures allocate at fb dimensions (not output dimensions) so the blur kernel is in source-pixel space — bigger UI footprint than output-pixel space would give. The composite step (`final = source * (1-bloom) + blur * bloom`) is also slice B-2 work; today's Phosphor just shows the pure blurred output, which is a stylistic choice rather than an accurate phosphor simulation.
- **Phase 3 status:** Slice A + B both ✅. Slice C — per-game shader preset TOML schema + parameter sliders + hot-reload — is the next renderer work. Slice D — HDR tone mapping — requires an HDR-aware swapchain format (R16G16B16A16Float) where the display reports HDR caps, can land anytime since it doesn't depend on the chain architecture.
- **Architecture note worth recording:** the multi-pass chain has zero impact on single-pass presets — Plain/Scanlines/CrtLite still use the original 1-draw-call path (`pipeline.set_bind_group(fb_tex.bind_group)`). Only Phosphor allocates intermediates + runs the extra encoder. The branch is sub-microsecond so frame-time impact is invisible on single-pass.
- **Next:** Phase 3 slice C (TOML preset format + hot-reload — when we want to expose preset parameters to power users), OR Phase 3 slice D (HDR), OR open Atari 7800 as system #3, OR Phase 4 differentiators. With Lynx + tg16 both live, slice C is the natural next renderer step — having two systems gives the preset format a real test of "preset X is the per-system default for Lynx, preset Y for tg16."

---

## 2026-05-18 — Lynx onboarding: system #2 comes online

Sidestep from Phase 3 to broaden the system coverage. Adding Atari Lynx as system #2 is the first real exercise of the multi-core architecture's "8-step recipe" since the 2026-05-16 libretro pivot. With the pivot, that recipe collapses to ~5 light steps — registry entry + CSS block + per-system Rust bindings + system_id threading + per-core docs — and stays within ~600 LOC. No vendoring, no `build.rs` changes, no Cargo workspace edits. Operator drops `mednafen_lynx_libretro.dll` from buildbot.libretro.com into `<exe_dir>/cores/` + `lynxboot.img` into `<exe_dir>/system/` and the new core is live.

- **Shipped (Rust bindings):** `apps/oa-shell/src/bindings.rs::lynx` module with button-bit constants laid out to match `RETRO_DEVICE_ID_JOYPAD_*` directly (B=1<<0, SELECT=1<<2, START=1<<3, dpad 4-7, A=1<<8, PAUSE=1<<10) — the Lynx → libretro remap is identity by construction. `LYNX_BUTTONS` canonical iteration table preserves Lynx-native names (OPT1 / OPT2 / PAUSE) for the bindings UI. `default_lynx_bindings()` ships Z/X on the action buttons + Enter/RShift on the options + Space on Pause + arrows on the d-pad (keeps muscle memory aligned with PCE for the action pair). `lynx_to_libretro_bits` masks high bits and returns identity. **System-aware dispatch primitives** — new `bit_for(system_id, button)`, `buttons_for(system_id)`, `to_libretro_bits(system_id, bits)` — replace the previously hardcoded tg16-only paths in `apply_bindings_to_poller` / `bindings_to_response` / `set_binding`. Unknown systems fall through to identity remap + empty button list, which is the right shape for "we haven't registered this system yet." 3 new unit tests: `defaults_cover_every_lynx_button`, `lynx_remap_is_identity` (locks the bit layout so accidental enum reordering doesn't silently mis-route input), `to_libretro_bits_dispatches_by_system` (proves PCE goes through the PCE remap, Lynx is identity, unknown systems pass through).
- **Shipped (Rust threading):** **`EmuCommand::LoadRom`** gained `system_id: String`. The emu thread tracks `current_system_id` (defaults `"tg16"` until first launch); `set_input_remapped` takes the active system + dispatches via `bindings::to_libretro_bits`; `apply_bindings_to_poller` takes system_id (so a Lynx binding like "OPT1" no longer skips via `pce_bit_for is None`); cores.json lookup uses the active system key (per-system pref + per-system default). **`launch_rom` Tauri command** gained an optional `systemId: Option<String>` param (defaults `"tg16"` for backward compat with any caller not yet updated). New helpers: `parse_system_id(s)` maps frontend strings → `oa_core::SystemId` for the libretro core's metadata tag, `default_core_dll_for_system("lynx") → "mednafen_lynx_libretro.dll"` (returns `"mednafen_pce_fast_libretro.dll"` for tg16 + any unknown system as the sensible fallback).
- **Shipped (frontend):** `themes/registry.ts` — `SystemId` union extends from `"tg16"` to `"tg16" | "lynx"`; new `systemThemes.lynx` entry with `.lnx` + `.lyx` extensions + landscape `tileAspect: "4/3"` (matches the Lynx box-art family, not the framebuffer's 160:102 — different concerns). `themes/systems.css` gained a `[data-system="lynx"]` block with `oklch(0.65 0.22 290)` purple accent + a soft lilac variant for text-on-color + the same accent at 35% alpha for hover glow. `library/launch.ts` now passes `entry.systemId` in the `launch_rom` invoke args. **No UI component changes** — the per-system settings page, per-game drawer, left sidebar, breadcrumb, and all the inheritance-chain resolvers automatically pick up the new system from the registry. Verifies the scaffolding work from Phase 2.8 slices C+D was sound.
- **Shipped (docs):** New `docs/cores/lynx/` with the standard 5 files (README, ROADMAP, SESSION_LOG, KNOWN_GAME_BUGS, DECISIONS). `docs/ACTIVE_CORE.md` flips from `tg16` to `lynx`. Operator setup documented in lynx README: download `mednafen_lynx_libretro.dll` from buildbot.libretro.com → drop in `<exe_dir>/cores/`; drop `lynxboot.img` (SHA-1 `e4ed47fae31693e016b081c6bda48da5b70d7ccb`, 512 bytes) in `<exe_dir>/system/`.
- **Validation:** `cargo test --workspace` 73/73 green (59 oa-shell — was 56, +3 Lynx tests + 13 oa-render + 1 oa-libretro doc-test). `cargo check -p oa-shell` clean (2 dead-code warnings on `pce_bit_for` + `lynx_bit_for` are intentional — they're the per-system helpers callers use; future input-pipeline rewrite will reach them). Frontend build: 53 modules (unchanged), 46.25 kB CSS (+0.15 for the Lynx theme block), 247.35 kB JS (+0.12 for the systemId passthrough), 856 ms. TypeScript `--noEmit` clean.
- **Almost:** Operator runtime validation. The full chain hasn't run end-to-end against a real Lynx ROM yet — operator needs to provide the .dll + BIOS. Expected validation flow: (a) drop the two files in place; (b) `cargo tauri dev`; (c) Settings → Library → Add a folder containing `.lnx` ROMs; (d) confirm tiles appear under a Lynx group in the sidebar with purple accent; (e) launch a game (California Games is the canonical "does it work?" pick); (f) confirm default keyboard map controls work; (g) F5/F8 save state round-trip (Mednafen Lynx supports `retro_serialize` so this should work via the existing path); (h) the per-game drawer's Core tab + Cores tab in PerSystemSettingsPage both list the new mednafen_lynx core.
- **Multi-core architecture exercise — lessons:** The recipe in [[feedback_multi_core_architecture_ready]] was written assuming static crates; with the libretro pivot it collapses to even fewer steps. The genuinely-new shape is the system_id threading work — it was previously implicit ("tg16 everywhere"), now it's an explicit parameter that flows from frontend launch → Tauri command → EmuCommand → emu thread state → input remap dispatch + cores.json lookup. That groundwork pays for itself as system #3 (Atari 7800?) arrives — no more "tg16 hardcoded" surface to find and replace. Worth recording: **adding Lynx took less code than slice 3.A** (the shader chain foundation). The architecture is doing its job.
- **Next:** Resume Phase 3. Slice B (multi-pass shader chain + bezel/phosphor + bundle Display/Region runtime wiring) is queued and lights up the per-game Display overrides at runtime — at which point Lynx's 75 Hz native + 160×102 framebuffer becomes a useful second test surface for the shader passes (different resolution + frame rate than tg16's 256×239 @ 59.8 Hz). Alternatively, the operator can validate Lynx first and shake out any pure-onboarding gotchas before more renderer work lands on top.

---

## 2026-05-18 — Phase 3 slice A: shader chain foundation (Plain / Scanlines / CrtLite)

First slice of Phase 3. Lights up the Shaders tab scaffolds from Phase 2.8 slices C + D against a real renderer surface — three runtime presets, full inheritance chain (per-game → per-system → OA-wide), resolved on every launch. Single-pass architecture for now (one branched fragment shader) — multi-pass chain (separable Gaussian for the phosphor bloom, bezel overlay composite) lands in slice B.

- **Shipped (Rust renderer):** `crates/oa-render/src/lib.rs` gained a `ShaderPreset` enum (Plain / Scanlines / CrtLite) with stable `id()` ints + `parse() / as_str()` round-trip helpers. Renderer gained a tiny 16-byte uniform buffer `{ preset_id: u32, fb_height: u32, _pad0: u32, _pad1: u32 }` bound at `@group(0) @binding(2)`. `present()` writes the uniform unconditionally each frame — cheap against the per-frame framebuffer texture upload. `create_fb_texture` extends the bind group with the uniform. New `set_shader_preset()` / `shader_preset()` methods. 2 new unit tests: `shader_preset_round_trips_strings` (parse/as_str symmetry + unknown strings fall back to Plain) + `shader_preset_ids_are_stable` (locks down the WGSL branch ids so accidental enum reordering doesn't silently mis-render). `crates/oa-render/shaders/blit.wgsl` rewritten: same vertex shader (oversized triangle), new fragment shader branches on `u.preset_id`. **Plain** = pass-through (Phase 1 baseline). **Scanlines** = alternate-row darken at 0.85 intensity locked to source `fb_height` (so scanline period stays crisp at any output resolution — the rasterizer's UV interp gives a continuous coordinate we round to source rows via `u32(uv.y * fb_h)`). **CrtLite** = heavier scanlines (0.75 intensity) + radial vignette (soft falloff, won't crush corners) + saturation lift via luminance-mix to recover the perceived dimming.
- **Shipped (Rust wiring):** `EmuCommand` gained `SetShaderPreset(oa_render::ShaderPreset)`. Emu thread routes the message to `renderer.set_shader_preset()` next to the existing `SetScalingMode` handler. New `set_shader_preset(preset: String)` Tauri command parses via `ShaderPreset::parse` (defaulting to Plain on unknown — same defense as the renderer's parse). `SystemSettings` (slice C) gained `shader_preset: Option<String>` field. `GameOverrides` (slice D) gained `shader_preset: Option<String>` field — also folded into `set_game_overrides` `is_empty` check so a Plain-everywhere bag still nulls out the column. Existing tests updated with the new fields; round-trip + clear assertions still hold via PartialEq's full-struct equality.
- **Shipped (frontend):** `settings/store.ts` — `ShaderPreset` union type, `SHADER_PRESET_OPTIONS` / `SHADER_PRESET_LABELS`, `shaderPreset` signal in `createSettingsStore`, persisted into `oa.settings.v1` localStorage. New `createEffect` pushes OA-wide changes via `set_shader_preset` immediately (mirrors the existing scalingMode effect). **PerSystemSettingsPage Shaders tab** swapped from scaffold-only to a live SettingRow + select; saves into `systems/<id>.json::shaderPreset`. **PerGameSettingsDrawer Shaders tab** same shape with the two-level inheritance chain — `inheritedShader()` resolver returns `{ label, from: "Per-system" | "OA default" }` like the other resolvers added in slice D. **App.tsx handleLaunch** now does a `Promise.all` over `get_system_settings` + `get_game_overrides`, resolves `effective = game.shaderPreset ?? sys.shaderPreset ?? settings.shaderPreset()`, and invokes `set_shader_preset({ preset: effective })` before `launchRom`. Soft-failures don't block launch — worst case the renderer keeps its previous preset.
- **Validation:** `cargo test --workspace` 70/70 green (56 oa-shell + 13 oa-render — 2 new on top of slice D's 11 + 1 oa-libretro doc-test). **Correction worth recording:** the prior 2.8.A-D session logs attributed the 11 to "oa-input" — actually those have always been oa-render tests (viewport math). oa-input has 0 unit tests. Counts in earlier entries should read "X oa-shell + 11 oa-render + 1 oa-libretro doc" not "X oa-shell + 11 oa-input". Slice 3.A is the first time this matters because both oa-shell + oa-render gained tests in the same slice. `cargo check -p oa-render` clean. Frontend build: 53 modules (unchanged), 46.10 kB CSS (unchanged), 247.23 kB JS (+2.16 kB for shader wiring + UI updates), 781 ms. TypeScript `--noEmit` clean.
- **Almost:** Operator runtime validation — (a) launch a TG-16 ROM with OA-wide preset = "scanlines"; confirm horizontal scanlines visible across the framebuffer; (b) switch to "crt-lite" via OA Settings or PerSystemSettingsPage Shaders tab; confirm vignette + heavier scanlines + the saturation lift compensating for dimming; (c) set a per-game shader override on a single game (drawer → Shaders tab → CrtLite); confirm SettingRow shows OA-default chip with line-through + launching THAT game uses crt-lite while other games use OA-wide; (d) clear the per-game override, confirm next launch reverts to OA-wide; (e) confirm `oa.settings.v1` localStorage carries `shaderPreset: "scanlines"` and survives app restart. **Renderer-side regressions to watch:** the new bind group has 3 entries instead of 2 — a stale fb_texture from a pre-slice build could mismatch the layout (no migration needed since textures rebuild on first launch, but worth eyeballing). The uniform write happens BEFORE `write_texture` in `present()` — order-independent at the wgpu level but if Vulkan validation warns about layout transitions, that's the place to look.
- **Phase 3 status:** 🟨 in progress. Slice A ships the chain foundation + 3 presets. **Next slices:** **B — multi-pass + bezel/phosphor effects.** Add intermediate render targets (one ping-pong pair sized to framebuffer-scaled-by-N), turn the single fragment shader into a chain of `EffectPass` objects, ship a separable Gaussian for the phosphor bloom + a static bezel-overlay composite that respects the scaling mode's viewport rect. **C — per-game shader preset TOML schema.** Replaces the hardcoded preset id u32 with a structured preset file at `shaders/presets/<name>.preset.toml` that picks which passes run + what parameters they use. Hot-reload via `notify` watching the presets dir. **D — HDR tone mapping.** Behind a settings checkbox; needs an HDR-aware swapchain format (R16G16B16A16Float) where the display reports it.
- **Display + Region runtime wiring (still pending from slices C/D):** The launch path now resolves the shader preset from the inheritance chain — adding the same resolution for scaling / window / monitor / region is mechanical and lands alongside slice B's multi-pass refactor (the launch-path rewrite touches enough of the same code that bundling makes sense). Tracked here so it doesn't get lost.

---

## 2026-05-18 — Phase 2.8 slice D: per-game settings drawer (phase complete)

Closes Phase 2.8. Slide-in-from-right drawer reached from the tile context menu's new "Game properties…" item. Seven tabs along the top (Overview / Core / Display / Audio / Input / Shaders / Region) at a 480px max-width since horizontal tabs scale better than a side rail in a narrow surface. Core + Display + Region tabs are wired through SettingRow with the full two-level inheritance chain (per-game → per-system → OA-wide); other tabs are scaffold-only.

- **Shipped (Rust):** **Schema v3.** `library_db.rs::SCHEMA_VERSION` bumped to 3; new `migrate_v2_to_v3` adds `games.overrides_json TEXT` via `ALTER TABLE` with the same PRAGMA-table_info guard pattern as the v1→v2 archive_inner_path migration so re-running after a mid-flight failure doesn't error. **`GameOverrides`** type holds `scaling_override` / `window_mode_override` / `monitor_index_override` / `region_override` (all Option, camelCase serde, defaults to all-None). Per-game core override deliberately stays in its existing `core_override` column (the launch path reads it directly; bridging would be needless churn). Two new Tauri commands: `get_game_overrides(id)` returns the parsed bag or default if column is NULL; `set_game_overrides(id, overrides)` writes the JSON or `NULL`s the column when every field is None (sparse column = cheaper queries + no stale empty `{}` blobs). Two new unit tests: `game_overrides_round_trip_and_clear` (set / read / clear / unknown-id-default) and `schema_v2_to_v3_migration` (builds a v2 DB by hand with a legacy row + opens through LibraryDb + confirms the migration adds the column + the legacy row survives + a v3-shaped override round-trips on it).
- **Shipped (frontend):** `frontend/src/components/PerGameSettingsDrawer.tsx` (~430 LOC). Slide-in-from-right `<aside>` capped at `max-w-[30rem]` (480px), full-height column with a 3-zone layout: themed header (system shortName + game title + close ✕), horizontal tab bar (overflow-x-auto so all 7 fit on narrow screens), scrollable content. Backdrop click + Esc close. Re-hydrates on every open: per-game overrides, per-system settings, cores list, monitors list, per-system core pref — five parallel `invoke()` calls in a `createEffect(props.entry, props.open)` watcher. **Two-level inheritance resolvers** (`inheritedScaling`, `inheritedWindow`, `inheritedMonitor`, `inheritedCore`) compute `{ label, from }` by checking `systemSettings.field ?? oaWide.field` and labeling the chip "Per-system" or "OA default" / "Auto-detect" accordingly — the first real demonstration of SettingRow's `inheritedFrom` prop in its rich two-level form. **Core tab** wires through `library.setCoreOverride` (the existing surface — same one TileContextMenu's CorePickerMenu uses; SettingRow just gives it a richer surrounding presentation). **Display tab** wires `set_game_overrides` for scaling / window / monitor — fields are scaffold (don't take runtime effect yet) but the inheritance UI is fully live. **Region tab** persists a `regionOverride` field for slice E (per-core BIOS region) to consume. **Overview tab** shows read-only metadata (title / system / ROM path / archive_inner_path if present / addedAt). **Audio / Input / Shaders tabs** are scaffold-only with a "Scaffold — Phase X" amber banner. `TileContextMenu` gained an `onOpenProperties: (entry) => void` prop + a "Game properties…" menu item between "Change core…" and the "Remove from library" divider. App.tsx mounts the drawer with a `propertiesFor: RomEntry | null` signal driving open state. The Esc handler uses capture-phase to win over any underlying listeners.
- **Validation:** `cargo test --workspace` 68/68 green (56 oa-shell — 2 new on top of slice C's 54 / 11 oa-input / 1 oa-libretro doc-test). `cargo check -p oa-shell` clean. Frontend build: 53 modules (was 52), 46.10 kB CSS (+0.60 kB for drawer styles), 245.07 kB JS (+12.64 kB for the drawer + inheritance resolvers + new commands wiring), 769 ms cold build. TypeScript `--noEmit` clean. One iteration: dropped `createResource` import after refactoring the hydrate-on-open flow to use `createEffect` (more natural for prop-driven re-hydration).
- **Almost:** Operator runtime validation — (a) right-click a tile, click "Game properties…", confirm drawer slides in from right with the system theme cascade; (b) cycle through 7 tabs, confirm Core + Display + Region are interactive; (c) set a per-game core override on the Core tab + confirm SettingRow shows the per-system value as the inherited chip with line-through; (d) clear the override (— Use per-system / auto —), confirm the chip un-line-throughs; (e) on Display tab, set a scaling override + confirm the `games.overrides_json` column updates (`sqlite3 appData/library/games.sqlite "SELECT id, overrides_json FROM games WHERE id = …"`); (f) close + reopen drawer, confirm the override hydrates back into the input; (g) Esc + backdrop click both close. Runtime-pending items (display/region don't yet alter runtime — same scope call as slice C's display tab).
- **Phase 2.8 status:** ✅ **closed**. All four slices shipped: A (Settings route), B (Quick Settings overlay), C (per-system settings + SettingRow primitive), D (per-game drawer). The three-tier inheritance model is live across all three surfaces — `<SettingRow>` is the load-bearing primitive shared by C + D.
- **Next:** **Phase 3 — Shader pipeline.** WGSL passes for scanline / CRT curve / phosphor / bezel; per-game preset format; live shader hot-reload in dev; HDR tone mapping where supported. This is also where the slice C+D "Display overrides persist but don't yet take effect" rows finally light up at runtime — the same launch-path rewrite that picks the active shader preset also resolves the active scaling / window / monitor from the inheritance chain. Estimated 3-4 weeks per the ROADMAP. Alternative paths if Phase 3 isn't next: (a) open Lynx as system #2 (`docs/ACTIVE_CORE.md` swap, 8-step recipe per [[feedback_multi_core_architecture_ready]]); (b) Phase 4 differentiators (rewind / TAS / WebM export / memory inspector); (c) Phase 5 PCE-CD bringup (Mednafen PCE full vendor + CHD support). The operator's call.

---

## 2026-05-18 — Phase 2.8 slice C: per-system settings page + SettingRow inheritance primitive

Builds the load-bearing UI primitive for the three-tier (OA-wide / per-system / per-game) inheritance model — `<SettingRow>` — and the first consumer page on top of it. Reached via a new `⚙ Settings` button in the system page header. Same left-rail tab layout as the slice-A OA Settings page but scoped to one system: Display / Audio / Input / Cores / Shaders / Theme. Display + Cores tabs are wired (overrides persist + the Cores tab takes effect on next launch — it bridges the existing `get_core_pref` / `set_core_pref` store); the other four tabs use SettingRow as scaffold-only with a "Scaffold — Phase X" amber-banner explaining what's coming. Slice D (per-game drawer) inherits SettingRow directly.

- **Shipped (Rust):** New `apps/oa-shell/src/system_settings.rs` (~150 LOC, 4 unit tests). `SystemSettings { scaling_override: Option<String>, window_mode_override: Option<String>, monitor_index_override: Option<i32> }` — all fields `Option<T>` so old files parse forward when new overrides land. Stored as one JSON file per system at `appDataDir/systems/<system_id>.json`; missing file → defaults; malformed JSON → defaults (matches the layout.rs / shell.rs / cores.rs convention). Two new Tauri commands: `get_system_settings(systemId)` + `set_system_settings(systemId, settings)`. The per-system core override deliberately stays in `cores.json` (its existing store) — slice C bridges both stores transparently in the UI rather than migrating, since migration without a need-to is churn. Tests: default-is-all-none, missing-file-returns-default, round-trip-through-disk (writes one system, reads back; untouched system still defaults), malformed-JSON-falls-back-to-default.
- **Shipped (frontend primitive):** `frontend/src/components/SettingRow.tsx` (~80 LOC). Three-column flex layout (label / input / inherited-value chip). The chip shows what the setting WOULD be if not overridden — when not overridden, the input renders at `opacity-70` to suggest "this is inherited, not active here"; when overridden, the chip renders with `line-through` to suggest "this is what you'd revert to." `inheritedFrom` prop labels the chip ("OA default" / "Per-system" / etc.) so per-game rows (slice D) can show their own two-level inheritance correctly. Presentation-only — parent owns the input shape (select / input / button group / whatever fits) so the component slots into both per-system + per-game without conditionalizing.
- **Shipped (frontend page):** `frontend/src/components/PerSystemSettingsPage.tsx` (~400 LOC). Two-pane layout: left tab nav (6 tabs) + right content. **Display tab** is wired against the new Rust commands: three SettingRow inputs (scaling mode / window mode / monitor) — each input shows the OA-wide value via the SettingRow chip and offers an `— Use OA default —` first option that clears the override (writes null, drops the field from the on-disk JSON via `patch`). **Cores tab** bridges existing `get_core_pref` / `set_core_pref` with a SettingRow whose inherited chip shows the first-auto-detected core's library name (matches the launch-path fallback in main.rs). **Audio / Input / Shaders / Theme tabs** are scaffold-only — show a `PendingRuntimeBanner` describing what's coming. Input tab specifically points back to the existing bindings editor on the system page (the bindings table already lives there + already persists per-system; moving it into this page is a future consolidation slice). Active tab persisted to `localStorage["oa.per-system-settings.activeTab"]`. Esc + Back button both navigate back to the system view via `setCurrentView({ kind: "system", id })`.
- **Shipped (wiring):** `SidebarView` gained `{ kind: "system-settings"; id: SystemId }` (LeftSidebar.tsx). `SystemPage` gained an optional `onOpenSystemSettings` prop + a `⚙ Settings` button in the header (top-right, system-accent hover) — flexbox header layout now puts title left + button right. `App.tsx`: new `systemSettingsPage` memo mirroring `systemPage`; new `<Match when={systemSettingsPage()} keyed>` clause in the route Switch ahead of the existing system Match (settings is the more specific kind, so it routes first). Breadcrumb memo handles the new kind (`["Library", "TG16", "Settings"]`). LibraryView's exhaustive title-switch gained a `case "system-settings"` arm for TS completeness (it never actually mounts in that mode — App.tsx Switch routes elsewhere).
- **Validation:** `cargo test --workspace` 66/66 green (54 oa-shell — 4 new on top of slice B's 50 / 11 oa-input / 1 oa-libretro doc-test). `cargo check -p oa-shell` clean. Frontend build: 52 modules (was 50), 45.50 kB CSS (+0.68 kB for SettingRow + page chrome), 232.43 kB JS (+13.44 kB for the page + the new commands wiring + SettingRow), 805 ms cold build. TypeScript `--noEmit` clean.
- **Almost:** Operator runtime validation — (a) navigate to a system page, click the new `⚙ Settings` button, confirm the per-system page loads with the system theme cascade applied; (b) cycle through 6 tabs, confirm Display + Cores are interactive and the scaffold tabs render their amber banners; (c) toggle scaling-mode override on Display tab, confirm the inherited chip line-throughs + `appData/systems/tg16.json` lands with `{"scalingOverride":"pixel-perfect"}`; (d) clear the override (pick `— Use OA default —`), confirm the field drops from the JSON file; (e) change the default core on the Cores tab, confirm it persists into `cores.json` (same store the OA Settings → Cores tab writes); (f) Back / Esc both return to the system page. Runtime-pending items NOT validated by slice C (they're docs-only stubs today): Display overrides don't yet alter the renderer at launch — the launch path still reads OA-wide `set_scaling_mode`. Wiring them is small but punts to Phase 3 (per-game shader work pulls the same code path; doing it now would duplicate that.)
- **Next:** **Phase 2.8 slice D — per-game settings drawer scaffolding.** Slide-in-from-right drawer reached from the right-sidebar `⋯` or Game Detail. Tabs along the top per the spec (Overview / Core / Display / Audio / Input / Shaders / Region). Reuses `<SettingRow>` directly — each row shows the per-system value (or OA-wide if no per-system override) as the inherited chip. Per-game overrides persist into the SQLite `games.custom_fields_json` column (already in the schema) keyed by override category, or into a sibling `games.overrides_json` if we want stricter type separation. Estimated ~500-600 LOC.

---

## 2026-05-18 — Phase 2.8 slice B: Quick Settings overlay

Replaces the slice-A-era single-window Esc → library-toggle behavior with a proper in-game Quick Settings overlay. Esc during single-window gameplay now opens a center-aligned card over a dimmed backdrop with the 7 most useful in-game actions: Resume / Save&Load states / Game info / Scaling (status row that opens All Settings → Display) / Shader (Phase 3 placeholder) / All settings / Exit to library. Library access flows through Exit-to-library now, not by toggling visibility — which eliminates the slice-A coupling where Esc fired both App.tsx's library toggle AND SettingsPage's onBack.

- **Shipped (frontend):** `frontend/src/components/QuickSettings.tsx` (~180 LOC). Props: `open`, `onClose`, `entry: RomEntry | null`, plus four action callbacks (`onShowSaves` / `onShowInfo` / `onExitToLibrary` / `onOpenAllSettings`) and the `SettingsStore` ref so the Scaling status row can render the current scaling label live. UI is a `max-w-sm` card with a system-themed accent header (system shortName + game title), a column of `ActionRow` buttons with icon / label / hint chip / destructive-flavor variant, and an Exit-to-library row separated by a hairline divider at the bottom. Capture-phase Esc listener inside the component wins over App.tsx's Esc handler so closing the overlay never bubbles to the library-toggle code path. Auto-focuses the first non-disabled action via a `createEffect` + RAF when the overlay opens; calls `invoke("set_ui_intercepting", true)` on open and `false` on close so gameplay input + F5/F8/digit hotkeys pause for the overlay's lifetime. Each ActionRow gets `data-quick-action` so the focus-selector picks the first button correctly across reorders. Backdrop click closes; the overlay carries `data-system={entry.systemId}` for theme cascade. **`App.tsx` wiring**: imported QuickSettings; added `runningEntry: RomEntry | null` signal (full entry, not just title) + `quickSettingsOpen: boolean`. `handleLaunch` + `SaveSlotsModal.onLaunchedFromSlot` + `GameInfoModal.onLaunched` all set the new signal; `handleUnload` clears it and closes the overlay (so a hardware-bound unload via Ctrl+W also dismisses the overlay cleanly). Esc handler in single-window + gameRunning mode no longer toggles `libraryVisible` — it sets `quickSettingsOpen(true)` (and the overlay's own listener handles Esc-to-close). `SUPPRESS_DEFAULT.has` gate extended to also bypass `BUTTON` and `SELECT` tag names so Enter on a focused button in the overlay activates the click instead of being swallowed — a quiet correctness fix that benefits every focusable button in the shell, not just QuickSettings.
- **Validation:** `cargo test -p oa-shell` 50/50 (unchanged — no Rust touched). Frontend build: 50 modules (+1 for QuickSettings.tsx), 44.82 kB CSS (+0.43), 218.99 kB JS (+3.87), 696 ms cold build. TypeScript `--noEmit` clean.
- **Almost:** Operator runtime validation — (a) launch a game in single-window mode, hit Esc, confirm the overlay opens centered over the game with the system accent; (b) Tab through actions, hit Enter on Save / Load states, confirm SaveSlotsModal opens for the running entry; (c) click Exit to library, confirm the ROM unloads and the library reappears; (d) launch a game in two-window mode, hit Esc on the game window — should be a no-op since two-window doesn't gate Esc to the overlay (the library window is its own surface). One open polish: Scaling row currently opens the full Settings page; the spec hints at an inline cycle through scaling modes which would be richer in slice B but adds a sub-menu shape worth deferring to the per-game drawer (slice D) where scaling becomes one of many per-game overrides.
- **Next:** **Phase 2.8 slice C — Per-system settings page scaffolding.** Linked from the system page header `⚙` button. Same left-rail tab layout as the OA-wide Settings page but scoped to one system: Display / Audio / Input / Cores / Shaders / Theme override. The interesting new component class is the **inherited-value row** — shows the OA-wide value greyed out next to the per-system override input, VS Code workspace-vs-user pattern. Once that primitive exists, slice D (per-game drawer) inherits it directly. ~400-500 LOC.

---

## 2026-05-18 — Phase 2.8 slice A: Settings as a route

First slice of Phase 2.8. Converts the SettingsModal overlay → SettingsPage rendered as a routed view inside the main content region. Same 6 tabs (Presentation / Display / Audio / Cores / Library / Game media), same body content, but now reachable as a `SidebarView` instead of a modal-open boolean. Toolbar ⚙ button + the new left-sidebar Settings entry + the overflow menu's Mode item all navigate to it. Quick Settings overlay during gameplay (the in-game Escape-card) is slice B; per-system / per-game settings drawers are slice C/D.

- **Shipped (frontend):** `frontend/src/components/SettingsPage.tsx` (renamed from `SettingsModal.tsx`). Drops the modal overlay chrome — no more `<Show when={props.open}>` wrapper, no more `fixed inset-0 ... bg-black/60 backdrop-blur-sm` backdrop, no more `aria-modal="true"`. New outer wrapper is a `flex h-full w-full flex-col` page; the inner tab nav + content area is unchanged. Header gets a left-aligned `‹ Back` button (replaces the Close button) wired to a new `onBack: () => void` prop (replaces `onClose`). Esc handler now navigates back via `onBack` instead of closing; gated against INPUT/TEXTAREA/SELECT so the user can still type into search/text fields without bouncing out. All five `createResource(() => props.open, async (open) => …)` calls simplified to fire-once-on-mount via `createResource(async () => …)` — un/remount on route change replaces the modal-era "fire on open" pattern, which is the right semantic for a page. Storage-stats refetch effect drops the `props.open` guard. **New `SidebarView` variant**: `{ kind: "settings" }` in `layout/LeftSidebar.tsx`. **New left-sidebar Settings entry**: pinned at the bottom of the sidebar next to the collapse toggle, full-width row with ⚙ icon + label (icon-only when sidebar collapsed). Highlights when `isActive({ kind: "settings" })`. The "shell-level actions" footer block — Settings + Collapse — is now its own section visually separated from the Library navigation items above (Steam / BigBox convention). **App.tsx routing**: imported `Switch` + `Match` from solid-js; the old `<Show when={systemPage()} fallback={LibraryView}>` is now a `<Switch fallback={LibraryView}>` with a `<Match when={currentView().kind === "settings"}>` clause that renders `<SettingsPage>` and a `<Match when={systemPage()} keyed>` clause for the existing SystemPage path. The fade-out `oa-library-fade` wrapper stays scoped to the LibraryView fallback only (Settings + SystemPage don't share gameplay hide-on-idle semantics). Toolbar ⚙ button + the overflow menu's "Mode: …" item swapped from `setSettingsOpen(true)` to `setCurrentView({ kind: "settings" })`. The legacy `settingsOpen` signal + `SettingsModal` mount in the modals block removed entirely. Breadcrumb memo updated: `cv.kind === "settings"` resolves to `["Settings"]` (no "Library" parent — it's a peer view, not a library facet). LibraryView's title-switch gained a `case "settings"` arm to satisfy TypeScript's exhaustiveness check (LibraryView never actually mounts in that mode — the Switch routes it to SettingsPage — but the discriminant has to be exhaustive).
- **Validation:** `cargo test -p oa-shell` 50/50 green (no Rust changes in slice 2.8.A). `cargo test --workspace` 62/62. Frontend build: 49 modules (unchanged from slice C — net rename, no new component file), 44.39 kB CSS (+0.15 kB for the Back button + Settings sidebar entry styling), 215.12 kB JS (+1.17 kB for the Switch/Match routing branch + the SidebarView discriminant expansions), 680 ms cold build. TypeScript `--noEmit` clean.
- **Almost:** Operator runtime validation — (a) click ⚙ in the toolbar AND the new Settings entry in the left sidebar AND the overflow menu's "Mode: …" item, confirm all three land on the Settings page; (b) cycle through the 6 tabs, confirm each tab's body renders the same content as the modal era (`createResource` once-on-mount means the data lands once per page mount instead of per modal-open); (c) Back button + Esc both navigate back to the previous view (currently hard-coded to `{ kind: "all" }` — Phase 2.8 slice B could add view-history if it's a real UX gap); (d) Settings entry in the left sidebar collapses to icon-only correctly when Ctrl+B is pressed. One minor coupling to revisit in slice B: in single-window + game-running mode, Esc fires both App.tsx's library-visibility toggle AND SettingsPage's onBack. The end state (library visible, back on All Games) is correct but the path is double-traveled. The proper fix is Phase 2.8 slice B's Quick Settings overlay — gameplay Esc opens the small in-game card instead of toggling the library directly, which eliminates this overlap entirely.
- **Next:** **Phase 2.8 slice B — Quick Settings overlay.** Small center-aligned card on Escape during gameplay (Resume / Restart / Save&Load states / Scaling / Shader / Game settings → opens per-game drawer / Exit to library). Controller-bindable so it works from a couch gamepad too. Wires the Quick Settings entries into existing scaling / shell / audio / save-state plumbing — they're not new features, just a faster surface. ~200-300 LOC, mostly frontend. After B: slice C (per-system settings page scaffolding) + slice D (per-game settings drawer scaffolding) + the three-tier inheritance UI primitive.

---

## 2026-05-18 — Phase 2.7 slice C: Import wizard + folder_rules CRUD (phase complete)

Closes Phase 2.7. Slice C delivers the 4-step Import wizard (`folder → mapping → preview → confirm`) plus the SQLite folder + folder_rules CRUD it sits on top of. The wizard replaces the toolbar `⋯ → Import folder…` click-handler; the existing dialog-then-progress flow stays on the LibraryView empty-state CTA and the drag-drop commit as the simpler fallback. The watcher + Rescan-all paths still read from `settings.libraryFolders()`; the wizard writes through to both stores so a future slice can migrate the watcher to read directly from SQLite without breakage.

- **Shipped (Rust):** `apps/oa-shell/src/library_db.rs` gained `Folder`, `FolderRule`, `FolderUpdate` types (serde camelCase) + seven methods on `LibraryDb`: `list_folders(include_rules)`, `get_folder_by_path(path, include_rules)`, `add_folder(path, scan_subfolders, subfolders_are_systems, watch_enabled)`, `update_folder(id, FolderUpdate)`, `remove_folder(id)`, `list_folder_rules(folder_id)`, `set_folder_rules(folder_id, &[FolderRule])`. The `set_folder_rules` path is transactional — wipes existing rules, inserts the new set, commits atomically — so a misconfigured wizard commit can't leave half-applied rules in the DB. Folder ids are stable djb2 hashes of the path (`folder-<16hex>`); add → remove → re-add of the same path lands on the same id (FK ON DELETE CASCADE wipes orphan rules between remove and re-add). `FolderUpdate` is partial: each field is `Option<T>`, untouched fields keep their existing value. Six new Tauri commands in `main.rs` (`list_folders`, `add_folder`, `update_folder`, `remove_folder`, `list_folder_rules`, `set_folder_rules`) wired into `invoke_handler!`. Two new unit tests — `folders_crud_roundtrip` (add/list/get/update/remove + duplicate-path error + unknown-id update error) and `folder_rules_replace_and_cascade` (set 3 rules → replace with 2 → eager-load via `list_folders(true)` → folder delete cascades to rules → set on missing folder errors).
- **Shipped (frontend):** `frontend/src/components/ImportWizard.tsx` (~640 LOC). Modal with three-zone shell: header (title + step pills + Close), per-step body, footer with Back / step-specific primary action. **Step 1** — folder pick: text field + Browse button (Tauri dialog), Recently-tracked dropdown sourced from `settings.libraryFolders()`, three toggles (Scan subfolders, Treat subfolders as systems, Watch for new ROMs). Detects when the picked folder is already tracked via `list_folders(true)` on mount + `array.find(path)` lookup; pre-loads its persisted rules, toggle state, and watch flag for editing. **Step 2** — extension → system rules editor. Defaults to `Object.keys(systemThemes).flatMap(s => s.extensions.map(e => ({ pattern: "*."+e, systemId: s })))`. Add rule / Reset to defaults / per-row delete. Pattern accepts `*.pce`, `.pce`, or bare `pce` and normalizes to the bare extension. **Step 3** — live preview. Auto-starts the scan on entry, sets up its own progress + complete listeners (inlined instead of `runBackgroundScan` because the wizard needs the jobId exposed for the Cancel button). Progress card with files-seen counter + indeterminate bar + current-file mono ticker. When complete, computes per-system tally + unmatched-extension warning by applying the current rule set + `systemForExtension` fallback on the inlined rows. Rescan button re-runs the walk with the current rules. **Step 4** — confirm: total count + four sync checkboxes (Cover art / Snapshots / Title screens / Year-genre-developer-publisher-players). Two primary actions: Skip sync (add games only) or Import + sync (fire `sync_media_for_system` + `sync_metadata_for_system` per affected system after add). Sync invocations are fire-and-forget — they emit their own progress events through the existing toolbar status pipeline. Commit step writes to: SQLite folders (add_folder or update_folder + last_scanned_at), SQLite folder_rules (set_folder_rules — transactional replace), library_db.games (via library.addScannedRoms which routes through add_games), and settings.libraryFolders (mirrored so the existing watcher + Rescan-all flows keep working). Status messages bridge to App.tsx's existing status bar via an `onStatus` prop. Modal teardown resets all transient state + tears down event listeners; Escape closes when not actively scanning/committing.
- **Shipped (wiring):** `App.tsx` import + mount; toolbar `⋯ → Import folder…` overflow item now opens the wizard instead of calling `pickFolderAndIngest` directly. `handlePickFolder` (the legacy dialog flow) stays as the LibraryView empty-state CTA's `onPickFolder` handler so the simple "fresh install → pick folder → done" flow still works in one click. Drag-drop commit (`commitDroppedPath`) also continues to use the legacy path — slice C scope didn't include rewriting the drag-drop ingest to invoke the wizard.
- **Validation:** `cargo test --workspace` 62/62 green (50 oa-shell — 2 new on top of slice B's 48 / 11 oa-input / 1 oa-libretro doc-test). `cargo check -p oa-shell` clean. Frontend build: 49 modules (was 48), 44.24 kB CSS (+1.67 kB for wizard styles), 213.95 kB JS (+21.0 kB for the wizard component + its scan/listener plumbing), 686 ms cold build. TypeScript clean (`tsc --noEmit` passes — `npm run build` chains both).
- **Almost:** Operator runtime validation — open `⋯ → Import folder…`, walk the four steps with a real ROMs folder, confirm: (a) Recently-tracked dropdown lists the folders settings remembers; (b) picking an already-tracked folder pre-loads its persisted rules into Step 2 + flags via the accent banner; (c) Step 3 progress bar moves during a large scan and Cancel scan flips the AtomicBool cleanly; (d) the per-system tally + unmatched-extension warning are accurate; (e) Step 4 commit creates the folder + rules in SQLite (verify via `sqlite3 appData/library/games.sqlite "SELECT * FROM folder_rules"`) and that the library tile count bumps + the watcher picks up subsequent new ROMs in the folder. Edge cases to eyeball: deleting every rule in Step 2 then committing (should leave folder + zero rules — scanner still uses registry fallback so the games still bucket); re-importing an already-tracked folder with different rules (UPDATE path should overwrite the old rule set transactionally).
- **Next:** Phase 2.7 is complete. Open: **Phase 2.8 — Settings as a route.** Convert SettingsModal → `/settings` page (keep the existing 5-tab layout), add Quick Settings overlay (Escape during gameplay → small center card with Resume / Restart / Save / Scaling / Shader / Exit-to-library), per-system settings page scaffolding (linked from system page header `⚙`), per-game settings drawer scaffolding (linked from Game Detail / right-sidebar `⋯`). The three-tier inheritance UI (greyed-out inherited values, VS Code workspace-vs-user pattern) is the one new component class to design — the rest is reorganization. Estimated ~1000 LOC, mostly frontend.

---

## 2026-05-18 — Phase 2.7 slice B: background scanner + filesystem watcher + bug-hunt on archive launches

Continuation of the Phase 2.7 import work. Archives now launch end-to-end via the menu's Import folder path (drag-drop has Windows-transparent-window issues that aren't shell-side fixable today; toolbar `⋯ → Import folder` and `Settings → Library → Add` are the working entry points). Two new Rust subsystems shipped: a cancellable async scan service that emits per-file progress over a tokio blocking task, and a recursive filesystem watcher that auto-adds newly-dropped ROMs to the library. Three serious bugs hunted down along the way: rescanFolders dropping the archive_inner_path field (the bug that was making archived games unlaunchable), the overflow menu closing on the same click that opened it, and a stacking-context issue where the overflow menu visually overlapped the right sidebar but clicks fell through to the sidebar underneath.

- **Shipped (bug hunt):** **(1) rescanFolders archiveInnerPath fix.** `frontend/src/library/ingest.ts::rescanFolders` was the only ingest path that didn't propagate `r.archiveInnerPath` into the constructed RomEntry — `ingestFolderPath` had the spread, but the rescan flow used by `Settings → Rescan all` was missing it. Every archived entry inserted via that path got `archive_inner_path = NULL` in SQLite. `INSERT OR IGNORE` then prevented subsequent rescans from updating, so the entries stayed broken across attempts. Fix is one line — add the spread — but the diagnostic path was hours of round-trips: instrumented Rust JSON output, frontend ingest logs, sample-row queries, until the asymmetry between the two near-identical functions became obvious. Both ingest paths now share identical entry construction. **(2) Overflow menu opens-then-closes.** The toolbar's `⋯` button toggles `overflowOpen`; a window-level click-outside listener resets it to false. Both fired on the same click — open via onClick, close via bubble-to-window — so the menu opened and closed atomically. Fix: `e.stopPropagation()` on the `⋯` button's onClick. **(3) Menu clicks fall through to right sidebar.** With the menu open, clicks landed on a SPAN in the right sidebar header even though the menu visually covered that area. Root cause: the toolbar's grid cell (`grid-area: toolbar`) sits earlier in document order than the right sidebar's grid cell (`grid-area: right`), so absent stacking-context overrides the right sidebar renders ON TOP of the toolbar's `z-30` menu. Fix: `position: relative; z-index: 40;` on the Shell's toolbar grid-cell wrapper, which creates a new stacking context that lifts the entire toolbar (menu and all) above sibling grid cells. Confirmed via a capture-phase `[oa-click]` diagnostic that logged the click target's tag/role/class chain.
- **Shipped (scan service):** New `apps/oa-shell/src/scan_service.rs` (~250 LOC). `ScanServiceState` holds an `Arc<Mutex<HashMap<u64, AtomicBool>>>` of in-flight job cancel flags; `next_job_id()` mints monotonic ids. `run_scan_blocking(job_id, handle, folder, wanted, cancel)` walks the tree (same depth-6 limit + dot-skip + archive-peek semantics as the synchronous `scan_recursive`), emits `oa://library-scan-progress` events throttled to ~12 Hz (`PROGRESS_THROTTLE_MS = 80`), and fills a `Vec<ScannedRom>`. Two new Tauri commands: `start_background_scan(folder, extensions) -> jobId` (spawns the walk via `tokio::task::spawn_blocking`, emits an `oa://library-scan-complete` event with the full rows array attached so the frontend hydrates in one round-trip) and `cancel_background_scan(jobId)` (flips the matching AtomicBool, the walker bails at its next directory boundary). Frontend: `frontend/src/library/ingest.ts` gained `runBackgroundScan(folder, extensions, onProgress?)` which sets up listeners, invokes the start command, resolves with the rows when complete. `ingestFolderPath` and `rescanFolders` rewritten to call `runBackgroundScan` instead of the synchronous `scan_rom_folder` Tauri command (the sync one stays in Rust as the underlying impl for backward compat). `App.tsx`'s `scanProgressReporter` plugs into the toolbar status bar so every Scan-in-progress shows `Scanning <folder>: N matched (M archived) · …<file>`.
- **Shipped (filesystem watcher):** New `apps/oa-shell/src/watcher.rs` (~180 LOC). `WatcherState::reconfigure(handle, folders, wanted)` tears down any existing `notify::RecommendedWatcher` and registers a new one watching each folder recursively. The watcher's event closure routes `Create` + `Modify::Name` events to `emit_path_found` (which peeks archives the same way the scanner does, emits `oa://library-watch-found` per ROM-like file or inner entry) and `Remove` events to `emit_path_found`'s sibling that fires `oa://library-watch-removed`. New `set_watched_folders(folders, extensions)` Tauri command rebinds the watcher set; called from a Solid `createEffect` watching `settings.libraryFolders()` so the live watcher set tracks the user's tracked-folder list. Soft removal policy — the frontend logs deletions but doesn't drop entries (user might be moving / renaming); a future Settings toggle can flip this to a hard remove. Two intentional `#[allow(dead_code)]` annotations on the inner struct's `watcher` field (held to keep the OS handle alive; dropped to stop watching) and the `folders()` accessor (kept for the future "Folders" settings panel that reads back the live set).
- **Shipped (drag-drop attempted fix):** DOM-level `dragover`/`dragleave`/`drop` listeners with `preventDefault()` to override WebView2's "no entry" cursor, plus an explicit overlay UI driven by them. The Tauri-side `onDragDropEvent` listener stayed in place. On Windows + transparent single-window mode (the operator's setup), neither layer fires reliably — the OS-level drop target gets confused by the WebView's transparent regions. **Known limitation** — drag-drop works in two-window (opaque library) mode but not single-window transparent mode. The toolbar `⋯ → Import folder` and `Settings → Library → Add` are the reliable entry points. A proper fix likely requires Tauri-side work on transparent-window drop-target registration or a switch away from `transparent(true)`.
- **Validation:** `cargo test --workspace` 60/60 green (48 oa-shell, 11 oa-input, 1 oa-libretro doc). `cargo check -p oa-shell` clean with two intentional `#[allow(dead_code)]` annotations on watcher fields. Frontend build: 48 modules, 42.57 kB CSS, 192.92 kB JS (+3.6 kB over the prior 2.6 build — scan service event listeners + watcher subscriptions + DOM drag handlers), 651 ms. zip + sevenz-rust + notify pull in a bunch of transitive deps but cache cleanly. Operator validation done: rescan via Settings → Library → Rescan all populates the library with the right archive_inner_path values, archive launches work for both cart formats (in-memory bytes) and CD sets (extract-to-temp, cleanup on unload + startup sweep), overflow menu opens and stays open until item-click or click-outside, Import folder menu item triggers the OS folder picker, scan progress shows in the toolbar status during the walk.
- **Almost:** **Drag-drop on transparent single-window mode** stays unresolved — DOM events don't fire (Tauri OS handler may be swallowing them silently) and Tauri events don't fire (the OS drop target isn't registering on a transparent HWND). Workaround documented above. Operator can switch to two-window mode (Settings → Display → Shell mode → Two windows) where the library WebView is opaque and drag-drop works through the standard Tauri path. **Filesystem watcher** is wired but not yet operator-validated end-to-end — dropping a new ROM into a tracked folder while the app is running SHOULD trigger an `oa://library-watch-found` event that auto-adds it. The plumbing is there; needs a live test to confirm `notify` on Windows fires the right event kinds for the user's folder type. **Per-folder rules CRUD** (the `folder_rules` table from 2.5) is shipped as the table only — no Tauri commands or UI yet. Currently irrelevant because the scanner uses a single global extension set; per-folder rules become useful only with the Import wizard.
- **Next:** **Phase 2.7 slice C — Import wizard.** Multi-step modal (folder → mapping → preview → confirm) that uses the background scanner + per-folder rules. Replaces the toolbar overflow's Import folder option with a richer experience: live preview of what's about to be added, per-folder extension-to-system mapping editor, archive-expansion summary ("this zip contains 17 PCE + 1 SGX"), and post-import auto-sync of media + metadata. Folder rules CRUD commands land alongside since the wizard is the only consumer. After that wizard ships, Phase 2.7 is complete; **Phase 2.8 — Settings as a route** is next (convert SettingsModal to `/settings` page + Quick Settings overlay during gameplay + per-system / per-game settings drawers per the three-tier split).

---

## 2026-05-17 — Phase 2.7 slice A: archive support (zip + 7z) end-to-end

First slice of Phase 2.7. The user can now drop a folder containing `.zip` / `.7z` archives, see one library entry per ROM-inside-archive, and launch them. Cart-format archives extract in-memory (zero disk pollution); CD-set archives extract to a per-game temp dir cleaned on unload + at startup. The remaining 2.7 work (background scan service, per-folder rules UI, filesystem watcher, import wizard) builds on this foundation but is deferred to a follow-up slice.

- **Shipped:** **SQLite schema v2** — `library_db.rs` now bumps `SCHEMA_VERSION` to 2 and splits bootstrap into `create_v1` + `migrate_v1_to_v2`. Migration adds `games.archive_inner_path TEXT` via `ALTER TABLE` (PRAGMA table_info check guards re-runs) plus a new `folder_rules` table (id, folder_id FK, match_pattern, system_id) for per-folder import rules. `GameRow` carries the new field (camelCase via serde); list/add/search queries updated. Two new tests: `archive_inner_path_round_trips` and `schema_v1_to_v2_migration` (builds a v1 DB by hand with a legacy row, opens via LibraryDb, asserts the legacy row survives + a v2-shaped row inserts). **Archive module** — new `apps/oa-shell/src/archive.rs` (~430 LOC, 9 unit tests) wrapping `zip = "2"` and `sevenz-rust = "0.6"`. Pure-Rust, no system deps. Public surface: `ArchiveKind::from_extension` / `is_unsupported_archive` (rar/tar/gz get a "convert to zip" warning); `list_rom_contents(archive, accepted_exts)` returns ROM-like inner entries filtered to the caller's wanted-extension set; `read_inner_to_bytes(archive, inner)` for cart-format launches; `extract_to_temp(archive, inner_entry, temp_root)` extracts the whole archive into appData/temp/<entry_id>/ and returns the absolute path of the entry-point file for CD sets; `cleanup_temp(temp_root, rom_id)` + `sweep_temp(temp_root)` for crash recovery; `encode_file_path`/`decode_file_path` round-trip the `<archive>#<inner>` convention. Path traversal guards: zip uses `enclosed_name()`; 7z manually rejects `ParentDir` components. Tests include a deliberately-crafted "../escaped.txt" zip to verify the entry stays inside the extraction root. **`is_cd_entry_extension`** — `cue|m3u|toc|ccd` are CD entries; everything else goes through the in-memory cart path. **Scanner extension** — `scan_rom_folder` now peeks inside archives at scan time (independent of the wanted-extension set, since users don't request "zip" as a playable extension). For each ROM-like inner entry, emits a `ScannedRom` with `path = "<archive>#<inner>"` (so `games.file_path` stays unique) and `archive_inner_path` set. `.rar`/.`tar`/`.gz` files surface a warning and are skipped. **Frontend**: `RomEntry.archiveInnerPath?: string` added; `ingest.ts` plumbs it through; `launch.ts` sends `archiveInnerPath` + `entryId` in the launch_rom invoke. **Launch path** — `launch_rom` Tauri command now branches on `archiveInnerPath`: if set, decodes via `archive::decode_file_path`, then routes either to `read_inner_to_bytes` (cart formats → `RomSource::Bytes`) or `extract_to_temp` (CD entry extensions → `RomSource::Path` at extracted entry, `state.active_archive_entry_id` set for cleanup tracking). Raw-path branch clears the active-entry tracking so old archives don't accidentally clean their temp on the next unload. Archived cart save-state stem uses just the inner ROM filename ("Bonk's Adventure (USA)") not the encoded `<archive>#<inner>` form so save dirs stay readable. **Cleanup lifecycle** — three triggers: (1) `unload_rom` reads `state.active_archive_entry_id` and calls `cleanup_temp`; (2) startup sweep runs `archive::sweep_temp(&app_data_dir.join("temp"))` in the setup hook to mop up after crashes; (3) `graceful_exit` calls `sweep_temp` before tearing down windows. **`AppState`** gained `active_archive_entry_id: Arc<Mutex<Option<String>>>` populated by launch_rom and consumed by unload_rom.
- **Validation:** `cargo test --workspace` 60/60 green (48 oa-shell — 11 new across schema-v2 + archive on top of the prior 37; 11 oa-input; 1 oa-libretro doc). `cargo check -p oa-shell` clean; only the 2 trivial unused-import/assignment warnings introduced during the launch_rom rewrite were fixed during the slice (no warnings remaining). Frontend build: 48 modules, 42.57 kB CSS (unchanged), 189.14 kB JS (+0.12 kB for archiveInnerPath plumbing), 755 ms. zip 2.4.2 + sevenz-rust 0.6.1 + their transitive deps (lzma-rust, nt-time, bzip2, zopfli) add ~1m 17s to the first cold build but cache after that.
- **Almost:** Operator runtime validation on the four threads: (a) point the library at a folder containing `bonk.zip` (single cart inside), confirm one tile appears + clicking it boots in-memory; (b) same flow with a multi-file 7z (e.g. `Castlevania CD.7z` containing cue + bins), confirm extraction lands in appData/temp/<rom_id>/ + unload cleans it + temp dir is gone after exit; (c) force-kill mid-game, restart, confirm the startup sweep wipes the leftover temp dir; (d) drop a `.rar` and confirm the warning is logged + the file is skipped (no crash). Edge cases worth eyeballing: archives where the inner ROM lives in a subfolder (`SubDir/foo.pce`); archives where multiple cart ROMs share a base name + region tags (each should get its own library entry via the unique encoded file_path). RAR support deferred per the design note in §archive.rs — pure-Rust readers are limited + licensing footprint is awkward; users can `7z x` and re-zip.
- **Next:** Remaining Phase 2.7 slices (decided at start of session): **(B) Background scan service** with progress events + cancellation. **(C) Per-folder rules** CRUD commands on the `folder_rules` table that shipped this slice (table exists, the wiring is the missing piece). **(D) Filesystem watcher** via `notify = "8"` (workspace dep already added) — auto-add newly-dropped ROMs into folders marked `watch_enabled`. **(E) Import wizard** — 4-step modal: folder pick → system mapping editor → live preview with progress bar + archive-expansion summary + system tally → confirm + run media + metadata sync. Wizard's preview step is the right place to surface "this zip contains 17 PCE ROMs + 1 SGX ROM" so users can edit per-folder rules before commit. After 2.7 fully ships: **Phase 2.8 — Settings as a route** + per-system/per-game settings drawers.

---

## 2026-05-17 — Phase 2.6 library polish: virtualization, view modes, sort/filter/group, drag-reorder, drop-target

Second slice of the main-window plan. Replaces the plain CSS grid with TanStack Virtual + 2D row-grouping, adds a sort/filter/group control bar, two view modes (Capsule grid / Detail list), drag-to-reorder systems in the left sidebar, search-as-you-type in the toolbar center, and a window-level folder drag-drop target. All visible UX, no engine changes.

- **Shipped:** **TanStack Solid Virtual** (`@tanstack/solid-virtual@3.13.24`, ~25 kB) installed. **`frontend/src/library/filter.ts`** — pure functions for the filter/sort/group pipeline: `filterEntries` honors the active sidebar view + search query (case-insensitive substring match), `sortEntries` handles title/addedAt/year with year falling back to title-tie-break when metadata's missing, `groupEntries` produces `{ id, label, entries }` buckets for `none|letter|system` group modes (letter strips leading articles + bucket-sorts so '#' precedes A-Z). **`VirtualLibraryGrid`** — 2D row-grouping virtualizer: derives `columnCount` from container width via `ResizeObserver` (220px tile + 12px gap, min 1 col), flattens grouped entries into alternating `{ kind: "header" }` + `{ kind: "tiles" }` rows, `createVirtualizer` over the row list with per-row `estimateSize` (48px header vs ~360px tile row), `measureElement` for accurate sizes, `content-visibility: auto` + `contain-intrinsic-size` + `contain: layout paint` on each tile (research doc §6 defense in depth). Solid adapter's `mergeProps` re-reads getter-shaped options reactively, so `get count() { return rows().length; }` re-runs `setOptions` whenever entries change. **`DetailListView`** — 1D virtualizer (`GAME_ROW_HEIGHT = 76px`) rendering one game per row with 80×56px boxart + title + system accent + year + developer; per-row data binding via `useMedia` for boxart URL + metadata; group headers reused. **`GridControls`** — sticky bar above the grid: title + count (left) / segmented view-mode picker (▦ Capsule / ≡ List) + Sort dropdown + Group dropdown (right). State lives in `LayoutStore` (extended with `viewMode`, `sortKey`, `groupBy`, `systemOrder` — Rust `LayoutPrefs` got matching fields with `#[serde(default = fn)]` so old `layout.json` files parse cleanly). **`LibraryView`** — wraps GridControls + the active view; lives inside `MediaProvider` so it can call `useMedia()` for year/genre/developer lookups (lifting the filter pipeline into a child component sidesteps App.tsx's "useMedia at top level" trap); empty state has a real CTA card with system-accent button + drop-folder hint. **Search-as-you-type in toolbar center** — replaces the status-text-only zone with a real `<input type="search">`, Escape clears and blurs; filtering is in-memory `.toLowerCase().includes()` (FTS5 in Rust is wired and ready when libraries cross 100K entries). **Drag-to-reorder systems** in left sidebar — HTML5 D&D on each `<li>` with top-half/bottom-half indicator zones derived from `getBoundingClientRect().height/2`, splice-and-adjust on drop (insert-index decrements when moving downward to compensate for the removed source), final drop zone after the last item lets the user drop-past-end for append. Result persisted to `layout.systemOrder: Vec<String>` — registry order is the fallback for unlisted systems so adding a new SystemId tomorrow doesn't disappear it. **Window-level folder drag-drop** via Tauri 2's `getCurrentWebview().onDragDropEvent` — `enter`/`over` shows a centered dashed-border overlay ("Drop folder to import"), `leave` clears it, `drop` extracts `event.payload.paths[0]` and forwards to the new `ingestFolderPath(store, path)` helper (extracted from `pickFolderAndIngest` so both the dialog path and the drop path share the scan + ingest tail). Library folder gets auto-tracked alongside dialog imports. **App.tsx** rewired: replaced `LibraryGrid` import with `LibraryView`, added `searchQuery` + `dropOverlayVisible` signals, embedded the drop overlay as a `pointer-events-none` fixed div so the Tauri-side drop handler stays authoritative. `LibraryGrid.tsx` deleted — superseded by `VirtualLibraryGrid` + `LibraryView`.
- **Validation:** `cargo test --workspace` 49/49 green (37 oa-shell, 11 oa-input, 1 oa-libretro doc — no Rust changes in 2.6 beyond LayoutPrefs schema extension, but the existing layout tests still cover `Default` shape + missing-file fallback + round-trip). `cargo check -p oa-shell` clean. Frontend build: 48 modules (was 37 in 2.5), 42.57 kB CSS (+1.88 kB for GridControls + drop overlay + drag indicators), 189.02 kB JS (+54.19 kB; TanStack Virtual adds ~14 kB minified gzip 4.5 kB on its own, the rest is new components + filter pipeline), 673 ms cold build. The Solid adapter's `createComputed` block automatically re-runs `setOptions` when the getter-shaped `count` accessor's source signals change — verified by toggling sort/group/search and watching the virtualizer recompute total height.
- **Almost:** Operator runtime validation pending — (a) drop a folder onto the window and see it ingest end-to-end (overlay visible during drag, scan progress in status); (b) seach-as-you-type narrows the grid live; (c) sort/group dropdowns persist across restart via layout.json; (d) drag a system from one position to another in the sidebar and confirm the order survives a restart; (e) view-mode toggle switches between Capsule and Detail List without losing scroll position (TanStack remembers offset by default, but the new virtualizer remounts on view change — acceptable for 2.6, smarter restoration is Phase 2.7+). Sidebar nesting (drag-onto-system to create a named subgroup) is still deferred per L-3's "flat-first" footnote — adding nested groups means a tree-shaped state in `systemOrder` and a recursive render, which we'll do alongside the "PC Engine family" tree work later.
- **Next:** **Phase 2.7 — Import wizard + background scanner.** 4-step wizard (folder → mapping → preview → confirm), Rust background scan service emitting progress events, per-folder rules persisting into the `folders` table that shipped in 2.5, filesystem watcher via `notify` crate. The wizard takes over from today's one-shot `pickFolderAndIngest` + the new drop-target — the drop-target stays as a quick path, the wizard is the full-featured import. Then **Phase 2.8 — Settings as a route** (modal → page, Quick Settings overlay during gameplay, per-system + per-game settings scaffolds).

---

## 2026-05-17 — Phase 2.5 layout shell + SQLite library (cross-cutting)

First slice of the BigBox-equivalent main-window plan. Replaces the single-header App.tsx with a real region layout (Top toolbar / Left sidebar / Main / Right sidebar) and moves the library catalog from `localStorage` to `appData/library/games.sqlite`. Plan is in `docs/PLANS/main-window.md`; landscape research is in `docs/RESEARCH/launcher-landscape.md`. Four user-confirmed locked decisions drove this slice (three presentation modes; Settings → route in Phase 2.8; sidebar systems are nested drag-and-drop; SQLite migration folded into 2.5).

- **Shipped:** **Layout primitives** — `frontend/src/layout/{Shell, TopToolbar, LeftSidebar, RightSidebar, state}.tsx` + three default right-sidebar widgets (Hero / Title / Metadata) in `layout/widgets/index.tsx`. CSS-variable-driven geometry tokens in `index.css` with per-presentation-mode overrides via `body[data-presentation="..."]` selectors: Desktop (56px toolbar / 280px left / 320px right / 14px font), Theater (72/320/360 + 16px), Cabinet (88/0/0 + 20px, chromeless). Motion design tokens (`--motion-{instant,fast,medium,slow}`, `--ease-{out,in-out,snap}`) shared across components per the Riot Hextech pattern. **Toolbar** — three-zone (breadcrumb ‹‹/search/actions) with `…` overflow menu (Import folder, Rescan, mode toggle, sidebar collapse) + per-mode idle-hide via existing `oa-header-fade` class. **Left sidebar** — quick destinations (Home / All Games / Favorites / Recent / Continue), system list with per-system count + accent dot + `data-system` cascade, Playlists + Smart Views section headers (empty for now), collapse-to-icons button (Ctrl+B), right-edge drag resizer (200-360px). **Right sidebar** — pin toggle, focused-entry tracking (via new `onFocus` prop on LibraryTile → LibraryGrid → App), three widgets rendered in user-configurable order from `widgetOrder`. **Presentation mode** — Settings → Presentation tab with 3-card picker + sidebar toggles. **Rust IO** — new `apps/oa-shell/src/layout.rs` (3 unit tests) with `appData/layout.json` + `presentation.json` files following the existing shell.json pattern; 4 new Tauri commands (get/set_layout, get/set_presentation_mode). **SQLite library** — new `apps/oa-shell/src/library_db.rs` (7 unit tests) wrapping rusqlite 0.32 (`bundled` feature, ships SQLite + FTS5 statically). Schema v1: `games` (id, system_id, file_path, title, normalized_title, added_at, core_override, cover_path, year/genre/developer/publisher/players, rating, play_time_secs, last_played_at, region, favorite, completed, custom_fields_json, seed) + indices on system_id / added_at / last_played_at + `games_fts` FTS5 virtual table (title + normalized_title + developer + publisher, unicode61 tokenizer) + INSERT/UPDATE/DELETE triggers keeping the index in sync, plus `folders` table for the (future Phase 2.7) import wizard. WAL mode + `synchronous = NORMAL` + foreign keys + memory temp store. 7 new Tauri commands: list_games / add_games / drop_seed_games / update_game_core_override / delete_game / search_games / migrate_library_from_local_storage. **Frontend library store** — rewritten to query Rust on mount (`list_games`), one-shot migrate from `localStorage[oa.library.v1]` if present (idempotent, INSERT OR IGNORE on Rust side, clears the localStorage key on success), seed-insert if truly fresh, mutations write-through via Tauri commands. **Layout state** — moved from localStorage to `layout.json` + `presentation.json` via Tauri; suppresses write-through until after hydrate to avoid echoing defaults back to disk. Locked decisions captured in `docs/PLANS/main-window.md` §12.
- **Validation:** `cargo check -p oa-shell` clean (6.82s for layout module; 34.6s including rusqlite first-time compile of libsqlite3-sys). `cargo test --workspace` 49/49 green (37 oa-shell — 13 new across layout + library_db on top of the prior 24; 11 oa-input; 1 oa-libretro doc-test). Frontend build: 37 modules (was 31), 40.69 kB CSS (+8.13 kB for layout + presentation tab), 134.83 kB JS (+25.34 kB for layout primitives + widgets + store rewrite), 614ms cold build. Two intentional `#[allow(dead_code)]` annotations on `LibraryDb::path()` + `get_cover_path()` + `db_path` field (diagnostics / future launch-path hydration — wired in Phase 3 alongside per-game shaders).
- **Almost:** Operator runtime validation pending — (a) `cargo tauri dev` end-to-end: layout renders, sidebar resize persists, presentation toggle flips body data-attr + geometry, right-sidebar widgets populate as user hovers tiles; (b) one-shot localStorage→SQLite migration on a real library (mark localStorage as having v1 entries before first launch after upgrade); (c) Ctrl+B sidebar toggle and overflow menu actions land where expected during gameplay (single-window) without breaking the existing idle-hide. The four locked decisions cover what to *build*; per-mode style polish (Theater font sizing, Cabinet ambient background, coverflow view) is Phase 2.6+ work.
- **Next:** **Phase 2.6 — Library polish.** TanStack Virtual on the main grid (research doc §5 — Solid adapter, 2D-row-grouping); view mode picker (Capsule grid + Detail list initially; Hero, Wall, Coverflow follow); sort/filter/group bar above the grid; drag-to-reorder systems in left sidebar (already nested via L-3, just needs the DnD wiring); empty-state CTA with whole-window drop target for Import. After 2.6: **Phase 2.7 — Import wizard** (4-step picker → mapping → preview → confirm, Rust background scan service, per-folder rules in the `folders` table that's already shipped, filesystem watcher via `notify`). Then **Phase 2.8 — Settings as a route** (current modal → `/settings` page; Quick Settings overlay during gameplay; per-system + per-game settings scaffolds with three-tier inheritance UI).

---

## 2026-05-15 — Project bootstrap + Spike 1

- **Shipped:** Approved setup plan (`C:\Users\Devilchi\.claude\plans\jazzy-spinning-blum.md`); Day-1 docs scaffolding (`CLAUDE.md`, full `docs/`, `LICENSE` stub, `NOTICES.md`, `.gitignore`); memory bootstrapped (3 entries); Rust 1.95.0 + LLVM 22.1.5 installed via winget. **All three Phase 0 spikes passed same day:** (1) two-window Tauri+wgpu @ 60.2 fps; (2) `cc-rs` builds Beetle PCE Fast Mednafen C on MSVC with one shim define; (3) hand-written FFI chosen over bindgen (4.8× faster builds, no libclang dep, idiomatic enum names). **Phase 1 skeleton landed same day:** full Cargo workspace (10 crates + `oa-shell` binary); `oa-core` trait designed (Core, Framebuffer, Timing, InputState, PortIndex, SystemId, CoreError); Beetle PCE Fast vendored at `crates/oa-pce-sys/vendor/` (5.5 MB) with ORIGIN.md + PATCHES/; `oa-pce-sys` compiles the proven Mednafen endian helpers; `oa-pce::PceCore` stubs `Core` with native timing (256×239 @ 59.826 Hz, 44.1 kHz). Workspace `cargo build` clean in 78s; `cargo test` 4/4 pass; `oa-shell` boots cleanly, opens Tauri WebView, ticks PceCore on emu thread at observed 59.9 Hz for 1320+ frames.
- **Almost:** Real PCE emulation. Today's PceCore is a stub that paints a frame-counter gradient — the renderer, audio sink, and input poller are all wired but produce no output yet. Beetle PCE Fast's full `pce_fast/*.cpp` set is not yet in the cc-rs build.
- **Next:** Two parallel tracks. (a) Expand `oa-pce-sys/build.rs` to compile the full pce_fast core (huc6280.c, vdc.c, psg.cpp, input.c, huc.h... -driven .cpp set), discover and document each integration shim as it surfaces, and write the C++ shim layer exposing `oa_pce_new` / `oa_pce_load_rom` / `oa_pce_run_frame` / `oa_pce_framebuffer` (per the Spike 3 surface). (b) Replace the `oa-render` stub with a real wgpu pipeline that takes a `Framebuffer<'_>` and draws it on the game window from Spike 1.

---

## 2026-05-15 — Renderer + two-window integration (still same day)

- **Shipped:** Real `oa-render::Renderer` (wgpu Surface + RGBA8 texture cache + fullscreen-triangle blit pipeline + nearest-neighbour sampler + WGSL shader at `crates/oa-render/shaders/blit.wgsl`). `oa-shell` now opens BOTH the library WebView AND a native game window with a wgpu surface attached — combined emu+render thread ticks `PceCore` at native rate and presents each frame. Steady **59.8 fps for 3360+ frames** observed; PCE-stub gradient visible in the game window. Three small bugs found and fixed during integration: wgpu 23 still uses the old `ImageCopyTexture`/`ImageDataLayout` names (not the `TexelCopy*` names from 24+); `raw-window-handle` must be a direct dep of `oa-shell`; `RawWindowHandle` is `!Send` so handle extraction must happen INSIDE the spawned thread, not before the spawn.
- **Almost:** Real PCE emulation. The renderer is now production-grade for the blit path; the wrapped C core is still a Rust stub painting a gradient. Audio sink and input poller are still stubs.
- **Next:** The PCE C-core build. Two sub-steps: (a) expand `oa-pce-sys/build.rs` to compile the full `vendor/mednafen/pce_fast/*.cpp` set — discover and document each integration shim define as it surfaces; (b) write a C++ shim layer (`crates/oa-pce-sys/src/shim.cpp`) wrapping Mednafen's globals into the `oa_pce_*` surface from Spike 3, and rewrite `oa-pce::PceCore` to call through that.

---

## 2026-05-15 — Beetle PCE Fast full C/C++ build compiles (still same day)

- **Shipped:** `oa-pce-sys/build.rs` now compiles the **entire** Beetle PCE Fast / Mednafen PCE Fast core tree — 93 object files, 9.6 MB C++ archive + 3.4 MB C archive linked into the Rust binary. Covers `pce_fast/*`, all mednafen helpers, `libretro.cpp` (the core engine), `libretro-common/*` portability layer, `mednafen/cdrom/*` for CD support, `mednafen/sound/Blip_Buffer.c` for audio resampling, `mednafen/tremor/*` for integer Vorbis (CD audio), and the full `deps/libchdr` + `lzma-19.00` + `zstd/lib` + `zlib-1.2.11` chain for CHD CD-image support. The 2 endian FFI bridge tests still pass through the much-larger native lib. **Two real iteration findings** captured in build.rs comments: (1) Mednafen-derived trees need `MEDNAFEN_VERSION_NUMERIC=931` + `STDC_HEADERS` + `__STDC_LIMIT_MACROS` + `_LOW_ACCURACY_` defines, not just `INLINE`; (2) MSVC C++ wants `/EHsc` + `/std:c++14` for Mednafen's pre-C++11 idioms with throw/try/catch. Build is incremental-fast (~16s) after the cold compile.
- **Almost:** A real PCE ROM running. The library compiles and links but no symbols are exposed to Rust yet — `oa-pce-sys/src/lib.rs` still binds only the endian helpers from the spike. The C++ shim layer translating libretro's frontend-callback API into our `oa_pce_*` surface is the next step.
- **Next:** Write `crates/oa-pce-sys/src/shim.cpp`. Provides the 5 libretro frontend callbacks (`retro_set_video_refresh` etc.), wires `retro_init` / `retro_load_game` / `retro_run` / `retro_serialize` into `oa_pce_new` / `oa_pce_load_rom` / `oa_pce_run_frame` / `oa_pce_save_state` (from the Spike-3 surface), and stashes the latest video frame + audio samples for the wrapper to read. Then update `oa-pce::PceCore` to call through, and oa-shell loads a real HuCard ROM from disk.

---

## 2026-05-15 — Bonk's Adventure runs 🎉 (still same day)

- **Shipped:** `crates/oa-pce-sys/shim.cpp` (~280 LOC) — full libretro frontend implementation: video/audio/input/environment/log callbacks, RGB565→RGBA8 + 0RGB1555→RGBA8 + XRGB8888→RGBA8 pixel conversion, retro_init/retro_load_game/retro_run/retro_unload_game/retro_deinit lifecycle wired into the `oa_pce_*` surface from Spike 3. Hand-written `extern "C"` bindings in `oa-pce-sys/src/lib.rs`. `oa-pce::PceCore` rewritten to call through (handle pointer wrapped in `NonNull`, Drop runs `oa_pce_free`, button-bit remap from our PCE layout to libretro's RETRO_DEVICE_ID_JOYPAD_* numbering). `oa-shell` reads `OA_ROM` env var and pipes the bytes through. **Bonk's Adventure (USA), 384 KB HuCard, runs at steady ~60 fps for 813 frames** — Beetle reports back `Samples / Frame: 734.3` (== 44100/60.05) and `Estimated FPS: 60.05`. The renderer detected the 256×243 PCE mode automatically and reallocated its FB texture from the 256×239 pre-load default. The Phase 1 acceptance gate is functionally hit.
- **One integration gotcha worth recording:** Beetle's `retro_load_game` doesn't accept `info->data` directly — when the frontend doesn't provide a file path, the core takes the `RETRO_ENVIRONMENT_GET_GAME_INFO_EXT` path, expecting the frontend to return a `retro_game_info_ext` struct via that environment callback with the data+size pointer and extension string. First load returned status 2 (rejection); adding `GET_GAME_INFO_EXT` support fixed it. Comments in `shim.cpp` flag the requirement for next-system bring-up.
- **Almost:** Audio + input. Beetle generates the samples (`g_audio_count` ticking up each `retro_run`) but `oa-audio` is still a stub — they're not piped to cpal yet. `oa-input` reports zeros so the player can't actually play.
- **Next:** (a) Wire cpal in `oa-audio` and pump the core's audio samples each frame so we get sound. (b) Wire real keyboard input from Tauri into `oa-input` mapped to the PCE button layout. (c) CI matrix.

---

## 2026-05-15 — Audio working (still same day)

- **Shipped:** Real `oa-audio::AudioSink` — cpal default output stream, SPSC ring buffer via `ringbuf` 0.4 between emu thread and audio callback, sample-format conversion (i16/f32/u16) inside the callback. First pass had the right wiring but wrong rate: Windows opened the device at 48 kHz while we push 44.1 kHz, producing ~9% pitch shift + underrun gaps. Added a stateful linear-interpolation resampler on the producer side that carries the last source frame across calls (click-free at batch boundaries). Bonk now sounds correct ("much better" — user-verified). `oa-shell` calls `sink.push(core.drain_audio())` after each frame; stats show 0 ring-buffer drops over multi-minute runs.
- **Almost:** Real input. The button bit-remap from our PCE layout to libretro's joypad numbering is wired through `oa_pce_set_input`; the only missing piece is reading actual keyboard/gamepad events. `oa-input` still returns zeros.
- **Next:** Tauri keyboard events → `oa-input::InputPoller` → PCE buttons. Then add gamepad via `gilrs`. Then CI matrix.

---

## 2026-05-15 — Keyboard input working: Bonk is playable 🎮 (still same day)

- **Shipped:** Real `oa-input::InputPoller` — cross-platform global keyboard polling via `device_query` 4.x. Generic `KeyboardMapping` table (32 bit slots × 5 ports, `Option<Keycode>` each) keeps `oa-input` system-agnostic — the shell binds keys to bit positions using the `oa_pce::buttons::*` constants. Default PCE map: arrow keys = d-pad, Z = I, X = II, Enter = RUN, RShift = SELECT. `Bonk's Adventure runs end-to-end with pixels, audio at correct pitch, AND playable keyboard input` — Phase 1 acceptance gate fully met.
- **Two real integration bugs caught while debugging** (both important to record):
  - **`tauri::Window::is_focused()` returns false for native (no-WebView) windows even when they have user focus.** Our focus gate prevented any input from reaching the core. For now `set_enabled(true)` unconditionally; proper focus tracking needs Tauri-event routing in Phase 2.
  - **`retro_set_controller_port_device` MUST be called AFTER `retro_load_game`, not before.** Beetle's `MDFNI_LoadGame` re-initializes the core and resets `pce_fast/input.c`'s `data_ptr[]` array, disconnecting any pre-load wiring. With pre-load wiring, the input pipeline reaches `g_input_bits[]` and `cb_input_state` returns the right values, but Mednafen never sees the data because its `INPUT_Frame` reads through `data_ptr[]` which points nowhere. Took 4 iterations of diagnostic logging in `cb_input_state` to localise.
- **Almost:** Gamepad. `gilrs` is in the dep graph but not wired. CI matrix is still empty.
- **Next:** Optional gamepad polling via `gilrs` (already in dep graph), then CI matrix (Windows / macOS / Linux × `cargo test` + `cargo tauri build`). That's the last open Phase 1 item; after CI green, Phase 2 (Solid UI shell + library + per-system theming) opens.

---

## 2026-05-16 (evening) — Phase 2 opens: UI shell scaffold landed

- **Shipped:** First Phase 2 bullet. New `frontend/` (Solid 1.9 + TS 5.7 + Tailwind v4 + Vite 6) wired into Tauri via `beforeDevCommand` + `devUrl: 127.0.0.1:5173`. Tailwind v4 chosen over v3 — CSS-first config via the `@tailwindcss/vite` plugin, no PostCSS layer, theme tokens defined inline in `src/index.css` via `@theme`. Cold build 936 ms, 7.3 kB JS + 6.2 kB CSS. Decision captured in `docs/DECISIONS.md`. Phase-1 placeholder `apps/oa-shell/dist/index.html` retired. `cargo check -p oa-shell` clean against the new config. See `docs/cores/tg16/SESSION_LOG.md` for the full play-by-play.
- **Almost:** First end-to-end `cargo tauri dev` run against the live Vite server (operator step). The Rust shell still loads `WebviewUrl::App("index.html")` — Tauri substitutes the devUrl in dev mode automatically, so no Rust code changed.
- **Next:** Phase 2 bullets 2-3 — per-system theming (promote the inline TG-16 tokens into a registerable per-system theme module) then the library grid + cover-art ingestion.

---

## 2026-05-16 — Phase 1 closed; Phase 1.5 aspect + save states shipped

- **Shipped:** Phase 1 fully closed. Gamepad polling wired (`gilrs` in `oa-input`, default PCE map: dpad + east=I / south=II / start=RUN / select=SELECT). Per-core docs scaffolded under `docs/cores/tg16/`. Repo pushed to https://github.com/devilchi666/overlooked-arcade (public, GPLv2). CI green on Windows + macOS + Ubuntu (`.github/workflows/ci.yml`, GitHub Actions). Phase 1.5 hardening: pixel aspect ratio + save states both shipped and visually verified with Bonk. See `docs/cores/tg16/SESSION_LOG.md` for the tg16-specific play-by-play.
- **Almost:** Two Phase 1.5 items deferred (focus-gated input, multitap). Gamepad code is wired but unplaytested by the human.
- **Next:** Phase 2 opens. Solid + Vite + Tailwind UI shell, library grid, per-system theming, settings panel including the window/scaling-mode toggles whose foundation landed today.

**Cross-platform lessons captured this session:**

- macOS arm64 / Xcode 16.4 — vendored zlib 1.2.11 needed a one-line patch to `zutil.h` (skip the `#define fdopen(fd,mode) NULL` redef on `__APPLE__`). Patch at `crates/oa-pce-sys/vendor/PATCHES/0001-*.patch`. Likely applies to any other vendored library of that vintage we bring online later.
- Ubuntu — Tauri's gtk/webkit dep tree is heavy. CI scope decision: build the full workspace on Windows, `--exclude oa-shell` on macOS/Linux. Recorded in `docs/DECISIONS.md`. Revisit at Phase 6+.
- `device_query` on Linux needs `libx11-dev libxi-dev libxtst-dev` in addition to ALSA + udev.
- The signed Azure Blob URLs GitHub serves for raw Actions logs (cogwheel → View raw logs) are publicly accessible for ~10 min — useful for log retrieval when GitHub's UI is paginated/lazy-rendered.

---

## End-of-session retrospective (2026-05-15)

**One session, one day, greenfield to a playable PCE emulator.** Built:

| Layer | Status | Notes |
|---|---|---|
| Workspace | ✅ | 10 crates + binary; clean `cargo build` (78s cold) |
| `oa-core` trait | ✅ | Designed for N systems; `SystemId` non-exhaustive |
| `oa-pce-sys` + `oa-pce` | ✅ | 93 C/C++ objects, 13 MB native libs; shim.cpp ~290 LOC |
| `oa-render` | ✅ | wgpu, RGBA8 texture cache, fullscreen-triangle WGSL, FIFO vsync |
| `oa-audio` | ✅ | cpal default output, SPSC ringbuf, linear-interp resampler |
| `oa-input` | ✅ keyboard | `device_query` polling, generic mapping table |
| `oa-shell` | ✅ | Two-window flow (library WebView + native game window), emu+render thread, audio + input wired |
| Phase 0 spikes | ✅✅✅ | All three passed |
| Phase 1 gate | 🟨 | Pixels + audio + keyboard live; gamepad + CI remaining for clean closure |

**Real lessons captured in auto-memory:**

- `reference_libretro_controller_after_load_game` — `retro_set_controller_port_device` must run AFTER `retro_load_game`. Applies to every libretro-style core we wrap.
- `feedback_tauri_no_webview_is_focused_unreliable` — Tauri Window's `is_focused()` returns false on no-WebView windows. Affects every future window-event wiring.
- `feedback_multi_core_architecture_ready` — every workspace crate except the PCE-specific pair is core-agnostic. Adding a new system follows an 8-step recipe, not a refactor.
- `project_current_state` updated end-of-session.

**Build artefact sizes (debug profile):**

- `oa-pce-sys` native libs: 9.6 MB (C++) + 3.4 MB (C) = ~13 MB
- `target/debug/oa-shell.exe`: not yet measured for release-profile but expect 30-50 MB debug, 15-25 MB release-stripped

**Stack quirks worth remembering:**

- wgpu 23 still uses `ImageCopyTexture` / `ImageDataLayout`; renamed to `TexelCopy*` in 24+.
- Tauri 2's `WindowBuilder` (no-WebView) is behind the `"unstable"` feature flag.
- `RawWindowHandle` is `!Send`; extract inside the spawned thread, never across.
- Beetle PCE Fast's `retro_load_game` needs `RETRO_ENVIRONMENT_GET_GAME_INFO_EXT` support when the frontend doesn't pass a file path — `info.data` alone isn't enough.
- Mednafen-derived headers need `INLINE=__inline` + `MEDNAFEN_VERSION_NUMERIC=931` + `STDC_HEADERS` + `_LOW_ACCURACY_` + the other ~15 shim defines catalogued in `crates/oa-pce-sys/build.rs`.

**Scope clarification (end of session):** the documented 10-system lineup in `docs/VISION.md` is the first wave, not the project ceiling. User clarified: "I want to do a lot more cores than what the documents show. I want to be able to run almost all of them plus new ones we work on." Captured in memory `project_expanded_scope_all_systems.md`; VISION + ROADMAP updated to reflect the broader ambition. The multi-core architecture wired in day one (`feedback_multi_core_architecture_ready.md`) means scope expansion is cheap per-system, so this is a tonal shift rather than a re-plan.

**Phase 2 backlog additions (end of session):** window modes (windowed / windowed-fullscreen / exclusive-fullscreen / monitor selection) and video scaling modes (pixel-perfect / aspect-correct fit / stretched / original 1:1 / explicit integer multiples). Recorded in `docs/ROADMAP.md` Phase 2; per-system aspect quirks + per-game scaling override parked in `docs/PARKING_LOT.md`.
