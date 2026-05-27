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
- ✅ Analog L/R trigger sensitivity test — closed by Phase B per-button analog pressure (`InputState.analog_buttons[12..=13]` carries gilrs trigger axes through `cb_input_state` RETRO_DEVICE_INDEX_ANALOG_BUTTON). Same shared infra closed PS2 + DC. Operator playtest pending against RE4 / F-Zero GX.
- ✅ GameCube vibration / Wii Remote rumble — closed by Phase F rumble interface (`RETRO_ENVIRONMENT_GET_RUMBLE_INTERFACE` wired through to gilrs).
- ✅ Save state F5/F8 round-trip — closed by cross-system save-state infra (`oa_libretro::LibretroCore::save_state / load_state`).
- ⬜ Wii ISO smoke-test (Wii Sports / Mario Kart Wii via classic-controller input — motion controls deferred) — operator playtest.

---

## ⬜ Phase 2 — Polish

- ✅ **Wii Remote / Nunchuk / Classic Controller** dispatch — shipped
  2026-05-27 via the per-game device-type override that's already
  wired (`arm_libretro_device` dispatches arbitrary u32s; no new
  Rust plumbing needed). Dolphin's libretro fork uses
  hand-encoded `((N << 8) | base)` subclass values (NOT the
  canonical libretro `RETRO_DEVICE_SUBCLASS` macro's `+1`
  convention — see `Source/Core/DolphinLibretro/Input.cpp:48-54`).
  Operator picks per-game in the Input dialog dropdown; the
  GameCube row of `DEVICE_ID_OPTIONS_GAMECUBE` in
  `frontend/src/components/GameDialogs.tsx` adds:
  - 513 — Wii Remote (sideways grip)
  - 769 — Wii Remote + Nunchuk
  - 1025 — Wii Remote + Classic Controller
  - 1281 — Wii Remote + Classic Controller Pro
  - 1537 — GameCube Controller in Wii mode (Wii U adapter slots)
  Real WiiMote / Bluetooth passthrough (1536) intentionally
  skipped — needs host Bluetooth pairing OA doesn't wire.
  Per-game hint block in the Additional Ports section explains
  which Wii titles need which peripheral (Skyward Sword →
  Nunchuk, Brawl → Classic Controller, NSMB Wii → sideways,
  etc.). Operator playtest pending.
- ✅ **Per-axis keyboard binding** for main stick and C-stick — closed by cross-system analog axes (`InputState.axes` + `compute_stick_output` with keyboard fallback).
- ✅ **Disc-id extraction** — shipped via `apps/oa-shell/src/cd_id.rs::extractors::gamecube` (6-byte DOL game ID at offset 0); `rom_hashes` points at `metadat/redump/Nintendo - GameCube`.
- ✅ **GC + Wii cover sync split** — shipped via `apps/oa-shell/src/media.rs::repos_for_entry` + `is_wii_dump` (per-game-region routing).

---

## ⬜ Phase 3+ — Stretch

- ⬜ **GameCube memory card** UX — operator-driven validation of Dolphin .raw cards.
- ⬜ **Wii motion-controls** beyond basic Wii Remote dispatch — gated on motion-sensor + IR-pointer infra.
- ⬜ **Triforce arcade hardware** — F-Zero AX, Mario Kart Arcade GP 1+2 — deferred (niche).
