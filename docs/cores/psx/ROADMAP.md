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

- ⬜ Operator validation: launch a real PSX CD image end-to-end
  (pixels + audio + CDDA + 14-button controller). Suggested reference
  set: **Castlevania: Symphony of the Night** (single-disc, broad
  compat), **Final Fantasy VII** (3-disc — also tests .m3u),
  **Metal Gear Solid** (2-disc), **Crash Bandicoot** (US),
  **Resident Evil** (US/EU). Pick a disc that matches a BIOS region
  the operator has on hand.
- ⬜ Save state F5/F8 round-trip mid-disc.
- ⬜ HW renderer surface validation — confirm Beetle PSX HW can
  obtain a Vulkan/OpenGL surface from our wgpu DX12 host. If it fails,
  operator swaps to Beetle PSX SW via the per-system Cores dialog (no
  manual .dll install needed since SW is a pre-registered catalog
  peer).
- ⬜ Multi-region testing: load JP + US + EU discs with matching
  regional BIOSes to confirm region auto-detect.
- ⬜ Multi-disc title via `.m3u` — **Final Fantasy VII** (3 discs) is
  the canonical PSX multi-disc test.
- ⬜ Per-game cover sync via libretro-thumbnails — **infra ready 2026-05-20,
  needs operator validation.** Mapping `psx → Sony_-_PlayStation`
  shipped in `media::repo_for_system_id`.
- ⬜ `.pbp` launch validation — try a known-good PSone Classics .pbp
  to confirm the PSP-format container path works through Beetle PSX HW.

**Acceptance gate:** A reference set of PSX games run with pixels +
audio + working digital DualPad at native 59.94 Hz NTSC.

---

## ⬜ Phase 2 — Polish

- ⬜ **DualShock analog stick support** — Phase 2 polish alongside
  shared analog-input infra. Blocks Ape Escape (the only PSX game
  requiring DualShock to play at all) and many later PSX titles
  (Tony Hawk 2/3/4, Crash 3, MGS) that use the right stick.
- ⬜ **Disc-id extraction** — PSX discs key against SYSTEM.CNF off the
  data track for the boot binary filename (e.g.
  "BOOT = cdrom:\\SLUS_007.06;1"). Extend `apps/oa-shell/src/cd_id.rs`
  with a PSX branch + switch `rom_hashes::libretro_dat_refs_for_system("psx")`
  from `&[]` to `&[DatRef { subdir: "metadat/redump", basename: "Sony - PlayStation" }]`.
- ⬜ **HW vs SW perf benchmarks** — operator-side comparison on
  representative hosts (low-end laptop, mid-range desktop, high-end
  gaming PC). Document in `DECISIONS.md` so operators know what to
  expect.
- ⬜ **PGXP geometry correction** — Beetle PSX HW's PGXP feature
  smooths the famously-shaky PSX 3D geometry. Operator surfaces it
  via core options; per-game override should expose a one-click toggle.
- ⬜ **Light gun support** (Time Crisis, Point Blank, Die Hard Trilogy).
  Beetle PSX HW supports the GunCon via libretro pointer device.

---

## ⬜ Phase 3+ — Stretch

- ⬜ **PSP-via-PSX backwards-compat polish** — `.pbp` containers may
  carry additional metadata (icon, save data) the PSP exposed; Beetle
  PSX HW handles the game data correctly but additional metadata
  surface in the library tile is a polish item.
- ⬜ **Memory card UX** — PSX memory cards live in
  `appDataDir/saves/psx/<game-stem>.mcr` per Beetle PSX convention.
  OA's library shows save state slots (F5/F8); per-game memory card
  visibility is a polish item.
- ⬜ **NetYaroze / dev BIOS variants** — extremely niche.

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
- **DualShock analog sticks deferred.** Phase 2 polish — shared
  analog-input infra.
