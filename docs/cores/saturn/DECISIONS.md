# saturn Decisions Log

Append-only. Newest at the bottom. Every entry: **what** we decided,
**when**, **why**, and **what we considered and rejected**.

saturn-specific integration choices live here. Project-wide decisions
(engine stack, license, libretro pivot, etc.) live in
`docs/DECISIONS.md`. Other Sega family decisions live in
`docs/cores/genesis/`, `docs/cores/segacd/`, `docs/cores/sega32x/`.

---

## 2026-05-20 — Beetle Saturn is the Saturn core (Mednafen lineage)

**Decision:** Ship Beetle Saturn (`mednafen_saturn_libretro.dll`) as
the default core for `saturn`. Kronos (`kronos_libretro.dll`) and
YabaSanshiro (`yabasanshiro_libretro.dll`) are available as per-system
/ per-game alternates via the per-system Cores override UI.

**Why:** Beetle Saturn is the mature Mednafen-derived Saturn
implementation — broad-compatibility, well-tested across the Saturn
library, same upstream lineage as Beetle PSX (PSX), Beetle PCE Fast
(TG-16/PCE-CD), Beetle VB (Virtual Boy), Beetle WonderSwan, Beetle
Lynx. Mednafen's emulator family is the most-validated libretro core
set OA ships; defaulting to it across the Mednafen-emulated systems
keeps the operator experience consistent.

**Considered and rejected:**

- **Kronos as default.** Lighter CPU footprint, actively developed
  (multi-platform fork of Yabause). Defeated by less-validated
  compatibility surface and less consistent libretro upstream
  cadence vs Beetle Saturn.
- **YabaSanshiro as default.** Android-focused fork; desktop libretro
  build less polished. Lower priority alternate.

---

## 2026-05-20 — Saturn is a separate SystemId, not a genesis variant

**Decision:** Sega Saturn games live under a dedicated `saturn`
`SystemId`. Distinct sidebar entry, theme (deepest purple 275°),
per-system settings file, library shelf, and per-system Cores
dropdown. Shares no plumbing with cart Genesis / segacd / sega32x
(different controller layout — Saturn 6-button face + L/R shoulders
vs MD 6-button).

**Why:** Saturn is a generationally distinct platform from Mega Drive.
Different CPU architecture (dual SH-2 vs 68000+Z80), different
controller, different software library, different cultural identity.
Lumping it under "genesis" would force inappropriate cross-system
defaults (BIOS-free vs BIOS-required, MD pad vs Saturn pad).

The split costs three things: a SystemId enum variant, a registry
entry, a CSS palette block. It does NOT cost a new core .dll (Mednafen
ships them all as separate libretro cores) or any shared infra.

**Considered and rejected:**

- **Nest under a "Sega family" sidebar tree** (genesis → segacd →
  sega32x → saturn). Sidebar nesting deferred — Phase 2.6 left the
  sidebar flat. Revisit when the Sega family hits ≥5 systems
  (dreamcast queued).

---

## 2026-05-20 — Deepest purple 275° theme

**Decision:** `[data-system="saturn"]` ships `oklch(0.45 0.18 275)` —
deepest purple in the violet cluster. Soft variant lifts to a pale
lavender for text-on-color contrast.

**Why:** Period-accurate to the 1994-1996 Saturn launch marketing
palette (the Saturn launch boxes used a saturated deep purple-on-black
that became the platform's visual identity). Hue 275° sits 5° from
SNES violet (270°) and 10° from GBA indigo (285°), but the lightness
ladder separates the cluster cleanly:

- SNES at L=0.62 — mid violet
- Lynx at L=0.65 — bright purple
- GBA at L=0.55 — deep indigo
- **Saturn at L=0.45 — DEEPEST purple**

Saturn is the bottom rung. The high-chroma + low-lightness profile
reads as "premium night purple" rather than the brighter
Nintendo-family purples. Operator accepted the cluster crowding for
period-correctness.

**Considered and rejected:**

- **Magenta-purple at 315°** (Plan B from the question — between
  WonderSwan 305° pearl and O2 325° rose). Would slot saturn into the
  magenta cluster rather than the violet cluster. Defeated because
  it's less brand-correct (Saturn launch was distinctly *purple*,
  not magenta).
- **Open-band teal at 180°** (Plan C). Cleanest hue separation but
  least brand-correct. Defeated because Saturn purple is iconic enough
  to justify the violet-cluster crowding.

---

## 2026-05-20 — CD extension disambiguation via Import Wizard per-folder hint

**Decision:** Register the standard CD container extensions
(`.cue / .chd / .iso / .m3u / .ccd / .toc`) for `saturn` — the same set
PCE-CD / segacd / PSX claim. Disambiguation between the four CD-shape
systems at scan time happens via per-folder rules in the Import Wizard.

**Why:** Same rationale segacd locked. All four CD-shape systems
share the libretro CD container set; content-sniffing each file at
scan time to read the disc-id signature is plausible (Saturn discs key
at offset 0x20 in the data track for the "SEGASATURN" magic), but for
Phase 0 the operator-rooted folder-based disambiguation is cheaper,
well-trodden (validated by pce-cd + segacd), and predictable.

**Considered and rejected:**

- **Disc-id extraction at scan time.** Robust — Saturn's "SEGASATURN"
  magic at offset 0x20 distinguishes it cleanly from PCE-CD / segacd /
  PSX. Deferred to Phase 2 polish because (a) the per-folder rule
  mechanism already exists and works, (b) extending `cd_id.rs` for
  Saturn requires the redump serial-format parser to also pick up
  Saturn serials, and (c) operator validation should happen against
  the launch path before investing in title-lookup polish.

---

## 2026-05-20 — Five canonical BIOS SHA-1s, OkUnknownHash fallback

**Decision:** `SATURN_BIOS_KNOWN_HASHES` ships with five entries —
JP v1.00 (`sega_100.bin`), JP v1.01 (`sega_101.bin`), US/EU v1.00
(`mpr-17933.bin`), EU PAL v1.01 (`mpr-19367b.bin`), and a generic
`saturn_bios.bin` alias mapping to the US/EU v1.00 hash. Mirrors the
PCE-CD / segacd BIOS check pattern; same `BiosCheck::{OkCanonical,
OkUnknownHash}` / `BiosError::{Missing, Io}` vocabulary so the
CD-launch path branches by system_id without duplicating the match
shape.

**Why:** Saturn ships three regional + revision variants. Five
entries cover ~95% of operator-installed BIOSes; the `OkUnknownHash`
branch allows launches to proceed with a warn-level toast when an
operator's BIOS hash doesn't match a known canonical (e.g. rare ST-V
arcade BIOS variants, Asian-region BIOSes not in the table).

---

## 2026-05-20 — 6-button face pad + L/R shoulders, 3D Pad analog stick Phase 2

**Decision:** `default_saturn_bindings()` ships the 13-button digital
Saturn 6-button face pad layout (A/B/C + X/Y/Z + L/R + START + d-pad).
The Saturn 3D Pad's analog stick (NiGHTS into Dreams pack-in 1996;
used by NiGHTS / Sonic R / Sega Rally Championship Plus) is deferred
to Phase 2 alongside shared analog-input infra.

**Why:** Same Phase 2 deferral pattern Virtual Boy used (right D-pad)
and Intellivision used (16-direction analog disc). The 3D Pad is a
relatively niche peripheral — only a handful of Saturn games
specifically require it; the vast majority of the library plays on
the 6-button digital pad.

The 6-button face buttons in a 2x3 physical grid exceed the Xbox-style
4-button diamond, so libretro spills the rightmost face buttons (C, Z)
to the L2/R2 trigger slots:

- Saturn A → libretro B (bit 0) — primary action
- Saturn B → libretro A (bit 8) — secondary
- Saturn C → libretro R2 (bit 13) — spilled to right trigger slot
- Saturn X → libretro Y (bit 1) — top-left face
- Saturn Y → libretro X (bit 9) — top-middle face
- Saturn Z → libretro L2 (bit 12) — spilled to left trigger slot
- Saturn L → libretro L (bit 10) — left shoulder
- Saturn R → libretro R (bit 11) — right shoulder

This is what Beetle Saturn's libretro input descriptors define; we
follow that mapping directly for identity remap.

Keyboard layout mirrors the Saturn pad's physical 2x3 face button grid
on QWERTY (a happy coincidence — the layout satisfies BOTH the
cross-system "Z is primary" rule AND the Saturn physical mapping):

```text
Keyboard cluster:       Saturn pad face:
  Q W                     L R         (shoulders)
  A S D                   X Y Z       (top row)
  Z X C                   A B C       (bottom row)
```

Saturn-button A (primary) → keyboard Z (cross-system rule satisfied).
Saturn-button B (secondary) → keyboard X (cross-system rule
satisfied). All six Saturn face buttons land on their physically-
corresponding keyboard keys — best-of-both-worlds for Saturn fighter
muscle memory (Virtua Fighter / Fighters Megamix / Capcom-vs-SNK
ports).

**Considered and rejected:**

- **Use Q/S/W for top row (Genesis convention).** Conflicts with Q/W
  reserved for L/R shoulders. Defeated.
- **Use D/S/F for top row.** Initial choice during onboarding but
  fails the cross-system test: Saturn-A as primary maps to keyboard Z,
  but the Genesis-copied default had Saturn-A → keyboard A (Genesis's
  tertiary slot). The Saturn libretro bit layout differs from
  Genesis (Saturn A is libretro B / primary, while Genesis A is
  libretro Y / tertiary), so blindly copying the Genesis keyboard
  pattern broke the "Z is primary" test. Caught by
  `z_is_the_primary_action_button_on_every_system` during validation.
