# segacd — Roadmap

Per-core phase tracking for Sega CD / Mega-CD. Mirrors the project-wide
ROADMAP shape but scoped to Sega CD.

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-20)

Paired with Sega 32X. Core comes online via the libretro pivot — no
Rust crate vendoring. Genesis Plus GX is the recommended default
(same .dll already shipping for SMS + Game Gear).

- ✅ System registered in `frontend/src/platform/themes/registry.ts` — `SystemId`
  union extended with `segacd`, `systemThemes.segacd` entry (extensions
  `cue / chd / ccd / toc / m3u / iso`, landscape tile aspect 4/3,
  `plain` default shader preset for FMV-heavy library).
- ✅ Per-system palette in `frontend/src/platform/themes/systemPalettes.ts` —
  sapphire blue at hue 235° + L=0.55 + C=0.20. Family-cousin to Genesis
  cobalt (245°) but visually distinct via lightness axis. Lives in the
  typed `SYSTEM_PALETTES` map, injected as `[data-system]` CSS at boot.
- ✅ Per-system input wiring — segacd shares the 6-button Mega Drive
  controller via the `"genesis" | "segacd" | "sega32x" => genesis_*`
  dispatch arms in `apps/oa-shell/src/bindings.rs`. Same pattern PCE-CD
  uses to share TG-16's controller.
- ✅ `default_core_dll_for_system("segacd") → "genesis_plus_gx_libretro.dll"`
  in `apps/oa-shell/src/main.rs`.
- ✅ `parse_system_id("segacd" | "sega-cd" | "mega-cd" | "megacd" | "mcd") →
  SystemId::SegaCd` (new variant on `oa_core::SystemId` enum).
- ✅ `rom_hashes::libretro_dat_refs_for_system("segacd")` returns `&[]`
  with NO_DAT_SYSTEMS entry — CD images aren't single-file SHA-1
  matched; disc-id extraction via `cd_id.rs` deferred to Phase 2.
- ✅ `media::repo_for_system_id("segacd")` returns
  `Some("Sega_-_Mega-CD_-_Sega_CD")` so cover sync works as soon as
  the operator runs it.
- ✅ BIOS pre-check via `check_sega_cd_bios` in main.rs — six canonical
  SHA-1 entries across US / JP / EU regional variants. CD-launch path
  branches by system_id to call the right BIOS check (PCE-CD or Sega CD).
- ✅ Per-core docs scaffold (this directory).

**Acceptance gate:** Operator can drop `genesis_plus_gx_libretro.dll`
into the install + a regional Sega CD BIOS in `<exe_dir>/system/`, scan
a Sega CD ROMs folder (marked via Import Wizard), see Sega CD-themed
(sapphire blue) tiles appear in the library, and click one to launch —
without rebuilding Rust.

---

## ⬜ Phase 1 — First Sega CD game running

- ⬜ Operator validation: **Sonic CD**, **Lunar: The Silver Star Complete**, **Snatcher**, **Popful Mail** — operator playtest.
- ✅ Save state F5/F8 round-trip mid-disc — closed by cross-system save-state infra (`oa_libretro::LibretroCore::save_state / load_state`).
- ⬜ Multi-region testing — operator playtest (USA + Europe + Japan CDs).
- ⬜ CDDA streaming validation — operator playtest (Sonic CD canonical).
- ✅ Per-game cover sync via libretro-thumbnails — closed by cross-system media sync (`media::sync_media_for_system`).
- ⬜ Multi-disc title via `.m3u` (Lunar: Eternal Blue) — operator playtest.

**Acceptance gate:** A reference set of Sega CD games run with pixels +
CDDA + working controller at native 59.92 Hz NTSC.

---

## ✅ Phase 2 — Polish

- ✅ **Disc-id extraction** — shipped via `apps/oa-shell/src/cd_id.rs::extractors::sega_cd` (reads SEGADISCSYSTEM-area serial in data track); `rom_hashes` points at `metadat/redump/Sega - Mega-CD - Sega CD`.
- ✅ **Switch `rom_hashes` to redump dat ref** — shipped (see above).
- ✅ **Per-game cover sync** — closed by cross-system media sync.
- ⬜ **3-button vs 6-button compatibility map** — operator-driven KNOWN_GAME_BUGS curation (per-game pad-mode override drawer shipped cross-system).
- ⬜ **Sega CD-specific theming polish** — operator-driven UI polish (per-system theming infra shipped cross-system).

---

## ⬜ Phase 3+ — Stretch

Sega CD-specific items:

- ⬜ **32X-CD games (Night Trap 32X, Corpse Killer 32X, Slam City)** — deferred (Phase 3+; needs stacked Sega CD + 32X end-to-end validation).
- ⬜ **Game Genie / Pro Action Replay code support** — operator-driven validation of Genesis Plus GX's `retro_cheat_set`.
- ⬜ **Custom forked Sega CD core** — deferred.

---

## Scope clarifications

- **No vendoring for Sega CD today.** The libretro pivot means we ship
  the upstream nightly Genesis Plus GX .dll alongside our binary. If
  we ever modify the core, we maintain a separate libretro-frontend
  build of our patched source — see project `DECISIONS.md` 2026-05-16
  entry.
- **BIOS REQUIRED.** Unlike cart Genesis, Sega CD playback can't proceed
  without a regional BIOS. The pre-check refuses early with a clear
  error toast naming `bios_CD_U.bin / bios_CD_J.bin / bios_CD_E.bin`
  in `<exe_dir>/system/`.
- **CD extension collision with PCE-CD.** All six CD container
  extensions (`.cue / .chd / .iso / .m3u / .ccd / .toc`) are also
  claimed by PCE-CD. Disambiguation happens at Import Wizard time via
  per-folder hint — same path PCE-CD navigated. Documented in
  `DECISIONS.md`.
