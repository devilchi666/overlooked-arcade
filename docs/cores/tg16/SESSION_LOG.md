# tg16 Session Log

Per-core day-to-day log. Format: date + three lines — **Shipped / Almost / Next**. Cross-cutting milestones (phase boundaries, shell-level features, new systems coming online) go in the project-wide `docs/SESSION_LOG.md`.

---

## 2026-05-16 — Gamepad path live, per-core docs scaffolded

- **Shipped:** Gamepad polling wired in `oa-input` — new `GamepadMapping` table (32 bits × 5 ports, each `Option<gilrs::Button>`) mirrors `KeyboardMapping`. `InputPoller` holds an optional `Gilrs` instance and a `port_pads: [Option<GamepadId>; 5]` array; pads bind to ports in connection order (first plug-in → Port0, etc.), auto-release on disconnect. Already-connected pads at startup are snapshotted via `g.gamepads()` so they don't need a Connected event to take a port. `poll()` ORs keyboard + gamepad bits together. `oa-shell` binds the PCE defaults: dpad → dpad, East = I, South = II, Start = RUN, Select = SELECT (matches RetroArch's Beetle PCE Fast layout). Existing tests (`oa-pce` 3/3) still pass; full workspace `cargo build` clean. Scaffolded `docs/cores/tg16/` per CLAUDE.md spec (README + ROADMAP + SESSION_LOG + KNOWN_GAME_BUGS + DECISIONS).
- **Almost:** CI matrix. Last open Phase 1 item — needs `.github/workflows/ci.yml` running `cargo test` + `cargo tauri build` on Windows / macOS / Linux. Bonk's Adventure on a real gamepad still pending user playtest (operator role).
- **Next:** CI matrix. After CI green, Phase 1 closes and Phase 2 (Solid UI shell + per-system theming + window/scaling modes) opens.

---

## 2026-05-15 — Project bootstrap + Phase 0 spikes (backfilled summary)

See project-wide `docs/SESSION_LOG.md` for the full narrative. tg16-relevant slice:

- **Shipped:** Beetle PCE Fast vendored at `crates/oa-pce-sys/vendor/` (5.5 MB) with ORIGIN.md + PATCHES/ scaffold. Spike 2 confirmed `cc-rs` + MSVC builds the Mednafen-endian helper cleanly with one shim define (`INLINE=__inline`). Spike 3 picked hand-written `extern "C"` over bindgen for all `oa-<sys>-sys` crates (4.8× faster builds, no libclang dep, idiomatic enum names). Spike 1 (Tauri+wgpu two-window) is shell-level, not tg16-specific.
- **Almost:** Real emulation. `PceCore` was a frame-counter-gradient stub at end of this stretch; renderer/audio/input wiring all stubbed too.
- **Next:** Expand `build.rs` to compile the full PCE Fast tree, write `shim.cpp`, swap the stub for real Mednafen.

## 2026-05-15 — Full PCE Fast tree compiles (backfilled)

- **Shipped:** `oa-pce-sys/build.rs` compiles the entire core — 93 object files, 9.6 MB C++ archive + 3.4 MB C archive. Covers `pce_fast/*`, all `mednafen/*` helpers, `mednafen/cdrom/*` (for future CD), `mednafen/sound/Blip_Buffer.c`, `mednafen/tremor/*`, full `deps/libchdr` + `lzma-19.00` + `zstd/lib` + `zlib-1.2.11`. The 2 endian FFI bridge tests still pass through the much-larger lib. Build is incremental-fast (~16s) after cold compile.
- **Almost:** No symbols exposed to Rust yet — `oa-pce-sys/src/lib.rs` still binds only the spike's endian helpers.
- **Next:** Write `shim.cpp` to provide libretro frontend callbacks + the `oa_pce_*` surface.

## 2026-05-15 — Bonk's Adventure runs 🎉 (backfilled)

- **Shipped:** `shim.cpp` (~280 LOC) — full libretro frontend implementation: video/audio/input/environment/log callbacks, RGB565→RGBA8 + 0RGB1555→RGBA8 + XRGB8888→RGBA8 pixel conversion, retro_init/retro_load_game/retro_run/retro_unload_game/retro_deinit lifecycle wired to the `oa_pce_*` surface. Hand-written `extern "C"` bindings in `oa-pce-sys/src/lib.rs`. `oa-pce::PceCore` calls through (handle in `NonNull`, Drop runs `oa_pce_free`, PCE→libretro button-bit remap). `oa-shell` reads `OA_ROM` env var and pipes ROM bytes through. **Bonk's Adventure (USA), 384 KB HuCard, runs at steady ~60 fps for 813 frames** — Beetle reports `Samples/Frame: 734.3` (≈ 44100/60.05) and `Estimated FPS: 60.05`. Renderer auto-detected the 256×243 PCE mode and reallocated the FB texture from 256×239.
- **One integration gotcha worth recording:** Beetle's `retro_load_game` doesn't accept `info->data` directly — when the frontend doesn't provide a file path, the core takes the `RETRO_ENVIRONMENT_GET_GAME_INFO_EXT` path, expecting the frontend to return a `retro_game_info_ext` struct via that environment callback with data+size pointer and extension string. First load returned status 2 (rejection); adding `GET_GAME_INFO_EXT` support fixed it. Comments in `shim.cpp` flag this for next-system bring-up. Captured in `DECISIONS.md`.
- **Almost:** Audio + input. Samples generated but `oa-audio` was a stub; `oa-input` reported zeros.
- **Next:** Wire cpal + keyboard.

## 2026-05-15 — Audio working (backfilled)

- **Shipped:** Real `oa-audio::AudioSink` — cpal default output, SPSC ring buffer via `ringbuf` 0.4, sample-format conversion (i16/f32/u16) inside the callback. First pass had right wiring but wrong rate: Windows opened at 48 kHz while we push 44.1 kHz, producing ~9% pitch shift + underrun gaps. Added a stateful linear-interpolation resampler on the producer side that carries the last source frame across calls (click-free at batch boundaries). Bonk now sounds correct ("much better" — user-verified). `oa-shell` calls `sink.push(core.drain_audio())` after each frame; 0 ring-buffer drops over multi-minute runs.
- **Almost:** Real input. Bit-remap wired but no keyboard/gamepad reads yet.
- **Next:** Keyboard via `device_query`.

## 2026-05-15 — Keyboard input: Bonk playable 🎮 (backfilled)

- **Shipped:** `oa-input::InputPoller` — cross-platform global keyboard polling via `device_query` 4.x. Generic `KeyboardMapping` table (32 bit slots × 5 ports, `Option<Keycode>` each) keeps `oa-input` system-agnostic — the shell binds keys to bit positions using `oa_pce::buttons::*` constants. Default PCE map: arrow keys = d-pad, Z = I, X = II, Enter = RUN, RShift = SELECT. **Bonk's Adventure runs end-to-end with pixels, audio at correct pitch, AND playable keyboard input** — Phase 1 acceptance gate functionally met.
- **Two real integration bugs caught while debugging** (both important and captured in memory + `DECISIONS.md`):
  - `tauri::Window::is_focused()` returns false for native (no-WebView) windows even when they have user focus. Our focus gate prevented any input. For now `set_enabled(true)` unconditionally; proper focus tracking needs Tauri-event routing in Phase 2.
  - **`retro_set_controller_port_device` MUST be called AFTER `retro_load_game`, not before.** Beetle's `MDFNI_LoadGame` re-initializes the core and resets `pce_fast/input.c`'s `data_ptr[]` array, disconnecting any pre-load wiring. Took 4 iterations of diagnostic logging in `cb_input_state` to localise.
- **Almost:** Gamepad. `gilrs` in dep graph but not wired. CI matrix empty.
- **Next:** Optional gamepad via `gilrs`, then CI.
