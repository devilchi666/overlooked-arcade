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

- ⬜ Operator validation: Super Mario 64 (analog stick test), GoldenEye
  (C-buttons for camera), Ocarina of Time, Mario Kart 64, Smash Bros 64.
- ⬜ Analog stick smoke-test — gamepad LeftStick should drive Mario's
  full movement range; deadzone at ~10% to filter stick drift.
- ⬜ Save state F5/F8 round-trip.
- ⬜ Multi-region testing (NTSC US + NTSC JP + PAL EU).

---

## ⬜ Phase 2 — Polish

- ⬜ **Byte-swap pass** for `.n64` + `.v64` dumps to enable
  libretro-database hash matching. `rom_header.rs` extension that
  detects byte order and normalizes to `.z64` Big-Endian sha1
  candidate before lookup.
- ⬜ **Analog stick deadzone + sensitivity** per-system Core Options
  surface — Mupen64Plus-Next exposes these.
- ⬜ **Per-axis keyboard binding** — Phase 2.5 polish would let
  keyboard-only users bind WASD to analog stick directions (instead
  of using the core's d-pad-to-analog hack).

---

## ⬜ Phase 3+ — Stretch

- ⬜ **N64 Memory Pak / Rumble Pak** UX. Mupen64Plus-Next handles
  these via core options; per-game surface needed.
- ⬜ **Transfer Pak** (GB carts via N64 — Pokémon Stadium 1/2). Niche.

---

## 2026-05-21 — Stale-cleanup audit

The Phase 1+ items above were written when this system onboarded, before cross-system infrastructure (Phases 1.5 / 2.5–2.8 / 3 / 4 + direct-launch CLI) landed. Many `⬜` items are actually shipped — see `docs/cores/AUDIT_2026-05-21.md` for the per-item breakdown (stale vs open-code vs open-operator) for this system.
