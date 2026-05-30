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

- ⬜ Operator validation: launch `.vec` ROMs. Suggested: **Mine Storm**, **Berzerk**, **Star Trek: The Motion Picture**, **Spike**, **Bedlam**, **Pole Position**, **Solar Quest**, **Scramble**, **Web Wars** — operator playtest.
- ⬜ Optional BIOS install (`vectrex.bin`) — operator-driven.
- ✅ Save state F5/F8 round-trip — closed by cross-system save-state infra (`oa_libretro::LibretroCore::save_state / load_state`).
- ✅ Cover sync + libretro-database hashing — closed by cross-system media sync (`media::sync_media_for_system`) + hash ID (`rom_hashes::resolve_rom_hashes_for_system`).

---

## ⬜ Phase 2 — Polish

- ✅ Dedicated `vector-phosphor` shader preset — Wider-σ Gaussian bloom on bright vector strokes (9-tap σ≈2.5 with luminance bright-pass) + persistent history accumulator at ~80ms half-life (`history_curr = current + history_prev * 0.866`). Shipped 2026-05-29 on `feat/vectrex-vector-phosphor-shader`; new files: `crates/oa-render/shaders/vector_blur.wgsl`, `crates/oa-render/shaders/persistence.wgsl`, `shaders/presets/vector-phosphor.preset.toml`. Per-system default flipped from `crt-lite` → `vector-phosphor` in `themes/registry.ts`. Operators tune the halo via the existing Settings → Display bloom slider (bloom_amount). 506 oa-shell tests green.
- ⬜ Translucent overlay rendering — per-game PNG overlay composited over the framebuffer to recreate the plastic-color-strip feature. Not yet shipped.
- ✅ Aspect ratio override — Vectrex CRT was portrait (3:4); defaulted 2026-05-24 via `system_settings::default_display_aspect("vectrex") = Some(0.75)`. Operators with a landscape monitor configuration toggle per-system via Display.
- ⬜ The Vectrex 3D Imager — deferred (niche).

---

## ⬜ Phase 3+ — Stretch

- ⬜ Cheat support — operator-driven validation that vecx exposes useful `retro_cheat_set`.
- ⬜ Custom-built vector renderer at OA-engine level — not yet shipped (Phase 3+, ~500 lines).
