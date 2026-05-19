# mame — Arcade (MAME)

Onboarded 2026-05-19. Drives arcade hardware via the libretro MAME core
(`mame_libretro.dll`). One core covers a long tail of arcade boards —
Capcom CPS1/2/3, SNK Neo Geo, Cave shmups, Williams classics, Nintendo
VS, etc. — with the .zip ROM-set name identifying which game/board.

## Upstream

- **Source:** https://github.com/libretro/mame
- **Buildbot:** https://buildbot.libretro.com/nightly/windows/x86_64/latest/mame_libretro.dll.zip
- **License:** GPL-2.0+ for the libretro wrapper; MAME-core itself ships under the MAME license (BSD-3-Clause for most code with assorted exceptions per-driver — see upstream `LICENSE.md`).
- **Vendored:** No. MAME is too large to vendor; we install the prebuilt buildbot .dll into `<exe_dir>/cores/` and treat it as a black box.

Alternate cores (also in the catalog, all libretro):

- `mame2003_plus_libretro.dll` — community-maintained MAME 2003 line, lighter perf, broader compat with old/weird boards.
- `mame2010_libretro.dll`, `mame2003_libretro.dll`, `mame2000_libretro.dll` — older snapshots for very weak hardware.

## ROM format

MAME ROM-sets are **`.zip` archives** named after the game's short ID (`pacman.zip`, `dkong.zip`, `sf2ce.zip`, etc.). The zip contains hardware-specific binary blobs (PROM dumps, sprite ROMs, sound ROMs) with no standardized extension. Some games (Killer Instinct, etc.) ship as `.chd` for CD/DVD/HDD-backed boards.

Sets must match the MAME version they were dumped against — a 0.78 set won't run on MAME 0.287. The libretro buildbot ships current MAME; collect ROMs targeting the same generation.

OA's library scanner peeks inside `.zip` archives first. For zips containing recognized inner extensions (`.nes`, `.smc`, etc.) the archive is reclassified to that system. MAME zips fall through to MAME by elimination because they don't contain a recognizable inner extension.

## Native timing

Varies by arcade board. The core reports its own `retro_system_av_info` per loaded set:

- 60 Hz is typical (NTSC operator boards).
- 57.5 Hz on some boards (e.g. Pac-Man-era hardware).
- Resolution ranges from 256×224 (Capcom/SNK) up to 768×576 (some Cave / late Capcom).

The renderer uses whatever the core reports; the scaling-mode picker handles non-standard aspect ratios.

## Input

12-button arcade layout (6 face buttons + d-pad + START + COIN), defined in `apps/oa-shell/src/bindings.rs::mame`. Maps identity-style to libretro RetroPad bits.

**Keyboard defaults (per the cross-system "Z is primary" rule):**

| Button | Key | Notes |
|---|---|---|
| UP/DOWN/LEFT/RIGHT | Arrows | 8-way stick |
| B1 (weak punch) | Z | Primary action |
| B2 (medium punch) | X | Secondary |
| B3 (strong punch) | A | |
| B4 (weak kick) | S | |
| B5 (medium kick) | Q | |
| B6 (strong kick) | W | |
| START (P1) | 1 | RetroArch standard |
| COIN (P1) | 5 | RetroArch standard |

**Gamepad defaults:** 6 face buttons map to East/South/West/North + LeftTrigger/RightTrigger; Start/Select for P1 Start/Coin.

Street Fighter purists will want to remap (SF veterans expect LP/MP/HP on the top row, LK/MK/HK on the bottom) — use the per-system Bindings dialog.

## Current status (2026-05-19)

**Works:**
- Core loads via `mame_libretro.dll` (v0.287 verified).
- 12-button arcade input mapped through `bindings::mame_to_libretro_bits` (identity).
- Library scanner classifies non-system `.zip` files as MAME.

**Not yet validated:**
- Real game launch — needs operator validation against a known-good ROM set matched to MAME 0.287.
- `.chd` arcade games (KI, etc.) — extension is registered but not exercised.
- Multi-button games beyond 6 face buttons (P2 simultaneous play, more than 4 directional inputs) — port 0 only today.
- BIOS-required boards (Neo Geo via MAME, etc.) — Neo Geo BIOS `neogeo.zip` goes alongside the ROM set when MAME drives it (vs. FBNeo's `system/` placement).

## Per-core docs

- `ROADMAP.md` — phase tracking for MAME specifically.
- `SESSION_LOG.md` — Shipped / Almost / Next per session.
- `KNOWN_GAME_BUGS.md` — per-game compatibility issues.
- `DECISIONS.md` — MAME-specific integration choices.
