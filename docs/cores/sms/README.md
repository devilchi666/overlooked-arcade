# sms — Sega Master System

Onboarded 2026-05-19. Drives the Sega Master System (and Japanese Mark
III variants cataloged together with SMS in libretro-database) via the
libretro **Genesis Plus GX** core (`genesis_plus_gx_libretro.dll`) by
default. SMS was Sega's 8-bit home console (1985-1992 retail), main
competitor to the Nintendo Entertainment System. Z80-A main CPU + VDP
(derived from the TMS9918) + SN76489 PSG + optional Yamaha YM2413 FM
sound (Japan-region carts only).

OA wires the SMS cart path only. The Sega Mark III / SMS family also
hosted Master System Card / Card Catcher add-ons and the FM Sound Unit;
those bring up automatically through Genesis Plus GX when the loaded
ROM declares them. No expansion-hardware slug needed beyond `sms` itself.

## Upstream

- **Default core (this onboarding):** Genesis Plus GX — https://github.com/libretro/Genesis-Plus-GX
  - Long-standing libretro multi-Sega core covering SMS, Game Gear,
    Mega Drive, and Sega CD behind one .dll.
  - Buildbot path: https://buildbot.libretro.com/nightly/windows/x86_64/latest/genesis_plus_gx_libretro.dll.zip
  - License: non-commercial (Eke / Genesis Plus GX maintainers).
- **Alternates (per-system Cores override):**
  - `picodrive_libretro.dll` — lighter footprint, MD-first but also
    handles SMS/GG. Worth a swap on lower-spec hosts.
- **Vendored:** No. Same convention as Genesis — we install the
  prebuilt buildbot .dll into `<exe_dir>/cores/` and treat it as a
  black box. The dynamic-loading pivot (2026-05-16) severs binary-wide
  GPL propagation; the SMS-handling .dll stays under its own license.

## ROM format

- **`.sms`** — canonical raw SMS dump. Headerless binary; the SMS BIOS
  region check is by Mega Drive style header signature embedded near
  the end of the ROM (or by ROM CRC if the header is missing). Every
  modern dump set (No-Intro, TOSEC) ships .sms as primary.
- **`.bin`** — intentionally NOT registered. Collides with PCE-CD track
  files, Sega CD audio tracks, ColecoVision, and Atari 2600 dumps.
  Users with `.bin` SMS dumps rename to `.sms` (same convention as
  Atari 7800 `.bin` → `.a78` and Genesis `.bin` → `.md`).

## BIOS

- **Optional.** Genesis Plus GX runs without `bios.sms` — boot logo
  and region-lockout splash get skipped, but games launch normally.
- For era-correct boot behavior the operator can drop the canonical
  `bios.sms` into `<exe_dir>/system/`. Multiple region BIOSes exist
  (`bios_E.sms`, `bios_U.sms`, `bios_J.sms`); GPGX picks based on the
  loaded ROM's region byte.

## Native timing

- **NTSC (US/JP):** 59.92 Hz, 256×192 visible (some games use 256×224 on
  the extended VDP mode).
- **PAL (EU/BR):** 49.70 Hz, 256×192 visible.
- Genesis Plus GX reports per-loaded-ROM via `retro_system_av_info` —
  the renderer takes whatever dimensions the core hands it.
- 256×192 source on a 4:3 CRT is the canonical aspect; non-square pixels
  in 256×224 mode get a small letterbox under the OA `4/3` tile aspect.

## Input

7-button layout defined in `apps/oa-shell/src/bindings.rs::sms`.
Identity-mapped to libretro RetroPad bits. Two face buttons (Button 1
and Button 2) plus a Pause button that lived on the SMS console
hardware originally; Genesis Plus GX surfaces SMS Pause via libretro
`RETRO_DEVICE_ID_JOYPAD_START`, so the binding sits on bit 3.

| OA Button | libretro bit | Keyboard | Pad | Notes |
|---|---|---|---|---|
| UP/DOWN/LEFT/RIGHT | UP/DOWN/LEFT/RIGHT | Arrows | DPad | 8-way d-pad |
| B1 | B (0) | Z | East | **Primary action** (matches the cross-system "Z + East = primary" rule) |
| B2 | A (8) | X | South | Secondary action |
| PAUSE | START (3) | Enter | Start | SMS Pause (mapped from console hardware by GPGX) |

Per the cross-system "Z is primary" rule (locked by the
`z_is_the_primary_action_button_on_every_system` test), keyboard **Z**
fires Button 1 — the primary fire / jump key for most SMS titles
(Alex Kidd's punch, Wonder Boy's jump, Shinobi's primary attack).
Keyboard **X** is Button 2 (secondary — magic / weapon / kick).

## Current status (2026-05-19)

**Works:**
- Core resolves to `genesis_plus_gx_libretro.dll` via
  `default_core_dll_for_system("sms")`.
- 7-button input mapped through `bindings::sms_to_libretro_bits` (identity).
- Library scanner classifies `.sms` as sms.
- Theme accent: neon magenta at hue 340°, distinct from every other
  claimed hue (closest: NES 28° at ~48° distance on the wheel).

**Not yet validated:**
- Real game launch — needs operator validation against a known-good
  ROM. Suggested test ROMs: **Alex Kidd in Miracle World**, **Phantasy
  Star**, **Wonder Boy III: The Dragon's Trap**, **Sonic the Hedgehog
  (SMS)**, **Shinobi**.
- libretro-database hash matching against `metadat/no-intro/Sega - Master System - Mark III.dat`
  — wired but needs operator-run `Settings → Library → Identify ROMs`
  pass to confirm canonical title lookup.
- Cover sync via libretro-thumbnails `Sega_-_Master_System_-_Mark_III`
  — wired but needs operator validation.

## Per-core docs

- `ROADMAP.md` — phase tracking for SMS specifically.
- `SESSION_LOG.md` — Shipped / Almost / Next per session.
- `KNOWN_GAME_BUGS.md` — per-game compatibility issues.
- `DECISIONS.md` — SMS-specific integration choices.
