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

## ⬜ Phase 1 — First Lynx ROM running

- ⬜ Operator validation: launch a real `.lnx` ROM end-to-end (pixels + audio + controller).
- ⬜ Save state F5/F8 round-trip confirmation. Mednafen Lynx supports `retro_serialize` so this should work via the existing path, but it hasn't been live-tested.
- ⬜ Multi-region testing: load USA + Europe + Japan ROM dumps to confirm BIOS region handling.
- ⬜ Per-game cover sync via libretro-thumbnails — **infra ready 2026-05-19, needs operator validation.** Mapping `lynx → Atari_-_Lynx` shipped in `apps/oa-shell/src/media.rs::repo_for_system_id`. Operator: run `Settings → Library → Sync media for Lynx` and confirm covers download.

**Acceptance gate:** A reference set of Lynx games (California Games, Blue Lightning, Lynx Casino, Crystal Mines II) all run with pixels + audio + working controller at native 75 Hz.

---

## ⬜ Phase 2 — Polish

- ⬜ Per-system shader tweaks: the 102-row source paints visually denser scanlines than tg16's 239-row source. Either a system-scoped scanline-intensity parameter or a per-system shader preset default that drops to `crt-lite`'s lower-period mode at low resolutions. Lands alongside Phase 3 slice C's TOML preset format.
- ⬜ Pause-button hotkey decision: the Lynx had a dedicated Pause button distinct from Option 1/2. Currently bound to libretro L; might want a more reachable face button on small gamepads (e.g. Mode/Home).
- ⬜ Library theming validation: the Lynx purple accent reads on dark backgrounds (slice 0 visual check); operator confirms it doesn't clash with the toolbar / sidebar regions in single-window gameplay overlay mode.

---

## ⬜ Phase 3+ — Stretch

Per the project ROADMAP, all post-Phase-3 work (rewind, TAS, WebM export, memory inspector) is system-agnostic and lights up automatically once the engine work ships. Lynx-specific items:

- ⬜ Lynx multiplayer comm port — the original Lynx supported up to 8-player ComLynx via a daisy chain cable. mednafen-lynx exposes this via a network option. Out of OA scope until a real demand surfaces.
- ⬜ Custom forked Lynx core — only if upstream regresses on a game we care about, or we want OA-specific extensions (e.g. per-game core options exposed through our settings UI). The recipe mirrors the modified Beetle PCE Fast plan: separate libretro-frontend build that emits a .dll we ship.

---

## Scope clarifications

- **No PCE-style vendoring for Lynx today.** The libretro pivot means we ship the upstream nightly .dll alongside our binary and tell operators to drop it into `<exe_dir>/cores/`. If we ever modify the core, we maintain a separate libretro-frontend build of our patched source and ship that .dll instead — see project `DECISIONS.md` 2026-05-16 entry.
- **BIOS responsibility is the operator's.** OA refuses to bundle `lynxboot.img` (copyright-distinct from the homebrew ROMs the user might own). The operator drops it into `<exe_dir>/system/` themselves.

---

## 2026-05-21 — Stale-cleanup audit

The Phase 1+ items above were written when this system onboarded, before cross-system infrastructure (Phases 1.5 / 2.5–2.8 / 3 / 4 + direct-launch CLI) landed. Many `⬜` items are actually shipped — see `docs/cores/AUDIT_2026-05-21.md` for the per-item breakdown (stale vs open-code vs open-operator) for this system.
