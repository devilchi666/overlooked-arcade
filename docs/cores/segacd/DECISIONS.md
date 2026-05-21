# segacd Decisions Log

Append-only. Newest at the bottom. Every entry: **what** we decided,
**when**, **why**, and **what we considered and rejected**.

segacd-specific integration choices live here. Project-wide decisions
(engine stack, license, libretro pivot, etc.) live in
`docs/DECISIONS.md`. Cart Mega Drive decisions live in
`docs/cores/genesis/DECISIONS.md`.

---

## 2026-05-20 — Genesis Plus GX is the Sega CD core (install-once value)

**Decision:** Ship Genesis Plus GX (`genesis_plus_gx_libretro.dll`) as
the default core for `segacd` — the same .dll already shipping for
`sms` + `gamegear`. PicoDrive (`picodrive_libretro.dll`) is available
as a per-game / per-system fallback via the per-system Cores override
UI; users drop it in `<exe_dir>/cores/` if a title needs it.

**Why:** One .dll, four Sega systems. The operator already drops
`genesis_plus_gx_libretro.dll` to play SMS + Game Gear; Sega CD
support comes "for free" with that same install. Reduces the operator
setup tax from "two .dlls for two new systems" to "two new systems
unlocked by the same existing .dll". Genesis Plus GX's Sega CD
implementation is mature (active development since ~2007) and the
canonical libretro Sega CD path.

ClownMDEmu (the cart-Genesis default) is intentionally NOT used for
Sega CD — ClownMDEmu is MD-cart-only and doesn't ship a CD code path.
The cart vs CD core split is acceptable for the same reason TG-16 vs
PCE-CD both use Beetle PCE Fast (in that case the same .dll covers
both) but Genesis splits across two .dlls (ClownMDEmu for cart,
Genesis Plus GX for CD): the operator-facing UX is identical (the
per-system Cores dropdown surfaces both for each system), and the
underlying .dll diversity is invisible until the operator opens the
Cores dialog.

**Considered and rejected:**

- **PicoDrive as default.** Lighter, covers 32X + Sega CD + cart MD.
  Defeated by operator preference for Genesis Plus GX on the broader
  Sega family (same default already shipping for SMS / GG). Single-
  family-default keeps the install-once value.
- **Beetle PCE Fast as default.** PCE Fast is the project's CD-shape
  precedent (validated for PCE-CD), but it's PC Engine-only — doesn't
  emulate the Mega Drive's 68000 / Z80 / VDP / YM2612 / SCD CDC. Not a
  candidate.
- **ClownMDEmu as default.** Cart-only; no CD path upstream. Not a
  candidate.

---

## 2026-05-20 — segacd is a separate SystemId, not a genesis variant

**Decision:** Sega CD / Mega-CD games live under a dedicated `segacd`
`SystemId` in the frontend registry — separate sidebar entry, separate
theme (sapphire 235°), separate per-system settings file, separate
library shelf. Cart Mega Drive games stay under `genesis`. The two
systems share the 6-button controller via the dispatch arms in
`bindings.rs::bit_for / buttons_for / defaults_for / to_libretro_bits`
(all four dispatch `genesis` + `segacd` + `sega32x` to the same
`GENESIS_BUTTONS` table and identity remap).

**Why:** Same rationale that drove the TG-16 / PCE-CD split (see
`docs/cores/pce-cd/DECISIONS.md` 2026-05-18 entry):

1. **User-visible split.** Cart Mega Drive (~900 retail releases) and
   Sega CD (~200 retail releases) are very different shelves in the
   real world. CD games are 5-10× larger on disk, carry CDDA + FMV +
   redbook-audio metadata that cart games don't, and the user
   experience (boot animation, CDDA streaming, multi-disc swap) differs
   meaningfully. A unified `genesis` system page would lose all that
   signal.
2. **BIOS-required vs BIOS-free.** Cart Mega Drive is BIOS-free; Sega
   CD requires a regional BIOS in `<exe_dir>/system/`. The per-system
   settings page surfaces BIOS warnings cleanly for Sega CD without
   adding noise to the cart Genesis page.
3. **Shared controller minimizes duplication.** The split costs four
   things: a SystemId enum variant, a registry entry, a CSS palette
   block, the per-system tracking docs. It does NOT cost a new button
   table, a new key-mapping UI, or a new bindings dispatch path —
   `segacd` reuses `GENESIS_BUTTONS` and `default_genesis_bindings()`
   via the shared dispatch arms.

**Considered and rejected:**

- **Keep CD games under genesis; surface "Cart only / CD only" as a
  filter view inside the existing genesis page.** Loses the per-system
  settings independence (BIOS hints, shader preset, theme color) and
  the at-a-glance library color cue.
- **Nested "Sega family" tree in the left sidebar** (genesis → segacd
  → sega32x). Cleaner long-term, but Phase 2.6 left sidebar nesting
  deferred — the flat sidebar order is what we have today. Revisit
  when the Sega family grows to ≥5 systems (saturn + dreamcast queued).
- **Per-game core override only (no SystemId split).** The override
  exists and works, but a user shouldn't have to set it per-game just
  to pick the right core for their entire CD collection — and the BIOS
  warnings + theme + per-system settings don't surface cleanly under
  a shared SystemId.

---

## 2026-05-20 — Sapphire 235° theme, family-cousin to Genesis cobalt

**Decision:** `[data-system="segacd"]` ships
`oklch(0.55 0.20 235)` — sapphire blue at L=0.55, C=0.20. Soft variant
lifts to a pale azure for text-on-color contrast; glow uses the same
accent at 35% alpha.

**Why:** The Sega family (PCE-CD 220° silver-cyan, segacd 235° sapphire,
Genesis 245° cobalt) clusters in the 220-250° band so a mixed library
reads as "related systems" — but each system gets a distinct lightness
profile so they don't visually merge:
- PCE-CD: 220° L=0.62 C=0.14 — silver-cyan (low chroma, period-correct
  for the silver-and-blue Duo hardware).
- Sega CD: 235° L=0.55 C=0.20 — darker sapphire, period-correct for
  Sega CD Model 2's silver-and-blue retail marketing.
- Genesis: 245° L=0.62 C=0.22 — electric cobalt (high chroma,
  "Genesis does what Nintendon't" launch palette).

The 10° hue gap + 7-point lightness gap between segacd and genesis is
the same readability budget the SNES/Lynx/GBA purple cluster runs at
(270/290/285° with L=0.62/0.65/0.55), so the precedent is well-tested.

**Considered and rejected:**

- **Direct Genesis hue 245°.** Would be the literal "family-coherent"
  answer but visually merges into the Genesis theme in a mixed library
  — defeats the per-system-identity-at-a-glance value.
- **Sega CD red (hue ~355° silver-red).** Brand-accurate to the Sega
  CD logotype + the boxart accent. Defeated by collision with Virtual
  Boy 7° (only 8° apart, both red — would muddy the lineup).
- **Brand-accurate cobalt at L=0.62 (matching Genesis lightness).**
  Would blend into Genesis 245° too tightly. The darker L=0.55
  separates Sega CD as "deeper sapphire" vs Genesis's "electric cobalt".

---

## 2026-05-20 — CD extension disambiguation via Import Wizard per-folder hint

**Decision:** Register the standard CD container extensions
(`.cue / .chd / .iso / .m3u / .ccd / .toc`) for `segacd` — the same set
PCE-CD claims. Disambiguation between segacd and pce-cd at scan time
happens via per-folder rules in the Import Wizard (the operator marks
a folder as `segacd` and matching extensions inside that folder route
there). Same path PCE-CD used.

**Why:** All six CD container extensions can hold either system's
content. Content-sniffing each `.cue` / `.chd` / `.iso` at scan time
to read the disc-id signature is plausible (the disc-id IS extractable
— see Phase 2 work below), but for Phase 0 the operator-rooted
disambiguation is cheaper, well-trodden (PCE-CD validated this path
2026-05-18), and predictable.

A user with a mixed library of TurboGrafx-CD + Sega CD images organizes
them by folder anyway (TG-CD lives at `D:\ROMs\TurboGrafxCD`, Sega CD
at `D:\ROMs\SegaCD`); the per-folder rule turns the existing folder
organization into the disambiguator.

**Considered and rejected:**

- **Disc-id extraction at scan time.** Robust — Sega CD discs carry a
  game-id signature (typically at offset 0x100-0x110 in the data track)
  that distinguishes them from PCE-CD (which keys at a different
  offset). Deferred to Phase 2 because (a) the per-folder rule
  mechanism already exists and works, (b) extending `cd_id.rs` for
  Sega CD requires understanding the redump game-id format and writing
  tests — not Phase 0 work, and (c) the operator's library is already
  organized by folder in practice.
- **Heuristic via path substring.** "If the path contains 'sega cd' or
  'mega cd', classify as segacd." Fragile (case sensitivity, alternate
  naming like `D:\Roms\MCD`, libraries with neutral names like
  `D:\CDs\Various`), and adds an explicit-content-derived rule on top
  of the explicit user-driven one (Import Wizard). Not worth the
  ambiguity surface.

---

## 2026-05-20 — Six canonical BIOS SHA-1s, OkUnknownHash fallback for unknown dumps

**Decision:** `SEGA_CD_BIOS_KNOWN_HASHES` (in `apps/oa-shell/src/main.rs`)
ships with six entries — two US (Model 1 v1.10, Model 2 v2.00, Model 2
v2.00w), two JP (Mega-CD v1.00p, v1.00s), one EU (Mega-CD v1.00). The
canonical filenames the table tests against are `bios_CD_U.bin` /
`bios_CD_J.bin` / `bios_CD_E.bin`, matching the libretro / Genesis Plus
GX convention. Mirrors the PCE-CD BIOS check pattern; same
`BiosCheck::OkCanonical / OkUnknownHash / Missing` vocabulary so the
launch path can branch by system_id without duplicating the match shape.

**Why:** Three regional variants of Sega CD hardware shipped (US Model
1 / Model 2 / Model 2w, JP Mega-CD Model 1 v1.00p / v1.00s / Model 2,
EU Mega-CD Model 1 / Model 2), and operators may have legitimately
canonical dumps from any of them. Six entries cover the most-commonly-
verified Genesis Plus GX-tested dumps; the `OkUnknownHash` branch
allows launches to proceed with a warn-level toast when an operator's
BIOS hash doesn't match (so we don't refuse legitimate but less-common
dumps like Multi-Mega v2.21 or Wondermega/X'Eye revisions).

**Considered and rejected:**

- **Minimal 3-entry table (one per region — US v2.00, JP v1.00p, EU
  v1.00).** Smaller maintenance surface, but operators with the older
  US Model 1 v1.10 dump or the JP v1.00s revision get an
  `OkUnknownHash` warning instead of `OkCanonical` — which is the
  difference between "you're good" and "this might crash". Six entries
  is a better default given the regional variant spread.
- **No pre-check; let the core crash.** Genesis Plus GX with a missing
  BIOS file fails CD-init with an access violation — same crash surface
  PCE-CD navigated. The pre-check trades a tiny launch-time SHA-1 cost
  (one file read + 20 KB SHA-1) for a clear error message instead of a
  baffling AV deep in core init.
- **Per-region BIOS auto-pick based on disc region.** Nice-to-have
  polish — read the disc's region byte at scan time, pick the matching
  BIOS file. Deferred until disc-id extraction lands in Phase 2 anyway.

---

## 2026-05-20 — disc-id extraction deferred to Phase 2

**Decision:** `apps/oa-shell/src/cd_id.rs` stays PCE-CD-only at Phase
0 close. Sega CD disc-id extraction (game serial / redump matching)
ships as Phase 2 polish, after operator validation confirms the
core / BIOS / launch path is working end-to-end.

**Why:** Phase 0 doesn't need disc-id extraction — operator-marked
folders disambiguate against PCE-CD, the libretro core handles
region detection from the disc itself, and library-side canonical
title matching can lag behind the launch path. Adding disc-id
extraction in the same session would double the scope without
unlocking anything that's blocking Phase 0 acceptance.

Sega CD's game-id signature lives at a different offset than PCE-CD's
(typically 0x100-0x110 in the data track for the "GM XXXXXXXX-XX"
serial format vs PCE-CD's signature at a different track location).
Adding the Sega CD branch to `cd_id.rs` requires:
1. Reading the data-track header structure (4-byte preamble + game ID).
2. Parsing the redump serial format into the `game_serials` shape
   `parse_libretro_dat` produces.
3. Switching `rom_hashes::libretro_dat_refs_for_system("segacd")` from
   `&[]` to `&[DatRef { subdir: "metadat/redump", basename: "Sega - Mega CD & Sega CD" }]`.
4. Tests covering both the happy path (well-formed disc-id) and the
   error path (corrupt header, missing data track).

That's a full mini-feature and lands as Phase 2 work after operator
validation.

**Considered and rejected:**

- **Ship disc-id extraction in Phase 0.** Doubles the session scope
  and doesn't unlock anything that's blocking the operator acceptance
  gate (launch + pixels + CDDA). Better to validate the launch path
  works at all before investing in the title-lookup polish.
