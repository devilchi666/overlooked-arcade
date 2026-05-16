# Roadmap

High-level phase plan. Mirrors §7 of the approved setup plan (`C:\Users\Devilchi\.claude\plans\jazzy-spinning-blum.md`). Update when phase scope shifts or a phase completes.

Status legend: ⬜ not started · 🟨 in progress · ✅ complete.

---

## ✅ Phase 0 — Spikes (completed 2026-05-15, same-day)

Three short-lived spikes proved the highest-risk integrations work. Phase 1 scaffolding unblocked.

- ✅ Tauri 2 + wgpu two-window spike. **Pass.** 60.2 fps steady; Tauri 2.11.1 + wgpu 23.0.1; `tauri::WindowBuilder` requires the `"unstable"` feature. Scratch: `scripts/spikes/01-tauri-wgpu-twowindow/`.
- ✅ Beetle PCE Fast build spike. **Pass.** `cc-rs` + MSVC 14.44 compiles Mednafen-derived C cleanly with one shim define (`INLINE=__inline`). Three FFI calls verified. Bonus: Beetle PCE Fast ships `pcecd.cpp` — Phase 5 may not need full Mednafen. Scratch: `scripts/spikes/02-beetle-pce-build/`.
- ✅ Bindgen surface decision. **Pass.** Hand-written `extern "C"` wins for our use case (4.8× faster builds, zero libclang dependency, idiomatic enum names). Scratch: `scripts/spikes/03-bindgen-vs-handwritten/`.

All three decisions recorded in `docs/DECISIONS.md`.

---

## 🟨 Phase 1 — Skeleton + first PCE ROM running (3-4 weeks)

- ✅ Cargo workspace scaffolded. 10 crates + `apps/oa-shell` binary; `cargo build` clean in 78s.
- ✅ `oa-core` trait defined. Core, Framebuffer, Timing, InputState, PortIndex, SystemId, CoreError.
- ✅ `oa-pce-sys` + `oa-pce` integrated. Full Beetle PCE Fast build (93 object files, 13 MB native libs). C++ shim layer (`shim.cpp`) implements libretro frontend callbacks + pixel format conversion + the `oa_pce_*` surface from Spike 3. `oa-pce::PceCore` wraps the C handle in `NonNull`, runs Drop cleanup, and translates between our PCE button bitfield and libretro's joypad numbering. **Bonk's Adventure (USA) HuCard runs at 60.05 fps with Beetle reporting `Samples/Frame: 734.3`.**
- ✅ `oa-render` minimal output pipeline. wgpu Surface + RGBA8 texture cache + fullscreen-triangle blit pipeline with nearest-neighbour sampler. Picks first sRGB surface format, locks to Vsync (FIFO). Tested end-to-end with the PceCore stub at 59.8 fps for 3360+ frames.
- ✅ `oa-audio` cpal sink with ring buffer + linear-interp resampler. Opens device default rate (Windows 48 kHz typical), resamples from 44.1 kHz PCE rate, zero overflow drops over multi-minute runs.
- 🟨 `oa-input` keyboard + gilrs. **Keyboard live** via `device_query` global polling + generic `KeyboardMapping` table; default PCE map (arrows, Z, X, Enter, RShift). Gamepad via gilrs is dep-wired but not yet implemented.
- ✅ `oa-shell` two-window flow. WebView library window + native game window with wgpu surface; combined emu+render thread ticks `PceCore` at 59.826 Hz native and presents each frame. ROM path via `OA_ROM` env var.
- ⬜ CI matrix (Windows/macOS/Linux) green.

**Acceptance gate:** Bonk's Adventure (HuCard) runs at 60 fps with audio, gamepad-controlled. **Status: pixels at 60 fps ✅, audio at correct pitch ✅, keyboard playable ✅, gamepad ⬜.** (Effectively met for the core flow; gamepad is a nice-to-have on the same poller.)

---

## ⬜ Phase 2 — Premium UI shell (4-6 weeks)

- ⬜ Solid + Tailwind scaffold.
- ⬜ Per-system theming via Tailwind config + CSS variables. TG-16 theme first.
- ⬜ Library grid + cover-art ingestion.
- ⬜ Game launch UX (smooth transition library → game).
- ⬜ Save state UI: timeline of thumbnails.
- ⬜ Settings panel — input mapping, audio device, plus the window + scaling modes below.
- ⬜ **Window modes** (game window — `oa-render` + `oa-shell`):
  - Windowed (resizable, current default)
  - Windowed fullscreen (borderless, fills the monitor, alt-tab friendly)
  - Exclusive fullscreen (proper FS, lowest input/present latency)
  - Monitor selection when multiple displays are attached
- ⬜ **Video / scaling modes** (renderer-side, viewport math in `oa-render`):
  - **Pixel Perfect** — largest integer scale that fits, native aspect, letterbox the rest
  - **Aspect-correct fit** — scale to fit window while preserving the system's native aspect ratio (PCE non-square pixels handled per-system)
  - **Stretched** — fill window, ignore aspect
  - **Original 1:1** — native resolution centered in window, no scaling
  - **Explicit integer multiple** — 2× / 3× / 4× / etc.
  - Per-game default mode (saved in settings)
- ⬜ Single-window-mode spike (1 week within Phase 2).

**Acceptance gate:** Library → pick ROM → game runs full-screen → ESC → save-state timeline → restore. UI looks like 2026.

---

## ⬜ Phase 3 — Shader pipeline (3-4 weeks)

- ⬜ WGSL passes: scanline, CRT curve, phosphor decay, bezel overlay.
- ⬜ Per-game shader preset format (TOML schema).
- ⬜ Live shader hot-reload in dev.
- ⬜ HDR tone mapping (behind setting, where display supports it).

**Acceptance gate:** Per-system default presets ship. Per-game override works. Preset survives restart.

---

## ⬜ Phase 4 — Differentiator features (4-6 weeks)

- ⬜ Rewind-scrubbing UI.
- ⬜ TAS recording + deterministic replay.
- ⬜ Frame-by-frame WebM export.
- ⬜ Memory inspector (dev/power-user mode).
- ⬜ Per-game milestone tracking.

---

## ⬜ Phase 5 — PCE-CD bringup (2-3 weeks)

- ⬜ Vendor Mednafen PCE (full, with CD) — Beetle PCE Fast doesn't ship CD.
- ⬜ `oa-cdrom` CHD/CUE/BIN loader.

**Acceptance gate:** Rondo of Blood boots from CHD, CDDA plays, gameplay starts.

---

## ⬜ Phase 6+ — Next systems (first-wave, then ongoing)

First-wave order: Lynx → 7800 → SMS/GG → MSX/MSX2 → ColecoVision → Vectrex → Virtual Boy → WonderSwan. After the first wave, additions are continuous — the project's long-term ambition is to host almost all of retro gaming (see `docs/VISION.md` for the broader picture, including the bigger list of likely future systems beyond the first wave).

Per-system steady-state recipe (the 8-step pattern documented in memory `feedback_multi_core_architecture_ready.md`):

1. Vendor upstream into `crates/oa-<sys>-sys/vendor/` + ORIGIN.md + PATCHES/
2. `build.rs` with cc-rs file list + integration `#define` shims
3. `shim.cpp` exposing the `oa_<sys>_*` C surface (same 9-function shape as PCE)
4. `crates/oa-<sys>/src/lib.rs` — `Core` impl + button-bit remap
5. Add to workspace members
6. Register in `oa-shell::core_registry`
7. Theme the system page (Solid UI, Phase 2+)
8. Per-core docs at `docs/cores/<sys>/`

Goal: 4-8 weeks per new system. The trait, renderer, audio, and input crates need zero changes per system — the multi-core architecture was wired in day one (see Phase 1 retrospective in `docs/SESSION_LOG.md`).
