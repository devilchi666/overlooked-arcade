# psx — Roadmap

Per-core phase tracking for Sony PlayStation (PS1). Mirrors the
project-wide ROADMAP shape but scoped to PSX.

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-20)

Paired with Saturn. Heavyweight CD-shape onboarding. Beetle PSX HW is
the recommended default; Beetle PSX SW pre-registered as a recommended
catalog peer for hosts where HW fails to obtain a Vulkan/OpenGL
surface from our wgpu DX12 backend.

- ✅ System registered in `frontend/src/themes/registry.ts` — `SystemId`
  union extended with `psx`, `systemThemes.psx` entry (CD container
  extensions + `.pbp`, landscape 4/3 tile, `crt-lite` shader preset).
- ✅ Theme block in `frontend/src/themes/systems.css` — teal cyan at
  hue 180° + L=0.65 + C=0.16. Open band (175-185°) — no hue crowding.
  Evokes PS1 launch palette's cool blue/cyan/silver identity.
- ✅ Per-system input wiring — 14-button digital DualPad module in
  `bindings.rs::psx` + `PSX_BUTTONS` table + `default_psx_bindings()`
  + `psx_to_libretro_bits` identity remap + all 4 dispatch arms.
  Three new tests lock the dispatch.
- ✅ `default_core_dll_for_system("psx") → "mednafen_psx_hw_libretro.dll"`
  in `apps/oa-shell/src/main.rs`. Alternates documented (Beetle PSX
  SW as catalog peer, SwanStation as additional alternate).
- ✅ `parse_system_id("psx" | "ps1" | "ps" | "playstation") →
  SystemId::Playstation` (new variant on `oa_core::SystemId` enum).
- ✅ `rom_hashes::libretro_dat_refs_for_system("psx")` returns `&[]`
  with NO_DAT_SYSTEMS entry — CD images aren't single-file SHA-1
  matched; disc-id extraction via `cd_id.rs` PSX branch is Phase 2.
- ✅ `media::repo_for_system_id("psx")` returns
  `Some("Sony_-_PlayStation")` so cover sync works as soon as the
  operator runs it.
- ✅ BIOS pre-check via `check_psx_bios` in main.rs — six canonical
  SHA-1 entries (JP v3.0, US v3.0, EU v3.0, US v4.1, US v4.4, US
  v2.2/PSone). CD-launch path's BIOS dispatch arm extended with
  `"psx"` branch.
- ✅ `.pbp` extension added to `is_cd_extension` — the PSP-format PS1
  EBOOT container needs the BIOS pre-check + path-based loading like
  other CD images.
- ✅ Per-core docs scaffold (this directory).

**Acceptance gate:** Operator can drop `mednafen_psx_hw_libretro.dll`
into the install + a regional PSX BIOS in `<exe_dir>/system/`, mark a
PSX ROMs folder via Import Wizard, see PSX-themed (teal cyan) tiles
appear in the library, and click one to launch.

---

## ⬜ Phase 1 — First PSX game running

- ⬜ Operator validation: **SotN**, **FF7** (3-disc — also tests .m3u), **MGS** (2-disc), **Crash Bandicoot**, **Resident Evil** — operator playtest.
- ✅ Save state F5/F8 round-trip mid-disc — closed by cross-system save-state infra (`oa_libretro::LibretroCore::save_state / load_state`).
- ⬜ HW renderer surface validation — operator-side confirmation that Beetle PSX HW obtains a Vulkan/OpenGL surface from our wgpu DX12 host.
- ⬜ Multi-region testing — operator playtest (JP + US + EU discs with matching regional BIOSes).
- ⬜ Multi-disc title via `.m3u` (Final Fantasy VII) — operator playtest.
- ✅ Per-game cover sync via libretro-thumbnails — closed by cross-system media sync (`media::sync_media_for_system`).
- ⬜ `.pbp` launch validation — operator playtest of a known-good PSone Classics .pbp.

**Acceptance gate:** A reference set of PSX games run with pixels +
audio + working digital DualPad at native 59.94 Hz NTSC.

---

## ⬜ Phase 2 — Polish

- ✅ **DualShock analog stick support** — closed by Phase A device-type override (operator picks "Analog / Paddle" = `RETRO_DEVICE_ANALOG` in per-game Input for Ape Escape / Spyro / later analog-required PSX titles). Analog axes infra was already shipped cross-system. Operator playtest pending.
- ✅ **DualShock rumble** — closed by Phase F rumble interface.
- ✅ **Disc-id extraction** — shipped via `apps/oa-shell/src/cd_id.rs::extractors::psx_family` (reads SLUS_/SCUS_/SLES_/SCES_/SLPS_/SLPM_/SCPS_ prefixes from SYSTEM.CNF); `rom_hashes` points at `metadat/redump/Sony - PlayStation`.
- ⬜ **HW vs SW perf benchmarks** — operator-driven DECISIONS doc.
- ⬜ **PGXP geometry correction** — operator-driven per-game core-option curation (per-game core-options drawer shipped cross-system).
- ⬜ **Light gun support** — operator validation. LIGHTGUN dispatch shipped 2026-05-25 on `feat/light-gun-harness`; IS_OFFSCREEN reload-by-aim flag plumbed 2026-05-27 via the new `in_viewport` field on `InputState.pointer` (`crates/oa-libretro/src/state.rs::lightgun_field_value`). Beetle PSX polls RETRO_DEVICE_LIGHTGUN for both the Namco GunCon (Time Crisis 1/2, Point Blank trilogy) AND the Konami Justifier (Lethal Enforcers, Crypt Killer); SCREEN_X/Y/TRIGGER/IS_OFFSCREEN reach the core. Time Crisis-style reload-by-aiming-off-screen now functional. Catalogued in `apps/oa-shell/src/light_gun_systems.rs`.
- ✅ **Light-gun gun-side buttons** (AUX_A/B/C + START + SELECT + DPAD + RELOAD) — shipped 2026-05-30 via Phase 4 of `feat/gameplay-fixes-batch`. New `oa_core::InputState.lightgun_buttons: u32` + State mirror + bit-keyed `lightgun_field_value` dispatch. Bindings derive from per-port RetroPad bits via `oa_input::lightgun_buttons_from_joypad_bits`. Time Crisis pedal-reload (foot-pedal-on-button alternative — the off-screen-aim gesture also reloads via IS_OFFSCREEN; both paths active) reaches the core through LIGHTGUN_RELOAD. Justifier's A/B/Start gun-side buttons (Lethal Enforcers menu nav) map to AUX_A/B/START.

---

## ⬜ Phase 3+ — Stretch

- ⬜ **PSP-via-PSX backwards-compat polish** — `.pbp` metadata surface in library tile is operator-driven polish.
- ⬜ **Memory card UX** — operator-driven polish.
- ⬜ **NetYaroze / dev BIOS variants** — deferred (extremely niche).

---

## Scope clarifications

- **No vendoring for PSX today.** Operator drops the buildbot .dll.
- **BIOS REQUIRED.** PSX region-locks at the BIOS level — JP discs
  need scph5500.bin, US discs need scph5501.bin, EU discs need
  scph5502.bin.
- **CD extension collision** with PCE-CD / segacd / saturn. `.pbp`
  is PSX-unique (no collision).
- **HW vs SW core peers.** Both are mature Beetle PSX builds; same
  BIOS file set, same compatibility profile, different rendering
  paths. HW is the visually-premium default; SW is the bulletproof
  fallback.
- **DualShock analog sticks shipped (2026-05-21).** Closed by shared
  analog input infra Phase A (PSX device-type override = ANALOG) +
  the already-shipped cross-system analog axes infra. Operator picks
  "Analog / Paddle" in per-game Input for Ape Escape and later
  analog-required titles.
