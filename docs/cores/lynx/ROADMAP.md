# lynx — Roadmap

Per-core phase tracking. Mirrors the project-wide ROADMAP shape (Phase 1 = first ROM running, Phase 2 = UI polish, Phase 3 = shaders, etc.) but scoped to Lynx.

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Onboarding (2026-05-18)

Core comes online as part of the project's transition to a multi-core launcher. The libretro pivot means Lynx didn't need a new Rust crate — it's a `.dll` operator drop-in.

- ✅ System registered in `frontend/src/themes/registry.ts` (SystemId union extended, `lynx` entry with `.lnx` / `.lyx` extensions + landscape tile aspect).
- ✅ Theme block in `frontend/src/themes/systems.css` — purple accent from the Lynx box-art family.
- ✅ Per-system button bits + bindings in `apps/oa-shell/src/bindings.rs::lynx` + `LYNX_BUTTONS` + `default_lynx_bindings()` + `defaults_for("lynx")` arm.
- ✅ `lynx_to_libretro_bits` identity remap (bits laid out to match libretro positions directly).
- ✅ `bindings::to_libretro_bits` + `bit_for` + `buttons_for` per-system dispatch — replaces the previous tg16-only hardcoded remap.
- ✅ `system_id` threaded through `EmuCommand::LoadRom`, `launch_rom` Tauri command, and the emu thread's `current_system_id` state.
- ✅ Per-core docs scaffold (this directory).

**Acceptance gate:** Operator can drop `mednafen_lynx_libretro.dll` + `lynxboot.img` into the install, scan a Lynx ROMs folder, see Lynx-themed tiles appear in the library, and click one to launch — without rebuilding Rust.

---

## 🟨 Phase 1 — First Lynx ROM running

- ✅ Operator validation: launch a real `.lnx` ROM end-to-end (pixels + audio + controller) — operator confirmed working 2026-05-27. Multi-region testing still open (Phase 1 not fully closed until USA / Europe / Japan all confirmed).
- ✅ Save state F5/F8 round-trip — closed by cross-system save-state infra (`oa_libretro::LibretroCore::save_state / load_state`).
- ⬜ Multi-region testing: load USA + Europe + Japan ROM dumps to confirm BIOS region handling — operator playtest.
- ✅ Per-game cover sync via libretro-thumbnails — closed by cross-system media sync (`media::sync_media_for_system`).

**Acceptance gate:** A reference set of Lynx games (California Games, Blue Lightning, Lynx Casino, Crystal Mines II) all run with pixels + audio + working controller at native 75 Hz.

---

## ⬜ Phase 2 — Polish

- ⬜ Per-system shader tweaks — operator-driven shader-preset choice (per-system shader override is shipped cross-system).
- ⬜ Pause-button hotkey decision — operator-driven binding preference.
- ⬜ Library theming validation — operator visual check (per-system theming infra shipped cross-system via `frontend/src/themes/systems.css` + `registry.ts`).

---

## ⬜ Phase 3+ — Stretch

Lynx-specific items:

- ⬜ Lynx multiplayer comm port — deferred (out of OA scope until a real demand surfaces).
- ⬜ Custom forked Lynx core — deferred.

---

## Scope clarifications

- **No PCE-style vendoring for Lynx today.** The libretro pivot means we ship the upstream nightly .dll alongside our binary and tell operators to drop it into `<exe_dir>/cores/`. If we ever modify the core, we maintain a separate libretro-frontend build of our patched source and ship that .dll instead — see project `DECISIONS.md` 2026-05-16 entry.
- **BIOS responsibility is the operator's.** OA refuses to bundle `lynxboot.img` (copyright-distinct from the homebrew ROMs the user might own). The operator drops it into `<exe_dir>/system/` themselves.
