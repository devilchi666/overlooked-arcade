# channelf — Roadmap

Per-core phase tracking for Fairchild Channel F. Status: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-19)

- ✅ `oa_core::SystemId::ChannelF` variant.
- ✅ `parse_system_id("channelf" | "channel-f" | "fairchild") → ChannelF`.
- ✅ `default_core_dll_for_system("channelf") → "freechaf_libretro.dll"`.
- ✅ `bindings.rs::channelf` — 9-button layout (D-pad 4-axis plunger + FIRE + MODE + TIME + START + HOLD), identity remap, dispatch.
- ✅ `media::repo_for_system_id("channelf") → "Fairchild_-_Channel_F"`.
- ✅ `rom_hashes::libretro_dat_refs_for_system("channelf") → metadat/no-intro/Fairchild - Channel F`.
- ✅ Frontend `systemThemes.channelf` (extension `["chf"]`, portrait 3/4, crt-lite).
- ✅ Theme CSS — cedar-brown hue 25° / L=0.45 / C=0.06 (sibling to 2600's yellow-pine 60° / L=0.60 / C=0.07).
- ✅ Per-core docs scaffold.

---

## ⬜ Phase 1 — First Channel F ROM running

- ⬜ Operator validation: launch a `.chf` ROM. Suggested: **Video Whizball**, **Spitfire**, **Dodge It**, **Memory Match**, **Tic-Tac-Toe** (the pack-in) — operator playtest.
- ⬜ Plunger controller mapping — operator playtest of D-pad bindings against pull/push/twist axes.
- ⬜ Console-switch bindings — MODE / TIME / START / HOLD on M / T / Enter / H — operator playtest.
- ⬜ Optional BIOS install (`sl31253.bin` + `sl31254.bin` + `sl90025.bin`) — operator-driven.
- ✅ libretro-database hash matching + cover sync — closed by cross-system hash ID (`rom_hashes::resolve_rom_hashes_for_system`) + media sync (`media::sync_media_for_system`).

---

## ⬜ Phase 2 — Polish

- ⬜ True multi-axis plunger analog mapping — the Channel F plunger was a real 3-axis stick (push/pull, twist L/R, push-in) — gated on shared analog-input infra.
- ⬜ Channel F System II (1979 redesign without wood-grain) — operator-driven cosmetic ROM-set distinction.
- ⬜ Homebrew scene catalog — operator-driven data curation.

---

## ⬜ Phase 3+ — Stretch

- ⬜ Custom BIOS rebuild — there are open-source Channel F BIOS clones that the operator could ship if licensing requires — deferred.
