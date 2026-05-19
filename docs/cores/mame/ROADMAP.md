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

## ⬜ Phase 1.5 — Hardening (post-Phase-1)

- ⬜ Validate a known-good ROM set against MAME 0.287 (the buildbot version operator installed). Suggested test sets: `pacman.zip` (Namco), `sf2ce.zip` (Capcom CPS1), `mslug.zip` (Neo Geo via MAME — needs `neogeo.zip` BIOS alongside).
- ⬜ Verify the 6-button SF mapping feels right on a real cabinet stick / fight pad; consider shipping an "SF-native" alternate default that puts B1-B3 on the top row.
- ⬜ Multi-player input — second-port wiring through libretro's per-port controller assignment. MAME games with simultaneous P2 (Final Fight, X-Men, etc.) need it.
- ⬜ Per-game ROM-set name resolution. Today the library tile shows the .zip filename; surfacing the human title (`Street Fighter II: Champion Edition`) requires a MAME-style metadata DB lookup — defer until per-game metadata sync work.
- ⬜ `.chd` arcade games — exercise against a known-good set (Killer Instinct, Atari System 2).
- ⬜ Verify aspect-ratio handling — many arcade boards run native rotation (Pac-Man = vertical 224×288); the renderer needs to read the libretro rotation flag.

---

## ⬜ Phase 2+ contributions

- ⬜ MAME-specific button glyphs for the bindings UI (LP/MP/HP/LK/MK/HK overlay icons).
- ⬜ Per-game ROM-set metadata (year, manufacturer, hardware) via MAME's listxml output or libretro thumbnails.
- ⬜ Multi-game-per-zip handling — some MAME sets are bundles (clones, alternate versions); decide whether to surface clones in the library or hide them.
