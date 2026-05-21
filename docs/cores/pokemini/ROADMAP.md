# pokemini Roadmap

Per-core phase tracking for the Nintendo Pokémon Mini.

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-20)

- ✅ `oa_core::SystemId::PokeMini` variant + `parse_system_id` arm.
- ✅ `apps/oa-shell/src/bindings.rs` — `pokemini` button module (7 buttons: d-pad + A + B + C), `POKEMINI_BUTTONS` table, dispatch arms in `bit_for` / `buttons_for` / `to_libretro_bits` / `defaults_for`.
- ✅ `default_core_dll_for_system("pokemini") → "pokemini_libretro.dll"`.
- ✅ `media::repo_for_system_id("pokemini") → "Nintendo_-_Pokemon_Mini"`.
- ✅ `rom_hashes::libretro_dat_refs_for_system("pokemini")` — points at the no-intro Pokemon Mini dat.
- ✅ `check_pokemini_bios` wired into the cart-shape BIOS dispatch arm (was pre-staged from the BIOS audit session; activated this session).
- ✅ Frontend `SystemId` union extended, `systemThemes` entry added, CSS theme block (sunny yellow `oklch(0.85 0.16 95)` — brightest tile in the lineup, appropriate for the tiniest, friendliest Nintendo platform).

**Acceptance gate:** Phase 1 operator validation pending — drop `pokemini_libretro.dll` + `bios.min` into the install, scan a `.min` library, launch a representative game.

---

## ⬜ Phase 1 — Operator validation

- ⬜ Drop `pokemini_libretro.dll` (from buildbot) into `<exe_dir>/cores/` — operator-driven.
- ⬜ Drop `bios.min` (SHA-1 `DAAD4113713ED776FBD47727762BCA81BA74915F`) into `<exe_dir>/system/` — operator-driven.
- ⬜ Mark a `.min` folder via the Import Wizard — operator-driven.
- ⬜ Launch (operator playtest):
  - Pokémon Pinball Mini — flagship pack-in (uses shake sensor; shipping fine without it on Phase 0)
  - Pokémon Party Mini — minigame collection
  - Pichu Bros. Mini — first-party puzzle/minigame

---

## ⬜ Phase 2 — Hardening

- ⬜ Shake sensor mapping via gamepad rumble / dedicated key — gated on rumble/motion infra (most games playable without).
- ⬜ Per-game core options templates (LCD ghosting, color overlays, etc.) — operator-driven curation.
