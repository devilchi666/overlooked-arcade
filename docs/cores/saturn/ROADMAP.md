# saturn — Roadmap

Per-core phase tracking for Sega Saturn. Mirrors the project-wide ROADMAP
shape but scoped to Saturn.

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-20)

Paired with PSX. First heavyweight CD-shape onboarding post-segacd. Core
comes online via the libretro pivot — no Rust crate vendoring. Beetle
Saturn is the recommended default.

- ✅ System registered in `frontend/src/themes/registry.ts` — `SystemId`
  union extended with `saturn`, `systemThemes.saturn` entry (CD container
  extensions, landscape 4/3 tile, `crt-lite` shader preset).
- ✅ Theme block in `frontend/src/themes/systems.css` — deepest purple
  at hue 275° + L=0.45 + C=0.18. Sits at the bottom of the violet
  cluster (SNES 270° L=0.62 / GBA 285° L=0.55 / Lynx 290° L=0.65 /
  Saturn 275° L=0.45). Period-accurate to the 1994-1996 Saturn launch
  marketing palette.
- ✅ Per-system input wiring — 13-button Saturn 6-button face pad
  module in `bindings.rs::saturn` + `SATURN_BUTTONS` table +
  `default_saturn_bindings()` + `saturn_to_libretro_bits` identity
  remap + dispatch arms.
- ✅ `default_core_dll_for_system("saturn") → "mednafen_saturn_libretro.dll"`
  in `apps/oa-shell/src/main.rs`.
- ✅ `parse_system_id("saturn" | "sat" | "ss" | "sega-saturn") →
  SystemId::Saturn` (new variant on `oa_core::SystemId` enum).
- ✅ `rom_hashes::libretro_dat_refs_for_system("saturn")` returns `&[]`
  with NO_DAT_SYSTEMS entry — CD images aren't single-file SHA-1
  matched; disc-id extraction via `cd_id.rs` Saturn branch is Phase 2.
- ✅ `media::repo_for_system_id("saturn")` returns
  `Some("Sega_-_Saturn")` so cover sync works as soon as the operator
  runs it.
- ✅ BIOS pre-check via `check_saturn_bios` in main.rs — five canonical
  SHA-1 entries (JP v1.00 / v1.01, US/EU v1.00, EU PAL v1.01, generic
  saturn_bios.bin alias). CD-launch path's BIOS dispatch arm extended
  with `"saturn"` branch.
- ✅ Per-core docs scaffold (this directory).

**Acceptance gate:** Operator can drop `mednafen_saturn_libretro.dll`
into the install + a regional Saturn BIOS in `<exe_dir>/system/`, mark
a Saturn ROMs folder via Import Wizard (disambiguates against
PCE-CD / segacd / PSX claims on the same extensions), see Saturn-themed
(deepest purple) tiles appear in the library, and click one to launch.

---

## ⬜ Phase 1 — First Saturn game running

- ⬜ Operator validation: **NiGHTS**, **Guardian Heroes**, **Radiant Silvergun**, **Saturn Bomberman** — operator playtest.
- ✅ Save state F5/F8 round-trip mid-disc — closed by cross-system save-state infra (`oa_libretro::LibretroCore::save_state / load_state`).
- ⬜ Multi-region testing — operator playtest (JP + US + EU discs with matching regional BIOSes).
- ✅ Per-game cover sync via libretro-thumbnails — closed by cross-system media sync (`media::sync_media_for_system`).
- ⬜ Multi-disc title via `.m3u` (Panzer Dragoon Saga, 4 discs) — operator playtest.
- ⬜ Cart RAM expansion (4MB / 1MB) — operator-driven Beetle Saturn core-option validation (per-game core options drawer shipped cross-system).

**Acceptance gate:** A reference set of Saturn games run with pixels +
audio + working 6-button pad at native 59.94 Hz NTSC.

---

## ⬜ Phase 2 — Polish

- ✅ **Disc-id extraction** — shipped via `apps/oa-shell/src/cd_id.rs::extractors::saturn` (reads SEGA SEGASATURN magic at disc header + T-/GS-prefix serial); `rom_hashes` points at `metadat/redump/Sega - Saturn`.
- ⬜ **3D Pad analog stick support** — gated on shared analog-input device-type wiring (analog axes infra is shipped cross-system).
- ⬜ **6-button Saturn pad glyphs** for the bindings UI — operator polish (bindings UI button-name chips shipped cross-system via `SystemBindingsEditor.tsx:226`).
- ⬜ **Kronos vs Beetle Saturn vs YabaSanshiro** — operator-driven DECISIONS doc.
- ⬜ **Light Gun support** — operator validation. LIGHTGUN dispatch shipped 2026-05-25 on `feat/light-gun-harness` (`crates/oa-libretro/src/state.rs::lightgun_field_value`). Beetle Saturn + Kronos both poll RETRO_DEVICE_LIGHTGUN for the Virtua Gun; SCREEN_X/Y/TRIGGER reach the core. Flagship validation: Virtua Cop 1/2 / House of the Dead / Death Crimson 2 / Crypt Killer. Catalogued in `apps/oa-shell/src/light_gun_systems.rs`.

---

## ⬜ Phase 3+ — Stretch

- ⬜ **ST-V arcade variant** — deferred (separate `stv` slug if shipped).
- ⬜ **Custom forked Saturn core** — deferred.

---

## Scope clarifications

- **No vendoring for Saturn today.** Operator drops the buildbot .dll.
- **BIOS REQUIRED.** Saturn region-locks strictly — JP discs need a JP
  BIOS, US/EU discs need a US/EU BIOS. The pre-check refuses early
  with a clear error toast naming the expected filenames.
- **CD extension collision with PCE-CD / segacd / PSX.** Disambiguation
  at Import Wizard time via per-folder hint — same path the other
  CD-shape systems use.
- **Analog stick deferred.** Phase 2 polish alongside shared analog-input
  infra.
