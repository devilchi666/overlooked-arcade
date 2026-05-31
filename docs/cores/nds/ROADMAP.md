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

- ⬜ Operator validation (button-only): NSMB DS, Mario Kart DS — operator playtest.
- ⬜ Operator validation (stylus): Phantom Hourglass, Brain Age, Picross DS — operator playtest of mouse-as-touch.
- ✅ Save state F5/F8 round-trip — closed by cross-system save-state infra (`oa_libretro::LibretroCore::save_state / load_state`).

---

## ⬜ Phase 2 — Polish

- ✅ **Window-relative pointer coordinates** — shipped via `InputPoller::set_pointer_viewport` + `Renderer::last_viewport()` + per-frame wiring in `apps/oa-shell/src/main.rs:5382-5393`.
- ⬜ **Microphone input** for blow-puzzles + voice puzzles — gated on libretro mic device dispatch (deferred-until-forced).
- 🟨 **Per-game touch overlay UI** — split:
  - ✅ **Visual stylus reticle** — `StylusOverlay` component (frontend/src/components/StylusOverlay.tsx) renders a hollow accent-colored ring at the cursor position while an NDS game is running; fills in + scales down on left-mouse-down so the operator sees explicit tap feedback the OS cursor doesn't provide. Mounted alongside SystemBackground in App.tsx, gated on `runningEntry?.systemId === "nds"` and a `STYLUS_SYSTEMS` set that can extend to other stylus-using systems later. Shipped 2026-05-27 in the system-fixes branch.
  - ⬜ **Per-game touch-area indicator overlay** — game-specific touch hotspots (Phantom Hourglass map screen, Mario Kart DS course-selection, Brain Age stylus zones, etc.). Needs per-game configuration data; out of scope for the visual-reticle slice.
- ⬜ **DSi enhancements** for games with DSi-exclusive features (camera, DSiWare) — deferred (Phase 3+).
- ✅ **Multi-touch support** — POINTER device's `index` parameter now dispatches index 0 → primary pointer, index 1 → secondary, ≥2 → zero; `POINTER_COUNT` reports 0/1/2 pressed total. `oa_core::InputState.pointer_secondary` companion field + `State.input_pointer_secondary[port]` mirror + `pointer_field_value` signature carries both tuples + index. V1 plumbing only: `InputPoller::poll` leaves secondary at `(0, 0, false, false)` until a real second-finger input source is wired (operator-driven; second-mouse / actual touchscreen / Surface-pen path are all additive at the poll site). Cores polling `index = 1` won't crash; they'll just see "no finger" until the source is added. Tests in `crates/oa-libretro/src/state.rs::tests::pointer_field_value_*` cover the index dispatch + COUNT semantics across both slots.

---

## ⬜ Phase 3+ — Stretch

- ⬜ DS rumble pak peripheral — gated on rumble infra.
- ⬜ GBA slot peripheral — deferred.
