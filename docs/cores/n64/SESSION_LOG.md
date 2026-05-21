# n64 Session Log

---

## 2026-05-20 — Phase 0 onboarding (paired with gamecube)

- **Shipped:** SystemId variant (N64), parse_system_id arm,
  `bindings.rs::n64` module (14-button digital — d-pad + A/B + L/R/Z +
  START + 4 C-buttons; analog stick flows separately via
  InputState.axes[0..2]). Default core Mupen64Plus-Next. Media +
  rom_hashes arms (no-intro N64 dat). CSS theme: Atomic Purple
  oklch(0.55 0.22 268), period-correct to the N64 launch + iconic
  transparent-shell variants. Per-core docs scaffold.
- **Shipped as part of this Phase 0:** the CROSS-CUTTING analog input
  infrastructure (RETRO_DEVICE_ANALOG dispatch in `oa-libretro::state`,
  per-port `input_axes [[i16; 4]; 5]` field, analog axis polling in
  `oa-input::InputPoller` via gilrs LeftStick + RightStick X/Y).
  This infra unblocks N64 + GameCube immediately and is shared with
  PSX DualShock / Saturn 3D Pad / VB right D-pad / Intv 16-way disc
  for their respective Phase 2 polish passes.
- **Almost:** Phase 1 operator validation. Gamepad-with-analog-stick
  required for the full N64 experience; keyboard-only users enable
  Mupen64Plus-Next's "Map d-pad to analog stick" core option.
- **Next:** Operator drops `mupen64plus_next_libretro.dll`, connects
  a gamepad with analog stick, launches Super Mario 64. The first
  test is whether the LeftStick smoothly moves Mario through Peach's
  Castle entrance.
