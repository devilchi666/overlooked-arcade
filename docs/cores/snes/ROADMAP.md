# snes — Roadmap

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-18)

- ✅ Registry entry + theme block (SNES violet accent — Nintendo's launch palette).
- ✅ Per-system bindings (12 buttons including X/Y face + L/R shoulders, identity libretro remap).
- ✅ `default_core_dll_for_system("snes") → "snes9x_libretro.dll"` (bsnes swap via per-system Cores settings).
- ✅ Per-core docs scaffold.

**Acceptance gate:** Operator can drop `snes9x_libretro.dll` into the install, scan a folder of `.sfc` ROMs, see SNES-themed tiles, launch a game.

---

## 🟨 Phase 1 — First SNES ROM running

- ⬜ Operator validation: launch Super Mario World or Chrono Trigger as a reference ROM. Confirm pixels + audio + controller.
- ⬜ Special chip game validation (Super Mario RPG / Star Fox / Yoshi's Island — exercise SA-1 / SuperFX paths in the core).
- ✅ Per-game cover sync via libretro-thumbnails — `snes → Nintendo_-_Super_Nintendo_Entertainment_System` shipped in `apps/oa-shell/src/media.rs::repos_for_system_id`.

---

## Phase 2 — Polish

- ⬜ bsnes swap validation — operator drops the higher-accuracy alternative via per-system Cores override (`SystemSettings`-level core picker shipped).
- ⬜ Hi-res game validation (Secret of Mana 2-player split-screen, RPM Racing menus, R-Type III) — 512×448 mode; operator-driven.
- ✅ Mouse support — per-game device-type override at id=2 ("SNES Mouse" label) shipped via the generic per-game Input dropdown in `frontend/src/platform/components/GameDialogs.tsx::systemSpecificDeviceLabel("snes", 2)`. Operator picks SNES Mouse per-game (Mario Paint, ACME Animation Factory); `arm_libretro_device` dispatches it to snes9x via `retro_set_controller_port_device`. Operator playtest of the flagship titles is a separate operator-driven gate.
- ✅ Super Multitap support — `DEVICE_ID_OPTIONS_SNES` subclass id 257 (`((1 << 8) | RETRO_DEVICE_JOYPAD)`, snes9x's hand-encoded CTL_MP5 wire value, matching the Dolphin pattern) in `frontend/src/platform/components/GameDialogs.tsx`; operator picks "Super Multitap (4-port adapter)" per-game per-port; `arm_libretro_device` already dispatches arbitrary u32s. Hint block names Super Bomberman 3/4/5 + Panic Bomber W (two-port = 8 players) and the 5-player titles (one-port). Operator playtest of the 8-player Bomberman flagship titles remains the validation gap.
