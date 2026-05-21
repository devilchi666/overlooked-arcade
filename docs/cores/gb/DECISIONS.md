# gb Decisions Log

Nintendo Game Boy / Game Boy Color-specific integration choices.
Project-wide decisions live in `docs/DECISIONS.md`. Append-only;
newest at the bottom.

---

## 2026-05-19 — Gambatte as the default GB core

**Decision:** `default_core_dll_for_system("gb")` returns
`"gambatte_libretro.dll"`.

**Why:** Gambatte is the long-standing libretro Game Boy default —
mature, broad compatibility, light CPU, handles both DMG and CGB from
the same .dll via ROM-header auto-detect. Most operators expect
"Game Boy = Gambatte" out of the box.

**Considered and rejected:**
- **SameBoy as default.** More cycle-accurate, slightly heavier.
  Better for the small minority of timing-sensitive ROMs that fight
  Gambatte. Defeated by Gambatte's broader test coverage + lighter
  footprint for the typical operator. SameBoy stays in the catalog
  as the per-system Cores override; users hitting specific compat
  problems swap with one click.
- **TGB Dual.** Link-Cable-focused dual-emulator. Defeated by Phase 0
  scope — single-instance playback is the priority; multiplayer is
  Phase 2+ polish.

---

## 2026-05-19 — Single slug for DMG + CGB

**Decision:** Both Nintendo Game Boy (DMG, 1989) and Game Boy Color
(CGB, 1998) share a single SystemId / slug (`gb`). Extensions `.gb`
and `.gbc` both route to the same library bucket; the libretro core
(Gambatte by default) auto-detects from the ROM header.

**Why:** The two systems share the same screen dimensions (160×144),
same controller layout, same CPU architecture, and CGB is fully
backward-compatible with DMG. Most users mentally bucket their library
as "my Game Boy games" without sharply distinguishing DMG vs CGB.
Splitting would force operators to maintain two separate library
folders + duplicate per-system settings for cosmetically similar
hardware, with no upside.

The libretro ecosystem agrees: every major GB core (Gambatte, SameBoy,
TGB Dual) handles both via one .dll. libretro-database does keep
separate `.dat` files for the two — we merge those into one local
corpus per `fetch_and_parse_all`.

**Considered and rejected:**
- **Separate `gb` + `gbc` slugs.** Defeated by the user-mental-model
  argument + the duplicate-config tax. The cover sync gap (only one
  thumbnails repo per slug today) is real but is a multi-repo
  follow-up that benefits any future single-slug-multi-variant system
  (Wonderswan mono+color is the next obvious case).

---

## 2026-05-19 — Default cover repo is `Nintendo_-_Game_Boy`, not `Nintendo_-_Game_Boy_Color`

**Decision:** `media::repo_for_system_id("gb")` returns
`Some("Nintendo_-_Game_Boy")`. CGB covers from the parallel
`Nintendo_-_Game_Boy_Color` thumbnails repo are out of scope for
Phase 0.

**Why:** Most users' Game Boy libraries are DMG-era (1989-1998 sales
volume) — the DMG repo coverage is broader and lands the most hits per
library. CGB covers are still important and should land eventually,
but require a `repo_for_system_id` signature change to optionally
return multiple repos (or a follow-up cover-sync command that consults
multiple repos for one slug). Deferring keeps Phase 0 narrow.

The same architectural gap affects any future single-slug-multi-
hardware-variant system — Wonderswan mono + Wonderswan Color is the
canonical next case. The multi-repo signature change benefits all of
them simultaneously, which is the cleanest implementation point.

**Tracked as a Phase 2 polish item** in `ROADMAP.md` rather than left
as ambient drift.

---

## 2026-05-19 — Muted DMG pea-green accent at hue 145°, chroma 0.13

**Decision:** `[data-system="gb"]` ships
`oklch(0.62 0.13 145)` — muted forest pea-green. Pairs well against
the OA dark surface; soft variant lifts to a pale pistachio for
text-on-color contrast.

**Why:** The DMG screen's iconic pea-soup green is the single most
recognizable Game Boy visual association across decades. The hue 145°
captures it; the chroma 0.13 (lower than most accents) captures the
muted, slightly-grayed quality of the actual DMG LCD, which was
chartreuse-on-olive rather than saturated emerald.

Game Gear already claims hue 130° at chroma 0.18 (the GG launch
packaging yellow-green). The 15° hue separation + 0.05 chroma gap make
the two systems read as decisively distinct families in a mixed
library — GG = bright yellow-green, GB = muted forest pea-soup.

**Considered and rejected:**
- **Teal (~180°).** Cleanly distinct from every claimed hue; evokes
  GBC translucent-plastic-era branding. Defeated by being less
  period-correct than the DMG green — the green is the iconic visual,
  the translucent plastic was secondary.
- **GBC atomic purple (~315°).** Late-'90s clear-plastic GBC color.
  Defeated by being less universally associated with "Game Boy" than
  the DMG green + sitting closer to SMS magenta (340°) than ideal.

---

## 2026-05-19 — Register `.gb` + `.gbc`; exclude `.bin` / `.cgb` / `.sgb`

**Decision:** GB registry extension list is `["gb", "gbc"]`.

**Why:**
- **`.bin` excluded.** Same collision rationale as every prior system.
  Modern GB dump sets ship `.gb` / `.gbc` as primary.
- **`.cgb` excluded.** An alternate extension some old dumpers used
  for CGB ROMs; not commonly shipped today. Users with `.cgb` files
  can rename to `.gbc`.
- **`.sgb` excluded.** Super Game Boy enhanced ROMs (SNES adapter
  palette data). Niche use case that's actually a `snes`-slug concern
  — the SGB hardware is a SNES cartridge, not a Game Boy variant.
  When SGB playback support lands, it routes through the `snes` slug
  and an SGB-aware SNES core, not here.
