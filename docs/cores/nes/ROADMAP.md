# nes — Roadmap

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-18)

- ✅ Registry entry + theme block (NES crimson accent).
- ✅ Per-system bindings (8 buttons, identity libretro remap, Z/X action keys + Enter/RShift + arrows).
- ✅ `default_core_dll_for_system("nes") → "fceumm_libretro.dll"` (Mesen swap via per-system Cores settings).
- ✅ Per-core docs scaffold.

**Acceptance gate:** Operator can drop `fceumm_libretro.dll` into the install, scan a folder of `.nes` ROMs, see NES-themed tiles, launch a game.

---

## ⬜ Phase 1 — First NES ROM running

- ⬜ Operator validation: launch Super Mario Bros (USA) or another reference ROM. Confirm pixels + audio + controller.
- ⬜ FDS validation with `disksys.rom` in `<exe_dir>/system/`.
- ⬜ Per-game cover sync via libretro-thumbnails — **infra ready 2026-05-19, needs operator validation.** Mapping `nes → Nintendo_-_Nintendo_Entertainment_System` shipped in `apps/oa-shell/src/media.rs::repo_for_system_id` (one of the most complete repos in the libretro-thumbnails family). Operator: run `Settings → Library → Sync media for NES` and confirm covers download.

---

## ⬜ Phase 2 — Polish

- ⬜ Mesen swap validation — the higher-accuracy alternative should drop in as a per-system Cores override without further wiring.
- ⬜ Per-system shader tweaks: NES's 256×224 visible area is similar to NTSC TV resolution; the default scanline preset reads cleanly. CrtLite is the natural per-system default once shader presets persist.
- ⬜ Light gun support — Zapper games need a special input device; libretro exposes this and OA's input layer can wire it once a per-game device-type setting lands.
