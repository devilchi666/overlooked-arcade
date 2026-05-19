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
- ⬜ Verify the new buttons round-trip through `to_libretro_bits("mame", …)` to the right libretro IDs against a running MAME core (test confirms the identity-mask is correct in software; live wiring to MAME's actual operator inputs depends on Phase 2 keyboard-passthrough since MAME's libretro core today routes Service / Tab through the keyboard device, not RetroPad bits).

### ⬜ Input — Phase 2 (keyboard passthrough infrastructure, cross-system unlock)

Lives in `oa-libretro`, NOT under MAME. Benefits MSX + every future computer-shaped system simultaneously.

- ⬜ Implement `retro_set_keyboard_callback` registration in `oa-libretro::loader`.
- ⬜ Add a `KeyboardEvent { down: bool, keycode: u32, character: u32, modifiers: u16 }` channel from `oa-input` to the emu thread; emu thread forwards to the core when keyboard device is enabled for any active port.
- ⬜ Translate `device_query::Keycode` → libretro `retro_key` values (a ~150-entry lookup table; reference: `libretro.h`'s `retro_key` enum).
- ⬜ Add a `keyboard_passthrough: bool` flag to `system_settings::SystemSettings` defaulting to `true` for `"mame"` and `"msx"` / `"msx2"`, `false` everywhere else.
- ⬜ Add a `Tools → Game focus` checkbox + hotkey toggle (proposed: `Scroll Lock`, with `Ctrl+G` fallback for tenkeyless keyboards). When ON, OA hotkeys (F1-F8 / Esc / Ctrl+W / etc.) pass through to the core instead of triggering OA actions. Status chip in the toolbar so user knows which mode is active.
- ⬜ Wire `RETRO_DEVICE_KEYBOARD` for the relevant port in `LibretroCore::set_controller_port_device` once MAME loads — needs to fire AFTER `retro_load_game` per the existing Mednafen-pattern memory.

### ⬜ Input — Phase 3 (analog, deferred until forced)

Triggered by a real game demanding it: OutRun (steering wheel), Marble Madness (trackball), Arkanoid (Vaus paddle), After Burner II (yoke).

- ⬜ Extend `oa-input::InputPoller` to surface gilrs analog axes alongside button bits.
- ⬜ Design a parallel axis-binding schema in `bindings.rs` (bitmask wrong shape for 0-65535 axis values). Likely a sibling `AnalogBinding` map keyed by libretro analog-axis IDs.
- ⬜ Wire `RETRO_DEVICE_ANALOG` for ports that want it.
- ⬜ Per-game profile selection (steering uses analog X for wheel + analog Y for pedals; trackball uses analog X/Y for ball movement; paddle uses analog X only).

### Other Phase 1.5 items

- ⬜ Validate a known-good ROM set against MAME 0.287 (the buildbot version operator installed). Suggested test sets: `pacman.zip` (Namco), `sf2ce.zip` (Capcom CPS1), `mslug.zip` (Neo Geo via MAME — needs `neogeo.zip` BIOS alongside).
- ⬜ Verify the 6-button SF mapping feels right on a real cabinet stick / fight pad; consider shipping an "SF-native" alternate default that puts B1-B3 on the top row.
- ⬜ Per-game ROM-set name resolution. Today the library tile shows the .zip filename; surfacing the human title (`Street Fighter II: Champion Edition`) requires a MAME-style metadata DB lookup — defer until per-game metadata sync work.
- ⬜ `.chd` arcade games — exercise against a known-good set (Killer Instinct, Atari System 2).
- ⬜ Verify aspect-ratio handling — many arcade boards run native rotation (Pac-Man = vertical 224×288); the renderer needs to read the libretro rotation flag.

---

## ⬜ Phase 2+ contributions

- ⬜ MAME-specific button glyphs for the bindings UI (LP/MP/HP/LK/MK/HK overlay icons).
- ⬜ Per-game ROM-set metadata (year, manufacturer, hardware) via MAME's listxml output or libretro thumbnails.
- ⬜ Multi-game-per-zip handling — some MAME sets are bundles (clones, alternate versions); decide whether to surface clones in the library or hide them.
