# Overlooked Arcade — External Advisor Brief

**Audience:** an outside advisor (e.g. an LLM consultant or a retro-community veteran) with no prior project context.

**Goal:** enough context that you can suggest (a) what features would make Overlooked Arcade stand out vs LaunchBox / BigBox / RetroArch / Pegasus / EmulationStation Desktop Edition / OpenEmu, and (b) what gaps in the current backlog warrant priority. The closing section enumerates specific questions where outside input would help most.

**Snapshot date:** 2026-05-25. The project moves fast — this document is a point-in-time read.

---

## 1. TL;DR

Overlooked Arcade ("OA") is a **premium, non-commercial emulator frontend** for the consoles modern emulators forgot — TG-16, Atari Lynx, 7800, Vectrex, Virtual Boy, MSX, ColecoVision, WonderSwan, Neo Geo Pocket, plus 30+ more popular ones added since launch. Built on a Tauri + wgpu + Solid stack with a libretro-frontend Rust core, it currently runs **40 systems** with per-system theming, save states, rewind/TAS, in-engine cheats, art-pack import, audio override, and a 27-slot LaunchBox-shape media taxonomy.

The project is a one-person effort with an LLM pair-programming partner. The author is making it as a **gift to the retro community** — no monetization, no telemetry, no analytics, GPL-2.0 binary today (license may move permissive after the dynamic-load pivot fully lands). The closest commercial competitor is LaunchBox/BigBox; the closest free competitor is Pegasus or ES-DE.

The strategic question: in a space dominated by RetroArch (technically deep, UX-thin) and LaunchBox (UX-premium, paywall for cabinet mode), where can OA carve a defensible identity?

---

## 2. Mission and positioning

**What OA is:**
- A polished, dedicated home for each system. Per-system theming (e.g. saturated gold for Jaguar, sherbet-purple for WonderSwan, gray-blue for Channel F). Operators see the system, not a generic "games" grid.
- A platform that handles bring-up details for systems most launchers ignore — BIOS pre-checks with canonical SHA-1 verification, byte-swap normalization for weird formats (N64 `.v64`/`.n64`), per-system keyboard passthrough (MAME / MSX / 5200), CD disc-id extraction for 8 CD-shape systems.
- An emulator shell that uses **upstream libretro cores dynamically loaded as DLLs**. We do not write our own emulators. We make the existing emulator landscape feel intentional rather than improvised.

**What OA is not:**
- Not commercial. No store, no subscriptions, no ads, no "premium tier."
- Not a RetroArch replacement. RetroArch is more technically capable for power users — netplay, run-ahead frame-buffering, cross-platform parity, achievements. OA aims at a different operator (the person who wants a polished home for their Jaguar collection, not the person who wants to tune scanline alignment to the frame).
- Not a ROM-management tool. We don't normalize, rename, or verify ROM sets against DAT files. We hash for cover-art lookup and that's it.
- Not multi-instance. One game at a time per OA process.

**Pillars (locked design decisions, not up for reconsideration):**
1. **Per-system experience** — every system gets a bring-up phase before it's considered ready. Bindings, BIOS check, theming, ROM-hash dat, libretro-thumbnails repo wiring, libretro core preference, KNOWN_GAME_BUGS file. Documented in `docs/cores/<id>/README.md`.
2. **Upstream cores with our patches when needed.** The "forked core" philosophy survived the dynamic-load pivot: when we want to modify a core, we maintain a libretro-frontend build of the patched source and ship that `.dll` in the installer.
3. **Top 80% of each system's library, not cycle accurate.** Cores already encode chip-level correctness; we don't second-guess upstream. If a game has a known bug, it goes in KNOWN_GAME_BUGS.
4. **One core at a time.** A single source of truth for "what we're working on" (`docs/ACTIVE_WORK.md`). Cross-cutting work lives under `docs/features/`.

---

## 3. Architecture (load-bearing summary)

**Stack:**
- **Backend:** Rust + Tauri 2 (window/shell), wgpu + WGSL (renderer), libretro `.dll` loaded via `libloading` (the `oa-libretro` crate is the entire FFI surface).
- **Frontend:** Solid + TypeScript + Tailwind + Vite. Heroic Games Launcher is the visual ceiling.
- **Audio:** cpal callback + rodio for non-emulator audio (UI sounds, platform music, ceremony, snap-audio).
- **Library:** SQLite (single file) for games + folders + folder rules + cheats + platform-media metadata.
- **Logs:** unified stderr + per-session file + in-memory ring buffer (read by `Help → Debug log…` dialog).

**Cores live next to the .exe** in `<exe_dir>/cores/` as `.dll` / `.so` / `.dylib`. Users can use community-built nightlies (`buildbot.libretro.com`) or our own DLL builds of patched source. BIOS files live in `<exe_dir>/system/`. User preferences (saves, bindings, audio overrides) live in `appData` because they're per-user, not per-install.

**Two shell modes, selectable at runtime:**
- **Single-window:** WebView + game render in the same window (modern default).
- **Two-window:** dedicated native game window + WebView library window (legacy / power-user mode).

**License posture:** the workspace is GPL-2.0 today. The dynamic-loading pivot severs binary-wide GPL propagation from any one core, so the shell could move to a permissive license once the installer ships with our own DLL builds. The GPL cores stay GPL inside their `.dll`. Repo is public from Day 1.

**Where third parties extend us:** there isn't a plugin API yet. The closest thing is the libretro `.dll` interface — drop a community-built core into `cores/` and OA will load it. Themes are not yet a plugin format (designed but not implemented; see `docs/features/kiosk-shell/KIOSK_PLAN.md`).

**Constraints worth knowing:**
- **WGSL-only shaders.** wgpu translates to DX12 / Vulkan / Metal / GL. Avoid features that don't translate cleanly to the GL fallback.
- **No network calls from emulator code.** The emulator runs fully offline. Network calls happen at the shell level (libretro-thumbnails sync, no-intro DAT fetch, libretro buildbot core download).
- **`libretro.h` ABI compatibility.** Adding a new device type means matching the upstream header byte-for-byte (e.g. `RETRO_DEVICE_LIGHTGUN` is id=4, can't be changed).

---

## 4. Capability snapshot — what works today

### Systems wired (40)

| Category | Systems | Status |
| --- | --- | --- |
| **8-bit / 16-bit consoles** | NES, SNES, Genesis, Master System, Game Gear, TG-16, PCE-CD, Channel F, ColecoVision, Intellivision, Odyssey², 2600, 5200, 7800, Vectrex | Phase 0+ for all; ~6 in Phase 1.5 polish |
| **32/64-bit consoles** | PSX, Saturn, N64, Sega CD, 32X, Neo Geo, Neo Geo CD, 3DO, Jaguar, PC-FX, Virtual Boy | Phase 0+ for all; CD systems have disc-id extraction shipped |
| **Modern consoles** | GameCube, Wii (via GC), Dreamcast, PSP, PS2 | Phase 0 wired; operator playtest pending |
| **Handhelds** | GB, GBC, GBA, Lynx, Pokémon Mini, NeoGeo Pocket, WonderSwan, Game Gear | Phase 0+; LCD-handheld shader default 2026-05-24 |
| **DS / dual-screen** | NDS | Phase 0; POINTER infra shipped for stylus |
| **Engine launchers** | DOSBox, ScummVM | Phase 1 shipped 2026-05-24; operator playtest pending |
| **Arcade / multi-game** | MAME, Neo Geo (ROM-set form) | Phase 1.5 hardening pending |

### Cross-system infrastructure (all shipped)

- **Save states** with multi-slot UI + thumbnails
- **Rewind ring** (zstd-compressed, configurable cap, hold Backspace for visual rewind)
- **TAS recording + replay** (input frames + pointer state captured)
- **Video capture** + memory inspector + milestones
- **Shader pipeline** — `ShaderPreset::{Plain, Scanlines, CrtLite, Phosphor, LcdHandheld}` + WGSL files + per-game/per-system override + hot-reload
- **Per-system settings page** — shader, bloom, aspect, overscan, bezel, region/revision priority, rewind config, analog routing, keyboard passthrough
- **Per-game settings drawer** — all of the above stack on top per-game; plus core_options map, patch path, keypad layout note
- **Core-option dynamic visibility** — libretro `SET_CORE_OPTIONS_DISPLAY` + the corresponding update callback are honored end-to-end
- **Library folders in SQLite** with drag-reorder, watch flag, per-folder rules
- **Shared analog input infrastructure** (Phases A–G) — per-button pressure, gilrs trigger axes, rumble interface with lazy-built effect handles, sensor interface (accelerometer / gyroscope / illuminance with keyboard-tilt fallback), mouse-as-stick analog source, per-port libretro device-type override
- **POINTER + LIGHTGUN device dispatch** (LIGHTGUN landed 2026-05-25; before that, every light-gun core silently got zeros)
- **Keyboard passthrough** + Game-Focus toggle (Ctrl+G). Default-on for `mame` / `msx` / `msx2` / `5200` / `scummvm`
- **BIOS pre-checks** for 9 CD systems + 10 cart-shape systems with canonical SHA-1 verification. Some are warn-on-missing (mGBA HLE fallback), some are block-on-missing (Jaguar's `jagboot.rom`)
- **Hash ROM identification** with `HeaderRule::ByteSwap` variant for N64 .v64/.n64 normalization
- **Multi-repo libretro-thumbnails sync** — gb has DMG + CGB repos, wonderswan has WS + WSC, gamecube classifies GC vs Wii via dump structure
- **27-slot LaunchBox-shape media taxonomy** — box-front/back/3d, cart, disc, screenshots, fanart, arcade-cabinet/marquee/controlpanel, video, music, manual, etc. Plus 9 per-platform slots (banner, clear-logo, console, controller, fanart, marquee, photo, wheel, background)
- **4-bus audio mixer** over rodio + symphonia (platform-music / ui-sounds / ceremony / snap-audio) with per-system and per-game overrides
- **Art-pack importer** — LaunchBox and EmuMovies layouts auto-detected, fuzzy-matched against library titles
- **Library scan + Import Wizard + folder watcher** — including ScummVM auto-detect from curated sentinel-filenames AND optional standalone `scummvm.exe --detect` CLI shell-out
- **Disc-id extraction** for pce-cd, segacd, saturn, psx, ps2, neocd, pcfx, gamecube, dreamcast (8 systems)
- **Per-system theming** — `frontend/src/themes/systems.css` + `registry.ts` with one CSS variable bundle per system
- **CJK font fallbacks** for PC-FX + FDS Japanese-only libraries
- **Multi-core CPU awareness** — rayon for ROM hashing + thumbnail generation, zstd-1 for rewind ring, parallel boot of independent disk reads, `spawn_blocking` for image decode/resize/encode
- **Direct-launch CLI** — `--system / --core / --rom / --slot / --state-file / --tas-replay / --fullscreen` for LaunchBox-style external launchers
- **Per-system cheat-code format declarations** — Game Genie / GameShark / Action Replay / CodeBreaker / Pro Action Replay etc. with operator-side regex validation per system
- **Cheat code engine** — SQLite-backed CRUD + frame-loop dispatch + libretro `retro_cheat_set` wiring + auto-arm on launch + cheat-search UI for memory scanning
- **Portable install** — drop a `portable.txt` next to the `.exe` and OA moves all per-user state (saves, bindings, library, logs) into `<exe_dir>/settings/`

### Recent ships (last 10 days)

Order matters here — this is the cadence to set advisor expectation:

- **2026-05-25** light-gun harness — RETRO_DEVICE_LIGHTGUN dispatch (was silently broken since NDS POINTER landed), 18 new tests, declarative LIGHT_GUN_SYSTEMS catalogue
- **2026-05-25** Import Wizard freeze fix — race condition in multi-job scan dispatch, plus logger always-flush so hard-kill preserves the diagnostic trail
- **2026-05-25** Jaguar KP8-KP_HASH keyboard-event dispatch — 5 high-bit keypad keys route through `retro_keyboard_event_t` (still awaiting operator playtest)
- **2026-05-24** ScummVM CLI mode — power-user toggle to use a standalone ScummVM install for full ~400-game catalogue detection
- **2026-05-24** ScummVM auto-detect — curated sentinel-filename heuristic for ~18 well-known SCUMM games
- **2026-05-24** Per-system cheat-code format declarations across 12 systems (NES, SNES, Genesis, etc.)
- **2026-05-24** DOSBox onboarding — directory-mode scan + per-game entry-point override + Import Wizard `systemHint` dispatch
- **2026-05-24** ScummVM onboarding — `.scummvm` descriptor parsing + per-core `system_dir` subdirectory + keyboard passthrough
- **2026-05-24** Full media taxonomy — 27 MediaKind variants, 9 per-platform slots, art-pack importer, existing-install migration, 4-bus audio mixer
- **2026-05-23** Portable install (opt-in via marker file)
- **2026-05-23** Window geometry persistence + tile-size slider
- **2026-05-22** Sidebar tier + view editor — manufacturers view, cross-container drag, accent picker, schema v2
- **2026-05-22** UI polish — menu-bar IA, dialog reorganization, window/tile persistence

The pace is roughly one major commit-arc per day, two on heavy days. The author tests + validates each before merge.

---

## 5. Backlog summary

### Code-ready (operator-independent, infra exists)

The HIGH band in `docs/NEXT.md` is empty as of this snapshot. The MEDIUM band:

1. **Vectrex `vector-phosphor` shader preset** (~250 lines WGSL + UI) — Gaussian glow on bright pixels (vector lines), optional persistence trail. Needs operator design input on glow radius + persistence half-life.
2. **Virtual Boy `vb-monochrome` shader** (~120 lines WGSL) — LED-grain noise + red-on-black tint + optional visor reflection. Needs operator design input.
3. **Jaguar KP keyboard-passthrough dispatch** — code SHIPPED on `feat/jaguar-keypad-passthrough` branch; awaiting operator playtest with canonical jagboot.rom.
4. **Multi-system light-gun smoke-test validation** — code SHIPPED on main 2026-05-25; awaiting operator playtest per system.

### Operator-driven (no code blocker; gated on playtest)

~21 systems sitting at "Phase 0/1 wired, awaiting playtest." Most-impact ones:
- **psx** — SotN, FF7 (.m3u multi-disc), MGS, Crash, Resident Evil
- **saturn** — NiGHTS, Guardian Heroes, Radiant Silvergun, Panzer Dragoon Saga (4-disc .m3u)
- **n64** — SM64, GoldenEye, Ocarina of Time, MK64, Smash 64
- **gamecube** — Smash Melee, Wind Waker, RE4, Metroid Prime, Pikmin, Wii ISO smoke-test
- **psp** — God of War: Chains of Olympus, Crisis Core, Patapon, MGS: Peace Walker
- **5200 + pokemini** — operator drops BIOS, launches flagship; full Phase 1 is just operator validation
- **dosbox + scummvm** — operator drops `.dll` + game data + validates

### Lower-band code work (~11 items)

- SNES Mouse + Super Multitap (~200 lines, niche — Mario Paint, Bomberman)
- O2 per-game keyboard-layout overlay UI (~150 lines — Quest for the Rings)
- Vectrex translucent overlay rendering + portrait aspect (~250 lines combined)
- NDS microphone input (~200 lines — Blow/voice puzzles)
- NDS per-game touch overlay UI + multi-touch
- Sega CD 3-button vs 6-button pad mode override
- SMS Light Phaser refinement (light-gun infra now shared)
- Genesis MD button-glyph polish (A/B/C diamond + 6-button shoulder)
- NGP-mono vs NGPC library-tile differentiation (~60 lines)
- PCFX FMV streaming validation (operator)

### Deferred (gated on larger infra not yet triggered)

- GameCube Wii Remote / Nunchuk / Classic Controller dispatch (~500 lines, new libretro device type)
- Dreamcast VMU peripheral (~400 lines, secondary screen)
- Real OS-level accelerometer access (~250 lines, Windows Sensor API / Linux iio / macOS Core Motion) — today GBA Boktai / Kirby Tilt 'n' Tumble use keyboard-arrows-as-tilt fallback
- Custom-built Vectrex vector renderer (~500 lines, native wgpu vector-stroke)
- Modern VR for Virtual Boy via OpenXR (~800 lines)
- Jaguar CD support (~300 lines)
- 32X-CD games
- ST-V arcade variant of Saturn

### Large unstarted scope: Kiosk shell

A full BigBox-class kiosk mode is **design-locked but not implemented**. Phase 0 (desktop polish) shipped 2026-05-22; Phases 1-7 (full 7-phase plan) is the unstarted scope. Locked spec lives at `docs/features/kiosk-shell/KIOSK_PLAN.md`. Includes:

- **Phase 1** MVP: `--kiosk` flag, wheel nav, snap video + audio on hover, launch ceremony v1, in-game menu v1, controller binding wizard, first-launch onboarding, perf budgets (4ms render @ 144Hz, button→ack <50ms, cold launch <2s, resident memory ≤500MB)
- **Phase 2** Theme substrate — TOML layout + animations + Rhai scripts + shaders, Rhai sandbox, in-engine Theme Studio editor, `.oatheme` archive + signing, 3 reference themes (Showcase / Minimalist / Cabinet)
- **Phase 3** Library depth — 5 nav patterns + custom escape hatch, predictive search, filter sheet, playlists + DSL, named arbitrary-hierarchy views, recents/favorites/most-played rails
- **Phase 4** Audio — 5-bus mixer + ducking matrix + theme curves, multi-monitor surfaces, attract Tier 1 (snap cycle), monitor assignment + test pattern
- **Phase 5** Kid mode (locked-to-subset, PIN exit), accessibility floor (7 surfaces: reduced motion, UI scale, high-contrast, hold thresholds, snap-audio captions, single-switch mode, TTS announcements)
- **Phase 6** Distribution — theme Index repo, in-app browser, sandbox security audit
- **Phase 7** Advanced — Attract Tier 2 (pre-recorded capture), Tier 3 (live emulation), phone-companion search, multi-user profiles, single-switch mode

Kiosk shell is a multi-month effort. Picking it up now would dominate the roadmap for a quarter. The author is uncertain whether to push on it next or keep the desktop-mode polish loop tight.

### Cross-cutting features that exist but haven't shipped

- **Region badges + publisher logos** — fully designed in `docs/PARKING_LOT.md`, just needs an asset-download pass + license check on LaunchBox / EmuMovies / Flagpedia art packs. Operator validated the design.
- **Sidebar v3.4 per-container art slots** — designed in `docs/features/sidebar/VIEW_EDITOR_PLAN.md` §0.8 + §4. Blocked on the kiosk theming substrate.

---

## 6. Competitive landscape

OA sits between two well-established categories.

### The premium-UX category

**LaunchBox + BigBox** (Windows; LaunchBox free, BigBox paid)
- Strengths: comprehensive metadata DB (LaunchBox Games Database), polished BigBox cabinet mode, video snaps everywhere, mature platform-art ecosystem, drag-drop emulator dispatch to any external emulator, RetroAchievements integration.
- Weaknesses: closed-source, paid for cabinet mode, Windows-only, slow boot, monolithic monolithic monolithic — does everything in one process.
- Closest to OA in visual ambition. Different audience: LaunchBox optimizes for the "I have 50K games and want to launch them all" power user. OA optimizes for the "I want a polished home for my Jaguar collection" curator.

**EmulationStation Desktop Edition (ES-DE)** (Linux-first, Windows + macOS available)
- Strengths: themeable, lots of community themes, batocera + retropie roots, free.
- Weaknesses: visual ceiling is lower than LaunchBox; theme format is dated; default UX is functional but not premium.
- Free competitor closest to OA's visual aim.

**OpenEmu** (macOS only)
- Strengths: Cocoa-native polish, beautiful, free, open-source.
- Weaknesses: macOS only, system support narrower than OA, has stalled in recent years.

**Pegasus** (cross-platform)
- Strengths: Lua/QML themable, modern UX, free.
- Weaknesses: smaller community than LaunchBox/ES-DE, configuration is hand-edited TOML, fewer built-in features.

### The technical-depth category

**RetroArch** (the libretro frontend)
- Strengths: every libretro core works, run-ahead frame-buffering for latency reduction, netplay, RetroAchievements built-in, shader presets shipped at depth, cross-platform parity (Switch / Vita / Wii U / Android / iOS / web / steam-deck), supports controllers OA doesn't even know about.
- Weaknesses: UX is divisive at best (the menu is a polarizing topic in the retro community), per-game configuration is a labyrinth, OS-specific bring-up details bleed through.
- This is what OA loads under the hood. We use libretro cores; we don't replace RetroArch's job of *being* a libretro frontend, we just make using one feel different.

**Mednafen, standalone Snes9x, DuckStation, Dolphin, Cemu, RPCS3, etc.** — single-system emulators with their own UIs. OA doesn't compete; OA shells around the libretro-port versions of these where they exist.

### Where OA's identity could form

- **Per-system home, not unified library.** Theme + bindings + BIOS + shader + cover-art pipeline per system. Most launchers theme the WHOLE library; very few theme per-system the way OA does. This is genuinely uncommon.
- **Cohesive bring-up.** A new system in OA gets the same 8-step recipe: SystemId variant, parse_system_id arm, bindings module, default core, rom_hashes, media repo, theme, per-core docs. This consistency is invisible from outside but pays compound interest as systems are added.
- **Forked-core philosophy preserved via DLL shipping.** When upstream Beetle PCE Fast doesn't fix a bug, we patch our fork, build, ship our DLL in the installer. This is rare — most frontends just use whatever the libretro buildbot puts out.
- **Non-commercial intent.** No telemetry, no analytics, no monetization, no "premium" gate. Repo is public from Day 1. This matters to a chunk of the community that's burned by LaunchBox's BigBox paywall.
- **Modern Rust stack.** Most competitors are C++ or Qt-era frameworks. The Rust + Tauri choice is unusual and may matter to a specific kind of contributor / power user.

### Where competitors are still ahead

- **Metadata database depth.** LaunchBox Games Database is the gold standard. We do hash-matched cover sync from libretro-thumbnails but don't have descriptions, ratings, developer logos, screenshots-per-region at that scale.
- **Netplay.** RetroArch has it; we don't.
- **RetroAchievements.** RetroArch has it; we don't.
- **Theme ecosystem.** Pegasus + ES-DE have community theme galleries. OA's theming is system-scoped and lacks a sharing format (designed; not shipped — kiosk shell Phase 2).
- **Mobile + handheld targets.** RetroArch ships everywhere. OA is Windows / Mac / Linux desktop only, no plan to expand.

---

## 7. Open strategic questions for the advisor

These are the genuine forks in the road. The author has opinions; outside input would be valuable.

### Q1. Kiosk shell now, or desktop polish first?

The kiosk shell is fully designed and would be a defensible BigBox alternative for cabinet builders. But it's a multi-month effort that would dominate the roadmap.

Alternative: keep iterating on desktop polish — sidebar v3.4 (per-container art slots), region badges + publisher logos, per-system shader curation, the deferred Lower-band items. Smaller wins, faster cadence, but doesn't open a new audience.

**What's the advisor's call?**

### Q2. What's the lowest-hanging-fruit competitive feature we're missing?

Candidates (rough effort estimates):
- **RetroAchievements integration** (~3-4 weeks). Free RA API, well-documented, would close one of two big RetroArch gaps. Community demand is high.
- **Built-in cover-art editor** (~2 weeks). LaunchBox has this; we currently rely on operator-supplied + libretro-thumbnails sync.
- **Per-game pause-menu shortcut keys** (~1 week). Hotkey to launch cheat editor, save state, screenshot, etc. without leaving the game.
- **Library export to LaunchBox-XML** (~1 week). Lets dual-use operators sync their OA library to LaunchBox for the metadata they don't have here.
- **Game-info popovers with rich metadata** (~2 weeks). Hover a tile, see year/developer/publisher/players/genre.
- **Per-system playlists or "favorites" rails** (~1 week). Personal curation surfaces.

**Which has highest "moves the needle" per week of effort?**

### Q3. Should we add netplay?

RetroArch has it; we don't. It would be a multi-month effort (libretro's netplay API + UI for connection management + tested across cores). Niche audience but devoted.

Counter-argument: RetroArch already does netplay well. We'd ship a worse version for years before catching up. Maybe the right call is "if you want netplay, use RetroArch — OA is for the offline single-player experience."

**Does shipping mediocre netplay help or hurt the brand?**

### Q4. Underserved retro communities — who would specifically benefit from OA?

Some candidates:
- **Handheld-focused collectors** — OA's per-system theming + LCD-handheld shader could be best-in-class for a GBA + Lynx + WonderSwan curator.
- **Arcade-cabinet builders** — once kiosk shell ships, this becomes a real audience.
- **Vector-graphics enthusiasts** (Vectrex, Asteroids arcade cabinets) — almost no launcher gives Vectrex a first-class home today.
- **Obscure-system collectors** (PC-FX, FM Towns, Apple IIgs, MSX2+, etc.) — the explicit OA mission.
- **Tournament organizers** — TAS recording + frame-perfect replay + save-state sharing could power local-tournament workflows.

**Is one of these audiences worth specifically targeting?**

### Q5. What makes a current LaunchBox/BigBox user switch?

LaunchBox is the closest visual competitor. Most users won't switch unless OA does something LaunchBox can't or won't:
- Per-system theming at a level LaunchBox doesn't offer
- Open-source code they can audit / fork / extend
- Bundled high-quality shipping experience (no separate emulator setup)
- A specific killer feature LaunchBox lacks

**What's the most LaunchBox-defector-friendly killer feature?**

### Q6. Theme ecosystem — design now, or wait?

The kiosk shell Phase 2 plan includes a full TOML + Rhai theme substrate with a Theme Studio editor and a `.oatheme` archive format. Themes would be community-shareable via a federated index repo.

But: launching a theme ecosystem requires (a) the format being stable, (b) the studio being usable, (c) a critical mass of themes before users find it useful, (d) ongoing maintenance of the index.

Alternative: stay with per-system CSS hardcoded in the repo. Operators who want different aesthetics can patch the CSS in their build. This is simpler but caps the community-contribution surface.

**Is the theme-ecosystem ambition worth the complexity, or is per-system CSS enough?**

### Q7. License posture — stay GPL, or go permissive?

Today: GPL-2.0 binary-wide. The dynamic-load pivot means we COULD relicense the shell permissively once the installer ships only our own DLL builds (which can choose any license themselves). GPL cores stay GPL inside their .dll.

Going permissive (MIT/Apache) would let companies build commercial products on top of the OA shell. Mission-aligned (we're a gift to the community) or mission-eroding (commercial actors profit on the author's gift)?

**What's the right call given the mission?**

### Q8. What's the right LLM-pair-programming workflow for this kind of project?

Meta-question. The author uses an LLM (Claude) as a pair-programming partner. The project moves fast as a result (one major commit-arc per day). What practices from this workflow generalize? What pitfalls have emerged? The advisor seeing OA's pace + quality might have suggestions for what other projects could borrow.

---

## 8. How to give feedback

If you're advising on this brief, the most useful response shapes are:

1. **Pick 2-3 questions from §7 and answer them concretely.** Don't try to address everything; depth beats breadth.
2. **Identify a gap in the capability snapshot (§4) that an external user would notice but the author may have normalized.** Outsider perspective is most valuable on things the author can't see.
3. **Propose one feature that doesn't exist in any of the listed competitors.** Originality is welcome — OA is non-commercial, so unconventional ideas with no obvious business model are fair game.
4. **Critique the strategic positioning (§2).** Where is OA's identity diffuse vs sharp?
5. **Flag any risk or technical-debt concern** the brief glosses over.

The author reads everything but doesn't promise to act on everything. Useful feedback is the kind that makes the author *think*, not just *do*.

---

## 9. Reference paths (for advisors with repo access)

If the advisor has access to the actual repo (https://github.com/devilchi666/overlooked-arcade):

- **Project mission + workflow**: `CLAUDE.md` at the repo root
- **What's in flight**: `docs/ACTIVE_WORK.md`
- **Cross-system priority queue**: `docs/NEXT.md`
- **Per-system status**: `docs/cores/<id>/ROADMAP.md` (40 files)
- **Cross-cutting features**: `docs/features/<name>/` (7 folders)
- **Out-of-scope ideas**: `docs/PARKING_LOT.md`
- **Project-wide design decisions**: `docs/DECISIONS.md`
- **Kiosk shell design**: `docs/features/kiosk-shell/KIOSK_PLAN.md` (multi-thousand-line locked spec)
- **Competitor analysis**: `docs/RESEARCH/launcher-landscape.md` + `docs/RESEARCH/launchbox-forum-feature-survey.md` + `docs/RESEARCH/retroarch-feature-survey.md`
- **Setup plan (off-repo)**: `C:\Users\Devilchi\.claude\plans\jazzy-spinning-blum.md` — the full Cargo layout, Tauri+wgpu integration approach, license discussion, build/dev workflow, phase plan, and risk list.
