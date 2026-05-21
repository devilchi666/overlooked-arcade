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

- ⬜ Operator validation: Super Smash Bros. Melee (analog C-stick for
  smash attacks — the canonical "is C-stick working" test), Wind
  Waker, Resident Evil 4 (analog L/R triggers test), Metroid Prime
  (free-aim via C-stick), Pikmin.
- ⬜ Analog L/R trigger sensitivity test (RE4's aim is famously
  trigger-pressure-sensitive on real hardware; Dolphin handles via
  digital-press → analog-pressure synthesis).
- ⬜ Save state F5/F8 round-trip.
- ⬜ Wii ISO smoke-test (Wii Sports / Mario Kart Wii via classic-
  controller input — motion controls deferred).

---

## ⬜ Phase 2 — Polish

- ⬜ **Wii Remote / Nunchuk / Classic Controller** dispatch. Phase
  2.5 — needs new libretro device-type plumbing (Wii Remote isn't a
  standard RetroPad; it's a separate libretro device class with IR
  pointer + accelerometer + 3-axis gyro).
- ⬜ **Per-axis keyboard binding** for main stick and C-stick (Phase
  2.5).
- ⬜ **Disc-id extraction** — GC discs key on 6-byte game ID at
  offset 0 of the disc header. cd_id.rs extension.
- ⬜ **GC + Wii cover sync split** — Wii thumbnails live in a
  separate libretro-thumbnails repo; need per-game-region cover sync
  routing.

---

## ⬜ Phase 3+ — Stretch

- ⬜ **GameCube memory card** UX — Dolphin handles .raw memory cards;
  per-game surface needed.
- ⬜ **Wii motion-controls** beyond basic Wii Remote dispatch.
  Pointer-based games (RE4 Wii Edition, Metroid Prime 3) need IR
  cursor support.
- ⬜ **Triforce arcade hardware** — F-Zero AX, Mario Kart Arcade GP
  1+2. Niche.

---

## 2026-05-21 — Stale-cleanup audit

The Phase 1+ items above were written when this system onboarded, before cross-system infrastructure (Phases 1.5 / 2.5–2.8 / 3 / 4 + direct-launch CLI) landed. Many `⬜` items are actually shipped — see `docs/cores/AUDIT_2026-05-21.md` for the per-item breakdown (stale vs open-code vs open-operator) for this system.
