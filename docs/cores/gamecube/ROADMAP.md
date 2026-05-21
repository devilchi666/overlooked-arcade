# gamecube — Roadmap

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-20)

- ✅ `oa_core::SystemId::GameCube` variant + `parse_system_id` arm
  (`gamecube | gc | wii | nintendo-gamecube`).
- ✅ `bindings.rs::gamecube` module — 12-button digital (d-pad + A/B/X/Y
  + L/R/Z + START). Main stick + C-stick flow through analog axes.
- ✅ `default_core_dll_for_system("gamecube") → "dolphin_libretro.dll"`.
- ✅ `rom_hashes` → `&[]` with NO_DAT_SYSTEMS entry (large multi-format
  disc images not single-file matched).
- ✅ `media::repo_for_system_id` → `Nintendo_-_GameCube` (Wii cover
  sync via separate repo is Phase 2.5 polish).
- ✅ Frontend SystemId / systemThemes / CSS (Plan A: Indigo GameCube
  oklch(0.48 0.22 280) — deep premium indigo in the Nintendo home
  cluster).
- ✅ Per-core docs scaffold.
- ✅ **Analog infra shipped alongside n64 onboarding** — gamepad
  LeftStick → main stick, RightStick → C-stick, both flow through
  RETRO_DEVICE_ANALOG dispatch.

**Acceptance gate:** Operator drops `dolphin_libretro.dll`, scans
GameCube + Wii ISOs, sees Indigo-themed tiles, launches a known-good
game with gamepad analog sticks driving movement + C-stick aim.

---

## ⬜ Phase 1 — First GC game running

- ⬜ Operator validation: Smash Melee (C-stick), Wind Waker, RE4 (analog L/R triggers), Metroid Prime, Pikmin — operator playtest.
- ⬜ Analog L/R trigger sensitivity test — gated on shared pressure-sensitive analog-trigger infra (shared with PS2/DC).
- ✅ Save state F5/F8 round-trip — closed by cross-system save-state infra (`oa_libretro::LibretroCore::save_state / load_state`).
- ⬜ Wii ISO smoke-test (Wii Sports / Mario Kart Wii via classic-controller input — motion controls deferred) — operator playtest.

---

## ⬜ Phase 2 — Polish

- ⬜ **Wii Remote / Nunchuk / Classic Controller** dispatch — gated on new libretro device-type plumbing.
- ✅ **Per-axis keyboard binding** for main stick and C-stick — closed by cross-system analog axes (`InputState.axes` + `compute_stick_output` with keyboard fallback).
- ✅ **Disc-id extraction** — shipped via `apps/oa-shell/src/cd_id.rs::extractors::gamecube` (6-byte DOL game ID at offset 0); `rom_hashes` points at `metadat/redump/Nintendo - GameCube`.
- ✅ **GC + Wii cover sync split** — shipped via `apps/oa-shell/src/media.rs::repos_for_entry` + `is_wii_dump` (per-game-region routing).

---

## ⬜ Phase 3+ — Stretch

- ⬜ **GameCube memory card** UX — operator-driven validation of Dolphin .raw cards.
- ⬜ **Wii motion-controls** beyond basic Wii Remote dispatch — gated on motion-sensor + IR-pointer infra.
- ⬜ **Triforce arcade hardware** — F-Zero AX, Mario Kart Arcade GP 1+2 — deferred (niche).
