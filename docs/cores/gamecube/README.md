# gamecube — Nintendo GameCube + Wii

Onboarded 2026-05-20 (paired with n64). Drives both the Nintendo
GameCube (2001-2007) and Nintendo Wii (2006-2017) via the libretro
**Dolphin** core (`dolphin_libretro.dll`). Single slug per the wiring
plan — Dolphin auto-detects hardware variant from disc container
shape.

The GameCube was Nintendo's 2001 6th-gen console; the Wii was its
2006 motion-controlled successor. Combined library: ~600 GameCube
retail releases + ~900 Wii retail releases. Standout titles:
**GameCube** — Super Smash Bros. Melee, Metroid Prime 1+2, Resident
Evil 4, Wind Waker, F-Zero GX, Eternal Darkness, Pikmin 1+2,
Animal Crossing. **Wii** — Super Mario Galaxy 1+2, Twilight Princess,
Skyward Sword, Wii Sports, Mario Kart Wii, Smash Bros Brawl, Monster
Hunter Tri, Xenoblade Chronicles.

## Upstream

- **Default core:** Dolphin libretro — https://github.com/libretro/dolphin
- **Vendored:** No.

## ROM format

- **GameCube formats:** `.iso`, `.gcm`, `.gcz` (Dolphin's compressed
  format), `.rvz` (Dolphin's modern compressed format).
- **Wii formats:** `.wbfs` (Wii ISO container), `.iso` (raw Wii disc).

Per-folder Import Wizard disambiguates `.iso` collisions against
PSX/3DO/Saturn libraries. Dolphin reads the disc header at load time
to determine GC vs Wii vs Triforce arcade hardware.

## BIOS

**None required.** Dolphin synthesizes the firmware behavior for
both GameCube and Wii internally. No external IPL.bin or boot ROM
needed.

## Native timing

- **GameCube NTSC:** 59.94 Hz, 640×480 (480p mode for HD-capable
  hardware; 640×448 for 480i CRT mode).
- **GameCube PAL:** 50 Hz.
- **Wii:** 480i/480p/720p (some games); Dolphin upscales internally.

Heavy CPU + GPU + 64-bit host required.

## Input

12-button digital layout + dual analog sticks + analog triggers (via
`InputState.axes`). Defined in
`apps/oa-shell/src/bindings.rs::gamecube`.

| GC button | libretro bit | Keyboard | Gamepad |
|---|---|---|---|
| A (primary) | B (0) | Z | East |
| B (secondary) | Y (1) | X | South |
| X (east face) | A (8) | A | West |
| Y (north face) | X (9) | S | North |
| L (analog trigger) | L (10) | Q | LeftTrigger |
| R (analog trigger) | R (11) | W | RightTrigger |
| Z (digital trigger) | R2 (13) | Space | RightTrigger2 |
| START | START (3) | Enter | Start |
| UP/DOWN/LEFT/RIGHT | (4-7) | Arrows | DPad |

**Analog axes:**
- Main stick = `InputState.axes[0..2]` (gamepad LeftStick).
- C-stick = `InputState.axes[2..4]` (gamepad RightStick).

Analog L/R triggers ARE pressure-sensitive on real GC pads but the
libretro mapping treats them as digital (Dolphin synthesizes pressure
from digital press level).

**Wii Remote / Nunchuk / Classic Controller / motion-controls** are
NOT covered by this binding layout — those ship as Phase 2.5 work
alongside the full per-system analog Bindings UI.

## Current status (2026-05-20)

Phase 0 onboarded. Awaits operator validation. The new analog input
infra (shipped this session) makes the GC pad's main stick and
C-stick playable on a connected gamepad.

**Test discs:** GameCube — Super Smash Bros. Melee, Wind Waker,
Resident Evil 4, Metroid Prime, Pikmin. **Wii (.wbfs)** — Wii Sports
(menu navigation only via classic-controller-style input at Phase 0;
motion gameplay is Phase 2.5).

## Per-core docs

- `ROADMAP.md`, `SESSION_LOG.md`, `KNOWN_GAME_BUGS.md`, `DECISIONS.md`.
