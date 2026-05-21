# sega32x Decisions Log

Append-only. Newest at the bottom. Every entry: **what** we decided,
**when**, **why**, and **what we considered and rejected**.

sega32x-specific integration choices live here. Project-wide decisions
(engine stack, license, libretro pivot, etc.) live in
`docs/DECISIONS.md`. Cart Mega Drive decisions live in
`docs/cores/genesis/DECISIONS.md`.

---

## 2026-05-20 — PicoDrive is the Sega 32X core (no alternative)

**Decision:** Ship PicoDrive (`picodrive_libretro.dll`) as the default
core for `sega32x`. No alternates registered in the per-system Cores
catalog because no practical alternate exists.

**Why:** PicoDrive is the only mainstream libretro core with Sega 32X
emulation support. Genesis Plus GX doesn't emulate the 32X's twin SH-2
RISC CPUs at all. ClownMDEmu is MD-cart-only. Standalone 32X emulators
exist (e.g. Gens/GS, Kega Fusion) but aren't shipped as libretro cores
and aren't on the buildbot.

PicoDrive's 32X path is mature (active development since ~2010) and
handles both the standard 32X cart-only mode and the 32X-CD stacked
mode (used by Night Trap 32X, Corpse Killer 32X, Slam City). The
operator-facing UX surfaces "32X" as a per-system Cores dropdown with
one entry today; if a competing libretro 32X core ever ships through
the buildbot, the dropdown grows.

**Considered and rejected:**

- **Genesis Plus GX as default.** Doesn't emulate 32X at all. Not a
  candidate.
- **ClownMDEmu as default.** MD-cart-only; no SH-2 emulation. Not a
  candidate.
- **Beetle PCE Fast as default.** PC Engine-only. Not a candidate.

---

## 2026-05-20 — sega32x is a separate SystemId, not a genesis variant

**Decision:** Sega 32X games live under a dedicated `sega32x`
`SystemId` in the frontend registry — separate sidebar entry, separate
theme (neon orange 42°), separate per-system settings file, separate
library shelf. Cart Mega Drive games stay under `genesis`. The two
systems share the 6-button controller via the shared dispatch arms
in `bindings.rs::bit_for / buttons_for / defaults_for / to_libretro_bits`
(all four dispatch `genesis` + `segacd` + `sega32x` to the same
`GENESIS_BUTTONS` table and identity remap).

**Why:** Different cores. Even though a 32X game is technically a Mega
Drive cart with extra hardware, classifying a 32X dump under the cart-
Genesis slug would route it to the cart-Genesis default
(ClownMDEmu, which can't emulate SH-2) and the game would boot to a
blank screen. The slug separation forces the right core (PicoDrive)
to load whenever the user clicks a 32X tile.

The split also gives 32X its own per-system settings page (different
shader preset, different region defaults, different per-system Cores
dropdown). Same rationale that drove the TG-16 / PCE-CD split (see
`docs/cores/pce-cd/DECISIONS.md` 2026-05-18 entry).

**Considered and rejected:**

- **Keep 32X games under genesis; auto-detect 32X via ROM header at
  scan time + swap cores on launch.** Technically feasible (the 32X
  ROM header has a recognizable marker) but conflates two scope
  decisions: library-shelf-membership + active-core-selection. The
  slug separation surfaces 32X as its own first-class system in the
  library + sidebar, which matches OA's "overlooked consoles get
  their own home" thesis.
- **Nested "Sega family" tree in the left sidebar** (genesis →
  segacd → sega32x). Deferred — Phase 2.6 left sidebar nesting
  deferred — the flat sidebar order is what we have today. Revisit
  when the Sega family grows to ≥5 systems.

---

## 2026-05-20 — Neon orange 42° theme, period-accurate to 1994 32X marketing

**Decision:** `[data-system="sega32x"]` ships `oklch(0.68 0.22 42)` —
neon orange at L=0.68, C=0.22. Soft variant lifts to a pale apricot
for text-on-color contrast; glow uses the same accent at 35% alpha.

**Why:** The 32X's 1994 retail marketing leaned heavily into a fiery
orange palette — the mushroom-cap unit itself was black with orange
"32X" branding, the box art uniformly used neon orange-on-black for
the logotype, and the launch advertising painted the system as "the
next generation lit on fire." Hue 42° captures that period-correct
identity.

Hue placement: 42° lands in the open 35-50° band (the only register-
able hue range between TG-16 orange 55° and the ChannelF 25° / NES 28°
/ MAME 12° red cluster). The 13° gap from TG-16 (55°) is the tightest
neighbor; chroma + lightness profile separation makes them readable
side-by-side:
- TG-16: 55° L=0.74 C=0.18 — warm orange (yellow-tilted)
- 32X: 42° L=0.68 C=0.22 — neon orange (red-tilted, higher chroma)

The 32X explicitly does NOT join the Sega family cobalt cluster
(PCE-CD 220° / segacd 235° / Genesis 245°) because 32X identity in the
era was visually distinct from Genesis/Sega CD — Sega marketed it as
a different category of product, not a Mega Drive family member.

**Considered and rejected:**

- **Cobalt at 255° (royal blue, between Genesis 245° and Intv 260°).**
  Family-coherent with Genesis/Sega CD, but would visually merge with
  Genesis at typical tile-grid scale + 32X branding was never blue.
- **32X red at hue 25°.** Brand-accurate to the "32X" logotype but
  exact collision with ChannelF (25°) and only 3° from NES (28°). Too
  tight.
- **Sonic gold at hue 50°.** Sega's other strong era color. Defeated
  by tight collision with TG-16 (55°) — only 5° apart — and 32X
  marketing was orange-not-gold.

---

## 2026-05-20 — `.32x` only, no `.bin` / `.md` / `.smd` cross-registration

**Decision:** `sega32x` registry entry ships
`extensions: ["32x"]` — single canonical extension. `.bin`, `.md`,
`.smd` intentionally NOT cross-registered.

**Why:**
- **`.bin` excluded** for the same reason Genesis excluded it: collides
  with PCE-CD CD-track files, future Atari 2600 / Sega CD audio
  tracks. Operators with `.bin` 32X dumps rename to `.32x` —
  PicoDrive doesn't care about the extension.
- **`.md` / `.smd` excluded** because even though 32X games are
  technically Mega Drive carts with extra hardware, classifying a
  `.md` 32X dump under the cart-Genesis slug would route it to
  ClownMDEmu (which can't emulate the SH-2s) and the game would boot
  to a blank screen. The slug separation forces the right core
  selection.

**Considered and rejected:**

- **`.32x + .bin`.** Easier for operators with old No-Intro `.bin` sets,
  but reintroduces the `.bin` collision the Genesis onboarding
  deliberately avoided. Would need scanner-level content sniffing to
  disambiguate against PCE-CD `.bin` tracks. Not worth the ambiguity
  surface.
- **`.32x + .smd + .md`.** Accept the full MD-cart extension family on
  the theory 32X games are MD games with extra hardware. Defeats the
  user-visible separation between genesis and sega32x slugs — a `.md`
  32X dump would be ambiguous (which slug? which core?). Modern dump
  sets ship 32X as `.32x` precisely to avoid this ambiguity.

---

## 2026-05-20 — No BIOS required for cart-only 32X path

**Decision:** Cart-only 32X playback is BIOS-free. No 32X entry in any
BIOS pre-check table; the launch path proceeds directly from
`is_cd_extension` check (false for `.32x`) into core load.

**Why:** PicoDrive synthesizes the SH-2 boot vector internally — no
external 32X BIOS file exists on real 32X hardware (the SH-2 firmware
is part of the 32X cart-slot addon's mask ROM, not a user-replaceable
component, and the cores treat that mask ROM as internal). Operators
don't need any 32X-specific BIOS in `<exe_dir>/system/` for stock 32X
cart playback.

32X-CD games (Night Trap 32X, Corpse Killer 32X, Slam City) stack on
top of Sega CD and DO need the Sega CD BIOS in `<exe_dir>/system/` —
those route through `segacd` and are covered by that slug's
`check_sega_cd_bios` pre-check. The cart-only 32X path (the path this
slug owns) is BIOS-free.

**Considered and rejected:**

- **Add a placeholder 32X BIOS pre-check for forward compatibility.**
  No, because no 32X BIOS exists — adding one would be wrong
  documentation that the operator would chase fruitlessly.
