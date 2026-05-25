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

## 🟨 Phase 1 — First NES ROM running

- ⬜ Operator validation: launch Super Mario Bros (USA) or another reference ROM. Confirm pixels + audio + controller.
- ⬜ FDS validation with `disksys.rom` in `<exe_dir>/system/`.
- ✅ Per-game cover sync via libretro-thumbnails — mapping `nes → Nintendo_-_Nintendo_Entertainment_System` shipped in `apps/oa-shell/src/media.rs::repos_for_system_id` (one of the most complete repos in the libretro-thumbnails family).

---

## Phase 2 — Polish

- ⬜ Mesen swap validation — the higher-accuracy alternative drops in as a per-system Cores override (`SystemSettings`-level core picker shipped); operator validation pending.
- ⬜ Per-system shader tweaks: NES's 256×224 visible area is similar to NTSC TV resolution; the default scanline preset reads cleanly. CrtLite is the natural per-system default once an operator picks one — operator preference call.
- ⬜ Light gun (Zapper) — operator validation. Code wiring shipped 2026-05-25 on `feat/light-gun-harness`: `cb_input_state` LIGHTGUN dispatch via `lightgun_field_value` in `crates/oa-libretro/src/state.rs` covers SCREEN_X / SCREEN_Y / TRIGGER + deprecated relative X/Y aliases. FCEUmm Zapper polls these and now sees the operator's mouse-as-gun coords. Catalogued in `apps/oa-shell/src/light_gun_systems.rs::LIGHT_GUN_SYSTEMS`. Flagship validation: Duck Hunt / Hogan's Alley / Wild Gunman.
