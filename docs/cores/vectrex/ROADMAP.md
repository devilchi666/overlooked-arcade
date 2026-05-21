# vectrex — Roadmap

Per-core phase tracking for GCE Vectrex. Status: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-20)

- ✅ `oa_core::SystemId::Vectrex` (already existed from Phase 0 placeholder).
- ✅ `parse_system_id("vectrex") → Vectrex` (already wired).
- ✅ `default_core_dll_for_system("vectrex") → "vecx_libretro.dll"`.
- ✅ `bindings.rs::vectrex` — 8-button layout (D-pad + B1/B2/B3/B4), identity remap, dispatch.
- ✅ `media::repo_for_system_id("vectrex") → "GCE_-_Vectrex"` (already wired).
- ✅ `rom_hashes::libretro_dat_refs_for_system("vectrex") → metadat/no-intro/GCE - Vectrex`.
- ✅ Frontend `systemThemes.vectrex` (extensions `["vec", "gam"]`, landscape 4/3, crt-lite).
- ✅ Theme CSS — bright phosphor-green 165° / L=0.80 / C=0.16.
- ✅ Per-core docs scaffold.

---

## ⬜ Phase 1 — First Vectrex ROM running

- ⬜ Operator validation: launch `.vec` ROMs. Suggested: **Mine Storm** (the built-in pack-in — needs BIOS), **Berzerk**, **Star Trek: The Motion Picture**, **Spike**, **Bedlam**, **Pole Position**, **Solar Quest**, **Scramble**, **Web Wars**.
- ⬜ Optional BIOS install (`vectrex.bin`) — confirm Mine Storm launches via the BIOS pack-in path.
- ⬜ Save state F5/F8 round-trip.
- ⬜ Cover sync + libretro-database hashing.

---

## ⬜ Phase 2 — Polish

- ⬜ Dedicated `vector-phosphor` shader preset — Gaussian glow on vector lines, no scanlines, optional persistence trail to mimic the CRT phosphor afterimage.
- ⬜ Translucent overlay rendering — per-game PNG overlay composited over the framebuffer to recreate the plastic-color-strip feature.
- ⬜ Aspect ratio override — Vectrex CRT was portrait (3:4 ish); per-system aspect tweak.
- ⬜ The Vectrex 3D Imager — period-correct stereoscopic accessory; niche, deferred.

---

## ⬜ Phase 3+ — Stretch

- ⬜ Cheat support (uncertain if vecx exposes useful `retro_cheat_set`).
- ⬜ Custom-built vector renderer at OA-engine level — eventually replacing vecx's raster output with native vector-stroke rendering on the OA wgpu pipeline.
