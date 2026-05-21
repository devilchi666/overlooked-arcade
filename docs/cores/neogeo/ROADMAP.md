# neogeo — Roadmap

Per-core phase tracking for SNK Neo Geo (AES + MVS).

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-20)

Paired with neocd + ngp. Cart-shape onboarding with `.zip` content-peek
disambiguation against MAME — first time the scanner runs a per-file
signature check.

- ✅ `oa_core::SystemId::NeoGeo` variant + `parse_system_id` arm
  (`neogeo | neo-geo | aes | mvs`).
- ✅ `bindings.rs::neogeo` module — 10-button arcade face pad shared
  with neocd via dispatch arms.
- ✅ `default_core_dll_for_system("neogeo") → "fbneo_libretro.dll"`.
- ✅ `rom_hashes::libretro_dat_refs_for_system("neogeo")` → no-intro
  SNK Neo Geo dat. (.neo single-file matches; .zip ROM-set matching is
  Phase 2.)
- ✅ `media::repo_for_system_id("neogeo")` → `SNK_-_Neo_Geo`.
- ✅ BIOS pre-check via `check_neogeo_bios` — existence-only at Phase
  0 (zip content validation deferred). Cart-launch BIOS dispatch lives
  in main.rs next to the CD-launch dispatch arm.
- ✅ Content-peek `.zip` disambiguation in `archive.rs::peek_zip_for_neogeo`
  + scanner integration in `scan_service.rs`. Neo Geo zips emit
  `systemHint: "neogeo"`; MAME zips fall through.
- ✅ Frontend: SystemId union extended, systemThemes entry (extensions
  `["neo", "zip"]`, landscape 4/3 tile, crt-lite), CSS block (deepest
  red 18°/L=0.50/C=0.27).
- ✅ Per-core docs scaffold.

**Acceptance gate:** Operator drops `fbneo_libretro.dll` + `neogeo.zip`
BIOS, scans a Neo Geo folder (mixed `.neo` + `.zip` files OK — the
scanner disambiguates), sees deepest-red Neo Geo tiles, launches a
known-good ROM-set.

---

## ⬜ Phase 1 — First Neo Geo game running

- ⬜ Operator validation against known-good ROM-sets. Suggested
  reference: **Metal Slug 1/2/3/X**, **KOF '97/'98**, **Samurai
  Shodown II**, **Garou: Mark of the Wolves**, **Magician Lord**.
- ⬜ `.neo` single-file launch validation.
- ⬜ `.zip` ROM-set launch validation (content-peek classified).
- ⬜ Save state F5/F8 round-trip.
- ⬜ Per-game cover sync via libretro-thumbnails — needs operator pass.

**Acceptance gate:** A reference set of Neo Geo games runs with
pixels + audio + working 4-button arcade pad.

---

## ⬜ Phase 2 — Polish

- ⬜ **Universe BIOS (Unibios) support.** Community-modified BIOS
  adding region toggle + cheat menu + soft-dip switch UX. Drop-in
  replacement for `neogeo.zip`. Document operator instructions in
  KNOWN_GAME_BUGS / DECISIONS.
- ⬜ **AES vs MVS mode toggle** — per-system core option. FBNeo
  exposes this via `RETRO_VARIABLE`; surface it in OA's per-system
  page.
- ⬜ **ROM-set content validation in `check_neogeo_bios`** — replace
  existence-only check with a peek into `neogeo.zip` confirming
  canonical BIOS ROM files (`sp-s2.sp1`, `sm1.sm1`, `lo-s.s2`, etc.)
  are present. Phase 2 upgrade from the Phase 0 existence-only check.
- ⬜ **ROM-set hash matching** — extend `rom_hashes` to match `.zip`
  ROM-sets against the no-intro Neo Geo dat at the set level (multi-
  file ROM-set, not single-file sha1). Same gap MAME has.

---

## ⬜ Phase 3+ — Stretch

- ⬜ **Neo Geo CD lineage cross-link** — UI surfaces showing "this
  title also released on Neo Geo CD" (and vice versa) since many
  titles cross-pollinated between the formats.
- ⬜ **Light-pen / mahjong stick support** — niche peripherals used
  by a small subset of Neo Geo titles (rare arcade-only).

---

## Scope clarifications

- **AES + MVS share one slug.** Same hardware, same controller, same
  ROM format. FBNeo's AES-vs-MVS mode toggle is per-game / per-system.
- **`.zip` content-peek is the first content-based classification** in
  OA's scanner. Future systems with `.zip` ROM-set shapes (CPS-1/2/3
  arcade, etc.) can extend `peek_zip_for_*` family.
- **BIOS REQUIRED** — `neogeo.zip` in `<exe_dir>/system/`. FBNeo
  cannot boot without it.

---

## 2026-05-21 — Stale-cleanup audit

The Phase 1+ items above were written when this system onboarded, before cross-system infrastructure (Phases 1.5 / 2.5–2.8 / 3 / 4 + direct-launch CLI) landed. Many `⬜` items are actually shipped — see `docs/cores/AUDIT_2026-05-21.md` for the per-item breakdown (stale vs open-code vs open-operator) for this system.
