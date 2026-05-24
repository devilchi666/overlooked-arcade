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

- ⬜ Operator validation: launch a `.o2` ROM. Suggested: **KC Munchkin**, **Pick Axe Pete**, **Atlantis**, **Smithereens!** — operator playtest.
- ✅ BIOS pre-check workflow — closed by `check_o2_bios` in `apps/oa-shell/src/main.rs` + dispatch arm for `o2` system_id. Recognizes `o2rom.bin` / `c52.bin` / `g7400.bin` / `jopac.bin` with libretro-database canonical SHA-1s; blocks launch + toasts when missing.
- ✅ Keyboard passthrough validation — closed by cross-system keyboard passthrough + Game-Focus + Ctrl+G.
- ⬜ Region auto-detect — operator spot-check (NTSC Odyssey² vs PAL Videopac).
- ✅ Cover sync + libretro-database hashing — closed by cross-system media sync (`media::sync_media_for_system`) + hash ID (`rom_hashes::resolve_rom_hashes_for_system`).

---

## ⬜ Phase 2 — Polish

- ⬜ Per-game keyboard layout overlay — **partial**: per-game `keypad_layout_note` field shipped in `GameOverrides`; full overlay-image UI surface still ⬜.
- ⬜ Videopac+ G7400 expansion — operator-driven O2EM core-option curation.
- ⬜ The Voice (Magnavox's speech-synthesis module) — deferred (niche).

---

## ⬜ Phase 3+ — Stretch

- ⬜ Cheat support — operator-driven validation that O2EM exposes `retro_cheat_set` usefully.
