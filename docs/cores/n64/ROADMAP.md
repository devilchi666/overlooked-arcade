# n64 — Roadmap

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-20)

- ✅ `oa_core::SystemId::N64` variant + `parse_system_id` arm
  (`n64 | nintendo-64 | nintendo64`).
- ✅ `bindings.rs::n64` module — 14-button digital (d-pad + A/B +
  L/R/Z + START + 4 C-buttons). All dispatch arms wired.
- ✅ `default_core_dll_for_system("n64") → "mupen64plus_next_libretro.dll"`.
- ✅ `rom_hashes` → no-intro Nintendo 64 dat (`.z64` keys directly;
  `.n64`/`.v64` need byte-swap pass — Phase 2).
- ✅ `media::repo_for_system_id` → `Nintendo_-_Nintendo_64`.
- ✅ Frontend SystemId / systemThemes / CSS (Plan A: Atomic Purple
  N64 at hue 268° L=0.55 C=0.22; slots into Nintendo home-console
  cluster between Intv 260° and SNES 270°).
- ✅ Per-core docs scaffold.
- ✅ **Analog input infra shipped** as part of this Phase 0 — gamepad
  LeftStick X/Y flows through `InputState.axes[0..2]` → libretro
  RETRO_DEVICE_ANALOG dispatch in oa-libretro → Mupen64Plus-Next reads
  the analog stick natively. Keyboard-only fallback via Mupen64Plus-Next's
  "Map d-pad to analog stick" core option.

**Acceptance gate:** Operator drops `mupen64plus_next_libretro.dll`,
scans N64 ROMs, sees Atomic Purple tiles, launches a known-good ROM
with a connected gamepad's analog stick driving Mario / Link.

---

## ⬜ Phase 1 — First N64 game running

- ⬜ Operator validation: SM64 / GoldenEye / Ocarina of Time / MK64 / Smash 64 — operator playtest.
- ✅ Analog stick smoke-test — closed by cross-system analog axes (`InputState.axes` + `compute_stick_output` with keyboard fallback + deadzone + sensitivity).
- ✅ Save state F5/F8 round-trip — closed by cross-system save-state infra (`oa_libretro::LibretroCore::save_state / load_state`).
- ⬜ Multi-region testing (NTSC US + NTSC JP + PAL EU) — operator playtest.

---

## ✅ Phase 2 — Polish

- ✅ **Byte-swap pass** for `.n64` + `.v64` dumps — shipped via `apps/oa-shell/src/rom_header.rs::HeaderRule::ByteSwap` (normalizes to `.z64` BE sha1 candidate before lookup).
- ⬜ **Analog stick deadzone + sensitivity** per-system Core Options surface — operator-driven Core Options curation (per-system Core Options page shipped cross-system).
- ✅ **Per-axis keyboard binding** — shipped via `system_settings::default_analog_routing("n64")` (WASD → analog stick default).

---

## ⬜ Phase 3+ — Stretch

- ⬜ **N64 Memory Pak / Rumble Pak** UX — operator-driven Mupen64Plus-Next core-option curation.
- ⬜ **Transfer Pak** (GB carts via N64 — Pokémon Stadium 1/2) — deferred (niche).
