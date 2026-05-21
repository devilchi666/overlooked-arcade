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

- ⬜ Validate launch against the no-BIOS suggested set (Asteroids, Centipede, Ms. Pac-Man) — operator playtest.
- ⬜ Validate launch against a BIOS-recommended title (Choplifter / Robotron 2084) — operator playtest.
- ⬜ Probe PAL compatibility — drop a PAL .a78 dump and check the European BIOS lookup — operator playtest.
- ✅ Per-game ROM-set name resolution — closed by cross-system hash ROM identification (`apps/oa-shell/src/rom_hashes.rs::resolve_rom_hashes_for_system`).
- ⬜ POKEY audio sanity-check — Ballblazer / Commando / Centipede use POKEY for music — operator playtest.
- ✅ Per-game cover sync via libretro-thumbnails — closed by cross-system media sync (`apps/oa-shell/src/media.rs::sync_media_for_system`); `atari7800 → Atari_-_7800` shipped in `media::repo_for_system_id`.

---

## ⬜ Phase 2+ contributions

- ⬜ Twin-stick (Robotron 2084 two-joystick mode) — gated on shared analog/second-port infra.
- ⬜ Light gun games (XEGS Light Gun titles: Sentinel, Crossbow) — gated on shared analog infra.
- ⬜ Trak-Ball / driving-controller support (Pole Position II, Asteroids Deluxe) — gated on shared analog infra.
- ⬜ "High Score Cartridge" save support — operator-driven validation that ProSystem's SRAM region round-trips through the save-state path.
