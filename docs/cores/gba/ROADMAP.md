# gba — Roadmap

Per-core phase tracking for Nintendo Game Boy Advance. Mirrors the
project-wide ROADMAP shape (Phase 0 = onboarded, Phase 1 = first ROM
running, Phase 2 = polish, Phase 3+ = shared infra) but scoped to GBA.

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-19)

Core comes online via the libretro pivot — no Rust crate vendoring.
mGBA installed by operator; OA wires the system into the existing
shell, scanner, bindings, library DB, and settings pipelines.

- ✅ `oa_core::SystemId::Gba` variant added.
- ✅ `parse_system_id("gba" | "game-boy-advance" | "gameboyadvance")
  → SystemId::Gba` in `apps/oa-shell/src/main.rs`.
- ✅ `default_core_dll_for_system("gba") → "mgba_libretro.dll"`.
- ✅ Per-system button bits + bindings in `apps/oa-shell/src/bindings.rs::gba`
  — 10-button layout (4-way d-pad + A + B + L + R + START + SELECT),
  `GBA_BUTTONS` table, `default_gba_bindings()`, `defaults_for("gba")` arm.
- ✅ `gba_to_libretro_bits` identity remap.
- ✅ `bindings::to_libretro_bits` + `bit_for` + `buttons_for` dispatch
  arms include `"gba"`.
- ✅ `rom_hashes::libretro_dat_refs_for_system("gba")` returns
  `metadat/no-intro/Nintendo - Game Boy Advance`.
- ✅ `media::repo_for_system_id("gba")` returns
  `Some("Nintendo_-_Game_Boy_Advance")`.
- ✅ System registered in `frontend/src/themes/registry.ts` — `SystemId`
  union extended with `gba`, `systemThemes.gba` entry (extension
  `["gba"]`, portrait 3/4 tile aspect, `crt-lite` default shader preset
  per the handheld convention).
- ✅ Theme block in `frontend/src/themes/systems.css` — deep indigo
  (hue 285°, lightness 0.55, chroma 0.20). Sits between SNES (270°,
  L=0.62) and Lynx (290°, L=0.65) in hue but the lightness axis
  separates the three: GBA = darkest, SNES = mid, Lynx = brightest.
- ✅ Per-core docs scaffold (this directory).

**Acceptance gate:** Operator can drop `mgba_libretro.dll` into the
install, scan a GBA ROMs folder, see GBA-themed (deep indigo) tiles
appear in the library, and click one to launch — without rebuilding
Rust.

---

## ⬜ Phase 1 — First GBA ROM running

- ⬜ Operator validation: **Minish Cap**, **Pokémon FR/LG/Emerald**, **Metroid: Zero Mission**, **Advance Wars**, **Aria of Sorrow**, **FFTA**, **MK Super Circuit**, **Mother 3** — operator playtest.
- ✅ Save state F5/F8 round-trip — closed by cross-system save-state infra (`oa_libretro::LibretroCore::save_state / load_state`).
- ⬜ Battery-save persistence — operator playtest.
- ✅ Per-game cover sync via libretro-thumbnails — closed by cross-system media sync (`media::sync_media_for_system`).
- ✅ Libretro-database hash matching — closed by cross-system hash ID (`rom_hashes::resolve_rom_hashes_for_system`).
- ✅ BIOS-optional vs BIOS-required behavior — closed by `check_gba_bios` in `apps/oa-shell/src/main.rs` + warn-on-missing dispatch arm. Recognizes `gba_bios.bin` with libretro-database canonical SHA-1 (300C20DF6731A33952DED8C436F7F186D25D3492); does NOT block launch (mGBA HLE works for most titles); logs a warn pointing the operator at the canonical dump.

**Acceptance gate:** A reference set of GBA games run with pixels +
audio + working controller at native 59.73 Hz.

---

## ⬜ Phase 2 — Polish

- ✅ Dedicated `lcd-handheld` shader preset — defaulted 2026-05-24 for `gba` (in `frontend/src/themes/registry.ts::systemThemes.gba.defaultShaderPreset`).
- ✅ Per-system aspect override — GBA is 3:2 — shipped via `system_settings::default_display_aspect("gba") = Some(1.5)`.
- ✅ BIOS auto-detection / pre-launch check — closed by `check_gba_bios` (see Phase 1 entry above). Warn-only by design — mGBA HLE handles BIOS-less launches gracefully for the common case.
- ✅ Game-tilt sensor support (Kirby Tilt 'n' Tumble, Yoshi Topsy-Turvy, WarioWare Twisted!) — closed by Phase G sensor interface (`RETRO_ENVIRONMENT_GET_SENSOR_INTERFACE` wired through to a keyboard-arrow-keys-as-tilt fallback in `apps/oa-shell/src/main.rs`). Real OS-level accelerometer access is a separate later phase; the keyboard fallback makes these games playable today. Operator playtest pending.
- ✅ Solar-sensor support (Boktai 1/2/3) — closed by Phase G sensor interface. Today the illuminance channel reads 0 (mock); operator-driven core-options on mGBA cover the per-game light-level fixing pattern most operators use anyway. A real ambient-light source would need OS-level sensor access; Phase 1 ships the protocol so games can read sensor input without crashing.
- ✅ Rumble support — closed by Phase F rumble interface. mGBA's rumble extension feeds through `RETRO_ENVIRONMENT_GET_RUMBLE_INTERFACE` automatically. Operator playtest pending against Drill Dozer / Pokémon Pinball: Ruby & Sapphire.

---

## ⬜ Phase 3+ — Stretch

GBA-specific items:

- ⬜ Game Genie / Action Replay / CodeBreaker code support — operator-driven validation of mGBA's `retro_cheat_set`.
- ⬜ Game Link Cable multiplayer — deferred (out of scope for single-instance playback).
- ⬜ GBA Wireless Adapter — deferred.
- ⬜ Custom forked mGBA — deferred.

---

## Scope clarifications

- **Separate slug from `gb`.** Despite the family name, GBA hardware
  is a different generation (32-bit ARM7TDMI vs Sharp LR35902) and
  the libretro cores don't share. Keeping the slugs separate matches
  the libretro / RetroArch convention + lets per-system settings
  (input, shader, BIOS path) diverge cleanly.
- **GB/GBC backward compat is `gb`-slug routing.** The GBA console
  hardware could play .gb/.gbc carts via the slot's hardware
  compatibility mode, but in OA terms those games still go through
  the `gb` slug + Gambatte. Users wanting to play a .gb game "the
  GBA way" can use the per-game core override to swap in mGBA, but
  that's the unusual case.
- **`.bin` extension intentionally excluded** to avoid collision.
  Users with `.bin` GBA dumps rename to `.gba`.
- **No vendoring.** Buildbot mGBA .dll, treated as a black box.
