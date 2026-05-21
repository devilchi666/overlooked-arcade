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

- ⬜ Operator validation: launch a real Saturn CD image end-to-end
  (pixels + audio + CDDA + 6-button controller). Suggested reference
  set: **NiGHTS into Dreams** (3D Pad-aware, single-stick mode works
  on digital pad), **Guardian Heroes**, **Radiant Silvergun**, **Saturn
  Bomberman** (multiplayer party game, well-tested compat). Pick a
  disc that matches a BIOS region the operator has on hand.
- ⬜ Save state F5/F8 round-trip mid-disc. Should work via libretro
  `retro_serialize` but Saturn state (dual SH-2 + 4MB main + 4MB video
  + dual VDPs) is large; explicit smoke-test.
- ⬜ Multi-region testing: load JP + US + EU discs with matching
  regional BIOSes to confirm region auto-detect.
- ⬜ Per-game cover sync via libretro-thumbnails — **infra ready 2026-05-20,
  needs operator validation.** Mapping `saturn → Sega_-_Saturn`
  shipped in `media::repo_for_system_id`.
- ⬜ Multi-disc title via `.m3u` — **Panzer Dragoon Saga** (4 discs) is
  the canonical Saturn multi-disc test.
- ⬜ Cart RAM expansion (4MB / 1MB) for the Capcom fighter library
  (X-Men vs SF, SF Alpha 3, KOF '95-'98). Beetle Saturn handles this
  via core options; needs operator validation that the per-game core
  options surface in OA's Per-Game Settings drawer.

**Acceptance gate:** A reference set of Saturn games run with pixels +
audio + working 6-button pad at native 59.94 Hz NTSC.

---

## ⬜ Phase 2 — Polish

- ⬜ **Disc-id extraction** — Saturn discs key at offset 0x20 in the
  data track for the "SEGASATURN" magic + the game-id serial.
  Extend `apps/oa-shell/src/cd_id.rs` with a Saturn branch + switch
  `rom_hashes::libretro_dat_refs_for_system("saturn")` from `&[]` to
  `&[DatRef { subdir: "metadat/redump", basename: "Sega - Saturn" }]`.
- ⬜ **3D Pad analog stick support** — NiGHTS into Dreams / Sonic R /
  Sega Rally Championship Plus need the analog stick for the
  intended experience. Depends on shared analog-input infra (also
  blocking PSX DualShock, Virtual Boy right D-pad, etc.).
- ⬜ **6-button Saturn pad glyphs** for the bindings UI (A/B/C bottom
  + X/Y/Z top + L/R shoulders visualization).
- ⬜ **Kronos vs Beetle Saturn vs YabaSanshiro** — operator-side
  benchmark on a representative host. Document in `DECISIONS.md`.
- ⬜ **Light Gun support** (Virtua Cop, House of the Dead, Maximum
  Force). Beetle Saturn handles the Saturn Stunner via libretro
  pointer device; needs explicit smoke-test against gun games.

---

## ⬜ Phase 3+ — Stretch

- ⬜ **ST-V arcade variant** — Saturn-based arcade hardware that ran
  Radiant Silvergun, Cotton 2, Hyper Duel etc. Same core can drive it
  but ROM layout differs; would be a separate `stv` slug if shipped.
- ⬜ **Custom forked Saturn core** — extremely unlikely. Beetle Saturn
  is mature; OA-specific extensions for a ~600-game library are hard
  to justify.

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

---

## 2026-05-21 — Stale-cleanup audit

The Phase 1+ items above were written when this system onboarded, before cross-system infrastructure (Phases 1.5 / 2.5–2.8 / 3 / 4 + direct-launch CLI) landed. Many `⬜` items are actually shipped — see `docs/cores/AUDIT_2026-05-21.md` for the per-item breakdown (stale vs open-code vs open-operator) for this system.
