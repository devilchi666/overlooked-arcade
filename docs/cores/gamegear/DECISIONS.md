# gamegear Decisions Log

Sega Game Gear-specific integration choices. Project-wide decisions
live in `docs/DECISIONS.md`. Append-only; newest at the bottom.

---

## 2026-05-19 — Genesis Plus GX as the default GG core (shared with SMS)

**Decision:** `default_core_dll_for_system("gamegear")` returns
`"genesis_plus_gx_libretro.dll"` — the same .dll used by `sms`.

**Why:** A single Genesis Plus GX install services both Game Gear and
Master System (and is also a per-system Cores alternate for Genesis).
Operators paired-onboarding SMS + GG on 2026-05-19 download one .dll
and get both systems. Genesis Plus GX has mature GG support including
the 6-color extension over SMS's 64-color palette, transparent SMS-mode
detection for GG ROMs that ship with SMS signatures, and PSG audio
accuracy.

**Considered and rejected:**
- **PicoDrive as default.** Lighter footprint, MD-first but also
  handles GG. Defeated by Genesis Plus GX's stronger GG-specific test
  coverage + the install-once shared .dll value with SMS.

---

## 2026-05-19 — 7-button layout labeled "START" (not "PAUSE")

**Decision:** `GAMEGEAR_BUTTONS` ships 7 entries — 4-way d-pad + B1 +
B2 + START. The third binding is labeled "START" rather than the SMS
convention "PAUSE".

**Why:** The Game Gear has a hardware Start button on the unit itself
(top-left edge), not on the controller. Unlike the SMS — where Pause
lived on the console hardware and is functionally a "pause" press —
the GG's Start button is the standard "menu / start game" press most
GG games use. Labeling it "START" matches the hardware label the
operator sees on the unit and matches the convention for every other
system in OA's lineup that has a Start button (NES, SNES, Genesis, etc.).

The libretro mapping is identical to SMS (bit 3, libretro START), but
the operator-facing label differs to match what's printed on the
hardware.

---

## 2026-05-19 — Yellow-green accent at hue 130°

**Decision:** `[data-system="gamegear"]` ships
`oklch(0.72 0.18 130)` — yellow-green. Pairs well against the OA dark
surface; soft variant lifts to a pale chartreuse for text-on-color
contrast.

**Why:** Game Gear launch packaging (1990-92 era) used a black-with-
teal-and-yellow palette — the unit's marketing photography frequently
showed it on a saturated green/teal/yellow background. Picking 130°
captures the yellow-green energy of that era while staying clearly
distinct from every other claimed hue. The wide-open 100-200° range
has no prior occupants; we pick from the warmer end (130° yellow-green
> 165° teal) because the yellow energy reads as more period-specific
to GG than a generic teal.

Slightly lower chroma (0.18 vs SMS's 0.22) compensates for the higher
inherent luminance of the green hue — at chroma 0.22 the accent
visually overwhelmed the OA dark surface; 0.18 reads cleaner.

**Considered and rejected:**
- **Cool teal (~175°).** Cleaner teal closer to the GG black-and-cyan
  industrial look. Defeated by being visually closer to PCE-CD (220°);
  the warmer 130° picks up more of the yellow-green identity.
- **Warm orange (~40°).** Defeated by being only 15° from TG-16's 55°
  orange — would read similar at a distance.

---

## 2026-05-19 — Register `.gg` only; exclude `.bin`

**Decision:** GG registry extension list is `["gg"]`. Headerless `.bin`
dumps intentionally excluded.

**Why:** Same collision rationale as SMS, Genesis, Atari 7800. `.bin`
is claimed by every other system that ever dumped headerless. Modern
GG dump sets ship `.gg` as primary; users with `.bin` GG dumps rename
to `.gg`.
