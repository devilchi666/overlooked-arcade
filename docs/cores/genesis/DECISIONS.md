# genesis Decisions Log

Genesis-specific integration choices. Project-wide decisions live in
`docs/DECISIONS.md`. Append-only; newest at the bottom.

---

## 2026-05-19 — ClownMDEmu as the default Genesis core

**Decision:** `default_core_dll_for_system("genesis")` returns
`"clownmdemu_libretro.dll"`. Operator-installed v1.6.11 (commit 1670f30)
drives the default Mega Drive path.

**Why:** Operator preference. ClownMDEmu is a modern actively-developed
core with clean architecture and a cleaner C code base than the older
Genesis Plus GX. It exists outside the multi-Sega family (focused on MD
only, no SMS/GG/SegaCD bundling) which keeps the scope tight for this
first-pass onboarding.

The catalog entry for `clownmdemu_libretro` is `recommended=false`
because **Genesis Plus GX** is the long-standing libretro multi-Sega
default — it covers SMS, Game Gear, Mega Drive, and Sega CD behind one
.dll and most operators land on it via search-by-system. We register
Genesis Plus GX as the recommended catalog pick for users browsing
through the Cores page, but the per-system *default* — what loads when
the user does nothing — is ClownMDEmu per operator direction.

**Considered and rejected:**
- **Genesis Plus GX as default.** Catalog-recommended pick, covers
  the full Sega family. Defeated by operator preference for ClownMDEmu
  on a single-system-focused onboarding.
- **PicoDrive as default.** Lighter than Genesis Plus GX, covers 32X +
  Sega CD too. Defeated by the same operator preference + ClownMDEmu's
  more-active recent development cadence.

Users who want any of the three alternates can swap via the per-system
Settings → Cores override; the swap is one click and persists in
`<appDataDir>/cores.json`.

---

## 2026-05-19 — 6-button Mega Drive controller as the default layout

**Decision:** `default_genesis_bindings()` ships the 6-button MD
controller layout (A/B/C + X/Y/Z + Start + Mode + d-pad) rather than the
3-button (A/B/C + Start + d-pad).

**Why:** Most modern dump sets assume 6-button is available, and the
core itself (ClownMDEmu, Genesis Plus GX, PicoDrive — all of them)
announce 6-button by default. The minority of titles that misbehave
with 6-button announce (Sonic 3D Blast and a few others) get worked
around via per-game Input override; they don't justify defaulting the
entire library to 3-button.

The keyboard side follows the cross-system "Z is primary action" rule
(locked by the `z_is_the_primary_action_button_on_every_system` test):
- **Z** → MD **B** (middle face, libretro bit 0) — most MD games use B
  for the main action (jump in Sonic, attack in Streets of Rage).
- **X** → MD **C** (right face, libretro bit 8) — jump-while-running /
  kick / secondary.
- **A** → MD **A** (left face, libretro Y bit 1) — tertiary.
- Top row (X/Y/Z) on **Q / S / W** — mirrors the SNES shoulder /
  diamond convention (SNES X=S top, L/R=Q/W shoulders).

The gamepad side places **B on East** (primary, matching the
lynx/nes/snes/atari7800 pad convention — every console-shape system in
OA pins its primary action to the East face button), **C on South**
(secondary, the natural "south of east" diamond neighbor), and **A on
West** (tertiary). The MD pad's horizontal A-B-C row is _not_ mapped
left-to-right onto West-South-East because that would route primary to
South and break OA's cross-system convention. X/Y/Z map to LeftTrigger
/ North / RightTrigger for the 6-button extras (the standard "extra
buttons on top" convention).

**Considered and rejected:**
- **3-button default.** Would match the original Mega Drive controller
  shipped with the launch console and the smaller compat set. Defeated
  by modern-dump assumption that 6-button is available + the per-game
  override path being already cheap.
- **Keyboard A/S/D for the bottom row.** Tempting because A/S/D is
  closer to the diamond shape. Defeated by the "Z is primary" rule —
  every other OA system pins primary to Z; consistency wins.

---

## 2026-05-19 — Cobalt blue accent at hue 245°, not the PCE-CD 220°

**Decision:** `[data-system="genesis"]` ships
`oklch(0.62 0.22 245)` — saturated cobalt blue. Pairs well against the
OA dark surface; soft variant lifts to a pale azure for text-on-color
contrast.

**Why:** The user picked "cobalt blue" from the onboarding question,
but the existing PCE-CD theme is already at hue 220° (cyan-blue evoking
the silver/blue Duo). A 25° shift to 245° plus a chroma bump from 0.14
to 0.22 produces a clearly distinct color — PCE-CD reads as silver-cyan,
Genesis reads as electric-cobalt. A mixed library shows them as
different systems at a glance.

Cobalt is also period-correct for the Sega marketing of the early '90s
("Genesis does what Nintendon't" leaned into a deep blue palette,
distinct from the SNES launch violet at hue 270°).

**Considered and rejected:**
- **Direct PCE-CD hue 220°.** Would be the literal "cobalt blue" answer
  but visually merges into the PCE-CD theme in a mixed library —
  defeats the per-system-identity-at-a-glance value.
- **Sonic gold (hue ~45°).** Sega's other strong era color (Sonic ring
  yellow / arcade flyer gold). Defeated by the gold range being taken
  by Atari 7800 (hue 80°) and tg16-orange (55°).
- **Mega Drive red (hue ~5°).** JP Mega Drive logo accent. Defeated by
  red being taken twice already (NES 28°, MAME 12°).

---

## 2026-05-19 — Register `.md` / `.smd` / `.gen` / `.68k`; exclude `.bin`

**Decision:** Genesis registry extension list is
`["md", "smd", "gen", "68k"]`. Headerless `.bin` dumps intentionally
excluded.

**Why:** `.bin` is the same collision problem the Atari 7800 onboarding
already navigated (see `docs/cores/atari7800/DECISIONS.md`). The PCE-CD
disc-track files use `.bin`, future Atari 2600 / older Sega CD audio
tracks would too, and content-sniffing each `.bin` at scan time to
disambiguate isn't worth supporting a near-deprecated dump format. Modern
MD dump sets (No-Intro, TOSEC) all ship `.md` as primary; users with
`.bin` MD dumps rename to `.md` — ClownMDEmu and Genesis Plus GX both
read the resulting file fine.

`.smd` deserves first-class scanning despite being a less-common
modern format because the Super Magic Drive era produced a lot of
preserved dumps in that interleaved layout, and modern cores handle
deinterleaving transparently. `.gen` is the Kega Fusion-era alternate
extension; raw bytes, same content as `.md`. `.68k` is rare but homebrew
sometimes uses the CPU name; the core treats it identically.
