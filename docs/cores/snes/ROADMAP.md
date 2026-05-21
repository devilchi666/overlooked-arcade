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

## ⬜ Phase 1 — First SNES ROM running

- ⬜ Operator validation: launch Super Mario World or Chrono Trigger as a reference ROM. Confirm pixels + audio + controller.
- ⬜ Special chip game validation (Super Mario RPG / Star Fox / Yoshi's Island — exercise SA-1 / SuperFX paths in the core).
- ⬜ Per-game cover sync via libretro-thumbnails — **infra ready 2026-05-19, needs operator validation.** Mapping `snes → Nintendo_-_Super_Nintendo_Entertainment_System` shipped in `apps/oa-shell/src/media.rs::repo_for_system_id` (one of the most complete libretro-thumbnails repos). Operator: run `Settings → Library → Sync media for SNES` and confirm covers download.

---

## ⬜ Phase 2 — Polish

- ⬜ bsnes swap validation — the higher-accuracy alternative via per-system Cores override.
- ⬜ Hi-res game validation (Secret of Mana 2-player split-screen, RPM Racing menus, R-Type III) — 512×448 mode.
- ⬜ Mouse support — Mario Paint, ACME Animation Factory. libretro exposes SNES Mouse; needs a per-game device-type setting.
- ⬜ Super Multitap support — Bomberman games, etc.

---

## 2026-05-21 — Stale-cleanup audit

The Phase 1+ items above were written when this system onboarded, before cross-system infrastructure (Phases 1.5 / 2.5–2.8 / 3 / 4 + direct-launch CLI) landed. Many `⬜` items are actually shipped — see `docs/cores/AUDIT_2026-05-21.md` for the per-item breakdown (stale vs open-code vs open-operator) for this system.
