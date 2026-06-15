# Sega 32X CD — Roadmap

Onboarding status for the `sega32xcd` slug. Added in branch
`feat/new-systems-jagcd-32xcd-stv` (2026-05-27). Code-side wiring
complete; operator playtest deferred (operator picked the
"have BIOSes + ROMs for jagcd only" path during the slice 3
planning prompt).

## Phase 0 — slug wiring (✅ shipped 2026-05-27)

- ✅ Frontend `SystemId` union, `systemThemes` entry (CD container
  extensions, 4/3 tile aspect, plain default shader for FMV-heavy
  library), `systemUIConfigs` baseline entry.
- ✅ Per-system palette in `frontend/src/platform/themes/systemPalettes.ts`
  — orange-red 32X family (hue 42°), L 0.60 to read distinctly from cart
  32X. Lives in the typed `SYSTEM_PALETTES` map, injected as `[data-system]`
  CSS at boot (was the retired `systems.css` data-system block).
- ✅ `parse_system_id` accepts `"sega32xcd" | "sega-32x-cd" |
  "32xcd" | "32x-cd"` and routes to `oa_core::SystemId::SegaCd`
  (CD-shape parent — the "stacked override" pattern, no new oa-core
  variant).
- ✅ `default_core_dll_for_system` returns `picodrive_libretro.dll`
  for the sega32xcd slug — different core from plain segacd
  (Genesis Plus GX), since PicoDrive is the only mainstream
  libretro core with 32X+CD combined-mode support.
- ✅ CD-shape BIOS dispatch reuses `check_sega_cd_bios` — same
  Sega CD regional BIOS suffices; 32X cart BIOSes are optional.
- ✅ Bindings re-use `default_genesis_bindings()` via the
  `"genesis" | "segacd" | "sega32x" | "sega32xcd"` defaults_for
  arm.
- ✅ `repos_for_system_id` → `Sega_-_32X` thumbnails repo (no
  dedicated 32X-CD repo exists upstream; FMV titles are usually
  cataloged with cart 32X).

## Phase 1 — operator playtest (⬜ awaiting BIOS + ROM)

- ⬜ Drop a regional Sega CD BIOS into `<exe_dir>/system/` (operator
  may already have it from segacd playtest).
- ⬜ Launch a 32X-CD title. Recommended first launch: Night Trap
  (32X CD) — well-known, FMV-stable. Corpse Killer is the
  secondary recommendation.
- ⬜ Confirm PicoDrive auto-detects 32X+CD mode and the launch
  doesn't hit the cart-only 32X code path.
- ⬜ Confirm save-state save / load works through PicoDrive
  (32X+CD save state format is distinct from cart 32X / plain
  segacd; check there's no collision).
- ⬜ Confirm `Sega_-_32X` thumbnails repo syncs cover art cleanly
  (operator-triggered sync from per-system Media tab).
- ⬜ KNOWN_GAME_BUGS triage — populate the per-title quirks file
  from playtest notes.

## Phase 2+ — polish (deferred until Phase 1 surfaces a need)

- ⬜ Per-game `core_option` overrides for FMV-heavy titles if the
  default PicoDrive settings underperform.
- ⬜ If operators ship a separate 32X CD thumbnails repo upstream
  later, swap `repos_for_system_id` away from the cart-32X repo.

## Known limitations

PicoDrive's 32X+CD support is functional but not historically as
polished as its plain-32X mode. Some titles (Slam City, Supreme
Warrior) may have FMV-decoding glitches; benchmark against the
upstream PicoDrive issue tracker if a title underperforms.
