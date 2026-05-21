# atari7800 Roadmap

Per-core phase tracking for Atari 7800. Mirrors `docs/ROADMAP.md` but only the atari7800 slice.

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## 🟨 Phase 1 — Onboarding (2026-05-19)

- ✅ `atari7800` slug added to `frontend/src/themes/registry.ts` (ext: `.a78`; theme block in `themes/systems.css` — amber/gold at hue 80°).
- ✅ `oa_core::SystemId::Atari7800` variant + `parse_system_id("atari7800")` already in place from earlier scaffolding.
- ✅ `apps/oa-shell/src/bindings.rs` — `atari7800` button module (B1/B2 + d-pad + PAUSE + SELECT, identity-mapped to libretro RetroPad), `ATARI7800_BUTTONS`, `default_atari7800_bindings()`, dispatch arms in `bit_for` / `buttons_for` / `to_libretro_bits` / `defaults_for`. 17 bindings tests green (3 new for atari7800 + existing cross-system checks updated).
- ✅ `default_core_dll_for_system("atari7800") → "prosystem_libretro.dll"` in `apps/oa-shell/src/main.rs`.
- ✅ Catalog entry `prosystem_libretro` was already present in `core_installer::CATALOG` from prior scaffolding; recommended=true, bios optional.

**Acceptance gate:** A real .a78 ROM (Asteroids / Centipede / Ms. Pac-Man) launches, plays, and accepts input. Pending operator validation.

---

## ⬜ Phase 1.5 — Hardening (post-Phase-1)

- ⬜ Validate launch against the no-BIOS suggested set (Asteroids, Centipede, Ms. Pac-Man) — confirms the cart path works without a BIOS dependency.
- ⬜ Validate launch against a BIOS-recommended title (Choplifter / Robotron 2084) — confirms BIOS lookup wires through to `<exe_dir>/system/` correctly.
- ⬜ Probe PAL compatibility — drop a PAL .a78 dump and check the European BIOS lookup. Some core versions also accept `7800 BIOS (E).rom` alongside the US ROM.
- ⬜ Per-game ROM-set name resolution. Today's library tile shows the filename; per-game metadata sync would surface the human title (e.g. `Ms. Pac-Man (USA) (Proto)` → `Ms. Pac-Man`).
- ⬜ POKEY audio sanity-check — Ballblazer / Commando / Centipede use POKEY for music; cores sometimes drop POKEY output on the buildbot path. If output is silent, the recommended-BIOS path is implicated.
- ⬜ Per-game cover sync via libretro-thumbnails — **infra ready 2026-05-19, needs operator validation.** Mapping `atari7800 → Atari_-_7800` shipped in `apps/oa-shell/src/media.rs::repo_for_system_id`. Operator: run `Settings → Library → Sync media for Atari 7800` and confirm covers download.

---

## ⬜ Phase 2+ contributions

- ⬜ Twin-stick (Robotron 2084 two-joystick mode) — needs the shell's second-port input wiring, which the broader Phase 6 cross-system port-handling pass will pick up.
- ⬜ Light gun games (XEGS Light Gun titles: Sentinel, Crossbow) — needs analog-input infrastructure from the Phase 6 Phase 3 deferred-until-forced analog work.
- ⬜ Trak-Ball / driving-controller support (Pole Position II, Asteroids Deluxe) — similar analog dependency.
- ⬜ "High Score Cartridge" save support — the original 7800 HSC stored leaderboards in a separate cart's RAM; the ProSystem core emulates this via a small SRAM region. Verify the save state path captures it.
