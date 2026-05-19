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

## Controls

16-button layout defined in `apps/oa-shell/src/bindings.rs::mame`. The first 12 buttons are the arcade base layer (6 face buttons + d-pad + P1 START + P1 COIN) and map identity-style onto libretro RetroPad bits. The remaining 4 are Phase-1.5 system buttons (P2 START / P2 COIN / SERVICE / MAME_MENU) parked on otherwise-unused RetroPad bits.

**Base layer — arcade controls (keyboard defaults follow the cross-system "Z is primary" rule):**

| Button | libretro bit | Key | Pad | Notes |
|---|---|---|---|---|
| UP/DOWN/LEFT/RIGHT | UP/DOWN/LEFT/RIGHT | Arrows | DPad | 8-way stick |
| B1 (weak punch) | B | Z | South | Primary action |
| B2 (medium punch) | A | X | East | Secondary |
| B3 (strong punch) | Y | A | West | |
| B4 (weak kick) | X | S | North | |
| B5 (medium kick) | L | Q | LeftTrigger | |
| B6 (strong kick) | R | W | RightTrigger | |
| START (P1) | START | 1 | Start | RetroArch standard |
| COIN (P1) | SELECT | 5 | Select | RetroArch standard |

Street Fighter purists will want to remap (SF veterans expect LP/MP/HP on the top row, LK/MK/HK on the bottom) — use the per-system Bindings dialog.

**System buttons (Phase-1.5):**

| Button | libretro bit | Key | Purpose |
|---|---|---|---|
| SERVICE | L3 | F2 | MAME's operator Service / Test mode |
| MAME_MENU | R3 | Tab | Opens MAME's per-driver input config |
| P2_START | R2 | 2 | Player-2 START (placeholder — see note below) |
| P2_COIN | L2 | 6 | Player-2 COIN (placeholder — see note below) |

The libretro RetroPad bit assignments for SERVICE / MAME_MENU are vestigial. MAME's libretro core actually receives `F2` (Service) and `Tab` (per-driver menu) over `RETRO_DEVICE_KEYBOARD` via the Phase 2 keyboard-passthrough pump — the joypad bits in the per-system Bindings UI just keep the names addressable for an eventual remap-while-the-pump-runs feature.

P2_START / P2_COIN are similarly placeholders. Shell input is single-port (port 0) today; libretro's standard convention is that P2 controls arrive on port 1 with the *same* START / SELECT bits, not as new bits on port 0. The per-port wiring is a follow-up — these entries reserve the names so the UI can register them now.

### The TAB workflow

The MAME core ships its own per-driver input menu, reachable in-game by pressing `Tab` on the keyboard. The Phase 2 keyboard-passthrough pump (live as of `4aac0f5`) forwards `Tab` straight to the core. That's the entry point for anything beyond the base 6-button arcade layout:

- Pinball flippers (Williams, Bally tables) — per-driver flipper / nudge / start
- Mahjong / Hanafuda games — 30+ named keys per player
- Lightgun games (Operation Wolf, Lethal Enforcers) — point + trigger axis
- Driving (OutRun, Pole Position) — steering wheel + pedals
- Yokes (After Burner II) — analog stick variants
- Spinners and trackballs (Tempest, Marble Madness) — relative-motion devices

Remaps made through the TAB menu persist in MAME's per-driver config under `<appData>/cfg/<driver>.cfg`. They survive across launches of the same ROM set.

### Game focus

Some MAME drivers bind important keys to F1-F8 / Esc / digits / Backspace — these also drive OA hotkeys (save state, reset, screenshot, rewind, etc.). When the conflict bites, toggle **Tools → Game focus** (or press `Ctrl+G`). Game focus ON tells OA to keep its hotkeys quiet for the duration; the keyboard pump still delivers every key to the core. A small chip in the toolbar shows the live state. Toggle off (`Ctrl+G` again) when you want OA's hotkeys back.

Game focus is per-session (default OFF every launch). The keyboard-passthrough pump itself is per-system; the `keyboard_passthrough` field in `<appData>/systems/mame.json` overrides the compiled-in default (true) if a user wants to suppress key forwarding entirely.

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
