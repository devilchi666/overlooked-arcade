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

- ⬜ Operator validation against known-good ROM-sets — operator playtest.
- ⬜ `.neo` single-file launch validation — operator playtest.
- ⬜ `.zip` ROM-set launch validation (content-peek classified) — operator playtest.
- ✅ Save state F5/F8 round-trip — closed by cross-system save-state infra (`oa_libretro::LibretroCore::save_state / load_state`).
- ✅ Per-game cover sync via libretro-thumbnails — closed by cross-system media sync (`media::sync_media_for_system`).

**Acceptance gate:** A reference set of Neo Geo games runs with
pixels + audio + working 4-button arcade pad.

---

## ✅ Phase 2 — Polish

- ✅ **Universe BIOS (Unibios) support.** Shipped via `check_neogeo_bios` + `neogeo_bios_flavour(filename)` (recognizes `uni-bios_2_3.rom` through `uni-bios_4_0.rom` + `sp1-1v1.bin` + `sp-1v1_3db8c.bin`); diagnostic message tags the active variant via `neogeo_bios_flavour`.
- ✅ **AES vs MVS mode toggle** — closed by FBNeo's RETRO_VARIABLEs flowing into the per-system core options page automatically via `core_options::refresh_schema`.
- ⬜ **ROM-set content validation in `check_neogeo_bios`** — Phase 2 upgrade beyond existence-only check; still operator-driven.
- ⬜ **ROM-set hash matching** — extend `rom_hashes` to match `.zip` ROM-sets at the set level (multi-file ROM-set, not single-file sha1) — same gap MAME has, deferred-until-forced.

---

## ⬜ Phase 3+ — Stretch

- ⬜ **Neo Geo CD lineage cross-link** — operator-driven UI polish.
- ⬜ **Light-pen / mahjong stick support** — deferred (niche).

---

## Scope clarifications

- **AES + MVS share one slug.** Same hardware, same controller, same
  ROM format. FBNeo's AES-vs-MVS mode toggle is per-game / per-system.
- **`.zip` content-peek is the first content-based classification** in
  OA's scanner. Future systems with `.zip` ROM-set shapes (CPS-1/2/3
  arcade, etc.) can extend `peek_zip_for_*` family.
- **BIOS REQUIRED** — `neogeo.zip` in `<exe_dir>/system/`. FBNeo
  cannot boot without it.
