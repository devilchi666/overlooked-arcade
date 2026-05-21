# o2 — Roadmap

Per-core phase tracking for Magnavox Odyssey² / Videopac. Status: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-19)

- ✅ `oa_core::SystemId::Odyssey2` variant.
- ✅ `parse_system_id("o2" | "odyssey2" | "odyssey-2" | "videopac") → Odyssey2`.
- ✅ `default_core_dll_for_system("o2") → "o2em_libretro.dll"`.
- ✅ `bindings.rs::o2` — 5-button layout (D-pad + ACTION), identity remap, dispatch.
- ✅ `media::repo_for_system_id("o2") → "Magnavox_-_Odyssey2"`.
- ✅ `rom_hashes::libretro_dat_refs_for_system("o2") → metadat/no-intro/Magnavox - Odyssey2`.
- ✅ Frontend `systemThemes.o2` (extension `["o2"]`, portrait 3/4, crt-lite).
- ✅ Theme CSS — rose-fuchsia hue 325° / L=0.62 / C=0.18.
- ✅ Per-core docs scaffold.

---

## ⬜ Phase 1 — First O2 ROM running

- ⬜ Operator validation: launch a `.o2` ROM. Suggested: **KC Munchkin** (the Pac-Man clone that prompted Atari's lawsuit), **Pick Axe Pete**, **Atlantis**, **Smithereens!**.
- ⬜ BIOS pre-check workflow — `o2rom.bin` / `c52.bin`.
- ⬜ Keyboard passthrough validation — launch a keyboard-required title (Quest for the Rings) and confirm OA's keyboard passthrough delivers letter keys to O2EM cleanly.
- ⬜ Region auto-detect — NTSC Odyssey² vs PAL Videopac.
- ⬜ Cover sync + libretro-database hashing.

---

## ⬜ Phase 2 — Polish

- ⬜ Per-game keyboard layout overlay — Quest for the Rings + the Master Strategy Series games used printed keyboard overlays to remap keys per-game; surface a per-game "overlay" image in the OA UI.
- ⬜ Videopac+ G7400 expansion — O2EM supports it via core option.
- ⬜ The Voice (Magnavox's speech-synthesis module) — niche but historically interesting.

---

## ⬜ Phase 3+ — Stretch

- ⬜ Cheat support — uncertain whether O2EM exposes `retro_cheat_set` usefully for this hardware.

---

## 2026-05-21 — Stale-cleanup audit

The Phase 1+ items above were written when this system onboarded, before cross-system infrastructure (Phases 1.5 / 2.5–2.8 / 3 / 4 + direct-launch CLI) landed. Many `⬜` items are actually shipped — see `docs/cores/AUDIT_2026-05-21.md` for the per-item breakdown (stale vs open-code vs open-operator) for this system.
