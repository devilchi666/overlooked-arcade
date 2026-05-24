# ngp — Roadmap

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-20)

- ✅ `oa_core::SystemId::NeoGeoPocket` variant + parse_system_id arm
  (`ngp | ngpc | neopocket | neo-geo-pocket`).
- ✅ `bindings.rs::ngp` module — 7-button handheld pad
  (d-pad + A + B + OPTION). All dispatch arms wired.
- ✅ `default_core_dll_for_system("ngp") → "mednafen_ngp_libretro.dll"`.
- ✅ `rom_hashes` → two no-intro dats merged (NGP + NGPC, gb/wonderswan
  pattern).
- ✅ `media::repo_for_system_id` → `SNK_-_Neo_Geo_Pocket_Color`.
- ✅ Frontend SystemId / systemThemes / CSS (Plan A: pearl yellow-green
  105° / L=0.80 / C=0.12, open band, evokes NGPC translucent shell).
- ✅ No BIOS pre-check needed (BIOS-free).
- ✅ Per-core docs scaffold.

**Acceptance gate:** Operator drops `mednafen_ngp_libretro.dll`, scans
NGP/NGPC ROMs, sees pearl-yellow-green tiles, launches a known-good
ROM.

---

## ⬜ Phase 1 — First NGP/NGPC game running

- ⬜ Operator validation: **SNK vs Capcom: Card Fighter's Clash 1+2**, **Match of the Millennium**, **Sonic Pocket Adventure**, **KOF R-2**, **Magical Drop Pocket** — operator playtest.
- ⬜ Mono (.ngp) + color (.ngc) auto-detect validation — operator spot-check.
- ✅ Save state F5/F8 round-trip — closed by cross-system save-state infra (`oa_libretro::LibretroCore::save_state / load_state`).
- ✅ Cover sync via libretro-thumbnails — closed by cross-system media sync (`media::sync_media_for_system`).

---

## ⬜ Phase 2 — Polish

- ✅ `lcd-handheld` shader preset — defaulted 2026-05-24 for `ngp` (in `frontend/src/themes/registry.ts::systemThemes.ngp.defaultShaderPreset`).
- ✅ NGP-mono vs NGPC visual differentiation — tile shortName now reads "NGP" for `.ngp` files / "NGPC" for `.ngc` files (in `frontend/src/components/LibraryTile.tsx::subsystemLabel`). Defaulted 2026-05-24.

---

## Scope clarifications

- **NGP + NGPC single slug.** Beetle NeoPop handles both via header
  auto-detect, same pattern as gb (DMG+CGB) and wonderswan (WS+WSC).
- **No BIOS required.**
- **No analog input.** NGP/NGPC was strictly digital.
