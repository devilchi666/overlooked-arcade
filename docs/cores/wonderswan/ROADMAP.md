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

- ⬜ Operator validation: launch `.ws` and `.wsc` ROMs. Suggested mono: **Klonoa: Moonlight Museum**, **GunPey**, **Rockman + Forte**, **Frontier Story**. Suggested Color: **Final Fantasy I**, **Final Fantasy II**, **Riviera: The Promised Land** (prototype), **Hunter X Hunter: Greed Adventure**.
- ⬜ Mono vs Color auto-detect — Beetle WS reads the ROM header; confirm `.ws` files render mono and `.wsc` files render color without operator intervention.
- ⬜ Vertical-rotation auto-handling — load a vertical-mode title (Riviera or any GunPey-family) and confirm Beetle WS rotates the framebuffer + active D-pad swap works correctly.
- ⬜ Save state F5/F8 round-trip.
- ⬜ Optional BIOS install — confirm WS boot splash + name-entry screen appears.
- ⬜ Cover sync — confirm primary `Bandai_-_WonderSwan` repo works; WSC-specific gaps documented for Phase 2.
- ⬜ libretro-database hashing — confirm merged WS + WS Color corpus matches both extensions.

---

## ⬜ Phase 2 — Polish

- ⬜ Multi-repo cover sync — extend `repo_for_system_id` (or add a parallel function) to allow returning multiple thumbnails repos per slug. Unlocks WSC-specific covers AND solves the same gap for `gb` (Game Boy + Game Boy Color).
- ⬜ Per-game framebuffer rotation override — operator preference for handling vertical-mode games (auto-rotate framebuffer vs keep monitor orientation fixed + render rotated).
- ⬜ Sound-volume button binding — the hardware Sound button currently lives in Beetle WS core options; could surface as a per-system input if anyone wants it.
- ⬜ Cable Link multiplayer — niche; no current libretro support.
- ⬜ SwanCrystal screen-improvement modeling — the 2002 SwanCrystal had a sharper LCD vs the original WS; per-game shader option could simulate the differences.

---

## ⬜ Phase 3+ — Stretch

- ⬜ Cheat support.
- ⬜ Pocket Challenge V2 / Pocket Challenge V1 — Bandai's earlier mono handhelds that share architectural lineage with the WS; uncertain whether Beetle WS handles them.
