# dreamcast Decisions Log

Append-only. Newest at the bottom.

---

## 2026-05-20 — Flycast as default

**Decision:** `flycast_libretro.dll`. Redream is a per-system
alternate but not always packaged by libretro buildbot.

**Why:** Flycast is the canonical libretro Dreamcast core — actively
maintained, broad compatibility, also handles Naomi arcade hardware.
Redream is lighter on weak hardware but less mature on libretro side.

---

## 2026-05-20 — Separate dreamcast SystemId (Sega family completion)

**Decision:** `dreamcast` SystemId. Completes the Sega family
(genesis / segacd / sega32x / sms / gamegear / saturn / dreamcast —
all seven Sega home/handheld platforms now wired).

**Why:** Standard "every console gets its own home" pattern. The Sega
family in OA's lineup now spans 7 distinct platforms with their own
sidebar entries, themes, BIOSes, and library shelves.

---

## 2026-05-20 — DC orange swirl 32° theme (warm zone, highest chroma)

**Decision:** `[data-system="dreamcast"]` ships `oklch(0.55 0.27 32)` —
period-correct DC orange swirl. Highest chroma in OA's warm zone
(C=0.27 ties neogeo for the highest). Slots between NES 28° crimson
and sega32x 42° neon orange.

**Why:** Period-correct to the iconic Dreamcast spiral logo +
9/9/99 launch marketing. The DC swirl is one of the most-recognized
console-era logos; making Dreamcast visually orange is essential to
the platform's identity.

The warm zone now hosts 12 systems in 73° (most-crowded cluster in
OA's lineup — VB/MAME/neogeo/ChannelF/NES/dreamcast/sega32x/neocd/
TG-16/2600/jaguar/A7800), but the L+C profile of each system
distinguishes them:
- dreamcast 32° L=0.55 C=0.27 — saturated orange swirl
- NES 28° L=0.62 C=0.22 — brighter, less saturated crimson
- sega32x 42° L=0.68 C=0.22 — neon orange, brightest L
- neogeo 18° L=0.50 C=0.27 — same chroma as DC but deepest red hue

Operator accepted the cluster crowding for period-correctness — same
precedent as saturn (deepest purple) and jaguar (saturated gold in
the Atari-warm zone).

**Considered and rejected:**
- **Plan B: Sega family cool blue at 255°.** Would extend the Sega
  family into the post-Genesis purple cluster but loses the iconic
  orange swirl identity. Defeated by period-correctness.
- **Plan C: Open-band yellow-green at 118°.** Cleanest separation
  but no brand anchor. Defeated.

---

## 2026-05-20 — Four-entry BIOS table (boot + 3 regional flash)

**Decision:** `DREAMCAST_BIOS_KNOWN_HASHES` ships with 4 entries —
`dc_boot.bin` (universal v1.01d) + `dc_flash.bin` US/JP/EU regional
variants.

**Why:** The boot ROM is region-agnostic; the flash file is region-
locked. Operators install one of each. Four entries cover ~95% of
real-world DC BIOS combinations.

---

## 2026-05-20 — Analog stick via shared analog infra (no per-system bindings)

**Decision:** Dreamcast's single analog stick flows through
`InputState.axes[0..2]` (gamepad LeftStick) via the cross-cutting
analog input infra shipped earlier today (n64 + gamecube session).
No analog entries in the bindings module.

**Why:** Same precedent N64 + GameCube set — analog axes don't live
in the per-system bindings table at Phase 0. Phase 2.5 polish adds
per-axis keyboard binding (for keyboard-only users) and stick-swap
options (for operators whose pads are reversed).

L/R analog triggers map to digital libretro L/R bits at Phase 0;
analog-pressure sensitivity is Phase 2.5 (shared deferral with
GameCube's analog L/R triggers).

---

## 2026-05-20 — No SELECT in the bindings module

**Decision:** The Dreamcast pad has no SELECT button; the bindings
module reflects this with no SELECT entry.

**Why:** Period-correct to the real DC controller. Some emulators
expose a "SELECT" slot anyway for menu/cheat shortcuts, but OA's
per-system Bindings page lets operators wire menu shortcuts to any
key without needing a phantom SELECT entry.
