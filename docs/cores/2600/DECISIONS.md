# 2600 Decisions Log

Atari 2600 / VCS-specific integration choices. Project-wide decisions
live in `docs/DECISIONS.md`. Append-only; newest at the bottom.

---

## 2026-05-19 — Stella as the default 2600 core

**Decision:** `default_core_dll_for_system("2600")` returns
`"stella_libretro.dll"`.

**Why:** Stella is THE libretro 2600 core. No serious alternate has
shipped through the libretro buildbot — the 2600 emulation space
consolidated around Stella by ~2010, and it's been the de-facto
standard since. Stella handles 50+ obscure bankswitching schemes that
2600 publishers invented during the cartridge era; reimplementing that
breadth elsewhere has never been worth the effort.

**Considered:** No real alternates. The "stella" vs "stella2014"
question (modern Stella vs the 2014 fork pinned for stability) is
resolved by whichever .dll the libretro buildbot ships as
`stella_libretro.dll` — both work for OA's pass-through use case.

---

## 2026-05-19 — `.a26` only at the global registry; `.bin` via per-folder rules

**Decision:** `systemThemes["2600"].extensions = ["a26"]`. The
historically-dominant `.bin` extension is NOT globally registered.

**Why:** This is the choice the plan flagged as needing disambiguation,
and it required operator input — see the question in the 2026-05-19
session header. The 2600 community NEVER standardized on `.a26` the
way Atari 7800 standardized on `.a78` — most real 2600 libraries are
`.bin`, including everything from the HARMONY cart export tradition,
Stella's own dump default, and AtariAge community packs.

But `.bin` collides with:
- PCE-CD disc track files (the `.cue` references `.bin` data tracks)
- Future Sega CD audio tracks (same shape)
- Future ColecoVision (canonical `.col` + `.bin` fallback)
- Future Intellivision (canonical `.int` + `.bin` fallback)
- Future Magnavox Odyssey 2 (canonical `.bin`)
- Future Fairchild Channel F (canonical `.bin` / `.chf`)

Auto-classifying `.bin` as 2600 globally would mis-tag PCE-CD track
files + every other `.bin`-claiming system's dumps. The collision is
too broad to ignore.

The chosen workaround is the import wizard's existing per-folder
rules mechanism (Step 2 Mapping in `ImportWizard.tsx`): operators with
`.bin`-shaped 2600 libraries add `*.bin → 2600` as a per-folder rule
on their Atari folder. The rule overrides the global classification
WITHIN that folder only — a `.bin` file in a PCE-CD folder still
classifies as PCE-CD (or, since PCE-CD doesn't claim `.bin` either,
gets dropped by the scanner unless the user adds the equivalent rule
for PCE-CD; in practice PCE-CD `.bin` tracks aren't scanned directly,
the `.cue` file is the scan target).

**Considered and rejected:**
- **Register `.bin` globally as 2600.** Friendlier for the typical
  2600 library shape but pollutes every future `.bin`-claiming
  system's onboarding. The collision cost is fundamentally bigger
  than the convenience win.
- **Register `.bin` globally as "Atari 2600 OR ColecoVision OR
  Intellivision" with a content-sniffing disambiguation pass.** The
  2600 / Coleco / Intv carts have different size + header signatures;
  a sniff would be reliable. Defeated by complexity — the per-folder
  rule does the same job without bespoke per-extension classification
  logic.

The Phase 1 acceptance gate includes "operator configures `*.bin →
2600` per-folder rule" as a workflow demo, so the path is documented.

---

## 2026-05-19 — Muted wood-grain brown accent at hue 60°, chroma 0.07

**Decision:** `[data-system="2600"]` ships
`oklch(0.60 0.07 60)` — muted warm brown. Pairs against the OA dark
surface as a quiet earth-tone; soft variant lifts to a pale tan for
text-on-color contrast.

**Why:** The original 1977 "Heavy Sixer" VCS shipped with a
wood-veneer front panel — and this single design choice is the most
iconic 2600 visual association in the entire console history. Atari
ditched the wood for the 4-switch (1980) and 2600 Jr (1986), but in
collective memory "Atari 2600 = wood grain". Choosing brown captures
that identity directly.

The hue (60°) sits 5° from TG-16 orange (55°), but the chroma (0.07
vs TG-16's 0.18) separates them on the saturation axis: TG-16 reads
as bright saturated orange, 2600 reads as muted warm brown. The
visual hierarchy in a mixed library is "bright colorful tiles + a
quiet brown one" — exactly the period-correct contrast (vibrant
modern consoles vs the wood-paneled progenitor).

**Considered and rejected:**
- **Atari rainbow red (~20°).** The classic Atari Inc. logo color.
  Defeated by collision with NES 28° + MAME 12° — three reds in the
  same hue band is visually unworkable.
- **Teal-aqua (~190°).** Clean unclaimed range. Defeated by zero
  period association with the 2600 — picking it forgoes the
  wood-grain identity for no design upside.

---

## 2026-05-19 — 7-button layout; first single-button system

**Decision:** `ATARI2600_BUTTONS` ships 7 entries — 4-way d-pad +
FIRE + SELECT + RESET. No secondary face button.

**Why:** The 2600 controller had a single fire button. SELECT and
RESET are the Game Select / Game Reset console switches, not
controller buttons — Stella surfaces them via libretro SELECT +
START respectively per the standard libretro 2600 mapping.

This makes the 2600 the FIRST system in OA's lineup that's
legitimately single-button. The cross-system `z_is_the_primary_action_button_on_every_system`
test fixture (which asserts both primary AND secondary keyboards land
on Z/X) omits the 2600 because there IS no secondary action. The
Z=FIRE assertion lives in `defaults_cover_every_2600_button` instead,
which checks both that the FIRE binding exists AND that its keyboard
is Z.

Documented inline in `bindings.rs` so future single-button systems
(Atari 2600 was the first; the Magnavox Odyssey 2 has a similar
single-button paddle but its Phase 2+ work will need the same
exception) follow the same pattern.

---

## 2026-05-19 — Difficulty / Color switches via Stella core options, not bindings

**Decision:** The 2600's Difficulty A/B (per-player) and Color/B&W
console switches are NOT exposed in the OA bindings UI. They go
through Stella's core options surface (per-system Settings → Core
Options).

**Why:** These are hardware toggles, not input buttons — they don't
fire repeatedly during gameplay, and most players set them once per
game session and forget them. Surfacing them in the per-system
Bindings UI alongside d-pad / FIRE / SELECT / RESET would imply
they're rebindable input events, which is misleading.

Stella's `RETRO_VARIABLE` callback exposes them as named options
(`stella_difficulty_p0` / `stella_difficulty_p1` / `stella_color_palette`)
which the OA Core Options page picks up automatically — no special-
case wiring needed.

---

## 2026-05-19 — Paddle / driving / keypad controllers deferred

**Decision:** Phase 0 ships joystick-only bindings. Paddle-required
titles (Breakout, Kaboom!, Warlords + a handful of others) load and
run but are unplayable.

**Why:** Paddle is analog input (a single rotary dial that the libretro
core reads as a 16-bit signed axis value). OA's current input layer is
purely digital — same blocker that defers Atari 7800 Trak-Ball and
Robotron 2084 twin-stick.

When the shared analog-input infrastructure lands (whatever shape it
takes), it'll light up paddle support for 2600, paddle-like input for
7800, and twin-stick / spinner / lightgun / Wii Remote / etc. all at
once. Targeting that shared infra rather than a per-system paddle path
is the cleanest implementation point.

Paddle-required titles documented in `KNOWN_GAME_BUGS.md` for
operator awareness.
