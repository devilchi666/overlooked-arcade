# wonderswan — Roadmap

Per-core phase tracking for Bandai WonderSwan + WonderSwan Color. Status: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-20)

- ✅ `oa_core::SystemId::WonderSwan` (already existed).
- ✅ `parse_system_id("wonderswan") → WonderSwan` (already wired).
- ✅ `default_core_dll_for_system("wonderswan") → "mednafen_wswan_libretro.dll"`.
- ✅ `bindings.rs::wonderswan` — 7-button layout (D-pad + A + B + START; Beetle WS handles dual-physical-D-pad rotation per game header), identity remap, dispatch.
- ✅ `media::repo_for_system_id("wonderswan") → "Bandai_-_WonderSwan"` (already wired; WS Color repo deferred — see DECISIONS).
- ✅ `rom_hashes::libretro_dat_refs_for_system("wonderswan") → metadat/no-intro/{Bandai - WonderSwan, Bandai - WonderSwan Color}` — TWO DatRefs merged into one corpus.
- ✅ Frontend `systemThemes.wonderswan` (extensions `["ws", "wsc"]`, portrait 3/4, crt-lite).
- ✅ Theme CSS — pearl lavender 305° / L=0.70 / C=0.14.
- ✅ Per-core docs scaffold.

---

## ⬜ Phase 1 — First WS ROM running

- ⬜ Operator validation (mono + color) — operator playtest.
- ⬜ Mono vs Color auto-detect — operator spot-check.
- ⬜ Vertical-rotation auto-handling — operator playtest.
- ✅ Save state F5/F8 round-trip — closed by cross-system save-state infra (`oa_libretro::LibretroCore::save_state / load_state`).
- ⬜ Optional BIOS install — operator-driven.
- ✅ Cover sync (WS + WSC) — closed by cross-system multi-repo cover sync (`media::repos_for_system_id` returning a slice).
- ✅ libretro-database hashing — closed by cross-system hash ID (`rom_hashes::resolve_rom_hashes_for_system`).

---

## ⬜ Phase 2 — Polish

- ✅ Multi-repo cover sync — shipped via `apps/oa-shell/src/media.rs::repos_for_system_id` returning a slice (WS + WSC).
- ⬜ Per-game framebuffer rotation override — operator-driven preference.
- ⬜ Sound-volume button binding — operator-driven binding decision.
- ⬜ Cable Link multiplayer — deferred (no current libretro support).
- ⬜ SwanCrystal screen-improvement modeling — operator-driven per-game shader option.

---

## ⬜ Phase 3+ — Stretch

- ⬜ Cheat support — operator-driven validation.
- ⬜ Pocket Challenge V2 / Pocket Challenge V1 — operator-driven validation of Beetle WS support.
