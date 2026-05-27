# mame Roadmap

Per-core phase tracking for MAME / Arcade. Mirrors `docs/ROADMAP.md` but only the mame slice.

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## 🟨 Phase 1 — Onboarding (2026-05-19)

- ✅ `mame` slug added to `frontend/src/themes/registry.ts` (extensions: `.zip` + `.chd`; theme block in `themes/systems.css` — neon arcade red at hue 12°).
- ✅ `oa_core::SystemId::Mame` variant added.
- ✅ `apps/oa-shell/src/bindings.rs` — `mame` button module (B1–B6 + START + COIN + d-pad, identity-mapped to libretro RetroPad), `MAME_BUTTONS` iteration order, `default_mame_bindings()`, dispatch arms in `bit_for` / `buttons_for` / `to_libretro_bits` / `defaults_for`. 14 unit tests green.
- ✅ `default_core_dll_for_system("mame") → "mame_libretro.dll"` in `apps/oa-shell/src/main.rs`.
- ✅ `parse_system_id("mame" | "arcade") → SystemId::Mame`.
- ✅ Catalog slug renamed `arcade-mame` → `mame` (5 MAME variants); `arcade-fbneo` → `fbneo` for symmetry.

**Acceptance gate:** A real ROM set launches, plays, and accepts input. Pending operator validation.

---

## 🟨 Phase 1.5 — Hardening (post-Phase-1)

Decision 2026-05-19 (project DECISIONS.md "Keyboard-heavy systems"): handle MAME's full input range via the three-phase hybrid below, NOT by inflating the OA bindings table indefinitely or punting entirely to MAME's native menu.

### ✅ Input — Phase 1 (small, ships first)

- ✅ Added `SERVICE`, `MAME_MENU`, `P2_START`, `P2_COIN` to `MAME_BUTTONS` in `apps/oa-shell/src/bindings.rs`. Default keys: `F2` (Service — RetroArch convention), `Tab` (MAME menu — also RetroArch convention), `Key2` (P2 Start), `Key6` (P2 Coin). Parked on libretro bits L3/R3/R2/L2 respectively (the four bits otherwise unused by the arcade base layer). Updated `mame_to_libretro_bits` mask and the `default_mame_bindings` defaults table. 14/14 bindings tests green.
- ✅ Documented the TAB workflow + the four system buttons in `docs/cores/mame/README.md` — "for non-standard controls (driving, lightgun, mahjong, pinball), press TAB in-game to open MAME's input config. MAME stores remaps per-driver under `<appData>/cfg/<driver>.cfg`."
- ✅ Verify the new buttons round-trip — clarified 2026-05-27 in the system-fixes branch. MAME's libretro core consumes Service / Tab / P2 inputs via the keyboard device callback (RETROK_F2 / RETROK_TAB / RETROK_2 / RETROK_6), NOT the RetroPad bits the four `MAME_BUTTONS` entries map to. The Phase 2 keyboard-passthrough shipped earlier (`oa-libretro`'s `retro_set_keyboard_callback` + the frame-driven keyboard pump in the emu thread firing `send_keyboard_event` per F2 / Tab / etc. press) is the actual live-wiring path; the RetroPad-bit mappings remain in `MAME_BUTTONS` as future-proofing in case MAME's libretro support adds RetroPad routing for these buttons later. Operator playtest of the keyboard-pump round-trip remains gated under the Phase 1.5 "real ROM set against MAME 0.287" acceptance below.

### ✅ Input — Phase 2 (keyboard passthrough infrastructure, cross-system unlock)

Lives in `oa-libretro`, NOT under MAME. Benefits MSX + every future computer-shaped system simultaneously. Shipped as commits `3acb696` (oa-libretro plumbing), `4aac0f5` (shell pump + SystemSettings flag), `<slice-3>` (Game-focus toggle).

- ✅ `retro_set_keyboard_callback` registration: `RETRO_ENVIRONMENT_SET_KEYBOARD_CALLBACK` moves out of `oa-libretro`'s decline list and stashes the callback in `State.keyboard_cb`. `LibretroCore::send_keyboard_event` calls it under the dropped-mutex pattern (the core may re-enter via GET_LOG_INTERFACE). `LibretroCore::has_keyboard_callback()` lets the shell short-circuit the pump when the core declined keyboard input.
- ✅ Frame-driven keyboard pump in the emu thread (no dedicated `KeyboardEvent` channel was needed — `oa-input::InputPoller::pressed_keys()` returns the current snapshot and the diff happens inline against a `HashSet<Keycode>` from the previous frame). Edge-detects press / release transitions and forwards them via `send_keyboard_event`. On focus-loss / passthrough-disabled / core-dropped, fires release events for every still-held key so the core doesn't see them as stuck.
- ✅ `oa_libretro::keycode_to_retro_key(device_query::Keycode) → u32` — full enum coverage, lowercase ASCII letters, KP keyspace, F1-F15, navigation cluster, L/R modifier distinction, macOS Option-as-ALT + Command-as-META folding. F16-F20 → `RETROK_UNKNOWN` since libretro stops at F15. 9 unit tests.
- ✅ `SystemSettings::keyboard_passthrough: Option<bool>` with `default_keyboard_passthrough(system_id)` → true for `mame` / `msx` / `msx2`, false everywhere else. `effective_keyboard_passthrough()` resolves override-vs-default. Refreshed on every LoadRom.
- ✅ `Tools → Game focus` MenuCheckbox + Ctrl+G hotkey. (Scroll Lock isn't queryable via `device_query` so the proposal's primary binding was downgraded to its fallback.) Status chip in `toolbarRight` visible only when active. Rising-edge detector in the emu thread runs unconditionally so the user can always toggle out; `oa://game-focus-changed` Tauri event syncs the frontend signal. Game-focus ON gates `hotkeys_enabled = enable && !game_focus`; F1/F2/F3/F5/F6/F7/F8/F12/Esc/digit/Backspace-rewind reads all use `hotkeys_enabled`.
- ⬜ Wiring `RETRO_DEVICE_KEYBOARD` via `LibretroCore::set_port_device` for MAME — **deliberately skipped.** MAME's libretro core expects JOYPAD on port 0 (arcade controls) with the keyboard callback running in parallel; rebinding port 0 to KEYBOARD would remove the 6-button arcade input. `set_port_device` stays available for the MSX onboarding which DOES need port-as-keyboard.

### ✅ Input — Phase 3 (analog)

Closed by shared analog input infra Phases A-D (2026-05-21).

- ✅ gilrs analog axes flowing through `cb_input_state` (shipped cross-system).
- ✅ Per-port analog routing schema lives in `system_settings::AnalogRoutingPrefs` + `oa_input::AnalogRouting` — deadzone, sensitivity, invert, keyboard fallback, mouse-as-stick.
- ✅ `RETRO_DEVICE_ANALOG` wired per-port via `arm_libretro_device` (per-game device-type override).
- ✅ Per-game profile selection — operator picks device-type "Analog / Paddle" in per-game Input + mouse-as-stick source (X for steering wheel + paddle, XY for trackball, Y for yoke pitch). Operator playtest pending against OutRun (wheel), Marble Madness (trackball), Arkanoid (Vaus paddle), After Burner II (yoke).

### Other Phase 1.5 items

- ⬜ Validate a known-good ROM set against MAME 0.287 — operator playtest.
- ⬜ Verify the 6-button SF mapping feels right on a real cabinet stick / fight pad — operator playtest of alternate "SF-native" defaults.
- ⬜ Per-game ROM-set name resolution — operator-driven curation (MAME listxml metadata pass deferred).
- ⬜ `.chd` arcade games — operator playtest against Killer Instinct / Atari System 2.
- ⬜ Verify aspect-ratio handling — operator validation that the renderer respects libretro rotation flag (per-system aspect override infra is shipped cross-system).

---

## ⬜ Phase 2+ contributions

- ⬜ MAME-specific button glyphs for the bindings UI — operator polish (bindings UI button-name chips shipped cross-system via `SystemBindingsEditor.tsx:226`).
- ✅ Per-game ROM-set metadata via libretro thumbnails — closed by cross-system media sync (`media::sync_media_for_system`); listxml metadata pass is deferred separately.
- ⬜ Multi-game-per-zip handling — operator-driven curation decision.
