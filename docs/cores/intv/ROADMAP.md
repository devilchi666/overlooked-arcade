# intv — Roadmap

Per-core phase tracking for Mattel Intellivision. Status: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-19)

- ✅ `oa_core::SystemId::Intellivision` variant.
- ✅ `parse_system_id("intv" | "intellivision") → Intellivision`.
- ✅ `default_core_dll_for_system("intv") → "freeintv_libretro.dll"`.
- ✅ `bindings.rs::intv` — 10-button layout, identity remap, dispatch.
- ✅ `media::repo_for_system_id("intv") → "Mattel_-_Intellivision"`.
- ✅ `rom_hashes::libretro_dat_refs_for_system("intv") → metadat/no-intro/Mattel - Intellivision`.
- ✅ Frontend `systemThemes.intv` (extension `["int"]`, portrait 3/4, crt-lite).
- ✅ Theme CSS — deep Mattel navy hue 260° / L=0.50 / C=0.17.
- ✅ Per-core docs scaffold.

**Acceptance gate:** Operator drops `freeintv_libretro.dll` + `exec.bin` + `grom.bin` BIOS files, scans Intv folder, sees navy-themed tiles, launches a game.

---

## ⬜ Phase 1 — First Intv ROM running

- ⬜ Operator validation: launch `.int` ROMs. Suggested: **Astrosmash**, **Utopia**, **Snafu**, **Star Strike**, **Major League Baseball**, **B-17 Bomber** (uses Intellivoice — Phase 2 polish).
- ⬜ BIOS pre-check workflow — confirm clear error when `exec.bin` or `grom.bin` missing.
- ⬜ 4 side-button bindings — confirm UPPER vs LOWER mapping reads correctly per-game (most games use lower buttons for primary action).
- ⬜ Save state F5/F8 round-trip.
- ⬜ libretro-database hash matching + cover sync.

---

## ⬜ Phase 2 — Polish

- ⬜ Full 12-button keypad coverage — same shape as Coleco's keypad. Keypad numbers spread across the remaining libretro RetroPad bits (Y/X/L2/R2/L3/R3 + the unused face buttons).
- ⬜ 16-direction disc analog input — shared analog-input infrastructure dependency. Until that lands, D-pad-as-8-way is the only option.
- ⬜ Intellivoice voice-synthesis module support — FreeIntv's voice-module emulation is core-side; needs operator validation that B-17 Bomber + Bomb Squad + Space Spartans speak correctly out of the box.
- ⬜ Intellivision Computer Module / ECS expansion — niche; deferred.

---

## ⬜ Phase 3+ — Stretch

- ⬜ Cheat support via libretro cheat path.
- ⬜ INTV Music Synthesizer (the ECS music synthesizer peripheral). Defer.

---

## 2026-05-21 — Stale-cleanup audit

The Phase 1+ items above were written when this system onboarded, before cross-system infrastructure (Phases 1.5 / 2.5–2.8 / 3 / 4 + direct-launch CLI) landed. Many `⬜` items are actually shipped — see `docs/cores/AUDIT_2026-05-21.md` for the per-item breakdown (stale vs open-code vs open-operator) for this system.
