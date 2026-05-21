# gba Decisions Log

Nintendo Game Boy Advance-specific integration choices. Project-wide
decisions live in `docs/DECISIONS.md`. Append-only; newest at the bottom.

---

## 2026-05-19 — mGBA as the default GBA core

**Decision:** `default_core_dll_for_system("gba")` returns
`"mgba_libretro.dll"`.

**Why:** mGBA is broadly considered the libretro GBA gold standard —
mature, near-universal compatibility, actively maintained, light CPU
footprint. The libretro / RetroArch community consensus has settled
on mGBA as the default GBA core since ~2016, and our catalog follows
that convention. Most operators expect "GBA = mGBA" out of the box.

**Considered and rejected:**
- **VBA-Next as default.** Lighter, older fork of VBA. Defeated by
  mGBA's broader compat + active development. Stays in the catalog as
  an alternate for lower-spec hosts.
- **VBA-M as default.** Closer-to-upstream VBA continuation. Defeated
  by mGBA's accuracy and stability advantages.

---

## 2026-05-19 — Separate slug from `gb`

**Decision:** Game Boy Advance is its own SystemId / slug (`gba`),
distinct from `gb` (which covers Game Boy + Game Boy Color).

**Why:** Despite the family name and the GBA hardware's backward-
compatibility with .gb/.gbc carts, the systems differ in CPU
architecture (32-bit ARM7TDMI vs 8-bit Sharp LR35902), audio hardware
(PSG + direct sound vs PSG only), screen resolution (240×160 vs
160×144), color depth, and cartridge format. The libretro ecosystem
also treats them separately — Gambatte (gb) and mGBA (gba) don't
overlap.

GB-on-GBA backward compat could in theory route through mGBA with a
.gb file, but in practice operators wanting that swap to mGBA on a
specific game via the per-game Core override; the default routing
keeps .gb / .gbc on Gambatte (gb slug) and .gba on mGBA (gba slug).

---

## 2026-05-19 — Deep indigo accent at hue 285°, lightness 0.55, chroma 0.20

**Decision:** `[data-system="gba"]` ships
`oklch(0.55 0.20 285)` — deep indigo. Pairs against the OA dark
surface with strong contrast; soft variant lifts to a pale violet for
text-on-color.

**Why:** Operator picked indigo (period-correct for the 2001 GBA
launch — Nintendo's clear-indigo plastic was the iconic original
color + the launch box art / marketing all leaned indigo) over the
unclaimed teal-aqua range, accepting the visual crowding with SNES
(270°) and Lynx (290°).

The crowding gets resolved on the lightness axis rather than hue:
- **GBA** — L=0.55, deep / dark indigo (this entry)
- **SNES** — L=0.62, mid violet
- **Lynx** — L=0.65, bright purple

In a mixed library tile grid the three read as distinctly different
shades-of-purple even though they sit within a 20° hue band. Chroma
0.20 sits between SNES (0.18) and Lynx (0.22), keeping the saturation
hierarchy SNES < GBA < Lynx.

**Considered and rejected:**
- **Teal-aqua (~190°).** Wide-open unclaimed hue, cleanest separation
  from every other system. Defeated by operator preference for
  period-correctness over visual ease — the GBA's identity is
  inseparable from the launch indigo.
- **Fuchsia-rose (~320°).** Evokes the GBA Micro pink variants;
  unclaimed range. Defeated by being less iconic for GBA's main
  hardware lineage (the launch unit + the SP) than the indigo.

---

## 2026-05-19 — Register `.gba` only; exclude `.bin`

**Decision:** GBA registry extension list is `["gba"]`.

**Why:**
- **`.bin` excluded.** Same collision rationale as every prior system.
  Modern GBA dump sets ship `.gba` as primary.
- **No headered variant.** GBA dumps are headerless raw cartridge
  images; the ROM header at offset 0xA0 is the canonical bytes (not
  a wrapper around inner bytes like .nes / .a78 / .lnx have). The
  raw sha1 candidate hits libretro-database directly without header
  strip — no `metadat/headered/Nintendo - Game Boy Advance.dat` exists
  upstream.

---

## 2026-05-19 — BIOS optional, but flag BIOS-required titles as Phase 2 polish

**Decision:** `gba_bios.bin` is OPTIONAL — mGBA runs without it via
the BIOS-less compatibility path. A small set of titles (Splinter
Cell, Hi-Hi Puffy AmiYumi, a handful of early licensed games) require
the real BIOS to boot.

**Why:** Most GBA titles (~99% by library count) launch fine without
the BIOS. Forcing BIOS presence at the per-system level would gate
every GBA launch on a file most operators don't need.

For the small set that DO need it, the launch UX is currently
suboptimal — mGBA's BIOS-less path emulates most BIOS functions but
not all, and some titles silently hang rather than surfacing an error.
A Phase 2 polish item adds a BIOS-required pre-check (similar to the
PCE-CD syscard pre-check at `apps/oa-shell/src/main.rs::check_pce_cd_bios`)
that warns the operator when launching a known-BIOS-required title
without `gba_bios.bin` present.

The known-BIOS-required title list builds via Phase 1 operator
validation + KNOWN_GAME_BUGS entries — not by maintaining a third-
party list upstream.
