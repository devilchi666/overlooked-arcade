# virtualboy — Roadmap

Per-core phase tracking for Nintendo Virtual Boy. Status: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-20)

- ✅ `oa_core::SystemId::VirtualBoy` (already existed).
- ✅ `parse_system_id("virtualboy" | "virtual-boy") → VirtualBoy` (already wired).
- ✅ `default_core_dll_for_system("virtualboy") → "mednafen_vb_libretro.dll"`.
- ✅ `bindings.rs::virtualboy` — 10-button layout (LEFT D-pad + A + B + L + R + START + SELECT), identity remap, dispatch.
- ✅ `media::repo_for_system_id("virtualboy") → "Nintendo_-_Virtual_Boy"` (already wired).
- ✅ `rom_hashes::libretro_dat_refs_for_system("virtualboy") → metadat/no-intro/Nintendo - Virtual Boy`.
- ✅ Frontend `systemThemes.virtualboy` (extension `["vb"]`, landscape 4/3, **plain** shader — see DECISIONS for why crt-lite was rejected for VB specifically).
- ✅ Theme CSS — deep VB red hue 7° / L=0.55 / C=0.26.
- ✅ Per-core docs scaffold.

---

## ⬜ Phase 1 — First VB ROM running

- ⬜ Operator validation: **Mario's Tennis** (pack-in), **V-Tetris**, **Wario Cruise** (Japan), **Jack Bros**, **Galactic Pinball**, **Virtual Boy Wario Land** — operator playtest.
- ⬜ 3D mode validation — operator playtest (anaglyph + side-by-side).
- ✅ Save state F5/F8 round-trip — closed by cross-system save-state infra (`oa_libretro::LibretroCore::save_state / load_state`).
- ✅ Cover sync + libretro-database hashing — closed by cross-system media sync (`media::sync_media_for_system`) + hash ID (`rom_hashes::resolve_rom_hashes_for_system`).

---

## ⬜ Phase 2 — Polish

- ✅ Right D-pad bindings — shipped 2026-05-24 via the shared analog routing infra. `analog_sticks_for("virtualboy") = Dual { left_label: "Left D-pad", right_label: "Right D-pad" }` surfaces both panels in the per-system Bindings UI; `default_analog_routing("virtualboy")` adds the Numpad 8/2/4/6 keyboard fallback for the right pad. Gamepad operators get it via the right analog stick out of the box. Unlocks Mario Clash, VB Wario Land, Teleroboxer, Red Alarm, Vertical Force.
- ⬜ Modern VR support via OpenXR — deferred (Phase 2+, ~800 lines).
- ⬜ Color-tinting palette options — operator-driven Beetle VB Core-Option curation (per-system Core Options page shipped cross-system).
- ⬜ Dedicated `vb-monochrome` shader — not yet shipped (~120 lines WGSL).

---

## ⬜ Phase 3+ — Stretch

- ⬜ Cheat support — operator-driven validation.
- ⬜ Eyestrain / break-reminder warning — operator-driven UI polish.
