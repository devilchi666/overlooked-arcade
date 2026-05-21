# sms Decisions Log

Sega Master System-specific integration choices. Project-wide decisions
live in `docs/DECISIONS.md`. Append-only; newest at the bottom.

---

## 2026-05-19 — Genesis Plus GX as the default SMS core

**Decision:** `default_core_dll_for_system("sms")` returns
`"genesis_plus_gx_libretro.dll"`.

**Why:** Genesis Plus GX is the de-facto libretro multi-Sega core —
SMS, Game Gear, Mega Drive, and Sega CD all live behind one .dll. For
an operator onboarding SMS + Game Gear together (Wave 1 paired
onboarding 2026-05-19), the same install services both slugs without
needing a second .dll. Mature, well-tested, broad compatibility.

**Considered and rejected:**
- **PicoDrive as default.** Lighter footprint, MD-first but also
  handles SMS/GG. Defeated by Genesis Plus GX's broader SMS test
  coverage + the multi-Sega install-once value. PicoDrive stays in
  the catalog as the lightweight alternate.

Users wanting PicoDrive can swap via the per-system Settings → Cores
override; the swap is one click and persists in `<appDataDir>/cores.json`.

---

## 2026-05-19 — 7-button layout (no SELECT)

**Decision:** `SMS_BUTTONS` ships 7 entries — 4-way d-pad + B1 + B2 +
PAUSE. No SELECT button.

**Why:** SMS hardware has no Select button. Pause lived on the console
hardware (not the controller), and Genesis Plus GX maps it to libretro
`RETRO_DEVICE_ID_JOYPAD_START` — so the binding sits on bit 3 (libretro
START), labeled "PAUSE" for operator clarity. The libretro SELECT bit
(bit 2) stays unbound rather than being aliased to anything meaningful,
which keeps the per-system Bindings UI honest about what the hardware
actually offered.

This differs from Atari 7800's 8-button layout (which adds a SELECT
binding for the 7800's hardware Select switch) — the two systems share
similar 2-button face shapes but the SMS controller is simpler.

---

## 2026-05-19 — Neon magenta accent at hue 340°

**Decision:** `[data-system="sms"]` ships
`oklch(0.65 0.22 340)` — neon magenta. Pairs well against the OA dark
surface; soft variant lifts to a desaturated pink for text-on-color
contrast.

**Why:** The 1986-1990 Western Big Box era (the US/EU launch packaging
of the SMS) used a distinctive black-with-neon-grid-floor box art —
saturated hot pink/magenta on dark with a cyan grid. Picking 340°
captures that era-specific palette while staying clearly distinct from
every other claimed hue (closest neighbor: NES 28° at ~48° distance on
the wheel). Chroma 0.22 matches the saturation of the era's marketing.

**Considered and rejected:**
- **Saturated teal (~190°).** Sega launch-era cyan-teal. Defeated by
  being visually closer to PCE-CD's silver-cyan (220°) — less
  period-correct for SMS specifically.
- **Deep crimson (~5°).** Sega's red logo. Defeated by collision with
  MAME scarlet (12°) and NES red (28°).

---

## 2026-05-19 — Register `.sms` only; exclude `.bin`

**Decision:** SMS registry extension list is `["sms"]`. Headerless
`.bin` dumps intentionally excluded.

**Why:** Same collision problem as Atari 7800 and Genesis already
navigated. `.bin` is claimed by PCE-CD disc tracks, future Sega CD
audio tracks, future Atari 2600 dumps, future ColecoVision dumps —
content-sniffing each `.bin` at scan time to disambiguate isn't worth
supporting a near-deprecated dump format. Modern SMS dump sets
(No-Intro, TOSEC) ship `.sms` as primary; users with `.bin` SMS dumps
rename to `.sms` — Genesis Plus GX reads the resulting file fine.
