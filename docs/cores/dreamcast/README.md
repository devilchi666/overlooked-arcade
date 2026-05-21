# dreamcast — Sega Dreamcast

Onboarded 2026-05-20. Drives the Sega Dreamcast via the libretro
**Flycast** core (`flycast_libretro.dll`). **Completes the Sega
family** — OA now hosts all six Sega home/cart consoles (Master
System, Game Gear, Genesis, Sega CD, 32X, Saturn, Dreamcast — seven
counting GG; nine if you count the SuperGrafx-era era).

The Dreamcast was Sega's 1998 (JP) / 1999 (US/EU) 128-bit CD-ROM
console — the final Sega console and one of the most beloved
"overlooked" platforms in retro gaming. Sega exited the console
hardware business in 2001, ending the Dreamcast's commercial life
early but cementing its cult-classic status. ~620 retail releases
worldwide. Standout titles: Shenmue I+II, Sonic Adventure 1+2,
Soulcalibur, Jet Set Radio, Crazy Taxi, Power Stone, Dead or Alive 2,
Resident Evil: Code Veronica, Phantasy Star Online, Skies of
Arcadia, Grandia II, Marvel vs. Capcom 2, Cannon Spike, Ikaruga
(GD-ROM Naomi port), Bangai-O, Space Channel 5.

## Upstream

- **Default core:** Flycast — https://github.com/libretro/flycast
- **Alternates:** `redream_libretro.dll` (not always packaged for
  libretro buildbot).
- **Vendored:** No.

## ROM format

Dreamcast games shipped on Sega's proprietary GD-ROM (Gigabyte Disc
ROM) format — 1.2 GB optical discs. Modern dumps use:

- **`.cdi`** — DiscJuggler image (most-common community dump format
  pre-2010).
- **`.gdi`** — Modern GD-ROM Image (track-list + per-track .bin
  files; preserves the high-density GD area + audio tracks).
- **`.chd`** — MAME-derived compressed CD container (also handles
  GD-ROM). Single-file convenience; cross-CD-system collision via
  per-folder Import Wizard rule.

`.cdi` and `.gdi` are Dreamcast-unique — no per-folder rule needed.

## BIOS

**Required.** Pre-checked by `check_dreamcast_bios` in main.rs
(slots into the CD-launch BIOS dispatch arm as the 8th CD-shape
system):

| Filename | Description |
|---|---|
| `dc_boot.bin` | Boot ROM v1.01d (universal — same across regions) |
| `dc_flash.bin` | Flash RAM (256 KB, region-specific: US/JP/EU) |

The boot ROM is region-agnostic; the flash file carries region-
locking + clock-region defaults + per-region BIOS strings. Operators
pick the flash matching their disc region.

## Native timing

- **NTSC:** 59.94 Hz, 640×480 (480p native — DC was the first
  console with native 480p support).
- **PAL:** 50 Hz, 640×480.

CPU + GPU intensive; not as heavy as GC/Wii but heavier than 5th-gen.

## Input

11-button digital layout + analog stick (via `InputState.axes`).
Defined in `apps/oa-shell/src/bindings.rs::dreamcast`.

| DC button | libretro bit | Keyboard | Gamepad |
|---|---|---|---|
| A (south face, primary) | B (0) | Z | East |
| B (east face, secondary) | A (8) | X | South |
| X (west face) | Y (1) | A | West |
| Y (north face) | X (9) | S | North |
| L (analog trigger) | L (10) | Q | LeftTrigger |
| R (analog trigger) | R (11) | W | RightTrigger |
| START | START (3) | Enter | Start |
| UP/DOWN/LEFT/RIGHT | (4-7) | Arrows | DPad |

No SELECT — the Dreamcast pad doesn't have one. Single analog stick =
`InputState.axes[0..2]` (gamepad LeftStick), flowing through the
cross-cutting analog input infra shipped with the n64+gamecube pair.

VMU peripheral (memory card with screen) + light gun support
(House of the Dead 2, Confidential Mission) deferred to Phase 2.5.

## Current status (2026-05-20)

Phase 0 onboarded. **Completes Sega family wiring** (Wave 2 of the
system-wiring-plan now fully done).

**Test discs:** Sonic Adventure (DC launch flagship), Crazy Taxi,
Jet Set Radio, Power Stone, Soulcalibur (canonical "is Dreamcast
working" test — released alongside the platform launch).

## Per-core docs

- `ROADMAP.md`, `SESSION_LOG.md`, `KNOWN_GAME_BUGS.md`, `DECISIONS.md`.
