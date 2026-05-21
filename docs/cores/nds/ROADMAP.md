# nds — Roadmap

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-20)

- ✅ `oa_core::SystemId::Nds` variant + parse_system_id arm
  (`nds | ds | nintendo-ds`).
- ✅ `bindings.rs::nds` module — 12 digital buttons (Nintendo
  diamond: A east primary, B south secondary, X north, Y west; +
  L/R + START + SELECT).
- ✅ `default_core_dll_for_system("nds") → "melonds_libretro.dll"`.
- ✅ `rom_hashes` → no-intro NDS dat (.nds single-file).
- ✅ `media::repo_for_system_id` → `Nintendo_-_Nintendo_DS`.
- ✅ **NEW POINTER input infra shipped** — `oa_core::InputState`
  extended with `pointer: (i16, i16, bool)`; `oa-libretro` adds
  RETRO_DEVICE_POINTER constants + state field + cb_input_state
  dispatch; `oa-input::InputPoller` polls device_query mouse position
  + left-button state. End-to-end mouse-as-touch flow.
- ✅ `check_nds_bios` (multi-file: bios7 + bios9 + firmware all
  required). Cart-shape pre-check arm next to neogeo.
- ✅ Frontend SystemId / systemThemes / CSS (Plan A: pearl yellow-
  green `oklch(0.78 0.14 95)` — Nintendo handheld pearl pattern).
- ✅ Per-core docs scaffold.

**Acceptance gate:** Operator drops `melonds_libretro.dll` +
`bios7.bin` + `bios9.bin` + `firmware.bin`, scans NDS ROMs, launches
a stylus-driven game (Phantom Hourglass works with mouse-as-touch).

---

## ⬜ Phase 1 — First NDS game running

- ⬜ Operator validation (button-only): NSMB DS, Mario Kart DS.
- ⬜ Operator validation (stylus): Phantom Hourglass, Brain Age,
  Picross DS. Mouse should translate to touch input cleanly.
- ⬜ Save state F5/F8 round-trip.

---

## ⬜ Phase 2 — Polish

- ⬜ **Window-relative pointer coordinates** — Phase 0 uses screen-
  relative mouse via device_query with assumed 1920×1080. Phase 2.5
  polish wires Tauri window context for pixel-perfect mapping to the
  game-output rectangle within the OA window.
- ⬜ **Microphone input** for blow-puzzles + voice puzzles + spoken-
  word minigames (Phantom Hourglass / Brain Age / Hotel Dusk).
- ⬜ **Per-game touch overlay UI** — visual stylus cursor + touch-
  area indicator overlay so operators see where their pointer is
  on the DS bottom screen.
- ⬜ **DSi enhancements** for games with DSi-exclusive features
  (camera, DSiWare). Phase 3+ work — melonDS supports DSi via
  separate firmware files.
- ⬜ **Multi-touch support** — POINTER device's index parameter
  selects between touches; Phase 0 only handles index 0. Niche
  (DS only has single-touch).

---

## ⬜ Phase 3+ — Stretch

- ⬜ DS rumble pak peripheral.
- ⬜ GBA slot peripheral (some DS games — Pokémon, Castlevania
  Portrait of Ruin — use the GBA slot for cart-detection).
