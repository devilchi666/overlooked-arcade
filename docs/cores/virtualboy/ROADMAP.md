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

- ⬜ Operator validation: launch `.vb` ROMs. Single-D-pad-friendly: **Mario's Tennis** (the pack-in), **V-Tetris**, **Wario Cruise** (Japan), **Jack Bros**, **Galactic Pinball**, **Virtual Boy Wario Land**.
- ⬜ 3D mode validation: try anaglyph (default), confirm red/cyan glasses produce the depth effect. Then try side-by-side mode if the operator has VR hardware.
- ⬜ Save state F5/F8 round-trip — Beetle VB supports `retro_serialize`.
- ⬜ Cover sync + libretro-database hashing.

---

## ⬜ Phase 2 — Polish

- ⬜ Right D-pad bindings — currently deferred (see DECISIONS). Two paths: bind to libretro L2/R2/L3/R3 + Beetle VB core option, OR wait for shared analog-input infra. Either way unlocks: **Mario Clash** (jump-platforming with right D-pad for jump), **Virtual Boy Wario Land** (right D-pad for special moves), **Teleroboxer** (left/right punches independently), **Red Alarm**, **Vertical Force**.
- ⬜ Modern VR support via OpenXR — render the side-by-side dual-perspective directly to a VR headset rather than anaglyph fallback. Significant engineering work; far-out polish.
- ⬜ Color-tinting palette options — Beetle VB has core options to tint the red LEDs with different palettes (red-on-black is canonical, but green-on-black / amber-on-black appeal to some operators). Surface in per-system Core Options page.
- ⬜ Dedicated `vb-monochrome` shader — the OA `plain` default is correct for VB, but a custom shader could add subtle LED-grain noise + the 1995-era curve of the visor's reflection.

---

## ⬜ Phase 3+ — Stretch

- ⬜ Cheat support.
- ⬜ Eyestrain / break-reminder warning — the real Virtual Boy famously caused headaches after extended play; a polite reminder UI could be quaint period homage.

---

## 2026-05-21 — Stale-cleanup audit

The Phase 1+ items above were written when this system onboarded, before cross-system infrastructure (Phases 1.5 / 2.5–2.8 / 3 / 4 + direct-launch CLI) landed. Many `⬜` items are actually shipped — see `docs/cores/AUDIT_2026-05-21.md` for the per-item breakdown (stale vs open-code vs open-operator) for this system.
